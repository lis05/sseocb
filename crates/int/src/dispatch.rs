use rv32_isa::{Instruction, parse};
use crate::asm;
use crate::int;

// Executes an instruction.
pub fn dispatch(word: u32, int: int::Interpreter) {
    if let Some(instr) = prase(word) {
    }
    else {
        asm::emit_ebreak();
        loop {}
    }
}
