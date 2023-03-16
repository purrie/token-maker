use iced::{
    event::Status,
    keyboard::Modifiers,
    mouse::{Button, ScrollDelta},
    ContentFit, Element, Length, Point, Size, Vector,
};
use iced_native::{
    image::Handle,
    layout::{Limits, Node},
    widget::Tree,
    Widget,
};

/// Widget that provides a trackpad-like functionality, allowing dragging and zooming messages to be processed on its surface
///
/// The widget can be controlled with mouse cursor, pressing onto its surface enables drag message that shifts the position.
///
/// Optional features give ability to also send zoom messages on mouse wheel, change size of displayed image when holding alt.
/// Holding shift allows more gradual changes
pub struct Trackpad<'a, Message> {
    handle: Handle,
    position: Point,
    on_drag: Option<Box<dyn Fn(Modifiers, Button, Point, Vector) -> Option<Message> + 'a>>,
    on_click: Option<Box<dyn Fn(Modifiers, Button, Point) -> Option<Message> + 'a>>,
    on_scroll: Option<Box<dyn Fn(Modifiers, ScrollDelta) -> Option<Message> + 'a>>,
    width: Length,
    height: Length,
    content_fit: ContentFit,
}

impl<'a, Message> Trackpad<'a, Message> {
    /// Creates a new `Trackpad` with basic hold-and-drag messages,
    /// resulting value is a new position with delta of mouse movement applied
    pub fn new(handle: Handle) -> Self {
        Self {
            handle,
            position: Point::default(),
            on_drag: None,
            on_click: None,
            on_scroll: None,
            width: Length::Fill,
            height: Length::Fill,
            content_fit: ContentFit::ScaleDown,
        }
    }

    /// Enables drag functionality
    ///
    /// `on_drag` function is provided with current
    ///     modifier keys, mouse button, resulting position point
    ///     and delta between original and resulting points
    pub fn with_drag<F>(mut self, position: Point, on_drag: F) -> Self
    where
        F: Fn(Modifiers, Button, Point, Vector) -> Option<Message> + 'a,
    {
        self.on_drag = Some(Box::new(on_drag));
        self.position = position;
        self
    }

    /// Enables click functionality
    ///
    /// `on_click` is provided with
    ///     currently held modifiers, clicked button and cursor position in local space
    pub fn with_click<F>(mut self, on_click: F) -> Self
    where
        F: Fn(Modifiers, Button, Point) -> Option<Message> + 'a,
    {
        self.on_click = Some(Box::new(on_click));
        self
    }

    /// Enables scrollwheel functionality
    ///
    /// `on_scroll` function is provided with
    ///     currently held modifiers and scrollwheel delta
    pub fn with_scroll<F>(mut self, on_scroll: F) -> Self
    where
        F: Fn(Modifiers, ScrollDelta) -> Option<Message> + 'a,
    {
        self.on_scroll = Some(Box::new(on_scroll));
        self
    }

    /// Sets the strategy for scaling the image
    pub fn with_content_fit(mut self, content_fit: ContentFit) -> Self {
        self.content_fit = content_fit;
        self
    }

    /// Sets the width for the widget
    pub fn width<L: Into<Length>>(mut self, width: L) -> Self {
        self.width = width.into();
        self
    }

    /// Sets the height for the widget
    pub fn height<L: Into<Length>>(mut self, height: L) -> Self {
        self.height = height.into();
        self
    }
}

