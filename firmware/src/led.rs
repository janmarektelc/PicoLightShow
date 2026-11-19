//! WS2812 output (PIO1 + DMA) and the render loop running on core 1.

use core::cell::RefCell;

use embassy_rp::clocks::clk_sys_freq;
use embassy_rp::dma;
use embassy_rp::peripherals::PIO1;
use embassy_rp::pio::{Common, Config, FifoJoin, LoadedProgram, Pin, ShiftConfig, ShiftDirection, StateMachine};
use embassy_sync::blocking_mutex::Mutex;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_time::{Duration, Instant, Ticker, Timer};
use fixed::types::U24F8;
use portable_atomic::{AtomicU32, Ordering};

use crate::color::{GAMMA, Rgb, Rng};
use crate::config::{self, ColorOrder, MAX_LEDS};
use crate::effects::{self, EffectParams, EffectRunner};
use crate::state::{self, LightState};
use crate::transitions::{Direction, Transition};

pub const FPS: u64 = 50;
const CROSSFADE_MS: u32 = 400;

/// Incremented by core 1 every frame; the watchdog task checks it.
pub static HEARTBEAT: AtomicU32 = AtomicU32::new(0);
/// Share of the frame time core 1 spends rendering, in 1/1000.
pub static RENDER_LOAD: AtomicU32 = AtomicU32::new(0);
/// Estimated current drawn by the strip (mA), after the power limit.
pub static CURRENT_MA: AtomicU32 = AtomicU32::new(0);

// ---------------------------------------------------------------------------
// WS2812 driver
// ---------------------------------------------------------------------------

const T1: u8 = 2; // start bit
const T2: u8 = 5; // data bit
const T3: u8 = 3; // stop bit
const CYCLES_PER_BIT: u32 = (T1 + T2 + T3) as u32;

pub fn load_program<'d>(common: &mut Common<'d, PIO1>) -> LoadedProgram<'d, PIO1> {
    let side_set = pio::SideSet::new(false, 1, false);
    let mut a: pio::Assembler<32> = pio::Assembler::new_with_side_set(side_set);
    let mut wrap_target = a.label();
    let mut wrap_source = a.label();
    let mut do_zero = a.label();
    a.set_with_side_set(pio::SetDestination::PINDIRS, 1, 0);
    a.bind(&mut wrap_target);
    a.out_with_delay_and_side_set(pio::OutDestination::X, 1, T3 - 1, 0);
    a.jmp_with_delay_and_side_set(pio::JmpCondition::XIsZero, &mut do_zero, T1 - 1, 1);
    a.jmp_with_delay_and_side_set(pio::JmpCondition::Always, &mut wrap_target, T2 - 1, 1);
    a.bind(&mut do_zero);
    a.nop_with_delay_and_side_set(T2 - 1, 0);
    a.bind(&mut wrap_source);
    let prg = a.assemble_with_wrap(wrap_source, wrap_target);
    common.load_program(&prg)
}

/// PIO + DMA driver. Kept small on purpose: it is moved to core 1, and a
/// large buffer passed by value there would overflow core 1's stack.
pub struct Ws2812 {
    sm: StateMachine<'static, PIO1, 0>,
    dma: dma::Channel<'static>,
    words: &'static mut [u32; MAX_LEDS],
}

impl Ws2812 {
    pub fn new(
        mut sm: StateMachine<'static, PIO1, 0>,
        dma: dma::Channel<'static>,
        pin: &Pin<'static, PIO1>,
        program: &LoadedProgram<'static, PIO1>,
        words: &'static mut [u32; MAX_LEDS],
    ) -> Self {
        let mut cfg = Config::default();
        cfg.set_out_pins(&[pin]);
        cfg.set_set_pins(&[pin]);
        cfg.use_program(program, &[pin]);
        let clock_freq = U24F8::from_num(clk_sys_freq() / 1000);
        let bit_freq = U24F8::from_num(800) * CYCLES_PER_BIT;
        cfg.clock_divider = clock_freq / bit_freq;
        cfg.fifo_join = FifoJoin::TxOnly;
        cfg.shift_out = ShiftConfig { auto_fill: true, threshold: 24, direction: ShiftDirection::Left };
        sm.set_config(&cfg);
        sm.set_enable(true);
        Self { sm, dma, words }
    }

    async fn flush(&mut self, n: usize) {
        if n == 0 {
            return;
        }
        self.sm.tx().dma_push(&mut self.dma, &self.words[..n], false).await;
        // Latch (> 50 us low).
        Timer::after_micros(80).await;
    }
}

