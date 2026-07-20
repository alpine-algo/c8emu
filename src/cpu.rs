use log::info;
use rand::Rng;
use std::fs::File;
use std::io::Read;
use thiserror::Error;

const PROGRAM_START: usize = 0x200;
const MEMORY_SIZE: usize = 0x1000;
const STACK_SIZE: usize = 16;

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
pub enum CpuFault {
    #[error("failed to open CHIP-8 ROM file: {source}")]
    RomOpen {
        #[source]
        source: std::io::Error,
    },
    #[error("failed to read CHIP-8 ROM file: {source}")]
    RomRead {
        #[source]
        source: std::io::Error,
    },
    #[error("CHIP-8 ROM must not be empty")]
    EmptyRom,
    #[error("CHIP-8 ROM too large for memory: expected at most {max}, got {actual} bytes")]
    RomTooLarge { max: usize, actual: usize },
    #[error("cannot fetch an opcode at PC {pc:#05X}")]
    InvalidFetch { pc: u16 },
    #[error("invalid opcode {opcode:#06X} at PC {pc:#05X}")]
    InvalidOpcode { pc: u16, opcode: u16 },
    #[error("stack underflow executing {opcode:#06X} at PC {pc:#05X}")]
    StackUnderflow { pc: u16, opcode: u16 },
    #[error("stack overflow executing {opcode:#06X} at PC {pc:#05X}")]
    StackOverflow { pc: u16, opcode: u16 },
    #[error(
        "invalid program-counter target {target:#05X} from opcode {opcode:#06X} at PC {pc:#05X}"
    )]
    ProgramCounterOutOfBounds { pc: u16, opcode: u16, target: usize },
    #[error(
        "memory range {start:#05X}..+{length} is out of bounds for opcode {opcode:#06X} at PC {pc:#05X}"
    )]
    MemoryOutOfBounds {
        pc: u16,
        opcode: u16,
        start: usize,
        length: usize,
    },
    #[error("index target {index:#05X} is out of bounds for opcode {opcode:#06X} at PC {pc:#05X}")]
    IndexOutOfBounds { pc: u16, opcode: u16, index: usize },
    #[error("key value {key:#04X} is out of bounds for opcode {opcode:#06X} at PC {pc:#05X}")]
    KeyOutOfBounds { pc: u16, opcode: u16, key: u8 },
}

pub struct Cpu {
    memory: [u8; MEMORY_SIZE],
    v: [u8; 16],
    i: u16,
    pc: u16,
    stack: [u16; STACK_SIZE],
    stack_depth: usize,
    dt: u8,
    st: u8,
    keypad: [bool; 16],
    display: [[bool; 64]; 32],
    waiting_for_key: Option<usize>,
}

pub struct RomLoadResult {
    pub bytes_read: usize,
}

