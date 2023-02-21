use std::sync::Arc;
use std::time::Duration;

use iced::{
    widget::{
        button, column as col, container, image::Handle, pick_list, row, text, text_input, Row,
    },
    Command, Element, Length, Point, Renderer, Subscription,
};

use image::DynamicImage;

use crate::data::{ProgramData, WorkspaceData};
use crate::image::{image_to_handle, ImageOperation, RgbaImage};
use crate::modifier::{ModifierBox, ModifierMessage, ModifierOperation, ModifierTag};
use crate::trackpad::Trackpad;

/// Workspace serves purpose of providing tools to take the source image through series of operations to final result
pub struct Workspace {
    /// Source image to be used as a starting point
    source: Arc<RgbaImage>,
    /// Result of the latest rendering job
    cached_result: Handle,
    /// List of modifiers in order which they should be applied to the image
    modifiers: Vec<ModifierBox>,
    /// Currently selected modifier, used to choose which modifier should draw its UI
    selected_modifier: usize,

    /// Information about how the image is to be processed
    data: WorkspaceData,
    /// Flag specifies whatever there is active rendering job in process
    rendering: bool,
}

#[derive(Debug, Clone)]
pub enum WorkspaceMessage {
    /// Change to the name of the file the image is to be writen to
    OutputNameChange(String),
    /// Request to add a specific modifier type
    AddModifier(ModifierTag),
    /// Request to remove a modifier on specified index
    RemoveModifier(usize),
    /// Modifier has received a message (index, message)
    ModifierMessage(usize, ModifierMessage),
    /// Changes which modifier is selected
    SelectModifier(usize),
    /// Prompt new render job
    Render,
    /// Rendering has completed with a result
    RenderResult(Handle),
    /// Change to image offset
    Slide(Point),
    /// Change to image size and how zoomed it is
    Zoom(f32),
    /// Change to size of the widget rendering the image
    View(f32),
}

pub type IndexedWorkspaceMessage = (usize, WorkspaceMessage);

impl Workspace {
    /// Creates a new workspace from provided image
    ///
    /// # Parameters
    ///
    /// `name`   - the name that should be used as default export name
    /// `source` - the image to be used as a base
    pub fn new(name: String, source: DynamicImage) -> Self {
        let source = source.into_rgba8();
        Self {
            cached_result: image_to_handle(source.clone()),
            data: WorkspaceData {
                output: name,
                dirty: true,
                offset: Point {
                    x: source.width() as f32 / 2.0,
                    y: source.height() as f32 / 10.0,
                },
                ..Default::default()
            },
            source: Arc::new(source),
            modifiers: Vec::new(),
            selected_modifier: 0,
            rendering: false,
        }
    }

    /// Workspace messaging update loop
    pub fn update(
        &mut self,
        msg: WorkspaceMessage,
        pdata: &ProgramData,
    ) -> Command<WorkspaceMessage> {
        match msg {
            WorkspaceMessage::OutputNameChange(s) => {
                self.data.output = s;
                self.data.dirty = true;
                self.update_modifiers(pdata)
            }
            WorkspaceMessage::Slide(x) => {
                self.data.offset = x;
                self.data.dirty = true;
                self.update_modifiers(pdata)
            }
            WorkspaceMessage::Zoom(x) => {
                self.data.zoom = x;
                self.data.dirty = true;
                self.update_modifiers(pdata)
            }
            WorkspaceMessage::View(x) => {
                self.data.view = x;
                self.update_modifiers(pdata)
            }
            WorkspaceMessage::RenderResult(r) => {
                self.cached_result = r;
                self.rendering = false;
                Command::none()
            }
            WorkspaceMessage::Render => self.produce_render(pdata),
            WorkspaceMessage::ModifierMessage(index, message) => {
                if let Some(m) = self.modifiers.get_mut(index) {
                    m.properties_update(message, pdata, &self.data)
                        .map(move |x| WorkspaceMessage::ModifierMessage(index, x))
                } else {
                    Command::none()
                }
            }
            WorkspaceMessage::RemoveModifier(i) => {
                if i < self.modifiers.len() {
                    self.modifiers.remove(i);
                    self.data.dirty = true;
                }
                Command::none()
            }
            WorkspaceMessage::AddModifier(m) => {
                let (command, modifier) = m.make_box(pdata, &self.data);
                let index = self.modifiers.len();
                self.modifiers.push(modifier);
                self.selected_modifier = index;
                command.map(move |x| WorkspaceMessage::ModifierMessage(index, x))
            }
            WorkspaceMessage::SelectModifier(index) => {
                self.selected_modifier = index;
                Command::none()
            }
        }
    }

    /// Sends update signal to the modifiers
    ///
    /// Purpose of this function is to let modifiers update their internal state or schedule jobs when workspace data has changed if they depend on it
    fn update_modifiers(&mut self, pdata: &ProgramData) -> Command<WorkspaceMessage> {
        let coms = self
            .modifiers
            .iter_mut()
            .enumerate()
            .fold(Vec::new(), |mut v, (i, m)| {
                v.push(
                    m.workspace_update(pdata, &self.data)
                        .map(move |x| WorkspaceMessage::ModifierMessage(i, x)),
                );
                v
            });
        Command::batch(coms)
    }

