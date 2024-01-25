#[derive(Debug, Clone, Copy)]
pub enum Immediate {
    U8(u8),
    U16(u16),
}

#[derive(Debug, Clone, Copy)]
pub enum Instruction {
    Unknown,

    // Load from memory
    LoadAImmediate(Immediate),
    LoadAAbsolute(u16),
    LoadADirectPage(u8),
    LoadADirectPageIndirectLong(u8),
    LoadAAbsoluteIndexedX(u16),
    LoadAAbsoluteLongIndexedX(u32),
    LoadXImmediate(Immediate),
    LoadXDirectPage(u8),
    LoadYImmediate(Immediate),
    LoadYDirectPage(u8),

    // Store to memory
    StoreAAbsolute(u16),
    StoreADirectPage(u8),
    StoreAAbsoluteIndexedX(u16),
    StoreAAbsoluteLongIndexedX(u32),
    StoreADirectPageIndexedX(u8),
    StoreXAbsolute(u16),
    StoreXDirectPage(u8),
    StoreYDirectPage(u8),
    StoreZeroAbsolute(u16),
    StoreZeroDirectPage(u8),
    StoreZeroAbsoluteIndexedX(u16),
    StoreZeroDirectPageIndexedX(u8),

    // Arithmatic
    AddWithCarryImmediate(Immediate),
    AddWithCarryAbsolute(u16),
    AddWithCarryDirectPage(u8),
    IncrementDirectPage(u8),
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
    ExchangeBA,

    // Block moves
    BlockMoveNext(u8, u8),

    // Logic
    CompareImmediate(Immediate),
    CompareAbsolute(u16),
    CompareDirectPage(u8),
    CompareAbsoluteLongIndexedX(u32),
    CompareXImmediate(Immediate),

    // Branching
    BranchCarryClear(u8),
    BranchCarrySet(u8),
    BranchNotEqual(u8),
    BranchEqual(u8),
    BranchAlways(u8),

    // Push to stack
    PushA,
    PushB,
    PushD,
    PushX,
    PushY,
    PushStatus,
    PushAbsolute(u16),

    // Pull from stack
    PullA,
    PullB,
    PullD,
    PullX,
    PullY,
    PullStatus,

    // Jumps
    JumpAbsolute(u16),

    // Subroutines
    JumpSubRoutineAbsolute(u16),
    JumpSubRoutineAbsoluteLong(u8, u16),
    Return,
    ReturnLong,

    // Change status flags
    ClearCarry,
    SetIrqDisable,
    ResetFlags(u8),
    SetFlags(u8),
    ExchangeCE,

    // Interrupts
    Break,
}

