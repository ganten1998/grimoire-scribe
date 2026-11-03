#!/usr/bin/env node
// live_pip_bench.js: Live view's picture in picture measured in real headless browsers (not part of
// make check: it needs the browsers, and an idle machine for its numbers). A stand-in Kindle serves
// /live (web/live.js, instrumented) and writes on its page at a realistic pace: a sample every 4 ms,
// each sent as it comes (--pen-ms), tile batches every 33 ms holding the ink rendered up to 25 ms ago, over a
// page of 1 px ruling and two Bayer-dithered patches. Each browser (Firefox Nightly, Firefox and
// Chrome from /Applications, a throwaway profile each, 1440 x 900 at 2x) loads it, and for a few
// seconds each, windowed, then in video PiP (Live view's own button) with the tab hidden and then
// shown, closed again, --repeats times, it reports: the page's own Latency meter (pen age at paint,
// p50/p90/p99/max ms, on one clock with the stand-in Kindle's), the playout delay and a sample's
// arrival to its paint, our time per frame (frame, averageInto, vpipDraw, the quick ink), a batch's
// arrival to its frame handed to the capture (p50/p90/max ms), batches merged into one frame, frames
// handed per second and the gaps between them, and which frames the video itself shows (each frame
// carries its number in its top left 8x8 pixels, read back from the video every 4 ms on a network
// clock that a hidden tab does not slow). On plain-http loopback Chromium would pick Document PiP;
// the page is told there is none, as on the Kindle's LAN address. Last, a frame drawn piecemeal is
// checked against the same picture drawn whole (--no-pixels skips it).
//   usage: node tests/live_pip_bench.js [--browsers nightly,firefox,chrome] [--port 8950]
//          [--seconds 5] [--repeats 3] [--live web/live.js] [--pip-size 360x480] [--fps 60]
//          [--pen-ms 4] [--capture-ms 0] [--prefs name=value,...] [--no-pixels] [--budget]
// Headless Firefox draws canvases in software: what a GPU costs per pixel does not show here
// (--capture-ms emulates a cost per million pixels of each captured frame).
// --budget fails (exit 1) when, in any browser whose video PiP opened, a hidden tab hands batches
// later than 50 ms at p90 or fewer than 30 frames a second while writing.
'use strict';
const http = require('http'), fs = require('fs'), os = require('os'), path = require('path'), zlib = require('zlib');
const {spawn, execFileSync} = require('child_process');
const arg = (name, def) => { const i = process.argv.indexOf('--' + name); return i > 0 ? process.argv[i + 1] : def; };
const REPO = path.join(__dirname, '..');
const PORT = +arg('port', process.env.GRIMOIRE_TEST_PORT || 8950);
const LIVE_JS = path.resolve(arg('live', path.join(REPO, 'web/live.js')));
const SECS = +arg('seconds', 5), REPS = +arg('repeats', 3), BUDGET = process.argv.includes('--budget');
const KINDS = arg('browsers', 'nightly,firefox,chrome').split(',');
const sleep = ms => new Promise(r => setTimeout(r, ms));
let failed = false;
// (every browser still running is stopped whatever ends the run: its PIDs, its profile's processes)
const running = new Set();
function stopAll() { for (const stop of running) stop.sync(); }
process.on('exit', stopAll);
for (const sig of ['SIGINT', 'SIGTERM']) process.on(sig, () => { stopAll(); process.exit(130); });
process.on('uncaughtException', e => { console.log('FAIL ' + (e && e.stack || e)); stopAll(); process.exit(1); });
const KINDLE_T0 = Date.now();  // (the stand-in Kindle's clock: its ms are 5e6 + the ms since this)
const FPS = +arg('fps', 60);  // (Firefox's refresh rate: headless has no display, 60 as a laptop's)
const PEN_MS = +arg('pen-ms', 4);  // (how often the pen stream sends what it has: the Kindle sends each sample as it comes)
const PIP_SIZE = arg('pip-size', '360x480').split('x').map(Number);  // (the PiP window, CSS pixels: a headless one is tiny)
const PREFS = (arg('prefs', '') || '').split(',').filter(Boolean);  // (more Firefox prefs, name=value)
const CHECK_PIXELS = !process.argv.includes("--no-pixels");  // (the piecemeal frames checked against a whole one)
const CAPTURE_MS = +arg('capture-ms', 0);  // (main-thread ms per million pixels each capture frame costs, emulated: a GPU canvas's readback)
// The stand-in Kindle: its page, the writer, the two streams, and the page's script instrumented.
const W = 1860, H = 2480, TILE = 64;

