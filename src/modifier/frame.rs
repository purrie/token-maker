use std::sync::Arc;

use iced::{
    widget::{
        button, column as col, radio, row, scrollable, scrollable::Properties, text, vertical_space,
    },
    Alignment, Color, Command, Length, Size,
};

use image::imageops::resize;

use crate::{
    data::{FrameImage, ProgramData, WorkspaceData},
    persistence::PersistentKey,
    style::Style,
};
use crate::{
    image::{GrayscaleImage, ImageOperation, RgbaImage},
    widgets::ColorPicker,
};

use super::{Modifier, ModifierOperation};

#[derive(Debug, Clone)]
pub enum FrameMessage {
    /// Result of resizing the frame to expected export size
    NewFrame(Arc<RgbaImage>, Option<Arc<GrayscaleImage>>),
    /// Changes the tint of the frame
    SetTint(Color),
    /// Opens the frame selection screen
    OpenFrameSelect,
    /// Signals the user selected a frame
    FrameSelected(usize),
    /// Cancels the frame browsing
    CancelFrame,
    /// Updates the filter for the frame
    SetFilter(String),
}

/// Frame modifier draws a frame around the image, optionally masking out any part that would stick out
#[derive(Clone, Debug, Default)]
pub struct Frame {
    /// Frame image to be put onto the source image
    image: Option<Arc<RgbaImage>>,
    mask: Option<Arc<GrayscaleImage>>,
    dirty: bool,
    select_frame: bool,
    tint: Color,
    filter: String,

    source: Option<Arc<RgbaImage>>,
    source_mask: Option<Arc<GrayscaleImage>>,
}

impl<'a> Modifier<'a> for Frame {
    type Message = FrameMessage;

    fn create(pdata: &ProgramData, wdata: &WorkspaceData) -> (Command<Self::Message>, Self) {
        let mut s = Self {
            tint: Color::WHITE,
            ..Default::default()
        };
        let c = if let Some(frame) = pdata
            .cache
            .get(PersistentData::ID, wdata.template)
            .and_then(|x| x.check_string())
        {
            match pdata.available_frames.iter().find(|x| x.id() == frame) {
                Some(f) => s.set_frame(f, wdata),
                None => {
                    s.select_frame = true;
                    Command::none()
                }
            }
        } else {
            Command::none()
        };
        (c, s)
    }

