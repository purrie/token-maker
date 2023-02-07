use std::path::PathBuf;

use iced::widget::{button, column as col, container, row, scrollable, text, Column, Container};
use iced::{Element, Length, Renderer};

pub struct Browser {
    path: PathBuf,
    selected: Option<PathBuf>,
    dir: Vec<PathBuf>,
    filter: Option<String>,
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

impl Browser {
    pub fn set_filter(&mut self, filter: &str) {
        if filter.len() == 0 {
            self.filter = None;
        } else {
            self.filter = Some(String::from(filter));
        }
    }
    pub fn refresh_path(&mut self) -> Result<(), std::io::Error> {
        self.dir.clear();
        let dir = std::fs::read_dir(&self.path)?;
        for f in dir {
            if let Ok(f) = f {
                let path = f.path();
                if path.is_file() && self.filter.is_some() {
                    let f = self.filter.as_ref().unwrap();
                    if let Some(ext) = path.extension().and_then(|x| x.to_str()) {
                        if ext != f {
                            continue;
                        }
                    } else {
                        continue;
                    }
                }
                self.dir.push(path);
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
            BrowserOperation::Accept => {
                if let Some(s) = &self.selected {
                    if s.is_file() {
                        Ok(BrowsingResult::Done(s.clone()))
                    } else {
                        Ok(BrowsingResult::Pending)
                    }
                } else {
                    Ok(BrowsingResult::Pending)
                }
            }
        }
    }
    pub fn view_raw(&self) -> Container<BrowserOperation, Renderer> {
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
        let mut move_up = button("..");
        if self.path.parent().is_some() {
            move_up = move_up.on_press(BrowserOperation::MoveUp);
        }

        let ui = col![
            row![
                button("Cancel").on_press(BrowserOperation::Cancel),
                text("|"),
                move_up,
                text("|"),
                text(format!("Directory: {}", self.path.to_string_lossy()))
            ]
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

impl Default for Browser {
    fn default() -> Self {
        let path = match std::env::var("HOME") {
            Ok(o) => o.into(),
            Err(_) => "./".into(),
        };
        Self {
            path,
            selected: None,
            filter: None,
            dir: Vec::new(),
        }
    }
}
