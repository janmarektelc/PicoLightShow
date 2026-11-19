//! HTTP server: static web UI, JSON REST API, WebSocket (live state +
//! preview) and firmware upload (OTA).

use core::cell::RefCell;

use embassy_executor::Spawner;
use embassy_futures::select::{Either as SelEither, select};
use embassy_net::Stack;
use embassy_sync::blocking_mutex::Mutex as BlockingMutex;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::once_lock::OnceLock;
use embassy_time::{Duration, Instant, Timer};
use heapless::{String, Vec};
use picoserve::extract::FromRequestParts;
use picoserve::futures::Either;
use picoserve::io::Read;
use picoserve::request::{Request, RequestParts};
use picoserve::response::fs::{Directory, File};
use picoserve::response::ws::{self, Message, WebSocketCallback, WebSocketUpgrade};
use picoserve::response::{Connection, IntoResponse, Json, Redirect, Response, ResponseWriter, StatusCode};
use picoserve::routing::{PathRouter, RequestHandlerService, get, get_service, post, post_service};
use picoserve::{ResponseSent, Router};
use serde::{Deserialize, Serialize};
use static_cell::StaticCell;

use crate::config::{self, BootPower, ColorOrder, DeviceConfig, WifiMode};
use crate::effects::{EFFECTS, EffectInfo};
use crate::state::{self, StateUpdate};
use crate::storage::{self, FlashMutex};
use crate::transitions::TRANSITIONS;
use crate::{led, mqtt, net, system};

pub const WORKERS: usize = 5;

mod assets {
    include!(concat!(env!("OUT_DIR"), "/assets.rs"));
}

static FLASH: OnceLock<&'static FlashMutex> = OnceLock::new();
static FILES: StaticCell<Vec<(&'static str, File), 12>> = StaticCell::new();
static FILE_LIST: OnceLock<&'static [(&'static str, File)]> = OnceLock::new();
static LAST_PANIC: BlockingMutex<CriticalSectionRawMutex, RefCell<Option<String<200>>>> =
    BlockingMutex::new(RefCell::new(None));

pub fn set_last_panic(msg: String<200>) {
    LAST_PANIC.lock(|p| *p.borrow_mut() = Some(msg));
}

static SERVER_CONFIG: picoserve::Config = picoserve::Config::new(picoserve::Timeouts {
    start_read_request: Duration::from_secs(5),
    persistent_start_read_request: Duration::from_secs(2),
    read_request: Duration::from_secs(5),
    write: Duration::from_secs(5),
})
.keep_connection_alive();

const GZIP_HEADERS: &[(&str, &str)] = &[("Content-Encoding", "gzip"), ("Cache-Control", "no-cache")];

pub fn start(spawner: Spawner, stack: Stack<'static>, flash: &'static FlashMutex) {
    let _ = FLASH.init(flash);
    let files = FILES.init(Vec::new());
    for a in assets::ASSETS {
        let file = File::with_content_type_and_headers(a.mime, a.gz, GZIP_HEADERS);
        if a.path == "index.html" {
            let _ = files.push(("", file.clone()));
        }
        let _ = files.push((a.path, file));
    }
    let files: &'static Vec<(&'static str, File), 12> = files;
    let _ = FILE_LIST.init(&files[..]);
    for id in 0..WORKERS {
        spawner.spawn(web_task(id, stack).unwrap());
    }
}

