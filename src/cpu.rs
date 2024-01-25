mod ops;

use std::fmt::Write;

use bitflags::bitflags;

use self::ops::*;
use crate::inst::{Immediate, Instruction};
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
    last_instruction: Instruction,
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
            last_instruction: Instruction::Unknown,
        }
    }

    pub fn current_addr(&self) -> u32 {
        bank_addr(self.program_bank, self.pc)
    }

    pub fn set_current_addr(&mut self, addr: u32) {
        self.program_bank = (addr >> 16) as u8;
        self.pc = (addr & 0x0000FFFF) as u16;
    }

    pub fn last_instruction(&self) -> Instruction {
        self.last_instruction
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

    fn fetch(&mut self, mmu: &Mmu) -> Instruction {
        let opcode = self.fetch_u8(mmu);

        match opcode {
            0x00 => Instruction::Break,
            0x08 => Instruction::PushStatus,
            0x0A => Instruction::ShiftLeft,
            0x0B => Instruction::PushD,
            0x18 => Instruction::ClearCarry,
            0x20 => Instruction::JumpSubRoutineAbsolute(self.fetch_u16(mmu)),

            0x22 => {
                let addr = self.fetch_u16(mmu);
                let bank = self.fetch_u8(mmu);

                Instruction::JumpSubRoutineAbsoluteLong(bank, addr)
            }

            0x28 => Instruction::PullStatus,
            0x2B => Instruction::PullD,
            0x4C => Instruction::JumpAbsolute(self.fetch_u16(mmu)),
            0x48 => Instruction::PushA,
            0x54 => Instruction::BlockMoveNext(self.fetch_u8(mmu), self.fetch_u8(mmu)),
            0x5A => Instruction::PushY,
            0x60 => Instruction::Return,
            0x64 => Instruction::StoreZeroDirectPage(self.fetch_u8(mmu)),
            0x65 => Instruction::AddWithCarryDirectPage(self.fetch_u8(mmu)),
            0x68 => Instruction::PullA,

            0x69 => {
                if self.emulation || self.status.contains(Flags::MEMORY_SELECT) {
                    Instruction::AddWithCarryImmediate(Immediate::U8(self.fetch_u8(mmu)))
                } else {
                    Instruction::AddWithCarryImmediate(Immediate::U16(self.fetch_u16(mmu)))
                }
            }

            0x6B => Instruction::ReturnLong,
            0x6D => Instruction::AddWithCarryAbsolute(self.fetch_u16(mmu)),
            0x74 => Instruction::StoreZeroDirectPageIndexedX(self.fetch_u8(mmu)),
            0x78 => Instruction::SetIrqDisable,
            0x7A => Instruction::PullY,
            0x7B => Instruction::MoveDA,
            0x80 => Instruction::BranchAlways(self.fetch_u8(mmu)),
            0x84 => Instruction::StoreYDirectPage(self.fetch_u8(mmu)),
            0x85 => Instruction::StoreADirectPage(self.fetch_u8(mmu)),
            0x86 => Instruction::StoreXDirectPage(self.fetch_u8(mmu)),
            0x88 => Instruction::DecrementY,
            0x8B => Instruction::PushB,
            0x8D => Instruction::StoreAAbsolute(self.fetch_u16(mmu)),
            0x8E => Instruction::StoreXAbsolute(self.fetch_u16(mmu)),
            0x90 => Instruction::BranchCarryClear(self.fetch_u8(mmu)),
            0x95 => Instruction::StoreADirectPageIndexedX(self.fetch_u8(mmu)),
            0x9A => Instruction::MoveXSP,
            0x9C => Instruction::StoreZeroAbsolute(self.fetch_u16(mmu)),
            0x9D => Instruction::StoreAAbsoluteIndexedX(self.fetch_u16(mmu)),
            0x9E => Instruction::StoreZeroAbsoluteIndexedX(self.fetch_u16(mmu)),
            0x9F => Instruction::StoreAAbsoluteLongIndexedX(self.fetch_long(mmu)),

            0xA0 => {
                if self.emulation || self.status.contains(Flags::INDEX_REGISTER) {
                    Instruction::LoadYImmediate(Immediate::U8(self.fetch_u8(mmu)))
                } else {
                    Instruction::LoadYImmediate(Immediate::U16(self.fetch_u16(mmu)))
                }
            }

            0xA2 => {
                if self.emulation || self.status.contains(Flags::INDEX_REGISTER) {
                    Instruction::LoadXImmediate(Immediate::U8(self.fetch_u8(mmu)))
                } else {
                    Instruction::LoadXImmediate(Immediate::U16(self.fetch_u16(mmu)))
                }
            }

            0xA4 => Instruction::LoadYDirectPage(self.fetch_u8(mmu)),
            0xA5 => Instruction::LoadADirectPage(self.fetch_u8(mmu)),
            0xA6 => Instruction::LoadXDirectPage(self.fetch_u8(mmu)),
            0xA8 => Instruction::MoveAY,
            0xA7 => Instruction::LoadADirectPageIndirectLong(self.fetch_u8(mmu)),

            0xA9 => {
                if self.emulation || self.status.contains(Flags::MEMORY_SELECT) {
                    Instruction::LoadAImmediate(Immediate::U8(self.fetch_u8(mmu)))
                } else {
                    Instruction::LoadAImmediate(Immediate::U16(self.fetch_u16(mmu)))
                }
            }

            0xAA => Instruction::MoveAX,
            0xAB => Instruction::PullB,
            0xAD => Instruction::LoadAAbsolute(self.fetch_u16(mmu)),
            0xB0 => Instruction::BranchCarrySet(self.fetch_u8(mmu)),
            0xBD => Instruction::LoadAAbsoluteIndexedX(self.fetch_u16(mmu)),
            0xBF => Instruction::LoadAAbsoluteLongIndexedX(self.fetch_long(mmu)),
            0xC2 => Instruction::ResetFlags(self.fetch_u8(mmu)),
            0xC5 => Instruction::CompareDirectPage(self.fetch_u8(mmu)),
            0xC8 => Instruction::IncrementY,

            0xC9 => {
                if self.emulation || self.status.contains(Flags::MEMORY_SELECT) {
                    Instruction::CompareImmediate(Immediate::U8(self.fetch_u8(mmu)))
                } else {
                    Instruction::CompareImmediate(Immediate::U16(self.fetch_u16(mmu)))
                }
            }

            0xCA => Instruction::DecrementX,
            0xCD => Instruction::CompareAbsolute(self.fetch_u16(mmu)),
            0xD0 => Instruction::BranchNotEqual(self.fetch_u8(mmu)),
            0xDA => Instruction::PushX,
            0xDF => Instruction::CompareAbsoluteLongIndexedX(self.fetch_long(mmu)),

            0xE0 => {
                if self.emulation || self.status.contains(Flags::INDEX_REGISTER) {
                    Instruction::CompareXImmediate(Immediate::U8(self.fetch_u8(mmu)))
                } else {
                    Instruction::CompareXImmediate(Immediate::U16(self.fetch_u16(mmu)))
                }
            }

            0xE2 => Instruction::SetFlags(self.fetch_u8(mmu)),
            0xE6 => Instruction::IncrementDirectPage(self.fetch_u8(mmu)),
            0xE8 => Instruction::IncrementX,
            0xEB => Instruction::ExchangeBA,
            0xF0 => Instruction::BranchEqual(self.fetch_u8(mmu)),
            0xF4 => Instruction::PushAbsolute(self.fetch_u16(mmu)),
            0xFA => Instruction::PullX,
            0xFB => Instruction::ExchangeCE,

            _ => Instruction::Unknown,
        }
    }

    pub fn tick(&mut self, mmu: &mut Mmu) {
        let inst = self.fetch(mmu);

        match inst {
            Instruction::Unknown { .. } => {}

            Instruction::LoadAImmediate(imm) => match imm {
                Immediate::U8(value) => load_u8(&mut self.a, &mut self.status, value),
                Immediate::U16(value) => load_u16(&mut self.a, &mut self.status, value),
            },

            Instruction::LoadAAbsolute(addr) => {
                // TODO: 16 bit mode
                load_u8(
                    &mut self.a,
                    &mut self.status,
                    mmu.read_u8(bank_addr(self.data_bank, addr)),
                )
            }

            Instruction::LoadADirectPage(addr) => {
                // TODO: 16 bit mode
                load_u8(
                    &mut self.a,
                    &mut self.status,
                    mmu.read_u8(self.direct_page as u32 + addr as u32),
                )
            }

            Instruction::LoadADirectPageIndirectLong(addr) => {
                let ptr = mmu.read_long(self.direct_page as u32 + addr as u32);

                // TODO: 16 bit mode
                load_u8(&mut self.a, &mut self.status, mmu.read_u8(ptr))
            }

            Instruction::LoadAAbsoluteIndexedX(addr) => {
                // TODO: 16 bit mode
                load_u8(
                    &mut self.a,
                    &mut self.status,
                    mmu.read_u8(bank_addr(self.data_bank, addr) + self.x as u32),
                )
            }

            Instruction::LoadAAbsoluteLongIndexedX(addr) => {
                // TODO: 16 bit mode

                load_u8(
                    &mut self.a,
                    &mut self.status,
                    mmu.read_u8(addr + self.x as u32),
                )
            }

            Instruction::LoadXImmediate(imm) => match imm {
                Immediate::U8(value) => load_u8(&mut self.x, &mut self.status, value),
                Immediate::U16(value) => load_u16(&mut self.x, &mut self.status, value),
            },

            Instruction::LoadXDirectPage(addr) => {
                // TODO: 8 bit mode?
                load_u16(
                    &mut self.x,
                    &mut self.status,
                    mmu.read_u16(self.direct_page as u32 + addr as u32),
                )
            }

            Instruction::LoadYImmediate(imm) => match imm {
                Immediate::U8(value) => load_u8(&mut self.y, &mut self.status, value),
                Immediate::U16(value) => load_u16(&mut self.y, &mut self.status, value),
            },

            Instruction::LoadYDirectPage(addr) => {
                // TODO: 8 bit mode?
                load_u16(
                    &mut self.y,
                    &mut self.status,
                    mmu.read_u16(self.direct_page as u32 + addr as u32),
                )
            }

            Instruction::StoreAAbsolute(addr) => {
                // TODO: 8 bit mode?
                mmu.store_u16(bank_addr(self.data_bank, addr), self.a);
            }

            Instruction::StoreADirectPage(addr) => {
                // TODO: 8 bit mode?
                mmu.store_u16(self.direct_page as u32 + addr as u32, self.a);
            }

            Instruction::StoreAAbsoluteIndexedX(addr) => {
                // TODO: 8 bit mode?
                mmu.store_u16(bank_addr(self.data_bank, addr) + self.x as u32, self.a);
            }

            Instruction::StoreAAbsoluteLongIndexedX(addr) => {
                // TODO: 8 bit mode?
                mmu.store_u16(addr + self.x as u32, self.a);
            }

            Instruction::StoreADirectPageIndexedX(addr) => {
                // TODO: 8 bit mode?
                mmu.store_u16(
                    self.direct_page as u32 + addr as u32 + self.x as u32,
                    self.a,
                );
            }

            Instruction::StoreXAbsolute(addr) => {
                // TODO: 8 bit mode?
                mmu.store_u16(bank_addr(self.data_bank, addr), self.x);
            }

            Instruction::StoreXDirectPage(addr) => {
                // TODO: 8 bit mode?
                mmu.store_u16(self.direct_page as u32 + addr as u32, self.x);
            }

            Instruction::StoreYDirectPage(addr) => {
                // TODO: 8 bit mode?
                mmu.store_u16(self.direct_page as u32 + addr as u32, self.y);
            }

            Instruction::StoreZeroAbsolute(addr) => {
                mmu.store_u8(bank_addr(self.data_bank, addr), 0);
            }

            Instruction::StoreZeroDirectPage(addr) => {
                mmu.store_u8(self.direct_page as u32 + addr as u32, 0);
            }

            Instruction::StoreZeroAbsoluteIndexedX(addr) => {
                mmu.store_u8(bank_addr(self.data_bank, addr) + self.x as u32, 0);
            }

            Instruction::StoreZeroDirectPageIndexedX(addr) => {
                mmu.store_u8(self.direct_page as u32 + addr as u32 + self.x as u32, 0);
            }

            Instruction::AddWithCarryImmediate(imm) => match imm {
                Immediate::U8(value) => adc_u8(&mut self.a, &mut self.status, value),
                Immediate::U16(value) => adc_u16(&mut self.a, &mut self.status, value),
            },

            Instruction::AddWithCarryAbsolute(addr) => {
                if self.emulation || self.status.contains(Flags::MEMORY_SELECT) {
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

            Instruction::AddWithCarryDirectPage(addr) => {
                if self.emulation || self.status.contains(Flags::MEMORY_SELECT) {
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

            Instruction::IncrementDirectPage(addr) => {
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

            Instruction::BlockMoveNext(dest, src) => {
                // TODO: 8 bit index registers - tbh I'm not sure about this one

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

            Instruction::CompareImmediate(imm) => match imm {
                Immediate::U8(value) => compare_u8(&mut self.status, self.a as u8, value),
                Immediate::U16(value) => compare_u16(&mut self.status, self.a, value),
            },

            Instruction::CompareAbsolute(addr) => {
                // TODO: 16 bit mode?
                compare_u8(
                    &mut self.status,
                    self.a as u8,
                    mmu.read_u8(bank_addr(self.data_bank, addr)),
                )
            }

            Instruction::CompareDirectPage(addr) => {
                // TODO: 16 bit mode?
                compare_u8(
                    &mut self.status,
                    self.a as u8,
                    mmu.read_u8(self.direct_page as u32 + addr as u32),
                )
            }

            Instruction::CompareAbsoluteLongIndexedX(addr) => {
                // TODO: 16 bit mode?
                compare_u8(
                    &mut self.status,
                    self.a as u8,
                    mmu.read_u8(addr + self.x as u32),
                )
            }

            Instruction::CompareXImmediate(imm) => match imm {
                Immediate::U8(value) => compare_u8(&mut self.status, self.x as u8, value),
                Immediate::U16(value) => compare_u16(&mut self.status, self.x, value),
            },

            Instruction::BranchCarryClear(offset) => {
                branch(&mut self.pc, offset, !self.status.contains(Flags::CARRY))
            }

            Instruction::BranchCarrySet(offset) => {
                branch(&mut self.pc, offset, self.status.contains(Flags::CARRY))
            }

            Instruction::BranchNotEqual(offset) => {
                branch(&mut self.pc, offset, !self.status.contains(Flags::ZERO))
            }

            Instruction::BranchEqual(offset) => {
                branch(&mut self.pc, offset, self.status.contains(Flags::ZERO))
            }

            Instruction::BranchAlways(offset) => branch(&mut self.pc, offset, true),

            Instruction::PushA => {
                if self.emulation || self.status.contains(Flags::MEMORY_SELECT) {
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
                if self.emulation || self.status.contains(Flags::INDEX_REGISTER) {
                    self.push_u8(mmu, self.x as u8);
                } else {
                    self.push_u16(mmu, self.x);
                }
            }

            Instruction::PushY => {
                if self.emulation || self.status.contains(Flags::INDEX_REGISTER) {
                    self.push_u8(mmu, self.y as u8);
                } else {
                    self.push_u16(mmu, self.y);
                }
            }

            Instruction::PushStatus => {
                self.push_u8(mmu, self.status.bits());
            }

            Instruction::PushAbsolute(addr) => {
                self.push_u16(mmu, addr);
            }

            Instruction::PullA => {
                if self.emulation || self.status.contains(Flags::MEMORY_SELECT) {
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
                if self.emulation || self.status.contains(Flags::INDEX_REGISTER) {
                    let value = self.pull_u8(mmu);
                    load_u8(&mut self.x, &mut self.status, value);
                } else {
                    let value = self.pull_u16(mmu);
                    load_u16(&mut self.x, &mut self.status, value);
                }
            }

            Instruction::PullY => {
                if self.emulation || self.status.contains(Flags::INDEX_REGISTER) {
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

            Instruction::JumpAbsolute(addr) => {
                self.pc = addr;
            }

            Instruction::JumpSubRoutineAbsolute(addr) => {
                self.push_u16(mmu, self.pc - 1); // TODO: bytes are reversed

                self.pc = addr;
            }

            Instruction::JumpSubRoutineAbsoluteLong(bank, addr) => {
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

            Instruction::ResetFlags(mask) => {
                self.status &= !Flags::from_bits_truncate(mask);
            }

            Instruction::SetFlags(mask) => {
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

        self.last_instruction = inst;
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
