use std::{fs::read_dir, path::PathBuf, sync::Arc};

use iced::widget::{column as col, horizontal_space, radio, row, text, vertical_space};
use iced::{Alignment, Command, Element, Length, Point, Renderer, Size};
use iced_native::image::Handle;

use crate::{
    file_browser::Browser,
    image::{image_to_handle, GrayscaleImage, ImageFormat, RgbaImage},
    style::Theme,
    workspace::WorkspaceTemplate,
};

/// Data and tools available in the program
#[derive(Default)]
pub struct ProgramData {
    /// File browser, used for allowing the user ease of access to the file system
    pub file: Browser,
    /// Intended export path, meant to be combined with individual names from workspaces
    pub output: PathBuf,
    /// Collection of frames loaded into the program
    pub available_frames: Vec<FrameImage>,
    /// Currently used color scheme for the UI
    pub theme: Theme,
}

/// Messages for customizing the program settings
#[derive(Debug, Clone)]
pub enum ProgramDataMessage {
    /// Sets a new theme
    SetTheme(Theme),
}

impl ProgramData {
    /// Draws UI for customizing program settings
    pub fn view(&self) -> Element<ProgramDataMessage, Renderer> {
        col![
            vertical_space(Length::Fill),
            row![
                horizontal_space(Length::Fill),
                text("Theme: "),
                radio("Light", Theme::Light, Some(self.theme.clone()), |x| {
                    ProgramDataMessage::SetTheme(x)
                }),
                radio("Dark", Theme::Dark, Some(self.theme.clone()), |x| {
                    ProgramDataMessage::SetTheme(x)
                }),
                horizontal_space(Length::Fill),
            ]
            .spacing(4)
            .width(Length::Fill)
            .align_items(Alignment::Center),
            vertical_space(Length::Fill),
        ]
        .align_items(Alignment::Center)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    /// Updates settings according to the message
    pub fn update(&mut self, message: ProgramDataMessage) -> Command<ProgramDataMessage> {
        match message {
            ProgramDataMessage::SetTheme(t) => {
                self.theme = t;
                Command::none()
            }
        }
    }
}

pub struct WorkspaceData {
    /// Size of the render
    pub export_size: Size<u32>,
    /// Size of the preview widget
    pub view: f32,
    /// Name of the file to export the result to
    pub output: String,
    pub format: ImageFormat,

    /// Flag used to signal to the workspace and its modifiers what is the intended output to better adjust default values
    pub template: WorkspaceTemplate,
    /// Offset applied to the source image for rendering
    pub offset: Point,
    /// Zoom applied to the source image for rendering
    pub zoom: f32,
    /// Denotes whatever the workspace needs to be rerendered
    pub dirty: bool,
}

impl Default for WorkspaceData {
    fn default() -> Self {
        Self {
            export_size: Size {
                width: 512,
                height: 512,
            },
            view: 1.0,
            output: Default::default(),
            offset: Default::default(),
            zoom: 1.0,
            dirty: Default::default(),
            format: ImageFormat::WebP,
            template: WorkspaceTemplate::None,
        }
    }
}

/// Holds images prepared to be used as frames for tokens
#[derive(Debug, Clone)]
pub struct FrameImage {
    /// iced native image format, used for rendering
    pub display: Handle,
    /// Image ready for use in rendering process
    pub frame: Arc<RgbaImage>,
    /// Optional mask for the frame
    pub mask: Option<Arc<GrayscaleImage>>,
}

/// Function crawls through frames folder and gathers all images for frames and their masks
pub async fn load_frames() -> std::io::Result<Vec<FrameImage>> {
    let location = PathBuf::from("./data/frames/");
    let dir = read_dir(location)?;
    let mut res = vec![];

    for d in dir {
        // Skip any entries that failed to load
        let Ok(d) = d else {
            continue;
        };
        // We're only interested in files
        let mut path = d.path();
        if path.is_file() == false {
            continue;
        }
        // Skipping mask images since we're loading them together with their real image
        let Some(name) = path.file_stem().and_then(|n| n.to_str()).and_then(|n| Some(n.to_string())) else {
            continue;
        };
        if name.contains("-mask") {
            continue;
        }
        // We let the image crate handle whatever the file is valid image or not
        let Ok(img) = image::open(&path) else {
            continue;
        };
        let img = img.into_rgba8();
        let display = image_to_handle(img.clone());

        // loading the mask here, then adding it to the final result if it succeeds
        if let Some(ext) = path.extension().and_then(|x| x.to_str()) {
            path.set_file_name(format!("{}-mask.{}", name, ext));
        } else {
            path.set_file_name(format!("{}-mask", name));
        }

        if let Ok(mask) = image::open(path) {
            res.push(FrameImage {
                display,
                frame: Arc::new(img),
                mask: Some(Arc::new(mask.into_luma8())),
            });
        } else {
            res.push(FrameImage {
                display,
                frame: Arc::new(img),
                mask: None,
            });
        }
    }
    Ok(res)
}
