use std::collections::HashSet;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::process;

use sim::cpu::{CPU, Error};
use sim::memory::{Permissions, bus::Bus, region::Region};

fn main() {
    let mut args: Vec<String> = env::args().collect();
    // Allow `cargo run sim ...` where cargo passes the binary name as an argument
    if args.len() > 1 && args[1] == "sim" {
        args.remove(1);
    }

    if args.len() == 3 && args[1] == "accept" {
        let path = &args[2];
        match run_accept(path) {
            Ok(count) => {
                println!(
                    "Accepted: successfully decoded {} instructions from {}",
                    count, path
                );
            }
            Err(err) => {
                eprintln!("Error: {}", err);
                process::exit(1);
            }
        }
        return;
    }

    if args.len() > 1 && args[1] == "export-script" {
        match parse_run_args(&args[2..]) {
            Ok(cfg) => match execute_export_script(&cfg) {
                Ok(()) => return,
                Err(err) => {
                    eprintln!("Error: {}", err);
                    process::exit(1);
                }
            },
            Err(err) => {
                eprintln!("Error: {}", err);
                process::exit(1);
            }
        }
    }

    if args.len() > 1 && args[1] == "run" {
        match parse_run_args(&args[2..]) {
            Ok(cfg) => match execute_run(&cfg) {
                Ok(()) => return,
                Err(err) => {
                    eprintln!("Error: {}", err);
                    process::exit(1);
                }
            },
            Err(err) => {
                eprintln!("Error: {}", err);
                process::exit(1);
            }
        }
    }

    if args.len() > 1 && args[1] == "debug" {
        match parse_run_args(&args[2..]) {
            Ok(cfg) => match execute_debug(&cfg) {
                Ok(()) => return,
                Err(err) => {
                    eprintln!("Error: {}", err);
                    process::exit(1);
                }
            },
            Err(err) => {
                eprintln!("Error: {}", err);
                process::exit(1);
            }
        }
    }

    if args.len() == 1 {
        println!("RV32I Simulator");
        return;
    }

    if args.len() == 2 && (args[1] == "-h" || args[1] == "--help") {
        print_usage();
        return;
    }

    print_usage();
    process::exit(1);
}

fn print_usage() {
    eprintln!("Usage:");
    eprintln!("  sim accept <file>");
    eprintln!("  sim export-script [OPTIONS]");
    eprintln!("  sim run [OPTIONS] <file>");
    eprintln!("  sim debug [OPTIONS] <file>");
    eprintln!();
    eprintln!("Options:");
    eprintln!("  --flash-base <ADDR>   Flash base address (default: 1M / 0x00100000)");
    eprintln!("  --flash-len <SIZE>    Flash length (default: 32K / 0x00008000)");
    eprintln!("  --sram-base <ADDR>    SRAM base address (default: 2M / 0x00200000)");
    eprintln!("  --sram-len <SIZE>     SRAM length (default: 32K / 0x00008000)");
    eprintln!("  --stack-base <ADDR>   Stack base address (default: 3M / 0x00300000)");
    eprintln!("  --stack-len <SIZE>    Stack length (default: 4K / 0x00001000)");
    eprintln!("  --uart-base <ADDR>    UART MMIO address (default: 1G / 0x40000000)");
    eprintln!(
        "  --max-cycles <CYCLES> Maximum cycle limit (default: 10M / 10000000, 0 for unlimited)"
    );
    eprintln!();
    eprintln!(
        "Values support hex ('0x...'), decimal, and suffixes ('K', 'M', 'G', 'KiB', 'MiB', 'GiB')."
    );
}

#[derive(Debug, PartialEq, Eq)]
struct RunConfig {
    flash_base: u32,
    flash_len: u32,
    sram_base: u32,
    sram_len: u32,
    stack_base: u32,
    stack_len: u32,
    uart_base: u32,
    max_cycles: u64,
    program_path: Option<String>,
}

