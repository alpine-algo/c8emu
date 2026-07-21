use super::RuntimeState;
use std::borrow::Cow;

#[derive(Debug, Clone)]
pub enum Message {
    RomPathChanged(String),
    LoadRom,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum LoadAttempt {
    NotAttempted,
    Succeeded { path: String, bytes: usize },
    Failed { path: String, reason: String },
}

pub struct RomLoader {
    rom_path: String,
    attempt: LoadAttempt,
}

impl RomLoader {
    pub fn new() -> Self {
        Self {
            rom_path: String::from("roms/test_opcode.ch8"),
            attempt: LoadAttempt::NotAttempted,
        }
    }

    pub(super) fn path(&self) -> &str {
        &self.rom_path
    }

    pub(super) fn set_path(&mut self, path: String) {
        self.rom_path = path;
    }

    pub(super) fn record_success(&mut self, bytes: usize) {
        self.attempt = LoadAttempt::Succeeded {
            path: self.rom_path.clone(),
            bytes,
        };
    }

    pub(super) fn record_failure(&mut self, reason: String) {
        self.attempt = LoadAttempt::Failed {
            path: self.rom_path.clone(),
            reason,
        };
    }

    pub(super) fn attempt(&self) -> &LoadAttempt {
        &self.attempt
    }

    pub(super) fn load_status_text(&self) -> Cow<'_, str> {
        match self.attempt() {
            LoadAttempt::NotAttempted => Cow::Borrowed("ROM load: No ROM loaded."),
            LoadAttempt::Succeeded { path, bytes } => Cow::Owned(format!(
                "ROM load: Successfully loaded {bytes} bytes from '{path}'."
            )),
            LoadAttempt::Failed { path, reason } => {
                Cow::Owned(format!("ROM load failed for '{path}': {reason}"))
            }
        }
    }

    pub(super) fn runtime_status_text(runtime: &RuntimeState) -> Cow<'static, str> {
        match runtime {
            RuntimeState::Idle => Cow::Borrowed("Runtime: Idle"),
            RuntimeState::Running => Cow::Borrowed("Runtime: Running"),
            RuntimeState::Faulted(fault) => {
                Cow::Owned(format!("Runtime: Faulted (stopped): {fault}"))
            }
        }
    }

    pub(super) fn view(&self, runtime: &RuntimeState) -> iced::Element<'_, Message> {
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
            iced::widget::Text::new(self.load_status_text()),
            iced::widget::Text::new(Self::runtime_status_text(runtime)),
        ];

        iced::widget::Container::new(cols).padding(15).into()
    }
}
