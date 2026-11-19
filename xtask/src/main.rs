//! Build helper.
//!
//!   cargo xtask dist    build bootloader + firmware, write dist/*.uf2 and dist/*-ota.bin
//!   cargo xtask flash   `dist` and copy the UF2 to a Pico in BOOTSEL mode (RPI-RP2 drive)
//!   cargo xtask ota <host> [password]   `dist` and upload the OTA image over Wi-Fi
//!   cargo xtask release ["release notes"]   production `dist`, copy the images to
//!                       releases/<version>/ and write releases/latest.json, the
//!                       manifest the devices check for online updates
//!
//! Add `--console` to any command for a build with log output on the USB serial
//! port (files get a `-console` suffix). Without it the build is for production.
//! `--standalone` builds a diagnostic image without the bootloader (no OTA).

use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Command, exit};
use std::time::Duration;

use object::{Object, ObjectSegment};

const FLASH_BASE: u32 = 0x1000_0000;
const STATE_START: u32 = 0x1000_6000;
const STATE_SIZE: u32 = 0x1000;
const APP_START: u32 = 0x1000_7000;
const APP_END: u32 = 0x1010_1000;
const UF2_FAMILY_RP2040: u32 = 0xe48b_ff56;

type Result<T> = std::result::Result<T, String>;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf()
}

fn cargo_build(dir: &Path, features: &[&str]) -> Result<()> {
    println!("==> building {} {}", dir.display(), if features.is_empty() { String::new() } else { format!("(features: {})", features.join(",")) });
    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let mut args = vec!["build".to_string(), "--release".to_string()];
    if !features.is_empty() {
        args.push("--features".into());
        args.push(features.join(","));
    }
    let status = Command::new(cargo)
        .args(&args)
        .current_dir(dir)
        // Do not leak the host toolchain configuration of the xtask build.
        .env_remove("CARGO_TARGET_DIR")
        .env_remove("RUSTFLAGS")
        .status()
        .map_err(|e| format!("cannot run cargo: {e}"))?;
    if !status.success() {
        return Err(format!("build failed in {}", dir.display()));
    }
    Ok(())
}

fn package_version(dir: &Path) -> String {
    let toml = fs::read_to_string(dir.join("Cargo.toml")).unwrap_or_default();
    toml.lines()
        .find_map(|l| l.trim().strip_prefix("version = \"").and_then(|v| v.strip_suffix('"')))
        .unwrap_or("0.0.0")
        .to_string()
}

