use log::{debug, error, info, warn};
use rand::Rng;
use std::fs::File;
use std::io::Read;
use thiserror::Error;

const BASE: usize = 0x200; // RAM (512) Base Program Memory
const END: usize = 0x1000; // RAM (4096) Memory End

#[derive(Error, Debug)]
pub enum CpuError {
    // Load ROM Errors
    #[error("Failed to open CHIP-8 ROM file: {err}")]
    RomOpenError { err: std::io::Error },
    #[error("Failed to read CHIP-8 ROM file: {err}")]
    RomReadError { err: std::io::Error },
    #[error("CHIP-8 ROM too large for memory. Expected <= {max}, got {actual} bytes")]
    RomSizeError { max: usize, actual: usize },
}

pub struct Cpu {
    memory: [u8; END],         // RAM: 0x000 (0) to 0xFFF (4095)
    rom_size: usize,           // Size of Loaded ROM (bytes)
    v: [u8; 16],               // V0 (0) .. VF (15) Registers
    i: u16,                    // Memory Address Store
    pc: u16,                   // Program Counter (currently executing address)
    stack: Vec<u16>,           // Stack, 16 Spaces
    sp: u8,                    // Stack Pointer
    dt: u8,                    // Delay Timer
    st: u8,                    // Sound Timer
    keypad: [bool; 16],        // Input Keypad
    display: [[bool; 64]; 32], // Display Buffer
}

pub struct RomLoadResult {
    pub bytes_read: usize,
}

impl Cpu {
    pub fn new() -> Self {
        Cpu {
            memory: [0; END],
            rom_size: 0,
            v: [0; 16],
            i: 0,
            pc: 0x200,         // Entry Point (EP)
            stack: Vec::new(), // LIFO
            sp: 0,
            dt: 0,
            st: 0,
            keypad: [false; 16],
            display: [[false; 64]; 32],
        }
    }

    pub fn load_rom(&mut self, rom_file: &str) -> Result<RomLoadResult, CpuError> {
        let mut f: File = File::open(rom_file).map_err(|e| CpuError::RomOpenError { err: e })?;

        let mut buf: Vec<u8> = Vec::new();
        let bytes_read: usize = f
            .read_to_end(&mut buf)
            .map_err(|e| CpuError::RomReadError { err: e })?;

        if bytes_read > END - BASE {
            return Err(CpuError::RomSizeError {
                max: END - BASE,
                actual: bytes_read,
            });
        }

        self.memory[BASE..BASE + bytes_read].copy_from_slice(&buf);
        self.rom_size = bytes_read;

        info!("Read {:?} bytes from CHIP-8 ROM '{}'", bytes_read, rom_file);

        Ok(RomLoadResult { bytes_read })
    }

    fn next_instr(&self) -> u16 {
        let b1: u8 = self.memory[self.pc as usize];
        let b2: u8 = self.memory[(self.pc + 1) as usize];
        ((b1 as u16) << 8) | b2 as u16
    }

