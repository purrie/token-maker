mod file_browser;
mod token_maker;
mod workspace;
mod modifier;
mod math;
mod data;

use iced::{Application, Settings};
use token_maker::TokenMaker;

fn main() {
    TokenMaker::run(Settings {
        default_text_size: 24,
        ..Default::default()
    })
    .unwrap()
}
