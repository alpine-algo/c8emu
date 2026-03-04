# CHIP-8 Emulator (Rust + Iced)

A robust, high-performance CHIP-8 emulator written in Rust using the [Iced](https://iced.rs/) GUI library. This project implements the full CHIP-8 specification, including all 35 standard opcodes, 60Hz timers, and a 64x32 monochrome display.

## Features

- **Full Instruction Set:** Complete implementation of all CHIP-8 opcodes.
- **Accurate Timing:** 60Hz timer ticks and ~600Hz CPU cycle speed.
- **Modern GUI:** Clean interface for loading ROMs and viewing the display.
- **Keyboard Mapping:** Intuitive mapping for the original 16-key hex keypad.

## Getting Started

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (latest stable version)
- A graphics driver supporting Vulkan or OpenGL.

### Installation & Execution

1. Clone the repository:
   ```bash
   git clone https://github.com/your-username/c8emu.git
   cd c8emu
   ```

2. Run the emulator:
   ```bash
   cargo run --release
   ```

3. Enter the path to a ROM file (e.g., `roms/test_opcode.ch8`) and click **Load**.

## Keypad Mapping

The original CHIP-8 hex keypad is mapped to your keyboard as follows:

| CHIP-8 Keypad | Computer Keyboard |
| :---: | :---: |
| `1 2 3 C` | `1 2 3 4` |
| `4 5 6 D` | `Q W E R` |
| `7 8 9 E` | `A S D F` |
| `A 0 B F` | `Z X C V` |

## Troubleshooting

### Vulkan Validation Errors (Linux)
If you see `wgpu_hal::vulkan` validation errors in your terminal on Linux, you can force the OpenGL backend, which is typically more stable for simple 2D applications:

```bash
WGPU_BACKEND=gl cargo run
```

## References

- [Cowgod's CHIP-8 Technical Reference](http://devernay.free.fr/hacks/chip8/C8TECH10.HTM)
- [CHIP-8 Technical Reference (Matt Mikolay)](https://github.com/mattmikolay/chip-8/wiki/CHIP%E2%80%908-Technical-Reference)
- [CHIP-8 VM Specification (Toni Sagrista)](https://tonisagrista.com/blog/2021/chip8-spec/)

## License

This project is licensed under the MIT License.
