mod cpu;
mod gui;

use crate::gui::Gui;
use iced::{Application, Settings};

fn main() -> iced::Result {
    env_logger::init();
    Gui::run(Settings::default())
}
