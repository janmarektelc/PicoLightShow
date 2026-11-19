//! System services: watchdog, reboot, panic capture, OTA boot confirmation.

use core::fmt::Write;
use core::mem::MaybeUninit;

use embassy_boot_rp::{FirmwareUpdater, FirmwareUpdaterConfig, State as BootState};
use embassy_embedded_hal::flash::partition::Partition;
use embassy_rp::watchdog::Watchdog;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_sync::watch::Watch;
use embassy_time::{Duration, Instant, Timer};
use heapless::String;
use portable_atomic::{AtomicI32, AtomicU8, AtomicU32, Ordering};

use crate::storage::FlashMutex;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

// ---------------------------------------------------------------------------
// Network ready flag
// ---------------------------------------------------------------------------

static NETWORK_READY: Watch<CriticalSectionRawMutex, bool, 6> = Watch::new();

pub fn set_network_ready() {
    NETWORK_READY.sender().send(true);
}

pub async fn wait_network() {
    if let Some(mut rx) = NETWORK_READY.receiver() {
        rx.changed_and(|v| *v).await;
    } else {
        Timer::after_secs(30).await;
    }
}

// ---------------------------------------------------------------------------
// Reboot
// ---------------------------------------------------------------------------

static REBOOT: Signal<CriticalSectionRawMutex, ()> = Signal::new();

/// Reboot shortly (gives the HTTP response time to leave).
pub fn request_reboot() {
    REBOOT.signal(());
}

#[embassy_executor::task]
pub async fn reboot_task() {
    REBOOT.wait().await;
    log::info!("rebooting...");
    Timer::after_millis(800).await;
    embassy_rp::pac::WATCHDOG.scratch0().write_value(REBOOT_MAGIC);
    cortex_m::peripheral::SCB::sys_reset();
}

const REBOOT_MAGIC: u32 = 0x5242_5154;
/// Watchdog scratch register used for the "requested reboot" marker. It survives
/// a software reset and the bootloader's update swap, and unlike a RAM variable its
/// location does not depend on the memory layout of the firmware version.
/// (Scratch 0 is used; the boot ROM only uses scratch 4..7.)

/// True if the previous run ended with a requested reboot (clears the marker).
pub fn take_planned_reboot() -> bool {
    let reg = embassy_rp::pac::WATCHDOG.scratch0();
    let planned = reg.read() == REBOOT_MAGIC;
    reg.write_value(0);
    planned
}

// ---------------------------------------------------------------------------
// Watchdog: fed only while the render core is alive.
// ---------------------------------------------------------------------------

#[embassy_executor::task]
pub async fn watchdog_task(mut wd: Watchdog) {
    let timeout = Duration::from_secs(8);
    wd.start(timeout);
    let mut last_beat = crate::led::HEARTBEAT.load(Ordering::Relaxed);
    let mut last_change = Instant::now();
    loop {
        Timer::after_secs(1).await;
        let beat = crate::led::HEARTBEAT.load(Ordering::Relaxed);
        if beat != last_beat {
            last_beat = beat;
            last_change = Instant::now();
        }
        if last_change.elapsed() < Duration::from_secs(4) {
            wd.feed(timeout);
        } else {
            log::error!("render core stalled, letting watchdog reset");
        }
    }
}

// ---------------------------------------------------------------------------
// Crash capture: panic / HardFault message and the last reached boot stage
// survive a soft reset in .uninit RAM and are reported on the next boot.
// ---------------------------------------------------------------------------

const PANIC_MAGIC: u32 = 0x5041_4e43;
const STAGE_MAGIC: u32 = 0x5354_4147;

struct CrashRecord {
    magic: u32,
    len: u32,
    buf: [u8; 200],
    stage_magic: u32,
    stage: u32,
}

#[unsafe(link_section = ".uninit.CRASH")]
static mut CRASH: MaybeUninit<CrashRecord> = MaybeUninit::uninit();

fn record() -> &'static mut CrashRecord {
    unsafe { &mut *(&raw mut CRASH).cast::<CrashRecord>() }
}

struct CrashWriter<'a> {
    buf: &'a mut [u8; 200],
    len: usize,
}

impl Write for CrashWriter<'_> {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        for b in s.bytes() {
            if self.len < self.buf.len() {
                self.buf[self.len] = b;
                self.len += 1;
            }
        }
        Ok(())
    }
}

fn store_crash(args: core::fmt::Arguments) {
    let rec = record();
    let mut w = CrashWriter { buf: &mut rec.buf, len: 0 };
    let _ = w.write_fmt(args);
    rec.len = w.len as u32;
    rec.magic = PANIC_MAGIC;
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    cortex_m::interrupt::disable();
    store_crash(format_args!("panic: {}", info));
    cortex_m::peripheral::SCB::sys_reset();
}

