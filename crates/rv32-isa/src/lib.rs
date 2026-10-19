#![no_std]
#[allow(non_camel_case_types)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Instruction {
    // Upper Immediate (U-type)
    lui { rd: usize, imm: i32 },
    auipc { rd: usize, imm: i32 },

    // Unconditional Jumps
    jal { rd: usize, imm: i32 },
    jalr { rd: usize, rs1: usize, imm: i32 },

    // Conditional Branches (B-type)
    beq { rs1: usize, rs2: usize, imm: i32 },
    bne { rs1: usize, rs2: usize, imm: i32 },
    blt { rs1: usize, rs2: usize, imm: i32 },
    bge { rs1: usize, rs2: usize, imm: i32 },
    bltu { rs1: usize, rs2: usize, imm: i32 },
    bgeu { rs1: usize, rs2: usize, imm: i32 },

    // Loads (I-type)
    lb { rd: usize, rs1: usize, imm: i32 },
    lh { rd: usize, rs1: usize, imm: i32 },
    lw { rd: usize, rs1: usize, imm: i32 },
    lbu { rd: usize, rs1: usize, imm: i32 },
    lhu { rd: usize, rs1: usize, imm: i32 },

    // Stores (S-type)
    sb { rs1: usize, rs2: usize, imm: i32 },
    sh { rs1: usize, rs2: usize, imm: i32 },
    sw { rs1: usize, rs2: usize, imm: i32 },

    // Immediate Arithmetic (I-type)
    addi { rd: usize, rs1: usize, imm: i32 },
    slti { rd: usize, rs1: usize, imm: i32 },
    sltiu { rd: usize, rs1: usize, imm: i32 },
    xori { rd: usize, rs1: usize, imm: i32 },
    ori { rd: usize, rs1: usize, imm: i32 },
    andi { rd: usize, rs1: usize, imm: i32 },
    slli { rd: usize, rs1: usize, shamt: u32 },
    srli { rd: usize, rs1: usize, shamt: u32 },
    srai { rd: usize, rs1: usize, shamt: u32 },

    // Register Arithmetic (R-type)
    add { rd: usize, rs1: usize, rs2: usize },
    sub { rd: usize, rs1: usize, rs2: usize },
    sll { rd: usize, rs1: usize, rs2: usize },
    slt { rd: usize, rs1: usize, rs2: usize },
    sltu { rd: usize, rs1: usize, rs2: usize },
    xor { rd: usize, rs1: usize, rs2: usize },
    srl { rd: usize, rs1: usize, rs2: usize },
    sra { rd: usize, rs1: usize, rs2: usize },
    or { rd: usize, rs1: usize, rs2: usize },
    and { rd: usize, rs1: usize, rs2: usize },

    // System & Sync
    fence,
    ecall,
    ebreak,
    unimp,
}

pub mod constants;
pub use constants::*;

#[inline(always)]
pub const fn opcode(raw: u32) -> u32 {
    raw & mask::OPCODE
}

#[inline(always)]
pub const fn funct3(raw: u32) -> u32 {
    (raw >> shift::FUNCT3) & mask::FUNCT3
}

#[inline(always)]
pub const fn funct7(raw: u32) -> u32 {
    (raw >> shift::FUNCT7) & mask::FUNCT7
}

#[inline(always)]
pub const fn rd(raw: u32) -> usize {
    ((raw >> shift::RD) & mask::RD) as usize
}

#[inline(always)]
pub const fn rs1(raw: u32) -> usize {
    ((raw >> shift::RS1) & mask::RS1) as usize
}

#[inline(always)]
pub const fn rs2(raw: u32) -> usize {
    ((raw >> shift::RS2) & mask::RS2) as usize
}

#[inline(always)]
pub const fn shamt(raw: u32) -> u32 {
    (raw >> shift::SHAMT) & mask::SHAMT
}

#[inline(always)]
pub const fn imm_u(raw: u32) -> i32 {
    (raw & !0xFFF) as i32
}

#[inline(always)]
pub const fn imm_j(raw: u32) -> i32 {
    (((raw as i32) >> 31) << 20)
        | (((raw >> 12) & 0xFF) << 12) as i32
        | (((raw >> 20) & 0x1) << 11) as i32
        | (((raw >> 21) & 0x3FF) << 1) as i32
}

#[inline(always)]
pub const fn imm_i(raw: u32) -> i32 {
    (raw as i32) >> 20
}

#[inline(always)]
pub const fn imm_b(raw: u32) -> i32 {
    (((raw as i32) >> 31) << 12)
        | (((raw >> 7) & 0x1) << 11) as i32
        | (((raw >> 25) & 0x3F) << 5) as i32
        | (((raw >> 8) & 0xF) << 1) as i32
}

#[inline(always)]
pub const fn imm_s(raw: u32) -> i32 {
    ((raw as i32 >> 25) << 5) | (((raw >> 7) & 0x1F) as i32)
}

#[inline(always)]
pub const fn is_compressed(first_byte: u8) -> bool {
    (first_byte & 0b11) != 0b11
}

#[inline(always)]
const fn r_prime(r: u16) -> usize {
    8 + ((r & 0x7) as usize)
}

