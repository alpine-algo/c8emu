#[derive(Debug, Clone)]
pub enum Message {
    RomPathChanged(String),
    LoadRom,
}

pub struct RomLoader {
    pub rom_path: String,
    pub size_bytes: usize,
    pub read_status: bool,
}

impl RomLoader {
    pub fn new() -> Self {
        Self {
            rom_path: String::from("roms/test_opcode.ch8"),
            size_bytes: 0,
            read_status: false,
        }
    }

    pub fn view(&self) -> iced::Element<Message> {
        let content = iced::widget::row![
            iced::widget::Text::new("Load ROM: "),
            iced::widget::TextInput::new("Enter ROM Path", &self.rom_path)
                .on_input(Message::RomPathChanged),
            iced::widget::Button::new("Load")
                .on_press(Message::LoadRom)
                .padding(15),
        ]
        .spacing(10)
        .align_items(iced::Alignment::Center);

        let cols = iced::widget::column![
            content,
            if self.read_status {
                iced::widget::Text::new(format!(
                    "*Successfuly loaded {} bytes from ROM file.",
                    self.size_bytes
                ))
            } else if self.size_bytes == 0 {
                iced::widget::Text::new("*Please load a ROM file.")
            } else {
                iced::widget::Text::new("*Error loading ROM file. Please check file path.")
            }
        ];

        let container = iced::widget::Container::new(cols).padding(15);

        container.into()
    }
}