const CRC = new Int32Array(256).map((_, n) => { let c = n; for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1; return c; });
function crc(buf) { let c = -1; for (let i = 0; i < buf.length; i++) c = CRC[(c ^ buf[i]) & 255] ^ (c >>> 8); return (c ^ -1) >>> 0; }
function chunk(type, data) {
    const out = Buffer.alloc(12 + data.length);
    out.writeUInt32BE(data.length, 0); out.write(type, 4, 'ascii'); data.copy(out, 8);
    out.writeUInt32BE(crc(out.subarray(4, 8 + data.length)), 8 + data.length);
    return out;
}
function png(fb, x, y, w, h) {
    const rows = Buffer.alloc((w + 1) * h);
    for (let r = 0; r < h; r++) { rows[r * (w + 1)] = 0; Buffer.from(fb.buffer, (y + r) * W + x, w).copy(rows, r * (w + 1) + 1); }
    const ihdr = Buffer.alloc(13); ihdr.writeUInt32BE(w, 0); ihdr.writeUInt32BE(h, 4); ihdr[8] = 8; ihdr[9] = 0;
    return Buffer.concat([Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]), chunk('IHDR', ihdr), chunk('IDAT', zlib.deflateSync(rows, {level: 1})), chunk('IEND', Buffer.alloc(0))]);
}
function frame(type, body) {
    const head = Buffer.alloc(5); head.writeUInt32LE(body.length + 1, 0); head[4] = type;
    return Buffer.concat([head, body]);
}
// The page under the writing: Seyes-like ruling (1 px lines, grays 170/160/150), a margin line, two
// patches ordered-dithered (8x8 Bayer, as DU frames are), earlier handwriting.
const BAYER = [0, 32, 8, 40, 2, 34, 10, 42, 48, 16, 56, 24, 50, 18, 58, 26, 12, 44, 4, 36, 14, 46, 6, 38, 60, 28, 52, 20, 62, 30, 54, 22,
    3, 35, 11, 43, 1, 33, 9, 41, 51, 19, 59, 27, 49, 17, 57, 25, 15, 47, 7, 39, 13, 45, 5, 37, 63, 31, 55, 23, 61, 29, 53, 21];
function background() {
    const fb = new Uint8Array(W * H).fill(255);
    for (let y = 150; y < H - 40; y += 24) for (let x = 0; x < W; x++) fb[y * W + x] = (y - 150) % 96 === 0 ? 150 : 170;
    for (let x = 220; x < W; x += 96) for (let y = 150; y < H - 40; y++) fb[y * W + x] = Math.min(fb[y * W + x], 160);
    for (let y = 0; y < H; y++) { fb[y * W + 180] = 120; fb[y * W + 181] = 120; }
    for (const [x0, y0, v] of [[300, 1700, 128], [900, 1700, 200]])
        for (let y = y0; y < y0 + 400; y++) for (let x = x0; x < x0 + 500; x++) fb[y * W + x] = v * 64 / 255 > BAYER[(y & 7) * 8 + (x & 7)] ? 255 : 0;
    // toolbar
    for (let y = 0; y < 120; y++) for (let x = 0; x < W; x++) fb[y * W + x] = 235;
    return fb;
}
function stamp(fb, cx, cy, r, dirty) {
    const x0 = Math.max(0, Math.floor(cx - r - 1)), x1 = Math.min(W - 1, Math.ceil(cx + r + 1));
    const y0 = Math.max(0, Math.floor(cy - r - 1)), y1 = Math.min(H - 1, Math.ceil(cy + r + 1));
    for (let y = y0; y <= y1; y++) for (let x = x0; x <= x1; x++) {
        const d = Math.hypot(x + 0.5 - cx, y + 0.5 - cy), cov = Math.max(0, Math.min(1, r + 0.5 - d));
        if (cov <= 0) continue;
        const v = Math.round(255 - (255 - 30) * cov), i = y * W + x;
        if (v < fb[i]) fb[i] = v;
    }
    for (let ty = Math.floor(y0 / TILE); ty <= Math.floor(y1 / TILE); ty++)
        for (let tx = Math.floor(x0 / TILE); tx <= Math.floor(x1 / TILE); tx++) dirty.add(ty * 1000 + tx);
}
// The writer: where the pen is at time t (ms since it started): handwriting, words of 0.9 s with
// 0.25 s lifts (hovering), lines 96 px apart.
function penAt(t) {
    const word = 1150, inWord = t % word, n = Math.floor(t / word);
    const perLine = 7, line = Math.floor(n / perLine) % 22, col = n % perLine;
    const x0 = 240 + col * 230, y0 = 246 + line * 96;
    const s = Math.min(inWord, 900) / 1000;
    const x = x0 + s * 200 + 14 * Math.sin(2 * Math.PI * 4.5 * s), y = y0 + 26 * Math.sin(2 * Math.PI * 5.5 * s) * (0.7 + 0.3 * Math.sin(9 * s));
    return {x, y, touch: inWord < 900, down: inWord < 4, p: 0.55 + 0.35 * Math.sin(3 * s)};
}

