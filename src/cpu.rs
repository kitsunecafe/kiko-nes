use crate::bus;
use crate::opcode;
use std::collections::HashMap;

#[derive(Debug, PartialEq)]
pub enum AddressingMode {
    Immediate,
    ZeroPage,
    ZeroPageX,
    ZeroPageY,
    Absolute,
    AbsoluteX,
    AbsoluteY,
    Indirect,
    IndirectX,
    IndirectY,
    None,
}

bitflags! {
    pub struct CPUFlags: u8 {
        const CARRY             = 0b0000_0001; // 0
        const ZERO              = 0b0000_0010; // 1
        const INTERRUPT_DISABLE = 0b0000_0100; // 2
        const DECIMAL           = 0b0000_1000; // 3
        const BREAK             = 0b0001_0000; // 4
        const EXPANSION         = 0b0010_0000; // 5
        const OVERFLOW          = 0b0100_0000; // 6
        const NEGATIVE          = 0b1000_0000; // 7
    }
}

// Stack located at 0x01FF..0x0100
const STACK: u16 = 0x0100;
const STACK_RESET: u8 = 0xfd;

pub struct CPU {
    pub register_a: u8,
    pub register_x: u8,
    pub register_y: u8,
    pub status: CPUFlags,
    pub stack_pointer: u8,
    pub program_counter: u16,
    pub bus: bus::Bus,
}

pub trait Mem {
    fn mem_read(&self, addr: u16) -> u8;
    fn mem_write(&mut self, addr: u16, data: u8);

    fn mem_read_u16(&self, addr: u16) -> u16 {
        let lo = self.mem_read(addr) as u16;
        let hi = self.mem_read(addr + 1) as u16;
        (hi << 8) | lo
    }

    fn mem_write_u16(&mut self, addr: u16, data: u16) {
        let hi = (data >> 8) as u8;
        let lo = (data & 0xff) as u8;
        self.mem_write(addr, lo);
        self.mem_write(addr + 1, hi);
    }
}

impl Mem for CPU {
    fn mem_read(&self, addr: u16) -> u8 {
        self.bus.mem_read(addr)
    }

    fn mem_write(&mut self, addr: u16, data: u8) {
        self.bus.mem_write(addr, data)
    }

    fn mem_read_u16(&self, addr: u16) -> u16 {
        self.bus.mem_read_u16(addr)
    }

    fn mem_write_u16(&mut self, addr: u16, data: u16) {
        self.bus.mem_write_u16(addr, data)
    }
}

impl CPU {
    pub fn new() -> Self {
        CPU {
            register_a: 0,
            register_x: 0,
            register_y: 0,
            program_counter: 0,
            stack_pointer: STACK_RESET,
            status: CPUFlags::from_bits_truncate(0b100100),
            bus: bus::Bus::new(),
        }
    }

    pub fn reset(&mut self) {
        self.register_a = 0;
        self.register_x = 0;
        self.register_y = 0;
        self.stack_pointer = STACK_RESET;
        self.status = CPUFlags::from_bits_truncate(0b100100);

        self.program_counter = self.mem_read_u16(0xFFFC);
    }

    pub fn load(&mut self, program: Vec<u8>) {
        for i in 0..(program.len() as u16) {
            self.mem_write(0x0600 + i, program[i as usize]);
        }

        self.mem_write_u16(0xFFFC, 0x0600);
    }

    pub fn load_and_run(&mut self, program: Vec<u8>) {
        self.load(program);
        self.reset();
        self.run();
    }

    fn stack_push(&mut self, value: u8) {
        self.mem_write(STACK + self.stack_pointer as u16, value);
        self.stack_pointer = self.stack_pointer.wrapping_sub(1);
    }

    fn stack_push_u16(&mut self, value: u16) {
        let hi = (value >> 8) as u8;
        let lo = (value & 0xff) as u8;
        self.stack_push(hi);
        self.stack_push(lo);
    }

    fn stack_pop(&mut self) -> u8 {
        self.stack_pointer = self.stack_pointer.wrapping_add(1);
        self.mem_read(STACK + self.stack_pointer as u16)
    }

    fn stack_pop_u16(&mut self) -> u16 {
        let lo = self.stack_pop() as u16;
        let hi = self.stack_pop() as u16;
        hi << 8 | lo
    }

    pub fn run(&mut self) {
        self.run_with_callback(|_| {});
    }

