//! This file consists of helpers to bridge between int and guest code.

unsafe extern "C" {
    fn __sseocb_i_read_guest_reg(reg: u32) -> u32;
    fn __sseocb_i_write_guest_reg(reg: u32, value: u32);
}

pub fn read_guest_reg(reg: usize) -> u32 {
    unsafe { __sseocb_i_read_guest_reg(reg as u32) }
}

pub fn write_guest_reg(reg: usize, value: u32) {
    unsafe { __sseocb_i_write_guest_reg(reg as u32, value) }
}
