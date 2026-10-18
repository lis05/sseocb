//! RISC-V RV32I opcodes, funct3, funct7, masks, shifts, and system constants.

pub mod opcode {
    pub const LUI: u32 = 0b0110111;
    pub const AUIPC: u32 = 0b0010111;
    pub const JAL: u32 = 0b1101111;
    pub const JALR: u32 = 0b1100111;
    pub const BRANCH: u32 = 0b1100011;
    pub const LOAD: u32 = 0b0000011;
    pub const STORE: u32 = 0b0100011;
    pub const OP_IMM: u32 = 0b0010011;
    pub const OP: u32 = 0b0110011;
    pub const FENCE: u32 = 0b0001111;
    pub const SYSTEM: u32 = 0b1110011;
}

pub use opcode as opcodes;

pub mod funct3 {
    // Conditional Branches (B-type)
    pub const BEQ: u32 = 0b000;
    pub const BNE: u32 = 0b001;
    pub const BLT: u32 = 0b100;
    pub const BGE: u32 = 0b101;
    pub const BLTU: u32 = 0b110;
    pub const BGEU: u32 = 0b111;

    // Loads (I-type)
    pub const LB: u32 = 0b000;
    pub const LH: u32 = 0b001;
    pub const LW: u32 = 0b010;
    pub const LBU: u32 = 0b100;
    pub const LHU: u32 = 0b101;

    // Stores (S-type)
    pub const SB: u32 = 0b000;
    pub const SH: u32 = 0b001;
    pub const SW: u32 = 0b010;

    // Immediate Arithmetic (I-type)
    pub const ADDI: u32 = 0b000;
    pub const SLTI: u32 = 0b010;
    pub const SLTIU: u32 = 0b011;
    pub const XORI: u32 = 0b100;
    pub const ORI: u32 = 0b110;
    pub const ANDI: u32 = 0b111;
    pub const SLLI: u32 = 0b001;
    pub const SRLI: u32 = 0b101;
    pub const SRAI: u32 = 0b101;
    pub const SRLI_SRAI: u32 = 0b101;

    // Register Arithmetic (R-type)
    pub const ADD: u32 = 0b000;
    pub const SUB: u32 = 0b000;
    pub const ADD_SUB: u32 = 0b000;
    pub const SLL: u32 = 0b001;
    pub const SLT: u32 = 0b010;
    pub const SLTU: u32 = 0b011;
    pub const XOR: u32 = 0b100;
    pub const SRL: u32 = 0b101;
    pub const SRA: u32 = 0b101;
    pub const SRL_SRA: u32 = 0b101;
    pub const OR: u32 = 0b110;
    pub const AND: u32 = 0b111;

    // JALR
    pub const JALR: u32 = 0b000;

    // FENCE
    pub const FENCE: u32 = 0b000;
}

pub mod funct7 {
    pub const BASE: u32 = 0x00;
    pub const ALT: u32 = 0x20;
}

pub use funct7 as funct7s;

pub mod system {
    pub const ECALL: u32 = 0x00000073;
    pub const EBREAK: u32 = 0x00100073;
    pub const UNIMP: u32 = 0xc0001073;
}

pub mod fence {
    pub const FENCE: u32 = 0x0000000F;
}

pub mod mask {
    pub const OPCODE: u32 = 0x7F;
    pub const FUNCT3: u32 = 0x7;
    pub const FUNCT7: u32 = 0x7F;
    pub const RD: u32 = 0x1F;
    pub const RS1: u32 = 0x1F;
    pub const RS2: u32 = 0x1F;
    pub const SHAMT: u32 = 0x1F;
}

pub mod shift {
    pub const OPCODE: u32 = 0;
    pub const RD: u32 = 7;
    pub const FUNCT3: u32 = 12;
    pub const RS1: u32 = 15;
    pub const RS2: u32 = 20;
    pub const SHAMT: u32 = 20;
    pub const FUNCT7: u32 = 25;
}
