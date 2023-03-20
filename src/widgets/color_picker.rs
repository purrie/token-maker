use iced::{
    event::Status, widget::canvas::Path, Color, Element, Length, Point, Rectangle, Size, Theme,
    Vector,
};
use iced_graphics::{Backend, Renderer};
use iced_native::{
    layout::{Limits, Node},
    renderer::Quad,
    text::{Renderer as _, Text},
    widget::tree,
    Renderer as _, Widget,
};

use crate::image::{color_to_hsv, hsv_to_color};

use super::text_box::{self, TextBox, TextBoxStyle};

pub struct ColorPicker<'c, M, R>
where
    R: iced_native::Renderer,
    R::Theme: StyleSheet,
    <R::Theme as StyleSheet>::Style: Default,
{
    color: Color,
    on_submit: Box<dyn 'c + Fn(Color) -> M>,
    width: Length,
    height: Length,
    style: <R::Theme as StyleSheet>::Style,
}

impl<'c, M, B, T> Widget<M, Renderer<B, T>> for ColorPicker<'c, M, Renderer<B, T>>
where
    M: Clone,
    B: Backend + iced_graphics::backend::Text,
    T: StyleSheet + text_box::StyleSheet<Style = TextBoxStyle>,
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
        let mut state = State::default();

        let inserter = |content: &mut String, cursor: &mut usize, c: char| {
            let point = content.find('.');
            let first_char = content.chars().nth(0);
            match (c, point, first_char) {
                ('1', Some(p), _) if p == *cursor => {
                    *content = String::from("1.00");
                    *cursor = 2;
                    Status::Captured
                }
                ('0', Some(p), _) if p == *cursor => {
                    *content = String::from("0.00");
                    *cursor = 2;
                    Status::Captured
                }
                ('.', None, Some(s)) => {
                    match s {
                        '0' => {
                            content.insert(1, '.');
                        }
                        '1' => {
                            *content = String::from("1.00");
                        }
                        _ => {
                            content.insert_str(0, "0.");
                        }
                    }
                    *cursor = 2;
                    Status::Captured
                }
                ('.', None, None) => {
                    content.insert_str(0, "0.");
                    *cursor = 2;
                    Status::Captured
                }
                ('1', None, _) => {
                    *content = String::from("1.00");
                    *cursor = 2;
                    Status::Captured
                }
                (x, None, _) if x.is_numeric() => {
                    *content = format!("0.{}", x);
                    *cursor = 3;
                    Status::Captured
                }
                (x, Some(p), Some('0')) if x.is_numeric() && p < *cursor && *cursor < 6 => {
                    content.insert(*cursor, x);
                    *cursor += 1;
                    if content.len() > 6 {
                        *content = content[..6].to_string();
                    }
                    Status::Captured
                }
                (x, Some(p), Some(_)) if x.is_numeric() && p < *cursor && *cursor < 6 => {
                    content.remove(0);
                    content.insert(0, '0');
                    content.insert(*cursor, x);
                    *cursor += 1;
                    if content.len() > 6 {
                        *content = content[..6].to_string();
                    }
                    Status::Captured
                }
                _ => Status::Ignored,
            }
        };
        state.r_input.set_input(inserter);
        state.g_input.set_input(inserter);
        state.b_input.set_input(inserter);

        tree::State::new(state)
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
            Some(Overlay::new(local_state, pos, &self.on_submit, &self.style).into())
        } else {
            None
        }
    }
}

