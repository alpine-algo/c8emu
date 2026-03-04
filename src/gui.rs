mod display;
mod rom_loader;

use crate::cpu::Cpu;
use crate::gui::display::Display;
use crate::gui::rom_loader::RomLoader;
use iced::keyboard::Key;
use iced::{event, Application, Command, Element, Event, Subscription, Theme};
use log::error;
use std::time::Duration;

#[derive(Debug, Clone)]
pub enum Message {
    Tick,
    Event(Event),
    RomLoader(rom_loader::Message),
    Display(display::Message),
}

pub struct Gui {
    cpu: Cpu,
    cpu_speed: u32, // Instructions per 60Hz tick
    rom_loader: RomLoader,
    display: Display,
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
                cpu_speed: 10, // ~600 Hz
                rom_loader: RomLoader::new(),
                display: Display::new(),
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("CHIP-8 Emulator - github/alpine-algo")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::Tick => {
                self.cpu.tick_timers();
                for _ in 0..self.cpu_speed {
                    self.cpu.cpu_exec();
                }
                self.display.update(self.cpu.get_display());
            }
            Message::Event(event) => {
                if let Event::Keyboard(kbd_event) = event {
                    let pressed = match kbd_event {
                        iced::keyboard::Event::KeyPressed { .. } => true,
                        iced::keyboard::Event::KeyReleased { .. } => false,
                        _ => return Command::none(),
                    };

                    let key = match kbd_event {
                        iced::keyboard::Event::KeyPressed { key, .. } => key,
                        iced::keyboard::Event::KeyReleased { key, .. } => key,
                        _ => return Command::none(),
                    };

                    if let Some(c8_key) = map_key(key) {
                        self.cpu.set_keypad(c8_key, pressed);
                    }
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
                            self.read_status_ok();
                        }
                        Err(e) => {
                            self.rom_loader.read_status = false;
                            error!("Error loading ROM: {}", e)
                        }
                    }
                }
            },
            Message::Display(_) => {}
        }
        Command::none()
    }

    fn view(&self) -> Element<'_, Message> {
        iced::widget::Column::new()
            .push(self.rom_loader.view().map(Message::RomLoader))
            .push(self.display.view().map(Message::Display))
            .padding(15)
            .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch(vec![
            iced::time::every(Duration::from_millis(16)).map(|_| Message::Tick),
            event::listen().map(Message::Event),
        ])
    }
}

impl Gui {
    fn read_status_ok(&mut self) {
        self.rom_loader.read_status = true;
    }
}

fn map_key(key: Key) -> Option<usize> {
    match key {
        Key::Character(s) => match s.as_str() {
            "1" => Some(0x1),
            "2" => Some(0x2),
            "3" => Some(0x3),
            "4" => Some(0xC),
            "q" | "Q" => Some(0x4),
            "w" | "W" => Some(0x5),
            "e" | "E" => Some(0x6),
            "r" | "R" => Some(0xD),
            "a" | "A" => Some(0x7),
            "s" | "S" => Some(0x8),
            "d" | "D" => Some(0x9),
            "f" | "F" => Some(0xE),
            "z" | "Z" => Some(0xA),
            "x" | "X" => Some(0x0),
            "c" | "C" => Some(0xB),
            "v" | "V" => Some(0xF),
            _ => None,
        },
        _ => None,
    }
}
