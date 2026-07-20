mod display;
mod rom_loader;

use crate::cpu::{Cpu, CpuFault};
use crate::gui::display::Display;
use crate::gui::rom_loader::RomLoader;
use iced::keyboard::Key;
use iced::{event, Application, Command, Element, Event, Subscription, Theme};
use log::error;
use std::time::Duration;

const TIMER_FREQUENCY_HZ: f64 = 60.0;
const INSTRUCTIONS_PER_TICK: usize = 10;

#[derive(Debug)]
enum RuntimeState {
    Idle,
    Running,
    Faulted(CpuFault),
}

#[derive(Debug, Clone)]
pub enum Message {
    Tick,
    Event(Event),
    RomLoader(rom_loader::Message),
    Display(display::Message),
}

pub struct Gui {
    cpu: Cpu,
    runtime: RuntimeState,
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
                runtime: RuntimeState::Idle,
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
                if matches!(&self.runtime, RuntimeState::Running) {
                    self.cpu.tick_timers();
                    for _ in 0..INSTRUCTIONS_PER_TICK {
                        if let Err(fault) = self.cpu.cpu_exec() {
                            self.runtime = RuntimeState::Faulted(fault);
                            if let RuntimeState::Faulted(fault) = &self.runtime {
                                error!("CPU execution fault: {}", fault);
                            }
                            break;
                        }
                    }
                }
                self.display.update(self.cpu.framebuffer());
            }
            Message::Event(event) => {
                if let Event::Keyboard(kbd_event) = event {
                    let (key, pressed) = match kbd_event {
                        iced::keyboard::Event::KeyPressed { key, .. } => (key, true),
                        iced::keyboard::Event::KeyReleased { key, .. } => (key, false),
                        _ => return Command::none(),
                    };

                    if matches!(&self.runtime, RuntimeState::Running) {
                        if let Some(c8_key) = map_key(key) {
                            self.cpu.set_keypad(c8_key, pressed);
                        }
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
                            self.runtime = RuntimeState::Running;
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
            iced::time::every(Duration::from_secs_f64(1.0 / TIMER_FREQUENCY_HZ))
                .map(|_| Message::Tick),
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

#[cfg(test)]
mod tests {
    use super::*;
    use iced::keyboard::{Location, Modifiers};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Once;

    const UNPRESSED_FAULT_PC: u16 = 0x204;
    const PRESSED_FAULT_PC: u16 = 0x206;
    const COUNTED_FAULT_OPCODE: u16 = 0x8128;

    struct FaultLogCounter;

    static FAULT_LOGGER: FaultLogCounter = FaultLogCounter;
    static FAULT_LOGGER_INIT: Once = Once::new();
    static COUNTED_FAULT_LOGS: AtomicUsize = AtomicUsize::new(0);

    impl log::Log for FaultLogCounter {
        fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
            metadata.level() <= log::Level::Error
        }

        fn log(&self, record: &log::Record<'_>) {
            if self.enabled(record.metadata())
                && record
                    .args()
                    .to_string()
                    .contains(&format!("{COUNTED_FAULT_OPCODE:#06X}"))
            {
                COUNTED_FAULT_LOGS.fetch_add(1, Ordering::Relaxed);
            }
        }

        fn flush(&self) {}
    }

    fn reset_counted_fault_logs() {
        FAULT_LOGGER_INIT.call_once(|| {
            log::set_logger(&FAULT_LOGGER).expect("install test logger");
            log::set_max_level(log::LevelFilter::Error);
        });
        COUNTED_FAULT_LOGS.store(0, Ordering::Relaxed);
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum StateKind {
        Idle,
        Running,
        Faulted,
    }

    #[derive(Debug, Clone, Copy)]
    enum Setup {
        Idle,
        RunningLoop,
        RunningFaultNext,
        Faulted,
    }

    #[derive(Debug, Clone, Copy)]
    enum Action {
        Tick,
        LoadSuccess,
        LoadFailure,
    }

    struct TransitionCase {
        name: &'static str,
        setup: Setup,
        action: Action,
        expected: StateKind,
    }

    fn new_gui() -> Gui {
        <Gui as Application>::new(()).0
    }

    fn send(gui: &mut Gui, message: Message) {
        let _ = gui.update(message);
    }

    fn tick(gui: &mut Gui) {
        send(gui, Message::Tick);
    }

    fn opcodes(values: &[u16]) -> Vec<u8> {
        values
            .iter()
            .flat_map(|opcode| opcode.to_be_bytes())
            .collect()
    }

    fn write_rom(name: &str, values: &[u16]) -> PathBuf {
        let path =
            std::env::temp_dir().join(format!("c8emu-gui-{}-{name}.ch8", std::process::id()));
        fs::write(&path, opcodes(values)).expect("write test ROM");
        path
    }

    fn remove_rom(path: &Path) {
        fs::remove_file(path).expect("remove test ROM");
    }

    fn load(gui: &mut Gui, path: &Path) {
        send(
            gui,
            Message::RomLoader(rom_loader::Message::RomPathChanged(
                path.to_string_lossy().into_owned(),
            )),
        );
        send(gui, Message::RomLoader(rom_loader::Message::LoadRom));
    }

    fn state_kind(gui: &Gui) -> StateKind {
        match &gui.runtime {
            RuntimeState::Idle => StateKind::Idle,
            RuntimeState::Running => StateKind::Running,
            RuntimeState::Faulted(_) => StateKind::Faulted,
        }
    }

    fn fault(gui: &Gui) -> &CpuFault {
        match &gui.runtime {
            RuntimeState::Faulted(fault) => fault,
            state => panic!("expected Faulted, got {state:?}"),
        }
    }

    fn invalid_opcode_fault_pc(gui: &Gui) -> u16 {
        match fault(gui) {
            CpuFault::InvalidOpcode { pc, .. } => *pc,
            other => panic!("expected invalid-opcode fault, got {other}"),
        }
    }

    fn prepare_timer_probe(gui: &mut Gui, path: &Path) {
        gui.cpu
            .load_rom(&path.to_string_lossy())
            .expect("install timer probe");
        gui.cpu.cpu_exec().expect("set timer value");
        gui.cpu.cpu_exec().expect("set delay timer");
    }

    fn assert_timer_probe_was_not_advanced(gui: &mut Gui) {
        gui.cpu.cpu_exec().expect("read delay timer");
        gui.cpu.cpu_exec().expect("branch on unchanged timer");
        let fault = gui.cpu.cpu_exec().expect_err("reach probe fault");
        match fault {
            CpuFault::InvalidOpcode { pc, .. } => assert_eq!(pc, 0x20A),
            other => panic!("expected invalid-opcode fault, got {other}"),
        }
    }

    fn setup_gui(setup: Setup, loop_path: &Path, fault_path: &Path) -> Gui {
        let mut gui = new_gui();
        match setup {
            Setup::Idle => {}
            Setup::RunningLoop => load(&mut gui, loop_path),
            Setup::RunningFaultNext => load(&mut gui, fault_path),
            Setup::Faulted => {
                load(&mut gui, fault_path);
                tick(&mut gui);
            }
        }
        gui
    }

    fn send_key(gui: &mut Gui, key: Key, pressed: bool) {
        let event = if pressed {
            iced::keyboard::Event::KeyPressed {
                key,
                location: Location::Standard,
                modifiers: Modifiers::empty(),
                text: None,
            }
        } else {
            iced::keyboard::Event::KeyReleased {
                key,
                location: Location::Standard,
                modifiers: Modifiers::empty(),
            }
        };
        send(gui, Message::Event(Event::Keyboard(event)));
    }

    fn key_probe(key: usize) -> [u16; 4] {
        [0x6000 | key as u16, 0xE09E, 0xFFFF, 0xFFFF]
    }

    #[test]
    fn lifecycle_transition_table() {
        let loop_path = write_rom("transitions-loop", &[0x1200]);
        let fault_path = write_rom("transitions-fault", &[0xFFFF]);
        let empty_path = write_rom("transitions-empty", &[]);
        let cases = [
            TransitionCase {
                name: "idle tick",
                setup: Setup::Idle,
                action: Action::Tick,
                expected: StateKind::Idle,
            },
            TransitionCase {
                name: "idle successful load",
                setup: Setup::Idle,
                action: Action::LoadSuccess,
                expected: StateKind::Running,
            },
            TransitionCase {
                name: "idle failed load",
                setup: Setup::Idle,
                action: Action::LoadFailure,
                expected: StateKind::Idle,
            },
            TransitionCase {
                name: "running successful tick",
                setup: Setup::RunningLoop,
                action: Action::Tick,
                expected: StateKind::Running,
            },
            TransitionCase {
                name: "running execution fault",
                setup: Setup::RunningFaultNext,
                action: Action::Tick,
                expected: StateKind::Faulted,
            },
            TransitionCase {
                name: "running successful replacement",
                setup: Setup::RunningLoop,
                action: Action::LoadSuccess,
                expected: StateKind::Running,
            },
            TransitionCase {
                name: "running failed replacement",
                setup: Setup::RunningLoop,
                action: Action::LoadFailure,
                expected: StateKind::Running,
            },
            TransitionCase {
                name: "faulted tick",
                setup: Setup::Faulted,
                action: Action::Tick,
                expected: StateKind::Faulted,
            },
            TransitionCase {
                name: "faulted successful replacement",
                setup: Setup::Faulted,
                action: Action::LoadSuccess,
                expected: StateKind::Running,
            },
            TransitionCase {
                name: "faulted failed replacement",
                setup: Setup::Faulted,
                action: Action::LoadFailure,
                expected: StateKind::Faulted,
            },
        ];

        for case in cases {
            let mut gui = setup_gui(case.setup, &loop_path, &fault_path);
            match case.action {
                Action::Tick => tick(&mut gui),
                Action::LoadSuccess => load(&mut gui, &loop_path),
                Action::LoadFailure => load(&mut gui, &empty_path),
            }
            assert_eq!(state_kind(&gui), case.expected, "{}", case.name);
        }

        remove_rom(&loop_path);
        remove_rom(&fault_path);
        remove_rom(&empty_path);
    }

    #[test]
    fn idle_and_faulted_ticks_perform_zero_cpu_work() {
        let timer_path = write_rom(
            "non-running-timer",
            &[0x6002, 0xF015, 0xF107, 0x3102, 0xFFFF, 0xFFFF],
        );
        let fault_path = write_rom("non-running-fault", &[0xFFFF]);

        let mut idle = new_gui();
        prepare_timer_probe(&mut idle, &timer_path);
        tick(&mut idle);
        assert_eq!(state_kind(&idle), StateKind::Idle);
        assert_timer_probe_was_not_advanced(&mut idle);

        let mut faulted = new_gui();
        load(&mut faulted, &fault_path);
        tick(&mut faulted);
        let retained_diagnostic = fault(&faulted).to_string();
        prepare_timer_probe(&mut faulted, &timer_path);
        tick(&mut faulted);
        assert_eq!(fault(&faulted).to_string(), retained_diagnostic);
        assert_timer_probe_was_not_advanced(&mut faulted);

        remove_rom(&timer_path);
        remove_rom(&fault_path);
    }

    #[test]
    fn execution_fault_is_retained_until_a_clean_restart() {
        reset_counted_fault_logs();
        let fault_path = write_rom("fault-retention", &[COUNTED_FAULT_OPCODE]);
        let empty_path = write_rom("fault-retention-empty", &[]);
        let loop_path = write_rom("fault-retention-loop", &[0x1200]);
        let mut gui = new_gui();

        load(&mut gui, &fault_path);
        tick(&mut gui);
        assert_eq!(invalid_opcode_fault_pc(&gui), 0x200);
        let first_diagnostic = fault(&gui).to_string();
        assert_eq!(COUNTED_FAULT_LOGS.load(Ordering::Relaxed), 1);

        tick(&mut gui);
        assert_eq!(fault(&gui).to_string(), first_diagnostic);
        assert_eq!(COUNTED_FAULT_LOGS.load(Ordering::Relaxed), 1);
        load(&mut gui, &empty_path);
        assert_eq!(fault(&gui).to_string(), first_diagnostic);

        load(&mut gui, &loop_path);
        assert_eq!(state_kind(&gui), StateKind::Running);
        tick(&mut gui);
        assert_eq!(state_kind(&gui), StateKind::Running);

        remove_rom(&fault_path);
        remove_rom(&empty_path);
        remove_rom(&loop_path);
    }

    #[test]
    fn running_tick_has_exact_instruction_and_timer_budgets() {
        let mut faults_on_tenth = vec![0x7000; 9];
        faults_on_tenth.push(0xFFFF);
        let tenth_path = write_rom("budget-tenth", &faults_on_tenth);
        let mut gui = new_gui();
        load(&mut gui, &tenth_path);
        tick(&mut gui);
        assert_eq!(invalid_opcode_fault_pc(&gui), 0x212);

        let mut faults_on_eleventh = vec![0x7000; 10];
        faults_on_eleventh.push(0xFFFF);
        let eleventh_path = write_rom("budget-eleventh", &faults_on_eleventh);
        let mut gui = new_gui();
        load(&mut gui, &eleventh_path);
        tick(&mut gui);
        assert_eq!(state_kind(&gui), StateKind::Running);
        tick(&mut gui);
        assert_eq!(invalid_opcode_fault_pc(&gui), 0x214);

        let mut timer_probe = vec![0x6002, 0xF015];
        timer_probe.extend([0x7000; 8]);
        timer_probe.extend([0xF107, 0x3101, 0xFFFF, 0xFFFF]);
        let timer_path = write_rom("budget-timer", &timer_probe);
        let mut gui = new_gui();
        load(&mut gui, &timer_path);
        tick(&mut gui);
        assert_eq!(state_kind(&gui), StateKind::Running);
        tick(&mut gui);
        assert_eq!(invalid_opcode_fault_pc(&gui), 0x21A);

        remove_rom(&tenth_path);
        remove_rom(&eleventh_path);
        remove_rom(&timer_path);
    }

    #[test]
    fn failed_replacement_preserves_the_running_session() {
        let mut values = vec![0x7000; 10];
        values.push(0xFFFF);
        let session_path = write_rom("failed-replacement-session", &values);
        let empty_path = write_rom("failed-replacement-empty", &[]);
        let mut gui = new_gui();

        load(&mut gui, &session_path);
        tick(&mut gui);
        assert_eq!(state_kind(&gui), StateKind::Running);

        load(&mut gui, &empty_path);
        assert_eq!(state_kind(&gui), StateKind::Running);
        tick(&mut gui);
        assert_eq!(invalid_opcode_fault_pc(&gui), 0x214);

        remove_rom(&session_path);
        remove_rom(&empty_path);
    }

    #[test]
    fn fixed_keypad_mapping_routes_events_and_clean_load_resets_keys() {
        let mappings = [
            ("1", 0x1),
            ("2", 0x2),
            ("3", 0x3),
            ("4", 0xC),
            ("q", 0x4),
            ("Q", 0x4),
            ("w", 0x5),
            ("W", 0x5),
            ("e", 0x6),
            ("E", 0x6),
            ("r", 0xD),
            ("R", 0xD),
            ("a", 0x7),
            ("A", 0x7),
            ("s", 0x8),
            ("S", 0x8),
            ("d", 0x9),
            ("D", 0x9),
            ("f", 0xE),
            ("F", 0xE),
            ("z", 0xA),
            ("Z", 0xA),
            ("x", 0x0),
            ("X", 0x0),
            ("c", 0xB),
            ("C", 0xB),
            ("v", 0xF),
            ("V", 0xF),
        ];
        let probe_path = write_rom("keypad-probe", &key_probe(0));

        for (character, keypad_index) in mappings {
            let key = Key::Character(character.into());
            assert_eq!(map_key(key.clone()), Some(keypad_index));
            fs::write(&probe_path, opcodes(&key_probe(keypad_index))).expect("rewrite key probe");

            let mut pressed = new_gui();
            load(&mut pressed, &probe_path);
            send_key(&mut pressed, key.clone(), true);
            tick(&mut pressed);
            assert_eq!(
                invalid_opcode_fault_pc(&pressed),
                PRESSED_FAULT_PC,
                "press {character}"
            );

            let mut released = new_gui();
            load(&mut released, &probe_path);
            send_key(&mut released, key.clone(), true);
            send_key(&mut released, key, false);
            tick(&mut released);
            assert_eq!(
                invalid_opcode_fault_pc(&released),
                UNPRESSED_FAULT_PC,
                "release {character}"
            );
        }

        for key in [Key::Character("g".into()), Key::Unidentified] {
            assert_eq!(map_key(key.clone()), None);
            fs::write(&probe_path, opcodes(&key_probe(0))).expect("rewrite key probe");
            let mut gui = new_gui();
            load(&mut gui, &probe_path);
            send_key(&mut gui, key, true);
            tick(&mut gui);
            assert_eq!(invalid_opcode_fault_pc(&gui), UNPRESSED_FAULT_PC);
        }

        let loop_path = write_rom("keypad-reset-loop", &[0x1200]);
        fs::write(&probe_path, opcodes(&key_probe(0x4))).expect("rewrite key probe");
        let mut gui = new_gui();
        load(&mut gui, &loop_path);
        send_key(&mut gui, Key::Character("q".into()), true);
        load(&mut gui, &probe_path);
        tick(&mut gui);
        assert_eq!(invalid_opcode_fault_pc(&gui), UNPRESSED_FAULT_PC);

        remove_rom(&probe_path);
        remove_rom(&loop_path);
    }

    #[test]
    fn idle_and_faulted_states_do_not_route_key_events() {
        let probe_path = write_rom("keypad-lifecycle-gate", &key_probe(0x4));
        let path = probe_path.to_string_lossy();

        let mut idle = new_gui();
        idle.cpu.load_rom(&path).expect("install probe");
        send_key(&mut idle, Key::Character("q".into()), true);
        idle.runtime = RuntimeState::Running;
        tick(&mut idle);
        assert_eq!(invalid_opcode_fault_pc(&idle), UNPRESSED_FAULT_PC);

        let mut faulted = new_gui();
        faulted.cpu.load_rom(&path).expect("install probe");
        faulted.runtime = RuntimeState::Faulted(CpuFault::InvalidOpcode {
            pc: 0x200,
            opcode: 0xFFFF,
        });
        send_key(&mut faulted, Key::Character("q".into()), true);
        faulted.runtime = RuntimeState::Running;
        tick(&mut faulted);
        assert_eq!(invalid_opcode_fault_pc(&faulted), UNPRESSED_FAULT_PC);

        remove_rom(&probe_path);
    }
}
