use iced::widget::{button, column as col, container, row, Row};
use iced::{executor, Application, Command, Element, Length, Renderer, Theme};

use crate::file_browser::{Browser, BrowserOperation, BrowsingResult};
use crate::workspace::{IndexedWorkspaceMessage, Workspace};

#[derive(Default)]
pub struct TokenMaker {
    operation: Mode,
    file: Browser,
    workspaces: Vec<Workspace>,
}

#[derive(Debug, Clone)]
pub enum Message {
    OpenFileBrowser,
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
                let mut s = Self {
                    ..Default::default()
                };
                s.file.set_filter("png");
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
            Message::OpenFileBrowser => {
                self.operation = Mode::FileBrowser;
                self.file.refresh_path().unwrap();
                Command::none()
            }
            Message::FileBrowser(x) => {
                if let Ok(x) = self.file.update(x) {
                    match x {
                        BrowsingResult::Pending => {}
                        BrowsingResult::Canceled => {}
                        BrowsingResult::Done(path) => {
                            if let Ok(img) = image::open(&path) {
                                let name = path.file_name().unwrap().to_string_lossy().to_string();
                                let new_workspace =
                                    Workspace::new(self.workspaces.len(), name, img);
                                self.workspaces.push(new_workspace);
                            }
                            self.operation = Mode::Workspace;
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
            Mode::FileBrowser => self.file.view().map(|x| Message::FileBrowser(x)),
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
        row![button("add"), button("remove")]
            .width(Length::Fill)
            .height(Length::Shrink)
            .into()
    }
    fn workspace_view(&self) -> Row<Message, Renderer> {
        Row::with_children(self.workspaces.iter().fold(Vec::new(), |mut c, x| {
            c.push(x.view().map(|x| Message::Workspace(x)));
            c
        }))
        .width(Length::Fill)
        .height(Length::Fill)
    }
    fn workspace_add(&self) -> Element<Message, Renderer> {
        col![button("Open new file").on_press(Message::OpenFileBrowser)]
            .height(Length::Fill)
            .width(Length::Fill)
            .into()
    }
}
