use iced::{theme, widget, Theme as IcedTheme, Vector};
use serde::{Deserialize, Serialize};

/// Tags for program color theme
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Theme {
    #[default]
    Light,
    Dark,
}

impl From<Theme> for iced::Theme {
    fn from(value: Theme) -> Self {
        match value {
            Theme::Light => Self::Light,
            Theme::Dark => Self::Dark,
        }
    }
}

/// Provides instruction as to how workspaces should be laid out
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Layout {
    /// One next to another
    #[default]
    Parallel,
    /// One at a time in tabs
    Stacking(usize),
}

/// Tag used to style UI widgets
pub enum Style {
    Header,
    Frame,
    Margins,
    Danger,
    Action,
}

impl widget::container::StyleSheet for Style {
    type Style = IcedTheme;

    fn appearance(&self, style: &Self::Style) -> widget::container::Appearance {
        use widget::container::Appearance;

        let ext = style.extended_palette();

        match self {
            Style::Header => {
                let color = ext.background.base.color;
                let text = ext.background.base.text;

                Appearance {
                    text_color: Some(text),
                    background: Some(color.into()),
                    border_radius: 0.0,
                    border_width: 0.0,
                    border_color: color,
                }
            }
            Style::Frame => {
                let border = ext.background.base.color;
                let color = ext.background.weak.color;
                let text = ext.background.weak.text;

                Appearance {
                    text_color: Some(text),
                    background: Some(color.into()),
                    border_radius: 2.0,
                    border_width: 2.0,
                    border_color: border,
                }
            }
            Style::Margins => {
                let color = ext.background.strong.color;
                let text = ext.background.strong.text;

                Appearance {
                    text_color: Some(text),
                    background: Some(color.into()),
                    border_radius: 0.0,
                    border_width: 0.0,
                    border_color: color,
                }
            }
            Style::Danger => {
                let color = ext.danger.base.color;
                let text = ext.danger.base.text;

                Appearance {
                    text_color: Some(text),
                    background: Some(color.into()),
                    border_radius: 0.0,
                    border_width: 0.0,
                    border_color: color,
                }
            }
            Style::Action => {
                let color = ext.primary.base.color;
                let text = ext.primary.base.text;

                Appearance {
                    text_color: Some(text),
                    background: Some(color.into()),
                    border_radius: 0.0,
                    border_width: 0.0,
                    border_color: color,
                }
            }
        }
    }
}

impl widget::button::StyleSheet for Style {
    type Style = IcedTheme;

    fn active(&self, style: &Self::Style) -> widget::button::Appearance {
        use widget::button::Appearance;

        let ext = style.extended_palette();

        match self {
            Style::Frame => {
                let bg = ext.background.weak.color;
                let text = ext.background.weak.text;
                let border = ext.background.base.color;

                Appearance {
                    background: bg.into(),
                    text_color: text,
                    border_color: border,
                    border_radius: 2.0,
                    border_width: 2.0,
                    shadow_offset: Vector { x: 2.0, y: 2.0 },
                }
            }
            Style::Danger | Style::Action | Style::Header | Style::Margins => unreachable!(), // unused?
        }
    }

    fn hovered(&self, style: &Self::Style) -> widget::button::Appearance {
        use widget::button::Appearance;

        let ext = style.extended_palette();

        match self {
            Style::Frame => {
                let bg = ext.background.weak.color;
                let text = ext.background.weak.text;
                let border = ext.background.base.color;

                Appearance {
                    background: bg.into(),
                    text_color: text,
                    border_color: border,
                    border_radius: 2.0,
                    border_width: 4.0,
                    shadow_offset: Vector { x: 2.0, y: 2.0 },
                }
            }
            Style::Danger | Style::Action | Style::Header | Style::Margins => unreachable!(), // unused?
        }
    }

    fn pressed(&self, style: &Self::Style) -> widget::button::Appearance {
        use widget::button::Appearance;

        let ext = style.extended_palette();

        match self {
            Style::Frame => {
                let bg = ext.background.weak.color;
                let text = ext.background.weak.text;
                let border = ext.background.base.color;

                Appearance {
                    background: bg.into(),
                    text_color: text,
                    border_color: border,
                    border_radius: 2.0,
                    border_width: 2.0,
                    shadow_offset: Vector { x: -2.0, y: -2.0 },
                }
            }
            Style::Danger | Style::Action | Style::Header | Style::Margins => unreachable!(), // unused?
        }
    }

    fn disabled(&self, style: &Self::Style) -> widget::button::Appearance {
        use widget::button::Appearance;

        let ext = style.extended_palette();

        match self {
            Style::Frame => {
                let bg = ext.secondary.weak.color;
                let text = ext.secondary.weak.text;
                let border = ext.secondary.base.color;

                Appearance {
                    background: bg.into(),
                    text_color: text,
                    border_color: border,
                    border_radius: 2.0,
                    border_width: 1.0,
                    shadow_offset: Vector { x: 1.0, y: 1.0 },
                }
            }
            Style::Danger | Style::Action | Style::Header | Style::Margins => unreachable!(), // unused?
        }
    }
}

impl From<Style> for theme::Container {
    fn from(value: Style) -> Self {
        theme::Container::Custom(Box::new(value))
    }
}

impl From<Style> for theme::Button {
    fn from(value: Style) -> Self {
        use theme::Button;

        match value {
            Style::Header => Button::Text,
            Style::Frame => Button::Custom(Box::new(value)),
            Style::Margins => Button::Secondary,
            Style::Danger => Button::Destructive,
            Style::Action => Button::Primary,
        }
    }
}
