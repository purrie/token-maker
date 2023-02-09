use iced::widget::image::Handle;
use iced::widget::{
    button, column as col, container, image as rndr_image, row, scrollable, text, text_input,
    Column, Row,
};
use iced::{Command, Element, Length, Renderer};
use image::DynamicImage;

use crate::data::OutputOptions;
use crate::modifier::{Frame, ModifierBox, ModifierMessage};

pub struct Workspace {
    source: DynamicImage,
    cached_result: Handle,
    output: String,
    output_options: OutputOptions,

    modifiers: Vec<ModifierBox>,
    selected_modifier: usize,
}

#[derive(Debug, Clone)]
pub enum WorkspaceMessage {
    OutputChange(String),
    OutputSize(String),
    AddFrame,
    ModifierMessage(usize, ModifierMessage),
    SelectModifier(usize),
}

pub type IndexedWorkspaceMessage = (usize, WorkspaceMessage);

impl Workspace {
    pub fn new(name: String, source: DynamicImage) -> Self {
        Self {
            cached_result: image_to_handle(&source),
            source,
            output: name,
            output_options: OutputOptions::default(),
            modifiers: Vec::new(),
            selected_modifier: 0,
        }
    }
    pub fn view<'a>(&'a self) -> Element<'a, WorkspaceMessage, Renderer> {
        let img = self.get_output();
        let img = rndr_image(img);

        col![
            self.toolbar().height(Length::FillPortion(1)),
            img.height(Length::FillPortion(5)),
        ]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
    pub fn update(&mut self, msg: WorkspaceMessage) -> Command<WorkspaceMessage> {
        match msg {
            WorkspaceMessage::OutputChange(s) => {
                self.output = s;
                Command::none()
            }
            WorkspaceMessage::AddFrame => {
                self.modifiers
                    .push(Frame::load("data/frames/ring.webp").unwrap().into());
                self.update_image();
                Command::none()
            }
            WorkspaceMessage::OutputSize(size) => {
                if let Ok(s) = size.parse::<u32>() {
                    self.output_options.size.x = s;
                    self.output_options.size.y = s;
                    self.update_image();
                }
                Command::none()
            }
            WorkspaceMessage::ModifierMessage(i, mess) => {
                if let Some(x) = self.modifiers.get_mut(i) {
                    let o = x
                        .properties_update(mess)
                        .map(move |x| WorkspaceMessage::ModifierMessage(i, x));
                    // for now, we assume that any modifier message changes modifiers and need to update the image.
                    // Change that to a separate return when that when it is no longer the case.
                    self.update_image();
                    o
                } else {
                    Command::none()
                }
            }
            WorkspaceMessage::SelectModifier(i) => {
                self.selected_modifier = i.min(self.modifiers.len());
                Command::none()
            }
        }
    }

    pub fn get_output(&self) -> Handle {
        self.cached_result.clone()
    }
    fn update_image(&mut self) {
        self.cached_result = image_to_handle(
            &self
                .modifiers
                .iter()
                .fold(self.source.clone(), |img, modif| {
                    modif.modify(img, &self.output_options)
                }),
        );
    }
    fn toolbar<'a>(&'a self) -> Row<'a, WorkspaceMessage, Renderer> {
        let mut r = row![
            col![
                text_input("Output name", &self.output, |x| {
                    WorkspaceMessage::OutputChange(x)
                }),
                text_input("", &self.output_options.size.x.to_string(), |x| {
                    WorkspaceMessage::OutputSize(x)
                }),
            ]
            .width(Length::FillPortion(1)),
            row![
                button("Add Frame")
                    .on_press(WorkspaceMessage::AddFrame)
                    .width(Length::Shrink),
                col![
                    text("Modifiers"),
                    scrollable(Column::with_children(
                        self.modifiers
                            .iter()
                            .enumerate()
                            .fold(Vec::new(), |mut v, (i, x)| {
                                v.push(
                                    button(x.label())
                                        .on_press(WorkspaceMessage::SelectModifier(i))
                                        .into(),
                                );
                                v
                            })
                    ))
                    .height(Length::Fill)
                ]
                .height(Length::Fill)
                .width(Length::FillPortion(1))
            ]
            .height(Length::Fill)
            .width(Length::FillPortion(1))
        ];
        if let Some(x) = self
            .modifiers
            .get(self.selected_modifier)
            .and_then(|x| x.properties_view())
        {
            let mod_props = x.map(|x| WorkspaceMessage::ModifierMessage(self.selected_modifier, x));
            r = r.push(
                container(mod_props)
                    .height(Length::Fill)
                    .width(Length::FillPortion(1)),
            );
        }
        r
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