function stream(req, res, pen) {
    res.writeHead(200, {'Content-Type': 'application/octet-stream', 'Cache-Control': 'no-store'});
    res.socket.setNoDelay(true);
    const t0 = Date.now(), base = 1e6, since = t0 - KINDLE_T0;
    const id = t => (base + Math.round(t * 1000)) >>> 0;
    const kms = t => (5e6 + since + Math.round(t)) >>> 0;
    const EPOCH = 7;
    let alive = true, timers = [];
    req.on('close', () => { alive = false; timers.forEach(clearInterval); });
    const hello = Buffer.alloc(12); hello.writeUInt16LE(3, 0); hello.writeUInt16LE(W, 2); hello.writeUInt16LE(H, 4); hello.writeUInt16LE(TILE, 6); hello.writeUInt32LE(EPOCH, 8);
    res.write(frame(5, Buffer.from([0, 0, 255])));
    res.write(frame(1, hello));
    if (pen) {
        const style = Buffer.alloc(64);
        style[0] = 1; style[1] = 0;
        [2, 5, 0, 1, 1, 1, 1].forEach((v, i) => style.writeFloatLE(v, 4 + i * 4));
        [0, 120, W, H - 120].forEach((v, i) => style.writeInt32LE(v, 32 + i * 4));
        [0, 0, W, 120].forEach((v, i) => style.writeInt32LE(v, 48 + i * 4));
        res.write(frame(8, style));
        const r0 = Buffer.alloc(8); r0.writeUInt32LE(EPOCH, 0); r0.writeUInt32LE(1, 4);
        res.write(frame(7, r0));
        let sent = 0;  // (ms of the writer's time already sent, in 4 ms steps)
        timers.push(setInterval(() => {
            if (!alive) return;
            const now = Date.now() - t0, parts = [];
            for (; sent + 4 <= now; sent += 4) {
                const t = sent + 4, p = penAt(t), b = Buffer.alloc(20);
                b.writeUInt32LE(id(t), 0); b.writeUInt32LE(kms(t), 4);
                b.writeUInt16LE(Math.round(p.x), 8); b.writeUInt16LE(Math.round(p.y), 10);
                b.writeUInt16LE(Math.round(p.p * 65535), 12); b[16] = p.touch ? 2 : 1;
                b[17] = (p.touch ? 16 : 0) | (p.down ? 4 : 0);
                parts.push(b);
            }
            if (!parts.length) return;
            const head = Buffer.alloc(8); head.writeUInt32LE(EPOCH, 0);
            res.write(frame(7, Buffer.concat([head, ...parts])));
        }, PEN_MS));
        return;
    }
    const fb = background();
    const rows = [];
    for (let y = 0; y < H; y += TILE) { const h = Math.min(TILE, H - y); const body = png(fb, 0, y, W, h); const hd = Buffer.alloc(8); hd.writeUInt16LE(0, 0); hd.writeUInt16LE(y, 2); hd.writeUInt16LE(W, 4); hd.writeUInt16LE(h, 6); rows.push(frame(2, Buffer.concat([hd, body]))); }
    res.write(Buffer.concat(rows));
    const b0 = Buffer.alloc(20); b0.writeUInt32LE(0, 0); b0.writeUInt32LE(EPOCH, 16);
    res.write(frame(3, b0));
    let drawn = 0, seq = 1;
    const dirty = new Set();
    timers.push(setInterval(() => {
        if (!alive) return;
        const now = Date.now() - t0, upto = now - 25;
        let ack = 0;
        for (; drawn + 4 <= upto; drawn += 4) {
            const t = drawn + 4, p = penAt(t);
            if (p.touch) {
                const prev = penAt(t - 4);
                const steps = prev.touch ? Math.max(1, Math.ceil(Math.hypot(p.x - prev.x, p.y - prev.y))) : 1;
                for (let k = 1; k <= steps; k++) stamp(fb, prev.x + (p.x - prev.x) * k / steps, prev.y + (p.y - prev.y) * k / steps, 1 + 1.5 * p.p, dirty);
                ack = t;
            }
        }
        const out = [];
        for (const key of dirty) {
            const tx = key % 1000, ty = Math.floor(key / 1000), x = tx * TILE, y = ty * TILE;
            const w = Math.min(TILE, W - x), h = Math.min(TILE, H - y), hd = Buffer.alloc(8);
            hd.writeUInt16LE(x, 0); hd.writeUInt16LE(y, 2); hd.writeUInt16LE(w, 4); hd.writeUInt16LE(h, 6);
            out.push(frame(2, Buffer.concat([hd, png(fb, x, y, w, h)])));
        }
        dirty.clear();
        const end = Buffer.alloc(20);
        end.writeUInt32LE(seq++, 0); end.writeUInt32LE(kms(now), 4);
        if (ack) { end.writeUInt32LE(id(ack), 8); end.writeUInt32LE(kms(ack), 12); }
        end.writeUInt32LE(EPOCH, 16);
        if (out.length || ack) res.write(Buffer.concat([...out, frame(3, end)]));
    }, 33));
}

