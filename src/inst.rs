#[derive(Debug, Clone, Copy)]
pub enum Instruction {
    Unknown,

    // Load from memory
    LoadAImmediate,
    LoadAAbsolute,
    LoadADirectPage,
    LoadADirectPageIndirectLong,
    LoadAAbsoluteIndexedX,
    LoadAAbsoluteLongIndexedX,
    LoadAAbsoluteIndexedY,
    LoadXImmediate,
    LoadXDirectPage,
    LoadYImmediate,
    LoadYDirectPage,

    // Store to memory
    StoreAAbsolute,
    StoreADirectPage,
    StoreAAbsoluteIndexedX,
    StoreAAbsoluteLongIndexedX,
    StoreAAbsoluteIndexedY,
    StoreADirectPageIndexedX,
    StoreXAbsolute,
    StoreXDirectPage,
    StoreYDirectPage,
    StoreZeroAbsolute,
    StoreZeroDirectPage,
    StoreZeroAbsoluteIndexedX,
    StoreZeroDirectPageIndexedX,

    // Arithmatic
    AddWithCarryImmediate,
    AddWithCarryAbsolute,
    AddWithCarryDirectPage,
    AddWithCarryAbsoluteIndexedY,
    IncrementDirectPage,
    IncrementA,
    IncrementX,
    IncrementY,
    DecrementX,
    DecrementY,

    // Shifts
    ShiftLeft,

    // Transfer register to register
    MoveAX,
    MoveAY,
    MoveDA,
    MoveXSP,
    MoveYA,
    ExchangeBA,

    // Block moves
    BlockMoveNext,

    // Logic
    CompareImmediate,
    CompareAbsolute,
    CompareDirectPage,
    CompareAbsoluteLongIndexedX,
    CompareXImmediate,

    // Branching
    BranchCarryClear,
    BranchCarrySet,
    BranchNotEqual,
    BranchEqual,
    BranchAlways,

    // Push to stack
    PushA,
    PushB,
    PushD,
    PushX,
    PushY,
    PushStatus,
    PushAbsolute,

    // Pull from stack
    PullA,
    PullB,
    PullD,
    PullX,
    PullY,
    PullStatus,

    // Jumps
    JumpAbsolute,

    // Subroutines
    JumpSubRoutineAbsolute,
    JumpSubRoutineAbsoluteLong,
    Return,
    ReturnLong,

    // Change status flags
    ClearCarry,
    SetIrqDisable,
    ResetFlags,
    SetFlags,
    ExchangeCE,

    // Interrupts
    Break,
}

impl Instruction {
    pub fn from_opcode(opcode: u8) -> Instruction {
        match opcode {
            0x00 => Instruction::Break,
            0x08 => Instruction::PushStatus,
            0x0A => Instruction::ShiftLeft,
            0x0B => Instruction::PushD,
            0x18 => Instruction::ClearCarry,
            0x1A => Instruction::IncrementA,
            0x20 => Instruction::JumpSubRoutineAbsolute,
            0x22 => Instruction::JumpSubRoutineAbsoluteLong,
            0x28 => Instruction::PullStatus,
            0x2B => Instruction::PullD,
            0x4C => Instruction::JumpAbsolute,
            0x48 => Instruction::PushA,
            0x54 => Instruction::BlockMoveNext,
            0x5A => Instruction::PushY,
            0x60 => Instruction::Return,
            0x64 => Instruction::StoreZeroDirectPage,
            0x65 => Instruction::AddWithCarryDirectPage,
            0x68 => Instruction::PullA,
            0x69 => Instruction::AddWithCarryImmediate,
            0x6B => Instruction::ReturnLong,
            0x6D => Instruction::AddWithCarryAbsolute,
            0x74 => Instruction::StoreZeroDirectPageIndexedX,
            0x78 => Instruction::SetIrqDisable,
            0x79 => Instruction::AddWithCarryAbsoluteIndexedY,
            0x7A => Instruction::PullY,
            0x7B => Instruction::MoveDA,
            0x80 => Instruction::BranchAlways,
            0x84 => Instruction::StoreYDirectPage,
            0x85 => Instruction::StoreADirectPage,
            0x86 => Instruction::StoreXDirectPage,
            0x88 => Instruction::DecrementY,
            0x8B => Instruction::PushB,
            0x8D => Instruction::StoreAAbsolute,
            0x8E => Instruction::StoreXAbsolute,
            0x90 => Instruction::BranchCarryClear,
            0x95 => Instruction::StoreADirectPageIndexedX,
            0x98 => Instruction::MoveYA,
            0x99 => Instruction::StoreAAbsoluteIndexedY,
            0x9A => Instruction::MoveXSP,
            0x9C => Instruction::StoreZeroAbsolute,
            0x9D => Instruction::StoreAAbsoluteIndexedX,
            0x9E => Instruction::StoreZeroAbsoluteIndexedX,
            0x9F => Instruction::StoreAAbsoluteLongIndexedX,
            0xA0 => Instruction::LoadYImmediate,
            0xA2 => Instruction::LoadXImmediate,
            0xA4 => Instruction::LoadYDirectPage,
            0xA5 => Instruction::LoadADirectPage,
            0xA6 => Instruction::LoadXDirectPage,
            0xA8 => Instruction::MoveAY,
            0xA7 => Instruction::LoadADirectPageIndirectLong,
            0xA9 => Instruction::LoadAImmediate,
            0xAA => Instruction::MoveAX,
            0xAB => Instruction::PullB,
            0xAD => Instruction::LoadAAbsolute,
            0xB0 => Instruction::BranchCarrySet,
            0xB9 => Instruction::LoadAAbsoluteIndexedY,
            0xBD => Instruction::LoadAAbsoluteIndexedX,
            0xBF => Instruction::LoadAAbsoluteLongIndexedX,
            0xC2 => Instruction::ResetFlags,
            0xC5 => Instruction::CompareDirectPage,
            0xC8 => Instruction::IncrementY,
            0xC9 => Instruction::CompareImmediate,
            0xCA => Instruction::DecrementX,
            0xCD => Instruction::CompareAbsolute,
            0xD0 => Instruction::BranchNotEqual,
            0xDA => Instruction::PushX,
            0xDF => Instruction::CompareAbsoluteLongIndexedX,
            0xE0 => Instruction::CompareXImmediate,
            0xE2 => Instruction::SetFlags,
            0xE6 => Instruction::IncrementDirectPage,
            0xE8 => Instruction::IncrementX,
            0xEB => Instruction::ExchangeBA,
            0xF0 => Instruction::BranchEqual,
            0xF4 => Instruction::PushAbsolute,
            0xFA => Instruction::PullX,
            0xFB => Instruction::ExchangeCE,

            _ => Instruction::Unknown,
        }
    }
}
