use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;

use iced::widget::{
    button, column as col, container, horizontal_space, row, scrollable, text, vertical_space, text_input,
};
use iced::{Alignment, Element, Length, Renderer};

use crate::data::{sanitize_file_name_ends, sanitize_dir_name};
use crate::status_bar::StatusBar;
use crate::style::Style;

pub struct Browser {
    path: PathBuf,
    selected: Option<PathBuf>,
    dir: Vec<PathBuf>,
    target: Target,
    roots: Vec<PathBuf>,
    favorites: Vec<PathBuf>,
    new_dir_name: Option<String>,
}

#[derive(Debug, Clone)]
pub enum BrowserOperation {
    MoveUp,
    MoveInto(PathBuf),
    Select(Option<PathBuf>),
    ToggleAddDirectory,
    CreateDirectory,
    UpdateDirectoryName(String),
    Favorite,
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
            roots: Browser::get_roots(),
            favorites: Self::get_favorites(),
            new_dir_name: None,
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
            roots: Browser::get_roots(),
            favorites: Self::get_favorites(),
            new_dir_name: None,
        }
    }

    fn get_favorites() -> Vec<PathBuf> {
        let favorites_path = dirs::config_dir().unwrap();
        let favorites_path = favorites_path.join(crate::data::PROJECT_NAME).join("favorites.list");
        if (!favorites_path.exists() || !favorites_path.is_file()) {
            return Vec::new();
        }
        let Ok(mut file) = File::open(favorites_path) else {
            return Vec::new();
        };
        let mut buff = String::new();
        let Ok(size) = file.read_to_string(&mut buff) else {
            return Vec::new();
        };

        let split = buff.split('\n');
        split.fold(Vec::new(), |mut v, s| { if s.len() > 0 { v.push(s.to_string().into()); } v })
    }

    fn save_favorite(&self) {
        let config_dir = dirs::config_dir().unwrap().join(crate::data::PROJECT_NAME);
        if !config_dir.exists() {
            std::fs::create_dir_all(config_dir.clone());
        }
        let favorites_path = config_dir.join("favorites.list");
        let Ok(mut file) = File::create(favorites_path) else {
            return ;
        };

        for path in self.favorites.iter() {
            if let Err(_) = file.write_fmt(format_args!("{}\n", path.to_str().unwrap())) {

            }
        }
    }

    fn get_roots() -> Vec<PathBuf> {
        let mut roots = Vec::new();
        if cfg!(windows) {
            for letter in 'A'..'Z' {
                let path = PathBuf::from(format!("{letter}:\\"));
                if path.exists() {
                    roots.push(path);
                }
            }
            roots.push(dirs::home_dir().unwrap());
        }
        else if cfg!(unix) {
            roots.push(PathBuf::from("/")) ;
            let mnt = PathBuf::from("/mnt");
            if let Ok(p) = std::fs::read_dir(mnt) {
                for dir in p {
                    let Ok(rdir) = dir else { continue };
                    let path = rdir.path();
                    if path.is_dir() {
                        roots.push(path);
                    }
                }
            }
            let media = PathBuf::from("/media");
            if let Ok(p) = std::fs::read_dir(media) {
                for dir in p {
                    let Ok(rdir) = dir else { continue };
                    let path = rdir.path();
                    if path.is_dir() {
                        roots.push(path);
                    }
                }
            }
        }
        roots.push(dirs::home_dir().unwrap());
        roots.push(dirs::picture_dir().unwrap());
        roots.push(dirs::download_dir().unwrap());
        roots
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

    pub fn update(&mut self, message: BrowserOperation, status: &mut StatusBar) -> Result<BrowsingResult, std::io::Error> {
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
            BrowserOperation::Favorite => if let Some(idx) = self.favorites.iter().position(|x| self.path.eq(x)) {
                self.favorites.remove(idx);
                self.save_favorite();
                Ok(BrowsingResult::Pending)
            }
            else {
                self.favorites.push(self.path.clone());
                self.save_favorite();
                Ok(BrowsingResult::Pending)
            },
            BrowserOperation::ToggleAddDirectory => if self.new_dir_name.is_none() {
                self.new_dir_name = Some("".into());
                Ok(BrowsingResult::Pending)
            }
            else {
                self.new_dir_name = None;
                Ok(BrowsingResult::Pending)
            },
            BrowserOperation::CreateDirectory => match self.new_dir_name.as_ref() {
                Some(name) => {
                    let p = self.path.join(sanitize_file_name_ends(name));
                    if let Err(e) = std::fs::create_dir_all(p) {
                        status.error(&format!("Couldn't create directory {}: {}", name, e));
                        return Ok(BrowsingResult::Pending);
                    }
                    self.refresh_path()?;
                    self.new_dir_name = None;
                    Ok(BrowsingResult::Pending)
                },
                None => unreachable!()
            },
            BrowserOperation::UpdateDirectoryName(name) => {
                self.new_dir_name = Some(sanitize_dir_name(name));
                Ok(BrowsingResult::Pending)
            }
        }
    }
    pub fn view(&self) -> Element<BrowserOperation, Renderer> {
        // calculating file list widgets
        let file_list = self
            .dir
            .iter()
            .filter_map(|x| {
                // Getting the name of the file
                if let Some(name) = x.file_name().and_then(|x| x.to_str()) {
                    Some((x, name))
                } else {
                    None
                }
            })
            .map(|(x, name)| {
                // turning it in to a row
                let r = row![
                    text(name).width(Length::FillPortion(5)),
                    if x.is_file() {
                        text("File")
                    } else {
                        text("Directory")
                    }
                    .width(Length::FillPortion(1)),
                ]
                .width(Length::Fill);
                (x, r)
            })
            .map(|(x, row)| {
                // each row is a button
                let b = button(row);
                (x, b)
            })
            .map(|(x, button)| {
                // depending on the type of the file, the button does different things
                if x.is_dir() {
                    button.on_press(BrowserOperation::MoveInto(x.clone()))
                } else {
                    match &self.selected {
                        Some(sel) if sel == x => button.on_press(BrowserOperation::Accept),
                        _ => button.on_press(BrowserOperation::Select(Some(x.clone()))),
                    }
                }
            })
            // fold it all up into a column
            .fold(col![].spacing(2), |col, butt| col.push(butt));

        let bottom = scrollable(file_list);
        let bottom = container(bottom)
            .style(Style::Margins)
            .padding(4)
            .width(Length::FillPortion(5))
            .height(Length::Fill);

        let quick_access_list = self
            .roots
            .iter()
            .map(|dir| {
                if let Some(name) = dir.file_name().and_then(|n| n.to_str()) {
                    (dir, name)
                }
                else {
                    if cfg!(windows) {
                        (dir, dir.to_str().unwrap())
                    }
                    else {
                        (dir, "root")
                    }
                }
            })
            .map(|( dir, name )| {
                button(text(name)).on_press(BrowserOperation::MoveInto(dir.clone()))
                    .width(Length::Fill)
            })
            .fold(col![].spacing(1), |col, butt| col.push(butt));

        let quick_access_list = container(quick_access_list)
            .style(Style::Margins)
            .padding(4)
            .width(Length::Fill);

        let favorites_list = self
            .favorites
            .iter()
            .map(|dir| {
                if let Some(name) = dir.file_name().and_then(|n| n.to_str()) {
                    (dir, name)
                }
                else {
                    if cfg!(windows) {
                        (dir, dir.to_str().unwrap())
                    }
                    else {
                        (dir, "root")
                    }
                }
            })
            .map(|( dir, name )| {
                button(text(name)).on_press(BrowserOperation::MoveInto(dir.clone()))
                    .width(Length::Fill)
            })
            .fold(col![].spacing(1), |col, butt| col.push(butt));


        let favorites_list = container(favorites_list)
            .style(Style::Margins)
            .padding(4)
            .width(Length::Fill);

        let side = col![
            text("Quick access"),
            quick_access_list,
            vertical_space(6),
            row![
                text("Favorites").width(Length::Fill),
                if self.favorites.contains(&self.path) {
                    button("-").on_press(BrowserOperation::Favorite)
                }
                else {
                    button("+").on_press(BrowserOperation::Favorite)
                }
            ].align_items(Alignment::End),
            vertical_space(2),
            favorites_list
        ]
            .width(Length::FillPortion(1))
            .padding(8);


        // calculating the toolbar widgets
        let move_up = if self.path.parent().is_some() {
            button("..").on_press(BrowserOperation::MoveUp)
        } else {
            button("..")
        };

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
        let (new_dir, making_directory) = match self.new_dir_name.as_ref() {
            Some(folder_name) => ( row![
                button("Cancel").on_press(BrowserOperation::ToggleAddDirectory),
                button("Make").on_press(BrowserOperation::CreateDirectory),
                text_input("Directory Name", folder_name, |x| BrowserOperation::UpdateDirectoryName(x))
            ], true),
            None => (row![button("Make Directory").on_press(BrowserOperation::ToggleAddDirectory)], false)
        };

        let top = if !making_directory {
            row![
                button("Cancel").on_press(BrowserOperation::Cancel),
                move_up,
                new_dir,
                text(format!("Directory: {}", self.path.to_string_lossy())),
                horizontal_space(Length::Fill),
                accept
            ]
        }
        else { new_dir }
        .align_items(Alignment::Center)
        .spacing(10);


        let top = container(top)
            .style(Style::Header)
            .padding(4)
            .width(Length::Fill)
            .height(Length::Shrink);

        let bottom = row![
            side,
            bottom
        ];

        col![top, bottom].into()
    }
}