pub fn expand_compressed(raw: u16) -> Option<u32> {
    let op = raw & 0x3;
    let funct3 = (raw >> 13) & 0x7;

    match op {
        // Quadrant 0 (0b00)
        rvc::OP_C0 => match funct3 {
            rvc::funct3::ADDI4SPN => {
                let rd = r_prime(raw >> 2);
                let nzuimm = (((raw >> 7) & 0xF) << 6)
                    | (((raw >> 11) & 0x3) << 4)
                    | (((raw >> 5) & 0x1) << 3)
                    | (((raw >> 6) & 0x1) << 2);
                if nzuimm == 0 {
                    return None;
                }
                Some(encode_i(opcode::OP_IMM, rd, funct3::ADDI, 2, nzuimm as i32))
            }
            rvc::funct3::LW => {
                let rd = r_prime(raw >> 2);
                let rs1 = r_prime(raw >> 7);
                let uimm = (((raw >> 5) & 0x1) << 6)
                    | (((raw >> 10) & 0x7) << 3)
                    | (((raw >> 6) & 0x1) << 2);
                Some(encode_i(opcode::LOAD, rd, funct3::LW, rs1, uimm as i32))
            }
            rvc::funct3::SW => {
                let rs2 = r_prime(raw >> 2);
                let rs1 = r_prime(raw >> 7);
                let uimm = (((raw >> 5) & 0x1) << 6)
                    | (((raw >> 10) & 0x7) << 3)
                    | (((raw >> 6) & 0x1) << 2);
                Some(encode_s(opcode::STORE, funct3::SW, rs1, rs2, uimm as i32))
            }
            _ => None,
        },

        // Quadrant 1 (0b01)
        rvc::OP_C1 => match funct3 {
            rvc::funct3::ADDI => {
                let rd = ((raw >> 7) & 0x1F) as usize;
                let raw_imm = (((raw >> 12) & 0x1) << 5) | ((raw >> 2) & 0x1F);
                let imm = ((raw_imm as i32) << 26) >> 26;
                Some(encode_i(opcode::OP_IMM, rd, funct3::ADDI, rd, imm))
            }
            rvc::funct3::JAL => {
                let o11 = (raw >> 12) & 1;
                let o4 = (raw >> 11) & 1;
                let o9_8 = (raw >> 9) & 0x3;
                let o10 = (raw >> 8) & 1;
                let o6 = (raw >> 7) & 1;
                let o7 = (raw >> 6) & 1;
                let o3_1 = (raw >> 3) & 0x7;
                let o5 = (raw >> 2) & 1;
                let raw_offset = (o11 << 11)
                    | (o10 << 10)
                    | (o9_8 << 8)
                    | (o7 << 7)
                    | (o6 << 6)
                    | (o5 << 5)
                    | (o4 << 4)
                    | (o3_1 << 1);
                let offset = ((raw_offset as i32) << 20) >> 20;
                Some(encode_j(opcode::JAL, 1, offset))
            }
            rvc::funct3::LI => {
                let rd = ((raw >> 7) & 0x1F) as usize;
                if rd == 0 {
                    return None;
                }
                let raw_imm = (((raw >> 12) & 0x1) << 5) | ((raw >> 2) & 0x1F);
                let imm = ((raw_imm as i32) << 26) >> 26;
                Some(encode_i(opcode::OP_IMM, rd, funct3::ADDI, 0, imm))
            }
            rvc::funct3::LUI_ADDI16SP => {
                let rd = ((raw >> 7) & 0x1F) as usize;
                if rd == 2 {
                    // C.ADDI16SP
                    let o9 = (raw >> 12) & 1;
                    let o4 = (raw >> 6) & 1;
                    let o6 = (raw >> 5) & 1;
                    let o8_7 = (raw >> 3) & 0x3;
                    let o5 = (raw >> 2) & 1;
                    let raw_imm = (o9 << 9) | (o8_7 << 7) | (o6 << 6) | (o5 << 5) | (o4 << 4);
                    let imm = ((raw_imm as i32) << 22) >> 22;
                    if imm == 0 {
                        return None;
                    }
                    Some(encode_i(opcode::OP_IMM, 2, funct3::ADDI, 2, imm))
                } else if rd != 0 {
                    // C.LUI
                    let o17 = ((raw >> 12) & 1) as u32;
                    let o16_12 = ((raw >> 2) & 0x1F) as u32;
                    let raw_imm = (o17 << 17) | (o16_12 << 12);
                    let imm = ((raw_imm as i32) << 14) >> 14;
                    if imm == 0 {
                        return None;
                    }
                    Some(encode_u(opcode::LUI, rd, imm))
                } else {
                    None
                }
            }
            rvc::funct3::MISC_ALU => {
                let subop = (raw >> 10) & 0x3;
                let rs1 = r_prime(raw >> 7);
                match subop {
                    0b00 => {
                        let shamt5 = (raw >> 12) & 1;
                        if shamt5 != 0 {
                            return None;
                        }
                        let shamt = ((raw >> 2) & 0x1F) as u32;
                        Some(encode_i_shift(
                            opcode::OP_IMM,
                            rs1,
                            funct3::SRLI,
                            rs1,
                            shamt,
                            funct7::BASE,
                        ))
                    }
                    0b01 => {
                        let shamt5 = (raw >> 12) & 1;
                        if shamt5 != 0 {
                            return None;
                        }
                        let shamt = ((raw >> 2) & 0x1F) as u32;
                        Some(encode_i_shift(
                            opcode::OP_IMM,
                            rs1,
                            funct3::SRAI,
                            rs1,
                            shamt,
                            funct7::ALT,
                        ))
                    }
                    0b10 => {
                        let raw_imm = (((raw >> 12) & 0x1) << 5) | ((raw >> 2) & 0x1F);
                        let imm = ((raw_imm as i32) << 26) >> 26;
                        Some(encode_i(opcode::OP_IMM, rs1, funct3::ANDI, rs1, imm))
                    }
                    0b11 => {
                        let bit12 = (raw >> 12) & 1;
                        if bit12 != 0 {
                            return None;
                        }
                        let funct2 = (raw >> 5) & 0x3;
                        let rs2 = r_prime(raw >> 2);
                        match funct2 {
                            0b00 => Some(encode_r(
                                opcode::OP,
                                rs1,
                                funct3::SUB,
                                rs1,
                                rs2,
                                funct7::ALT,
                            )),
                            0b01 => Some(encode_r(
                                opcode::OP,
                                rs1,
                                funct3::XOR,
                                rs1,
                                rs2,
                                funct7::BASE,
                            )),
                            0b10 => Some(encode_r(
                                opcode::OP,
                                rs1,
                                funct3::OR,
                                rs1,
                                rs2,
                                funct7::BASE,
                            )),
                            0b11 => Some(encode_r(
                                opcode::OP,
                                rs1,
                                funct3::AND,
                                rs1,
                                rs2,
                                funct7::BASE,
                            )),
                            _ => unreachable!(),
                        }
                    }
                    _ => unreachable!(),
                }
            }
            rvc::funct3::J => {
                let o11 = (raw >> 12) & 1;
                let o4 = (raw >> 11) & 1;
                let o9_8 = (raw >> 9) & 0x3;
                let o10 = (raw >> 8) & 1;
                let o6 = (raw >> 7) & 1;
                let o7 = (raw >> 6) & 1;
                let o3_1 = (raw >> 3) & 0x7;
                let o5 = (raw >> 2) & 1;
                let raw_offset = (o11 << 11)
                    | (o10 << 10)
                    | (o9_8 << 8)
                    | (o7 << 7)
                    | (o6 << 6)
                    | (o5 << 5)
                    | (o4 << 4)
                    | (o3_1 << 1);
                let offset = ((raw_offset as i32) << 20) >> 20;
                Some(encode_j(opcode::JAL, 0, offset))
            }
            rvc::funct3::BEQZ => {
                let rs1 = r_prime(raw >> 7);
                let o8 = (raw >> 12) & 1;
                let o4_3 = (raw >> 10) & 0x3;
                let o7_6 = (raw >> 5) & 0x3;
                let o2_1 = (raw >> 3) & 0x3;
                let o5 = (raw >> 2) & 1;
                let raw_offset = (o8 << 8) | (o7_6 << 6) | (o5 << 5) | (o4_3 << 3) | (o2_1 << 1);
                let offset = ((raw_offset as i32) << 23) >> 23;
                Some(encode_b(opcode::BRANCH, funct3::BEQ, rs1, 0, offset))
            }
            rvc::funct3::BNEZ => {
                let rs1 = r_prime(raw >> 7);
                let o8 = (raw >> 12) & 1;
                let o4_3 = (raw >> 10) & 0x3;
                let o7_6 = (raw >> 5) & 0x3;
                let o2_1 = (raw >> 3) & 0x3;
                let o5 = (raw >> 2) & 1;
                let raw_offset = (o8 << 8) | (o7_6 << 6) | (o5 << 5) | (o4_3 << 3) | (o2_1 << 1);
                let offset = ((raw_offset as i32) << 23) >> 23;
                Some(encode_b(opcode::BRANCH, funct3::BNE, rs1, 0, offset))
            }
            _ => None,
        },

        // Quadrant 2 (0b10)
        rvc::OP_C2 => match funct3 {
            rvc::funct3::SLLI => {
                let rd = ((raw >> 7) & 0x1F) as usize;
                if rd == 0 {
                    return None;
                }
                let shamt5 = (raw >> 12) & 1;
                if shamt5 != 0 {
                    return None;
                }
                let shamt = ((raw >> 2) & 0x1F) as u32;
                Some(encode_i_shift(
                    opcode::OP_IMM,
                    rd,
                    funct3::SLLI,
                    rd,
                    shamt,
                    funct7::BASE,
                ))
            }
            rvc::funct3::LWSP => {
                let rd = ((raw >> 7) & 0x1F) as usize;
                if rd == 0 {
                    return None;
                }
                let uimm = (((raw >> 2) & 0x3) << 6)
                    | (((raw >> 12) & 1) << 5)
                    | (((raw >> 4) & 0x7) << 2);
                Some(encode_i(opcode::LOAD, rd, funct3::LW, 2, uimm as i32))
            }
            rvc::funct3::JR_MV_EBREAK_JALR_ADD => {
                let bit12 = (raw >> 12) & 1;
                let rs1 = ((raw >> 7) & 0x1F) as usize;
                let rs2 = ((raw >> 2) & 0x1F) as usize;
                if bit12 == 0 {
                    if rs2 == 0 {
                        // C.JR
                        if rs1 == 0 {
                            return None;
                        }
                        Some(encode_i(opcode::JALR, 0, funct3::JALR, rs1, 0))
                    } else {
                        // C.MV
                        if rs1 == 0 {
                            return None;
                        }
                        Some(encode_r(opcode::OP, rs1, funct3::ADD, 0, rs2, funct7::BASE))
                    }
                } else if rs2 == 0 {
                    if rs1 == 0 {
                        // C.EBREAK
                        Some(system::EBREAK)
                    } else {
                        // C.JALR
                        Some(encode_i(opcode::JALR, 1, funct3::JALR, rs1, 0))
                    }
                } else {
                    // C.ADD
                    if rs1 == 0 {
                        return None;
                    }
                    Some(encode_r(
                        opcode::OP,
                        rs1,
                        funct3::ADD,
                        rs1,
                        rs2,
                        funct7::BASE,
                    ))
                }
            }
            rvc::funct3::SWSP => {
                let rs2 = ((raw >> 2) & 0x1F) as usize;
                let uimm = (((raw >> 7) & 0x3) << 6) | (((raw >> 9) & 0xF) << 2);
                Some(encode_s(opcode::STORE, funct3::SW, 2, rs2, uimm as i32))
            }
            _ => None,
        },

        _ => None,
    }
}

pub fn expand_compressed_to_instruction(raw: u16) -> Option<Instruction> {
    expand_compressed(raw).and_then(parse)
}

