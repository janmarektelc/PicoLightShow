//! Pico Light Show - WS2812 controller firmware for the Raspberry Pi Pico W.
//!
//! Core 0: Wi-Fi, HTTP/WebSocket UI, MQTT, DDP, flash storage, OTA.
//! Core 1: effect rendering and LED output.

#![no_std]
#![no_main]

mod color;
mod config;
mod ddp;
mod effects;
mod fetch;
mod led;
mod mqtt;
mod net;
mod state;
mod storage;
mod system;
mod transitions;
mod update;
mod web;

use cyw43::aligned_bytes;
use cyw43_pio::{DEFAULT_CLOCK_DIVIDER, PioSpi};
use embassy_executor::{Executor, Spawner};
use embassy_rp::flash::Flash;
use embassy_rp::gpio::{Level, Output};
use embassy_rp::multicore::{Stack as CoreStack, spawn_core1};
use embassy_rp::peripherals::{DMA_CH0, DMA_CH1, DMA_CH2, PIO0, PIO1};
use embassy_rp::pio::Pio;
use embassy_rp::watchdog::Watchdog;
use embassy_rp::{bind_interrupts, dma, pio};
use embassy_sync::mutex::Mutex;
use static_cell::{ConstStaticCell, StaticCell};

use crate::config::BootPower;
use crate::led::{Renderer, Ws2812};
use crate::state::LightState;
use crate::storage::FlashMutex;

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => pio::InterruptHandler<PIO0>;
    PIO1_IRQ_0 => pio::InterruptHandler<PIO1>;
    DMA_IRQ_0 => dma::InterruptHandler<DMA_CH0>, dma::InterruptHandler<DMA_CH1>, dma::InterruptHandler<DMA_CH2>;
});

#[cfg(feature = "console")]
bind_interrupts!(struct UsbIrqs {
    USBCTRL_IRQ => embassy_rp::usb::InterruptHandler<embassy_rp::peripherals::USB>;
});

static EXECUTOR0: StaticCell<Executor> = StaticCell::new();
static EXECUTOR1: StaticCell<Executor> = StaticCell::new();
const CORE1_STACK_SIZE: usize = 16 * 1024;
static CORE1_STACK: ConstStaticCell<CoreStack<CORE1_STACK_SIZE>> = ConstStaticCell::new(CoreStack::new());
static LED_WORDS: ConstStaticCell<[u32; config::MAX_LEDS]> = ConstStaticCell::new([0; config::MAX_LEDS]);
static RENDERER: ConstStaticCell<Renderer> = ConstStaticCell::new(Renderer::new());
static FLASH: StaticCell<FlashMutex> = StaticCell::new();

/// Number of sockets: DHCP client, DNS client, HTTP workers, MQTT, DDP,
/// DHCP server, captive DNS, mDNS, update download.
const SOCKETS: usize = web::WORKERS + 9;

fn random_u32() -> u32 {
    let mut v = 0u32;
    for _ in 0..32 {
        v = (v << 1) | (embassy_rp::pac::ROSC.randombit().read().randombit() as u32);
    }
    v
}

/// Breadcrumb left in RAM by the bootloader (see bootloader/src/main.rs).
static BOOTLOADER_CRUMB: portable_atomic::AtomicU32 = portable_atomic::AtomicU32::new(0);

fn read_bootloader_crumb() {
    const ADDR: *mut u32 = 0x2004_1000 as *mut u32;
    unsafe {
        if core::ptr::read_volatile(ADDR) == 0xB007_1D00 {
            BOOTLOADER_CRUMB.store(core::ptr::read_volatile(ADDR.add(1)) | 0x8000_0000, portable_atomic::Ordering::Relaxed);
            core::ptr::write_volatile(ADDR, 0);
        }
    }
}

#[cortex_m_rt::entry]
fn main() -> ! {
    read_bootloader_crumb();
    paint_core0_stack();
    // Trap core 0 stack overflows instead of silently corrupting RAM.
    let _ = embassy_rp::install_core0_stack_guard();
    let p = embassy_rp::init(Default::default());
    let executor0 = EXECUTOR0.init(Executor::new());
    executor0.run(|spawner| spawner.spawn(main_task(spawner, p).unwrap()));
}

