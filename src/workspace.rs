use std::sync::Arc;

use iced::widget::image::Handle;
use iced::widget::{column as col, container, image as rndr_image, row, text_input, Row};
use iced::{Command, Element, Length, Renderer};
use image::DynamicImage;

use crate::frame::{Frame, FrameMessage};
use crate::image::{image_to_handle, RgbaImage};
// use crate::modifier::{Frame, ModifierBox, ModifierMessage};

pub struct Workspace {
    source: Arc<RgbaImage>,
    cached_result: Handle,
    output: String,

    // modifiers: Vec<ModifierBox>,
    // selected_modifier: usize,
    frame: Frame,
    renderer: bool,
}

#[derive(Debug, Clone)]
pub enum WorkspaceMessage {
    OutputChange(String),
    FrameMessage(FrameMessage),
    // ModifierMessage(usize, ModifierMessage),
    // SelectModifier(usize),
    RenderResult(Handle),
}

pub type IndexedWorkspaceMessage = (usize, WorkspaceMessage);

impl Workspace {
    pub fn new(name: String, source: DynamicImage) -> Self {
        let frame = Frame::load("ring").unwrap();
        let source = source.into_rgba8();
        let source = Arc::new(source);
        Self {
            frame,
            cached_result: image_to_handle(source.as_ref().clone()),
            source,
            output: name,
            // selected_modifier: 0,
            renderer: false,
        }
    }
    pub fn view<'a>(&'a self) -> Element<'a, WorkspaceMessage, Renderer> {
        let img = self.get_output();
        let img = rndr_image(img);
        let size = self.frame.expected_size();

        let preview = container(
            img.height(Length::Units(size.x as u16))
                .width(Length::Units(size.y as u16)),
        )
        .center_x()
        .center_y()
        .height(Length::FillPortion(5))
        .width(Length::Fill);

        col![self.toolbar().height(Length::FillPortion(1)), preview,]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
    pub fn update(&mut self, msg: WorkspaceMessage) -> Command<WorkspaceMessage> {
        let mut render = false;
        let mut cmd = match msg {
            WorkspaceMessage::OutputChange(s) => {
                self.output = s;
                Command::none()
            }
            WorkspaceMessage::RenderResult(r) => {
                self.cached_result = r;
                self.renderer = false;
                Command::none()
            }
            WorkspaceMessage::FrameMessage(x) => {
                let cmd = self.frame.properties_update(x);
                if self.frame.is_dirty() {
                    render = true;
                }
                cmd.map(|x| WorkspaceMessage::FrameMessage(x))
            }
        };
        if render && self.renderer == false {
            let render = self.produce_render();
            cmd = Command::batch([cmd, render]);
            self.renderer = true;
        }
        cmd
    }

    fn produce_render(&mut self) -> Command<WorkspaceMessage> {
        let render = self.frame.prepare_image(self.source.clone());
        // here goes modifiers
        let render = self.frame.finalize_image(render);
        self.frame.clean();
        Command::perform(render, WorkspaceMessage::RenderResult)
    }

    pub fn get_output(&self) -> Handle {
        self.cached_result.clone()
    }

    fn toolbar<'a>(&'a self) -> Row<'a, WorkspaceMessage, Renderer> {
        let frame = self
            .frame
            .properties_view()
            .map(WorkspaceMessage::FrameMessage);
        let frame = container(frame).width(Length::Fill).height(Length::Fill);
        let r = row![col![
            text_input("Output name", &self.output, |x| {
                WorkspaceMessage::OutputChange(x)
            }),
            frame,
        ]
        .width(Length::FillPortion(1)),];
        r
    }
}
