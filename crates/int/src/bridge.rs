//! This file consists of helpers to bridge between int and guest code.

unsafe extern "C" {
    fn __sseocb_i_read_guest_reg(reg: u32) -> u32;
    fn __sseocb_i_write_guest_reg(reg: u32, value: u32);
    pub fn __sseocb_i_native_call(addr: u32);
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
                "jalr x1, {stub}",
                stub = in(reg) $crate::bridge::__sseocb_i_native_call,
                in("x10") $target,
                options(nostack)
            );
        }
    };
}
pub use native_call;
