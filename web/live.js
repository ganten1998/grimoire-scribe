// Live view: what Grimoire on the Kindle shows, as it shows it. src/live.c (rust/src/live.rs)
// streams the screen as PNG tiles over one long response (GET /live/stream); this draws them on a
// canvas and keeps it connected. The pen comes on a stream of its own (GET /live/pen): its samples
// draw a quick line on a canvas over the picture (fast ink), played out a few milliseconds behind
// the pen at the pace the Kindle read it (Playout), so a burst of samples draws as a smooth line
// and a gliding dot; the tiles' real ink replaces it as their batches say it reaches past it,
// while PNGs are still being decoded. Frames are little-endian: u32 length (of the type byte and
// the payload), u8 type, payload. 1 hello (u16 version 3, w, h, tile; u32 the screen's epoch, or
// on /live/pen the ink epoch), 2 tile (u16 x, y, w, h; the PNG), 3 batch end (u32 seq, u32 the
// Kindle's ms, u32 the newest pen sample's identity its ink holds, u32 that sample's ms, u32 the
// ink epoch it was presented in), 4 ping, 5 state (u8 0 awake, 1 asleep, 2 pairing, 3 bye; u8 1
// when Grimoire shows its colors inverted; u8 the front light's warmth, 0..24, 255 for none: the
// picture follows both unless the Invert switch or the Warmth slider here says otherwise), 6 pen
// (u16 x, y; u8 0 away, 1 hovering, 2 touching: drawn over the picture, never in it, while there
// is no /live/pen; a hello or any state but awake takes it away), 7 samples (u32 ink epoch, u32
// reset, then 20 bytes a sample, the first u32 its identity: the kernel's time of its frame in
// microseconds), 8 ink style (64 bytes), 9 layer (u8 0 the toolbar, 1 the clock and battery
// corner; u8 0; u16 x, y, w, h; the PNG, none while it is not on the screen: drawn over the page
// unless switched off here). src/live.h (live.rs's head) has them byte by byte. Under node
// (tests/live_check.js) it only exports the frame reader, the pure parts below and CodecRecorder
// (which the check runs on a stand-in encoder); tests/live_browser_check.js runs the page itself
// on DOM and canvas stand-ins.
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
    // (ox, oy), and u of its pixels to a unit (a CSS pixel on the page): a dot of the chosen color
    // (dot.color) and size (dot.size: DOT_SIZES), see-through while the pen hovers (alpha, the color's
    // opacity: the writing under it shows) and smaller and solid while it touches (less of the line it
    // draws hidden), inside a light edge and a dark one so it shows on paper, ink, mid gray, the warm
    // light and inverted alike. box: the whole pixels it covers, to take it away again. null without
    // a pen.
    const DOT_SIZES = {small: [4, 3], medium: [7, 5], large: [11, 8]};
    const DOT_COLORS = ['#f03c28', '#2f6fe4', '#1e9e4a', '#ff8a00', '#b02fd0'];
    const DOT = {color: DOT_COLORS[0], size: 'small'};
    const PEN_EDGE = 1, PEN_SEE = 0.55;
    function penMark(pen, k, u, ox = 0, oy = 0, dot = DOT) {
        if (!pen || !pen.state) return null;
        const hover = pen.state === 1, x = ox + (pen.x + 0.5) * k, y = oy + (pen.y + 0.5) * k;
        const radii = DOT_SIZES[dot.size] || DOT_SIZES.small;
        const r = (hover ? radii[0] : radii[1]) * u, edge = PEN_EDGE * u;
        const out = r + 2 * edge + 1;  // (1 more for the antialiased edge)
        const x0 = Math.floor(x - out), y0 = Math.floor(y - out);
        return {x, y, r, edge, alpha: hover ? PEN_SEE : 1, color: dot.color || DOT_COLORS[0],
                box: {x: x0, y: y0, w: Math.ceil(x + out) - x0, h: Math.ceil(y + out) - y0}};
    }
    // A dot color as the page remembers it: #rrggbb, else the first preset.
    function dotColor(v) { return /^#[0-9a-f]{6}$/i.test(String(v)) ? String(v).toLowerCase() : DOT_COLORS[0]; }
    function dotSize(v) { return Object.prototype.hasOwnProperty.call(DOT_SIZES, v) ? v : 'small'; }
    // The mark drawn: the dark edge outermost, the light one inside it, then the color. Only the color
    // is see-through while hovering: a faint edge would lose the dot on a page of its own shade. (Here
    // with the pure parts so tests/live_check.js can draw it on a stand-in context.)
    const PEN_LIGHT = 'rgba(255,255,255,0.9)', PEN_DARK = 'rgba(0,0,0,0.45)';
    function drawMark(ctx, m) {
        ctx.save();
        ctx.lineWidth = m.edge;
        ctx.strokeStyle = PEN_DARK;
        ctx.beginPath();
        ctx.arc(m.x, m.y, m.r + 1.5 * m.edge, 0, 2 * Math.PI);
        ctx.stroke();
        ctx.strokeStyle = PEN_LIGHT;
        ctx.beginPath();
        ctx.arc(m.x, m.y, m.r + 0.5 * m.edge, 0, 2 * Math.PI);
        ctx.stroke();
        ctx.globalAlpha = m.alpha;
        ctx.fillStyle = m.color || DOT_COLORS[0];
        ctx.beginPath();
        ctx.arc(m.x, m.y, m.r, 0, 2 * Math.PI);
        ctx.fill();
        ctx.restore();
    }
    // Where a w x h picture goes in a fw x fh film: whole, centered, scale k. A recording keeps the
    // size it started with (an MP4's track cannot change it: Chrome's frames after a turn came out
    // corrupt), so the turned screen goes in it smaller.
    function filmFit(w, h, fw, fh) {
        if (w === fw && h === fh) return {k: 1, x: 0, y: 0};
        const k = Math.min(fw / w, fh / h);
        return {k, x: Math.floor((fw - w * k) / 2), y: Math.floor((fh - h * k) / 2)};
    }
    // Record's format: an MP4 wherever the browser can make one, Transparent paper too (an MP4 has
    // no transparency, so the paper is drawn in). supported(type) is MediaRecorder.isTypeSupported;
    // h264, what probeH264 found VideoEncoder takes (filmChoices' first that it supports: {codec,
    // width, height, format, hw}), null for none. First MediaRecorder's own MP4 (Chrome, Safari: via
    // 'recorder'), else the page's own (via 'codecs': Firefox's MediaRecorder writes only WebM and
    // Ogg, but its VideoEncoder makes H.264, which CodecRecorder puts in an MP4), and WebM only where
    // neither can (why 'browser'). A browser that names none records its own choice, which nothing
    // here can name.
    const MP4 = ['video/mp4;codecs=avc1.42E01E', 'video/mp4;codecs=avc1', 'video/mp4'];
    const WEBM = ['video/webm;codecs=vp8', 'video/webm;codecs=vp9', 'video/webm'];
    function recordType(supported, h264) {
        const first = list => list.find(t => { try { return !!supported(t); } catch (e) { return false; } });
        const mp4 = first(MP4), webm = first(WEBM);
        if (mp4) return {mime: mp4, ext: 'mp4', why: '', via: 'recorder'};
        if (h264) return {mime: 'video/mp4', ext: 'mp4', why: '', via: 'codecs', codec: h264.codec, film: h264};
        if (webm) return {mime: webm, ext: 'webm', why: 'browser', via: 'recorder'};
        return {mime: '', ext: '', why: '', via: 'recorder'};  // (the file named by the type the recorder says it made)
    }
    // The H.264 asked of VideoEncoder: Baseline only, which has no B frames, so the frames come in
    // the order they show and mp4Boxes needs no reordering table. Constrained Baseline first (what
    // hardware encoders and Firefox's take), plain Baseline next, at levels 5.1, 5.0 (enough for
    // both Scribes' screens: 1986 x 2648 is 20,750 macroblocks) and 4.2 and 3.1 for a smaller film.
    const H264 = ['avc1.42E033', 'avc1.420033', 'avc1.42E032', 'avc1.42E02A', 'avc1.42001f'];
    const FILM_FPS = 15, FILM_BITRATE = 4e6;
    function filmConfig(codec, width, height, format = 'avc', hw = '') {
        const c = {codec, width, height, bitrate: FILM_BITRATE, framerate: FILM_FPS, avc: {format}};
        if (hw) c.hardwareAcceleration = hw;
        return c;
    }
    // What probeH264 asks VideoEncoder, in order, for a w x h screen: the screen's own size, then
    // smaller ones (a hardware encoder may take no side over 2304, or 1920: Firefox's refused the
    // Scribe's 2480 px), each with the length-prefixed form (description: avcC) and Annex B (start
    // codes, the parameter sets in the stream: annexbSample makes the MP4's samples of it), and the
    // browser's own choice of hardware before asking for software or hardware outright. Sizes are
    // even (4:2:0).
    function filmChoices(w, h) {
        const even = v => Math.max(2, 2 * Math.round(v / 2));
        const sizes = [];
        for (const box of [Infinity, 2304, 1920, 1280]) {
            const k = Math.min(1, box / Math.max(w, h));
            const s = {width: even(w * k), height: even(h * k)};
            if (!sizes.some(t => t.width === s.width && t.height === s.height)) sizes.push(s);
        }
        const out = [];
        for (const size of sizes)
            for (const hw of ['', 'prefer-software', 'prefer-hardware'])
                for (const format of ['avc', 'annexb'])
                    for (const codec of H264) out.push({codec, width: size.width, height: size.height, format, hw});
        return out;
    }
    // Annex B's NAL units (after each 00 00 01 or 00 00 00 01) as views of data.
    function annexbNals(data) {
        const out = [];
        let start = -1;
        for (let i = 0; i + 2 < data.length; i++) {
            if (data[i] === 0 && data[i + 1] === 0 && data[i + 2] === 1) {
                if (start >= 0) {
                    let end = i;
                    while (end > start && data[end - 1] === 0) end--;  // (the 4-byte code's leading zero)
                    out.push(data.subarray(start, end));
                }
                start = i + 3;
                i += 2;
            }
        }
        if (start >= 0 && start < data.length) out.push(data.subarray(start));
        return out;
    }
    // An Annex B chunk as an MP4 sample: its pictures' NAL units, each after its length (4 bytes, big
    // endian); the parameter sets (7, 8: kept for avcC) and access unit delimiters (9) left out.
    // sps and pps: the parameter sets it carried, if any.
    function annexbSample(data) {
        const nals = annexbNals(data), keep = [];
        let sps = null, pps = null, n = 0;
        for (const nal of nals) {
            const type = nal[0] & 31;
            if (type === 7) sps = sps || nal;
            else if (type === 8) pps = pps || nal;
            else if (type !== 9) { keep.push(nal); n += 4 + nal.length; }
        }
        const out = new Uint8Array(n);
        let at = 0;
        for (const nal of keep) {
            out[at] = nal.length >>> 24; out[at + 1] = (nal.length >>> 16) & 255;
            out[at + 2] = (nal.length >>> 8) & 255; out[at + 3] = nal.length & 255;
            out.set(nal, at + 4);
            at += 4 + nal.length;
        }
        return {sample: out, sps, pps};
    }
    // The avcC box's body of one SPS and one PPS (Baseline: no extension fields).
    function avcCOf(sps, pps) {
        const out = new Uint8Array(11 + sps.length + pps.length);
        out.set([1, sps[1], sps[2], sps[3], 0xFF, 0xE1, sps.length >> 8, sps.length & 255]);
        out.set(sps, 8);
        out.set([1, pps.length >> 8, pps.length & 255], 8 + sps.length);
        out.set(pps, 11 + sps.length);
        return out;
    }
    // An MP4 of one H.264 track from WebCodecs' chunks, as its boxes: {ftyp, mdat (its header),
    // moov}; the file is ftyp, mdat, every sample's bytes in order, then moov, which says where each
    // one is (each sample its own chunk). samples: [{size, ts (us), key}] in the order they came;
    // avcC: the decoder config's description (the SPS and PPS); endUs: when the last one stops
    // showing. A frame comes only when the picture changed, each lasting until the next (a still
    // screen is one long frame). Past 4 GB the offsets and mdat's size take 64 bits.
    const MP4_SCALE = 30000;  // the track's ticks a second (its u32 duration holds 39 hours of them)
    function mp4Boxes(samples, avcC, width, height, endUs) {
        function be(n, size) {
            const b = new Uint8Array(size);
            for (let i = size - 1; i >= 0 && n > 0; i--, n = Math.floor(n / 256)) b[i] = n % 256;
            return b;
        }
        function cat(parts) {
            let n = 0;
            for (const p of parts) n += p.length;
            const out = new Uint8Array(n);
            for (let i = 0, at = 0; i < parts.length; at += parts[i].length, i++) out.set(parts[i], at);
            return out;
        }
        const text = s => Uint8Array.from(s, c => c.charCodeAt(0));
        const box = (type, ...parts) => { const body = cat(parts); return cat([be(body.length + 8, 4), text(type), body]); };
        const full = (type, version, flags, ...parts) => box(type, be(version, 1), be(flags, 3), ...parts);
        function table(values, size) {
            const out = new Uint8Array(values.length * size), d = new DataView(out.buffer);
            values.forEach((v, i) => {
                if (size === 8) d.setUint32(i * 8, Math.floor(v / 4294967296));
                d.setUint32(i * size + size - 4, v % 4294967296);
            });
            return out;
        }
        const ticks = us => Math.round(us * MP4_SCALE / 1e6);
        const n = samples.length, deltas = [], stts = [], keys = [], offsets = [];
        for (let i = 0; i < n; i++) {
            const next = i + 1 < n ? samples[i + 1].ts : endUs;
            const d = Math.max(1, ticks(next) - ticks(samples[i].ts));
            deltas.push(d);
            if (stts.length && stts[stts.length - 1][1] === d) stts[stts.length - 1][0]++;
            else stts.push([1, d]);
            if (samples[i].key) keys.push(i + 1);
        }
        const ftyp = box('ftyp', text('isom'), be(512, 4), text('isomiso2avc1mp41'));
        let bytes = 0;
        for (const s of samples) bytes += s.size;
        const mdat = bytes + 8 > 0xFFFFFFFF ? cat([be(1, 4), text('mdat'), be(bytes + 16, 8)])
                                            : cat([be(bytes + 8, 4), text('mdat')]);
        let at = ftyp.length + mdat.length;
        for (const s of samples) {
            offsets.push(at);
            at += s.size;
        }
        const dur = deltas.reduce((a, b) => a + b, 0), movie = Math.round(dur * 1000 / MP4_SCALE);
        const matrix = table([0x10000, 0, 0, 0, 0x10000, 0, 0, 0, 0x40000000], 4);
        const avc1 = box('avc1', be(0, 6), be(1, 2), be(0, 16), be(width, 2), be(height, 2),
            be(0x480000, 4), be(0x480000, 4), be(0, 4), be(1, 2), be(0, 32), be(0x18, 2), be(0xFFFF, 2),
            box('avcC', avcC));
        const stbl = box('stbl',
            full('stsd', 0, 0, be(1, 4), avc1),
            full('stts', 0, 0, be(stts.length, 4), table([].concat(...stts), 4)),
            full('stss', 0, 0, be(keys.length, 4), table(keys, 4)),
            full('stsc', 0, 0, be(1, 4), be(1, 4), be(1, 4), be(1, 4)),
            full('stsz', 0, 0, be(0, 4), be(n, 4), table(samples.map(s => s.size), 4)),
            at > 0xFFFFFFFF ? full('co64', 0, 0, be(n, 4), table(offsets, 8))
                            : full('stco', 0, 0, be(n, 4), table(offsets, 4)));
        const moov = box('moov',
            full('mvhd', 0, 0, be(0, 8), be(1000, 4), be(movie, 4), be(0x10000, 4), be(0x100, 2), be(0, 10),
                matrix, be(0, 24), be(2, 4)),
            box('trak',
                full('tkhd', 0, 3, be(0, 8), be(1, 4), be(0, 4), be(movie, 4), be(0, 16), matrix,
                    be(width * 65536, 4), be(height * 65536, 4)),
                box('mdia',
                    full('mdhd', 0, 0, be(0, 8), be(MP4_SCALE, 4), be(dur, 4), be(0x55C4, 2), be(0, 2)),  // ('und')
                    full('hdlr', 0, 0, be(0, 4), text('vide'), be(0, 12), text('VideoHandler\0')),
                    box('minf',
                        full('vmhd', 0, 1, be(0, 8)),
                        box('dinf', full('dref', 0, 0, be(1, 4), full('url ', 0, 1))),
                        stbl))));
        return {ftyp, mdat, moov};
    }
    // Record's MP4 where MediaRecorder writes none (recordType's 'codecs'): canvas's frames encoded
    // as H.264 by WebCodecs and put in an MP4 by mp4Boxes. frame() says the canvas changed: the frame
    // is taken once the code that changed it has run to its end (the pen's mark is drawn away and
    // back in two steps), in a microtask rather than at an animation frame, so a recording goes on
    // while the tab is hidden; at most FILM_FPS a second (one asked sooner waits on a timer), a key
    // frame when 2 s have passed since the last, so the file seeks. stop() takes the picture still
    // waiting, finishes the encoding and gives done the MP4 as a Blob, or null when nothing came
    // (the encoder refused the size, or failed before its first frame).
    function CodecRecorder(canvas, codec, done, choice) {
        this.canvas = canvas;
        this.width = canvas.width;
        this.height = canvas.height;
        this.format = (choice && choice.format) || 'avc';
        this.done = done;
        this.samples = [];  // {size, ts, key}
        this.datas = [];    // their bytes
        this.avcC = null;
        this.t0 = -1;       // performance.now() at the first frame, whose timestamp is 0
        this.last = -Infinity;    // performance.now() at the last frame taken
        this.lastTs = -Infinity;  // and its timestamp
        this.lastKey = -Infinity;
        this.soon = false;
        this.timer = 0;
        this.stopped = false;
        this.encoder = new VideoEncoder({
            output: (chunk, meta) => this.took(chunk, meta),
            error: () => this.stop(),  // (what came before it is kept)
        });
        this.encoder.configure(filmConfig(codec, this.width, this.height, this.format, (choice && choice.hw) || ''));
    }
    CodecRecorder.prototype.frame = function () {
        if (this.stopped || this.soon) return;
        const wait = this.last + 1000 / FILM_FPS - performance.now();
        if (wait > 0) {
            if (!this.timer) this.timer = setTimeout(() => { this.timer = 0; this.frame(); }, wait);
            return;
        }
        this.soon = true;
        queueMicrotask(() => {
            if (!this.soon) return;  // (taken already, by stop)
            this.soon = false;
            this.take();
        });
    };
    CodecRecorder.prototype.take = function () {
        if (this.timer) {
            clearTimeout(this.timer);
            this.timer = 0;
        }
        if (this.encoder.state !== 'configured') return;
        if (this.encoder.encodeQueueSize > 3 && !this.stopped) {  // (behind: this picture once it caught up)
            this.timer = setTimeout(() => { this.timer = 0; this.frame(); }, 1000 / FILM_FPS);
            return;
        }
        const now = performance.now();
        if (this.t0 < 0) this.t0 = now;
        // (Stop's picture may come sooner than 1/15 s after the last: it takes the next slot.)
        const ts = Math.max(Math.round((now - this.t0) * 1000), this.lastTs + Math.round(1e6 / FILM_FPS));
        const key = ts - this.lastKey >= 2e6;
        let frame = null;
        try {
            frame = new VideoFrame(this.canvas, {timestamp: ts});
            this.encoder.encode(frame, {keyFrame: key});
            this.last = now;
            this.lastTs = ts;
            if (key) this.lastKey = ts;
        } catch (e) { /* this picture is lost; the next change brings another */ }
        if (frame) frame.close();
    };
    CodecRecorder.prototype.took = function (chunk, meta) {
        const d = meta && meta.decoderConfig && meta.decoderConfig.description;
        if (d && !this.avcC)
            this.avcC = new Uint8Array(ArrayBuffer.isView(d) ? d.buffer.slice(d.byteOffset, d.byteOffset + d.byteLength) : d.slice(0));
        let data = new Uint8Array(chunk.byteLength);
        chunk.copyTo(data);
        if (this.format === 'annexb' || (!d && !this.avcC && data.length > 3 && data[0] === 0 && data[1] === 0 &&
                                         (data[2] === 1 || (data[2] === 0 && data[3] === 1)))) {
            // Annex B (asked for, or what came without a description): the parameter sets go to avcC.
            const b = annexbSample(data);
            if (b.sps && b.pps && !this.avcC) this.avcC = avcCOf(b.sps, b.pps);
            data = b.sample;
        }
        this.datas.push(data);
        this.samples.push({size: data.length, ts: chunk.timestamp, key: chunk.type === 'key'});
    };
    CodecRecorder.prototype.stop = function () {
        if (this.stopped) return;
        this.stopped = true;
        if (this.timer || this.soon) {  // (the last change, still waiting for its turn)
            this.soon = false;
            this.take();
        }
        const end = performance.now();
        const finish = () => {
            try { this.encoder.close(); } catch (e) { /* closed already, by its error */ }
            let blob = null;
            const n = this.samples.length;
            if (n && this.avcC) {
                // The last picture shows until Stop, and for a frame's time at least.
                const endUs = Math.max(Math.round((end - this.t0) * 1000), this.samples[n - 1].ts + Math.round(1e6 / FILM_FPS));
                const b = mp4Boxes(this.samples, this.avcC, this.width, this.height, endUs);
                blob = new Blob([b.ftyp, b.mdat].concat(this.datas, [b.moov]), {type: 'video/mp4'});
            }
            this.samples = [];
            this.datas = [];
            this.done(blob);
        };
        if (this.encoder.state === 'configured') this.encoder.flush().then(finish, finish);
        else finish();
    };
    // Fast ink has a separate transport and canvas. Identities, not clock comparisons, reconcile it:
    // each sample's is the kernel's time of its frame in microseconds, which the tiles' watermark
    // names too, and they wrap (u32).
    const after = (a, b) => ((a - b) | 0) > 0;
    function readStyle(body) {
        if (body.length !== 64) throw new Error('bad ink style');
        const d = new DataView(body.buffer, body.byteOffset, body.length);
        const f = i => d.getFloat32(i, true), r = i => Array.from({length: 4}, (_, k) => d.getInt32(i + k * 4, true));
        return {enabled: !!body[0], tool: body[1], white: !!body[2], whole: !!body[3],
            min: f(4), max: f(8), floor: f(12), ceiling: f(16), gamma: f(20),
            opacityMin: f(24), opacityMax: f(28), clip: r(32), exclude: r(48)};
    }
    function readSamples(body) {
        if (body.length < 8 || (body.length - 8) % 20) throw new Error('bad pen samples');
        const d = new DataView(body.buffer, body.byteOffset, body.length), samples = [];
        for (let at = 8; at < body.length; at += 20) samples.push({
            id: d.getUint32(at, true), ms: d.getUint32(at + 4, true),
            x: d.getUint16(at + 8, true), y: d.getUint16(at + 10, true), pressure: d.getUint16(at + 12, true) / 65535,
            tiltX: d.getInt8(at + 14), tiltY: d.getInt8(at + 15), state: body[at + 16], flags: body[at + 17]});
        return {epoch: d.getUint32(0, true), reset: !!d.getUint32(4, true), samples};
    }
    const inside = (r, s) => s.x >= r[0] && s.y >= r[1] && s.x < r[0] + r[2] && s.y < r[1] + r[3];
    function inkWidth(style, s) {
        const range = style.ceiling - style.floor;
        const p = Math.pow(Math.min(1, Math.max(0, (s.pressure - style.floor) / Math.max(.0001, range))), style.gamma);
        return {width: Math.max(.1, style.min + (style.max - style.min) * p),
            opacity: Math.min(1, Math.max(0, style.opacityMin + (style.opacityMax - style.opacityMin) * p))};
    }
    // The samples' times on one line (ms, a float): the identities' microseconds unwrapped from the
    // first one after a reset. Several frames in one millisecond keep their order and spacing.
    function Timeline() { this.reset(); }
    Timeline.prototype.reset = function () { this.last = null; this.at = 0; };
    Timeline.prototype.time = function (id) {
        if (this.last === null) { this.last = id >>> 0; this.at = 0; return 0; }
        const d = ((id - this.last) | 0) / 1000;
        if (d > 0) { this.last = id >>> 0; this.at += d; return this.at; }
        return this.at + d;  // (an older one, or the same frame again: where it falls)
    };
    function FastInk() { this.line = new Timeline(); this.reset(); }
    // (The timeline goes on: identities are the Kindle's clock across strokes and epochs alike.)
    FastInk.prototype.reset = function () {
        this.segments = []; this.prev = null; this.active = false; this.id = 0; this.ack = 0; this.pen = null;
        this.trail = [];
    };
    // A sample in (false for one older than the last: a stream never goes back). t: its time on the
    // timeline. A gap (LIVE_GAP) breaks the stroke; only a pen-down (LIVE_DOWN) starts one.
    FastInk.prototype.accept = function (sample, style) {
        if (this.id && after(this.id, sample.id)) return false;
        sample.t = this.line.time(sample.id);
        if (sample.flags & 8) { this.prev = null; this.active = false; }
        this.id = sample.id;
        this.pen = sample.state ? sample : null;
        this.trail.push(sample);
        if (this.trail.length > 512) this.trail.splice(0, this.trail.length - 512);
        if (sample.flags & 4) {
            this.active = !!(style && style.enabled && !style.whole && (sample.flags & 16) &&
                inside(style.clip, sample) && !inside(style.exclude, sample));
            this.prev = null;
        }
        if (sample.state !== 2 || !(sample.flags & 16) || !style || !style.enabled) {
            this.active = false; this.prev = null; return true;
        }
        if (this.active) {
            const to = Object.assign({}, sample, inkWidth(style, sample));
            if (!this.ack || after(sample.id, this.ack)) {
                this.segments.push({from: this.prev || to, to, style});
                // A stalled tile transport cannot grow this canvas's history without bound.
                if (this.segments.length > 2048) this.segments.shift();
            }
            this.prev = to;
        }
        return true;
    };
    FastInk.prototype.reconcile = function (id) {
        if (!id || (this.ack && !after(id, this.ack))) return;
        this.ack = id;
        this.segments = this.segments.filter(s => after(s.to.id, id));
        // Keep prev: the next segment must still start where the now-authoritative ink ended.
    };
    // Where the pen is at time t of the timeline: between the samples on either side of it, where both
    // say the same (hovering, or touching) and are close in time (a glide, not a jump across a gap or
    // a lift); else the last one before it. null while away, or before the first sample.
    FastInk.prototype.penAt = function (t) {
        const tr = this.trail;
        let i = tr.length - 1;
        while (i >= 0 && tr[i].t > t) i--;
        if (i < 0) return null;
        const a = tr[i], b = tr[i + 1];
        if (!a.state) return null;
        if (!b || b.state !== a.state || (b.flags & 12) || b.t - a.t > 60 || b.t <= a.t) return {x: a.x, y: a.y, state: a.state};
        const f = (t - a.t) / (b.t - a.t);
        return {x: a.x + (b.x - a.x) * f, y: a.y + (b.y - a.y) * f, state: a.state};
    };
    // Whether anything is still to play after time t (another animation frame is wanted).
    FastInk.prototype.pending = function (t) {
        return this.trail.length > 0 && this.trail[this.trail.length - 1].t > t;
    };
    // The playout: when each sample shows, on this computer's clock. The Kindle reads the pen at an
    // even pace, but the samples reach the page in bursts (Wi-Fi, and the Kindle's own work); drawn as
    // they come, the line grows in jerks and the dot skips. Each is shown instead at its own time plus
    // offset (the least this computer's clock has run ahead of the samples' recently: the quickest
    // path from the pen to here) plus delay: what most samples come late by (the 90th percentile of
    // the last ones) and one sample's spacing more, so the sample after the clock is there too and
    // the dot glides between the two, at most PLAYOUT_MAX. A burst within it plays out as the pen
    // moved; one later than that shows the moment it comes. The delay rises at once and falls slowly.
    const PLAYOUT_MAX = 20, PLAYOUT_SPAN = 4000;
    function Playout() { this.reset(); }
    Playout.prototype.reset = function () {
        this.seen = []; this.lates = []; this.offset = null; this.delay = 0; this.last = null; this.spacing = 0;
    };
    Playout.prototype.arrive = function (t, now) {
        if (this.last !== null && t > this.last)  // (the samples' own spacing, eased: about 4 ms on a Scribe)
            this.spacing += (Math.min(10, t - this.last) - this.spacing) * (this.spacing ? 0.05 : 1);
        this.last = t;
        const d = now - t;
        this.seen.push([now, d]);
        while (this.seen.length && (now - this.seen[0][0] > PLAYOUT_SPAN || this.seen.length > 2048)) this.seen.shift();
        let least = Infinity;
        for (const s of this.seen) if (s[1] < least) least = s[1];
        // (a jump of the samples' line, a reset, can make the old offsets meaningless: start again)
        if (this.offset !== null && Math.abs(least - this.offset) > 5000) { this.seen = [[now, d]]; least = d; this.lates = []; }
        this.offset = least;
        this.lates.push(d - least);
        if (this.lates.length > 120) this.lates.shift();
        const sorted = [...this.lates].sort((x, y) => x - y);
        const want = Math.min(PLAYOUT_MAX, sorted[Math.floor((sorted.length - 1) * 0.9)] + this.spacing);
        this.delay = want > this.delay ? want : this.delay + (want - this.delay) * 0.02;
    };
    // The timeline's time showing now (Infinity before any sample: draw what there is).
    Playout.prototype.clock = function (now) {
        return this.offset === null ? Infinity : now - this.offset - this.delay;
    };
    // What each tool deposits, 0 (black) .. 255 (src/brush.c's look_of, by tool id): the quick
    // line's shade, before the picture's own look (inverted, the warm light, transparent paper).
    const INK_LEVEL = [38, 32, 12, 255, 110, 8, 5, 22, 14, 6];
    // A pixel of the Kindle's (0 ink .. 255 paper) as repaint() shows it, look being {invert, clear,
    // light}: [r, g, b, alpha]. White ink and the precise eraser bring the paper back (255): black
    // while inverted, else the light; with transparent paper, nothing.
    function shade(v, look) {
        const [r, g, b] = look.light, dark = 255 - v;
        if (look.clear) return look.invert ? [r, g, b, dark / 255] : [0, 0, 0, dark / 255];
        const lit = (look.invert ? dark : v) / 255;
        return [r * lit, g * lit, b * lit, 1];
    }
    // A segment as far as the playout has reached (until, on the samples' timeline): whole, cut
    // short where until falls inside it, or null when it starts later.
    function played(s, until) {
        if (until === undefined || until >= s.to.t || s.to.t === undefined) return s;
        if (s.from.t === undefined || until < s.from.t || s.from === s.to) return null;
        const f = (until - s.from.t) / (s.to.t - s.from.t);
        const to = Object.assign({}, s.to, {x: s.from.x + (s.to.x - s.from.x) * f, y: s.from.y + (s.to.y - s.from.y) * f,
            width: s.from.width + (s.to.width - s.from.width) * f});
        return {from: s.from, to, style: s.style};
    }
    // Segments i..j-1 (one run) on c, in its own pixels with k to each of the screen's, as far as the
    // playout has reached. The panel stays above a stroke even when that stroke crosses it, while the
    // panel shows here (exclude).
    function strokeRun(c, segs, i, j, k, color, until, exclude = true) {
        const style = segs[i].style;
        c.save();
        c.scale(k, k);
        c.beginPath();
        c.rect(...style.clip);
        c.clip();
        if (exclude) {
            c.beginPath();
            c.rect(...style.clip);
            c.rect(...style.exclude);
            c.clip('evenodd');
        }
        c.strokeStyle = c.fillStyle = color;
        c.lineCap = c.lineJoin = 'round';
        for (let n = i; n < j; n++) {
            const s = played(segs[n], until);
            if (!s) break;
            c.lineWidth = (s.from.width + s.to.width) / 2;
            c.beginPath();
            if (s.from === s.to) { c.arc(s.to.x, s.to.y, s.to.width / 2, 0, Math.PI * 2); c.fill(); }
            else { c.moveTo(s.from.x, s.from.y); c.lineTo(s.to.x, s.to.y); c.stroke(); }
        }
        c.restore();
    }
    // The quick ink on ctx (cleared by the caller), run by run: a run is one stroke's segments in a
    // row with one style. A see-through one is drawn whole on layer ({canvas, ctx}, as big as ctx's
    // canvas, clear) at full strength, then laid over ctx once at its strongest opacity: segments laid
    // one over another at their own (a few to ten or more cover each pixel of a highlighter's band)
    // would add up to near black over the writing it highlights. Without a layer (the checks) each
    // segment goes straight on ctx.
    // until: the playout's time (Playout.clock): only what it has reached is drawn (all without it).
    // look.exclude false: the toolbar is switched off here, so the quick ink shows where it was.
    FastInk.prototype.draw = function (ctx, k, look, layer, until) {
        const segs = this.segments, exclude = look.exclude !== false;
        for (let i = 0, j; i < segs.length; i = j) {
            for (j = i + 1; j < segs.length && segs[j].style === segs[i].style && segs[j].from === segs[j - 1].to; j++) {}
            if (!played(segs[i], until)) continue;
            const style = segs[i].style;
            let opacity = 0;
            for (let n = i; n < j; n++) opacity = Math.max(opacity, segs[n].to.opacity);
            const [r, g, b, a] = shade(style.white ? 255 : (INK_LEVEL[style.tool] ?? INK_LEVEL[0]), look);
            // White ink on transparent paper takes the quick ink under it away, as it shows no paper.
            const out = style.white && look.clear;
            const color = 'rgb(' + r + ',' + g + ',' + b + ')', alpha = out ? opacity : opacity * a;
            if (!layer || (alpha >= 1 && !out)) {
                ctx.save();
                ctx.globalAlpha = alpha;
                if (out) ctx.globalCompositeOperation = 'destination-out';
                strokeRun(ctx, segs, i, j, k, color, until, exclude);
                ctx.restore();
                continue;
            }
            strokeRun(layer.ctx, segs, i, j, k, color, until, exclude);
            ctx.save();
            ctx.globalAlpha = alpha;
            ctx.globalCompositeOperation = out ? 'destination-out' : 'source-over';
            ctx.drawImage(layer.canvas, 0, 0);
            ctx.restore();
            layer.ctx.clearRect(0, 0, layer.canvas.width, layer.canvas.height);
        }
    };
    // A sw x sh source drawn into dw x dh pixels, each output pixel the mean of the area of the source
    // it covers (alpha-weighted, for transparent paper): the browser's own scaling (of a canvas, a
    // video, and drawImage even at "high") picks pixels, so 1 px lines came out bright, dim or gone by
    // where they fell, and every line here keeps its weight. Only the output pixels that the source
    // rect d reaches; read(x, y, w, h) gives the source's RGBA pixels of a rect (getImageData's data),
    // asked once. Returns the output pixels as {x, y, w, h, data} (data: RGBA, for putImageData at x,
    // y), or null. under: [r, g, b] laid under it (a film with no transparency), else none.
    function areaAverage(read, sw, sh, dw, dh, d, under) {
        const kx = sw / dw, ky = sh / dh;
        const vx0 = Math.max(0, Math.floor(d.x / kx)), vy0 = Math.max(0, Math.floor(d.y / ky));
        const vx1 = Math.min(dw, Math.ceil((d.x + d.w) / kx)), vy1 = Math.min(dh, Math.ceil((d.y + d.h) / ky));
        if (vx1 <= vx0 || vy1 <= vy0) return null;
        const sx0 = Math.floor(vx0 * kx), sy0 = Math.floor(vy0 * ky);
        const sx1 = Math.min(sw, Math.ceil(vx1 * kx)), sy1 = Math.min(sh, Math.ceil(vy1 * ky));
        const rw = sx1 - sx0, rh = sy1 - sy0, ow = vx1 - vx0, oh = vy1 - vy0;
        const src = read(sx0, sy0, rw, rh);
        // Each output column (row) as source columns (rows) of the rect read, and how much of each
        // it covers.
        function spans(v0, n, k, s0, limit) {
            const out = [];
            for (let i = 0; i < n; i++) {
                const a = (v0 + i) * k - s0, b = Math.min(a + k, limit), cols = [];
                for (let s = Math.floor(a); s < b; s++) cols.push(s, (Math.min(b, s + 1) - Math.max(a, s)) / (b - a));
                out.push(cols);
            }
            return out;
        }
        const xs = spans(vx0, ow, kx, sx0, rw), ys = spans(vy0, oh, ky, sy0, rh);
        // Across: rh rows of ow pixels, premultiplied.
        const mid = new Float32Array(rh * ow * 4);
        for (let r = 0; r < rh; r++)
            for (let o = 0; o < ow; o++) {
                const cols = xs[o];
                let cr = 0, cg = 0, cb = 0, ca = 0;
                for (let j = 0; j < cols.length; j += 2) {
                    const i = (r * rw + cols[j]) * 4, wa = cols[j + 1] * src[i + 3];
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
        // Down, then back from premultiplied (over under, where there is one).
        const p = new Uint8ClampedArray(ow * oh * 4);
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
                if (under) {
                    const a = ca / 255;
                    p[q] = cr / 255 + under[0] * (1 - a);
                    p[q + 1] = cg / 255 + under[1] * (1 - a);
                    p[q + 2] = cb / 255 + under[2] * (1 - a);
                    p[q + 3] = 255;
                    continue;
                }
                if (ca > 0) {
                    p[q] = cr / ca;
                    p[q + 1] = cg / ca;
                    p[q + 2] = cb / ca;
                }
                p[q + 3] = ca;
            }
        }
        return {x: vx0, y: vy0, w: ow, h: oh, data: p};
    }
    function PaintAges() { this.best = Infinity; this.samples = []; this.tiles = []; }
    PaintAges.prototype.sync = function (start, end, server) {
        if (end < start || end - start >= this.best) return;
        this.best = end - start; this.mid = (start + end) / 2; this.server = server;
    };
    PaintAges.prototype.age = function (now, ms) {
        return this.best < Infinity ? now - this.mid + ((this.server - ms) | 0) : null;
    };
    PaintAges.prototype.add = function (list, now, ms) {
        const age = this.age(now, ms);
        if (age === null || !Number.isFinite(age)) return;
        list.push(age); if (list.length > 1000) list.shift();
    };
    PaintAges.prototype.stats = function (list) {
        if (!list.length) return null;
        const sorted = [...list].sort((a, b) => a - b), n = sorted.length;
        return [sorted[Math.floor((n - 1) * .5)], sorted[Math.floor((n - 1) * .9)], sorted[n - 1]];
    };

    if (typeof module !== 'undefined' && module.exports) {
        module.exports = {Frames, lightColor, readPen, penMark, drawMark, filmFit, recordType, H264, filmConfig,
            filmChoices, annexbNals, annexbSample, avcCOf, mp4Boxes, MP4_SCALE, CodecRecorder, readStyle, readSamples,
            FastInk, Playout, Timeline, inkWidth, PaintAges, shade, INK_LEVEL, areaAverage, DOT_SIZES, DOT_COLORS,
            dotColor, dotSize};
        return;
    }

    const words = window.GRIMOIRE_LIVE_WORDS;
    const host = document.getElementById('live');
    if (!host || !words) return;
    if (!window.fetch || !window.ReadableStream || !window.createImageBitmap) {
        host.textContent = words.old;
        return;
    }
    const page = document.createElement('canvas');     // the screen as the tiles have it: the page under the layers
    const pageCtx = page.getContext('2d');
    const raw = document.createElement('canvas');      // the screen as the Kindle has it, the layers switched on over the page
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
    const inkLayer = document.createElement('canvas');
    const inkCtx = inkLayer.getContext('2d');
    inkLayer.className = 'pen';
    stage.append(view, inkLayer, penLayer);
    const ink = new FastInk(), ages = new PaintAges(), playout = new Playout();
    // penEpoch: the ink epoch /live/pen is in (0 before its hello); tileInk: the one the picture as
    // drawn is in (the newest painted batch's), so quick ink is drawn only over the screen it was
    // drawn on: a page turned, a pinch or a turn shows none until the picture has caught up.
    // batchAck, batchAckMs, batchInk: what the batches read since the last paint vouch for, taken by
    // the next one (painted); paintedAck, paintedInk: what the painted picture holds, for a new or
    // reset pen stream to start from. tileHellos: how many tile hellos came (runFast waits for one).
    let penEpoch = 0, tileInk = 0, fastConnected = false, paintSamples = [], debug = false, monitor = false;
    let batchAck = 0, batchAckMs = 0, batchInk = 0, paintedAck = 0, paintedInk = 0, layer = null;
    let tileHellos = 0, helloWaiters = [], penW = 0, penH = 0;
    // The layers (frame 9) as the stream has them: [{x, y, w, h, bmp}] (w 0: not on the screen), and
    // whether each shows here (layerOn: the toolbar, the clock and battery corner; both by default).
    const layers = [{x: 0, y: 0, w: 0, h: 0, bmp: null}, {x: 0, y: 0, w: 0, h: 0, bmp: null}];
    const layerOn = [true, true];
    try {
        layerOn[0] = localStorage.getItem('grimoire.layer.toolbar') !== '0';
        layerOn[1] = localStorage.getItem('grimoire.layer.status') !== '0';
        DOT.color = dotColor(localStorage.getItem('grimoire.dot.color'));
        DOT.size = dotSize(localStorage.getItem('grimoire.dot.size'));
    } catch (e) { /* private browsing: the defaults */ }
    const meter = document.createElement('output');
    meter.className = 'latency';
    meter.hidden = true;
    try { monitor = localStorage.getItem('grimoire.monitor') === '1'; } catch (e) { /* private browsing */ }
    const film = document.createElement('canvas');     // shown with the pen on it: what Record and PiP take
    const filmCtx = film.getContext('2d');
    // pen: where the Kindle's pen is (readPen), null while away; penShown, filmPen: its marks drawn on
    // the page and in film now, to take away; fitIn: shown's place in film (filmFit); flat: a
    // recording that is no WebM (an MP4), which has no transparency, so film has the paper drawn in
    // while Transparent paper is on; h264: the codec VideoEncoder takes at the screen's size
    // (probeH264: its first choice VideoEncoder takes), null for none, undefined while not known yet.
    // pipWindow: Picture in picture's window while it is open (film is drawn at its pixels then).
    let pen = null, showPen = true, penShown = null, penQueued = false, filmPen = null;
    let fitIn = {k: 1, x: 0, y: 0}, flat = false, recordAs = null, pipVideo = null, h264, probing = '', pipWindow = null;
    const failedFilms = new Set();                     // (choices an encoder refused once it began)
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
    // The dot's color (a few presets and any other) and size, remembered in this browser.
    const dotBox = document.createElement('span');
    dotBox.className = 'dot';
    dotBox.setAttribute('role', 'group');
    dotBox.setAttribute('aria-label', words.dotColor);
    const swatches = DOT_COLORS.map((c, i) => {
        const b = document.createElement('button');
        b.type = 'button';
        b.className = 'swatch';
        b.style.background = c;  // (inline: no pressed button's colors cover it)
        b.title = [words.dotRed, words.dotBlue, words.dotGreen, words.dotOrange, words.dotPurple][i] || c;
        b.setAttribute('aria-label', b.title);
        b.addEventListener('click', () => setDot(c, DOT.size));
        dotBox.append(b);
        return b;
    });
    const dotPicker = document.createElement('input');
    dotPicker.type = 'color';
    dotPicker.title = words.dotCustom;
    dotPicker.setAttribute('aria-label', words.dotCustom);
    dotPicker.addEventListener('input', () => setDot(dotPicker.value, DOT.size));
    dotBox.append(dotPicker);
    const dotSizes = document.createElement('select');
    dotSizes.title = words.dotSize;
    dotSizes.setAttribute('aria-label', words.dotSize);
    for (const [v, label] of [['small', words.small], ['medium', words.medium], ['large', words.large]]) {
        const o = document.createElement('option');
        o.value = v;
        o.textContent = label;
        dotSizes.append(o);
    }
    dotSizes.addEventListener('change', () => setDot(DOT.color, dotSizes.value));
    dotBox.append(dotSizes);
    bar.append(dotBox);
    function setDot(color, size) {
        DOT.color = dotColor(color);
        DOT.size = dotSize(size);
        try {
            localStorage.setItem('grimoire.dot.color', DOT.color);
            localStorage.setItem('grimoire.dot.size', DOT.size);
        } catch (e) { /* private browsing: for this visit only */ }
        swatches.forEach((b, i) => b.setAttribute('aria-pressed', String(DOT_COLORS[i] === DOT.color)));
        dotPicker.value = DOT.color;
        dotSizes.value = DOT.size;
        movePen(pen);
    }
    setDot(DOT.color, DOT.size);
    // The toolbar and the clock and battery corner, each shown over the page or not.
    const layerSwitches = [words.toolbar, words.statusCorner].map((label, i) => {
        const b = button(label, () => {
            layerOn[i] = !layerOn[i];
            b.setAttribute('aria-checked', String(layerOn[i]));
            try { localStorage.setItem(i ? 'grimoire.layer.status' : 'grimoire.layer.toolbar', layerOn[i] ? '1' : '0'); } catch (e) {}
            const l = layers[i];
            if (l.w) compose(l.x, l.y, l.w, l.h);
            penSoon();  // (the quick ink keeps off the toolbar only while it shows)
        });
        b.setAttribute('role', 'switch');
        b.className = 'switch';
        b.setAttribute('aria-checked', String(layerOn[i]));
        return b;
    });
    const recordButton = button(words.record, record);
    const canRecord = !!(window.MediaRecorder && film.captureStream) || !!(window.VideoEncoder && window.VideoFrame);
    if (!canRecord) recordButton.hidden = true;
    const pipButton = button(words.pip, pip);
    pipButton.setAttribute('aria-pressed', 'false');
    if (!document.pictureInPictureEnabled || !film.captureStream ||
        !HTMLVideoElement.prototype.requestPictureInPicture) pipButton.hidden = true;
    button(words.full, full);
    const monitorButton = button(words.monitor, () => setMonitor(!monitor, true));
    const debugButton = button(words.latency, () => {
        debug = !debug;
        meter.hidden = !debug;
        debugButton.setAttribute('aria-pressed', String(debug));
        if (debug) syncClock();
    });
    // The quick ink is drawn only over the picture of the ink epoch it is in (penEpoch, tileInk).
    function drawable() { return penEpoch !== 0 && penEpoch === tileInk; }
    // The samples' pen over any picture of the same size (penW x penH, the pen stream's hello's): a
    // page turned keeps it, a turn of the screen takes it away until both streams are turned.
    function dotShown() { return penEpoch !== 0 && tileInk !== 0 && penW === raw.width && penH === raw.height; }
    // A canvas as big as inkLayer for FastInk.draw's see-through runs (made at the first need).
    function inkScratch() {
        if (!layer) {
            const canvas = document.createElement('canvas');
            layer = {canvas, ctx: canvas.getContext('2d')};
        }
        if (layer.canvas.width !== inkLayer.width || layer.canvas.height !== inkLayer.height) {
            layer.canvas.width = inkLayer.width;
            layer.canvas.height = inkLayer.height;
        }
        return layer;
    }
    function setMonitor(on, gesture) {
        monitor = on; fit = true; showPen = true;
        penSwitch.setAttribute('aria-checked', 'true');
        document.body.classList.toggle('monitor', on);
        monitorButton.setAttribute('aria-pressed', String(on));
        try { localStorage.setItem('grimoire.monitor', on ? '1' : '0'); } catch (e) {}
        if (gesture && on && !document.fullscreenElement && document.documentElement.requestFullscreen)
            document.documentElement.requestFullscreen().catch(() => {});
        if (gesture && !on && document.fullscreenElement && document.exitFullscreen)
            document.exitFullscreen().catch(() => {});
        layout();
    }
    async function syncClock() {
        ages.best = Infinity;
        for (let i = 0; debug && i < 8; i++) {
            try {
                const start = performance.now();
                const r = await fetch('/live/time', {cache: 'no-store', credentials: 'same-origin'});
                const data = await r.json(), end = performance.now();
                if (r.ok && Number.isInteger(data.ms)) ages.sync(start, end, data.ms);
            } catch (e) { /* try the next request */ }
        }
    }
    function showAges() {
        if (!debug) return;
        const fmt = list => { const s = ages.stats(list); return s ? s.map(x => x.toFixed(1)).join(' / ') + ' ms' : '...'; };
        meter.textContent = words.sampleAge + ': ' + fmt(ages.samples) + ' | ' + words.tileAge + ': ' + fmt(ages.tiles) +
            ' | ' + words.stats + ' | ' + words.playout + ': ' + (fastConnected ? playout.delay.toFixed(1) + ' ms' : '...') +
            ' | ' + words.clockError + ': ' + (ages.best < Infinity ? (ages.best / 2).toFixed(1) + ' ms' : '...');
    }
    setInterval(() => { if (debug) syncClock(); }, 30000);
    recordLabel();
    host.replaceChildren(bar, meter, stage);
    setMonitor(monitor, false);
    addEventListener('keydown', e => {
        if ((e.key === 'm' || e.key === 'M') && !e.metaKey && !e.ctrlKey && !e.altKey) setMonitor(!monitor, true);
        if ((e.key === 'f' || e.key === 'F') && !e.metaKey && !e.ctrlKey && !e.altKey) full();
    });
    addEventListener('resize', layout);
    addEventListener('fullscreenchange', layout);

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
            const room = host.clientWidth, tall = innerHeight - (monitor ? 2 : bar.offsetHeight + 24);
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
        inkLayer.style.cssText = penLayer.style.cssText;
        inkLayer.width = view.width; inkLayer.height = view.height;
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
    // The quick ink and the dot as far as the playout has reached (until), and again at the next frame
    // while samples are still to play. A sample's age counts as it shows.
    function drawPen() {
        penQueued = false;
        const now = performance.now(), until = fastConnected ? playout.clock(now) : undefined;
        inkCtx.clearRect(0, 0, inkLayer.width, inkLayer.height);
        if (raw.width && drawable())
            ink.draw(inkCtx, view.width / raw.width, {invert: inverted(), clear, light, exclude: layerOn[0]}, inkScratch(), until);
        let i = 0;
        while (i < paintSamples.length && !(until < paintSamples[i].t)) i++;
        if (debug) for (const s of paintSamples.slice(0, i)) ages.add(ages.samples, now, s.ms);
        paintSamples.splice(0, i);
        showAges();
        if (fastConnected) {
            const at = dotShown() && until !== undefined ? ink.penAt(until) : null;
            if (!samePen(at, pen)) placePen(at);
            if (ink.pending(until)) penSoon();
        }
        if (penShown) penCtx.clearRect(penShown.x, penShown.y, penShown.w, penShown.h);
        const m = showPen && raw.width ? penMark(pen, view.width / raw.width, window.devicePixelRatio || 1) : null;
        penShown = m && m.box;
        if (m) drawMark(penCtx, m);
    }
    function samePen(a, b) { return a === b || (!!a && !!b && a.x === b.x && a.y === b.y && a.state === b.state); }
    // The pen moved, came, went, or was switched off: on the page at the next frame, in film now (it
    // is drawn as frames come, so a recording and Picture in picture go on while the tab is hidden).
    function movePen(next) {
        placePen(next);
        penSoon();
    }
    // The pen's place in film, now (pen: where the page draws it at its next frame).
    function placePen(next) {
        pen = next;
        const was = filmPen;
        filmPen = filmMark();
        if (was) filmDraw(was.box.x, was.box.y, was.box.w, was.box.h);
        if (filmPen) filmDraw(filmPen.box.x, filmPen.box.y, filmPen.box.w, filmPen.box.h);
    }
    // film is kept only while something takes it: a recording, or Picture in picture showing.
    function filming() { return !!recorder || pipShowing(); }
    function pipShowing() { return !!pipVideo && document.pictureInPictureElement === pipVideo; }
    // The mark's unit in film, which has the screen's own pixels: about as big as on a laptop at Fit.
    function filmMark() {
        if (!filming() || !showPen) return null;
        const unit = Math.max(1, Math.round(Math.min(film.width, film.height) / 620));
        return penMark(pen, fitIn.k, unit, fitIn.x, fitIn.y, DOT);
    }
    // A rect of film (its own pixels) drawn again: shown there (the paper first while it is clear in
    // a recording with no transparency), then the pen's mark where it falls. Only while filming,
    // unless forced (a first frame for the video).
    function filmDraw(x, y, w, h, force) {
        if (!force && !filming()) return;
        const x0 = Math.max(0, Math.floor(x)), y0 = Math.max(0, Math.floor(y));
        const x1 = Math.min(film.width, Math.ceil(x + w)), y1 = Math.min(film.height, Math.ceil(y + h));
        if (x1 <= x0 || y1 <= y0) return;
        let rx0 = x0, ry0 = y0, rx1 = x1, ry1 = y1;  // (what was drawn again: the mark goes back over all of it)
        const paper = flat && clear ? (inverted() ? [0, 0, 0] : light) : null;
        if (fitIn.k < 1) {
            // Smaller than the screen (Picture in picture at its window's pixels, a recording an
            // encoder takes only smaller): each film pixel the mean of what it covers, as on the page.
            const pw = Math.max(1, Math.round(shown.width * fitIn.k)), ph = Math.max(1, Math.round(shown.height * fitIn.k));
            const kx = shown.width / pw, ky = shown.height / ph;
            filmCtx.clearRect(x0, y0, x1 - x0, y1 - y0);
            const d = {x: (x0 - fitIn.x) * kx, y: (y0 - fitIn.y) * ky, w: (x1 - x0) * kx, h: (y1 - y0) * ky};
            const out = areaAverage((x, y, w, h) => shownCtx.getImageData(x, y, w, h).data, shown.width, shown.height, pw, ph, d, paper);
            if (out) {
                const img = filmCtx.createImageData(out.w, out.h);
                img.data.set(out.data);
                const ox = fitIn.x + out.x, oy = fitIn.y + out.y;
                filmCtx.putImageData(img, ox, oy);
                rx0 = Math.min(rx0, ox); ry0 = Math.min(ry0, oy);
                rx1 = Math.max(rx1, ox + out.w); ry1 = Math.max(ry1, oy + out.h);
            }
            if (filmPen) {
                filmCtx.save();
                filmCtx.beginPath();
                filmCtx.rect(rx0, ry0, rx1 - rx0, ry1 - ry0);
                filmCtx.clip();
                drawMark(filmCtx, filmPen);
                filmCtx.restore();
            }
            if (recorder && recorder.frame) recorder.frame();
            return;
        }
        filmCtx.save();
        filmCtx.beginPath();
        filmCtx.rect(x0, y0, x1 - x0, y1 - y0);
        filmCtx.clip();
        filmCtx.clearRect(x0, y0, x1 - x0, y1 - y0);
        const pw = shown.width * fitIn.k, ph = shown.height * fitIn.k;
        if (paper) {
            filmCtx.fillStyle = 'rgb(' + paper.join(',') + ')';
            filmCtx.fillRect(fitIn.x, fitIn.y, pw, ph);
        }
        const cw = x1 - x0, ch = y1 - y0;
        if (fitIn.k === 1) filmCtx.drawImage(shown, x0 - fitIn.x, y0 - fitIn.y, cw, ch, x0, y0, cw, ch);
        else filmCtx.drawImage(shown, fitIn.x, fitIn.y, pw, ph);
        if (filmPen) drawMark(filmCtx, filmPen);
        filmCtx.restore();
        if (recorder && recorder.frame) recorder.frame();  // (CodecRecorder: film changed)
    }
    // All of film again, at the size it keeps (a recording's) or shown's: its mark worked out anew.
    // Without a recording, film has Picture in picture's window's own pixels while it is open (so the
    // video plays at about 1:1, the averaging done here, not by the browser's video scaler), else
    // shown's size.
    function filmAll(force) {
        const want = pipShowing() && pipWindow && pipWindow.width > 0 && pipWindow.height > 0
            ? {w: Math.max(2, Math.round(pipWindow.width * (window.devicePixelRatio || 1))),
               h: Math.max(2, Math.round(pipWindow.height * (window.devicePixelRatio || 1)))}
            : {w: shown.width, h: shown.height};
        if (!recorder && (film.width !== want.w || film.height !== want.h)) {
            film.width = want.w;
            film.height = want.h;
        }
        fitIn = filmFit(shown.width, shown.height, film.width, film.height);
        filmPen = filmMark();
        filmDraw(0, 0, film.width, film.height, force);
    }
    // A rect of shown (its pixels) changed: the same in film.
    function filmShown(x, y, w, h) {
        if (!filming()) return;
        if (fitIn.k > 1) filmDraw(0, 0, film.width, film.height);  // (scaled up: all of it, no seams)
        else if (fitIn.k < 1) {  // (averaged: the film pixels the rect reaches, whole)
            const k = fitIn.k, fx = Math.floor(x * k), fy = Math.floor(y * k);
            filmDraw(fitIn.x + fx, fitIn.y + fy, Math.ceil((x + w) * k) - fx + 1, Math.ceil((y + h) * k) - fy + 1);
        } else filmDraw(x + fitIn.x, y + fitIn.y, w, h);
    }
    // Record's button says what it makes: Record (MP4), or Record (WebM) with why in its tooltip where
    // it cannot be an MP4; plain Record while it is not known yet (the H.264 probe is a promise) or the
    // browser names none. The MP4's tooltip says how (the codec and size VideoEncoder takes).
    function recordLabel() {
        if (!canRecord) return;
        if (recorder) {
            recordButton.textContent = words.stop;
            return;
        }
        const viaRecorder = window.MediaRecorder ? t => MediaRecorder.isTypeSupported(t) : () => false;
        recordAs = recordType(viaRecorder, h264 || null);
        const waiting = h264 === undefined && !(recordAs.via === 'recorder' && recordAs.ext === 'mp4');
        recordButton.textContent = waiting || !recordAs.ext ? words.record : recordAs.ext === 'mp4' ? words.recordMp4 : words.recordWebm;
        recordButton.title = waiting ? '' : recordAs.why ? words.webmBrowser :
            recordAs.film ? recordAs.film.codec + ', ' + recordAs.film.width + ' x ' + recordAs.film.height + ' (WebCodecs, ' +
                recordAs.film.format + (recordAs.film.hw ? ', ' + recordAs.film.hw : '') + ')' : '';
    }
    // The first of filmChoices that VideoEncoder takes for a w x h screen, asked when a hello gives the
    // size (and again, past the one that failed, when an encoder refused once it began). The answer
    // is a promise: Record's button says Record until it comes.
    function probeH264(w, h, again) {
        const size = w + 'x' + h;
        if ((probing === size && !again) || !window.VideoEncoder || !window.VideoFrame || !VideoEncoder.isConfigSupported) {
            if (!window.VideoEncoder || !window.VideoFrame || !VideoEncoder.isConfigSupported) h264 = null;
            return;
        }
        probing = size;
        h264 = undefined;
        recordLabel();
        (async () => {
            let found = null;
            for (const c of filmChoices(w, h)) {
                if (failedFilms.has(JSON.stringify(c))) continue;
                try {
                    const r = await VideoEncoder.isConfigSupported(filmConfig(c.codec, c.width, c.height, c.format, c.hw));
                    if (r && r.supported) {
                        found = c;
                        break;
                    }
                } catch (e) { /* not this one */ }
                if (probing !== size) return;
            }
            if (probing !== size) return;  // (another size was asked meanwhile: its answer counts)
            h264 = found;
            recordLabel();
        })();
    }
    // shown drawn into view, which has a pixel for each of the screen's, by areaAverage: only the
    // rect that changed (in shown's pixels); at full size or larger, a plain draw.
    function present() {
        presentQueued = false;
        const d = dirty;
        dirty = null;
        if (!raw.width || !d) return;
        if (shown.width <= view.width && shown.height <= view.height) {
            viewCtx.imageSmoothingEnabled = true;
            viewCtx.clearRect(0, 0, view.width, view.height);
            viewCtx.drawImage(shown, 0, 0, view.width, view.height);
            return;
        }
        const out = areaAverage((x, y, w, h) => shownCtx.getImageData(x, y, w, h).data, shown.width, shown.height,
            view.width, view.height, d, null);
        if (!out) return;
        const img = viewCtx.createImageData(out.w, out.h);
        img.data.set(out.data);
        viewCtx.putImageData(img, out.x, out.y);
    }
    function presentSoon(x, y, w, h) {
        const r = {x, y, w, h};
        dirty = dirty ? union(dirty, r) : r;
        if (presentQueued) return;
        presentQueued = true;
        requestAnimationFrame(() => {
            present();
            painted();
            drawPen();
        });
    }
    // The batches read since the last paint are on the page now, all their tiles in this same frame:
    // the picture is in their ink epoch, and the quick ink they vouch for goes. Their age is the
    // newest pen sample's in them (the Kindle's own handling, rendering, PNGs and the network
    // included), never the batch's: an ack-only batch counts too, a tile-less one with none not.
    function painted() {
        if (!batchInk) return;
        tileInk = batchInk;
        if (batchAck) {
            paintedAck = batchAck;
            paintedInk = batchInk;
            if (batchInk === penEpoch) ink.reconcile(batchAck);
            if (debug) ages.add(ages.tiles, performance.now(), batchAckMs);
        }
        batchAck = batchInk = 0;
        if (fastConnected) penSoon();  // (the dot plays out from the samples: drawPen)
    }
    // A stream of another version: this page is not that Grimoire's (a tab left open across an
    // update), and the Kindle serves the matching one with each load. Loaded again, once a minute
    // at most; where the page cannot remember when it last did (no storage), never.
    function protocolMismatch() {
        try {
            const now = Date.now(), last = Number(sessionStorage.getItem('grimoire.live.reload')) || 0;
            if (!(now - last >= 0 && now - last < 60000)) {
                sessionStorage.setItem('grimoire.live.reload', String(now));
                location.reload();
            }
        } catch (e) { /* no storage */ }
        throw new Error('unsupported live protocol');
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
            pipVideo.addEventListener('leavepictureinpicture', () => {
                pipButton.setAttribute('aria-pressed', 'false');
                pipWindow = null;
            });
            document.body.append(pipVideo);
            filmAll(true);  // a first frame, so the video knows its size
        }
        try {
            await pipVideo.play();
            const win = await pipVideo.requestPictureInPicture();
            // Its window's size, and each change of it: film is drawn at its pixels.
            pipWindow = win || null;
            if (win && win.addEventListener) win.addEventListener('resize', () => { if (pipWindow === win) filmAll(); });
            filmAll();
        } catch (e) { /* refused (no picture yet, or not from a tap): the button stays as it was */ }
    }
    // Record: film (shown and the pen) at 15 frames a second, as an MP4 wherever the browser can make
    // one (recordType): by MediaRecorder, or by CodecRecorder where MediaRecorder writes no MP4. film
    // keeps its size until the recording stops (filmFit).
    function record() {
        if (recorder) {
            recorder.stop();
            return;
        }
        if (!raw.width) return;
        recordLabel();
        const as = recordAs;
        film.width = as.film ? as.film.width : shown.width;  // (what the encoder takes: shown fits in it, averaged)
        film.height = as.film ? as.film.height : shown.height;
        flat = as.ext !== 'webm';
        chunks = [];
        try {
            if (as.via === 'codecs') {
                recorder = new CodecRecorder(film, as.codec, blob => {
                    stopped();
                    if (blob) save(blob, 'mp4');
                    else {  // (the encoder refused, or failed at once: the next choice, or WebM)
                        failedFilms.add(JSON.stringify(as.film));
                        probeH264(raw.width, raw.height, true);
                    }
                }, as.film);
            } else {
                const rec = new MediaRecorder(film.captureStream(15), as.mime ? {mimeType: as.mime} : undefined);
                rec.ondataavailable = e => { if (e.data && e.data.size) chunks.push(e.data); };
                rec.onstop = () => {
                    const type = rec.mimeType || as.mime || 'video/webm';
                    stopped();
                    save(new Blob(chunks, {type}), as.ext || (type.indexOf('mp4') >= 0 ? 'mp4' : 'webm'));
                };
                recorder = rec;
            }
        } catch (e) {
            recorder = null;
            flat = false;
            return;
        }
        filmAll();  // (CodecRecorder's first frame)
        if (recorder.start) recorder.start(1000);
        recordLabel();
    }
    function stopped() {
        recorder = null;
        flat = false;
        recordLabel();
        if (filming()) filmAll();  // (Picture in picture goes on at shown's size)
    }
    function save(blob, ext) {
        const a = document.createElement('a');
        a.href = URL.createObjectURL(blob);
        a.download = 'Grimoire live view.' + ext;
        document.body.append(a);
        a.click();
        setTimeout(() => { URL.revokeObjectURL(a.href); a.remove(); }, 1000);
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
    // A rect of raw made again from the page and the layers that show over it, then shown as repaint
    // shows it.
    function compose(x, y, w, h) {
        const x0 = Math.max(0, Math.floor(x)), y0 = Math.max(0, Math.floor(y));
        const x1 = Math.min(raw.width, Math.ceil(x + w)), y1 = Math.min(raw.height, Math.ceil(y + h));
        if (x1 <= x0 || y1 <= y0) return;
        rawCtx.drawImage(page, x0, y0, x1 - x0, y1 - y0, x0, y0, x1 - x0, y1 - y0);
        layers.forEach((l, i) => {
            if (!layerOn[i] || !l.w || !l.bmp) return;
            const ix0 = Math.max(x0, l.x), iy0 = Math.max(y0, l.y), ix1 = Math.min(x1, l.x + l.w), iy1 = Math.min(y1, l.y + l.h);
            if (ix1 > ix0 && iy1 > iy0) rawCtx.drawImage(l.bmp, ix0 - l.x, iy0 - l.y, ix1 - ix0, iy1 - iy0, ix0, iy0, ix1 - ix0, iy1 - iy0);
        });
        repaint(x0, y0, x1 - x0, y1 - y0);
    }
    function union(a, b) {
        const x = Math.min(a.x, b.x), y = Math.min(a.y, b.y);
        return {x, y, w: Math.max(a.x + a.w, b.x + b.w) - x, h: Math.max(a.y + a.h, b.y + b.h) - y};
    }

    // Draws the stream until it ends: 'bye' when Live view was turned off on the Kindle.
    async function read(reader) {
        const frames = new Frames();
        let pending = [], pendingLayers = [];
        for (;;) {
            const {value, done} = await reader.read();
            if (done) return null;
            for (const {type, body} of frames.push(value)) {
                const d = new DataView(body.buffer, body.byteOffset, body.length);
                if (type === 1 && body.length >= 12) {
                    if (d.getUint16(0, true) !== 3) protocolMismatch();
                    // The picture starts again from white (a start, a turn, a new stream): no quick
                    // ink until a batch says which ink epoch it is in.
                    tileInk = batchAck = batchInk = paintedAck = paintedInk = 0;
                    const w = d.getUint16(2, true), h = d.getUint16(4, true);
                    raw.width = shown.width = page.width = w;
                    raw.height = shown.height = page.height = h;
                    pageCtx.fillStyle = rawCtx.fillStyle = '#fff';
                    pageCtx.fillRect(0, 0, w, h);
                    rawCtx.fillRect(0, 0, w, h);
                    pending = [];
                    pendingLayers = [];
                    for (const l of layers) {  // (a hello takes them away: each comes again)
                        if (l.bmp && l.bmp.close) l.bmp.close();
                        Object.assign(l, {x: 0, y: 0, w: 0, h: 0, bmp: null});
                    }
                    movePen(null);  // (a hello takes the pen away: the next pen frame or sample is on the new screen)
                    probeH264(w, h);
                    layout();
                    if (filming()) filmAll();
                    tileHellos++;
                    for (const wake of helloWaiters.splice(0)) wake();
                } else if (type === 2 && body.length > 8) {
                    const x = d.getUint16(0, true), y = d.getUint16(2, true);
                    const w = d.getUint16(4, true), h = d.getUint16(6, true);
                    const png = new Blob([body.subarray(8)], {type: 'image/png'});
                    pending.push(createImageBitmap(png).then(bmp => ({x, y, w, h, bmp})));
                } else if (type === 9 && body.length >= 10) {
                    const i = body[0], x = d.getUint16(2, true), y = d.getUint16(4, true);
                    const w = d.getUint16(6, true), h = d.getUint16(8, true);
                    if (i < layers.length)
                        pendingLayers.push(w && body.length > 10
                            ? createImageBitmap(new Blob([body.subarray(10)], {type: 'image/png'})).then(bmp => ({i, x, y, w, h, bmp}))
                            : Promise.resolve({i, x: 0, y: 0, w: 0, h: 0, bmp: null}));
                } else if (type === 3) {
                    // A batch drawn whole, in order: no half-updated screen shows.
                    const tiles = await Promise.all(pending), news = await Promise.all(pendingLayers);
                    pending = [];
                    pendingLayers = [];
                    let dirty = null;
                    for (const t of tiles) {
                        pageCtx.drawImage(t.bmp, t.x, t.y);
                        if (t.bmp.close) t.bmp.close();
                        dirty = dirty ? union(dirty, t) : {x: t.x, y: t.y, w: t.w, h: t.h};
                    }
                    for (const n of news) {  // (where it was and where it is now)
                        const l = layers[n.i];
                        if (l.w) dirty = dirty ? union(dirty, l) : {x: l.x, y: l.y, w: l.w, h: l.h};
                        if (l.bmp && l.bmp.close) l.bmp.close();
                        Object.assign(l, {x: n.x, y: n.y, w: n.w, h: n.h, bmp: n.bmp});
                        if (n.w) dirty = dirty ? union(dirty, n) : {x: n.x, y: n.y, w: n.w, h: n.h};
                    }
                    // What it vouches for (none in a batch cut short: its ink epoch is 0 then), kept
                    // for the paint that shows it. A later batch of another ink epoch replaces it.
                    const ack = body.length >= 20 ? d.getUint32(8, true) : 0, at = body.length >= 20 ? d.getUint32(16, true) : 0;
                    if (at) {
                        if (at !== batchInk) batchAck = 0;
                        batchInk = at;
                        if (ack) {
                            batchAck = ack;
                            batchAckMs = d.getUint32(12, true);
                        }
                    }
                    if (dirty) compose(dirty.x, dirty.y, dirty.w, dirty.h);
                    else presentSoon(0, 0, 1, 1);
                    status(words.live, true);
                } else if (type === 6 && body.length >= 5) {
                    if (!fastConnected) movePen(readPen(body));
                } else if (type === 5 && body.length >= 1) {
                    const s = body[0];
                    const inv = body.length >= 2 ? body[1] === 1 : kindleInvert;
                    const warm = body.length >= 3 ? (body[2] === 255 ? null : body[2]) : kindleWarmth;
                    if (inv !== kindleInvert || warm !== kindleWarmth) {
                        kindleInvert = inv;
                        kindleWarmth = warm;
                        look();
                    }
                    if (s !== 0) { ink.reset(); paintSamples = []; movePen(null); }  // (any state but awake takes the pen away)
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
    // The pen stream until it ends: 'bye' when Live view was turned off on the Kindle.
    async function readFast(reader) {
        const frames = new Frames();
        let style = null;
        for (;;) {
            const {value, done} = await reader.read();
            if (done) return null;
            for (const {type, body} of frames.push(value)) {
                if (type === 1) {
                    if (body.length !== 12 || new DataView(body.buffer).getUint16(0, true) !== 3) protocolMismatch();
                    // A new ink epoch (or stream): nothing drawn before joins what comes.
                    const hd = new DataView(body.buffer);
                    penEpoch = hd.getUint32(8, true);
                    penW = hd.getUint16(2, true);
                    penH = hd.getUint16(4, true);
                    ink.reset();
                    if (paintedInk === penEpoch) ink.reconcile(paintedAck);
                    paintSamples = [];
                    style = null;
                    movePen(null);
                } else if (type === 8) style = readStyle(body);
                else if (type === 7) {
                    const packet = readSamples(body);
                    if (packet.reset) {
                        ink.reset();
                        paintSamples = [];
                        if (paintedInk === penEpoch) ink.reconcile(paintedAck);
                    }
                    if (packet.epoch !== penEpoch) continue;
                    const now = performance.now();
                    for (const sample of packet.samples)
                        if (ink.accept(sample, style)) {
                            paintSamples.push(sample);
                            playout.arrive(sample.t, now);
                        }
                    if (paintSamples.length > 2048) paintSamples.splice(0, paintSamples.length - 2048);
                    // The dot plays out at the next frames (drawPen); a hidden tab has none, so a
                    // recording or Picture in picture takes the newest place at once.
                    if (document.hidden) placePen(dotShown() ? ink.pen : null);
                    penSoon();
                } else if (type === 5 && body[0] !== 0) {
                    ink.reset();
                    paintSamples = [];
                    movePen(null);
                    if (body[0] === 3) return 'bye';
                }
            }
        }
    }
    // The next tile hello after the n-th: the tile stream (re)connected to a Kindle that streams.
    function tileHello(n) {
        return tileHellos > n ? Promise.resolve() : new Promise(r => helloWaiters.push(r));
    }
    // The pen stream: once the tile stream has said hello, and again after it fails as run() does it
    // (from 500 ms, twice as long each time, up to 8 s). Live view off (404, which a Grimoire with no
    // /live/pen answers too) or turned off (bye) waits for the tile stream's next hello instead; no
    // longer paired (401, 403) stops it.
    (async function runFast() {
        let seen = 0;
        for (let delay = 500;;) {
            await tileHello(seen);
            let how = null;
            try {
                const r = await fetch('/live/pen', {cache: 'no-store', credentials: 'same-origin'});
                if (r.status === 401 || r.status === 403) return;
                if (r.status === 404) how = 'gone';
                else if (r.ok && r.body) {
                    delay = 500;
                    fastConnected = true;
                    how = await readFast(r.body.getReader());
                }
            } catch (e) { /* the next connection starts with a fresh stroke */ }
            fastConnected = false;
            ink.reset();
            ink.line.reset();  // (a new stream: its samples' line and pace are learned anew)
            playout.reset();
            paintSamples = [];
            penEpoch = 0;
            movePen(null);
            if (how === 'gone' || how === 'bye') {
                seen = tileHellos;
                continue;
            }
            await sleep(delay);
            delay = Math.min(delay * 2, 8000);
        }
    })();
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
