#![no_main]
#![no_std]

use defmt;

use defmt_rtt as _;
use panic_probe as _;

#[cortex_m_rt::entry]
fn main() -> ! {
    defmt::println!("Hello, non-secure world!");

    loop {
        cortex_m::asm::wfe();
    }
}