impl Default for RunConfig {
    fn default() -> Self {
        Self {
            flash_base: 0x0010_0000, // 1M
            flash_len: 0x8000,       // 32K
            sram_base: 0x0020_0000,  // 2M
            sram_len: 0x8000,        // 32K
            stack_base: 0x0030_0000, // 3M
            stack_len: 0x1000,       // 4K
            uart_base: 0x4000_0000,  // 1G
            max_cycles: 10_000_000,  // 10M
            program_path: None,
        }
    }
}

fn parse_u64_with_suffix(s: &str) -> Result<u64, String> {
    let s = s.trim();
    if s.is_empty() {
        return Err("empty value".to_string());
    }

    let (num_str, multiplier) = if let Some(stripped) = s
        .strip_suffix("GiB")
        .or_else(|| s.strip_suffix('G'))
        .or_else(|| s.strip_suffix('g'))
    {
        (stripped, 1024 * 1024 * 1024u64)
    } else if let Some(stripped) = s
        .strip_suffix("MiB")
        .or_else(|| s.strip_suffix('M'))
        .or_else(|| s.strip_suffix('m'))
    {
        (stripped, 1024 * 1024u64)
    } else if let Some(stripped) = s
        .strip_suffix("KiB")
        .or_else(|| s.strip_suffix('K'))
        .or_else(|| s.strip_suffix('k'))
    {
        (stripped, 1024u64)
    } else {
        (s, 1u64)
    };

    let base_val = if let Some(hex) = num_str
        .strip_prefix("0x")
        .or_else(|| num_str.strip_prefix("0X"))
    {
        u64::from_str_radix(hex, 16).map_err(|e| format!("invalid hex '{}': {}", num_str, e))?
    } else {
        num_str
            .parse::<u64>()
            .map_err(|e| format!("invalid number '{}': {}", num_str, e))?
    };

    let total = base_val
        .checked_mul(multiplier)
        .ok_or_else(|| format!("value '{}' overflows integer size", s))?;
    Ok(total)
}

fn parse_size_or_addr(s: &str) -> Result<u32, String> {
    let val = parse_u64_with_suffix(s)?;
    u32::try_from(val).map_err(|_| format!("value '{}' exceeds 32-bit address space", s))
}

