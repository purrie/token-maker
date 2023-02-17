use std::sync::Arc;

use iced::widget::image::Handle;
use iced::widget::{column as col, container, row, text_input, Row};
use iced::{Command, Element, Length, Point, Renderer};
use image::DynamicImage;

use crate::frame::{Frame, FrameMessage};
use crate::image::{image_to_handle, RgbaImage};
use crate::trackpad::Trackpad;
// use crate::modifier::{Frame, ModifierBox, ModifierMessage};

pub struct Workspace {
    source: Arc<RgbaImage>,
    cached_result: Handle,
    output: String,

    // modifiers: Vec<ModifierBox>,
    // selected_modifier: usize,
    frame: Frame,
    renderer: bool,
    view: f32,
}

#[derive(Debug, Clone)]
pub enum WorkspaceMessage {
    OutputChange(String),
    FrameMessage(FrameMessage),
    // ModifierMessage(usize, ModifierMessage),
    // SelectModifier(usize),
    RenderResult(Handle),
    Slide(Point),
    Zoom(f32),
    View(f32),
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
            view: 1.0,
        }
    }
    pub fn view<'a>(&'a self) -> Element<'a, WorkspaceMessage, Renderer> {
        let img = self.get_output();
        let img = Trackpad::new(img, self.frame.get_offset(), |x| WorkspaceMessage::Slide(x))
            .with_zoom(self.frame.get_zoom(), |x| WorkspaceMessage::Zoom(x))
            .zoom_step(0.1)
            .with_view_zoom(self.view, |x| WorkspaceMessage::View(x))
            .position_step(2.0);
        // let size = self.frame.expected_size();

        let preview = container(img)
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
            WorkspaceMessage::Slide(x) => {
                self.frame.set_offset(x);
                render = true;
                Command::none()
            }
            WorkspaceMessage::Zoom(x) => {
                self.frame.set_zoom(x);
                render = true;
                Command::none()
            }
            WorkspaceMessage::View(x) => {
                self.view = x;
                Command::none()
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
