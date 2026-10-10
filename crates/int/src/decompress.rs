use crate::asm;
use crate::config;

pub trait Decompressor {
    fn decompress_u32(vp_offset: u32) -> u32;
}

pub fn is_u32_inside_vp(vp_offset: u32) -> bool {
    vp_offset <= config::G_VPSIZE - 4 && (vp_offset & 3) == 0
}

// Noop implementation of D() which works with uncompressed vP.
pub struct NoopDecompressor;

impl Decompressor for NoopDecompressor {
    fn decompress_u32(vp_offset: u32) -> u32 {
        if !is_u32_inside_vp(vp_offset) {
            asm::emit_ebreak();
            loop {}
        }

        let ptr = (config::I_VPBASE as usize + vp_offset as usize) as *const u32;
        unsafe { *ptr }
    }
}
