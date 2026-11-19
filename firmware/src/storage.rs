//! Settings persistence in the STORAGE flash region (wear-levelled key/value
//! map from `sequential-storage`). Values are JSON documents.

use embassy_embedded_hal::flash::partition::Partition;
use embassy_rp::flash::{Async, Flash};
use embassy_rp::peripherals::FLASH;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Timer, with_timeout};
use sequential_storage::cache::{Cache, Uncached};
use sequential_storage::map::{MapConfig, MapStorage};
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::config::{self, DeviceConfig};
use crate::state::{self, MAX_PRESETS, Preset, StoredLight};

pub const FLASH_SIZE: usize = 2 * 1024 * 1024;
pub type FlashDev = Flash<'static, FLASH, Async, FLASH_SIZE>;
pub type FlashMutex = Mutex<CriticalSectionRawMutex, FlashDev>;
type Part = Partition<'static, CriticalSectionRawMutex, FlashDev>;

const KEY_CONFIG: u8 = 1;
const KEY_LIGHT: u8 = 2;
const KEY_PRESETS: u8 = 3;

struct Storage {
    map: MapStorage<u8, Part, Cache<Uncached, Uncached, Uncached, u8>>,
    json: [u8; 3584],
    work: [u8; 4096],
}

static STORAGE: Mutex<CriticalSectionRawMutex, Option<Storage>> = Mutex::new(None);

fn storage_range() -> (u32, u32) {
    unsafe extern "C" {
        static __storage_start: u32;
        static __storage_end: u32;
    }
    unsafe {
        let start = &__storage_start as *const u32 as u32;
        let end = &__storage_end as *const u32 as u32;
        (start, end)
    }
}

pub async fn init(flash: &'static FlashMutex) {
    let (start, end) = storage_range();
    let part = Partition::new(flash, start, end - start);
    let size = end - start;
    let map = MapStorage::new(part, MapConfig::new(0..size), Cache::new_uncached());
    *STORAGE.lock().await = Some(Storage { map, json: [0; 3584], work: [0; 4096] });
}

async fn load<T: DeserializeOwned>(key: u8) -> Option<T> {
    let mut guard = STORAGE.lock().await;
    let st = guard.as_mut()?;
    let Storage { map, json, work } = st;
    let len = match map.fetch_item::<&[u8]>(work, &key).await {
        Ok(Some(data)) => {
            let n = data.len().min(json.len());
            json[..n].copy_from_slice(&data[..n]);
            n
        }
        Ok(None) => return None,
        Err(e) => {
            log::warn!("storage: read key {} failed: {:?}", key, e);
            if matches!(e, sequential_storage::Error::Corrupted { .. }) {
                log::warn!("storage: corrupted, erasing");
                let _ = map.erase_all().await;
            }
            return None;
        }
    };
    match serde_json_core::from_slice::<T>(&json[..len]) {
        Ok((v, _)) => Some(v),
        Err(e) => {
            log::warn!("storage: key {} has invalid JSON: {:?}", key, e);
            None
        }
    }
}

/// Store a value; `fill` serializes it synchronously into the storage buffer,
/// so callers never keep large values alive across the flash write.
async fn store_with(key: u8, fill: impl FnOnce(&mut [u8]) -> Option<usize>) -> bool {
    let mut guard = STORAGE.lock().await;
    let Some(st) = guard.as_mut() else { return false };
    let Storage { map, json, work } = st;
    let Some(len) = fill(json) else {
        log::error!("storage: value for key {} too large", key);
        return false;
    };
    let data: &[u8] = &json[..len];
    match map.store_item(work, &key, &data).await {
        Ok(()) => true,
        Err(e) => {
            log::error!("storage: write key {} failed: {:?}", key, e);
            false
        }
    }
}

fn to_json<T: Serialize>(v: &T, buf: &mut [u8]) -> Option<usize> {
    serde_json_core::to_slice(v, buf).ok()
}

pub async fn load_config() -> DeviceConfig {
    let mut cfg: DeviceConfig = load(KEY_CONFIG).await.unwrap_or_default();
    cfg.sanitize();
    cfg
}

/// Persist the active configuration (`config::CONFIG`).
pub async fn save_config() -> bool {
    store_with(KEY_CONFIG, |buf| config::with(|c| to_json(c, buf))).await
}

pub async fn load_light() -> Option<StoredLight> {
    load(KEY_LIGHT).await
}

pub async fn load_presets() -> heapless::Vec<Preset, MAX_PRESETS> {
    load(KEY_PRESETS).await.unwrap_or_default()
}

pub async fn save_presets() -> bool {
    store_with(KEY_PRESETS, |buf| state::PRESETS.lock(|p| to_json(&*p.borrow(), buf))).await
}

pub async fn factory_reset() {
    let mut guard = STORAGE.lock().await;
    if let Some(st) = guard.as_mut() {
        if let Err(e) = st.map.erase_all().await {
            log::error!("storage: erase failed: {:?}", e);
        }
    }
}

/// Saves the light state a few seconds after the last change, so that slider
/// movements do not wear out the flash.
#[embassy_executor::task]
pub async fn saver_task() {
    let mut rx = state::CHANGED.receiver().expect("watch receivers exhausted");
    loop {
        rx.changed().await;
        // Debounce: wait until there was no change for 5 s.
        while with_timeout(Duration::from_secs(5), rx.changed()).await.is_ok() {}
        if store_with(KEY_LIGHT, |buf| to_json(&StoredLight::from_state(&state::snapshot()), buf)).await {
            log::info!("storage: light state saved");
        }
        Timer::after_millis(100).await;
    }
}
