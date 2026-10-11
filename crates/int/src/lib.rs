#![no_std]

pub mod asm;
pub mod bridge;
pub mod decompress;
pub mod int;

use decompress::{ChosenDecompressor, Decompressor};
use rv32_isa::{Instruction, parse};

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    asm::emit_ebreak();
    loop {}
}

macro_rules! label {
    ($name:ident) => {
        #[allow(named_asm_labels)]
        unsafe {
            core::arch::asm!(
                concat!(".globl __sseocb_i_h_", stringify!($name)),
                concat!("__sseocb_i_h_", stringify!($name), ":"),
                options(nostack, preserves_flags)
            );
        }
    };
}

#[unsafe(no_mangle)]
pub extern "C" fn __sseocb_i_main() {
    let mut int = int::Interpreter::new();

    loop {
        let word = ChosenDecompressor::decompress_u32(&int, int.vPC);
        let Some(instr) = parse(word) else {
            asm::emit_ebreak();
            loop {}
        };
        match instr {
            Instruction::lui { rd, imm } => {
                label!(lui);
                bridge::write_guest_reg(rd, imm as u32);
                int.vPC = int.vPC.wrapping_add(4);
            }
            Instruction::auipc { rd, imm } => {
                label!(auipc);
                let guest_pc = int.g_vpbase.wrapping_add(int.vPC);
                bridge::write_guest_reg(rd, guest_pc.wrapping_add_signed(imm));
                int.vPC = int.vPC.wrapping_add(4);
            }
            _ => {
                asm::emit_ebreak();
                loop {}
            }
        }
    }
}
