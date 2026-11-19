//! Light effects.
//!
//! Every effect is described by an [`EffectInfo`] (name, number of colors and
//! options). The web UI builds its forms from this metadata, so a new effect
//! only needs an entry in [`EFFECTS`] and a branch in [`EffectRunner`].
//!
//! Effects advance in discrete *steps* (rate given by the speed setting) and
//! are *rendered* every frame.

use heapless::Vec;
use serde::{Deserialize, Serialize};

use crate::color::{Rgb, Rng};
use crate::config::MAX_LEDS;

pub const MAX_COLORS: usize = 8;
pub const MAX_OPTS: usize = 4;

/// User adjustable parameters of one effect.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct EffectParams {
    pub colors: Vec<Rgb, MAX_COLORS>,
    pub opts: [u8; MAX_OPTS],
}

impl EffectParams {
    pub const EMPTY: EffectParams = EffectParams { colors: Vec::new(), opts: [0; MAX_OPTS] };
}

impl Default for EffectParams {
    fn default() -> Self {
        Self::EMPTY
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Solid,
    Pattern,
    ColorChange,
    Rainbow,
    RainbowCycle,
    TheaterChase,
    Comet,
    Scanner,
    Fire,
    Twinkle,
    Gradient,
    Plasma,
    Candle,
    Sparkle,
}

#[derive(Serialize)]
pub struct OptInfo {
    pub label: &'static str,
    /// "range", "toggle" or "select"
    pub kind: &'static str,
    pub min: u8,
    pub max: u8,
    pub choices: &'static [&'static str],
}

