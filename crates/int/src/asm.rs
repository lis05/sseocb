use core::arch::asm;

#[inline(always)]
pub fn emit_ebreak() {
    unsafe {
        asm!("ebreak", options(nomem, nostack, preserves_flags));
    }
}

#[inline(always)]
pub fn crash() -> ! {
    crate::bridge::write_guest_reg(10, !0);
    emit_ebreak();
    loop {}
}
