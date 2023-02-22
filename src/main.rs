mod data;
mod file_browser;
mod image;
mod modifier;
mod token_maker;
mod trackpad;
mod workspace;

use iced::{Application, Settings};
use token_maker::TokenMaker;

fn main() {
    TokenMaker::run(Settings {
        default_text_size: 20.0,
        ..Default::default()
    })
    .unwrap()
}
