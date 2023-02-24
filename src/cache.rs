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
    pub fn check_string(&self) -> Option<&str> {
        match self {
            CacheValue::String(s) => Some(s),
            _ => None,
        }
    }
    pub fn to_string(self) -> String {
        match self {
            Self::String(x) => x,
            _ => String::new(),
        }
    }
    pub fn to_theme(self) -> Theme {
        match self {
            Self::Theme(x) => x,
            _ => Theme::default(),
        }
    }
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

#[derive(Default, Debug)]
pub struct Cache {
    cache: CacheMap,
}

impl Cache {
    pub fn get(&self, id: &str, key: &str) -> Option<&CacheValue> {
        self.cache.get(id).and_then(|x| x.get(key))
    }
    pub fn get_copy(&self, id: &str, key: &str) -> Option<CacheValue> {
        self.cache.get(id).and_then(|x| x.get(key)).cloned()
    }
    pub fn set(&mut self, id: &str, key: String, value: CacheValue) {
        match self.cache.get_mut(id) {
            Some(m) => {
                m.insert(key, value);
            }
            None => {
                let mut m = HashMap::new();
                m.insert(key, value);
                self.cache.insert(id.to_string(), m);
            }
        }
    }
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
    pub fn save(&self) {
        let s = ron::to_string(&self.cache).unwrap();
        let path = Cache::cache_file();
        std::fs::write(path, s).unwrap();
    }
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