impl Instruction {
    pub fn asm(&self) -> String {
        match self {
            Instruction::Unknown => "???".into(),

            Instruction::LoadAImmediate(imm) => immediate("LDA", imm),
            Instruction::LoadAAbsolute(addr) => absolute("LDA", addr),
            Instruction::LoadADirectPage(addr) => direct_page("LDA", addr),
            Instruction::LoadADirectPageIndirectLong(addr) => {
                direct_page_indirect_long("LDA", addr)
            }
            Instruction::LoadAAbsoluteIndexedX(addr) => absolute_indexed_x("LDA", addr),
            Instruction::LoadAAbsoluteLongIndexedX(addr) => absolute_long_indexed_x("LDA", addr),
            Instruction::LoadXImmediate(imm) => immediate("LDX", imm),
            Instruction::LoadXDirectPage(addr) => direct_page("LDX", addr),
            Instruction::LoadYImmediate(imm) => immediate("LDY", imm),
            Instruction::LoadYDirectPage(addr) => direct_page("LDY", addr),

            Instruction::StoreAAbsolute(addr) => absolute("STA", addr),
            Instruction::StoreADirectPage(addr) => direct_page("STA", addr),
            Instruction::StoreAAbsoluteIndexedX(addr) => absolute_indexed_x("STA", addr),
            Instruction::StoreAAbsoluteLongIndexedX(addr) => absolute_long_indexed_x("STA", addr),
            Instruction::StoreADirectPageIndexedX(addr) => direct_page_indexed_x("STA", addr),
            Instruction::StoreXAbsolute(addr) => absolute("STX", addr),
            Instruction::StoreXDirectPage(addr) => direct_page("STX", addr),
            Instruction::StoreYDirectPage(addr) => direct_page("STY", addr),
            Instruction::StoreZeroAbsolute(addr) => absolute("STZ", addr),
            Instruction::StoreZeroDirectPage(addr) => direct_page("STZ", addr),
            Instruction::StoreZeroAbsoluteIndexedX(addr) => absolute_indexed_x("STZ", addr),
            Instruction::StoreZeroDirectPageIndexedX(addr) => direct_page_indexed_x("STZ", addr),

            Instruction::AddWithCarryImmediate(imm) => immediate("ADC", imm),
            Instruction::AddWithCarryAbsolute(addr) => absolute("ADC", addr),
            Instruction::AddWithCarryDirectPage(addr) => direct_page("ADC", addr),
            Instruction::IncrementDirectPage(addr) => direct_page("INC", addr),
            Instruction::IncrementX => "INX".into(),
            Instruction::IncrementY => "INY".into(),
            Instruction::DecrementX => "DEX".into(),
            Instruction::DecrementY => "DEY".into(),

            Instruction::ShiftLeft => "ASL".into(),

            Instruction::MoveAX => "TAX".into(),
            Instruction::MoveAY => "TAY".into(),
            Instruction::MoveDA => "TDC".into(),
            Instruction::MoveXSP => "TXS".into(),
            Instruction::ExchangeBA => "XBA".into(),

            Instruction::BlockMoveNext(dest, src) => format!("MVN ${:02X},${:02X}", src, dest),

            Instruction::CompareImmediate(imm) => immediate("CMP", imm),
            Instruction::CompareAbsolute(addr) => absolute("CMP", addr),
            Instruction::CompareDirectPage(addr) => direct_page("CMP", addr),
            Instruction::CompareAbsoluteLongIndexedX(addr) => absolute_long_indexed_x("CMP", addr),
            Instruction::CompareXImmediate(imm) => immediate("CPX", imm),

            Instruction::BranchCarryClear(offset) => branch("BCC", offset),
            Instruction::BranchCarrySet(offset) => branch("BCS", offset),
            Instruction::BranchNotEqual(offset) => branch("BNE", offset),
            Instruction::BranchEqual(offset) => branch("BEQ", offset),
            Instruction::BranchAlways(offset) => branch("BRA", offset),

            Instruction::PushA => "PHA".into(),
            Instruction::PushB => "PHB".into(),
            Instruction::PushD => "PHD".into(),
            Instruction::PushX => "PHX".into(),
            Instruction::PushY => "PHY".into(),
            Instruction::PushStatus => "PHP".into(),
            Instruction::PushAbsolute(addr) => absolute("PEA", addr),
            Instruction::PullA => "PLA".into(),
            Instruction::PullB => "PLB".into(),
            Instruction::PullD => "PLD".into(),
            Instruction::PullX => "PLX".into(),
            Instruction::PullY => "PLY".into(),
            Instruction::PullStatus => "PLP".into(),

            Instruction::JumpAbsolute(addr) => absolute("JMP", addr),

            Instruction::JumpSubRoutineAbsolute(addr) => absolute("JSR", addr),
            Instruction::JumpSubRoutineAbsoluteLong(bank, addr) => absolute_long("JSL", bank, addr),
            Instruction::Return => "RTS".into(),
            Instruction::ReturnLong => "RTL".into(),

            Instruction::ClearCarry => "CLC".into(),
            Instruction::SetIrqDisable => "SEI".into(),
            Instruction::ResetFlags(mask) => format!("REP #${:02X}", mask),
            Instruction::SetFlags(mask) => format!("SEP #${:02X}", mask),
            Instruction::ExchangeCE => "XCE".into(),

            Instruction::Break => "BRK".into(),
        }
    }
}

fn immediate(opcode: &str, imm: &Immediate) -> String {
    match imm {
        Immediate::U8(v) => format!("{} #${:02X}", opcode, v),
        Immediate::U16(v) => format!("{} #${:04X}", opcode, v),
    }
}

fn absolute(opcode: &str, addr: &u16) -> String {
    format!("{} ${:04X}", opcode, addr)
}

fn absolute_long(opcode: &str, bank: &u8, addr: &u16) -> String {
    format!("{} ${:02X}{:04X}", opcode, bank, addr)
}

fn direct_page(opcode: &str, addr: &u8) -> String {
    format!("{} ${:02X}", opcode, addr)
}

fn direct_page_indexed_x(opcode: &str, addr: &u8) -> String {
    format!("{} ${:02X},X", opcode, addr)
}

fn direct_page_indirect_long(opcode: &str, addr: &u8) -> String {
    format!("{} [${:02X}]", opcode, addr)
}

fn absolute_indexed_x(opcode: &str, addr: &u16) -> String {
    format!("{} ${:04X},X", opcode, addr)
}

fn absolute_long_indexed_x(opcode: &str, addr: &u32) -> String {
    format!("{} ${:04X},X", opcode, addr)
}

fn branch(opcode: &str, offset: &u8) -> String {
    let sign_bit = offset >> 7;
    let value = if sign_bit == 1 { !offset + 1 } else { *offset };
    let sign = if sign_bit == 1 { "-" } else { "+" };

    format!("{} {}${:02X}", opcode, sign, value)
}