/// Load all PT_LOAD segments of an ELF into a sparse byte map keyed by physical address.
fn load_elf(path: &Path, image: &mut BTreeMap<u32, u8>) -> Result<(u32, u32)> {
    let data = fs::read(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let file = object::File::parse(&*data).map_err(|e| format!("bad ELF {}: {e}", path.display()))?;
    let (mut lo, mut hi) = (u32::MAX, 0u32);
    // `object` exposes virtual addresses; for these images LMA == VMA except for
    // .data, whose load address is read from the program header below.
    if let object::File::Elf32(elf) = &file {
        use object::read::elf::ProgramHeader;
        let endian = elf.endian();
        for ph in elf.elf_program_headers() {
            if ph.p_type(endian) != object::elf::PT_LOAD || ph.p_filesz(endian) == 0 {
                continue;
            }
            let paddr = ph.p_paddr(endian);
            if !(FLASH_BASE..FLASH_BASE + 0x20_0000).contains(&paddr) {
                continue; // RAM-only segment
            }
            let bytes = ph.data(endian, &*data).map_err(|_| "bad segment".to_string())?;
            for (i, b) in bytes.iter().enumerate() {
                image.insert(paddr + i as u32, *b);
            }
            lo = lo.min(paddr);
            hi = hi.max(paddr + bytes.len() as u32);
        }
    } else {
        for seg in file.segments() {
            let bytes = seg.data().map_err(|e| e.to_string())?;
            let addr = seg.address() as u32;
            for (i, b) in bytes.iter().enumerate() {
                image.insert(addr + i as u32, *b);
            }
        }
    }
    if lo == u32::MAX {
        return Err(format!("no flash segments in {}", path.display()));
    }
    Ok((lo, hi))
}

fn write_uf2(image: &BTreeMap<u32, u8>, out: &Path) -> Result<()> {
    // Group into 256-byte pages.
    let mut pages: BTreeMap<u32, [u8; 256]> = BTreeMap::new();
    for (&addr, &b) in image {
        let page = addr & !0xff;
        pages.entry(page).or_insert([0xff; 256])[(addr - page) as usize] = b;
    }
    let total = pages.len() as u32;
    let mut buf = Vec::with_capacity(pages.len() * 512);
    for (i, (addr, data)) in pages.iter().enumerate() {
        let mut block = [0u8; 512];
        let words: [u32; 8] =
            [0x0A32_4655, 0x9E5D_5157, 0x0000_2000, *addr, 256, i as u32, total, UF2_FAMILY_RP2040];
        for (j, w) in words.iter().enumerate() {
            block[j * 4..j * 4 + 4].copy_from_slice(&w.to_le_bytes());
        }
        block[32..32 + 256].copy_from_slice(data);
        block[508..512].copy_from_slice(&0x0AB1_6F30u32.to_le_bytes());
        buf.extend_from_slice(&block);
    }
    fs::write(out, buf).map_err(|e| format!("cannot write {}: {e}", out.display()))
}

struct Dist {
    uf2: PathBuf,
    ota: PathBuf,
}

fn dist(console: bool, standalone: bool) -> Result<Dist> {
    if standalone {
        return dist_standalone(console);
    }
    let root = root();
    let fw_dir = root.join("firmware");
    let bl_dir = root.join("bootloader");
    cargo_build(&bl_dir, &[])?;
    cargo_build(&fw_dir, if console { &["console"] } else { &[] })?;

    let bl_elf = bl_dir.join("target/thumbv6m-none-eabi/release/pico-light-show-bootloader");
    let fw_elf = fw_dir.join("target/thumbv6m-none-eabi/release/pico-light-show");

    // OTA image: application only, starting at the ACTIVE partition.
    let mut app = BTreeMap::new();
    let (lo, hi) = load_elf(&fw_elf, &mut app)?;
    if lo != APP_START {
        return Err(format!("firmware does not start at {APP_START:#x} (found {lo:#x})"));
    }
    if hi > APP_END {
        return Err(format!("firmware too large: ends at {hi:#x}, limit {APP_END:#x}"));
    }
    let mut ota = vec![0xffu8; (hi - lo) as usize];
    for (&a, &b) in &app {
        ota[(a - lo) as usize] = b;
    }

    // Full image: bootloader + erased boot state + application.
    let mut full = BTreeMap::new();
    let (blo, bhi) = load_elf(&bl_elf, &mut full)?;
    if blo != FLASH_BASE || bhi > STATE_START {
        return Err(format!("bootloader layout unexpected: {blo:#x}..{bhi:#x}"));
    }
    for a in STATE_START..STATE_START + STATE_SIZE {
        full.insert(a, 0xff);
    }
    full.extend(app);

    let version = package_version(&fw_dir) + if console { "-console" } else { "" };
    let out_dir = root.join("dist");
    fs::create_dir_all(&out_dir).map_err(|e| e.to_string())?;
    let uf2 = out_dir.join(format!("pico-light-show-{version}.uf2"));
    let ota_path = out_dir.join(format!("pico-light-show-{version}-ota.bin"));
    write_uf2(&full, &uf2)?;
    fs::write(&ota_path, &ota).map_err(|e| e.to_string())?;

    println!();
    println!("firmware size : {} KiB of {} KiB", (hi - lo) / 1024, (APP_END - APP_START) / 1024);
    println!("USB install   : {}", uf2.display());
    println!("OTA update    : {}", ota_path.display());
    if console {
        println!("console build : logs on the USB serial port (VS Code: Serial Monitor)");
    }
    Ok(Dist { uf2, ota: ota_path })
}

/// Firmware without bootloader: the ELF contains BOOT2 and starts at 0x10000000.
fn dist_standalone(console: bool) -> Result<Dist> {
    let root = root();
    let fw_dir = root.join("firmware");
    let mut features = vec!["standalone"];
    if console {
        features.push("console");
    }
    cargo_build(&fw_dir, &features)?;
    let fw_elf = fw_dir.join("target/thumbv6m-none-eabi/release/pico-light-show");
    let mut image = BTreeMap::new();
    let (lo, hi) = load_elf(&fw_elf, &mut image)?;
    if lo != FLASH_BASE {
        return Err(format!("standalone firmware must start at {FLASH_BASE:#x} (found {lo:#x})"));
    }
    let version = package_version(&fw_dir) + "-standalone" + if console { "-console" } else { "" };
    let out_dir = root.join("dist");
    fs::create_dir_all(&out_dir).map_err(|e| e.to_string())?;
    let uf2 = out_dir.join(format!("pico-light-show-{version}.uf2"));
    write_uf2(&image, &uf2)?;
    println!();
    println!("firmware size : {} KiB (standalone, no bootloader, no OTA)", (hi - lo) / 1024);
    println!("USB install   : {}", uf2.display());
    Ok(Dist { uf2, ota: PathBuf::new() })
}

fn find_rp2_drive() -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if cfg!(windows) {
        for letter in b'D'..=b'Z' {
            candidates.push(PathBuf::from(format!("{}:\\", letter as char)));
        }
    } else {
        candidates.push(PathBuf::from("/Volumes/RPI-RP2"));
        if let Ok(user) = env::var("USER") {
            candidates.push(PathBuf::from(format!("/media/{user}/RPI-RP2")));
            candidates.push(PathBuf::from(format!("/run/media/{user}/RPI-RP2")));
        }
    }
    candidates.into_iter().find(|p| {
        fs::read_to_string(p.join("INFO_UF2.TXT")).map(|s| s.contains("RP2")).unwrap_or(false)
    })
}

