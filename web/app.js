'use strict';
// Pico Light Show web UI - plain JavaScript, no dependencies.

const $ = (s, el = document) => el.querySelector(s);
const $$ = (s, el = document) => [...el.querySelectorAll(s)];
const clean = arr => arr.flat().filter(x => x !== null && x !== undefined && x !== false);
const h = (tag, attrs = {}, ...kids) => {
  const e = document.createElement(tag);
  for (const [k, v] of Object.entries(attrs)) {
    if (k.startsWith('on')) e.addEventListener(k.slice(2), v);
    else if (k === 'class') e.className = v;
    else if (v !== undefined && v !== null && v !== false) e.setAttribute(k, v === true ? '' : v);
  }
  for (const kid of kids.flat()) if (kid !== null && kid !== undefined) e.append(kid);
  return e;
};

const PALETTE = ['#ff0000', '#ff7a00', '#ffd000', '#00ff40', '#00ffd0', '#0080ff', '#2000ff', '#a000ff', '#ff00a0', '#ffffff', '#ffb070', '#000000'];
const LED_PINS = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 26, 27, 28];

let meta = { effects: [], transitions: [] };
let st = null;        // last known light state
let ws = null;
let wsOk = false;
let editing = 0;      // timestamp of last local edit (ignore echoes briefly)

function toast(msg, err = false) {
  const t = h('div', { class: 'toast' + (err ? ' err' : '') }, msg);
  document.body.append(t);
  setTimeout(() => t.remove(), 2600);
}

async function api(path, opts = {}) {
  const r = await fetch(path, {
    method: opts.method || (opts.body !== undefined ? 'POST' : 'GET'),
    headers: opts.body !== undefined && !(opts.body instanceof Blob) ? { 'Content-Type': 'application/json' } : {},
    body: opts.body === undefined ? undefined : (opts.body instanceof Blob ? opts.body : JSON.stringify(opts.body)),
  });
  const text = await r.text();
  let data = null;
  try { data = text ? JSON.parse(text) : null; } catch { data = text; }
  if (!r.ok) throw new Error((data && data.message) || text || r.statusText);
  return data;
}

// ---------------------------------------------------------------- navigation
$$('nav button').forEach(b => b.addEventListener('click', () => showPage(b.dataset.page)));
function showPage(p) {
  $$('nav button').forEach(b => b.classList.toggle('active', b.dataset.page === p));
  $$('.page').forEach(s => s.classList.toggle('active', s.id === 'page-' + p));
  if (p === 'settings') loadConfig();
  if (p === 'system') { loadInfo(); loadUpdate(); }
  if (p === 'presets') loadPresets();
  if (location.hash.slice(1) !== p) history.replaceState(null, '', '#' + p);
}
window.addEventListener('hashchange', () => {
  const p = location.hash.slice(1);
  if (['light', 'presets', 'settings', 'system'].includes(p)) showPage(p);
});

const themeKey = 'pls-theme';
function applyTheme(t) {
  document.documentElement.classList.toggle('light', t === 'light');
  document.documentElement.classList.toggle('dark', t === 'dark');
}
applyTheme(localStorage.getItem(themeKey));
$('#themeBtn').onclick = () => {
  const cur = document.documentElement.classList.contains('light') ? 'light'
    : document.documentElement.classList.contains('dark') ? 'dark'
    : (matchMedia('(prefers-color-scheme: light)').matches ? 'light' : 'dark');
  const next = cur === 'light' ? 'dark' : 'light';
  localStorage.setItem(themeKey, next);
  applyTheme(next);
};

// ---------------------------------------------------------------- state sync
let sendTimer = null;
let pending = {};
function send(update, immediate = false) {
  editing = Date.now();
  Object.assign(pending, update);
  if (sendTimer) return;
  const flush = () => {
    sendTimer = null;
    const u = pending; pending = {};
    if (wsOk) ws.send(JSON.stringify(u));
    else api('/api/state', { body: u }).then(applyState).catch(e => toast(e.message, true));
  };
  if (immediate) flush(); else sendTimer = setTimeout(flush, 70);
}

