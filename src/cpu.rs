use std::fmt::Write;

use bitflags::bitflags;

use crate::inst::Instruction;
use crate::mmu::Mmu;

fn bank_addr(bank: u8, addr: u16) -> u32 {
    (bank as u32) << 16 | (addr as u32)
}

#[derive(Clone, Copy)]
pub enum Register {
    A,
    D,
    X,
    Y,
}

#[derive(Clone, Copy)]
pub enum AddressingMode {
    Immediate8,
    Immediate16,
    Absolute,
    DirectPage,
    DirectPageIndirectLong,
    AbsoluteIndexedX,
    AbsoluteLongIndexedX,
    AbsoluteIndexedY,
    DirectPageIndexedX,
}

bitflags! {
    pub struct Flags: u8 {
        const CARRY          = 0b00000001;
        const ZERO           = 0b00000010;
        const IRQ_DISABLE    = 0b00000100;
        const DECIMAL_MODE   = 0b00001000;
        const INDEX_REGISTER = 0b00010000;
        const BREAK_FLAG     = 0b00010000;
        const MEMORY_SELECT  = 0b00100000;
        const UNUSED         = 0b00100000;
        const OVERFLOW       = 0b01000000;
        const NEGATIVE       = 0b10000000;
    }
}

pub struct Cpu {
    // Registers
    a: u16,
    x: u16,
    y: u16,
    pc: u16,
    sp: u16,
    direct_page: u16,
    program_bank: u8,
    data_bank: u8,
    status: Flags,
    emulation: bool,

    // Debug info
    sp_base: u16,
}

impl Cpu {
    pub fn new() -> Cpu {
        Cpu {
            a: 0,
            x: 0,
            y: 0,

            pc: 0,
            sp: 0x1FF,
            direct_page: 0,

            program_bank: 0,
            data_bank: 0,

            status: Flags::empty(),

            emulation: true,

            sp_base: 0x1FF,
        }
    }

    pub fn current_addr(&self) -> u32 {
        bank_addr(self.program_bank, self.pc)
    }

    pub fn set_current_addr(&mut self, addr: u32) {
        self.program_bank = (addr >> 16) as u8;
        self.pc = (addr & 0x0000FFFF) as u16;
    }

    pub fn xy_u8_mode(&self) -> bool {
        self.emulation || self.status.contains(Flags::INDEX_REGISTER)
    }

    fn fetch_u8(&mut self, mmu: &Mmu) -> u8 {
        let value = mmu.read_u8(self.current_addr());
        self.pc += 1;

        value
    }

    fn fetch_u16(&mut self, mmu: &Mmu) -> u16 {
        let value = mmu.read_u16(self.current_addr());
        self.pc += 2;

        value
    }

    fn fetch_long(&mut self, mmu: &Mmu) -> u32 {
        let value = mmu.read_long(self.current_addr());
        self.pc += 3;

        value
    }

    fn fetch_addr(&mut self, mmu: &Mmu, addr_mode: AddressingMode) -> u32 {
        match addr_mode {
            AddressingMode::Immediate8 => {
                let addr = self.current_addr();
                self.pc += 1;

                addr
            }

            AddressingMode::Immediate16 => {
                let addr = self.current_addr();
                self.pc += 2;

                addr
            }

            AddressingMode::Absolute => {
                let addr = self.fetch_u16(mmu);

                bank_addr(self.data_bank, addr)
            }

            AddressingMode::DirectPage => {
                let addr = self.fetch_u8(mmu);

                self.direct_page as u32 + addr as u32
            }

            AddressingMode::DirectPageIndirectLong => {
                let ptr = self.fetch_addr(mmu, AddressingMode::DirectPage);

                mmu.read_long(ptr)
            }

            AddressingMode::AbsoluteIndexedX => {
                let addr = self.fetch_u16(mmu);

                bank_addr(self.data_bank, addr) + self.x as u32
            }

            AddressingMode::AbsoluteLongIndexedX => {
                let addr = self.fetch_long(mmu);

                addr + self.x as u32
            }

            AddressingMode::AbsoluteIndexedY => {
                let addr = self.fetch_u16(mmu);

                bank_addr(self.data_bank, addr) + self.y as u32
            }

            AddressingMode::DirectPageIndexedX => {
                let addr = self.fetch_u8(mmu);

                self.direct_page as u32 + addr as u32 + self.x as u32
            }
        }
    }

