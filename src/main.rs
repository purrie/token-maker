mod data;
mod frame_maker;
mod image;
mod modifier;
mod naming_convention;
mod persistence;
mod status_bar;
mod style;
mod token_maker;
mod widgets;
mod workspace;

use iced::{Application, Settings};
use token_maker::TokenMaker;

fn main() {
    TokenMaker::run(Settings {
        default_text_size: 18.0,
        ..Default::default()
    })
    .unwrap()
}