function connectWs() {
  try { ws = new WebSocket(`ws://${location.host}/ws`); } catch { return setTimeout(connectWs, 3000); }
  ws.binaryType = 'arraybuffer';
  ws.onopen = () => {
    wsOk = true; $('#wsdot').classList.add('on');
    if ($('#previewOn').checked) ws.send('preview:on');
  };
  ws.onclose = () => { wsOk = false; $('#wsdot').classList.remove('on'); setTimeout(connectWs, 2500); };
  ws.onerror = () => ws.close();
  ws.onmessage = ev => {
    if (typeof ev.data === 'string') { try { applyState(JSON.parse(ev.data)); } catch {} }
    else drawPreview(new Uint8Array(ev.data));
  };
}

// Fallback polling while the WebSocket is down.
setInterval(() => { if (!wsOk && document.visibilityState === 'visible') api('/api/state').then(applyState).catch(() => {}); }, 4000);

function applyState(s) {
  if (!s || typeof s !== 'object') return;
  const effectChanged = !st || st.effect !== s.effect;
  const recentEdit = Date.now() - editing < 600;
  st = s;
  $('#power').classList.toggle('on', s.on);
  if (!recentEdit) {
    setSlider('brightness', s.brightness);
    setSlider('speed', s.speed);
  }
  $('#running').checked = s.running;
  $('#ddpBanner').classList.toggle('hidden', !s.ddp);
  $('#transition').value = s.transition;
  if (document.activeElement !== $('#transitionMs')) $('#transitionMs').value = s.transition_ms;
  $$('#effects button').forEach((b, i) => b.classList.toggle('active', i === s.effect));
  if (effectChanged || !recentEdit) renderEffectSettings();
  const c = (s.colors && s.colors[0]) || '#ff8a3d';
  if (c !== '#000000') document.documentElement.style.setProperty('--accent', c);
}

$('#effectReset').onclick = async () => {
  try {
    const s = await api('/api/state', { body: { reset: true } });
    editing = 0;
    applyState(s);
    renderEffectSettings();
    toast(`${meta.effects[s.effect].name}: default settings restored`);
  } catch (e) { toast(e.message, true); }
};

function setSlider(id, v) {
  $('#' + id).value = v;
  $('#' + id + 'Out').textContent = Math.round(v * 100 / 255) + '%';
}

// ---------------------------------------------------------------- light page
$('#power').onclick = () => send({ on: !(st && st.on) }, true);
for (const id of ['brightness', 'speed']) {
  $('#' + id).addEventListener('input', e => {
    setSlider(id, +e.target.value);
    send({ [id]: +e.target.value });
  });
}
$('#running').onchange = e => send({ running: e.target.checked }, true);
$('#transition').onchange = e => send({ transition: +e.target.value }, true);
$('#transitionMs').onchange = e => send({ transition_ms: Math.max(0, Math.min(10000, +e.target.value)) }, true);
$('#previewOn').onchange = e => {
  $('#preview').classList.toggle('hidden', !e.target.checked);
  if (wsOk) ws.send(e.target.checked ? 'preview:on' : 'preview:off');
};

function drawPreview(buf) {
  const cv = $('#preview');
  if (cv.classList.contains('hidden') || buf.length < 2) return;
  const n = (buf[0] << 8) | buf[1];
  const w = cv.clientWidth || 600;
  if (cv.width !== w) cv.width = w;
  const ctx = cv.getContext('2d');
  ctx.fillStyle = '#000';
  ctx.fillRect(0, 0, cv.width, cv.height);
  const px = cv.width / Math.max(n, 1);
  for (let i = 0; i < n; i++) {
    const r = buf[2 + i * 3], g = buf[3 + i * 3], b = buf[4 + i * 3];
    ctx.fillStyle = `rgb(${r},${g},${b})`;
    ctx.beginPath();
    const cx = i * px + px / 2, rad = Math.max(1.5, Math.min(px * 0.42, 11));
    ctx.arc(cx, cv.height / 2, rad, 0, 7);
    ctx.fill();
  }
}

