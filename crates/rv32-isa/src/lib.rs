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
}

pub fn parse(raw: u32) -> Option<Instruction> {
    let opcode = raw & 0x7F;
    let funct3 = (raw >> 12) & 0x7;
    let funct7 = (raw >> 25) & 0x7F;
    let rd = ((raw >> 7) & 0x1F) as usize;
    let rs1 = ((raw >> 15) & 0x1F) as usize;
    let rs2 = ((raw >> 20) & 0x1F) as usize;
    let shamt = (raw >> 20) & 0x1F;
    // https://hoult.org/rv32i.pdf (page 14)

    match opcode {
        // LUI (U-type)
        0b0110111 => {
            let imm = (raw & !0xFFF) as i32;
            Some(Instruction::lui { rd, imm })
        }

        // AUIPC (U-type)
        0b0010111 => {
            let imm = (raw & !0xFFF) as i32;
            Some(Instruction::auipc { rd, imm })
        }

        // JAL (J-type)
        // imm[20] = raw[31], imm[19:12] = raw[19:12], imm[11] = raw[20], imm[10:1] = raw[30:21], imm[0] = 0
        0b1101111 => {
            let imm = (((raw as i32) >> 31) << 20)
                | (((raw >> 12) & 0xFF) << 12) as i32
                | (((raw >> 20) & 0x1) << 11) as i32
                | (((raw >> 21) & 0x3FF) << 1) as i32;
            Some(Instruction::jal { rd, imm })
        }

        // JALR (I-type)
        0b1100111 => {
            if funct3 != 0 {
                return None;
            }
            let imm = (raw as i32) >> 20;
            Some(Instruction::jalr { rd, rs1, imm })
        }

        // B-type
        // imm[12] = raw[31], imm[11] = raw[7], imm[10:5] = raw[30:25], imm[4:1] = raw[11:8], imm[0] = 0
        0b1100011 => {
            let imm = (((raw as i32) >> 31) << 12)
                | (((raw >> 7) & 0x1) << 11) as i32
                | (((raw >> 25) & 0x3F) << 5) as i32
                | (((raw >> 8) & 0xF) << 1) as i32;
            match funct3 {
                0b000 => Some(Instruction::beq { rs1, rs2, imm }),
                0b001 => Some(Instruction::bne { rs1, rs2, imm }),
                0b100 => Some(Instruction::blt { rs1, rs2, imm }),
                0b101 => Some(Instruction::bge { rs1, rs2, imm }),
                0b110 => Some(Instruction::bltu { rs1, rs2, imm }),
                0b111 => Some(Instruction::bgeu { rs1, rs2, imm }),
                _ => None,
            }
        }

        // Loads (I-type)
        0b0000011 => {
            let imm = (raw as i32) >> 20;
            match funct3 {
                0b000 => Some(Instruction::lb { rd, rs1, imm }),
                0b001 => Some(Instruction::lh { rd, rs1, imm }),
                0b010 => Some(Instruction::lw { rd, rs1, imm }),
                0b100 => Some(Instruction::lbu { rd, rs1, imm }),
                0b101 => Some(Instruction::lhu { rd, rs1, imm }),
                _ => None,
            }
        }

        // Stores (S-type)
        // imm[11:5] = raw[31:25], imm[4:0] = raw[11:7]
        0b0100011 => {
            let imm = ((raw as i32 >> 25) << 5) | (((raw >> 7) & 0x1F) as i32);
            match funct3 {
                0b000 => Some(Instruction::sb { rs1, rs2, imm }),
                0b001 => Some(Instruction::sh { rs1, rs2, imm }),
                0b010 => Some(Instruction::sw { rs1, rs2, imm }),
                _ => None,
            }
        }

        // I-type ALU & shifts
        0b0010011 => {
            let imm = (raw as i32) >> 20;
            match funct3 {
                0b000 => Some(Instruction::addi { rd, rs1, imm }),
                0b010 => Some(Instruction::slti { rd, rs1, imm }),
                0b011 => Some(Instruction::sltiu { rd, rs1, imm }),
                0b100 => Some(Instruction::xori { rd, rs1, imm }),
                0b110 => Some(Instruction::ori { rd, rs1, imm }),
                0b111 => Some(Instruction::andi { rd, rs1, imm }),
                // Shift amount is lower 5 bits (shamt), funct7 must be 0x00
                0b001 if funct7 == 0x00 => Some(Instruction::slli { rd, rs1, shamt }),
                // SRLI funct7 == 0x00, SRAI funct7 == 0x20 (bit 30 set for arithmetic shift)
                0b101 if funct7 == 0x00 => Some(Instruction::srli { rd, rs1, shamt }),
                0b101 if funct7 == 0x20 => Some(Instruction::srai { rd, rs1, shamt }),
                _ => None,
            }
        }

        // R-type
        // Bit 30 differentiates add/sub and srl/sra
        0b0110011 => match (funct3, funct7) {
            (0b000, 0x00) => Some(Instruction::add { rd, rs1, rs2 }),
            (0b000, 0x20) => Some(Instruction::sub { rd, rs1, rs2 }),
            (0b001, 0x00) => Some(Instruction::sll { rd, rs1, rs2 }),
            (0b010, 0x00) => Some(Instruction::slt { rd, rs1, rs2 }),
            (0b011, 0x00) => Some(Instruction::sltu { rd, rs1, rs2 }),
            (0b100, 0x00) => Some(Instruction::xor { rd, rs1, rs2 }),
            (0b101, 0x00) => Some(Instruction::srl { rd, rs1, rs2 }),
            (0b101, 0x20) => Some(Instruction::sra { rd, rs1, rs2 }),
            (0b110, 0x00) => Some(Instruction::or { rd, rs1, rs2 }),
            (0b111, 0x00) => Some(Instruction::and { rd, rs1, rs2 }),
            _ => None,
        },

        // FENCE
        0b0001111 => {
            if funct3 == 0b000 && rd == 0 && rs1 == 0 {
                Some(Instruction::fence)
            } else {
                None
            }
        }

        // ECALL / EBREAK
        0b1110011 => match raw {
            0x00000073 => Some(Instruction::ecall),
            0x00100073 => Some(Instruction::ebreak),
            _ => None,
        },

        _ => None,
    }
}

