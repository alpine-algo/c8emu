use super::*;

#[derive(Debug, PartialEq)]
struct CpuSnapshot {
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

impl From<&Cpu> for CpuSnapshot {
    fn from(cpu: &Cpu) -> Self {
        Self {
            memory: cpu.memory,
            v: cpu.v,
            i: cpu.i,
            pc: cpu.pc,
            stack: cpu.stack,
            stack_depth: cpu.stack_depth,
            dt: cpu.dt,
            st: cpu.st,
            keypad: cpu.keypad,
            display: cpu.display,
            waiting_for_key: cpu.waiting_for_key,
        }
    }
}

fn write_opcode(cpu: &mut Cpu, address: usize, opcode: u16) {
    cpu.memory[address] = (opcode >> 8) as u8;
    cpu.memory[address + 1] = opcode as u8;
}

fn execute_opcode(cpu: &mut Cpu, opcode: u16) -> Result<(), CpuFault> {
    write_opcode(cpu, cpu.pc as usize, opcode);
    cpu.cpu_exec()
}

#[test]
fn valid_install_resets_every_field_and_replaces_a_longer_rom() {
    let mut cpu = Cpu::new();
    cpu.install_rom(&[1, 2, 3, 4]).unwrap();
    cpu.memory[0] = 0;
    cpu.memory[100] = 0xAA;
    cpu.v = [0x11; 16];
    cpu.i = 0x345;
    cpu.pc = 0x300;
    cpu.stack = [0x222; STACK_SIZE];
    cpu.stack_depth = 7;
    cpu.dt = 8;
    cpu.st = 9;
    cpu.keypad = [true; 16];
    cpu.display = [[true; 64]; 32];
    cpu.waiting_for_key = Some(4);

    let result = cpu.install_rom(&[0xAA, 0xBB]).unwrap();

    let mut expected = Cpu::new();
    expected.memory[PROGRAM_START..PROGRAM_START + 2].copy_from_slice(&[0xAA, 0xBB]);
    assert_eq!(result.bytes_read, 2);
    assert_eq!(CpuSnapshot::from(&cpu), CpuSnapshot::from(&expected));
    assert!(cpu.memory[PROGRAM_START + 2..]
        .iter()
        .all(|byte| *byte == 0));
}

#[test]
fn rejected_install_is_typed_and_preserves_the_complete_state() {
    let mut cpu = Cpu::new();
    cpu.install_rom(&[0x60, 0x01, 0x61, 0x02]).unwrap();
    cpu.v[3] = 0xAB;
    cpu.i = 0x345;
    cpu.stack[0] = 0x288;
    cpu.stack_depth = 1;
    cpu.dt = 4;
    cpu.st = 5;
    cpu.keypad[0xA] = true;
    cpu.display[7][9] = true;
    cpu.waiting_for_key = Some(2);
    let before = CpuSnapshot::from(&cpu);

    assert!(matches!(cpu.install_rom(&[]), Err(CpuFault::EmptyRom)));
    assert_eq!(CpuSnapshot::from(&cpu), before);

    let oversized = vec![0; MEMORY_SIZE - PROGRAM_START + 1];
    let fault = match cpu.install_rom(&oversized) {
        Err(fault) => fault,
        Ok(_) => panic!("oversized ROM unexpectedly loaded"),
    };
    assert!(matches!(
        fault,
        CpuFault::RomTooLarge {
            max: 3584,
            actual: 3585
        }
    ));
    assert_eq!(CpuSnapshot::from(&cpu), before);
}

struct OpcodeCase {
    name: &'static str,
    opcode: u16,
    arrange: fn(&mut Cpu),
    verify: fn(&Cpu),
}

#[test]
fn advertised_35_instruction_forms_and_quirks_are_deterministic() {
    let cases: [OpcodeCase; 35] = [
        OpcodeCase {
            name: "00e0_clear_screen",
            opcode: 0x00E0,
            arrange: |cpu| cpu.display[4][5] = true,
            verify: |cpu| {
                assert!(cpu.display.iter().flatten().all(|pixel| !pixel));
                assert_eq!(cpu.pc, 0x202);
            },
        },
        OpcodeCase {
            name: "00ee_return",
            opcode: 0x00EE,
            arrange: |cpu| {
                cpu.stack[0] = 0x300;
                cpu.stack_depth = 1;
            },
            verify: |cpu| {
                assert_eq!(cpu.pc, 0x300);
                assert_eq!(cpu.stack_depth, 0);
            },
        },
        OpcodeCase {
            name: "0nnn_sys_is_ignored",
            opcode: 0x0123,
            arrange: |_| {},
            verify: |cpu| assert_eq!(cpu.pc, 0x202),
        },
        OpcodeCase {
            name: "1nnn_jump",
            opcode: 0x1300,
            arrange: |_| {},
            verify: |cpu| assert_eq!(cpu.pc, 0x300),
        },
        OpcodeCase {
            name: "2nnn_call",
            opcode: 0x2300,
            arrange: |_| {},
            verify: |cpu| {
                assert_eq!(cpu.pc, 0x300);
                assert_eq!(cpu.stack_depth, 1);
                assert_eq!(cpu.stack[0], 0x202);
            },
        },
        OpcodeCase {
            name: "3xkk_skip_equal_byte",
            opcode: 0x31AA,
            arrange: |cpu| cpu.v[1] = 0xAA,
            verify: |cpu| assert_eq!(cpu.pc, 0x204),
        },
        OpcodeCase {
            name: "4xkk_skip_not_equal_byte",
            opcode: 0x41AA,
            arrange: |cpu| cpu.v[1] = 0xAB,
            verify: |cpu| assert_eq!(cpu.pc, 0x204),
        },
        OpcodeCase {
            name: "5xy0_skip_equal_registers",
            opcode: 0x5120,
            arrange: |cpu| {
                cpu.v[1] = 7;
                cpu.v[2] = 7;
            },
            verify: |cpu| assert_eq!(cpu.pc, 0x204),
        },
        OpcodeCase {
            name: "6xkk_load_byte",
            opcode: 0x61AB,
            arrange: |_| {},
            verify: |cpu| assert_eq!(cpu.v[1], 0xAB),
        },
        OpcodeCase {
            name: "7xkk_add_byte_wraps",
            opcode: 0x7102,
            arrange: |cpu| cpu.v[1] = 0xFF,
            verify: |cpu| assert_eq!(cpu.v[1], 1),
        },
        OpcodeCase {
            name: "8xy0_load_register",
            opcode: 0x8120,
            arrange: |cpu| cpu.v[2] = 0x44,
            verify: |cpu| assert_eq!(cpu.v[1], 0x44),
        },
        OpcodeCase {
            name: "8xy1_or_preserves_vf",
            opcode: 0x8121,
            arrange: |cpu| {
                cpu.v[1] = 0xF0;
                cpu.v[2] = 0x0F;
                cpu.v[0xF] = 0xA5;
            },
            verify: |cpu| {
                assert_eq!(cpu.v[1], 0xFF);
                assert_eq!(cpu.v[0xF], 0xA5);
            },
        },
        OpcodeCase {
            name: "8xy2_and_preserves_vf",
            opcode: 0x8122,
            arrange: |cpu| {
                cpu.v[1] = 0xF3;
                cpu.v[2] = 0x0F;
                cpu.v[0xF] = 0xA5;
            },
            verify: |cpu| {
                assert_eq!(cpu.v[1], 3);
                assert_eq!(cpu.v[0xF], 0xA5);
            },
        },
        OpcodeCase {
            name: "8xy3_xor_preserves_vf",
            opcode: 0x8123,
            arrange: |cpu| {
                cpu.v[1] = 0xF0;
                cpu.v[2] = 0x0F;
                cpu.v[0xF] = 0xA5;
            },
            verify: |cpu| {
                assert_eq!(cpu.v[1], 0xFF);
                assert_eq!(cpu.v[0xF], 0xA5);
            },
        },
        OpcodeCase {
            name: "8xy4_add_registers",
            opcode: 0x8124,
            arrange: |cpu| {
                cpu.v[1] = 250;
                cpu.v[2] = 10;
            },
            verify: |cpu| {
                assert_eq!(cpu.v[1], 4);
                assert_eq!(cpu.v[0xF], 1);
            },
        },
        OpcodeCase {
            name: "8xy5_subtract_registers",
            opcode: 0x8125,
            arrange: |cpu| {
                cpu.v[1] = 5;
                cpu.v[2] = 3;
            },
            verify: |cpu| {
                assert_eq!(cpu.v[1], 2);
                assert_eq!(cpu.v[0xF], 1);
            },
        },
        OpcodeCase {
            name: "8xy6_shift_right_uses_vx",
            opcode: 0x8126,
            arrange: |cpu| {
                cpu.v[1] = 3;
                cpu.v[2] = 0x80;
            },
            verify: |cpu| {
                assert_eq!(cpu.v[1], 1);
                assert_eq!(cpu.v[0xF], 1);
            },
        },
        OpcodeCase {
            name: "8xy7_reverse_subtract",
            opcode: 0x8127,
            arrange: |cpu| {
                cpu.v[1] = 3;
                cpu.v[2] = 5;
            },
            verify: |cpu| {
                assert_eq!(cpu.v[1], 2);
                assert_eq!(cpu.v[0xF], 1);
            },
        },
        OpcodeCase {
            name: "8xye_shift_left_uses_vx",
            opcode: 0x812E,
            arrange: |cpu| {
                cpu.v[1] = 0x81;
                cpu.v[2] = 1;
            },
            verify: |cpu| {
                assert_eq!(cpu.v[1], 2);
                assert_eq!(cpu.v[0xF], 1);
            },
        },
        OpcodeCase {
            name: "9xy0_skip_not_equal_registers",
            opcode: 0x9120,
            arrange: |cpu| {
                cpu.v[1] = 1;
                cpu.v[2] = 2;
            },
            verify: |cpu| assert_eq!(cpu.pc, 0x204),
        },
        OpcodeCase {
            name: "annn_load_index",
            opcode: 0xA345,
            arrange: |_| {},
            verify: |cpu| assert_eq!(cpu.i, 0x345),
        },
        OpcodeCase {
            name: "bnnn_jump_uses_v0",
            opcode: 0xB300,
            arrange: |cpu| {
                cpu.v[0] = 4;
                cpu.v[3] = 10;
            },
            verify: |cpu| assert_eq!(cpu.pc, 0x304),
        },
        OpcodeCase {
            name: "cxkk_random_is_masked",
            opcode: 0xC1F0,
            arrange: |_| {},
            verify: |cpu| {
                assert_eq!(cpu.v[1] & 0x0F, 0);
                assert_eq!(cpu.pc, 0x202);
            },
        },
        OpcodeCase {
            name: "dxyn_draw_clips_at_edges",
            opcode: 0xD122,
            arrange: |cpu| {
                cpu.i = 0x300;
                cpu.memory[0x300] = 0xC0;
                cpu.memory[0x301] = 0x80;
                cpu.v[1] = 63;
                cpu.v[2] = 31;
            },
            verify: |cpu| {
                assert!(cpu.display[31][63]);
                assert!(!cpu.display[31][0]);
                assert_eq!(cpu.v[0xF], 0);
            },
        },
        OpcodeCase {
            name: "ex9e_skip_pressed_key",
            opcode: 0xE19E,
            arrange: |cpu| {
                cpu.v[1] = 0xA;
                cpu.keypad[0xA] = true;
            },
            verify: |cpu| assert_eq!(cpu.pc, 0x204),
        },
        OpcodeCase {
            name: "exa1_skip_released_key",
            opcode: 0xE1A1,
            arrange: |cpu| cpu.v[1] = 0xA,
            verify: |cpu| assert_eq!(cpu.pc, 0x204),
        },
        OpcodeCase {
            name: "fx07_read_delay_timer",
            opcode: 0xF107,
            arrange: |cpu| cpu.dt = 9,
            verify: |cpu| assert_eq!(cpu.v[1], 9),
        },
        OpcodeCase {
            name: "fx0a_wait_for_key",
            opcode: 0xF10A,
            arrange: |_| {},
            verify: |cpu| {
                assert_eq!(cpu.waiting_for_key, Some(1));
                assert_eq!(cpu.pc, 0x202);
            },
        },
        OpcodeCase {
            name: "fx15_set_delay_timer",
            opcode: 0xF115,
            arrange: |cpu| cpu.v[1] = 8,
            verify: |cpu| assert_eq!(cpu.dt, 8),
        },
        OpcodeCase {
            name: "fx18_set_sound_timer",
            opcode: 0xF118,
            arrange: |cpu| cpu.v[1] = 7,
            verify: |cpu| assert_eq!(cpu.st, 7),
        },
        OpcodeCase {
            name: "fx1e_add_to_index_without_wrapping",
            opcode: 0xF11E,
            arrange: |cpu| {
                cpu.i = 0x300;
                cpu.v[1] = 4;
            },
            verify: |cpu| assert_eq!(cpu.i, 0x304),
        },
        OpcodeCase {
            name: "fx29_select_font_sprite",
            opcode: 0xF129,
            arrange: |cpu| cpu.v[1] = 0xD,
            verify: |cpu| {
                assert_eq!(cpu.i, 65);
                assert_eq!(&cpu.memory[65..70], &[0xE0, 0x90, 0x90, 0x90, 0xE0]);
            },
        },
        OpcodeCase {
            name: "fx33_store_bcd",
            opcode: 0xF133,
            arrange: |cpu| {
                cpu.i = 0x300;
                cpu.v[1] = 231;
            },
            verify: |cpu| assert_eq!(&cpu.memory[0x300..0x303], &[2, 3, 1]),
        },
        OpcodeCase {
            name: "fx55_store_registers_and_increment_i",
            opcode: 0xF255,
            arrange: |cpu| {
                cpu.i = 0x300;
                cpu.v[..3].copy_from_slice(&[1, 2, 3]);
            },
            verify: |cpu| {
                assert_eq!(&cpu.memory[0x300..0x303], &[1, 2, 3]);
                assert_eq!(cpu.i, 0x303);
            },
        },
        OpcodeCase {
            name: "fx65_load_registers_and_increment_i",
            opcode: 0xF265,
            arrange: |cpu| {
                cpu.i = 0x300;
                cpu.memory[0x300..0x303].copy_from_slice(&[1, 2, 3]);
            },
            verify: |cpu| {
                assert_eq!(&cpu.v[..3], &[1, 2, 3]);
                assert_eq!(cpu.i, 0x303);
            },
        },
    ];

    for case in &cases {
        let mut cpu = Cpu::new();
        write_opcode(&mut cpu, PROGRAM_START, case.opcode);
        (case.arrange)(&mut cpu);
        cpu.cpu_exec()
            .unwrap_or_else(|fault| panic!("{} unexpectedly faulted: {fault}", case.name));
        (case.verify)(&cpu);
    }
}

#[test]
fn malformed_and_unknown_encodings_fault_without_advancing() {
    for opcode in [0x5121, 0x912F, 0x8128, 0xE100, 0xF100] {
        let mut cpu = Cpu::new();
        write_opcode(&mut cpu, PROGRAM_START, opcode);
        let before = CpuSnapshot::from(&cpu);

        let fault = cpu.cpu_exec().unwrap_err();

        assert!(matches!(
            fault,
            CpuFault::InvalidOpcode {
                pc: 0x200,
                opcode: faulting_opcode
            } if faulting_opcode == opcode
        ));
        assert_eq!(CpuSnapshot::from(&cpu), before);
    }
}

struct FaultCase {
    name: &'static str,
    opcode: u16,
    arrange: fn(&mut Cpu),
    verify: fn(&CpuFault),
}

#[test]
fn execution_boundaries_return_typed_faults_without_partial_writes() {
    let cases: &[FaultCase] = &[
        FaultCase {
            name: "stack_underflow",
            opcode: 0x00EE,
            arrange: |_| {},
            verify: |fault| {
                assert!(matches!(
                    fault,
                    CpuFault::StackUnderflow {
                        pc: 0x200,
                        opcode: 0x00EE
                    }
                ));
            },
        },
        FaultCase {
            name: "stack_overflow",
            opcode: 0x2200,
            arrange: |cpu| cpu.stack_depth = STACK_SIZE,
            verify: |fault| {
                assert!(matches!(
                    fault,
                    CpuFault::StackOverflow {
                        pc: 0x200,
                        opcode: 0x2200
                    }
                ));
            },
        },
        FaultCase {
            name: "jump_target_out_of_bounds",
            opcode: 0x1FFF,
            arrange: |_| {},
            verify: |fault| match fault {
                CpuFault::ProgramCounterOutOfBounds { pc, opcode, target } => {
                    assert_eq!((*pc, *opcode, *target), (0x200, 0x1FFF, 0xFFF));
                }
                _ => panic!("unexpected fault: {fault}"),
            },
        },
        FaultCase {
            name: "sequential_pc_fault_precedes_register_write",
            opcode: 0x60FF,
            arrange: |cpu| cpu.pc = 0xFFE,
            verify: |fault| match fault {
                CpuFault::ProgramCounterOutOfBounds { pc, opcode, target } => {
                    assert_eq!((*pc, *opcode, *target), (0xFFE, 0x60FF, 0x1000));
                }
                _ => panic!("unexpected fault: {fault}"),
            },
        },
        FaultCase {
            name: "draw_memory_range",
            opcode: 0xD012,
            arrange: |cpu| cpu.i = 0xFFF,
            verify: |fault| {
                assert!(matches!(
                    fault,
                    CpuFault::MemoryOutOfBounds {
                        pc: 0x200,
                        opcode: 0xD012,
                        start: 0xFFF,
                        length: 2
                    }
                ));
            },
        },
        FaultCase {
            name: "bcd_memory_range",
            opcode: 0xF033,
            arrange: |cpu| cpu.i = 0xFFE,
            verify: |fault| {
                assert!(matches!(
                    fault,
                    CpuFault::MemoryOutOfBounds {
                        pc: 0x200,
                        opcode: 0xF033,
                        start: 0xFFE,
                        length: 3
                    }
                ));
            },
        },
        FaultCase {
            name: "store_index_increment",
            opcode: 0xF055,
            arrange: |cpu| {
                cpu.i = 0xFFF;
                cpu.v[0] = 0xAA;
                cpu.memory[0xFFF] = 0x55;
            },
            verify: |fault| {
                assert!(matches!(
                    fault,
                    CpuFault::IndexOutOfBounds {
                        pc: 0x200,
                        opcode: 0xF055,
                        index: 0x1000
                    }
                ));
            },
        },
        FaultCase {
            name: "load_index_increment",
            opcode: 0xF065,
            arrange: |cpu| {
                cpu.i = 0xFFF;
                cpu.v[0] = 0xAA;
                cpu.memory[0xFFF] = 0x55;
            },
            verify: |fault| {
                assert!(matches!(
                    fault,
                    CpuFault::IndexOutOfBounds {
                        pc: 0x200,
                        opcode: 0xF065,
                        index: 0x1000
                    }
                ));
            },
        },
        FaultCase {
            name: "index_addition",
            opcode: 0xF01E,
            arrange: |cpu| {
                cpu.i = 0xFFF;
                cpu.v[0] = 1;
            },
            verify: |fault| {
                assert!(matches!(
                    fault,
                    CpuFault::IndexOutOfBounds {
                        pc: 0x200,
                        opcode: 0xF01E,
                        index: 0x1000
                    }
                ));
            },
        },
        FaultCase {
            name: "key_value",
            opcode: 0xE09E,
            arrange: |cpu| cpu.v[0] = 16,
            verify: |fault| {
                assert!(matches!(
                    fault,
                    CpuFault::KeyOutOfBounds {
                        pc: 0x200,
                        opcode: 0xE09E,
                        key: 16
                    }
                ));
            },
        },
    ];

    for case in cases {
        let mut cpu = Cpu::new();
        (case.arrange)(&mut cpu);
        let pc = cpu.pc as usize;
        write_opcode(&mut cpu, pc, case.opcode);
        let before = CpuSnapshot::from(&cpu);

        let fault = cpu.cpu_exec().unwrap_err();

        (case.verify)(&fault);
        assert_eq!(CpuSnapshot::from(&cpu), before, "{}", case.name);
    }
}

#[test]
fn invalid_fetch_is_typed_and_does_not_change_state() {
    let mut cpu = Cpu::new();
    cpu.pc = 0xFFF;
    let before = CpuSnapshot::from(&cpu);

    let fault = cpu.cpu_exec().unwrap_err();

    assert!(matches!(fault, CpuFault::InvalidFetch { pc: 0xFFF }));
    assert_eq!(CpuSnapshot::from(&cpu), before);
}

#[test]
fn fixed_stack_accepts_sixteen_calls_then_faults_atomically() {
    let mut cpu = Cpu::new();
    write_opcode(&mut cpu, PROGRAM_START, 0x2200);

    for expected_depth in 1..=STACK_SIZE {
        cpu.cpu_exec().unwrap();
        assert_eq!(cpu.stack_depth, expected_depth);
        assert_eq!(cpu.pc, PROGRAM_START as u16);
    }

    let before = CpuSnapshot::from(&cpu);
    assert!(matches!(
        cpu.cpu_exec(),
        Err(CpuFault::StackOverflow {
            pc: 0x200,
            opcode: 0x2200
        })
    ));
    assert_eq!(CpuSnapshot::from(&cpu), before);
}

#[test]
fn timers_and_key_wait_use_explicit_core_transitions() {
    let mut cpu = Cpu::new();
    cpu.v[1] = 2;
    execute_opcode(&mut cpu, 0xF115).unwrap();
    cpu.v[1] = 1;
    execute_opcode(&mut cpu, 0xF118).unwrap();

    cpu.tick_timers();
    assert_eq!((cpu.dt, cpu.st), (1, 0));
    cpu.tick_timers();
    cpu.tick_timers();
    assert_eq!((cpu.dt, cpu.st), (0, 0));

    execute_opcode(&mut cpu, 0xF20A).unwrap();
    let waiting = CpuSnapshot::from(&cpu);
    cpu.cpu_exec().unwrap();
    assert_eq!(CpuSnapshot::from(&cpu), waiting);

    cpu.set_keypad(0xA, true);
    assert_eq!(cpu.v[2], 0xA);
    assert_eq!(cpu.waiting_for_key, None);
    assert!(cpu.keypad[0xA]);
    cpu.set_keypad(0xA, false);
    assert!(!cpu.keypad[0xA]);
}

#[test]
fn framebuffer_is_the_authoritative_borrow_and_preserves_pixel_semantics() {
    let mut cpu = Cpu::new();
    assert!(std::ptr::eq(cpu.framebuffer(), &cpu.display));
    let frame_pointer = cpu.framebuffer().as_ptr();

    cpu.i = 0x300;
    cpu.memory[0x300] = 0x80;
    cpu.v[1] = 4;
    cpu.v[2] = 5;
    execute_opcode(&mut cpu, 0xD121).unwrap();
    assert_eq!(cpu.framebuffer().as_ptr(), frame_pointer);
    assert!(cpu.framebuffer()[5][4]);
    assert_eq!(cpu.v[0xF], 0);

    cpu.pc = PROGRAM_START as u16;
    execute_opcode(&mut cpu, 0xD121).unwrap();
    assert!(!cpu.framebuffer()[5][4]);
    assert_eq!(cpu.v[0xF], 1);

    cpu.reset();
    assert_eq!(cpu.framebuffer().as_ptr(), frame_pointer);
    assert!(cpu.framebuffer().iter().flatten().all(|pixel| !pixel));
}
