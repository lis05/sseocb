use std::env;
use std::fs;
use std::path::Path;

fn parse_u32(var_name: &str, s: &str) -> u32 {
    let s = s.trim().replace('_', "");
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        u32::from_str_radix(hex, 16)
            .unwrap_or_else(|e| panic!("invalid hex in {var_name} '{s}': {e}"))
    } else {
        s.parse::<u32>()
            .unwrap_or_else(|e| panic!("invalid integer in {var_name} '{s}': {e}"))
    }
}

fn main() {
    println!("cargo:rerun-if-env-changed=SSEOCB_G_VPBASE");
    println!("cargo:rerun-if-env-changed=SSEOCB_G_VPSIZE");
    println!("cargo:rerun-if-env-changed=SSEOCB_I_VPBASE");
    println!("cargo:rerun-if-env-changed=SSEOCB_I_VPSIZE");

    let g_vpbase_raw =
        env::var("SSEOCB_G_VPBASE").expect("Environment variable SSEOCB_G_VPBASE must be set");
    let g_vpsize_raw =
        env::var("SSEOCB_G_VPSIZE").expect("Environment variable SSEOCB_G_VPSIZE must be set");
    let i_vpbase_raw =
        env::var("SSEOCB_I_VPBASE").expect("Environment variable SSEOCB_I_VPBASE must be set");
    let i_vpsize_raw =
        env::var("SSEOCB_I_VPSIZE").expect("Environment variable SSEOCB_I_VPSIZE must be set");

    let g_vpbase = parse_u32("SSEOCB_G_VPBASE", &g_vpbase_raw);
    let g_vpsize = parse_u32("SSEOCB_G_VPSIZE", &g_vpsize_raw);
    let i_vpbase = parse_u32("SSEOCB_I_VPBASE", &i_vpbase_raw);
    let i_vpsize = parse_u32("SSEOCB_I_VPSIZE", &i_vpsize_raw);

    let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set by cargo");
    let dest_path = Path::new(&out_dir).join("config.rs");

    let content = format!(
        "pub const G_VPBASE: u32 = 0x{g_vpbase:08x};\n\
         pub const G_VPSIZE: u32 = 0x{g_vpsize:08x};\n\
         pub const I_VPBASE: u32 = 0x{i_vpbase:08x};\n\
         pub const I_VPSIZE: u32 = 0x{i_vpsize:08x};\n"
    );

    fs::write(dest_path, content).expect("failed to write config.rs");
}
