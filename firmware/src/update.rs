//! Online firmware updates.
//!
//! The device reads a small JSON manifest (by default `releases/latest.json`
//! in the GitHub repository) after start-up and then once a day:
//!
//! ```json
//! {"version":"1.1.0","ota":"1.1.0/pico-light-show-1.1.0-ota.bin","size":709120,
//!  "sha256":"<64 hex digits>","notes":"What changed"}
//! ```
//!
//! `ota` may be absolute or relative to the manifest URL. Installing streams the
//! image straight into the DFU slot, checks size and SHA-256, marks the update
//! and reboots; the bootloader swaps it in and rolls back if it does not start.
//! Triggered from the web UI (`/api/update`) or Home Assistant (MQTT update entity).

use core::cell::RefCell;

use embassy_futures::select::{Either, select};
use embassy_net::Stack;
use embassy_sync::blocking_mutex::Mutex;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_sync::watch::Watch;
use embassy_time::{Duration, Instant, Timer};
use heapless::String;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use static_cell::ConstStaticCell;

use crate::fetch::{self, Buffers, Sink};
use crate::storage::FlashMutex;
use crate::{config, net, system};

pub const DEFAULT_URL: &str = "https://raw.githubusercontent.com/janmarektelc/PicoLightShow/main/releases/latest.json";
/// Shown in Home Assistant as "release notes" link.
pub const RELEASE_URL: &str = "https://github.com/janmarektelc/PicoLightShow/tree/main/releases";

const CHECK_EVERY: Duration = Duration::from_secs(24 * 3600);
const RETRY_AFTER_ERROR: Duration = Duration::from_secs(3600);
const FIRST_CHECK: Duration = Duration::from_secs(60);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    /// Not checked yet.
    Idle,
    Checking,
    UpToDate,
    Available,
    Downloading,
    /// Image stored, rebooting into it.
    Installing,
    Error,
}

#[derive(Clone, Serialize)]
pub struct Status {
    pub phase: Phase,
    pub installed: &'static str,
    /// Newest version from the manifest (empty = unknown).
    pub latest: String<16>,
    pub notes: String<240>,
    /// Download progress 0..=100.
    pub progress: u8,
    pub error: String<64>,
    /// Seconds since the last successful check (None = never).
    pub checked_ago_s: Option<u64>,
    #[serde(skip)]
    checked_at: Option<Instant>,
    #[serde(skip)]
    ota_url: String<200>,
    #[serde(skip)]
    size: u32,
    #[serde(skip)]
    sha256: [u8; 32],
}

impl Status {
    const fn new() -> Self {
        Self {
            phase: Phase::Idle,
            installed: system::VERSION,
            latest: String::new(),
            notes: String::new(),
            progress: 0,
            error: String::new(),
            checked_ago_s: None,
            checked_at: None,
            ota_url: String::new(),
            size: 0,
            sha256: [0; 32],
        }
    }
}

static STATUS: Mutex<CriticalSectionRawMutex, RefCell<Status>> = Mutex::new(RefCell::new(Status::new()));
/// Fires on every status change (MQTT publishes the update entity state).
pub static CHANGED: Watch<CriticalSectionRawMutex, (), 2> = Watch::new();

#[derive(Clone, Copy)]
enum Command {
    Check,
    Install,
}
static COMMAND: Signal<CriticalSectionRawMutex, Command> = Signal::new();

pub fn request_check() {
    COMMAND.signal(Command::Check);
}

pub fn request_install() {
    COMMAND.signal(Command::Install);
}

/// Current status (for the web UI / MQTT).
pub fn status() -> Status {
    STATUS.lock(|s| {
        let mut st = s.borrow().clone();
        st.checked_ago_s = st.checked_at.map(|t| t.elapsed().as_secs());
        st
    })
}

pub fn status_json(buf: &mut [u8]) -> Option<usize> {
    serde_json_core::to_slice(&status(), buf).ok()
}

/// State payload of the Home Assistant MQTT `update` entity.
pub fn ha_state_json(buf: &mut [u8]) -> Option<usize> {
    #[derive(Serialize)]
    struct HaState<'a> {
        installed_version: &'a str,
        latest_version: &'a str,
        title: &'a str,
        release_summary: &'a str,
        release_url: &'a str,
        in_progress: bool,
        update_percentage: Option<u8>,
    }
    let st = status();
    let busy = matches!(st.phase, Phase::Downloading | Phase::Installing);
    let v = HaState {
        installed_version: st.installed,
        latest_version: if st.latest.is_empty() { st.installed } else { st.latest.as_str() },
        title: "Pico Light Show firmware",
        release_summary: st.notes.as_str(),
        release_url: RELEASE_URL,
        in_progress: busy,
        update_percentage: if busy { Some(st.progress) } else { None },
    };
    serde_json_core::to_slice(&v, buf).ok()
}