pub fn parse(raw: u32) -> Option<Instruction> {
    let opcode = opcode(raw);
    let funct3 = funct3(raw);
    let funct7 = funct7(raw);
    let rd = rd(raw);
    let rs1 = rs1(raw);
    let rs2 = rs2(raw);
    let shamt = shamt(raw);
    // https://hoult.org/rv32i.pdf (page 14)

    match opcode {
        // LUI (U-type)
        opcode::LUI => {
            let imm = imm_u(raw);
            Some(Instruction::lui { rd, imm })
        }

        // AUIPC (U-type)
        opcode::AUIPC => {
            let imm = imm_u(raw);
            Some(Instruction::auipc { rd, imm })
        }

        // JAL (J-type)
        // imm[20] = raw[31], imm[19:12] = raw[19:12], imm[11] = raw[20], imm[10:1] = raw[30:21], imm[0] = 0
        opcode::JAL => {
            let imm = imm_j(raw);
            Some(Instruction::jal { rd, imm })
        }

        // JALR (I-type)
        opcode::JALR => {
            if funct3 != funct3::JALR {
                return None;
            }
            let imm = imm_i(raw);
            Some(Instruction::jalr { rd, rs1, imm })
        }

        // B-type
        // imm[12] = raw[31], imm[11] = raw[7], imm[10:5] = raw[30:25], imm[4:1] = raw[11:8], imm[0] = 0
        opcode::BRANCH => {
            let imm = imm_b(raw);
            match funct3 {
                funct3::BEQ => Some(Instruction::beq { rs1, rs2, imm }),
                funct3::BNE => Some(Instruction::bne { rs1, rs2, imm }),
                funct3::BLT => Some(Instruction::blt { rs1, rs2, imm }),
                funct3::BGE => Some(Instruction::bge { rs1, rs2, imm }),
                funct3::BLTU => Some(Instruction::bltu { rs1, rs2, imm }),
                funct3::BGEU => Some(Instruction::bgeu { rs1, rs2, imm }),
                _ => None,
            }
        }

        // Loads (I-type)
        opcode::LOAD => {
            let imm = imm_i(raw);
            match funct3 {
                funct3::LB => Some(Instruction::lb { rd, rs1, imm }),
                funct3::LH => Some(Instruction::lh { rd, rs1, imm }),
                funct3::LW => Some(Instruction::lw { rd, rs1, imm }),
                funct3::LBU => Some(Instruction::lbu { rd, rs1, imm }),
                funct3::LHU => Some(Instruction::lhu { rd, rs1, imm }),
                _ => None,
            }
        }

        // Stores (S-type)
        // imm[11:5] = raw[31:25], imm[4:0] = raw[11:7]
        opcode::STORE => {
            let imm = imm_s(raw);
            match funct3 {
                funct3::SB => Some(Instruction::sb { rs1, rs2, imm }),
                funct3::SH => Some(Instruction::sh { rs1, rs2, imm }),
                funct3::SW => Some(Instruction::sw { rs1, rs2, imm }),
                _ => None,
            }
        }

        // I-type ALU & shifts
        opcode::OP_IMM => {
            let imm = imm_i(raw);
            match funct3 {
                funct3::ADDI => Some(Instruction::addi { rd, rs1, imm }),
                funct3::SLTI => Some(Instruction::slti { rd, rs1, imm }),
                funct3::SLTIU => Some(Instruction::sltiu { rd, rs1, imm }),
                funct3::XORI => Some(Instruction::xori { rd, rs1, imm }),
                funct3::ORI => Some(Instruction::ori { rd, rs1, imm }),
                funct3::ANDI => Some(Instruction::andi { rd, rs1, imm }),
                // Shift amount is lower 5 bits (shamt), funct7 must be 0x00
                funct3::SLLI if funct7 == funct7::BASE => {
                    Some(Instruction::slli { rd, rs1, shamt })
                }
                // SRLI funct7 == 0x00, SRAI funct7 == 0x20 (bit 30 set for arithmetic shift)
                funct3::SRLI_SRAI if funct7 == funct7::BASE => {
                    Some(Instruction::srli { rd, rs1, shamt })
                }
                funct3::SRLI_SRAI if funct7 == funct7::ALT => {
                    Some(Instruction::srai { rd, rs1, shamt })
                }
                _ => None,
            }
        }

        // R-type
        // Bit 30 differentiates add/sub and srl/sra
        opcode::OP => match (funct3, funct7) {
            (funct3::ADD_SUB, funct7::BASE) => Some(Instruction::add { rd, rs1, rs2 }),
            (funct3::ADD_SUB, funct7::ALT) => Some(Instruction::sub { rd, rs1, rs2 }),
            (funct3::SLL, funct7::BASE) => Some(Instruction::sll { rd, rs1, rs2 }),
            (funct3::SLT, funct7::BASE) => Some(Instruction::slt { rd, rs1, rs2 }),
            (funct3::SLTU, funct7::BASE) => Some(Instruction::sltu { rd, rs1, rs2 }),
            (funct3::XOR, funct7::BASE) => Some(Instruction::xor { rd, rs1, rs2 }),
            (funct3::SRL_SRA, funct7::BASE) => Some(Instruction::srl { rd, rs1, rs2 }),
            (funct3::SRL_SRA, funct7::ALT) => Some(Instruction::sra { rd, rs1, rs2 }),
            (funct3::OR, funct7::BASE) => Some(Instruction::or { rd, rs1, rs2 }),
            (funct3::AND, funct7::BASE) => Some(Instruction::and { rd, rs1, rs2 }),
            _ => None,
        },

        // FENCE
        opcode::FENCE => {
            if funct3 == funct3::FENCE && rd == 0 && rs1 == 0 {
                Some(Instruction::fence)
            } else {
                None
            }
        }

        // ECALL / EBREAK / UNIMP
        opcode::SYSTEM => match raw {
            system::ECALL => Some(Instruction::ecall),
            system::EBREAK => Some(Instruction::ebreak),
            system::UNIMP => Some(Instruction::unimp),
            _ => None,
        },

        _ => None,
    }
}

