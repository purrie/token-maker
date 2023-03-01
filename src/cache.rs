use std::{collections::HashMap, path::PathBuf};

use serde::{Deserialize, Serialize};

use crate::style::{Layout, Theme};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CacheValue {
    String(String),
    Theme(Theme),
    Layout(Layout),
}

impl CacheValue {
    /// Checks the string value if the type of `CacheValue` is a string.
    pub fn check_string(&self) -> Option<&str> {
        match self {
            CacheValue::String(s) => Some(s),
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

impl From<String> for CacheValue {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}
impl From<&str> for CacheValue {
    fn from(value: &str) -> Self {
        Self::String(value.to_string())
    }
}
impl From<PathBuf> for CacheValue {
    fn from(value: PathBuf) -> Self {
        Self::String(value.to_string_lossy().to_string())
    }
}
impl From<Theme> for CacheValue {
    fn from(value: Theme) -> Self {
        Self::Theme(value)
    }
}
impl From<Layout> for CacheValue {
    fn from(value: Layout) -> Self {
        Self::Layout(value)
    }
}

pub type CacheMap = HashMap<String, HashMap<String, CacheValue>>;

/// Persistent storage for any data meant to be stored between program runs
///
/// # Warning
/// All values in the cache are saved to drive when the cache is dropped.
/// This provides automatic saving of the cache but requires care when insantiating and dropping it, potentially losing data if done incorrectly.
/// General expectation is that there will be only one cache object alive at a time created at start of the program and it will be dropped when the program closes.
#[derive(Default, Debug)]
pub struct Cache {
    cache: CacheMap,
}

impl Cache {
    /// Returns a borrowed value if present
    pub fn get(&self, id: &str, key: &str) -> Option<&CacheValue> {
        self.cache.get(id).and_then(|x| x.get(key))
    }
    /// Returns a cloned value if it's present
    pub fn get_copy(&self, id: &str, key: &str) -> Option<CacheValue> {
        self.cache.get(id).and_then(|x| x.get(key)).cloned()
    }
    /// Stores the value in cache, if the cache for this id and key wasn't present, it creates it
    pub fn set<C: Into<CacheValue>>(&mut self, id: &str, key: String, value: C) {
        match self.cache.get_mut(id) {
            Some(m) => {
                m.insert(key, value.into());
            }
            None => {
                let mut m = HashMap::new();
                m.insert(key, value.into());
                self.cache.insert(id.to_string(), m);
            }
        }
    }
    /// Loads cache from drive
    pub fn load() -> Self {
        let path = Cache::cache_file();
        if path.exists() == false {
            return Self {
                cache: CacheMap::new(),
            };
        }
        let s = std::fs::read(path).unwrap();
        Self {
            cache: ron::de::from_bytes::<CacheMap>(&s).unwrap(),
        }
    }
    /// Saves cache to drive
    pub fn save(&self) {
        let s = ron::to_string(&self.cache).unwrap();
        let path = Cache::cache_file();
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

impl Drop for Cache {
    fn drop(&mut self) {
        self.save();
    }
}
