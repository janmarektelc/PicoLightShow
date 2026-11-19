//! Wi-Fi management (client / access point with fallback), DHCP server and
//! captive DNS for AP mode, and a tiny mDNS responder (`<name>.local`).

use core::cell::RefCell;
use core::net::Ipv4Addr;

use cyw43::{Control, JoinOptions, ScanOptions};
use embassy_executor::Spawner;
use embassy_futures::select::{Either, select};
use embassy_net::udp::{PacketMetadata, UdpSocket};
use embassy_net::{ConfigV4, DhcpConfig, Ipv4Cidr, IpEndpoint, Stack, StaticConfigV4};
use embassy_sync::blocking_mutex::Mutex;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_sync::signal::Signal;
use embassy_time::{Duration, Instant, Timer, with_timeout};
use heapless::{String, Vec};
use portable_atomic::{AtomicU64, Ordering};
use serde::Serialize;

use crate::config::{DeviceConfig, WifiMode};

pub const AP_ADDR: Ipv4Addr = Ipv4Addr::new(192, 168, 4, 1);
const AP_CHANNEL: u8 = 6;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NetMode {
    Starting,
    Connecting,
    Client,
    Ap,
    ApFallback,
}

#[derive(Clone, Debug, Serialize)]
pub struct NetInfo {
    pub mode: NetMode,
    pub ssid: String<32>,
    pub ip: [u8; 4],
    pub gateway: [u8; 4],
    pub mac: [u8; 6],
    pub rssi: i32,
}

pub static NET: Mutex<CriticalSectionRawMutex, RefCell<NetInfo>> = Mutex::new(RefCell::new(NetInfo {
    mode: NetMode::Starting,
    ssid: String::new(),
    ip: [0; 4],
    gateway: [0; 4],
    mac: [0; 6],
    rssi: 0,
}));

pub fn info() -> NetInfo {
    NET.lock(|n| n.borrow().clone())
}

fn set_mode(mode: NetMode, ssid: &str) {
    NET.lock(|n| {
        let mut n = n.borrow_mut();
        n.mode = mode;
        n.ssid.clear();
        let _ = n.ssid.push_str(ssid);
    });
}

pub fn mac_hex() -> String<12> {
    let mac = NET.lock(|n| n.borrow().mac);
    let mut s = String::new();
    for b in mac {
        let _ = core::fmt::write(&mut s, format_args!("{:02x}", b));
    }
    s
}

/// Time of the last HTTP request; used to avoid dropping the fallback AP
/// while somebody is configuring the device.
pub static LAST_HTTP_MS: AtomicU64 = AtomicU64::new(0);

/// Until when (uptime seconds) the access point has a client with a valid DHCP lease.
static AP_CLIENT_UNTIL_S: AtomicU64 = AtomicU64::new(0);

fn ap_has_client() -> bool {
    Instant::now().as_secs() < AP_CLIENT_UNTIL_S.load(Ordering::Relaxed)
}

pub fn touch_http() {
    LAST_HTTP_MS.store(Instant::now().as_millis(), Ordering::Relaxed);
}

// ---------------------------------------------------------------------------
// Commands for the Wi-Fi task (it owns the CYW43 control handle)
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize)]
pub struct ScanEntry {
    pub ssid: String<32>,
    pub rssi: i16,
    pub secure: bool,
}

pub type ScanResult = Vec<ScanEntry, 24>;

pub enum WifiCmd {
    Scan,
}

pub static WIFI_CMD: Channel<CriticalSectionRawMutex, WifiCmd, 1> = Channel::new();
static SCAN_DONE: Signal<CriticalSectionRawMutex, ()> = Signal::new();
/// Result of the last scan (kept in a static so web handlers stay small).
pub static SCAN_RESULT: Mutex<CriticalSectionRawMutex, RefCell<ScanResult>> = Mutex::new(RefCell::new(ScanResult::new()));

/// Run a scan; the result is left in `SCAN_RESULT`.
pub async fn scan() {
    SCAN_DONE.reset();
    WIFI_CMD.send(WifiCmd::Scan).await;
    let _ = with_timeout(Duration::from_secs(15), SCAN_DONE.wait()).await;
}

// ---------------------------------------------------------------------------
// Wi-Fi task
// ---------------------------------------------------------------------------

