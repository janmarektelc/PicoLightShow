# <img src="web/favicon.svg" width="30"> Pico Light Show

Firmware for the **Raspberry Pi Pico W** that drives WS2812 / NeoPixel LED strips, written in Rust on top
of [Embassy](https://embassy.dev).

- **Web UI**: works on phone and desktop, dark and light theme, live LED preview, presets, backup and restore
- **18 effects**: Solid color, Running point, Running colors, Snakes, Color change, Breath, Rainbow,
  Theater chase, Comet, Scanner, Fire, Twinkle, Gradient, Plasma, Candle, Sparkle and more
- **8 on/off transitions** and a smooth cross-fade when switching effects
- **MQTT** with Home Assistant auto-discovery (light entity with effects, plus a speed slider)
- **DDP** (UDP 4048) for OpenRGB, xLights and other real-time senders
- **Online updates**: the device checks GitHub for new firmware daily; install with one click in the web UI
  or from Home Assistant (*Settings → Updates*), with automatic rollback if the new firmware does not start
- Wi-Fi client or access point. If the client connection fails, the device falls back to its own access point
  and keeps retrying in the background. Includes a captive portal and `http://<name>.local` (mDNS).
- Gamma correction, configurable color order and data pin, power (current) limiter
- Settings are stored in flash with wear-levelling and survive firmware updates
- System page with chip temperature, estimated LED current, render load, memory usage and the reason of the last restart

## Download

Ready-made images are in [`releases/<version>/`](releases/); the newest version is named in
[`releases/latest.json`](releases/latest.json).

- `pico-light-show-<version>.uf2` - bootloader and firmware, for installing over USB
- `pico-light-show-<version>-ota.bin` - firmware only, for updates over Wi-Fi

## Hardware

| Strip | Pico W |
|---|---|
| Data (DIN) | **GP14** (default, any free GPIO can be chosen under Settings → LED strip) |
| GND | GND (shared with the power supply) |
| +5 V | 5 V power supply sized for the strip (about 60 mA per LED at full white) |

- Do not power longer strips from the Pico's VBUS pin; connect the supply directly to the strip and join
  the grounds. Set the power limit in Settings to protect the supply.
- A 300-500 Ω resistor in the data line and a 1000 µF capacitor across the strip's supply are recommended.
- Most WS2812 strips accept the 3.3 V data signal over short wires; for long wires use a level shifter
  (e.g. 74AHCT125).

## Installing

1. Download the images (see above) or build them yourself (see below):
   - `pico-light-show-<ver>.uf2` contains the bootloader and the firmware. Use it for the first install over USB.
   - `pico-light-show-<ver>-ota.bin` is the firmware only. Use it for updates over Wi-Fi.
2. Hold **BOOTSEL**, plug in the Pico and copy the `.uf2` file to the `RPI-RP2` drive.
3. Connect to the Wi-Fi network **PicoLightShow-XXXXXX** (the end of the device's MAC address, password `picolight`). The setup page opens by itself
   (captive portal); otherwise go to <http://192.168.4.1>.
4. Under **Settings → Wi-Fi** choose *Connect to network*, pick your network, save and reboot.
   The device is then reachable at `http://pico-light-show-xxxxxx.local` (or at its IP address).
   The automatic name is unique per device; set your own under Settings → Device. If two devices
   claim the same name, the later one adds its MAC suffix (mDNS conflict detection).
   `.local` (mDNS) works within one network. If the Pico sits in a separate IoT network/VLAN,
   use the name without `.local` (most routers register DHCP host names in their DNS) or the IP.

### Updating

- **Online:** after start-up and then once a day the device reads [`releases/latest.json`](releases/latest.json)
  from GitHub. A new version is shown on the System page (with a dot on the *System* tab) and in Home
  Assistant under *Settings → Updates*. **Install update** (web) or **Install** (Home Assistant) downloads
  the image over HTTPS straight into the second flash slot, checks its size and SHA-256 and reboots.
  Nothing is installed automatically. The daily check and the update source can be changed under
  Settings → Firmware updates (e.g. a manifest on your own server, `http://` works too).
- **Manual upload:** System → Firmware update → Manual upload → choose `*-ota.bin`.
- From the command line: `cargo xtask ota pico-light-show-xxxxxx.local [admin-password]`.
- Over USB: copy the `.uf2` again. Settings are kept.

In all cases the new firmware is confirmed once Wi-Fi and the web server are up. If it crashes or hangs
before that, the watchdog resets the device and the bootloader restores the previous version.

The download is not authenticated beyond the checksum in the manifest (the TLS certificate is not
verified), so it only protects against broken downloads, not against a manipulated network.

### Publishing a release

1. Raise `version` in `firmware/Cargo.toml`.
2. `cargo xtask release "Short release notes (shown in the web UI and Home Assistant)"` builds the
   production images, copies them to `releases/<version>/` and rewrites `releases/latest.json`:

   ```json
   {
     "version": "1.1.0",
     "ota": "1.1.0/pico-light-show-1.1.0-ota.bin",
     "size": 851968,
     "sha256": "…",
     "notes": "Short release notes"
   }
   ```

3. Commit and push. Devices see the new version within a day, or immediately after *Check now*.

## Building

Requirements: Rust (stable) with the target `rustup target add thumbv6m-none-eabi`.

```bash
cargo xtask dist
```

```bash
cargo xtask flash
```

```bash
cargo test -p host-tests
```

- `cargo xtask dist` builds the bootloader and the firmware (production, no USB console) and writes the two images to `dist/`. Add `--console` for a debug build.
- `cargo xtask flash` runs `dist`, then copies the `.uf2` to a Pico in BOOTSEL mode.
- `cargo test -p host-tests` runs the effects, transitions and colors on the PC.

With a debug probe you can also flash and debug directly: flash the bootloader once
(`cd bootloader && cargo run --release`), then `cd firmware && cargo run --release`.

UI development without hardware: `python tools/mock_server.py` serves `web/` with a mock API on
<http://localhost:8080>. The web files are gzipped and embedded into the firmware at build time.

### Serial console (debugging)

Production images do not use USB at all. For debugging build an image with logs on the USB serial port:

```bash
cargo xtask flash --console
```

In VS Code run the task **Flash (console build)** (`Ctrl+Shift+B`), then open the **Serial Monitor**
panel (extension *ms-vscode.vscode-serial-monitor*), pick the Pico's COM port and press *Start Monitoring*
(the baud rate does not matter). The first 4 KiB of boot messages are kept until the port is opened.
After a reset the port disappears for a moment; start monitoring again.

Console builds start USB first and wait 1.5 s, so the port appears even if the firmware crashes later.
If the previous run crashed (panic or HardFault, with the code address) or hung (watchdog reset), the next
boot prints the reason and the last reached boot stage every 2 s for a minute before it continues.

To rule out the bootloader, `cargo xtask flash --standalone --console` builds the firmware without
bootloader and OTA (it starts directly at `0x10000100`); settings are shared with the normal build.

The on-board LED shows the state without a console: **on** = connecting to the Wi-Fi network,
**off** = connected (or a phone is connected to the access point), **blinking** = access point waiting for
a client, **off right after power-up** = the firmware did not reach the Wi-Fi start.

## Home Assistant (MQTT)

Enable MQTT in Settings and enter the broker address (IP or host name). The device appears automatically
as a light with brightness, RGB color and an effect list, a *Speed* number entity and a *Firmware* update
entity (Settings → Updates in Home Assistant, with Install button and progress).

| Topic | Meaning |
|---|---|
| `picolightshow/<mac>/state` | JSON state (retained) |
| `picolightshow/<mac>/set` | commands, e.g. `{"state":"ON","brightness":120,"effect":"Fire","color":{"r":255,"g":0,"b":0}}` |
| `picolightshow/<mac>/speed/set` | speed 1–255 |
| `picolightshow/<mac>/status` | `online` / `offline` (last will) |
| `picolightshow/<mac>/update` | firmware update state for the HA `update` entity (retained) |
| `picolightshow/<mac>/update/install` | `install` starts the online update |

Setting a color changes the first color of the current effect. Effects without colors (Rainbow, Fire, …)
switch to *Solid color*.

## DDP / OpenRGB

In OpenRGB add a DDP device (Settings → DDP Devices) with the Pico's IP, port 4048 and the number of LEDs. While packets
arrive they replace the running effect; 3 s after the last packet the effect continues. Brightness and the
power limit apply to DDP too.

## HTTP API

| Method | Path | Description |
|---|---|---|
| GET/POST | `/api/state` | light state; POST accepts any subset of `on, toggle, brightness, speed, running, effect, transition, transition_ms, colors, opts, preset` |
| GET | `/api/effects` | effect metadata (names, colors, options) and transition names |
| GET | `/api/info` | version, uptime, network, MQTT status |
| GET/POST | `/api/presets` | list / save (`{"name":"…"}` stores the current look) |
| POST | `/api/presets/delete` | `{"index":n}` |
| GET/POST 🔒 | `/api/config` | settings (passwords are never returned; missing fields stay unchanged) |
| GET 🔒 | `/api/wifi/scan` | nearby networks |
| POST 🔒 | `/api/reboot`, `/api/factory-reset`, `/api/ota` | |
| GET / POST 🔒 | `/api/update` | online update status; POST `{"action":"check"}` or `{"action":"install"}` |
| WS | `/ws` | pushes state JSON on every change; accepts the same JSON as POST `/api/state`; `preview:on` starts binary preview frames |

🔒 = protected by the admin password (HTTP basic auth) when one is set.

Example: `curl -X POST http://pico-light-show-xxxxxx.local/api/state -d '{"on":true,"effect":12}'`

## Architecture

```
core 0 (Embassy executor)                     core 1 (Embassy executor)
├─ Wi-Fi manager (CYW43, AP fallback)         └─ render loop, 50 fps
├─ HTTP server ×5 workers + WebSocket            ├─ effect step/render
├─ MQTT client (HA discovery)                    ├─ transitions / cross-fade
├─ DDP receiver ─────── frame buffer ──────────► ├─ gamma, brightness, power limit
├─ DHCP server + captive DNS (AP mode)           └─ WS2812 via PIO1 + DMA
├─ mDNS responder
├─ settings saver (flash, debounced)          shared state: one mutex-protected
├─ OTA writer / online updates (HTTPS)        struct + version counter
├─ boot confirmation
└─ watchdog (fed only while core 1 renders)
```

Flash layout (2 MiB):

| Address | Size | Content |
|---|---|---|
| `0x10000000` | 24 KiB | bootloader (embassy-boot) |
| `0x10006000` | 4 KiB | bootloader state |
| `0x10007000` | 1000 KiB | active firmware |
| `0x10101000` | 1004 KiB | update slot (DFU) |
| `0x101FC000` | 16 KiB | settings |

## Upgrading from the C++ firmware (0.9.x)

The previous C++ (Pico SDK) firmware is kept in the [`legacy-cpp`](https://github.com/janmarektelc/PicoLightShow/tree/legacy-cpp) branch
(last release tagged `v0.9.5-cpp`).

- Install version 1.0 once over USB (`.uf2`); later updates work over Wi-Fi.
- Settings are not carried over: the device starts as access point `PicoLightShow-XXXXXX` again.
- The default data pin is **GP14** (was GP2); change it under Settings → LED strip if needed.
- MQTT keeps the Home Assistant device and light entity (same unique id), so automations keep working.
  Effect names changed, so automations that select an effect by name need an update.
- The WLED JSON API is not implemented; use the MQTT integration in Home Assistant instead.
- RGBW strips are not supported yet.

## License

MIT or Apache-2.0. The CYW43 Wi-Fi firmware in `firmware/cyw43-firmware` is distributed under its own
permissive binary license (see the file in that folder).
