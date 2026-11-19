#!/usr/bin/env python3
"""Mock of the device HTTP API for developing the web UI without hardware.

    python tools/mock_server.py [port]      -> http://localhost:8080

Serves ../web and emulates the REST API in memory. There is no WebSocket,
so the UI falls back to polling (the connection dot stays red).
"""
import json
import sys
import time
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

ROOT = Path(__file__).resolve().parent
WEB = ROOT.parent / "web"
META = json.loads((ROOT / "effects.json").read_text())
START = time.time()

DEFAULT_COLORS = {0: ["#ffffff"], 1: ["#ff0000"], 2: ["#ff0000", "#ffff00", "#00ff00", "#00ffff", "#0000ff", "#ff00ff"]}

state = {"on": False, "brightness": 128, "speed": 200, "running": True, "effect": 0,
         "transition": 1, "transition_ms": 800, "colors": ["#ffffff"], "opts": [], "ddp": False}
params = {}
presets = []
config = {
    "name": "", "default_name": "pico-light-show-227a9a", "default_ap_ssid": "PicoLightShow-227A9A", "wifi_mode": "client", "wifi_ssid": "HomeWiFi", "wifi_pass_set": True,
    "dhcp": True, "ip": [192, 168, 1, 50], "netmask": [255, 255, 255, 0], "gateway": [192, 168, 1, 1],
    "dns": [192, 168, 1, 1], "ap_ssid": "", "ap_pass_set": True, "led_count": 60, "led_pin": 14,
    "color_order": "grb", "max_current_ma": 0, "gamma": True, "boot_power": "off", "mqtt_enabled": False,
    "mqtt_host": "", "mqtt_port": 1883, "mqtt_user": "", "mqtt_pass_set": False,
    "mqtt_discovery_prefix": "homeassistant", "mqtt_topic": "", "ddp_enabled": True, "admin_pass_set": False,
}


upd = {"phase": "idle", "since": 0.0}


def update_view():
    t = time.time() - upd["since"]
    if upd["phase"] == "checking" and t > 1.5:
        upd["phase"] = "available"
    if upd["phase"] == "downloading" and t > 6:
        upd["phase"] = "installing"
    known = upd["phase"] not in ("idle", "checking")
    return {"phase": upd["phase"], "installed": "1.0.0", "latest": "1.1.0" if known else "",
            "notes": "Online updates from GitHub and Home Assistant.\nNew effects." if known else "",
            "progress": min(100, int(t / 6 * 100)) if upd["phase"] == "downloading" else 0,
            "error": "", "checked_ago_s": 42 if known else None}


def effect_params(i):
    if i not in params:
        e = META["effects"][i]
        cols = DEFAULT_COLORS.get(i, ["#ff8000", "#0080ff"][: max(e["colors_min"], min(2, e["colors_max"]))])
        params[i] = {"colors": cols[: e["colors_max"]], "opts": [o["min"] for o in e["opts"]]}
    return params[i]


def view():
    p = effect_params(state["effect"])
    return {**state, "colors": p["colors"], "opts": p["opts"]}


def apply(u):
    if "preset" in u and u["preset"] < len(presets):
        p = presets[u["preset"]]
        state.update(effect=p["effect"], brightness=p["brightness"], speed=p["speed"], on=True)
        params[p["effect"]] = json.loads(json.dumps(p["params"]))
    for k in ("on", "brightness", "speed", "running", "effect", "transition", "transition_ms"):
        if k in u:
            state[k] = u[k]
    if u.get("toggle"):
        state["on"] = not state["on"]
    p = effect_params(state["effect"])
    if "colors" in u:
        p["colors"] = u["colors"]
    if "opts" in u:
        p["opts"] = u["opts"]


