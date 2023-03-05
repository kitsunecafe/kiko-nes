use std::ops::Add;

use crate::opcode;

#[derive(Debug)]
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
pub const STACK: u16 = 0x01FF;

pub struct CPU {
    pub register_a: u8,
    pub register_x: u8,
    pub register_y: u8,
    pub status: CPUFlags,
    pub stack_pointer: u8,
    pub program_counter: u16,
    memory: [u8; 0xFFFF],
}

trait Mem {
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
        self.memory[addr as usize]
    }

    fn mem_write(&mut self, addr: u16, data: u8) {
        self.memory[addr as usize] = data;
    }
}

impl CPU {
    pub fn new() -> Self {
        CPU {
            register_a: 0,
            register_x: 0,
            register_y: 0,
            status: CPUFlags::empty(),
            stack_pointer: 0,
            program_counter: 0,
            memory: [0; 0xFFFF],
        }
    }

    pub fn reset(&mut self) {
        self.register_a = 0;
        self.register_x = 0;
        self.status = CPUFlags::empty();

        self.program_counter = self.mem_read_u16(0xFFFC);
    }

    pub fn load(&mut self, program: Vec<u8>) {
        self.memory[0x8000..(0x8000 + program.len())].copy_from_slice(&program[..]);
        self.mem_write_u16(0xFFFC, 0x8000);
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
        let ref opcodes = *opcode::OP_CODE_MAP;

        loop {
            let code = self.mem_read(self.program_counter);
            self.program_counter += 1;
            let program_counter_state = self.program_counter;
            let opcode = opcodes
                .get(&code)
                .expect(&format!("Opcode {:?} is not recognized.", code));

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

                0x69 | 0x65 | 0x75 | 0x6d | 0x7d | 0x79 | 0x61 | 071 => {
                    self.adc(&opcode.mode);
                }

                0xe9 | 0xe5 | 0xf5 | 0xed | 0xfd | 0xf9 | 0xe1 | 0xf1 => {
                    self.sbc(&opcode.mode);
                }

                0x48 => self.pha(),
                0x08 => self.php(),
                0x68 => self.pla(),
                0x28 => self.plp(),
                0xaa => self.tax(),
                0xe8 => self.inx(),
                // 0x40 => self.rti(),
                0x00 => {
                    self.brk();
                    return
                }
                _ => todo!(),
            }

            if self.program_counter == program_counter_state {
                self.program_counter += (opcode.len - 1) as u16;
            }
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

    fn inx(&mut self) {
        self.register_x = self.register_x.wrapping_add(1);
        self.update_zero_and_set_negative_flags(self.register_x);
    }

    fn sta(&mut self, mode: &AddressingMode) {
        self.mem_write(self.get_operand_addressing(mode), self.register_a);
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

    fn clone_status(&self, b: bool) -> CPUFlags {
        let mut status = self.status.clone();
        status.insert(CPUFlags::EXPANSION);

        if b {
            status.insert(CPUFlags::BREAK);
        }

        status
    }

    fn add_to_register_a(&mut self, data: u8) {
        let sum = self.register_a as u16 + data as u16 + self.get_carry() as u16;

        let carry = sum > 0xff;

        if carry {
            self.status.insert(CPUFlags::CARRY);
        } else {
            self.status.remove(CPUFlags::CARRY);
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

    fn update_zero_and_set_negative_flags(&mut self, result: u8) {
        if result == 0 {
            self.status.insert(CPUFlags::ZERO);
        } else {
            self.status.remove(CPUFlags::ZERO);
        }

        if result & 0b1000_0000 != 0 {
            self.status.insert(CPUFlags::NEGATIVE);
        } else {
            self.status.remove(CPUFlags::NEGATIVE);
        }
    }

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
                let hi = self.mem_read(addr.wrapping_add(1) as u16);
                let deref_base = (hi as u16) << 8 | (lo as u16);
                deref_base.wrapping_add(self.register_y as u16)
            }
            _ => panic!("{:?} is not supported", mode),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_0xa9_lda_immediate_load_data() {
        let mut cpu = CPU::new();

        cpu.load_and_run(vec![0xa9, 0x05, 0x00]);
        assert_eq!(cpu.register_a, 0x05);
        assert!(cpu.status.bits & 0b0000_0010 == 0b00);
        assert!(cpu.status.bits & 0b1000_0000 == 0);
    }

    #[test]
    fn test_0xa9_lda_zero_flag() {
        let mut cpu = CPU::new();

        cpu.load_and_run(vec![0xa9, 0x00, 0x00]);
        assert!(cpu.status.bits & 0b0000_0010 == 0b10);
    }

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
    fn test_inx_overflow() {
        let mut cpu = CPU::new();
        cpu.load(vec![0xe8, 0xe8, 0x00]);
        cpu.reset();
        cpu.register_x = 0xff;
        cpu.run();

        assert_eq!(cpu.register_x, 1)
    }

    #[test]
    fn test_lda_from_memory() {
        let mut cpu = CPU::new();
        cpu.mem_write(0x10, 0x55);
        cpu.load_and_run(vec![0xa5, 0x10, 0x00]);

        assert_eq!(cpu.register_a, 0x55);
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

    #[test]
    fn test_0xa2_ldx_immediate_load_data() {
        let mut cpu = CPU::new();

        cpu.load_and_run(vec![0xa2, 0x05, 0x00]);
        assert_eq!(cpu.register_x, 0x05);
        assert!(cpu.status.bits & 0b0000_0010 == 0b00);
        assert!(cpu.status.bits & 0b1000_0000 == 0);
    }

    #[test]
    fn test_0xa0_ldy_immediate_load_data() {
        let mut cpu = CPU::new();

        cpu.load_and_run(vec![0xa0, 0x05, 0x00]);
        assert_eq!(cpu.register_y, 0x05);
        assert!(cpu.status.bits & 0b0000_0010 == 0b00);
        assert!(cpu.status.bits & 0b1000_0000 == 0);
    }

    #[test]
    fn test_0xb4_ldy_zero_x_load_data() {
        let mut cpu = CPU::new();

        cpu.load(vec![0xb4, 0x00]);
        cpu.reset();
        cpu.register_x = 0x07;
        cpu.run();
        assert_eq!(cpu.register_y, 0x07);
    }

    #[test]
    fn test_0xb6_ldx_zero_y_load_data() {
        let mut cpu = CPU::new();

        cpu.load(vec![0xb6, 0x00]);
        cpu.reset();
        cpu.register_y = 0x10;
        cpu.mem_write(0x10, 0x12);
        cpu.run();
        assert_eq!(cpu.register_x, 0x12);
    }
}
