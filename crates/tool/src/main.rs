use object::elf;
use object::{Object, ObjectSection, ObjectSymbol, SectionKind};
use rv32_isa::Instruction;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fs;
use std::path::Path;

fn get_addressable_symbols(file: &object::File) -> HashMap<String, usize> {
    let mut res = HashSet::new();

    for section in file.sections() {
        for (_offset, reloc) in section.relocations() {
            if !match reloc.flags() {
                object::RelocationFlags::Elf { r_type } => {
                    matches!(
                        r_type,
                        elf::R_RISCV_32
                            | elf::R_RISCV_PCREL_HI20
                            | elf::R_RISCV_HI20
                            | elf::R_RISCV_32_PCREL
                            | elf::R_RISCV_PLT32
                    )
                }
                _ => false,
            } {
                continue;
            }
            if let object::RelocationTarget::Symbol(index) = reloc.target()
                && let Ok(sym) = file.symbol_by_index(index)
                && !sym.is_undefined()
                && sym.kind() == object::SymbolKind::Text
                && let Ok(name) = sym.name()
            {
                res.insert(name.to_string());
            }
        }
    }

    let mut sorted_symbols: Vec<String> = res.into_iter().collect();
    sorted_symbols.sort();

    let mut map = HashMap::new();
    for (i, val) in sorted_symbols.into_iter().enumerate() {
        map.insert(val, i);
    }

    map
}

fn emit_plt_for_addressable(symbols: &HashMap<String, usize>, dir: &Path) -> std::io::Result<()> {
    let mut trampolines = String::from(".section .text.__sseocb_g.plt, \"ax\", @progbits\n\n");
    let mut linker = String::new();
    let mut wrap = String::new();

    let mut entries: Vec<(&String, &usize)> = symbols.iter().collect();
    entries.sort_by_key(|(_, num)| *num);

    for (sym, num) in entries {
        let plt_sym = format!("__sseocb_g_plt_{sym}");
        trampolines.push_str(&format!(
            ".global {plt_sym}\n\
            {plt_sym}:\n    \
            li x16, {num}\n    \
            j __sseocb_i_plt_call\n\n"
        ));

        linker.push_str(&format!("__wrap_{sym} = {plt_sym};\n"));
        wrap.push_str(&format!("--wrap={sym}\n"));
    }

    fs::write(dir.join("trampolines.s"), trampolines)?;
    fs::write(dir.join("plt.ld"), linker)?;
    fs::write(dir.join("plt.wrap"), wrap)?;

    Ok(())
}

fn parse_addr(s: &str) -> Result<u64, Box<dyn Error>> {
    let s = s.trim();
    let val = if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        u64::from_str_radix(hex, 16)?
    } else {
        s.parse::<u64>()?
    };
    Ok(val)
}

fn parse_size(s: &str) -> Result<u64, Box<dyn Error>> {
    let s = s.trim();
    let (num_str, multiplier) = if let Some(rest) = s
        .strip_suffix("GiB")
        .or_else(|| s.strip_suffix('G'))
        .or_else(|| s.strip_suffix("GB"))
    {
        (rest, 1024 * 1024 * 1024)
    } else if let Some(rest) = s
        .strip_suffix("MiB")
        .or_else(|| s.strip_suffix('M'))
        .or_else(|| s.strip_suffix("MB"))
    {
        (rest, 1024 * 1024)
    } else if let Some(rest) = s
        .strip_suffix("KiB")
        .or_else(|| s.strip_suffix('K'))
        .or_else(|| s.strip_suffix("KB"))
    {
        (rest, 1024)
    } else {
        (s, 1)
    };

    let base_val = parse_addr(num_str)?;
    Ok(base_val * multiplier)
}

fn collect_sections_for_vp(
    file: &object::File,
    dir: &Path,
    vpbase: u64,
    vpsize: u64,
) -> std::io::Result<()> {
    let mut script = format!(
        "MEMORY\n\
        {{\n    \
            __SSEOCB_G_VIRTUAL (rx) : ORIGIN = 0x{vpbase:08x}, LENGTH = 0x{vpsize:x}\n\
        }}\n\n\
        SECTIONS\n\
        {{\n    \
            .__sseocb_g_vP : {{\n",
    );
    for section in file.sections() {
        if section.kind() != SectionKind::Text {
            continue;
        }

        if let Ok(name) = section.name() {
            script.push_str(&format!("        *({name})\n"));
        }
    }
    script.push_str("        . = ALIGN(4);\n    } > __SSEOCB_G_VIRTUAL\n}\n");
    fs::write(dir.join("virtual.ld"), script)?;

    Ok(())
}

