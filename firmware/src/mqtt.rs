//! Minimal MQTT 3.1.1 client with Home Assistant integration
//! (MQTT discovery, JSON light schema, availability / last will).
//!
//! Topics (base = `picolightshow/<mac>` unless configured):
//! * `<base>/state`      retained JSON state
//! * `<base>/set`        JSON commands (HA JSON schema)
//! * `<base>/speed/set`  animation speed 1..255
//! * `<base>/status`     `online` / `offline` (last will)

use core::fmt::Write as _;
use core::net::Ipv4Addr;
use core::str::FromStr;

use embassy_futures::select::{Either4, select4};
use embassy_net::dns::DnsQueryType;
use embassy_net::tcp::TcpSocket;
use embassy_net::{IpAddress, Stack};
use embassy_time::{Duration, Instant, Timer, with_timeout};
use embedded_io_async::Write;
use heapless::{String, Vec};
use portable_atomic::{AtomicU8, Ordering};
use serde::Deserialize;

use crate::config::DeviceConfig;
use crate::effects::{EFFECT_COUNT, EFFECTS};
use crate::state;
use crate::{net, system, update};

const KEEPALIVE_S: u16 = 60;
const PING_EVERY: Duration = Duration::from_secs(30);

static STATUS: AtomicU8 = AtomicU8::new(0);

pub fn status_str() -> &'static str {
    match STATUS.load(Ordering::Relaxed) {
        1 => "connecting",
        2 => "connected",
        3 => "error",
        _ => "disabled",
    }
}

#[derive(Debug)]
enum Error {
    Dns,
    Connect,
    Io,
    Protocol,
    Refused(#[allow(dead_code)] u8),
    Timeout,
}

// ---------------------------------------------------------------------------
// Packet encoding
// ---------------------------------------------------------------------------

type Packet = Vec<u8, 2048>;

fn put_u16(p: &mut Packet, v: u16) -> Result<(), Error> {
    p.extend_from_slice(&v.to_be_bytes()).map_err(|_| Error::Protocol)
}

fn put_str(p: &mut Packet, s: &[u8]) -> Result<(), Error> {
    put_u16(p, s.len() as u16)?;
    p.extend_from_slice(s).map_err(|_| Error::Protocol)
}

/// Prepend fixed header (type byte + remaining length) to `body`.
fn finish(kind: u8, body: &Packet) -> Result<Packet, Error> {
    let mut out = Packet::new();
    out.push(kind).map_err(|_| Error::Protocol)?;
    let mut len = body.len();
    loop {
        let mut b = (len % 128) as u8;
        len /= 128;
        if len > 0 {
            b |= 0x80;
        }
        out.push(b).map_err(|_| Error::Protocol)?;
        if len == 0 {
            break;
        }
    }
    out.extend_from_slice(body).map_err(|_| Error::Protocol)?;
    Ok(out)
}

struct Conn<'a> {
    sock: TcpSocket<'a>,
    rx: [u8; 1536],
    rx_len: usize,
    last_rx: Instant,
}

impl<'a> Conn<'a> {
    async fn send(&mut self, pkt: &Packet) -> Result<(), Error> {
        self.sock.write_all(pkt).await.map_err(|_| Error::Io)?;
        self.sock.flush().await.map_err(|_| Error::Io)
    }

    async fn publish(&mut self, topic: &str, payload: &[u8], retain: bool) -> Result<(), Error> {
        let mut body = Packet::new();
        put_str(&mut body, topic.as_bytes())?;
        body.extend_from_slice(payload).map_err(|_| Error::Protocol)?;
        let pkt = finish(0x30 | retain as u8, &body)?;
        self.send(&pkt).await
    }

    async fn subscribe(&mut self, id: u16, topic: &str) -> Result<(), Error> {
        let mut body = Packet::new();
        put_u16(&mut body, id)?;
        put_str(&mut body, topic.as_bytes())?;
        body.push(0).map_err(|_| Error::Protocol)?; // QoS 0
        let pkt = finish(0x82, &body)?;
        self.send(&pkt).await
    }

