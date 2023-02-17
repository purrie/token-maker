mod file_browser;
mod token_maker;
mod workspace;
mod math;
mod data;
mod frame;
mod image;
mod trackpad;

use iced::{Application, Settings};
use token_maker::TokenMaker;

fn main() {
    TokenMaker::run(Settings {
        default_text_size: 24,
        ..Default::default()
    })
    .unwrap()
}