fn modify(f: impl FnOnce(&mut Status)) {
    STATUS.lock(|s| f(&mut s.borrow_mut()));
    CHANGED.sender().send(());
}

fn fail(msg: &str) {
    log::warn!("update: {}", msg);
    modify(|s| {
        s.phase = Phase::Error;
        s.progress = 0;
        s.error.clear();
        let _ = s.error.push_str(&msg[..msg.len().min(64)]);
    });
}

/// `a.b.c` -> comparable tuple.
fn parse_version(v: &str) -> Option<(u16, u16, u16)> {
    let mut it = v.trim().trim_start_matches('v').split('.');
    let a = it.next()?.parse().ok()?;
    let b = it.next().unwrap_or("0").parse().ok()?;
    let c = it.next().unwrap_or("0").split(|ch: char| !ch.is_ascii_digit()).next()?.parse().ok()?;
    Some((a, b, c))
}

fn parse_hex32(s: &str) -> Option<[u8; 32]> {
    let s = s.as_bytes();
    if s.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    for (i, o) in out.iter_mut().enumerate() {
        let hex = |c: u8| (c as char).to_digit(16);
        *o = (hex(s[2 * i])? * 16 + hex(s[2 * i + 1])?) as u8;
    }
    Some(out)
}

// ---------------------------------------------------------------------------
// Task
// ---------------------------------------------------------------------------

static BUFFERS: ConstStaticCell<Buffers> = ConstStaticCell::new(Buffers::new());

#[embassy_executor::task]
pub async fn update_task(stack: Stack<'static>, flash: &'static FlashMutex) {
    let bufs = BUFFERS.take();
    system::wait_network().await;
    let mut next_auto = Instant::now() + FIRST_CHECK;
    loop {
        let auto = config::with(|c| c.update_check);
        let cmd = if auto {
            match select(COMMAND.wait(), Timer::at(next_auto)).await {
                Either::First(c) => c,
                Either::Second(()) => Command::Check,
            }
        } else {
            COMMAND.wait().await
        };
        if !matches!(net::info().mode, net::NetMode::Client) {
            fail("no internet connection (device is in access point mode)");
            next_auto = Instant::now() + RETRY_AFTER_ERROR;
            continue;
        }
        let ok = match cmd {
            Command::Check => check(stack, bufs).await,
            Command::Install => {
                let ready = STATUS.lock(|s| s.borrow().phase == Phase::Available) || check(stack, bufs).await;
                if ready && STATUS.lock(|s| s.borrow().phase == Phase::Available) {
                    install(stack, bufs, flash).await
                } else {
                    ready
                }
            }
        };
        next_auto = Instant::now() + if ok { CHECK_EVERY } else { RETRY_AFTER_ERROR };
    }
}

// ---------------------------------------------------------------------------
// Check
// ---------------------------------------------------------------------------

struct ManifestSink {
    buf: [u8; 1024],
    len: usize,
}

impl Sink for ManifestSink {
    async fn begin(&mut self, len: Option<u32>) -> Result<(), &'static str> {
        match len {
            Some(l) if l as usize > self.buf.len() => Err("manifest too large"),
            _ => Ok(()),
        }
    }
    async fn data(&mut self, data: &[u8]) -> Result<(), &'static str> {
        let end = self.len + data.len();
        if end > self.buf.len() {
            return Err("manifest too large");
        }
        self.buf[self.len..end].copy_from_slice(data);
        self.len = end;
        Ok(())
    }
}

#[derive(Deserialize)]
struct Manifest<'a> {
    version: &'a str,
    ota: &'a str,
    size: u32,
    sha256: &'a str,
    #[serde(default)]
    notes: &'a str,
}

