use std::path::Path;

use iced::widget::{column as col, row, slider, text};
use iced::{Command, Length, Point};
use image::{imageops::resize, DynamicImage};

use crate::data::OutputOptions;

use super::Modifier;

#[derive(Debug)]
pub struct Frame {
    image: Option<DynamicImage>,
    offset: Point,
    scale: Point,
}

impl Modifier for Frame {
    type Message = FrameMessage;

    fn modify(&self, image: DynamicImage, options: &OutputOptions) -> DynamicImage {
        // will copy the frame over the image for now just to see if this works
        if let Some(frame) = &self.image {
            let mut img = image.into_rgba8();
            let img_width = (img.width() as f32 * self.scale.x) as u32;
            let img_height = (img.height() as f32 * self.scale.y) as u32;
            img = resize(
                &img,
                img_width,
                img_height,
                image::imageops::FilterType::Nearest,
            );
            let frame = resize(
                frame,
                options.size.x,
                options.size.y,
                image::imageops::FilterType::Nearest,
            );
            let offset_x = (self.offset.x * img_width as f32) as u32;
            let offset_y = (self.offset.y * img_height as f32) as u32;

            for x in 0..frame.width() {
                for y in 0..frame.height() {
                    let tx = x + offset_x;
                    let ty = y + offset_y;
                    if img.width() <= tx || img.height() <= ty {
                        continue;
                    }
                    let pixel = frame.get_pixel(x, y);
                    let target = img.get_pixel(tx, ty);
                    let pa = pixel.0[3] as f32 / u8::MAX as f32;
                    let res = [
                        (pixel.0[0] as f32 * pa + target.0[0] as f32 * (1.0 - pa)) as u8,
                        (pixel.0[1] as f32 * pa + target.0[1] as f32 * (1.0 - pa)) as u8,
                        (pixel.0[2] as f32 * pa + target.0[2] as f32 * (1.0 - pa)) as u8,
                        target.0[3].max(pixel.0[3]),
                    ];
                    img.put_pixel(tx, ty, res.into());
                }
            }
            img.into()
        } else {
            // if an image haven't been set for the frame then it will pass through
            image
        }
    }

    fn label(&self) -> &str {
        "Frame"
    }
    fn properties_view(&self) -> Option<iced::Element<Self::Message, iced::Renderer>> {
        let v = col![
            row![
                text("Location X: ").width(Length::Shrink),
                slider(0.0..=1.0, self.offset.x, |x| FrameMessage::MoveX(x))
                    .width(Length::Fill)
                    .step(0.1),
            ],
            row![
                text("Location Y: ").width(Length::Shrink),
                slider(0.0..=1.0, self.offset.y, |x| FrameMessage::MoveY(x))
                    .width(Length::Fill)
                    .step(0.1),
            ],
            row![
                text("Scale: ").width(Length::Shrink),
                slider(0.1..=2.0, self.scale.x, |x| FrameMessage::Scale(x))
                    .width(Length::Fill)
                    .step(0.1),
            ],
        ];
        Some(v.into())
    }
    fn properties_update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            FrameMessage::MoveX(x) => self.offset.x = x,
            FrameMessage::MoveY(x) => self.offset.y = x,
            FrameMessage::Scale(x) => self.scale = Point { x, y: x },
        }
        Command::none()
    }
}

#[derive(Debug, Clone)]
pub enum FrameMessage {
    MoveX(f32),
    MoveY(f32),
    Scale(f32),
}

impl Default for Frame {
    fn default() -> Self {
        Self {
            image: None,
            offset: Point::new(0.0, 0.0),
            scale: Point::new(1.0, 1.0),
        }
    }
}

impl Frame {
    pub fn load<T: AsRef<Path>>(path: T) -> Result<Self, image::ImageError> {
        let image = image::open(path)?;
        Ok(Self {
            image: Some(image),
            ..Default::default()
        })
    }
}
