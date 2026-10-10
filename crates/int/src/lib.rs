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
