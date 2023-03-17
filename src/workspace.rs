use std::sync::Arc;
use std::time::Duration;
use std::{fmt::Display, path::PathBuf};

use iced::{
    widget::{
        button, column as col, container, horizontal_space, image::Handle, row, scrollable, text,
        text_input,
    },
    Alignment, Command, ContentFit, Element, Length, Point, Renderer, Size, Subscription,
};

use iced_native::{image::Data, widget::PickList};
use serde::{Deserialize, Serialize};

use crate::modifier::{ModifierBox, ModifierMessage, ModifierOperation, ModifierTag};
use crate::widgets::Trackpad;
use crate::{
    data::{has_invalid_characters, sanitize_file_name, ProgramData, WorkspaceData},
    naming_convention::NamingConvention,
    persistence::{PersistentKey, PersistentValue},
};
use crate::{
    image::{image_arc_to_handle, image_to_handle, ImageFormat, ImageOperation, RgbaImage},
    style::Style,
};

/// Workspace serves purpose of providing tools to take the source image through series of operations to final result
pub struct Workspace {
    /// Source image to be used as a starting point
    source: Arc<RgbaImage>,
    /// Result of the latest rendering job
    cached_result: Handle,
    /// Image used for displaying this workspace's preview when offering to copy the image for a new workspace
    cached_preview: Handle,
    /// List of modifiers in order which they should be applied to the image
    modifiers: Vec<ModifierBox>,
    /// Currently selected modifier, used to choose which modifier should draw its UI
    selected_modifier: usize,

    /// Information about how the image is to be processed
    data: WorkspaceData,
    /// Flag specifies whatever there is active rendering job in process
    rendering: bool,
    /// Carrier for the width of the exported image, when it is a valid number, it is transformed into actual value
    width_carrier: String,
    /// Carrier for the height of the exported image, when it is a valid number, it is transformed into actual value
    height_carrier: String,
}

#[derive(Debug, Clone)]
pub enum WorkspaceMessage {
    /// Change to the name of the file the image is to be writen to
    OutputNameChange(String),
    /// Sets desired image format for the exported file
    SetFormat(ImageFormat),
    /// Sets width for the exported image. It uses string carrier to allow user input invalid input without breaking the input
    SetOutputWidth(String),
    /// Sets height for the exported image. It uses string carrier to allow user input invalid input without breaking the input
    SetOutputHeight(String),
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
    /// Resets the view zoom level
    ResetViewZoom,
}

impl Workspace {
    /// Creates a new workspace from provided image
    ///
    /// # Parameters
    /// `name`     - the name that should be used as default export name
    /// `source`   - the image to be used as a base
    /// `pdata`    - program data used for loading parameters for workspace and its modifiers
    /// `template` - setting to set up the workspace with defaults for specific template
    pub fn new(
        name: String,
        source: Arc<RgbaImage>,
        pdata: &ProgramData,
        template: WorkspaceTemplate,
    ) -> (Command<WorkspaceMessage>, Self) {
        let mut data = WorkspaceData {
            output: name,
            dirty: true,
            offset: Point {
                x: 0.0,
                y: source.height() as f32 / 3.0,
            },
            template,
            format: pdata
                .cache
                .get_copy(PersistentData::WorkspaceID, PersistentData::Format)
                .and_then(|x| {
                    if let PersistentValue::ImageFormat(x) = x {
                        Some(x)
                    } else {
                        None
                    }
                })
                .unwrap_or(ImageFormat::WebP),
            ..Default::default()
        };
        let mut modifiers = Vec::new();

        let command = match template {
            WorkspaceTemplate::None => Command::none(),
            WorkspaceTemplate::Token => {
                let (command, frame) = ModifierTag::Frame.make_box(pdata, &data);
                modifiers.push(frame);
                command.map(|x| WorkspaceMessage::ModifierMessage(0, x))
            }
            WorkspaceTemplate::Portrait => {
                data.export_size = Size {
                    width: source.width(),
                    height: source.height(),
                };
                data.offset = Point { x: 0.0, y: 0.0 };
                let (command, frame) = ModifierTag::Frame.make_box(pdata, &data);
                modifiers.push(frame);
                command.map(|x| WorkspaceMessage::ModifierMessage(0, x))
            }
        };

        let s = Self {
            width_carrier: data.export_size.width.to_string(),
            height_carrier: data.export_size.height.to_string(),
            data,
            modifiers,

            cached_preview: image_arc_to_handle(&source),
            cached_result: image_arc_to_handle(&source),
            source,
            selected_modifier: 0,
            rendering: false,
        };
        (command, s)
    }

