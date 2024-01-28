mod cpu;
mod inst;
mod mmu;

use std::collections::VecDeque;
use std::fmt::Write;

use self::cpu::Cpu;
use self::inst::Instruction;
use self::mmu::Mmu;

fn main() {
    let rom = std::fs::read("ff2.sfc").unwrap();

    let mut mmu = Mmu::new(rom);
    let mut cpu = Cpu::new();
    cpu.set_current_addr(mmu.reset_vector() as u32);

    let mut snapshots = VecDeque::new();

    loop {
        if snapshots.len() >= 200 {
            snapshots.pop_front();
        }

        snapshots.push_back(cpu.clone());

        let opcode = mmu.read_u8(cpu.current_addr());
        let inst = Instruction::from_opcode(opcode);

        cpu.tick(&mut mmu);

        if let Instruction::Unknown = inst {
            break;
        }
    }

    let mut output = String::new();

    for snapshot in snapshots {
        let current_addr = snapshot.current_addr();
        let opcode = mmu.read_u8(current_addr);
        let inst = Instruction::from_opcode(opcode);

        let _ = writeln!(
            output,
            "[{:>06X}] {:02X} {:?}\n         {}\n         Stack: [{}]",
            current_addr,
            opcode,
            inst,
            snapshot.register_debug(),
            snapshot.stack_debug(&mmu) // TODO: This isn't accurate for snapshots
        );
    }

    let _ = std::fs::write("output.log", &output);
}