    /// Read more bytes from the socket (cancel-safe).
    async fn fill(&mut self) -> Result<(), Error> {
        if self.rx_len == self.rx.len() {
            return Err(Error::Protocol); // packet larger than our buffer
        }
        let n = self.sock.read(&mut self.rx[self.rx_len..]).await.map_err(|_| Error::Io)?;
        if n == 0 {
            return Err(Error::Io);
        }
        self.rx_len += n;
        self.last_rx = Instant::now();
        Ok(())
    }

    /// If a complete packet is buffered, return (type byte, payload range, total length).
    fn packet(&self) -> Option<(u8, usize, usize)> {
        if self.rx_len < 2 {
            return None;
        }
        let mut len = 0usize;
        let mut mul = 1usize;
        let mut i = 1;
        loop {
            let b = *self.rx[..self.rx_len].get(i)?;
            len += (b & 0x7f) as usize * mul;
            mul *= 128;
            i += 1;
            if b & 0x80 == 0 {
                break;
            }
            if i > 4 {
                return None;
            }
        }
        let total = i + len;
        if self.rx_len < total { None } else { Some((self.rx[0], i, total)) }
    }

    fn consume(&mut self, n: usize) {
        self.rx.copy_within(n..self.rx_len, 0);
        self.rx_len -= n;
    }
}

// ---------------------------------------------------------------------------
// Home Assistant payloads
// ---------------------------------------------------------------------------

struct Topics {
    base: String<64>,
    node: String<32>,
}

fn topic(t: &Topics, suffix: &str) -> String<96> {
    let mut s = String::new();
    let _ = write!(s, "{}/{}", t.base, suffix);
    s
}

fn state_payload(out: &mut String<384>) {
    let s = state::snapshot();
    let p = s.current();
    let c = p.colors.first().copied().unwrap_or(crate::color::Rgb::WHITE);
    out.clear();
    let _ = write!(
        out,
        "{{\"state\":\"{}\",\"brightness\":{},\"color_mode\":\"rgb\",\"color\":{{\"r\":{},\"g\":{},\"b\":{}}},\"effect\":\"{}\",\"speed\":{},\"running\":{}}}",
        if s.on { "ON" } else { "OFF" },
        s.brightness,
        c.r,
        c.g,
        c.b,
        EFFECTS[s.effect as usize].name,
        s.speed,
        s.running
    );
}