    /// Main rendering job builder
    ///
    /// The function constructs and schedules a rendering job for the image
    /// It will do so only if there is no rendering job in progress and either workspace data or modifiers have dirty flag enabled
    fn produce_render(&mut self, pdata: &ProgramData) -> Command<WorkspaceMessage> {
        if self.rendering {
            return Command::none();
        }
        if self.data.dirty || self.modifiers.iter().any(|x| x.is_dirty()) {
            self.data.dirty = false;
            self.rendering = true;

            let mut ops = vec![ImageOperation::Begin {
                image: self.source.clone(),
                resolution: self.data.export_size,
                offset: self.data.offset,
                size: self.data.zoom,
            }];

            self.modifiers.iter_mut().for_each(|x| {
                match x.get_image_operation(pdata, &self.data) {
                    ModifierOperation::None => {}
                    ModifierOperation::Single(o) => ops.push(o),
                    ModifierOperation::Double(first, second) => {
                        ops.push(first);
                        ops.push(second);
                    }
                    ModifierOperation::Multiple(mut o) => ops.append(&mut o),
                }
            });

            Command::perform(
                async move {
                    let start = ops.remove(0);
                    let mut img = start.begin().await;
                    for op in ops {
                        img = op.perform(img).await;
                    }
                    image_to_handle(img)
                },
                |x| WorkspaceMessage::RenderResult(x),
            )
        } else {
            Command::none()
        }
    }

    /// Creates a schedule for rendering jobs
    pub fn subscribtion(&self) -> Subscription<WorkspaceMessage> {
        iced::time::every(Duration::from_secs_f32(0.05)).map(|_| WorkspaceMessage::Render)
    }

    /// Returns a clone of the latest rendering result
    pub fn get_output(&self) -> Handle {
        self.cached_result.clone()
    }

    /// Workspace UI
    pub fn view<'a>(&'a self, pdata: &ProgramData) -> Element<'a, WorkspaceMessage, Renderer> {
        let img = self.get_output();
        let selected_mod = self.selected_modifier;
        // handles switching between regular image preview and controls, and whatever the modifier needs to render at the time
        let preview = if let Some(wid) = self.modifiers.get(selected_mod).and_then(|x| {
            if x.wants_main_view(pdata, &self.data) {
                Some(x)
            } else {
                None
            }
        }) {
            container(
                wid.main_view(img, pdata, &self.data)
                    .map(move |x| WorkspaceMessage::ModifierMessage(selected_mod, x)),
            )
        } else {
            let img = Trackpad::new(img, self.data.offset, |x| WorkspaceMessage::Slide(x))
                .with_zoom(self.data.zoom, |x| WorkspaceMessage::Zoom(x))
                .zoom_step(0.1)
                .with_view_zoom(self.data.view, |x| WorkspaceMessage::View(x))
                .position_step(2.0);
            container(img)
        }
        .center_x()
        .center_y()
        .height(Length::FillPortion(5))
        .width(Length::Fill);

        col![self.toolbar(pdata).height(Length::FillPortion(1)), preview,]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    /// Constructs the toolbar portion of the workspace UI
    fn toolbar<'a>(&'a self, pdata: &ProgramData) -> Row<'a, WorkspaceMessage, Renderer> {
        // main controls are mostly for customizing the workspace
        let main_controls = col![
            text_input("Output name", &self.data.output, |x| {
                WorkspaceMessage::OutputNameChange(x)
            }),
            pick_list(&ModifierTag::ALL[..], None, WorkspaceMessage::AddModifier)
                .placeholder("Add modifier"),
        ]
        .width(Length::FillPortion(1))
        .height(Length::Fill);
        // TODO implement ability to reorder modifiers
        // list of modifiers, to allow switching between them
        let modifier_list = self.modifiers.iter().enumerate().fold(
            // column for modifiers
            col![text("Active Modifiers:")]
                .spacing(2)
                .height(Length::Fill)
                .width(Length::FillPortion(1)),
            |col, (i, m)| {
                let r = row![
                    button(m.label()).on_press(WorkspaceMessage::SelectModifier(i)),
                    button("X").on_press(WorkspaceMessage::RemoveModifier(i)),
                ]
                .spacing(2);
                col.push(r)
            },
        );
        let mut r = row![main_controls, modifier_list];
        // show modifier UI
        if let Some(selected) = self
            .modifiers
            .get(self.selected_modifier)
            .and_then(|x| x.properties_view(pdata, &self.data))
        {
            let selected =
                selected.map(move |x| WorkspaceMessage::ModifierMessage(self.selected_modifier, x));
            let selected = container(selected)
                .width(Length::FillPortion(2))
                .height(Length::Fill);
            r = r.push(selected);
        }
        r.spacing(5)
    }
}
