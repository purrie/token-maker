use iced::widget::image::Handle;
use iced::widget::{column as col, image as rndr_image, row, text_input, Row};
use iced::{Command, Element, Length, Renderer};
use image::DynamicImage;

pub struct Workspace {
    /// Index of the workspace, it must not change
    index: usize,
    source: DynamicImage,
    cached_result: Handle,
    output: String,
}

#[derive(Debug, Clone)]
pub enum WorkspaceMessage {
    OutputChange(String),
}

pub type IndexedWorkspaceMessage = (usize, WorkspaceMessage);

impl Workspace {
    pub fn new(index: usize, name: String, source: DynamicImage) -> Self {
        Self {
            cached_result: image_to_handle(&source),
            index,
            source,
            output: name,
        }
    }
    pub fn view<'a>(&'a self) -> Element<'a, IndexedWorkspaceMessage, Renderer> {
        let img = self.get_output();
        let img = rndr_image(img);

        col![self.toolbar(), img,]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
    pub fn update(&mut self, msg: WorkspaceMessage) -> Command<WorkspaceMessage> {
        match msg {
            WorkspaceMessage::OutputChange(s) => self.output = s,
        }
        Command::none()
    }

    pub fn get_output(&self) -> Handle {
        self.cached_result.clone()
    }
    fn toolbar<'a>(&'a self) -> Row<'a, IndexedWorkspaceMessage, Renderer> {
        row![text_input("Output name", &self.output, |x| {
            (self.index, WorkspaceMessage::OutputChange(x))
        })]
    }
}

pub fn image_to_handle(image: &DynamicImage) -> Handle {
    let img = image.as_rgba8().unwrap();
    Handle::from_pixels(
        img.width(),
        img.height(),
        img.pixels().fold(Vec::new(), |mut v, x| {
            x.0.iter().for_each(|x| v.push(*x));
            v
        }),
    )
}