function instrumented() {
    let src = fs.readFileSync(LIVE_JS, 'utf8');
    const swap = (a, b) => { if (!src.includes(a)) throw new Error('patch not found: ' + a.slice(0, 60)); src = src.replace(a, b); };
    for (const name of ['averageInto', 'vpipDraw', 'frame', 'compose', 'drawPen', 'present']) {
        const re = new RegExp('\\n(\\s*)function ' + name + '\\(([^)]*)\\) \\{');
        const m = src.match(re);
        if (!m) throw new Error('no function ' + name);
        const args = m[2].replace(/=[^,]*/g, '');
        src = src.replace(re, `\n${m[1]}function ${name}(${m[2]}) { const __t = performance.now(); try { return ${name}__(${args}); } finally { __P.span('${name}', __t); } }\n${m[1]}function ${name}__(${m[2]}) {`);
    }
    swap('FastInk.prototype.draw = function (ctx, k, look, layer, until) {', 'FastInk.prototype.draw = function (ctx, k, look, layer, until) { const __t = performance.now(); try { return this.draw__(ctx, k, look, layer, until); } finally { __P.span("ink", __t); } };\n    FastInk.prototype.draw__ = function (ctx, k, look, layer, until) {');
    swap('} else if (type === 3) {', '} else if (type === 3) { __P.arrive();');
    swap('for (const sample of packet.samples) {', 'for (const sample of packet.samples) { sample.at = now;');
    swap('if (debug) for (const s of paintSamples.slice(0, i)) ages.add(ages.samples, now, s.ms);', 'if (debug) for (const s of paintSamples.slice(0, i)) { ages.add(ages.samples, now, s.ms); __P.queue.push(now - s.at); }');
    swap('if (dirty) composeSoon(dirty.x', '__P.decoded(); if (dirty) composeSoon(dirty.x');
    // handed: the capture asked for a frame (video PiP), or a windowed frame done
    const at = src.indexOf('if (vpip.frame) vpip.frame();', src.indexOf('function vpipDraw__('));
    if (at < 0) throw new Error('vpipDraw handed patch');
    src = src.slice(0, at) + '{ __P.stamp(c); __P.busy(vpip.out.width * vpip.out.height); if (vpip.frame) vpip.frame(); __P.handed(true); }' + src.slice(at + 'if (vpip.frame) vpip.frame();'.length);
    swap('        if (penQueued) drawPen();\n    }', '        if (penQueued) drawPen();\n        if (!vpip.on) __P.handed(false);\n    }');
    swap('    (async function runFast() {', '    window.__hooks = {vpip, vpipFit, vpipOn, vpipOff, vpipOpen, vpipFirefox, pip, tier, get raw() { return raw; }, get shown() { return shown; }, get view() { return view; }, get ages() { return ages; }, vpipDraw, get playout() { return playout; }, get fastConnected() { return fastConnected; }, frameSoon};\n    (async function runFast() {');
    return src;
}