    pub fn run_with_callback<F>(&mut self, mut callback: F)
    where
        F: FnMut(&mut CPU),
    {
        let ref opcodes: HashMap<u8, &'static opcode::OpCode> = *opcode::OP_CODE_MAP;

        loop {
            let code = self.mem_read(self.program_counter);
            self.program_counter += 1;
            let program_counter_state = self.program_counter;
            let opcode = opcodes.get(&code).unwrap();

            // print!(
            //     "pc: {:#x}, {} ({:#x})",
            //     self.program_counter, opcode.mnemonic, code
            // );

            // if opcode.mode != AddressingMode::None {
            //     print!(" ({:#x})", self.get_operand_addressing(&opcode.mode));
            // }

            // print!("\n");

            match code {
                0xa9 | 0xa5 | 0xb5 | 0xad | 0xbd | 0xb9 | 0xa1 | 0xb1 => {
                    self.lda(&opcode.mode);
                }

                0xa2 | 0xae | 0xbe | 0xa6 | 0xb6 => {
                    self.ldx(&opcode.mode);
                }

                0xa0 | 0xac | 0xbc | 0xa4 | 0xb4 => {
                    self.ldy(&opcode.mode);
                }

                0x85 | 0x95 | 0x8d | 0x9d | 0x99 | 0x81 | 0x91 => {
                    self.sta(&opcode.mode);
                }

                0x8e | 0x86 | 0x96 => {
                    self.stx(&opcode.mode);
                }

                0x8c | 0x84 | 0x94 => {
                    self.sty(&opcode.mode);
                }

                0x69 | 0x65 | 0x75 | 0x6d | 0x7d | 0x79 | 0x61 | 0x71 => {
                    self.adc(&opcode.mode);
                }

                0xe9 | 0xe5 | 0xf5 | 0xed | 0xfd | 0xf9 | 0xe1 | 0xf1 => {
                    self.sbc(&opcode.mode);
                }

                0x0a => {
                    self.asl_a();
                }

                0x0e | 0x1e | 0x06 | 0x16 => {
                    self.asl(&opcode.mode);
                }

                0x4a => {
                    self.lsr_a();
                }

                0x4e | 0x5e | 0x46 | 0x56 => {
                    self.lsr(&opcode.mode);
                }

                0x2a => {
                    self.rol_a();
                }

                0x2e | 0x3e | 0x26 | 0x36 => {
                    self.rol(&opcode.mode);
                }

                0x6a => {
                    self.ror_a();
                }

                0x6e | 0x7e | 0x66 | 0x76 => {
                    self.ror(&opcode.mode);
                }

                0xce | 0xde | 0xc6 | 0xd6 => {
                    self.dec(&opcode.mode);
                }

                0xee | 0xfe | 0xe6 | 0xf6 => {
                    self.inc(&opcode.mode);
                }

                0xc9 | 0xcd | 0xdd | 0xd9 | 0xc5 | 0xd5 | 0xc1 | 0xd1 => {
                    self.compare(&opcode.mode, self.register_a);
                }

                0xe0 | 0xec | 0xe4 => {
                    self.compare(&opcode.mode, self.register_x);
                }

                0xc0 | 0xcc | 0xc4 => {
                    self.compare(&opcode.mode, self.register_y);
                }

                0x29 | 0x2d | 0x3d | 0x39 | 0x25 | 0x35 | 0x21 | 0x31 => {
                    self.and(&opcode.mode);
                }

                0x2c | 0x24 => {
                    self.bit(&opcode.mode);
                }

                0x49 | 0x4d | 0x5d | 0x59 | 0x45 | 0x55 | 0x41 | 0x51 => {
                    self.eor(&opcode.mode);
                }

                0x09 | 0x0d | 0x1d | 0x19 | 0x05 | 0x15 | 0x01 | 0x11 => {
                    self.ora(&opcode.mode);
                }

                0x90 => self.branch(!self.status.contains(CPUFlags::CARRY)),
                0xb0 => self.branch(self.status.contains(CPUFlags::CARRY)),
                0xd0 => self.branch(!self.status.contains(CPUFlags::ZERO)),
                0xf0 => self.branch(self.status.contains(CPUFlags::ZERO)),
                0x10 => self.branch(!self.status.contains(CPUFlags::NEGATIVE)),
                0x30 => self.branch(self.status.contains(CPUFlags::NEGATIVE)),
                0x50 => self.branch(!self.status.contains(CPUFlags::OVERFLOW)),
                0x70 => self.branch(self.status.contains(CPUFlags::OVERFLOW)),

                0x18 => self.remove_carry_flag(),
                0xd8 => self.status.remove(CPUFlags::DECIMAL),
                0x58 => self.status.remove(CPUFlags::INTERRUPT_DISABLE),
                0xb8 => self.status.remove(CPUFlags::OVERFLOW),
                0x38 => self.set_carry_flag(),
                0xf8 => self.status.insert(CPUFlags::DECIMAL),
                0x78 => self.status.insert(CPUFlags::INTERRUPT_DISABLE),

                0x4c => self.jmp_absolute(),
                0x6c => self.jmp_indirect(),
                0x20 => self.jsr(),
                0x40 => self.rti(),
                0x60 => self.rts(),
                0x48 => self.pha(),
                0x08 => self.php(),
                0x68 => self.pla(),
                0x28 => self.plp(),
                0xaa => self.tax(),
                0xa8 => self.tay(),
                0xba => self.tsx(),
                0x8a => self.txa(),
                0x9a => self.txs(),
                0x98 => self.tya(),
                0xca => self.dex(),
                0x88 => self.dey(),
                0xe8 => self.inx(),
                0xc8 => self.iny(),
                0xea => {}
                0x00 => {
                    self.brk();
                    return;
                }
                _ => todo!(),
            }

            if self.program_counter == program_counter_state {
                self.program_counter += (opcode.len - 1) as u16;
            }

            callback(self);
        }
    }

    fn lda(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_addressing(mode);
        let value = self.mem_read(addr);

        self.set_register_a(value);
    }

    fn ldx(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_addressing(mode);
        let value = self.mem_read(addr);
        self.register_x = value;
        self.update_zero_and_set_negative_flags(self.register_x);
    }

    fn ldy(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_addressing(mode);
        let value = self.mem_read(addr);
        self.register_y = value;
        self.update_zero_and_set_negative_flags(self.register_y);
    }

    fn tax(&mut self) {
        self.register_x = self.register_a;
        self.update_zero_and_set_negative_flags(self.register_x);
    }

    fn tay(&mut self) {
        self.register_y = self.register_a;
        self.update_zero_and_set_negative_flags(self.register_y);
    }

    fn tsx(&mut self) {
        self.register_x = self.stack_pointer;
        self.update_zero_and_set_negative_flags(self.register_x);
    }

    fn txa(&mut self) {
        self.register_a = self.register_x;
        self.update_zero_and_set_negative_flags(self.register_a);
    }

    fn txs(&mut self) {
        self.stack_pointer = self.register_x;
    }

    fn tya(&mut self) {
        self.register_a = self.register_y;
        self.update_zero_and_set_negative_flags(self.register_a);
    }

    fn dec(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_addressing(mode);
        let value = self.mem_read(addr).wrapping_sub(1);
        self.mem_write(addr, value);
        self.update_zero_and_set_negative_flags(value);
    }

    fn dex(&mut self) {
        self.register_x = self.register_x.wrapping_sub(1);
        self.update_zero_and_set_negative_flags(self.register_x);
    }

    fn dey(&mut self) {
        self.register_y = self.register_y.wrapping_sub(1);
        self.update_zero_and_set_negative_flags(self.register_y);
    }

    fn inc(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_addressing(mode);
        let value = self.mem_read(addr).wrapping_add(1);
        self.mem_write(addr, value);
        self.update_zero_and_set_negative_flags(value);
    }

    fn inx(&mut self) {
        self.register_x = self.register_x.wrapping_add(1);
        self.update_zero_and_set_negative_flags(self.register_x);
    }

    fn iny(&mut self) {
        self.register_y = self.register_y.wrapping_add(1);
        self.update_zero_and_set_negative_flags(self.register_y);
    }

    fn sta(&mut self, mode: &AddressingMode) {
        self.mem_write(self.get_operand_addressing(mode), self.register_a);
    }

    fn stx(&mut self, mode: &AddressingMode) {
        self.mem_write(self.get_operand_addressing(mode), self.register_x);
    }

    fn sty(&mut self, mode: &AddressingMode) {
        self.mem_write(self.get_operand_addressing(mode), self.register_y);
    }

    fn adc(&mut self, mode: &AddressingMode) {
        self.add_to_register_a(self.read_value_from_memory(mode));
    }

    fn sbc(&mut self, mode: &AddressingMode) {
        let value = self.read_value_from_memory(mode);
        self.add_to_register_a((value as i8).wrapping_neg().wrapping_sub(1) as u8);
    }

    fn pha(&mut self) {
        self.stack_push(self.register_a);
    }

    fn php(&mut self) {
        self.stack_push(self.clone_status(true).bits());
    }

    fn pla(&mut self) {
        let value = self.stack_pop();
        self.set_register_a(value);
    }

