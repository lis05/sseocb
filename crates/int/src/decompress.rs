use crate::asm;
use crate::int;

pub trait Decompressor {
    fn decompress_u32(vp_offset: u32) -> u32;
}

#[inline(always)]
pub fn is_u32_inside_vp(vp_offset: u32) -> bool {
    vp_offset <= int::g_vpsize() - 4 && (vp_offset & 3) == 0
}

// Noop implementation of D() which works with uncompressed vP.
pub struct NoopDecompressor;

impl Decompressor for NoopDecompressor {
    fn decompress_u32(vp_offset: u32) -> u32 {
        if !is_u32_inside_vp(vp_offset) {
            asm::emit_ebreak();
            loop {}
        }

        let ptr = (int::i_vpbase() as usize + vp_offset as usize) as *const u32;
        unsafe { *ptr }
    }
}

#[cfg(feature = "noop-dec")]
pub type ChosenDecompressor = NoopDecompressor;

#[cfg(feature = "blaris-dec")]
pub struct BlarisDecompressor;

#[cfg(feature = "blaris-dec")]
impl Decompressor for BlarisDecompressor {
    #[unsafe(link_section = ".__sseocb_i.decompressor")]
    fn decompress_u32(vp_offset: u32) -> u32 {
        if !is_u32_inside_vp(vp_offset) {
            asm::emit_ebreak();
            loop {}
        }

        let base = int::i_vpbase() as *const u8;
        let end = int::i_vpend() as *const u8;
        let len = (end as usize).saturating_sub(base as usize);
        let from = unsafe { core::slice::from_raw_parts(base, len) };

        let mut buf = [0u8; 4];
        let ok = blaris_decompress::decompress::decompress(from, &mut buf, vp_offset as usize);
        if !ok {
            asm::emit_ebreak();
            loop {}
        }

        u32::from_le_bytes(buf)
    }
}

#[cfg(feature = "blaris-dec")]
pub type ChosenDecompressor = BlarisDecompressor;

#[cfg(feature = "graemi-dec")]
pub struct GraemiDecompressor;

#[cfg(feature = "graemi-dec")]
impl Decompressor for GraemiDecompressor {
    #[unsafe(link_section = ".__sseocb_i.decompressor")]
    fn decompress_u32(vp_offset: u32) -> u32 {
        if !is_u32_inside_vp(vp_offset) {
            asm::emit_ebreak();
            loop {}
        }

        let base = int::i_vpbase() as *const u8;
        let end = int::i_vpend() as *const u8;
        let len = (end as usize).saturating_sub(base as usize);
        let from = unsafe { core::slice::from_raw_parts(base, len) };

        let mut buf = [0u8; 4];
        let ok = graemi_decompress::decompress(from, &mut buf, vp_offset as usize);
        if !ok {
            asm::emit_ebreak();
            loop {}
        }

        u32::from_le_bytes(buf)
    }
}

#[cfg(feature = "graemi-dec")]
pub type ChosenDecompressor = GraemiDecompressor;
