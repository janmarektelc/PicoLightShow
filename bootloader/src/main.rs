//! Bootloader for Pico Light Show.
//!
//! Normal boot (no pending update): read the state flag straight from flash
//! and jump into the application WITHOUT touching clocks or peripherals, so
//! the application starts in the same state as after a plain power-up.
//!
//! Pending update (state = SWAP): swap the DFU image into the active slot
//! with embassy-boot. If the new firmware does not confirm itself
//! (`mark_booted`) before the next reset, embassy-boot reverts it.
//! Clocks are not reconfigured here either (flash ROM routines run fine on
//! the ring oscillator).

#![no_std]
#![no_main]

use core::cell::RefCell;

use cortex_m_rt::{entry, exception};
use embassy_boot_rp::*;
use embassy_rp::pac;
use embassy_sync::blocking_mutex::Mutex;
use embassy_time::Duration;

const FLASH_SIZE: usize = 2 * 1024 * 1024;
const SWAP_MAGIC: u8 = 0xF0;

/// Breadcrumb for the application (read and logged by its console build).
const BREADCRUMB: *mut u32 = 0x2004_1000 as *mut u32;
const BREADCRUMB_MAGIC: u32 = 0xB007_1D00;

unsafe extern "C" {
    static __bootloader_state_start: u32;
    static __bootloader_active_start: u32;
}

fn leave_breadcrumb(path: u32, state: u8) {
    unsafe {
        core::ptr::write_volatile(BREADCRUMB, BREADCRUMB_MAGIC);
        core::ptr::write_volatile(BREADCRUMB.add(1), (path << 8) | state as u32);
    }
}

fn jump_to_app() -> ! {
    let active = unsafe { &__bootloader_active_start as *const u32 as u32 };
    unsafe {
        let p = cortex_m::Peripherals::steal();
        p.SCB.vtor.write(embassy_rp::flash::FLASH_BASE as u32 + active);
        cortex_m::asm::bootload((embassy_rp::flash::FLASH_BASE as u32 + active) as *const u32)
    }
}

#[entry]
fn main() -> ! {
    let state_offset = unsafe { &__bootloader_state_start as *const u32 as u32 };
    let state = unsafe { core::ptr::read_volatile((embassy_rp::flash::FLASH_BASE as u32 + state_offset) as *const u8) };

    if state != SWAP_MAGIC {
        // Fast path: nothing to do.
        leave_breadcrumb(1, state);
        jump_to_app();
    }

    // Update pending. The flash driver checks DMA channels, so take DMA out of
    // reset; give the watchdog a tick source (clk_ref = ring oscillator here,
    // so the timeout is only approximate).
    pac::RESETS.reset().modify(|w| w.set_dma(false));
    while !pac::RESETS.reset_done().read().dma() {}
    pac::WATCHDOG.tick().write(|w| {
        w.set_cycles(12);
        w.set_enable(true);
    });

    let p = unsafe { embassy_rp::Peripherals::steal() };
    let flash = WatchdogFlash::<FLASH_SIZE>::start(p.FLASH, p.WATCHDOG, Duration::from_secs(8));
    let flash = Mutex::new(RefCell::new(flash));
    let config = BootLoaderConfig::from_linkerfile_blocking(&flash, &flash, &flash);
    let bl: BootLoader = BootLoader::prepare(config);
    let new_state = unsafe { core::ptr::read_volatile((embassy_rp::flash::FLASH_BASE as u32 + state_offset) as *const u8) };
    leave_breadcrumb(2, new_state);
    let _ = bl;
    jump_to_app();
}

#[unsafe(no_mangle)]
#[cfg_attr(target_os = "none", unsafe(link_section = ".HardFault.user"))]
unsafe extern "C" fn HardFault() {
    leave_breadcrumb(0xEE, 0);
    cortex_m::peripheral::SCB::sys_reset();
}

#[exception]
unsafe fn DefaultHandler(_: i16) -> ! {
    cortex_m::peripheral::SCB::sys_reset();
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    leave_breadcrumb(0xEF, 0);
    cortex_m::peripheral::SCB::sys_reset();
}
