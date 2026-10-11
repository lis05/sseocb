#[allow(non_snake_case)]
pub struct Interpreter {
    pub vPC: u32,
    pub g_vpbase: u32,
    pub g_vpsize: u32,
    pub i_vpbase: u32,
    pub i_vpsize: u32,
}

unsafe extern "C" {
    // vP offset to the guest entry
    static __sseocb_g_entry: u8;
    // Where vP_compressed is located
    static __sseocb_g_vpbase: u8;
    static __sseocb_g_vpsize: u8;
    // Where vP is in guest view
    static __sseocb_i_vpbase: u8;
    static __sseocb_i_vpsize: u8;
}

impl Interpreter {
    #[inline(always)]
    pub fn new() -> Self {
        Self {
            vPC: core::ptr::addr_of!(__sseocb_g_entry) as usize as u32,
            g_vpbase: core::ptr::addr_of!(__sseocb_g_vpbase) as usize as u32,
            g_vpsize: core::ptr::addr_of!(__sseocb_g_vpsize) as usize as u32,
            i_vpbase: core::ptr::addr_of!(__sseocb_i_vpbase) as usize as u32,
            i_vpsize: core::ptr::addr_of!(__sseocb_i_vpsize) as usize as u32,
        }
    }
}
