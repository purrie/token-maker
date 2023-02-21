use iced::widget::{button, column as col, container, row, tooltip, vertical_space, Row};
use iced::{
    executor, Alignment, Application, Command, Element, Length, Renderer, Subscription, Theme,
};

use crate::data::{load_frames, FrameImage, ProgramData};
use crate::file_browser::{Browser, BrowserOperation, BrowsingResult, Target};
use crate::workspace::{IndexedWorkspaceMessage, Workspace};

/// Main application, manages general aspects of the application
#[derive(Default)]
pub struct TokenMaker {
    operation: Mode,
    data: ProgramData,
    workspaces: Vec<Workspace>,
}

#[derive(Debug, Clone)]
pub enum Message {
    /// Opens file browser to look for an image file
    LookForImage,
    /// Opens file browser to look for a folder to which workspaces will export their images
    LookForOutputFolder,
    /// Message related to the file browser
    FileBrowser(BrowserOperation),
    /// Message related to the workspace
    Workspace(IndexedWorkspaceMessage),
    /// Result of a task which loads in all the frames
    LoadedFrames(Vec<FrameImage>),
    /// Error message
    /// TODO turn this into a proper error handling
    Error(String),
}

#[derive(Debug, Default)]
pub enum Mode {
    /// This mode instructs the program to display workspace creation UI
    #[default]
    CreateWorkspace,
    /// Regular operation of the program, displays all active workspaces
    Workspace,
    /// Displays the file browser
    FileBrowser,
}

impl Application for TokenMaker {
    type Executor = executor::Default;

    type Message = Message;

    type Theme = Theme;

    type Flags = ();

    fn new(_flags: Self::Flags) -> (Self, Command<Self::Message>) {
        (
            {
                let s = Self {
                    data: ProgramData {
                        file: Browser::new("./"),
                        ..Default::default()
                    },
                    ..Default::default()
                };
                s
            },
            Command::perform(load_frames(), |x| {
                if let Ok(x) = x {
                    if x.len() > 0 {
                        Message::LoadedFrames(x)
                    } else {
                        Message::Error("Could not find any frames".to_string())
                    }
                } else {
                    Message::Error("Failed to load frames".to_string())
                }
            }),
        )
    }

    fn title(&self) -> String {
        String::from("Token Maker")
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            Message::LookForImage => {
                self.operation = Mode::FileBrowser;
                self.data.file.set_target(Target::Filtered("png".into()));
                self.data.file.refresh_path().unwrap();
                Command::none()
            }
            Message::LookForOutputFolder => {
                self.operation = Mode::FileBrowser;
                self.data.file.set_target(Target::Directory);
                self.data.file.refresh_path().unwrap();
                Command::none()
            }
            Message::FileBrowser(x) => {
                if let Ok(x) = self.data.file.update(x) {
                    match x {
                        BrowsingResult::Pending => {}
                        BrowsingResult::Canceled => {}
                        BrowsingResult::Done(path) => {
                            if path.is_file() {
                                if let Ok(img) = image::open(&path) {
                                    let img = img.into_rgba8();
                                    let name =
                                        path.file_name().unwrap().to_string_lossy().to_string();
                                    let new_workspace = Workspace::new(name, img.into());
                                    self.workspaces.push(new_workspace);
                                }
                            } else {
                                self.data.output = path;
                            }
                            if self.workspaces.len() > 0 {
                                self.operation = Mode::Workspace;
                            } else {
                                self.operation = Mode::CreateWorkspace;
                            }
                        }
                    }
                }
                Command::none()
            }
            Message::Workspace((index, message)) => {
                if let Some(workspace) = self.workspaces.get_mut(index) {
                    workspace
                        .update(message, &self.data)
                        .map(move |x| Message::Workspace((index, x)))
                } else {
                    Command::none()
                }
            }
            Message::LoadedFrames(frames) => {
                self.data.available_frames = frames;
                Command::none()
            }
            Message::Error(e) => {
                eprintln!("Error: {}", e);
                Command::none()
            }
        }
    }

    fn view(&self) -> iced::Element<'_, Self::Message, iced::Renderer<Self::Theme>> {
        let top_bar = self.top_bar();
        let ui = match self.operation {
            Mode::FileBrowser => self.data.file.view().map(|x| Message::FileBrowser(x)),
            Mode::CreateWorkspace => self.workspace_view().push(self.workspace_add()).into(),
            Mode::Workspace => self.workspace_view().into(),
        };
        let ui = col![top_bar, ui].height(Length::Fill).width(Length::Fill);
        container(ui)
            .height(Length::Fill)
            .width(Length::Fill)
            .center_x()
            .center_y()
            .into()
    }
    fn subscription(&self) -> Subscription<Self::Message> {
        // collects subscribtions from workspaces and sends them to the framework
        // Everything is worked into regular workspace update cycle
        let mut subs = Vec::new();
        self.workspaces.iter().enumerate().for_each(|(i, x)| {
            let s = x
                .subscribtion()
                .with(i)
                .map(|(i, m)| Message::Workspace((i, m)));
            subs.push(s)
        });
        if subs.len() > 0 {
            Subscription::batch(subs)
        } else {
            Subscription::none()
        }
    }
}

impl TokenMaker {
    /// Main program UI located at the top of the window
    fn top_bar(&self) -> Element<Message, Renderer> {
        row![
            button("add"),
            button("remove"),
            tooltip(
                button("Set Output Folder").on_press(Message::LookForOutputFolder),
                format!("Current output: {}", self.data.output.to_string_lossy()),
                tooltip::Position::Right
            )
        ]
        .width(Length::Fill)
        .height(Length::Shrink)
        .into()
    }
    /// Constructs UI for displaying all workspaces
    fn workspace_view(&self) -> Row<Message, Renderer> {
        Row::with_children(
            self.workspaces
                .iter()
                .enumerate()
                .fold(Vec::new(), |mut c, (i, x)| {
                    c.push(x.view(&self.data).map(move |x| Message::Workspace((i, x))));
                    c
                }),
        )
        .width(Length::Fill)
        .height(Length::Fill)
    }
    /// Constructs UI for creating a new workspace
    fn workspace_add(&self) -> Element<Message, Renderer> {
        col![
            vertical_space(Length::Fill),
            button("Open file").on_press(Message::LookForImage),
            vertical_space(Length::Fill)
        ]
        .height(Length::Fill)
        .width(Length::Fill)
        .align_items(Alignment::Center)
        .into()
    }
}
