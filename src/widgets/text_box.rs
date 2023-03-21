use iced::{
    alignment::{Horizontal, Vertical},
    event::Status,
    keyboard, mouse, Color, Point, Rectangle,
};
use iced_native::{renderer::Quad, text::Text};

/// This struct is focused on providing text input functionality to use within other widgets
#[derive(Default)]
pub struct TextBox {
    content: String,
    focus: bool,
    inserter: Option<Box<dyn Fn(&mut String, &mut usize, char) -> Status>>,
    cursor: usize,
    // TODO add font size
    // TODO add fonts
    // TODO add style
}

impl TextBox {
    pub fn set_input<F: Fn(&mut String, &mut usize, char) -> Status + 'static>(
        &mut self,
        inserter: F,
    ) {
        self.inserter = Some(Box::new(inserter));
    }

    pub fn set_content(&mut self, content: String) {
        self.content = content;
    }

    pub fn get_content(&self) -> &String {
        &self.content
    }

    pub fn draw<Renderer, Theme>(
        &self,
        area: Rectangle,
        theme: &Theme,
        renderer: &mut Renderer,
        cursor_position: Point,
    ) where
        Renderer: iced_native::text::Renderer,
        Theme: StyleSheet<Style = TextBoxStyle>,
    {
        let colors = if self.focus {
            theme.focus(&TextBoxStyle::Regular)
        } else if area.contains(cursor_position) {
            theme.hover(&TextBoxStyle::Regular)
        } else {
            theme.regular(&TextBoxStyle::Regular)
        };

        renderer.fill_quad(
            Quad {
                bounds: area,
                border_radius: colors.border_radius.into(),
                border_width: colors.border_width,
                border_color: colors.border_color,
            },
            colors.background,
        );

        let size = renderer.default_size() - 4.0;

        let r = |renderer: &mut Renderer| {
            // TODO scroll text if it doesn't fit the box
            renderer.fill_text(Text {
                content: &self.content,
                bounds: Rectangle {
                    x: area.x + 2.0,
                    y: area.y + area.height * 0.5,
                    ..area
                },
                size,
                color: colors.text_color,
                font: Default::default(),
                horizontal_alignment: Horizontal::Left,
                vertical_alignment: Vertical::Center,
            });
        };
        let text_width = renderer
            .measure(&self.content, area.height, Default::default(), area.size())
            .0;

        if text_width > area.width {
            renderer.with_layer(area, r);
        } else {
            r(renderer);
        }

        if self.focus {
            let pos = if self.cursor == 0 {
                0.0
            } else {
                let slice = &self.content[..self.cursor];
                renderer.measure_width(slice, size, Default::default())
            };
            let margin = 3.0;
            let area = Rectangle {
                x: area.x + pos + 2.0,
                y: area.y + margin,
                width: 1.0,
                height: area.height - margin * 2.0,
            };
            renderer.fill_quad(
                Quad {
                    bounds: area,
                    border_radius: 0.0.into(),
                    border_width: 0.0,
                    border_color: Color::BLACK,
                },
                colors.cursor_color,
            );
        }
    }

    pub fn on_event<Renderer>(
        &mut self,
        area: Rectangle,
        event: &iced::Event,
        renderer: &Renderer,
        cursor_position: Point,
        // clipboard: &mut dyn iced_native::Clipboard,
    ) -> TextBoxStatus
    where
        Renderer: iced_native::text::Renderer,
    {
        match event {
            iced::Event::Keyboard(kbd) if self.focus => match kbd {
                keyboard::Event::CharacterReceived(c) => {
                    if let Some(filter) = &self.inserter {
                        match filter(&mut self.content, &mut self.cursor, *c) {
                            Status::Ignored => TextBoxStatus::Captured,
                            Status::Captured => TextBoxStatus::ContentChanged,
                        }
                    } else {
                        self.content.insert(self.cursor, *c);
                        self.cursor += 1;
                        TextBoxStatus::ContentChanged
                    }
                }

                keyboard::Event::KeyPressed {
                    key_code,
                    modifiers: _,
                } => match key_code {
                    keyboard::KeyCode::Left if self.cursor > 0 => {
                        self.cursor -= 1;
                        TextBoxStatus::Captured
                    }

                    keyboard::KeyCode::Right if self.content.len() > self.cursor => {
                        self.cursor += 1;
                        TextBoxStatus::Captured
                    }

                    keyboard::KeyCode::Backspace if self.cursor > 0 => {
                        self.cursor -= 1;
                        self.content.remove(self.cursor);
                        TextBoxStatus::ContentChanged
                    }

                    keyboard::KeyCode::Delete if self.cursor < self.content.len() => {
                        self.content.remove(self.cursor);
                        TextBoxStatus::ContentChanged
                    }
                    _ => TextBoxStatus::Ignored,
                },
                _ => TextBoxStatus::Ignored,
            },

            iced::Event::Mouse(crs) => match crs {
                mouse::Event::ButtonPressed(_) => {
                    self.focus = area.contains(cursor_position);
                    if self.focus {
                        let click_pos = cursor_position.x - area.x;
                        self.cursor = 0;
                        let mut len = click_pos;
                        for i in 1..=self.content.len() {
                            let slice_len = renderer
                                .measure(
                                    &self.content[..i],
                                    renderer.default_size() - 4.0,
                                    Default::default(),
                                    area.size(),
                                )
                                .0;
                            let dist = (click_pos - slice_len).abs();
                            if dist < len {
                                len = dist;
                                self.cursor += 1;
                            } else {
                                break;
                            }
                        }
                    }
                    TextBoxStatus::Ignored
                }
                _ => TextBoxStatus::Ignored,
            },
            _ => TextBoxStatus::Ignored,
        }
    }
}

