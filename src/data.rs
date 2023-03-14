use std::fs::create_dir_all;
use std::{fs::read_dir, path::PathBuf, sync::Arc};

use iced::widget::{column as col, horizontal_space, radio, row, text, text_input, vertical_space};
use iced::{Alignment, Command, Element, Length, Point, Renderer, Size};
use iced_native::image::Handle;

use crate::naming_convention::NamingConvention;
use crate::persistence::{Persistence, PersistentKey, PersistentValue};
use crate::status_bar::StatusBar;
use crate::style::Layout;
use crate::{
    image::{image_to_handle, GrayscaleImage, ImageFormat, RgbaImage},
    style::Theme,
    widgets::Browser,
    workspace::WorkspaceTemplate,
};

/// Data and tools available in the program
pub struct ProgramData {
    /// File browser, used for allowing the user ease of access to the file system
    pub file: Browser,
    /// Status line for giving feedback to the user
    pub status: StatusBar,
    /// Intended export path, meant to be combined with individual names from workspaces
    pub output: PathBuf,
    /// Collection of frames loaded into the program
    pub available_frames: Vec<FrameImage>,
    /// Currently used color scheme for the UI
    pub theme: Theme,
    /// Determines which layout the workspaces should be displayed with
    pub layout: Layout,
    /// Naming conventions to use in the program
    pub naming: NamingConvention,
    /// Values saved across sessions
    pub cache: Persistence,
}

/// Messages for customizing the program settings
#[derive(Debug, Clone)]
pub enum ProgramDataMessage {
    /// Sets a new theme
    SetTheme(Theme),
    SetLayout(Layout),
    SetNamingConvention(WorkspaceTemplate, String),
    SetProjectName(String),
}

enum PersistentData {
    SettingsID,
    FileBrowserID,
    Theme,
    Layout,
    Output,
    Folder,
}
impl PersistentKey for PersistentData {
    fn get_id(&self) -> &'static str {
        match self {
            PersistentData::SettingsID => "settings",
            PersistentData::FileBrowserID => "file-browser",
            PersistentData::Theme => "theme",
            PersistentData::Layout => "layout",
            PersistentData::Output => "output",
            PersistentData::Folder => "folder",
        }
    }
}

