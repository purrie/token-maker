use iced::{
    widget::{button, column as col, horizontal_space, row, slider, text, tooltip},
    Color, Command, Length, Vector,
};

use crate::{
    image::ImageOperation,
    style::Style,
    widgets::{ColorPicker, PixelSampler},
};

use super::Modifier;

#[derive(Debug, Clone)]
pub struct Greenscreen {
    color: Color,
    range: f32,
    blending: f32,

    dirty: bool,
    sampling_pixel: bool,
}

#[derive(Debug, Clone)]
pub enum GreenscreenMessage {
    SetColor(Color),
    SetRange(f32),
    SetBlending(f32),
    StartSampling,
    StopSampling,
    SetSample(Vector<u32>),
}

impl<'a> Modifier<'a> for Greenscreen {
    type Message = GreenscreenMessage;

    fn get_image_operation(
        &self,
        _pdata: &crate::data::ProgramData,
        _wdata: &crate::data::WorkspaceData,
    ) -> super::ModifierOperation {
        ImageOperation::MaskColor {
            color: self.color,
            range: self.range,
            soft_border: self.blending,
        }
        .into()
    }

    fn create(
        _pdata: &crate::data::ProgramData,
        _wdata: &crate::data::WorkspaceData,
    ) -> (iced::Command<Self::Message>, Self) {
        (
            Command::none(),
            Self {
                color: Color::WHITE,
                range: 0.1,
                blending: 0.01,
                dirty: true,
                sampling_pixel: false,
            },
        )
    }

    fn label() -> &'static str {
        "Greenscreen"
    }

    fn tooltip() -> &'static str {
        "Hides parts of the image that match selected color"
    }

    fn is_dirty(&self) -> bool {
        self.dirty
    }

    fn set_clean(&mut self) {
        self.dirty = false;
    }

    fn wants_main_view(
        &self,
        _pdata: &crate::data::ProgramData,
        _wdata: &crate::data::WorkspaceData,
    ) -> bool {
        self.sampling_pixel
    }

    fn properties_update(
        &mut self,
        message: Self::Message,
        _pdata: &mut crate::data::ProgramData,
        wdata: &mut crate::data::WorkspaceData,
    ) -> Command<Self::Message> {
        match message {
            GreenscreenMessage::SetColor(color) => {
                self.color = color;
                self.dirty = true;
                Command::none()
            }
            GreenscreenMessage::SetRange(range) => {
                self.range = range;
                self.dirty = true;
                Command::none()
            }
            GreenscreenMessage::SetBlending(blending) => {
                self.blending = blending;
                self.dirty = true;
                Command::none()
            }
            GreenscreenMessage::StartSampling => {
                self.sampling_pixel = true;
                Command::none()
            }
            GreenscreenMessage::StopSampling => {
                self.sampling_pixel = false;
                Command::none()
            }
            GreenscreenMessage::SetSample(pixel) => {
                let pixel = wdata.source.get_pixel(pixel.x, pixel.y);
                self.color = Color {
                    r: pixel[0] as f32 / 255.0,
                    g: pixel[1] as f32 / 255.0,
                    b: pixel[2] as f32 / 255.0,
                    a: 1.0,
                };
                self.sampling_pixel = false;
                self.dirty = true;
                Command::none()
            }
        }
    }

    fn properties_view(
        &'a self,
        _pdata: &'a crate::data::ProgramData,
        _wdata: &'a crate::data::WorkspaceData,
    ) -> Option<iced::Element<Self::Message, iced::Renderer>> {
        let picker = ColorPicker::new(self.color, |x| GreenscreenMessage::SetColor(x))
            .width(26)
            .height(26);
        let butt = if self.sampling_pixel {
            button("Cancel Sampling").on_press(GreenscreenMessage::StopSampling)
        } else {
            button("Sample from image").on_press(GreenscreenMessage::StartSampling)
        };

        let slider_range =
            slider(0.0..=1.0, self.range, |x| GreenscreenMessage::SetRange(x)).step(0.001);
        let slider_blend = slider(0.0..=1.0, self.blending, |x| {
            GreenscreenMessage::SetBlending(x)
        })
        .step(0.001);

        let label_threshold = text("Threshold: ").width(Length::Fill);
        let label_soft_edge = text("Soft Edge: ").width(Length::Fill);

        let label_threshold = tooltip(
            label_threshold,
            "Determines how close the color has to be to the selected color to count as part of the mask.",
            tooltip::Position::Bottom
        ).style(Style::Frame);

        let label_soft_edge = tooltip(
            label_soft_edge,
            "Allows the mask to extend further than the threshold, softening up the mask edges.",
            tooltip::Position::Bottom,
        )
        .style(Style::Frame);

        Some(
            col![
                row![butt, picker]
                    .spacing(4)
                    .align_items(iced::Alignment::Center),
                row![
                    label_threshold,
                    slider_range.width(Length::FillPortion(4)),
                    horizontal_space(Length::FillPortion(2))
                ]
                .spacing(4),
                row![
                    label_soft_edge,
                    slider_blend.width(Length::FillPortion(4)),
                    horizontal_space(Length::FillPortion(2))
                ]
                .spacing(4),
            ]
            .spacing(6)
            .into(),
        )
    }

    fn main_view(
        &'a self,
        _pdata: &'a crate::data::ProgramData,
        wdata: &'a crate::data::WorkspaceData,
    ) -> iced::Element<Self::Message, iced::Renderer> {
        let picker = PixelSampler::new(wdata.source_preview.clone(), |x| {
            GreenscreenMessage::SetSample(x)
        });
        picker.into()
    }
}