const fn range(label: &'static str, min: u8, max: u8) -> OptInfo {
    OptInfo { label, kind: "range", min, max, choices: &[] }
}
const fn toggle(label: &'static str) -> OptInfo {
    OptInfo { label, kind: "toggle", min: 0, max: 1, choices: &[] }
}
const fn select(label: &'static str, choices: &'static [&'static str]) -> OptInfo {
    OptInfo { label, kind: "select", min: 0, max: choices.len() as u8 - 1, choices }
}

#[derive(Serialize)]
pub struct EffectInfo {
    pub name: &'static str,
    pub colors_min: u8,
    pub colors_max: u8,
    pub opts: &'static [OptInfo],
    #[serde(skip)]
    pub kind: Kind,
    #[serde(skip)]
    pub default_colors: &'static [u32],
    #[serde(skip)]
    pub default_opts: [u8; MAX_OPTS],
}

const PATTERN_OPTS: &[OptInfo] = &[
    select("Draw mode", &["Once", "Repeat", "Stretch"]),
    toggle("Ping pong"),
    select("Direction", &["Left", "Right"]),
];
const RAINBOW6: &[u32] = &[0xff0000, 0xffff00, 0x00ff00, 0x00ffff, 0x0000ff, 0xff00ff];

pub static EFFECTS: [EffectInfo; EFFECT_COUNT] = [
    EffectInfo { name: "Solid color", kind: Kind::Solid, colors_min: 1, colors_max: 1, opts: &[], default_colors: &[0xffffff], default_opts: [0; 4] },
    EffectInfo { name: "Running point", kind: Kind::Pattern, colors_min: 1, colors_max: 8, opts: PATTERN_OPTS, default_colors: &[0xff0000], default_opts: [0, 1, 1, 0] },
    EffectInfo { name: "Running colors", kind: Kind::Pattern, colors_min: 1, colors_max: 8, opts: PATTERN_OPTS, default_colors: RAINBOW6, default_opts: [2, 1, 0, 0] },
    EffectInfo { name: "Running colors 2", kind: Kind::Pattern, colors_min: 1, colors_max: 8, opts: PATTERN_OPTS, default_colors: RAINBOW6, default_opts: [1, 0, 1, 0] },
    EffectInfo { name: "Snakes", kind: Kind::Pattern, colors_min: 1, colors_max: 8, opts: PATTERN_OPTS, default_colors: &[0xff0000, 0, 0x00ff00, 0, 0x0000ff, 0], default_opts: [2, 0, 0, 0] },
    EffectInfo { name: "Color change", kind: Kind::ColorChange, colors_min: 2, colors_max: 8, opts: &[range("Smoothness", 10, 250)], default_colors: &[0x640000, 0xff0000, 0x006400, 0x00ff00, 0x000064, 0x0000ff], default_opts: [100, 0, 0, 0] },
    EffectInfo { name: "Breath", kind: Kind::ColorChange, colors_min: 2, colors_max: 8, opts: &[range("Smoothness", 10, 250)], default_colors: &[0x640000, 0xff0000], default_opts: [100, 0, 0, 0] },
    EffectInfo { name: "Rainbow", kind: Kind::Rainbow, colors_min: 0, colors_max: 0, opts: &[range("Rainbows on strip", 1, 10)], default_colors: &[], default_opts: [1, 0, 0, 0] },
    EffectInfo { name: "Rainbow cycle", kind: Kind::RainbowCycle, colors_min: 0, colors_max: 0, opts: &[], default_colors: &[], default_opts: [0; 4] },
    EffectInfo { name: "Theater chase", kind: Kind::TheaterChase, colors_min: 1, colors_max: 4, opts: &[range("Gap", 2, 8)], default_colors: &[0xffb050], default_opts: [3, 0, 0, 0] },
    EffectInfo { name: "Comet", kind: Kind::Comet, colors_min: 1, colors_max: 1, opts: &[range("Tail length", 2, 60), toggle("Ping pong")], default_colors: &[0x00a0ff], default_opts: [12, 0, 0, 0] },
    EffectInfo { name: "Scanner", kind: Kind::Scanner, colors_min: 1, colors_max: 1, opts: &[range("Width", 1, 20)], default_colors: &[0xff0000], default_opts: [4, 0, 0, 0] },
    EffectInfo { name: "Fire", kind: Kind::Fire, colors_min: 0, colors_max: 0, opts: &[range("Cooling", 20, 100), range("Sparking", 50, 200)], default_colors: &[], default_opts: [55, 120, 0, 0] },
    EffectInfo { name: "Twinkle", kind: Kind::Twinkle, colors_min: 1, colors_max: 8, opts: &[range("Density", 1, 100)], default_colors: &[0xffffff, 0xffd080, 0x80c0ff], default_opts: [30, 0, 0, 0] },
    EffectInfo { name: "Gradient", kind: Kind::Gradient, colors_min: 2, colors_max: 8, opts: &[toggle("Moving")], default_colors: &[0xff0000, 0x0000ff], default_opts: [0, 0, 0, 0] },
    EffectInfo { name: "Plasma", kind: Kind::Plasma, colors_min: 0, colors_max: 0, opts: &[range("Scale", 1, 10)], default_colors: &[], default_opts: [3, 0, 0, 0] },
    EffectInfo { name: "Candle", kind: Kind::Candle, colors_min: 1, colors_max: 1, opts: &[range("Flicker", 0, 100)], default_colors: &[0xff9329], default_opts: [50, 0, 0, 0] },
    EffectInfo { name: "Sparkle", kind: Kind::Sparkle, colors_min: 2, colors_max: 2, opts: &[range("Density", 1, 100)], default_colors: &[0x101040, 0xffffff], default_opts: [20, 0, 0, 0] },
];

pub const EFFECT_COUNT: usize = 18;
const _: () = assert!(EFFECT_COUNT <= 32);

pub fn default_params(id: usize) -> EffectParams {
    let info = &EFFECTS[id];
    let mut colors = Vec::new();
    for c in info.default_colors {
        let _ = colors.push(Rgb::hex(*c));
    }
    EffectParams { colors, opts: info.default_opts }
}

/// Make params consistent with the effect description.
pub fn sanitize_params(id: usize, p: &mut EffectParams) {
    let info = &EFFECTS[id];
    let defaults = default_params(id);
    let max = info.colors_max as usize;
    let min = info.colors_min as usize;
    p.colors.truncate(max);
    while p.colors.len() < min {
        let c = defaults.colors.get(p.colors.len()).copied().unwrap_or(Rgb::WHITE);
        let _ = p.colors.push(c);
    }
    for (i, o) in info.opts.iter().enumerate() {
        if p.opts[i] < o.min || p.opts[i] > o.max {
            p.opts[i] = defaults.opts[i].clamp(o.min, o.max);
        }
    }
    for i in info.opts.len()..MAX_OPTS {
        p.opts[i] = 0;
    }
}

/// Speed (1..=255) to step interval in ms (same curve as the original firmware: 100 ms .. 15 ms).
pub fn step_interval_ms(speed: u8) -> u32 {
    let speed = speed.max(1) as u32;
    15 + ((255 - speed) * 85) / 254
}

/// Approximate sine, input 0..=255 is one period, output 0..=255.
pub fn sin8(x: u8) -> u8 {
    // Parabolic approximation of each half wave.
    let half = x & 0x7f; // 0..127
    let t = half as i32 * 2; // 0..254
    let y = (t * (255 - t)) / 64; // 0..~254
    let y = y.min(127);
    if x < 128 { (128 + y) as u8 } else { (127 - y) as u8 }
}

/// Runtime state of the active effect. One instance lives on the render core.
pub struct EffectRunner {
    id: usize,
    time: i32,
    dir: i32,
    idx: usize,
    phase: u32,
    a: [u8; MAX_LEDS],
    b: [u8; MAX_LEDS],
}

impl EffectRunner {
    pub const fn new() -> Self {
        Self { id: 0, time: 0, dir: 1, idx: 0, phase: 0, a: [0; MAX_LEDS], b: [0; MAX_LEDS] }
    }

    pub fn reset(&mut self, id: usize, p: &EffectParams) {
        self.id = id.min(EFFECTS.len() - 1);
        self.time = 0;
        self.idx = 0;
        self.phase = 0;
        self.a.fill(0);
        self.b.fill(0);
        self.dir = match EFFECTS[self.id].kind {
            Kind::Pattern => {
                if p.opts[2] == 1 { 1 } else { -1 }
            }
            _ => 1,
        };
    }

    /// Called when parameters change without switching the effect.
    pub fn params_changed(&mut self, p: &EffectParams) {
        match EFFECTS[self.id].kind {
            Kind::Pattern => {
                let d = if p.opts[2] == 1 { 1 } else { -1 };
                if d != self.dir {
                    self.dir = d;
                }
                self.time = 0;
            }
            Kind::ColorChange => {
                if self.idx >= p.colors.len() {
                    self.idx = 0;
                }
            }
            _ => {}
        }
    }

    /// Advance the animation by one step.
    pub fn step(&mut self, p: &EffectParams, n: usize, rng: &mut Rng) {
        if n == 0 {
            return;
        }
        let n_i = n as i32;
        let colors = &p.colors;
        match EFFECTS[self.id].kind {
            Kind::Solid => {}
            Kind::Pattern => {
                let size = colors.len().max(1) as i32;
                let ping_pong = p.opts[1] != 0;
                let limit = match p.opts[0] {
                    0 => (n_i - size).max(1), // once
                    1 => size,                // repeat
                    _ => (n_i - 1).max(1),    // stretch
                };
                self.time += self.dir;
                if self.time >= limit {
                    if ping_pong {
                        self.dir = -self.dir;
                        self.time = limit;
                    } else {
                        self.time = 0;
                    }
                }
                if self.time < 0 {
                    if ping_pong {
                        self.dir = -self.dir;
                        self.time = 1.min(limit);
                    } else {
                        self.time = limit - 1;
                    }
                }
            }
            Kind::ColorChange => {
                let steps = p.opts[0].max(10) as i32;
                self.time += 1;
                if self.time >= steps {
                    self.time = 0;
                    self.idx = (self.idx + 1) % colors.len().max(1);
                }
            }
            Kind::Rainbow | Kind::RainbowCycle | Kind::Plasma => {
                self.phase = self.phase.wrapping_add(1);
            }
            Kind::TheaterChase => {
                self.phase = self.phase.wrapping_add(1);
            }
            Kind::Comet => {
                let ping_pong = p.opts[1] != 0;
                self.time += self.dir;
                if ping_pong {
                    if self.time >= n_i - 1 {
                        self.time = n_i - 1;
                        self.dir = -1;
                    } else if self.time <= 0 {
                        self.time = 0;
                        self.dir = 1;
                    }
                } else {
                    let tail = p.opts[0] as i32;
                    if self.time >= n_i + tail {
                        self.time = 0;
                    }
                }
            }
            Kind::Scanner => {
                self.time += self.dir;
                if self.time >= n_i - 1 {
                    self.time = n_i - 1;
                    self.dir = -1;
                } else if self.time <= 0 {
                    self.time = 0;
                    self.dir = 1;
                }
            }
            Kind::Fire => {
                // Fire2012 by Mark Kriegsman, heat in `a`.
                let cooling = p.opts[0] as u32;
                let sparking = p.opts[1] as u32;
                let heat = &mut self.a[..n];
                for h in heat.iter_mut() {
                    let cool = rng.below((cooling * 10) / n as u32 + 2) as u8;
                    *h = h.saturating_sub(cool);
                }
                for k in (2..n).rev() {
                    heat[k] = ((heat[k - 1] as u16 + heat[k - 2] as u16 * 2) / 3) as u8;
                }
                if rng.below(255) < sparking {
                    let y = rng.below(7.min(n as u32)) as usize;
                    heat[y] = heat[y].saturating_add(160 + rng.below(96) as u8);
                }
            }
            Kind::Twinkle => {
                // a = brightness, b = color index | 0x80 when fading out
                let density = p.opts[0] as u32;
                let nc = colors.len().max(1) as u32;
                for i in 0..n {
                    let lvl = self.a[i];
                    let falling = self.b[i] & 0x80 != 0;
                    if lvl == 0 {
                        if rng.below(2000) < density {
                            self.a[i] = 1;
                            self.b[i] = rng.below(nc) as u8;
                        }
                    } else if !falling {
                        let v = lvl as u16 + 24;
                        if v >= 255 {
                            self.a[i] = 255;
                            self.b[i] |= 0x80;
                        } else {
                            self.a[i] = v as u8;
                        }
                    } else {
                        let v = lvl.saturating_sub(8);
                        self.a[i] = v;
                        if v == 0 {
                            self.b[i] = 0;
                        }
                    }
                }
            }
            Kind::Gradient => {
                self.phase = self.phase.wrapping_add(4);
            }
            Kind::Candle => {
                // a = current level, b = target level
                let flicker = p.opts[0] as u32;
                for i in 0..n {
                    if self.a[i] == self.b[i] || rng.below(8) == 0 {
                        let dip = rng.below(flicker * 2 + 1).min(200) as u8;
                        self.b[i] = 255 - dip;
                    }
                    let cur = self.a[i] as i16;
                    let tgt = self.b[i] as i16;
                    let d = (tgt - cur).clamp(-24, 24);
                    self.a[i] = (cur + d) as u8;
                }
            }
            Kind::Sparkle => {
                let density = p.opts[0] as u32;
                for v in self.a[..n].iter_mut() {
                    *v = v.saturating_sub(40);
                }
                let sparks = (n as u32 * density) / 400 + 1;
                for _ in 0..sparks {
                    if rng.below(100) < density {
                        let i = rng.below(n as u32) as usize;
                        self.a[i] = 255;
                    }
                }
            }
        }
    }

    /// Draw the current animation state into `buf` (length = LED count).
    pub fn render(&mut self, p: &EffectParams, buf: &mut [Rgb]) {
        let n = buf.len();
        if n == 0 {
            return;
        }
        let colors = &p.colors;
        let c0 = colors.first().copied().unwrap_or(Rgb::WHITE);
        match EFFECTS[self.id].kind {
            Kind::Solid => buf.fill(c0),
            Kind::Pattern => {
                let size = colors.len() as i32;
                if size == 0 {
                    buf.fill(Rgb::BLACK);
                    return;
                }
                let n_i = n as i32;
                match p.opts[0] {
                    0 => {
                        for (i, px) in buf.iter_mut().enumerate() {
                            let pi = i as i32 - self.time;
                            *px = if (0..size).contains(&pi) { colors[pi as usize] } else { Rgb::BLACK };
                        }
                    }
                    1 => {
                        for (i, px) in buf.iter_mut().enumerate() {
                            let mut pi = (i as i32 % size) - self.time;
                            while pi < 0 {
                                pi += size;
                            }
                            *px = colors[(pi % size) as usize];
                        }
                    }
                    _ => {
                        for (i, px) in buf.iter_mut().enumerate() {
                            let mut x = i as i32 - self.time;
                            while x < 0 {
                                x += n_i;
                            }
                            let pi = ((x * size) / n_i).clamp(0, size - 1);
                            *px = colors[pi as usize];
                        }
                    }
                }
            }
            Kind::ColorChange => {
                let len = colors.len().max(1);
                let a = colors.get(self.idx % len).copied().unwrap_or(c0);
                let b = colors.get((self.idx + 1) % len).copied().unwrap_or(c0);
                let steps = p.opts[0].max(10) as i32;
                let t = (self.time * 255 / steps) as u8;
                buf.fill(a.blend(b, t));
            }
            Kind::Rainbow => {
                let spread = p.opts[0].max(1) as u32;
                for (i, px) in buf.iter_mut().enumerate() {
                    let hue = self.phase + (i as u32 * 256 * spread) / n as u32;
                    *px = Rgb::wheel(hue as u8);
                }
            }
            Kind::RainbowCycle => buf.fill(Rgb::wheel(self.phase as u8)),
            Kind::TheaterChase => {
                let gap = p.opts[0].max(2) as u32;
                let nc = colors.len().max(1) as u32;
                let t = self.phase;
                for (i, px) in buf.iter_mut().enumerate() {
                    let k = i as u32 + t;
                    *px = if k % gap == 0 { colors.get(((k / gap) % nc) as usize).copied().unwrap_or(c0) } else { Rgb::BLACK };
                }
            }
            Kind::Comet => {
                let tail = p.opts[0].max(2) as i32;
                let head = self.time;
                let dir = if p.opts[1] != 0 { self.dir } else { 1 };
                buf.fill(Rgb::BLACK);
                for k in 0..tail {
                    let pos = head - k * dir;
                    if (0..n as i32).contains(&pos) {
                        let level = (((tail - k) * (tail - k) * 256) / (tail * tail)) as u16;
                        buf[pos as usize] = c0.scale(level);
                    }
                }
            }
            Kind::Scanner => {
                let width = p.opts[0].max(1) as i32;
                for (i, px) in buf.iter_mut().enumerate() {
                    let d = (i as i32 - self.time).abs();
                    *px = if d > width {
                        Rgb::BLACK
                    } else {
                        let f = (width + 1 - d) as u32;
                        let w1 = (width + 1) as u32;
                        c0.scale(((f * f * 256) / (w1 * w1)) as u16)
                    };
                }
            }
            Kind::Fire => {
                for (i, px) in buf.iter_mut().enumerate() {
                    *px = heat_color(self.a[i]);
                }
            }
            Kind::Twinkle => {
                for (i, px) in buf.iter_mut().enumerate() {
                    let lvl = self.a[i];
                    *px = if lvl == 0 {
                        Rgb::BLACK
                    } else {
                        let c = colors.get((self.b[i] & 0x7f) as usize).copied().unwrap_or(c0);
                        c.scale(lvl as u16 + 1)
                    };
                }
            }
            Kind::Gradient => {
                let len = colors.len();
                if len < 2 {
                    buf.fill(c0);
                    return;
                }
                let moving = p.opts[0] != 0;
                for (i, px) in buf.iter_mut().enumerate() {
                    *px = if moving {
                        let span = 256 * len as u32;
                        let pos = ((i as u32 * span) / n as u32 + self.phase) % span;
                        let seg = (pos / 256) as usize;
                        colors[seg].blend(colors[(seg + 1) % len], (pos % 256) as u8)
                    } else {
                        let denom = (n - 1).max(1) as u32;
                        let pos = (i as u32 * (len as u32 - 1) * 256) / denom;
                        let seg = ((pos / 256) as usize).min(len - 2);
                        let t = (pos - seg as u32 * 256).min(255) as u8;
                        colors[seg].blend(colors[seg + 1], t)
                    };
                }
            }
            Kind::Plasma => {
                let scale = p.opts[0].max(1) as u32;
                let t = self.phase;
                for (i, px) in buf.iter_mut().enumerate() {
                    let x = i as u32 * scale * 4;
                    let v1 = sin8((x + t) as u8) as u32;
                    let v2 = sin8((x / 2 + t * 3 / 2) as u8) as u32;
                    let v3 = sin8((x * 3 / 2 + 255 - (t % 256)) as u8) as u32;
                    let hue = ((v1 + v2 + v3) / 3 + t / 4) as u8;
                    *px = Rgb::wheel(hue);
                }
            }
            Kind::Candle => {
                for (i, px) in buf.iter_mut().enumerate() {
                    *px = c0.scale(self.a[i] as u16 + 1);
                }
            }
            Kind::Sparkle => {
                let base = c0;
                let spark = colors.get(1).copied().unwrap_or(Rgb::WHITE);
                for (i, px) in buf.iter_mut().enumerate() {
                    *px = base.blend(spark, self.a[i]);
                }
            }
        }
    }
}

/// Black -> red -> yellow -> white heat palette.
fn heat_color(t: u8) -> Rgb {
    let t192 = ((t as u16 * 191) / 255) as u8;
    let ramp = (t192 & 0x3f) << 2;
    if t192 > 0x80 {
        Rgb::new(255, 255, ramp)
    } else if t192 > 0x40 {
        Rgb::new(255, ramp, 0)
    } else {
        Rgb::new(ramp, 0, 0)
    }
}
