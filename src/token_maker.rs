use std::collections::HashSet;
use std::sync::Arc;

use iced::widget::{
    button, column as col, container, horizontal_space, image as picture, radio, row, text,
    text_input, tooltip, vertical_space, Row,
};
use iced::{
    executor, Alignment, Application, Command, ContentFit, Element, Length, Renderer, Subscription,
    Theme,
};

use crate::data::{load_frames, FrameImage, ProgramData, ProgramDataMessage};
use crate::file_browser::{BrowserOperation, BrowsingResult, Target};
use crate::image::{image_arc_to_handle, RgbaImage};
use crate::style::Layout;
use crate::workspace::{Workspace, WorkspaceMessage, WorkspaceTemplate};

/// Main application, manages general aspects of the application
pub struct TokenMaker {
    operation: Mode,
    data: ProgramData,
    workspaces: Vec<Workspace>,

    new_workspace_template: WorkspaceTemplate,
}

#[derive(Debug, Clone)]
pub enum Message {
    /// Opens file browser to look for an image file
    LookForImage,
    /// Opens file browser to look for a folder to which workspaces will export their images
    LookForOutputFolder,
    /// Message related to the file browser
    FileBrowser(BrowserOperation),
    /// Request to display settings
    DisplaySettings,
    /// Opens UI for adding a new workspace
    DisplayWorkspaceCreation,
    /// Request to display currently active workspaces.
    /// If none were created so far, it displays workspace creation screen
    DisplayWorkspaces,
    /// Message related to the workspace
    Workspace(usize, WorkspaceMessage),
    /// Request to close specified workspace
    WorkspaceClose(usize),
    /// Selects which workspace should be shown to the user in stacking layout
    WorkspaceSelect(usize),
    /// Request to create a new workspace and copy image used by other workspace as the base for it
    WorkspaceNewFromSource(usize),
    /// Sets default workspace template to use for new workspaces
    WorkspaceTemplate(WorkspaceTemplate),
    /// Message related to program settings
    SettingsMessage(ProgramDataMessage),
    /// Result of a task which loads in all the frames
    LoadedFrames(Vec<FrameImage>),
    /// Error message
    /// TODO turn this into a proper error handling
    Error(String),
    /// Saves images from all workspaces
    Export,
}

/// Describes which mode the program should operate in
#[derive(Debug, Default, PartialEq)]
pub enum Mode {
    /// This mode instructs the program to display workspace creation UI
    #[default]
    CreateWorkspace,
    /// Regular operation of the program, displays all active workspaces
    Workspace,
    /// Displays the file browser
    FileBrowser,
    /// Display UI for customizing how the program works or looks
    Settings,
}

impl Application for TokenMaker {
    type Executor = executor::Default;

    type Message = Message;

    type Theme = Theme;

    type Flags = ();

