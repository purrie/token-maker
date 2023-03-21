use std::{path::PathBuf, sync::Arc};

use iced::{
    widget::{button, column as col, radio, row},
    Color, Command, Point, Size,
};
use iced_native::image::Handle;

use crate::{
    image::{
        convert::image_arc_to_handle, download_image, image_filter, operations::resample_image,
        ImageOperation, RgbaImage,
    },
    widgets::{BrowserOperation, BrowsingResult, ColorPicker, Trackpad},
};

use super::{Modifier, ModifierOperation};

#[derive(Debug, Clone)]
pub struct Background {
    background: BackgroundType,
    color: Color,
    source: Option<Arc<RgbaImage>>,
    image: Option<Arc<RgbaImage>>,
    preview: Option<Handle>,

    dirty: bool,
    rendering: bool,
    browsing: bool,
    repositioning: bool,
    offset: Point,
    zoom: f32,
}

#[derive(Debug, Clone)]
pub enum BackgroundMessage {
    SetColor(Color),
    SetMode(BackgroundType),
    SetOffset(Point),
    SetZoom(f32),
    SetImage(Result<(Arc<RgbaImage>, Handle), PathBuf>),
    UpdateImage(Arc<RgbaImage>, Handle),
    LookForImage,
    LookForUrl,
    DownloadImage(String),
    DownloadedImage(Result<RgbaImage, String>),
    RepositionImage,
    Browser(BrowserOperation),
}

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum BackgroundType {
    Image,
    Solid,
    // TODO add gradients
}

impl<'a> Modifier<'a> for Background {
    type Message = BackgroundMessage;

    fn get_image_operation(
        &self,
        _pdata: &crate::data::ProgramData,
        _wdata: &crate::data::WorkspaceData,
    ) -> ModifierOperation {
        match &self.background {
            BackgroundType::Image if self.image.is_some() => {
                ImageOperation::BackgroundImage(self.image.clone().unwrap()).into()
            }
            BackgroundType::Solid => ImageOperation::BackgroundColor(self.color).into(),
            _ => ModifierOperation::None,
        }
    }

    fn create(
        _pdata: &crate::data::ProgramData,
        _wdata: &crate::data::WorkspaceData,
    ) -> (iced::Command<Self::Message>, Self) {
        let s = Self {
            background: BackgroundType::Solid,
            dirty: true,
            color: Color::WHITE,
            source: None,
            image: None,
            preview: None,
            browsing: false,
            repositioning: false,
            rendering: false,
            offset: Point::ORIGIN,
            zoom: 1.0,
        };
        (Command::none(), s)
    }

    fn properties_update(
        &mut self,
        message: Self::Message,
        pdata: &mut crate::data::ProgramData,
        wdata: &mut crate::data::WorkspaceData,
    ) -> Command<Self::Message> {
        match message {
            BackgroundMessage::SetColor(color) => {
                self.color = color;
                self.dirty = true;
                Command::none()
            }
            BackgroundMessage::SetMode(mode) => {
                self.background = mode;
                self.dirty = true;
                Command::none()
            }
            BackgroundMessage::Browser(op) => match pdata.file.update(op) {
                Ok(o) => match o {
                    BrowsingResult::Pending => Command::none(),
                    BrowsingResult::Canceled => {
                        self.browsing = false;
                        Command::none()
                    }
                    BrowsingResult::Done(path) => {
                        self.browsing = false;
                        pdata.status.log(&format!("loading background: {:?}", path));
                        Command::perform(
                            load_image(path, wdata.export_size),
                            BackgroundMessage::SetImage,
                        )
                    }
                },
                Err(e) => {
                    pdata.status.error(&format!("Error: {}", e));
                    self.browsing = false;
                    Command::none()
                }
            },
            BackgroundMessage::SetImage(Ok((img, rendr))) => {
                self.source = Some(img.clone());
                self.image = Some(img);
                self.preview = Some(rendr);
                self.dirty = true;
                Command::none()
            }
            BackgroundMessage::SetImage(Err(path)) => {
                pdata.status.error(&format!(
                    "Error: Path {:?} doesn't point to a valid image.",
                    path
                ));
                Command::none()
            }
            BackgroundMessage::UpdateImage(image, preview) => {
                self.image = Some(image);
                self.preview = Some(preview);
                self.rendering = false;
                self.dirty = true;
                Command::none()
            }
            BackgroundMessage::SetOffset(o) => {
                self.offset = Point {
                    x: self.offset.x + o.x,
                    y: self.offset.y + o.y,
                };

                if self.rendering {
                    return Command::none();
                }
                self.rendering = true;

                Command::perform(
                    resize_image(
                        self.source.as_ref().unwrap().clone(),
                        self.offset,
                        self.zoom,
                        wdata.export_size,
                    ),
                    |x| BackgroundMessage::UpdateImage(x.0, x.1),
                )
            }
            BackgroundMessage::SetZoom(z) => {
                self.zoom -= z;

                if self.rendering {
                    return Command::none();
                }
                self.rendering = true;

                Command::perform(
                    resize_image(
                        self.source.as_ref().unwrap().clone(),
                        self.offset,
                        self.zoom,
                        wdata.export_size,
                    ),
                    |x| BackgroundMessage::UpdateImage(x.0, x.1),
                )
            }
            BackgroundMessage::RepositionImage => {
                if self.source.is_some() {
                    self.repositioning = !self.repositioning;
                    if self.repositioning {
                        self.browsing = false;
                    }
                }
                Command::none()
            }
            BackgroundMessage::LookForImage => {
                self.browsing = true;
                pdata.file.set_filter(image_filter);
                Command::none()
            }
            BackgroundMessage::LookForUrl => iced::clipboard::read(|x| {
                let Some(url) = x else {
                    return BackgroundMessage::DownloadedImage(Err("Clipboard is empty".to_string()));
                };
                BackgroundMessage::DownloadImage(url)
            }),
            BackgroundMessage::DownloadImage(url) => {
                pdata.status.log("Downloading image...");
                Command::perform(
                    async move {
                        let img = download_image(url).await;
                        BackgroundMessage::DownloadedImage(img)
                    },
                    |x| x,
                )
            }
            BackgroundMessage::DownloadedImage(img) => match img {
                Ok(img) => {
                    pdata.status.log("Image downloaded");
                    let img = Arc::new(img);
                    self.source = Some(img.clone());
                    let offset = self.offset;
                    let zoom = self.zoom;
                    let size = wdata.export_size;
                    Command::perform(
                        async move {
                            let img = resize_image(img, offset, zoom, size).await;
                            BackgroundMessage::UpdateImage(img.0, img.1)
                        },
                        |x| x,
                    )
                }
                Err(er) => {
                    pdata.status.error(&er);
                    Command::none()
                }
            },
        }
    }