function renderEffects() {
  const box = $('#effects');
  box.replaceChildren(...meta.effects.map((e, i) => h('button', { onclick: () => send({ effect: i, on: true }, true) }, e.name)));
  $('#transition').replaceChildren(...meta.transitions.map((t, i) => h('option', { value: i }, t)));
}

function renderEffectSettings() {
  if (!st || !meta.effects[st.effect]) return;
  const e = meta.effects[st.effect];
  $('#effectTitle').textContent = e.name;
  const colors = [...(st.colors || [])];

  $('#colorsBox').classList.toggle('hidden', e.colors_max === 0);
  const sendColors = (c) => { st.colors = c; send({ colors: c }); };
  $('#colors').replaceChildren(...clean([
    ...colors.map((c, i) => h('div', { class: 'swatch', style: `background:${c}` },
      h('input', { type: 'color', value: c, oninput: ev => {
        colors[i] = ev.target.value; ev.target.parentNode.style.background = ev.target.value; sendColors([...colors]);
      } }),
      colors.length > e.colors_min ? h('button', { class: 'x', title: 'Remove', onclick: () => { colors.splice(i, 1); sendColors(colors); renderEffectSettings(); } }, '×') : null)),
    colors.length < e.colors_max ? h('button', { class: 'add', title: 'Add color', onclick: () => {
      colors.push(colors[colors.length - 1] || '#ffffff'); sendColors(colors); renderEffectSettings();
    } }, '+') : null,
  ]));
  $('#palette').replaceChildren(...(e.colors_max ? PALETTE : []).map(c => h('button', {
    style: `background:${c}`, title: c, onclick: () => {
      const c2 = colors.length ? [...colors] : [c];
      c2[c2.length === 1 ? 0 : c2.length - 1] = c;
      sendColors(c2); renderEffectSettings();
    },
  })));

  const opts = st.opts || [];
  $('#opts').replaceChildren(...e.opts.map((o, i) => {
    const set = v => { const a = [...opts]; a[i] = v; st.opts = a; send({ opts: a }); };
    let input;
    if (o.kind === 'toggle') {
      input = h('label', { class: 'switch' }, h('input', { type: 'checkbox', checked: opts[i] ? true : null, onchange: ev => set(ev.target.checked ? 1 : 0) }), h('span'));
    } else if (o.kind === 'select') {
      input = h('select', { onchange: ev => set(+ev.target.value) }, ...o.choices.map((c, j) => h('option', { value: j, selected: j === opts[i] ? true : null }, c)));
    } else {
      const out = h('output', {}, String(opts[i]));
      input = h('div', { class: 'slider-row' }, h('input', { type: 'range', min: o.min, max: o.max, value: opts[i], oninput: ev => { out.textContent = ev.target.value; set(+ev.target.value); } }), out);
    }
    return h('div', { class: 'row' }, h('span', { class: 'lbl' }, o.label), h('div', { class: 'val' }, input));
  }));
  $('#effectCard').classList.toggle('hidden', e.colors_max === 0 && e.opts.length === 0);
}

