use std::sync::Arc;

use iced::widget::{button, column as col, horizontal_space, row, slider, text, tooltip};
use iced::{Command, Length, Point, Vector};

use crate::image::convert::pixel_to_color;
use crate::image::operations::flood_fill_mask;
use crate::image::{GrayscaleImage, ImageOperation, RgbaImage};
use crate::style::Style;
use crate::widgets::PixelSampler;

use super::{Modifier, ModifierOperation};

#[derive(Debug, Clone)]
pub struct FloodMask {
    mask: Option<Arc<GrayscaleImage>>,
    treshhold: f32,
    soft_border: f32,
    start: Point,

    dirty: bool,
    picking_pixel: bool,
    rendering: bool,
}

#[derive(Debug, Clone)]
pub enum FloodMaskMessage {
    StartPicking,
    StopPicking,
    Picked(Point),
    GotMask(Arc<GrayscaleImage>),
    SetTolerance(f32),
    SetSoftBorder(f32),
}

impl<'a> Modifier<'a> for FloodMask {
    type Message = FloodMaskMessage;

    fn properties_update(
        &mut self,
        message: Self::Message,
        _pdata: &mut crate::data::ProgramData,
        wdata: &mut crate::data::WorkspaceData,
    ) -> Command<Self::Message> {
        match message {
            FloodMaskMessage::StartPicking => {
                self.picking_pixel = true;
                Command::none()
            }
            FloodMaskMessage::StopPicking => {
                self.picking_pixel = false;
                Command::none()
            }
            FloodMaskMessage::Picked(point) => {
                self.start = point;
                Command::perform(
                    regenerate_mask(
                        wdata.source.clone(),
                        self.start,
                        self.treshhold,
                        self.soft_border,
                    ),
                    |x| FloodMaskMessage::GotMask(x),
                )
            }
            FloodMaskMessage::SetTolerance(v) => {
                self.treshhold = v;
                if self.rendering {
                    return Command::none();
                }
                self.rendering = true;
                Command::perform(
                    regenerate_mask(
                        wdata.source.clone(),
                        self.start,
                        self.treshhold,
                        self.soft_border,
                    ),
                    |x| FloodMaskMessage::GotMask(x),
                )
            }
            FloodMaskMessage::SetSoftBorder(v) => {
                self.soft_border = v;
                if self.rendering {
                    return Command::none();
                }
                self.rendering = true;
                Command::perform(
                    regenerate_mask(
                        wdata.source.clone(),
                        self.start,
                        self.treshhold,
                        self.soft_border,
                    ),
                    |x| FloodMaskMessage::GotMask(x),
                )
            }
            FloodMaskMessage::GotMask(mask) => {
                self.mask = Some(mask);
                self.picking_pixel = false;
                self.rendering = false;
                self.dirty = true;
                Command::none()
            }
        }
    }

    fn properties_view(
        &'a self,
        _pdata: &'a crate::data::ProgramData,
        _wdata: &'a crate::data::WorkspaceData,
    ) -> Option<iced::Element<'a, Self::Message, iced::Renderer>> {
        let butt = if self.picking_pixel {
            button("Cancel picking")
                .on_press(FloodMaskMessage::StopPicking)
                .style(Style::Highlight.into())
        } else {
            button("Pick pixel to mask").on_press(FloodMaskMessage::StartPicking)
        };
        let label_threshold = text("Threshold: ").width(Length::Fill);
        let label_edge = text("Soft Edge: ").width(Length::Fill);

        let label_threshold = tooltip(
            label_threshold,
            "Determines how close the color has to be to the selected color to count as part of the mask.",
            tooltip::Position::Bottom
        )
        .style(Style::Frame);

        let label_edge = tooltip(
            label_edge,
            "Allows the mask to extend further than the threshold, softening up the mask edges.",
            tooltip::Position::Bottom,
        )
        .style(Style::Frame);