impl Instruction {
    pub fn encode(&self) -> u32 {
        match *self {
            // Upper Immediate (U-type)
            Instruction::lui { rd, imm } => encode_u(0b0110111, rd, imm),
            Instruction::auipc { rd, imm } => encode_u(0b0010111, rd, imm),

            // Unconditional Jumps
            Instruction::jal { rd, imm } => encode_j(0b1101111, rd, imm),
            Instruction::jalr { rd, rs1, imm } => encode_i(0b1100111, rd, 0b000, rs1, imm),

            // Conditional Branches (B-type)
            Instruction::beq { rs1, rs2, imm } => encode_b(0b1100011, 0b000, rs1, rs2, imm),
            Instruction::bne { rs1, rs2, imm } => encode_b(0b1100011, 0b001, rs1, rs2, imm),
            Instruction::blt { rs1, rs2, imm } => encode_b(0b1100011, 0b100, rs1, rs2, imm),
            Instruction::bge { rs1, rs2, imm } => encode_b(0b1100011, 0b101, rs1, rs2, imm),
            Instruction::bltu { rs1, rs2, imm } => encode_b(0b1100011, 0b110, rs1, rs2, imm),
            Instruction::bgeu { rs1, rs2, imm } => encode_b(0b1100011, 0b111, rs1, rs2, imm),

            // Loads (I-type)
            Instruction::lb { rd, rs1, imm } => encode_i(0b0000011, rd, 0b000, rs1, imm),
            Instruction::lh { rd, rs1, imm } => encode_i(0b0000011, rd, 0b001, rs1, imm),
            Instruction::lw { rd, rs1, imm } => encode_i(0b0000011, rd, 0b010, rs1, imm),
            Instruction::lbu { rd, rs1, imm } => encode_i(0b0000011, rd, 0b100, rs1, imm),
            Instruction::lhu { rd, rs1, imm } => encode_i(0b0000011, rd, 0b101, rs1, imm),

            // Stores (S-type)
            Instruction::sb { rs1, rs2, imm } => encode_s(0b0100011, 0b000, rs1, rs2, imm),
            Instruction::sh { rs1, rs2, imm } => encode_s(0b0100011, 0b001, rs1, rs2, imm),
            Instruction::sw { rs1, rs2, imm } => encode_s(0b0100011, 0b010, rs1, rs2, imm),

            // Immediate Arithmetic (I-type)
            Instruction::addi { rd, rs1, imm } => encode_i(0b0010011, rd, 0b000, rs1, imm),
            Instruction::slti { rd, rs1, imm } => encode_i(0b0010011, rd, 0b010, rs1, imm),
            Instruction::sltiu { rd, rs1, imm } => encode_i(0b0010011, rd, 0b011, rs1, imm),
            Instruction::xori { rd, rs1, imm } => encode_i(0b0010011, rd, 0b100, rs1, imm),
            Instruction::ori { rd, rs1, imm } => encode_i(0b0010011, rd, 0b110, rs1, imm),
            Instruction::andi { rd, rs1, imm } => encode_i(0b0010011, rd, 0b111, rs1, imm),
            Instruction::slli { rd, rs1, shamt } => {
                encode_i_shift(0b0010011, rd, 0b001, rs1, shamt, 0x00)
            }
            Instruction::srli { rd, rs1, shamt } => {
                encode_i_shift(0b0010011, rd, 0b101, rs1, shamt, 0x00)
            }
            Instruction::srai { rd, rs1, shamt } => {
                encode_i_shift(0b0010011, rd, 0b101, rs1, shamt, 0x20)
            }

            // Register Arithmetic (R-type)
            Instruction::add { rd, rs1, rs2 } => encode_r(0b0110011, rd, 0b000, rs1, rs2, 0x00),
            Instruction::sub { rd, rs1, rs2 } => encode_r(0b0110011, rd, 0b000, rs1, rs2, 0x20),
            Instruction::sll { rd, rs1, rs2 } => encode_r(0b0110011, rd, 0b001, rs1, rs2, 0x00),
            Instruction::slt { rd, rs1, rs2 } => encode_r(0b0110011, rd, 0b010, rs1, rs2, 0x00),
            Instruction::sltu { rd, rs1, rs2 } => encode_r(0b0110011, rd, 0b011, rs1, rs2, 0x00),
            Instruction::xor { rd, rs1, rs2 } => encode_r(0b0110011, rd, 0b100, rs1, rs2, 0x00),
            Instruction::srl { rd, rs1, rs2 } => encode_r(0b0110011, rd, 0b101, rs1, rs2, 0x00),
            Instruction::sra { rd, rs1, rs2 } => encode_r(0b0110011, rd, 0b101, rs1, rs2, 0x20),
            Instruction::or { rd, rs1, rs2 } => encode_r(0b0110011, rd, 0b110, rs1, rs2, 0x00),
            Instruction::and { rd, rs1, rs2 } => encode_r(0b0110011, rd, 0b111, rs1, rs2, 0x00),

            // System & Sync
            Instruction::fence => 0x0000000F,
            Instruction::ecall => 0x00000073,
            Instruction::ebreak => 0x00100073,
        }
    }
}