    fn workspace_update(
        &mut self,
        _pdata: &crate::data::ProgramData,
        wdata: &crate::data::WorkspaceData,
    ) -> Command<Self::Message> {
        if let Some(img) = &self.image {
            if wdata.export_size.width != img.width() || wdata.export_size.height != img.height() {
                let img = self.source.clone().unwrap();
                let offset = self.offset;
                let zoom = self.zoom;
                let size = wdata.export_size;
                return Command::perform(
                    async move {
                        let img = resize_image(img, offset, zoom, size).await;
                        BackgroundMessage::UpdateImage(img.0, img.1)
                    },
                    |x| x,
                );
            }
        }
        Command::none()
    }

    fn properties_view(
        &self,
        _pdata: &crate::data::ProgramData,
        _wdata: &crate::data::WorkspaceData,
    ) -> Option<iced::Element<Self::Message, iced::Renderer>> {
        let modes = col![
            radio("Color", BackgroundType::Solid, Some(self.background), |x| {
                BackgroundMessage::SetMode(x)
            }),
            radio("Image", BackgroundType::Image, Some(self.background), |x| {
                BackgroundMessage::SetMode(x)
            }),
        ]
        .spacing(4);
        let ui = match &self.background {
            BackgroundType::Image => {
                let file = button("Choose Image").on_press(BackgroundMessage::LookForImage);
                let down = button("Paste URL").on_press(BackgroundMessage::LookForUrl);
                let transform = if self.image.is_some() {
                    let label = if self.repositioning {
                        "Stop Repositioning"
                    } else {
                        "Reposition Image"
                    };
                    button(label).on_press(BackgroundMessage::RepositionImage)
                } else {
                    button("Reposition Image")
                };
                col![file, down, transform].spacing(4)
            }
            BackgroundType::Solid => {
                let col = ColorPicker::new(self.color, |x| BackgroundMessage::SetColor(x))
                    .width(32)
                    .height(32);
                col![col]
            }
        };

        let ui = row![modes, ui].spacing(4);
        Some(ui.into())
    }

    fn main_view(
        &'a self,
        _image: iced_native::image::Handle,
        pdata: &'a crate::data::ProgramData,
        _wdata: &'a crate::data::WorkspaceData,
    ) -> iced::Element<Self::Message, iced::Renderer> {
        if self.browsing {
            return pdata.file.view().map(|x| BackgroundMessage::Browser(x));
        }

        if self.repositioning {
            let tr = Trackpad::new(self.preview.as_ref().unwrap().clone())
                .with_drag(self.offset, |mods, _button, _point, delta| {
                    let offset = if mods.shift() {
                        Point {
                            x: delta.x * 0.1,
                            y: delta.y * 0.1,
                        }
                    } else {
                        Point {
                            x: delta.x,
                            y: delta.y,
                        }
                    };
                    Some(BackgroundMessage::SetOffset(offset))
                })
                .with_scroll(|mods, scroll| match scroll {
                    iced::mouse::ScrollDelta::Lines { x: _, y }
                    | iced::mouse::ScrollDelta::Pixels { x: _, y } => {
                        let y = if mods.alt() { y * 0.01 } else { y * 0.1 };
                        Some(BackgroundMessage::SetZoom(y))
                    }
                });
            return tr.into();
        }

        unreachable!()
    }

    fn wants_main_view(
        &self,
        _pdata: &crate::data::ProgramData,
        _wdata: &crate::data::WorkspaceData,
    ) -> bool {
        self.browsing || self.repositioning
    }

    fn label() -> &'static str {
        "Background"
    }

    fn is_dirty(&self) -> bool {
        self.dirty
    }

    fn set_clean(&mut self) {
        self.dirty = false;
    }
}

async fn load_image(path: PathBuf, size: Size<u32>) -> Result<(Arc<RgbaImage>, Handle), PathBuf> {
    let Ok(img) = image::open(&path) else {
        return Err(path);
    };
    let (img, preview) = resize_image(Arc::new(img.into_rgba8()), Point::ORIGIN, 1.0, size).await;
    Ok((img, preview))
}

async fn resize_image(
    source: Arc<RgbaImage>,
    offset: Point,
    zoom: f32,
    size: Size<u32>,
) -> (Arc<RgbaImage>, Handle) {
    let center = Point {
        x: source.width() as f32 * 0.5 - offset.x,
        y: source.height() as f32 * 0.5 - offset.y,
    };
    let img = resample_image(source, size, center, zoom).await;

    let img = Arc::new(img);
    let preview = image_arc_to_handle(&img);
    (img, preview)
}
