use core::arch::asm;

#[inline(always)]
pub fn emit_ebreak() {
    unsafe {
        asm!("ebreak", options(nomem, nostack, preserves_flags));
    }
}