#[embassy_executor::task(pool_size = WORKERS)]
async fn web_task(id: usize, stack: Stack<'static>) -> ! {
    let mut rx = [0u8; 1536];
    let mut tx = [0u8; 2048];
    let mut http = [0u8; 2048];
    let app = make_app();
    picoserve::Server::new(&app, &SERVER_CONFIG, &mut http)
        .listen_and_serve(id, stack, 80, &mut rx, &mut tx)
        .await
        .into_never()
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct ApiMsg {
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<&'static str>,
    #[serde(skip_serializing_if = "core::ops::Not::not")]
    reboot_required: bool,
}

fn ok() -> Json<ApiMsg> {
    Json(ApiMsg { ok: true, message: None, reboot_required: false })
}

fn to_json<T: Serialize>(v: &T, buf: &mut [u8]) -> Option<usize> {
    serde_json_core::to_slice(v, buf).ok()
}

fn msg_json(ok: bool, message: Option<&'static str>, reboot_required: bool, buf: &mut [u8]) -> Option<usize> {
    to_json(&ApiMsg { ok, message, reboot_required }, buf)
}

// Response buffers. Handlers must not keep large values in their futures:
// picoserve moves futures through many generic layers and every copy ends up
// on the (shared) stack of core 0. JSON is therefore serialized synchronously
// into one of these static buffers and the response only holds a guard.
const RESP_BUF: usize = 2048;
static RESP_POOL: [embassy_sync::mutex::Mutex<CriticalSectionRawMutex, [u8; RESP_BUF]>; WORKERS] =
    [const { embassy_sync::mutex::Mutex::new([0; RESP_BUF]) }; WORKERS];

type RespGuard = embassy_sync::mutex::MutexGuard<'static, CriticalSectionRawMutex, [u8; RESP_BUF]>;

async fn resp_buf() -> RespGuard {
    loop {
        for m in &RESP_POOL {
            if let Ok(g) = m.try_lock() {
                return g;
            }
        }
        Timer::after_millis(5).await;
    }
}

/// JSON response serialized into a pooled static buffer.
pub struct JsonOut {
    status: StatusCode,
    buf: RespGuard,
    len: usize,
}

impl JsonOut {
    async fn with(status: StatusCode, fill: impl FnOnce(&mut [u8]) -> Option<usize>) -> Self {
        let mut buf = resp_buf().await;
        let len = match fill(&mut buf[..]) {
            Some(n) => n,
            None => {
                status_too_large(&mut buf[..])
            }
        };
        Self { status, buf, len }
    }

    async fn ok(fill: impl FnOnce(&mut [u8]) -> Option<usize>) -> Self {
        Self::with(StatusCode::OK, fill).await
    }

    async fn error(status: StatusCode, message: &'static str) -> Self {
        Self::with(status, |b| msg_json(false, Some(message), false, b)).await
    }
}

fn status_too_large(buf: &mut [u8]) -> usize {
    msg_json(false, Some("response too large"), false, buf).unwrap_or(0)
}

impl IntoResponse for JsonOut {
    async fn write_to<R: Read, W: ResponseWriter<Error = R::Error>>(
        self,
        connection: Connection<'_, R>,
        response_writer: W,
    ) -> Result<ResponseSent, W::Error> {
        Response::new(self.status, &self.buf[..self.len])
            .with_content_type("application/json")
            .write_to(connection, response_writer)
            .await
    }
}

// ---------------------------------------------------------------------------
// Authentication (HTTP basic auth, any user name, password = admin_pass)
// ---------------------------------------------------------------------------

pub struct Auth;

pub struct Unauthorized;

impl IntoResponse for Unauthorized {
    async fn write_to<R: Read, W: ResponseWriter<Error = R::Error>>(
        self,
        connection: Connection<'_, R>,
        response_writer: W,
    ) -> Result<ResponseSent, W::Error> {
        (StatusCode::UNAUTHORIZED, ("WWW-Authenticate", "Basic realm=\"Pico Light Show\""), "Unauthorized\n")
            .write_to(connection, response_writer)
            .await
    }
}

fn b64_val(c: u8) -> Option<u8> {
    match c {
        b'A'..=b'Z' => Some(c - b'A'),
        b'a'..=b'z' => Some(c - b'a' + 26),
        b'0'..=b'9' => Some(c - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}

fn b64_decode(input: &str, out: &mut Vec<u8, 128>) -> Option<()> {
    let mut acc: u32 = 0;
    let mut bits = 0;
    for c in input.bytes() {
        if c == b'=' {
            break;
        }
        acc = (acc << 6) | b64_val(c)? as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8).ok()?;
        }
    }
    Some(())
}

/// Constant-time comparison.
fn secure_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

fn check_auth(parts: &RequestParts<'_>) -> bool {
    let pass = config::with(|c| c.admin_pass.clone());
    if pass.is_empty() {
        return true;
    }
    let Some(header) = parts.headers().get("Authorization") else { return false };
    let Ok(header) = header.as_str() else { return false };
    let Some(encoded) = header.strip_prefix("Basic ") else { return false };
    let mut decoded = Vec::new();
    if b64_decode(encoded.trim(), &mut decoded).is_none() {
        return false;
    }
    let Some(colon) = decoded.iter().position(|&b| b == b':') else { return false };
    secure_eq(&decoded[colon + 1..], pass.as_bytes())
}

impl<'r, S> FromRequestParts<'r, S> for Auth {
    type Rejection = Unauthorized;

    async fn from_request_parts(_state: &'r S, parts: &RequestParts<'r>) -> Result<Self, Self::Rejection> {
        if check_auth(parts) { Ok(Auth) } else { Err(Unauthorized) }
    }
}

// ---------------------------------------------------------------------------
// Router
// ---------------------------------------------------------------------------

fn make_app() -> Router<impl PathRouter> {
    let dir = Directory { files: FILE_LIST.try_get().copied().unwrap_or(&[]), sub_directories: &[] };
    Router::from_service(dir)
        // captive portal probes (AP mode)
        .route("/generate_204", get(async || Redirect::to("/")))
        .route("/hotspot-detect.html", get(async || Redirect::to("/")))
        .route("/connecttest.txt", get(async || Redirect::to("/")))
        .route("/ncsi.txt", get(async || Redirect::to("/")))
        // light state
        .route(
            "/api/state",
            get(async || {
                net::touch_http();
                Json(state::view())
            })
            .post(async |Json(update): Json<StateUpdate>| {
                net::touch_http();
                state::apply(&update);
                Json(state::view())
            }),
        )
        .route("/api/effects", get(async || Json(EffectsMeta { effects: &EFFECTS, transitions: TRANSITIONS })))
        .route(
            "/api/info",
            get(async || {
                net::touch_http();
                JsonOut::ok(|b| to_json(&info_view(), b)).await
            }),
        )
        .route(
            "/api/presets",
            get(async || JsonOut::ok(presets_json).await).post_service(BodyService { auth: false, kind: BodyKind::Preset }),
        )
        .route(
            "/api/presets/delete",
            post(async |Json(req): Json<PresetIndex>| {
                state::delete_preset(req.index as usize);
                storage::save_presets().await;
                JsonOut::ok(presets_json).await
            }),
        )
        // settings (protected)
        .route(
            "/api/config",
            get(async |_a: Auth| JsonOut::ok(|b| config::with(|c| to_json(&ConfigView::from(c.clone()), b))).await)
                .post_service(BodyService { auth: true, kind: BodyKind::Config }),
        )
        .route(
            "/api/wifi/scan",
            get(async |_a: Auth| {
                net::scan().await;
                JsonOut::ok(|b| net::SCAN_RESULT.lock(|r| to_json(&*r.borrow(), b))).await
            }),
        )
        .route(
            "/api/reboot",
            post(async |_a: Auth| {
                system::request_reboot();
                ok()
            }),
        )
        .route(
            "/api/factory-reset",
            post(async |_a: Auth| {
                storage::factory_reset().await;
                system::request_reboot();
                ok()
            }),
        )
        .route("/api/ota", post_service(OtaService))
        .route("/ws", get(async |upgrade: WebSocketUpgrade| upgrade.on_upgrade(WsSession)))
        .route("/favicon.ico", get_service(File::with_content_type("image/x-icon", &[])))
}

fn needs_reboot(a: &DeviceConfig, b: &DeviceConfig) -> bool {
    a.name != b.name
        || a.wifi_mode != b.wifi_mode
        || a.wifi_ssid != b.wifi_ssid
        || a.wifi_pass != b.wifi_pass
        || a.dhcp != b.dhcp
        || a.ip != b.ip
        || a.netmask != b.netmask
        || a.gateway != b.gateway
        || a.dns != b.dns
        || a.ap_ssid != b.ap_ssid
        || a.ap_pass != b.ap_pass
        || a.led_pin != b.led_pin
        || a.mqtt_enabled != b.mqtt_enabled
        || a.mqtt_host != b.mqtt_host
        || a.mqtt_port != b.mqtt_port
        || a.mqtt_user != b.mqtt_user
        || a.mqtt_pass != b.mqtt_pass
        || a.mqtt_discovery_prefix != b.mqtt_discovery_prefix
        || a.mqtt_topic != b.mqtt_topic
        || a.ddp_enabled != b.ddp_enabled
}

// ---------------------------------------------------------------------------
// DTOs
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct EffectsMeta {
    effects: &'static [EffectInfo],
    transitions: &'static [&'static str],
}