class Handler(SimpleHTTPRequestHandler):
    def __init__(self, *a, **kw):
        super().__init__(*a, directory=str(WEB), **kw)

    def log_message(self, fmt, *args):
        if "/api/" in (args[0] if args else ""):
            sys.stderr.write("%s\n" % (fmt % args))

    def json(self, obj, code=200):
        body = json.dumps(obj).encode()
        self.send_response(code)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def body(self):
        n = int(self.headers.get("Content-Length") or 0)
        raw = self.rfile.read(n) if n else b""
        return raw

    def do_GET(self):
        p = self.path.split("?")[0]
        if p == "/api/state":
            return self.json(view())
        if p == "/api/effects":
            return self.json(META)
        if p == "/api/info":
            return self.json({"name": config["name"] or config["default_name"], "version": "1.0.0-mock", "uptime_s": int(time.time() - START),
                              "net": {"mode": "client", "ssid": config["wifi_ssid"], "ip": [127, 0, 0, 1],
                                      "gateway": [0, 0, 0, 0], "mac": [0x28, 0xcd, 0xc1, 1, 2, 3], "rssi": -58},
                              "hostname": config["name"] or config["default_name"], "mqtt": "disabled", "ddp_active": False,
                              "boot": "normal", "last_panic": None, "led_count": config["led_count"],
                              "auth": config["admin_pass_set"], "fps": 50, "cpu_mhz": 125, "chip_temp_c": 27.4,
                              "reset_reason": "power-on / reboot", "ram_static": 150 * 1024, "ram_total": 264 * 1024,
                              "stack_free": [56000, 14700], "stack_size": [87568, 16384],
                              "flash_used": 701 * 1024, "flash_total": 1000 * 1024,
                              "render_load": 84, "current_ma": 412, "ws_clients": 1})
        if p == "/api/config":
            return self.json(config)
        if p == "/api/update":
            return self.json(update_view())
        if p == "/api/presets":
            return self.json(presets)
        if p == "/api/wifi/scan":
            time.sleep(1.5)
            return self.json([{"ssid": "HomeWiFi", "rssi": -48, "secure": True},
                              {"ssid": "Neighbour", "rssi": -80, "secure": True},
                              {"ssid": "Cafe", "rssi": -86, "secure": False}])
        if p == "/ws":
            return self.json({"error": "no websocket in mock"}, 400)
        return super().do_GET()

    def do_POST(self):
        p = self.path.split("?")[0]
        raw = self.body()
        data = json.loads(raw or b"{}") if p != "/api/ota" else None
        if p == "/api/state":
            apply(data)
            return self.json(view())
        if p == "/api/presets":
            name = data.get("name", "")
            if not name:
                return self.json({"ok": False, "message": "name required"}, 400)
            entry = {"name": name, "effect": data.get("effect", state["effect"]),
                     "brightness": data.get("brightness", state["brightness"]),
                     "speed": data.get("speed", state["speed"]),
                     "params": data.get("params", effect_params(state["effect"]))}
            for i, x in enumerate(presets):
                if x["name"] == name:
                    presets[i] = entry
                    break
            else:
                if len(presets) >= 8:
                    return self.json({"ok": False, "message": "too many presets (max 8)"}, 400)
                presets.append(entry)
            return self.json(presets)
        if p == "/api/presets/delete":
            i = data.get("index", 99)
            if i < len(presets):
                presets.pop(i)
            return self.json(presets)
        if p == "/api/config":
            for k, v in data.items():
                if k.endswith("_pass"):
                    config[k + "_set"] = bool(v)
                elif k in config:
                    config[k] = v
            return self.json({"ok": True, "reboot_required": True})
        if p == "/api/update":
            upd["phase"] = "checking" if data.get("action") == "check" else "downloading"
            upd["since"] = time.time()
            return self.json(update_view())
        if p in ("/api/reboot", "/api/factory-reset"):
            return self.json({"ok": True})
        if p == "/api/ota":
            time.sleep(1)
            if raw[:4] == b"UF2\n":
                return self.json({"ok": False}, 400)
            self.send_response(200)
            self.end_headers()
            self.wfile.write(b"OK, rebooting into the new firmware\n")
            return
        self.json({"error": "not found"}, 404)


if __name__ == "__main__":
    port = int(sys.argv[1]) if len(sys.argv) > 1 else 8080
    print(f"Mock device on http://localhost:{port}")
    ThreadingHTTPServer(("127.0.0.1", port), Handler).serve_forever()
