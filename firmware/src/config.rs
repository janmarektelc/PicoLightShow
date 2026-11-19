//! Persistent device configuration (network, LED hardware, MQTT, ...).
//!
//! Stored in flash as JSON so that adding or removing fields in a future
//! firmware version keeps the remaining settings (`#[serde(default)]`).

use core::cell::RefCell;

use embassy_sync::blocking_mutex::Mutex;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use heapless::String;
use serde::{Deserialize, Serialize};

pub const MAX_LEDS: usize = 1000;
/// Prefix of the automatic access point name (`PicoLightShow-A1B2C3`).
pub const DEFAULT_AP_SSID: &str = "PicoLightShow";
/// Prefix of the automatic host name (`pico-light-show-a1b2c3`).
pub const DEFAULT_HOSTNAME: &str = "pico-light-show";
pub const DEFAULT_AP_PASS: &str = "picolight";
pub const DEFAULT_LED_PIN: u8 = 14;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WifiMode {
    Ap,
    Client,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ColorOrder {
    Grb,
    Rgb,
    Brg,
    Rbg,
    Gbr,
    Bgr,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BootPower {
    Off,
    On,
    Restore,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct DeviceConfig {
    /// Device name; also used as DHCP host name and mDNS name (`<name>.local`).
    /// Empty = automatic, derived from the MAC address.
    pub name: String<32>,

    pub wifi_mode: WifiMode,
    pub wifi_ssid: String<32>,
    pub wifi_pass: String<64>,
    pub dhcp: bool,
    pub ip: [u8; 4],
    pub netmask: [u8; 4],
    pub gateway: [u8; 4],
    pub dns: [u8; 4],

    /// Access point used in AP mode and as fallback when the client connection fails.
    /// Empty SSID = automatic, derived from the MAC address.
    pub ap_ssid: String<32>,
    pub ap_pass: String<64>,

    pub led_count: u16,
    pub led_pin: u8,
    pub color_order: ColorOrder,
    /// Power supply limit in mA, 0 = unlimited.
    pub max_current_ma: u16,
    pub gamma: bool,
    pub boot_power: BootPower,

    pub mqtt_enabled: bool,
    pub mqtt_host: String<64>,
    pub mqtt_port: u16,
    pub mqtt_user: String<32>,
    pub mqtt_pass: String<64>,
    pub mqtt_discovery_prefix: String<32>,
    /// Base topic; empty = `picolightshow/<mac>`.
    pub mqtt_topic: String<48>,

    pub ddp_enabled: bool,

    /// Password protecting settings, OTA and reboot (HTTP basic auth, user `admin`). Empty = no auth.
    pub admin_pass: String<32>,
}

fn s<const N: usize>(v: &str) -> String<N> {
    let mut out = String::new();
    let _ = out.push_str(v);
    out
}

impl Default for DeviceConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            wifi_mode: WifiMode::Ap,
            wifi_ssid: String::new(),
            wifi_pass: String::new(),
            dhcp: true,
            ip: [192, 168, 1, 50],
            netmask: [255, 255, 255, 0],
            gateway: [192, 168, 1, 1],
            dns: [192, 168, 1, 1],
            ap_ssid: String::new(),
            ap_pass: s(DEFAULT_AP_PASS),
            led_count: 30,
            led_pin: DEFAULT_LED_PIN,
            color_order: ColorOrder::Grb,
            max_current_ma: 0,
            gamma: true,
            boot_power: BootPower::Off,
            mqtt_enabled: false,
            mqtt_host: String::new(),
            mqtt_port: 1883,
            mqtt_user: String::new(),
            mqtt_pass: String::new(),
            mqtt_discovery_prefix: s("homeassistant"),
            mqtt_topic: String::new(),
            ddp_enabled: true,
            admin_pass: String::new(),
        }
    }
}

impl DeviceConfig {
    /// Clamp values into supported ranges after loading / receiving them.
    pub fn sanitize(&mut self) {
        self.led_count = self.led_count.clamp(1, MAX_LEDS as u16);
        if !is_valid_led_pin(self.led_pin) {
            self.led_pin = DEFAULT_LED_PIN;
        }
        // Older firmware stored fixed defaults; turn them into "automatic".
        if self.ap_ssid.as_str() == DEFAULT_AP_SSID {
            self.ap_ssid.clear();
        }
        // WPA2 needs 8..=63 characters.
        if !self.ap_pass.is_empty() && self.ap_pass.len() < 8 {
            self.ap_pass = s(DEFAULT_AP_PASS);
        }
        if self.name.as_str() == DEFAULT_HOSTNAME {
            self.name.clear();
        }
        self.name = String::try_from(self.name.trim()).unwrap_or_default();
        if self.mqtt_port == 0 {
            self.mqtt_port = 1883;
        }
        if self.mqtt_discovery_prefix.is_empty() {
            self.mqtt_discovery_prefix = s("homeassistant");
        }
    }

