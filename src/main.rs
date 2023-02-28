mod cache;
mod data;
mod frame_maker;
mod image;
mod modifier;
mod style;
mod token_maker;
mod widgets;
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