/// Save a preset: only `name` = store the current look, otherwise a full
/// preset (used when restoring a backup).
#[derive(Deserialize)]
struct PresetSave {
    name: String<24>,
    effect: Option<u8>,
    #[serde(default)]
    brightness: Option<u8>,
    #[serde(default)]
    speed: Option<u8>,
    #[serde(default)]
    params: Option<crate::effects::EffectParams>,
}

#[derive(Deserialize)]
struct PresetIndex {
    index: u8,
}

#[derive(Serialize)]
struct InfoView {
    name: String<32>,
    version: &'static str,
    uptime_s: u64,
    net: net::NetInfo,
    hostname: String<40>,
    mqtt: &'static str,
    ddp_active: bool,
    boot: &'static str,
    last_panic: Option<String<200>>,
    led_count: u16,
    auth: bool,
    fps: u64,
    cpu_mhz: u32,
    chip_temp_c: Option<f32>,
    reset_reason: &'static str,
    ram_static: u32,
    ram_total: u32,
    stack_free: [u32; 2],
    stack_size: [u32; 2],
    flash_used: u32,
    flash_total: u32,
    render_load: u32,
    current_ma: u32,
    ws_clients: u8,
}

fn info_view() -> InfoView {
    use portable_atomic::Ordering::Relaxed;
    let cfg = config::get();
    let (ram_static, ram_total) = system::ram_static();
    let (flash_used, flash_total) = system::flash_usage();
    InfoView {
        hostname: net::hostname(),
        name: cfg.display_name(),
        version: system::VERSION,
        uptime_s: Instant::now().as_secs(),
        net: net::info(),
        mqtt: mqtt::status_str(),
        ddp_active: crate::ddp::is_active(),
        boot: match system::BOOT_KIND.load(portable_atomic::Ordering::Relaxed) {
            1 => "updated",
            2 => "rolled_back",
            _ => "normal",
        },
        last_panic: LAST_PANIC.lock(|p| p.borrow().clone()),
        led_count: cfg.led_count,
        auth: !cfg.admin_pass.is_empty(),
        fps: led::FPS,
        cpu_mhz: embassy_rp::clocks::clk_sys_freq() / 1_000_000,
        chip_temp_c: match system::CHIP_TEMP_CENTI.load(Relaxed) {
            i32::MIN => None,
            t => Some(t as f32 / 100.0),
        },
        reset_reason: match system::RESET_REASON.load(Relaxed) {
            1 => "watchdog reset",
            2 => "watchdog timeout (hang)",
            3 => "reboot",
            4 => "crash",
            _ => "power-on",
        },
        ram_static,
        ram_total,
        stack_free: [system::STACK_FREE[0].load(Relaxed), system::STACK_FREE[1].load(Relaxed)],
        stack_size: [system::STACK_SIZE[0].load(Relaxed), system::STACK_SIZE[1].load(Relaxed)],
        flash_used,
        flash_total,
        render_load: led::RENDER_LOAD.load(Relaxed),
        current_ma: led::CURRENT_MA.load(Relaxed),
        ws_clients: system::WS_CLIENTS.load(Relaxed),
    }
}