#[cortex_m_rt::exception]
unsafe fn HardFault(ef: &cortex_m_rt::ExceptionFrame) -> ! {
    store_crash(format_args!(
        "HardFault at pc={:#010x} lr={:#010x} (stage {})",
        ef.pc(),
        ef.lr(),
        record().stage
    ));
    cortex_m::peripheral::SCB::sys_reset();
}

#[cortex_m_rt::exception]
unsafe fn DefaultHandler(irqn: i16) {
    store_crash(format_args!("unexpected interrupt {}", irqn));
    cortex_m::peripheral::SCB::sys_reset();
}

/// Message of the crash that caused the previous reset, if any.
pub fn take_last_panic() -> Option<String<200>> {
    let rec = record();
    if rec.magic != PANIC_MAGIC {
        return None;
    }
    rec.magic = 0;
    let len = (rec.len as usize).min(rec.buf.len());
    let mut s = String::new();
    for &b in &rec.buf[..len] {
        let _ = s.push(if b.is_ascii() { b as char } else { '?' });
    }
    Some(s)
}

/// Boot stages, so that a hang (watchdog reset) can be located afterwards.
pub const STAGE_NAMES: &[&str] = &[
    "reset",
    "main task started",
    "settings loaded",
    "render core started",
    "wifi chip powered",
    "wifi chip initialised",
    "network stack created",
    "services started",
    "wifi up",
    "running",
];

pub fn stage(n: u32) {
    let rec = record();
    rec.stage = n;
    rec.stage_magic = STAGE_MAGIC;
    log::info!("boot stage {}: {}", n, STAGE_NAMES.get(n as usize).copied().unwrap_or("?"));
}

/// Boot stage reached by the previous run (call before the first `stage()`).
pub fn previous_stage() -> Option<u32> {
    let rec = record();
    if rec.stage_magic == STAGE_MAGIC { Some(rec.stage) } else { None }
}

// ---------------------------------------------------------------------------
// Firmware update state
// ---------------------------------------------------------------------------

/// 0 = normal boot, 1 = first boot after an update, 2 = rolled back
pub static BOOT_KIND: AtomicU8 = AtomicU8::new(0);

pub type Updater<'a> = FirmwareUpdater<
    'a,
    Partition<'static, CriticalSectionRawMutex, crate::storage::FlashDev>,
    Partition<'static, CriticalSectionRawMutex, crate::storage::FlashDev>,
>;

pub fn updater_config(
    flash: &'static FlashMutex,
) -> FirmwareUpdaterConfig<
    Partition<'static, CriticalSectionRawMutex, crate::storage::FlashDev>,
    Partition<'static, CriticalSectionRawMutex, crate::storage::FlashDev>,
> {
    unsafe extern "C" {
        static __bootloader_state_start: u32;
        static __bootloader_state_end: u32;
        static __bootloader_dfu_start: u32;
        static __bootloader_dfu_end: u32;
    }
    unsafe {
        let ss = &__bootloader_state_start as *const u32 as u32;
        let se = &__bootloader_state_end as *const u32 as u32;
        let ds = &__bootloader_dfu_start as *const u32 as u32;
        let de = &__bootloader_dfu_end as *const u32 as u32;
        FirmwareUpdaterConfig { dfu: Partition::new(flash, ds, de - ds), state: Partition::new(flash, ss, se - ss) }
    }
}

pub fn dfu_size() -> u32 {
    unsafe extern "C" {
        static __bootloader_dfu_start: u32;
        static __bootloader_dfu_end: u32;
    }
    unsafe { (&__bootloader_dfu_end as *const u32 as u32) - (&__bootloader_dfu_start as *const u32 as u32) }
}

/// Scratch buffer for the boot state: max(WRITE_SIZE, READ_SIZE) of the async
/// flash = 4 bytes, word aligned for DMA reads.
#[repr(align(4))]
pub struct StateBuf(pub [u8; 4]);

impl StateBuf {
    pub const fn new() -> Self {
        Self([0; 4])
    }
}

/// After an OTA update the new firmware has to confirm that it works,
/// otherwise the bootloader rolls back on the next reset. We confirm once the
/// network is up and the system ran stable for a while.
#[embassy_executor::task]
pub async fn confirm_boot_task(flash: &'static FlashMutex) {
    if cfg!(feature = "standalone") {
        return; // no bootloader, nothing to confirm
    }
    let mut buf = StateBuf::new();
    let aligned = &mut buf.0;
    let state = {
        let mut updater = FirmwareUpdater::new(updater_config(flash), aligned);
        updater.get_state().await
    };
    match state {
        Ok(BootState::Boot) => return,
        Ok(BootState::Swap) => {
            BOOT_KIND.store(1, Ordering::Relaxed);
            log::info!("ota: first boot of a new firmware, waiting for health check");
        }
        Ok(BootState::Revert) => {
            BOOT_KIND.store(2, Ordering::Relaxed);
            log::warn!("ota: previous update failed, rolled back");
        }
        Ok(_) => {}
        Err(e) => {
            log::error!("ota: cannot read boot state: {:?}", e);
            return;
        }
    }
    wait_network().await;
    Timer::after_secs(10).await;
    let mut updater = FirmwareUpdater::new(updater_config(flash), aligned);
    match updater.mark_booted().await {
        Ok(()) => log::info!("ota: firmware confirmed"),
        Err(e) => log::error!("ota: mark_booted failed: {:?}", e),
    }
}

