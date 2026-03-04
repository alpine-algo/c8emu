use log::{debug, info, warn};
use rand::Rng;
use std::fs::File;
use std::io::Read;
use thiserror::Error;

const BASE: usize = 0x200; // RAM (512) Base Program Memory
const END: usize = 0x1000; // RAM (4096) Memory End

const FONTS: [u8; 80] = [
    0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
    0x20, 0x60, 0x20, 0x20, 0x70, // 1
    0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
    0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
    0x90, 0x90, 0xF0, 0x10, 0x10, // 4
    0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
    0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
    0xF0, 0x10, 0x20, 0x40, 0x40, // 7
    0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
    0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
    0xF0, 0x90, 0xF0, 0x90, 0x90, // A
    0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
    0xF0, 0x80, 0x80, 0x80, 0xF0, // C
    0xE0, 0x90, 0x90, 0x90, 0xE0, // D
    0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
    0xF0, 0x80, 0xF0, 0x80, 0x80, // F
];

#[derive(Error, Debug)]
pub enum CpuError {
    #[error("Failed to open CHIP-8 ROM file: {err}")]
    RomOpenError { err: std::io::Error },
    #[error("Failed to read CHIP-8 ROM file: {err}")]
    RomReadError { err: std::io::Error },
    #[error("CHIP-8 ROM too large for memory. Expected <= {max}, got {actual} bytes")]
    RomSizeError { max: usize, actual: usize },
}

pub struct Cpu {
    memory: [u8; END],
    v: [u8; 16],
    i: u16,
    pc: u16,
    stack: Vec<u16>,
    dt: u8,
    st: u8,
    keypad: [bool; 16],
    display: [[bool; 64]; 32],
    waiting_for_key: Option<usize>, // Register index waiting for key
}

pub struct RomLoadResult {
    pub bytes_read: usize,
}

impl Cpu {
    pub fn new() -> Self {
        let mut memory = [0; END];
        // Load fonts into memory starting at 0x000
        memory[0..80].copy_from_slice(&FONTS);

        Cpu {
            memory,
            v: [0; 16],
            i: 0,
            pc: BASE as u16,
            stack: Vec::with_capacity(16),
            dt: 0,
            st: 0,
            keypad: [false; 16],
            display: [[false; 64]; 32],
            waiting_for_key: None,
        }
    }

    pub fn reset(&mut self) {
        self.v = [0; 16];
        self.i = 0;
        self.pc = BASE as u16;
        self.stack.clear();
        self.dt = 0;
        self.st = 0;
        self.display = [[false; 64]; 32];
        self.waiting_for_key = None;
    }

    pub fn load_rom(&mut self, rom_file: &str) -> Result<RomLoadResult, CpuError> {
        let mut f = File::open(rom_file).map_err(|e| CpuError::RomOpenError { err: e })?;
        let mut buf = Vec::new();
        let bytes_read = f
            .read_to_end(&mut buf)
            .map_err(|e| CpuError::RomReadError { err: e })?;

        if bytes_read > END - BASE {
            return Err(CpuError::RomSizeError {
                max: END - BASE,
                actual: bytes_read,
            });
        }

        self.reset();
        self.memory[BASE..BASE + bytes_read].copy_from_slice(&buf);
        info!("Read {} bytes from CHIP-8 ROM '{}'", bytes_read, rom_file);

        Ok(RomLoadResult { bytes_read })
    }

    pub fn tick_timers(&mut self) {
        if self.dt > 0 {
            self.dt -= 1;
        }
        if self.st > 0 {
            self.st -= 1;
        }
    }

    pub fn is_beeping(&self) -> bool {
        self.st > 0
    }

    pub fn set_keypad(&mut self, key: usize, pressed: bool) {
        if key < 16 {
            self.keypad[key] = pressed;
            if pressed {
                if let Some(reg_idx) = self.waiting_for_key {
                    self.v[reg_idx] = key as u8;
                    self.waiting_for_key = None;
                }
            }
        }
    }

    fn next_instr(&self) -> u16 {
        let b1 = self.memory[self.pc as usize];
        let b2 = self.memory[(self.pc + 1) as usize];
        ((b1 as u16) << 8) | b2 as u16
    }