fn parse_run_args(args: &[String]) -> Result<RunConfig, String> {
    let mut cfg = RunConfig::default();
    let mut i = 0;

    while i < args.len() {
        let arg = &args[i];
        if arg == "--flash-base" {
            i += 1;
            if i >= args.len() {
                return Err("missing value for --flash-base".to_string());
            }
            cfg.flash_base = parse_size_or_addr(&args[i])?;
            i += 1;
        } else if let Some(val) = arg.strip_prefix("--flash-base=") {
            cfg.flash_base = parse_size_or_addr(val)?;
            i += 1;
        } else if arg == "--flash-len" {
            i += 1;
            if i >= args.len() {
                return Err("missing value for --flash-len".to_string());
            }
            cfg.flash_len = parse_size_or_addr(&args[i])?;
            i += 1;
        } else if let Some(val) = arg.strip_prefix("--flash-len=") {
            cfg.flash_len = parse_size_or_addr(val)?;
            i += 1;
        } else if arg == "--sram-base" {
            i += 1;
            if i >= args.len() {
                return Err("missing value for --sram-base".to_string());
            }
            cfg.sram_base = parse_size_or_addr(&args[i])?;
            i += 1;
        } else if let Some(val) = arg.strip_prefix("--sram-base=") {
            cfg.sram_base = parse_size_or_addr(val)?;
            i += 1;
        } else if arg == "--sram-len" {
            i += 1;
            if i >= args.len() {
                return Err("missing value for --sram-len".to_string());
            }
            cfg.sram_len = parse_size_or_addr(&args[i])?;
            i += 1;
        } else if let Some(val) = arg.strip_prefix("--sram-len=") {
            cfg.sram_len = parse_size_or_addr(val)?;
            i += 1;
        } else if arg == "--stack-base" {
            i += 1;
            if i >= args.len() {
                return Err("missing value for --stack-base".to_string());
            }
            cfg.stack_base = parse_size_or_addr(&args[i])?;
            i += 1;
        } else if let Some(val) = arg.strip_prefix("--stack-base=") {
            cfg.stack_base = parse_size_or_addr(val)?;
            i += 1;
        } else if arg == "--stack-len" {
            i += 1;
            if i >= args.len() {
                return Err("missing value for --stack-len".to_string());
            }
            cfg.stack_len = parse_size_or_addr(&args[i])?;
            i += 1;
        } else if let Some(val) = arg.strip_prefix("--stack-len=") {
            cfg.stack_len = parse_size_or_addr(val)?;
            i += 1;
        } else if arg == "--uart-base" {
            i += 1;
            if i >= args.len() {
                return Err("missing value for --uart-base".to_string());
            }
            cfg.uart_base = parse_size_or_addr(&args[i])?;
            i += 1;
        } else if let Some(val) = arg.strip_prefix("--uart-base=") {
            cfg.uart_base = parse_size_or_addr(val)?;
            i += 1;
        } else if arg == "--max-cycles" {
            i += 1;
            if i >= args.len() {
                return Err("missing value for --max-cycles".to_string());
            }
            cfg.max_cycles = parse_u64_with_suffix(&args[i])?;
            i += 1;
        } else if let Some(val) = arg.strip_prefix("--max-cycles=") {
            cfg.max_cycles = parse_u64_with_suffix(val)?;
            i += 1;
        } else if arg == "-h" || arg == "--help" {
            print_usage();
            process::exit(0);
        } else if arg.starts_with('-') {
            return Err(format!("unknown option '{}'", arg));
        } else if cfg.program_path.is_none() {
            cfg.program_path = Some(arg.clone());
            i += 1;
        } else {
            return Err(format!("unexpected extra argument '{}'", arg));
        }
    }

    Ok(cfg)
}

fn create_bus(cfg: &RunConfig, program_bytes: Option<&[u8]>) -> Result<Bus, String> {
    let mut bus = Bus::new();

    // FLASH
    let mut flash_data = vec![0u8; cfg.flash_len as usize];
    if let Some(bytes) = program_bytes {
        if bytes.len() > flash_data.len() {
            return Err(format!(
                "program size ({} bytes) exceeds flash length ({} bytes)",
                bytes.len(),
                flash_data.len()
            ));
        }
        flash_data[..bytes.len()].copy_from_slice(bytes);
    }
    let flash = Region::try_new("FLASH", cfg.flash_base, Permissions::RX, flash_data)
        .map_err(|e| format!("failed to create FLASH region: {:?}", e))?;
    bus.add(flash)
        .map_err(|e| format!("failed to add FLASH to bus: {:?}", e))?;

    // RAM (SRAM)
    let sram_data = vec![0u8; cfg.sram_len as usize];
    let sram = Region::try_new("RAM", cfg.sram_base, Permissions::RWX, sram_data)
        .map_err(|e| format!("failed to create RAM region: {:?}", e))?;
    bus.add(sram)
        .map_err(|e| format!("failed to add RAM to bus: {:?}", e))?;

    // STACK
    let stack_data = vec![0u8; cfg.stack_len as usize];
    let stack = Region::try_new("STACK", cfg.stack_base, Permissions::RW, stack_data)
        .map_err(|e| format!("failed to create STACK region: {:?}", e))?;
    bus.add(stack)
        .map_err(|e| format!("failed to add STACK to bus: {:?}", e))?;

    // UART (4 bytes aperture for 8-bit, 16-bit, and 32-bit MMIO stores)
    let uart = Region::try_new("UART", cfg.uart_base, Permissions::RW, vec![0u8; 4])
        .map_err(|e| format!("failed to create UART region: {:?}", e))?
        .with_write_proxy(|_addr, buf| {
            std::io::stdout()
                .write_all(buf)
                .map_err(|_| sim::memory::region::Error::AddressViolation)
        });
    bus.add(uart)
        .map_err(|e| format!("failed to add UART to bus: {:?}", e))?;

    Ok(bus)
}

