use std::path::PathBuf;

use iced::widget::{
    button, column as col, container, horizontal_space, row, scrollable, text, Column, Container,
};
use iced::{Element, Length, Renderer};

pub struct Browser {
    path: PathBuf,
    selected: Option<PathBuf>,
    dir: Vec<PathBuf>,
    target: Target,
}

#[derive(Debug, Clone)]
pub enum BrowserOperation {
    MoveUp,
    MoveInto(PathBuf),
    Select(Option<PathBuf>),
    Cancel,
    Accept,
}

pub enum BrowsingResult {
    Pending,
    Canceled,
    Done(PathBuf),
}

#[derive(Default)]
pub enum Target {
    #[default]
    File,
    Filtered(Box<dyn Fn(&PathBuf) -> bool>),
    Directory,
}

#[allow(unused)]
impl Browser {
    /// Creates a browser and sets its current directory to supplied path
    pub fn new<P: Into<PathBuf>>(path: P) -> Self {
        Self {
            path: path.into(),
            selected: None,
            dir: Vec::new(),
            target: Target::File,
        }
    }

    /// Creates a browser and sets browser path to home directory
    pub fn start_at_home() -> Self {
        let path = dirs::home_dir().unwrap();
        Self {
            path,
            selected: None,
            dir: Vec::new(),
            target: Target::File,
        }
    }

    /// Sets current path for the browser
    pub fn set_path<P: Into<PathBuf>>(&mut self, path: P) {
        self.path = path.into()
    }

    /// Peeks current path of the browser
    pub fn get_path(&self) -> &PathBuf {
        &self.path
    }

    /// Sets target to file with supplied filter function
    pub fn set_filter<F: Fn(&PathBuf) -> bool + 'static>(&mut self, filter: F) {
        self.target = Target::Filtered(Box::new(filter));
    }

    /// Sets target to filter out specific results in the browser
    pub fn set_target(&mut self, target: Target) {
        self.target = target;
    }

    /// Updates browser cache with files and directories from current path
    pub fn refresh_path(&mut self) -> Result<(), std::io::Error> {
        self.dir.clear();
        let dir = std::fs::read_dir(&self.path)?;
        for f in dir {
            if let Ok(f) = f {
                let path = f.path();
                match &self.target {
                    // skipping files the filter deems unwanted
                    Target::Filtered(f) if path.is_file() && f(&path) == false => continue,
                    Target::Directory if path.is_file() => continue,
                    _ => self.dir.push(path),
                }
            }
        }

        Ok(())
    }

    pub fn update(&mut self, message: BrowserOperation) -> Result<BrowsingResult, std::io::Error> {
        match message {
            BrowserOperation::MoveUp => {
                if let Some(_) = self.path.parent() {
                    self.path.pop();
                    self.refresh_path()?;
                    self.selected = None;
                    Ok(BrowsingResult::Pending)
                } else {
                    // this should never happen.
                    unreachable!()
                }
            }
            BrowserOperation::MoveInto(path) => {
                if path.is_dir() {
                    self.path = path;
                    self.refresh_path()?;
                    self.selected = None;
                    Ok(BrowsingResult::Pending)
                } else {
                    // this should never happen
                    unreachable!()
                }
            }
            BrowserOperation::Select(path) => {
                self.selected = path;
                Ok(BrowsingResult::Pending)
            }
            BrowserOperation::Cancel => {
                self.selected = None;
                Ok(BrowsingResult::Canceled)
            }
            BrowserOperation::Accept => match (&self.selected, &self.target) {
                (Some(p), Target::File) if p.is_file() => Ok(BrowsingResult::Done(p.clone())),
                (Some(p), Target::Filtered(_)) => Ok(BrowsingResult::Done(p.clone())),
                (_, Target::Directory) => Ok(BrowsingResult::Done(self.path.clone())),
                _ => Ok(BrowsingResult::Pending),
            },
        }
    }
    pub fn view_raw(&self) -> Container<BrowserOperation, Renderer> {
        // calculating file list widgets
        let mut file_list = Column::new();
        for x in self.dir.iter() {
            if let Some(name) = x.file_name().and_then(|x| x.to_str()) {
                let mut butt = button(
                    row![
                        text(name).width(Length::FillPortion(5)),
                        if x.is_file() {
                            text("File")
                        } else {
                            text("Directory")
                        }
                        .width(Length::FillPortion(1)),
                    ]
                    .width(Length::Fill),
                );
                if x.is_dir() {
                    butt = butt.on_press(BrowserOperation::MoveInto(x.clone()));
                } else {
                    match &self.selected {
                        Some(sel) if sel == x => {
                            butt = butt.on_press(BrowserOperation::Accept);
                        }
                        _ => {
                            butt = butt.on_press(BrowserOperation::Select(Some(x.clone())));
                        }
                    }
                }
                file_list = file_list.push(butt);
            }
        }
        // calculating the toolbar widgets
        let mut move_up = button("..");
        if self.path.parent().is_some() {
            move_up = move_up.on_press(BrowserOperation::MoveUp);
        }
        let accept = match (&self.target, &self.selected) {
            (Target::File, Some(p)) if p.is_file() => {
                button("Accept").on_press(BrowserOperation::Accept)
            }
            (Target::Filtered(filter), Some(p)) if filter(&p) => {
                button("Accept").on_press(BrowserOperation::Accept)
            }
            (Target::Directory, _) => button("Accept").on_press(BrowserOperation::Accept),
            _ => button("Accept"),
        };

        let ui = col![
            row![
                button("Cancel").on_press(BrowserOperation::Cancel),
                move_up,
                text(format!("Directory: {}", self.path.to_string_lossy())),
                horizontal_space(Length::Fill),
                accept
            ]
            .spacing(10)
            .height(Length::Shrink)
            .width(Length::Fill),
            scrollable(file_list).height(Length::Fill),
        ]
        .padding(2)
        .spacing(2);

        container(ui)
    }

    pub fn view(&self) -> Element<BrowserOperation, Renderer> {
        self.view_raw().into()
    }
}