// ---------------------------------------------------------------- presets
async function loadPresets() {
  try { renderPresets(await api('/api/presets')); } catch (e) { toast(e.message, true); }
}
function renderPresets(list) {
  const box = $('#presetList');
  if (!list || !list.length) return box.replaceChildren(h('p', { class: 'muted' }, 'No presets yet.'));
  box.replaceChildren(...list.map((p, i) => h('div', { class: 'item' },
    h('div', { class: 'colors' }, ...(p.params.colors.length ? p.params.colors : ['#444']).slice(0, 4).map(c => h('span', { class: 'dot', style: `background:${c};width:14px;height:14px` }))),
    h('div', { class: 'grow' }, h('b', {}, p.name), h('div', { class: 'muted' }, (meta.effects[p.effect] || {}).name || '')),
    h('button', { class: 'btn small primary', onclick: () => send({ preset: i }, true) }, 'Apply'),
    h('button', { class: 'btn small danger', onclick: async () => {
      if (!confirm(`Delete preset "${p.name}"?`)) return;
      renderPresets(await api('/api/presets/delete', { body: { index: i } }));
    } }, 'Delete'))));
}
$('#presetSave').onclick = async () => {
  const name = $('#presetName').value.trim();
  if (!name) return toast('Enter a name', true);
  try { renderPresets(await api('/api/presets', { body: { name } })); $('#presetName').value = ''; toast('Preset saved'); }
  catch (e) { toast(e.message, true); }
};

// ---------------------------------------------------------------- settings
let cfg = null;
function ipInputs() {
  $$('[data-ip]').forEach(box => {
    if (box.children.length) return;
    box.append(h('input', { type: 'text', inputmode: 'decimal', autocomplete: 'off', spellcheck: 'false', placeholder: '0.0.0.0' }));
  });
}
function parseIp(text) {
  const m = text.trim().match(/^(\d{1,3})\.(\d{1,3})\.(\d{1,3})\.(\d{1,3})$/);
  if (!m) return null;
  const v = m.slice(1).map(Number);
  return v.every(x => x <= 255) ? v : null;
}
function updateVisibility() {
  const f = $('#cfgForm');
  $('[data-show=client]').classList.toggle('hidden', f.wifi_mode.value !== 'client');
  $('[data-show=static]').classList.toggle('hidden', f.dhcp.checked);
  $('[data-show=mqtt]').classList.toggle('hidden', !f.mqtt_enabled.checked);
}
async function loadConfig() {
  ipInputs();
  const f = $('#cfgForm');
  if (!f.led_pin.options.length) f.led_pin.replaceChildren(...LED_PINS.map(p => h('option', { value: p }, 'GP' + p)));
  try { cfg = await api('/api/config'); } catch (e) { return toast('Cannot load settings: ' + e.message, true); }
  for (const el of f.elements) {
    if (!el.name || !(el.name in cfg)) continue;
    if (el.type === 'checkbox') el.checked = !!cfg[el.name];
    else el.value = cfg[el.name];
  }
  for (const p of ['wifi_pass', 'ap_pass', 'mqtt_pass', 'admin_pass']) {
    f[p].value = '';
    f[p].placeholder = cfg[p + '_set'] ? 'unchanged (set)' : 'not set';
  }
  $('#clearAdmin').checked = false;
  $$('[data-ip]').forEach(box => {
    $('input', box).value = (cfg[box.dataset.ip] || [0, 0, 0, 0]).join('.');
    $('input', box).classList.remove('invalid');
  });
  f.name.placeholder = 'automatic: ' + cfg.default_name;
  f.ap_ssid.placeholder = 'automatic: ' + cfg.default_ap_ssid;
  api('/api/info').then(i => {
    $('#hostHint').textContent = `Reachable as http://${i.hostname}.local on the same network, via the router's DNS as http://${i.hostname}/, or at http://${i.net.ip.join('.')}/. Leave the name empty for the automatic, unique name.`;
  }).catch(() => {});
  updateVisibility();
}
function hostname(n) { return (n || '').toLowerCase().replace(/[^a-z0-9]/g, '-').replace(/-+/g, '-').replace(/^-|-$/g, '') || 'pico-light-show'; }
$('#cfgForm').addEventListener('change', updateVisibility);
$('#cfgForm').addEventListener('submit', async ev => {
  ev.preventDefault();
  const f = ev.target;
  const out = {};
  for (const el of f.elements) {
    if (!el.name) continue;
    if (el.type === 'checkbox') out[el.name] = el.checked;
    else if (el.type === 'number' || el.name === 'led_pin') out[el.name] = +el.value;
    else if (el.type === 'password') { if (el.value) out[el.name] = el.value; }
    else out[el.name] = el.value;
  }
  if ($('#clearAdmin').checked) out.admin_pass = '';
  if (out.ap_pass && out.ap_pass.length < 8) return toast('Access point password needs at least 8 characters', true);
  for (const box of $$('[data-ip]')) {
    const inp = $('input', box), v = parseIp(inp.value);
    inp.classList.toggle('invalid', !v);
    if (!v) {
      if (!f.dhcp.checked) { inp.focus(); return toast(`Invalid address: "${inp.value}"`, true); }
      continue; // hidden while DHCP is on: keep the stored value
    }
    out[box.dataset.ip] = v;
  }
  try {
    const r = await api('/api/config', { body: out });
    toast('Settings saved');
    $('#rebootBanner').classList.toggle('hidden', !r.reboot_required);
    loadConfig();
  } catch (e) { toast(e.message, true); }
});
$('#scanBtn').onclick = async () => {
  const box = $('#scanList');
  box.classList.remove('hidden');
  box.replaceChildren(h('p', { class: 'muted' }, 'Scanning…'));
  try {
    const list = await api('/api/wifi/scan');
    box.replaceChildren(...(list.length ? list.map(n => h('button', { type: 'button', onclick: () => {
      $('#cfgForm').wifi_ssid.value = n.ssid; box.classList.add('hidden');
    } }, h('span', {}, (n.secure ? '🔒 ' : '') + n.ssid), h('span', { class: 'muted' }, n.rssi + ' dBm'))) : [h('p', { class: 'muted' }, 'No networks found.')]));
  } catch (e) { box.replaceChildren(h('p', { class: 'muted' }, 'Scan failed: ' + e.message)); }
};
async function reboot() {
  try { await api('/api/reboot', { body: {} }); toast('Rebooting…'); setTimeout(() => location.reload(), 9000); }
  catch (e) { toast(e.message, true); }
}
$('#rebootNow').onclick = reboot;