fn run_pre(
    input_file: &str,
    output_dir: &str,
    vpbase: u64,
    vpsize: u64,
) -> Result<(), Box<dyn Error>> {
    let data = fs::read(input_file)?;
    let file = object::File::parse(&*data)?;

    println!("Sections:");
    for section in file.sections() {
        println!("  {}", section.name()?);
    }

    let addressable_symbols = get_addressable_symbols(&file);
    println!("Addressable ({} symbols):", addressable_symbols.len());
    let mut sorted_symbols: Vec<(&String, &usize)> = addressable_symbols.iter().collect();
    sorted_symbols.sort_by_key(|(_, num)| *num);
    for (symbol, num) in sorted_symbols {
        println!("  {}: {}", num, symbol);
    }

    let output_dir = Path::new(output_dir);
    fs::create_dir_all(output_dir)?;

    emit_plt_for_addressable(&addressable_symbols, output_dir)?;
    collect_sections_for_vp(&file, output_dir, vpbase, vpsize)?;

    let mut slots_json = serde_json::to_string_pretty(&addressable_symbols)?;
    slots_json.push('\n');
    fs::write(output_dir.join("plt_index.json"), slots_json)?;

    Ok(())
}

fn run_promote(
    input_path: &str,
    output_path: &str,
    target_arch: Option<&str>,
) -> Result<(), Box<dyn Error>> {
    let mut data = fs::read(input_path)?;
    if data.len() < 52 {
        return Err(format!(
            "File '{}' is too small to be a valid ELF header",
            input_path
        )
        .into());
    }

    if data[0..4] != elf::ELFMAG {
        return Err(format!("File '{}' is not an ELF binary", input_path).into());
    }

    if data[4] != 1 {
        return Err(format!(
            "File '{}' is not a 32-bit ELF binary (class: {})",
            input_path, data[4]
        )
        .into());
    }

    let e_machine = u16::from_le_bytes(data[18..20].try_into()?);
    if e_machine != elf::EM_RISCV.0 {
        return Err(format!("ELF machine is not RISC-V (e_machine = 0x{:x})", e_machine).into());
    }

    let flags_offset = 36;
    let orig_flags = u32::from_le_bytes(data[flags_offset..flags_offset + 4].try_into()?);

    let target = target_arch.unwrap_or("rv32i");
    let new_flags = match target {
        "rv32i" => orig_flags & !elf::EF_RISCV_RVE.0,
        "rv32e" => orig_flags | elf::EF_RISCV_RVE.0,
        other => {
            return Err(format!(
                "Unknown target architecture '{}'. Expected 'rv32i' or 'rv32e'",
                other
            )
            .into());
        }
    };
    data[flags_offset..flags_offset + 4].copy_from_slice(&new_flags.to_le_bytes());

    // Also update architecture string in .riscv.attributes if present
    let (from_str, to_str) = match target {
        "rv32i" => (b"rv32e", b"rv32i"),
        "rv32e" => (b"rv32i", b"rv32e"),
        _ => unreachable!(),
    };
    if let Some(pos) = data.windows(from_str.len()).position(|w| w == from_str) {
        data[pos..pos + to_str.len()].copy_from_slice(to_str);
    }

    fs::write(output_path, data)?;
    println!(
        "Promoted '{}' -> '{}' to {} (EF_RISCV_RVE: 0x{:x} -> 0x{:x})",
        input_path, output_path, target, orig_flags, new_flags
    );

    Ok(())
}

fn remap_register(reg: &mut usize) -> Result<(), Box<dyn Error>> {
    if *reg > 15 {
        return Err(format!("register x{} exceeds x15; expected RV32E input", *reg).into());
    }
    if *reg != 0 {
        *reg += 15;
    }
    Ok(())
}