// Headless browsers driven over their own protocols (CDP for Chrome, WebDriver BiDi for Firefox), a
// throwaway profile each; every process started on that profile is stopped by its PID.
const BINS = {
    chrome: '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',
    firefox: '/Applications/Firefox.app/Contents/MacOS/firefox',
    nightly: '/Applications/Firefox Nightly.app/Contents/MacOS/firefox',
};
function rpc(url) {
    return new Promise((resolve, reject) => {
        const ws = new WebSocket(url);
        let next = 1;
        const waiting = new Map(), events = [];
        ws.onmessage = m => {
            const msg = JSON.parse(m.data);
            if (msg.id && waiting.has(msg.id)) {
                const [ok, no] = waiting.get(msg.id);
                waiting.delete(msg.id);
                if (msg.error) no(new Error(JSON.stringify(msg.error) + ' ' + (msg.message || '')));
                else ok(msg.result);
            } else events.push(msg);
        };
        ws.onopen = () => resolve({
            send(method, params = {}, extra = {}) {
                const id = next++;
                ws.send(JSON.stringify(Object.assign({id, method, params}, extra)));
                return new Promise((ok, no) => waiting.set(id, [ok, no]));
            }, events, close() { ws.close(); },
        });
        ws.onerror = e => reject(new Error('ws ' + url + ' ' + (e.message || '')));
    });
}
function launch(kind, opts = {}) {
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'grimoire-pip-'));
    let args;
    if (kind === 'chrome') {
        args = ['--headless=new', '--user-data-dir=' + dir, '--remote-debugging-port=0', '--no-first-run', '--no-default-browser-check',
            '--window-size=1440,900', '--force-device-scale-factor=2', '--autoplay-policy=no-user-gesture-required', 'about:blank'];
    } else {
        fs.writeFileSync(path.join(dir, 'user.js'), [
            'user_pref("layout.css.devPixelsPerPx", "2.0");',
            'user_pref("browser.shell.checkDefaultBrowser", false);',
            'user_pref("datareporting.policy.dataSubmissionEnabled", false);',
            'user_pref("toolkit.telemetry.reportingpolicy.firstRun", false);',
            'user_pref("browser.aboutwelcome.enabled", false);',
            'user_pref("browser.startup.homepage_override.mstone", "ignore");',
            'user_pref("app.update.disabledForTesting", true);',
            'user_pref("media.autoplay.default", 0);',
            `user_pref("layout.frame_rate", ${FPS});`,
            ...PREFS.map(x => { const [k, v] = x.split('='); return `user_pref("${k}", ${/^(true|false|-?\d+)$/.test(v) ? v : JSON.stringify(v)});`; }),
            ...(opts.prefs || []),
        ].join('\n'));
        args = ['--headless', '--profile', dir, '--no-remote', '--remote-debugging-port', '0', '--width', '1440', '--height', '900'];
    }
    const proc = spawn(BINS[kind], args, {stdio: ['ignore', 'pipe', 'pipe']});
    let log = '';
    return new Promise((resolve, reject) => {
        const timer = setTimeout(() => {
            for (const pid of [proc.pid, ...(() => { try { return execFileSync('pgrep', ['-f', dir]).toString().trim().split(/\s+/).map(Number).filter(Boolean); } catch (e) { return []; } })()]) { try { process.kill(pid, 'SIGKILL'); } catch (e) {} }
            reject(new Error('no endpoint: ' + log.slice(-2000)));
        }, 30000);
        const look = d => {
            log += d;
            const m = kind === 'chrome' ? log.match(/DevTools listening on (ws:\S+)/) : log.match(/WebDriver BiDi listening on (ws:\S+)/);
            if (!m) return;
            clearTimeout(timer);
            proc.stdout.off('data', look); proc.stderr.off('data', look);
            proc.stdout.on('data', () => {}); proc.stderr.on('data', () => {});
            resolve(connect(kind, m[1], proc, dir));
        };
        proc.stdout.on('data', look); proc.stderr.on('data', look);
        // (Firefox may start again in a process of its own on a fresh profile: it keeps our pipes)
        proc.on('exit', c => { if (c) reject(new Error('exited ' + c + ' ' + log.slice(-2000))); });
    });
}
async function connect(kind, url, proc, dir) {
    // every process started on this profile (its unique directory): the browser and its children
    const mine = () => { try { return execFileSync('pgrep', ['-f', dir]).toString().trim().split(/\s+/).map(Number).filter(Boolean); } catch (e) { return []; } };
    const stop = async () => {
        running.delete(stop);
        for (const pid of [proc.pid, ...mine()]) { try { process.kill(pid, 'SIGTERM'); } catch (e) {} }
        for (let i = 0; i < 50 && mine().length; i++) await new Promise(r => setTimeout(r, 100));
        for (const pid of mine()) { try { process.kill(pid, 'SIGKILL'); } catch (e) {} }
        fs.rmSync(dir, {recursive: true, force: true});
    };
    stop.sync = () => {  // (at exit: no waiting, SIGKILL)
        for (const pid of [proc.pid, ...mine()]) { try { process.kill(pid, 'SIGKILL'); } catch (e) {} }
        try { fs.rmSync(dir, {recursive: true, force: true}); } catch (e) {}
    };
    running.add(stop);
    if (kind === 'chrome') {
        const port = new URL(url).port;
        const list = await (await fetch('http://127.0.0.1:' + port + '/json/list')).json();
        const target = list.find(t => t.type === 'page');
        const page = await rpc(target.webSocketDebuggerUrl), browser = await rpc(url);
        await page.send('Page.enable');
        return {
            kind, pid: proc.pid, stop,
            async go(u) { await page.send('Page.navigate', {url: u}); },
            async eval(expr) {
                const r = await page.send('Runtime.evaluate', {expression: expr, awaitPromise: true, returnByValue: true, userGesture: true});
                if (r.exceptionDetails) throw new Error(JSON.stringify(r.exceptionDetails).slice(0, 800));
                return r.result.value;
            },
            async click(x, y) {
                for (const type of ['mousePressed', 'mouseReleased'])
                    await page.send('Input.dispatchMouseEvent', {type, x, y, button: 'left', clickCount: 1});
            },
            async prepare() {},
            async hide() {
                const t = await browser.send('Target.createTarget', {url: 'about:blank', newWindow: false, background: false});
                this.other = t.targetId;
                await browser.send('Target.activateTarget', {targetId: t.targetId});
            },
            async show() {
                if (this.other) await browser.send('Target.closeTarget', {targetId: this.other});
                this.other = null;
                await browser.send('Target.activateTarget', {targetId: target.id});
            },
        };
    }
    const bidi = await rpc(url + (url.endsWith('/session') ? '' : '/session'));
    await bidi.send('session.new', {capabilities: {}});
    const tree = await bidi.send('browsingContext.getTree', {});
    const context = tree.contexts[0].context;
    return {
        kind, pid: proc.pid, stop,
        async go(u) { await bidi.send('browsingContext.navigate', {context, url: u, wait: 'complete'}); },
        async eval(expr) {
            const r = await bidi.send('script.evaluate', {expression: expr, target: {context}, awaitPromise: true, resultOwnership: 'none', userActivation: true,
                serializationOptions: {maxObjectDepth: 20}});
            if (r.type === 'exception') throw new Error(JSON.stringify(r.exceptionDetails).slice(0, 800));
            return fromBidi(r.result);
        },
        async click(x, y) {
            await bidi.send('input.performActions', {context, actions: [{type: 'pointer', id: 'm', parameters: {pointerType: 'mouse'},
                actions: [{type: 'pointerMove', x: Math.round(x), y: Math.round(y)}, {type: 'pointerDown', button: 0}, {type: 'pointerUp', button: 0}]}]});
        },
        // (A tab made while a PiP window is open never answers: prepare() makes it before.)
        async prepare() {
            if (this.other) return;
            const r = await bidi.send('browsingContext.create', {type: 'tab', background: true});
            this.other = r.context;
            await bidi.send('browsingContext.activate', {context});
        },
        async hide() {
            await this.prepare();
            await bidi.send('browsingContext.activate', {context: this.other});
        },
        async show() { await bidi.send('browsingContext.activate', {context}); },
    };
}
function fromBidi(v) {
    if (!v) return v;
    switch (v.type) {
        case 'undefined': return undefined;
        case 'null': return null;
        case 'string': case 'boolean': return v.value;
        case 'number': return typeof v.value === 'string' ? Number(v.value) : v.value;
        case 'array': return v.value.map(fromBidi);
        case 'object': return Object.fromEntries(v.value.map(([k, x]) => [typeof k === 'string' ? k : fromBidi(k), fromBidi(x)]));
        default: return v.value;
    }
}