/// USB serial console (only in builds with `--features console`). The 4 KiB
/// buffer keeps the boot messages until a terminal opens the port.
#[cfg(feature = "console")]
#[embassy_executor::task]
async fn logger_task(usb: embassy_rp::Peri<'static, embassy_rp::peripherals::USB>) {
    use embassy_usb_logger::ReceiverHandler as _;
    let driver = embassy_rp::usb::Driver::new(usb, UsbIrqs);
    embassy_usb_logger::run!(4096, log::LevelFilter::Info, driver, ConsoleCommands);
}

/// Commands typed into the serial console (console builds only):
/// `bootsel` = reboot into the USB bootloader (RPI-RP2 drive / picotool),
/// `reboot` = normal restart.
#[cfg(feature = "console")]
struct ConsoleCommands {
    line: embassy_sync::mutex::Mutex<embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex, heapless::Vec<u8, 32>>,
}

#[cfg(feature = "console")]
impl embassy_usb_logger::ReceiverHandler for ConsoleCommands {
    fn new() -> Self {
        Self { line: embassy_sync::mutex::Mutex::new(heapless::Vec::new()) }
    }

    async fn handle_data(&self, data: &[u8]) {
        let mut line = self.line.lock().await;
        for &b in data {
            if b == b'\r' || b == b'\n' {
                match line.as_slice() {
                    b"bootsel" | b"elf2uf2-term" => embassy_rp::rom_data::reset_to_usb_boot(0, 0),
                    b"reboot" => cortex_m::peripheral::SCB::sys_reset(),
                    _ => {}
                }
                line.clear();
            } else if line.push(b).is_err() {
                line.clear();
            }
        }
    }
}

#[embassy_executor::task]
async fn cyw43_task(runner: cyw43::Runner<'static, cyw43::SpiBus<Output<'static>, PioSpi<'static, PIO0, 0>>>) -> ! {
    runner.run().await
}

#[embassy_executor::task]
async fn net_task(mut runner: embassy_net::Runner<'static, cyw43::NetDriver<'static>>) -> ! {
    runner.run().await
}

const STACK_PAINT: u32 = 0xC0FF_EE00;
/// Bytes at the bottom of the core 0 stack that are covered by the MPU guard.
const GUARD_SKIP: usize = 64;

unsafe extern "C" {
    static _stack_end: u32;
    static _stack_start: u32;
}

/// Fill the unused core 0 stack with a pattern (before the stack guard is
/// installed), so the high-water mark can be measured later.
fn paint_core0_stack() {
    let bottom = (&raw const _stack_end) as usize + GUARD_SKIP;
    let sp = cortex_m::register::msp::read() as usize;
    let mut p = bottom;
    while p + 512 < sp {
        unsafe { core::ptr::write_volatile(p as *mut u32, STACK_PAINT) };
        p += 4;
    }
}

fn core0_stack_free() -> (usize, usize) {
    let bottom = (&raw const _stack_end) as usize + GUARD_SKIP;
    let top = (&raw const _stack_start) as usize;
    let words = unsafe { core::slice::from_raw_parts(bottom as *const u32, (top - bottom) / 4) };
    (words.iter().take_while(|w| **w == STACK_PAINT).count() * 4, top - bottom)
}

/// Periodically logs the stack high-water marks of both cores.
#[embassy_executor::task]
async fn stack_report_task(addr: usize) {
    let mut last = usize::MAX;
    loop {
        let words = unsafe { core::slice::from_raw_parts(addr as *const u32, CORE1_STACK_SIZE / 4) };
        let core1_free = words.iter().take_while(|w| **w == 0).count() * 4;
        let (core0_free, core0_size) = core0_stack_free();
        let o = portable_atomic::Ordering::Relaxed;
        system::STACK_FREE[0].store(core0_free as u32, o);
        system::STACK_SIZE[0].store(core0_size as u32, o);
        system::STACK_FREE[1].store(core1_free as u32, o);
        system::STACK_SIZE[1].store(CORE1_STACK_SIZE as u32, o);
        if core0_free != last {
            last = core0_free;
            log::info!(
                "stack free: core 0 {} of {} bytes, core 1 {} of {} bytes",
                core0_free, core0_size, core1_free, CORE1_STACK_SIZE
            );
        }
        embassy_time::Timer::after_secs(10).await;
    }
}

/// Convert the configured GPIO number into a PIO pin.
macro_rules! led_pin {
    ($common:expr, $num:expr, $($n:literal => $pin:ident),* $(,)?) => {
        match $num {
            $( $n => $common.make_pio_pin(unsafe { embassy_rp::peripherals::$pin::steal() }), )*
            _ => $common.make_pio_pin(unsafe { embassy_rp::peripherals::PIN_14::steal() }),
        }
    };
}

