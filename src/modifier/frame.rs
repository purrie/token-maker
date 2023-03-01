use std::sync::Arc;

use iced::{
    widget::{button, column as col, horizontal_space, row, scrollable},
    Command, Length, Size,
};

use image::imageops::resize;

use crate::data::{FrameImage, ProgramData, WorkspaceData};
use crate::image::{GrayscaleImage, ImageOperation, RgbaImage};

use super::{Modifier, ModifierOperation};

#[derive(Debug, Clone)]
pub enum FrameMessage {
    /// Result of resizing the frame to expected export size
    NewFrame(Arc<RgbaImage>, Option<Arc<GrayscaleImage>>),
    /// Opens the frame selection screen
    OpenFrameSelect,
    /// Signals the user selected a frame
    FrameSelected(usize),
    /// Cancels the frame browsing
    CancelFrame,
}

/// Frame modifier draws a frame around the image, optionally masking out any part that would stick out
#[derive(Clone, Debug, Default)]
pub struct Frame {
    /// Frame image to be put onto the source image
    image: Option<Arc<RgbaImage>>,
    mask: Option<Arc<GrayscaleImage>>,
    dirty: bool,
    select_frame: bool,

    source: Option<Arc<RgbaImage>>,
    source_mask: Option<Arc<GrayscaleImage>>,
}

impl Modifier for Frame {
    type Message = FrameMessage;

    fn create(pdata: &ProgramData, wdata: &WorkspaceData) -> (Command<Self::Message>, Self) {
        let mut s = Self {
            select_frame: true,
            ..Default::default()
        };
        let c = if let Some(frame) = pdata
            .cache
            .get("frame-modifier", wdata.template.get_id())
            .and_then(|x| x.check_string())
        {
            match pdata.available_frames.iter().find(|x| x.name() == frame) {
                Some(f) => s.set_frame(f, wdata),
                None => Command::none(),
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
                pdata.cache.set(
                    "frame-modifier",
                    wdata.template.get_id().to_string(),
                    f.name(),
                );
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
                    update_frame(source.clone(), self.source_mask.clone(), wdata.export_size),
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
            button("Select Frame")
                .on_press(FrameMessage::OpenFrameSelect)
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
            .height(Length::Fill);
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
                    .on_press(FrameMessage::FrameSelected(total)),
            );
            total += 1;
            count += 1;
        }
        // last row won't be pushed in the loop so if it has any items in it, we add it here
        if count > 0 {
            images = images.push(row);
        }
        col![
            row![
                button("Cancel").on_press(FrameMessage::CancelFrame),
                horizontal_space(Length::Fill),
            ]
            .height(Length::Shrink),
            scrollable(images)
        ]
        .width(Length::Fill)
        .into()
    }
}

impl Frame {
    /// Sets the frame image to be used within the frame. It returns a task to resize the frame image to the same size as expected export size
    fn set_frame(&mut self, frame: &FrameImage, wdata: &WorkspaceData) -> Command<FrameMessage> {
        self.source = Some(frame.image());
        self.source_mask = frame.mask();
        let size = wdata.export_size;
        let mask = frame.mask();
        let frame = frame.image();
        Command::perform(update_frame(frame, mask, size), |x| {
            FrameMessage::NewFrame(x.0, x.1)
        })
    }
}

/// Function performs resizing operations on the frame and its mask to match the export size
async fn update_frame(
    frame: Arc<RgbaImage>,
    mask: Option<Arc<GrayscaleImage>>,
    size: Size<u32>,
) -> (Arc<RgbaImage>, Option<Arc<GrayscaleImage>>) {
    let frame = resize(
        frame.as_ref(),
        size.width,
        size.height,
        image::imageops::FilterType::Nearest,
    );
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
