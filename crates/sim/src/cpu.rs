use crate::instruction;
use crate::instruction::Instruction;
use crate::memory::{Permissions, bus};

pub type Register = u32;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Error {
    Bus(bus::Error),
    IllegalInstruction,
    Ecall,
    Ebreak,
}

pub type Result<T> = std::result::Result<T, Error>;

impl From<bus::Error> for Error {
    fn from(err: bus::Error) -> Self {
        Error::Bus(err)
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct CPU {
    pc: Register,
    registers: [Register; 32],
}

impl CPU {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_pc(&self) -> Register {
        self.pc
    }

    pub fn set_pc(&mut self, pc: Register) {
        self.pc = pc;
    }

    pub fn get_registers(&self) -> [Register; 32] {
        self.registers
    }

    pub fn read_reg(&self, reg: usize) -> Register {
        assert!(reg < self.registers.len());
        if reg == 0 { 0 } else { self.registers[reg] }
    }

    pub fn write_reg(&mut self, reg: usize, val: Register) {
        assert!(reg < self.registers.len());
        if reg != 0 {
            self.registers[reg] = val;
        }
    }

    pub fn fetch(&self, bus: &bus::Bus) -> Result<Instruction> {
        if self.pc & 0x3 != 0 {
            return Err(Error::IllegalInstruction);
        }
        let raw = bus.read_u32(self.pc, Permissions::RX)?;
        let instr = instruction::parse(raw).ok_or(Error::IllegalInstruction)?;
        Ok(instr)
    }

    pub fn step(&mut self, bus: &mut bus::Bus) -> Result<Instruction> {
        let instr = self.fetch(bus)?;
        self.execute(instr, bus)?;
        Ok(instr)
    }

    pub fn execute(&mut self, instr: Instruction, bus: &mut bus::Bus) -> Result<()> {
        match instr {
            Instruction::lui { rd, imm } => {
                self.write_reg(rd, imm as Register);
            }
            Instruction::auipc { rd, imm } => {
                self.write_reg(rd, self.pc.wrapping_add(imm as Register));
            }
            Instruction::jal { .. } | Instruction::jalr { .. } => {
                let (rd, target) = match instr {
                    Instruction::jal { rd, imm } => (rd, self.pc.wrapping_add_signed(imm)),
                    Instruction::jalr { rd, rs1, imm } => {
                        (rd, self.read_reg(rs1).wrapping_add_signed(imm) & !0x1)
                    }
                    _ => unreachable!(),
                };
                if target & 0x3 != 0 {
                    return Err(Error::IllegalInstruction);
                }
                self.write_reg(rd, self.pc.wrapping_add(4));
                self.pc = target;
                return Ok(());
            }
            Instruction::beq { rs1, rs2, imm }
            | Instruction::bne { rs1, rs2, imm }
            | Instruction::blt { rs1, rs2, imm }
            | Instruction::bge { rs1, rs2, imm }
            | Instruction::bltu { rs1, rs2, imm }
            | Instruction::bgeu { rs1, rs2, imm } => {
                let r1 = self.read_reg(rs1);
                let r2 = self.read_reg(rs2);
                let taken = match instr {
                    Instruction::beq { .. } => r1 == r2,
                    Instruction::bne { .. } => r1 != r2,
                    Instruction::blt { .. } => (r1 as i32) < (r2 as i32),
                    Instruction::bge { .. } => (r1 as i32) >= (r2 as i32),
                    Instruction::bltu { .. } => r1 < r2,
                    Instruction::bgeu { .. } => r1 >= r2,
                    _ => unreachable!(),
                };

                if taken {
                    let target = self.pc.wrapping_add_signed(imm);
                    if target & 0x3 != 0 {
                        return Err(Error::IllegalInstruction);
                    }
                    self.pc = target;
                    return Ok(());
                }
            }
            Instruction::lb { rd, rs1, imm }
            | Instruction::lh { rd, rs1, imm }
            | Instruction::lw { rd, rs1, imm }
            | Instruction::lbu { rd, rs1, imm }
            | Instruction::lhu { rd, rs1, imm } => {
                let addr = self.read_reg(rs1).wrapping_add_signed(imm);
                let value = match instr {
                    Instruction::lb { .. } => bus.read_u8(addr, Permissions::RO)? as i8 as u32,
                    Instruction::lh { .. } => bus.read_u16(addr, Permissions::RO)? as i16 as u32,
                    Instruction::lw { .. } => bus.read_u32(addr, Permissions::RO)?,
                    Instruction::lbu { .. } => bus.read_u8(addr, Permissions::RO)? as u32,
                    Instruction::lhu { .. } => bus.read_u16(addr, Permissions::RO)? as u32,
                    _ => unreachable!(),
                };
                self.write_reg(rd, value);
            }
            Instruction::sb { rs1, rs2, imm }
            | Instruction::sh { rs1, rs2, imm }
            | Instruction::sw { rs1, rs2, imm } => {
                let addr = self.read_reg(rs1).wrapping_add_signed(imm);
                let val = self.read_reg(rs2);
                match instr {
                    Instruction::sb { .. } => bus.write_u8(addr, val as u8)?,
                    Instruction::sh { .. } => bus.write_u16(addr, val as u16)?,
                    Instruction::sw { .. } => bus.write_u32(addr, val)?,
                    _ => unreachable!(),
                }
            }
            Instruction::addi { rd, rs1, imm } => {
                self.write_reg(rd, self.read_reg(rs1).wrapping_add_signed(imm));
            }
            Instruction::slti { rd, rs1, imm } => {
                self.write_reg(rd, u32::from((self.read_reg(rs1) as i32) < imm));
            }
            Instruction::sltiu { rd, rs1, imm } => {
                self.write_reg(rd, u32::from(self.read_reg(rs1) < (imm as u32)));
            }
            Instruction::xori { rd, rs1, imm } => {
                self.write_reg(rd, self.read_reg(rs1) ^ (imm as u32));
            }
            Instruction::ori { rd, rs1, imm } => {
                self.write_reg(rd, self.read_reg(rs1) | (imm as u32));
            }
            Instruction::andi { rd, rs1, imm } => {
                self.write_reg(rd, self.read_reg(rs1) & (imm as u32));
            }
            Instruction::slli { rd, rs1, shamt } => {
                self.write_reg(rd, self.read_reg(rs1) << shamt);
            }
            Instruction::srli { rd, rs1, shamt } => {
                self.write_reg(rd, self.read_reg(rs1) >> shamt);
            }
            Instruction::srai { rd, rs1, shamt } => {
                self.write_reg(rd, ((self.read_reg(rs1) as i32) >> shamt) as u32);
            }
            Instruction::add { rd, rs1, rs2 } => {
                self.write_reg(rd, self.read_reg(rs1).wrapping_add(self.read_reg(rs2)));
            }
            Instruction::sub { rd, rs1, rs2 } => {
                self.write_reg(rd, self.read_reg(rs1).wrapping_sub(self.read_reg(rs2)));
            }
            Instruction::slt { rd, rs1, rs2 } => {
                self.write_reg(
                    rd,
                    u32::from((self.read_reg(rs1) as i32) < self.read_reg(rs2) as i32),
                );
            }
            Instruction::sltu { rd, rs1, rs2 } => {
                self.write_reg(rd, u32::from(self.read_reg(rs1) < self.read_reg(rs2)));
            }
            Instruction::xor { rd, rs1, rs2 } => {
                self.write_reg(rd, self.read_reg(rs1) ^ self.read_reg(rs2));
            }
            Instruction::or { rd, rs1, rs2 } => {
                self.write_reg(rd, self.read_reg(rs1) | self.read_reg(rs2));
            }
            Instruction::and { rd, rs1, rs2 } => {
                self.write_reg(rd, self.read_reg(rs1) & self.read_reg(rs2));
            }
            Instruction::sll { rd, rs1, rs2 } => {
                self.write_reg(rd, self.read_reg(rs1) << (self.read_reg(rs2) & 0x1F));
            }
            Instruction::srl { rd, rs1, rs2 } => {
                self.write_reg(rd, self.read_reg(rs1) >> (self.read_reg(rs2) & 0x1F));
            }
            Instruction::sra { rd, rs1, rs2 } => {
                self.write_reg(
                    rd,
                    ((self.read_reg(rs1) as i32) >> (self.read_reg(rs2) & 0x1F)) as u32,
                );
            }
            Instruction::fence => {}
            Instruction::ecall => return Err(Error::Ecall),
            Instruction::ebreak => return Err(Error::Ebreak),
        }
        self.pc = self.pc.wrapping_add(4);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::region::Region;

    #[test]
    fn test_cpu_default_state() {
        let cpu = CPU::new();
        assert_eq!(cpu.get_pc(), 0);
        assert_eq!(cpu.get_registers(), [0; 32]);
    }

    #[test]
    fn test_cpu_fetch_success() {
        let mut bus = bus::Bus::new();
        // addi x1, x0, 42 -> 0x02a00093
        let code = 0x02a00093u32.to_le_bytes().to_vec();
        let region = Region::new("ROM", 0x1000, Permissions::RX, code);
        bus.add(region).unwrap();

        let mut cpu = CPU::new();
        cpu.pc = 0x1000;

        let instr = cpu.fetch(&bus).expect("fetch should succeed");
        assert_eq!(
            instr,
            Instruction::addi {
                rd: 1,
                rs1: 0,
                imm: 42
            }
        );
    }

    #[test]
    fn test_cpu_step() {
        let mut bus = bus::Bus::new();
        // addi x1, x0, 42 -> 0x02a00093
        let code = 0x02a00093u32.to_le_bytes().to_vec();
        let region = Region::new("ROM", 0x1000, Permissions::RX, code);
        bus.add(region).unwrap();

        let mut cpu = CPU::new();
        cpu.set_pc(0x1000);

        let instr = cpu.step(&mut bus).expect("step should succeed");
        assert_eq!(
            instr,
            Instruction::addi {
                rd: 1,
                rs1: 0,
                imm: 42
            }
        );
        assert_eq!(cpu.read_reg(1), 42);
        assert_eq!(cpu.get_pc(), 0x1004);
    }

    #[test]
    fn test_cpu_fetch_illegal_instruction() {
        let mut bus = bus::Bus::new();
        let region = Region::new("ROM", 0x1000, Permissions::RX, vec![0; 4]);
        bus.add(region).unwrap();

        let mut cpu = CPU::new();
        cpu.pc = 0x1000;

        assert_eq!(cpu.fetch(&bus), Err(Error::IllegalInstruction));
    }

    #[test]
    fn test_cpu_fetch_bus_error() {
        let bus = bus::Bus::new();
        let cpu = CPU::new();
        assert_eq!(
            cpu.fetch(&bus),
            Err(Error::Bus(bus::Error::AddressViolation))
        );
    }

    #[test]
    fn test_cpu_fetch_unaligned() {
        let mut bus = bus::Bus::new();
        let code = vec![0; 16];
        bus.add(Region::new("ROM", 0x1000, Permissions::RX, code))
            .unwrap();

        let mut cpu = CPU::new();
        cpu.pc = 0x1001;
        assert_eq!(cpu.fetch(&bus), Err(Error::IllegalInstruction));

        cpu.pc = 0x1002;
        assert_eq!(cpu.fetch(&bus), Err(Error::IllegalInstruction));

        cpu.pc = 0x1003;
        assert_eq!(cpu.fetch(&bus), Err(Error::IllegalInstruction));
    }

    #[test]
    fn test_cpu_fetch_permissions_violation() {
        let mut bus = bus::Bus::new();
        // RW-only memory (no Execute permission)
        let data = vec![0; 16];
        bus.add(Region::new("RAM", 0x2000, Permissions::RW, data))
            .unwrap();

        let mut cpu = CPU::new();
        cpu.pc = 0x2000;
        assert_eq!(
            cpu.fetch(&bus),
            Err(Error::Bus(bus::Error::PermissionsViolation))
        );
    }

    #[test]
    fn test_cpu_execute_lui() {
        let mut bus = bus::Bus::new();
        let mut cpu = CPU::new();
        let instr = Instruction::lui {
            rd: 1,
            imm: 0x12345000,
        };

        cpu.execute(instr, &mut bus).unwrap();
        assert_eq!(cpu.read_reg(1), 0x12345000);
        assert_eq!(cpu.get_pc(), 4);
    }

    #[test]
    fn test_cpu_execute_lui_x0_ignored() {
        let mut bus = bus::Bus::new();
        let mut cpu = CPU::new();
        let instr = Instruction::lui {
            rd: 0,
            imm: 0x12345000,
        };

        cpu.execute(instr, &mut bus).unwrap();
        assert_eq!(cpu.read_reg(0), 0);
        assert_eq!(cpu.get_pc(), 4);
    }

    #[test]
    fn test_cpu_execute_auipc() {
        let mut bus = bus::Bus::new();
        let mut cpu = CPU::new();
        cpu.pc = 0x1000;
        let instr = Instruction::auipc { rd: 2, imm: 0x2000 };

        cpu.execute(instr, &mut bus).unwrap();
        assert_eq!(cpu.read_reg(2), 0x3000);
        assert_eq!(cpu.get_pc(), 0x1004);
    }

    #[test]
    fn test_cpu_execute_auipc_wrapping_and_x0() {
        let mut bus = bus::Bus::new();
        let mut cpu = CPU::new();
        cpu.pc = 0x1000;

        // Negative offset wrapping: 0x1000 + (-0x1000) = 0
        let instr_neg = Instruction::auipc {
            rd: 3,
            imm: -0x1000,
        };
        cpu.execute(instr_neg, &mut bus).unwrap();
        assert_eq!(cpu.read_reg(3), 0);
        assert_eq!(cpu.get_pc(), 0x1004);

        // Writing to x0 must be ignored
        let instr_x0 = Instruction::auipc { rd: 0, imm: 0x5000 };
        cpu.execute(instr_x0, &mut bus).unwrap();
        assert_eq!(cpu.read_reg(0), 0);
        assert_eq!(cpu.get_pc(), 0x1008);
    }

    #[test]
    fn test_cpu_execute_jal() {
        let mut bus = bus::Bus::new();
        let mut cpu = CPU::new();
        cpu.pc = 0x1000;

        // jal x1, +16
        let instr = Instruction::jal { rd: 1, imm: 16 };
        cpu.execute(instr, &mut bus).unwrap();

        assert_eq!(cpu.get_pc(), 0x1010);
        assert_eq!(cpu.read_reg(1), 0x1004); // Return address pc + 4
    }

    #[test]
    fn test_cpu_execute_jal_unaligned() {
        let mut bus = bus::Bus::new();
        let mut cpu = CPU::new();
        cpu.pc = 0x1000;

        // Misaligned offset (not multiple of 4)
        let instr = Instruction::jal { rd: 1, imm: 2 };
        assert_eq!(cpu.execute(instr, &mut bus), Err(Error::IllegalInstruction));
    }

    #[test]
    fn test_cpu_execute_jalr() {
        let mut bus = bus::Bus::new();
        let mut cpu = CPU::new();
        cpu.pc = 0x1000;
        cpu.write_reg(2, 0x2000);

        // jalr x1, x2, 8 -> target = (0x2000 + 8) & !1 = 0x2008
        let instr = Instruction::jalr {
            rd: 1,
            rs1: 2,
            imm: 8,
        };
        cpu.execute(instr, &mut bus).unwrap();

        assert_eq!(cpu.get_pc(), 0x2008);
        assert_eq!(cpu.read_reg(1), 0x1004); // Return address pc + 4
    }

    #[test]
    fn test_cpu_execute_jalr_clears_lsb_and_x0() {
        let mut bus = bus::Bus::new();
        let mut cpu = CPU::new();
        cpu.pc = 0x1000;
        // Base address has bit 0 set (odd)
        cpu.write_reg(2, 0x2001);

        // imm: 3 -> (0x2001 + 3) = 0x2004 & !1 = 0x2004
        let instr = Instruction::jalr {
            rd: 0,
            rs1: 2,
            imm: 3,
        };
        cpu.execute(instr, &mut bus).unwrap();

        assert_eq!(cpu.get_pc(), 0x2004);
        assert_eq!(cpu.read_reg(0), 0); // x0 remains 0
    }

    #[test]
    fn test_cpu_execute_jalr_unaligned() {
        let mut bus = bus::Bus::new();
        let mut cpu = CPU::new();
        cpu.pc = 0x1000;
        cpu.write_reg(2, 0x2000);

        // Target: (0x2000 + 2) & !1 = 0x2002 (not 4-byte aligned)
        let instr = Instruction::jalr {
            rd: 1,
            rs1: 2,
            imm: 2,
        };
        assert_eq!(cpu.execute(instr, &mut bus), Err(Error::IllegalInstruction));
    }

    #[test]
    fn test_cpu_execute_beq_bne() {
        let mut bus = bus::Bus::new();
        let mut cpu = CPU::new();
        cpu.write_reg(1, 42);
        cpu.write_reg(2, 42);
        cpu.write_reg(3, 99);

        // beq taken: x1 == x2 (42 == 42)
        cpu.pc = 0x1000;
        let beq_taken = Instruction::beq {
            rs1: 1,
            rs2: 2,
            imm: 16,
        };
        cpu.execute(beq_taken, &mut bus).unwrap();
        assert_eq!(cpu.get_pc(), 0x1010);

        // beq not taken: x1 == x3 (42 == 99)
        cpu.pc = 0x1000;
        let beq_not_taken = Instruction::beq {
            rs1: 1,
            rs2: 3,
            imm: 16,
        };
        cpu.execute(beq_not_taken, &mut bus).unwrap();
        assert_eq!(cpu.get_pc(), 0x1004);

        // bne taken: x1 != x3 (42 != 99)
        cpu.pc = 0x1000;
        let bne_taken = Instruction::bne {
            rs1: 1,
            rs2: 3,
            imm: 16,
        };
        cpu.execute(bne_taken, &mut bus).unwrap();
        assert_eq!(cpu.get_pc(), 0x1010);

        // bne not taken: x1 != x2 (42 != 42)
        cpu.pc = 0x1000;
        let bne_not_taken = Instruction::bne {
            rs1: 1,
            rs2: 2,
            imm: 16,
        };
        cpu.execute(bne_not_taken, &mut bus).unwrap();
        assert_eq!(cpu.get_pc(), 0x1004);
    }

    #[test]
    fn test_cpu_execute_signed_branches_blt_bge() {
        let mut bus = bus::Bus::new();
        let mut cpu = CPU::new();
        // -5 in two's complement and +5
        cpu.write_reg(1, (-5i32) as u32);
        cpu.write_reg(2, 5);

        // blt taken: -5 < 5
        cpu.pc = 0x1000;
        let blt_taken = Instruction::blt {
            rs1: 1,
            rs2: 2,
            imm: 16,
        };
        cpu.execute(blt_taken, &mut bus).unwrap();
        assert_eq!(cpu.get_pc(), 0x1010);

        // blt not taken: 5 < -5
        cpu.pc = 0x1000;
        let blt_not_taken = Instruction::blt {
            rs1: 2,
            rs2: 1,
            imm: 16,
        };
        cpu.execute(blt_not_taken, &mut bus).unwrap();
        assert_eq!(cpu.get_pc(), 0x1004);

        // bge taken: 5 >= -5
        cpu.pc = 0x1000;
        let bge_taken = Instruction::bge {
            rs1: 2,
            rs2: 1,
            imm: 16,
        };
        cpu.execute(bge_taken, &mut bus).unwrap();
        assert_eq!(cpu.get_pc(), 0x1010);

        // bge not taken: -5 >= 5
        cpu.pc = 0x1000;
        let bge_not_taken = Instruction::bge {
            rs1: 1,
            rs2: 2,
            imm: 16,
        };
        cpu.execute(bge_not_taken, &mut bus).unwrap();
        assert_eq!(cpu.get_pc(), 0x1004);
    }

    #[test]
    fn test_cpu_execute_unsigned_branches_bltu_bgeu() {
        let mut bus = bus::Bus::new();
        let mut cpu = CPU::new();
        // 0xFFFF_FFFB (4294967291 unsigned) vs 5
        cpu.write_reg(1, (-5i32) as u32);
        cpu.write_reg(2, 5);

        // bltu not taken: 0xFFFF_FFFB < 5 is false unsigned
        cpu.pc = 0x1000;
        let bltu_not_taken = Instruction::bltu {
            rs1: 1,
            rs2: 2,
            imm: 16,
        };
        cpu.execute(bltu_not_taken, &mut bus).unwrap();
        assert_eq!(cpu.get_pc(), 0x1004);

        // bltu taken: 5 < 0xFFFF_FFFB is true unsigned
        cpu.pc = 0x1000;
        let bltu_taken = Instruction::bltu {
            rs1: 2,
            rs2: 1,
            imm: 16,
        };
        cpu.execute(bltu_taken, &mut bus).unwrap();
        assert_eq!(cpu.get_pc(), 0x1010);

        // bgeu taken: 0xFFFF_FFFB >= 5 is true unsigned
        cpu.pc = 0x1000;
        let bgeu_taken = Instruction::bgeu {
            rs1: 1,
            rs2: 2,
            imm: 16,
        };
        cpu.execute(bgeu_taken, &mut bus).unwrap();
        assert_eq!(cpu.get_pc(), 0x1010);

        // bgeu not taken: 5 >= 0xFFFF_FFFB is false unsigned
        cpu.pc = 0x1000;
        let bgeu_not_taken = Instruction::bgeu {
            rs1: 2,
            rs2: 1,
            imm: 16,
        };
        cpu.execute(bgeu_not_taken, &mut bus).unwrap();
        assert_eq!(cpu.get_pc(), 0x1004);
    }

    #[test]
    fn test_cpu_execute_branch_unaligned_only_when_taken() {
        let mut bus = bus::Bus::new();
        let mut cpu = CPU::new();
        cpu.write_reg(1, 10);
        cpu.write_reg(2, 20);

        // Not taken branch with unaligned imm (2): must NOT error
        cpu.pc = 0x1000;
        let not_taken = Instruction::beq {
            rs1: 1,
            rs2: 2,
            imm: 2,
        };
        assert!(cpu.execute(not_taken, &mut bus).is_ok());
        assert_eq!(cpu.get_pc(), 0x1004);

        // Taken branch with unaligned imm (2): MUST error
        cpu.pc = 0x1000;
        let taken = Instruction::bne {
            rs1: 1,
            rs2: 2,
            imm: 2,
        };
        assert_eq!(cpu.execute(taken, &mut bus), Err(Error::IllegalInstruction));
    }

    #[test]
    fn test_cpu_execute_loads_sign_extension() {
        let mut bus = bus::Bus::new();
        // Memory content: [0x80, 0x7F, 0x00, 0xFF] at 0x2000
        let data = vec![0x80, 0x7F, 0x00, 0xFF];
        let region = Region::new("RAM", 0x2000, Permissions::RW, data);
        bus.add(region).unwrap();

        let mut cpu = CPU::new();
        cpu.write_reg(1, 0x2000);

        // 1. LB at 0x2000 (byte 0x80 -> signed negative: 0xFFFF_FF80)
        let instr_lb = Instruction::lb {
            rd: 2,
            rs1: 1,
            imm: 0,
        };
        cpu.execute(instr_lb, &mut bus).unwrap();
        assert_eq!(cpu.read_reg(2), 0xFFFF_FF80);

        // 2. LBU at 0x2000 (byte 0x80 -> unsigned zero-extended: 0x0000_0080)
        let instr_lbu = Instruction::lbu {
            rd: 3,
            rs1: 1,
            imm: 0,
        };
        cpu.execute(instr_lbu, &mut bus).unwrap();
        assert_eq!(cpu.read_reg(3), 0x0000_0080);

        // 3. LH at 0x2002 (halfword 0xFF00 -> signed negative: 0xFFFF_FF00)
        let instr_lh = Instruction::lh {
            rd: 4,
            rs1: 1,
            imm: 2,
        };
        cpu.execute(instr_lh, &mut bus).unwrap();
        assert_eq!(cpu.read_reg(4), 0xFFFF_FF00);

        // 4. LHU at 0x2002 (halfword 0xFF00 -> unsigned zero-extended: 0x0000_FF00)
        let instr_lhu = Instruction::lhu {
            rd: 5,
            rs1: 1,
            imm: 2,
        };
        cpu.execute(instr_lhu, &mut bus).unwrap();
        assert_eq!(cpu.read_reg(5), 0x0000_FF00);

        // 5. LW at 0x2000 (word 0xFF007F80)
        let instr_lw = Instruction::lw {
            rd: 6,
            rs1: 1,
            imm: 0,
        };
        cpu.execute(instr_lw, &mut bus).unwrap();
        assert_eq!(cpu.read_reg(6), 0xFF00_7F80);

        // Loading into x0 must discard the result
        let instr_lw_x0 = Instruction::lw {
            rd: 0,
            rs1: 1,
            imm: 0,
        };
        cpu.execute(instr_lw_x0, &mut bus).unwrap();
        assert_eq!(cpu.read_reg(0), 0);
    }

    #[test]
    fn test_cpu_execute_loads_bus_errors() {
        let mut bus = bus::Bus::new();
        // Region without read permissions
        let no_read_region = Region::new(
            "NO_READ",
            0x3000,
            Permissions {
                read: false,
                write: true,
                execute: false,
            },
            vec![0x12, 0x34],
        );
        bus.add(no_read_region).unwrap();

        let mut cpu = CPU::new();

        // 1. Unmapped memory address violation
        cpu.write_reg(1, 0x9000);
        let instr_unmapped = Instruction::lw {
            rd: 2,
            rs1: 1,
            imm: 0,
        };
        assert_eq!(
            cpu.execute(instr_unmapped, &mut bus),
            Err(Error::Bus(bus::Error::AddressViolation))
        );

        // 2. Permissions violation
        cpu.write_reg(1, 0x3000);
        let instr_no_perm = Instruction::lb {
            rd: 2,
            rs1: 1,
            imm: 0,
        };
        assert_eq!(
            cpu.execute(instr_no_perm, &mut bus),
            Err(Error::Bus(bus::Error::PermissionsViolation))
        );
    }

    #[test]
    fn test_cpu_execute_stores() {
        let mut bus = bus::Bus::new();
        let region = Region::new("RAM", 0x2000, Permissions::RW, vec![0; 16]);
        bus.add(region).unwrap();

        let mut cpu = CPU::new();
        cpu.write_reg(1, 0x2000);
        cpu.write_reg(2, 0x12345678);

        // 1. SB: write lowest byte (0x78) at 0x2000
        let instr_sb = Instruction::sb {
            rs1: 1,
            rs2: 2,
            imm: 0,
        };
        cpu.execute(instr_sb, &mut bus).unwrap();
        assert_eq!(bus.read_u8(0x2000, Permissions::RO).unwrap(), 0x78);

        // 2. SH: write lowest halfword (0x5678) at 0x2002
        let instr_sh = Instruction::sh {
            rs1: 1,
            rs2: 2,
            imm: 2,
        };
        cpu.execute(instr_sh, &mut bus).unwrap();
        assert_eq!(bus.read_u16(0x2002, Permissions::RO).unwrap(), 0x5678);

        // 3. SW: write full word (0x12345678) at 0x2004
        let instr_sw = Instruction::sw {
            rs1: 1,
            rs2: 2,
            imm: 4,
        };
        cpu.execute(instr_sw, &mut bus).unwrap();
        assert_eq!(bus.read_u32(0x2004, Permissions::RO).unwrap(), 0x12345678);
    }

    #[test]
    fn test_cpu_execute_stores_bus_errors() {
        let mut bus = bus::Bus::new();
        // Read-only flash region
        let ro_region = Region::new("FLASH", 0x1000, Permissions::RO, vec![0; 8]);
        bus.add(ro_region).unwrap();

        let mut cpu = CPU::new();
        cpu.write_reg(1, 0x1000);
        cpu.write_reg(2, 0x42);

        // Store to read-only memory -> PermissionsViolation
        let instr_sw_ro = Instruction::sw {
            rs1: 1,
            rs2: 2,
            imm: 0,
        };
        assert_eq!(
            cpu.execute(instr_sw_ro, &mut bus),
            Err(Error::Bus(bus::Error::PermissionsViolation))
        );

        // Store to unmapped memory -> AddressViolation
        cpu.write_reg(1, 0x9000);
        let instr_sw_unmapped = Instruction::sw {
            rs1: 1,
            rs2: 2,
            imm: 0,
        };
        assert_eq!(
            cpu.execute(instr_sw_unmapped, &mut bus),
            Err(Error::Bus(bus::Error::AddressViolation))
        );
    }

    #[test]
    fn test_cpu_execute_addi() {
        let mut bus = bus::Bus::new();
        let mut cpu = CPU::new();

        // 1. ADDI with positive immediate: x1 = x0 + 42
        cpu.execute(
            Instruction::addi {
                rd: 1,
                rs1: 0,
                imm: 42,
            },
            &mut bus,
        )
        .unwrap();
        assert_eq!(cpu.read_reg(1), 42);
        assert_eq!(cpu.get_pc(), 4);

        // 2. ADDI with negative immediate: x2 = x1 + (-10) = 32
        cpu.execute(
            Instruction::addi {
                rd: 2,
                rs1: 1,
                imm: -10,
            },
            &mut bus,
        )
        .unwrap();
        assert_eq!(cpu.read_reg(2), 32);
        assert_eq!(cpu.get_pc(), 8);

        // 3. ADDI wrapping: x3 = x0 + (-1) = 0xFFFF_FFFF
        cpu.execute(
            Instruction::addi {
                rd: 3,
                rs1: 0,
                imm: -1,
            },
            &mut bus,
        )
        .unwrap();
        assert_eq!(cpu.read_reg(3), 0xFFFF_FFFF);

        // x3 = x3 + 1 = 0 (wrap around)
        cpu.execute(
            Instruction::addi {
                rd: 3,
                rs1: 3,
                imm: 1,
            },
            &mut bus,
        )
        .unwrap();
        assert_eq!(cpu.read_reg(3), 0);

        // 4. ADDI to x0: discarded
        cpu.execute(
            Instruction::addi {
                rd: 0,
                rs1: 1,
                imm: 100,
            },
            &mut bus,
        )
        .unwrap();
        assert_eq!(cpu.read_reg(0), 0);
    }

    #[test]
    fn test_cpu_execute_slti() {
        let mut bus = bus::Bus::new();
        let mut cpu = CPU::new();

        // Set x1 = 5
        cpu.write_reg(1, 5);

        // 5 < 10 -> 1
        cpu.execute(
            Instruction::slti {
                rd: 2,
                rs1: 1,
                imm: 10,
            },
            &mut bus,
        )
        .unwrap();
        assert_eq!(cpu.read_reg(2), 1);

        // 5 < 5 -> 0
        cpu.execute(
            Instruction::slti {
                rd: 2,
                rs1: 1,
                imm: 5,
            },
            &mut bus,
        )
        .unwrap();
        assert_eq!(cpu.read_reg(2), 0);

        // 5 < 3 -> 0
        cpu.execute(
            Instruction::slti {
                rd: 2,
                rs1: 1,
                imm: 3,
            },
            &mut bus,
        )
        .unwrap();
        assert_eq!(cpu.read_reg(2), 0);

        // Signed comparison: -1 in two's-complement
        cpu.write_reg(3, 0xFFFF_FFFF);

        // -1 < 0 -> 1 (even though 0xFFFF_FFFF > 0 as unsigned)
        cpu.execute(
            Instruction::slti {
                rd: 4,
                rs1: 3,
                imm: 0,
            },
            &mut bus,
        )
        .unwrap();
        assert_eq!(cpu.read_reg(4), 1);

        // -1 < -2 -> 0
        cpu.execute(
            Instruction::slti {
                rd: 4,
                rs1: 3,
                imm: -2,
            },
            &mut bus,
        )
        .unwrap();
        assert_eq!(cpu.read_reg(4), 0);

        // Write to x0 is ignored
        cpu.execute(
            Instruction::slti {
                rd: 0,
                rs1: 3,
                imm: 0,
            },
            &mut bus,
        )
        .unwrap();
        assert_eq!(cpu.read_reg(0), 0);
    }

    #[test]
    fn test_cpu_execute_sltiu() {
        let mut bus = bus::Bus::new();
        let mut cpu = CPU::new();

        // Set x1 = 5
        cpu.write_reg(1, 5);

        // 5 < 10 -> 1
        cpu.execute(
            Instruction::sltiu {
                rd: 2,
                rs1: 1,
                imm: 10,
            },
            &mut bus,
        )
        .unwrap();
        assert_eq!(cpu.read_reg(2), 1);
        assert_eq!(cpu.get_pc(), 4);

        // 5 < 5 -> 0
        cpu.execute(
            Instruction::sltiu {
                rd: 2,
                rs1: 1,
                imm: 5,
            },
            &mut bus,
        )
        .unwrap();
        assert_eq!(cpu.read_reg(2), 0);

        // Contrast with signed slti:
        // Set x3 = 0xFFFF_FFFF (-1 signed, but 4294967295 unsigned)
        cpu.write_reg(3, 0xFFFF_FFFF);

        // Unsigned comparison: 0xFFFF_FFFF < 1 is false (0), unlike signed slti (-1 < 1 which is 1)
        cpu.execute(
            Instruction::sltiu {
                rd: 4,
                rs1: 3,
                imm: 1,
            },
            &mut bus,
        )
        .unwrap();
        assert_eq!(cpu.read_reg(4), 0);

        // Immediate is sign-extended to 32 bits, then treated as unsigned.
        // imm = -1 (0xFFF) becomes 0xFFFF_FFFF as unsigned.
        // For x1 = 5: 5 < 0xFFFF_FFFF is true (1).
        cpu.execute(
            Instruction::sltiu {
                rd: 5,
                rs1: 1,
                imm: -1,
            },
            &mut bus,
        )
        .unwrap();
        assert_eq!(cpu.read_reg(5), 1);

        // 0xFFFF_FFFF < 0xFFFF_FFFF is false (0)
        cpu.execute(
            Instruction::sltiu {
                rd: 5,
                rs1: 3,
                imm: -1,
            },
            &mut bus,
        )
        .unwrap();
        assert_eq!(cpu.read_reg(5), 0);

        // Write to x0 is ignored
        cpu.execute(
            Instruction::sltiu {
                rd: 0,
                rs1: 1,
                imm: 10,
            },
            &mut bus,
        )
        .unwrap();
        assert_eq!(cpu.read_reg(0), 0);
    }

    #[test]
    fn test_cpu_execute_xori() {
        let mut bus = bus::Bus::new();
        let mut cpu = CPU::new();

        // 1. Basic XOR: 0b1010_1010 ^ 0b0101_0101 = 0b1111_1111
        cpu.write_reg(1, 0b1010_1010);
        cpu.execute(
            Instruction::xori {
                rd: 2,
                rs1: 1,
                imm: 0b0101_0101,
            },
            &mut bus,
        )
        .unwrap();
        assert_eq!(cpu.read_reg(2), 0b1111_1111);
        assert_eq!(cpu.get_pc(), 4);

        // 2. Sign-extended immediate: bitwise NOT via imm = -1 (0xFFFF_FFFF)
        cpu.write_reg(3, 0x1234_5678);
        cpu.execute(
            Instruction::xori {
                rd: 4,
                rs1: 3,
                imm: -1,
            },
            &mut bus,
        )
        .unwrap();
        assert_eq!(cpu.read_reg(4), !0x1234_5678);

        // 3. Write to x0 is ignored
        cpu.execute(
            Instruction::xori {
                rd: 0,
                rs1: 1,
                imm: -1,
            },
            &mut bus,
        )
        .unwrap();
        assert_eq!(cpu.read_reg(0), 0);
    }

    #[test]
    fn test_cpu_execute_ori() {
        let mut bus = bus::Bus::new();
        let mut cpu = CPU::new();

        // 1. Basic OR: 0xF0 | 0x0F = 0xFF
        cpu.write_reg(1, 0xF0);
        cpu.execute(
            Instruction::ori {
                rd: 2,
                rs1: 1,
                imm: 0x0F,
            },
            &mut bus,
        )
        .unwrap();
        assert_eq!(cpu.read_reg(2), 0xFF);
        assert_eq!(cpu.get_pc(), 4);

        // 2. Sign-extended immediate: imm = -1 sets all bits to 1
        cpu.write_reg(3, 0x1234_0000);
        cpu.execute(
            Instruction::ori {
                rd: 4,
                rs1: 3,
                imm: -1,
            },
            &mut bus,
        )
        .unwrap();
        assert_eq!(cpu.read_reg(4), 0xFFFF_FFFF);

        // 3. Write to x0 is ignored
        cpu.execute(
            Instruction::ori {
                rd: 0,
                rs1: 1,
                imm: 0xFF,
            },
            &mut bus,
        )
        .unwrap();
        assert_eq!(cpu.read_reg(0), 0);
    }

    #[test]
    fn test_cpu_execute_andi() {
        let mut bus = bus::Bus::new();
        let mut cpu = CPU::new();

        // 1. Basic AND: 0b1111_0000 & 0b0101_0101 = 0b0101_0000
        cpu.write_reg(1, 0b1111_0000);
        cpu.execute(
            Instruction::andi {
                rd: 2,
                rs1: 1,
                imm: 0b0101_0101,
            },
            &mut bus,
        )
        .unwrap();
        assert_eq!(cpu.read_reg(2), 0b0101_0000);
        assert_eq!(cpu.get_pc(), 4);

        // 2. Sign-extended immediate: imm = -1 keeps all bits intact (mask with 0xFFFF_FFFF)
        cpu.write_reg(3, 0x1234_5678);
        cpu.execute(
            Instruction::andi {
                rd: 4,
                rs1: 3,
                imm: -1,
            },
            &mut bus,
        )
        .unwrap();
        assert_eq!(cpu.read_reg(4), 0x1234_5678);

        // 3. Mask lowest 8 bits (positive imm 0xFF is zero-extended in 12-bit, so upper 20 bits are 0)
        cpu.write_reg(5, 0xABCD_EF42);
        cpu.execute(
            Instruction::andi {
                rd: 6,
                rs1: 5,
                imm: 0x0FF,
            },
            &mut bus,
        )
        .unwrap();
        assert_eq!(cpu.read_reg(6), 0x42);

        // 4. Write to x0 is ignored
        cpu.execute(
            Instruction::andi {
                rd: 0,
                rs1: 1,
                imm: 0x0F,
            },
            &mut bus,
        )
        .unwrap();
        assert_eq!(cpu.read_reg(0), 0);
    }

    #[test]
    fn test_cpu_execute_slli() {
        let mut bus = bus::Bus::new();
        let mut cpu = CPU::new();

        // 1. Basic left shift: 1 << 4 = 16 (0x10)
        cpu.write_reg(1, 1);
        cpu.execute(
            Instruction::slli {
                rd: 2,
                rs1: 1,
                shamt: 4,
            },
            &mut bus,
        )
        .unwrap();
        assert_eq!(cpu.read_reg(2), 0x10);
        assert_eq!(cpu.get_pc(), 4);

        // 2. Shift by 31: 1 << 31 = 0x8000_0000
        cpu.execute(
            Instruction::slli {
                rd: 3,
                rs1: 1,
                shamt: 31,
            },
            &mut bus,
        )
        .unwrap();
        assert_eq!(cpu.read_reg(3), 0x8000_0000);

        // 3. Shift out upper bits: 0x8000_0001 << 1 = 2
        cpu.write_reg(4, 0x8000_0001);
        cpu.execute(
            Instruction::slli {
                rd: 5,
                rs1: 4,
                shamt: 1,
            },
            &mut bus,
        )
        .unwrap();
        assert_eq!(cpu.read_reg(5), 2);

        // 4. Shift by 0: value unchanged
        cpu.execute(
            Instruction::slli {
                rd: 6,
                rs1: 4,
                shamt: 0,
            },
            &mut bus,
        )
        .unwrap();
        assert_eq!(cpu.read_reg(6), 0x8000_0001);

        // 5. Write to x0 is ignored
        cpu.execute(
            Instruction::slli {
                rd: 0,
                rs1: 1,
                shamt: 4,
            },
            &mut bus,
        )
        .unwrap();
        assert_eq!(cpu.read_reg(0), 0);
    }

    #[test]
    fn test_cpu_execute_srli() {
        let mut bus = bus::Bus::new();
        let mut cpu = CPU::new();

        // 1. Basic logical right shift: 0x10 >> 4 = 1
        cpu.write_reg(1, 0x10);
        cpu.execute(
            Instruction::srli {
                rd: 2,
                rs1: 1,
                shamt: 4,
            },
            &mut bus,
        )
        .unwrap();
        assert_eq!(cpu.read_reg(2), 1);
        assert_eq!(cpu.get_pc(), 4);

        // 2. Logical shift inserts zeros at MSB: 0x8000_0000 >> 1 = 0x4000_0000
        cpu.write_reg(3, 0x8000_0000);
        cpu.execute(
            Instruction::srli {
                rd: 4,
                rs1: 3,
                shamt: 1,
            },
            &mut bus,
        )
        .unwrap();
        assert_eq!(cpu.read_reg(4), 0x4000_0000);

        // 3. Shift by 31: 0x8000_0000 >> 31 = 1
        cpu.execute(
            Instruction::srli {
                rd: 5,
                rs1: 3,
                shamt: 31,
            },
            &mut bus,
        )
        .unwrap();
        assert_eq!(cpu.read_reg(5), 1);

        // 4. Write to x0 is ignored
        cpu.execute(
            Instruction::srli {
                rd: 0,
                rs1: 1,
                shamt: 2,
            },
            &mut bus,
        )
        .unwrap();
        assert_eq!(cpu.read_reg(0), 0);
    }

    #[test]
    fn test_cpu_execute_srai() {
        let mut bus = bus::Bus::new();
        let mut cpu = CPU::new();

        // 1. Positive number shift: sign bit is 0, so zeros are shifted in
        cpu.write_reg(1, 0x4000_0000);
        cpu.execute(
            Instruction::srai {
                rd: 2,
                rs1: 1,
                shamt: 1,
            },
            &mut bus,
        )
        .unwrap();
        assert_eq!(cpu.read_reg(2), 0x2000_0000);
        assert_eq!(cpu.get_pc(), 4);

        // 2. Negative number shift: sign bit is 1, so ones are replicated
        // 0x8000_0000 >> 1 = 0xC000_0000
        cpu.write_reg(3, 0x8000_0000);
        cpu.execute(
            Instruction::srai {
                rd: 4,
                rs1: 3,
                shamt: 1,
            },
            &mut bus,
        )
        .unwrap();
        assert_eq!(cpu.read_reg(4), 0xC000_0000);

        // 3. Negative number shifted by 31: all bits become 1 (-1 in two's complement)
        cpu.execute(
            Instruction::srai {
                rd: 5,
                rs1: 3,
                shamt: 31,
            },
            &mut bus,
        )
        .unwrap();
        assert_eq!(cpu.read_reg(5), 0xFFFF_FFFF);

        // 4. Positive number shifted by 31: all bits become 0
        cpu.write_reg(6, 0x7FFF_FFFF);
        cpu.execute(
            Instruction::srai {
                rd: 7,
                rs1: 6,
                shamt: 31,
            },
            &mut bus,
        )
        .unwrap();
        assert_eq!(cpu.read_reg(7), 0);

        // 5. Write to x0 is ignored
        cpu.execute(
            Instruction::srai {
                rd: 0,
                rs1: 3,
                shamt: 1,
            },
            &mut bus,
        )
        .unwrap();
        assert_eq!(cpu.read_reg(0), 0);
    }

    #[test]
    fn test_cpu_execute_add() {
        let mut bus = bus::Bus::new();
        let mut cpu = CPU::new();

        // 1. Basic addition: 10 + 25 = 35
        cpu.write_reg(1, 10);
        cpu.write_reg(2, 25);
        cpu.execute(
            Instruction::add {
                rd: 3,
                rs1: 1,
                rs2: 2,
            },
            &mut bus,
        )
        .unwrap();
        assert_eq!(cpu.read_reg(3), 35);
        assert_eq!(cpu.get_pc(), 4);

        // 2. Overflow wrapping: 0xFFFF_FFFF + 1 = 0
        cpu.write_reg(4, 0xFFFF_FFFF);
        cpu.write_reg(5, 1);
        cpu.execute(
            Instruction::add {
                rd: 6,
                rs1: 4,
                rs2: 5,
            },
            &mut bus,
        )
        .unwrap();
        assert_eq!(cpu.read_reg(6), 0);

        // 3. Two's complement negative: -5 + 10 = 5
        cpu.write_reg(7, (-5i32) as u32);
        cpu.execute(
            Instruction::add {
                rd: 8,
                rs1: 7,
                rs2: 1,
            },
            &mut bus,
        )
        .unwrap();
        assert_eq!(cpu.read_reg(8), 5);

        // 4. Write to x0 is ignored
        cpu.execute(
            Instruction::add {
                rd: 0,
                rs1: 1,
                rs2: 2,
            },
            &mut bus,
        )
        .unwrap();
        assert_eq!(cpu.read_reg(0), 0);
    }

    #[test]
    fn test_cpu_execute_sub() {
        let mut bus = bus::Bus::new();
        let mut cpu = CPU::new();

        // 1. Basic subtraction: 25 - 10 = 15
        cpu.write_reg(1, 25);
        cpu.write_reg(2, 10);
        cpu.execute(
            Instruction::sub {
                rd: 3,
                rs1: 1,
                rs2: 2,
            },
            &mut bus,
        )
        .unwrap();
        assert_eq!(cpu.read_reg(3), 15);
        assert_eq!(cpu.get_pc(), 4);

        // 2. Negative result (wrapping): 2 - 5 = -3 (0xFFFF_FFFD)
        cpu.write_reg(4, 2);
        cpu.write_reg(5, 5);
        cpu.execute(
            Instruction::sub {
                rd: 6,
                rs1: 4,
                rs2: 5,
            },
            &mut bus,
        )
        .unwrap();
        assert_eq!(cpu.read_reg(6), (-3i32) as u32);

        // 3. Subtraction to zero
        cpu.execute(
            Instruction::sub {
                rd: 7,
                rs1: 1,
                rs2: 1,
            },
            &mut bus,
        )
        .unwrap();
        assert_eq!(cpu.read_reg(7), 0);

        // 4. Write to x0 is ignored
        cpu.execute(
            Instruction::sub {
                rd: 0,
                rs1: 1,
                rs2: 2,
            },
            &mut bus,
        )
        .unwrap();
        assert_eq!(cpu.read_reg(0), 0);
    }

    #[test]
    fn test_cpu_execute_slt() {
        let mut bus = bus::Bus::new();
        let mut cpu = CPU::new();

        cpu.write_reg(1, 5);
        cpu.write_reg(2, 10);
        cpu.write_reg(3, (-5i32) as u32);

        // 5 < 10 -> 1
        cpu.execute(
            Instruction::slt {
                rd: 4,
                rs1: 1,
                rs2: 2,
            },
            &mut bus,
        )
        .unwrap();
        assert_eq!(cpu.read_reg(4), 1);
        assert_eq!(cpu.get_pc(), 4);

        // 10 < 5 -> 0
        cpu.execute(
            Instruction::slt {
                rd: 5,
                rs1: 2,
                rs2: 1,
            },
            &mut bus,
        )
        .unwrap();
        assert_eq!(cpu.read_reg(5), 0);

        // Signed comparison: -5 < 5 -> 1 (even though 0xFFFF_FFFB > 5 as unsigned)
        cpu.execute(
            Instruction::slt {
                rd: 6,
                rs1: 3,
                rs2: 1,
            },
            &mut bus,
        )
        .unwrap();
        assert_eq!(cpu.read_reg(6), 1);

        // 5 < -5 -> 0
        cpu.execute(
            Instruction::slt {
                rd: 7,
                rs1: 1,
                rs2: 3,
            },
            &mut bus,
        )
        .unwrap();
        assert_eq!(cpu.read_reg(7), 0);

        // Write to x0 is ignored
        cpu.execute(
            Instruction::slt {
                rd: 0,
                rs1: 1,
                rs2: 2,
            },
            &mut bus,
        )
        .unwrap();
        assert_eq!(cpu.read_reg(0), 0);
    }

    #[test]
    fn test_cpu_execute_sltu() {
        let mut bus = bus::Bus::new();
        let mut cpu = CPU::new();

        cpu.write_reg(1, 5);
        cpu.write_reg(2, 10);
        cpu.write_reg(3, 0xFFFF_FFFB); // large unsigned number

        // 5 < 10 -> 1
        cpu.execute(
            Instruction::sltu {
                rd: 4,
                rs1: 1,
                rs2: 2,
            },
            &mut bus,
        )
        .unwrap();
        assert_eq!(cpu.read_reg(4), 1);
        assert_eq!(cpu.get_pc(), 4);

        // 5 < 0xFFFF_FFFB -> 1
        cpu.execute(
            Instruction::sltu {
                rd: 5,
                rs1: 1,
                rs2: 3,
            },
            &mut bus,
        )
        .unwrap();
        assert_eq!(cpu.read_reg(5), 1);

        // Unsigned comparison: 0xFFFF_FFFB < 5 -> 0 (opposite of signed slt!)
        cpu.execute(
            Instruction::sltu {
                rd: 6,
                rs1: 3,
                rs2: 1,
            },
            &mut bus,
        )
        .unwrap();
        assert_eq!(cpu.read_reg(6), 0);

        // Write to x0 is ignored
        cpu.execute(
            Instruction::sltu {
                rd: 0,
                rs1: 1,
                rs2: 2,
            },
            &mut bus,
        )
        .unwrap();
        assert_eq!(cpu.read_reg(0), 0);
    }

    #[test]
    fn test_cpu_execute_xor() {
        let mut bus = bus::Bus::new();
        let mut cpu = CPU::new();

        cpu.write_reg(1, 0b1100);
        cpu.write_reg(2, 0b1010);
        cpu.execute(
            Instruction::xor {
                rd: 3,
                rs1: 1,
                rs2: 2,
            },
            &mut bus,
        )
        .unwrap();
        assert_eq!(cpu.read_reg(3), 0b0110);
        assert_eq!(cpu.get_pc(), 4);

        // Write to x0 is ignored
        cpu.execute(
            Instruction::xor {
                rd: 0,
                rs1: 1,
                rs2: 2,
            },
            &mut bus,
        )
        .unwrap();
        assert_eq!(cpu.read_reg(0), 0);
    }

    #[test]
    fn test_cpu_execute_or() {
        let mut bus = bus::Bus::new();
        let mut cpu = CPU::new();

        cpu.write_reg(1, 0b1100);
        cpu.write_reg(2, 0b1010);
        cpu.execute(
            Instruction::or {
                rd: 3,
                rs1: 1,
                rs2: 2,
            },
            &mut bus,
        )
        .unwrap();
        assert_eq!(cpu.read_reg(3), 0b1110);
        assert_eq!(cpu.get_pc(), 4);

        // Write to x0 is ignored
        cpu.execute(
            Instruction::or {
                rd: 0,
                rs1: 1,
                rs2: 2,
            },
            &mut bus,
        )
        .unwrap();
        assert_eq!(cpu.read_reg(0), 0);
    }

    #[test]
    fn test_cpu_execute_and() {
        let mut bus = bus::Bus::new();
        let mut cpu = CPU::new();

        cpu.write_reg(1, 0b1100);
        cpu.write_reg(2, 0b1010);
        cpu.execute(
            Instruction::and {
                rd: 3,
                rs1: 1,
                rs2: 2,
            },
            &mut bus,
        )
        .unwrap();
        assert_eq!(cpu.read_reg(3), 0b1000);
        assert_eq!(cpu.get_pc(), 4);

        // Write to x0 is ignored
        cpu.execute(
            Instruction::and {
                rd: 0,
                rs1: 1,
                rs2: 2,
            },
            &mut bus,
        )
        .unwrap();
        assert_eq!(cpu.read_reg(0), 0);
    }

    #[test]
    fn test_cpu_execute_sll() {
        let mut bus = bus::Bus::new();
        let mut cpu = CPU::new();

        // 1. Basic shift: 1 << 4 = 16
        cpu.write_reg(1, 1);
        cpu.write_reg(2, 4);
        cpu.execute(
            Instruction::sll {
                rd: 3,
                rs1: 1,
                rs2: 2,
            },
            &mut bus,
        )
        .unwrap();
        assert_eq!(cpu.read_reg(3), 16);
        assert_eq!(cpu.get_pc(), 4);

        // 2. Only lower 5 bits of rs2 are used: rs2 = 0x24 (36) -> shift by (36 & 0x1F) = 4
        cpu.write_reg(4, 0x24);
        cpu.execute(
            Instruction::sll {
                rd: 5,
                rs1: 1,
                rs2: 4,
            },
            &mut bus,
        )
        .unwrap();
        assert_eq!(cpu.read_reg(5), 16);

        // Write to x0 is ignored
        cpu.execute(
            Instruction::sll {
                rd: 0,
                rs1: 1,
                rs2: 2,
            },
            &mut bus,
        )
        .unwrap();
        assert_eq!(cpu.read_reg(0), 0);
    }

    #[test]
    fn test_cpu_execute_srl() {
        let mut bus = bus::Bus::new();
        let mut cpu = CPU::new();

        // 1. Logical shift right zeros MSB: 0x8000_0000 >> 1 = 0x4000_0000
        cpu.write_reg(1, 0x8000_0000);
        cpu.write_reg(2, 1);
        cpu.execute(
            Instruction::srl {
                rd: 3,
                rs1: 1,
                rs2: 2,
            },
            &mut bus,
        )
        .unwrap();
        assert_eq!(cpu.read_reg(3), 0x4000_0000);
        assert_eq!(cpu.get_pc(), 4);

        // 2. Only lower 5 bits of rs2 are used: rs2 = 0x41 (65) -> shift by (65 & 0x1F) = 1
        cpu.write_reg(4, 0x41);
        cpu.execute(
            Instruction::srl {
                rd: 5,
                rs1: 1,
                rs2: 4,
            },
            &mut bus,
        )
        .unwrap();
        assert_eq!(cpu.read_reg(5), 0x4000_0000);

        // Write to x0 is ignored
        cpu.execute(
            Instruction::srl {
                rd: 0,
                rs1: 1,
                rs2: 2,
            },
            &mut bus,
        )
        .unwrap();
        assert_eq!(cpu.read_reg(0), 0);
    }

    #[test]
    fn test_cpu_execute_sra() {
        let mut bus = bus::Bus::new();
        let mut cpu = CPU::new();

        // 1. Arithmetic shift right copies sign: 0x8000_0000 >> 1 = 0xC000_0000
        cpu.write_reg(1, 0x8000_0000);
        cpu.write_reg(2, 1);
        cpu.execute(
            Instruction::sra {
                rd: 3,
                rs1: 1,
                rs2: 2,
            },
            &mut bus,
        )
        .unwrap();
        assert_eq!(cpu.read_reg(3), 0xC000_0000);
        assert_eq!(cpu.get_pc(), 4);

        // 2. Positive number arithmetic shift: 0x4000_0000 >> 1 = 0x2000_0000
        cpu.write_reg(4, 0x4000_0000);
        cpu.execute(
            Instruction::sra {
                rd: 5,
                rs1: 4,
                rs2: 2,
            },
            &mut bus,
        )
        .unwrap();
        assert_eq!(cpu.read_reg(5), 0x2000_0000);

        // 3. Only lower 5 bits of rs2 are used: rs2 = 0x61 (97) -> shift by (97 & 0x1F) = 1
        cpu.write_reg(6, 0x61);
        cpu.execute(
            Instruction::sra {
                rd: 7,
                rs1: 1,
                rs2: 6,
            },
            &mut bus,
        )
        .unwrap();
        assert_eq!(cpu.read_reg(7), 0xC000_0000);

        // Write to x0 is ignored
        cpu.execute(
            Instruction::sra {
                rd: 0,
                rs1: 1,
                rs2: 2,
            },
            &mut bus,
        )
        .unwrap();
        assert_eq!(cpu.read_reg(0), 0);
    }

    #[test]
    fn test_cpu_execute_fence() {
        let mut bus = bus::Bus::new();
        let mut cpu = CPU::new();

        cpu.execute(Instruction::fence, &mut bus).unwrap();
        assert_eq!(cpu.get_pc(), 4);
    }

    #[test]
    fn test_cpu_execute_ecall() {
        let mut bus = bus::Bus::new();
        let mut cpu = CPU::new();
        cpu.pc = 0x1000;

        assert_eq!(cpu.execute(Instruction::ecall, &mut bus), Err(Error::Ecall));
        assert_eq!(cpu.get_pc(), 0x1000); // PC remains at the trapped instruction
    }

    #[test]
    fn test_cpu_execute_ebreak() {
        let mut bus = bus::Bus::new();
        let mut cpu = CPU::new();
        cpu.pc = 0x2000;

        assert_eq!(
            cpu.execute(Instruction::ebreak, &mut bus),
            Err(Error::Ebreak)
        );
        assert_eq!(cpu.get_pc(), 0x2000); // PC remains at the trapped instruction
    }
}