        let slider_threshold = slider(0.0..=1.0, self.treshhold, |x| {
            FloodMaskMessage::SetTolerance(x)
        })
        .step(0.001)
        .width(Length::FillPortion(4));

        let slider_edge = slider(0.0..=1.0, self.soft_border, |x| {
            FloodMaskMessage::SetSoftBorder(x)
        })
        .step(0.001)
        .width(Length::FillPortion(4));

        let ui = col![
            butt,
            row![
                label_threshold,
                slider_threshold,
                horizontal_space(Length::FillPortion(2))
            ]
            .spacing(4),
            row![
                label_edge,
                slider_edge,
                horizontal_space(Length::FillPortion(2))
            ]
            .spacing(4),
        ]
        .spacing(6);

        Some(ui.into())
    }

    fn main_view(
        &'a self,
        _pdata: &'a crate::data::ProgramData,
        wdata: &'a crate::data::WorkspaceData,
    ) -> iced::Element<'a, Self::Message, iced::Renderer> {
        PixelSampler::new(wdata.source_preview.clone(), |x| {
            FloodMaskMessage::Picked(Point {
                x: x.x as f32,
                y: x.y as f32,
            })
        })
        .into()
    }

    fn wants_main_view(
        &self,
        _pdata: &crate::data::ProgramData,
        _wdata: &crate::data::WorkspaceData,
    ) -> bool {
        self.picking_pixel
    }

    fn get_image_operation(
        &self,
        _pdata: &crate::data::ProgramData,
        wdata: &crate::data::WorkspaceData,
    ) -> super::ModifierOperation {
        if let Some(mask) = &self.mask {
            ImageOperation::MaskWithOffset {
                mask: mask.clone(),
                center: Point {
                    x: wdata.source.width() as f32 * 0.5 - wdata.offset.x,
                    y: wdata.source.height() as f32 * 0.5 - wdata.offset.y,
                },
                size: wdata.zoom,
            }
            .into()
        } else {
            ModifierOperation::None
        }
    }

    fn create(
        _pdata: &crate::data::ProgramData,
        _wdata: &crate::data::WorkspaceData,
    ) -> (iced::Command<Self::Message>, Self) {
        (
            Command::none(),
            Self {
                mask: None,
                treshhold: 0.1,
                soft_border: 0.1,
                start: Point::ORIGIN,
                dirty: false,
                rendering: false,
                picking_pixel: true,
            },
        )
    }

    fn label() -> &'static str {
        "Flood Mask"
    }

    fn tooltip() -> &'static str {
        "Hides parts of the image spreading from selected point through similar colors"
    }

    fn is_dirty(&self) -> bool {
        self.dirty
    }

    fn set_clean(&mut self) {
        self.dirty = false;
    }
}

async fn regenerate_mask(
    image: Arc<RgbaImage>,
    starting: Point,
    tolerance: f32,
    soft_border: f32,
) -> Arc<GrayscaleImage> {
    let start = Vector {
        x: starting.x as u32,
        y: starting.y as u32,
    };
    let range = tolerance.min(1.0).max(0.0).powi(2);
    let soft_border = soft_border.min(1.0).max(0.0).powi(2);
    let soft_border_range = range + soft_border;
    let color = pixel_to_color(image.get_pixel(start.x, start.y));

    let mask = flood_fill_mask(image.as_ref(), start, 255, |p| {
        let (r, g, b) = (
            p[0] as f32 / 255.0,
            p[1] as f32 / 255.0,
            p[2] as f32 / 255.0,
        );

        let r = (r - color.r).abs().powi(2);
        let g = (g - color.g).abs().powi(2);
        let b = (b - color.b).abs().powi(2);
        let len = r + g + b;

        if len < range {
            Some(0)
        } else if len < soft_border_range {
            let comb = len - range;
            let shade = comb / soft_border;
            let shade = (shade * 255.0) as u8;
            Some(shade.min(254))
        } else {
            None
        }
    });

    Arc::new(mask)
}