fn ip_of(stack: &Stack<'static>) -> ([u8; 4], [u8; 4]) {
    match stack.config_v4() {
        Some(c) => (c.address.address().octets(), c.gateway.map(|g| g.octets()).unwrap_or([0; 4])),
        None => ([0; 4], [0; 4]),
    }
}

async fn join_client(control: &mut Control<'static>, stack: Stack<'static>, cfg: &DeviceConfig) -> bool {
    set_mode(NetMode::Connecting, &cfg.wifi_ssid);
    for attempt in 1..=3 {
        log::info!("wifi: joining '{}' (attempt {})", cfg.wifi_ssid.as_str(), attempt);
        let opts = if cfg.wifi_pass.is_empty() {
            JoinOptions::new_open()
        } else {
            JoinOptions::new(cfg.wifi_pass.as_bytes())
        };
        match with_timeout(Duration::from_secs(20), control.join(&cfg.wifi_ssid, opts)).await {
            Ok(Ok(())) => {
                if with_timeout(Duration::from_secs(20), stack.wait_config_up()).await.is_ok() {
                    let (ip, gw) = ip_of(&stack);
                    NET.lock(|n| {
                        let mut n = n.borrow_mut();
                        n.ip = ip;
                        n.gateway = gw;
                    });
                    set_mode(NetMode::Client, &cfg.wifi_ssid);
                    log::info!("wifi: connected, ip {}.{}.{}.{}", ip[0], ip[1], ip[2], ip[3]);
                    return true;
                }
                log::warn!("wifi: no IP address (DHCP timeout)");
                control.leave().await;
            }
            Ok(Err(e)) => log::warn!("wifi: join failed: {:?}", e),
            Err(_) => log::warn!("wifi: join timeout"),
        }
        Timer::after_secs(2).await;
    }
    false
}

fn client_config(cfg: &DeviceConfig) -> ConfigV4 {
    if cfg.dhcp {
        let mut d = DhcpConfig::default();
        let mut h = heapless::String::new();
        let _ = h.push_str(cfg.hostname().as_str());
        d.hostname = Some(h);
        ConfigV4::Dhcp(d)
    } else {
        let prefix = u32::from_be_bytes(cfg.netmask).count_ones() as u8;
        let mut dns = heapless::Vec::new();
        if cfg.dns != [0; 4] {
            let _ = dns.push(Ipv4Addr::from(cfg.dns));
        }
        ConfigV4::Static(StaticConfigV4 {
            address: Ipv4Cidr::new(Ipv4Addr::from(cfg.ip), prefix),
            gateway: if cfg.gateway == [0; 4] { None } else { Some(Ipv4Addr::from(cfg.gateway)) },
            dns_servers: dns,
        })
    }
}

async fn start_ap(control: &mut Control<'static>, stack: Stack<'static>, cfg: &DeviceConfig, fallback: bool) {
    let ssid = cfg.ap_ssid_effective();
    stack.set_config_v4(ConfigV4::Static(StaticConfigV4 {
        address: Ipv4Cidr::new(AP_ADDR, 24),
        gateway: Some(AP_ADDR),
        dns_servers: heapless::Vec::new(),
    }));
    if cfg.ap_pass.is_empty() {
        control.start_ap_open(&ssid, AP_CHANNEL).await;
    } else {
        control.start_ap_wpa2(&ssid, &cfg.ap_pass, AP_CHANNEL).await;
    }
    NET.lock(|n| {
        let mut n = n.borrow_mut();
        n.ip = AP_ADDR.octets();
        n.gateway = AP_ADDR.octets();
        n.rssi = 0;
    });
    set_mode(if fallback { NetMode::ApFallback } else { NetMode::Ap }, &ssid);
    log::info!("wifi: access point '{}' started, http://192.168.4.1", ssid.as_str());
}

async fn do_scan(control: &mut Control<'static>) {
    SCAN_RESULT.lock(|r| r.borrow_mut().clear());
    let mut scanner = control.scan(ScanOptions::default()).await;
    while let Some(bss) = scanner.next().await {
        let len = (bss.ssid_len as usize).min(32);
        let Ok(name) = core::str::from_utf8(&bss.ssid[..len]) else { continue };
        if name.is_empty() {
            continue;
        }
        let rssi = bss.rssi;
        let secure = bss.capability & 0x10 != 0;
        SCAN_RESULT.lock(|r| {
            let mut out = r.borrow_mut();
            if let Some(e) = out.iter_mut().find(|e| e.ssid.as_str() == name) {
                e.rssi = e.rssi.max(rssi);
                return;
            }
            let mut ssid = String::new();
            let _ = ssid.push_str(name);
            let _ = out.push(ScanEntry { ssid, rssi, secure });
        });
    }
    SCAN_RESULT.lock(|r| r.borrow_mut().sort_unstable_by(|a, b| b.rssi.cmp(&a.rssi)));
}