impl Instruction {
    pub fn encode(&self) -> u32 {
        match *self {
            // Upper Immediate (U-type)
            Instruction::lui { rd, imm } => encode_u(opcode::LUI, rd, imm),
            Instruction::auipc { rd, imm } => encode_u(opcode::AUIPC, rd, imm),

            // Unconditional Jumps
            Instruction::jal { rd, imm } => encode_j(opcode::JAL, rd, imm),
            Instruction::jalr { rd, rs1, imm } => {
                encode_i(opcode::JALR, rd, funct3::JALR, rs1, imm)
            }

            // Conditional Branches (B-type)
            Instruction::beq { rs1, rs2, imm } => {
                encode_b(opcode::BRANCH, funct3::BEQ, rs1, rs2, imm)
            }
            Instruction::bne { rs1, rs2, imm } => {
                encode_b(opcode::BRANCH, funct3::BNE, rs1, rs2, imm)
            }
            Instruction::blt { rs1, rs2, imm } => {
                encode_b(opcode::BRANCH, funct3::BLT, rs1, rs2, imm)
            }
            Instruction::bge { rs1, rs2, imm } => {
                encode_b(opcode::BRANCH, funct3::BGE, rs1, rs2, imm)
            }
            Instruction::bltu { rs1, rs2, imm } => {
                encode_b(opcode::BRANCH, funct3::BLTU, rs1, rs2, imm)
            }
            Instruction::bgeu { rs1, rs2, imm } => {
                encode_b(opcode::BRANCH, funct3::BGEU, rs1, rs2, imm)
            }

            // Loads (I-type)
            Instruction::lb { rd, rs1, imm } => encode_i(opcode::LOAD, rd, funct3::LB, rs1, imm),
            Instruction::lh { rd, rs1, imm } => encode_i(opcode::LOAD, rd, funct3::LH, rs1, imm),
            Instruction::lw { rd, rs1, imm } => encode_i(opcode::LOAD, rd, funct3::LW, rs1, imm),
            Instruction::lbu { rd, rs1, imm } => encode_i(opcode::LOAD, rd, funct3::LBU, rs1, imm),
            Instruction::lhu { rd, rs1, imm } => encode_i(opcode::LOAD, rd, funct3::LHU, rs1, imm),

            // Stores (S-type)
            Instruction::sb { rs1, rs2, imm } => encode_s(opcode::STORE, funct3::SB, rs1, rs2, imm),
            Instruction::sh { rs1, rs2, imm } => encode_s(opcode::STORE, funct3::SH, rs1, rs2, imm),
            Instruction::sw { rs1, rs2, imm } => encode_s(opcode::STORE, funct3::SW, rs1, rs2, imm),

            // Immediate Arithmetic (I-type)
            Instruction::addi { rd, rs1, imm } => {
                encode_i(opcode::OP_IMM, rd, funct3::ADDI, rs1, imm)
            }
            Instruction::slti { rd, rs1, imm } => {
                encode_i(opcode::OP_IMM, rd, funct3::SLTI, rs1, imm)
            }
            Instruction::sltiu { rd, rs1, imm } => {
                encode_i(opcode::OP_IMM, rd, funct3::SLTIU, rs1, imm)
            }
            Instruction::xori { rd, rs1, imm } => {
                encode_i(opcode::OP_IMM, rd, funct3::XORI, rs1, imm)
            }
            Instruction::ori { rd, rs1, imm } => {
                encode_i(opcode::OP_IMM, rd, funct3::ORI, rs1, imm)
            }
            Instruction::andi { rd, rs1, imm } => {
                encode_i(opcode::OP_IMM, rd, funct3::ANDI, rs1, imm)
            }
            Instruction::slli { rd, rs1, shamt } => {
                encode_i_shift(opcode::OP_IMM, rd, funct3::SLLI, rs1, shamt, funct7::BASE)
            }
            Instruction::srli { rd, rs1, shamt } => {
                encode_i_shift(opcode::OP_IMM, rd, funct3::SRLI, rs1, shamt, funct7::BASE)
            }
            Instruction::srai { rd, rs1, shamt } => {
                encode_i_shift(opcode::OP_IMM, rd, funct3::SRAI, rs1, shamt, funct7::ALT)
            }

            // Register Arithmetic (R-type)
            Instruction::add { rd, rs1, rs2 } => {
                encode_r(opcode::OP, rd, funct3::ADD, rs1, rs2, funct7::BASE)
            }
            Instruction::sub { rd, rs1, rs2 } => {
                encode_r(opcode::OP, rd, funct3::SUB, rs1, rs2, funct7::ALT)
            }
            Instruction::sll { rd, rs1, rs2 } => {
                encode_r(opcode::OP, rd, funct3::SLL, rs1, rs2, funct7::BASE)
            }
            Instruction::slt { rd, rs1, rs2 } => {
                encode_r(opcode::OP, rd, funct3::SLT, rs1, rs2, funct7::BASE)
            }
            Instruction::sltu { rd, rs1, rs2 } => {
                encode_r(opcode::OP, rd, funct3::SLTU, rs1, rs2, funct7::BASE)
            }
            Instruction::xor { rd, rs1, rs2 } => {
                encode_r(opcode::OP, rd, funct3::XOR, rs1, rs2, funct7::BASE)
            }
            Instruction::srl { rd, rs1, rs2 } => {
                encode_r(opcode::OP, rd, funct3::SRL, rs1, rs2, funct7::BASE)
            }
            Instruction::sra { rd, rs1, rs2 } => {
                encode_r(opcode::OP, rd, funct3::SRA, rs1, rs2, funct7::ALT)
            }
            Instruction::or { rd, rs1, rs2 } => {
                encode_r(opcode::OP, rd, funct3::OR, rs1, rs2, funct7::BASE)
            }
            Instruction::and { rd, rs1, rs2 } => {
                encode_r(opcode::OP, rd, funct3::AND, rs1, rs2, funct7::BASE)
            }

            // System & Sync
            Instruction::fence => fence::FENCE,
            Instruction::ecall => system::ECALL,
            Instruction::ebreak => system::EBREAK,
            Instruction::unimp => system::UNIMP,
        }
    }
}

#[inline(always)]
fn encode_r(opcode: u32, rd: usize, funct3: u32, rs1: usize, rs2: usize, funct7: u32) -> u32 {
    opcode
        | (((rd as u32) & mask::RD) << shift::RD)
        | ((funct3 & mask::FUNCT3) << shift::FUNCT3)
        | (((rs1 as u32) & mask::RS1) << shift::RS1)
        | (((rs2 as u32) & mask::RS2) << shift::RS2)
        | ((funct7 & mask::FUNCT7) << shift::FUNCT7)
}

#[inline(always)]
fn encode_i(opcode: u32, rd: usize, funct3: u32, rs1: usize, imm: i32) -> u32 {
    opcode
        | (((rd as u32) & mask::RD) << shift::RD)
        | ((funct3 & mask::FUNCT3) << shift::FUNCT3)
        | (((rs1 as u32) & mask::RS1) << shift::RS1)
        | (((imm as u32) & 0xFFF) << 20)
}

#[inline(always)]
fn encode_i_shift(opcode: u32, rd: usize, funct3: u32, rs1: usize, shamt: u32, funct7: u32) -> u32 {
    opcode
        | (((rd as u32) & mask::RD) << shift::RD)
        | ((funct3 & mask::FUNCT3) << shift::FUNCT3)
        | (((rs1 as u32) & mask::RS1) << shift::RS1)
        | ((shamt & mask::SHAMT) << shift::SHAMT)
        | ((funct7 & mask::FUNCT7) << shift::FUNCT7)
}

#[inline(always)]
fn encode_s(opcode: u32, funct3: u32, rs1: usize, rs2: usize, imm: i32) -> u32 {
    let imm_u = imm as u32;
    opcode
        | ((imm_u & 0x1F) << 7)
        | ((funct3 & mask::FUNCT3) << shift::FUNCT3)
        | (((rs1 as u32) & mask::RS1) << shift::RS1)
        | (((rs2 as u32) & mask::RS2) << shift::RS2)
        | (((imm_u >> 5) & 0x7F) << 25)
}

#[inline(always)]
fn encode_b(opcode: u32, funct3: u32, rs1: usize, rs2: usize, imm: i32) -> u32 {
    let imm_u = imm as u32;
    let b_12 = (imm_u >> 12) & 1;
    let b_11 = (imm_u >> 11) & 1;
    let b_10_5 = (imm_u >> 5) & 0x3F;
    let b_4_1 = (imm_u >> 1) & 0xF;
    opcode
        | (b_11 << 7)
        | (b_4_1 << 8)
        | ((funct3 & mask::FUNCT3) << shift::FUNCT3)
        | (((rs1 as u32) & mask::RS1) << shift::RS1)
        | (((rs2 as u32) & mask::RS2) << shift::RS2)
        | (b_10_5 << 25)
        | (b_12 << 31)
}

#[inline(always)]
fn encode_u(opcode: u32, rd: usize, imm: i32) -> u32 {
    opcode | (((rd as u32) & mask::RD) << shift::RD) | ((imm as u32) & !0xFFF)
}

#[inline(always)]
fn encode_j(opcode: u32, rd: usize, imm: i32) -> u32 {
    let imm_u = imm as u32;
    let j_20 = (imm_u >> 20) & 1;
    let j_19_12 = (imm_u >> 12) & 0xFF;
    let j_11 = (imm_u >> 11) & 1;
    let j_10_1 = (imm_u >> 1) & 0x3FF;
    opcode
        | (((rd as u32) & mask::RD) << shift::RD)
        | (j_19_12 << 12)
        | (j_11 << 20)
        | (j_10_1 << 21)
        | (j_20 << 31)
}

impl core::fmt::Display for Instruction {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match *self {
            // Upper Immediate (U-type)
            Instruction::lui { rd, imm } => write!(f, "lui x{}, {}", rd, imm),
            Instruction::auipc { rd, imm } => write!(f, "auipc x{}, {}", rd, imm),

            // Unconditional Jumps
            Instruction::jal { rd, imm } => write!(f, "jal x{}, {}", rd, imm),
            Instruction::jalr { rd, rs1, imm } => write!(f, "jalr x{}, {}(x{})", rd, imm, rs1),

            // Conditional Branches (B-type)
            Instruction::beq { rs1, rs2, imm } => write!(f, "beq x{}, x{}, {}", rs1, rs2, imm),
            Instruction::bne { rs1, rs2, imm } => write!(f, "bne x{}, x{}, {}", rs1, rs2, imm),
            Instruction::blt { rs1, rs2, imm } => write!(f, "blt x{}, x{}, {}", rs1, rs2, imm),
            Instruction::bge { rs1, rs2, imm } => write!(f, "bge x{}, x{}, {}", rs1, rs2, imm),
            Instruction::bltu { rs1, rs2, imm } => write!(f, "bltu x{}, x{}, {}", rs1, rs2, imm),
            Instruction::bgeu { rs1, rs2, imm } => write!(f, "bgeu x{}, x{}, {}", rs1, rs2, imm),

