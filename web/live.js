// Live view: what Grimoire on the Kindle shows, as it shows it. src/live.c streams the screen as
// PNG tiles over one long response (GET /live/stream); this draws them on a canvas and keeps it
// connected. Frames are little-endian: u32 length (of the type byte and the payload), u8 type,
// payload. 1 hello (u16 version, w, h, tile; u32 epoch), 2 tile (u16 x, y, w, h; the PNG),
// 3 batch end (u32 seq, u32 the Kindle's ms), 4 ping, 5 state (u8 0 awake, 1 asleep, 2 pairing,
// 3 bye; u8 1 when Grimoire shows its colors inverted; u8 the front light's warmth, 0..24, 255 for
// none: the picture follows both unless the Invert switch or the Warmth slider here says otherwise).
// Under node (tests/live_check.js) it only exports the frame reader and lightColor.
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
    // The front light's color at a warmth (0..WARMEST, the Kindle's amber level): 6500 K at 0 (white:
    // the color is taken relative to it) to 2400 K at the warmest, in even steps of mireds (as the eye
    // sees them), from Tanner Helland's fit of the blackbody colors. [r, g, b], 0..255.
    const WARMEST = 24;
    function lightColor(level) {
        function rgb(kelvin) {
            const t = kelvin / 100;  // (at most 66 here: red is whole throughout)
            return [255, 99.4708025861 * Math.log(t) - 161.1195681661,
                t <= 19 ? 0 : 138.5177312231 * Math.log(t - 10) - 305.0447927307];
        }
        const w = Math.min(Math.max(level, 0), WARMEST) / WARMEST;
        const c = rgb(1e6 / (1e6 / 6500 + (1e6 / 2400 - 1e6 / 6500) * w)), white = rgb(6500);
        return c.map((v, i) => Math.round(Math.min(255, Math.max(0, v * 255 / white[i]))));
    }
    if (typeof module !== 'undefined' && module.exports) {
        module.exports = {Frames, lightColor};
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
    const shown = document.createElement('canvas');    // how the page shows it, at its size (what Record takes)
    const shownCtx = shown.getContext('2d');
    const view = document.createElement('canvas');     // shown at the size it takes on this screen
    const viewCtx = view.getContext('2d');
    view.className = 'screen';
    let presentQueued = false, dirty = null;           // (present's rect still to draw, in shown's pixels)
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
    const bat = document.getElementById('live-bat');  // Grimoire's pixel bat (httpd.c), first in the bar
    if (bat && bat.content) bar.append(bat.content.cloneNode(true));
    bar.append(pill);
    // invertChoice, warmthChoice: null while the picture follows the Kindle's Invert colors and front
    // light warmth, else this page's own. kindleWarmth: null on a Kindle without warm light.
    let fit = true, clear = false, recorder = null, chunks = [], kindleInvert = false, invertChoice = null;
    let kindleWarmth = null, warmthChoice = null, light = [255, 255, 255];
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
    const warmthLabel = document.createElement('label');
    warmthLabel.className = 'warmth';
    const warmthSlider = document.createElement('input');
    warmthSlider.type = 'range';
    warmthSlider.min = '0';
    warmthSlider.max = String(WARMEST);
    warmthSlider.step = '1';
    warmthSlider.addEventListener('input', () => {
        const want = Number(warmthSlider.value);
        warmthChoice = want === (kindleWarmth || 0) ? null : want;  // (the Kindle's: following again)
        look();
    });
    warmthLabel.append(words.warmth, warmthSlider);
    bar.append(warmthLabel);
    const followButton = button(words.follow, () => {
        invertChoice = warmthChoice = null;
        look();
    });
    look();
    const recordButton = button(words.record, record);
    if (!window.MediaRecorder || !shown.captureStream) recordButton.hidden = true;
    const pipButton = button(words.pip, pip);
    pipButton.setAttribute('aria-pressed', 'false');
    if (!document.pictureInPictureEnabled || !shown.captureStream ||
        !HTMLVideoElement.prototype.requestPictureInPicture) pipButton.hidden = true;
    let pipVideo = null;
    button(words.full, full);
    host.replaceChildren(bar, view);
    addEventListener('keydown', e => {
        if ((e.key === 'f' || e.key === 'F') && !e.metaKey && !e.ctrlKey && !e.altKey) full();
    });
    addEventListener('resize', layout);

    function inverted() { return invertChoice === null ? kindleInvert : invertChoice; }
    function warmth() { return warmthChoice === null ? (kindleWarmth || 0) : warmthChoice; }
    // The Invert switch and the Warmth slider as they stand, "As on the Kindle" only while either says
    // otherwise, the page (black while inverted) and the picture.
    function look() {
        const on = inverted();
        invertSwitch.setAttribute('aria-checked', on ? 'true' : 'false');
        warmthSlider.value = String(warmth());
        followButton.hidden = (invertChoice === null || invertChoice === kindleInvert) &&
            (warmthChoice === null || warmthChoice === (kindleWarmth || 0));
        document.body.classList.toggle('inverted', on);
        light = lightColor(warmth());
        document.body.style.setProperty('--light', 'rgb(' + light.join(',') + ')');
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
        let w = raw.width / ratio, h = raw.height / ratio;
        if (fit) {
            const room = host.clientWidth, tall = innerHeight - bar.offsetHeight - 24;
            const scale = Math.min(room / raw.width, tall / raw.height);
            w = Math.max(1, Math.floor(raw.width * scale));
            h = Math.max(1, Math.floor(raw.height * scale));
        }
        view.style.width = w + 'px';
        view.style.height = h + 'px';
        view.width = Math.max(1, Math.round(w * ratio));
        view.height = Math.max(1, Math.round(h * ratio));
        presentSoon(0, 0, raw.width, raw.height);
    }
    // shown drawn into view, which has a pixel for each of the screen's. The browser's own scaling (of
    // a canvas, and drawImage even at "high") picks pixels, so 1 px lines came out bright, dim or gone
    // by where they fell: here each view pixel is the mean of the area of shown it covers (alpha-
    // weighted, for transparent paper), so every line keeps its weight. Only the rect that changed
    // (in shown's pixels); at full size or larger, a plain draw.
    function present() {
        presentQueued = false;
        const d = dirty;
        dirty = null;
        if (!raw.width || !d) return;
        const kx = shown.width / view.width, ky = shown.height / view.height;
        if (kx <= 1 && ky <= 1) {
            viewCtx.imageSmoothingEnabled = true;
            viewCtx.clearRect(0, 0, view.width, view.height);
            viewCtx.drawImage(shown, 0, 0, view.width, view.height);
            return;
        }
        const vx0 = Math.max(0, Math.floor(d.x / kx)), vy0 = Math.max(0, Math.floor(d.y / ky));
        const vx1 = Math.min(view.width, Math.ceil((d.x + d.w) / kx));
        const vy1 = Math.min(view.height, Math.ceil((d.y + d.h) / ky));
        if (vx1 <= vx0 || vy1 <= vy0) return;
        const sx0 = Math.floor(vx0 * kx), sy0 = Math.floor(vy0 * ky);
        const sx1 = Math.min(shown.width, Math.ceil(vx1 * kx)), sy1 = Math.min(shown.height, Math.ceil(vy1 * ky));
        const sw = sx1 - sx0, sh = sy1 - sy0, ow = vx1 - vx0, oh = vy1 - vy0;
        const src = shownCtx.getImageData(sx0, sy0, sw, sh).data;
        // Each output column (row) as source columns (rows) and how much of each it covers.
        function spans(v0, n, k, s0, limit) {
            const out = [];
            for (let i = 0; i < n; i++) {
                const a = (v0 + i) * k - s0, b = Math.min(a + k, limit), cols = [];
                for (let s = Math.floor(a); s < b; s++) cols.push(s, (Math.min(b, s + 1) - Math.max(a, s)) / (b - a));
                out.push(cols);
            }
            return out;
        }
        const xs = spans(vx0, ow, kx, sx0, sw), ys = spans(vy0, oh, ky, sy0, sh);
        // Across: sh rows of ow pixels, premultiplied.
        const mid = new Float32Array(sh * ow * 4);
        for (let r = 0; r < sh; r++)
            for (let o = 0; o < ow; o++) {
                const cols = xs[o];
                let cr = 0, cg = 0, cb = 0, ca = 0;
                for (let j = 0; j < cols.length; j += 2) {
                    const i = (r * sw + cols[j]) * 4, wa = cols[j + 1] * src[i + 3];
                    cr += src[i] * wa;
                    cg += src[i + 1] * wa;
                    cb += src[i + 2] * wa;
                    ca += wa;
                }
                const m = (r * ow + o) * 4;
                mid[m] = cr;
                mid[m + 1] = cg;
                mid[m + 2] = cb;
                mid[m + 3] = ca;
            }
        // Down, then back from premultiplied.
        const img = viewCtx.createImageData(ow, oh), p = img.data;
        for (let o = 0; o < oh; o++) {
            const rows = ys[o];
            for (let c = 0; c < ow; c++) {
                let cr = 0, cg = 0, cb = 0, ca = 0;
                for (let j = 0; j < rows.length; j += 2) {
                    const m = (rows[j] * ow + c) * 4, w = rows[j + 1];
                    cr += mid[m] * w;
                    cg += mid[m + 1] * w;
                    cb += mid[m + 2] * w;
                    ca += mid[m + 3] * w;
                }
                const q = (o * ow + c) * 4;
                if (ca > 0) {
                    p[q] = cr / ca;
                    p[q + 1] = cg / ca;
                    p[q + 2] = cb / ca;
                }
                p[q + 3] = ca;
            }
        }
        viewCtx.putImageData(img, vx0, vy0);
    }
    function presentSoon(x, y, w, h) {
        const r = {x, y, w, h};
        dirty = dirty ? union(dirty, r) : r;
        if (presentQueued) return;
        presentQueued = true;
        requestAnimationFrame(present);
    }
    function full() {
        const el = document.documentElement;
        if (document.fullscreenElement) document.exitFullscreen();
        else if (el.requestFullscreen) el.requestFullscreen();
    }
    // Picture in picture: shown, as Record takes it, played in a video the browser floats over other
    // windows. It goes on while this tab is hidden: shown is drawn as tiles come, not by frames.
    async function pip() {
        if (document.pictureInPictureElement) {
            document.exitPictureInPicture().catch(() => {});
            return;
        }
        if (!pipVideo) {
            pipVideo = document.createElement('video');
            pipVideo.muted = true;
            pipVideo.playsInline = true;
            pipVideo.className = 'pip';  // (in the page, out of sight: some browsers float only those)
            pipVideo.srcObject = shown.captureStream();
            pipVideo.addEventListener('enterpictureinpicture', () => pipButton.setAttribute('aria-pressed', 'true'));
            pipVideo.addEventListener('leavepictureinpicture', () => pipButton.setAttribute('aria-pressed', 'false'));
            document.body.append(pipVideo);
            repaint(0, 0, raw.width, raw.height);  // a first frame, so the video knows its size
        }
        try {
            await pipVideo.play();
            await pipVideo.requestPictureInPicture();
        } catch (e) { /* refused (no picture yet, or not from a tap): the button stays as it was */ }
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
    // over another picture; in the front light's color (lightColor) where it is light. In the pixels
    // themselves, so a recording has them as they show.
    function repaint(x, y, w, h) {
        if (!w || !h) return;
        const inv = inverted(), [r, g, b] = light, white = r === 255 && g === 255 && b === 255;
        if (!clear && !inv && white) {
            shownCtx.clearRect(x, y, w, h);
            shownCtx.drawImage(raw, x, y, w, h, x, y, w, h);
            presentSoon(x, y, w, h);
            return;
        }
        const img = rawCtx.getImageData(x, y, w, h), p = img.data;
        for (let i = 0; i < p.length; i += 4) {
            const dark = 255 - p[i], lit = (inv ? dark : p[i]) / 255;
            if (clear) {
                if (inv) {
                    p[i] = r;
                    p[i + 1] = g;
                    p[i + 2] = b;
                } else {
                    p[i] = p[i + 1] = p[i + 2] = 0;
                }
                p[i + 3] = dark;
            } else {
                p[i] = r * lit;
                p[i + 1] = g * lit;
                p[i + 2] = b * lit;
            }
        }
        shownCtx.putImageData(img, x, y);
        presentSoon(x, y, w, h);
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
                    const inv = body.length >= 2 ? body[1] === 1 : kindleInvert;
                    const warm = body.length >= 3 ? (body[2] === 255 ? null : body[2]) : kindleWarmth;
                    if (inv !== kindleInvert || warm !== kindleWarmth) {
                        kindleInvert = inv;
                        kindleWarmth = warm;
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