    fn push_u8(&mut self, mmu: &mut Mmu, value: u8) {
        mmu.store_u8(self.sp as u32, value);
        self.sp -= 1;
    }

    fn push_u16(&mut self, mmu: &mut Mmu, value: u16) {
        mmu.store_u16(self.sp as u32 - 1, value);
        self.sp -= 2;
    }

    fn pull_u8(&mut self, mmu: &mut Mmu) -> u8 {
        self.sp += 1;
        mmu.read_u8(self.sp as u32)
    }

    fn pull_u16(&mut self, mmu: &mut Mmu) -> u16 {
        self.sp += 2;
        mmu.read_u16(self.sp as u32 - 1)
    }

    pub fn get_register(&self, register: Register) -> u16 {
        match register {
            Register::A => self.a,
            Register::D => self.direct_page,
            Register::X => self.x,
            Register::Y => self.y,
        }
    }

    pub fn set_register(&mut self, register: Register, value: u16) {
        match register {
            Register::A => self.a = value,
            Register::D => self.direct_page = value,
            Register::X => self.x = value,
            Register::Y => self.y = value,
        }
    }

    pub fn is_eight_bit_mode(&self, register: Register) -> bool {
        match register {
            Register::A => self.emulation || self.status.contains(Flags::MEMORY_SELECT),
            Register::D => false,
            Register::X | Register::Y => {
                self.emulation || self.status.contains(Flags::INDEX_REGISTER)
            }
        }
    }