    pub fn cpu_exec(&mut self) {
        let cmd: u16 = self.next_instr();
        let ind: u16 = (cmd >> 8) >> 4; // 4-bit instruction indicator (0xF000)

        debug!("INSTR: {:X}, IND: {:X}", cmd, ind);

        match ind {
            0x0 => {
                match cmd {
                    0x00E0 => {
                        // CLS - Clear display
                        self.display = [[false; 64]; 32];
                    }
                    0x00EE => {
                        // RET - Return from a subroutine
                        self.pc = match self.stack.pop() {
                            Some(addr) => addr,
                            None => return,
                        };
                        debug!("RET {:X}", self.pc);
                    }
                    _ => {
                        // 0NNN - Execute subroutine at address NNN
                        warn!("SYSTEM JMP to {:X} - Not Implemented!", 0x0FFF & cmd);
                    }
                }
            }
            0x1 => {
                // JMP addr - Jump to location nnn (1nnn)
                self.pc = 0x0FFF & cmd;
                debug!("JMP {:X}", self.pc);
            }
            0x2 => {
                // CALL addr - Call subroutine at nnn (2nnn)
                self.stack.push(self.pc);
                self.pc = 0x0FFF & cmd;
                debug!("CALL {:X}", self.pc);
            }
            0x3 => {
                // SE Vx, byte -- 3xkk, skip next instr if Vx = kk
                if self.v[((0x0F00 & cmd) >> 8) as usize] == (0x00FF & cmd) as u8 {
                    self.pc += 2;
                }
                debug!("SE V{:X}, {:X}", (0x0F00 & cmd) >> 8, (0x00FF & cmd));
            }
            0x4 => {
                // SNE Vx, byte -- 4xkk, skip next instr if Vx != kk
                if self.v[((0x0F00 & cmd) >> 8) as usize] != (0x00FF & cmd) as u8 {
                    self.pc += 2;
                }
                debug!("SNE V{:X}, {:X}", (0x0F00 & cmd) >> 8, (0x00FF & cmd));
            }
            0x5 => {
                // SE Vx, Vy -- 5xy0, skip next instr if Vx = Vy
                if self.v[((0x0F00 & cmd) >> 8) as usize] == self.v[((0x00F0 & cmd) >> 4) as usize]
                {
                    self.pc += 2;
                }
                debug!("SE V{:X}, V{:X}", (0x0F00 & cmd) >> 8, (0x00F0 & cmd) >> 4);
            }
            0x6 => {
                // LD Vx, byte -- 6xkk, loads kk into Vx
                self.v[((0x0F00 & cmd) >> 8) as usize] = (0x00FF & cmd) as u8;
                self.pc += 2;
                debug!("LD {:X} into V{:X}", 0x00FF & cmd, (0x0F00 & cmd) >> 8);
            }
            0x7 => {
                // ADD Vx, byte -- 7xkk, Vx += kk
                self.v[((0x0F00 & cmd) >> 8) as usize] += (0x00FF & cmd) as u8;
                self.pc += 2;
                debug!("V{:X} += {:X}", ((0x0F00 & cmd) >> 8), (0x00FF & cmd));
            }
            0x8 => match 0x000F & cmd {
                // 8xyN matching
                0x0 => {
                    // 8xy0 - LD Vx, Vy, Set Vx = Vy
                    self.v[((0x0F00 & cmd) >> 8) as usize] = self.v[((0x00F0 & cmd) >> 4) as usize];
                    self.pc += 2;
                    debug!("V{:X} = V{:X}", (0x0F00 & cmd) >> 8, (0x00F0 & cmd) >> 4);
                }
                0x1 => {
                    // 8xy1 - OR Vx, Vy, Set Vx = Vx OR Vy, Vx |= Vy
                    self.v[((0x0F00 & cmd) >> 8) as usize] |=
                        self.v[((0x00F0 & cmd) >> 4) as usize];
                    self.pc += 2;
                    debug!("V{:X} |= V{:X}", (0x0F00 & cmd) >> 8, (0x00F0 & cmd) >> 4);
                }
                0x2 => {
                    // 8xy2 - AND Vx, Vy, Set Vx = Vx AND Vy
                    self.v[((0x0F00 & cmd) >> 8) as usize] &=
                        self.v[((0x00F0 & cmd) >> 4) as usize];
                    self.pc += 2;
                    debug!("V{:X} &= V{:X}", (0x0F00 & cmd) >> 8, (0x00F0 & cmd) >> 4);
                }
                0x3 => {
                    // 8xy3 - XOR Vx, Vy, Set Vx = Vx XOR Vy
                    self.v[((0x0F00 & cmd) >> 8) as usize] ^=
                        self.v[((0x00F0 & cmd) >> 4) as usize];
                    self.pc += 2;
                    debug!("V{:X} ^= V{:X}", (0x0F00 & cmd) >> 8, (0x00F0 & cmd) >> 4);
                }
                0x4 => {
                    // 8xy4 - ADD Vx, Vy, Set Vx = Vx + Vy, set VF = carry
                    // If the result is greater than 8 bits (i.e., > 255,) VF is set to 1, otherwise 0.
                    // Only the lowest 8 bits of the result are kept, and stored in Vx.
                    let x = ((0x0F00 & cmd) >> 8) as usize;
                    let y = ((0x00F0 & cmd) >> 4) as usize;

                    let sum = self.v[x] + self.v[y];

                    if sum > u8::MAX {
                        self.v[0xF] = 1
                    } else {
                        self.v[0xF] = 0
                    }

                    self.pc += 2;
                    self.v[x] = sum & 0xFF;

                    debug!("V{:X} += V{:X}, Carry Flag VF: {:X}", x, y, self.v[0xF]);
                }
                0x5 => {
                    // 8xy5 - SUB Vx, Vy, Set Vx = Vx - Vy, set VF = NOT borrow
                    // VF = 1 when NO borrow (Vx > Vy)
                    let x = ((0x0F00 & cmd) >> 8) as usize;
                    let y = ((0x00F0 & cmd) >> 4) as usize;

                    let sub = self.v[x] - self.v[y];

                    if self.v[x] >= self.v[y] {
                        self.v[0xF] = 1;
                    } else {
                        self.v[0xF] = 0;
                    }

                    self.pc += 2;
                    self.v[x] = sub;

                    debug!("V{:X} -= V{:X}, Carry Flag VF: {:X}", x, y, self.v[0xF]);
                }
                0x6 => {
                    // 8xy6 - Set Vx = Vy SHR 1
                    // If the least-significant bit is 1, then VF is set to 1, otherwise 0.
                    // Note: Make configurable for Vx = Vx >> 1 ??
                    let x = ((0x0F00 & cmd) >> 8) as usize;
                    let y = ((0x00F0 & cmd) >> 4) as usize;

                    self.v[0xF] = self.v[y] & 1;
                    self.v[x] = self.v[y] >> 1;
                    self.pc += 2;

                    debug!("V{:X} = V{:X} >> 1, Carry Flag VF: {:X}", x, y, self.v[0xF]);
                }
                0x7 => {
                    // 8xy7 - SUBN Vx, Vy, Set Vx = Vy - Vx, set VF = NOT borrow.
                    // VF = 1 when NO borrow (Vy > Vx)
                    let x = ((0x0F00 & cmd) >> 8) as usize;
                    let y = ((0x00F0 & cmd) >> 4) as usize;

                    let sub = self.v[y] - self.v[x];

                    if self.v[y] >= self.v[x] {
                        self.v[0xF] = 1;
                    } else {
                        self.v[0xF] = 0;
                    }

                    self.pc += 2;
                    self.v[x] = sub;

                    debug!(
                        "V{:X} = V{:X} - V{:X}, Carry Flag VF: {:X}",
                        x, y, x, self.v[0xF]
                    );
                }
                0xE => {
                    // 8xyE - Set Vx = Vy SHL 1.
                    // If the most-significant bit is 1, then VF is set to 1, otherwise to 0.
                    // Note: Make configurable for Vx = Vx << 1 ??
                    let x = ((0x0F00 & cmd) >> 8) as usize;
                    let y = ((0x00F0 & cmd) >> 4) as usize;

                    self.v[0xF] = (self.v[y] >> 7) & 1;
                    self.v[x] = self.v[y] << 1;
                    self.pc += 2;

                    debug!("V{:X} = V{:X} << 1, Carry Flag VF: {:X}", x, y, self.v[0xF]);
                }
                _ => (), // Misc 8NNN Instruction
            },
            0x9 => {
                // 9xy0 - SNE Vx, Vy, Skip next instruction if Vx != Vy.
                if self.v[((0x0F00 & cmd) >> 8) as usize] != self.v[((0x00F0 & cmd) >> 4) as usize]
                {
                    self.pc += 2;
                }
                debug!("SNE V{:X}, {:X}", (0x0F00 & cmd) >> 8, (0x00F0 & cmd) >> 4);
            }
            0xA => {
                // Annn - LD I, addr, Set I = nnn
                self.i = 0x0FFF & cmd;
                self.pc += 2;
                debug!("I = {:X}", 0x0FFF & cmd);
            }
            0xB => {
                // Bnnn, JMP [NNN + V0]
                self.pc = (0x0FFF & cmd) + (self.v[0x0] as u16);
                debug!("JMP [{:X} + V0]", 0x0FFF & cmd)
            }
            0xC => {
                // Cxnn, LD VX, rand() & nn
                let rand: u8 = rand::thread_rng().gen();
                self.v[((0x0F00 & cmd) >> 8) as usize] = rand & (0x00FF & cmd) as u8;
                self.pc += 2;
                debug!("V{:X} = rand() & {:X}", (0x0F00 & cmd) >> 8, 0x00FF & cmd);
            }
            0xD => {
                // Dxyn, DRAW pos_x: Vx, pos_y: Vy, dat_bytes: n, sprite_addr: I
                // If any set pixels are unset, VF = 1; else VF = 0
            }
            0xE => match cmd & 0x00FF {
                0x9E => {
                    // Ex9E, SKP Vx
                    // Skip next instr if key with value of Vx is pressed
                }
                0xA1 => {
                    // ExA1, SKNP Vx
                    // Skip next instr if key with value of Vx is *not* pressed
                }
                _ => (),
            },
            0xF => match cmd & 0x00FF {
                0x07 => {
                    // Fx07
                    // LD VX, DT, Store current delay timer value in Vx
                    let x: usize = ((0x0F00 & cmd) >> 8) as usize;
                    self.v[x] = self.dt;
                    self.pc += 2;
                    debug!("V{:X} = {:X} (delay timer value)", x, self.dt);
                }
                0x0A => {
                    // Fx0A
                    // WAIT_KEY Vx, Wait for a keypress and store result in Vx
                    // Blocks execution until keypress; after keypress, running resumes
                    let x: usize = ((0x0F00 & cmd) >> 8) as usize;
                }
                0x15 => {
                    // Fx15
                    // LD DT, VX, Set delay timer to value in Vx
                    let x: usize = ((0x0F00 & cmd) >> 8) as usize;
                    self.dt = self.v[x];
                    self.pc += 2;
                    debug!("Delay Timer = V{:X} (Vx val: {:X})", x, self.v[x]);
                }
                0x18 => {
                    // Fx18
                    // LD ST, VX, Set sound timer to value in Vx
                    let x: usize = ((0x0F00 & cmd) >> 8) as usize;
                    self.st = self.v[x];
                    self.pc += 2;
                    debug!("Sound Timer = V{:X}, (Vx val: {:X})", x, self.v[x]);
                }
                0x1E => {
                    // Fx1E
                    // ADD I, VX, I = I + VX
                    // Set VF = 1 if overflows past 0xFFF? (set configurable?)
                    let x: usize = ((0x0F00 & cmd) >> 8) as usize;
                    self.i += self.v[x] as u16;
                    self.pc += 2;
                    debug!("I += V{:X} (Vx val: {:X})", x, self.v[x]);
                }
                0x29 => {
                    // Fx29
                    // I = font_table[Vx]
                    // Set I to the memory address of the 5-byte font sprite for the hexadecimal digit stored in Vx.
                    let x: usize = ((0x0F00 & cmd) >> 8) as usize;
                }
                0x33 => {
                    // Fx33
                    // Store binary-coded decimal equivalent of value in Vx at addresses: I, I+1, and I+2
                    // I = hundreds digit; I+1 = tens digit; I+2 = ones digit
                    let x: usize = ((0x0F00 & cmd) >> 8) as usize;
                }
                0x55 => {
                    // Fx55
                    // Store values of registers V0 to VX (inclusive) in memory starting at address I
                    // After operation, I = I + X + 1 (points to next address after last accessed memory loc)
                    let x: usize = ((0x0F00 & cmd) >> 8) as usize;
                }
                0x65 => {
                    // Fx65
                    // Fill registers V0 to VX (inclusive) with the values stored in memory starting at address I
                    // After operation, I = I + X + 1 (points to next address after last accessed memory loc)
                    let x: usize = ((0x0F00 & cmd) >> 8) as usize;
                }
                _ => (),
            },
            _ => (), // Misc Instruction
        }
    }

    pub fn set_display(&mut self, x: usize, y: usize, value: bool) {
        self.display[y][x] = value;
    }

    pub fn get_display(&self) -> [[bool; 64]; 32] {
        return self.display;
    }
}
