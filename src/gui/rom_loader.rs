#[derive(Debug, Clone)]
pub enum Message {
    RomPathChanged(String),
    LoadRom,
}

pub enum RomLoaderResult {
    None,
    LoadRom(String),
}

pub struct RomLoader {
    rom_path: String,
    size_bytes: usize,
}

impl RomLoader {
    pub fn new() -> Self {
        Self {
            rom_path: String::from("roms/test_opcode.ch8"),
            size_bytes: 0,
        }
    }

    pub fn view(&self) -> iced::Element<Message> {
        iced::widget::column![
            iced::widget::Text::new("Load ROM"),
            iced::widget::TextInput::new("Enter ROM Path", &self.rom_path)
                .on_input(Message::RomPathChanged),
            iced::widget::Button::new("Load").on_press(Message::LoadRom),
            iced::widget::Text::new(format!("ROM Size: {} bytes", self.size_bytes))
        ]
        .into()
    }

    pub fn update(&mut self, message: Message) -> RomLoaderResult {
        match message {
            Message::RomPathChanged(new_path) => {
                self.rom_path = new_path;
                RomLoaderResult::None
            }
            Message::LoadRom => RomLoaderResult::LoadRom(self.rom_path.clone()),
        }
    }

    pub fn update_bytes(&mut self, size: usize) {
        self.size_bytes = size;
    }
}