    fn new(_flags: Self::Flags) -> (Self, Command<Self::Message>) {
        (
            {
                let data = ProgramData::new();
                let s = Self {
                    data,
                    new_workspace_template: WorkspaceTemplate::None,
                    operation: Mode::CreateWorkspace,
                    workspaces: Vec::new(),
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

    fn theme(&self) -> Self::Theme {
        self.data.theme.into()
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
                self.data.file.set_path(&self.data.output);
                self.data.file.refresh_path().unwrap();
                Command::none()
            }
            Message::FileBrowser(x) => {
                if let Ok(x) = self.data.file.update(x) {
                    match x {
                        BrowsingResult::Pending => Command::none(),
                        BrowsingResult::Canceled => {
                            if self.workspaces.len() > 0 {
                                self.operation = Mode::Workspace;
                            } else {
                                self.operation = Mode::CreateWorkspace;
                            }
                            Command::none()
                        }
                        BrowsingResult::Done(path) => {
                            let command = if path.is_file() {
                                if let Ok(img) = image::open(&path) {
                                    let img = img.into_rgba8();
                                    let name =
                                        path.file_stem().unwrap().to_string_lossy().to_string();
                                    self.add_workspace(name, img.into())
                                } else {
                                    Command::none()
                                }
                            } else {
                                self.data.output = path;
                                Command::none()
                            };

                            if self.workspaces.len() > 0 {
                                self.operation = Mode::Workspace;
                            } else {
                                self.operation = Mode::CreateWorkspace;
                            }
                            command
                        }
                    }
                } else {
                    Command::none()
                }
            }
            Message::WorkspaceNewFromSource(index) => {
                let command = if let Some(w) = self.workspaces.get(index) {
                    let img = w.get_source().clone();
                    let name = w.get_output_name().to_string();
                    self.add_workspace(name, img)
                } else {
                    Command::none()
                };
                self.operation = Mode::Workspace;
                command
            }
            Message::DisplayWorkspaceCreation => {
                self.operation = Mode::CreateWorkspace;
                Command::none()
            }
            Message::DisplayWorkspaces => {
                if self.workspaces.len() > 0 {
                    self.operation = Mode::Workspace;
                } else {
                    self.operation = Mode::CreateWorkspace;
                }
                Command::none()
            }
            Message::DisplaySettings => {
                self.operation = Mode::Settings;
                Command::none()
            }
            Message::SettingsMessage(x) => self.data.update(x).map(|x| Message::SettingsMessage(x)),
            Message::WorkspaceClose(index) => {
                if self.workspaces.len() > index {
                    self.workspaces.remove(index);
                    if self.workspaces.len() == 0 {
                        self.operation = Mode::CreateWorkspace;
                        self.data.naming.project_name = String::from("");
                    }
                }
                Command::none()
            }
            Message::WorkspaceSelect(i) => {
                self.data.layout = Layout::Stacking(i);
                Command::none()
            }
            Message::Workspace(index, message) => {
                if let Some(workspace) = self.workspaces.get_mut(index) {
                    workspace
                        .update(message, &mut self.data)
                        .map(move |x| Message::Workspace(index, x))
                } else {
                    Command::none()
                }
            }
            Message::WorkspaceTemplate(t) => {
                self.new_workspace_template = t;
                Command::none()
            }
            Message::LoadedFrames(frames) => {
                self.data.available_frames = frames;
                Command::none()
            }
            Message::Error(e) => {
                eprintln!("Error: {}", e);
                Command::none()
            }
            Message::Export => {
                self.workspaces.iter().for_each(|x| x.export(&self.data));
                Command::none()
            }
        }
    }

    fn view(&self) -> iced::Element<'_, Self::Message, iced::Renderer<Self::Theme>> {
        let top_bar = self.top_bar();
        let ui = match self.operation {
            Mode::FileBrowser => self.data.file.view().map(|x| Message::FileBrowser(x)),
            Mode::CreateWorkspace => self.workspace_add_view(),
            Mode::Workspace => self.workspace_view(),
            Mode::Settings => self.settings_view(),
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
                .map(|(i, m)| Message::Workspace(i, m));
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
    /// This function adds a new workspace with given data
    fn add_workspace(&mut self, name: String, image: Arc<RgbaImage>) -> Command<Message> {
        let i = self.workspaces.len();
        // Updating project name if we have nothing open
        if i == 0 && self.data.naming.project_name.len() == 0 {
            self.data.naming.project_name = name;
        }
        let name = self.data.naming.get(&self.new_workspace_template);

        let (command, new_workspace) =
            Workspace::new(name, image, &self.data, self.new_workspace_template);
        let command = command.map(move |x| Message::Workspace(i, x));

        // Switching to a new tab if the layout is stacking
        if matches!(self.data.layout, Layout::Stacking(_)) {
            self.data.layout = Layout::Stacking(i)
        }
        self.workspaces.push(new_workspace);
        command
    }

    /// Checks if it is save to save images
    fn can_save(&self) -> bool {
        if self.data.output.exists() == false {
            return false;
        }
        if self.workspaces.len() == 0 {
            return false;
        }
        self.workspaces.iter().all(|x| x.can_save())
    }
    /// Main program UI located at the top of the window
    fn top_bar(&self) -> Element<Message, Renderer> {
        row![
            button("Add").on_press(Message::DisplayWorkspaceCreation),
            tooltip(
                button("Set Output Folder").on_press(Message::LookForOutputFolder),
                format!("Current output: {}", self.data.output.to_string_lossy()),
                tooltip::Position::Right
            ),
            if self.can_save() {
                button("Save").on_press(Message::Export)
            } else {
                button("Save")
            },
            horizontal_space(Length::Fill),
            text("Project Name: "),
            text_input("Project Name", &self.data.naming.project_name, |x| {
                Message::SettingsMessage(ProgramDataMessage::SetProjectName(x))
            }),
            horizontal_space(Length::Fill),
            if self.operation != Mode::Settings {
                button("Settings").on_press(Message::DisplaySettings)
            } else {
                button("Workspaces").on_press(Message::DisplayWorkspaces)
            },
        ]
        .align_items(Alignment::Center)
        .spacing(2)
        .width(Length::Fill)
        .height(Length::Shrink)
        .into()
    }
    /// Constructs UI for customizing program settings
    fn settings_view(&self) -> Element<Message, Renderer> {
        self.data.view().map(|x| Message::SettingsMessage(x))
    }
    /// Constructs UI for displaying all workspaces
    fn workspace_view(&self) -> Element<Message, Renderer> {
        // Handles converting the workspace message into program message
        macro_rules! event {
            ($ev:expr, $i:expr) => {
                match &$ev {
                    // Handling requests sent from workspace to the application
                    WorkspaceMessage::Close => Message::WorkspaceClose($i),
                    // only specific requests are considered for application, others are routed back to the workspace
                    _ => Message::Workspace($i, $ev),
                }
            };
        }

        // Different drawings for different layouts
        match self.data.layout {
            Layout::Parallel => {
                container(Row::with_children(self.workspaces.iter().enumerate().fold(
                    Vec::new(),
                    |mut c, (i, x)| {
                        c.push(x.view(&self.data).map(move |x| event!(x, i)));
                        c
                    },
                )))
            }
            Layout::Stacking(i) => {
                let ui = self.workspaces.get(i).unwrap();
                let ui = ui.view(&self.data).map(move |x| event!(x, i));
                container(col![
                    (0..self.workspaces.len()).fold(
                        row![text("Workspaces: ")]
                            .spacing(2)
                            .align_items(Alignment::Center),
                        |r, i| r.push(
                            button(text(i.to_string())).on_press(Message::WorkspaceSelect(i))
                        )
                    ),
                    ui
                ])
            }
        }
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    /// Constructs UI for creating a new workspace
    fn workspace_add_view(&self) -> Element<Message, Renderer> {
        let templates = col![
            text("Template for new workspace:"),
            WorkspaceTemplate::ALL
                .iter()
                .fold(row![].spacing(10), |r, wt| {
                    let wt = *wt;
                    let opt = radio(wt.to_string(), wt, Some(self.new_workspace_template), |x| {
                        Message::WorkspaceTemplate(x)
                    });
                    r.push(opt)
                }),
        ]
        .spacing(4);

        let openers = button("Open file").on_press(Message::LookForImage);

        if self.workspaces.len() > 0 {
            // checker has function of preventing multiple of the same image being shown to user
            let mut checker = HashSet::new();

            // sourcers allow user to use already loaded image for the new frame
            let sourcers = col![
                row![
                    button("Cancel").on_press(Message::DisplayWorkspaces),
                    text("Source from other workspace:"),
                ]
                .spacing(2),
                self.workspaces
                    .iter()
                    .enumerate()
                    .fold(row![], |r, (i, w)| {
                        let img = w.get_source();
                        if checker.contains(img) {
                            return r;
                        }
                        let r = r.push(
                            button(
                                picture(image_arc_to_handle(img))
                                    .content_fit(ContentFit::ScaleDown)
                                    .width(Length::Shrink),
                            )
                            .on_press(Message::WorkspaceNewFromSource(i)),
                        );
                        checker.insert(img);
                        r
                    })
            ]
            .spacing(2);

            col![
                vertical_space(Length::Fill),
                templates,
                vertical_space(Length::Fill),
                sourcers,
                vertical_space(Length::Fill),
                openers,
                vertical_space(Length::Fill)
            ]
        } else {
            col![
                vertical_space(Length::Fill),
                templates,
                vertical_space(Length::Fill),
                openers,
                vertical_space(Length::Fill)
            ]
        }
        .spacing(4)
        .height(Length::Fill)
        .width(Length::Fill)
        .align_items(Alignment::Center)
        .into()
    }
}
