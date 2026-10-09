use object::elf;
use object::{Object, ObjectSection, ObjectSymbol, SectionKind};
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

fn run_promote(input_path: &str, output_path: &str) -> Result<(), Box<dyn Error>> {
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
    let new_flags = orig_flags & !elf::EF_RISCV_RVE.0;
    data[flags_offset..flags_offset + 4].copy_from_slice(&new_flags.to_le_bytes());

    fs::write(output_path, data)?;
    println!(
        "Promoted '{}' -> '{}' (cleared EF_RISCV_RVE: 0x{:x} -> 0x{:x})",
        input_path, output_path, orig_flags, new_flags
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
        eprintln!("Usage: guest-tool <pre|post|promote> <args...>");
        eprintln!("  guest-tool pre <input.o> <output_dir> [--vpbase <addr>] [--vpsize <size>]");
        eprintln!("  guest-tool post <linked_o> <output_dir>");
        eprintln!("  guest-tool promote <input.o> <output.o>");
        std::process::exit(1);
    }

    match args[1].as_str() {
        "promote" => {
            if args.len() < 4 {
                eprintln!("Usage: guest-tool promote <input.o> <output.o>");
                std::process::exit(1);
            }
            run_promote(&args[2], &args[3])?;
        }
        "pre" => {
            if args.len() < 4 {
                eprintln!(
                    "Usage: guest-tool pre <input.o> <output_dir> [--vpbase <addr>] [--vpsize <size>]"
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
                eprintln!("Usage: guest-tool post <linked_o> <output_dir>");
                std::process::exit(1);
            }
            run_post(&args[2], &args[3])?;
        }
        other => {
            eprintln!(
                "Unknown command: '{}'. Expected 'pre', 'post', or 'promote'",
                other
            );
            std::process::exit(1);
        }
    }

    Ok(())
}
