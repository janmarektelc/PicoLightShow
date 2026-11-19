//! Power on / power off transitions (ported from the C++ firmware) plus a
//! cross-fade used when switching effects.

use crate::color::{Rgb, Rng};
use crate::config::MAX_LEDS;

pub static TRANSITIONS: &[&str] = &[
    "None",
    "Fade",
    "Wipe from start",
    "Wipe from end",
    "Wipe from center",
    "Wipe to center",
    "Sparkle",
    "Flicker",
];

pub const DEFAULT_TRANSITION: u8 = 1;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    In,
    Out,
}

pub struct Transition {
    kind: u8,
    dir: Direction,
    elapsed_ms: u32,
    duration_ms: u32,
    order: [u16; MAX_LEDS],
    order_len: usize,
}

impl Transition {
    pub const fn new() -> Self {
        Self { kind: 0, dir: Direction::In, elapsed_ms: 0, duration_ms: 0, order: [0; MAX_LEDS], order_len: 0 }
    }

    pub fn start(&mut self, kind: u8, dir: Direction, duration_ms: u32, n: usize, rng: &mut Rng) {
        self.kind = if (kind as usize) < TRANSITIONS.len() { kind } else { DEFAULT_TRANSITION };
        self.dir = dir;
        self.elapsed_ms = 0;
        self.duration_ms = duration_ms;
        if self.kind == 6 {
            // Sparkle: random permutation of LED indices (Fisher-Yates).
            let n = n.min(MAX_LEDS);
            for (i, o) in self.order[..n].iter_mut().enumerate() {
                *o = i as u16;
            }
            for i in (1..n).rev() {
                let j = rng.below(i as u32 + 1) as usize;
                self.order.swap(i, j);
            }
            self.order_len = n;
        }
    }

    pub fn direction(&self) -> Direction {
        self.dir
    }

    /// Visible part 0..=1000 (in = growing, out = shrinking).
    fn visible(&self) -> u32 {
        let p = if self.duration_ms == 0 { 1000 } else { (self.elapsed_ms * 1000 / self.duration_ms).min(1000) };
        match self.dir {
            Direction::In => p,
            Direction::Out => 1000 - p,
        }
    }

    pub fn is_done(&self) -> bool {
        self.kind == 0 || self.elapsed_ms >= self.duration_ms
    }

    /// Advance time and mask `buf` according to the transition progress.
    pub fn apply(&mut self, buf: &mut [Rgb], dt_ms: u32, rng: &mut Rng) {
        self.elapsed_ms = self.elapsed_ms.saturating_add(dt_ms);
        let n = buf.len();
        let vis = self.visible();
        match self.kind {
            0 => {
                if self.dir == Direction::Out {
                    buf.fill(Rgb::BLACK);
                }
            }
            1 => {
                let f = (vis * 256 / 1000) as u16;
                for px in buf.iter_mut() {
                    *px = px.scale(f);
                }
            }
            2..=5 => {
                let visible = n * vis as usize / 1000;
                let hidden = n - visible;
                let half = n / 2;
                match self.kind {
                    2 => buf[visible..].fill(Rgb::BLACK),
                    3 => buf[..hidden].fill(Rgb::BLACK),
                    4 => {
                        let hv = visible / 2;
                        let start = half.saturating_sub(hv);
                        let end = (half + hv).min(n);
                        if visible < n {
                            buf[..start].fill(Rgb::BLACK);
                            buf[end..].fill(Rgb::BLACK);
                        }
                    }
                    _ => {
                        let hh = hidden / 2;
                        let start = half.saturating_sub(hh);
                        let end = (half + hh + hidden % 2).min(n);
                        buf[start..end].fill(Rgb::BLACK);
                    }
                }
            }
            6 => {
                let len = self.order_len.min(n);
                let visible = len * vis as usize / 1000;
                for &i in &self.order[visible..len] {
                    if (i as usize) < n {
                        buf[i as usize] = Rgb::BLACK;
                    }
                }
            }
            _ => {
                // Flicker: random dark frames, less frequent as the light settles.
                let v = vis as u64;
                let threshold = (v * v / 1000) as u32; // 0..1000
                if rng.below(1000) >= threshold {
                    let f = if self.dir == Direction::In { 13 } else { 0 };
                    for px in buf.iter_mut() {
                        *px = px.scale(f);
                    }
                }
            }
        }
    }
}
