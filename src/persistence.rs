use std::{collections::HashMap, path::PathBuf};

use serde::{Deserialize, Serialize};

use crate::{
    image::ImageFormat,
    style::{Layout, Theme},
    workspace::WorkspaceTemplate,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PersistentValue {
    String(String),
    Theme(Theme),
    Layout(Layout),
    WorkspaceTemplate(WorkspaceTemplate),
    ImageFormat(ImageFormat),
}

impl PersistentValue {
    /// Checks the string value if the type of `CacheValue` is a string.
    pub fn check_string(&self) -> Option<&str> {
        match self {
            PersistentValue::String(s) => Some(s),
            _ => None,
        }
    }
    /// Consumes the value and turns it into a string. If the value was not a string, it will return an empty string.
    pub fn to_string(self) -> String {
        match self {
            Self::String(x) => x,
            _ => String::new(),
        }
    }
    /// Consumes the value and returns the theme within it. If the type wasn't theme, a default theme is returned instead.
    pub fn to_theme(self) -> Theme {
        match self {
            Self::Theme(x) => x,
            _ => Theme::default(),
        }
    }
    /// Consumes the value and returns the layout within it. If the type wasn't layout, a default layout is returned instead.
    pub fn to_layout(self) -> Layout {
        match self {
            Self::Layout(x) => x,
            _ => Layout::default(),
        }
    }
}

impl From<String> for PersistentValue {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}
impl From<&str> for PersistentValue {
    fn from(value: &str) -> Self {
        Self::String(value.to_string())
    }
}
impl From<PathBuf> for PersistentValue {
    fn from(value: PathBuf) -> Self {
        Self::String(value.to_string_lossy().to_string())
    }
}
impl From<Theme> for PersistentValue {
    fn from(value: Theme) -> Self {
        Self::Theme(value)
    }
}
impl From<Layout> for PersistentValue {
    fn from(value: Layout) -> Self {
        Self::Layout(value)
    }
}
impl From<WorkspaceTemplate> for PersistentValue {
    fn from(value: WorkspaceTemplate) -> Self {
        Self::WorkspaceTemplate(value)
    }
}
impl From<ImageFormat> for PersistentValue {
    fn from(value: ImageFormat) -> Self {
        Self::ImageFormat(value)
    }
}

pub trait PersistentKey {
    fn get_id(&self) -> &str;
    fn with_id(&self, other: &impl PersistentKey) -> String {
        format!("{}-{}", self.get_id(), other.get_id())
    }
}

impl PersistentKey for String {
    fn get_id(&self) -> &str {
        self
    }
}

impl PersistentKey for &str {
    fn get_id(&self) -> &str {
        self
    }
}

pub type PersistentDatabase = HashMap<String, HashMap<String, PersistentValue>>;

/// Persistent storage for any data meant to be stored between program runs
///
/// # Warning
/// All values in the cache are saved to drive when the cache is dropped.
/// This provides automatic saving of the cache but requires care when insantiating and dropping it, potentially losing data if done incorrectly.
/// General expectation is that there will be only one cache object alive at a time created at start of the program and it will be dropped when the program closes.
#[derive(Default, Debug)]
pub struct Persistence {
    db: PersistentDatabase,
}

impl Persistence {
    /// Returns a borrowed value if present
    pub fn get(&self, id: impl PersistentKey, key: impl PersistentKey) -> Option<&PersistentValue> {
        self.db.get(id.get_id()).and_then(|x| x.get(key.get_id()))
    }
    /// Returns a cloned value if it's present
    pub fn get_copy(
        &self,
        id: impl PersistentKey,
        key: impl PersistentKey,
    ) -> Option<PersistentValue> {
        self.db
            .get(id.get_id())
            .and_then(|x| x.get(key.get_id()))
            .cloned()
    }
    /// Stores the value in cache, if the cache for this id and key wasn't present, it creates it
    pub fn set(
        &mut self,
        id: impl PersistentKey,
        key: impl PersistentKey,
        value: impl Into<PersistentValue>,
    ) {
        match self.db.get_mut(id.get_id()) {
            Some(m) => {
                m.insert(key.get_id().to_string(), value.into());
            }
            None => {
                let mut m = HashMap::new();
                m.insert(key.get_id().to_string(), value.into());
                self.db.insert(id.get_id().to_string(), m);
            }
        }
    }
    /// Loads cache from drive
    pub fn load() -> Self {
        let path = Persistence::cache_file();
        if path.exists() == false {
            return Self {
                db: PersistentDatabase::new(),
            };
        }
        let s = std::fs::read(path).unwrap();
        Self {
            db: ron::de::from_bytes::<PersistentDatabase>(&s).unwrap(),
        }
    }
    /// Saves cache to drive
    pub fn save(&self) {
        let s = ron::to_string(&self.db).unwrap();
        let path = Persistence::cache_file();
        std::fs::write(path, s).unwrap();
    }
    /// Gets path to cache file, it also makes sure the folder leading to the file is present
    pub fn cache_file() -> PathBuf {
        let mut path = dirs::cache_dir().unwrap();
        path.push("token-maker");
        if path.exists() == false {
            std::fs::create_dir_all(&path).unwrap();
        }
        path.push("cache");
        path.set_extension("ron");
        path
    }
}

impl Drop for Persistence {
    fn drop(&mut self) {
        self.save();
    }
}
