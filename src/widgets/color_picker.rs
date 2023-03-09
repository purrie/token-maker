use iced::{event::Status, Color, Element, Length, Point, Rectangle, Size, Vector};
use iced_graphics::{Backend, Renderer};
use iced_native::{
    layout::{Limits, Node},
    renderer::Quad,
    text::{Renderer as _, Text},
    widget::tree,
    Renderer as _, Widget,
};

use crate::image::{color_to_hsv, hsv_to_color};

pub struct ColorPicker<'c, M> {
    color: Color,
    on_submit: Box<dyn 'c + Fn(Color) -> M>,
    width: Length,
    height: Length,
}

impl<'c, M, B, T> Widget<M, Renderer<B, T>> for ColorPicker<'c, M>
where
    M: Clone,
    B: Backend + iced_graphics::backend::Text,
{
    fn width(&self) -> iced::Length {
        self.width
    }

    fn height(&self) -> iced::Length {
        self.height
    }

    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn layout(&self, _renderer: &Renderer<B, T>, limits: &Limits) -> Node {
        let min = Size {
            width: 8.0,
            height: 8.0,
        };
        let size = limits.width(self.width).height(self.height).resolve(min);
        Node::new(size)
    }
    fn draw(
        &self,
        state: &iced_native::widget::Tree,
        renderer: &mut Renderer<B, T>,
        _theme: &<Renderer<B, T> as iced_native::Renderer>::Theme,
        _style: &iced_native::renderer::Style,
        layout: iced_native::Layout<'_>,
        cursor_position: Point,
        _viewport: &iced::Rectangle,
    ) {
        let bounds = layout.bounds();
        let local_state = state.state.downcast_ref::<State>();
        let is_mouse_over = bounds.contains(cursor_position);
        let border_color = if local_state.button_pressed {
            Color::WHITE
        } else {
            Color::BLACK
        };
        let border_width = if is_mouse_over { 2.0 } else { 1.0 };
        let border_radius = border_width.into();

        renderer.fill_quad(
            iced_native::renderer::Quad {
                bounds,
                border_width,
                border_radius,
                border_color,
            },
            self.color,
        );
    }

    fn on_event(
        &mut self,
        state: &mut iced_native::widget::Tree,
        event: iced::Event,
        layout: iced_native::Layout<'_>,
        cursor_position: iced::Point,
        _renderer: &Renderer<B, T>,
        _clipboard: &mut dyn iced_native::Clipboard,
        _shell: &mut iced_native::Shell<'_, M>,
    ) -> Status {
        let bounds = layout.bounds();
        let is_mouse_over = bounds.contains(cursor_position);
        let local_state = state.state.downcast_mut::<State>();

        let iced::Event::Mouse(butt) = event else {
            return Status::Ignored;
        };
        match butt {
            iced::mouse::Event::ButtonPressed(_) if is_mouse_over => {
                local_state.button_pressed = true;
                Status::Captured
            }
            iced::mouse::Event::ButtonReleased(_) if local_state.button_pressed => {
                local_state.button_pressed = false;
                if is_mouse_over {
                    local_state.open = !local_state.open;
                    Status::Captured
                } else {
                    Status::Ignored
                }
            }
            _ => Status::Ignored,
        }
    }

    fn overlay<'a>(
        &'a mut self,
        state: &'a mut iced_native::widget::Tree,
        layout: iced_native::Layout<'_>,
        _renderer: &Renderer<B, T>,
    ) -> Option<iced_native::overlay::Element<'a, M, Renderer<B, T>>> {
        let local_state = state.state.downcast_mut::<State>();
        let bounds = layout.bounds();
        let pos = Point {
            x: bounds.x + bounds.width,
            y: bounds.y + bounds.width,
        };

        if local_state.open {
            Some(Overlay::new(local_state, pos, &self.on_submit).into())
        } else {
            None
        }
    }
}

impl<'a, M> ColorPicker<'a, M> {
    pub fn new<F: 'a + Fn(Color) -> M>(color: Color, on_submit: F) -> Self {
        Self {
            color,
            on_submit: Box::new(on_submit),
            height: Length::Shrink,
            width: Length::Shrink,
        }
    }

    pub fn width(mut self, width: impl Into<Length>) -> Self {
        self.width = width.into();
        self
    }

    pub fn height(mut self, height: impl Into<Length>) -> Self {
        self.height = height.into();
        self
    }
}

