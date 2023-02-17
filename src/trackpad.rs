use iced::{event::Status, keyboard::Modifiers, Element, Length, Point, Rectangle, Size};
use iced_native::{
    image::Handle,
    layout::{Limits, Node},
    widget::Tree,
    Widget,
};

/// Widget that provides a trackpad-like functionality, allowing dragging and zooming messages to be processed on its surface
pub struct Trackpad<'a, Message> {
    handle: Handle,
    position: Point,
    zoom: f32,
    position_step: f32,
    zoom_step: f32,
    on_drag: Option<Box<dyn Fn(Point) -> Message + 'a>>,
    on_zoom: Option<Box<dyn Fn(f32) -> Message + 'a>>,
    on_view_zoom: Option<Box<dyn Fn(f32) -> Message + 'a>>,
    width: Length,
    height: Length,
    view_size: f32,
}

impl<'a, Message> Trackpad<'a, Message> {
    pub fn new<F>(handle: Handle, position: Point, on_drag: F) -> Self
    where
        F: Fn(Point) -> Message + 'a,
    {
        Self {
            handle,
            position,
            on_drag: Some(Box::new(on_drag)),
            on_zoom: None,
            on_view_zoom: None,
            width: Length::Fill,
            height: Length::Fill,
            view_size: 1.0,
            zoom: 1.0,
            position_step: 1.0,
            zoom_step: 1.0,
        }
    }
    pub fn with_zoom<F>(mut self, zoom: f32, on_change: F) -> Self
    where
        F: Fn(f32) -> Message + 'a,
    {
        self.on_zoom = Some(Box::new(on_change));
        self.zoom = zoom;
        self
    }
    pub fn zoom_step(mut self, step: f32) -> Self {
        self.zoom_step = step;
        self
    }
    pub fn position_step(mut self, step: f32) -> Self {
        self.position_step = step;
        self
    }
    pub fn with_view_zoom<F>(mut self, view_zoom: f32, on_change: F) -> Self
    where
        F: Fn(f32) -> Message + 'a,
    {
        self.view_size = view_zoom;
        self.on_view_zoom = Some(Box::new(on_change));
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
    fn layout(&self, _renderer: &Renderer, limits: &Limits) -> Node {
        Node::new(limits.max())
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
        let size = renderer.dimensions(&image);
        let bounds = layout.bounds();

        // Scaling the image to desired size
        let size = Size {
            width: size.width as f32 * self.view_size,
            height: size.height as f32 * self.view_size,
        };

        // Creating the drawing area, with centering if the size of the image is smaller than the area we have for drawing
        let area = Rectangle {
            x: bounds.x + (bounds.width - size.width).max(0.0) / 2.0,
            y: bounds.y + (bounds.height - size.height).max(0.0) / 2.0,
            width: size.width,
            height: size.height,
        };

        // rendering, with clipping if the image is larger than the area we have for drawing
        let render = move |r: &mut Renderer| r.draw(image, area);
        if size.width > bounds.width || size.height > bounds.height {
            renderer.with_layer(bounds, render);
        } else {
            render(renderer);
        }
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
                iced::mouse::Event::CursorMoved { position } => {
                    if local_state.tracking {
                        let delta = position - local_state.cursor;
                        let delta = if local_state.mods.shift() {
                            delta * self.position_step * 0.1
                        } else {
                            delta * self.position_step
                        };
                        let new_point = self.position - delta;
                        if let Some(f) = &self.on_drag {
                            let m = f(new_point);
                            shell.publish(m);
                        }
                        local_state.cursor = position;
                        Status::Captured
                    } else {
                        local_state.cursor = position;
                        Status::Ignored
                    }
                }
                iced::mouse::Event::ButtonPressed(_button) => {
                    if bounds.contains(cursor_position) {
                        local_state.tracking = true;
                        Status::Captured
                    } else {
                        Status::Ignored
                    }
                }
                iced::mouse::Event::ButtonReleased(_button) => {
                    if bounds.contains(cursor_position) {
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
                    let delta = match delta {
                        iced::mouse::ScrollDelta::Lines { y, .. } => y,
                        iced::mouse::ScrollDelta::Pixels { y, .. } => y,
                    } * self.zoom_step;
                    let delta = if local_state.mods.shift() {
                        delta * 0.1
                    } else {
                        delta
                    };
                    if local_state.mods.alt() {
                        if let Some(z) = &self.on_view_zoom {
                            let new_zoom = self.view_size + delta;
                            let m = z(new_zoom);
                            shell.publish(m);
                            Status::Captured
                        } else {
                            Status::Ignored
                        }
                    } else {
                        if let Some(z) = &self.on_zoom {
                            let new_zoom = self.zoom - delta;
                            let m = z(new_zoom);
                            shell.publish(m);
                            Status::Captured
                        } else {
                            Status::Ignored
                        }
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

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct State {
    tracking: bool,
    cursor: Point,
    mods: Modifiers,
}