    fn label() -> &'static str {
        "Frame"
    }

    fn is_dirty(&self) -> bool {
        self.dirty
    }

    fn set_clean(&mut self) {
        self.dirty = false;
    }

    fn get_image_operation(
        &self,
        _pdata: &ProgramData,
        _wdata: &WorkspaceData,
    ) -> ModifierOperation {
        if let Some(img) = &self.image {
            if let Some(msk) = &self.mask {
                (
                    ImageOperation::Mask { mask: msk.clone() },
                    ImageOperation::Blend {
                        overlay: img.clone(),
                    },
                )
                    .into()
            } else {
                ImageOperation::Blend {
                    overlay: img.clone(),
                }
                .into()
            }
        } else {
            ModifierOperation::None
        }
    }

    fn wants_main_view(&self, _pdata: &ProgramData, _wdata: &WorkspaceData) -> bool {
        self.select_frame
    }

    fn properties_update(
        &mut self,
        message: Self::Message,
        pdata: &mut ProgramData,
        wdata: &mut WorkspaceData,
    ) -> Command<Self::Message> {
        match message {
            FrameMessage::OpenFrameSelect => {
                self.select_frame = true;
                Command::none()
            }
            FrameMessage::FrameSelected(index) => {
                let Some(f) = pdata.available_frames.get(index) else {
                    return Command::none();
                };
                pdata.cache.set(PersistentData::ID, wdata.template, f.id());
                self.set_frame(f, wdata)
            }
            FrameMessage::CancelFrame => {
                self.select_frame = false;
                Command::none()
            }
            FrameMessage::NewFrame(frame, mask) => {
                self.image = Some(frame);
                self.mask = mask;
                self.dirty = true;
                self.select_frame = false;
                Command::none()
            }
            FrameMessage::SetTint(c) => {
                self.tint = c;
                if let Some(frame) = &self.source {
                    Command::perform(
                        update_frame(
                            frame.clone(),
                            self.source_mask.clone(),
                            self.tint,
                            wdata.export_size,
                        ),
                        |x| FrameMessage::NewFrame(x.0, x.1),
                    )
                } else {
                    Command::none()
                }
            }
            FrameMessage::SetFilter(f) => {
                self.filter = f;
                Command::none()
            }
        }
    }

    fn workspace_update(
        &mut self,
        _pdata: &ProgramData,
        wdata: &WorkspaceData,
    ) -> Command<Self::Message> {
        let Some( frame ) = &self.image else {
            return Command::none();
        };
        if frame.width() != wdata.export_size.width || frame.height() != wdata.export_size.height {
            if let Some(source) = &self.source {
                Command::perform(
                    update_frame(
                        source.clone(),
                        self.source_mask.clone(),
                        self.tint,
                        wdata.export_size,
                    ),
                    |x| FrameMessage::NewFrame(x.0, x.1),
                )
            } else {
                Command::none()
            }
        } else {
            Command::none()
        }
    }

    fn properties_view(
        &self,
        _pdata: &ProgramData,
        _wdata: &WorkspaceData,
    ) -> Option<iced::Element<Self::Message, iced::Renderer>> {
        Some(
            col![
                button("Select Frame").on_press(FrameMessage::OpenFrameSelect),
                text("Tint: "),
                ColorPicker::new(self.tint, |c| FrameMessage::SetTint(c))
                    .width(Length::Fixed(32.0))
                    .height(Length::Fixed(32.0)),
            ]
            .spacing(10)
            .into(),
        )
    }

    fn main_view(
        &self,
        _image: iced_native::image::Handle,
        pdata: &ProgramData,
        _wdata: &WorkspaceData,
    ) -> iced::Element<Self::Message, iced::Renderer> {
        // Images column is there to store all the frame buttons
        let mut images = col![]
            .align_items(iced::Alignment::Center)
            .padding(2)
            .width(Length::Fill)
            .height(Length::Shrink);

        // those counters are used for both messaging to know which button was clicked and for layout
        let mut total = 0;
        let mut count = 0;

        let mut row = row![]
            .align_items(iced::Alignment::Center)
            .padding(2)
            .spacing(2)
            .width(Length::Fill)
            .height(Length::Shrink);

        // this collects frames in rows
        for img in pdata.available_frames.iter() {
            if self.filter.len() > 0 {
                if self.filter.as_str() != img.category() {
                    total += 1;
                    continue;
                }
            }
            if count > 3 {
                count = 0;
                images = images.push(row);
                row = row![]
                    .align_items(iced::Alignment::Center)
                    .padding(2)
                    .spacing(2)
                    .width(Length::Fill)
                    .height(Length::Shrink);
            }
            row = row.push(
                button(iced::widget::image(img.preview()).content_fit(iced::ContentFit::Contain))
                    .on_press(FrameMessage::FrameSelected(total))
                    .width(Length::Fill)
                    .style(Style::Frame.into()),
            );
            total += 1;
            count += 1;
        }

        // last row won't be pushed in the loop so if it has any items in it, we add it here
        if count > 0 {
            images = images.push(row);
        }

        let filter = pdata.available_frames.iter().fold(Vec::new(), |mut v, f| {
            if v.contains(&f.category()) == false {
                v.push(f.category());
            }
            v
        });

        let filter = filter
            .iter()
            .map(|x| {
                radio(x.as_str(), x.as_str(), Some(self.filter.as_str()), |x| {
                    FrameMessage::SetFilter(x.to_string())
                })
            })
            .fold(
                row![
                    text("Filter: "),
                    radio("All", "", Some(self.filter.as_str()), |x| {
                        FrameMessage::SetFilter(x.to_string())
                    })
                ]
                .spacing(4)
                .padding(2)
                .align_items(Alignment::Center),
                |r, w| r.push(w),
            );

        // adding vertical space just so the scrollbar doesn't cover the radio buttons
        let filter = col![filter, vertical_space(10)];
        let filter = scrollable(filter).horizontal_scroll(Properties::default());

        col![
            row![
                col![
                    button("Cancel").on_press(FrameMessage::CancelFrame),
                    vertical_space(10)
                ],
                filter,
            ]
            .align_items(Alignment::Center)
            .spacing(10)
            .padding(4)
            .height(Length::Shrink),
            scrollable(images).height(Length::Fill)
        ]
        .width(Length::Fill)
        .into()
    }
}

impl Frame {
    /// Sets the frame image to be used within the frame. It returns a task to resize the frame image to the same size as expected export size
    fn set_frame(&mut self, frame: &FrameImage, wdata: &WorkspaceData) -> Command<FrameMessage> {
        self.select_frame = false;
        self.source = Some(frame.image());
        self.source_mask = frame.mask();
        let mask = frame.mask();
        let frame = frame.image();
        Command::perform(
            update_frame(frame, mask, self.tint, wdata.export_size),
            |x| FrameMessage::NewFrame(x.0, x.1),
        )
    }
}

/// Function performs resizing operations on the frame and its mask to match the export size
async fn update_frame(
    frame: Arc<RgbaImage>,
    mask: Option<Arc<GrayscaleImage>>,
    tint: Color,
    size: Size<u32>,
) -> (Arc<RgbaImage>, Option<Arc<GrayscaleImage>>) {
    let mut frame = resize(
        frame.as_ref(),
        size.width,
        size.height,
        image::imageops::FilterType::Nearest,
    );

    frame.pixels_mut().filter(|x| x[3] > 0).for_each(|x| {
        let r = (x[0] as f32 / u8::MAX as f32) * tint.r;
        let g = (x[1] as f32 / u8::MAX as f32) * tint.g;
        let b = (x[2] as f32 / u8::MAX as f32) * tint.b;
        x[0] = (r * u8::MAX as f32) as u8;
        x[1] = (g * u8::MAX as f32) as u8;
        x[2] = (b * u8::MAX as f32) as u8;
    });

    if let Some(mask) = mask {
        let mask = resize(
            mask.as_ref(),
            size.width,
            size.height,
            image::imageops::FilterType::Nearest,
        );
        (Arc::new(frame), Some(Arc::new(mask)))
    } else {
        (Arc::new(frame), None)
    }
}

enum PersistentData {
    ID,
}

impl PersistentKey for PersistentData {
    fn get_id(&self) -> &'static str {
        match self {
            PersistentData::ID => "modifier-frame",
        }
    }
}