impl Cpu {
    pub fn new() -> Self {
        let mut memory = [0; MEMORY_SIZE];
        memory[..FONTS.len()].copy_from_slice(&FONTS);

        Self {
            memory,
            v: [0; 16],
            i: 0,
            pc: PROGRAM_START as u16,
            stack: [0; STACK_SIZE],
            stack_depth: 0,
            dt: 0,
            st: 0,
            keypad: [false; 16],
            display: [[false; 64]; 32],
            waiting_for_key: None,
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }

    pub fn load_rom(&mut self, rom_file: &str) -> Result<RomLoadResult, CpuFault> {
        let mut file = File::open(rom_file).map_err(|source| CpuFault::RomOpen { source })?;
        let mut rom = Vec::new();
        file.read_to_end(&mut rom)
            .map_err(|source| CpuFault::RomRead { source })?;

        let result = self.install_rom(&rom)?;
        info!(
            "Read {} bytes from CHIP-8 ROM '{}'",
            result.bytes_read, rom_file
        );
        Ok(result)
    }

    fn install_rom(&mut self, rom: &[u8]) -> Result<RomLoadResult, CpuFault> {
        if rom.is_empty() {
            return Err(CpuFault::EmptyRom);
        }

        let max = MEMORY_SIZE - PROGRAM_START;
        if rom.len() > max {
            return Err(CpuFault::RomTooLarge {
                max,
                actual: rom.len(),
            });
        }

        self.reset();
        self.memory[PROGRAM_START..PROGRAM_START + rom.len()].copy_from_slice(rom);
        Ok(RomLoadResult {
            bytes_read: rom.len(),
        })
    }

    pub fn tick_timers(&mut self) {
        if self.dt > 0 {
            self.dt -= 1;
        }
        if self.st > 0 {
            self.st -= 1;
        }
    }

    pub fn set_keypad(&mut self, key: usize, pressed: bool) {
        if key >= self.keypad.len() {
            return;
        }

        self.keypad[key] = pressed;
        if pressed {
            if let Some(register) = self.waiting_for_key {
                self.v[register] = key as u8;
                self.waiting_for_key = None;
            }
        }
    }

    fn fetch_opcode(&self) -> Result<u16, CpuFault> {
        let pc = self.pc as usize;
        if pc > MEMORY_SIZE - 2 {
            return Err(CpuFault::InvalidFetch { pc: self.pc });
        }

        Ok(((self.memory[pc] as u16) << 8) | self.memory[pc + 1] as u16)
    }

    fn checked_pc_target(pc: u16, opcode: u16, target: usize) -> Result<u16, CpuFault> {
        if target > MEMORY_SIZE - 2 {
            return Err(CpuFault::ProgramCounterOutOfBounds { pc, opcode, target });
        }
        Ok(target as u16)
    }

    fn sequential_pc(pc: u16, opcode: u16, bytes: usize) -> Result<u16, CpuFault> {
        Self::checked_pc_target(pc, opcode, pc as usize + bytes)
    }

    fn checked_memory_start(
        pc: u16,
        opcode: u16,
        start: usize,
        length: usize,
    ) -> Result<usize, CpuFault> {
        match start.checked_add(length) {
            Some(end) if start < MEMORY_SIZE && end <= MEMORY_SIZE => Ok(start),
            _ => Err(CpuFault::MemoryOutOfBounds {
                pc,
                opcode,
                start,
                length,
            }),
        }
    }

    fn checked_index(pc: u16, opcode: u16, index: usize) -> Result<u16, CpuFault> {
        if index >= MEMORY_SIZE {
            return Err(CpuFault::IndexOutOfBounds { pc, opcode, index });
        }
        Ok(index as u16)
    }

    pub fn cpu_exec(&mut self) -> Result<(), CpuFault> {
        if self.waiting_for_key.is_some() {
            return Ok(());
        }

        let pc = self.pc;
        let opcode = self.fetch_opcode()?;
        let family = (opcode & 0xF000) >> 12;
        let x = ((opcode & 0x0F00) >> 8) as usize;
        let y = ((opcode & 0x00F0) >> 4) as usize;
        let nnn = opcode & 0x0FFF;
        let kk = (opcode & 0x00FF) as u8;
        let n = (opcode & 0x000F) as u8;

        match family {
            0x0 => match opcode {
                0x00E0 => {
                    let next_pc = Self::sequential_pc(pc, opcode, 2)?;
                    self.display = [[false; 64]; 32];
                    self.pc = next_pc;
                }
                0x00EE => {
                    if self.stack_depth == 0 {
                        return Err(CpuFault::StackUnderflow { pc, opcode });
                    }
                    let target = self.stack[self.stack_depth - 1] as usize;
                    let target = Self::checked_pc_target(pc, opcode, target)?;
                    self.stack_depth -= 1;
                    self.pc = target;
                }
                _ => {
                    self.pc = Self::sequential_pc(pc, opcode, 2)?;
                }
            },
            0x1 => {
                self.pc = Self::checked_pc_target(pc, opcode, nnn as usize)?;
            }
            0x2 => {
                if self.stack_depth == STACK_SIZE {
                    return Err(CpuFault::StackOverflow { pc, opcode });
                }
                let return_pc = Self::sequential_pc(pc, opcode, 2)?;
                let target = Self::checked_pc_target(pc, opcode, nnn as usize)?;
                self.stack[self.stack_depth] = return_pc;
                self.stack_depth += 1;
                self.pc = target;
            }
            0x3 => {
                let bytes = if self.v[x] == kk { 4 } else { 2 };
                self.pc = Self::sequential_pc(pc, opcode, bytes)?;
            }
            0x4 => {
                let bytes = if self.v[x] != kk { 4 } else { 2 };
                self.pc = Self::sequential_pc(pc, opcode, bytes)?;
            }
            0x5 => {
                if n != 0 {
                    return Err(CpuFault::InvalidOpcode { pc, opcode });
                }
                let bytes = if self.v[x] == self.v[y] { 4 } else { 2 };
                self.pc = Self::sequential_pc(pc, opcode, bytes)?;
            }
            0x6 => {
                let next_pc = Self::sequential_pc(pc, opcode, 2)?;
                self.v[x] = kk;
                self.pc = next_pc;
            }
            0x7 => {
                let next_pc = Self::sequential_pc(pc, opcode, 2)?;
                self.v[x] = self.v[x].wrapping_add(kk);
                self.pc = next_pc;
            }
            0x8 => {
                if !matches!(n, 0x0..=0x7 | 0xE) {
                    return Err(CpuFault::InvalidOpcode { pc, opcode });
                }
                let next_pc = Self::sequential_pc(pc, opcode, 2)?;
                match n {
                    0x0 => self.v[x] = self.v[y],
                    0x1 => self.v[x] |= self.v[y],
                    0x2 => self.v[x] &= self.v[y],
                    0x3 => self.v[x] ^= self.v[y],
                    0x4 => {
                        let (result, overflow) = self.v[x].overflowing_add(self.v[y]);
                        self.v[x] = result;
                        self.v[0xF] = u8::from(overflow);
                    }
                    0x5 => {
                        let (result, overflow) = self.v[x].overflowing_sub(self.v[y]);
                        self.v[x] = result;
                        self.v[0xF] = u8::from(!overflow);
                    }
                    0x6 => {
                        self.v[0xF] = self.v[x] & 0x1;
                        self.v[x] >>= 1;
                    }
                    0x7 => {
                        let (result, overflow) = self.v[y].overflowing_sub(self.v[x]);
                        self.v[x] = result;
                        self.v[0xF] = u8::from(!overflow);
                    }
                    0xE => {
                        self.v[0xF] = (self.v[x] >> 7) & 0x1;
                        self.v[x] <<= 1;
                    }
                    _ => unreachable!(),
                }
                self.pc = next_pc;
            }
            0x9 => {
                if n != 0 {
                    return Err(CpuFault::InvalidOpcode { pc, opcode });
                }
                let bytes = if self.v[x] != self.v[y] { 4 } else { 2 };
                self.pc = Self::sequential_pc(pc, opcode, bytes)?;
            }
            0xA => {
                let next_pc = Self::sequential_pc(pc, opcode, 2)?;
                self.i = nnn;
                self.pc = next_pc;
            }
            0xB => {
                let target = nnn as usize + self.v[0] as usize;
                self.pc = Self::checked_pc_target(pc, opcode, target)?;
            }
            0xC => {
                let next_pc = Self::sequential_pc(pc, opcode, 2)?;
                self.v[x] = rand::thread_rng().gen::<u8>() & kk;
                self.pc = next_pc;
            }
            0xD => {
                let next_pc = Self::sequential_pc(pc, opcode, 2)?;
                let start = Self::checked_memory_start(pc, opcode, self.i as usize, n as usize)?;
                let start_x = (self.v[x] % 64) as usize;
                let start_y = (self.v[y] % 32) as usize;
                self.v[0xF] = 0;

                for row in 0..n as usize {
                    if start_y + row >= 32 {
                        break;
                    }
                    let sprite_byte = self.memory[start + row];
                    for column in 0..8 {
                        if start_x + column >= 64 {
                            break;
                        }
                        if (sprite_byte >> (7 - column)) & 1 == 1 {
                            let pixel = &mut self.display[start_y + row][start_x + column];
                            if *pixel {
                                self.v[0xF] = 1;
                            }
                            *pixel = !*pixel;
                        }
                    }
                }
                self.pc = next_pc;
            }
            0xE => {
                if !matches!(kk, 0x9E | 0xA1) {
                    return Err(CpuFault::InvalidOpcode { pc, opcode });
                }
                let key = self.v[x];
                if key as usize >= self.keypad.len() {
                    return Err(CpuFault::KeyOutOfBounds { pc, opcode, key });
                }
                let pressed = self.keypad[key as usize];
                let skip = if kk == 0x9E { pressed } else { !pressed };
                let bytes = if skip { 4 } else { 2 };
                self.pc = Self::sequential_pc(pc, opcode, bytes)?;
            }
            0xF => match kk {
                0x07 => {
                    let next_pc = Self::sequential_pc(pc, opcode, 2)?;
                    self.v[x] = self.dt;
                    self.pc = next_pc;
                }
                0x0A => {
                    let next_pc = Self::sequential_pc(pc, opcode, 2)?;
                    self.waiting_for_key = Some(x);
                    self.pc = next_pc;
                }
                0x15 => {
                    let next_pc = Self::sequential_pc(pc, opcode, 2)?;
                    self.dt = self.v[x];
                    self.pc = next_pc;
                }
                0x18 => {
                    let next_pc = Self::sequential_pc(pc, opcode, 2)?;
                    self.st = self.v[x];
                    self.pc = next_pc;
                }
                0x1E => {
                    let next_pc = Self::sequential_pc(pc, opcode, 2)?;
                    let index = self.i as usize + self.v[x] as usize;
                    let index = Self::checked_index(pc, opcode, index)?;
                    self.i = index;
                    self.pc = next_pc;
                }
                0x29 => {
                    let next_pc = Self::sequential_pc(pc, opcode, 2)?;
                    self.i = (self.v[x] as u16 & 0xF) * 5;
                    self.pc = next_pc;
                }
                0x33 => {
                    let next_pc = Self::sequential_pc(pc, opcode, 2)?;
                    let start = Self::checked_memory_start(pc, opcode, self.i as usize, 3)?;
                    let value = self.v[x];
                    self.memory[start] = value / 100;
                    self.memory[start + 1] = (value / 10) % 10;
                    self.memory[start + 2] = value % 10;
                    self.pc = next_pc;
                }
                0x55 => {
                    let next_pc = Self::sequential_pc(pc, opcode, 2)?;
                    let length = x + 1;
                    let start = Self::checked_memory_start(pc, opcode, self.i as usize, length)?;
                    let index = Self::checked_index(pc, opcode, start + length)?;
                    self.memory[start..start + length].copy_from_slice(&self.v[..length]);
                    self.i = index;
                    self.pc = next_pc;
                }
                0x65 => {
                    let next_pc = Self::sequential_pc(pc, opcode, 2)?;
                    let length = x + 1;
                    let start = Self::checked_memory_start(pc, opcode, self.i as usize, length)?;
                    let index = Self::checked_index(pc, opcode, start + length)?;
                    self.v[..length].copy_from_slice(&self.memory[start..start + length]);
                    self.i = index;
                    self.pc = next_pc;
                }
                _ => return Err(CpuFault::InvalidOpcode { pc, opcode }),
            },
            _ => unreachable!(),
        }

        Ok(())
    }

    pub fn framebuffer(&self) -> &[[bool; 64]; 32] {
        &self.display
    }
}

#[cfg(test)]
mod tests;