async fn send_discovery(conn: &mut Conn<'_>, cfg: &DeviceConfig, t: &Topics) -> Result<(), Error> {
    let net = net::info();
    let ip = net.ip;
    let mut device: String<320> = String::new();
    let _ = write!(
        device,
        "\"dev\":{{\"ids\":[\"{node}\"],\"name\":\"{name}\",\"mdl\":\"Raspberry Pi Pico W\",\"mf\":\"Pico Light Show\",\"sw\":\"{ver}\",\"cu\":\"http://{a}.{b}.{c}.{d}/\"}}",
        node = t.node,
        name = cfg.display_name(),
        ver = system::VERSION,
        a = ip[0],
        b = ip[1],
        c = ip[2],
        d = ip[3]
    );

    // light entity
    let mut payload: String<1800> = String::new();
    let _ = write!(
        payload,
        "{{\"name\":null,\"uniq_id\":\"{node}\",\"schema\":\"json\",\"cmd_t\":\"{base}/set\",\"stat_t\":\"{base}/state\",\"avty_t\":\"{base}/status\",\"brightness\":true,\"bri_scl\":255,\"sup_clrm\":[\"rgb\"],\"effect\":true,\"fx_list\":[",
        node = t.node,
        base = t.base
    );
    for (i, e) in EFFECTS.iter().enumerate() {
        let _ = write!(payload, "{}\"{}\"", if i > 0 { "," } else { "" }, e.name);
    }
    let _ = write!(payload, "],{}}}", device);
    let mut ctopic: String<128> = String::new();
    let _ = write!(ctopic, "{}/light/{}/config", cfg.mqtt_discovery_prefix, t.node);
    conn.publish(&ctopic, payload.as_bytes(), true).await?;

    // speed number entity
    payload.clear();
    let _ = write!(
        payload,
        "{{\"name\":\"Speed\",\"uniq_id\":\"{node}_speed\",\"cmd_t\":\"{base}/speed/set\",\"stat_t\":\"{base}/state\",\"val_tpl\":\"{{{{ value_json.speed }}}}\",\"min\":1,\"max\":255,\"mode\":\"slider\",\"icon\":\"mdi:speedometer\",\"avty_t\":\"{base}/status\",\"dev\":{{\"ids\":[\"{node}\"]}}}}",
        node = t.node,
        base = t.base
    );
    ctopic.clear();
    let _ = write!(ctopic, "{}/number/{}_speed/config", cfg.mqtt_discovery_prefix, t.node);
    conn.publish(&ctopic, payload.as_bytes(), true).await?;

    // firmware update entity (Settings -> Updates in Home Assistant)
    payload.clear();
    let _ = write!(
        payload,
        "{{\"name\":\"Firmware\",\"uniq_id\":\"{node}_firmware\",\"dev_cla\":\"firmware\",\"ent_cat\":\"config\",\"stat_t\":\"{base}/update\",\"cmd_t\":\"{base}/update/install\",\"pl_inst\":\"install\",\"avty_t\":\"{base}/status\",\"dev\":{{\"ids\":[\"{node}\"]}}}}",
        node = t.node,
        base = t.base
    );
    ctopic.clear();
    let _ = write!(ctopic, "{}/update/{}/config", cfg.mqtt_discovery_prefix, t.node);
    conn.publish(&ctopic, payload.as_bytes(), true).await
}

async fn publish_update_state(conn: &mut Conn<'_>, topic: &str) -> Result<(), Error> {
    let mut buf = [0u8; 512];
    match update::ha_state_json(&mut buf) {
        Some(n) => conn.publish(topic, &buf[..n], true).await,
        None => Ok(()),
    }
}