            // Loads (I-type)
            Instruction::lb { rd, rs1, imm } => write!(f, "lb x{}, {}(x{})", rd, imm, rs1),
            Instruction::lh { rd, rs1, imm } => write!(f, "lh x{}, {}(x{})", rd, imm, rs1),
            Instruction::lw { rd, rs1, imm } => write!(f, "lw x{}, {}(x{})", rd, imm, rs1),
            Instruction::lbu { rd, rs1, imm } => write!(f, "lbu x{}, {}(x{})", rd, imm, rs1),
            Instruction::lhu { rd, rs1, imm } => write!(f, "lhu x{}, {}(x{})", rd, imm, rs1),

            // Stores (S-type)
            Instruction::sb { rs1, rs2, imm } => write!(f, "sb x{}, {}(x{})", rs2, imm, rs1),
            Instruction::sh { rs1, rs2, imm } => write!(f, "sh x{}, {}(x{})", rs2, imm, rs1),
            Instruction::sw { rs1, rs2, imm } => write!(f, "sw x{}, {}(x{})", rs2, imm, rs1),

            // Immediate Arithmetic (I-type)
            Instruction::addi { rd, rs1, imm } => write!(f, "addi x{}, x{}, {}", rd, rs1, imm),
            Instruction::slti { rd, rs1, imm } => write!(f, "slti x{}, x{}, {}", rd, rs1, imm),
            Instruction::sltiu { rd, rs1, imm } => write!(f, "sltiu x{}, x{}, {}", rd, rs1, imm),
            Instruction::xori { rd, rs1, imm } => write!(f, "xori x{}, x{}, {}", rd, rs1, imm),
            Instruction::ori { rd, rs1, imm } => write!(f, "ori x{}, x{}, {}", rd, rs1, imm),
            Instruction::andi { rd, rs1, imm } => write!(f, "andi x{}, x{}, {}", rd, rs1, imm),
            Instruction::slli { rd, rs1, shamt } => write!(f, "slli x{}, x{}, {}", rd, rs1, shamt),
            Instruction::srli { rd, rs1, shamt } => write!(f, "srli x{}, x{}, {}", rd, rs1, shamt),
            Instruction::srai { rd, rs1, shamt } => write!(f, "srai x{}, x{}, {}", rd, rs1, shamt),

            // Register Arithmetic (R-type)
            Instruction::add { rd, rs1, rs2 } => write!(f, "add x{}, x{}, x{}", rd, rs1, rs2),
            Instruction::sub { rd, rs1, rs2 } => write!(f, "sub x{}, x{}, x{}", rd, rs1, rs2),
            Instruction::sll { rd, rs1, rs2 } => write!(f, "sll x{}, x{}, x{}", rd, rs1, rs2),
            Instruction::slt { rd, rs1, rs2 } => write!(f, "slt x{}, x{}, x{}", rd, rs1, rs2),
            Instruction::sltu { rd, rs1, rs2 } => write!(f, "sltu x{}, x{}, x{}", rd, rs1, rs2),
            Instruction::xor { rd, rs1, rs2 } => write!(f, "xor x{}, x{}, x{}", rd, rs1, rs2),
            Instruction::srl { rd, rs1, rs2 } => write!(f, "srl x{}, x{}, x{}", rd, rs1, rs2),
            Instruction::sra { rd, rs1, rs2 } => write!(f, "sra x{}, x{}, x{}", rd, rs1, rs2),
            Instruction::or { rd, rs1, rs2 } => write!(f, "or x{}, x{}, x{}", rd, rs1, rs2),
            Instruction::and { rd, rs1, rs2 } => write!(f, "and x{}, x{}, x{}", rd, rs1, rs2),

            // System & Sync
            Instruction::fence => write!(f, "fence"),
            Instruction::ecall => write!(f, "ecall"),
            Instruction::ebreak => write!(f, "ebreak"),
            Instruction::unimp => write!(f, "unimp"),
        }
    }
}

#[cfg(test)]
#[allow(clippy::identity_op)]
mod tests {
    use super::*;

    #[test]
    fn test_u_type() {
        // lui x1, 0x12345000
        let raw = 0x123450B7;
        assert_eq!(
            parse(raw),
            Some(Instruction::lui {
                rd: 1,
                imm: 0x12345000
            })
        );

        // lui with negative immediate: bit 31 set
        let raw = 0x800000B7;
        assert_eq!(
            parse(raw),
            Some(Instruction::lui {
                rd: 1,
                imm: -0x80000000
            })
        );

        // auipc x2, 0x00001000
        let raw = 0x00001117;
        assert_eq!(parse(raw), Some(Instruction::auipc { rd: 2, imm: 0x1000 }));
    }

    #[test]
    fn test_j_type() {
        // jal x1, 4 (forward)
        // imm = 4 -> imm[20]=0, imm[10:1]=2, imm[11]=0, imm[19:12]=0
        let raw = (2 << 21) | (1 << 7) | 0x6F;
        assert_eq!(parse(raw), Some(Instruction::jal { rd: 1, imm: 4 }));

        // jal x1, -4 (backward)
        let raw = 0xFFDFF0EF;
        assert_eq!(parse(raw), Some(Instruction::jal { rd: 1, imm: -4 }));
    }

    #[test]
    fn test_jalr() {
        // jalr x1, x2, 42
        let raw = (42 << 20) | (2 << 15) | (0 << 12) | (1 << 7) | 0x67;
        assert_eq!(
            parse(raw),
            Some(Instruction::jalr {
                rd: 1,
                rs1: 2,
                imm: 42
            })
        );

        // jalr x1, x2, -4
        let raw = ((-4i32 as u32 & 0xFFF) << 20) | (2 << 15) | (0 << 12) | (1 << 7) | 0x67;
        assert_eq!(
            parse(raw),
            Some(Instruction::jalr {
                rd: 1,
                rs1: 2,
                imm: -4
            })
        );

        // invalid funct3
        let raw_invalid = (42 << 20) | (2 << 15) | (1 << 12) | (1 << 7) | 0x67;
        assert_eq!(parse(raw_invalid), None);
    }

    fn encode_b(imm: i32, rs1: usize, rs2: usize, funct3: u32) -> u32 {
        let imm_u = imm as u32;
        let b_12 = (imm_u >> 12) & 1;
        let b_11 = (imm_u >> 11) & 1;
        let b_10_5 = (imm_u >> 5) & 0x3F;
        let b_4_1 = (imm_u >> 1) & 0xF;
        (b_12 << 31)
            | (b_10_5 << 25)
            | ((rs2 as u32) << 20)
            | ((rs1 as u32) << 15)
            | (funct3 << 12)
            | (b_4_1 << 8)
            | (b_11 << 7)
            | 0x63
    }

    #[test]
    fn test_b_type() {
        let funct3_list = [
            (
                0b000,
                Instruction::beq {
                    rs1: 1,
                    rs2: 2,
                    imm: -4,
                },
            ),
            (
                0b001,
                Instruction::bne {
                    rs1: 1,
                    rs2: 2,
                    imm: -4,
                },
            ),
            (
                0b100,
                Instruction::blt {
                    rs1: 1,
                    rs2: 2,
                    imm: -4,
                },
            ),
            (
                0b101,
                Instruction::bge {
                    rs1: 1,
                    rs2: 2,
                    imm: -4,
                },
            ),
            (
                0b110,
                Instruction::bltu {
                    rs1: 1,
                    rs2: 2,
                    imm: -4,
                },
            ),
            (
                0b111,
                Instruction::bgeu {
                    rs1: 1,
                    rs2: 2,
                    imm: -4,
                },
            ),
        ];

        for (f3, expected) in funct3_list {
            let raw = encode_b(-4, 1, 2, f3);
            assert_eq!(parse(raw), Some(expected));
        }

        // Forward branch: +8
        let raw_fwd = encode_b(8, 1, 2, 0b000);
        assert_eq!(
            parse(raw_fwd),
            Some(Instruction::beq {
                rs1: 1,
                rs2: 2,
                imm: 8
            })
        );

        // Invalid funct3 (0b010)
        let raw_invalid = 0x0020A063;
        assert_eq!(parse(raw_invalid), None);
    }

    #[test]
    fn test_loads() {
        let funct3_loads = [
            (
                0b000,
                Instruction::lb {
                    rd: 3,
                    rs1: 4,
                    imm: -4,
                },
            ),
            (
                0b001,
                Instruction::lh {
                    rd: 3,
                    rs1: 4,
                    imm: -4,
                },
            ),
            (
                0b010,
                Instruction::lw {
                    rd: 3,
                    rs1: 4,
                    imm: -4,
                },
            ),
            (
                0b100,
                Instruction::lbu {
                    rd: 3,
                    rs1: 4,
                    imm: -4,
                },
            ),
            (
                0b101,
                Instruction::lhu {
                    rd: 3,
                    rs1: 4,
                    imm: -4,
                },
            ),
        ];

        for (f3, expected) in funct3_loads {
            let raw =
                ((-4i32 as u32 & 0xFFF) << 20) | (4 << 15) | ((f3 as u32) << 12) | (3 << 7) | 0x03;
            assert_eq!(parse(raw), Some(expected));
        }

        // Invalid load funct3 (0b011)
        let raw_invalid = (4 << 15) | (3 << 12) | (3 << 7) | 0x03;
        assert_eq!(parse(raw_invalid), None);
    }

