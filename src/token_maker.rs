
use iced::widget::{button, column as col, container, row, tooltip, vertical_space, Row};
use iced::{executor, Alignment, Application, Command, Element, Length, Renderer, Theme};

use crate::data::Data;
use crate::file_browser::{BrowserOperation, BrowsingResult, Target, Browser};
use crate::workspace::{IndexedWorkspaceMessage, Workspace};

#[derive(Default)]
pub struct TokenMaker {
    operation: Mode,
    data: Data,
    workspaces: Vec<Workspace>,
}

#[derive(Debug, Clone)]
pub enum Message {
    LookForImage,
    LookForOutputFolder,
    FileBrowser(BrowserOperation),
    Workspace(IndexedWorkspaceMessage),
}

#[derive(Debug, Default)]
pub enum Mode {
    #[default]
    CreateWorkspace,
    Workspace,
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
                    data: Data {
                        file: Browser::new("./"),
                        ..Default::default()
                    },
                    ..Default::default()
                };
                s
            },
            Command::none(),
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
                                    let new_workspace =
                                        Workspace::new(name, img.into());
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
                        .update(message)
                        .map(move |x| Message::Workspace((index, x)))
                } else {
                    Command::none()
                }
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
}

impl TokenMaker {
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
    fn workspace_view(&self) -> Row<Message, Renderer> {
        Row::with_children(self.workspaces.iter().enumerate().fold(Vec::new(), |mut c, (i, x), | {
            c.push(x.view().map(move |x| Message::Workspace((i, x))));
            c
        }))
        .width(Length::Fill)
        .height(Length::Fill)
    }
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