async fn check(stack: Stack<'static>, bufs: &mut Buffers) -> bool {
    modify(|s| {
        s.phase = Phase::Checking;
        s.error.clear();
    });
    let url = config::with(|c| c.update_url_effective());
    log::info!("update: checking {}", url.as_str());
    let started = Instant::now();
    let mut sink = ManifestSink { buf: [0; 1024], len: 0 };
    if let Err(e) = fetch::get(stack, bufs, &url, &mut sink).await {
        fail(e);
        return false;
    }
    log::info!("update: manifest received in {} ms", started.elapsed().as_millis());
    let mut scratch = [0u8; 256];
    let Ok((m, _)) = serde_json_core::from_slice_escaped::<Manifest>(&sink.buf[..sink.len], &mut scratch) else {
        fail("invalid manifest");
        return false;
    };
    let (Some(latest), Some(installed)) = (parse_version(m.version), parse_version(system::VERSION)) else {
        fail("invalid version in manifest");
        return false;
    };
    let Some(sha) = parse_hex32(m.sha256) else {
        fail("invalid sha256 in manifest");
        return false;
    };
    let mut ota_url = String::new();
    if let Err(e) = fetch::join_url(&url, m.ota, &mut ota_url) {
        fail(e);
        return false;
    }
    let newer = latest > installed;
    log::info!("update: latest {} (installed {}){}", m.version, system::VERSION, if newer { " - update available" } else { "" });
    modify(|s| {
        s.phase = if newer { Phase::Available } else { Phase::UpToDate };
        s.latest = String::try_from(m.version).unwrap_or_default();
        s.notes.clear();
        for c in m.notes.chars() {
            if s.notes.push(c).is_err() {
                break;
            }
        }
        s.checked_at = Some(Instant::now());
        s.ota_url = ota_url;
        s.size = m.size;
        s.sha256 = sha;
    });
    true
}

// ---------------------------------------------------------------------------
// Install
// ---------------------------------------------------------------------------

struct FirmwareSink<'a> {
    updater: system::Updater<'a>,
    hasher: Sha256,
    expected: u32,
    offset: u32,
    last_pct: u8,
}

impl Sink for FirmwareSink<'_> {
    async fn begin(&mut self, len: Option<u32>) -> Result<(), &'static str> {
        match len {
            Some(l) if l != self.expected => Err("download size does not match the manifest"),
            _ => Ok(()),
        }
    }
    async fn data(&mut self, data: &[u8]) -> Result<(), &'static str> {
        if self.offset + data.len() as u32 > self.expected {
            return Err("download larger than expected");
        }
        if self.offset == 0 && !system::looks_like_app_image(data) {
            return Err("not a Pico Light Show application image");
        }
        self.hasher.update(data);
        self.updater.write_firmware(self.offset as usize, data).await.map_err(|e| {
            log::error!("update: flash write failed: {:?}", e);
            "flash write failed"
        })?;
        self.offset += data.len() as u32;
        net::touch_http();
        let pct = (self.offset as u64 * 100 / self.expected.max(1) as u64) as u8;
        if pct != self.last_pct {
            self.last_pct = pct;
            modify(|s| s.progress = pct);
        }
        Ok(())
    }
}

async fn install(stack: Stack<'static>, bufs: &mut Buffers, flash: &'static FlashMutex) -> bool {
    if cfg!(feature = "standalone") {
        fail("updates are not available in the standalone (no bootloader) build");
        return false;
    }
    let (url, size, sha) = STATUS.lock(|s| {
        let s = s.borrow();
        (s.ota_url.clone(), s.size, s.sha256)
    });
    if size < 1024 || size > system::dfu_size() {
        fail("invalid image size in manifest");
        return false;
    }
    let Ok(_lock) = system::OTA_LOCK.try_lock() else {
        fail("another firmware upload is in progress");
        return false;
    };
    log::info!("update: downloading {} ({} bytes)", url.as_str(), size);
    modify(|s| {
        s.phase = Phase::Downloading;
        s.progress = 0;
        s.error.clear();
    });
    let started = Instant::now();
    let mut state_buf = system::StateBuf::new();
    let mut sink = FirmwareSink {
        updater: system::Updater::new(system::updater_config(flash), &mut state_buf.0),
        hasher: Sha256::new(),
        expected: size,
        offset: 0,
        last_pct: 0,
    };
    if let Err(e) = fetch::get(stack, bufs, &url, &mut sink).await {
        fail(e);
        return false;
    }
    if sink.offset != size {
        fail("download incomplete");
        return false;
    }
    let digest: [u8; 32] = sink.hasher.finalize_reset().into();
    if digest != sha {
        fail("checksum mismatch - download corrupted");
        return false;
    }
    if sink.updater.mark_updated().await.is_err() {
        fail("cannot mark the update");
        return false;
    }
    log::info!("update: image verified in {} s, rebooting", started.elapsed().as_secs());
    modify(|s| {
        s.phase = Phase::Installing;
        s.progress = 100;
    });
    // Give MQTT / the web UI a moment to report "installing".
    Timer::after_secs(2).await;
    system::request_reboot();
    true
}
