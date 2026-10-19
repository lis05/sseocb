use crate::asm;
use crate::int;

pub trait Decompressor {
    fn decompress(vp_offset: u32, buf: &mut [u8]);
}

#[inline(always)]
pub fn is_slice_inside_vp(vp_offset: u32, len: usize) -> bool {
    let vpsize = int::g_vpsize() as usize;
    (vp_offset as usize)
        .checked_add(len)
        .is_some_and(|end| end <= vpsize)
}

// Noop implementation of D() which works with uncompressed vP.
pub struct NoopDecompressor;

impl Decompressor for NoopDecompressor {
    fn decompress(vp_offset: u32, buf: &mut [u8]) {
        if !is_slice_inside_vp(vp_offset, buf.len()) {
            asm::emit_ebreak();
            loop {}
        }

        let ptr = (int::i_vpbase() as usize + vp_offset as usize) as *const u8;
        unsafe {
            core::ptr::copy_nonoverlapping(ptr, buf.as_mut_ptr(), buf.len());
        }
    }
}

#[cfg(feature = "noop-dec")]
pub type ChosenDecompressor = NoopDecompressor;

#[cfg(feature = "blaris-dec")]
pub struct BlarisDecompressor;

#[cfg(feature = "blaris-dec")]
impl Decompressor for BlarisDecompressor {
    #[unsafe(link_section = ".__sseocb_i.decompressor")]
    fn decompress(vp_offset: u32, buf: &mut [u8]) {
        if !is_slice_inside_vp(vp_offset, buf.len()) {
            asm::emit_ebreak();
            loop {}
        }

        let base = int::i_vpbase() as *const u8;
        let end = int::i_vpend() as *const u8;
        let len = (end as usize).saturating_sub(base as usize);
        let from = unsafe { core::slice::from_raw_parts(base, len) };

        let ok = blaris_decompress::decompress::decompress(from, buf, vp_offset as usize);
        if !ok {
            asm::emit_ebreak();
            loop {}
        }
    }
}

#[cfg(feature = "blaris-dec")]
pub type ChosenDecompressor = BlarisDecompressor;

#[cfg(feature = "graemi-dec")]
pub struct GraemiDecompressor;

#[cfg(feature = "graemi-dec")]
impl Decompressor for GraemiDecompressor {
    #[unsafe(link_section = ".__sseocb_i.decompressor")]
    fn decompress(vp_offset: u32, buf: &mut [u8]) {
        if !is_slice_inside_vp(vp_offset, buf.len()) {
            asm::emit_ebreak();
            loop {}
        }

        let base = int::i_vpbase() as *const u8;
        let end = int::i_vpend() as *const u8;
        let len = (end as usize).saturating_sub(base as usize);
        let from = unsafe { core::slice::from_raw_parts(base, len) };

        let ok = graemi_decompress::decompress(from, buf, vp_offset as usize);
        if !ok {
            asm::emit_ebreak();
            loop {}
        }
    }
}

#[cfg(feature = "graemi-dec")]
pub type ChosenDecompressor = GraemiDecompressor;