    #[test]
    fn test_stores() {
        let funct3_stores = [
            (
                0b000,
                Instruction::sb {
                    rs1: 1,
                    rs2: 2,
                    imm: -4,
                },
            ),
            (
                0b001,
                Instruction::sh {
                    rs1: 1,
                    rs2: 2,
                    imm: -4,
                },
            ),
            (
                0b010,
                Instruction::sw {
                    rs1: 1,
                    rs2: 2,
                    imm: -4,
                },
            ),
        ];

        for (f3, expected) in funct3_stores {
            // imm = -4 (0xFFC): imm[11:5] = 0x7F, imm[4:0] = 0x1C
            let raw =
                (0x7F << 25) | (2 << 20) | (1 << 15) | ((f3 as u32) << 12) | (0x1C << 7) | 0x23;
            assert_eq!(parse(raw), Some(expected));
        }

        // Positive offset store: +12
        // imm = 12 (0x00C): imm[11:5] = 0, imm[4:0] = 0x0C
        let raw_pos = (0 << 25) | (2 << 20) | (1 << 15) | (2 << 12) | (0x0C << 7) | 0x23;
        assert_eq!(
            parse(raw_pos),
            Some(Instruction::sw {
                rs1: 1,
                rs2: 2,
                imm: 12
            })
        );

        // Invalid store funct3 (0b011)
        let raw_invalid = (2 << 20) | (1 << 15) | (3 << 12) | 0x23;
        assert_eq!(parse(raw_invalid), None);
    }

    #[test]
    fn test_i_type_alu() {
        assert_eq!(
            parse((42 << 20) | (2 << 15) | (0 << 12) | (1 << 7) | 0x13),
            Some(Instruction::addi {
                rd: 1,
                rs1: 2,
                imm: 42
            })
        );
        assert_eq!(
            parse(((-5i32 as u32 & 0xFFF) << 20) | (2 << 15) | (2 << 12) | (1 << 7) | 0x13),
            Some(Instruction::slti {
                rd: 1,
                rs1: 2,
                imm: -5
            })
        );
        assert_eq!(
            parse((100 << 20) | (2 << 15) | (3 << 12) | (1 << 7) | 0x13),
            Some(Instruction::sltiu {
                rd: 1,
                rs1: 2,
                imm: 100
            })
        );
        assert_eq!(
            parse((0xFF << 20) | (2 << 15) | (4 << 12) | (1 << 7) | 0x13),
            Some(Instruction::xori {
                rd: 1,
                rs1: 2,
                imm: 0xFF
            })
        );
        assert_eq!(
            parse((0x0F << 20) | (2 << 15) | (6 << 12) | (1 << 7) | 0x13),
            Some(Instruction::ori {
                rd: 1,
                rs1: 2,
                imm: 0x0F
            })
        );
        assert_eq!(
            parse((0xAA << 20) | (2 << 15) | (7 << 12) | (1 << 7) | 0x13),
            Some(Instruction::andi {
                rd: 1,
                rs1: 2,
                imm: 0xAA
            })
        );

        // slli
        let raw_slli = (0x00 << 25) | (5 << 20) | (2 << 15) | (1 << 12) | (1 << 7) | 0x13;
        assert_eq!(
            parse(raw_slli),
            Some(Instruction::slli {
                rd: 1,
                rs1: 2,
                shamt: 5
            })
        );

        // slli with non-zero funct7 must fail
        let raw_slli_invalid = (0x01 << 25) | (5 << 20) | (2 << 15) | (1 << 12) | (1 << 7) | 0x13;
        assert_eq!(parse(raw_slli_invalid), None);

        // srli
        let raw_srli = (0x00 << 25) | (31 << 20) | (2 << 15) | (5 << 12) | (1 << 7) | 0x13;
        assert_eq!(
            parse(raw_srli),
            Some(Instruction::srli {
                rd: 1,
                rs1: 2,
                shamt: 31
            })
        );

        // srai
        let raw_srai = (0x20 << 25) | (16 << 20) | (2 << 15) | (5 << 12) | (1 << 7) | 0x13;
        assert_eq!(
            parse(raw_srai),
            Some(Instruction::srai {
                rd: 1,
                rs1: 2,
                shamt: 16
            })
        );

        // srli/srai with invalid funct7
        let raw_shift_invalid = (0x10 << 25) | (5 << 20) | (2 << 15) | (5 << 12) | (1 << 7) | 0x13;
        assert_eq!(parse(raw_shift_invalid), None);
    }

    #[test]
    fn test_r_type_alu() {
        let r_cases = [
            (
                0b000,
                0x00,
                Instruction::add {
                    rd: 1,
                    rs1: 2,
                    rs2: 3,
                },
            ),
            (
                0b000,
                0x20,
                Instruction::sub {
                    rd: 1,
                    rs1: 2,
                    rs2: 3,
                },
            ),
            (
                0b001,
                0x00,
                Instruction::sll {
                    rd: 1,
                    rs1: 2,
                    rs2: 3,
                },
            ),
            (
                0b010,
                0x00,
                Instruction::slt {
                    rd: 1,
                    rs1: 2,
                    rs2: 3,
                },
            ),
            (
                0b011,
                0x00,
                Instruction::sltu {
                    rd: 1,
                    rs1: 2,
                    rs2: 3,
                },
            ),
            (
                0b100,
                0x00,
                Instruction::xor {
                    rd: 1,
                    rs1: 2,
                    rs2: 3,
                },
            ),
            (
                0b101,
                0x00,
                Instruction::srl {
                    rd: 1,
                    rs1: 2,
                    rs2: 3,
                },
            ),
            (
                0b101,
                0x20,
                Instruction::sra {
                    rd: 1,
                    rs1: 2,
                    rs2: 3,
                },
            ),
            (
                0b110,
                0x00,
                Instruction::or {
                    rd: 1,
                    rs1: 2,
                    rs2: 3,
                },
            ),
            (
                0b111,
                0x00,
                Instruction::and {
                    rd: 1,
                    rs1: 2,
                    rs2: 3,
                },
            ),
        ];

        for (f3, f7, expected) in r_cases {
            let raw = (f7 << 25) | (3 << 20) | (2 << 15) | (f3 << 12) | (1 << 7) | 0x33;
            assert_eq!(parse(raw), Some(expected));
        }

        // Invalid funct7 for add/sub
        let raw_invalid = (0x01 << 25) | (3 << 20) | (2 << 15) | (0 << 12) | (1 << 7) | 0x33;
        assert_eq!(parse(raw_invalid), None);
    }

    #[test]
    fn test_system_and_fence() {
        assert_eq!(parse(0x0000000F), Some(Instruction::fence));
        // fence with non-zero funct3 should be None
        assert_eq!(parse(0x0000100F), None);
        // fence with non-zero rd or rs1 should be None
        assert_eq!(parse(0x0000008F), None); // rd = 1
        assert_eq!(parse(0x0000800F), None); // rs1 = 1

        assert_eq!(parse(0x00000073), Some(Instruction::ecall));
        assert_eq!(parse(0x00100073), Some(Instruction::ebreak));
        assert_eq!(parse(0xc0001073), Some(Instruction::unimp));
        // CSR or other opcode 0x73
        assert_eq!(parse(0x00001073), None);
    }

    #[test]
    fn test_invalid_opcodes() {
        assert_eq!(parse(0x00000000), None);
        assert_eq!(parse(0xFFFFFFFF), None);
        assert_eq!(parse(0x0000007B), None);
    }

    fn assert_instruction_roundtrip(instr: Instruction) {
        let word = instr.encode();
        let parsed = parse(word);
        assert_eq!(
            parsed,
            Some(instr),
            "parse(encode({:?})) failed! word=0x{:08X}",
            instr,
            word
        );
        assert_eq!(
            parsed.unwrap().encode(),
            word,
            "re-encode of parsed instruction mismatch for {:?}",
            instr
        );
    }