    pub fn tick(&mut self, mmu: &mut Mmu) {
        let opcode = self.fetch_u8(mmu);
        let inst = Instruction::from_opcode(opcode);

        match inst {
            Instruction::Unknown { .. } => {}

            Instruction::LoadAImmediate => {
                if self.is_eight_bit_mode(Register::A) {
                    self.load(mmu, Register::A, AddressingMode::Immediate8);
                } else {
                    self.load(mmu, Register::A, AddressingMode::Immediate16);
                }
            }

            Instruction::LoadAAbsolute => {
                self.load(mmu, Register::A, AddressingMode::Absolute);
            }

            Instruction::LoadADirectPage => {
                self.load(mmu, Register::A, AddressingMode::DirectPage);
            }

            Instruction::LoadADirectPageIndirectLong => {
                self.load(mmu, Register::A, AddressingMode::DirectPageIndirectLong);
            }

            Instruction::LoadAAbsoluteIndexedX => {
                self.load(mmu, Register::A, AddressingMode::AbsoluteIndexedX);
            }

            Instruction::LoadAAbsoluteLongIndexedX => {
                self.load(mmu, Register::A, AddressingMode::AbsoluteLongIndexedX);
            }

            Instruction::LoadAAbsoluteIndexedY => {
                self.load(mmu, Register::A, AddressingMode::AbsoluteIndexedY);
            }

            Instruction::LoadXImmediate => {
                if self.xy_u8_mode() {
                    self.load(mmu, Register::X, AddressingMode::Immediate8);
                } else {
                    self.load(mmu, Register::X, AddressingMode::Immediate16);
                }
            }

            Instruction::LoadXDirectPage => {
                self.load(mmu, Register::X, AddressingMode::DirectPage);
            }

            Instruction::LoadYImmediate => {
                if self.xy_u8_mode() {
                    self.load(mmu, Register::Y, AddressingMode::Immediate8);
                } else {
                    self.load(mmu, Register::Y, AddressingMode::Immediate16);
                }
            }

            Instruction::LoadYDirectPage => {
                self.load(mmu, Register::Y, AddressingMode::DirectPage);
            }

            Instruction::StoreAAbsolute => {
                self.store(mmu, Register::A, AddressingMode::Absolute);
            }

            Instruction::StoreADirectPage => {
                self.store(mmu, Register::A, AddressingMode::DirectPage);
            }

            Instruction::StoreAAbsoluteIndexedX => {
                self.store(mmu, Register::A, AddressingMode::AbsoluteIndexedX);
            }

            Instruction::StoreAAbsoluteLongIndexedX => {
                self.store(mmu, Register::A, AddressingMode::AbsoluteLongIndexedX);
            }

            Instruction::StoreAAbsoluteIndexedY => {
                self.store(mmu, Register::A, AddressingMode::AbsoluteIndexedY);
            }

            Instruction::StoreADirectPageIndexedX => {
                self.store(mmu, Register::A, AddressingMode::DirectPageIndexedX);
            }

            Instruction::StoreXAbsolute => {
                self.store(mmu, Register::X, AddressingMode::Absolute);
            }

            Instruction::StoreXDirectPage => {
                self.store(mmu, Register::X, AddressingMode::DirectPage);
            }

            Instruction::StoreYDirectPage => {
                self.store(mmu, Register::Y, AddressingMode::DirectPage);
            }

            Instruction::StoreZeroAbsolute => {
                self.store_zero(mmu, AddressingMode::Absolute);
            }

            Instruction::StoreZeroDirectPage => {
                self.store_zero(mmu, AddressingMode::DirectPage);
            }

            Instruction::StoreZeroAbsoluteIndexedX => {
                self.store_zero(mmu, AddressingMode::AbsoluteIndexedX);
            }

            Instruction::StoreZeroDirectPageIndexedX => {
                self.store_zero(mmu, AddressingMode::DirectPageIndexedX);
            }

            Instruction::AddWithCarryImmediate => {
                if self.is_eight_bit_mode(Register::A) {
                    self.add_with_carry(mmu, AddressingMode::Immediate8);
                } else {
                    self.add_with_carry(mmu, AddressingMode::Immediate16);
                }
            }

            Instruction::AddWithCarryAbsolute => {
                self.add_with_carry(mmu, AddressingMode::Absolute);
            }

            Instruction::AddWithCarryDirectPage => {
                self.add_with_carry(mmu, AddressingMode::DirectPage);
            }

            Instruction::AddWithCarryAbsoluteIndexedY => {
                self.add_with_carry(mmu, AddressingMode::AbsoluteIndexedY);
            }

            Instruction::IncrementDirectPage => {
                self.inc_dec_memory(mmu, AddressingMode::DirectPage, 1);
            }

            Instruction::IncrementA => {
                self.inc_dec_register(Register::A, 1);
            }

            Instruction::IncrementX => {
                self.inc_dec_register(Register::X, 1);
            }

            Instruction::IncrementY => {
                self.inc_dec_register(Register::Y, 1);
            }

            Instruction::DecrementX => {
                self.inc_dec_register(Register::X, -1);
            }

            Instruction::DecrementY => {
                self.inc_dec_register(Register::Y, -1);
            }

            Instruction::ShiftLeft => {
                // TODO: 16 bit mode
                let original = self.a as u8;
                let value = original << 1;

                self.status.set(Flags::NEGATIVE, (value >> 7) & 1 == 1);
                self.status.set(Flags::ZERO, self.y == 0);
                self.status.set(Flags::CARRY, (original >> 7) & 1 == 1);

                self.a = value as u16;
            }

            Instruction::MoveAX => {
                // TODO: 8 bit mode
                self.x = self.a;

                self.status.set(Flags::NEGATIVE, (self.x >> 15) & 1 == 1);
                self.status.set(Flags::ZERO, self.x == 0);
            }

            Instruction::MoveAY => {
                // TODO: 8 bit mode
                self.y = self.a;

                self.status.set(Flags::NEGATIVE, (self.y >> 15) & 1 == 1);
                self.status.set(Flags::ZERO, self.y == 0);
            }

            Instruction::MoveDA => {
                // NOTE: This is always 16 bit, regardless of flags
                self.a = self.direct_page;

                self.status.set(Flags::NEGATIVE, (self.a >> 15) & 1 == 1);
                self.status.set(Flags::ZERO, self.a == 0);
            }

            Instruction::MoveXSP => {
                // TODO: Emulation mode
                self.sp = self.x;
                self.sp_base = self.x;

                self.status.set(Flags::NEGATIVE, (self.sp >> 15) & 1 == 1);
                self.status.set(Flags::ZERO, self.sp == 0);
            }

            Instruction::MoveYA => {
                if self.is_eight_bit_mode(Register::A) || self.is_eight_bit_mode(Register::Y) {
                    // TODO: This shouldn't wipe out the high byte when A is 8bit.
                    self.a = self.y & 0x00FF;
                } else {
                    self.a = self.y;
                }
            }

            Instruction::ExchangeBA => {
                self.a = (self.a << 8) | (self.a >> 8);

                // TODO: I don't think these are right
                self.status.set(Flags::NEGATIVE, (self.a & 1) == 1);
                self.status.set(Flags::ZERO, (self.a & 1) == 1);
            }

            Instruction::BlockMoveNext => {
                // TODO: 8 bit index registers - tbh I'm not sure about this one
                let dest = self.fetch_u8(mmu);
                let src = self.fetch_u8(mmu);

                self.data_bank = dest;

                while self.a != 0xFFFF {
                    // TODO: Add a way to break out of this if it gets stuck

                    let value = mmu.read_u8(bank_addr(src, self.x));
                    mmu.store_u8(bank_addr(self.data_bank, self.y), value);

                    self.a = self.a.wrapping_sub(1);
                    self.x = self.x.wrapping_add(1);
                    self.y = self.y.wrapping_add(1);
                }
            }

            Instruction::CompareImmediate => {
                if self.is_eight_bit_mode(Register::A) {
                    self.compare(mmu, Register::A, AddressingMode::Immediate8);
                } else {
                    self.compare(mmu, Register::A, AddressingMode::Immediate16);
                }
            }

            Instruction::CompareAbsolute => {
                self.compare(mmu, Register::A, AddressingMode::Absolute);
            }

            Instruction::CompareDirectPage => {
                self.compare(mmu, Register::A, AddressingMode::DirectPage);
            }

            Instruction::CompareAbsoluteLongIndexedX => {
                self.compare(mmu, Register::A, AddressingMode::AbsoluteLongIndexedX);
            }

            Instruction::CompareXImmediate => {
                if self.xy_u8_mode() {
                    self.compare(mmu, Register::X, AddressingMode::Immediate8);
                } else {
                    self.compare(mmu, Register::X, AddressingMode::Immediate16);
                }
            }

            Instruction::BranchCarryClear => {
                self.branch(mmu, !self.status.contains(Flags::CARRY));
            }

            Instruction::BranchCarrySet => {
                self.branch(mmu, self.status.contains(Flags::CARRY));
            }

            Instruction::BranchNotEqual => {
                self.branch(mmu, !self.status.contains(Flags::ZERO));
            }

            Instruction::BranchEqual => {
                self.branch(mmu, self.status.contains(Flags::ZERO));
            }

            Instruction::BranchAlways => {
                self.branch(mmu, true);
            }

            Instruction::PushA => {
                if self.is_eight_bit_mode(Register::A) {
                    self.push_u8(mmu, self.a as u8);
                } else {
                    self.push_u16(mmu, self.a);
                }
            }

            Instruction::PushB => {
                self.push_u8(mmu, self.data_bank);
            }

            Instruction::PushD => {
                self.push_u16(mmu, self.direct_page);
            }

            Instruction::PushX => {
                if self.xy_u8_mode() {
                    self.push_u8(mmu, self.x as u8);
                } else {
                    self.push_u16(mmu, self.x);
                }
            }

            Instruction::PushY => {
                if self.xy_u8_mode() {
                    self.push_u8(mmu, self.y as u8);
                } else {
                    self.push_u16(mmu, self.y);
                }
            }

            Instruction::PushStatus => {
                self.push_u8(mmu, self.status.bits());
            }

            Instruction::PushAbsolute => {
                let addr = self.fetch_u16(mmu);

                self.push_u16(mmu, addr);
            }

            Instruction::PullA => {
                self.pull(mmu, Register::A);
            }

            Instruction::PullB => {
                // TODO: Can't use helper function here because target is a u8
                let value = self.pull_u8(mmu);

                self.data_bank = value;

                self.status.set(Flags::NEGATIVE, (value >> 7) & 1 == 1);
                self.status.set(Flags::ZERO, value == 0);
            }

            Instruction::PullD => {
                self.pull(mmu, Register::D);
            }

            Instruction::PullX => {
                self.pull(mmu, Register::X);
            }

            Instruction::PullY => {
                self.pull(mmu, Register::Y);
            }

            Instruction::PullStatus => {
                let value = self.pull_u8(mmu);

                self.status = Flags::from_bits_truncate(value);

                self.status.set(Flags::NEGATIVE, (value >> 7) & 1 == 1);
                self.status.set(Flags::ZERO, value == 0);
            }

            Instruction::JumpAbsolute => {
                let addr = self.fetch_u16(mmu);

                self.pc = addr;
            }

            Instruction::JumpSubRoutineAbsolute => {
                let addr = self.fetch_u16(mmu);

                self.push_u16(mmu, self.pc - 1); // TODO: bytes are reversed

                self.pc = addr;
            }

            Instruction::JumpSubRoutineAbsoluteLong => {
                let addr = self.fetch_u16(mmu);
                let bank = self.fetch_u8(mmu);

                self.push_u16(mmu, self.pc - 1); // TODO: bytes are reversed
                self.push_u8(mmu, self.program_bank);

                self.program_bank = bank;
                self.pc = addr;
            }

            Instruction::Return => {
                let addr = self.pull_u16(mmu);

                self.pc = addr.wrapping_add(1);
            }

            Instruction::ReturnLong => {
                let bank = self.pull_u8(mmu);
                let addr = self.pull_u16(mmu);

                self.pc = addr.wrapping_add(1);
                self.program_bank = bank;
            }

            Instruction::ClearCarry => {
                self.status.remove(Flags::CARRY);
            }

            Instruction::SetIrqDisable => {
                self.status.insert(Flags::IRQ_DISABLE);
            }

            Instruction::ResetFlags => {
                let mask = self.fetch_u8(mmu);

                self.status &= !Flags::from_bits_truncate(mask);
            }

            Instruction::SetFlags => {
                let mask = self.fetch_u8(mmu);

                self.status |= Flags::from_bits_truncate(mask);
            }

            Instruction::ExchangeCE => {
                let carry = self.status.contains(Flags::CARRY);

                self.emulation = carry;
                self.status.toggle(Flags::CARRY);
            }

            Instruction::Break => {
                // TODO: Probably not right but let's see if it works
            }
        }
    }