impl<'a, M, B, T> From<ColorPicker<'a, M>> for Element<'a, M, Renderer<B, T>>
where
    M: Clone + 'a,
    B: Backend + iced_graphics::backend::Text,
{
    fn from(value: ColorPicker<'a, M>) -> Self {
        Self::new(value)
    }
}

#[derive(Default)]
struct State {
    open: bool,
    button_pressed: bool,
    hue_widget: iced::widget::canvas::Cache,
    color_widget: iced::widget::canvas::Cache,

    hue: f32,
    saturation: f32,
    value: f32,
}

struct Overlay<'a, M> {
    state: &'a mut State,
    area: Rectangle,
    margin: f32,
    spacing: f32,
    on_submit: &'a Box<dyn 'a + Fn(Color) -> M>,
}

impl<'a, M> Overlay<'a, M> {
    fn new(state: &'a mut State, pos: Point, on_submit: &'a Box<dyn 'a + Fn(Color) -> M>) -> Self {
        Self {
            state,
            area: Rectangle {
                x: pos.x,
                y: pos.y,
                width: 400.0,
                height: 200.0,
            },
            margin: 10.0,
            spacing: 10.0,
            on_submit,
        }
    }
}

impl<'a, M, B, T> iced_native::Overlay<M, Renderer<B, T>> for Overlay<'a, M>
where
    M: Clone,
    B: Backend + iced_graphics::backend::Text,
{
    fn layout(&self, _renderer: &Renderer<B, T>, _bounds: Size, _position: iced::Point) -> Node {
        let mut n = Node::new(self.area.size());
        n.move_to(self.area.position());
        n
    }

    fn draw(
        &self,
        renderer: &mut Renderer<B, T>,
        _theme: &<Renderer<B, T> as iced_native::Renderer>::Theme,
        _style: &iced_native::renderer::Style,
        layout: iced_native::Layout<'_>,
        cursor_position: iced::Point,
    ) {
        let bounds = layout.bounds();

        let background_color = Color::WHITE;

        // Background
        renderer.fill_quad(
            Quad {
                border_color: Color::BLACK,
                border_radius: 2.0.into(),
                border_width: 2.0,
                bounds,
            },
            background_color,
        );

        // Hue widget, used to determine the hue of the color to be picked
        let hue_area = self.hue_widget_rect();
        let hue = self.state.hue_widget.draw(hue_area.size(), |f| {
            let cols = f.width() as u16;
            let h = f.height();

            for i in 0..cols {
                let hue = i as f32 / cols as f32;
                f.fill_rectangle(
                    Point {
                        x: i as f32,
                        y: 0.0,
                    },
                    Size {
                        width: 1.0,
                        height: h,
                    },
                    hsv_to_color(hue, 1.0, 1.0),
                );
            }
        });

        renderer.with_translation(
            Vector {
                x: hue_area.x,
                y: hue_area.y,
            },
            |f| {
                f.draw_primitive(hue.into_primitive());
            },
        );

        // Color widget, allows choosing the saturation and value of the color
        let color_area = self.color_widget_rect();
        let color = self.state.color_widget.draw(color_area.size(), |f| {
            let cols = f.width() as u16;
            let rows = f.height() as u16;

            for x in 0..cols {
                for y in 0..rows {
                    let s = 1.0 - y as f32 / rows as f32;
                    let v = x as f32 / cols as f32;
                    let col = hsv_to_color(self.state.hue, s, v);

                    f.fill_rectangle(
                        Point {
                            x: x as f32,
                            y: y as f32,
                        },
                        Size::UNIT,
                        col,
                    );
                }
            }
        });

        renderer.with_translation(
            Vector {
                x: color_area.x,
                y: color_area.y,
            },
            |r| {
                r.draw_primitive(color.into_primitive());
            },
        );

        // Drawing sliders for choosing specific colors
        let mut r_area = self.r_widget_rect();
        let mut g_area = self.g_widget_rect();
        let mut b_area = self.b_widget_rect();
        let p_area = self.preview_rect();
        let col = hsv_to_color(self.state.hue, self.state.saturation, self.state.value);

        // Drawing borders
        renderer.fill_quad(
            Quad {
                border_color: Color::BLACK,
                border_radius: 0.0.into(),
                border_width: 1.0,
                bounds: r_area,
            },
            background_color,
        );
        renderer.fill_quad(
            Quad {
                border_color: Color::BLACK,
                border_radius: 0.0.into(),
                border_width: 1.0,
                bounds: g_area,
            },
            background_color,
        );
        renderer.fill_quad(
            Quad {
                border_color: Color::BLACK,
                border_radius: 0.0.into(),
                border_width: 1.0,
                bounds: b_area,
            },
            background_color,
        );

        r_area.width *= col.r;
        g_area.width *= col.g;
        b_area.width *= col.b;

        // Drawing fills for the sliders
        renderer.fill_quad(
            Quad {
                border_color: background_color,
                border_radius: 0.0.into(),
                border_width: 0.0,
                bounds: r_area,
            },
            Color::from_rgb(col.r, 0.0, 0.0),
        );
        renderer.fill_quad(
            Quad {
                border_color: background_color,
                border_radius: 0.0.into(),
                border_width: 0.0,
                bounds: g_area,
            },
            Color::from_rgb(0.0, col.g, 0.0),
        );
        renderer.fill_quad(
            Quad {
                border_color: background_color,
                border_radius: 0.0.into(),
                border_width: 0.0,
                bounds: b_area,
            },
            Color::from_rgb(0.0, 0.0, col.b),
        );

        // preview square
        renderer.fill_quad(
            Quad {
                border_color: Color::BLACK,
                border_radius: 0.0.into(),
                border_width: 1.0,
                bounds: p_area,
            },
            col,
        );

        // accept button
        let butt = self.accept_rect();
        let butt_border = if butt.contains(cursor_position) {
            2.0
        } else {
            1.0
        };

        renderer.fill_quad(
            Quad {
                border_color: Color::BLACK,
                border_radius: butt_border.into(),
                border_width: butt_border,
                bounds: butt,
            },
            Color::WHITE,
        );

        renderer.fill_text(Text {
            bounds: butt,
            color: Color::BLACK,
            content: " >",
            size: butt.height,
            font: Default::default(),
            horizontal_alignment: iced::alignment::Horizontal::Left,
            vertical_alignment: iced::alignment::Vertical::Top,
        });
    }

    fn on_event(
        &mut self,
        event: iced::Event,
        _layout: iced_native::Layout<'_>,
        cursor_position: iced::Point,
        _renderer: &Renderer<B, T>,
        _clipboard: &mut dyn iced_native::Clipboard,
        shell: &mut iced_native::Shell<'_, M>,
    ) -> Status {
        match event {
            iced::Event::Mouse(event) => match event {
                iced::mouse::Event::ButtonPressed(_) if self.area.contains(cursor_position) => {
                    if let Some(p) =
                        rect_local_point_normalized(self.hue_widget_rect(), cursor_position)
                    {
                        self.state.hue = p.x;
                        self.state.hue_widget.clear();
                        self.state.color_widget.clear();
                        Status::Captured
                    } else if let Some(p) =
                        rect_local_point_normalized(self.color_widget_rect(), cursor_position)
                    {
                        self.state.value = p.x;
                        self.state.saturation = 1.0 - p.y;
                        self.state.color_widget.clear();
                        Status::Captured
                    } else if let Some(p) =
                        rect_local_point_normalized(self.r_widget_rect(), cursor_position)
                    {
                        let mut col =
                            hsv_to_color(self.state.hue, self.state.saturation, self.state.value);
                        col.r = p.x;
                        let (h, s, v) = color_to_hsv(col);
                        self.state.hue = h;
                        self.state.saturation = s;
                        self.state.value = v;
                        self.state.hue_widget.clear();
                        self.state.color_widget.clear();
                        Status::Captured
                    } else if let Some(p) =
                        rect_local_point_normalized(self.g_widget_rect(), cursor_position)
                    {
                        let mut col =
                            hsv_to_color(self.state.hue, self.state.saturation, self.state.value);
                        col.g = p.x;
                        let (h, s, v) = color_to_hsv(col);
                        self.state.hue = h;
                        self.state.saturation = s;
                        self.state.value = v;
                        self.state.hue_widget.clear();
                        self.state.color_widget.clear();
                        Status::Captured
                    } else if let Some(p) =
                        rect_local_point_normalized(self.b_widget_rect(), cursor_position)
                    {
                        let mut col =
                            hsv_to_color(self.state.hue, self.state.saturation, self.state.value);
                        col.b = p.x;
                        let (h, s, v) = color_to_hsv(col);
                        self.state.hue = h;
                        self.state.saturation = s;
                        self.state.value = v;
                        self.state.hue_widget.clear();
                        self.state.color_widget.clear();
                        Status::Captured
                    } else if self.accept_rect().contains(cursor_position) {
                        let col =
                            hsv_to_color(self.state.hue, self.state.saturation, self.state.value);
                        let m = (self.on_submit)(col);
                        self.state.open = false;
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

impl<'a, M, B, T> From<Overlay<'a, M>> for iced_native::overlay::Element<'a, M, Renderer<B, T>>
where
    M: Clone,
    B: Backend + iced_graphics::backend::Text,
{
    fn from(value: Overlay<'a, M>) -> Self {
        Self::new(iced::Point { x: 0.0, y: 0.0 }, Box::new(value))
    }
}

impl<'a, M> Overlay<'a, M> {
    fn hue_widget_rect(&self) -> Rectangle {
        Rectangle {
            x: self.area.x + self.margin,
            y: self.area.y + self.margin,
            width: self.area.width * 0.5 - self.spacing * 0.5 - self.margin,
            height: self.area.height * 0.1,
        }
    }

    fn color_widget_rect(&self) -> Rectangle {
        Rectangle {
            x: self.area.x + self.margin,
            y: self.area.y + self.area.height * 0.1 + self.spacing + self.margin,
            width: self.area.width * 0.5 - self.spacing * 0.5 - self.margin,
            height: self.area.height - self.margin * 2.0 - self.spacing - self.area.height * 0.1,
        }
    }

    fn r_widget_rect(&self) -> Rectangle {
        Rectangle {
            x: self.area.x + self.area.width * 0.5 + self.spacing * 0.5,
            y: self.area.y + self.margin,
            width: self.area.width * 0.5 - self.margin - self.spacing * 0.5,
            height: self.area.height * 0.1,
        }
    }

    fn g_widget_rect(&self) -> Rectangle {
        Rectangle {
            x: self.area.x + self.area.width * 0.5 + self.spacing * 0.5,
            y: self.area.y + self.margin + self.area.height * 0.1 + self.spacing,
            width: self.area.width * 0.5 - self.margin - self.spacing * 0.5,
            height: self.area.height * 0.1,
        }
    }

    fn b_widget_rect(&self) -> Rectangle {
        Rectangle {
            x: self.area.x + self.area.width * 0.5 + self.spacing * 0.5,
            y: self.area.y + self.margin + self.area.height * 0.2 + self.spacing * 2.0,
            width: self.area.width * 0.5 - self.margin - self.spacing * 0.5,
            height: self.area.height * 0.1,
        }
    }

    fn preview_rect(&self) -> Rectangle {
        Rectangle {
            x: self.area.x + self.area.width * 0.95 - self.margin,
            y: self.area.y + self.margin + self.area.height * 0.3 + self.spacing * 3.0,
            width: self.area.width * 0.05,
            height: self.area.height * 0.1,
        }
    }

    fn accept_rect(&self) -> Rectangle {
        Rectangle {
            x: self.area.x + self.area.width * 0.9 - self.margin,
            y: self.area.y + self.area.height * 0.8 - self.margin,
            width: self.area.width * 0.1,
            height: self.area.height * 0.2,
        }
    }
}

fn rect_local_point_normalized(rect: Rectangle, point: Point) -> Option<Point> {
    if rect.contains(point) {
        Some(Point {
            x: (point.x - rect.x) / rect.width,
            y: (point.y - rect.y) / rect.height,
        })
    } else {
        None
    }
}