    /// Workspace messaging update loop
    pub fn update(
        &mut self,
        msg: WorkspaceMessage,
        pdata: &mut ProgramData,
    ) -> Command<WorkspaceMessage> {
        match msg {
            WorkspaceMessage::OutputNameChange(s) => {
                if has_invalid_characters(&s) {
                    pdata
                        .status
                        .warning("Removed invalid characters from the name")
                }
                self.data.output = sanitize_file_name(s);
                self.update_modifiers(pdata)
            }
            WorkspaceMessage::SetOutputWidth(w) => {
                if let Ok(p) = w.parse::<u32>() {
                    self.data.export_size.width = p;
                    self.width_carrier = w;
                    self.data.dirty = true;
                    self.update_modifiers(pdata)
                } else {
                    if w.len() == 0 {
                        self.width_carrier = w;
                    }
                    Command::none()
                }
            }
            WorkspaceMessage::SetOutputHeight(h) => {
                if let Ok(p) = h.parse::<u32>() {
                    self.data.export_size.height = p;
                    self.height_carrier = h;
                    self.data.dirty = true;
                    self.update_modifiers(pdata)
                } else {
                    if h.len() == 0 {
                        self.height_carrier = h;
                    }
                    Command::none()
                }
            }
            WorkspaceMessage::Slide(x) => {
                self.data.offset = x;
                self.data.dirty = true;
                self.update_modifiers(pdata)
            }
            WorkspaceMessage::Zoom(x) => {
                self.data.zoom -= x;
                self.data.dirty = true;
                self.update_modifiers(pdata)
            }
            WorkspaceMessage::View(x) => {
                self.data.view += x;
                self.update_modifiers(pdata)
            }
            WorkspaceMessage::ResetViewZoom => {
                self.data.view = 1.0;
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
                    m.properties_update(message, pdata, &mut self.data)
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
            WorkspaceMessage::SetFormat(format) => {
                self.data.format = format;
                pdata
                    .cache
                    .set(PersistentData::WorkspaceID, PersistentData::Format, format);
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
                focus_point: Point {
                    x: self.source.width() as f32 * 0.5 - self.data.offset.x,
                    y: self.source.height() as f32 * 0.5 - self.data.offset.y,
                },
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

    /// Replaces the image
    pub fn set_source(
        &mut self,
        source: Arc<RgbaImage>,
        pdata: &ProgramData,
    ) -> Command<WorkspaceMessage> {
        match &self.data.template {
            WorkspaceTemplate::Portrait => {
                self.data.export_size = Size {
                    width: source.width(),
                    height: source.height(),
                }
            }
            _ => {}
        }
        self.cached_preview = image_arc_to_handle(&source);
        self.source = source;
        self.data.dirty = true;
        self.update_modifiers(pdata)
    }

    /// Returns the source image this workspace uses
    pub fn get_source(&self) -> &Arc<RgbaImage> {
        &self.source
    }

    /// Returns a preview image
    pub fn get_source_preview(&self) -> Handle {
        self.cached_preview.clone()
    }

    /// Returns the rendered image with all workspace operations applied
    pub fn get_preview(&self) -> Handle {
        self.cached_result.clone()
    }

    /// Returns the name this workspace will save the output to
    pub fn get_output_name(&self) -> &str {
        &self.data.output
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
            let img = Trackpad::new(img)
                .with_drag(self.data.offset, |mods, butt, point, delta| match butt {
                    iced::mouse::Button::Left => Some(WorkspaceMessage::Slide(if mods.shift() {
                        // decreasing the speed of movement for more granular control
                        Point {
                            x: point.x - delta.x * 0.9,
                            y: point.y - delta.y * 0.9,
                        }
                    } else {
                        point
                    })),
                    _ => None,
                })
                .with_click(|mods, button, _| match button {
                    iced::mouse::Button::Middle if mods.alt() => {
                        Some(WorkspaceMessage::ResetViewZoom)
                    }
                    _ => None,
                })
                .with_scroll(|mods, delta| {
                    let change = match delta {
                        iced::mouse::ScrollDelta::Lines { x: _, y } => y,
                        iced::mouse::ScrollDelta::Pixels { x: _, y } => y,
                    } * 0.1;
                    let change = if mods.shift() { change * 0.1 } else { change };
                    if mods.alt() {
                        Some(WorkspaceMessage::View(change))
                    } else {
                        Some(WorkspaceMessage::Zoom(change))
                    }
                })
                .width(self.data.export_size.width as f32 * self.data.view)
                .height(self.data.export_size.height as f32 * self.data.view)
                .with_content_fit(ContentFit::Contain);

            container(img)
        }
        .style(Style::Margins)
        .center_x()
        .center_y()
        .height(Length::FillPortion(3))
        .width(Length::Fill);

        col![self.toolbar(pdata), preview]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    /// Constructs the toolbar portion of the workspace UI
    fn toolbar<'a>(&'a self, pdata: &ProgramData) -> Element<'a, WorkspaceMessage, Renderer> {
        // main controls are mostly for customizing the workspace
        let main_controls = col![
            row![
                text_input("Output name", &self.data.output, |x| {
                    WorkspaceMessage::OutputNameChange(x)
                }),
                PickList::new(&ImageFormat::EXPORTABLE[..], Some(self.data.format), |x| {
                    WorkspaceMessage::SetFormat(x)
                }),
            ]
            .height(Length::Shrink)
            .align_items(Alignment::Center),
            row![
                text(&format!(
                    "Image size: {}x{}",
                    self.source.width(),
                    self.source.height()
                )),
                horizontal_space(Length::FillPortion(1)),
                text("Zoom: "),
                text_input("Zoom", &format!("{:.2}", self.data.zoom), |x| {
                    if let Ok(x) = x.parse() {
                        WorkspaceMessage::Zoom(x)
                    } else {
                        WorkspaceMessage::Zoom(self.data.zoom)
                    }
                })
                .width(Length::FillPortion(2)),
            ]
            .height(Length::Shrink)
            .spacing(5)
            .align_items(Alignment::Center),
            row![
                text("Offset: ")
                    .width(Length::FillPortion(1))
                    .vertical_alignment(iced::alignment::Vertical::Center),
                text_input("x", &format!("{:.2}", self.data.offset.x), |x| {
                    if let Ok(x) = x.parse() {
                        WorkspaceMessage::Slide(Point {
                            x: x,
                            y: self.data.offset.y,
                        })
                    } else {
                        WorkspaceMessage::Slide(self.data.offset)
                    }
                })
                .width(Length::FillPortion(2)),
                text("x"),
                text_input("y", &format!("{:.2}", self.data.offset.y), |y| {
                    if let Ok(y) = y.parse() {
                        WorkspaceMessage::Slide(Point {
                            y: y,
                            x: self.data.offset.x,
                        })
                    } else {
                        WorkspaceMessage::Slide(self.data.offset)
                    }
                })
                .width(Length::FillPortion(2)),
            ]
            .height(Length::Shrink)
            .spacing(5)
            .align_items(Alignment::Center),
            row![
                text("Size: ")
                    .width(Length::FillPortion(1))
                    .vertical_alignment(iced::alignment::Vertical::Center),
                text_input("Width", &self.width_carrier, |x| {
                    WorkspaceMessage::SetOutputWidth(x)
                })
                .width(Length::FillPortion(2)),
                text("x"),
                text_input("Height", &self.height_carrier, |x| {
                    WorkspaceMessage::SetOutputHeight(x)
                })
                .width(Length::FillPortion(2)),
            ]
            .height(Length::Shrink)
            .spacing(5)
            .align_items(Alignment::Center),
        ]
        .width(Length::Fill)
        .height(Length::Shrink)
        .spacing(5);

        // list of modifiers, to allow switching between them
        let modifier_list = self.modifiers.iter().enumerate().fold(
            // column for modifiers
            col![]
                .spacing(2)
                .padding(5)
                .height(Length::Shrink)
                .width(Length::Shrink),
            |col, (i, m)| {
                let r = row![
                    button("X").on_press(WorkspaceMessage::RemoveModifier(i)),
                    button(m.label()).on_press(WorkspaceMessage::SelectModifier(i)),
                    // TODO implement ability to reorder modifiers
                ]
                .spacing(2);
                col.push(r)
            },
        );

        let modifier_list = row![modifier_list, horizontal_space(8)];
        let modifier_list = scrollable(modifier_list).height(Length::Fill);
        let modifiers = PickList::new(&ModifierTag::ALL[..], None, WorkspaceMessage::AddModifier)
            .placeholder("Add new");

        let modifier_list = col![text("Active Modifiers:"), modifiers, modifier_list,].spacing(5);

        let main_controls = container(main_controls)
            .width(Length::Fill)
            .style(Style::Frame)
            .padding(5);
        let modifier_list = container(modifier_list)
            .width(Length::Shrink)
            .style(Style::Frame)
            .padding(5);

        // Switching between displaying just the regular controls and the UI for selected modifier
        let top = if let Some(selected) = self
            .modifiers
            .get(self.selected_modifier)
            .and_then(|x| x.properties_view(pdata, &self.data))
        {
            let modifier_properties =
                selected.map(move |x| WorkspaceMessage::ModifierMessage(self.selected_modifier, x));

            let modifier_properties = container(modifier_properties)
                .padding(5)
                .style(Style::Frame)
                .width(Length::Fill)
                .height(Length::Fill);

            row![
                modifier_list,
                col![main_controls, modifier_properties]
                    .spacing(2)
                    .width(Length::Fill)
            ]
        } else {
            row![modifier_list, main_controls]
        }
        .width(Length::Fill)
        .spacing(2)
        .padding(2);

        container(top)
            .style(Style::Margins)
            .height(Length::Fill)
            .into()
    }

    /// Constructs the path buffer pointing to the desired export path for the image
    fn construct_export_path(&self, pdata: &ProgramData) -> PathBuf {
        let mut path = pdata.output.clone();
        // Constructing the final name for the export
        let name = self
            .data
            .output
            .replace(
                NamingConvention::KEYWORD_PROJECT,
                &pdata.naming.project_name,
            )
            .replace('$', "");
        path.push(name);
        path.set_extension(self.data.format.to_string());
        path
    }

    /// Tests if the path set as export in this workspace already contains a file
    pub fn is_destructive_export(&self, pdata: &ProgramData) -> bool {
        self.construct_export_path(pdata).exists()
    }

    /// Exports latest preview image to drive
    pub fn export(&self, pdata: &ProgramData) {
        let path = self.construct_export_path(pdata);
        // Produce the image
        let Data::Rgba { width, height, pixels } = self.cached_result.data() else {
            panic!("doesn't work!");
        };
        image::save_buffer(path, pixels, *width, *height, image::ColorType::Rgba8).unwrap();
    }

    /// Tests whatever the workspace can save its result to drive
    pub fn can_save(&self) -> bool {
        // Can't save while the image is rendering
        if self.rendering {
            return false;
        }
        // To be valid, the name must have at least one alphanumeric character
        self.data.output.chars().any(|x| x.is_alphanumeric())
    }
}

/// Allows the program to define which default values should be used for the workspace and its modifiers
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum WorkspaceTemplate {
    /// None means there should be no specialization for the workspace
    #[default]
    None,
    Token,
    Portrait,
    // TODO Card,
    // TODO Standee,
}

impl WorkspaceTemplate {
    pub const ALL: [WorkspaceTemplate; 3] = [
        WorkspaceTemplate::None,
        WorkspaceTemplate::Token,
        WorkspaceTemplate::Portrait,
    ];

    pub fn get_default_file_name(&self) -> &'static str {
        match self {
            WorkspaceTemplate::None => "",
            WorkspaceTemplate::Token => "-token",
            WorkspaceTemplate::Portrait => "-portrait",
        }
    }
}

impl PersistentKey for WorkspaceTemplate {
    fn get_id(&self) -> &'static str {
        match self {
            WorkspaceTemplate::None => "none",
            WorkspaceTemplate::Token => "token",
            WorkspaceTemplate::Portrait => "portrait",
        }
    }
}

impl Display for WorkspaceTemplate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                WorkspaceTemplate::None => "None",
                WorkspaceTemplate::Token => "Token",
                WorkspaceTemplate::Portrait => "Portrait",
            }
        )
    }
}

enum PersistentData {
    WorkspaceID,
    Format,
}

impl PersistentKey for PersistentData {
    fn get_id(&self) -> &'static str {
        match self {
            PersistentData::WorkspaceID => "workspace",
            PersistentData::Format => "image-format",
        }
    }
}