fn remap_instruction(instr: &mut Instruction) -> Result<(), Box<dyn Error>> {
    match instr {
        Instruction::lui { rd, .. } | Instruction::auipc { rd, .. } => {
            remap_register(rd)?;
        }
        Instruction::jal { rd, .. } => {
            remap_register(rd)?;
        }
        Instruction::jalr { rd, rs1, .. } => {
            remap_register(rd)?;
            remap_register(rs1)?;
        }
        Instruction::beq { rs1, rs2, .. }
        | Instruction::bne { rs1, rs2, .. }
        | Instruction::blt { rs1, rs2, .. }
        | Instruction::bge { rs1, rs2, .. }
        | Instruction::bltu { rs1, rs2, .. }
        | Instruction::bgeu { rs1, rs2, .. } => {
            remap_register(rs1)?;
            remap_register(rs2)?;
        }
        Instruction::lb { rd, rs1, .. }
        | Instruction::lh { rd, rs1, .. }
        | Instruction::lw { rd, rs1, .. }
        | Instruction::lbu { rd, rs1, .. }
        | Instruction::lhu { rd, rs1, .. } => {
            remap_register(rd)?;
            remap_register(rs1)?;
        }
        Instruction::sb { rs1, rs2, .. }
        | Instruction::sh { rs1, rs2, .. }
        | Instruction::sw { rs1, rs2, .. } => {
            remap_register(rs1)?;
            remap_register(rs2)?;
        }
        Instruction::addi { rd, rs1, .. }
        | Instruction::slti { rd, rs1, .. }
        | Instruction::sltiu { rd, rs1, .. }
        | Instruction::xori { rd, rs1, .. }
        | Instruction::ori { rd, rs1, .. }
        | Instruction::andi { rd, rs1, .. }
        | Instruction::slli { rd, rs1, .. }
        | Instruction::srli { rd, rs1, .. }
        | Instruction::srai { rd, rs1, .. } => {
            remap_register(rd)?;
            remap_register(rs1)?;
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
            remap_register(rd)?;
            remap_register(rs1)?;
            remap_register(rs2)?;
        }
        Instruction::fence | Instruction::ecall | Instruction::ebreak => {}
    }
    Ok(())
}

fn run_remap(input_path: &str, output_path: &str) -> Result<(), Box<dyn Error>> {
    let mut data = fs::read(input_path)?;
    if data.len() < 52 {
        return Err(format!(
            "File '{}' is too small to be a valid ELF header",
            input_path
        )
        .into());
    }

    if data[0..4] != elf::ELFMAG {
        return Err(format!("File '{}' is not an ELF binary", input_path).into());
    }

    if data[4] != 1 {
        return Err(format!(
            "File '{}' is not a 32-bit ELF binary (class: {})",
            input_path, data[4]
        )
        .into());
    }

    let e_machine = u16::from_le_bytes(data[18..20].try_into()?);
    if e_machine != elf::EM_RISCV.0 {
        return Err(format!("ELF machine is not RISC-V (e_machine = 0x{:x})", e_machine).into());
    }

    let file = object::File::parse(&*data)?;

    let mut exec_sections: Vec<(String, usize, usize)> = Vec::new();
    for section in file.sections() {
        let is_exec = match section.flags() {
            object::SectionFlags::Elf { sh_flags, .. } => (sh_flags.0 & elf::SHF_EXECINSTR.0) != 0,
            _ => false,
        };
        if is_exec && let Some((offset, size)) = section.file_range() {
            let name = section.name().unwrap_or("<unnamed>").to_string();
            exec_sections.push((name, offset as usize, size as usize));
        }
    }

    let mut total_instructions_remapped = 0;
    let section_count = exec_sections.len();

    for (name, offset, size) in exec_sections {
        let end = offset + size;
        let mut warned_non_instr = false;
        let mut pos = offset;

        while pos + 4 <= end {
            let raw = u32::from_le_bytes(data[pos..pos + 4].try_into()?);
            if raw == 0 {
                // Zero alignment padding, leave as-is
                pos += 4;
                continue;
            }

            match rv32_isa::parse(raw) {
                Some(mut instr) => {
                    remap_instruction(&mut instr)?;
                    let new_raw = instr.encode();
                    data[pos..pos + 4].copy_from_slice(&new_raw.to_le_bytes());
                    total_instructions_remapped += 1;
                }
                None => {
                    if !warned_non_instr {
                        eprintln!(
                            "Warning: section '{}' contains unparseable word(s) (e.g. 0x{:08x} at file offset 0x{:08x}); leaving untouched",
                            name, raw, pos
                        );
                        warned_non_instr = true;
                    }
                }
            }
            pos += 4;
        }
    }

    fs::write(output_path, &data)?;
    println!(
        "Remapped '{}' -> '{}': patched {} instructions across {} executable sections",
        input_path, output_path, total_instructions_remapped, section_count
    );

    Ok(())
}

