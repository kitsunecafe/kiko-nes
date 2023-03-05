use std::collections::HashMap;
use crate::cpu::AddressingMode;

pub struct OpCode {
    pub code: u8,
    pub mnemonic: &'static str,
    pub len: u8,
    pub cycles: u8,
    pub mode: AddressingMode
}

impl OpCode {
    pub fn new(code: u8, mnemonic: &'static str, len: u8, cycles: u8, mode: AddressingMode) -> Self {
        OpCode {
            code,
            mnemonic,
            len,
            cycles,
            mode
        }
    }
}

lazy_static!(
    pub static ref CPU_OP_CODES: Vec<OpCode> = vec![
        OpCode::new(0x00, "BRK", 1, 7, AddressingMode::None),
        OpCode::new(0xaa, "TAX", 1, 2, AddressingMode::None),
        OpCode::new(0xe8, "INX", 1, 2, AddressingMode::None),
        OpCode::new(0x48, "PHA", 1, 3, AddressingMode::None),
        OpCode::new(0x08, "PHP", 1, 3, AddressingMode::None),
        OpCode::new(0x68, "PLA", 1, 4, AddressingMode::None),
        OpCode::new(0x28, "PLP", 1, 4, AddressingMode::None),
        OpCode::new(0x40, "RTI", 1, 6, AddressingMode::None),

        OpCode::new(0x69, "ADC", 2, 2, AddressingMode::Immediate),
        OpCode::new(0x65, "ADC", 2, 3, AddressingMode::ZeroPage),
        OpCode::new(0x75, "ADC", 2, 4, AddressingMode::ZeroPageX),
        OpCode::new(0x6d, "ADC", 3, 4, AddressingMode::Absolute),
        OpCode::new(0x7d, "ADC", 3, 4, AddressingMode::AbsoluteX),
        OpCode::new(0x79, "ADC", 3, 4, AddressingMode::AbsoluteY),
        OpCode::new(0x61, "ADC", 2, 6, AddressingMode::IndirectX),
        OpCode::new(0x71, "ADC", 2, 5, AddressingMode::IndirectY),

        OpCode::new(0xe9, "SBC", 2, 2, AddressingMode::Immediate),
        OpCode::new(0xe5, "SBC", 2, 3, AddressingMode::ZeroPage),
        OpCode::new(0xf5, "SBC", 2, 4, AddressingMode::ZeroPageX),
        OpCode::new(0xed, "SBC", 3, 4, AddressingMode::Absolute),
        OpCode::new(0xfd, "SBC", 3, 4, AddressingMode::AbsoluteX),
        OpCode::new(0xf9, "SBC", 3, 4, AddressingMode::AbsoluteY),
        OpCode::new(0xe1, "SBC", 2, 6, AddressingMode::IndirectX),
        OpCode::new(0xf1, "SBC", 2, 5, AddressingMode::IndirectY),

        OpCode::new(0xa9, "LDA", 2, 2, AddressingMode::Immediate),
        OpCode::new(0xa5, "LDA", 2, 3, AddressingMode::ZeroPage),
        OpCode::new(0xb5, "LDA", 2, 4, AddressingMode::ZeroPageX),
        OpCode::new(0xad, "LDA", 3, 4, AddressingMode::Absolute),
        OpCode::new(0xbd, "LDA", 3, 4, AddressingMode::AbsoluteX),
        OpCode::new(0xb9, "LDA", 3, 4, AddressingMode::AbsoluteY),
        OpCode::new(0xa1, "LDA", 2, 6, AddressingMode::IndirectX),
        OpCode::new(0xb1, "LDA", 2, 5, AddressingMode::IndirectY),

        OpCode::new(0xa2, "LDX", 2, 2, AddressingMode::Immediate),
        OpCode::new(0xae, "LDX", 2, 4, AddressingMode::ZeroPage),
        OpCode::new(0xbe, "LDX", 2, 4, AddressingMode::ZeroPageY),
        OpCode::new(0xa6, "LDX", 3, 3, AddressingMode::Absolute),
        OpCode::new(0xb6, "LDX", 3, 4, AddressingMode::AbsoluteY),

        OpCode::new(0xa0, "LDY", 2, 2, AddressingMode::Immediate),
        OpCode::new(0xac, "LDY", 2, 4, AddressingMode::ZeroPage),
        OpCode::new(0xbc, "LDY", 2, 4, AddressingMode::ZeroPageX),
        OpCode::new(0xa4, "LDY", 3, 3, AddressingMode::Absolute),
        OpCode::new(0xb4, "LDY", 3, 4, AddressingMode::AbsoluteX),

        OpCode::new(0x85, "STA", 2, 3, AddressingMode::ZeroPage),
        OpCode::new(0x95, "STA", 2, 4, AddressingMode::ZeroPageX),
        OpCode::new(0x8d, "STA", 3, 4, AddressingMode::Absolute),
        OpCode::new(0x9d, "STA", 3, 5, AddressingMode::AbsoluteX),
        OpCode::new(0x99, "STA", 3, 5, AddressingMode::AbsoluteY),
        OpCode::new(0x81, "STA", 2, 6, AddressingMode::IndirectX),
        OpCode::new(0x91, "STA", 2, 6, AddressingMode::IndirectY),
    ];

    pub static ref OP_CODE_MAP: HashMap<u8, &'static OpCode> = {
        let mut map = HashMap::new();
        for op in &*CPU_OP_CODES {
            map.insert(op.code, op);
        };
        map
    };
);

