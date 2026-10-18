#![no_std]

pub mod asm;
pub mod bridge;
pub mod decompress;
pub mod int;

use decompress::{ChosenDecompressor, Decompressor};
use rv32_isa::{funct3, funct7, imm_b, imm_i, imm_j, imm_s, imm_u, mask, opcode, system};

#[macro_export]
macro_rules! label {
    ($name:ident) => {
        #[allow(named_asm_labels)]
        unsafe {
            ::core::arch::asm!(
                concat!(".globl __sseocb_m_", stringify!($name)),
                concat!("__sseocb_m_", stringify!($name), ":"),
                options(nostack, preserves_flags)
            );
        }
    };
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    bridge::write_guest_reg(10, !0);
    label!(panic_ebreak);
    crash();
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

#[inline(always)]
fn crash() -> ! {
    asm::emit_ebreak();
    loop {}
}

#[allow(non_snake_case)]
#[unsafe(no_mangle)]
pub extern "C" fn __sseocb_i_main(mut vPC: u32) {
    loop {
        let raw = ChosenDecompressor::decompress_u32(vPC);

        let op = rv32_isa::opcode(raw);
        let f3 = rv32_isa::funct3(raw);
        let f7 = rv32_isa::funct7(raw);
        let rd = rv32_isa::rd(raw);
        let rs1 = rv32_isa::rs1(raw);
        let rs2 = rv32_isa::rs2(raw);
        let shamt = rv32_isa::shamt(raw);

        label!(match);
        match op {
            opcode::LUI => {
                label!(lui);
                let imm = imm_u(raw);

                bridge::write_guest_reg(rd, imm as u32);
                vPC = vPC.wrapping_add(4);
            }
            opcode::AUIPC => {
                label!(auipc);
                let imm = imm_u(raw);

                let guest_pc = int::g_vpbase().wrapping_add(vPC);
                bridge::write_guest_reg(rd, guest_pc.wrapping_add_signed(imm));
                vPC = vPC.wrapping_add(4);
            }
            opcode::JAL => {
                label!(jal);
                let imm = imm_j(raw);

                let link_addr = int::g_vpbase().wrapping_add(vPC).wrapping_add(4);
                bridge::write_guest_reg(rd, link_addr);

                let target_pc = int::g_vpbase().wrapping_add(vPC).wrapping_add_signed(imm);
                if is_instr_in_vp(target_pc) {
                    vPC = target_pc.wrapping_sub(int::g_vpbase());
                } else {
                    // This must never happen. vP must be isolated from all other code, so no
                    // target withing 1MiB should ever be found.
                    label!(jal_ebreak);
                    crash();
                }
            }
            opcode::JALR => {
                label!(jalr);
                if f3 != funct3::JALR {
                    label!(jalr_ebreak);
                    crash();
                }
                let imm = imm_i(raw);

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
                    if bridge::is_trampoline(target_pc) {
                        vPC = bridge::get_trampoline_vp_offset(target_pc);
                    } else {
                        bridge::native_call!(target_pc);
                        vPC = vPC.wrapping_add(4);
                    }
                }
            }
            // Branches
            opcode::BRANCH => {
                label!(branch);
                let imm = imm_b(raw);

                let r1 = bridge::read_guest_reg(rs1);
                let r2 = bridge::read_guest_reg(rs2);
                let cond = match f3 {
                    funct3::BEQ => r1 == r2,
                    funct3::BNE => r1 != r2,
                    funct3::BLT => (r1 as i32) < (r2 as i32),
                    funct3::BGE => (r1 as i32) >= (r2 as i32),
                    funct3::BLTU => r1 < r2,
                    funct3::BGEU => r1 >= r2,
                    _ => crash(),
                };
                if cond {
                    let addr = int::g_vpbase().wrapping_add(vPC).wrapping_add_signed(imm);
                    if is_instr_in_vp(addr) {
                        vPC = addr.wrapping_sub(int::g_vpbase());
                    } else {
                        // This must never happen. vP must be isolated from all other code, so no
                        // target within 4KiB should ever be found.
                        label!(branch_ebreak);
                        crash();
                    }
                } else {
                    vPC = vPC.wrapping_add(4);
                }
            }
            opcode::LOAD => {
                label!(load);
                let imm = imm_i(raw);
                let addr = bridge::read_guest_reg(rs1).wrapping_add_signed(imm) as usize;
                if is_addr_in_vp(addr as u32) {
                    // This must never happen. TODO: disable such checks with a feature.
                    label!(load_ebreak);
                    crash();
                }

                let value = unsafe {
                    match f3 {
                        funct3::LB => *(addr as *const i8) as u32,
                        funct3::LH => *(addr as *const i16) as u32,
                        funct3::LW => *(addr as *const u32),
                        funct3::LBU => *(addr as *const u8) as u32,
                        funct3::LHU => *(addr as *const u16) as u32,
                        _ => crash(),
                    }
                };

                bridge::write_guest_reg(rd, value);
                vPC = vPC.wrapping_add(4);
            }
            opcode::STORE => {
                label!(store);
                let imm = imm_s(raw);
                let addr = bridge::read_guest_reg(rs1).wrapping_add_signed(imm) as usize;
                if is_addr_in_vp(addr as u32) {
                    // This must never happen. TODO: disable such checks with a feature.
                    label!(store_ebreak);
                    crash();
                }

                let value = bridge::read_guest_reg(rs2);
                unsafe {
                    match f3 {
                        funct3::SB => *(addr as *mut u8) = value as u8,
                        funct3::SH => *(addr as *mut u16) = value as u16,
                        funct3::SW => *(addr as *mut u32) = value,
                        _ => crash(),
                    }
                }

                vPC = vPC.wrapping_add(4);
            }
            opcode::OP_IMM => {
                let rs1_val = bridge::read_guest_reg(rs1);
                let imm = imm_i(raw);
                let value = match f3 {
                    funct3::ADDI
                    | funct3::SLTI
                    | funct3::SLTIU
                    | funct3::XORI
                    | funct3::ORI
                    | funct3::ANDI => {
                        label!(imm1);
                        match f3 {
                            funct3::ADDI => rs1_val.wrapping_add_signed(imm),
                            funct3::SLTI => ((rs1_val as i32) < imm) as u32,
                            funct3::SLTIU => (rs1_val < (imm as u32)) as u32,
                            funct3::XORI => rs1_val ^ (imm as u32),
                            funct3::ORI => rs1_val | (imm as u32),
                            funct3::ANDI => rs1_val & (imm as u32),
                            _ => unreachable!(),
                        }
                    }
                    funct3::SLLI | funct3::SRLI_SRAI => {
                        label!(imm2);
                        match (f3, f7) {
                            (funct3::SLLI, funct7::BASE) => rs1_val << shamt,
                            (funct3::SRLI_SRAI, funct7::BASE) => rs1_val >> shamt,
                            (funct3::SRLI_SRAI, funct7::ALT) => ((rs1_val as i32) >> shamt) as u32,
                            _ => crash(),
                        }
                    }
                    _ => crash(),
                };
                bridge::write_guest_reg(rd, value);
                vPC = vPC.wrapping_add(4);
            }
            opcode::OP => {
                label!(reg);
                let r1 = bridge::read_guest_reg(rs1);
                let r2 = bridge::read_guest_reg(rs2);
                let shamt_r = r2 & mask::SHAMT;
                let value = match (f3, f7) {
                    (funct3::ADD_SUB, funct7::BASE) => r1.wrapping_add(r2),
                    (funct3::ADD_SUB, funct7::ALT) => r1.wrapping_sub(r2),
                    (funct3::SLL, funct7::BASE) => r1 << shamt_r,
                    (funct3::SLT, funct7::BASE) => ((r1 as i32) < (r2 as i32)) as u32,
                    (funct3::SLTU, funct7::BASE) => (r1 < r2) as u32,
                    (funct3::XOR, funct7::BASE) => r1 ^ r2,
                    (funct3::SRL_SRA, funct7::BASE) => r1 >> shamt_r,
                    (funct3::SRL_SRA, funct7::ALT) => ((r1 as i32) >> shamt_r) as u32,
                    (funct3::OR, funct7::BASE) => r1 | r2,
                    (funct3::AND, funct7::BASE) => r1 & r2,
                    _ => crash(),
                };
                bridge::write_guest_reg(rd, value);
                vPC = vPC.wrapping_add(4);
            }
            opcode::FENCE => {
                if f3 == funct3::FENCE && rd == 0 && rs1 == 0 {
                    vPC = vPC.wrapping_add(4);
                } else {
                    crash();
                }
            }
            opcode::SYSTEM if raw == system::EBREAK => {
                label!(ebreak_ebreak);
                asm::emit_ebreak();
                vPC = vPC.wrapping_add(4);
            }
            _ => {
                label!(other_ebreak);
                crash();
            }
        }
    }
}
