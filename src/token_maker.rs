use std::collections::HashSet;
use std::sync::Arc;

use iced::widget::tooltip::Position;
use iced::widget::{
    button, column as col, container, horizontal_space, image as picture, radio, row, text,
    text_input, tooltip, vertical_space, Row,
};
use iced::{
    executor, Alignment, Application, Command, ContentFit, Element, Length, Renderer, Subscription,
    Theme,
};

use crate::data::{load_frames, FrameImage, ProgramData, ProgramDataMessage};
use crate::frame_maker::{FrameMaker, FrameMakerMessage};
use crate::image::{download_image, image_filter, RgbaImage};
use crate::style::{Layout, Style};
use crate::widgets::{BrowserOperation, BrowsingResult, Target};
use crate::workspace::{Workspace, WorkspaceMessage, WorkspaceTemplate};

/// Main application, manages general aspects of the application
pub struct TokenMaker {
    operation: Mode,
    data: ProgramData,
    workspaces: Vec<Workspace>,
    frame_maker: FrameMaker,

    download_in_progress: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    /// Opens file browser to look for an image file
    LookForImage,
    /// Grabs URL address from clipboard and passes it further to download
    LookForImageFromUrl,
    /// Starts a download of an image
    DownloadImage(String),
    /// Result of the image download
    ImageDownloadResult(Result<RgbaImage, String>),
    /// Opens file browser to look for a folder to which workspaces will export their images
    LookForOutputFolder,
    /// Message related to the file browser
    FileBrowser(BrowserOperation),
    /// Request to display settings
    DisplaySettings,
    /// Opens UI for adding a new workspace
    DisplayWorkspaceCreation,
    /// Switches screen to let user close a workspace
    DisplayCloseWorkspace,
    /// Request to display currently active workspaces.
    /// If none were created so far, it displays workspace creation screen
    DisplayWorkspaces,
    /// Displays screen for replacing image in all workspaces
    DisplaySourceImageReplacement,
    /// Request to display frame making editor
    LookForFrame,
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
    /// Messages meant for frame maker editor
    FrameMakerMessage(FrameMakerMessage),
    /// Request to export frame in frame editor
    FrameMakerExport,
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
    /// Display widgets for closing open workspaces
    CloseWorkspace,
    /// Display the file browser
    FileBrowser(BrowsingFor),
    /// Display UI for customizing how the program works or looks
    Settings,
    /// Display editor for making frames
    FrameMaker,
    /// Screen for swapping image in all open workspaces
    SourceSwap,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BrowsingFor {
    Token,
    ReplacementToken,
    Output,
    Frame,
}

impl Application for TokenMaker {
    type Executor = executor::Default;

    type Message = Message;

    type Theme = Theme;

    type Flags = ();

