use crate::asm;
use crate::int;

pub trait Decompressor {
    fn decompress_u32(int: &int::Interpreter, vp_offset: u32) -> u32;
}

#[inline(always)]
pub fn is_u32_inside_vp(int: &int::Interpreter, vp_offset: u32) -> bool {
    vp_offset <= int.g_vpsize - 4 && (vp_offset & 3) == 0
}

// Noop implementation of D() which works with uncompressed vP.
pub struct NoopDecompressor;

impl Decompressor for NoopDecompressor {
    fn decompress_u32(int: &int::Interpreter, vp_offset: u32) -> u32 {
        if !is_u32_inside_vp(int, vp_offset) {
            asm::emit_ebreak();
            loop {}
        }

        let ptr = (int.i_vpbase as usize + vp_offset as usize) as *const u32;
        unsafe { *ptr }
    }
}

#[cfg(feature = "noop-dec")]
pub type ChosenDecompressor = NoopDecompressor;
