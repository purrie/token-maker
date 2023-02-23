/// Tags for program color theme
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
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
