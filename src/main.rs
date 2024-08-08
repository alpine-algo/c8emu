mod cpu;

use cpu::Cpu;
use log::{debug, error, info, warn};

const INPUT_ROM: &str = "roms/test_opcode.ch8";

fn main() {
    env_logger::init();

    let mut cpu: Cpu = Cpu::new();

    if let Err(e) = cpu.load_rom(INPUT_ROM) {
        error!("Error loading ROM: {}", e);
        return;
    }

    for _ in 0..15 {
        cpu.cpu_exec();
    }
}
