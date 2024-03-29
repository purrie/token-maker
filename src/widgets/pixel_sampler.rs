use iced::{event::Status, ContentFit, Element, Length, Point, Rectangle, Size, Vector};
use iced_native::{
    image::Handle,
    layout::{Limits, Node},
    renderer::Style,
    widget::Tree,
    Layout, Widget,
};

pub struct PixelSampler<'a, Message> {
    handle: Handle,
    on_click: Box<dyn Fn(Vector<u32>) -> Message + 'a>,
    width: Length,
    height: Length,
}

impl<'a, Message> PixelSampler<'a, Message> {
    pub fn new<F: Fn(Vector<u32>) -> Message + 'a>(image: Handle, on_click: F) -> Self {
        Self {
            handle: image,
            on_click: Box::new(on_click),
            width: Length::Fill,
            height: Length::Fill,
        }
    }
}

impl<'a, Message, Renderer> Widget<Message, Renderer> for PixelSampler<'a, Message>
where
    Renderer: iced_native::image::Renderer<Handle = Handle>,
{
    fn width(&self) -> Length {
        self.width
    }

    fn height(&self) -> Length {
        self.height
    }

    fn layout(&self, renderer: &Renderer, limits: &Limits) -> Node {
        let size = renderer.dimensions(&self.handle);
        let size = Size {
            width: size.width as f32,
            height: size.height as f32,
        };
        Node::new(limits.max().min(size).into())
    }

    fn draw(
        &self,
        _state: &Tree,
        renderer: &mut Renderer,
        _theme: &Renderer::Theme,
        _style: &Style,
        layout: Layout<'_>,
        _cursor_position: Point,
        _viewport: &Rectangle,
    ) {
        let image = self.handle.clone();
        let bounds = layout.bounds();

        let bounds = preview_rect(&image, renderer, &bounds);

        renderer.draw(image, bounds);
    }
    fn on_event(
        &mut self,
        _state: &mut Tree,
        event: iced::Event,
        layout: Layout<'_>,
        cursor_position: Point,
        renderer: &Renderer,
        _clipboard: &mut dyn iced_native::Clipboard,
        shell: &mut iced_native::Shell<'_, Message>,
    ) -> Status {
        match event {
            iced::Event::Mouse(mouse) => match mouse {
                iced::mouse::Event::ButtonPressed(_) => {
                    let bounds = layout.bounds();
                    let bounds = preview_rect(&self.handle, renderer, &bounds);
                    if bounds.contains(cursor_position) {
                        let mut pos = Vector {
                            x: cursor_position.x - bounds.x,
                            y: cursor_position.y - bounds.y,
                        };
                        let pic_size = renderer.dimensions(&self.handle);
                        if pic_size.width != bounds.width as u32
                            || pic_size.height != bounds.height as u32
                        {
                            let x = pic_size.width as f32 / bounds.width;
                            let y = pic_size.height as f32 / bounds.height;
                            pos = Vector {
                                x: pos.x * x,
                                y: pos.y * y,
                            };
                        }
                        let pos = Vector {
                            x: pos.x as u32,
                            y: pos.y as u32,
                        };
                        let m = (self.on_click)(pos);
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

impl<'a, Message: 'a, Renderer> From<PixelSampler<'a, Message>> for Element<'a, Message, Renderer>
where
    Renderer: iced_native::image::Renderer<Handle = Handle>,
{
    fn from(value: PixelSampler<'a, Message>) -> Self {
        Self::new(value)
    }
}

fn preview_rect<Renderer>(image: &Handle, renderer: &Renderer, bounds: &Rectangle) -> Rectangle
where
    Renderer: iced_native::image::Renderer<Handle = Handle>,
{
    let size = renderer.dimensions(image);
    let size = Size {
        width: size.width as f32,
        height: size.height as f32,
    };

    let c = ContentFit::Contain.fit(size, bounds.size());

    Rectangle {
        width: c.width,
        height: c.height,
        x: bounds.x + (bounds.width - c.width).max(0.0) / 2.0,
        y: bounds.y + (bounds.height - c.height).max(0.0) / 2.0,
    }
}