    fn new(_flags: Self::Flags) -> (Self, Command<Self::Message>) {
        (
            {
                let mut data = ProgramData::new();
                data.status.log("Loading frames...");
                let s = Self {
                    data,
                    operation: Mode::CreateWorkspace,
                    workspaces: Vec::new(),
                    frame_maker: FrameMaker::new(),
                    download_in_progress: false,
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
        self.data.get_theme().into()
    }
    fn title(&self) -> String {
        String::from("Token Maker")
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            Message::LookForImage => {
                match self.operation {
                    Mode::CreateWorkspace => self.operation = Mode::FileBrowser(BrowsingFor::Token),
                    Mode::SourceSwap => {
                        self.operation = Mode::FileBrowser(BrowsingFor::ReplacementToken)
                    }
                    _ => unreachable!(),
                }

                self.data.file.set_filter(image_filter);
                self.data.file.refresh_path().unwrap();
                Command::none()
            }

            Message::LookForImageFromUrl => iced::clipboard::read(|clip| {
                let Some(clip) = clip else {
                        return Message::ImageDownloadResult(Err("Clipboard is empty".to_string()));
                    };
                Message::DownloadImage(clip)
            }),

            Message::DownloadImage(url) => {
                self.download_in_progress = true;
                Command::perform(
                    async move {
                        let img = download_image(url).await;
                        Message::ImageDownloadResult(img)
                    },
                    |x| x,
                )
            }

            Message::ImageDownloadResult(res) => {
                self.download_in_progress = false;
                match res {
                    Ok(img) => match self.operation {
                        Mode::CreateWorkspace => {
                            let name = String::from("image");
                            let c = self.add_workspace(name, img.into());
                            self.main_screen();
                            c
                        }
                        Mode::SourceSwap => {
                            let arc = Arc::new(img);
                            self.data.naming.project_name = String::from("image");
                            let cmd = self
                                .workspaces
                                .iter_mut()
                                .map(|x| x.set_source(arc.clone(), &self.data))
                                .enumerate()
                                .map(|(i, x)| x.map(move |x| Message::Workspace(i, x)))
                                .fold(vec![], |mut v, c| {
                                    v.push(c);
                                    v
                                });

                            self.main_screen();
                            Command::batch(cmd)
                        }
                        _ => unreachable!(),
                    },
                    Err(e) => {
                        self.data.status.error(&e);
                        Command::none()
                    }
                }
            }

            Message::LookForOutputFolder => {
                self.operation = Mode::FileBrowser(BrowsingFor::Output);
                self.data.file.set_target(Target::Directory);
                if self.data.get_output_folder().exists() {
                    self.data
                        .file
                        .set_path(self.data.get_output_folder().clone());
                }
                self.data.file.refresh_path().unwrap();
                Command::none()
            }

            Message::LookForFrame => {
                self.operation = Mode::FileBrowser(BrowsingFor::Frame);
                self.data.file.set_filter(image_filter);
                self.data.file.refresh_path().unwrap();
                Command::none()
            }

            Message::FileBrowser(x) => {
                if let Ok(x) = self.data.file.update(x, &mut self.data.status) {
                    match x {
                        BrowsingResult::Pending => Command::none(),
                        BrowsingResult::Canceled => {
                            self.main_screen();
                            Command::none()
                        }
                        BrowsingResult::Done(path) => {
                            let Mode::FileBrowser(reason) = &self.operation else {
                                panic!("How did we get here...");
                            };
                            match reason {
                                BrowsingFor::Token => {
                                    if let Ok(img) = image::open(&path) {
                                        let img = img.into_rgba8();
                                        let name =
                                            path.file_stem().unwrap().to_string_lossy().to_string();
                                        let c = self.add_workspace(name, img.into());
                                        self.main_screen();
                                        c
                                    } else {
                                        Command::none()
                                    }
                                }

                                BrowsingFor::ReplacementToken => {
                                    if let Ok(img) = image::open(&path) {
                                        let img = img.into_rgba8();
                                        let name =
                                            path.file_stem().unwrap().to_string_lossy().to_string();
                                        self.data.naming.project_name = name;
                                        let img = Arc::new(img);
                                        let cmd = self
                                            .workspaces
                                            .iter_mut()
                                            .map(|x| x.set_source(img.clone(), &self.data))
                                            .enumerate()
                                            .map(|(i, x)| x.map(move |x| Message::Workspace(i, x)))
                                            .fold(vec![], |mut v, c| {
                                                v.push(c);
                                                v
                                            });
                                        self.main_screen();
                                        Command::batch(cmd)
                                    } else {
                                        Command::none()
                                    }
                                }

                                BrowsingFor::Output => {
                                    self.data.set_output_folder(path);
                                    self.main_screen();
                                    Command::none()
                                }

                                BrowsingFor::Frame => {
                                    if let Ok(img) = image::open(&path) {
                                        let img = img.into_rgba8();
                                        self.frame_maker.load(img);
                                        self.frame_maker.set_name(
                                            path.file_stem()
                                                .and_then(|x| x.to_str())
                                                .and_then(|x| Some(x.to_string()))
                                                .unwrap(),
                                        );
                                        self.operation = Mode::FrameMaker;
                                    } else {
                                        self.main_screen();
                                    }
                                    Command::none()
                                }
                            }
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

            Message::DisplayCloseWorkspace => {
                self.operation = Mode::CloseWorkspace;
                Command::none()
            }

            Message::DisplayWorkspaceCreation => {
                self.operation = Mode::CreateWorkspace;
                Command::none()
            }

            Message::DisplayWorkspaces => {
                self.main_screen();
                Command::none()
            }

            Message::DisplaySourceImageReplacement => {
                self.operation = Mode::SourceSwap;
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
                self.main_screen();
                Command::none()
            }

            Message::WorkspaceSelect(i) => {
                self.data.set_layout(Layout::Stacking(i));
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
                self.data.set_workspace_template(t);
                Command::none()
            }

            Message::LoadedFrames(frames) => {
                self.data.available_frames = frames;
                self.data.status.log("Frames loaded");
                Command::none()
            }

            Message::Error(e) => {
                eprintln!("Error: {}", e);
                self.data.status.error(&e);
                Command::none()
            }

            Message::Export => {
                self.workspaces.iter().for_each(|x| x.export(&self.data));
                self.data.status.log("Export successful");
                Command::none()
            }

            Message::FrameMakerMessage(x) => self
                .frame_maker
                .update(x, &mut self.data)
                .map(|x| Message::FrameMakerMessage(x)),

            Message::FrameMakerExport => {
                self.main_screen();
                let frame = self.frame_maker.create_frame();
                frame.save_frame();
                self.data.status.log("Frame saved successfully");
                self.data.available_frames.push(frame);
                Command::none()
            }
        }
    }

    fn view(&self) -> iced::Element<'_, Self::Message, iced::Renderer<Self::Theme>> {
        let top_bar = self.top_bar();

        let status = self
            .data
            .status
            .view()
            .map(|_| Message::Error("Status line shouldn't cast messages for now".to_string()));

        let ui = match self.operation {
            Mode::FileBrowser(_) => col![
                self.data.file.view().map(|x| Message::FileBrowser(x)),
                status
            ],
            Mode::SourceSwap => col![top_bar, self.swap_source_image_view(), status,],
            Mode::CreateWorkspace => col![top_bar, self.workspace_add_view(), status],
            Mode::Workspace => col![top_bar, self.workspace_view(), status],
            Mode::Settings => col![top_bar, self.settings_view(), status],
            Mode::FrameMaker => col![
                top_bar,
                self.frame_maker
                    .view(&self.data)
                    .map(|x| Message::FrameMakerMessage(x)),
                status
            ],
            Mode::CloseWorkspace => col![top_bar, self.workspace_close_view(), status],
        };

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
    fn main_screen(&mut self) {
        if self.workspaces.len() > 0 {
            self.operation = Mode::Workspace;
        } else {
            self.operation = Mode::CreateWorkspace;
        }
    }

    /// This function adds a new workspace with given data
    fn add_workspace(&mut self, name: String, image: Arc<RgbaImage>) -> Command<Message> {
        let i = self.workspaces.len();
        // Updating project name if we have nothing open
        if i == 0 && self.data.naming.project_name.len() == 0 {
            self.data.naming.project_name = name;
        }
        let name = self.data.naming.get(&self.data.get_workspace_template());

        let (command, new_workspace) = Workspace::new(name, image, &self.data);
        let command = command.map(move |x| Message::Workspace(i, x));

        // Switching to a new tab if the layout is stacking
        if matches!(self.data.get_layout(), Layout::Stacking(_)) {
            self.data.set_layout(Layout::Stacking(i))
        }
        self.workspaces.push(new_workspace);
        command
    }

    /// Checks if it is save to save images
    fn can_save(&self) -> Result<(), String> {
        if self.data.get_output_folder().exists() == false {
            return Err(String::from("Export folder not set"));
        }
        if self.workspaces.len() == 0 {
            return Err(String::from("There's nothing to export"));
        }
        for (ix, x) in self.workspaces.iter().enumerate() {
            if x.can_save() == false {
                return Err(String::from("Waitning for workspaces"));
            }
            if self
                .workspaces
                .iter()
                .enumerate()
                .all(|(io, o)| io == ix || o.get_output_name() != x.get_output_name())
                == false
            {
                return Err(String::from(
                    "Can't set the same export name for multiple workspaces",
                ));
            }
        }
        Ok(())
    }

    /// Main program UI located at the top of the window
    fn top_bar(&self) -> Element<Message, Renderer> {
        let left = match self.operation {
            Mode::Workspace => row![
                text("Workspace: "),
                button("Add").on_press(Message::DisplayWorkspaceCreation),
                button("Close").on_press(Message::DisplayCloseWorkspace),
                tooltip(
                    button("Replace Image").on_press(Message::DisplaySourceImageReplacement),
                    "Replace images in all open workspaces",
                    Position::Bottom
                )
                .style(Style::Frame)
            ]
            .align_items(Alignment::Center)
            .spacing(4),
            Mode::CreateWorkspace if self.workspaces.len() > 0 => {
                row![button("Cancel").on_press(Message::DisplayWorkspaces)]
            }
            Mode::CloseWorkspace => {
                row![button("Cancel").on_press(Message::DisplayWorkspaces)]
            }
            Mode::FrameMaker => {
                row![button("Cancel").on_press(Message::DisplayWorkspaces)]
            }
            Mode::SourceSwap => {
                row![button("Cancel").on_press(Message::DisplayWorkspaces)]
            }
            _ => {
                row![]
            }
        };

        let right = match self.operation {
            Mode::FrameMaker => {
                if self.frame_maker.can_save() {
                    row![button("Export").on_press(Message::FrameMakerExport)]
                } else {
                    row![tooltip(
                        button("Can't save yet"),
                        "Click on the image to create the mask first",
                        tooltip::Position::Left
                    )
                    .style(Style::Frame)]
                }
            }
            Mode::Settings => {
                row![button("Close").on_press(Message::DisplayWorkspaces)]
            }
            Mode::FileBrowser(_) => {
                row![]
            }
            _ => {
                row![
                    button("Frame Maker").on_press(Message::LookForFrame),
                    button("Settings").on_press(Message::DisplaySettings)
                ]
            }
        }
        .spacing(5);

        let bar = if self.workspaces.len() > 0 && self.operation == Mode::Workspace {
            let center = row![
                text("Project Name: "),
                text_input("Project Name", &self.data.naming.project_name, |x| {
                    Message::SettingsMessage(ProgramDataMessage::SetProjectName(x))
                }),
                tooltip(
                    button("Set Export Path").on_press(Message::LookForOutputFolder),
                    format!(
                        "Current Path: {}",
                        self.data.get_output_folder().to_string_lossy()
                    ),
                    Position::Bottom
                )
                .style(Style::Frame),
                if let Err(e) = self.can_save() {
                    tooltip(button("Export"), e, Position::Bottom).style(Style::Frame)
                } else {
                    if self
                        .workspaces
                        .iter()
                        .any(|x| x.is_destructive_export(&self.data))
                    {
                        tooltip(
                            button("Export")
                                .on_press(Message::Export)
                                .style(Style::Danger.into()),
                            "One or more workspaces will override existing file",
                            Position::Bottom,
                        )
                        .style(Style::Frame)
                    } else {
                        tooltip(
                            button("Export").on_press(Message::Export),
                            "Export to selected folder",
                            tooltip::Position::Bottom,
                        )
                        .style(Style::Frame)
                    }
                },
            ]
            .spacing(5)
            .align_items(Alignment::Center);

            row![
                left,
                horizontal_space(Length::Fill),
                center.width(Length::FillPortion(4)),
                horizontal_space(Length::Fill),
                right,
            ]
        } else {
            row![left, horizontal_space(Length::Fill), right,]
        }
        .align_items(Alignment::Center)
        .spacing(2);

        container(bar)
            .padding(5)
            .style(Style::Header)
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
        // Different drawings for different layouts
        match self.data.get_layout() {
            Layout::Parallel => {
                container(Row::with_children(self.workspaces.iter().enumerate().fold(
                    Vec::new(),
                    |mut c, (i, x)| {
                        c.push(x.view(&self.data).map(move |x| Message::Workspace(i, x)));
                        c
                    },
                )))
            }
            Layout::Stacking(i) => {
                let ui = self.workspaces.get(i).unwrap();
                let ui = ui.view(&self.data).map(move |x| Message::Workspace(i, x));
                container(col![
                    (0..self.workspaces.len()).fold(
                        row![text("Workspaces: ")]
                            .spacing(2)
                            .padding(5)
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

    fn download_in_progress_view(&self) -> Element<Message, Renderer> {
        let content = row![
            horizontal_space(Length::Fill),
            container(text("Loading..."))
                .style(Style::Frame)
                .padding(20),
            horizontal_space(Length::Fill),
        ]
        .align_items(Alignment::Center)
        .height(Length::Fill)
        .width(Length::Fill);
        container(content)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(Style::Margins)
            .into()
    }

    fn swap_source_image_view(&self) -> Element<Message, Renderer> {
        if self.download_in_progress {
            return self.download_in_progress_view();
        }

        let openers = row![
            button("Open file").on_press(Message::LookForImage),
            button("Paste URL").on_press(Message::LookForImageFromUrl),
        ]
        .spacing(5);

        let openers = container(openers).style(Style::Frame).padding(20);

        container(col![
            vertical_space(Length::Fill),
            row![
                horizontal_space(Length::Fill),
                openers,
                horizontal_space(Length::Fill),
            ],
            vertical_space(Length::Fill),
        ])
        .width(Length::Fill)
        .height(Length::Fill)
        .style(Style::Margins)
        .into()
    }

    fn workspace_close_view(&self) -> Element<Message, Renderer> {
        let views = self
            .workspaces
            .iter()
            .enumerate()
            .map(|(i, x)| (i, picture(x.get_preview())))
            .map(|(i, x)| {
                // all this just to have UI parity with the workspace
                let butt = button("Close").on_press(Message::WorkspaceClose(i));
                let butt = container(butt)
                    .width(Length::Fill)
                    .height(Length::FillPortion(1))
                    .style(Style::Frame)
                    .center_x()
                    .center_y();
                let butt = container(butt)
                    .padding(2)
                    .width(Length::Fill)
                    .height(Length::FillPortion(1))
                    .style(Style::Margins);
                let x = container(x)
                    .width(Length::Fill)
                    .height(Length::FillPortion(3))
                    .style(Style::Margins)
                    .center_x()
                    .center_y();
                col![butt, x]
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .align_items(Alignment::Center)
            })
            .fold(row![], |r, c| r.push(c));

        views.into()
    }

    /// Constructs UI for creating a new workspace
    fn workspace_add_view(&self) -> Element<Message, Renderer> {
        if self.download_in_progress {
            return self.download_in_progress_view();
        }
        let templates = WorkspaceTemplate::ALL.iter().fold(
            row![text("Template:")]
                .spacing(10)
                .align_items(Alignment::Center),
            |r, wt| {
                let wt = *wt;
                let opt = radio(
                    wt.to_string(),
                    wt,
                    Some(self.data.get_workspace_template()),
                    |x| Message::WorkspaceTemplate(x),
                );
                r.push(opt)
            },
        );

        let openers = row![
            tooltip(
                button("Open file").on_press(Message::LookForImage),
                "Open an image from local drive",
                Position::Bottom,
            )
            .style(Style::Frame),
            tooltip(
                button("Paste URL").on_press(Message::LookForImageFromUrl),
                "Copy an URL and click this button to load an image from the internet",
                Position::Bottom,
            )
            .style(Style::Frame)
        ]
        .spacing(5);

        let templates = container(templates).style(Style::Frame).padding(20);
        let openers = container(openers).style(Style::Frame).padding(20);

        let ui = if self.workspaces.len() > 0 {
            // checker has function of preventing multiple of the same image being shown to user
            let mut checker = HashSet::new();

            // sourcers allow user to use already loaded image for the new frame
            let sourcers = col![
                tooltip(
                    text("Use Existing:"),
                    "Use image that's already loaded in another workspace",
                    Position::Bottom,
                )
                .style(Style::Frame),
                self.workspaces
                    .iter()
                    .enumerate()
                    .fold(row![].spacing(5), |r, (i, w)| {
                        let img = w.get_source();
                        if checker.contains(img) {
                            return r;
                        }
                        let r = r.push(
                            button(
                                picture(w.get_source_preview())
                                    .content_fit(ContentFit::Contain)
                                    .width(256)
                                    .height(256),
                            )
                            .style(Style::Frame.into())
                            .on_press(Message::WorkspaceNewFromSource(i)),
                        );
                        checker.insert(img);
                        r
                    })
            ]
            .align_items(Alignment::Center)
            .spacing(2);

            col![
                vertical_space(Length::Fill),
                templates,
                vertical_space(10),
                openers,
                vertical_space(10),
                sourcers,
                vertical_space(Length::Fill)
            ]
        } else {
            col![
                vertical_space(Length::Fill),
                templates,
                vertical_space(10),
                openers,
                vertical_space(Length::Fill)
            ]
        }
        .spacing(4)
        .height(Length::Fill)
        .width(Length::Fill)
        .align_items(Alignment::Center);

        container(ui)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(Style::Margins)
            .into()
    }
}
