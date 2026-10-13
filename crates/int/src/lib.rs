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

#[inline(always)]
fn is_instr_in_vp(addr: u32) -> bool {
    let g_vpbase = int::g_vpbase();
    let g_vpsize = int::g_vpsize();
    return (addr & 3) == 0 && g_vpbase <= addr && addr - g_vpbase < g_vpsize;
}

#[inline(always)]
fn is_addr_in_vp(addr: u32) -> bool {
    let g_vpbase = int::g_vpbase();
    let g_vpsize = int::g_vpsize();
    return g_vpbase <= addr && addr - g_vpbase < g_vpsize;
}

#[allow(non_snake_case)]
#[unsafe(no_mangle)]
pub extern "C" fn __sseocb_i_main(mut vPC: u32) {
    loop {
        let word = ChosenDecompressor::decompress_u32(vPC);
        let Some(instr) = parse(word) else {
            asm::emit_ebreak();
            loop {}
        };
        match instr {
            Instruction::lui { rd, imm } => {
                bridge::write_guest_reg(rd, imm as u32);
                vPC = vPC.wrapping_add(4);
            }
            Instruction::auipc { rd, imm } => {
                let guest_pc = int::g_vpbase().wrapping_add(vPC);
                bridge::write_guest_reg(rd, guest_pc.wrapping_add_signed(imm));
                vPC = vPC.wrapping_add(4);
            }
            Instruction::jal { rd, imm } => {
                let link_addr = int::g_vpbase().wrapping_add(vPC).wrapping_add(4);
                bridge::write_guest_reg(rd, link_addr);

                let target_pc = int::g_vpbase().wrapping_add(vPC).wrapping_add_signed(imm);
                if is_instr_in_vp(target_pc) {
                    vPC = target_pc.wrapping_sub(int::g_vpbase());
                } else {
                    // This must never happen. vP must be isolated from all other code, so no
                    // target withing 1MiB should ever be found.
                    asm::emit_ebreak();
                    loop {}
                }
            }
            Instruction::jalr { rd, rs1, imm } => {
                let rs1_addr = bridge::read_guest_reg(rs1);
                let target_pc = rs1_addr.wrapping_add_signed(imm) & (!1);

                if target_pc == bridge::plt_return_addr() {
                    // plt_call -> returns to plt_return
                    return;
                }

                let link_addr = int::g_vpbase().wrapping_add(vPC).wrapping_add(4);
                bridge::write_guest_reg(rd, link_addr);

                if is_instr_in_vp(target_pc) {
                    vPC = target_pc.wrapping_sub(int::g_vpbase());
                } else {
                    bridge::native_call!(target_pc);
                    vPC = vPC.wrapping_add(4);
                }
            }
            Instruction::beq { rs1, rs2, imm }
            | Instruction::bne { rs1, rs2, imm }
            | Instruction::blt { rs1, rs2, imm }
            | Instruction::bge { rs1, rs2, imm }
            | Instruction::bltu { rs1, rs2, imm }
            | Instruction::bgeu { rs1, rs2, imm } => {
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
                    let addr = int::g_vpbase().wrapping_add(vPC).wrapping_add_signed(imm);
                    if is_instr_in_vp(addr) {
                        vPC = addr.wrapping_sub(int::g_vpbase());
                    } else {
                        // This must never happen. vP must be isolated from all other code, so no
                        // target within 4KiB should ever be found.
                        asm::emit_ebreak();
                        loop {}
                    }
                } else {
                    vPC = vPC.wrapping_add(4);
                }
            }
            Instruction::lb { rd, rs1, imm }
            | Instruction::lh { rd, rs1, imm }
            | Instruction::lw { rd, rs1, imm }
            | Instruction::lbu { rd, rs1, imm }
            | Instruction::lhu { rd, rs1, imm } => {
                let addr = bridge::read_guest_reg(rs1).wrapping_add_signed(imm) as usize;
                if is_addr_in_vp(addr as u32) {
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
                vPC = vPC.wrapping_add(4);
            }
            Instruction::sb { rs1, rs2, imm }
            | Instruction::sh { rs1, rs2, imm }
            | Instruction::sw { rs1, rs2, imm } => {
                let addr = bridge::read_guest_reg(rs1).wrapping_add_signed(imm) as usize;
                if is_addr_in_vp(addr as u32) {
                    // This must never happen. TODO: disable such checks with a feature.
                    asm::emit_ebreak();
                    loop {}
                }

                let value = bridge::read_guest_reg(rs2);
                unsafe {
                    match instr {
                        Instruction::sb { .. } => *(addr as *mut u8) = value as u8,
                        Instruction::sh { .. } => *(addr as *mut u16) = value as u16,
                        Instruction::sw { .. } => *(addr as *mut u32) = value,
                        _ => unreachable!(),
                    }
                }

                vPC = vPC.wrapping_add(4);
            }
            Instruction::addi { rd, rs1, imm }
            | Instruction::slti { rd, rs1, imm }
            | Instruction::sltiu { rd, rs1, imm }
            | Instruction::xori { rd, rs1, imm }
            | Instruction::ori { rd, rs1, imm }
            | Instruction::andi { rd, rs1, imm } => {
                let rs1 = bridge::read_guest_reg(rs1);
                let value = match instr {
                    Instruction::addi { .. } => rs1.wrapping_add_signed(imm),
                    Instruction::slti { .. } => ((rs1 as i32) < imm) as u32,
                    Instruction::sltiu { .. } => (rs1 < (imm as u32)) as u32,
                    Instruction::xori { .. } => rs1 ^ (imm as u32),
                    Instruction::ori { .. } => rs1 | (imm as u32),
                    Instruction::andi { .. } => rs1 & (imm as u32),
                    _ => unreachable!(),
                };
                bridge::write_guest_reg(rd, value);
                vPC = vPC.wrapping_add(4);
            }
            Instruction::slli { rd, rs1, shamt }
            | Instruction::srli { rd, rs1, shamt }
            | Instruction::srai { rd, rs1, shamt } => {
                let rs1 = bridge::read_guest_reg(rs1);
                let value = match instr {
                    Instruction::slli { .. } => rs1 << shamt,
                    Instruction::srli { .. } => rs1 >> shamt,
                    Instruction::srai { .. } => ((rs1 as i32) >> shamt) as u32,
                    _ => unreachable!(),
                };
                bridge::write_guest_reg(rd, value);
                vPC = vPC.wrapping_add(4);
            }
            Instruction::add { rd, rs1, rs2 }
            | Instruction::sub { rd, rs1, rs2 }
            | Instruction::sll { rd, rs1, rs2 }
            | Instruction::slt { rd, rs1, rs2 }
            | Instruction::sltu { rd, rs1, rs2 }
            | Instruction::xor { rd, rs1, rs2 }
            | Instruction::srl { rd, rs1, rs2 }
            | Instruction::sra { rd, rs1, rs2 }
            | Instruction::or { rd, rs1, rs2 }
            | Instruction::and { rd, rs1, rs2 } => {
                let rs1 = bridge::read_guest_reg(rs1);
                let rs2 = bridge::read_guest_reg(rs2);
                let shamt = rs2 & 0x1f;
                let value = match instr {
                    Instruction::add { .. } => rs1.wrapping_add(rs2),
                    Instruction::sub { .. } => rs1.wrapping_sub(rs2),
                    Instruction::sll { .. } => rs1 << shamt,
                    Instruction::slt { .. } => ((rs1 as i32) < (rs2 as i32)) as u32,
                    Instruction::sltu { .. } => (rs1 < rs2) as u32,
                    Instruction::xor { .. } => rs1 ^ rs2,
                    Instruction::srl { .. } => rs1 >> shamt,
                    Instruction::sra { .. } => ((rs1 as i32) >> shamt) as u32,
                    Instruction::or { .. } => rs1 | rs2,
                    Instruction::and { .. } => rs1 & rs2,
                    _ => unreachable!(),
                };
                bridge::write_guest_reg(rd, value);
                vPC = vPC.wrapping_add(4);
            }
            Instruction::fence => {
                vPC = vPC.wrapping_add(4);
            }
            Instruction::ebreak => {
                asm::emit_ebreak();
                vPC = vPC.wrapping_add(4);
            }
            _ => {
                asm::emit_ebreak();
                loop {}
            }
        }
    }
}