    /// Host name for DHCP / mDNS: lowercase alphanumerics and dashes. The
    /// automatic name is `pico-light-show-<last 3 MAC bytes>`.
    pub fn hostname(&self) -> String<32> {
        let mut out: String<32> = String::new();
        for c in self.name.chars() {
            let c = c.to_ascii_lowercase();
            let c = if c.is_ascii_alphanumeric() { c } else { '-' };
            if !(c == '-' && (out.is_empty() || out.ends_with('-'))) {
                let _ = out.push(c);
            }
        }
        while out.ends_with('-') {
            out.pop();
        }
        if out.is_empty() {
            out = default_hostname();
        }
        out
    }

    /// Name shown in the UI and in Home Assistant.
    pub fn display_name(&self) -> String<32> {
        if self.name.is_empty() { self.hostname() } else { self.name.clone() }
    }

    /// SSID of the access point (automatic `PicoLightShow-A1B2C3` when empty).
    pub fn ap_ssid_effective(&self) -> String<32> {
        if self.ap_ssid.is_empty() { default_ap_ssid() } else { self.ap_ssid.clone() }
    }
}

// ---------------------------------------------------------------------------
// MAC based defaults
// ---------------------------------------------------------------------------

static MAC: Mutex<CriticalSectionRawMutex, core::cell::Cell<[u8; 6]>> = Mutex::new(core::cell::Cell::new([0; 6]));

/// Called once the Wi-Fi chip reported its MAC address.
pub fn set_mac(mac: [u8; 6]) {
    MAC.lock(|m| m.set(mac));
}

fn mac_suffix(upper: bool) -> String<6> {
    let mac = MAC.lock(|m| m.get());
    let mut out = String::new();
    for b in &mac[3..] {
        let _ = if upper {
            core::fmt::write(&mut out, format_args!("{:02X}", b))
        } else {
            core::fmt::write(&mut out, format_args!("{:02x}", b))
        };
    }
    out
}

pub fn default_hostname() -> String<32> {
    let mut out = s(DEFAULT_HOSTNAME);
    let _ = out.push('-');
    let _ = out.push_str(&mac_suffix(false));
    out
}

pub fn default_ap_ssid() -> String<32> {
    let mut out = s(DEFAULT_AP_SSID);
    let _ = out.push('-');
    let _ = out.push_str(&mac_suffix(true));
    out
}

/// GPIOs usable for the LED data line. 23, 24, 25 and 29 are wired to the
/// CYW43 Wi-Fi chip on the Pico W.
pub fn is_valid_led_pin(pin: u8) -> bool {
    matches!(pin, 0..=22 | 26..=28)
}

/// LED output settings that can change at runtime (read by the render core).
#[derive(Clone, Copy, Debug)]
pub struct LedSettings {
    pub count: u16,
    pub color_order: ColorOrder,
    pub max_current_ma: u16,
    pub gamma: bool,
}

impl LedSettings {
    pub const fn new() -> Self {
        Self { count: 30, color_order: ColorOrder::Grb, max_current_ma: 0, gamma: true }
    }
    pub fn from_config(c: &DeviceConfig) -> Self {
        Self { count: c.led_count, color_order: c.color_order, max_current_ma: c.max_current_ma, gamma: c.gamma }
    }
}

pub static LED_SETTINGS: Mutex<CriticalSectionRawMutex, RefCell<LedSettings>> =
    Mutex::new(RefCell::new(LedSettings::new()));

pub fn led_settings() -> LedSettings {
    LED_SETTINGS.lock(|s| *s.borrow())
}

pub fn set_led_settings(v: LedSettings) {
    LED_SETTINGS.lock(|s| *s.borrow_mut() = v);
}

/// The active configuration (as loaded at boot or last saved). A blocking
/// mutex: access is synchronous, so async handlers never hold a copy of the
/// (large) config across an await point.
pub static CONFIG: Mutex<CriticalSectionRawMutex, RefCell<Option<DeviceConfig>>> = Mutex::new(RefCell::new(None));

/// Clone of the active configuration (only use outside of async handlers).
pub fn get() -> DeviceConfig {
    CONFIG.lock(|c| c.borrow().clone().unwrap_or_default())
}

/// Read access without copying the whole config.
pub fn with<R>(f: impl FnOnce(&DeviceConfig) -> R) -> R {
    CONFIG.lock(|c| match c.borrow().as_ref() {
        Some(cfg) => f(cfg),
        None => f(&DeviceConfig::default()),
    })
}

pub fn set(cfg: DeviceConfig) {
    set_led_settings(LedSettings::from_config(&cfg));
    CONFIG.lock(|c| *c.borrow_mut() = Some(cfg));
}