// ---------------------------------------------------------------- system
function fmtUptime(s) {
  const d = Math.floor(s / 86400), hh = Math.floor(s % 86400 / 3600), m = Math.floor(s % 3600 / 60);
  return (d ? d + 'd ' : '') + hh + 'h ' + m + 'm';
}
const MODES = { starting: 'starting', connecting: 'connecting…', client: 'connected', ap: 'access point', ap_fallback: 'fallback access point' };
function signal(rssi) {
  return rssi >= -55 ? 'excellent' : rssi >= -67 ? 'good' : rssi >= -75 ? 'fair' : 'weak';
}
async function loadInfo() {
  try {
    const i = await api('/api/info');
    $('#title').textContent = i.name;
    document.title = i.name;
    const ip = i.net.ip.join('.');
    const mac = i.net.mac.map(b => b.toString(16).padStart(2, '0')).join(':');
    const kib = b => (b / 1024).toFixed(b < 10240 ? 1 : 0) + ' KiB';
    const pct = (a, b) => b ? ` (${Math.round(a * 100 / b)} %)` : '';
    const stack = c => i.stack_size[c] ? `${kib(i.stack_size[c] - i.stack_free[c])} of ${kib(i.stack_size[c])}` : '…';
    const groups = [
      ['Device', [
        ['Firmware', i.version + (i.boot === 'updated' ? ' (just updated)' : i.boot === 'rolled_back' ? ' (update failed, rolled back)' : '')],
        ['Uptime', fmtUptime(i.uptime_s)],
        ['Last restart', i.reset_reason],
        ['Password', i.auth ? 'set' : 'not set'],
      ]],
      ['Network', [
        ['Wi-Fi', `${MODES[i.net.mode] || i.net.mode} – ${i.net.ssid}`],
        ['Signal', i.net.rssi ? `${i.net.rssi} dBm – ${signal(i.net.rssi)}` : '–'],
        ['IP address', ip], ['Gateway', i.net.gateway.join('.')],
        ['Host name', i.hostname], ['MAC', mac],
        ['MQTT', i.mqtt], ['DDP stream', i.ddp_active ? 'active' : 'idle'],
        ['Open web pages', i.ws_clients],
      ]],
      ['LED output', [
        ['LEDs', `${i.led_count} @ ${i.fps} fps`],
        ['Est. current', i.current_ma >= 1000 ? (i.current_ma / 1000).toFixed(2) + ' A' : i.current_ma + ' mA'],
        ['Render load', (i.render_load / 10).toFixed(1) + ' % of core 1'],
      ]],
      ['Hardware', [
        ['Chip', `RP2040 @ ${i.cpu_mhz} MHz`],
        ['Chip temperature', i.chip_temp_c == null ? '…' : i.chip_temp_c.toFixed(1) + ' °C'],
        ['RAM (static)', `${kib(i.ram_static)} of ${kib(i.ram_total)}` + pct(i.ram_static, i.ram_total)],
        ['Stack peak core 0', stack(0)],
        ['Stack peak core 1', stack(1)],
        ['Firmware size', `${kib(i.flash_used)} of ${kib(i.flash_total)}` + pct(i.flash_used, i.flash_total)],
      ]],
    ];
    $('#info').replaceChildren(...groups.flatMap(([title, rows]) => [
      h('div', { class: 'kv-head' }, title),
      ...rows.flatMap(([k, v]) => [h('div', { class: 'k' }, k), h('div', {}, String(v))]),
    ]));
    $('#panicBanner').classList.toggle('hidden', !i.last_panic);
    $('#panicBanner').textContent = i.last_panic ? 'The device restarted after an error: ' + i.last_panic : '';
  } catch (e) { toast(e.message, true); }
}
setInterval(() => { if ($('#page-system').classList.contains('active')) loadInfo(); }, 5000);
setInterval(loadUpdate, 10 * 60 * 1000);

