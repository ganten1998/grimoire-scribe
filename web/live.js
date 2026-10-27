// Live view: what Grimoire on the Kindle shows, as it shows it. src/live.c streams the screen as
// PNG tiles over one long response (GET /live/stream); this draws them on a canvas and keeps it
// connected. Frames are little-endian: u32 length (of the type byte and the payload), u8 type,
// payload. 1 hello (u16 version, w, h, tile; u32 epoch), 2 tile (u16 x, y, w, h; the PNG),
// 3 batch end (u32 seq, u32 the Kindle's ms), 4 ping, 5 state (u8 0 awake, 1 asleep, 2 pairing,
// 3 bye; u8 1 when Grimoire shows its colors inverted; u8 the front light's warmth, 0..24, 255 for
// none: the picture follows both unless the Invert switch or the Warmth slider here says otherwise),
// 6 pen (u16 x, y; u8 0 away, 1 hovering, 2 touching: drawn over the picture, never in it; a hello
// or any state but awake takes it away). Under node (tests/live_check.js) it only exports the frame
// reader and the pure parts below.
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
    // A pen frame's place: {x, y, state} (1 hovering, 2 touching), or null while the pen is away.
    function readPen(body) {
        if (body.length < 5 || !body[4]) return null;
        return {x: body[0] | body[1] << 8, y: body[2] | body[3] << 8, state: body[4]};
    }
    // The pen's mark in a canvas with k of its pixels to each of the screen's, the screen's corner at
    // (ox, oy), and u of its pixels to a unit (a CSS pixel on the page): a ring while the pen hovers, a
    // dot while it touches, each with a light halo so it shows on paper, ink, the warm light and
    // inverted alike. box: the whole pixels it covers, to take it away again. null without a pen.
    const PEN_RING = 9, PEN_DOT = 4.5, PEN_LINE = 2, PEN_HALO = 1.5;
    function penMark(pen, k, u, ox = 0, oy = 0) {
        if (!pen || !pen.state) return null;
        const ring = pen.state === 1, x = ox + (pen.x + 0.5) * k, y = oy + (pen.y + 0.5) * k;
        const r = (ring ? PEN_RING : PEN_DOT) * u, line = PEN_LINE * u, halo = PEN_HALO * u;
        const out = r + (ring ? line / 2 : 0) + halo + 1;  // (1 more for the antialiased edge)
        const x0 = Math.floor(x - out), y0 = Math.floor(y - out);
        return {x, y, r, line, halo, ring, box: {x: x0, y: y0, w: Math.ceil(x + out) - x0, h: Math.ceil(y + out) - y0}};
    }
    // Where a w x h picture goes in a fw x fh film: whole, centered, scale k. A recording keeps the
    // size it started with (an MP4's track cannot change it: Chrome's frames after a turn came out
    // corrupt), so the turned screen goes in it smaller.
    function filmFit(w, h, fw, fh) {
        if (w === fw && h === fh) return {k: 1, x: 0, y: 0};
        const k = Math.min(fw / w, fh / h);
        return {k, x: Math.floor((fw - w * k) / 2), y: Math.floor((fh - h * k) / 2)};
    }
    // Record's format, from what supported(type) says (MediaRecorder.isTypeSupported): MP4 (H.264)
    // wherever the browser records it, Transparent paper too (an MP4 has no transparency, so the
    // paper is drawn in); WebM only where it cannot (Firefox: why 'browser'). A browser that names
    // neither records its own choice, which nothing here can name.
    const MP4 = ['video/mp4;codecs=avc1.42E01E', 'video/mp4;codecs=avc1', 'video/mp4'];
    const WEBM = ['video/webm;codecs=vp8', 'video/webm;codecs=vp9', 'video/webm'];
    function recordType(supported) {
        const first = list => list.find(t => { try { return !!supported(t); } catch (e) { return false; } });
        const mp4 = first(MP4), webm = first(WEBM);
        if (mp4) return {mime: mp4, ext: 'mp4', why: ''};
        if (webm) return {mime: webm, ext: 'webm', why: 'browser'};
        return {mime: '', ext: '', why: ''};  // (the file named by the type the recorder says it made)
    }
    if (typeof module !== 'undefined' && module.exports) {
        module.exports = {Frames, lightColor, readPen, penMark, filmFit, recordType};
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
    const penLayer = document.createElement('canvas'); // the pen over view: never in its averaged pixels
    const penCtx = penLayer.getContext('2d');
    penLayer.className = 'pen';
    const stage = document.createElement('div');
    stage.className = 'stage';
    stage.append(view, penLayer);
    const film = document.createElement('canvas');     // shown with the pen on it: what Record and PiP take
    const filmCtx = film.getContext('2d');
    // pen: where the Kindle's pen is (readPen), null while away; penShown, filmPen: its marks drawn on
    // the page and in film now, to take away; fitIn: shown's place in film (filmFit); flat: a
    // recording that is no WebM (an MP4), which has no transparency, so film has the paper drawn in
    // while Transparent paper is on.
    let pen = null, showPen = true, penShown = null, penQueued = false, filmPen = null;
    let fitIn = {k: 1, x: 0, y: 0}, flat = false, recordAs = null, pipVideo = null;
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
    const penSwitch = button(words.pen, () => {
        showPen = !showPen;
        penSwitch.setAttribute('aria-checked', showPen ? 'true' : 'false');
        movePen(pen);
    });
    penSwitch.setAttribute('role', 'switch');
    penSwitch.className = 'switch';
    penSwitch.setAttribute('aria-checked', 'true');
    const recordButton = button(words.record, record);
    const canRecord = !!(window.MediaRecorder && film.captureStream);
    if (!canRecord) recordButton.hidden = true;
    const pipButton = button(words.pip, pip);
    pipButton.setAttribute('aria-pressed', 'false');
    if (!document.pictureInPictureEnabled || !film.captureStream ||
        !HTMLVideoElement.prototype.requestPictureInPicture) pipButton.hidden = true;
    button(words.full, full);
    recordLabel();
    host.replaceChildren(bar, stage);
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
        penLayer.style.left = view.offsetLeft + view.clientLeft + 'px';  // (inside view's border)
        penLayer.style.top = view.offsetTop + view.clientTop + 'px';
        penLayer.style.width = w + 'px';
        penLayer.style.height = h + 'px';
        penLayer.width = view.width;  // (which clears it)
        penLayer.height = view.height;
        penShown = null;
        penSoon();
        presentSoon(0, 0, raw.width, raw.height);
    }
    // The pen on the page, at the next frame: its mark on penLayer, over view, in view's pixels.
    function penSoon() {
        if (penQueued) return;
        penQueued = true;
        requestAnimationFrame(drawPen);
    }
    function drawPen() {
        penQueued = false;
        if (penShown) penCtx.clearRect(penShown.x, penShown.y, penShown.w, penShown.h);
        const m = showPen && raw.width ? penMark(pen, view.width / raw.width, window.devicePixelRatio || 1) : null;
        penShown = m && m.box;
        if (m) mark(penCtx, m);
    }
    const PEN_COLOR = '#f03c28', PEN_HALO_COLOR = 'rgba(255,255,255,0.9)';
    function mark(ctx, m) {
        ctx.beginPath();
        ctx.arc(m.x, m.y, m.r, 0, 2 * Math.PI);
        ctx.strokeStyle = PEN_HALO_COLOR;
        ctx.lineWidth = m.ring ? m.line + 2 * m.halo : 2 * m.halo;
        ctx.stroke();
        if (m.ring) {
            ctx.strokeStyle = PEN_COLOR;
            ctx.lineWidth = m.line;
            ctx.stroke();
        } else {
            ctx.fillStyle = PEN_COLOR;
            ctx.fill();
        }
    }
    // The pen moved, came, went, or was switched off: on the page at the next frame, in film now (it
    // is drawn as frames come, so a recording and Picture in picture go on while the tab is hidden).
    function movePen(next) {
        pen = next;
        const was = filmPen;
        filmPen = filmMark();
        if (was) filmDraw(was.box.x, was.box.y, was.box.w, was.box.h);
        if (filmPen) filmDraw(filmPen.box.x, filmPen.box.y, filmPen.box.w, filmPen.box.h);
        penSoon();
    }
    // film is kept only while something takes it: a recording, or Picture in picture showing.
    function filming() { return !!recorder || (!!pipVideo && document.pictureInPictureElement === pipVideo); }
    // The mark's unit in film, which has the screen's own pixels: about as big as on a laptop at Fit.
    function filmMark() {
        if (!filming() || !showPen) return null;
        const unit = Math.max(1, Math.round(Math.min(film.width, film.height) / 620));
        return penMark(pen, fitIn.k, unit, fitIn.x, fitIn.y);
    }
    // A rect of film (its own pixels) drawn again: shown there (the paper first while it is clear in
    // a recording with no transparency), then the pen's mark where it falls. Only while filming,
    // unless forced (a first frame for the video).
    function filmDraw(x, y, w, h, force) {
        if (!force && !filming()) return;
        const x0 = Math.max(0, Math.floor(x)), y0 = Math.max(0, Math.floor(y));
        const x1 = Math.min(film.width, Math.ceil(x + w)), y1 = Math.min(film.height, Math.ceil(y + h));
        if (x1 <= x0 || y1 <= y0) return;
        filmCtx.save();
        filmCtx.beginPath();
        filmCtx.rect(x0, y0, x1 - x0, y1 - y0);
        filmCtx.clip();
        filmCtx.clearRect(x0, y0, x1 - x0, y1 - y0);
        const pw = shown.width * fitIn.k, ph = shown.height * fitIn.k;
        if (flat && clear) {
            filmCtx.fillStyle = inverted() ? '#000' : 'rgb(' + light.join(',') + ')';
            filmCtx.fillRect(fitIn.x, fitIn.y, pw, ph);
        }
        const cw = x1 - x0, ch = y1 - y0;
        if (fitIn.k === 1) filmCtx.drawImage(shown, x0 - fitIn.x, y0 - fitIn.y, cw, ch, x0, y0, cw, ch);
        else filmCtx.drawImage(shown, fitIn.x, fitIn.y, pw, ph);
        if (filmPen) mark(filmCtx, filmPen);
        filmCtx.restore();
    }
    // All of film again, at the size it keeps (a recording's) or shown's: its mark worked out anew.
    function filmAll(force) {
        if (!recorder && (film.width !== shown.width || film.height !== shown.height)) {
            film.width = shown.width;
            film.height = shown.height;
        }
        fitIn = filmFit(shown.width, shown.height, film.width, film.height);
        filmPen = filmMark();
        filmDraw(0, 0, film.width, film.height, force);
    }
    // A rect of shown (its pixels) changed: the same in film.
    function filmShown(x, y, w, h) {
        if (!filming()) return;
        if (fitIn.k !== 1) filmDraw(0, 0, film.width, film.height);  // (scaled: all of it, no seams)
        else filmDraw(x + fitIn.x, y + fitIn.y, w, h);
    }
    // Record's button: Record, or Record (WebM) with why in its tooltip where it cannot be an MP4.
    function recordLabel() {
        if (!canRecord) return;
        if (recorder) {
            recordButton.textContent = words.stop;
            return;
        }
        recordAs = recordType(t => MediaRecorder.isTypeSupported(t));
        recordButton.textContent = recordAs.ext === 'webm' ? words.recordWebm : words.record;
        recordButton.title = recordAs.why ? words.webmBrowser : '';
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
    // Picture in picture: film, as Record takes it (shown with the pen), played in a video the browser
    // floats over other windows. It goes on while this tab is hidden: film is drawn as tiles and pen
    // frames come, not by animation frames.
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
            pipVideo.srcObject = film.captureStream();
            pipVideo.addEventListener('enterpictureinpicture', () => {
                pipButton.setAttribute('aria-pressed', 'true');
                filmAll();  // (what came while it opened)
            });
            pipVideo.addEventListener('leavepictureinpicture', () => pipButton.setAttribute('aria-pressed', 'false'));
            document.body.append(pipVideo);
            filmAll(true);  // a first frame, so the video knows its size
        }
        try {
            await pipVideo.play();
            await pipVideo.requestPictureInPicture();
        } catch (e) { /* refused (no picture yet, or not from a tap): the button stays as it was */ }
    }
    // Record: film (shown and the pen) at 15 frames a second, as an MP4 where the browser records one
    // (recordType). film keeps its size until the recording stops (filmFit).
    function record() {
        if (recorder) {
            recorder.stop();
            return;
        }
        if (!raw.width) return;
        recordLabel();
        const as = recordAs;
        film.width = shown.width;
        film.height = shown.height;
        flat = as.ext !== 'webm';
        chunks = [];
        try {
            recorder = new MediaRecorder(film.captureStream(15), as.mime ? {mimeType: as.mime} : undefined);
        } catch (e) {
            recorder = null;
            flat = false;
            return;
        }
        filmAll();
        recorder.ondataavailable = e => { if (e.data && e.data.size) chunks.push(e.data); };
        recorder.onstop = () => {
            const type = recorder.mimeType || as.mime || 'video/webm';
            const blob = new Blob(chunks, {type});
            recorder = null;
            flat = false;
            recordLabel();
            if (filming()) filmAll();  // (Picture in picture goes on at shown's size)
            const a = document.createElement('a');
            a.href = URL.createObjectURL(blob);
            a.download = 'Grimoire live view.' + (as.ext || (type.indexOf('mp4') >= 0 ? 'mp4' : 'webm'));
            document.body.append(a);
            a.click();
            setTimeout(() => { URL.revokeObjectURL(a.href); a.remove(); }, 1000);
        };
        recorder.start(1000);
        recordLabel();
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
            filmShown(x, y, w, h);
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
        filmShown(x, y, w, h);
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
                    pen = null;  // (a hello takes the pen away: the next pen frame is on the new screen)
                    layout();
                    if (filming()) filmAll();
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
                } else if (type === 6 && body.length >= 5) {
                    movePen(readPen(body));
                } else if (type === 5 && body.length >= 1) {
                    const s = body[0];
                    const inv = body.length >= 2 ? body[1] === 1 : kindleInvert;
                    const warm = body.length >= 3 ? (body[2] === 255 ? null : body[2]) : kindleWarmth;
                    if (inv !== kindleInvert || warm !== kindleWarmth) {
                        kindleInvert = inv;
                        kindleWarmth = warm;
                        look();
                    }
                    if (s !== 0 && pen) movePen(null);  // (any state but awake takes the pen away)
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
            if (pen) movePen(null);  // (a new stream starts without one)
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