    pub fn load(&mut self, mmu: &Mmu, register: Register, addr_mode: AddressingMode) {
        let addr = self.fetch_addr(mmu, addr_mode);

        if self.is_eight_bit_mode(register) {
            let value = mmu.read_u8(addr);

            self.set_register(register, value as u16);

            self.status.set(Flags::NEGATIVE, (value >> 7) & 1 == 1);
            self.status.set(Flags::ZERO, value == 0);
        } else {
            let value = mmu.read_u16(addr);

            self.set_register(register, value);

            self.status.set(Flags::NEGATIVE, (value >> 15) & 1 == 1);
            self.status.set(Flags::ZERO, value == 0);
        }
    }

    pub fn pull(&mut self, mmu: &mut Mmu, register: Register) {
        if self.is_eight_bit_mode(register) {
            let value = self.pull_u8(mmu);

            self.set_register(register, value as u16);

            self.status.set(Flags::NEGATIVE, (value >> 7) & 1 == 1);
            self.status.set(Flags::ZERO, value == 0);
        } else {
            let value = self.pull_u16(mmu);

            self.set_register(register, value);

            self.status.set(Flags::NEGATIVE, (value >> 15) & 1 == 1);
            self.status.set(Flags::ZERO, value == 0);
        }
    }

    pub fn store(&mut self, mmu: &mut Mmu, register: Register, addr_mode: AddressingMode) {
        let addr = self.fetch_addr(mmu, addr_mode);

        if self.is_eight_bit_mode(register) {
            mmu.store_u8(addr, self.get_register(register) as u8);
        } else {
            mmu.store_u16(addr, self.get_register(register));
        }
    }

