use std::path::Path;

use iced::widget::{column as col, row, slider, text};
use iced::{Command, Length, Point};
use image::{imageops::resize, DynamicImage};
use image::{ImageBuffer, Pixel, Rgba};

use super::Modifier;

#[derive(Clone, Debug)]
pub struct Frame {
    image: Option<DynamicImage>,
    offset: Point,
    scale: Point,
    cached: Option<ImageBuffer<Rgba<u8>, Vec<u8>>>,
    export_size: u8,
}

impl Modifier for Frame {
    type Message = FrameMessage;

    fn modify(&self, image: DynamicImage) -> DynamicImage {
        // will copy the frame over the image for now just to see if this works
        let Some(frame) = &self.cached else {
            return image;
        };

        let source = image.into_rgba8();
        let mut out = frame.clone();
        let aspect_calc = source.width().max(source.height()) as f32;
        // swapped because we want longer side cropped
        let aspect_y = source.width() as f32 / aspect_calc;
        let aspect_x = source.height() as f32 / aspect_calc;

        let ow = out.width();
        let oh = out.height();

        for x in 0..ow {
            for y in 0..oh {
                let pixel = out.get_pixel_mut(x, y);
                if pixel[3] < u8::MAX {
                    let tx = {
                        // percent location of the pixel in range 0..1
                        let mut self_percent = x as f32 / ow as f32 * aspect_x;
                        let zoom = self.scale.x;
                        if zoom > 1.001 {
                            let scale = self_percent / zoom;
                            let scaled_offset = self.offset.x - self.offset.x / zoom;
                            self_percent = scale + scaled_offset;
                        }
                        let source_scaled = source.width() as f32 * self_percent;
                        source_scaled as u32
                    };
                    let ty = {
                        // percent location of the pixel in range 0..1
                        let mut self_percent = y as f32 / oh as f32 * aspect_y;
                        // scale uses only x for now
                        let zoom = self.scale.x;
                        if zoom > 1.001 {
                            let scale = self_percent / zoom;
                            let scaled_offset = self.offset.y - self.offset.y / zoom;
                            self_percent = scale + scaled_offset;
                        }
                        let source_scaled = source.height() as f32 * self_percent;
                        source_scaled as u32
                    };

                    let mut s = source.get_pixel(tx, ty).clone();
                    s.blend(pixel);
                    *pixel = s;
                }
            }
        }
        out.into()
    }

    fn label(&self) -> &str {
        "Frame"
    }
    fn properties_view(&self) -> Option<iced::Element<Self::Message, iced::Renderer>> {
        let v = row![
            col![
                text("Location X: "),
                text("Location Y: "),
                text("Zoom: "),
                text("Export Size: "),
            ]
            .width(Length::Shrink),
            col![
                slider(0.0..=1.0, self.offset.x, |x| FrameMessage::MoveX(x)).step(0.01),
                slider(0.0..=1.0, self.offset.y, |x| FrameMessage::MoveY(x)).step(0.01),
                slider(1.0..=20.0, self.scale.x, |x| FrameMessage::Scale(x)).step(0.01),
                slider(6..=10, self.export_size, |x| FrameMessage::Size(x)).width(Length::Fill),
            ]
            .width(Length::Fill),
            col![
                text(&format!("{0:.01}", self.offset.x)),
                text(&format!("{0:.01}", self.offset.y)),
                text(&format!("{0:.01}", self.scale.x)),
                text(&format!("{}px", u32::pow(2, self.export_size.into()))),
            ]
            .width(Length::Shrink),
        ];
        Some(v.into())
    }
    fn properties_update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            FrameMessage::MoveX(x) => self.offset.x = x,
            FrameMessage::MoveY(x) => self.offset.y = x,
            FrameMessage::Scale(x) => self.scale = Point { x, y: x },
            FrameMessage::Size(x) => {
                if x == self.export_size {
                    return Command::none();
                }
                self.export_size = x;
                self.cache_image();
            }
        }
        Command::none()
    }
}

#[derive(Debug, Clone)]
pub enum FrameMessage {
    MoveX(f32),
    MoveY(f32),
    Scale(f32),
    Size(u8),
}

impl Default for Frame {
    fn default() -> Self {
        Self {
            image: None,
            offset: Point::new(0.5, 0.1),
            scale: Point::new(1.0, 1.0),
            cached: None,
            export_size: 8,
        }
    }
}

impl Frame {
    pub fn load<T: AsRef<Path>>(path: T) -> Result<Self, image::ImageError> {
        let image = image::open(path)?;
        let mut s = Self {
            image: Some(image),
            ..Default::default()
        };
        s.cache_image();
        Ok(s)
    }
    fn cache_image(&mut self) {
        let size = u32::pow(2, self.export_size.into());

        let Some(frame) = &self.image else {
            panic!("Frame doesn't have image!");
        };

        let img = resize(frame, size, size, image::imageops::FilterType::Nearest);
        self.cached = Some(img);
    }
}