// ---------------------------------------------------------------- online update
let updTimer = null;
function agoText(s) {
  if (s == null) return '';
  if (s < 90) return 'just now';
  if (s < 5400) return Math.round(s / 60) + ' min ago';
  if (s < 129600) return Math.round(s / 3600) + ' h ago';
  return Math.round(s / 86400) + ' days ago';
}
function renderUpdate(u) {
  $('#updInstalled').textContent = u.installed;
  $('#updLatest').textContent = u.latest || '–';
  const busy = u.phase === 'checking' || u.phase === 'downloading' || u.phase === 'installing';
  const texts = {
    idle: 'Not checked yet.',
    checking: 'Checking for updates…',
    up_to_date: 'The firmware is up to date' + (u.checked_ago_s != null ? ` (checked ${agoText(u.checked_ago_s)}).` : '.'),
    available: `Version ${u.latest} is available.`,
    downloading: `Downloading and writing the update… ${u.progress} %`,
    installing: 'Update stored – the device restarts into the new firmware. This page reloads in 25 s.',
    error: 'Update check failed: ' + u.error,
  };
  $('#updState').textContent = texts[u.phase] || u.phase;
  $('#updState').classList.toggle('err', u.phase === 'error');
  $('#updInstall').classList.toggle('hidden', u.phase !== 'available');
  $('#updCheck').disabled = busy;
  $('#updNotes').textContent = u.notes || '';
  $('#updNotes').classList.toggle('hidden', !(u.notes && (u.phase === 'available' || u.phase === 'downloading')));
  $('#updProgress').classList.toggle('hidden', u.phase !== 'downloading' && u.phase !== 'installing');
  $('#updProgress').value = u.phase === 'installing' ? 100 : u.progress;
  $('nav [data-page=system]').classList.toggle('badge', u.phase === 'available');
  clearTimeout(updTimer);
  if (u.phase === 'installing') {
    if (!window.updReloading) { window.updReloading = true; setTimeout(() => location.reload(), 25000); }
  } else if (busy) updTimer = setTimeout(loadUpdate, 1000);
}
async function loadUpdate() {
  try { renderUpdate(await api('/api/update')); } catch {}
}
async function updateAction(action) {
  try { renderUpdate(await api('/api/update', { body: { action } })); }
  catch (e) { return toast(e.message, true); }
  setTimeout(loadUpdate, 500);
}
$('#updCheck').onclick = () => updateAction('check');
$('#updInstall').onclick = () => {
  if (confirm(`Install firmware ${$('#updLatest').textContent}? The light switches off for about half a minute while the device restarts.`)) updateAction('install');
};

