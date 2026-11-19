//! DDP (Distributed Display Protocol) receiver on UDP 4048, e.g. for OpenRGB,
//! xLights or WLED-compatible senders. While frames are arriving they replace
//! the running effect; after `TIMEOUT_MS` without data the effect resumes.

use core::cell::RefCell;

use embassy_net::Stack;
use embassy_net::udp::{PacketMetadata, UdpSocket};
use embassy_sync::blocking_mutex::Mutex;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_time::Instant;
use portable_atomic::{AtomicU64, Ordering};

use crate::color::Rgb;
use crate::config::MAX_LEDS;

pub const PORT: u16 = 4048;
const TIMEOUT_MS: u64 = 3000;
const HEADER_LEN: usize = 10;
const FLAG_TIMECODE: u8 = 0x10;
const FLAG_QUERY: u8 = 0x02;

static ACTIVE_UNTIL_MS: AtomicU64 = AtomicU64::new(0);
static FRAME: Mutex<CriticalSectionRawMutex, RefCell<[u8; MAX_LEDS * 3]>> =
    Mutex::new(RefCell::new([0; MAX_LEDS * 3]));

pub fn is_active() -> bool {
    Instant::now().as_millis() < ACTIVE_UNTIL_MS.load(Ordering::Relaxed)
}

/// Copy the last received frame into `out` (RGB order).
pub fn copy_frame(out: &mut [Rgb]) {
    FRAME.lock(|f| {
        let f = f.borrow();
        for (i, px) in out.iter_mut().enumerate() {
            *px = Rgb::new(f[i * 3], f[i * 3 + 1], f[i * 3 + 2]);
        }
    });
}

fn handle_packet(pkt: &[u8]) {
    if pkt.len() < HEADER_LEN {
        return;
    }
    let flags = pkt[0];
    if flags & 0xc0 != 0x40 || flags & FLAG_QUERY != 0 {
        return; // unsupported version or query packet
    }
    let header = if flags & FLAG_TIMECODE != 0 { HEADER_LEN + 4 } else { HEADER_LEN };
    if pkt.len() < header {
        return;
    }
    let offset = u32::from_be_bytes([pkt[4], pkt[5], pkt[6], pkt[7]]) as usize;
    let len = u16::from_be_bytes([pkt[8], pkt[9]]) as usize;
    let data = &pkt[header..];
    let len = len.min(data.len());
    if offset >= MAX_LEDS * 3 {
        return;
    }
    let len = len.min(MAX_LEDS * 3 - offset);
    FRAME.lock(|f| {
        f.borrow_mut()[offset..offset + len].copy_from_slice(&data[..len]);
    });
    ACTIVE_UNTIL_MS.store(Instant::now().as_millis() + TIMEOUT_MS, Ordering::Relaxed);
}

#[embassy_executor::task]
pub async fn ddp_task(stack: Stack<'static>) {
    let mut rx_meta = [PacketMetadata::EMPTY; 8];
    let mut rx_buf = [0u8; 6144];
    let mut tx_meta = [PacketMetadata::EMPTY; 1];
    let mut tx_buf = [0u8; 64];
    let mut socket = UdpSocket::new(stack, &mut rx_meta, &mut rx_buf, &mut tx_meta, &mut tx_buf);
    if let Err(e) = socket.bind(PORT) {
        log::error!("ddp: bind failed: {:?}", e);
        return;
    }
    log::info!("ddp: listening on UDP {}", PORT);
    let mut buf = [0u8; 1500];
    loop {
        match socket.recv_from(&mut buf).await {
            Ok((n, _)) => handle_packet(&buf[..n]),
            Err(e) => log::warn!("ddp: recv error {:?}", e),
        }
    }
}