impl ProgramData {
    pub fn new() -> ProgramData {
        let cache = Persistence::load();
        let file = match cache
            .get(PersistentData::FileBrowserID, PersistentData::Folder)
            .and_then(|x| x.check_string())
        {
            Some(p) => Browser::new(p),
            None => Browser::start_at_home(),
        };
        let theme = match cache.get_copy(PersistentData::SettingsID, PersistentData::Theme) {
            Some(t) => t.to_theme(),
            None => Theme::default(),
        };
        let layout = match cache.get_copy(PersistentData::SettingsID, PersistentData::Layout) {
            Some(l) => l.to_layout(),
            None => Layout::default(),
        };
        let output = match cache.get_copy(PersistentData::SettingsID, PersistentData::Output) {
            Some(o) => o.to_string(),
            None => String::new(),
        }
        .into();
        let naming = NamingConvention::new(&cache);

        Self {
            file,
            output,
            available_frames: Vec::new(),
            status: StatusBar::new(),
            theme,
            layout,
            naming,
            cache,
        }
    }
    /// Draws UI for customizing program settings
    pub fn view(&self) -> Element<ProgramDataMessage, Renderer> {
        col![
            vertical_space(Length::Fill),
            row![
                horizontal_space(Length::Fill),
                text("Theme: "),
                radio("Light", Theme::Light, Some(self.theme), |x| {
                    ProgramDataMessage::SetTheme(x)
                }),
                radio("Dark", Theme::Dark, Some(self.theme), |x| {
                    ProgramDataMessage::SetTheme(x)
                }),
                horizontal_space(Length::Fill),
            ]
            .spacing(4)
            .width(Length::Fill)
            .align_items(Alignment::Center),
            row![
                horizontal_space(Length::Fill),
                text("Workspace Layout: "),
                radio("Parallel", Layout::Parallel, Some(self.layout), |x| {
                    ProgramDataMessage::SetLayout(x)
                }),
                radio("Tabs", Layout::Stacking(0), Some(self.layout), |x| {
                    ProgramDataMessage::SetLayout(x)
                }),
                horizontal_space(Length::Fill),
            ],
            row![
                horizontal_space(Length::Fill),
                text("Naming Convention: "),
                col![
                    vertical_space(10),
                    row![
                        text("Default: ").width(Length::Fill),
                        text_input(
                            "Default Name",
                            self.naming.check(&WorkspaceTemplate::None),
                            |x| ProgramDataMessage::SetNamingConvention(WorkspaceTemplate::None, x)
                        )
                        .width(Length::FillPortion(5)),
                    ]
                    .align_items(Alignment::Center),
                    row![
                        text("Token: ").width(Length::Fill),
                        text_input(
                            "Default Name",
                            self.naming.check(&WorkspaceTemplate::Token),
                            |x| ProgramDataMessage::SetNamingConvention(
                                WorkspaceTemplate::Token,
                                x
                            )
                        )
                        .width(Length::FillPortion(5)),
                    ]
                    .align_items(Alignment::Center),
                    row![
                        text("Portrait: ").width(Length::Fill),
                        text_input(
                            "Default Name",
                            self.naming.check(&WorkspaceTemplate::Portrait),
                            |x| ProgramDataMessage::SetNamingConvention(
                                WorkspaceTemplate::Portrait,
                                x
                            )
                        )
                        .width(Length::FillPortion(5)),
                    ]
                    .align_items(Alignment::Center)
                ]
                .width(Length::FillPortion(2)),
                horizontal_space(Length::Fill)
            ]
            .spacing(5),
            vertical_space(Length::Fill),
        ]
        .align_items(Alignment::Center)
        .spacing(4)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    /// Updates settings according to the message
    pub fn update(&mut self, message: ProgramDataMessage) -> Command<ProgramDataMessage> {
        match message {
            ProgramDataMessage::SetTheme(t) => {
                self.theme = t;
                self.cache.set(
                    PersistentData::SettingsID,
                    PersistentData::Theme,
                    self.theme,
                );
                Command::none()
            }
            ProgramDataMessage::SetLayout(l) => {
                self.layout = l;
                self.cache.set(
                    PersistentData::SettingsID,
                    PersistentData::Layout,
                    self.layout,
                );
                Command::none()
            }
            ProgramDataMessage::SetNamingConvention(template, text) => {
                if has_invalid_characters(&text) {
                    self.status
                        .warning("Removed invalid characters from the name");
                }
                self.naming.set(template, text, &mut self.cache);
                Command::none()
            }
            ProgramDataMessage::SetProjectName(n) => {
                if has_invalid_characters(&n) {
                    self.status
                        .warning("Removed invalid characters from the name");
                }
                self.naming.project_name = sanitize_file_name(n);
                Command::none()
            }
        }
    }
}

impl Drop for ProgramData {
    fn drop(&mut self) {
        // saving cache for browser, we do it here to not pollute the widget's module so it will be easier to extract it in case it's something worth using in another project
        let path = self.file.get_path().to_string_lossy().to_string();
        self.cache.set(
            PersistentData::FileBrowserID,
            PersistentData::Folder,
            PersistentValue::String(path),
        );
        self.cache.set(
            PersistentData::SettingsID,
            PersistentData::Output,
            self.output.clone(),
        );
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

pub const PROJECT_NAME: &str = "token-maker";
pub const PROJECT_DATA_FOLDER: &str = "data";
pub const PROJECT_FRAMES_FOLDER: &str = "frames";

/// This is the primary data path intended for use in saving content to drive
///
/// This leads to the same folder as the executable is on windows and in debug build
/// and to user data path on unix systems in release
macro_rules! save_data_path {
    () => {{
            if cfg!(windows) || cfg!(debug_assertions) {
                // if we're on windows or in debug build then we're expected to use the same directory as the binary is in
                let mut d = std::env::current_dir().unwrap();
                d.push(PROJECT_DATA_FOLDER);
                d
            } else {
                // On unix we grab the path from user data location
                let mut d = dirs::data_local_dir().unwrap();
                d.push(PROJECT_NAME);
                d
            }
    }};
    ($($path:expr), +) => {{
            let mut d = save_data_path!();
            $(
                d.push($path);
            )+
            d
    }}
}
pub(crate) use save_data_path;

/// Path where it is expected to save the frames to
macro_rules! save_frames_path {
    () => {
        save_data_path!(PROJECT_FRAMES_FOLDER)
    };
    ($($path:expr), +) => {{
        save_data_path!(PROJECT_FRAMES_FOLDER, $($path), +)
    }};
}
pub(crate) use save_frames_path;

/// All the paths program can store any data to load from
macro_rules! load_data_path {
    ($($paths:expr),*) => {
        [
            {
                let mut d = std::env::current_dir().unwrap();
                d.push(PROJECT_DATA_FOLDER);
                $(
                    d.push($paths);
                )*
                d
            },
            {
                let mut d = dirs::data_local_dir().unwrap();
                d.push(PROJECT_NAME);
                $(
                    d.push($paths);
                )*
                d
            },
            {
                let mut d = dirs::data_dir().unwrap();
                d.push(PROJECT_NAME);
                $(
                    d.push($paths);
                )*
                d
            }
        ]
    }
}
pub(crate) use load_data_path;

/// All the paths the program searches to load data from
macro_rules! load_frames_path {
    () => {
        load_data_path!(PROJECT_FRAMES_FOLDER)
    };
    ($($path:expr),+) => {
        load_data_path!(PROJECT_FRAMES_FOLDER, $($path)+)
    };
}
pub(crate) use load_frames_path;

/// Removes any character from the string that could be problematic for use in file names.
///
/// The resulting string is all lowercase to prevent weirdness when using the results across different platforms.
///
/// Char `$` is purposefully omitted since it's used for variable names.
/// Workspaces are responsible for removing those from the final file name.
pub fn sanitize_file_name(name: String) -> String {
    name.chars()
        .map(|x| if x.is_whitespace() { '-' } else { x })
        .filter(|x| x.is_alphanumeric() || *x == '-' || *x == '_' || *x == '$')
        .map(|x| x.to_ascii_lowercase())
        .collect()
}

pub fn has_invalid_characters(name: &str) -> bool {
    name.chars().any(|x| {
        x.is_whitespace() == false
            && x != '-'
            && x != '_'
            && x != '$'
            && x != std::path::MAIN_SEPARATOR
    })
}

/// Removes characters problematic for file paths from the string
///
/// Works exactly the same as `sanitize_file_name` but allows path breaks
pub fn sanitize_file_name_allow_path(name: String) -> String {
    name.chars()
        .map(|x| if x.is_whitespace() { '-' } else { x })
        .filter(|x| {
            x.is_alphanumeric() || *x == '-' || *x == '_' || *x == std::path::MAIN_SEPARATOR
        })
        .map(|x| x.to_ascii_lowercase())
        .collect()
}

/// Removes any special characters from beginning and end of the string
pub fn sanitize_file_name_ends(name: &String) -> String {
    name.chars()
        .enumerate()
        .filter(|(i, c)| (*i != 0 && *i != name.len() - 1) || c.is_alphanumeric())
        .map(|(_, x)| x)
        .collect()
}

/// Holds images prepared to be used as frames for tokens
#[derive(Debug, Clone)]
pub struct FrameImage {
    /// Name of the image
    name: String,
    /// name of the folder the frame was placed in
    category: String,
    /// iced native image format, used for rendering
    display: Handle,
    /// Image ready for use in rendering process
    frame: Arc<RgbaImage>,
    /// Optional mask for the frame
    mask: Option<Arc<GrayscaleImage>>,
    /// Identifier used to distinguish the frame from others
    id: String,
}

impl FrameImage {
    /// Creates a new frame image
    /// The function ensures the name and category is correct
    pub fn new(
        name: String,
        category: String,
        frame: RgbaImage,
        mask: Option<GrayscaleImage>,
    ) -> Self {
        let mut name = sanitize_file_name_ends(&name);
        if name.len() == 0 {
            name = "untitled".to_string();
        }
        let category = sanitize_file_name_ends(&category);
        let display = image_to_handle(frame.clone());
        let frame = Arc::new(frame);
        let mask = mask.and_then(|x| Some(Arc::new(x)));
        let id = format!("{}/{}", category, name);
        Self {
            name,
            category,
            display,
            frame,
            mask,
            id,
        }
    }

    /// Identifier used to uniquely identify this frame image from any other
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Clones the image handle
    pub fn preview(&self) -> Handle {
        self.display.clone()
    }

    /// Clones the pointer to the frame image
    pub fn image(&self) -> Arc<RgbaImage> {
        self.frame.clone()
    }

    /// Clones the pointer to the mask image
    pub fn mask(&self) -> Option<Arc<GrayscaleImage>> {
        self.mask.clone()
    }

    /// Saves the frame using its name for path location
    pub fn save_frame(&self) {
        let mut location = save_frames_path!(&self.category);
        if location.exists() == false {
            create_dir_all(&location).unwrap();
        }
        location.push(format!("{}.webp", &self.name));

        image::save_buffer(
            &location,
            &self.frame,
            self.frame.width(),
            self.frame.height(),
            image::ColorType::Rgba8,
        )
        .unwrap();

        location.set_file_name(format!("{}-mask.webp", &self.name));
        let mask = self.mask.as_ref().unwrap();
        let pix = mask.as_raw();
        let width = mask.width() as usize;
        let mask = RgbaImage::from_fn(mask.width(), mask.height(), |x, y| {
            let i = width * y as usize + x as usize;
            let pix = pix[i];
            [pix, pix, pix, pix].into()
        });

        image::save_buffer(
            location,
            &mask,
            mask.width(),
            mask.height(),
            image::ColorType::Rgba8,
        )
        .unwrap();
    }
}

/// Function crawls through frames folder and gathers all images for frames and their masks
pub async fn load_frames() -> std::io::Result<Vec<FrameImage>> {
    let mut res = vec![];
    let mut dirs = Vec::from(load_frames_path!());

    // loads all the images from the frames folder and its subfolders
    while let Some(p) = dirs.pop() {
        // read directory or skip if that failed
        let Ok(dir) = read_dir(p) else {
            continue;
        };

        for d in dir {
            // Skip any entries that failed to load
            let Ok(d) = d else {
                continue;
            };
            let mut path = d.path();

            // recurse into subdirectories
            if path.is_dir() {
                dirs.push(path.clone());
                continue;
            }

            // Skipping mask images since we're loading them together with their real image
            let Some(name) = path.file_stem().and_then(|n| n.to_str()).and_then(|n| Some(n.to_string())) else {
                continue;
            };
            if name.contains("-mask") {
                continue;
            }

            // loading the image
            let Ok(img) = image::open(&path) else {
                continue;
            };

            // converting the image into desired formats
            let img = img.into_rgba8();

            // Constructing the category for the image
            let category = {
                let mut image_folder = path.clone();
                image_folder.pop();
                let mut found = false;
                let category = image_folder.iter().fold(String::from(""), |mut s, i| {
                    if found {
                        s.insert(0, '/');
                        s.insert_str(0, i.to_str().unwrap());
                        s
                    } else {
                        if i.to_string_lossy() == PROJECT_FRAMES_FOLDER {
                            found = true;
                        }
                        s
                    }
                });
                if category.len() == 0 {
                    String::from("Uncategoriezed")
                } else {
                    category
                }
            };

            // loading the mask here, then adding it to the final result if it succeeds
            if let Some(ext) = path.extension().and_then(|x| x.to_str()) {
                path.set_file_name(format!("{}-mask.{}", name, ext));
            } else {
                path.set_file_name(format!("{}-mask", name));
            }

            if let Ok(mask) = image::open(path) {
                res.push(FrameImage::new(
                    name,
                    category,
                    img,
                    Some(mask.into_luma8()),
                ));
            } else {
                res.push(FrameImage::new(name, category, img, None));
            }
        }
    }

    Ok(res)
}