#[inline]
fn pack(order: ColorOrder, c: Rgb) -> u32 {
    let (a, b, d) = match order {
        ColorOrder::Grb => (c.g, c.r, c.b),
        ColorOrder::Rgb => (c.r, c.g, c.b),
        ColorOrder::Brg => (c.b, c.r, c.g),
        ColorOrder::Rbg => (c.r, c.b, c.g),
        ColorOrder::Gbr => (c.g, c.b, c.r),
        ColorOrder::Bgr => (c.b, c.g, c.r),
    };
    ((a as u32) << 24) | ((b as u32) << 16) | ((d as u32) << 8)
}

// ---------------------------------------------------------------------------
// Live preview for the web UI
// ---------------------------------------------------------------------------

pub const PREVIEW_MAX: usize = 240;

pub struct Preview {
    pub seq: u32,
    pub len: usize,
    pub data: [u8; PREVIEW_MAX * 3],
}

pub static PREVIEW: Mutex<CriticalSectionRawMutex, RefCell<Preview>> =
    Mutex::new(RefCell::new(Preview { seq: 0, len: 0, data: [0; PREVIEW_MAX * 3] }));

fn update_preview(frame: &[Rgb]) {
    let n = frame.len();
    let m = n.min(PREVIEW_MAX);
    PREVIEW.lock(|p| {
        let mut p = p.borrow_mut();
        for j in 0..m {
            let c = frame[j * n / m.max(1)];
            p.data[j * 3] = c.r;
            p.data[j * 3 + 1] = c.g;
            p.data[j * 3 + 2] = c.b;
        }
        p.len = m;
        p.seq = p.seq.wrapping_add(1);
    });
}

// ---------------------------------------------------------------------------
// Render loop
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Off,
    TurningOn,
    On,
    TurningOff,
}

pub struct Renderer {
    runner: EffectRunner,
    transition: Transition,
    frame: [Rgb; MAX_LEDS],
    prev: [Rgb; MAX_LEDS],
    state: LightState,
}

impl Renderer {
    pub const fn new() -> Self {
        Self {
            runner: EffectRunner::new(),
            transition: Transition::new(),
            frame: [Rgb::BLACK; MAX_LEDS],
            prev: [Rgb::BLACK; MAX_LEDS],
            state: LightState::INIT,
        }
    }
}

fn params_of(s: &LightState) -> &EffectParams {
    s.current()
}