$('#otaBtn').onclick = () => {
  const file = $('#otaFile').files[0];
  if (!file) return toast('Choose the *-ota.bin file first', true);
  if (file.name.endsWith('.uf2')) return toast('Use the *-ota.bin file, not the .uf2', true);
  const bar = $('#otaProgress'), msg = $('#otaMsg');
  bar.classList.remove('hidden'); bar.value = 0; msg.textContent = 'Uploading…';
  const x = new XMLHttpRequest();
  x.open('POST', '/api/ota');
  x.setRequestHeader('Content-Type', 'application/octet-stream');
  x.upload.onprogress = e => { if (e.lengthComputable) bar.value = e.loaded * 100 / e.total; };
  x.onload = () => {
    if (x.status === 200) {
      bar.value = 100; msg.textContent = 'Done – the device is rebooting. This page reloads in 20 s.';
      setTimeout(() => location.reload(), 20000);
    } else msg.textContent = 'Update failed: ' + (x.responseText || x.status);
  };
  x.onerror = () => { msg.textContent = 'Upload failed (connection lost).'; };
  x.send(file);
};

$('#backupBtn').onclick = async () => {
  try {
    const [config, presets] = await Promise.all([api('/api/config'), api('/api/presets')]);
    const blob = new Blob([JSON.stringify({ kind: 'pico-light-show-backup', version: 1, config, presets }, null, 2)], { type: 'application/json' });
    const a = h('a', { href: URL.createObjectURL(blob), download: `${hostname(config.name || config.default_name)}-backup.json` });
    document.body.append(a); a.click(); a.remove();
  } catch (e) { toast(e.message, true); }
};
$('#restoreFile').onchange = async ev => {
  const file = ev.target.files[0];
  if (!file) return;
  try {
    const b = JSON.parse(await file.text());
    if (b.kind !== 'pico-light-show-backup') throw new Error('Not a Pico Light Show backup');
    const c = { ...b.config };
    for (const k of Object.keys(c)) if (k.endsWith('_set')) delete c[k];
    await api('/api/config', { body: c });
    for (const p of b.presets || []) await api('/api/presets', { body: p });
    toast('Backup restored – reboot to apply network changes');
    $('#rebootBanner').classList.remove('hidden');
  } catch (e) { toast('Restore failed: ' + e.message, true); }
  ev.target.value = '';
};
$('#rebootBtn').onclick = () => { if (confirm('Reboot the device?')) reboot(); };
$('#resetBtn').onclick = async () => {
  if (!confirm('Erase all settings and presets? The device will start as access point "PicoLightShow-…".')) return;
  try { await api('/api/factory-reset', { body: {} }); toast('Factory reset – rebooting…'); }
  catch (e) { toast(e.message, true); }
};

// ---------------------------------------------------------------- start
(async function init() {
  try { meta = await api('/api/effects'); } catch (e) { toast('Cannot reach the device', true); }
  renderEffects();
  try { applyState(await api('/api/state')); } catch {}
  loadInfo();
  loadUpdate();
  connectWs();
  const p = location.hash.slice(1);
  if (['presets', 'settings', 'system'].includes(p)) showPage(p);
})();
