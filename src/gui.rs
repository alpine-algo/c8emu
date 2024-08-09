mod rom_loader;

use crate::cpu::Cpu;
use crate::gui::rom_loader::RomLoader;
use iced::{Application, Command, Element, Subscription, Theme};
use log::error;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub enum Message {
    CpuTick,
    DisplayTick,
    RomLoader(rom_loader::Message),
}

pub struct Gui {
    cpu: Cpu,
    last_cpu_update: Instant,
    last_display_update: Instant,
    cpu_hz: u64,
    display_hz: u64,
    rom_loader: RomLoader,
}

impl Application for Gui {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Flags = ();
    type Theme = Theme;

    fn new(_flags: ()) -> (Self, Command<Message>) {
        (
            Self {
                cpu: Cpu::new(),
                last_cpu_update: Instant::now(),
                last_display_update: Instant::now(),
                cpu_hz: 1, //500,
                display_hz: 60,
                rom_loader: RomLoader::new(),
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("CHIP-8 Emulator")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::CpuTick => {
                let now = Instant::now();
                let elapsed = now.duration_since(self.last_cpu_update);
                if elapsed >= Duration::from_secs_f64(1.0 / self.cpu_hz as f64) {
                    self.cpu.cpu_exec();
                    self.last_cpu_update = now;
                }
            }
            Message::DisplayTick => {
                let now = Instant::now();
                let elapsed = now.duration_since(self.last_display_update);
                if elapsed >= Duration::from_secs_f64(1.0 / self.display_hz as f64) {
                    // Update display here
                    self.last_display_update = now;
                }
            }
            Message::RomLoader(msg) => match self.rom_loader.update(msg) {
                rom_loader::RomLoaderResult::None => {}
                rom_loader::RomLoaderResult::LoadRom(path) => match self.cpu.load_rom(&path) {
                    Ok(result) => self.rom_loader.update_bytes(result.bytes_read),
                    Err(e) => {
                        error!("Error loading ROM: {}", e);
                    }
                },
            },
        }
        Command::none()
    }

    fn view(&self) -> Element<Message> {
        // GUI layout here
        iced::widget::Column::new()
            .push(self.rom_loader.view().map(Message::RomLoader))
            .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch(vec![
            // 16 ms = ~60 Hz
            iced::time::every(Duration::from_millis(16)).map(|_| Message::CpuTick),
            iced::time::every(Duration::from_millis(16)).map(|_| Message::DisplayTick),
        ])
    }
}
