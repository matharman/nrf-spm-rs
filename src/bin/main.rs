#![no_main]
#![no_std]

use nrf_spm_rs::{configure_secure_regions, non_secure_jump};

#[cortex_m_rt::entry]
fn main() -> ! {
    let ns_jump_addr = 0x8000;
    const NS_FLASH_KB: usize = 32;
    const NS_RAM_KB: usize = 32;

    configure_secure_regions(NS_FLASH_KB, NS_RAM_KB);
    non_secure_jump(ns_jump_addr);
}
