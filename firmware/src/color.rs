//! RGB color type and color math helpers.

use core::fmt;

use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// 8-bit RGB color. Serialized as a `"#rrggbb"` string (JSON and flash).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const BLACK: Rgb = Rgb::new(0, 0, 0);
    pub const WHITE: Rgb = Rgb::new(255, 255, 255);

    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub const fn hex(v: u32) -> Self {
        Self::new((v >> 16) as u8, (v >> 8) as u8, v as u8)
    }

    /// Scale by `f` / 256.
    #[inline]
    pub fn scale(self, f: u16) -> Self {
        Self::new(
            ((self.r as u16 * f) >> 8) as u8,
            ((self.g as u16 * f) >> 8) as u8,
            ((self.b as u16 * f) >> 8) as u8,
        )
    }

    /// Linear blend: `t` = 0 -> self, `t` = 255 -> other.
    #[inline]
    pub fn blend(self, other: Rgb, t: u8) -> Self {
        let t = t as i32;
        let mix = |a: u8, b: u8| (a as i32 + (((b as i32 - a as i32) * t) / 255)) as u8;
        Self::new(mix(self.r, other.r), mix(self.g, other.g), mix(self.b, other.b))
    }

    /// Hue (0..=255) to a fully saturated color.
    pub fn wheel(hue: u8) -> Self {
        Self::hsv(hue, 255, 255)
    }

    /// Integer HSV -> RGB, all channels 0..=255.
    pub fn hsv(h: u8, s: u8, v: u8) -> Self {
        if s == 0 {
            return Self::new(v, v, v);
        }
        let region = h / 43;
        let rem = (h - region * 43) as u16 * 6;
        let v16 = v as u16;
        let s16 = s as u16;
        let p = ((v16 * (255 - s16)) >> 8) as u8;
        let q = ((v16 * (255 - ((s16 * rem) >> 8))) >> 8) as u8;
        let t = ((v16 * (255 - ((s16 * (255 - rem)) >> 8))) >> 8) as u8;
        match region {
            0 => Self::new(v, t, p),
            1 => Self::new(q, v, p),
            2 => Self::new(p, v, t),
            3 => Self::new(p, q, v),
            4 => Self::new(t, p, v),
            _ => Self::new(v, p, q),
        }
    }
}

impl Serialize for Rgb {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let mut buf = [b'#'; 7];
        for (i, c) in [self.r, self.g, self.b].iter().enumerate() {
            buf[1 + i * 2] = HEX[(c >> 4) as usize];
            buf[2 + i * 2] = HEX[(c & 0xf) as usize];
        }
        // SAFETY: only ASCII bytes were written.
        s.serialize_str(unsafe { core::str::from_utf8_unchecked(&buf) })
    }
}

impl<'de> Deserialize<'de> for Rgb {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl Visitor<'_> for V {
            type Value = Rgb;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a color in #rrggbb format")
            }
            fn visit_str<E: de::Error>(self, v: &str) -> Result<Rgb, E> {
                parse_hex(v).ok_or_else(|| E::custom("bad color"))
            }
        }
        d.deserialize_str(V)
    }
}

/// Parse `#rrggbb` / `rrggbb`.
pub fn parse_hex(v: &str) -> Option<Rgb> {
    let v = v.strip_prefix('#').unwrap_or(v);
    if v.len() != 6 {
        return None;
    }
    let n = u32::from_str_radix(v, 16).ok()?;
    Some(Rgb::hex(n))
}

/// Gamma 2.2 lookup table.
pub static GAMMA: [u8; 256] = {
    let mut t = [0u8; 256];
    // Integer approximation of (i/255)^2.2 * 255 computed at compile time:
    // use x^2.2 ~= x^2 * x^0.2, x^0.2 approximated with a few Newton steps is
    // overkill here; a cubic/quadratic mix is visually indistinguishable.
    let mut i = 0;
    while i < 256 {
        let x = i as u32; // 0..255
        // 0.6 * x^2/255 + 0.4 * x^3/255^2
        let sq = x * x / 255;
        let cu = x * x * x / (255 * 255);
        let v = (sq * 6 + cu * 4) / 10;
        t[i] = if i > 0 && v == 0 { 1 } else { v as u8 };
        i += 1;
    }
    t[0] = 0;
    t
};

/// Simple xorshift PRNG; cheap and good enough for visual effects.
#[derive(Clone)]
pub struct Rng(u32);

impl Rng {
    pub const fn new(seed: u32) -> Self {
        Self(if seed == 0 { 0x1234_5678 } else { seed })
    }
    #[inline]
    pub fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        x
    }
    /// Uniform in `0..n` (n > 0).
    #[inline]
    pub fn below(&mut self, n: u32) -> u32 {
        if n == 0 { 0 } else { self.next_u32() % n }
    }
}