// ---------------------------------------------------------------------------
// Runtime statistics shown on the System page
// ---------------------------------------------------------------------------

/// 0 = power-on / reset pin, 1 = forced by watchdog, 2 = watchdog timeout (hang),
/// 3 = requested reboot, 4 = crash (panic / fault).
pub static RESET_REASON: AtomicU8 = AtomicU8::new(0);
/// Lowest free stack seen so far, per core (bytes), and the stack sizes.
pub static STACK_FREE: [AtomicU32; 2] = [AtomicU32::new(0), AtomicU32::new(0)];
pub static STACK_SIZE: [AtomicU32; 2] = [AtomicU32::new(0), AtomicU32::new(0)];
/// RP2040 die temperature in 1/100 °C (`i32::MIN` = not measured yet).
pub static CHIP_TEMP_CENTI: AtomicI32 = AtomicI32::new(i32::MIN);
/// Open WebSocket connections (browser tabs with the UI open).
pub static WS_CLIENTS: AtomicU8 = AtomicU8::new(0);

/// Counts an open WebSocket while alive.
pub struct WsClientGuard;

impl WsClientGuard {
    pub fn new() -> Self {
        critical_section::with(|_| {
            let n = WS_CLIENTS.load(Ordering::Relaxed);
            WS_CLIENTS.store(n.saturating_add(1), Ordering::Relaxed);
        });
        Self
    }
}

impl Drop for WsClientGuard {
    fn drop(&mut self) {
        critical_section::with(|_| {
            let n = WS_CLIENTS.load(Ordering::Relaxed);
            WS_CLIENTS.store(n.saturating_sub(1), Ordering::Relaxed);
        });
    }
}

/// Bytes of RAM used by static data (.data + .bss + .uninit), and the total RAM.
pub fn ram_static() -> (u32, u32) {
    unsafe extern "C" {
        static __sdata: u32;
        static __euninit: u32;
    }
    let start = &raw const __sdata as u32;
    let end = &raw const __euninit as u32;
    (end - start, 264 * 1024)
}

/// Bytes of flash used by the application image, and the size of its partition.
pub fn flash_usage() -> (u32, u32) {
    unsafe extern "C" {
        static __app_start: u32;
        static __app_end: u32;
        static __sidata: u32;
        static __sdata: u32;
        static __edata: u32;
    }
    let start = &raw const __app_start as u32;
    let end = &raw const __app_end as u32;
    let image_end = &raw const __sidata as u32 + (&raw const __edata as u32 - &raw const __sdata as u32);
    (image_end - start, end - start)
}

/// Samples the on-chip temperature sensor every few seconds.
#[embassy_executor::task]
pub async fn sensor_task(
    mut adc: embassy_rp::adc::Adc<'static, embassy_rp::adc::Blocking>,
    mut ts: embassy_rp::adc::Channel<'static>,
) {
    let mut avg: Option<f32> = None;
    loop {
        if let Ok(raw) = adc.blocking_read(&mut ts) {
            let v = raw as f32 * 3.3 / 4096.0;
            let t = 27.0 - (v - 0.706) / 0.001721;
            let a = match avg {
                Some(a) => a + (t - a) * 0.2,
                None => t,
            };
            avg = Some(a);
            CHIP_TEMP_CENTI.store((a * 100.0) as i32, Ordering::Relaxed);
        }
        Timer::after_secs(5).await;
    }
}

/// Held while a firmware image is written into the DFU slot (web upload or online update).
pub static OTA_LOCK: embassy_sync::mutex::Mutex<CriticalSectionRawMutex, ()> = embassy_sync::mutex::Mutex::new(());

/// Plausibility check of the first bytes of an application image: initial stack
/// pointer in RAM and reset vector inside the application flash region.
pub fn looks_like_app_image(head: &[u8]) -> bool {
    if head.len() < 8 || &head[..4] == b"UF2\n" {
        return false;
    }
    let (start, end) = {
        unsafe extern "C" {
            static __app_start: u32;
            static __app_end: u32;
        }
        (&raw const __app_start as u32, &raw const __app_end as u32)
    };
    let sp = u32::from_le_bytes([head[0], head[1], head[2], head[3]]);
    let reset = u32::from_le_bytes([head[4], head[5], head[6], head[7]]) & !1;
    (0x2000_0000..=0x2004_2000).contains(&sp) && (start..end).contains(&reset)
}
