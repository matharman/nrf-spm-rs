#![no_main]
#![no_std]

use defmt;
use nrf_spm_rs as _;

#[cortex_m_rt::entry]
fn main() -> ! {
    defmt::println!("Hello, world!");
    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
