#[derive(Debug, Clone)]
pub enum Message {}

pub struct Display {}

// CHIP-8 display is 64 x 32

impl Display {
    pub fn new() -> Self {
        Self {}
    }

    pub fn view(&self) -> iced::Element<Message> {
        iced::widget::column![].into()
    }

    pub fn update(&mut self, message: Message) {
        match message {
            _ => (),
        }
    }
}
