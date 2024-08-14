mod display;
mod rom_loader;

use crate::cpu::Cpu;
use crate::gui::display::Display;
use crate::gui::rom_loader::RomLoader;
use iced::{Application, Command, Element, Subscription, Theme};
use log::error;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub enum Message {
    CpuTick,
    DisplayTick,
    RomLoader(rom_loader::Message),
    Display(display::Message),
}

pub struct Gui {
    cpu: Cpu,
    last_cpu_update: Instant,
    last_display_update: Instant,
    cpu_hz: u64,
    display_hz: u64,
    rom_loader: RomLoader,
    display: Display,
    count: u32,
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
                cpu_hz: 1,      //500,
                display_hz: 60, // 60
                rom_loader: RomLoader::new(),
                display: Display::new(),
                count: 0,
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("CHIP-8 Emulator - github/alpine-algo")
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
                    self.cpu.set_display(self.count as usize % 64, 10, true);
                    self.count += 1;

                    self.display.update(self.cpu.get_display()); // Update display buffer on display tick
                    self.last_display_update = now;
                }
            }
            Message::RomLoader(msg) => match msg {
                rom_loader::Message::RomPathChanged(path) => {
                    self.rom_loader.rom_path = path;
                }
                rom_loader::Message::LoadRom => {
                    match self.cpu.load_rom(&self.rom_loader.rom_path) {
                        Ok(result) => {
                            self.rom_loader.size_bytes = result.bytes_read;
                            self.rom_loader.read_status = true;
                        }
                        Err(e) => {
                            self.rom_loader.read_status = false;
                            error!("Error loading ROM: {}", e)
                        }
                    }
                }
            },
            Message::Display(msg) => match msg {
                _ => {}
            },
        }
        Command::none()
    }

    fn view(&self) -> Element<Message> {
        // GUI layout here
        iced::widget::Column::new()
            .push(self.rom_loader.view().map(Message::RomLoader))
            .push(self.display.view().map(Message::Display))
            .padding(15)
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