fn presets_json(buf: &mut [u8]) -> Option<usize> {
    state::PRESETS.lock(|p| to_json(&*p.borrow(), buf))
}

/// Parse and store a preset (synchronous: nothing large lives in a future).
fn save_preset_body(body: &[u8]) -> Result<(), &'static str> {
    let mut unescape = [0u8; 64];
    let (req, _): (PresetSave, _) =
        serde_json_core::from_slice_escaped(body, &mut unescape).map_err(|_| "invalid preset JSON")?;
    if req.name.is_empty() {
        return Err("name required");
    }
    let full = req.effect.map(|effect| state::Preset {
        name: String::new(),
        effect,
        brightness: req.brightness.unwrap_or(128),
        speed: req.speed.unwrap_or(128),
        params: req.params.clone().unwrap_or_default(),
    });
    if state::save_preset(&req.name, full) { Ok(()) } else { Err("too many presets (max 8)") }
}

/// Parse a configuration update, merge it into the active configuration and
/// activate it. Returns whether a reboot is needed for it to take effect.
fn apply_config_body(body: &[u8]) -> Result<bool, &'static str> {
    let mut unescape = [0u8; 256];
    let (upd, _): (ConfigUpdate, _) =
        serde_json_core::from_slice_escaped(body, &mut unescape).map_err(|_| "invalid settings JSON")?;
    let old = config::get();
    let mut cfg = old.clone();
    upd.apply(&mut cfg);
    cfg.sanitize();
    let reboot_required = needs_reboot(&old, &cfg);
    config::set(cfg);
    Ok(reboot_required)
}

