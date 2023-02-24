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