fn execute_export_script(cfg: &RunConfig) -> Result<(), String> {
    let bus = create_bus(cfg, None)?;
    print!("{}", bus.to_linker_script());
    Ok(())
}

fn execute_run(cfg: &RunConfig) -> Result<(), String> {
    let path = cfg
        .program_path
        .as_deref()
        .ok_or_else(|| "missing binary program to execute".to_string())?;

    let bytes = fs::read(path).map_err(|e| format!("failed to read file '{}': {}", path, e))?;

    if bytes.is_empty() {
        return Err("program file is empty".to_string());
    }

    let mut bus = create_bus(cfg, Some(&bytes))?;
    let mut cpu = CPU::new();
    cpu.set_pc(cfg.flash_base);

    let mut cycles_executed = 0u64;

    loop {
        if cfg.max_cycles > 0 && cycles_executed >= cfg.max_cycles {
            let _ = std::io::stdout().flush();
            return Err(format!(
                "execution limit of {} cycles reached (possible infinite loop)",
                cfg.max_cycles
            ));
        }

        match cpu.step(&mut bus) {
            Ok(_) => {
                cycles_executed += 1;
            }
            Err(Error::Ebreak) | Err(Error::Ecall) => {
                let _ = std::io::stdout().flush();
                let exit_code = cpu.read_reg(10);
                if exit_code != 0 {
                    return Err(format!(
                        "guest program exited with non-zero code: {}",
                        exit_code
                    ));
                }
                return Ok(());
            }
            Err(e) => {
                let _ = std::io::stdout().flush();
                return Err(format!(
                    "CPU execution error: {:?} at pc {:#010x}",
                    e,
                    cpu.get_pc()
                ));
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MemSize {
    Byte,
    Half,
    Word,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MemType {
    size: MemSize,
    signed: bool,
    count: usize,
}

fn parse_number(s: &str) -> Option<u32> {
    let s = s.trim();
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        u32::from_str_radix(hex, 16).ok()
    } else if let Some(bin) = s.strip_prefix("0b").or_else(|| s.strip_prefix("0B")) {
        u32::from_str_radix(bin, 2).ok()
    } else if let Ok(val) = s.parse::<u32>() {
        Some(val)
    } else if let Ok(val) = s.parse::<i32>() {
        Some(val as u32)
    } else {
        None
    }
}

fn parse_mem_type(s: &str) -> Option<MemType> {
    let s = s.trim().to_lowercase();
    let mut size = None;
    let mut signed = false;
    let mut unsigned = false;
    let mut num_str = String::new();

    for ch in s.chars() {
        if ch.is_ascii_digit() {
            num_str.push(ch);
        } else {
            match ch {
                'b' => {
                    if size.is_some() {
                        return None;
                    }
                    size = Some(MemSize::Byte);
                }
                'h' => {
                    if size.is_some() {
                        return None;
                    }
                    size = Some(MemSize::Half);
                }
                'w' => {
                    if size.is_some() {
                        return None;
                    }
                    size = Some(MemSize::Word);
                }
                'u' => {
                    if signed || unsigned {
                        return None;
                    }
                    unsigned = true;
                }
                's' => {
                    if signed || unsigned {
                        return None;
                    }
                    signed = true;
                }
                _ => return None,
            }
        }
    }

    let size = size?;
    let count = if num_str.is_empty() {
        1
    } else {
        num_str.parse::<usize>().ok()?
    };

    if count == 0 {
        return None;
    }

    Some(MemType {
        size,
        signed,
        count,
    })
}

fn print_memory_val(addr: u32, mtype: &MemType, bus: &Bus) {
    let perms = Permissions::RO;
    match mtype.size {
        MemSize::Word => match bus.read_u32(addr, perms) {
            Ok(val) => {
                let dec_str = if mtype.signed {
                    (val as i32).to_string()
                } else {
                    val.to_string()
                };
                let hex_str = format!("{:#010x}", val);
                let bin_str = format!("{:032b}", val);
                println!(
                    "*{:#010x}: {:<10}    {:>11}    {}",
                    addr, hex_str, dec_str, bin_str
                );
            }
            Err(e) => println!("*{:#010x}: <read error: {:?}>", addr, e),
        },
        MemSize::Half => match bus.read_u16(addr, perms) {
            Ok(val) => {
                let dec_str = if mtype.signed {
                    (val as i16 as i32).to_string()
                } else {
                    val.to_string()
                };
                let hex_str = format!("{:#06x}", val);
                let bin_str = format!("{:016b}", val);
                println!(
                    "*{:#010x}: {:<10}    {:>11}    {}",
                    addr, hex_str, dec_str, bin_str
                );
            }
            Err(e) => println!("*{:#010x}: <read error: {:?}>", addr, e),
        },
        MemSize::Byte => match bus.read_u8(addr, perms) {
            Ok(val) => {
                let dec_str = if mtype.signed {
                    (val as i8 as i32).to_string()
                } else {
                    val.to_string()
                };
                let hex_str = format!("{:#04x}", val);
                let bin_str = format!("{:08b}", val);
                println!(
                    "*{:#010x}: {:<10}    {:>11}    {}",
                    addr, hex_str, dec_str, bin_str
                );
            }
            Err(e) => println!("*{:#010x}: <read error: {:?}>", addr, e),
        },
    }
}

fn execute_debug(cfg: &RunConfig) -> Result<(), String> {
    let path = cfg
        .program_path
        .as_deref()
        .ok_or_else(|| "missing binary program to debug".to_string())?;

    let bytes = fs::read(path).map_err(|e| format!("failed to read file '{}': {}", path, e))?;

    if bytes.is_empty() {
        return Err("program file is empty".to_string());
    }

    let mut bus = create_bus(cfg, Some(&bytes))?;
    let mut cpu = CPU::new();
    let mut breakpoints = HashSet::new();
    let mut last_cmd = String::new();

    let print_pc = |cpu: &CPU| {
        println!("PC: {:#010x}", cpu.get_pc());
    };

    let print_regions = |bus: &Bus| {
        print!("{}", bus.to_linker_script());
    };

    let print_regs = |cpu: &CPU| {
        let regs = cpu.get_registers();
        for (i, &val) in regs.iter().enumerate() {
            print!("{:>2}: {:#010x}    ", i, val);
            if i % 4 == 3 {
                println!();
            }
        }
    };

    let print_reg = |cpu: &CPU, reg: usize| {
        if reg >= 32 {
            println!("Invalid reg {} (must be 0-31)", reg);
            return;
        }
        let value = cpu.read_reg(reg);
        println!(
            "{:<2}: {:#010x}    {:>11}    {:032b}",
            reg, value, value, value
        );
    };

    cpu.set_pc(cfg.flash_base);

    println!("Debugger: loaded '{}' ({} bytes)", path, bytes.len());
    print_pc(&cpu);
    print_regions(&bus);
    loop {
        print!("(dbg) ");
        let _ = io::stdout().flush();

        let mut line = String::new();
        if io::stdin().read_line(&mut line).unwrap_or(0) == 0 {
            break; // EOF
        }

        let trimmed = line.trim();
        let cmd_str = if trimmed.is_empty() {
            last_cmd // Repeat previous command on empty Enter
        } else {
            trimmed.to_string()
        };
        last_cmd = cmd_str.to_string();

        let parts: Vec<&str> = cmd_str.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        match parts[0] {
            "end" => {
                println!("Stopped");
                break;
            }
            "pc" => {
                print_pc(&cpu);
            }
            "regions" => {
                print_regions(&bus);
            }
            "r" => {
                if parts.len() == 1 {
                    print_regs(&cpu);
                } else {
                    for reg in &parts[1..] {
                        let reg_str = reg
                            .strip_prefix('x')
                            .or_else(|| reg.strip_prefix('X'))
                            .unwrap_or(reg);
                        let Ok(index) = reg_str.parse::<usize>() else {
                            println!("Invalid reg: '{}'", reg);
                            continue;
                        };
                        print_reg(&cpu, index);
                    }
                }
            }
            "rw" => {
                if parts.len() < 3 {
                    println!("Usage: rw <reg> <value>");
                } else {
                    let reg_str = parts[1]
                        .strip_prefix('x')
                        .or_else(|| parts[1].strip_prefix('X'))
                        .unwrap_or(parts[1]);
                    let Ok(index) = reg_str.parse::<usize>() else {
                        println!("Invalid reg: '{}'", parts[1]);
                        continue;
                    };
                    if index >= 32 {
                        println!("Invalid reg {} (must be 0-31)", index);
                        continue;
                    }
                    let Some(val) = parse_number(parts[2]) else {
                        println!("Invalid value: '{}'", parts[2]);
                        continue;
                    };
                    cpu.write_reg(index, val);
                    print_reg(&cpu, index);
                }
            }
            "m" => {
                if parts.len() < 3 {
                    println!("Usage: m <type> <address1> [address2 ...]");
                } else {
                    let Some(mtype) = parse_mem_type(parts[1]) else {
                        println!(
                            "Invalid type '{}'. Use combination of length (w/h/b), sign (u/s), and optional count (e.g. wu16, ws4, hu8, b32)",
                            parts[1]
                        );
                        continue;
                    };
                    let step_bytes = match mtype.size {
                        MemSize::Word => 4u32,
                        MemSize::Half => 2u32,
                        MemSize::Byte => 1u32,
                    };
                    for addr_str in &parts[2..] {
                        let Some(base_addr) = parse_number(addr_str) else {
                            println!("Invalid address: '{}'", addr_str);
                            continue;
                        };
                        for i in 0..mtype.count {
                            let curr_addr = base_addr.wrapping_add(i as u32 * step_bytes);
                            print_memory_val(curr_addr, &mtype, &bus);
                        }
                    }
                }
            }
            "mw" => {
                if parts.len() < 4 {
                    println!("Usage: mw <length> <address> <value>");
                } else {
                    let len_str = parts[1].to_lowercase();
                    let Some(addr) = parse_number(parts[2]) else {
                        println!("Invalid address: '{}'", parts[2]);
                        continue;
                    };
                    let Some(val) = parse_number(parts[3]) else {
                        println!("Invalid value: '{}'", parts[3]);
                        continue;
                    };

                    match len_str.as_str() {
                        "w" => match bus.write_u32(addr, val) {
                            Ok(()) => print_memory_val(
                                addr,
                                &MemType {
                                    size: MemSize::Word,
                                    signed: false,
                                    count: 1,
                                },
                                &bus,
                            ),
                            Err(e) => println!("Memory write error at {:#010x}: {:?}", addr, e),
                        },
                        "h" => match bus.write_u16(addr, val as u16) {
                            Ok(()) => print_memory_val(
                                addr,
                                &MemType {
                                    size: MemSize::Half,
                                    signed: false,
                                    count: 1,
                                },
                                &bus,
                            ),
                            Err(e) => println!("Memory write error at {:#010x}: {:?}", addr, e),
                        },
                        "b" => match bus.write_u8(addr, val as u8) {
                            Ok(()) => print_memory_val(
                                addr,
                                &MemType {
                                    size: MemSize::Byte,
                                    signed: false,
                                    count: 1,
                                },
                                &bus,
                            ),
                            Err(e) => println!("Memory write error at {:#010x}: {:?}", addr, e),
                        },
                        _ => println!(
                            "Invalid length '{}'. Expected 'w' (word), 'h' (halfword), or 'b' (byte)",
                            parts[1]
                        ),
                    }
                }
            }
            "i" => {
                let (count, base_addr) = match parts.len() {
                    1 => (1, cpu.get_pc()),
                    2 => {
                        if let Some(c) = parse_number(parts[1]) {
                            (c as usize, cpu.get_pc())
                        } else {
                            println!("Invalid count: '{}'", parts[1]);
                            continue;
                        }
                    }
                    _ => {
                        let Some(c) = parse_number(parts[1]) else {
                            println!("Invalid count: '{}'", parts[1]);
                            continue;
                        };
                        let Some(addr) = parse_number(parts[2]) else {
                            println!("Invalid address: '{}'", parts[2]);
                            continue;
                        };
                        (c as usize, addr)
                    }
                };

                for idx in 0..count {
                    let addr = base_addr.wrapping_add((idx as u32) * 4);
                    let bp_col = if breakpoints.contains(&addr) {
                        "b "
                    } else {
                        "  "
                    };
                    match bus.read_u32(addr, Permissions::RO) {
                        Ok(raw) => match sim::instruction::parse(raw) {
                            Some(instr) => println!("{}*{:#010x}: {:?}", bp_col, addr, instr),
                            None => {
                                println!(
                                    "{}*{:#010x}: <unknown instruction: {:#010x}>",
                                    bp_col, addr, raw
                                )
                            }
                        },
                        Err(e) => {
                            println!("{}*{:#010x}: <read error: {:?}>", bp_col, addr, e);
                            break;
                        }
                    }
                }
            }
            "b" => {
                let addr = if parts.len() > 1 {
                    if let Some(a) = parse_number(parts[1]) {
                        a
                    } else {
                        println!("Invalid address: '{}'", parts[1]);
                        continue;
                    }
                } else {
                    cpu.get_pc()
                };

                if breakpoints.contains(&addr) {
                    breakpoints.remove(&addr);
                    println!("Breakpoint removed at {:#010x}", addr);
                } else {
                    breakpoints.insert(addr);
                    println!("Breakpoint set at {:#010x}", addr);
                }
            }
            "s" => match cpu.step(&mut bus) {
                Ok(_) => {
                    let pc = cpu.get_pc();
                    if breakpoints.contains(&pc) {
                        println!("breakpoint {:#010x}", pc);
                    } else {
                        print_pc(&cpu);
                    }
                }
                Err(e) => {
                    println!("Halted: {:?}", e);
                    print_pc(&cpu);
                }
            },
            "c" => loop {
                match cpu.step(&mut bus) {
                    Ok(_) => {
                        let pc = cpu.get_pc();
                        if breakpoints.contains(&pc) {
                            println!("breakpoint {:#010x}", pc);
                            break;
                        }
                    }
                    Err(e) => {
                        println!("Halted: {:?}", e);
                        print_pc(&cpu);
                        break;
                    }
                }
            },
            _ => {}
        }
    }
    Ok(())
}

fn run_accept(path: &str) -> Result<usize, String> {
    let bytes = fs::read(path).map_err(|e| format!("failed to read file '{}': {}", path, e))?;

    if bytes.is_empty() {
        return Err("file is empty".to_string());
    }

    if bytes.len() % 4 != 0 {
        return Err(format!(
            "file size ({} bytes) is not a multiple of 4 bytes",
            bytes.len()
        ));
    }

    let mut count = 0;
    for (i, chunk) in bytes.chunks_exact(4).enumerate() {
        let raw = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
        match sim::instruction::parse(raw) {
            Some(_) => count += 1,
            None => {
                let offset = i * 4;
                return Err(format!(
                    "rejected instruction at byte offset {:#010x}: {:#010x}",
                    offset, raw
                ));
            }
        }
    }

    Ok(count)
}