#[embassy_executor::task]
pub async fn wifi_task(mut control: Control<'static>, stack: Stack<'static>, spawner: Spawner, cfg: DeviceConfig) {
    // On-board LED (wired to the Wi-Fi chip): on while connecting / connected,
    // blinking in access point mode. Stays dark if the firmware does not get here.
    control.gpio_set(0, true).await;
    let mac = control.address().await;
    NET.lock(|n| n.borrow_mut().mac = mac);
    crate::config::set_mac(mac);
    log::info!(
        "wifi: MAC {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
        mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]
    );
    // Allow mDNS multicast frames through the Wi-Fi chip filter.
    let _ = control.add_multicast_address([0x01, 0x00, 0x5e, 0x00, 0x00, 0xfb]).await;

    let want_client = cfg.wifi_mode == WifiMode::Client && !cfg.wifi_ssid.is_empty();
    let mut fallback = false;
    let mut ap_running = false;

    if want_client {
        stack.set_config_v4(client_config(&cfg));
        if !join_client(&mut control, stack, &cfg).await {
            log::warn!("wifi: client connection failed, starting fallback access point");
            fallback = true;
        }
    }
    if !want_client || fallback {
        start_ap(&mut control, stack, &cfg, fallback).await;
        ap_running = true;
        spawner.spawn(dhcp_server_task(stack).unwrap());
        spawner.spawn(captive_dns_task(stack).unwrap());
    }
    // (Re-)register the mDNS group with the Wi-Fi chip now that the interface
    // is up; joining a network may reset the chip's multicast filter.
    let _ = control.add_multicast_address([0x01, 0x00, 0x5e, 0x00, 0x00, 0xfb]).await;
    crate::system::set_network_ready();
    crate::system::stage(8);

    let mut last_retry = Instant::now();
    let mut tick: u32 = 0;
    let mut led = true;
    loop {
        // 500 ms tick: LED blinking; every 20th tick (10 s): connection housekeeping.
        let timeout = async {
            loop {
                Timer::after_millis(500).await;
                tick = tick.wrapping_add(1);
                // LED: on = connecting, off = connected (network or AP client),
                // blinking = access point waiting for a client.
                let want = match info().mode {
                    NetMode::Ap | NetMode::ApFallback => !ap_has_client() && tick % 2 == 0,
                    NetMode::Client => false,
                    _ => true,
                };
                if want != led {
                    led = want;
                    control.gpio_set(0, want).await;
                }
                if tick == 20 {
                    crate::system::stage(9);
                }
                if tick % 20 == 0 {
                    break;
                }
            }
        };
        let cmd = select(WIFI_CMD.receive(), timeout).await;
        match cmd {
            Either::First(WifiCmd::Scan) => {
                do_scan(&mut control).await;
                SCAN_DONE.signal(());
            }
            Either::Second(()) => {
                if want_client && !fallback {
                    if stack.is_link_up() && stack.is_config_up() {
                        let rssi = control.get_rssi().await;
                        let (ip, gw) = ip_of(&stack);
                        NET.lock(|n| {
                            let mut n = n.borrow_mut();
                            n.rssi = rssi;
                            n.ip = ip;
                            n.gateway = gw;
                        });
                    } else {
                        log::warn!("wifi: connection lost, reconnecting");
                        control.leave().await;
                        if join_client(&mut control, stack, &cfg).await {
                            let _ = control.add_multicast_address([0x01, 0x00, 0x5e, 0x00, 0x00, 0xfb]).await;
                        }
                    }
                } else if fallback && ap_running {
                    // Periodically retry the configured network (e.g. the router was
                    // still booting after a power cut), but not while someone uses the AP.
                    let idle_ms = Instant::now().as_millis().saturating_sub(LAST_HTTP_MS.load(Ordering::Relaxed));
                    if last_retry.elapsed() > Duration::from_secs(300) && idle_ms > 120_000 {
                        last_retry = Instant::now();
                        log::info!("wifi: retrying client connection");
                        control.close_ap().await;
                        stack.set_config_v4(client_config(&cfg));
                        if join_client(&mut control, stack, &cfg).await {
                            let _ = control.add_multicast_address([0x01, 0x00, 0x5e, 0x00, 0x00, 0xfb]).await;
                            fallback = false;
                            ap_running = false;
                        } else {
                            start_ap(&mut control, stack, &cfg, true).await;
                        }
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// DHCP server + captive DNS (AP mode)
// ---------------------------------------------------------------------------

#[embassy_executor::task]
pub async fn dhcp_server_task(stack: Stack<'static>) {
    let mut rx_meta = [PacketMetadata::EMPTY; 4];
    let mut rx_buf = [0u8; 1024];
    let mut tx_meta = [PacketMetadata::EMPTY; 4];
    let mut tx_buf = [0u8; 1024];
    let mut socket = UdpSocket::new(stack, &mut rx_meta, &mut rx_buf, &mut tx_meta, &mut tx_buf);
    if socket.bind(67).is_err() {
        log::error!("dhcp: bind failed");
        return;
    }
    let mut gw = [AP_ADDR];
    let dns = [AP_ADDR];
    let mut options = edge_dhcp::server::ServerOptions::new(AP_ADDR, Some(&mut gw));
    options.dns = &dns;
    options.captive_url = Some("http://192.168.4.1/");
    // Short leases: clients renew every few minutes, so we notice when they leave.
    options.lease_duration_secs = 300;
    let mut server = edge_dhcp::server::Server::<_, 8>::new_with_et(AP_ADDR);
    let mut buf = [0u8; 576];
    loop {
        let Ok((n, remote)) = socket.recv_from(&mut buf).await else { continue };
        // Never act as a DHCP server on somebody else's network.
        if !matches!(info().mode, NetMode::Ap | NetMode::ApFallback) {
            continue;
        }
        let Ok(request) = edge_dhcp::Packet::decode(&buf[..n]) else { continue };
        let mut opt_buf = edge_dhcp::Options::buf();
        let reply = server.handle_request(&mut opt_buf, &options, &request);
        // Clients renew after half of the lease time, so a client counts as
        // present until a little after one lease period without any request.
        let until = if server.leases.is_empty() {
            0
        } else if reply.is_some() {
            Instant::now().as_secs() + options.lease_duration_secs as u64 + 30
        } else {
            AP_CLIENT_UNTIL_S.load(Ordering::Relaxed)
        };
        AP_CLIENT_UNTIL_S.store(until, Ordering::Relaxed);
        if let Some(reply) = reply {
            let mut out = [0u8; 576];
            if let Ok(data) = reply.encode(&mut out) {
                let ep = IpEndpoint::new(Ipv4Addr::BROADCAST.into(), remote.endpoint.port);
                let _ = socket.send_to(data, ep).await;
            }
        }
    }
}

#[embassy_executor::task]
pub async fn captive_dns_task(stack: Stack<'static>) {
    let mut rx_meta = [PacketMetadata::EMPTY; 4];
    let mut rx_buf = [0u8; 1024];
    let mut tx_meta = [PacketMetadata::EMPTY; 4];
    let mut tx_buf = [0u8; 1024];
    let mut socket = UdpSocket::new(stack, &mut rx_meta, &mut rx_buf, &mut tx_meta, &mut tx_buf);
    if socket.bind(53).is_err() {
        log::error!("dns: bind failed");
        return;
    }
    let mut req = [0u8; 512];
    let mut resp = [0u8; 512];
    loop {
        let Ok((n, remote)) = socket.recv_from(&mut req).await else { continue };
        // Only answer while we are actually the access point.
        if !matches!(info().mode, NetMode::Ap | NetMode::ApFallback) {
            continue;
        }
        if let Ok(len) = edge_captive::reply(&req[..n], &AP_ADDR.octets(), Duration::from_secs(60).into(), &mut resp) {
            let _ = socket.send_to(&resp[..len], remote.endpoint).await;
        }
    }
}

// ---------------------------------------------------------------------------
// mDNS responder: answers A queries for `<hostname>.local`
// ---------------------------------------------------------------------------

const MDNS_ADDR: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 251);
const MDNS_PORT: u16 = 5353;

/// Parse a DNS name starting at `pos`; returns the labels (no compression
/// support needed for questions in practice) and the position after the name.
fn read_name<'a>(pkt: &'a [u8], mut pos: usize, labels: &mut Vec<&'a [u8], 4>) -> Option<usize> {
    loop {
        let len = *pkt.get(pos)? as usize;
        if len == 0 {
            return Some(pos + 1);
        }
        if len & 0xc0 != 0 {
            // compressed pointer - follow it once
            let off = ((len & 0x3f) << 8) | *pkt.get(pos + 1)? as usize;
            let mut sub = Vec::new();
            read_name(pkt, off, &mut sub)?;
            for l in sub {
                labels.push(l).ok()?;
            }
            return Some(pos + 2);
        }
        labels.push(pkt.get(pos + 1..pos + 1 + len)?).ok()?;
        pos += 1 + len;
    }
}

fn build_mdns_answer(out: &mut [u8], id: u16, host: &str, ip: [u8; 4]) -> Option<usize> {
    let mut w = 0;
    let mut put = |data: &[u8], w: &mut usize| -> Option<()> {
        out.get_mut(*w..*w + data.len())?.copy_from_slice(data);
        *w += data.len();
        Some(())
    };
    put(&id.to_be_bytes(), &mut w)?;
    put(&[0x84, 0x00, 0, 0, 0, 1, 0, 0, 0, 0], &mut w)?; // response, authoritative, 1 answer
    put(&[host.len() as u8], &mut w)?;
    put(host.as_bytes(), &mut w)?;
    put(&[5], &mut w)?;
    put(b"local", &mut w)?;
    put(&[0], &mut w)?;
    put(&[0, 1, 0x80, 1], &mut w)?; // type A, class IN + cache flush
    put(&120u32.to_be_bytes(), &mut w)?;
    put(&[0, 4], &mut w)?;
    put(&ip, &mut w)?;
    Some(w)
}

/// Host name answered via mDNS (may get a suffix after a name conflict).
static MDNS_NAME: Mutex<CriticalSectionRawMutex, RefCell<String<40>>> = Mutex::new(RefCell::new(String::new()));

/// Effective host name (`<name>.local`).
pub fn hostname() -> String<40> {
    let n = MDNS_NAME.lock(|n| n.borrow().clone());
    if !n.is_empty() {
        return n;
    }
    let mut out = String::new();
    let _ = out.push_str(&crate::config::with(|c| c.hostname()));
    out
}

fn name_is(labels: &[&[u8]], host: &str) -> bool {
    labels.len() == 2 && labels[0].eq_ignore_ascii_case(host.as_bytes()) && labels[1].eq_ignore_ascii_case(b"local")
}

/// Query for `<host>.local` (type A, "QU" bit) used to probe for a conflict.
fn build_mdns_probe(out: &mut [u8], host: &str) -> Option<usize> {
    let need = 12 + 1 + host.len() + 7 + 4;
    let out = out.get_mut(..need)?;
    out[..12].copy_from_slice(&[0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0]);
    let mut w = 12;
    out[w] = host.len() as u8;
    out[w + 1..w + 1 + host.len()].copy_from_slice(host.as_bytes());
    w += 1 + host.len();
    out[w..w + 7].copy_from_slice(b"\x05local\x00");
    w += 7;
    out[w..w + 4].copy_from_slice(&[0, 1, 0x80, 1]);
    Some(w + 4)
}

/// Does this mDNS response contain an A record for `<host>.local`?
fn response_claims(pkt: &[u8], host: &str) -> bool {
    if pkt.len() < 12 || pkt[2] & 0x80 == 0 {
        return false;
    }
    let qd = u16::from_be_bytes([pkt[4], pkt[5]]);
    let an = u16::from_be_bytes([pkt[6], pkt[7]]);
    let mut pos = 12;
    for _ in 0..qd {
        let mut labels = Vec::new();
        let Some(next) = read_name(pkt, pos, &mut labels) else { return false };
        pos = next + 4;
    }
    for _ in 0..an {
        let mut labels = Vec::new();
        let Some(next) = read_name(pkt, pos, &mut labels) else { return false };
        let Some(h) = pkt.get(next..next + 10) else { return false };
        let rtype = u16::from_be_bytes([h[0], h[1]]);
        let rdlen = u16::from_be_bytes([h[8], h[9]]) as usize;
        if rtype == 1 && name_is(&labels, host) {
            return true;
        }
        pos = next + 10 + rdlen;
    }
    false
}

#[embassy_executor::task]
pub async fn mdns_task(stack: Stack<'static>) {
    crate::system::wait_network().await;
    if let Err(e) = stack.join_multicast_group(MDNS_ADDR) {
        log::warn!("mdns: join group failed: {:?}", e);
    }
    let mut rx_meta = [PacketMetadata::EMPTY; 4];
    let mut rx_buf = [0u8; 1536];
    let mut tx_meta = [PacketMetadata::EMPTY; 2];
    let mut tx_buf = [0u8; 512];
    let mut socket = UdpSocket::new(stack, &mut rx_meta, &mut rx_buf, &mut tx_meta, &mut tx_buf);
    if socket.bind(MDNS_PORT).is_err() {
        log::error!("mdns: bind failed");
        return;
    }
    let mut pkt = [0u8; 512];
    let mut out = [0u8; 128];
    let group = IpEndpoint::new(MDNS_ADDR.into(), MDNS_PORT);
    let our_ip = || embassy_net::IpAddress::Ipv4(Ipv4Addr::from(info().ip));

    // --- probing: is the name already used by another device? ---
    let mut host: String<40> = String::new();
    let _ = host.push_str(&crate::config::with(|c| c.hostname()));
    for attempt in 0..3u8 {
        let mut conflict = false;
        for _ in 0..3 {
            if let Some(len) = build_mdns_probe(&mut out, &host) {
                let _ = socket.send_to(&out[..len], group).await;
            }
            let until = Instant::now() + Duration::from_millis(250);
            while let Ok(Ok((n, remote))) =
                with_timeout(until.saturating_duration_since(Instant::now()), socket.recv_from(&mut pkt)).await
            {
                if remote.endpoint.addr != our_ip() && response_claims(&pkt[..n], &host) {
                    conflict = true;
                }
            }
        }
        if !conflict {
            break;
        }
        log::warn!("mdns: {}.local is already used by another device", host.as_str());
        // Make the name unique: add the MAC suffix (or a counter if it already has it).
        let base = crate::config::default_hostname();
        let suffix = &base[base.len() - 6..];
        if !host.ends_with(suffix) {
            let _ = host.push('-');
            let _ = host.push_str(suffix);
        } else {
            let _ = host.push('-');
            let _ = host.push((b'2' + attempt) as char);
        }
    }
    MDNS_NAME.lock(|n| *n.borrow_mut() = host.clone());
    log::info!("mdns: responding to {}.local", host.as_str());

    // --- announce ---
    for _ in 0..2 {
        let ip = info().ip;
        if let Some(len) = build_mdns_answer(&mut out, 0, host.as_str(), ip) {
            let _ = socket.send_to(&out[..len], group).await;
        }
        Timer::after_secs(1).await;
    }

    // --- respond ---
    loop {
        let Ok((n, remote)) = socket.recv_from(&mut pkt).await else { continue };
        let pkt = &pkt[..n];
        if n < 12 {
            continue;
        }
        if pkt[2] & 0x80 != 0 {
            if remote.endpoint.addr != our_ip() && response_claims(pkt, &host) {
                log::warn!("mdns: another device ({:?}) claims {}.local", remote.endpoint.addr, host.as_str());
            }
            continue;
        }
        let id = u16::from_be_bytes([pkt[0], pkt[1]]);
        let qd = u16::from_be_bytes([pkt[4], pkt[5]]);
        let mut pos = 12;
        let mut matched = false;
        let mut unicast = false;
        for _ in 0..qd {
            let mut labels = Vec::new();
            let Some(next) = read_name(pkt, pos, &mut labels) else { break };
            let Some(q) = pkt.get(next..next + 4) else { break };
            pos = next + 4;
            let qtype = u16::from_be_bytes([q[0], q[1]]);
            let qclass = u16::from_be_bytes([q[2], q[3]]);
            if (qtype == 1 || qtype == 255) && name_is(&labels, &host) {
                matched = true;
                unicast |= qclass & 0x8000 != 0; // "QU": querier asks for a unicast reply
            }
        }
        if !matched {
            continue;
        }
        let ip = info().ip;
        if ip == [0; 4] {
            continue;
        }
        let legacy = remote.endpoint.port != MDNS_PORT;
        let Some(len) = build_mdns_answer(&mut out, if legacy { id } else { 0 }, host.as_str(), ip) else { continue };
        if legacy || unicast {
            let _ = socket.send_to(&out[..len], remote.endpoint).await;
        }
        if !legacy {
            let _ = socket.send_to(&out[..len], group).await;
        }
    }
}