/// Configuration as sent to the UI; secrets are replaced by `*_set` flags.
#[derive(Serialize)]
struct ConfigView {
    name: String<32>,
    /// Names used when `name` / `ap_ssid` are left empty.
    default_name: String<32>,
    default_ap_ssid: String<32>,
    wifi_mode: WifiMode,
    wifi_ssid: String<32>,
    wifi_pass_set: bool,
    dhcp: bool,
    ip: [u8; 4],
    netmask: [u8; 4],
    gateway: [u8; 4],
    dns: [u8; 4],
    ap_ssid: String<32>,
    ap_pass_set: bool,
    led_count: u16,
    led_pin: u8,
    color_order: ColorOrder,
    max_current_ma: u16,
    gamma: bool,
    boot_power: BootPower,
    mqtt_enabled: bool,
    mqtt_host: String<64>,
    mqtt_port: u16,
    mqtt_user: String<32>,
    mqtt_pass_set: bool,
    mqtt_discovery_prefix: String<32>,
    mqtt_topic: String<48>,
    ddp_enabled: bool,
    admin_pass_set: bool,
}

impl From<DeviceConfig> for ConfigView {
    fn from(c: DeviceConfig) -> Self {
        ConfigView {
                default_name: config::default_hostname(),
                default_ap_ssid: config::default_ap_ssid(),
                wifi_pass_set: !c.wifi_pass.is_empty(),
                ap_pass_set: !c.ap_pass.is_empty(),
                mqtt_pass_set: !c.mqtt_pass.is_empty(),
                admin_pass_set: !c.admin_pass.is_empty(),
                name: c.name,
                wifi_mode: c.wifi_mode,
                wifi_ssid: c.wifi_ssid,
                dhcp: c.dhcp,
                ip: c.ip,
                netmask: c.netmask,
                gateway: c.gateway,
                dns: c.dns,
                ap_ssid: c.ap_ssid,
                led_count: c.led_count,
                led_pin: c.led_pin,
                color_order: c.color_order,
                max_current_ma: c.max_current_ma,
                gamma: c.gamma,
                boot_power: c.boot_power,
                mqtt_enabled: c.mqtt_enabled,
                mqtt_host: c.mqtt_host,
                mqtt_port: c.mqtt_port,
                mqtt_user: c.mqtt_user,
                mqtt_discovery_prefix: c.mqtt_discovery_prefix,
                mqtt_topic: c.mqtt_topic,
                ddp_enabled: c.ddp_enabled,
        }
    }
}

/// Partial configuration update; missing fields keep their value.
/// Secrets: field missing = unchanged, empty string = cleared.
#[derive(Deserialize, Default)]
#[serde(default)]
struct ConfigUpdate {
    name: Option<String<32>>,
    wifi_mode: Option<WifiMode>,
    wifi_ssid: Option<String<32>>,
    wifi_pass: Option<String<64>>,
    dhcp: Option<bool>,
    ip: Option<[u8; 4]>,
    netmask: Option<[u8; 4]>,
    gateway: Option<[u8; 4]>,
    dns: Option<[u8; 4]>,
    ap_ssid: Option<String<32>>,
    ap_pass: Option<String<64>>,
    led_count: Option<u16>,
    led_pin: Option<u8>,
    color_order: Option<ColorOrder>,
    max_current_ma: Option<u16>,
    gamma: Option<bool>,
    boot_power: Option<BootPower>,
    mqtt_enabled: Option<bool>,
    mqtt_host: Option<String<64>>,
    mqtt_port: Option<u16>,
    mqtt_user: Option<String<32>>,
    mqtt_pass: Option<String<64>>,
    mqtt_discovery_prefix: Option<String<32>>,
    mqtt_topic: Option<String<48>>,
    ddp_enabled: Option<bool>,
    admin_pass: Option<String<32>>,
}

impl ConfigUpdate {
    fn apply(self, c: &mut DeviceConfig) {
        macro_rules! set {
            ($($f:ident),*) => { $( if let Some(v) = self.$f { c.$f = v; } )* };
        }
        set!(
            name, wifi_mode, wifi_ssid, wifi_pass, dhcp, ip, netmask, gateway, dns, ap_ssid, ap_pass, led_count,
            led_pin, color_order, max_current_ma, gamma, boot_power, mqtt_enabled, mqtt_host, mqtt_port, mqtt_user,
            mqtt_pass, mqtt_discovery_prefix, mqtt_topic, ddp_enabled, admin_pass
        );
    }
}