fn run_post(linked_elf_path: &str, output_dir: &str) -> Result<(), Box<dyn Error>> {
    let out_dir = Path::new(output_dir);
    let index_path = out_dir.join("plt_index.json");
    if !index_path.exists() {
        return Err(format!("Missing 'plt_index.json' in '{}'", output_dir).into());
    }

    let index_content = fs::read_to_string(&index_path)?;
    let slots: HashMap<String, usize> = serde_json::from_str(&index_content)?;

    let elf_data = fs::read(linked_elf_path)?;
    let file = object::File::parse(&*elf_data)?;

    let vp_section = file
        .sections()
        .find(|s| s.name().map(|n| n == ".__sseocb_g_vP").unwrap_or(false))
        .ok_or_else(|| "Section '.__sseocb_g_vP' not found in linked ELF".to_string())?;

    let vp_base = vp_section.address();

    let mut symbol_map = HashMap::new();
    for sym in file.symbols() {
        if let Ok(name) = sym.name() {
            symbol_map.insert(name.to_string(), sym.address());
        }
    }

    let mut entries: Vec<(&String, &usize)> = slots.iter().collect();
    entries.sort_by_key(|(_, slot)| *slot);

    let mut s_content = String::from(
        ".section .rodata\n\
        .global __sseocb_i_plt_vP\n\
        __sseocb_i_plt_vP:\n",
    );

    for (sym_name, slot) in entries {
        let sym_addr = symbol_map.get(sym_name).ok_or_else(|| {
            format!(
                "Symbol '{}' for slot {} not found in linked ELF",
                sym_name, slot
            )
        })?;

        if *sym_addr < vp_base {
            return Err(format!(
                "Symbol '{}' address 0x{:08x} is below __sseocb_g_vP base 0x{:08x}",
                sym_name, sym_addr, vp_base
            )
            .into());
        }

        let offset = sym_addr - vp_base;
        println!(
            "Slot {}: '{}' at 0x{:08x} (vP offset: 0x{:08x})",
            slot, sym_name, sym_addr, offset
        );

        s_content.push_str(&format!(
            "    .word 0x{:08x} /* slot {}: {} */\n",
            offset, slot, sym_name
        ));
    }

    let plt_vp_path = out_dir.join("plt_vp.s");
    fs::write(&plt_vp_path, s_content)?;
    println!("Generated '{}'", plt_vp_path.display());

    let vp_data = vp_section.data()?;
    let vp_raw_path = out_dir.join("vP_raw.bin");
    fs::write(&vp_raw_path, vp_data)?;
    println!(
        "Extracted '.__sseocb_g_vP' ({} bytes) -> '{}'",
        vp_data.len(),
        vp_raw_path.display()
    );

    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: tool <pre|post|promote|remap> <args...>");
        eprintln!("  tool pre <input.o> <output_dir> [--vpbase <addr>] [--vpsize <size>]");
        eprintln!("  tool post <linked_o> <output_dir>");
        eprintln!("  tool promote <input.o> <output.o> [rv32i|rv32e]");
        eprintln!("  tool remap <input.o> <output.o>");
        std::process::exit(1);
    }

    match args[1].as_str() {
        "promote" => {
            if args.len() < 4 {
                eprintln!("Usage: tool promote <input.o> <output.o> [rv32i|rv32e]");
                std::process::exit(1);
            }
            let target_arch = args.get(4).map(|s| s.as_str());
            run_promote(&args[2], &args[3], target_arch)?;
        }
        "remap" => {
            if args.len() < 4 {
                eprintln!("Usage: tool remap <input.o> <output.o>");
                std::process::exit(1);
            }
            run_remap(&args[2], &args[3])?;
        }
        "pre" => {
            if args.len() < 4 {
                eprintln!(
                    "Usage: tool pre <input.o> <output_dir> [--vpbase <addr>] [--vpsize <size>]"
                );
                std::process::exit(1);
            }

            let mut vpbase = 0xFFF0_0000u64; // Default: 4 GiB - 1 MiB
            let mut vpsize = 0x0010_0000u64; // Default: 1 MiB

            let mut i = 4;
            while i < args.len() {
                match args[i].as_str() {
                    "--vpbase" | "--vp-base" => {
                        i += 1;
                        if i >= args.len() {
                            eprintln!("Missing value for --vpbase");
                            std::process::exit(1);
                        }
                        vpbase = parse_addr(&args[i])?;
                    }
                    "--vpsize" | "--vp-size" => {
                        i += 1;
                        if i >= args.len() {
                            eprintln!("Missing value for --vpsize");
                            std::process::exit(1);
                        }
                        vpsize = parse_size(&args[i])?;
                    }
                    other => {
                        eprintln!("Unknown option '{}'", other);
                        std::process::exit(1);
                    }
                }
                i += 1;
            }

            run_pre(&args[2], &args[3], vpbase, vpsize)?;
        }
        "post" => {
            if args.len() < 4 {
                eprintln!("Usage: tool post <linked_o> <output_dir>");
                std::process::exit(1);
            }
            run_post(&args[2], &args[3])?;
        }
        other => {
            eprintln!(
                "Unknown command: '{}'. Expected 'pre', 'post', 'promote', or 'remap'",
                other
            );
            std::process::exit(1);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remap_register() {
        let mut r = 0;
        remap_register(&mut r).unwrap();
        assert_eq!(r, 0); // x0 is unchanged

        let mut r = 1;
        remap_register(&mut r).unwrap();
        assert_eq!(r, 16); // x1 -> x16

        let mut r = 15;
        remap_register(&mut r).unwrap();
        assert_eq!(r, 30); // x15 -> x30

        let mut r = 16;
        assert!(remap_register(&mut r).is_err()); // x16 rejected

        let mut r = 31;
        assert!(remap_register(&mut r).is_err()); // x31 rejected
    }

    #[test]
    fn test_remap_instruction_r_type() {
        let mut instr = Instruction::add {
            rd: 1,
            rs1: 2,
            rs2: 3,
        };
        remap_instruction(&mut instr).unwrap();
        assert_eq!(
            instr,
            Instruction::add {
                rd: 16,
                rs1: 17,
                rs2: 18
            }
        );

        let mut instr_zero = Instruction::sub {
            rd: 0,
            rs1: 1,
            rs2: 15,
        };
        remap_instruction(&mut instr_zero).unwrap();
        assert_eq!(
            instr_zero,
            Instruction::sub {
                rd: 0,
                rs1: 16,
                rs2: 30
            }
        );
    }

    #[test]
    fn test_remap_instruction_i_type() {
        let mut instr = Instruction::addi {
            rd: 1,
            rs1: 0,
            imm: 42,
        };
        remap_instruction(&mut instr).unwrap();
        assert_eq!(
            instr,
            Instruction::addi {
                rd: 16,
                rs1: 0,
                imm: 42
            }
        );

        let mut instr_load = Instruction::lw {
            rd: 10,
            rs1: 2,
            imm: -4,
        };
        remap_instruction(&mut instr_load).unwrap();
        assert_eq!(
            instr_load,
            Instruction::lw {
                rd: 25,
                rs1: 17,
                imm: -4
            }
        );

        let mut instr_shift = Instruction::slli {
            rd: 15,
            rs1: 5,
            shamt: 2,
        };
        remap_instruction(&mut instr_shift).unwrap();
        assert_eq!(
            instr_shift,
            Instruction::slli {
                rd: 30,
                rs1: 20,
                shamt: 2
            }
        );
    }

    #[test]
    fn test_remap_instruction_s_type() {
        let mut instr = Instruction::sw {
            rs1: 2,
            rs2: 1,
            imm: 12,
        };
        remap_instruction(&mut instr).unwrap();
        assert_eq!(
            instr,
            Instruction::sw {
                rs1: 17,
                rs2: 16,
                imm: 12
            }
        );
    }

    #[test]
    fn test_remap_instruction_b_type() {
        let mut instr = Instruction::beq {
            rs1: 1,
            rs2: 2,
            imm: -8,
        };
        remap_instruction(&mut instr).unwrap();
        assert_eq!(
            instr,
            Instruction::beq {
                rs1: 16,
                rs2: 17,
                imm: -8
            }
        );
    }

    #[test]
    fn test_remap_instruction_u_and_j_type() {
        let mut instr_lui = Instruction::lui {
            rd: 15,
            imm: 0x12345000,
        };
        remap_instruction(&mut instr_lui).unwrap();
        assert_eq!(
            instr_lui,
            Instruction::lui {
                rd: 30,
                imm: 0x12345000
            }
        );

        let mut instr_jal = Instruction::jal { rd: 1, imm: 100 };
        remap_instruction(&mut instr_jal).unwrap();
        assert_eq!(instr_jal, Instruction::jal { rd: 16, imm: 100 });
    }

    #[test]
    fn test_remap_instruction_system() {
        let mut fence = Instruction::fence;
        remap_instruction(&mut fence).unwrap();
        assert_eq!(fence, Instruction::fence);

        let mut ebreak = Instruction::ebreak;
        remap_instruction(&mut ebreak).unwrap();
        assert_eq!(ebreak, Instruction::ebreak);
    }

    #[test]
    fn test_remap_instruction_rejects_above_x15() {
        let mut instr = Instruction::add {
            rd: 16,
            rs1: 1,
            rs2: 2,
        };
        assert!(remap_instruction(&mut instr).is_err());

        let mut instr = Instruction::lw {
            rd: 1,
            rs1: 20,
            imm: 0,
        };
        assert!(remap_instruction(&mut instr).is_err());
    }
}
