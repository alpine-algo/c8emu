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
        let content = iced::widget::Row::new()
            .spacing(10)
            .align_items(iced::Alignment::Center)
            .push(iced::widget::Text::new("Load ROM: "))
            .push(
                iced::widget::TextInput::new("Enter ROM Path", &self.rom_path)
                    .on_input(Message::RomPathChanged),
            )
            .push(
                iced::widget::Button::new("Load")
                    .on_press(Message::LoadRom)
                    .padding(15),
            );

        let cols = iced::widget::column![
            content,
            if self.read_status {
                iced::widget::Text::new(format!(
                    "*Successfuly read {} bytes from ROM file.",
                    self.size_bytes
                ))
            } else {
                iced::widget::Text::new("*Error reading ROM file. Please check file path.")
            }
        ];

        let container = iced::widget::Container::new(cols).padding(15);

        container.into()
    }
}
