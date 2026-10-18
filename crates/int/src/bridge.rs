//! This file consists of helpers to bridge between int and guest code.

unsafe extern "C" {
    fn __sseocb_i_read_guest_reg(reg: u32) -> u32;
    fn __sseocb_i_write_guest_reg(reg: u32, value: u32);
    pub fn __sseocb_i_native_call(addr: u32);
    static __sseocb_i_plt_return: u8;
    static __sseocb_i_plt_offsets: u8;
    static __sseocb_i_trampolines: u8;
    static __sseocb_i_trampolines_end: u8;
}

pub fn read_guest_reg(reg: usize) -> u32 {
    unsafe { __sseocb_i_read_guest_reg(reg as u32) }
}

pub fn write_guest_reg(reg: usize, value: u32) {
    unsafe { __sseocb_i_write_guest_reg(reg as u32, value) }
}
#[macro_export]
macro_rules! native_call {
    ($target:expr) => {
        unsafe {
            core::arch::asm!(
                "jalr x1, {stub}", // x16
                stub = in(reg) $crate::bridge::__sseocb_i_native_call,
                in("x10") $target,
                out("x1") _,
            );
        }
    };
}
pub use native_call;

#[inline(always)]
pub fn plt_return_addr() -> u32 {
    core::ptr::addr_of!(__sseocb_i_plt_return) as u32
}

#[inline(always)]
pub fn is_trampoline(target_pc: u32) -> bool {
    return (target_pc & 3) == 0
        && core::ptr::addr_of!(__sseocb_i_trampolines) as u32 <= target_pc
        && target_pc < core::ptr::addr_of!(__sseocb_i_trampolines_end) as u32;
}

#[inline(always)]
pub fn get_trampoline_vp_offset(target_pc: u32) -> u32 {
    // Assuming it is a trampoline
    // NOTE: this works for 2^18 unique trampolines; enough in my opinion...
    let offset = target_pc - core::ptr::addr_of!(__sseocb_i_trampolines) as u32;
    let ptr = (core::ptr::addr_of!(__sseocb_i_plt_offsets) as u32 + offset) as *const u32;
    return unsafe { *ptr };
}