#[embassy_executor::task]
pub async fn render_task(mut ws: Ws2812, r: &'static mut Renderer, seed: u32) {
    let mut rng = Rng::new(seed);
    let mut phase = Phase::Off;
    let mut seen_version: u32;
    let mut last = Instant::now();
    let mut step_acc: u32 = 0;
    let mut xfade_ms: u32 = CROSSFADE_MS; // >= CROSSFADE_MS = no crossfade
    let mut last_n: usize = 0;
    let mut frame_no: u32 = 0;
    let mut busy_us: u64 = 0;
    let mut ticker = Ticker::every(Duration::from_hz(FPS));

    r.state = state::snapshot();
    r.runner.reset(r.state.effect as usize, params_of(&r.state));
    if r.state.on {
        let n = (config::led_settings().count as usize).clamp(1, MAX_LEDS);
        r.transition.start(r.state.transition, Direction::In, r.state.transition_ms as u32, n, &mut rng);
        phase = Phase::TurningOn;
    }
    seen_version = state::version();

    loop {
        ticker.next().await;
        HEARTBEAT.fetch_add(1, Ordering::Relaxed);
        frame_no = frame_no.wrapping_add(1);

        let now = Instant::now();
        let dt = (now - last).as_millis().min(500) as u32;
        last = now;

        let led = config::led_settings();
        let n = (led.count as usize).clamp(1, MAX_LEDS);

        // --- pick up state changes ---
        let v = state::version();
        if v != seen_version {
            seen_version = v;
            let new = state::snapshot();
            if new.effect != r.state.effect {
                if phase != Phase::Off {
                    r.prev[..n].copy_from_slice(&r.frame[..n]);
                    xfade_ms = 0;
                }
                r.runner.reset(new.effect as usize, params_of(&new));
                step_acc = 0;
            } else if params_of(&new) != params_of(&r.state) {
                r.runner.params_changed(params_of(&new));
            }
            if new.on && !r.state.on {
                if phase == Phase::Off {
                    r.runner.reset(new.effect as usize, params_of(&new));
                }
                r.transition.start(new.transition, Direction::In, new.transition_ms as u32, n, &mut rng);
                phase = Phase::TurningOn;
            } else if !new.on && r.state.on {
                r.transition.start(new.transition, Direction::Out, new.transition_ms as u32, n, &mut rng);
                phase = Phase::TurningOff;
            }
            r.state = new;
        }

        let ddp = crate::ddp::is_active();
        let frame = &mut r.frame[..n];

        if ddp {
            crate::ddp::copy_frame(frame);
        } else if phase == Phase::Off {
            frame.fill(Rgb::BLACK);
        } else {
            // advance animation
            if r.state.running {
                step_acc += dt;
                let interval = effects::step_interval_ms(r.state.speed);
                if step_acc > 2000 {
                    step_acc = interval;
                }
                while step_acc >= interval {
                    r.runner.step(params_of(&r.state), n, &mut rng);
                    step_acc -= interval;
                }
            }
            r.runner.render(params_of(&r.state), frame);

            if xfade_ms < CROSSFADE_MS {
                xfade_ms += dt;
                let t = (xfade_ms.min(CROSSFADE_MS) * 255 / CROSSFADE_MS) as u8;
                for (px, old) in frame.iter_mut().zip(r.prev[..n].iter()) {
                    *px = old.blend(*px, t);
                }
            }

            if matches!(phase, Phase::TurningOn | Phase::TurningOff) {
                r.transition.apply(frame, dt, &mut rng);
                if r.transition.is_done() {
                    phase = if r.transition.direction() == Direction::In { Phase::On } else { Phase::Off };
                    if phase == Phase::Off {
                        frame.fill(Rgb::BLACK);
                    }
                }
            }
        }

        if frame_no % 5 == 0 {
            update_preview(frame);
        }

        // Nothing to show: refresh black only once per second.
        if frame_no % (FPS as u32) == 0 {
            // busy time of the last second in 1/1000 of a second
            RENDER_LOAD.store((busy_us / 1000) as u32, Ordering::Relaxed);
            busy_us = 0;
        }
        if phase == Phase::Off && !ddp && last_n == n && frame_no % (FPS as u32) != 0 {
            CURRENT_MA.store(n as u32, Ordering::Relaxed);
            busy_us += (Instant::now() - now).as_micros();
            continue;
        }

        // --- post processing: gamma, brightness, power limit, color order ---
        let bri = r.state.brightness as u32 + 1;
        let mut sum: u32 = 0;
        for (w, px) in ws.words.iter_mut().zip(frame.iter()) {
            let mut c = *px;
            if led.gamma && !ddp {
                c = Rgb::new(GAMMA[c.r as usize], GAMMA[c.g as usize], GAMMA[c.b as usize]);
            }
            let c = Rgb::new(
                ((c.r as u32 * bri) >> 8) as u8,
                ((c.g as u32 * bri) >> 8) as u8,
                ((c.b as u32 * bri) >> 8) as u8,
            );
            sum += c.r as u32 + c.g as u32 + c.b as u32;
            *w = pack(led.color_order, c);
        }
        // ~20 mA per channel at full scale, ~1 mA idle per LED.
        let idle = n as u32;
        let est = idle + sum * 20 / 255;
        CURRENT_MA.store(
            if led.max_current_ma > 0 { est.min((led.max_current_ma as u32).max(idle)) } else { est },
            Ordering::Relaxed,
        );
        if led.max_current_ma > 0 {
            let limit = led.max_current_ma as u32;
            if est > limit && est > idle {
                let f = ((limit.saturating_sub(idle)) * 256 / (est - idle)).min(256);
                for w in ws.words[..n].iter_mut() {
                    let x = *w;
                    let a = (((x >> 24) & 0xff) * f) >> 8;
                    let b = (((x >> 16) & 0xff) * f) >> 8;
                    let d = (((x >> 8) & 0xff) * f) >> 8;
                    *w = (a << 24) | (b << 16) | (d << 8);
                }
            }
        }

        // If the strip got shorter, blank the LEDs that are no longer used.
        let out_n = n.max(last_n).min(MAX_LEDS);
        for w in ws.words[n..out_n].iter_mut() {
            *w = 0;
        }
        last_n = n;
        busy_us += (Instant::now() - now).as_micros();
        ws.flush(out_n).await;
    }
}
