//! Shared light state (power, brightness, effect, ...), the single source of
//! truth for the web UI, MQTT and the render core.
//!
//! Every modification goes through [`update`], which validates the state,
//! bumps a version counter and notifies all listeners (render core, MQTT,
//! WebSocket clients, flash saver).

use core::cell::RefCell;

use embassy_sync::blocking_mutex::Mutex;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::watch::Watch;
use heapless::{String, Vec};
use portable_atomic::{AtomicU32, Ordering};
use serde::{Deserialize, Serialize};

use crate::color::Rgb;
use crate::effects::{self, EFFECT_COUNT, EffectParams, MAX_COLORS, MAX_OPTS};
use crate::transitions::{DEFAULT_TRANSITION, TRANSITIONS};

#[derive(Clone, Debug)]
pub struct LightState {
    pub on: bool,
    pub brightness: u8,
    pub speed: u8,
    /// false = animation frozen
    pub running: bool,
    pub effect: u8,
    pub transition: u8,
    pub transition_ms: u16,
    pub params: [EffectParams; EFFECT_COUNT],
}

impl LightState {
    pub const INIT: LightState = LightState {
        on: false,
        brightness: 128,
        speed: 200,
        running: true,
        effect: 0,
        transition: DEFAULT_TRANSITION,
        transition_ms: 800,
        params: [EffectParams::EMPTY; EFFECT_COUNT],
    };

    pub fn defaults() -> Self {
        let mut s = Self::INIT;
        for (i, p) in s.params.iter_mut().enumerate() {
            *p = effects::default_params(i);
        }
        s
    }

    pub fn current(&self) -> &EffectParams {
        &self.params[self.effect as usize]
    }

    fn sanitize(&mut self) {
        if self.effect as usize >= EFFECT_COUNT {
            self.effect = 0;
        }
        if self.transition as usize >= TRANSITIONS.len() {
            self.transition = DEFAULT_TRANSITION;
        }
        self.transition_ms = self.transition_ms.min(10_000);
        self.speed = self.speed.max(1);
        for (i, p) in self.params.iter_mut().enumerate() {
            effects::sanitize_params(i, p);
        }
    }
}

pub static LIGHT: Mutex<CriticalSectionRawMutex, RefCell<LightState>> = Mutex::new(RefCell::new(LightState::INIT));
static VERSION: AtomicU32 = AtomicU32::new(1);
/// Broadcasts the state version after each change.
pub static CHANGED: Watch<CriticalSectionRawMutex, u32, 8> = Watch::new();

pub fn version() -> u32 {
    VERSION.load(Ordering::Acquire)
}

pub fn snapshot() -> LightState {
    LIGHT.lock(|s| s.borrow().clone())
}

/// Replace the whole state (used at boot). Does not notify the saver.
pub fn init(state: LightState) {
    let mut state = state;
    state.sanitize();
    LIGHT.lock(|s| *s.borrow_mut() = state);
    VERSION.fetch_add(1, Ordering::AcqRel);
}

/// Modify the state and notify listeners.
pub fn update(f: impl FnOnce(&mut LightState)) {
    LIGHT.lock(|s| {
        let mut s = s.borrow_mut();
        f(&mut s);
        s.sanitize();
    });
    let v = VERSION.fetch_add(1, Ordering::AcqRel) + 1;
    CHANGED.sender().send(v);
}

/// Partial update as received from the web API, WebSocket or MQTT.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct StateUpdate {
    pub on: Option<bool>,
    pub toggle: Option<bool>,
    pub brightness: Option<u8>,
    pub speed: Option<u8>,
    pub running: Option<bool>,
    pub effect: Option<u8>,
    pub transition: Option<u8>,
    pub transition_ms: Option<u16>,
    pub colors: Option<Vec<Rgb, MAX_COLORS>>,
    pub opts: Option<Vec<u8, MAX_OPTS>>,
    /// Apply a stored preset (index).
    pub preset: Option<u8>,
    /// Reset colors and options of the current effect to its defaults.
    pub reset: Option<bool>,
}

pub fn apply(u: &StateUpdate) {
    let preset = u.preset.and_then(|i| PRESETS.lock(|p| p.borrow().get(i as usize).cloned()));
    update(|s| {
        if let Some(p) = &preset {
            if (p.effect as usize) < EFFECT_COUNT {
                s.effect = p.effect;
                s.brightness = p.brightness;
                s.speed = p.speed;
                s.params[p.effect as usize] = p.params.clone();
                s.on = true;
            }
        }
        if let Some(v) = u.effect {
            if (v as usize) < EFFECT_COUNT {
                s.effect = v;
            }
        }
        if let Some(v) = u.on {
            s.on = v;
        }
        if u.toggle == Some(true) {
            s.on = !s.on;
        }
        if let Some(v) = u.brightness {
            s.brightness = v;
        }
        if let Some(v) = u.speed {
            s.speed = v;
        }
        if let Some(v) = u.running {
            s.running = v;
        }
        if let Some(v) = u.transition {
            s.transition = v;
        }
        if let Some(v) = u.transition_ms {
            s.transition_ms = v;
        }
        let e = s.effect as usize;
        if u.reset == Some(true) {
            s.params[e] = effects::default_params(e);
        }
        if let Some(c) = &u.colors {
            s.params[e].colors = c.clone();
        }
        if let Some(o) = &u.opts {
            for (i, v) in o.iter().enumerate() {
                s.params[e].opts[i] = *v;
            }
        }
    });
}

