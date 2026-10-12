#![no_std]

pub mod asm;
pub mod bridge;
pub mod decompress;
pub mod int;

use decompress::{ChosenDecompressor, Decompressor};
use int::Interpreter;
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

#[inline(always)]
fn is_instr_in_vp(int: &Interpreter, addr: u32) -> bool {
    return (addr & 3) == 0 && int.g_vpbase <= addr && addr - int.g_vpbase < int.g_vpsize;
}

#[inline(always)]
fn is_addr_in_vp(int: &Interpreter, addr: u32) -> bool {
    return int.g_vpbase <= addr && addr - int.g_vpbase < int.g_vpsize;
}

#[unsafe(no_mangle)]
pub extern "C" fn __sseocb_i_main() {
    let mut int = Interpreter::new();

    loop {
        let word = ChosenDecompressor::decompress_u32(&int, int.vPC);
        let Some(instr) = parse(word) else {
            asm::emit_ebreak();
            loop {}
        };
        label!(match);
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
            Instruction::jal { rd, imm } => {
                label!(jal);
                let link_addr = int.g_vpbase.wrapping_add(int.vPC).wrapping_add(4);
                bridge::write_guest_reg(rd, link_addr);

                let target_pc = int.g_vpbase.wrapping_add(int.vPC).wrapping_add_signed(imm);
                if is_instr_in_vp(&int, target_pc) {
                    int.vPC = target_pc.wrapping_sub(int.g_vpbase);
                } else {
                    // This must never happen. vP must be isolated from all other code, so no
                    // target withing 1MiB should ever be found.
                    asm::emit_ebreak();
                    loop {}
                }
            }
            Instruction::jalr { rd, rs1, imm } => {
                label!(jalr);
                let rs1_addr = bridge::read_guest_reg(rs1);
                let link_addr = int.g_vpbase.wrapping_add(int.vPC).wrapping_add(4);
                bridge::write_guest_reg(rd, link_addr);

                let target_pc = rs1_addr.wrapping_add_signed(imm) & (!1);
                if is_instr_in_vp(&int, target_pc) {
                    int.vPC = target_pc.wrapping_sub(int.g_vpbase);
                } else {
                    bridge::native_call(target_pc);
                    int.vPC = int.vPC.wrapping_add(4);
                }
            }
            Instruction::beq { rs1, rs2, imm }
            | Instruction::bne { rs1, rs2, imm }
            | Instruction::blt { rs1, rs2, imm }
            | Instruction::bge { rs1, rs2, imm }
            | Instruction::bltu { rs1, rs2, imm }
            | Instruction::bgeu { rs1, rs2, imm } => {
                label!(branch);
                let r1 = bridge::read_guest_reg(rs1);
                let r2 = bridge::read_guest_reg(rs2);
                let cond = match instr {
                    Instruction::beq { .. } => r1 == r2,
                    Instruction::bne { .. } => r1 != r2,
                    Instruction::blt { .. } => (r1 as i32) < (r2 as i32),
                    Instruction::bge { .. } => (r1 as i32) >= (r2 as i32),
                    Instruction::bltu { .. } => r1 < r2,
                    Instruction::bgeu { .. } => r1 >= r2,
                    _ => unreachable!(),
                };

                if cond {
                    let addr = int.g_vpbase.wrapping_add(int.vPC).wrapping_add_signed(imm);
                    if is_instr_in_vp(&int, addr) {
                        int.vPC = addr.wrapping_sub(int.g_vpbase);
                    } else {
                        // This must never happen. vP must be isolated from all other code, so no
                        // target within 4KiB should ever be found.
                        asm::emit_ebreak();
                        loop {}
                    }
                } else {
                    int.vPC = int.vPC.wrapping_add(4);
                }
            }
            Instruction::lb { rd, rs1, imm }
            | Instruction::lh { rd, rs1, imm }
            | Instruction::lw { rd, rs1, imm }
            | Instruction::lbu { rd, rs1, imm }
            | Instruction::lhu { rd, rs1, imm } => {
                label!(load);
                let addr = bridge::read_guest_reg(rs1).wrapping_add_signed(imm) as usize;
                if is_addr_in_vp(&int, addr as u32) {
                    // This must never happen. TODO: disable such checks with a feature.
                    asm::emit_ebreak();
                    loop {}
                }

                let value = unsafe {
                    match instr {
                        Instruction::lb { .. } => *(addr as *const i8) as u32,
                        Instruction::lh { .. } => *(addr as *const i16) as u32,
                        Instruction::lw { .. } => *(addr as *const u32),
                        Instruction::lbu { .. } => *(addr as *const u8) as u32,
                        Instruction::lhu { .. } => *(addr as *const u16) as u32,
                        _ => unreachable!(),
                    }
                };

                bridge::write_guest_reg(rd, value);
                int.vPC = int.vPC.wrapping_add(4);
            }
            _ => {
                asm::emit_ebreak();
                loop {}
            }
        }
    }
}
