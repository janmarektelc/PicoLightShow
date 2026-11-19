//! Host-side tests for the firmware logic that does not touch hardware.
//! Run with `cargo test -p host-tests`.

#[path = "../../firmware/src/color.rs"]
pub mod color;
#[path = "../../firmware/src/effects.rs"]
pub mod effects;
#[path = "../../firmware/src/transitions.rs"]
pub mod transitions;

pub mod config {
    pub const MAX_LEDS: usize = 1000;
}

#[cfg(test)]
mod tests {
    use crate::color::{Rgb, Rng, parse_hex};
    use crate::effects::{self, EFFECTS, EffectParams, EffectRunner};
    use crate::transitions::{Direction, TRANSITIONS, Transition};

    #[test]
    fn all_effects_all_options_never_panic() {
        let mut rng = Rng::new(42);
        let mut buf = vec![Rgb::BLACK; 1000];
        let mut runner = Box::new(EffectRunner::new());
        for (id, info) in EFFECTS.iter().enumerate() {
            for &n in &[1usize, 2, 3, 7, 30, 144, 1000] {
                // vary the option values over their whole range
                for variant in 0..6u8 {
                    let mut p = effects::default_params(id);
                    for (i, o) in info.opts.iter().enumerate() {
                        let span = (o.max - o.min) as u16;
                        p.opts[i] = o.min + ((span * variant as u16) / 5) as u8;
                    }
                    // vary color count between min and max
                    let want = (info.colors_min as usize + variant as usize).min(info.colors_max as usize);
                    while p.colors.len() < want { p.colors.push(Rgb::new(10, 20, 30)).unwrap(); }
                    p.colors.truncate(want.max(info.colors_min as usize));
                    effects::sanitize_params(id, &mut p);
                    runner.reset(id, &p);
                    for step in 0..(n * 3 + 50) {
                        runner.step(&p, n, &mut rng);
                        if step % 7 == 0 {
                            runner.render(&p, &mut buf[..n]);
                        }
                    }
                    runner.params_changed(&p);
                    runner.render(&p, &mut buf[..n]);
                }
            }
            println!("ok: {}", info.name);
        }
    }

    #[test]
    fn params_sanitize_fills_minimum_colors() {
        for (id, info) in EFFECTS.iter().enumerate() {
            let mut p = EffectParams::default();
            p.opts = [255; 4];
            effects::sanitize_params(id, &mut p);
            assert!(p.colors.len() >= info.colors_min as usize, "{}", info.name);
            assert!(p.colors.len() <= info.colors_max as usize, "{}", info.name);
            for (i, o) in info.opts.iter().enumerate() {
                assert!(p.opts[i] >= o.min && p.opts[i] <= o.max, "{} opt {}", info.name, o.label);
            }
        }
    }

    #[test]
    fn transitions_never_panic_and_finish() {
        let mut rng = Rng::new(7);
        let mut t = Box::new(Transition::new());
        for kind in 0..TRANSITIONS.len() as u8 {
            for &n in &[1usize, 2, 5, 60, 1000] {
                for dir in [Direction::In, Direction::Out] {
                    let mut buf = vec![Rgb::WHITE; n];
                    t.start(kind, dir, 500, n, &mut rng);
                    let mut frames = 0;
                    while !t.is_done() {
                        buf.fill(Rgb::WHITE);
                        t.apply(&mut buf, 20, &mut rng);
                        frames += 1;
                        assert!(frames < 100);
                    }
                    if dir == Direction::Out && kind != 7 {
                        buf.fill(Rgb::WHITE);
                        t.apply(&mut buf, 0, &mut rng);
                        assert!(buf.iter().all(|c| *c == Rgb::BLACK), "{} out leaves light on (n={})", TRANSITIONS[kind as usize], n);
                    }
                }
            }
        }
    }

    #[test]
    fn color_helpers() {
        assert_eq!(parse_hex("#ff8000"), Some(Rgb::new(255, 128, 0)));
        assert_eq!(parse_hex("00ff00"), Some(Rgb::new(0, 255, 0)));
        assert_eq!(parse_hex("#ff80"), None);
        assert_eq!(Rgb::BLACK.blend(Rgb::WHITE, 255), Rgb::WHITE);
        assert_eq!(Rgb::BLACK.blend(Rgb::WHITE, 0), Rgb::BLACK);
        for h in 0..=255u8 { let _ = Rgb::wheel(h); }
        assert_eq!(crate::color::GAMMA[0], 0);
        assert_eq!(crate::color::GAMMA[255], 255);
        assert!(crate::color::GAMMA.windows(2).all(|w| w[0] <= w[1]));
    }

    #[test]
    fn rgb_json_roundtrip() {
        let p = effects::default_params(2);
        let mut buf = [0u8; 512];
        let n = serde_json_core::to_slice(&p, &mut buf).unwrap();
        let s = core::str::from_utf8(&buf[..n]).unwrap();
        assert!(s.contains("\"#ff0000\""), "{s}");
        let (back, _): (EffectParams, _) = serde_json_core::from_slice(&buf[..n]).unwrap();
        assert_eq!(back, p);
    }

    #[test]
    fn speed_curve_matches_original() {
        assert_eq!(effects::step_interval_ms(255), 15);
        assert_eq!(effects::step_interval_ms(1), 100);
    }
}

#[cfg(test)]
mod json_compat {
    use serde::Deserialize;

    #[derive(Deserialize, Default, Debug, PartialEq)]
    #[serde(default)]
    struct V2 {
        a: u8,
        b: Option<heapless::String<8>>,
        c: [u8; 4],
    }

    /// Settings are stored as JSON: newer/older firmware must tolerate
    /// unknown and missing fields.
    #[test]
    fn unknown_and_missing_fields() {
        let (v, _): (V2, _) = serde_json_core::from_str(r#"{"a":5,"zzz":[1,{"x":"y"}],"c":[1,2,3,4]}"#).unwrap();
        assert_eq!(v, V2 { a: 5, b: None, c: [1, 2, 3, 4] });
        let (v, _): (V2, _) = serde_json_core::from_str(r#"{}"#).unwrap();
        assert_eq!(v, V2::default());
    }
}

/// Effect metadata as served by `GET /api/effects` (used by tools/mock_server.py).
pub fn effects_json() -> String {
    #[derive(serde::Serialize)]
    struct Meta {
        effects: &'static [effects::EffectInfo],
        transitions: &'static [&'static str],
    }
    let mut buf = vec![0u8; 16384];
    let n = serde_json_core::to_slice(&Meta { effects: &effects::EFFECTS, transitions: transitions::TRANSITIONS }, &mut buf).unwrap();
    String::from_utf8(buf[..n].to_vec()).unwrap()
}

#[cfg(test)]
mod dump {
    /// `cargo test -p host-tests dump_effects -- --ignored --nocapture > tools/effects.json`
    #[test]
    #[ignore]
    fn dump_effects() {
        println!("{}", crate::effects_json());
    }
}
