// Live view: authoritative PNG tiles on /live/stream and early pen samples on /live/pen.
// src/live.h defines the version 2 wire format. The separate reader and overlay can paint ink
// while PNG decoding is pending. Tests run the pure model and this page with DOM/canvas stand-ins.
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
    // (ox, oy), and u of its pixels to a unit (a CSS pixel on the page): a red dot, see-through while
    // the pen hovers (alpha, the red's opacity: the writing under it shows) and smaller and solid while
    // it touches (less of the line it draws hidden), inside a light edge and a dark one so it shows on
    // paper, ink, mid gray, the warm light and inverted alike. box: the whole pixels it covers, to take
    // it away again. null without a pen.
    const PEN_HOVER = 7, PEN_TOUCH = 5, PEN_EDGE = 1, PEN_SEE = 0.55;
    function penMark(pen, k, u, ox = 0, oy = 0) {
        if (!pen || !pen.state) return null;
        const hover = pen.state === 1, x = ox + (pen.x + 0.5) * k, y = oy + (pen.y + 0.5) * k;
        const r = (hover ? PEN_HOVER : PEN_TOUCH) * u, edge = PEN_EDGE * u;
        const out = r + 2 * edge + 1;  // (1 more for the antialiased edge)
        const x0 = Math.floor(x - out), y0 = Math.floor(y - out);
        return {x, y, r, edge, alpha: hover ? PEN_SEE : 1,
                box: {x: x0, y: y0, w: Math.ceil(x + out) - x0, h: Math.ceil(y + out) - y0}};
    }
    // The mark drawn: the dark edge outermost, the light one inside it, then the red. Only the red is
    // see-through while hovering: a faint edge would lose the dot on a page of its own shade. (Here
    // with the pure parts so tests/live_check.js can draw it on a stand-in context.)
    const PEN_COLOR = '#f03c28', PEN_LIGHT = 'rgba(255,255,255,0.9)', PEN_DARK = 'rgba(0,0,0,0.45)';
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
        ctx.fillStyle = PEN_COLOR;
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
    // h264, the H.264 codec WebCodecs' VideoEncoder takes at the screen's size ('' for none). First
    // MediaRecorder's own MP4 (Chrome, Safari: via 'recorder'), else the page's own (via 'codecs':
    // Firefox's MediaRecorder writes only WebM and Ogg, but its VideoEncoder makes H.264, which
    // CodecRecorder puts in an MP4), and WebM only where neither can (why 'browser'). A browser that
    // names none records its own choice, which nothing here can name.
    const MP4 = ['video/mp4;codecs=avc1.42E01E', 'video/mp4;codecs=avc1', 'video/mp4'];
    const WEBM = ['video/webm;codecs=vp8', 'video/webm;codecs=vp9', 'video/webm'];
    function recordType(supported, h264) {
        const first = list => list.find(t => { try { return !!supported(t); } catch (e) { return false; } });
        const mp4 = first(MP4), webm = first(WEBM);
        if (mp4) return {mime: mp4, ext: 'mp4', why: '', via: 'recorder'};
        if (h264) return {mime: 'video/mp4', ext: 'mp4', why: '', via: 'codecs', codec: h264};
        if (webm) return {mime: webm, ext: 'webm', why: 'browser', via: 'recorder'};
        return {mime: '', ext: '', why: '', via: 'recorder'};  // (the file named by the type the recorder says it made)
    }
    // The H.264 asked of VideoEncoder, in order: Baseline, which has no B frames, so the frames come
    // in the order they show and mp4Boxes needs no reordering table. Level 5.1 holds both Scribes'
    // screens (1986 x 2648 is 20,750 macroblocks, 3.1's limit is 3,600); 3.1 last, for an encoder
    // that takes only it (Firefox's raises the level itself).
    const H264 = ['avc1.420033', 'avc1.42E033', 'avc1.42001f'];
    const FILM_FPS = 15, FILM_BITRATE = 4e6;
    function filmConfig(codec, width, height) {
        return {codec, width, height, bitrate: FILM_BITRATE, framerate: FILM_FPS, avc: {format: 'avc'}};
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
    function CodecRecorder(canvas, codec, done) {
        this.canvas = canvas;
        this.width = canvas.width;
        this.height = canvas.height;
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
        this.encoder.configure(filmConfig(codec, this.width, this.height));
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
        const data = new Uint8Array(chunk.byteLength);
        chunk.copyTo(data);
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
    // Fast ink has a separate transport and canvas. Sequence numbers, not clock comparisons,
    // reconcile it: several digitizer frames can share a millisecond, and the clock wraps.
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
            seq: d.getUint32(at, true), ms: d.getUint32(at + 4, true),
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
    function FastInk() { this.reset(); }
    FastInk.prototype.reset = function () {
        this.segments = []; this.prev = null; this.active = false; this.seq = 0; this.ack = 0; this.pen = null;
    };
    FastInk.prototype.accept = function (sample, style) {
        if (this.seq && !after(sample.seq, this.seq)) return false;
        const gap = (this.seq && ((sample.seq - this.seq) >>> 0) !== 1) || (sample.flags & 8);
        if (gap) { this.prev = null; this.active = false; }
        this.seq = sample.seq;
        this.pen = sample.state ? sample : null;
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
            if (!this.ack || after(sample.seq, this.ack)) {
                this.segments.push({from: this.prev || to, to, style});
                // A stalled tile transport cannot grow this canvas's history without bound.
                if (this.segments.length > 2048) this.segments.shift();
            }
            this.prev = to;
        }
        return true;
    };
    FastInk.prototype.reconcile = function (seq) {
        if (!seq || (this.ack && !after(seq, this.ack))) return;
        this.ack = seq;
        this.segments = this.segments.filter(s => after(s.to.seq, seq));
        // Keep prev: the next segment must still start where the now-authoritative ink ended.
    };
    FastInk.prototype.draw = function (ctx, k, invert, light) {
        for (const s of this.segments) {
            ctx.save();
            ctx.scale(k, k);
            ctx.beginPath();
            ctx.rect(...s.style.clip);
            ctx.clip();
            // The panel stays above a stroke even when that stroke crosses it.
            ctx.beginPath();
            ctx.rect(...s.style.clip);
            ctx.rect(...s.style.exclude);
            ctx.clip('evenodd');
            const white = s.style.white !== invert;
            const color = white ? 'rgb(' + light.join(',') + ')' : '#222';
            ctx.strokeStyle = ctx.fillStyle = color;
            ctx.globalAlpha = s.to.opacity;
            ctx.lineWidth = (s.from.width + s.to.width) / 2;
            ctx.lineCap = ctx.lineJoin = 'round';
            ctx.beginPath();
            if (s.from === s.to) { ctx.arc(s.to.x, s.to.y, s.to.width / 2, 0, Math.PI * 2); ctx.fill(); }
            else { ctx.moveTo(s.from.x, s.from.y); ctx.lineTo(s.to.x, s.to.y); ctx.stroke(); }
            ctx.restore();
        }
    };
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
            mp4Boxes, MP4_SCALE, CodecRecorder, readStyle, readSamples, FastInk, inkWidth, PaintAges};
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
    const inkLayer = document.createElement('canvas');
    const inkCtx = inkLayer.getContext('2d');
    inkLayer.className = 'pen';
    stage.append(view, inkLayer, penLayer);
    const ink = new FastInk(), ages = new PaintAges();
    let epoch = 0, penEpoch = 0, fastConnected = false, paintSamples = [], debug = false, monitor = false;
    let tileAck = 0, paintedAck = 0, tileStamp = null;
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
    // (probeH264), '' for none or not known yet.
    let pen = null, showPen = true, penShown = null, penQueued = false, filmPen = null;
    let fitIn = {k: 1, x: 0, y: 0}, flat = false, recordAs = null, pipVideo = null, h264 = '', probing = '';
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
            ' | p50 / p90 / max | ' + words.clockError + ': ' + (ages.best < Infinity ? (ages.best / 2).toFixed(1) + ' ms' : '...');
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
    function drawPen() {
        penQueued = false;
        inkCtx.clearRect(0, 0, inkLayer.width, inkLayer.height);
        if (raw.width && penEpoch === epoch) ink.draw(inkCtx, view.width / raw.width, inverted(), light);
        if (debug) for (const s of paintSamples) ages.add(ages.samples, performance.now(), s.ms);
        paintSamples = [];
        showAges();
        if (penShown) penCtx.clearRect(penShown.x, penShown.y, penShown.w, penShown.h);
        const m = showPen && raw.width ? penMark(pen, view.width / raw.width, window.devicePixelRatio || 1) : null;
        penShown = m && m.box;
        if (m) drawMark(penCtx, m);
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
        if (filmPen) drawMark(filmCtx, filmPen);
        filmCtx.restore();
        if (recorder && recorder.frame) recorder.frame();  // (CodecRecorder: film changed)
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
        recordAs = recordType(window.MediaRecorder ? t => MediaRecorder.isTypeSupported(t) : () => false, h264);
        recordButton.textContent = recordAs.ext === 'webm' ? words.recordWebm : words.record;
        recordButton.title = recordAs.why ? words.webmBrowser : '';
    }
    // The first of H264 that VideoEncoder takes at w x h, asked when a hello gives the size. The
    // answer is a promise, so until it comes Record's button says what it would do without it (in
    // Firefox, WebM, for the moment it takes).
    function probeH264(w, h) {
        const size = w + 'x' + h;
        if (probing === size || !window.VideoEncoder || !window.VideoFrame || !VideoEncoder.isConfigSupported) return;
        probing = size;
        (async () => {
            let found = '';
            for (const codec of H264) {
                try {
                    const r = await VideoEncoder.isConfigSupported(filmConfig(codec, w, h));
                    if (r && r.supported) {
                        found = codec;
                        break;
                    }
                } catch (e) { /* not this one */ }
            }
            if (probing !== size) return;  // (another size was asked meanwhile: its answer counts)
            h264 = found;
            recordLabel();
        })();
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
        requestAnimationFrame(() => {
            present();
            if (tileAck) { paintedAck = tileAck; if (penEpoch === epoch) ink.reconcile(tileAck); }
            if (debug && tileStamp !== null) ages.add(ages.tiles, performance.now(), tileStamp);
            tileStamp = null;
            drawPen();
        });
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
        film.width = shown.width;
        film.height = shown.height;
        flat = as.ext !== 'webm';
        chunks = [];
        try {
            if (as.via === 'codecs') {
                recorder = new CodecRecorder(film, as.codec, blob => {
                    stopped();
                    if (blob) save(blob, 'mp4');
                    else {  // (the encoder refused, or failed at once: WebM next time, where there is one)
                        h264 = '';
                        recordLabel();
                    }
                });
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
                    if (d.getUint16(0, true) !== 2) throw new Error('unsupported live protocol');
                    const nextEpoch = d.getUint32(8, true);
                    if (nextEpoch !== epoch) {
                        if (nextEpoch !== penEpoch) { ink.reset(); paintSamples = []; }
                        paintedAck = 0;
                    }
                    epoch = nextEpoch; tileAck = 0;
                    const w = d.getUint16(2, true), h = d.getUint16(4, true);
                    raw.width = shown.width = w;
                    raw.height = shown.height = h;
                    rawCtx.fillStyle = '#fff';
                    rawCtx.fillRect(0, 0, w, h);
                    pending = [];
                    movePen(penEpoch === epoch ? ink.pen : null);
                    probeH264(w, h);
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
                    tileAck = body.length >= 16 ? d.getUint32(8, true) : 0;
                    tileStamp = body.length >= 8 ? d.getUint32(4, true) : null;
                    if (dirty) repaint(dirty.x, dirty.y, dirty.w, dirty.h);
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
    async function readFast(reader) {
        const frames = new Frames();
        let style = null;
        for (;;) {
            const {value, done} = await reader.read();
            if (done) return;
            for (const {type, body} of frames.push(value)) {
                if (type === 1) {
                    if (body.length !== 12 || new DataView(body.buffer).getUint16(0, true) !== 2)
                        throw new Error('unsupported pen protocol');
                    penEpoch = new DataView(body.buffer).getUint32(8, true);
                    ink.reset(); if (penEpoch === epoch) ink.reconcile(paintedAck);
                    paintSamples = []; style = null;
                    movePen(null);
                } else if (type === 8) style = readStyle(body);
                else if (type === 7) {
                    const packet = readSamples(body);
                    if (packet.reset) { ink.reset(); paintSamples = []; if (penEpoch === epoch) ink.reconcile(paintedAck); }
                    if (packet.epoch !== penEpoch) continue;
                    for (const sample of packet.samples) if (ink.accept(sample, style)) paintSamples.push(sample);
                    if (paintSamples.length > 2048) paintSamples.splice(0, paintSamples.length - 2048);
                    movePen(penEpoch === epoch ? ink.pen : null);
                } else if (type === 5 && body[0] !== 0) {
                    ink.reset(); paintSamples = []; movePen(null);
                    if (body[0] === 3) return;
                }
            }
        }
    }
    (async function runFast() {
        for (;;) {
            try {
                const r = await fetch('/live/pen', {cache: 'no-store', credentials: 'same-origin'});
                if (r.status === 401 || r.status === 403) return;
                if (r.ok && r.body) { fastConnected = true; await readFast(r.body.getReader()); }
            } catch (e) { /* the next connection starts with a fresh stroke */ }
            fastConnected = false; ink.reset(); paintSamples = []; movePen(null);
            await sleep(500);
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