    fn plp(&mut self) {
        self.status.bits = self.stack_pop();
        self.status.remove(CPUFlags::BREAK);
    }

    fn brk(&mut self) {
        self.status.insert(CPUFlags::INTERRUPT_DISABLE);
        self.stack_push(self.clone_status(true).bits());
    }

    fn irq(&mut self) {
        self.status.insert(CPUFlags::INTERRUPT_DISABLE);
        self.stack_push(self.clone_status(false).bits());
    }

    fn nmi(&mut self) {
        self.status.insert(CPUFlags::INTERRUPT_DISABLE);
        self.stack_push(self.clone_status(false).bits());
    }

    fn asl_a(&mut self) {
        let value = self.register_a;

        if value >> 7 == 1 {
            self.set_carry_flag();
        } else {
            self.remove_carry_flag();
        }

        self.set_register_a(value << 1);
    }

    fn asl(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_addressing(mode);
        let mut value = self.mem_read(addr);

        if value >> 7 == 1 {
            self.set_carry_flag();
        } else {
            self.remove_carry_flag();
        }

        value = value << 1;
        self.mem_write(addr, value);
        self.update_zero_and_set_negative_flags(value);
    }

    fn lsr_a(&mut self) {
        let value = self.register_a;

        if value & 1 == 1 {
            self.set_carry_flag();
        } else {
            self.remove_carry_flag();
        }

        self.set_register_a(value >> 1);
    }

    fn lsr(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_addressing(mode);
        let mut value = self.mem_read(addr);

        if value & 1 == 1 {
            self.set_carry_flag();
        } else {
            self.remove_carry_flag();
        }

        value = value >> 1;
        self.mem_write(addr, value);
        self.update_zero_and_set_negative_flags(value);
    }

    fn rol_a(&mut self) {
        let mut value = self.register_a;
        let carry = self.status.contains(CPUFlags::CARRY);

        if value >> 7 == 1 {
            self.set_carry_flag();
        } else {
            self.remove_carry_flag();
        }

        value = value << 1;

        if carry {
            value = value | 1;
        }

        self.set_register_a(value);
    }

    fn rol(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_addressing(mode);
        let mut value = self.mem_read(addr);
        let carry = self.status.contains(CPUFlags::CARRY);

        if value >> 7 == 1 {
            self.set_carry_flag();
        } else {
            self.remove_carry_flag();
        }

        value = value << 1;

        if carry {
            value = value | 1;
        }

        self.mem_write(addr, value);
        self.update_zero_and_set_negative_flags(value);
    }

    fn ror_a(&mut self) {
        let mut value = self.register_a;
        let carry = self.status.contains(CPUFlags::CARRY);

        if value & 1 == 1 {
            self.set_carry_flag();
        } else {
            self.remove_carry_flag();
        }

        value = value >> 1;

        if carry {
            value = value | 0b1000_0000;
        }

        self.set_register_a(value);
    }

    fn ror(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_addressing(mode);
        let mut value = self.mem_read(addr);
        let carry = self.status.contains(CPUFlags::CARRY);

        if value & 1 == 1 {
            self.set_carry_flag();
        } else {
            self.remove_carry_flag();
        }

        value = value >> 1;

        if carry {
            value = value | 0b1000_0000;
        }

        self.mem_write(addr, value);
        self.update_negative_flags(value)
    }

    fn compare(&mut self, mode: &AddressingMode, other: u8) {
        let addr = self.get_operand_addressing(mode);
        let memory = self.mem_read(addr);

        if memory <= other {
            self.set_carry_flag();
        } else {
            self.remove_carry_flag();
        }

        self.update_zero_and_set_negative_flags(other.wrapping_sub(memory));
    }

    fn and(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_addressing(mode);
        let value = self.mem_read(addr);
        self.set_register_a(self.register_a & value);
    }

    fn bit(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_addressing(mode);
        let value = self.mem_read(addr);
        let and = self.register_a & value;
        if and == 0 {
            self.status.insert(CPUFlags::ZERO);
        } else {
            self.status.remove(CPUFlags::ZERO);
        }

        self.status.set(CPUFlags::NEGATIVE, value & 0b10000000 > 0);
        self.status.set(CPUFlags::OVERFLOW, value & 0b01000000 > 0);
    }

    fn eor(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_addressing(mode);
        let value = self.mem_read(addr);
        self.set_register_a(self.register_a ^ value);
    }

    fn ora(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_addressing(mode);
        let value = self.mem_read(addr);
        self.set_register_a(self.register_a | value);
    }

    // [PC + 1] -> PCL, [PC + 2] -> PCH
    fn jmp_absolute(&mut self) {
        self.program_counter = self.mem_read_u16(self.program_counter);
    }

    fn jmp_indirect(&mut self) {
        let addr = self.mem_read_u16(self.program_counter);

        let indirect_ref = if addr & 0x00ff == 0x00ff {
            let lo = self.mem_read(addr);
            let hi = self.mem_read(addr & 0xFF00);
            (hi as u16) << 8 | (lo as u16)
        } else {
            self.mem_read_u16(addr)
        };

        self.program_counter = indirect_ref;
    }

    fn jsr(&mut self) {
        self.stack_push_u16(self.program_counter + 1);
        self.program_counter = self.mem_read_u16(self.program_counter);
    }

    fn rti(&mut self) {
        self.status.bits = self.stack_pop();
        self.status.remove(CPUFlags::BREAK);
        self.status.insert(CPUFlags::EXPANSION);
        self.program_counter = self.stack_pop_u16();
    }

    fn rts(&mut self) {
        self.program_counter = self.stack_pop_u16() + 1;
    }

    fn branch(&mut self, condition: bool) {
        if condition {
            let jump = self.mem_read(self.program_counter) as i8;
            let jump_addr = self
                .program_counter
                .wrapping_add(1)
                .wrapping_add(jump as u16);

            self.program_counter = jump_addr;
        }
    }

    fn clone_status(&self, b: bool) -> CPUFlags {
        let mut status = self.status.clone();
        status.insert(CPUFlags::EXPANSION);

        if b {
            status.insert(CPUFlags::BREAK);
        }

        status
    }

    fn set_carry_flag(&mut self) {
        self.status.insert(CPUFlags::CARRY);
    }

    fn remove_carry_flag(&mut self) {
        self.status.remove(CPUFlags::CARRY);
    }

    fn add_to_register_a(&mut self, data: u8) {
        let sum = self.register_a as u16 + data as u16 + self.get_carry() as u16;

        let carry = sum > 0xff;

        if carry {
            self.set_carry_flag();
        } else {
            self.remove_carry_flag();
        }

        let result = sum as u8;

        if (data ^ result) & (self.register_a ^ result) & 0x80 != 0 {
            self.status.insert(CPUFlags::OVERFLOW);
        } else {
            self.status.remove(CPUFlags::OVERFLOW);
        }

        self.set_register_a(result);
    }