/// Compact view of the state for the UI / API.
#[derive(Serialize)]
pub struct StateView {
    pub on: bool,
    pub brightness: u8,
    pub speed: u8,
    pub running: bool,
    pub effect: u8,
    pub transition: u8,
    pub transition_ms: u16,
    pub colors: Vec<Rgb, MAX_COLORS>,
    pub opts: Vec<u8, MAX_OPTS>,
    pub ddp: bool,
}

pub fn view() -> StateView {
    let ddp = crate::ddp::is_active();
    LIGHT.lock(|s| {
        let s = s.borrow();
        let p = s.current();
        let n_opts = effects::EFFECTS[s.effect as usize].opts.len();
        StateView {
            on: s.on,
            brightness: s.brightness,
            speed: s.speed,
            running: s.running,
            effect: s.effect,
            transition: s.transition,
            transition_ms: s.transition_ms,
            colors: p.colors.clone(),
            opts: Vec::from_slice(&p.opts[..n_opts]).unwrap_or_default(),
            ddp,
        }
    })
}

/// Serialize the current state as JSON into `buf`, returns the length.
pub fn state_json(buf: &mut [u8]) -> usize {
    serde_json_core::to_slice(&view(), buf).unwrap_or(0)
}

// ---------------------------------------------------------------------------
// Persistence format (JSON in flash)
// ---------------------------------------------------------------------------

/// Light state as stored in flash. Params are a list so that adding effects
/// in later firmware versions keeps the existing ones.
#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
pub struct StoredLight {
    pub on: bool,
    pub brightness: u8,
    pub speed: u8,
    pub running: bool,
    pub effect: u8,
    pub transition: u8,
    pub transition_ms: u16,
    pub params: Vec<EffectParams, 32>,
}

impl StoredLight {
    pub fn from_state(s: &LightState) -> Self {
        let mut params = Vec::new();
        for p in &s.params {
            let _ = params.push(p.clone());
        }
        Self {
            on: s.on,
            brightness: s.brightness,
            speed: s.speed,
            running: s.running,
            effect: s.effect,
            transition: s.transition,
            transition_ms: s.transition_ms,
            params,
        }
    }

    pub fn into_state(self) -> LightState {
        let mut s = LightState::defaults();
        s.on = self.on;
        s.brightness = self.brightness;
        s.speed = self.speed;
        s.running = self.running;
        s.effect = self.effect;
        s.transition = self.transition;
        s.transition_ms = self.transition_ms;
        for (i, p) in self.params.into_iter().enumerate().take(EFFECT_COUNT) {
            if !(p.colors.is_empty() && effects::EFFECTS[i].colors_min > 0) {
                s.params[i] = p;
            }
        }
        s
    }
}

// ---------------------------------------------------------------------------
// Presets
// ---------------------------------------------------------------------------

pub const MAX_PRESETS: usize = 8;

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Preset {
    pub name: String<24>,
    pub effect: u8,
    pub brightness: u8,
    pub speed: u8,
    pub params: EffectParams,
}

pub static PRESETS: Mutex<CriticalSectionRawMutex, RefCell<Vec<Preset, MAX_PRESETS>>> =
    Mutex::new(RefCell::new(Vec::new()));

/// Store a preset (`None` = the current light state). Replaces a preset with the same name.
pub fn save_preset(name: &str, preset: Option<Preset>) -> bool {
    let mut n: String<24> = String::new();
    for c in name.chars() {
        if n.push(c).is_err() {
            break;
        }
    }
    let mut preset = match preset {
        Some(p) => p,
        None => {
            let s = snapshot();
            Preset { name: String::new(), effect: s.effect, brightness: s.brightness, speed: s.speed, params: s.current().clone() }
        }
    };
    if preset.effect as usize >= EFFECT_COUNT {
        return false;
    }
    effects::sanitize_params(preset.effect as usize, &mut preset.params);
    preset.name = n;
    PRESETS.lock(|p| {
        let mut p = p.borrow_mut();
        if let Some(existing) = p.iter_mut().find(|x| x.name == preset.name) {
            *existing = preset;
            true
        } else {
            p.push(preset).is_ok()
        }
    })
}

pub fn delete_preset(index: usize) -> bool {
    PRESETS.lock(|p| {
        let mut p = p.borrow_mut();
        if index < p.len() {
            p.remove(index);
            true
        } else {
            false
        }
    })
}
