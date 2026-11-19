//! Minimal HTTP/1.0 GET client over plain TCP or TLS 1.3 (`embedded-tls`).
//!
//! Used to download the update manifest and firmware images. The server
//! certificate is not verified (the RP2040 has no practical way to keep a CA
//! store up to date); the image is checked against the SHA-256 in the manifest
//! instead, which catches broken or truncated downloads.

use core::str::FromStr;

use embassy_net::dns::DnsQueryType;
use embassy_net::tcp::TcpSocket;
use core::net::Ipv4Addr;

use embassy_net::{IpAddress, Stack};
use embassy_rp::clocks::RoscRng;
use embassy_time::{Duration, with_timeout};
use embedded_io_async::{Read, Write};
use embedded_tls::{Aes128GcmSha256, TlsConfig, TlsConnection, TlsContext, UnsecureProvider};
use heapless::String;

/// Size of a body chunk handed to the sink (all chunks except the last one).
pub const CHUNK: usize = 1024;

/// Receives the response body.
pub trait Sink {
    /// Called once with the Content-Length (if the server sent one).
    async fn begin(&mut self, len: Option<u32>) -> Result<(), &'static str>;
    /// Called with consecutive body chunks of `CHUNK` bytes (the last one may be shorter).
    async fn data(&mut self, data: &[u8]) -> Result<(), &'static str>;
}

/// Buffers for one request (kept in a static by the caller).
pub struct Buffers {
    tcp_rx: [u8; 2048],
    tcp_tx: [u8; 1024],
    tls_rx: [u8; 16640],
    tls_tx: [u8; 1024],
    work: Work,
}

#[repr(align(4))]
struct Work([u8; 2048]);

impl Buffers {
    pub const fn new() -> Self {
        Self { tcp_rx: [0; 2048], tcp_tx: [0; 1024], tls_rx: [0; 16640], tls_tx: [0; 1024], work: Work([0; 2048]) }
    }
}

pub struct Url<'a> {
    pub tls: bool,
    pub host: &'a str,
    pub port: u16,
    pub path: &'a str,
}

pub fn parse_url(url: &str) -> Option<Url<'_>> {
    let (tls, rest) = if let Some(r) = url.strip_prefix("https://") {
        (true, r)
    } else if let Some(r) = url.strip_prefix("http://") {
        (false, r)
    } else {
        return None;
    };
    let (authority, path) = match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, "/"),
    };
    let (host, port) = match authority.rfind(':') {
        Some(i) => (&authority[..i], authority[i + 1..].parse().ok()?),
        None => (authority, if tls { 443 } else { 80 }),
    };
    if host.is_empty() {
        return None;
    }
    Some(Url { tls, host, port, path })
}

/// Resolve `location` (absolute URL or absolute path) relative to `base`.
pub fn join_url(base: &str, location: &str, out: &mut String<200>) -> Result<(), &'static str> {
    out.clear();
    let too_long = "URL too long";
    if location.starts_with("http://") || location.starts_with("https://") {
        return out.push_str(location).map_err(|_| too_long);
    }
    let b = parse_url(base).ok_or("invalid URL")?;
    let scheme_host_len = base.len() - b.path.len();
    out.push_str(&base[..scheme_host_len]).map_err(|_| too_long)?;
    if location.starts_with('/') {
        out.push_str(location).map_err(|_| too_long)
    } else {
        // relative to the directory of the base path
        let dir = &b.path[..b.path.rfind('/').map(|i| i + 1).unwrap_or(0)];
        out.push_str(dir).map_err(|_| too_long)?;
        out.push_str(location).map_err(|_| too_long)
    }
}

