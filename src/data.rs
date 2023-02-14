use std::path::PathBuf;

use crate::file_browser::Browser;

#[derive(Default)]
pub struct Data {
    pub file: Browser,
    pub output: PathBuf,
}
