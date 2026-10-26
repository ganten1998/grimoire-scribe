// Live view: what Grimoire on the Kindle shows, as it shows it. src/live.c streams the screen as
// PNG tiles over one long response (GET /live/stream); this draws them on a canvas and keeps it
// connected. Frames are little-endian: u32 length (of the type byte and the payload), u8 type,
// payload. 1 hello (u16 version, w, h, tile; u32 epoch), 2 tile (u16 x, y, w, h; the PNG),
// 3 batch end (u32 seq, u32 the Kindle's ms), 4 ping, 5 state (u8 0 awake, 1 asleep, 2 pairing,
// 3 bye; u8 1 when Grimoire shows its colors inverted, which the picture follows unless the Invert
// switch here says otherwise). Under node (tests/live_check.js) it only exports the frame reader.
(function () {
    'use strict';
    // The stream's frames from its chunks as they come: a frame may be cut anywhere. push gives the
    // frames the chunk completes, each {type, body} (body: the payload, its own bytes).
    function Frames() {
        this.buf = new Uint8Array(0);
    }
    Frames.prototype.push = function (chunk) {
        const joined = new Uint8Array(this.buf.length + chunk.length);
        joined.set(this.buf);
        joined.set(chunk, this.buf.length);
        const out = [];
        let at = 0;
        while (joined.length - at >= 5) {
            const len = new DataView(joined.buffer, at, 4).getUint32(0, true);
            if (len < 1 || len > 64 * 1024 * 1024) throw new Error('bad frame');
            if (joined.length - at < 4 + len) break;
            out.push({type: joined[at + 4], body: joined.slice(at + 5, at + 4 + len)});
            at += 4 + len;
        }
        this.buf = joined.slice(at);
        return out;
    };
    if (typeof module !== 'undefined' && module.exports) {
        module.exports = {Frames};
        return;
    }

    const words = window.GRIMOIRE_LIVE_WORDS;
    const host = document.getElementById('live');
    if (!host || !words) return;
    if (!window.fetch || !window.ReadableStream || !window.createImageBitmap) {
        host.textContent = words.old;
        return;
    }
    const raw = document.createElement('canvas');      // the screen as the Kindle has it
    const rawCtx = raw.getContext('2d');
    const shown = document.createElement('canvas');    // what the page shows (paper made clear or not)
    const shownCtx = shown.getContext('2d');
    shown.className = 'screen';
    const bar = document.createElement('div');
    bar.className = 'bar';
    const pill = document.createElement('span');
    pill.className = 'pill';
    pill.setAttribute('role', 'status');
    pill.setAttribute('aria-live', 'polite');
    function button(label, act) {
        const b = document.createElement('button');
        b.type = 'button';
        b.textContent = label;
        b.addEventListener('click', act);
        bar.append(b);
        return b;
    }
    bar.append(pill);
    // invertChoice: null while the picture follows the Kindle's Invert colors, else this page's own.
    let fit = true, clear = false, recorder = null, chunks = [], kindleInvert = false, invertChoice = null;
    const fitButton = button(words.actual, () => { fit = !fit; layout(); });
    const clearButton = button(words.clear, () => {
        clear = !clear;
        clearButton.setAttribute('aria-pressed', clear ? 'true' : 'false');
        document.body.classList.toggle('clear', clear);
        repaint(0, 0, raw.width, raw.height);
    });
    clearButton.setAttribute('aria-pressed', 'false');
    const invertSwitch = button(words.invert, () => {
        const want = !inverted();
        invertChoice = want === kindleInvert ? null : want;  // (back to the Kindle's: following it again)
        look();
    });
    invertSwitch.setAttribute('role', 'switch');
    invertSwitch.className = 'switch';
    const followButton = button(words.follow, () => {
        invertChoice = null;
        look();
    });
    look();
    const recordButton = button(words.record, record);
    if (!window.MediaRecorder || !shown.captureStream) recordButton.hidden = true;
    button(words.full, full);
    host.replaceChildren(bar, shown);
    addEventListener('keydown', e => {
        if ((e.key === 'f' || e.key === 'F') && !e.metaKey && !e.ctrlKey && !e.altKey) full();
    });
    addEventListener('resize', layout);

    function inverted() { return invertChoice === null ? kindleInvert : invertChoice; }
    // The Invert switch as it stands, "As on the Kindle" only while it says otherwise, the picture.
    function look() {
        const on = inverted();
        invertSwitch.setAttribute('aria-checked', on ? 'true' : 'false');
        followButton.hidden = invertChoice === null || invertChoice === kindleInvert;
        document.body.classList.toggle('inverted', on);
        repaint(0, 0, raw.width, raw.height);
    }
    function status(text, live) {
        pill.textContent = text;
        pill.classList.toggle('on', !!live);
    }
    function layout() {
        fitButton.textContent = fit ? words.actual : words.fit;
        if (!raw.width) return;
        const ratio = window.devicePixelRatio || 1;
        if (fit) {
            const room = host.clientWidth, tall = innerHeight - bar.offsetHeight - 24;
            const scale = Math.min(room / raw.width, tall / raw.height);
            shown.style.width = Math.max(1, Math.floor(raw.width * scale)) + 'px';
            shown.style.height = Math.max(1, Math.floor(raw.height * scale)) + 'px';
        } else {
            shown.style.width = raw.width / ratio + 'px';
            shown.style.height = raw.height / ratio + 'px';
        }
    }
    function full() {
        const el = document.documentElement;
        if (document.fullscreenElement) document.exitFullscreen();
        else if (el.requestFullscreen) el.requestFullscreen();
    }
    function record() {
        if (recorder) {
            recorder.stop();
            return;
        }
        chunks = [];
        try {
            recorder = new MediaRecorder(shown.captureStream(15));
        } catch (e) {
            recorder = null;
            return;
        }
        recorder.ondataavailable = e => { if (e.data && e.data.size) chunks.push(e.data); };
        recorder.onstop = () => {
            const blob = new Blob(chunks, {type: recorder.mimeType || 'video/webm'});
            recorder = null;
            recordButton.textContent = words.record;
            const a = document.createElement('a');
            a.href = URL.createObjectURL(blob);
            a.download = 'Grimoire live view.' + (blob.type.indexOf('mp4') >= 0 ? 'mp4' : 'webm');
            document.body.append(a);
            a.click();
            setTimeout(() => { URL.revokeObjectURL(a.href); a.remove(); }, 1000);
        };
        recorder.start(1000);
        recordButton.textContent = words.stop;
    }
    // The rect of raw drawn again on the page: as it is, inverted, or with the paper made clear
    // (each pixel the ink's color, black or inverted white, as opaque as it is dark), for a video
    // over another picture. In the pixels themselves, so a recording has them as they show.
    function repaint(x, y, w, h) {
        if (!w || !h) return;
        const inv = inverted();
        if (!clear && !inv) {
            shownCtx.clearRect(x, y, w, h);
            shownCtx.drawImage(raw, x, y, w, h, x, y, w, h);
            return;
        }
        const img = rawCtx.getImageData(x, y, w, h), p = img.data, ink = inv ? 255 : 0;
        for (let i = 0; i < p.length; i += 4) {
            const dark = 255 - p[i];
            if (clear) {
                p[i] = p[i + 1] = p[i + 2] = ink;
                p[i + 3] = dark;
            } else {
                p[i] = p[i + 1] = p[i + 2] = dark;
            }
        }
        shownCtx.putImageData(img, x, y);
    }
    function union(a, b) {
        const x = Math.min(a.x, b.x), y = Math.min(a.y, b.y);
        return {x, y, w: Math.max(a.x + a.w, b.x + b.w) - x, h: Math.max(a.y + a.h, b.y + b.h) - y};
    }

    // Draws the stream until it ends: 'bye' when Live view was turned off on the Kindle.
    async function read(reader) {
        const frames = new Frames();
        let pending = [];
        for (;;) {
            const {value, done} = await reader.read();
            if (done) return null;
            for (const {type, body} of frames.push(value)) {
                const d = new DataView(body.buffer, body.byteOffset, body.length);
                if (type === 1 && body.length >= 12) {
                    const w = d.getUint16(2, true), h = d.getUint16(4, true);
                    raw.width = shown.width = w;
                    raw.height = shown.height = h;
                    rawCtx.fillStyle = '#fff';
                    rawCtx.fillRect(0, 0, w, h);
                    pending = [];
                    layout();
                } else if (type === 2 && body.length > 8) {
                    const x = d.getUint16(0, true), y = d.getUint16(2, true);
                    const w = d.getUint16(4, true), h = d.getUint16(6, true);
                    const png = new Blob([body.subarray(8)], {type: 'image/png'});
                    pending.push(createImageBitmap(png).then(bmp => ({x, y, w, h, bmp})));
                } else if (type === 3) {
                    // A batch drawn whole, in order: no half-updated screen shows.
                    const tiles = await Promise.all(pending);
                    pending = [];
                    let dirty = null;
                    for (const t of tiles) {
                        rawCtx.drawImage(t.bmp, t.x, t.y);
                        if (t.bmp.close) t.bmp.close();
                        dirty = dirty ? union(dirty, t) : {x: t.x, y: t.y, w: t.w, h: t.h};
                    }
                    if (dirty) repaint(dirty.x, dirty.y, dirty.w, dirty.h);
                    status(words.live, true);
                } else if (type === 5 && body.length >= 1) {
                    const s = body[0];
                    if (body.length >= 2 && (body[1] === 1) !== kindleInvert) {
                        kindleInvert = body[1] === 1;
                        look();
                    }
                    if (s === 0) status(words.live, true);
                    else if (s === 1) status(words.asleep);
                    else if (s === 2) status(words.pairing);
                    else if (s === 3) {
                        status(words.ended);
                        return 'bye';
                    }
                }
            }
        }
    }
    const sleep = ms => new Promise(r => setTimeout(r, ms));
    (async function run() {
        for (let delay = 500;;) {
            status(words.connecting);
            let how = null;
            try {
                const r = await fetch('/live/stream', {cache: 'no-store', credentials: 'same-origin'});
                if (r.status === 401 || r.status === 403) {
                    location.replace('/live');  // no longer paired: the code form
                    return;
                }
                if (r.ok && r.body) {
                    delay = 500;
                    how = await read(r.body.getReader());
                }
            } catch (e) { /* the Kindle went to sleep, or out of reach: try again */ }
            if (how === 'bye') {
                // Live view turned off on the Kindle: see now and then whether it is back.
                await sleep(5000);
                continue;
            }
            status(words.reconnecting);
            await sleep(delay);
            delay = Math.min(delay * 2, 8000);
        }
    })();
})();