    pub fn store_zero(&mut self, mmu: &mut Mmu, addr_mode: AddressingMode) {
        let addr = self.fetch_addr(mmu, addr_mode);

        if self.is_eight_bit_mode(Register::A) {
            mmu.store_u8(addr, 0);
        } else {
            mmu.store_u16(addr, 0);
        }
    }

    pub fn add_with_carry(&mut self, mmu: &Mmu, addr_mode: AddressingMode) {
        let addr = self.fetch_addr(mmu, addr_mode);

        if self.is_eight_bit_mode(Register::A) {
            let value = mmu.read_u8(addr);

            let result = (self.a as u8)
                .wrapping_add(value)
                .wrapping_add(self.status.contains(Flags::CARRY) as u8);

            self.status.set(Flags::NEGATIVE, (result >> 7) & 1 == 1);
            self.status.set(Flags::ZERO, result == 0);
            self.status.set(Flags::CARRY, result < self.a as u8);

            // TODO: Overflow flag

            self.a = result as u16;
        } else {
            let value = mmu.read_u16(addr);

            let result = self
                .a
                .wrapping_add(value)
                .wrapping_add(self.status.contains(Flags::CARRY) as u16);

            self.status.set(Flags::NEGATIVE, (result >> 15) & 1 == 1);
            self.status.set(Flags::ZERO, result == 0);
            self.status.set(Flags::CARRY, result < self.a);

            // TODO: Overflow flag

            self.a = result;
        }
    }