    pub fn cpu_exec(&mut self) {
        if self.waiting_for_key.is_some() {
            return;
        }

        let cmd = self.next_instr();
        let opcode = (cmd & 0xF000) >> 12;
        let x = ((cmd & 0x0F00) >> 8) as usize;
        let y = ((cmd & 0x00F0) >> 4) as usize;
        let nnn = cmd & 0x0FFF;
        let kk = (cmd & 0x00FF) as u8;
        let n = (cmd & 0x000F) as u8;

        self.pc += 2;

        match opcode {
            0x0 => match cmd {
                0x00E0 => {
                    self.display = [[false; 64]; 32];
                }
                0x00EE => {
                    if let Some(addr) = self.stack.pop() {
                        self.pc = addr;
                    } else {
                        warn!("Stack underflow on RET!");
                    }
                }
                _ => {
                    warn!("SYS instruction {:04X} ignored", cmd);
                }
            },
            0x1 => {
                self.pc = nnn;
            }
            0x2 => {
                self.stack.push(self.pc);
                self.pc = nnn;
            }
            0x3 => {
                if self.v[x] == kk {
                    self.pc += 2;
                }
            }
            0x4 => {
                if self.v[x] != kk {
                    self.pc += 2;
                }
            }
            0x5 => {
                if self.v[x] == self.v[y] {
                    self.pc += 2;
                }
            }
            0x6 => {
                self.v[x] = kk;
            }
            0x7 => {
                self.v[x] = self.v[x].wrapping_add(kk);
            }
            0x8 => match n {
                0x0 => self.v[x] = self.v[y],
                0x1 => self.v[x] |= self.v[y],
                0x2 => self.v[x] &= self.v[y],
                0x3 => self.v[x] ^= self.v[y],
                0x4 => {
                    let (res, overflow) = self.v[x].overflowing_add(self.v[y]);
                    self.v[x] = res;
                    self.v[0xF] = if overflow { 1 } else { 0 };
                }
                0x5 => {
                    let (res, overflow) = self.v[x].overflowing_sub(self.v[y]);
                    self.v[x] = res;
                    self.v[0xF] = if !overflow { 1 } else { 0 }; // VF=1 if NO borrow
                }
                0x6 => {
                    // Cowgod: VF is set to the least significant bit of VX before the shift
                    // Most programs use the COSMAC version: VX = VY >> 1
                    // But some use VX = VX >> 1. 
                    // Let's stick to the modern/standard CHIP-8 expectation where it shifts VX.
                    // Actually let's use the provided specs: "VF := VX & 0x01; VX := VX / 2"
                    self.v[0xF] = self.v[x] & 0x1;
                    self.v[x] >>= 1;
                }
                0x7 => {
                    let (res, overflow) = self.v[y].overflowing_sub(self.v[x]);
                    self.v[x] = res;
                    self.v[0xF] = if !overflow { 1 } else { 0 }; // VF=1 if NO borrow
                }
                0xE => {
                    self.v[0xF] = (self.v[x] >> 7) & 0x1;
                    self.v[x] <<= 1;
                }
                _ => warn!("Unknown 8xyN opcode: {:04X}", cmd),
            },
            0x9 => {
                if self.v[x] != self.v[y] {
                    self.pc += 2;
                }
            }
            0xA => {
                self.i = nnn;
            }
            0xB => {
                self.pc = nnn + self.v[0] as u16;
            }
            0xC => {
                let rand: u8 = rand::thread_rng().gen();
                self.v[x] = rand & kk;
            }
            0xD => {
                // DRW Vx, Vy, nibble
                let start_x = (self.v[x] % 64) as usize;
                let start_y = (self.v[y] % 32) as usize;
                self.v[0xF] = 0;

                for row in 0..n as usize {
                    if start_y + row >= 32 {
                        break;
                    }
                    let sprite_byte = self.memory[self.i as usize + row];
                    for col in 0..8 {
                        if start_x + col >= 64 {
                            break;
                        }
                        let sprite_pixel = (sprite_byte >> (7 - col)) & 1;
                        if sprite_pixel == 1 {
                            if self.display[start_y + row][start_x + col] {
                                self.v[0xF] = 1;
                                self.display[start_y + row][start_x + col] = false;
                            } else {
                                self.display[start_y + row][start_x + col] = true;
                            }
                        }
                    }
                }
            }
            0xE => match kk {
                0x9E => {
                    if self.keypad[self.v[x] as usize & 0xF] {
                        self.pc += 2;
                    }
                }
                0xA1 => {
                    if !self.keypad[self.v[x] as usize & 0xF] {
                        self.pc += 2;
                    }
                }
                _ => warn!("Unknown ExNN opcode: {:04X}", cmd),
            },
            0xF => match kk {
                0x07 => self.v[x] = self.dt,
                0x0A => {
                    self.waiting_for_key = Some(x);
                }
                0x15 => self.dt = self.v[x],
                0x18 => self.st = self.v[x],
                0x1E => {
                    self.i = self.i.wrapping_add(self.v[x] as u16);
                }
                0x29 => {
                    self.i = (self.v[x] as u16 & 0xF) * 5;
                }
                0x33 => {
                    let val = self.v[x];
                    self.memory[self.i as usize] = val / 100;
                    self.memory[self.i as usize + 1] = (val / 10) % 10;
                    self.memory[self.i as usize + 2] = val % 10;
                }
                0x55 => {
                    for idx in 0..=x {
                        self.memory[self.i as usize + idx] = self.v[idx];
                    }
                }
                0x65 => {
                    for idx in 0..=x {
                        self.v[idx] = self.memory[self.i as usize + idx];
                    }
                }
                _ => warn!("Unknown FxNN opcode: {:04X}", cmd),
            },
            _ => warn!("Unknown opcode: {:04X}", cmd),
        }
    }

    pub fn get_display(&self) -> [[bool; 64]; 32] {
        self.display
    }
}