impl<'a, M, R> ColorPicker<'a, M, R>
where
    R: iced_native::Renderer,
    R::Theme: StyleSheet,
    <R::Theme as StyleSheet>::Style: Default,
{
    pub fn new<F: 'a + Fn(Color) -> M>(color: Color, on_submit: F) -> Self {
        Self {
            color,
            on_submit: Box::new(on_submit),
            height: Length::Shrink,
            width: Length::Shrink,
            style: <R::Theme as StyleSheet>::Style::default(),
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

#[derive(Default)]
struct State {
    open: bool,
    button_pressed: bool,
    hue_widget: iced::widget::canvas::Cache,
    color_widget: iced::widget::canvas::Cache,

    r_input: TextBox,
    g_input: TextBox,
    b_input: TextBox,

    hue: f32,
    saturation: f32,
    value: f32,

    mouseover_hue: bool,
    mouseover_color: bool,
}

impl State {
    fn regenerate_ui(&mut self) {
        self.hue_widget.clear();
        self.color_widget.clear();

        let Color { r, g, b, a: _ } = hsv_to_color(self.hue, self.saturation, self.value);
        let r = format!("{:.4}", r);
        let g = format!("{:.4}", g);
        let b = format!("{:.4}", b);

        self.r_input.set_content(r);
        self.g_input.set_content(g);
        self.b_input.set_content(b);
    }

    fn update_color_from_input(&mut self) {
        let Ok(r) = self.r_input.get_content().parse() else {
            return;
        };
        let Ok(g) = self.g_input.get_content().parse() else {
            return;
        };
        let Ok(b) = self.b_input.get_content().parse() else {
            return;
        };

        let (hue, sat, val) = color_to_hsv(Color { r, g, b, a: 1.0 });
        self.hue = hue;
        self.saturation = sat;
        self.value = val;

        self.hue_widget.clear();
        self.color_widget.clear();
    }
}

struct Overlay<'a, M, R>
where
    R: iced_native::Renderer,
    R::Theme: StyleSheet,
{
    state: &'a mut State,
    area: Rectangle,
    margin: f32,
    spacing: f32,
    on_submit: &'a Box<dyn 'a + Fn(Color) -> M>,
    style: &'a <R::Theme as StyleSheet>::Style,
}

impl<'a, M, B, T> Overlay<'a, M, Renderer<B, T>>
where
    B: Backend,
    T: StyleSheet,
{
    fn new(
        state: &'a mut State,
        pos: Point,
        on_submit: &'a Box<dyn 'a + Fn(Color) -> M>,
        style: &'a T::Style,
    ) -> Self {
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
            style,
        }
    }
}

impl<'a, M, B, T> iced_native::Overlay<M, Renderer<B, T>> for Overlay<'a, M, Renderer<B, T>>
where
    M: Clone,
    B: Backend + iced_graphics::backend::Text,
    T: StyleSheet + text_box::StyleSheet<Style = TextBoxStyle>,
{
    fn layout(&self, _renderer: &Renderer<B, T>, bounds: Size, _position: iced::Point) -> Node {
        let mut n = Node::new(self.area.size());
        let my_side = self.area.width + self.area.x;
        let on_side = bounds.width;
        let pos = if my_side > on_side {
            Point {
                x: self.area.x - (my_side - on_side),
                y: self.area.y,
            }
        } else {
            self.area.position()
        };
        n.move_to(pos);
        n
    }

    fn draw(
        &self,
        renderer: &mut Renderer<B, T>,
        theme: &<Renderer<B, T> as iced_native::Renderer>::Theme,
        _style: &iced_native::renderer::Style,
        layout: iced_native::Layout<'_>,
        cursor_position: iced::Point,
    ) {
        let bounds = layout.bounds();

        let style = theme.style(self.style);

        // Background
        renderer.fill_quad(
            Quad {
                border_color: style.border_color,
                border_radius: style.border_radius.into(),
                border_width: style.border_width,
                bounds,
            },
            style.background,
        );

        // Hue widget, used to determine the hue of the color to be picked
        let hue_area = hue_widget_rect(&bounds, self.margin, self.spacing);
        let mouse_over_hue = self.state.mouseover_hue;
        let hue = self.state.hue_widget.draw(hue_area.size(), |f| {
            let cols = f.width() as u16;
            let h = f.height();
            let line = (self.state.hue * f.width()) as u16;

            for i in 0..cols {
                let hue = i as f32 / cols as f32;

                let col = if i == line {
                    Color::BLACK
                } else {
                    hsv_to_color(hue, 1.0, 1.0)
                };

                f.fill_rectangle(
                    Point {
                        x: i as f32,
                        y: 0.0,
                    },
                    Size {
                        width: 1.0,
                        height: h,
                    },
                    col,
                );
            }
            let (size, color) = if mouse_over_hue {
                (style.hover_border_width, style.hover_border_color)
            } else {
                (style.border_width, style.border_color)
            };

            let top = Path::rectangle(
                Point { x: 0.0, y: 0.0 },
                Size {
                    width: f.width(),
                    height: size,
                },
            );
            let bottom = Path::rectangle(
                Point {
                    x: 0.0,
                    y: f.height() - size - 1.0,
                },
                Size {
                    width: f.width(),
                    height: size,
                },
            );
            let left = Path::rectangle(
                Point { x: 0.0, y: 0.0 },
                Size {
                    width: size,
                    height: f.height(),
                },
            );
            let right = Path::rectangle(
                Point {
                    x: f.width() - size - 1.0,
                    y: 0.0,
                },
                Size {
                    width: size,
                    height: f.height(),
                },
            );

            f.fill(&top, color);
            f.fill(&bottom, color);
            f.fill(&left, color);
            f.fill(&right, color);
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
        let color_area = color_widget_rect(&bounds, self.margin, self.spacing);
        let mouse_over_color = self.state.mouseover_color;
        let color = self.state.color_widget.draw(color_area.size(), |f| {
            let cols = f.width() as u16;
            let rows = f.height() as u16;

            let line_h = ((1.0 - self.state.saturation) * f.height()) as u16;
            let line_w = (self.state.value * f.width()) as u16;

            for x in 0..cols {
                for y in 0..rows {
                    let s = 1.0 - y as f32 / rows as f32;
                    let v = x as f32 / cols as f32;
                    let col = if x == line_w || y == line_h {
                        Color::BLACK
                    } else {
                        hsv_to_color(self.state.hue, s, v)
                    };

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
            let (size, color) = if mouse_over_color {
                (style.hover_border_width, style.hover_border_color)
            } else {
                (style.border_width, style.border_color)
            };

            let top = Path::rectangle(
                Point { x: 0.0, y: 0.0 },
                Size {
                    width: f.width(),
                    height: size,
                },
            );
            let bottom = Path::rectangle(
                Point {
                    x: 0.0,
                    y: f.height() - size - 1.0,
                },
                Size {
                    width: f.width(),
                    height: size,
                },
            );
            let left = Path::rectangle(
                Point { x: 0.0, y: 0.0 },
                Size {
                    width: size,
                    height: f.height(),
                },
            );
            let right = Path::rectangle(
                Point {
                    x: f.width() - size - 1.0,
                    y: 0.0,
                },
                Size {
                    width: size,
                    height: f.height(),
                },
            );

            f.fill(&top, color);
            f.fill(&bottom, color);
            f.fill(&left, color);
            f.fill(&right, color);
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
        let r_area = slider_widget_rect(&bounds, self.margin, self.spacing, 0.0);
        let g_area = slider_widget_rect(&bounds, self.margin, self.spacing, 1.0);
        let b_area = slider_widget_rect(&bounds, self.margin, self.spacing, 2.0);
        let p_area = preview_rect(&bounds, self.margin, self.spacing);
        let r_input = slider_text_box_rect(&bounds, self.margin, self.spacing, 0.0);
        let g_input = slider_text_box_rect(&bounds, self.margin, self.spacing, 1.0);
        let b_input = slider_text_box_rect(&bounds, self.margin, self.spacing, 2.0);
        let col = hsv_to_color(self.state.hue, self.state.saturation, self.state.value);

        let mut r_border = if r_area.contains(cursor_position) {
            Quad {
                border_color: style.hover_border_color,
                border_radius: style.hover_border_radius.into(),
                border_width: style.hover_border_width,
                bounds: r_area,
            }
        } else {
            Quad {
                border_color: style.border_color,
                border_radius: style.border_radius.into(),
                border_width: style.border_width,
                bounds: r_area,
            }
        };
        let mut g_border = if g_area.contains(cursor_position) {
            Quad {
                border_color: style.hover_border_color,
                border_radius: style.hover_border_radius.into(),
                border_width: style.hover_border_width,
                bounds: g_area,
            }
        } else {
            Quad {
                border_color: style.border_color,
                border_radius: style.border_radius.into(),
                border_width: style.border_width,
                bounds: g_area,
            }
        };
        let mut b_border = if b_area.contains(cursor_position) {
            Quad {
                border_color: style.hover_border_color,
                border_radius: style.hover_border_radius.into(),
                border_width: style.hover_border_width,
                bounds: b_area,
            }
        } else {
            Quad {
                border_color: style.border_color,
                border_radius: style.border_radius.into(),
                border_width: style.border_width,
                bounds: b_area,
            }
        };

        // Drawing borders
        renderer.fill_quad(r_border, style.background);
        renderer.fill_quad(g_border, style.background);
        renderer.fill_quad(b_border, style.background);

        r_border.bounds.width *= col.r;
        g_border.bounds.width *= col.g;
        b_border.bounds.width *= col.b;

        // Drawing fills for the sliders
        renderer.fill_quad(r_border, Color::from_rgb(col.r, 0.0, 0.0));
        renderer.fill_quad(g_border, Color::from_rgb(0.0, col.g, 0.0));
        renderer.fill_quad(b_border, Color::from_rgb(0.0, 0.0, col.b));

        // draw the text input boxes
        self.state
            .r_input
            .draw(r_input, theme, renderer, cursor_position);
        self.state
            .g_input
            .draw(g_input, theme, renderer, cursor_position);
        self.state
            .b_input
            .draw(b_input, theme, renderer, cursor_position);

        // preview square
        renderer.fill_quad(
            Quad {
                border_color: style.border_color,
                border_radius: style.border_radius.into(),
                border_width: style.border_width,
                bounds: p_area,
            },
            col,
        );

        // accept button
        let butt = accept_rect(&bounds, self.margin);
        let accept_quad = if butt.contains(cursor_position) {
            Quad {
                border_color: style.hover_border_color,
                bounds: butt,
                border_radius: style.hover_border_radius.into(),
                border_width: style.hover_border_width,
            }
        } else {
            Quad {
                border_color: style.border_color,
                bounds: butt,
                border_radius: style.border_radius.into(),
                border_width: style.border_width,
            }
        };

        renderer.fill_quad(accept_quad, style.button_color);

        renderer.fill_text(Text {
            bounds: butt,
            color: style.text_color,
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
        layout: iced_native::Layout<'_>,
        cursor_position: iced::Point,
        renderer: &Renderer<B, T>,
        _clipboard: &mut dyn iced_native::Clipboard,
        shell: &mut iced_native::Shell<'_, M>,
    ) -> Status {
        let bounds = layout.bounds();

        let r_input = slider_text_box_rect(&bounds, self.margin, self.spacing, 0.0);
        match self
            .state
            .r_input
            .on_event(r_input, &event, renderer, cursor_position)
        {
            text_box::TextBoxStatus::Ignored => {}
            text_box::TextBoxStatus::Captured => return Status::Captured,
            text_box::TextBoxStatus::ContentChanged => {
                self.state.update_color_from_input();
                return Status::Captured;
            }
        }

        let g_input = slider_text_box_rect(&bounds, self.margin, self.spacing, 1.0);
        match self
            .state
            .g_input
            .on_event(g_input, &event, renderer, cursor_position)
        {
            text_box::TextBoxStatus::Ignored => {}
            text_box::TextBoxStatus::Captured => return Status::Captured,
            text_box::TextBoxStatus::ContentChanged => {
                self.state.update_color_from_input();
                return Status::Captured;
            }
        }

        let b_input = slider_text_box_rect(&bounds, self.margin, self.spacing, 2.0);
        match self
            .state
            .b_input
            .on_event(b_input, &event, renderer, cursor_position)
        {
            text_box::TextBoxStatus::Ignored => {}
            text_box::TextBoxStatus::Captured => return Status::Captured,
            text_box::TextBoxStatus::ContentChanged => {
                self.state.update_color_from_input();
                return Status::Captured;
            }
        }

        match event {
            iced::Event::Mouse(event) => match event {
                iced::mouse::Event::ButtonPressed(_) if self.area.contains(cursor_position) => {
                    if let Some(p) = rect_local_point_normalized(
                        hue_widget_rect(&bounds, self.margin, self.spacing),
                        cursor_position,
                    ) {
                        self.state.hue = p.x;
                        self.state.regenerate_ui();
                        Status::Captured
                    } else if let Some(p) = rect_local_point_normalized(
                        color_widget_rect(&bounds, self.margin, self.spacing),
                        cursor_position,
                    ) {
                        self.state.value = p.x;
                        self.state.saturation = 1.0 - p.y;
                        self.state.regenerate_ui();
                        Status::Captured
                    } else if let Some(p) = rect_local_point_normalized(
                        slider_widget_rect(&bounds, self.margin, self.spacing, 0.0),
                        cursor_position,
                    ) {
                        let mut col =
                            hsv_to_color(self.state.hue, self.state.saturation, self.state.value);
                        col.r = p.x;
                        let (h, s, v) = color_to_hsv(col);
                        self.state.hue = h;
                        self.state.saturation = s;
                        self.state.value = v;
                        self.state.regenerate_ui();
                        Status::Captured
                    } else if let Some(p) = rect_local_point_normalized(
                        slider_widget_rect(&bounds, self.margin, self.spacing, 1.0),
                        cursor_position,
                    ) {
                        let mut col =
                            hsv_to_color(self.state.hue, self.state.saturation, self.state.value);
                        col.g = p.x;
                        let (h, s, v) = color_to_hsv(col);
                        self.state.hue = h;
                        self.state.saturation = s;
                        self.state.value = v;
                        self.state.regenerate_ui();
                        Status::Captured
                    } else if let Some(p) = rect_local_point_normalized(
                        slider_widget_rect(&bounds, self.margin, self.spacing, 2.0),
                        cursor_position,
                    ) {
                        let mut col =
                            hsv_to_color(self.state.hue, self.state.saturation, self.state.value);
                        col.b = p.x;
                        let (h, s, v) = color_to_hsv(col);
                        self.state.hue = h;
                        self.state.saturation = s;
                        self.state.value = v;
                        self.state.regenerate_ui();
                        Status::Captured
                    } else if accept_rect(&bounds, self.margin).contains(cursor_position) {
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
                iced::mouse::Event::CursorMoved { position } => {
                    if hue_widget_rect(&bounds, self.margin, self.spacing).contains(position)
                        != self.state.mouseover_hue
                    {
                        self.state.hue_widget.clear();
                        self.state.mouseover_hue = !self.state.mouseover_hue;
                    }
                    if color_widget_rect(&bounds, self.margin, self.spacing).contains(position)
                        != self.state.mouseover_color
                    {
                        self.state.color_widget.clear();
                        self.state.mouseover_color = !self.state.mouseover_color;
                    }
                    Status::Ignored
                }
                _ => Status::Ignored,
            },
            _ => Status::Ignored,
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

fn slider_widget_rect(area: &Rectangle, margin: f32, spacing: f32, offset: f32) -> Rectangle {
    let width = area.width * 0.3 - margin - spacing * 0.5;
    let height = area.height * 0.1;
    let x = area.x + area.width * 0.5 + spacing * 0.5;
    let y = area.y + margin + (height + spacing) * offset;
    Rectangle {
        x,
        y,
        width,
        height,
    }
}

fn slider_text_box_rect(area: &Rectangle, margin: f32, spacing: f32, offset: f32) -> Rectangle {
    let width = area.width * 0.15 - margin - spacing * 0.5;
    let height = area.height * 0.1;
    let x = area.x + area.width * 0.8 + spacing * 0.5;
    let y = area.y + margin + (height + spacing) * offset;
    Rectangle {
        x,
        y,
        width,
        height,
    }
}

fn hue_widget_rect(area: &Rectangle, margin: f32, spacing: f32) -> Rectangle {
    Rectangle {
        x: area.x + margin,
        y: area.y + margin,
        width: area.width * 0.5 - spacing * 0.5 - margin,
        height: area.height * 0.1,
    }
}

fn color_widget_rect(area: &Rectangle, margin: f32, spacing: f32) -> Rectangle {
    Rectangle {
        x: area.x + margin,
        y: area.y + area.height * 0.1 + spacing + margin,
        width: area.width * 0.5 - spacing * 0.5 - margin,
        height: area.height - margin * 2.0 - spacing - area.height * 0.1,
    }
}

fn preview_rect(area: &Rectangle, margin: f32, spacing: f32) -> Rectangle {
    Rectangle {
        x: area.x + area.width * 0.95 - margin,
        y: area.y + margin + area.height * 0.3 + spacing * 3.0,
        width: area.width * 0.05,
        height: area.height * 0.1,
    }
}

fn accept_rect(area: &Rectangle, margin: f32) -> Rectangle {
    Rectangle {
        x: area.x + area.width * 0.9 - margin,
        y: area.y + area.height * 0.8 - margin,
        width: area.width * 0.1,
        height: area.height * 0.2,
    }
}

// TODO make different functions to get different states for normal, hover, and pressed instead of having one massive appearance

/// Dictates the look of the `ColorPicker` widget
pub struct Appearance {
    background: Color,
    text_color: Color,
    button_color: Color,
    border_color: Color,
    hover_border_color: Color,
    border_width: f32,
    border_radius: f32,
    hover_border_width: f32,
    hover_border_radius: f32,
}

/// Style generator for `ColorPicker` widget
pub trait StyleSheet {
    type Style: Default;
    fn style(&self, style: &Self::Style) -> Appearance;
}

#[derive(Default)]
pub enum PickerStyle {
    #[default]
    Regular,
    // TODO make more appearance types and a custom one
}

impl StyleSheet for Theme {
    type Style = PickerStyle;

    fn style(&self, style: &Self::Style) -> Appearance {
        let col = self.extended_palette();
        match style {
            PickerStyle::Regular => Appearance {
                background: col.background.base.color,
                border_color: col.background.weak.color,
                hover_border_color: col.background.strong.color,
                button_color: col.primary.base.color,
                text_color: col.primary.base.text,
                border_width: 1.0,
                border_radius: 0.0,
                hover_border_width: 2.0,
                hover_border_radius: 0.0,
            },
        }
    }
}

impl<'a, M, B, T> From<ColorPicker<'a, M, Renderer<B, T>>> for Element<'a, M, Renderer<B, T>>
where
    M: Clone + 'a,
    B: Backend + iced_graphics::backend::Text + 'a,
    T: StyleSheet + 'a + Default + text_box::StyleSheet<Style = TextBoxStyle>,
{
    fn from(value: ColorPicker<'a, M, Renderer<B, T>>) -> Self {
        Self::new(value)
    }
}

impl<'a, M, B, T> From<Overlay<'a, M, Renderer<B, T>>>
    for iced_native::overlay::Element<'a, M, Renderer<B, T>>
where
    M: Clone,
    B: Backend + iced_graphics::backend::Text + 'a,
    T: StyleSheet + 'a + text_box::StyleSheet<Style = TextBoxStyle>,
{
    fn from(value: Overlay<'a, M, Renderer<B, T>>) -> Self {
        Self::new(iced::Point { x: 0.0, y: 0.0 }, Box::new(value))
    }
}