#[embassy_executor::task]
async fn main_task(spawner: Spawner, p: embassy_rp::Peripherals) {
    let prev_stage = system::previous_stage();
    let last_crash = system::take_last_panic();
    let planned_reboot = system::take_planned_reboot();

    // Console builds: bring USB up first, so the serial port appears even if
    // something later crashes, and give the host a moment to enumerate it.
    #[cfg(feature = "console")]
    {
        spawner.spawn(logger_task(p.USB).unwrap());
        embassy_time::Timer::after_millis(1500).await;
    }

    let mut watchdog = Watchdog::new(p.WATCHDOG);
    watchdog.start(embassy_time::Duration::from_secs(8));
    let reset_reason = watchdog.reset_reason();
    // A software reset (SCB) does not clear the watchdog's reason register,
    // so the RAM markers take precedence over it.
    system::RESET_REASON.store(
        match reset_reason {
            _ if last_crash.is_some() => 4,
            _ if planned_reboot => 3,
            Some(embassy_rp::watchdog::ResetReason::Forced) => 1,
            Some(embassy_rp::watchdog::ResetReason::TimedOut) => 2,
            None => 0,
        },
        portable_atomic::Ordering::Relaxed,
    );
    system::stage(1);
    log::info!("Pico Light Show {} starting (reset reason: {:?})", system::VERSION, reset_reason);
    let crumb = BOOTLOADER_CRUMB.load(portable_atomic::Ordering::Relaxed);
    if crumb == 0 {
        log::info!("bootloader: none (standalone build or first start)");
    } else {
        let path = match (crumb >> 8) & 0xff {
            1 => "fast path (no update pending)",
            2 => "update swapped / reverted",
            _ => "error",
        };
        log::info!("bootloader: {}, state byte {:#04x}", path, crumb & 0xff);
    }

    let hung = matches!(reset_reason, Some(embassy_rp::watchdog::ResetReason::TimedOut))
        && last_crash.is_none()
        && !planned_reboot
        && prev_stage.is_some_and(|s| s < 9);
    if last_crash.is_some() || hung {
        let stage_name = prev_stage.and_then(|s| system::STAGE_NAMES.get(s as usize)).copied().unwrap_or("unknown");
        // In console builds stop here for a minute, so the report can be read
        // before the device possibly crashes again.
        let repeats = if cfg!(feature = "console") { 30 } else { 1 };
        for _ in 0..repeats {
            match &last_crash {
                Some(msg) => log::error!("PREVIOUS RUN CRASHED: {} (last boot stage: {})", msg.as_str(), stage_name),
                None => log::error!("PREVIOUS RUN HUNG (watchdog reset) after boot stage: {}", stage_name),
            }
            watchdog.feed(embassy_time::Duration::from_secs(8));
            if repeats > 1 {
                embassy_time::Timer::after_secs(2).await;
            }
        }
    }
    if let Some(msg) = last_crash {
        web::set_last_panic(msg);
    }
    spawner.spawn(system::watchdog_task(watchdog).unwrap());
    spawner.spawn(system::reboot_task().unwrap());

    // --- flash & settings ---
    let flash: &'static FlashMutex = FLASH.init(Mutex::new(Flash::new(p.FLASH, p.DMA_CH2, Irqs)));
    storage::init(flash).await;
    let cfg = storage::load_config().await;
    log::info!(
        "settings loaded: wifi mode {:?}, ssid '{}', {} LEDs on GP{}",
        cfg.wifi_mode,
        cfg.wifi_ssid.as_str(),
        cfg.led_count,
        cfg.led_pin
    );
    let mut light = match storage::load_light().await {
        Some(stored) => stored.into_state(),
        None => LightState::defaults(),
    };
    light.on = match cfg.boot_power {
        BootPower::Off => false,
        BootPower::On => true,
        BootPower::Restore => light.on,
    };
    state::init(light);
    let presets = storage::load_presets().await;
    state::PRESETS.lock(|p| *p.borrow_mut() = presets);
    config::set_led_settings(config::LedSettings::from_config(&cfg));
    config::set(cfg.clone());
    system::stage(2);

    // --- LED output on core 1 ---
    let Pio { mut common, sm0, .. } = Pio::new(p.PIO1, Irqs);
    let program = led::load_program(&mut common);
    let pin = led_pin!(common, cfg.led_pin,
        0 => PIN_0, 1 => PIN_1, 2 => PIN_2, 3 => PIN_3, 4 => PIN_4, 5 => PIN_5, 6 => PIN_6,
        7 => PIN_7, 8 => PIN_8, 9 => PIN_9, 10 => PIN_10, 11 => PIN_11, 12 => PIN_12,
        13 => PIN_13, 14 => PIN_14, 15 => PIN_15, 16 => PIN_16, 17 => PIN_17, 18 => PIN_18,
        19 => PIN_19, 20 => PIN_20, 21 => PIN_21, 22 => PIN_22, 26 => PIN_26, 27 => PIN_27,
        28 => PIN_28,
    );
    let ws = Ws2812::new(sm0, dma::Channel::new(p.DMA_CH1, Irqs), &pin, &program, LED_WORDS.take());
    // The PIO common block, pin and program must stay alive while the state machine runs.
    core::mem::forget(common);
    core::mem::forget(pin);
    core::mem::forget(program);
    let renderer = RENDERER.take();
    let seed = random_u32();
    let core1_stack = CORE1_STACK.take();
    let core1_stack_addr = core1_stack as *mut CoreStack<CORE1_STACK_SIZE> as usize;
    spawn_core1(p.CORE1, core1_stack, move || {
        let executor1 = EXECUTOR1.init(Executor::new());
        executor1.run(|s| s.spawn(led::render_task(ws, renderer, seed).unwrap()));
    });
    system::stage(3);

    // --- Wi-Fi ---
    let fw = aligned_bytes!("../cyw43-firmware/43439A0.bin");
    let clm = aligned_bytes!("../cyw43-firmware/43439A0_clm.bin");
    let nvram = aligned_bytes!("../cyw43-firmware/nvram_rp2040.bin");

    let pwr = Output::new(p.PIN_23, Level::Low);
    let cs = Output::new(p.PIN_25, Level::High);
    let mut pio0 = Pio::new(p.PIO0, Irqs);
    let spi = PioSpi::new(
        &mut pio0.common,
        pio0.sm0,
        DEFAULT_CLOCK_DIVIDER,
        pio0.irq0,
        cs,
        p.PIN_24,
        p.PIN_29,
        dma::Channel::new(p.DMA_CH0, Irqs),
    );
    system::stage(4);
    static CYW43_STATE: StaticCell<cyw43::State> = StaticCell::new();
    let (net_device, mut control, runner) = cyw43::new(CYW43_STATE.init(cyw43::State::new()), pwr, spi, fw, nvram).await;
    spawner.spawn(cyw43_task(runner).unwrap());
    control.init(clm).await;
    control.set_power_management(cyw43::PowerManagementMode::None).await;
    system::stage(5);

    static RESOURCES: StaticCell<embassy_net::StackResources<SOCKETS>> = StaticCell::new();
    let seed64 = ((random_u32() as u64) << 32) | random_u32() as u64;
    let (stack, net_runner) =
        embassy_net::new(net_device, embassy_net::Config::default(), RESOURCES.init(embassy_net::StackResources::new()), seed64);
    spawner.spawn(net_task(net_runner).unwrap());
    system::stage(6);
    spawner.spawn(net::wifi_task(control, stack, spawner, cfg.clone()).unwrap());

    // --- services ---
    web::start(spawner, stack, flash);
    spawner.spawn(storage::saver_task().unwrap());
    spawner.spawn(system::confirm_boot_task(flash).unwrap());
    spawner.spawn(net::mdns_task(stack).unwrap());
    spawner.spawn(update::update_task(stack, flash).unwrap());
    if cfg.ddp_enabled {
        spawner.spawn(ddp::ddp_task(stack).unwrap());
    }
    if cfg.mqtt_enabled && !cfg.mqtt_host.is_empty() {
        spawner.spawn(mqtt::mqtt_task(stack, cfg.clone()).unwrap());
    }
    system::stage(7);
    spawner.spawn(stack_report_task(core1_stack_addr).unwrap());
    let adc = embassy_rp::adc::Adc::new_blocking(p.ADC, embassy_rp::adc::Config::default());
    let ts = embassy_rp::adc::Channel::new_temp_sensor(p.ADC_TEMP_SENSOR);
    spawner.spawn(system::sensor_task(adc, ts).unwrap());
}