    fn get_carry(&self) -> u8 {
        if self.status.contains(CPUFlags::CARRY) {
            1
        } else {
            0
        }
    }

    fn set_register_a(&mut self, value: u8) {
        self.register_a = value;
        self.update_zero_and_set_negative_flags(self.register_a);
    }

    fn read_value_from_memory(&self, mode: &AddressingMode) -> u8 {
        self.mem_read(self.get_operand_addressing(mode))
    }

    fn update_negative_flags(&mut self, result: u8) {
        if result & 0b1000_0000 != 0 {
            self.status.insert(CPUFlags::NEGATIVE);
        } else {
            self.status.remove(CPUFlags::NEGATIVE);
        }
    }

    fn update_zero_flags(&mut self, result: u8) {
        if result == 0 {
            self.status.insert(CPUFlags::ZERO);
        } else {
            self.status.remove(CPUFlags::ZERO);
        }
    }

    fn update_zero_and_set_negative_flags(&mut self, result: u8) {
        self.update_zero_flags(result);
        self.update_negative_flags(result);
    }

    // http://www.emulator101.com/6502-addressing-modes.html
    fn get_operand_addressing(&self, mode: &AddressingMode) -> u16 {
        match mode {
            AddressingMode::Immediate => self.program_counter,
            AddressingMode::ZeroPage => self.mem_read(self.program_counter) as u16,
            AddressingMode::Absolute => self.mem_read_u16(self.program_counter),
            AddressingMode::ZeroPageX => self
                .mem_read(self.program_counter)
                .wrapping_add(self.register_x) as u16,
            AddressingMode::ZeroPageY => self
                .mem_read(self.program_counter)
                .wrapping_add(self.register_y) as u16,
            AddressingMode::AbsoluteX => self
                .mem_read_u16(self.program_counter)
                .wrapping_add(self.register_x as u16),
            AddressingMode::AbsoluteY => self
                .mem_read_u16(self.program_counter)
                .wrapping_add(self.register_y as u16),
            AddressingMode::IndirectX => {
                let addr = self.mem_read(self.program_counter);

                let ptr = (addr as u8).wrapping_add(self.register_x);
                let lo = self.mem_read(ptr as u16);
                let hi = self.mem_read(ptr.wrapping_add(1) as u16);
                (hi as u16) << 8 | (lo as u16)
            }
            AddressingMode::IndirectY => {
                let addr = self.mem_read(self.program_counter);

                let lo = self.mem_read(addr as u16);
                let hi = self.mem_read((addr as u8).wrapping_add(1) as u16);
                let deref_base = (hi as u16) << 8 | (lo as u16);
                deref_base.wrapping_add(self.register_y as u16)
            }
            _ => panic!("AddressingMode {:?} is not supported", mode),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_0xaa_tax_move_a_to_x() {
        let mut cpu = CPU::new();

        cpu.load(vec![0xaa, 0x00]);
        cpu.reset();
        cpu.register_a = 10;
        cpu.run();

        assert_eq!(cpu.register_x, 10);
    }

    #[test]
    fn test_5_ops_working_together() {
        let mut cpu = CPU::new();
        cpu.load_and_run(vec![0xa9, 0xc0, 0xaa, 0xe8, 0x00]);

        assert_eq!(cpu.register_x, 0xc1)
    }

    #[test]
    fn test_0xca_dex_overflow() {
        let mut cpu = CPU::new();
        cpu.load(vec![0xca, 0xca, 0x00]);
        cpu.reset();
        cpu.register_x = 1;
        cpu.run();

        assert_eq!(cpu.register_x, 0xff)
    }

    #[test]
    fn test_0x88_iny_overflow() {
        let mut cpu = CPU::new();
        cpu.load(vec![0x88, 0x88, 0x00]);
        cpu.reset();
        cpu.register_y = 1;
        cpu.run();

        assert_eq!(cpu.register_y, 0xff)
    }

    #[test]
    fn test_0xe8_inx_overflow() {
        let mut cpu = CPU::new();
        cpu.load(vec![0xe8, 0xe8, 0x00]);
        cpu.reset();
        cpu.register_x = 0xff;
        cpu.run();

        assert_eq!(cpu.register_x, 1)
    }

    #[test]
    fn test_0xc8_iny_overflow() {
        let mut cpu = CPU::new();
        cpu.load(vec![0xc8, 0xc8, 0x00]);
        cpu.reset();
        cpu.register_y = 0xff;
        cpu.run();

        assert_eq!(cpu.register_y, 1)
    }

    #[test]
    fn test_adc_0x80_plus_0x80() {
        let mut cpu = CPU::new();
        cpu.load(vec![0x65, 0x10, 0x00]);
        cpu.reset();
        cpu.register_a = 0x80;
        cpu.mem_write(0x10, 0x80);
        cpu.run();

        assert!(cpu.status.contains(CPUFlags::CARRY));
        assert_eq!(cpu.register_a, 0x00);
    }

    #[test]
    fn test_sbc_0x00_sub_0x05() {
        let mut cpu = CPU::new();
        cpu.load(vec![0xe5, 0x10, 0x00]);
        cpu.reset();
        cpu.register_a = 0x00;
        cpu.mem_write(0x10, 0x05);
        cpu.run();

        assert_eq!(cpu.register_a, 0xfa);
    }

    // LDA
    #[test]
    fn test_0xa9_lda_immediate() {
        let mut cpu = CPU::new();

        cpu.load_and_run(vec![0xa9, 0x05, 0x00]);
        assert_eq!(cpu.register_a, 0x05);
        assert!(cpu.status.bits() & 0b0000_0010 == 0b00);
        assert!(cpu.status.bits() & 0b1000_0000 == 0);
    }

    #[test]
    fn test_0xa9_lda_zero_flag() {
        let mut cpu = CPU::new();

        cpu.load_and_run(vec![0xa9, 0x00, 0x00]);
        assert!(cpu.status.bits & 0b0000_0010 == 0b10);
    }

    #[test]
    fn test_lda_from_memory() {
        let mut cpu = CPU::new();
        cpu.mem_write(0x10, 0x55);
        cpu.load_and_run(vec![0xa5, 0x10, 0x00]);

        assert_eq!(cpu.register_a, 0x55);
    }

    #[test]
    fn test_0xa5_lda_zero_page() {
        let mut cpu = CPU::new();

        cpu.load(vec![0xa5, 0x10, 0x00]);
        cpu.reset();
        cpu.mem_write(0x10, 0x14);
        cpu.run();
        assert_eq!(cpu.register_a, 0x14);
    }

    #[test]
    fn test_0xb5_lda_zero_page_x() {
        let mut cpu = CPU::new();

        cpu.load(vec![0xb5, 0x00]);
        cpu.reset();
        cpu.register_x = 0x10;
        cpu.mem_write(0x10, 0x04);
        cpu.run();
        assert_eq!(cpu.register_a, 0x04);
    }

    #[test]
    fn test_0xad_lda_absolute() {
        let mut cpu = CPU::new();

        cpu.load(vec![0xad, 0x00, 0x10, 0x00]);
        cpu.reset();
        cpu.mem_write(0x1000, 0x34);
        cpu.run();
        assert_eq!(cpu.register_a, 0x34);
    }

    #[test]
    fn test_0xbd_lda_absolute_x() {
        let mut cpu = CPU::new();

        cpu.load(vec![0xbd, 0x00, 0x10, 0x00]);
        cpu.reset();
        cpu.register_x = 0x20;
        cpu.mem_write(0x1020, 0x04);
        cpu.run();
        assert_eq!(cpu.register_a, 0x04);
    }

    #[test]
    fn test_0xb9_lda_absolute_y() {
        let mut cpu = CPU::new();

        cpu.load(vec![0xb9, 0x00, 0x10, 0x00]);
        cpu.reset();
        cpu.register_y = 0x10;
        cpu.mem_write(0x1010, 0x02);
        cpu.run();
        assert_eq!(cpu.register_a, 0x02);
    }

    #[test]
    fn test_0xa1_lda_indirect_x() {
        let mut cpu = CPU::new();

        cpu.load(vec![0xa1, 0x10, 0x00]);
        cpu.reset();
        cpu.register_x = 0x00;
        cpu.mem_write(0x10, 0x12);
        cpu.mem_write(0x12, 0x26);
        cpu.run();
        assert_eq!(cpu.register_a, 0x26);
    }

    #[test]
    fn test_0xb1_lda_indirect_y() {
        let mut cpu = CPU::new();

        cpu.load(vec![0xa1, 0x10, 0x00]);
        cpu.reset();
        cpu.register_x = 0x10;
        cpu.mem_write(0x20, 0x12);
        cpu.mem_write(0x12, 0x4a);
        cpu.run();
        assert_eq!(cpu.register_a, 0x4a);
    }

    // LDX
    #[test]
    fn test_0xa2_ldx_immediate() {
        let mut cpu = CPU::new();

        cpu.load_and_run(vec![0xa2, 0x05, 0x00]);
        assert_eq!(cpu.register_x, 0x05);
        assert!(cpu.status.bits() & 0b0000_0010 == 0b00);
        assert!(cpu.status.bits() & 0b1000_0000 == 0);
    }

    #[test]
    fn test_0xae_ldx_absolute() {
        let mut cpu = CPU::new();

        cpu.load(vec![0xae, 0x01, 0x10, 0x00]);
        cpu.reset();
        cpu.mem_write(0x1001, 0xcb);
        cpu.run();
        assert_eq!(cpu.register_x, 0xcb);
    }

    #[test]
    fn test_0xbe_ldx_absolute_y() {
        let mut cpu = CPU::new();

        cpu.load(vec![0xbe, 0x00]);
        cpu.reset();
        cpu.register_y = 0x10;
        cpu.mem_write(0x10, 0xaa);
        cpu.run();
        assert_eq!(cpu.register_x, 0xaa);
    }

    #[test]
    fn test_0xa6_ldx_zero_page() {
        let mut cpu = CPU::new();

        cpu.load(vec![0xa6, 0x12, 0x00]);
        cpu.reset();
        cpu.mem_write(0x12, 0xac);
        cpu.run();
        assert_eq!(cpu.register_x, 0xac);
    }

    #[test]
    fn test_0xb6_ldx_zero_page_y() {
        let mut cpu = CPU::new();

        cpu.load(vec![0xb6, 0x00]);
        cpu.reset();
        cpu.register_y = 0x15;
        cpu.mem_write(0x15, 0xe1);
        cpu.run();
        assert_eq!(cpu.register_x, 0xe1);
    }

    // LDY
    #[test]
    fn test_0xa0_ldy_immediate() {
        let mut cpu = CPU::new();

        cpu.load_and_run(vec![0xa0, 0x05, 0x00]);
        assert_eq!(cpu.register_y, 0x05);
        assert!(cpu.status.bits & 0b0000_0010 == 0b00);
        assert!(cpu.status.bits & 0b1000_0000 == 0);
    }

    #[test]
    fn test_0xa4_ldy_zero_page() {
        let mut cpu = CPU::new();

        cpu.load(vec![0xa4, 0x2e, 0x00]);
        cpu.reset();
        cpu.mem_write(0x2e, 0x61);
        cpu.run();
        assert_eq!(cpu.register_y, 0x61);
    }

    #[test]
    fn test_0xb4_ldy_zero_page_x() {
        let mut cpu = CPU::new();

        cpu.load(vec![0xb4, 0x00]);
        cpu.reset();
        cpu.register_x = 0x10;
        cpu.mem_write(0x10, 0x07);
        cpu.run();
        assert_eq!(cpu.register_y, 0x07);
    }

    #[test]
    fn test_0xac_ldy_absolute() {
        let mut cpu = CPU::new();

        cpu.load(vec![0xac, 0xe2, 0x10, 0x00]);
        cpu.reset();
        cpu.mem_write(0x10e2, 0x66);
        cpu.run();
        assert_eq!(cpu.register_y, 0x66);
    }

    #[test]
    fn test_0xbc_ldy_absolute_x() {
        let mut cpu = CPU::new();

        cpu.load(vec![0xbc, 0x21, 0x00]);
        cpu.reset();
        cpu.register_x = 0x2e;
        cpu.mem_write(0x4f, 0xff);
        cpu.run();
        assert_eq!(cpu.register_y, 0xff);
    }

    // STA
    #[test]
    fn test_0x8d_sta_absolute() {
        let mut cpu = CPU::new();

        cpu.load(vec![0x8d, 0x25, 0x10, 0x00]);
        cpu.reset();
        cpu.register_a = 0xee;
        cpu.run();
        assert_eq!(cpu.mem_read(0x1025), 0xee);
    }

    #[test]
    fn test_0x9d_sta_absolute_x() {
        let mut cpu = CPU::new();

        cpu.load(vec![0x9d, 0x05, 0x10, 0x00]);
        cpu.reset();
        cpu.register_x = 0x02;
        cpu.register_a = 0xc5;
        cpu.run();
        assert_eq!(cpu.mem_read(0x1007), 0xc5);
    }

    #[test]
    fn test_0x99_sta_absolute_y() {
        let mut cpu = CPU::new();

        cpu.load(vec![0x99, 0xe0, 0x1a, 0x00]);
        cpu.reset();
        cpu.register_y = 0x17;
        cpu.register_a = 0x67;
        cpu.run();
        assert_eq!(cpu.mem_read(0x1af7), 0x67);
    }

    #[test]
    fn test_0x85_sta_zero_page() {
        let mut cpu = CPU::new();

        cpu.load(vec![0x85, 0xe5, 0x00]);
        cpu.reset();
        cpu.register_a = 0x0a;
        cpu.run();

        assert_eq!(cpu.mem_read(0xe5), 0x0a);
    }

    #[test]
    fn test_0x95_sta_zero_page_x() {
        let mut cpu = CPU::new();

        cpu.load(vec![0x95, 0x01, 0x00]);
        cpu.reset();
        cpu.register_x = 0x0f;
        cpu.register_a = 0x50;
        cpu.run();

        assert_eq!(cpu.mem_read(0x10), 0x50);
    }

    #[test]
    fn test_0x81_sta_zero_page_y_indirect() {
        let mut cpu = CPU::new();

        cpu.load(vec![0x81, 0x20, 0x00]);
        cpu.reset();
        cpu.register_y = 0x10;
        cpu.register_a = 0x91;
        cpu.mem_write(0x20, 0x1d);
        cpu.run();

        assert_eq!(cpu.mem_read(0x1d), 0x91);
    }

    // STX
    #[test]
    fn test_0x8e_stx_absolute() {
        let mut cpu = CPU::new();

        cpu.load(vec![0x8e, 0x31, 0x10, 0x00]);
        cpu.reset();
        cpu.register_x = 0xfe;
        cpu.run();

        assert_eq!(cpu.mem_read(0x1031), 0xfe);
    }

    #[test]
    fn test_0x86_stx_zero_page() {
        let mut cpu = CPU::new();

        cpu.load(vec![0x86, 0x2e, 0x00]);
        cpu.reset();
        cpu.register_x = 0xcc;
        cpu.run();

        assert_eq!(cpu.mem_read(0x2e), 0xcc);
    }

    #[test]
    fn test_0x96_stx_zero_page_y() {
        let mut cpu = CPU::new();

        cpu.load(vec![0x96, 0x10, 0x00]);
        cpu.reset();
        cpu.register_y = 0x12;
        cpu.register_x = 0xab;
        cpu.run();

        assert_eq!(cpu.mem_read(0x22), 0xab);
    }

    // STY
    #[test]
    fn test_0x8c_sty_absolute() {
        let mut cpu = CPU::new();

        cpu.load(vec![0x8c, 0x1a, 0x12, 0x00]);
        cpu.reset();
        cpu.register_y = 0x71;
        cpu.run();

        assert_eq!(cpu.mem_read(0x121a), 0x71);
    }

    #[test]
    fn test_0x84_sty_zero_page() {
        let mut cpu = CPU::new();

        cpu.load(vec![0x84, 0x19, 0x00]);
        cpu.reset();
        cpu.register_y = 0x9f;
        cpu.run();

        assert_eq!(cpu.mem_read(0x19), 0x9f);
    }

    #[test]
    fn test_0x94_sty_zero_page_x() {
        let mut cpu = CPU::new();

        cpu.load(vec![0x94, 0x14, 0x00]);
        cpu.reset();
        cpu.register_x = 0xa0;
        cpu.register_y = 0x28;
        cpu.run();

        assert_eq!(cpu.mem_read(0xb4), 0x28);
    }

    #[test]
    fn test_0xaa_tax() {
        let mut cpu = CPU::new();

        cpu.load(vec![0xaa, 0x00]);
        cpu.reset();
        cpu.register_a = 0xff;
        cpu.run();

        assert_eq!(cpu.register_a, cpu.register_x);
    }

    #[test]
    fn test_0xa8_tay() {
        let mut cpu = CPU::new();

        cpu.load(vec![0xa8, 0x00]);
        cpu.reset();
        cpu.register_a = 0xff;
        cpu.run();

        assert_eq!(cpu.register_a, cpu.register_y);
    }

    #[test]
    fn test_0xba_tsx() {
        let mut cpu = CPU::new();

        cpu.load(vec![0x08, 0x08, 0xba, 0x00]);
        cpu.reset();
        cpu.run();

        assert_eq!(cpu.register_x, 254);
    }

    #[test]
    fn test_0x8a_txa() {
        let mut cpu = CPU::new();

        cpu.load(vec![0x8a, 0x00]);
        cpu.reset();
        cpu.register_x = 0xaf;
        cpu.run();

        assert_eq!(cpu.register_a, 0xaf);
    }

    #[test]
    fn test_0x9a_txs() {
        let mut cpu = CPU::new();

        cpu.load(vec![0x9a, 0x00]);
        cpu.reset();
        cpu.register_x = 0x05;
        cpu.run();

        assert_eq!(cpu.stack_pointer, 0x04);
    }

    #[test]
    fn test_0x98_tya() {
        let mut cpu = CPU::new();

        cpu.load(vec![0x98, 0x00]);
        cpu.reset();
        cpu.register_y = 0xbe;
        cpu.run();

        assert_eq!(cpu.register_a, 0xbe);
    }

    #[test]
    fn test_0x48_pha() {
        let mut cpu = CPU::new();

        cpu.load(vec![0x48, 0x00]);
        cpu.reset();
        cpu.register_a = 0x8b;
        cpu.run();

        assert_eq!(cpu.mem_read(STACK), 0x8b);
    }

    #[test]
    fn test_0x08_php() {
        let mut cpu = CPU::new();

        cpu.load(vec![0x08, 0x00]);
        cpu.reset();
        cpu.run();

        assert_eq!(cpu.mem_read(STACK), 48);
    }

    #[test]
    fn test_0x68_pla() {
        let mut cpu = CPU::new();

        cpu.load(vec![0x68, 0x00]);
        cpu.reset();
        cpu.stack_push(0x11);
        cpu.run();

        assert_eq!(cpu.register_a, 0x11);
        assert_eq!(cpu.stack_pointer as u16, 255);
    }

    #[test]
    fn test_0x28_plp() {
        let mut cpu = CPU::new();

        cpu.load(vec![0x28, 0x00]);
        cpu.reset();
        cpu.stack_push(0xff);
        cpu.run();

        assert_eq!(cpu.stack_pointer, 0xff);
    }

    #[test]
    fn test_0x0a_asl_accumulator() {
        let mut cpu = CPU::new();

        cpu.load(vec![0x0a, 0x00]);
        cpu.reset();
        cpu.register_a = 0x02;
        cpu.run();

        assert!(!cpu.status.contains(CPUFlags::CARRY));
        assert_eq!(cpu.register_a, 4);
    }

    #[test]
    fn test_0x0a_asl_accumulator_overflow() {
        let mut cpu = CPU::new();

        cpu.load(vec![0x0a, 0x00]);
        cpu.reset();
        cpu.register_a = 0xff;
        cpu.run();

        assert!(cpu.status.contains(CPUFlags::CARRY));
        assert_eq!(cpu.register_a, 0xfe);
    }

    #[test]
    fn test_0x0e_asl_absolute() {
        let mut cpu = CPU::new();

        cpu.load(vec![0x0e, 0x02, 0x10, 0x00]);
        cpu.reset();
        cpu.mem_write(0x1002, 0x04);
        cpu.run();

        assert_eq!(cpu.mem_read(0x1002), 8);
    }

    #[test]
    fn test_0x1e_asl_absolute_x() {
        let mut cpu = CPU::new();

        cpu.load(vec![0x1e, 0x02, 0x10, 0x00]);
        cpu.reset();
        cpu.register_x = 0x14;
        cpu.mem_write(0x1016, 8);
        cpu.run();

        assert_eq!(cpu.mem_read(0x1016), 16);
    }

    #[test]
    fn test_0x06_asl_zero_page() {
        let mut cpu = CPU::new();

        cpu.load(vec![0x06, 0x1e, 0x00]);
        cpu.reset();
        cpu.mem_write(0x001e, 16);
        cpu.run();

        assert_eq!(cpu.mem_read(0x001e), 32);
    }

    #[test]
    fn test_0x16_asl_zero_page_x() {
        let mut cpu = CPU::new();

        cpu.load(vec![0x16, 0x1e, 0x00]);
        cpu.reset();
        cpu.register_x = 0x10;
        cpu.mem_write(0x002e, 32);
        cpu.run();

        assert_eq!(cpu.mem_read(0x2e), 64);
    }

    #[test]
    fn test_0x4a_lsr_accumulator() {
        let mut cpu = CPU::new();

        cpu.load(vec![0x4a, 0x00]);
        cpu.reset();
        cpu.register_a = 64;
        cpu.run();

        assert!(!cpu.status.contains(CPUFlags::CARRY));
        assert_eq!(cpu.register_a, 32);
    }

    #[test]
    fn test_0x4a_lsr_accumulator_overflow() {
        let mut cpu = CPU::new();

        cpu.load(vec![0x4a, 0x00]);
        cpu.reset();
        cpu.register_a = 255;
        cpu.run();

        assert!(cpu.status.contains(CPUFlags::CARRY));
        assert_eq!(cpu.register_a, 127);
    }

    #[test]
    fn test_0x4e_lsr_absolute() {
        let mut cpu = CPU::new();

        cpu.load(vec![0x4e, 0x2e, 0x10, 0x00]);
        cpu.reset();
        cpu.mem_write(0x102e, 32);
        cpu.run();

        assert_eq!(cpu.mem_read(0x102e), 16);
    }

    #[test]
    fn test_0x4e_lsr_absolute_overflow() {
        let mut cpu = CPU::new();

        cpu.load(vec![0x4e, 0x2e, 0x10, 0x00]);
        cpu.reset();
        cpu.mem_write(0x102e, 0x01);
        cpu.run();

        assert!(cpu.status.contains(CPUFlags::CARRY));
        assert_eq!(cpu.mem_read(0x102e), 0);
    }

    #[test]
    fn test_0x5e_lsr_absolute_x() {
        let mut cpu = CPU::new();

        cpu.load(vec![0x5e, 0x52, 0x10, 0x00]);
        cpu.reset();
        cpu.register_x = 0x28;
        cpu.mem_write(0x107a, 8);
        cpu.run();

        assert_eq!(cpu.mem_read(0x107a), 4);
    }

    #[test]
    fn test_0x46_lsr_zero_page() {
        let mut cpu = CPU::new();

        cpu.load(vec![0x46, 0x66, 0x00]);
        cpu.reset();
        cpu.mem_write(0x66, 4);
        cpu.run();

        assert_eq!(cpu.mem_read(0x66), 2);
    }

    #[test]
    fn test_0x56_lsr_zero_page_x() {
        let mut cpu = CPU::new();

        cpu.load(vec![0x56, 0x33, 0x00]);
        cpu.reset();
        cpu.register_x = 0x10;
        cpu.mem_write(0x0043, 2);
        cpu.run();

        assert_eq!(cpu.mem_read(0x43), 1);
    }

    #[test]
    fn test_0xce_dec_absolute() {
        let mut cpu = CPU::new();

        cpu.load(vec![0xce, 0x51, 0x50, 0x00]);
        cpu.reset();
        cpu.run();

        assert_eq!(cpu.mem_read(0x5051), 0xff);
    }

    #[test]
    fn test_0xde_dec_absolute_x() {
        let mut cpu = CPU::new();

        cpu.load(vec![0xde, 0x01, 0x21, 0x00]);
        cpu.reset();
        cpu.register_x = 0x05;
        cpu.run();

        assert_eq!(cpu.mem_read(0x2106), 0xff);
    }

    #[test]
    fn test_0xc6_dec_zero_page() {
        let mut cpu = CPU::new();

        cpu.load(vec![0xc6, 0x16, 0x00]);
        cpu.reset();
        cpu.run();

        assert_eq!(cpu.mem_read(0x16), 0xff);
    }

    #[test]
    fn test_0xd6_dec_zero_page_x() {
        let mut cpu = CPU::new();

        cpu.load(vec![0xd6, 0x20, 0x00]);
        cpu.reset();
        cpu.register_x = 0x1a;
        cpu.mem_write(0x3a, 0x99);
        cpu.run();

        assert_eq!(cpu.mem_read(0x3a), 0x98);
    }

    #[test]
    fn test_0xee_inc_absolute() {
        let mut cpu = CPU::new();

        cpu.load(vec![0xee, 0x0a, 0xf0, 0x00]);
        cpu.reset();
        cpu.run();

        assert_eq!(cpu.mem_read(0xf00a), 0x01);
    }

    #[test]
    fn test_0xfe_inc_absolute_x() {
        let mut cpu = CPU::new();

        cpu.load(vec![0xfe, 0x2a, 0xf1, 0x00]);
        cpu.reset();
        cpu.register_x = 0x0f;
        cpu.run();

        assert_eq!(cpu.mem_read(0xf139), 0x01);
    }

    #[test]
    fn test_0xe6_inc_zero_page() {
        let mut cpu = CPU::new();

        cpu.load(vec![0xe6, 0xff, 0x00]);
        cpu.reset();
        cpu.mem_write(0xff, 0xab);
        cpu.run();

        assert_eq!(cpu.mem_read(0xff), 0xac);
    }

    #[test]
    fn test_0xf6_inc_zero_page_x() {
        let mut cpu = CPU::new();

        cpu.load(vec![0xf6, 0x20, 0x00]);
        cpu.reset();
        cpu.register_x = 0x55;
        cpu.mem_write(0x75, 0x30);
        cpu.run();

        assert_eq!(cpu.mem_read(0x75), 0x31);
    }

    #[test]
    fn test_0xc9_cmp_immediate_positive() {
        let mut cpu = CPU::new();

        cpu.load(vec![0xc9, 10, 0x00]);
        cpu.reset();
        cpu.register_a = 15;
        cpu.run();

        assert!(cpu.status.contains(CPUFlags::CARRY));
        assert!(!cpu.status.contains(CPUFlags::NEGATIVE));
        assert!(!cpu.status.contains(CPUFlags::ZERO));
    }

    #[test]
    fn test_0xc9_cmp_immediate_negative() {
        let mut cpu = CPU::new();

        cpu.load(vec![0xc9, 15, 0x00]);
        cpu.reset();
        cpu.register_a = 10;
        cpu.run();

        assert!(!cpu.status.contains(CPUFlags::CARRY));
        assert!(cpu.status.contains(CPUFlags::NEGATIVE));
        assert!(!cpu.status.contains(CPUFlags::ZERO));
    }

    #[test]
    fn test_0xc9_cmp_immediate_zero() {
        let mut cpu = CPU::new();

        cpu.load(vec![0xc9, 15, 0x00]);
        cpu.reset();
        cpu.register_a = 15;
        cpu.run();

        assert!(cpu.status.contains(CPUFlags::CARRY));
        assert!(!cpu.status.contains(CPUFlags::NEGATIVE));
        assert!(cpu.status.contains(CPUFlags::ZERO));
    }

    #[test]
    fn test_0xc9_cmp_immediate_overflow() {
        let mut cpu = CPU::new();

        cpu.load(vec![0xc9, 0x15, 0x00]);
        cpu.reset();
        cpu.register_a = 0xff;
        cpu.run();

        assert!(cpu.status.contains(CPUFlags::CARRY));
        assert!(cpu.status.contains(CPUFlags::NEGATIVE));
        assert!(!cpu.status.contains(CPUFlags::ZERO));
    }

    #[test]
    fn test_0xe0_cpx_immediate_positive() {
        let mut cpu = CPU::new();

        cpu.load(vec![0xe0, 10, 0x00]);
        cpu.reset();
        cpu.register_x = 15;
        cpu.run();

        assert!(cpu.status.contains(CPUFlags::CARRY));
        assert!(!cpu.status.contains(CPUFlags::NEGATIVE));
        assert!(!cpu.status.contains(CPUFlags::ZERO));
    }

    #[test]
    fn test_0xe0_cpx_immediate_negative() {
        let mut cpu = CPU::new();

        cpu.load(vec![0xe0, 10, 0x00]);
        cpu.reset();
        cpu.register_x = 5;
        cpu.run();

        assert!(!cpu.status.contains(CPUFlags::CARRY));
        assert!(cpu.status.contains(CPUFlags::NEGATIVE));
        assert!(!cpu.status.contains(CPUFlags::ZERO));
    }

    #[test]
    fn test_0xe0_cpx_immediate_zero() {
        let mut cpu = CPU::new();

        cpu.load(vec![0xe0, 5, 0x00]);
        cpu.reset();
        cpu.register_x = 5;
        cpu.run();

        assert!(cpu.status.contains(CPUFlags::CARRY));
        assert!(!cpu.status.contains(CPUFlags::NEGATIVE));
        assert!(cpu.status.contains(CPUFlags::ZERO));
    }

    #[test]
    fn test_0xc0_cpy_immediate_positive() {
        let mut cpu = CPU::new();

        cpu.load(vec![0xc0, 10, 0x00]);
        cpu.reset();
        cpu.register_y = 15;
        cpu.run();

        assert!(cpu.status.contains(CPUFlags::CARRY));
        assert!(!cpu.status.contains(CPUFlags::NEGATIVE));
        assert!(!cpu.status.contains(CPUFlags::ZERO));
    }

    #[test]
    fn test_0xc0_cpy_immediate_negative() {
        let mut cpu = CPU::new();

        cpu.load(vec![0xc0, 10, 0x00]);
        cpu.reset();
        cpu.register_y = 5;
        cpu.run();

        assert!(!cpu.status.contains(CPUFlags::CARRY));
        assert!(cpu.status.contains(CPUFlags::NEGATIVE));
        assert!(!cpu.status.contains(CPUFlags::ZERO));
    }

    #[test]
    fn test_0xc0_cpy_immediate_zero() {
        let mut cpu = CPU::new();

        cpu.load(vec![0xc0, 5, 0x00]);
        cpu.reset();
        cpu.register_y = 5;
        cpu.run();

        assert!(cpu.status.contains(CPUFlags::CARRY));
        assert!(!cpu.status.contains(CPUFlags::NEGATIVE));
        assert!(cpu.status.contains(CPUFlags::ZERO));
    }

    #[test]
    fn test_0x2a_rol_accumulator() {
        let mut cpu = CPU::new();

        cpu.load(vec![0x2a, 0x00]);
        cpu.reset();
        cpu.register_a = 0xff;
        cpu.run();

        assert!(cpu.status.contains(CPUFlags::CARRY));
        assert_eq!(cpu.register_a, 0xfe);
    }

    #[test]
    fn test_0x6a_ror_accumulator() {
        let mut cpu = CPU::new();

        cpu.load(vec![0x6a, 0x00]);
        cpu.reset();
        cpu.register_a = 0xff;
        cpu.run();

        assert!(cpu.status.contains(CPUFlags::CARRY));
        assert_eq!(cpu.register_a, 0x7f);
    }

    #[test]
    fn test_0x6c_jmp_page_bug() {
        let mut cpu = CPU::new();

        cpu.load(vec![0x6c, 0xff, 0x30, 0x00]);
        cpu.reset();
        cpu.mem_write(0x3000, 0x40);
        cpu.mem_write(0x30ff, 0x80);
        cpu.mem_write(0x3100, 0x50);
        cpu.run();
        assert_eq!(cpu.program_counter, 0x4081);
    }

    #[test]
    fn test_0x6c_jmp() {
        let mut cpu = CPU::new();

        cpu.load(vec![0x6c, 0xfe, 0x30, 0x00]);
        cpu.reset();
        cpu.mem_write(0x30ff, 0x40);
        cpu.mem_write(0x30fe, 0x80);
        cpu.mem_write(0x3100, 0x50);
        cpu.run();
        assert_eq!(cpu.program_counter, 0x4081);
    }

    #[test]
    fn test_0x20_jsr() {
        let mut cpu = CPU::new();

        cpu.load(vec![0x20, 0x09, 0x06]);
        cpu.reset();
        cpu.run();
        assert_eq!(cpu.program_counter, 0x060a);
    }
}