#[derive(Deserialize)]
struct HaColor {
    r: u8,
    g: u8,
    b: u8,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct HaCommand {
    state: Option<String<8>>,
    brightness: Option<u8>,
    color: Option<HaColor>,
    effect: Option<String<32>>,
}

fn handle_command(payload: &[u8]) {
    let Ok((cmd, _)) = serde_json_core::from_slice::<HaCommand>(payload) else {
        log::warn!("mqtt: invalid command");
        return;
    };
    let effect = cmd.effect.as_ref().and_then(|name| EFFECTS.iter().position(|e| e.name == name.as_str()));
    state::update(|s| {
        if let Some(e) = effect {
            s.effect = e as u8;
        }
        if let Some(st) = &cmd.state {
            s.on = st.as_str() == "ON";
        }
        if let Some(b) = cmd.brightness {
            s.brightness = b;
        }
        if let Some(c) = &cmd.color {
            let color = crate::color::Rgb::new(c.r, c.g, c.b);
            let cur = s.effect as usize;
            if EFFECTS[cur].colors_max == 0 {
                s.effect = 0; // effect has no colors: switch to solid color
            }
            let e = s.effect as usize;
            if e < EFFECT_COUNT {
                if s.params[e].colors.is_empty() {
                    let _ = s.params[e].colors.push(color);
                } else {
                    s.params[e].colors[0] = color;
                }
            }
        }
    });
}

// ---------------------------------------------------------------------------
// Session
// ---------------------------------------------------------------------------

async fn resolve(stack: Stack<'static>, host: &str) -> Result<IpAddress, Error> {
    if let Ok(ip) = Ipv4Addr::from_str(host) {
        return Ok(IpAddress::Ipv4(ip));
    }
    let addrs = stack.dns_query(host, DnsQueryType::A).await.map_err(|_| Error::Dns)?;
    addrs.first().copied().ok_or(Error::Dns)
}

async fn session(stack: Stack<'static>, cfg: &DeviceConfig, t: &Topics) -> Result<(), Error> {
    let addr = resolve(stack, &cfg.mqtt_host).await?;
    let mut rx_buf = [0u8; 2048];
    let mut tx_buf = [0u8; 2048];
    let mut sock = TcpSocket::new(stack, &mut rx_buf, &mut tx_buf);
    sock.set_timeout(Some(Duration::from_secs(KEEPALIVE_S as u64 * 3 / 2)));
    log::info!("mqtt: connecting to {}:{}", cfg.mqtt_host.as_str(), cfg.mqtt_port);
    with_timeout(Duration::from_secs(10), sock.connect((addr, cfg.mqtt_port)))
        .await
        .map_err(|_| Error::Timeout)?
        .map_err(|_| Error::Connect)?;

    let mut conn = Conn { sock, rx: [0; 1536], rx_len: 0, last_rx: Instant::now() };

    // CONNECT
    let status_topic = topic(t, "status");
    let mut body = Packet::new();
    put_str(&mut body, b"MQTT")?;
    let mut flags = 0x02 | 0x04 | 0x20; // clean session, will, will retain
    if !cfg.mqtt_user.is_empty() {
        flags |= 0x80;
        if !cfg.mqtt_pass.is_empty() {
            flags |= 0x40;
        }
    }
    body.extend_from_slice(&[4, flags]).map_err(|_| Error::Protocol)?;
    put_u16(&mut body, KEEPALIVE_S)?;
    let mut client_id: String<40> = String::new();
    let _ = write!(client_id, "pls-{}", net::mac_hex());
    put_str(&mut body, client_id.as_bytes())?;
    put_str(&mut body, status_topic.as_bytes())?;
    put_str(&mut body, b"offline")?;
    if flags & 0x80 != 0 {
        put_str(&mut body, cfg.mqtt_user.as_bytes())?;
    }
    if flags & 0x40 != 0 {
        put_str(&mut body, cfg.mqtt_pass.as_bytes())?;
    }
    let pkt = finish(0x10, &body)?;
    conn.send(&pkt).await?;

    // CONNACK
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some((kind, start, total)) = conn.packet() {
            if kind & 0xf0 != 0x20 || total < start + 2 {
                return Err(Error::Protocol);
            }
            let rc = conn.rx[start + 1];
            conn.consume(total);
            if rc != 0 {
                return Err(Error::Refused(rc));
            }
            break;
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        with_timeout(remaining, conn.fill()).await.map_err(|_| Error::Timeout)??;
    }
    log::info!("mqtt: connected");
    STATUS.store(2, Ordering::Relaxed);

    let set_topic = topic(t, "set");
    let speed_topic = topic(t, "speed/set");
    let state_topic = topic(t, "state");
    let install_topic = topic(t, "update/install");
    let update_topic = topic(t, "update");
    conn.subscribe(1, &set_topic).await?;
    conn.subscribe(2, &speed_topic).await?;
    conn.subscribe(3, &install_topic).await?;
    conn.publish(&status_topic, b"online", true).await?;
    send_discovery(&mut conn, cfg, t).await?;
    let mut payload: String<384> = String::new();
    state_payload(&mut payload);
    conn.publish(&state_topic, payload.as_bytes(), true).await?;
    publish_update_state(&mut conn, &update_topic).await?;

    let mut watch = state::CHANGED.receiver();
    let mut update_watch = update::CHANGED.receiver();
    let mut update_dirty = false;
    let mut last_ping = Instant::now();
    let mut dirty = false;

    loop {
        // handle all complete packets
        while let Some((kind, start, total)) = conn.packet() {
            match kind & 0xf0 {
                0x30 => {
                    let qos = (kind >> 1) & 3;
                    let data = &conn.rx[start..total];
                    if data.len() >= 2 {
                        let tlen = u16::from_be_bytes([data[0], data[1]]) as usize;
                        let mut pos = 2 + tlen;
                        if qos > 0 {
                            pos += 2;
                        }
                        if pos <= data.len() {
                            let topic_bytes = &data[2..2 + tlen];
                            let msg = &data[pos..];
                            if topic_bytes == set_topic.as_bytes() {
                                handle_command(msg);
                            } else if topic_bytes == install_topic.as_bytes() {
                                if msg == b"install" {
                                    log::info!("mqtt: firmware update requested");
                                    update::request_install();
                                }
                            } else if topic_bytes == speed_topic.as_bytes() {
                                if let Some(v) = core::str::from_utf8(msg).ok().and_then(|s| s.trim().parse::<f32>().ok()) {
                                    let v = (v as i32).clamp(1, 255) as u8;
                                    state::update(|s| s.speed = v);
                                }
                            }
                        }
                    }
                }
                0x90 | 0xd0 | 0xb0 => {} // SUBACK, PINGRESP, UNSUBACK
                _ => log::debug!("mqtt: ignoring packet type {:#x}", kind),
            }
            conn.consume(total);
        }

        if conn.last_rx.elapsed() > Duration::from_secs(KEEPALIVE_S as u64 * 3 / 2) {
            return Err(Error::Timeout);
        }

        let changed = async {
            match watch.as_mut() {
                Some(w) => {
                    w.changed().await;
                }
                None => core::future::pending::<()>().await,
            }
        };
        let update_changed = async {
            match update_watch.as_mut() {
                Some(w) => {
                    w.changed().await;
                }
                None => core::future::pending::<()>().await,
            }
        };
        let ping_in = PING_EVERY.checked_sub(last_ping.elapsed()).unwrap_or(Duration::from_ticks(0));
        let wait = if dirty || update_dirty { Duration::from_millis(150) } else { ping_in };
        match select4(conn.fill(), changed, Timer::after(wait), update_changed).await {
            Either4::First(r) => r?,
            Either4::Second(()) => dirty = true,
            Either4::Fourth(()) => update_dirty = true,
            Either4::Third(()) => {
                if update_dirty {
                    update_dirty = false;
                    publish_update_state(&mut conn, &update_topic).await?;
                }
                if dirty {
                    dirty = false;
                    state_payload(&mut payload);
                    conn.publish(&state_topic, payload.as_bytes(), true).await?;
                }
                if last_ping.elapsed() >= PING_EVERY {
                    last_ping = Instant::now();
                    conn.send(&finish(0xc0, &Packet::new())?).await?;
                }
            }
        }
    }
}

#[embassy_executor::task]
pub async fn mqtt_task(stack: Stack<'static>, cfg: DeviceConfig) {
    STATUS.store(1, Ordering::Relaxed);
    system::wait_network().await;
    let mut t = Topics { base: String::new(), node: String::new() };
    let mac = net::mac_hex();
    if cfg.mqtt_topic.is_empty() {
        let _ = write!(t.base, "picolightshow/{}", mac);
    } else {
        let _ = t.base.push_str(cfg.mqtt_topic.trim_end_matches('/'));
    }
    // Same id as the original C++ firmware: Home Assistant keeps the existing
    // device and light entity (entity_id, automations) after the upgrade.
    let _ = write!(t.node, "pico_light_show_{}", mac);

    let mut backoff = 5u64;
    loop {
        if !matches!(net::info().mode, net::NetMode::Client) {
            Timer::after_secs(10).await;
            continue;
        }
        STATUS.store(1, Ordering::Relaxed);
        let started = Instant::now();
        match session(stack, &cfg, &t).await {
            Ok(()) => {}
            Err(e) => log::warn!("mqtt: disconnected: {:?}", e),
        }
        STATUS.store(3, Ordering::Relaxed);
        if started.elapsed() > Duration::from_secs(120) {
            backoff = 5;
        }
        Timer::after_secs(backoff).await;
        backoff = (backoff * 2).min(60);
    }
}
