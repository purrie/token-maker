use std::future::Future;
use std::path::Path;
use std::sync::Arc;

use iced::widget::image::Handle;
use iced::widget::{column as col, row, slider, text, text_input};
use iced::{Command, Length, Point};
use image::{imageops::resize, DynamicImage};
use image::{GenericImageView, Rgba};

use crate::image::{blend_images, image_to_handle, resample_image, RgbaImage};
use crate::math::Vec2u;

#[derive(Debug, Clone)]
pub enum FrameMessage {
    /// Changing offset for the source image
    MoveX(f32),
    /// Changing offset for the source image
    MoveY(f32),
    /// Changing zoom on the source image
    Zoom(f32),
    /// Changing size of the final image
    SizeX(u32),
    /// Changing size of the final image
    SizeY(u32),
    /// Result of recomputing the frame overlay
    Frame(RgbaImage),
}

#[derive(Clone, Debug)]
pub struct Frame {
    /// Frame image to be put onto the source image
    image: Option<Arc<DynamicImage>>,
    /// Cached image of the frame, already prepared for overlaying
    cached_image: Option<Arc<RgbaImage>>,
    offset: Point,
    zoom: f32,
    export_size: Vec2u,
    dirty: bool,
}

impl Frame {
    /// The function loads an image from the `path`
    ///
    /// TODO change the function to load a metadata file that would contain information about the image, mask and any other that I will need in the future.
    pub fn load<T: AsRef<Path>>(path: T) -> Result<Self, image::ImageError> {
        let image = image::open(path)?;
        let image = Arc::new(image);
        let mut s = Self {
            export_size: Vec2u {
                x: image.width(),
                y: image.height(),
            },
            image: Some(image),
            ..Default::default()
        };
        s.cache_image();
        Ok(s)
    }
    /// Caches the image, resizing it to a desired export size. The function also processes the mask if it is present
    fn cache_image(&mut self) {
        let Some(frame) = &self.image else {
            panic!("Frame doesn't have image!");
        };

        let img = resize(
            frame.as_ref(),
            self.export_size.x,
            self.export_size.y,
            image::imageops::FilterType::Nearest,
        );
        let img = Arc::new(img);
        self.cached_image = Some(img);
    }
    /// Creates a future which will produce a base image to which rest of the modifier stack can apply its effects
    pub fn prepare_image(&self, source: Arc<RgbaImage>) -> impl Future<Output = RgbaImage> {
        resize_frame(source, self.offset, self.zoom, self.export_size)
    }
    /// Creates a future that will apply the frame image to result of the source future
    pub fn finalize_image(
        &self,
        source: impl Future<Output = RgbaImage>,
    ) -> impl Future<Output = Handle> {
        apply_frame(source, self.cached_image.clone())
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }
    pub fn clean(&mut self) {
        self.dirty = false;
    }
    pub fn expected_size(&self) -> Vec2u {
        self.export_size
    }

    /// UI for the frame
    pub fn properties_view(&self) -> iced::Element<FrameMessage, iced::Renderer> {
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
                slider(0.1..=10.0, self.zoom, |x| FrameMessage::Zoom(x)).step(0.01),
                row![
                    text_input("", &self.export_size.x.to_string(), |x| {
                        if let Ok(x) = x.parse() {
                            FrameMessage::SizeX(x)
                        } else {
                            FrameMessage::SizeX(self.export_size.x)
                        }
                    }),
                    text("x"),
                    text_input("", &self.export_size.y.to_string(), |y| {
                        if let Ok(y) = y.parse() {
                            FrameMessage::SizeY(y)
                        } else {
                            FrameMessage::SizeY(self.export_size.y)
                        }
                    }),
                ]
            ]
            .width(Length::Fill),
            col![
                text(&format!("{0:.2}", self.offset.x)),
                text(&format!("{0:.2}", self.offset.y)),
                if self.zoom >= 10.0 {
                    text(&format!("{0:.1}", self.zoom))
                } else {
                    text(&format!("{0:.2}", self.zoom))
                },
                text("px"),
            ]
            .width(Length::Shrink),
        ];
        v.into()
    }
    /// Handling of the UI messages
    pub fn properties_update(&mut self, message: FrameMessage) -> Command<FrameMessage> {
        match message {
            FrameMessage::MoveX(x) => {
                self.offset.x = x;
                self.dirty = true;
                Command::none()
            }
            FrameMessage::MoveY(x) => {
                self.offset.y = x;
                self.dirty = true;
                Command::none()
            }
            FrameMessage::Zoom(x) => {
                self.zoom = x;
                self.dirty = true;
                Command::none()
            }
            FrameMessage::SizeX(x) => {
                self.export_size.x = x;
                if let Some(image) = &self.image {
                    Command::perform(
                        resize_frame(image.clone(), Point::default(), 1.0, self.export_size),
                        FrameMessage::Frame,
                    )
                } else {
                    // TODO probably worth handling the lack of image
                    Command::none()
                }
            }
            FrameMessage::SizeY(y) => {
                self.export_size.y = y;
                if let Some(image) = &self.image {
                    Command::perform(
                        resize_frame(image.clone(), Point::default(), 1.0, self.export_size),
                        FrameMessage::Frame,
                    )
                } else {
                    // TODO probably worth handling the lack of image
                    Command::none()
                }
            }
            FrameMessage::Frame(x) => {
                let x = Arc::new(x);
                self.cached_image = Some(x);
                self.dirty = true;
                Command::none()
            }
        }
    }
}
impl Default for Frame {
    fn default() -> Self {
        Self {
            image: None,
            cached_image: None,
            offset: Point::new(0.5, 0.1),
            zoom: 1.0,
            export_size: Vec2u { x: 512, y: 512 },
            dirty: true,
        }
    }
}

async fn resize_frame<T: GenericImageView<Pixel = Rgba<u8>>>(
    image: Arc<T>,
    offset: Point,
    zoom: f32,
    size: Vec2u,
) -> RgbaImage {
    resample_image(image.as_ref(), &size, &offset, zoom)
}

async fn apply_frame(
    img: impl Future<Output = RgbaImage>,
    frame: Option<Arc<RgbaImage>>,
) -> Handle {
    let mut img = img.await;
    if let Some(x) = frame {
        blend_images(&mut img, x.as_ref());
    }
    image_to_handle(&img)
}
