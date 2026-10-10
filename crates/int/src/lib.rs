#![no_std]

pub mod asm;
pub mod config {
    include!(concat!(env!("OUT_DIR"), "/config.rs"));
}
pub mod decompress;

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    asm::emit_ebreak();
    loop {}
}

#[unsafe(no_mangle)]
pub extern "C" fn __sseocb_i_main() {
    let _ = rv32_isa::parse(0x00000013);
    asm::emit_ebreak();
    loop {}
}