impl<'a, Message, Renderer> Widget<Message, Renderer> for Trackpad<'a, Message>
where
    Renderer: iced_native::image::Renderer<Handle = Handle>,
{
    fn width(&self) -> iced::Length {
        self.width
    }

    fn height(&self) -> iced::Length {
        self.height
    }

    fn tag(&self) -> iced_native::widget::tree::Tag {
        iced_native::widget::tree::Tag::of::<State>()
    }

    fn state(&self) -> iced_native::widget::tree::State {
        iced_native::widget::tree::State::new(State::default())
    }

    fn layout(&self, renderer: &Renderer, limits: &Limits) -> Node {
        let image_size = renderer.dimensions(&self.handle);
        let image_size = Size {
            width: image_size.width as f32,
            height: image_size.height as f32,
        };

        let size = self.content_fit.fit(
            image_size,
            limits.width(self.width).height(self.height).max(),
        );
        Node::new(size)
    }

    fn draw(
        &self,
        _state: &Tree,
        renderer: &mut Renderer,
        _theme: &<Renderer as iced_native::Renderer>::Theme,
        _style: &iced_native::renderer::Style,
        layout: iced_native::Layout<'_>,
        _cursor_position: Point,
        _viewport: &iced::Rectangle,
    ) {
        let image = self.handle.clone();
        let bounds = layout.bounds();
        renderer.draw(image, bounds);
    }
    fn on_event(
        &mut self,
        state: &mut Tree,
        event: iced::Event,
        layout: iced_native::Layout<'_>,
        cursor_position: Point,
        _renderer: &Renderer,
        _clipboard: &mut dyn iced_native::Clipboard,
        shell: &mut iced_native::Shell<'_, Message>,
    ) -> Status {
        let local_state = state.state.downcast_mut::<State>();
        let bounds = layout.bounds();

        match event {
            iced::Event::Keyboard(key) => match key {
                iced::keyboard::Event::ModifiersChanged(mods) => {
                    local_state.mods = mods;
                    Status::Ignored
                }
                _ => Status::Ignored,
            },

            iced::Event::Mouse(mouse) => match mouse {
                iced::mouse::Event::CursorMoved { position } => match &self.on_drag {
                    Some(on_drag) if local_state.tracking => {
                        let delta = position - local_state.cursor;
                        let new_point = self.position - delta;
                        let m = (on_drag)(local_state.mods, local_state.button, new_point, delta);
                        let Some(m) = m else {
                            return Status::Ignored;
                        };
                        shell.publish(m);
                        local_state.cursor = position;
                        Status::Captured
                    }
                    _ => {
                        local_state.cursor = position;
                        Status::Ignored
                    }
                },

                iced::mouse::Event::ButtonPressed(button) => {
                    if bounds.contains(cursor_position) {
                        local_state.tracking = true;
                        local_state.button = button;

                        if let Some(on_click) = &self.on_click {
                            let local_cursor_position = Point {
                                x: cursor_position.x - bounds.x,
                                y: cursor_position.y - bounds.y,
                            };
                            if let Some(m) =
                                on_click(local_state.mods, button, local_cursor_position)
                            {
                                shell.publish(m);
                            }
                        }
                        Status::Captured
                    } else {
                        Status::Ignored
                    }
                }

                iced::mouse::Event::ButtonReleased(_button) => {
                    if local_state.tracking {
                        local_state.tracking = false;
                        Status::Captured
                    } else {
                        Status::Ignored
                    }
                }

                iced::mouse::Event::WheelScrolled { delta } => {
                    if bounds.contains(cursor_position) == false {
                        return Status::Ignored;
                    }
                    if let Some(scroll) = &self.on_scroll {
                        let m = scroll(local_state.mods, delta);
                        let Some(m) = m else {
                            return Status::Ignored;
                        };
                        shell.publish(m);
                        Status::Captured
                    } else {
                        Status::Ignored
                    }
                }
                _ => Status::Ignored,
            },
            _ => Status::Ignored,
        }
    }
}

impl<'a, Message: 'a, Renderer> From<Trackpad<'a, Message>> for Element<'a, Message, Renderer>
where
    Renderer: iced_native::image::Renderer<Handle = Handle>,
{
    fn from(value: Trackpad<'a, Message>) -> Element<'a, Message, Renderer> {
        Self::new(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct State {
    tracking: bool,
    cursor: Point,
    mods: Modifiers,
    button: Button,
}

impl Default for State {
    fn default() -> Self {
        Self {
            tracking: Default::default(),
            cursor: Default::default(),
            mods: Default::default(),
            button: Button::Left,
        }
    }
}
