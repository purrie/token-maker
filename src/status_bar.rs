use iced::{
    widget::{row, text},
    Element, Length, Renderer,
};

pub struct StatusBar {
    current_line: Status,
}

enum Status {
    None,
    Log(String),
    Warning(String),
    Error(String),
}

impl StatusBar {
    pub fn new() -> Self {
        Self {
            current_line: Status::None,
        }
    }

    pub fn log(&mut self, text: &str) {
        self.current_line = Status::Log(text.to_string());
    }
    pub fn error(&mut self, text: &str) {
        self.current_line = Status::Error(text.to_string());
    }
    pub fn warning(&mut self, text: &str) {
        self.current_line = Status::Warning(text.to_string());
    }

    pub fn view(&self) -> Element<'_, (), Renderer> {
        let t = match &self.current_line {
            Status::None => "",
            Status::Log(l) => l,
            Status::Warning(w) => w,
            Status::Error(e) => e,
        };
        row![text(t)]
            .width(Length::Fill)
            .height(Length::Shrink)
            .into()
    }
}
