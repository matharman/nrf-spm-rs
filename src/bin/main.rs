#![no_main]
#![no_std]

use cortex_m_rt::exception;
use nrf_spm_rs::{configure_secure_regions, non_secure_jump};

#[cortex_m_rt::exception]
fn SecureFault() {
    defmt::panic!("received secure fault");
}

#[cortex_m_rt::entry]
fn main() -> ! {
    const NS_FLASH_KB: usize = 64;
    const NS_RAM_KB: usize = 16;

    configure_secure_regions(NS_FLASH_KB, NS_RAM_KB);
    non_secure_jump((NS_FLASH_KB * 1024) as u32);
}