// ---------------------------------------------------------------------------
// WebSocket: pushes state changes and (on request) a live LED preview.
// Accepts the same JSON as POST /api/state, plus "preview:on"/"preview:off".
// ---------------------------------------------------------------------------

struct WsSession;

impl WebSocketCallback for WsSession {
    async fn run<R: Read, W: picoserve::io::Write<Error = R::Error>>(
        self,
        mut rx: ws::SocketRx<R>,
        mut tx: ws::SocketTx<W>,
    ) -> Result<(), W::Error> {
        let _client = system::WsClientGuard::new();
        let mut watch = state::CHANGED.receiver();
        let mut buffer = [0u8; 256];
        let mut out = [0u8; 2 + led::PREVIEW_MAX * 3];
        let mut preview = false;
        let mut last_seq = 0u32;
        let mut seen = 0u32;

        let n = state::state_json(&mut out);
        tx.send_text(core::str::from_utf8(&out[..n]).unwrap_or("{}")).await?;

        loop {
            let tick = Duration::from_millis(if preview { 100 } else { 1000 });
            let signal = async {
                match watch.as_mut() {
                    Some(w) => match select(w.changed(), Timer::after(tick)).await {
                        SelEither::First(_) => true,
                        SelEither::Second(_) => false,
                    },
                    None => {
                        Timer::after(tick).await;
                        state::version() != seen
                    }
                }
            };
            match rx.next_message(&mut buffer, signal).await? {
                Either::First(Ok(Message::Text(text))) => {
                    net::touch_http();
                    match text {
                        "preview:on" => preview = true,
                        "preview:off" => preview = false,
                        _ => {
                            if let Ok((u, _)) = serde_json_core::from_str::<StateUpdate>(text) {
                                state::apply(&u);
                            }
                        }
                    }
                }
                Either::First(Ok(Message::Binary(_))) => {}
                Either::First(Ok(Message::Ping(data))) => tx.send_pong(data).await?,
                Either::First(Ok(Message::Pong(_))) => {}
                Either::First(Ok(Message::Close(_))) => break,
                Either::First(Err(_)) => break,
                Either::Second(changed) => {
                    if changed || state::version() != seen {
                        seen = state::version();
                        let n = state::state_json(&mut out);
                        tx.send_text(core::str::from_utf8(&out[..n]).unwrap_or("{}")).await?;
                    }
                    if preview {
                        let frame = &mut out;
                        let len = led::PREVIEW.lock(|p| {
                            let p = p.borrow();
                            if p.seq == last_seq {
                                return 0;
                            }
                            last_seq = p.seq;
                            frame[0..2].copy_from_slice(&(p.len as u16).to_be_bytes());
                            frame[2..2 + p.len * 3].copy_from_slice(&p.data[..p.len * 3]);
                            2 + p.len * 3
                        });
                        if len > 0 {
                            tx.send_binary(&frame[..len]).await?;
                        }
                    }
                }
            }
        }
        tx.close(None).await
    }
}

// ---------------------------------------------------------------------------
// POST handlers that parse their body synchronously (no large values in futures)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
enum BodyKind {
    Preset,
    Config,
}

struct BodyService {
    auth: bool,
    kind: BodyKind,
}

impl RequestHandlerService<()> for BodyService {
    async fn call_request_handler_service<R: Read, W: ResponseWriter<Error = R::Error>>(
        &self,
        _state: &(),
        _params: (),
        mut request: Request<'_, R>,
        response_writer: W,
    ) -> Result<ResponseSent, W::Error> {
        if self.auth && !check_auth(&request.parts) {
            return Unauthorized.write_to(request.body_connection.finalize().await?, response_writer).await;
        }
        net::touch_http();
        let result = match request.body_connection.body().read_all().await {
            Ok(body) => match self.kind {
                BodyKind::Preset => save_preset_body(body).map(|()| false),
                BodyKind::Config => apply_config_body(body),
            },
            Err(_) => Err("request body too large or incomplete"),
        };
        let response = match result {
            Ok(reboot_required) => {
                let ok = match self.kind {
                    BodyKind::Preset => storage::save_presets().await,
                    BodyKind::Config => storage::save_config().await,
                };
                match self.kind {
                    BodyKind::Preset => JsonOut::ok(presets_json).await,
                    BodyKind::Config => {
                        JsonOut::ok(|b| msg_json(ok, if ok { None } else { Some("saving failed") }, reboot_required, b)).await
                    }
                }
            }
            Err(e) => JsonOut::error(StatusCode::BAD_REQUEST, e).await,
        };
        response.write_to(request.body_connection.finalize().await?, response_writer).await
    }
}

