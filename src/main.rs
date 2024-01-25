mod cpu;
mod inst;
mod mmu;

use std::fmt::Write;

use self::cpu::Cpu;
use self::inst::Instruction;
use self::mmu::Mmu;

fn main() {
    let rom = std::fs::read("ff2.sfc").unwrap();

    let mut mmu = Mmu::new(rom);
    let mut cpu = Cpu::new();
    cpu.set_current_addr(mmu.reset_vector() as u32);

    let mut output = String::new();

    loop {
        let current_addr = cpu.current_addr();
        let opcode = mmu.read_u8(cpu.current_addr());

        cpu.tick(&mut mmu);

        let _ = writeln!(
            output,
            "[{:>06X}] {:02X} {}\n         {}\n         Stack: [{}]",
            current_addr,
            opcode,
            cpu.last_instruction().asm(),
            cpu.register_debug(),
            cpu.stack_debug(&mmu)
        );

        if let Instruction::Unknown = cpu.last_instruction() {
            break;
        }
    }

    let _ = std::fs::write("output.log", &output);
}