// Before the page's script: its words, no Document PiP, and the probes (__P).
const PRELUDE = String.raw`window.GRIMOIRE_LIVE_WORDS = new Proxy({}, {get: (_, k) => typeof k === "string" ? k : undefined});
// (The owner's page is plain http on the LAN, where Chromium has no Document PiP: the same here on loopback.)
try { Object.defineProperty(window, 'documentPictureInPicture', {value: undefined, configurable: true}); } catch (e) {}
window.__P = {
    spans: {}, arrived: [], ready: [], lat: [], decode: [], merged: [], hands: [], on: true,
    span(name, t) { if (this.on) (this.spans[name] = this.spans[name] || []).push(performance.now() - t); },
    arrive() { this.arrived.push(performance.now()); },
    decoded() { const now = performance.now(); for (const a of this.arrived) { this.decode.push(now - a); this.ready.push(a); } this.arrived = []; },
    delay: [], queue: [],
    busy(px) { const ms = (window.__CAPTURE_MS || 0) * px / 1e6, t = performance.now(); while (performance.now() - t < ms) {} },
    handed(pip) {
        const now = performance.now();
        if (window.__hooks) this.delay.push(window.__hooks.playout.delay);
        this.hands.push(now);
        if (this.ready.length) { for (const a of this.ready) this.lat.push(now - a); this.merged.push(this.ready.length); this.ready = []; }
    },
    reset() { this.spans = {}; this.lat = []; this.decode = []; this.merged = []; this.hands = []; this.cap = []; this.shown = []; this.shownLag = []; this.delay = []; this.queue = []; },
    cap: [], stampN: 0, stampAt: new Map(), shown: [], shownLag: [],
    // each frame handed to the capture carries its number in its top left 8x8 pixels
    stamp(c) { const n = ++this.stampN % 4096; c.fillStyle = 'rgb(' + (n & 15) * 16 + ',' + (n >> 4 & 15) * 16 + ',' + (n >> 8 & 15) * 16 + ')'; c.fillRect(0, 0, 8, 8); this.stampAt.set(n, performance.now()); if (this.stampAt.size > 8192) this.stampAt.clear(); },
    // which frame the video (what the PiP window shows) has, read every 4 ms on a network clock a hidden tab does not slow
    async watchVideo(video) {
        const pc = document.createElement('canvas'); pc.width = pc.height = 8; const x = pc.getContext('2d', {willReadFrequently: true});
        const rd = (await fetch('/tick?ms=4')).body.getReader(); let last = -1;
        for (;;) { const {done} = await rd.read(); if (done) return;
            try { x.drawImage(video, 0, 0, 8, 8, 0, 0, 8, 8); } catch (e) { continue; }
            const d = x.getImageData(4, 4, 1, 1).data, n = Math.round(d[0] / 16) + Math.round(d[1] / 16) * 16 + Math.round(d[2] / 16) * 256;
            if (n !== last) { last = n; const now = performance.now(); this.shown.push(now); if (this.stampAt.has(n)) this.shownLag.push(now - this.stampAt.get(n)); } }
    },
    // frames that reach the capture track (MediaStreamTrackProcessor), else the video's presented frames
    watch(track, video) {
        const self = this;
        if (window.MediaStreamTrackProcessor && track) {
            const reader = new MediaStreamTrackProcessor({track: track.clone()}).readable.getReader();
            self.capHow = 'processor';
            (async () => { for (;;) { const {value, done} = await reader.read(); if (done) return; self.cap.push(performance.now()); value.close(); } })();
        } else if (video && video.requestVideoFrameCallback) {
            self.capHow = 'rvfc';
            const on = (now, meta) => { self.cap.push(performance.now()); self.capPresented = meta.presentedFrames; video.requestVideoFrameCallback(on); };
            video.requestVideoFrameCallback(on);
        }
    },
};
`;
function serve() {
    return new Promise(resolve => {
        const server = http.createServer((req, res) => {
            const url = req.url.split('?')[0];
            if (url === '/live') {
                const css = fs.readFileSync(path.join(REPO, 'web/live.css'), 'utf8');
                let script;
                try { script = instrumented(); } catch (e) {  // (a live.js the probes do not fit: said, and the run ends cleanly)
                    console.log('FAIL the page could not be instrumented: ' + e.message);
                    failed = true;
                    res.writeHead(500); res.end(); return;
                }
                res.writeHead(200, {'Content-Type': 'text/html; charset=utf-8'});
                res.end(`<!DOCTYPE html><html><head><meta charset=utf-8><title>Live view</title></head><body><div id=live><p class=meta>Connecting...</p></div>` +
                    `<script>window.__CAPTURE_MS = ${CAPTURE_MS};${PRELUDE}</script><style>${css}</style><script>${script}</script></body></html>`);
            } else if (url === '/live/stream') stream(req, res, false);
            else if (url === '/live/pen') stream(req, res, true);
            else if (url === '/tick') {  // a byte every ms= ms: a clock a hidden tab does not slow down
                res.writeHead(200, {'Content-Type': 'application/octet-stream', 'Cache-Control': 'no-store'});
                res.socket.setNoDelay(true);
                const t = setInterval(() => res.write(Buffer.from([1])), +(req.url.split('ms=')[1] || 16));
                req.on('close', () => clearInterval(t));
            } else if (url === '/live/keys') { res.writeHead(200, {'Content-Type': 'application/json'}); res.end('[]'); }
            else if (url === '/live/time') { res.writeHead(200, {'Content-Type': 'application/json'}); res.end(JSON.stringify({ms: 5e6 + Date.now() - KINDLE_T0})); }
            else { res.writeHead(404); res.end(); }
        });
        server.listen(PORT, '127.0.0.1', () => resolve(server));
    });
}
const q = (a, p) => { if (!a.length) return NaN; const s = [...a].sort((x, y) => x - y); return s[Math.min(s.length - 1, Math.floor((s.length - 1) * p))]; };
const r1 = x => Math.round(x * 10) / 10;
const gaps = a => a.slice(1).map((t, i) => t - a[i]);
function stats(d) {
    const out = {spans: {}};
    for (const [k, v] of Object.entries(d.spans)) out.spans[k] = {per_s: r1(v.length / SECS), p50: r1(q(v, .5)), p90: r1(q(v, .9)), max: r1(Math.max(...v)), ms_s: r1(v.reduce((a, b) => a + b, 0) / SECS)};
    out.latency = {p50: r1(q(d.lat, .5)), p90: r1(q(d.lat, .9)), max: r1(Math.max(...d.lat)), n: d.lat.length};
    out.merged = {frames: d.merged.filter(m => m > 1).length, of: d.merged.length};
    const hg = gaps(d.hands);
    out.handed = {fps: r1(d.hands.length / SECS), gap_p50: r1(q(hg, .5)), gap_p90: r1(q(hg, .9)), gap_max: r1(Math.max(...hg))};
    if (d.shown.length > 1) {
        const sg = gaps(d.shown);
        out.video = {fps: r1(d.shown.length / SECS), gap_p50: r1(q(sg, .5)), gap_p90: r1(q(sg, .9)), gap_max: r1(Math.max(...sg)),
            over50: sg.filter(g => g > 50).length, lag_p50: r1(q(d.shownLag, .5)), lag_p90: r1(q(d.shownLag, .9))};
    }
    const pa = d.penAge || [];
    out.penAge = {p50: r1(q(pa, .5)), p90: r1(q(pa, .9)), p99: r1(q(pa, .99)), max: r1(Math.max(...pa)), n: pa.length};
    out.queue = {p50: r1(q(d.queue || [], .5)), p90: r1(q(d.queue || [], .9))};
    out.delay = {p50: r1(q(d.delay || [], .5)), p90: r1(q(d.delay || [], .9))};
    return out;
}
async function measure(kind) {
    const b = await launch(kind);
    const results = {kind, modes: {}};
    const to = (p, ms, what) => Promise.race([p, sleep(ms).then(() => { throw new Error('timeout ' + what); })]);
    try {
        await b.go('http://127.0.0.1:' + PORT + '/live');
        await b.prepare();
        // the page's own latency meter on (View settings, Diagnostics, Latency): its pen age at paint
        await b.eval(`[...document.querySelectorAll('button')].find(x => x.textContent === 'latency').click(), true`);
        await sleep(3000);
        const window_ = async (name, n = REPS) => {
            for (let i = 0; i < n; i++) {
                await b.eval('__P.reset(); __hooks.ages.samples.length = 0; true');
                await sleep(SECS * 1000);
                const d = JSON.parse(await b.eval('JSON.stringify({penAge: __hooks.ages.samples, delay: __P.delay, queue: __P.queue, spans: __P.spans, lat: __P.lat, merged: __P.merged, hands: __P.hands, shown: __P.shown, shownLag: __P.shownLag, vis: document.visibilityState})'));
                (results.modes[name] = results.modes[name] || []).push(Object.assign({vis: d.vis}, stats(d)));
            }
        };
        for (let cycle = 0; cycle < REPS; cycle++) {
            await window_('windowed', 1);
            // (Firefox's BiDi cannot switch tabs while a PiP window is open: hide first, then open it
            // from the hidden tab with the user activation a click gives, then come back.)
            await to(b.hide(), 10000, 'hide');
            await sleep(500);
            await b.eval(`(async () => { __hooks.pip(); for (let i = 0; i < 40 && !__hooks.vpip.on; i++) await new Promise(r => setTimeout(r, 50)); return __hooks.vpip.on; })()`);
            // (a headless PiP window is tiny: the picture sized as a 360 x 480 one, as on a laptop)
            results.pip = await b.eval(`(() => { const v = __hooks.vpip; const real = v.win ? [v.win.width, v.win.height] : null;
                if (v.on) { __hooks.vpipFit(${PIP_SIZE[0]}, ${PIP_SIZE[1]}); if (!__P.watching) { __P.watching = true; __P.watchVideo(v.video); } }
                const t = document.createElement('canvas'); t.width = 1800; t.height = 2400; const tx = t.getContext('2d'); tx.fillRect(0, 0, 1800, 2400); tx.fillStyle = '#f00'; tx.fillRect(10, 10, 100, 100);
                const t0 = performance.now(); tx.getImageData(0, 0, 1800, 2400); const readback = Math.round((performance.now() - t0) * 10) / 10;
                return {readback, on: v.on, floating: !!document.pictureInPictureElement, window: real, out: v.out ? [v.out.width, v.out.height] : null, inPage: v.firefox}; })()`);
            if (!results.pip.on) break;
            await sleep(1000);
            await window_('pip hidden', 1);
            await to(b.show(), 10000, 'show');
            await sleep(1000);
            await window_('pip visible', 1);
            // what the video shows after many frames drawn piecemeal against the same picture drawn
            // whole, in one task (no data comes between): they must not differ (the stamp aside)
            if (CHECK_PIXELS) (results.pixels = results.pixels || []).push(await b.eval(`(() => {
                const v = __hooks.vpip, w = v.out.width, h = v.out.height, until = __hooks.fastConnected ? __hooks.playout.clock(performance.now()) : undefined;
                if (!('whole' in v)) return null;
                __hooks.vpipDraw(until); const a = v.outCtx.getImageData(0, 0, w, h).data;
                v.whole = true; __hooks.vpipDraw(until); const b = v.outCtx.getImageData(0, 0, w, h).data;
                let n = 0, max = 0;
                for (let i = 0; i < a.length; i += 4) { const x = (i / 4) % w, y = Math.floor(i / 4 / w); if (x < 8 && y < 8) continue;
                    const d = Math.max(Math.abs(a[i] - b[i]), Math.abs(a[i + 1] - b[i + 1]), Math.abs(a[i + 2] - b[i + 2])); if (d) { n++; max = Math.max(max, d); } }
                return {differ: n, max, of: w * h}; })()`));
            await b.eval(`(async () => { __hooks.pip(); for (let i = 0; i < 40 && __hooks.vpip.on; i++) await new Promise(r => setTimeout(r, 50)); return !__hooks.vpip.on; })()`);
            await sleep(1000);
        }
    } catch (e) { results.error = String(e); } finally { await b.stop(); }
    return results;
}
function print(r) {
    if (r.pixels) console.log(`  piecemeal frames against the same picture drawn whole: ${r.pixels.map(p => p ? p.differ + ' of ' + p.of + ' pixels differ, by up to ' + p.max : 'n/a (drawn whole each frame)').join('; ')}`);
    console.log(`== ${r.kind}: video PiP ${r.pip && r.pip.on ? (r.pip.floating ? 'floating' : 'in the page (Firefox\'s own button)') + ', picture ' + r.pip.out.join('x') + ', a 1800x2400 GPU-canvas readback ' + r.pip.readback + ' ms' : 'did not open'}${r.error ? ' (' + r.error + ')' : ''}`);
    for (const [mode, runs] of Object.entries(r.modes)) {
        console.log(`  ${mode} (${runs[0].vis})`);
        for (const x of runs) {
            const sp = k => x.spans[k] ? `${k} ${x.spans[k].p50}/${x.spans[k].p90}/${x.spans[k].max} ms, ${x.spans[k].per_s}/s` : '';
            console.log(`    pen age at paint (the page's meter) ${x.penAge.p50}/${x.penAge.p90}/${x.penAge.p99}/${x.penAge.max} ms (p50/p90/p99/max, ${x.penAge.n} samples), playout delay ${x.delay.p50}/${x.delay.p90} ms, arrival to paint ${x.queue.p50}/${x.queue.p90} ms`);
            console.log(`    batch to frame ${x.latency.p50}/${x.latency.p90}/${x.latency.max} ms, merged ${x.merged.frames}/${x.merged.of}; handed ${x.handed.fps}/s, gaps ${x.handed.gap_p50}/${x.handed.gap_p90}/${x.handed.gap_max} ms` +
                (x.video ? `; video shows ${x.video.fps}/s, gaps ${x.video.gap_p50}/${x.video.gap_p90}/${x.video.gap_max} ms (${x.video.over50} over 50), ${x.video.lag_p50}/${x.video.lag_p90} ms after handed` : ''));
            console.log('      ' + ['frame', 'averageInto', 'vpipDraw', 'ink'].map(sp).filter(Boolean).join('; '));
        }
    }
}
(async () => {
    const server = await serve();
    let fail = 0;
    for (const kind of KINDS) {
        if (!fs.existsSync(BINS[kind])) { console.log(`skip ${kind} (no ${BINS[kind]})`); continue; }
        const r = await measure(kind);
        print(r);
        if (BUDGET && r.pip && r.pip.on) {
            for (const x of r.modes['pip hidden'] || []) {
                if (!(x.latency.p90 <= 50) || !(x.handed.fps >= 30)) {
                    console.log(`FAIL ${kind}: a hidden tab's video PiP handed batches ${x.latency.p90} ms late at p90, ${x.handed.fps} frames a second`);
                    fail = 1;
                }
            }
        }
    }
    server.close();
    if (BUDGET && !fail) console.log('ok   video PiP in a hidden tab keeps up (p90 at most 50 ms, at least 30 frames a second)');
    process.exit(fail || failed ? 1 : 0);
})();
