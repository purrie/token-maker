use std::path::PathBuf;

use crate::{file_browser::Browser, math::Vec2u};

#[derive(Default)]
pub struct Data {
    pub file: Browser,
    pub output: PathBuf,
}


#[derive(Clone, Debug)]
pub struct OutputOptions {
    pub size: Vec2u,
}

impl Default for OutputOptions {
    fn default() -> Self {
        Self { size: Vec2u { x: 512, y: 512 } }
    }
}