pub enum TextBoxStatus {
    Ignored,
    Captured,
    ContentChanged,
}

#[derive(Default)]
pub enum TextBoxStyle {
    #[default]
    Regular,
}

pub struct Appearance {
    background: Color,
    text_color: Color,
    cursor_color: Color,
    border_color: Color,
    border_width: f32,
    border_radius: f32,
}

pub trait StyleSheet {
    type Style: Default;

    /// Appearance for the `TextBox` when it has focus
    fn focus(&self, style: &Self::Style) -> Appearance;
    /// Appearance for the `TextBox` when it doesn't have focus
    fn regular(&self, style: &Self::Style) -> Appearance;
    /// Appearance for the `TextBox` when user hovers cursor over it
    fn hover(&self, style: &Self::Style) -> Appearance;
}

impl StyleSheet for iced::Theme {
    type Style = TextBoxStyle;

    fn focus(&self, style: &Self::Style) -> Appearance {
        let pal = self.extended_palette();
        match style {
            TextBoxStyle::Regular => Appearance {
                background: pal.background.base.color,
                text_color: pal.background.base.text,
                cursor_color: pal.primary.base.color,
                border_color: pal.background.strong.color,
                border_width: 1.0,
                border_radius: 0.0,
            },
        }
    }
    fn regular(&self, style: &Self::Style) -> Appearance {
        let pal = self.extended_palette();
        match style {
            TextBoxStyle::Regular => Appearance {
                background: pal.background.base.color,
                text_color: pal.background.base.text,
                cursor_color: pal.primary.base.color,
                border_color: pal.background.strong.color,
                border_width: 1.0,
                border_radius: 0.0,
            },
        }
    }
    fn hover(&self, style: &Self::Style) -> Appearance {
        let pal = self.extended_palette();
        match style {
            TextBoxStyle::Regular => Appearance {
                background: pal.background.base.color,
                text_color: pal.background.base.text,
                cursor_color: pal.primary.base.color,
                border_color: pal.background.strong.color,
                border_width: 2.0,
                border_radius: 0.0,
            },
        }
    }
}
