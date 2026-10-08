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
    let mut trampolines = String::from(".section .text.sseocb.plt, \"ax\", @progbits\n\n");
    let mut linker = String::new();

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

        linker.push_str(&format!("{sym} = {plt_sym};\n"));
    }

    fs::write(dir.join("trampolines.s"), trampolines)?;
    fs::write(dir.join("plt.ld"), linker)?;

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

#[allow(non_snake_case)]
fn collect_sections_for_vP(
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
    collect_sections_for_vP(&file, output_dir, vpbase, vpsize)?;

    let mut slots_json = serde_json::to_string_pretty(&addressable_symbols)?;
    slots_json.push('\n');
    fs::write(output_dir.join("plt_index.json"), slots_json)?;

    println!(
        "Generated trampolines.s, plt.ld, virtual.ld, plt_index.json in '{}'",
        output_dir.display()
    );

    Ok(())
}

fn run_post(_linked_elf_path: &str, _output_dir: &str) -> Result<(), Box<dyn Error>> {
    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: guest-tool <pre|post> <args...>");
        eprintln!("  guest-tool pre <input.o> <output_dir> [--vpbase <addr>] [--vpsize <size>]");
        eprintln!("  guest-tool post <linked_o> <output_dir>");
        std::process::exit(1);
    }

    match args[1].as_str() {
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
            eprintln!("Unknown command: '{}'. Expected 'pre' or 'post'", other);
            std::process::exit(1);
        }
    }

    Ok(())
}