#[inline(always)]
fn encode_r(opcode: u32, rd: usize, funct3: u32, rs1: usize, rs2: usize, funct7: u32) -> u32 {
    opcode
        | (((rd as u32) & 0x1F) << 7)
        | ((funct3 & 0x7) << 12)
        | (((rs1 as u32) & 0x1F) << 15)
        | (((rs2 as u32) & 0x1F) << 20)
        | ((funct7 & 0x7F) << 25)
}

#[inline(always)]
fn encode_i(opcode: u32, rd: usize, funct3: u32, rs1: usize, imm: i32) -> u32 {
    opcode
        | (((rd as u32) & 0x1F) << 7)
        | ((funct3 & 0x7) << 12)
        | (((rs1 as u32) & 0x1F) << 15)
        | (((imm as u32) & 0xFFF) << 20)
}

#[inline(always)]
fn encode_i_shift(opcode: u32, rd: usize, funct3: u32, rs1: usize, shamt: u32, funct7: u32) -> u32 {
    opcode
        | (((rd as u32) & 0x1F) << 7)
        | ((funct3 & 0x7) << 12)
        | (((rs1 as u32) & 0x1F) << 15)
        | ((shamt & 0x1F) << 20)
        | ((funct7 & 0x7F) << 25)
}

#[inline(always)]
fn encode_s(opcode: u32, funct3: u32, rs1: usize, rs2: usize, imm: i32) -> u32 {
    let imm_u = imm as u32;
    opcode
        | ((imm_u & 0x1F) << 7)
        | ((funct3 & 0x7) << 12)
        | (((rs1 as u32) & 0x1F) << 15)
        | (((rs2 as u32) & 0x1F) << 20)
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
        | ((funct3 & 0x7) << 12)
        | (((rs1 as u32) & 0x1F) << 15)
        | (((rs2 as u32) & 0x1F) << 20)
        | (b_10_5 << 25)
        | (b_12 << 31)
}

#[inline(always)]
fn encode_u(opcode: u32, rd: usize, imm: i32) -> u32 {
    opcode | (((rd as u32) & 0x1F) << 7) | ((imm as u32) & !0xFFF)
}

#[inline(always)]
fn encode_j(opcode: u32, rd: usize, imm: i32) -> u32 {
    let imm_u = imm as u32;
    let j_20 = (imm_u >> 20) & 1;
    let j_19_12 = (imm_u >> 12) & 0xFF;
    let j_11 = (imm_u >> 11) & 1;
    let j_10_1 = (imm_u >> 1) & 0x3FF;
    opcode
        | (((rd as u32) & 0x1F) << 7)
        | (j_19_12 << 12)
        | (j_11 << 20)
        | (j_10_1 << 21)
        | (j_20 << 31)
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
    }
}