    pub fn inc_dec_register(&mut self, register: Register, amount: i8) {
        if self.is_eight_bit_mode(register) {
            let value = (self.get_register(register) as u8).wrapping_add_signed(amount);

            // TODO: This shouldn't wipe out upper byte
            self.set_register(register, value as u16);

            self.status.set(Flags::NEGATIVE, (value >> 7) & 1 == 1);
            self.status.set(Flags::ZERO, value == 0);
        } else {
            let value = self
                .get_register(register)
                .wrapping_add_signed(amount.into());

            self.set_register(register, value);

            self.status.set(Flags::NEGATIVE, (value >> 15) & 1 == 1);
            self.status.set(Flags::ZERO, value == 0);
        }
    }

    pub fn inc_dec_memory(&mut self, mmu: &mut Mmu, addr_mode: AddressingMode, amount: i8) {
        // TODO: Can this be 16-bit?

        let addr = self.fetch_addr(mmu, addr_mode);
        let value = mmu.read_u8(addr).wrapping_add_signed(amount);

        mmu.store_u8(addr, value);

        self.status.set(Flags::NEGATIVE, (value >> 7) & 1 == 1);
        self.status.set(Flags::ZERO, value == 0);
    }

    pub fn compare(&mut self, mmu: &Mmu, register: Register, addr_mode: AddressingMode) {
        let addr = self.fetch_addr(mmu, addr_mode);

        if self.is_eight_bit_mode(register) {
            let lhs = self.get_register(register) as u8;
            let rhs = mmu.read_u8(addr);

            let result = lhs.wrapping_sub(rhs);

            self.status.set(Flags::NEGATIVE, (result >> 7) & 1 == 1);
            self.status.set(Flags::ZERO, result == 0);
            self.status.set(Flags::CARRY, lhs >= rhs);
        } else {
            let lhs = self.get_register(register);
            let rhs = mmu.read_u16(addr);

            let result = lhs.wrapping_sub(rhs);

            self.status.set(Flags::NEGATIVE, (result >> 15) & 1 == 1);
            self.status.set(Flags::ZERO, result == 0);
            self.status.set(Flags::CARRY, lhs >= rhs);
        }
    }

    pub fn branch(&mut self, mmu: &Mmu, should_branch: bool) {
        let offset = self.fetch_u8(mmu);

        if should_branch {
            let sign_bit = offset >> 7;

            // TODO: Is this overflow behaviour right, or should it increment the bank?
            if sign_bit == 1 {
                self.pc = self.pc.wrapping_sub((!offset + 1) as u16);
            } else {
                self.pc = self.pc.wrapping_add(offset as u16);
            }
        }
    }

    pub fn register_debug(&self) -> String {
        fn flag_or_empty(flag: &str, value: bool) -> &str {
            if value {
                flag
            } else {
                ""
            }
        }

        format!(
            "A: {:04X} | X: {:04X} | Y: {:04X} | SP: {:04X} | D: {:04X} | DB: {:02X} | PB: {:02X} | Flags: {}{}{}{}{}{}{}{}{}",
            self.a,
            self.x,
            self.y,
            self.sp,
            self.direct_page,
            self.data_bank,
            self.program_bank,
            flag_or_empty("N", self.status.contains(Flags::NEGATIVE)),
            flag_or_empty("V", self.status.contains(Flags::OVERFLOW)),
            flag_or_empty("M", self.status.contains(Flags::MEMORY_SELECT)),
            flag_or_empty("X", self.status.contains(Flags::INDEX_REGISTER)),
            flag_or_empty("D", self.status.contains(Flags::DECIMAL_MODE)),
            flag_or_empty("I", self.status.contains(Flags::IRQ_DISABLE)),
            flag_or_empty("Z", self.status.contains(Flags::ZERO)),
            flag_or_empty("C", self.status.contains(Flags::CARRY)),
            flag_or_empty("E", self.emulation),
        )
    }

    pub fn stack_debug(&self, mmu: &Mmu) -> String {
        let mut output = String::new();

        for addr in (self.sp + 1..=self.sp_base).rev() {
            // TODO: Is stack always zero paged?
            write!(
                &mut output,
                "0x{:02X}{}",
                mmu.read_u8(addr as u32),
                if addr == self.sp + 1 { "" } else { ", " }
            )
            .unwrap();
        }

        output
    }
}
