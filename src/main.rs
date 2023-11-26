#![no_main]
#![no_std]
#![feature(abi_c_cmse_nonsecure_call)]

use cortex_m::{
    peripheral::scb::{Exception, FpuAccessMode},
    register::control::{Npriv, Spsel},
};
use cortex_m_rt::exception;
use defmt;
use nrf9160_pac as pac;
// use nrf_spm_rs::{configure_secure_regions, non_secure_jump};

use defmt_rtt as _;

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    cortex_m::interrupt::disable();

    // Make this a hardfault. This has a lot of advantages:
    // - No interrupt can interrupt a hardfault
    // - Recursion cannot happen because the hardware prevents that
    // - It is the natural endpoint for program failures on cortex-m
    cortex_m::asm::udf();
}

#[cortex_m_rt::exception]
fn SecureFault() {
    defmt::panic!("received secure fault");
}

const NON_SECURE_FLASH_OFFSET: usize = 64 * 1024;
const NON_SECURE_RAM_OFFSET: usize = 16 * 1024;

#[cortex_m_rt::entry]
fn main() -> ! {
    // const NS_FLASH_KB: usize = 64;
    // const NS_RAM_KB: usize = 16;
    //
    // configure_secure_regions(NS_FLASH_KB, NS_RAM_KB);
    // non_secure_jump((NS_FLASH_KB * 1024) as u32);

    let chip = pac::Peripherals::take().unwrap();
    let mut cpu = cortex_m::Peripherals::take().unwrap();

    unsafe {
        let spu = chip.SPU_S;

        const FLASH_SIZE: usize = 1024 * 1024;
        const SPU_FLASH_REGION_SIZE: usize = 32 * 1024;

        const RAM_SIZE: usize = 256 * 1024;
        const SPU_RAM_REGION_SIZE: usize = 8 * 1024;

        for (i, region_start) in (0..FLASH_SIZE).step_by(SPU_FLASH_REGION_SIZE).enumerate() {
            spu.flashregion[i].perm.write(|w| {
                w.read().enable();
                w.write().enable();
                w.execute().enable();
                if region_start < NON_SECURE_FLASH_OFFSET {
                    w.secattr().secure()
                } else {
                    w.secattr().non_secure()
                }
            });
        }

        spu.flashnsc[0].region.write(|w| w.region().bits(1));
        spu.flashnsc[0].size.write(|w| w.size().bits(1));

        for (i, region_start) in (0..RAM_SIZE).step_by(SPU_RAM_REGION_SIZE).enumerate() {
            spu.ramregion[i].perm.write(|w| {
                w.read().enable();
                w.write().enable();
                w.execute().enable();
                if region_start < NON_SECURE_RAM_OFFSET {
                    w.secattr().secure()
                } else {
                    w.secattr().non_secure()
                }
            });
        }

        spu.dppi[0].perm.write(|w| w.bits(0));
        spu.gpioport[0].perm.write(|w| w.bits(0));

        // NVIC->ITNS[0] to NVIC->ITNS[15]
        // Each of these registers is 32 bits wide.
        // Bit n in NVIC->ITNS[m] corresponds to IRQ number 32n + m
        unsafe fn nvic_itns_set_non_secure(id: usize) {
            const NVIC_ITNS_WIDTH: usize = 32;

            let nvic_itns_n = id / NVIC_ITNS_WIDTH;
            let itns_m = id % NVIC_ITNS_WIDTH;

            // defmt::trace!("Periph ID {} NVIC->ITNS[{}]:{}", id, nvic_itns_n, itns_m);

            let nvic_itns_base = 0xE000E380 as *mut u32;
            let itns = nvic_itns_base.add(nvic_itns_n);

            *itns |= 1 << itns_m;
        }

        for id in 3..spu.periphid.len() {
            // Special case for GPIOTE1's which has incorrect PERM properties.
            const GPIOTE1_ID: usize = 49;

            let bits_on_rst = spu.periphid[id].perm.read();

            let present = bits_on_rst.present().is_is_present();
            let split = bits_on_rst.securemapping().is_split();
            let usel = bits_on_rst.securemapping().is_user_selectable();
            let configurable = present && (split || usel);

            if configurable || GPIOTE1_ID == id {
                nvic_itns_set_non_secure(id);

                spu.periphid[id].perm.modify(|_r, w| {
                    w.secattr().non_secure();
                    w.dmasec().non_secure()
                });
            }

            let bits_after = spu.periphid[id].perm.read();

            defmt::println!(
                "Periph ID {}: {:#X} -> {:#X}",
                id,
                bits_on_rst.bits(),
                bits_after.bits()
            );
        }

        let ns_vector_table_addr = 0x00010000_u32 as *const u32;
        let ns_msp: u32 = *ns_vector_table_addr;
        let ns_vtor: u32 = *ns_vector_table_addr.add(1);

        defmt::info!("NS MSP {:#X}", ns_msp);
        defmt::info!("NS VTOR {:#X}", ns_vtor);

        const NS_OFFSET: u32 = 0x00020000;
        let scb_ns_address: u32 = cortex_m::peripheral::SCB::PTR as u32 + NS_OFFSET;
        let scb_ns = &*(scb_ns_address as *const cortex_m::peripheral::scb::RegisterBlock);
        scb_ns.vtor.write(ns_vector_table_addr as u32);

        // Write the Non-Secure Main Stack Pointer before switching state. Its value is the first
        // entry of the Non Secure Vector Table.
        cortex_m::register::msp::write_ns(ns_msp);
        cortex_m::register::psp::write_ns(0);

        let mut control = cortex_m::register::control::read_ns();
        control.set_npriv(Npriv::Privileged);
        control.set_spsel(Spsel::Msp);
        cortex_m::register::control::write_ns(control);

        const VECTKEY_Pos: u32 = 16;
        const VECTKEY_Msk: u32 = 0xFFFF << VECTKEY_Pos;
        const VECTKEY_PERMIT_WRITE: u32 = (0x05FA << VECTKEY_Pos) & VECTKEY_Msk;

        const PRIS_Pos: u32 = 14;
        const PRIS_Msk: u32 = 1 << PRIS_Pos;

        const BFHFNMINS_Pos: u32 = 13;
        const BFHFNMINS_Msk: u32 = 1 << BFHFNMINS_Pos;

        const SYSRESETREQS_Pos: u32 = 3;
        const SYSRESETREQS_Msk: u32 = 1 << SYSRESETREQS_Pos;

        // Enable secure fault
        cpu.SCB.enable(Exception::SecureFault);

        // Prioritize secure exceptions
        cpu.SCB.aircr.modify(|bits| {
            let aircr_payload = bits & (!VECTKEY_Msk);
            let aircr_payload = aircr_payload | PRIS_Msk;
            VECTKEY_PERMIT_WRITE | aircr_payload
        });

        // Non-banked exceptions should target non-secure
        cpu.SCB.aircr.modify(|bits| {
            let aircr_payload = bits & (!VECTKEY_Msk);
            let aircr_payload = aircr_payload | BFHFNMINS_Msk;
            VECTKEY_PERMIT_WRITE | aircr_payload
        });

        // Non-secure code may request reset
        cpu.SCB.aircr.modify(|bits| {
            let aircr_payload = bits & (!VECTKEY_Msk);
            let aircr_payload = aircr_payload & (!SYSRESETREQS_Msk);
            VECTKEY_PERMIT_WRITE | aircr_payload
        });

        // Disable SAU, and let SPU have precedence over it
        cpu.SAU.ctrl.write(cortex_m::peripheral::sau::Ctrl(0));
        cpu.SAU.ctrl.write(cortex_m::peripheral::sau::Ctrl(2));

        cortex_m::asm::dsb();
        cortex_m::asm::isb();

        cpu.SCB.set_fpu_access_mode(FpuAccessMode::Enabled);

        // Create a Non-Secure function pointer to the address of the second entry of the Non
        // Secure Vector Table.
        let ns_reset_vector: extern "C-cmse-nonsecure-call" fn() -> ! =
            core::mem::transmute::<u32, _>(ns_vtor);

        ns_reset_vector();
    }
}