    #[test]
    fn test_roundtrip_u_type() {
        let test_cases = [
            Instruction::lui {
                rd: 1,
                imm: 0x12345000,
            },
            Instruction::lui {
                rd: 15,
                imm: -0x80000000,
            },
            Instruction::lui {
                rd: 31,
                imm: 0x00001000,
            },
            Instruction::lui {
                rd: 0,
                imm: 0x7FFFF000,
            },
            Instruction::auipc {
                rd: 2,
                imm: 0x00001000,
            },
            Instruction::auipc {
                rd: 1,
                imm: -0x1000,
            },
            Instruction::auipc {
                rd: 16,
                imm: 0x7FFFF000,
            },
            Instruction::auipc {
                rd: 0,
                imm: -0x80000000,
            },
        ];

        for instr in test_cases {
            assert_instruction_roundtrip(instr);
        }
    }

    #[test]
    fn test_roundtrip_j_type() {
        let test_cases = [
            Instruction::jal { rd: 1, imm: 4 },
            Instruction::jal { rd: 5, imm: 200 },
            Instruction::jal { rd: 1, imm: -4 },
            Instruction::jal {
                rd: 0,
                imm: -1048576,
            },
            Instruction::jal {
                rd: 31,
                imm: 1048574,
            },
            Instruction::jal { rd: 15, imm: 0 },
        ];

        for instr in test_cases {
            assert_instruction_roundtrip(instr);
        }

        // Multiple combinations of registers and offsets
        for rd in [0, 1, 10, 15, 31] {
            for imm in [-1048576, -4, 0, 4, 1000, 1048574] {
                assert_instruction_roundtrip(Instruction::jal { rd, imm });
            }
        }
    }

    #[test]
    fn test_roundtrip_jalr() {
        let test_cases = [
            Instruction::jalr {
                rd: 1,
                rs1: 2,
                imm: 42,
            },
            Instruction::jalr {
                rd: 1,
                rs1: 2,
                imm: -4,
            },
            Instruction::jalr {
                rd: 0,
                rs1: 1,
                imm: 0,
            }, // ret
            Instruction::jalr {
                rd: 31,
                rs1: 30,
                imm: 2047,
            },
            Instruction::jalr {
                rd: 5,
                rs1: 10,
                imm: -2048,
            },
            Instruction::jalr {
                rd: 0,
                rs1: 0,
                imm: 0,
            },
        ];

        for instr in test_cases {
            assert_instruction_roundtrip(instr);
        }

        for rd in [0, 1, 15, 31] {
            for rs1 in [0, 1, 15, 31] {
                for imm in [-2048, -4, 0, 4, 2047] {
                    assert_instruction_roundtrip(Instruction::jalr { rd, rs1, imm });
                }
            }
        }
    }

    #[test]
    fn test_roundtrip_b_type() {
        let offsets = [-4096, -8, -4, 0, 4, 8, 4094];

        for &imm in &offsets {
            for &(rs1, rs2) in &[(1, 2), (0, 15), (31, 30), (0, 0)] {
                assert_instruction_roundtrip(Instruction::beq { rs1, rs2, imm });
                assert_instruction_roundtrip(Instruction::bne { rs1, rs2, imm });
                assert_instruction_roundtrip(Instruction::blt { rs1, rs2, imm });
                assert_instruction_roundtrip(Instruction::bge { rs1, rs2, imm });
                assert_instruction_roundtrip(Instruction::bltu { rs1, rs2, imm });
                assert_instruction_roundtrip(Instruction::bgeu { rs1, rs2, imm });
            }
        }
    }

    #[test]
    fn test_roundtrip_loads() {
        let offsets = [-2048, -4, 0, 4, 12, 2047];

        for &imm in &offsets {
            for &(rd, rs1) in &[(3, 4), (0, 1), (15, 31), (31, 0)] {
                assert_instruction_roundtrip(Instruction::lb { rd, rs1, imm });
                assert_instruction_roundtrip(Instruction::lh { rd, rs1, imm });
                assert_instruction_roundtrip(Instruction::lw { rd, rs1, imm });
                assert_instruction_roundtrip(Instruction::lbu { rd, rs1, imm });
                assert_instruction_roundtrip(Instruction::lhu { rd, rs1, imm });
            }
        }
    }

    #[test]
    fn test_roundtrip_stores() {
        let offsets = [-2048, -4, 0, 4, 12, 2047];

        for &imm in &offsets {
            for &(rs1, rs2) in &[(1, 2), (0, 1), (15, 31), (31, 0)] {
                assert_instruction_roundtrip(Instruction::sb { rs1, rs2, imm });
                assert_instruction_roundtrip(Instruction::sh { rs1, rs2, imm });
                assert_instruction_roundtrip(Instruction::sw { rs1, rs2, imm });
            }
        }
    }

    #[test]
    fn test_roundtrip_i_type_alu() {
        let immediates = [-2048, -42, 0, 1, 100, 2047];

        for &imm in &immediates {
            for &(rd, rs1) in &[(1, 2), (0, 15), (31, 30), (10, 0)] {
                assert_instruction_roundtrip(Instruction::addi { rd, rs1, imm });
                assert_instruction_roundtrip(Instruction::slti { rd, rs1, imm });
                assert_instruction_roundtrip(Instruction::sltiu { rd, rs1, imm });
                assert_instruction_roundtrip(Instruction::xori { rd, rs1, imm });
                assert_instruction_roundtrip(Instruction::ori { rd, rs1, imm });
                assert_instruction_roundtrip(Instruction::andi { rd, rs1, imm });
            }
        }
    }

    #[test]
    fn test_roundtrip_shifts() {
        let shamts = [0, 1, 5, 16, 31];

        for &shamt in &shamts {
            for &(rd, rs1) in &[(1, 2), (0, 15), (31, 30), (10, 0)] {
                assert_instruction_roundtrip(Instruction::slli { rd, rs1, shamt });
                assert_instruction_roundtrip(Instruction::srli { rd, rs1, shamt });
                assert_instruction_roundtrip(Instruction::srai { rd, rs1, shamt });
            }
        }
    }

    #[test]
    fn test_roundtrip_r_type_alu() {
        let reg_pairs = [(1, 2, 3), (0, 15, 16), (31, 0, 31), (10, 11, 12), (0, 0, 0)];

        for (rd, rs1, rs2) in reg_pairs {
            assert_instruction_roundtrip(Instruction::add { rd, rs1, rs2 });
            assert_instruction_roundtrip(Instruction::sub { rd, rs1, rs2 });
            assert_instruction_roundtrip(Instruction::sll { rd, rs1, rs2 });
            assert_instruction_roundtrip(Instruction::slt { rd, rs1, rs2 });
            assert_instruction_roundtrip(Instruction::sltu { rd, rs1, rs2 });
            assert_instruction_roundtrip(Instruction::xor { rd, rs1, rs2 });
            assert_instruction_roundtrip(Instruction::srl { rd, rs1, rs2 });
            assert_instruction_roundtrip(Instruction::sra { rd, rs1, rs2 });
            assert_instruction_roundtrip(Instruction::or { rd, rs1, rs2 });
            assert_instruction_roundtrip(Instruction::and { rd, rs1, rs2 });
        }
    }

    #[test]
    fn test_roundtrip_system() {
        assert_instruction_roundtrip(Instruction::fence);
        assert_instruction_roundtrip(Instruction::ecall);
        assert_instruction_roundtrip(Instruction::ebreak);
        assert_instruction_roundtrip(Instruction::unimp);
    }

    #[test]
    fn test_display() {
        extern crate alloc;
        use alloc::format;

        assert_eq!(
            format!(
                "{}",
                Instruction::auipc {
                    rd: 2,
                    imm: 2105344
                }
            ),
            "auipc x2, 2105344"
        );
        assert_eq!(
            format!(
                "{}",
                Instruction::addi {
                    rd: 2,
                    rs1: 2,
                    imm: -1152
                }
            ),
            "addi x2, x2, -1152"
        );
        assert_eq!(
            format!(
                "{}",
                Instruction::lw {
                    rd: 10,
                    rs1: 2,
                    imm: 4
                }
            ),
            "lw x10, 4(x2)"
        );
        assert_eq!(
            format!(
                "{}",
                Instruction::sw {
                    rs1: 2,
                    rs2: 10,
                    imm: 8
                }
            ),
            "sw x10, 8(x2)"
        );
        assert_eq!(
            format!(
                "{}",
                Instruction::jalr {
                    rd: 1,
                    rs1: 2,
                    imm: 0
                }
            ),
            "jalr x1, 0(x2)"
        );
        assert_eq!(
            format!("{}", Instruction::jal { rd: 1, imm: 16 }),
            "jal x1, 16"
        );
        assert_eq!(
            format!(
                "{}",
                Instruction::beq {
                    rs1: 1,
                    rs2: 2,
                    imm: -4
                }
            ),
            "beq x1, x2, -4"
        );
        assert_eq!(
            format!(
                "{}",
                Instruction::add {
                    rd: 1,
                    rs1: 2,
                    rs2: 3
                }
            ),
            "add x1, x2, x3"
        );
        assert_eq!(
            format!(
                "{}",
                Instruction::slli {
                    rd: 1,
                    rs1: 2,
                    shamt: 5
                }
            ),
            "slli x1, x2, 5"
        );
        assert_eq!(format!("{}", Instruction::fence), "fence");
        assert_eq!(format!("{}", Instruction::ecall), "ecall");
        assert_eq!(format!("{}", Instruction::ebreak), "ebreak");
        assert_eq!(format!("{}", Instruction::unimp), "unimp");
    }