// ---------------------------------------------------------------------------
// OTA: POST /api/ota with the raw application binary (`*-ota.bin`).
// ---------------------------------------------------------------------------

struct OtaService;

const APP_START: u32 = 0x1000_7000;
const APP_END: u32 = 0x1010_1000;

fn ota_error(msg: &'static str) -> (StatusCode, &'static str) {
    (StatusCode::BAD_REQUEST, msg)
}

impl RequestHandlerService<()> for OtaService {
    async fn call_request_handler_service<R: Read, W: ResponseWriter<Error = R::Error>>(
        &self,
        _state: &(),
        _params: (),
        mut request: Request<'_, R>,
        response_writer: W,
    ) -> Result<ResponseSent, W::Error> {
        if !check_auth(&request.parts) {
            return Unauthorized.write_to(request.body_connection.finalize().await?, response_writer).await;
        }
        let len = request.body_connection.content_length();
        let result = ota_receive(&mut request, len).await;
        let connection = request.body_connection.finalize().await?;
        match result {
            Ok(()) => {
                log::info!("ota: update stored ({} bytes), rebooting", len);
                system::request_reboot();
                (StatusCode::OK, "OK, rebooting into the new firmware\n").write_to(connection, response_writer).await
            }
            Err(msg) => {
                log::warn!("ota: {}", msg);
                ota_error(msg).write_to(connection, response_writer).await
            }
        }
    }
}

#[repr(align(4))]
struct Aligned([u8; 1024]);

async fn ota_receive<R: Read>(request: &mut Request<'_, R>, len: usize) -> Result<(), &'static str> {
    if cfg!(feature = "standalone") {
        return Err("firmware update is not available in the standalone (no bootloader) build");
    }
    let flash = *FLASH.try_get().ok_or("flash not ready")?;
    if len < 1024 {
        return Err("file too small");
    }
    if len as u32 > system::dfu_size() {
        return Err("file too large for the update partition");
    }
    net::touch_http();
    let mut state_buf = system::StateBuf::new();
    let mut updater = system::Updater::new(system::updater_config(flash), &mut state_buf.0);
    // The server's read_request timeout (5 s) covers the whole request; a
    // ~700 KB upload with flash writes takes longer, so give the body its own.
    let mut reader = request.body_connection.body().reader().with_different_timeout(Duration::from_secs(180));
    let mut chunk = Aligned([0xff; 1024]);
    let mut offset = 0usize;
    let mut fill = 0usize;
    let mut checked = false;
    loop {
        let n = reader.read(&mut chunk.0[fill..]).await.map_err(|_| "upload interrupted")?;
        fill += n;
        if fill < chunk.0.len() && n != 0 {
            continue;
        }
        if fill > 0 {
            if !checked {
                checked = true;
                if &chunk.0[..4] == b"UF2\n" {
                    return Err("this is a UF2 file - upload the *-ota.bin file instead");
                }
                let sp = u32::from_le_bytes([chunk.0[0], chunk.0[1], chunk.0[2], chunk.0[3]]);
                let reset = u32::from_le_bytes([chunk.0[4], chunk.0[5], chunk.0[6], chunk.0[7]]);
                if !(0x2000_0000..=0x2004_2000).contains(&sp) || !(APP_START..APP_END).contains(&(reset & !1)) {
                    return Err("not a Pico Light Show application image");
                }
            }
            updater.write_firmware(offset, &chunk.0[..fill]).await.map_err(|e| {
                log::error!("ota: write failed: {:?}", e);
                "flash write failed (device still confirming a previous update? retry in a minute)"
            })?;
            offset += fill;
            fill = 0;
            chunk.0.fill(0xff);
            net::touch_http();
        }
        if n == 0 {
            break;
        }
    }
    if offset != len {
        return Err("upload incomplete");
    }
    updater.mark_updated().await.map_err(|_| "cannot mark update")?;
    Ok(())
}