async fn resolve(stack: Stack<'static>, host: &str) -> Result<IpAddress, &'static str> {
    if let Ok(ip) = Ipv4Addr::from_str(host) {
        return Ok(IpAddress::Ipv4(ip));
    }
    let addrs = with_timeout(Duration::from_secs(10), stack.dns_query(host, DnsQueryType::A))
        .await
        .map_err(|_| "DNS timeout")?
        .map_err(|_| "cannot resolve host name")?;
    addrs.first().copied().ok_or("cannot resolve host name")
}

/// GET `url` and stream the body into `sink`. Follows up to 3 redirects.
pub async fn get<S: Sink>(stack: Stack<'static>, bufs: &mut Buffers, url: &str, sink: &mut S) -> Result<(), &'static str> {
    let mut current: String<200> = String::new();
    current.push_str(url).map_err(|_| "URL too long")?;
    for _ in 0..4 {
        let mut location: String<200> = String::new();
        match get_once(stack, bufs, &current, sink, &mut location).await? {
            Outcome::Done => return Ok(()),
            Outcome::Redirect => {
                let mut next = String::new();
                join_url(&current, &location, &mut next)?;
                log::info!("fetch: redirected to {}", next.as_str());
                current = next;
            }
        }
    }
    Err("too many redirects")
}

enum Outcome {
    Done,
    Redirect,
}

async fn get_once<S: Sink>(
    stack: Stack<'static>,
    bufs: &mut Buffers,
    url: &str,
    sink: &mut S,
    location: &mut String<200>,
) -> Result<Outcome, &'static str> {
    let u = parse_url(url).ok_or("invalid URL")?;
    let addr = resolve(stack, u.host).await?;
    let Buffers { tcp_rx, tcp_tx, tls_rx, tls_tx, work } = bufs;
    let mut sock = TcpSocket::new(stack, tcp_rx, tcp_tx);
    sock.set_timeout(Some(Duration::from_secs(20)));
    with_timeout(Duration::from_secs(15), sock.connect((addr, u.port)))
        .await
        .map_err(|_| "connection timeout")?
        .map_err(|_| "cannot connect")?;

    let result = if u.tls {
        let mut tls: TlsConnection<'_, _, Aes128GcmSha256> = TlsConnection::new(sock, tls_rx, tls_tx);
        // Offer RSA-PSS too: many servers (GitHub / Fastly) only have RSA
        // certificates. The certificate itself is not verified (see module docs).
        let config = TlsConfig::new().enable_rsa_signatures().with_server_name(u.host);
        tls.open(TlsContext::new(&config, UnsecureProvider::new::<Aes128GcmSha256>(RoscRng)))
            .await
            .map_err(|e| {
                log::warn!("fetch: TLS handshake failed: {:?}", e);
                "TLS handshake failed"
            })?;
        let r = exchange(&mut tls, &u, &mut work.0, sink, location).await;
        let _ = tls.close().await;
        r
    } else {
        let r = exchange(&mut sock, &u, &mut work.0, sink, location).await;
        sock.close();
        r
    };
    result
}

/// Send the request, parse the response head and stream the body.
async fn exchange<C: Read + Write, S: Sink>(
    conn: &mut C,
    u: &Url<'_>,
    buf: &mut [u8],
    sink: &mut S,
    location: &mut String<200>,
) -> Result<Outcome, &'static str> {
    // --- request ---
    let mut req: String<400> = String::new();
    let _ = core::fmt::write(
        &mut req,
        format_args!(
            "GET {} HTTP/1.0\r\nHost: {}\r\nUser-Agent: pico-light-show/{}\r\nAccept: */*\r\nConnection: close\r\n\r\n",
            u.path,
            u.host,
            crate::system::VERSION
        ),
    );
    if req.len() >= req.capacity() - 1 {
        return Err("URL too long");
    }
    conn.write_all(req.as_bytes()).await.map_err(|_| "send failed")?;
    conn.flush().await.map_err(|_| "send failed")?;

    // --- response head ---
    let mut filled = 0usize;
    let head_end = loop {
        if filled == buf.len() {
            return Err("response header too large");
        }
        let n = conn.read(&mut buf[filled..]).await.map_err(|_| "receive failed")?;
        if n == 0 {
            return Err("connection closed by server");
        }
        filled += n;
        if let Some(i) = find(&buf[..filled], b"\r\n\r\n") {
            break i + 4;
        }
    };
    let head = core::str::from_utf8(&buf[..head_end]).map_err(|_| "invalid response")?;
    let mut lines = head.split("\r\n");
    let status_line = lines.next().unwrap_or("");
    let status: u16 = status_line.split(' ').nth(1).and_then(|s| s.parse().ok()).ok_or("invalid response")?;
    let mut content_length: Option<u32> = None;
    for line in lines {
        if let Some((k, v)) = line.split_once(':') {
            let v = v.trim();
            if k.eq_ignore_ascii_case("content-length") {
                content_length = v.parse().ok();
            } else if k.eq_ignore_ascii_case("location") {
                location.clear();
                location.push_str(v).map_err(|_| "redirect URL too long")?;
            }
        }
    }
    match status {
        200 => {}
        301 | 302 | 303 | 307 | 308 if !location.is_empty() => return Ok(Outcome::Redirect),
        404 => return Err("not found on the server (HTTP 404)"),
        _ => {
            log::warn!("fetch: {}", status_line);
            return Err("unexpected HTTP status");
        }
    }

    // --- body: hand out CHUNK sized pieces ---
    sink.begin(content_length).await?;
    let mut remaining = content_length.map(|l| l as usize);
    buf.copy_within(head_end..filled, 0);
    let mut fill = filled - head_end;
    loop {
        let want = remaining.map(|r| r.min(CHUNK)).unwrap_or(CHUNK);
        let mut eof = false;
        while fill < want {
            let n = match conn.read(&mut buf[fill..want]).await {
                Ok(n) => n,
                // Without a Content-Length the body ends when the connection closes.
                Err(_) if remaining.is_none() => 0,
                Err(_) => return Err("download interrupted"),
            };
            if n == 0 {
                eof = true;
                break;
            }
            fill += n;
        }
        let take = fill.min(want);
        if take > 0 {
            sink.data(&buf[..take]).await?;
            buf.copy_within(take..fill, 0);
            fill -= take;
            if let Some(r) = remaining.as_mut() {
                *r -= take;
            }
        }
        match remaining {
            Some(0) => return Ok(Outcome::Done),
            Some(_) if eof => return Err("download incomplete"),
            None if eof => return Ok(Outcome::Done),
            _ => {}
        }
    }
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}