    #[test]
    fn test_is_compressed() {
        assert!(is_compressed(0x00));
        assert!(is_compressed(0x01));
        assert!(is_compressed(0x02));
        assert!(!is_compressed(0x03)); // 0b11
        assert!(!is_compressed(0x13)); // 0b11 (addi opcode)
        assert!(!is_compressed(0x37)); // 0b11 (lui opcode)
        assert!(!is_compressed(0xef)); // 0b11 (jal opcode)
    }

    #[test]
    fn test_compressed_instructions_roundtrip() {
        let test_cases: &[(u16, Instruction)] = &[
            // Quadrant 0
            (
                0x0808,
                Instruction::addi {
                    rd: 10,
                    rs1: 2,
                    imm: 16,
                },
            ),
            (
                0x1fec,
                Instruction::addi {
                    rd: 11,
                    rs1: 2,
                    imm: 1020,
                },
            ),
            (
                0x4188,
                Instruction::lw {
                    rd: 10,
                    rs1: 11,
                    imm: 0,
                },
            ),
            (
                0x5ef0,
                Instruction::lw {
                    rd: 12,
                    rs1: 13,
                    imm: 124,
                },
            ),
            (
                0xc188,
                Instruction::sw {
                    rs1: 11,
                    rs2: 10,
                    imm: 0,
                },
            ),
            (
                0xdef0,
                Instruction::sw {
                    rs1: 13,
                    rs2: 12,
                    imm: 124,
                },
            ),
            // Quadrant 1
            (
                0x0001,
                Instruction::addi {
                    rd: 0,
                    rs1: 0,
                    imm: 0,
                },
            ), // C.NOP
            (
                0x0505,
                Instruction::addi {
                    rd: 10,
                    rs1: 10,
                    imm: 1,
                },
            ),
            (
                0x157d,
                Instruction::addi {
                    rd: 10,
                    rs1: 10,
                    imm: -1,
                },
            ),
            (0x2811, Instruction::jal { rd: 1, imm: 20 }), // C.JAL
            (
                0x4505,
                Instruction::addi {
                    rd: 10,
                    rs1: 0,
                    imm: 1,
                },
            ), // C.LI
            (
                0x557d,
                Instruction::addi {
                    rd: 10,
                    rs1: 0,
                    imm: -1,
                },
            ),
            (
                0x6141,
                Instruction::addi {
                    rd: 2,
                    rs1: 2,
                    imm: 16,
                },
            ), // C.ADDI16SP
            (
                0x717d,
                Instruction::addi {
                    rd: 2,
                    rs1: 2,
                    imm: -16,
                },
            ),
            (0x6505, Instruction::lui { rd: 10, imm: 4096 }), // C.LUI
            (0x757d, Instruction::lui { rd: 10, imm: -4096 }),
            (
                0x8105,
                Instruction::srli {
                    rd: 10,
                    rs1: 10,
                    shamt: 1,
                },
            ),
            (
                0x8505,
                Instruction::srai {
                    rd: 10,
                    rs1: 10,
                    shamt: 1,
                },
            ),
            (
                0x8905,
                Instruction::andi {
                    rd: 10,
                    rs1: 10,
                    imm: 1,
                },
            ),
            (
                0x997d,
                Instruction::andi {
                    rd: 10,
                    rs1: 10,
                    imm: -1,
                },
            ),
            (
                0x8d0d,
                Instruction::sub {
                    rd: 10,
                    rs1: 10,
                    rs2: 11,
                },
            ),
            (
                0x8d2d,
                Instruction::xor {
                    rd: 10,
                    rs1: 10,
                    rs2: 11,
                },
            ),
            (
                0x8d4d,
                Instruction::or {
                    rd: 10,
                    rs1: 10,
                    rs2: 11,
                },
            ),
            (
                0x8d6d,
                Instruction::and {
                    rd: 10,
                    rs1: 10,
                    rs2: 11,
                },
            ),
            (0xa811, Instruction::jal { rd: 0, imm: 20 }), // C.J
            (
                0xc911,
                Instruction::beq {
                    rs1: 10,
                    rs2: 0,
                    imm: 20,
                },
            ), // C.BEQZ
            (
                0xe911,
                Instruction::bne {
                    rs1: 10,
                    rs2: 0,
                    imm: 20,
                },
            ), // C.BNEZ
            // Quadrant 2
            (
                0x0506,
                Instruction::slli {
                    rd: 10,
                    rs1: 10,
                    shamt: 1,
                },
            ),
            (
                0x4502,
                Instruction::lw {
                    rd: 10,
                    rs1: 2,
                    imm: 0,
                },
            ),
            (
                0x557e,
                Instruction::lw {
                    rd: 10,
                    rs1: 2,
                    imm: 252,
                },
            ),
            (
                0x8502,
                Instruction::jalr {
                    rd: 0,
                    rs1: 10,
                    imm: 0,
                },
            ), // C.JR
            (
                0x852e,
                Instruction::add {
                    rd: 10,
                    rs1: 0,
                    rs2: 11,
                },
            ), // C.MV
            (0x9002, Instruction::ebreak), // C.EBREAK
            (
                0x9502,
                Instruction::jalr {
                    rd: 1,
                    rs1: 10,
                    imm: 0,
                },
            ), // C.JALR
            (
                0x952e,
                Instruction::add {
                    rd: 10,
                    rs1: 10,
                    rs2: 11,
                },
            ), // C.ADD
            (
                0xc02a,
                Instruction::sw {
                    rs1: 2,
                    rs2: 10,
                    imm: 0,
                },
            ),
            (
                0xdfaa,
                Instruction::sw {
                    rs1: 2,
                    rs2: 10,
                    imm: 252,
                },
            ),
        ];

        for &(raw16, expected_instr) in test_cases {
            assert!(
                is_compressed(raw16 as u8),
                "raw {:#06x} should be compressed",
                raw16
            );
            let expanded_instr = expand_compressed_to_instruction(raw16);
            assert_eq!(
                expanded_instr,
                Some(expected_instr),
                "Failed expanding raw {:#06x}",
                raw16
            );

            let raw32 = expand_compressed(raw16).expect("expand_compressed failed");
            let parsed = parse(raw32).expect("parse expanded raw32 failed");
            assert_eq!(
                parsed, expected_instr,
                "Roundtrip parse failed for {:#06x} -> {:#010x}",
                raw16, raw32
            );
        }
    }

    #[test]
    fn test_compressed_reserved_and_illegal() {
        // 0x0000 is reserved / illegal instruction
        assert_eq!(expand_compressed(0x0000), None);

        // c.addi16sp with imm == 0 (nzimm == 0 is reserved)
        assert_eq!(expand_compressed(0x6101), None);

        // c.lui with imm == 0 (nzimm == 0 is reserved)
        assert_eq!(expand_compressed(0x6501), None);

        // c.lui with rd == 0 is reserved
        assert_eq!(expand_compressed(0x6005), None);

        // c.li with rd == 0 is reserved
        assert_eq!(expand_compressed(0x4005), None);

        // c.srli with shamt[5] == 1 is reserved in RV32
        assert_eq!(expand_compressed(0x9105), None);

        // c.srai with shamt[5] == 1 is reserved in RV32
        assert_eq!(expand_compressed(0x9505), None);

        // c.slli with shamt[5] == 1 is reserved in RV32
        assert_eq!(expand_compressed(0x1506), None);

        // c.slli with rd == 0 is reserved
        assert_eq!(expand_compressed(0x0006), None);

        // c.lwsp with rd == 0 is reserved
        assert_eq!(expand_compressed(0x4002), None);

        // c.jr with rs1 == 0 is reserved
        assert_eq!(expand_compressed(0x8002), None);

        // c.mv with rd == 0 is reserved
        assert_eq!(expand_compressed(0x802e), None);

        // c.add with rd == 0 is reserved
        assert_eq!(expand_compressed(0x902e), None);
    }
}
