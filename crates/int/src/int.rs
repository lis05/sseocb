unsafe extern "C" {
    // Where vP is in guest view
    static __sseocb_g_vpbase: u8;
    static __sseocb_g_vpsize: u8;
    // Where vP is in interpreter view (Flash)
    static __sseocb_i_vpbase: u8;
    static __sseocb_i_vpend: u8;
    static __sseocb_i_vpsize: u8;
}

#[inline(always)]
pub fn g_vpbase() -> u32 {
    core::ptr::addr_of!(__sseocb_g_vpbase) as usize as u32
}

#[inline(always)]
pub fn g_vpsize() -> u32 {
    core::ptr::addr_of!(__sseocb_g_vpsize) as usize as u32
}

#[inline(always)]
pub fn i_vpbase() -> u32 {
    core::ptr::addr_of!(__sseocb_i_vpbase) as usize as u32
}

#[inline(always)]
pub fn i_vpend() -> u32 {
    core::ptr::addr_of!(__sseocb_i_vpend) as usize as u32
}

#[inline(always)]
pub fn i_vpsize() -> u32 {
    core::ptr::addr_of!(__sseocb_i_vpsize) as usize as u32
}
