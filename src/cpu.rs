mod ops;

use std::fmt::Write;

use bitflags::bitflags;

use self::ops::*;
use crate::inst::Instruction;
use crate::mmu::Mmu;

fn bank_addr(bank: u8, addr: u16) -> u32 {
    (bank as u32) << 16 | (addr as u32)
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

    pub fn a_u8_mode(&self) -> bool {
        self.emulation || self.status.contains(Flags::MEMORY_SELECT)
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

    pub fn tick(&mut self, mmu: &mut Mmu) {
        let opcode = self.fetch_u8(mmu);
        let inst = Instruction::from_opcode(opcode);

        match inst {
            Instruction::Unknown { .. } => {}

            Instruction::LoadAImmediate => {
                if self.a_u8_mode() {
                    let value = self.fetch_u8(mmu);
                    load_u8(&mut self.a, &mut self.status, value);
                } else {
                    let value = self.fetch_u16(mmu);
                    load_u16(&mut self.a, &mut self.status, value);
                }
            }

            Instruction::LoadAAbsolute => {
                // TODO: 16 bit mode
                let addr = self.fetch_u16(mmu);

                load_u8(
                    &mut self.a,
                    &mut self.status,
                    mmu.read_u8(bank_addr(self.data_bank, addr)),
                )
            }

            Instruction::LoadADirectPage => {
                // TODO: 16 bit mode
                let addr = self.fetch_u8(mmu);

                load_u8(
                    &mut self.a,
                    &mut self.status,
                    mmu.read_u8(self.direct_page as u32 + addr as u32),
                )
            }

            Instruction::LoadADirectPageIndirectLong => {
                let addr = self.fetch_u8(mmu);

                let ptr = mmu.read_long(self.direct_page as u32 + addr as u32);

                // TODO: 16 bit mode
                load_u8(&mut self.a, &mut self.status, mmu.read_u8(ptr))
            }

            Instruction::LoadAAbsoluteIndexedX => {
                // TODO: 16 bit mode
                let addr = self.fetch_u16(mmu);

                load_u8(
                    &mut self.a,
                    &mut self.status,
                    mmu.read_u8(bank_addr(self.data_bank, addr) + self.x as u32),
                )
            }

            Instruction::LoadAAbsoluteLongIndexedX => {
                // TODO: 16 bit mode
                let addr = self.fetch_long(mmu);

                load_u8(
                    &mut self.a,
                    &mut self.status,
                    mmu.read_u8(addr + self.x as u32),
                )
            }

            Instruction::LoadAAbsoluteIndexedY => {
                // TODO: 16 bit mode
                let addr = self.fetch_u16(mmu);

                load_u8(
                    &mut self.a,
                    &mut self.status,
                    mmu.read_u8(bank_addr(self.data_bank, addr) + self.y as u32),
                )
            }

            Instruction::LoadXImmediate => {
                if self.xy_u8_mode() {
                    let value = self.fetch_u8(mmu);
                    load_u8(&mut self.x, &mut self.status, value);
                } else {
                    let value = self.fetch_u16(mmu);
                    load_u16(&mut self.x, &mut self.status, value)
                }
            }

            Instruction::LoadXDirectPage => {
                // TODO: 8 bit mode?
                let addr = self.fetch_u8(mmu);

                load_u16(
                    &mut self.x,
                    &mut self.status,
                    mmu.read_u16(self.direct_page as u32 + addr as u32),
                )
            }

            Instruction::LoadYImmediate => {
                if self.xy_u8_mode() {
                    let value = self.fetch_u8(mmu);
                    load_u8(&mut self.y, &mut self.status, value);
                } else {
                    let value = self.fetch_u16(mmu);
                    load_u16(&mut self.y, &mut self.status, value)
                }
            }

            Instruction::LoadYDirectPage => {
                // TODO: 8 bit mode?
                let addr = self.fetch_u8(mmu);

                load_u16(
                    &mut self.y,
                    &mut self.status,
                    mmu.read_u16(self.direct_page as u32 + addr as u32),
                )
            }

            Instruction::StoreAAbsolute => {
                // TODO: 8 bit mode?
                let addr = self.fetch_u16(mmu);

                mmu.store_u16(bank_addr(self.data_bank, addr), self.a);
            }

            Instruction::StoreADirectPage => {
                // TODO: 8 bit mode?
                let addr = self.fetch_u8(mmu);

                mmu.store_u16(self.direct_page as u32 + addr as u32, self.a);
            }

            Instruction::StoreAAbsoluteIndexedX => {
                // TODO: 8 bit mode?
                let addr = self.fetch_u16(mmu);

                mmu.store_u16(bank_addr(self.data_bank, addr) + self.x as u32, self.a);
            }

            Instruction::StoreAAbsoluteLongIndexedX => {
                // TODO: 8 bit mode?
                let addr = self.fetch_long(mmu);

                mmu.store_u16(addr + self.x as u32, self.a);
            }

            Instruction::StoreADirectPageIndexedX => {
                // TODO: 8 bit mode?
                let addr = self.fetch_u8(mmu);

                mmu.store_u16(
                    self.direct_page as u32 + addr as u32 + self.x as u32,
                    self.a,
                );
            }

            Instruction::StoreXAbsolute => {
                // TODO: 8 bit mode?
                let addr = self.fetch_u16(mmu);

                mmu.store_u16(bank_addr(self.data_bank, addr), self.x);
            }

            Instruction::StoreXDirectPage => {
                // TODO: 8 bit mode?
                let addr = self.fetch_u8(mmu);

                mmu.store_u16(self.direct_page as u32 + addr as u32, self.x);
            }

            Instruction::StoreYDirectPage => {
                // TODO: 8 bit mode?
                let addr = self.fetch_u8(mmu);

                mmu.store_u16(self.direct_page as u32 + addr as u32, self.y);
            }

            Instruction::StoreZeroAbsolute => {
                let addr = self.fetch_u16(mmu);

                mmu.store_u8(bank_addr(self.data_bank, addr), 0);
            }

            Instruction::StoreZeroDirectPage => {
                let addr = self.fetch_u16(mmu);

                mmu.store_u8(self.direct_page as u32 + addr as u32, 0);
            }

            Instruction::StoreZeroAbsoluteIndexedX => {
                let addr = self.fetch_u16(mmu);

                mmu.store_u8(bank_addr(self.data_bank, addr) + self.x as u32, 0);
            }

            Instruction::StoreZeroDirectPageIndexedX => {
                let addr = self.fetch_u8(mmu);

                mmu.store_u8(self.direct_page as u32 + addr as u32 + self.x as u32, 0);
            }

            Instruction::AddWithCarryImmediate => {
                if self.a_u8_mode() {
                    let value = self.fetch_u8(mmu);
                    adc_u8(&mut self.a, &mut self.status, value)
                } else {
                    let value = self.fetch_u16(mmu);
                    adc_u16(&mut self.a, &mut self.status, value)
                }
            }

            Instruction::AddWithCarryAbsolute => {
                let addr = self.fetch_u16(mmu);

                if self.a_u8_mode() {
                    adc_u8(
                        &mut self.a,
                        &mut self.status,
                        mmu.read_u8(bank_addr(self.data_bank, addr)),
                    )
                } else {
                    adc_u16(
                        &mut self.a,
                        &mut self.status,
                        mmu.read_u16(bank_addr(self.data_bank, addr)),
                    )
                }
            }

            Instruction::AddWithCarryDirectPage => {
                let addr = self.fetch_u8(mmu);

                if self.a_u8_mode() {
                    adc_u8(
                        &mut self.a,
                        &mut self.status,
                        mmu.read_u8(self.direct_page as u32 + addr as u32),
                    )
                } else {
                    adc_u16(
                        &mut self.a,
                        &mut self.status,
                        mmu.read_u16(self.direct_page as u32 + addr as u32),
                    )
                }
            }

            Instruction::IncrementDirectPage => {
                let addr = self.fetch_u8(mmu);

                let offset_addr = self.direct_page as u32 + addr as u32;
                let value = mmu.read_u8(offset_addr).wrapping_add(1);

                mmu.store_u8(offset_addr, value);

                self.status.set(Flags::NEGATIVE, (value >> 7) & 1 == 1);
                self.status.set(Flags::ZERO, value == 0);
            }

            Instruction::IncrementX => {
                self.x = self.x.wrapping_add(1);

                // TODO: 8 bit mode
                self.status.set(Flags::NEGATIVE, (self.x >> 15) & 1 == 1);
                self.status.set(Flags::ZERO, self.x == 0);
            }

            Instruction::IncrementY => {
                self.y = self.y.wrapping_add(1);

                // TODO: 8 bit mode
                self.status.set(Flags::NEGATIVE, (self.y >> 15) & 1 == 1);
                self.status.set(Flags::ZERO, self.y == 0);
            }

            Instruction::DecrementX => {
                self.x = self.x.wrapping_sub(1);

                // TODO: 8 bit mode
                self.status.set(Flags::NEGATIVE, (self.x >> 15) & 1 == 1);
                self.status.set(Flags::ZERO, self.x == 0);
            }

            Instruction::DecrementY => {
                self.y = self.y.wrapping_sub(1);

                // TODO: 8 bit mode
                self.status.set(Flags::NEGATIVE, (self.y >> 15) & 1 == 1);
                self.status.set(Flags::ZERO, self.y == 0);
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
                if self.a_u8_mode() {
                    let value = self.fetch_u8(mmu);
                    compare_u8(&mut self.status, self.a as u8, value)
                } else {
                    let value = self.fetch_u16(mmu);
                    compare_u16(&mut self.status, self.a, value)
                }
            }

            Instruction::CompareAbsolute => {
                // TODO: 16 bit mode?
                let addr = self.fetch_u16(mmu);

                compare_u8(
                    &mut self.status,
                    self.a as u8,
                    mmu.read_u8(bank_addr(self.data_bank, addr)),
                )
            }

            Instruction::CompareDirectPage => {
                // TODO: 16 bit mode?
                let addr = self.fetch_u8(mmu);

                compare_u8(
                    &mut self.status,
                    self.a as u8,
                    mmu.read_u8(self.direct_page as u32 + addr as u32),
                )
            }

            Instruction::CompareAbsoluteLongIndexedX => {
                // TODO: 16 bit mode?
                let addr = self.fetch_long(mmu);

                compare_u8(
                    &mut self.status,
                    self.a as u8,
                    mmu.read_u8(addr + self.x as u32),
                )
            }

            Instruction::CompareXImmediate => {
                if self.xy_u8_mode() {
                    let value = self.fetch_u8(mmu);
                    compare_u8(&mut self.status, self.x as u8, value)
                } else {
                    let value = self.fetch_u16(mmu);
                    compare_u16(&mut self.status, self.x, value)
                }
            }

            Instruction::BranchCarryClear => {
                let offset = self.fetch_u8(mmu);

                branch(&mut self.pc, offset, !self.status.contains(Flags::CARRY))
            }

            Instruction::BranchCarrySet => {
                let offset = self.fetch_u8(mmu);

                branch(&mut self.pc, offset, self.status.contains(Flags::CARRY))
            }

            Instruction::BranchNotEqual => {
                let offset = self.fetch_u8(mmu);

                branch(&mut self.pc, offset, !self.status.contains(Flags::ZERO))
            }

            Instruction::BranchEqual => {
                let offset = self.fetch_u8(mmu);

                branch(&mut self.pc, offset, self.status.contains(Flags::ZERO))
            }

            Instruction::BranchAlways => {
                let offset = self.fetch_u8(mmu);
                branch(&mut self.pc, offset, true)
            }

            Instruction::PushA => {
                if self.a_u8_mode() {
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
                if self.a_u8_mode() {
                    let value = self.pull_u8(mmu);
                    load_u8(&mut self.a, &mut self.status, value);
                } else {
                    let value = self.pull_u16(mmu);
                    load_u16(&mut self.a, &mut self.status, value);
                }
            }

            Instruction::PullB => {
                // TODO: Can't use helper function here because target is a u8
                let value = self.pull_u8(mmu);

                self.data_bank = value;

                self.status.set(Flags::NEGATIVE, (value >> 7) & 1 == 1);
                self.status.set(Flags::ZERO, value == 0);
            }

            Instruction::PullD => {
                let value = self.pull_u16(mmu);
                load_u16(&mut self.direct_page, &mut self.status, value);
            }

            Instruction::PullX => {
                if self.xy_u8_mode() {
                    let value = self.pull_u8(mmu);
                    load_u8(&mut self.x, &mut self.status, value);
                } else {
                    let value = self.pull_u16(mmu);
                    load_u16(&mut self.x, &mut self.status, value);
                }
            }

            Instruction::PullY => {
                if self.xy_u8_mode() {
                    let value = self.pull_u8(mmu);
                    load_u8(&mut self.y, &mut self.status, value);
                } else {
                    let value = self.pull_u16(mmu);
                    load_u16(&mut self.y, &mut self.status, value);
                }
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