fn b64(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in data.chunks(3) {
        let n = chunk.iter().enumerate().fold(0u32, |acc, (i, b)| acc | (*b as u32) << (16 - 8 * i));
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(T[((n >> (18 - 6 * i)) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

fn upload_ota(host: &str, password: Option<&str>, image: &Path) -> Result<()> {
    let body = fs::read(image).map_err(|e| e.to_string())?;
    let addr = if host.contains(':') { host.to_string() } else { format!("{host}:80") };
    println!("==> uploading {} bytes to http://{addr}/api/ota", body.len());
    let mut s = TcpStream::connect(&addr).map_err(|e| format!("connect {addr}: {e}"))?;
    s.set_read_timeout(Some(Duration::from_secs(60))).ok();
    let mut req = format!(
        "POST /api/ota HTTP/1.1\r\nHost: {host}\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\nConnection: close\r\n",
        body.len()
    );
    if let Some(p) = password {
        req.push_str(&format!("Authorization: Basic {}\r\n", b64(format!("admin:{p}").as_bytes())));
    }
    req.push_str("\r\n");
    s.write_all(req.as_bytes()).map_err(|e| e.to_string())?;
    for (i, chunk) in body.chunks(4096).enumerate() {
        s.write_all(chunk).map_err(|e| format!("upload failed: {e}"))?;
        if i % 16 == 0 {
            print!("\r{:3}%", (i * 4096 * 100) / body.len().max(1));
            std::io::stdout().flush().ok();
        }
    }
    println!("\r100%");
    let mut resp = String::new();
    s.read_to_string(&mut resp).ok();
    let first = resp.lines().next().unwrap_or("");
    println!("{}", resp.split("\r\n\r\n").nth(1).unwrap_or(first).trim());
    if first.contains(" 200 ") { Ok(()) } else { Err(format!("device answered: {first}")) }
}

fn json_escape(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {}
            c => out.push(c),
        }
    }
    out
}

/// Copy a production build into releases/<version>/ and update releases/latest.json.
fn release(notes: &str) -> Result<()> {
    use sha2::{Digest, Sha256};
    if notes.len() > 230 {
        return Err("release notes too long (max. 230 characters, they are shown in Home Assistant)".into());
    }
    let d = dist(false, false)?;
    let root = root();
    let version = package_version(&root.join("firmware"));
    let rel_dir = root.join("releases");
    let ver_dir = rel_dir.join(&version);
    fs::create_dir_all(&ver_dir).map_err(|e| e.to_string())?;
    let uf2_name = d.uf2.file_name().unwrap().to_string_lossy().to_string();
    let ota_name = d.ota.file_name().unwrap().to_string_lossy().to_string();
    fs::copy(&d.uf2, ver_dir.join(&uf2_name)).map_err(|e| e.to_string())?;
    fs::copy(&d.ota, ver_dir.join(&ota_name)).map_err(|e| e.to_string())?;
    let ota = fs::read(&d.ota).map_err(|e| e.to_string())?;
    let sha: String = Sha256::digest(&ota).iter().map(|b| format!("{b:02x}")).collect();
    let manifest = format!(
        "{{\n  \"version\": \"{version}\",\n  \"ota\": \"{version}/{ota_name}\",\n  \"size\": {},\n  \"sha256\": \"{sha}\",\n  \"notes\": \"{}\"\n}}\n",
        ota.len(),
        json_escape(notes)
    );
    fs::write(rel_dir.join("latest.json"), &manifest).map_err(|e| e.to_string())?;
    println!();
    println!("release       : {}", ver_dir.display());
    println!("manifest      : {}", rel_dir.join("latest.json").display());
    println!("{manifest}");
    println!("Commit and push releases/ - devices pick the update up within a day or on \"Check now\".");
    Ok(())
}

fn run() -> Result<()> {
    let all: Vec<String> = env::args().skip(1).collect();
    let console = all.iter().any(|a| a == "--console");
    let standalone = all.iter().any(|a| a == "--standalone");
    let args: Vec<String> = all.into_iter().filter(|a| !a.starts_with("--")).collect();
    match args.first().map(String::as_str) {
        Some("dist") => dist(console, standalone).map(|_| ()),
        Some("flash") => {
            let d = dist(console, standalone)?;
            let drive = find_rp2_drive()
                .ok_or("no RPI-RP2 drive found - hold BOOTSEL while plugging in the Pico")?;
            let target = drive.join(d.uf2.file_name().unwrap());
            println!("==> copying to {}", target.display());
            fs::copy(&d.uf2, &target).map_err(|e| e.to_string())?;
            println!("done, the Pico reboots automatically");
            Ok(())
        }
        Some("ota") => {
            let host = args.get(1).ok_or("usage: cargo xtask ota <host> [password]")?;
            if standalone {
                return Err("the standalone build has no OTA support".into());
            }
            let d = dist(console, false)?;
            upload_ota(host, args.get(2).map(String::as_str), &d.ota)
        }
        Some("release") => {
            if console || standalone {
                return Err("releases are always production builds (no --console / --standalone)".into());
            }
            release(args.get(1).map(String::as_str).unwrap_or(""))
        }
        _ => {
            eprintln!("usage: cargo xtask <dist | flash | ota <host> [password] | release [\"notes\"]> [--console] [--standalone]");
            exit(2);
        }
    }
}

fn main() {
    if let Err(e) = run() {
        eprintln!("error: {e}");
        exit(1);
    }
}
