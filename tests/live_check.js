#!/usr/bin/env node
// live_check.js: the browser's frame reader (web/live.js, Frames) on a stream the Kindle sent
// (tests/live_stream.py keeps its bytes and the frames it read): cut into pieces of every size,
// from one byte up, the frames must come out the same, in order. Then the page's pure parts: the
// front light's color, the pen frame (the stream's own, against pen.txt) and its mark, where a turned
// screen goes in a recording, and Record's format for what a browser records. Last, the MP4 the page
// makes where MediaRecorder makes none: its boxes, CodecRecorder on a stand-in VideoEncoder, and,
// where ffmpeg has libx264, a real H.264 stream put in an MP4 that ffmpeg must decode to the same
// frames at the same times (skipped without it).
//   usage: node tests/live_check.js <stream.bin> <stream.txt> [pen.txt]
'use strict';
const fs = require('fs');
const path = require('path');
const { execFileSync, spawnSync } = require('child_process');
const { Frames, lightColor, readPen, penMark, filmFit, recordType, H264, filmConfig, mp4Boxes, MP4_SCALE,
        CodecRecorder } = require(path.join(__dirname, '..', 'web', 'live.js'));

const [bin, txt, pentxt] = process.argv.slice(2);
const bytes = new Uint8Array(fs.readFileSync(bin));
const want = fs.readFileSync(txt, 'utf8').trim().split('\n').map(l => l.split(' ').map(Number));
let fail = 0;
function run(name, sizes) {
    const f = new Frames(), got = [];
    for (let at = 0, i = 0; at < bytes.length; i++) {
        const n = Math.max(1, sizes(i));
        for (const fr of f.push(bytes.subarray(at, at + n))) got.push([fr.type, fr.body.length]);
        at += n;
    }
    const ok = got.length === want.length && got.every((g, i) => g[0] === want[i][0] && g[1] === want[i][1]) &&
               f.buf.length === 0;
    console.log((ok ? 'ok   ' : 'FAIL ') + name + ` (${got.length} frames)`);
    if (!ok) fail = 1;
}
run('whole', () => bytes.length);
run('a byte at a time', () => 1);
run('5 bytes (a frame head) at a time', () => 5);
let seed = 7;
const rand = () => (seed = (seed * 1103515245 + 12345) & 0x7fffffff) / 0x7fffffff;
for (let k = 0; k < 20; k++) run('random pieces #' + (k + 1), () => 1 + Math.floor(rand() * 9000));
try {
    new Frames().push(new Uint8Array([0, 0, 0, 0, 1]));
    console.log('FAIL a frame of length 0 is refused');
    fail = 1;
} catch (e) {
    console.log('ok   a frame of length 0 is refused');
}
// The front light's color: white at warmth 0, warmer (less blue, then less green) at every step up
// to the warmest, never past it, red whole throughout.
function light(name, ok) {
    console.log((ok ? 'ok   ' : 'FAIL ') + name);
    if (!ok) fail = 1;
}
const steps = Array.from({length: 25}, (_, i) => lightColor(i));
light('light: white at warmth 0', steps[0].join() === '255,255,255');
light('light: red whole at every warmth', steps.every(c => c[0] === 255));
light('light: each step warmer than the last', steps.every((c, i) => !i ||
    (c[2] < steps[i - 1][2] && c[1] <= steps[i - 1][1])));
light('light: the warmest is amber (' + steps[24] + ')', steps[24][1] > 140 && steps[24][1] < 170 &&
    steps[24][2] > 45 && steps[24][2] < 80);
light('light: past the warmest is the warmest', lightColor(99).join() === steps[24].join() &&
    lightColor(-5).join() === steps[0].join());
const ok = light;
// The pen frame: the stream's own (the pen live_stream.py read), little-endian, and away as null.
if (pentxt && fs.existsSync(pentxt) && fs.readFileSync(pentxt, 'utf8').trim()) {
    const want = fs.readFileSync(pentxt, 'utf8').trim().split(' ').map(Number);
    const pens = new Frames().push(bytes).filter(f => f.type === 6).map(f => readPen(f.body));
    ok('pen: the stream\'s pen frame reads as ' + want.join(','), pens.length === 1 && pens[0] &&
       [pens[0].x, pens[0].y, pens[0].state].join() === want.join());
}
const far = readPen(new Uint8Array([0x43, 0x07, 0xAF, 0x09, 2]));
ok('pen: both bytes of each place (1859,2479 touching)', far.x === 1859 && far.y === 2479 && far.state === 2);
ok('pen: away, or too short, is no pen', readPen(new Uint8Array([5, 0, 6, 0, 0])) === null &&
   readPen(new Uint8Array([5, 0, 6, 0])) === null);
// Its mark: at the pixel's center, scaled; a ring hovering, a smaller dot touching; the box whole
// pixels around all of it (the halo and the antialiased edge too), so taking it away leaves nothing.
function covers(m) {
    const reach = m.r + (m.ring ? m.line / 2 : 0) + m.halo;
    return Number.isInteger(m.box.x) && Number.isInteger(m.box.y) && Number.isInteger(m.box.w) &&
        Number.isInteger(m.box.h) && m.box.x <= m.x - reach - 1 && m.box.y <= m.y - reach - 1 &&
        m.box.x + m.box.w >= m.x + reach + 1 && m.box.y + m.box.h >= m.y + reach + 1;
}
const hover = penMark({x: 100, y: 200, state: 1}, 0.25, 2), touch = penMark({x: 100, y: 200, state: 2}, 0.25, 2);
ok('mark: at the pixel\'s center, scaled (25.125, 50.125)', hover.x === 25.125 && hover.y === 50.125);
ok('mark: a ring hovering, a smaller dot touching', hover.ring && !touch.ring && touch.r < hover.r);
ok('mark: its unit scales it (2 CSS px to the pixel: twice the size)', hover.r === 2 * penMark({x: 0, y: 0, state: 1}, 1, 1).r);
ok('mark: the box covers all of it, in whole pixels', [hover, touch, penMark({x: 1859, y: 2479, state: 1}, 1, 3),
    penMark({x: 3, y: 7, state: 2}, 0.3071, 1.5, 0, 542)].every(covers));
ok('mark: placed in a film at its offset', penMark({x: 0, y: 0, state: 2}, 0.75, 1, 0, 542).y === 542.375);
ok('mark: none while away', penMark(null, 1, 1) === null && penMark({x: 1, y: 1, state: 0}, 1, 1) === null);
// A recording keeps its size: the same screen fills it, a turned one goes in whole, centered.
const same = filmFit(1860, 2480, 1860, 2480), side = filmFit(2480, 1860, 1860, 2480);
ok('film: the same size, as it is', same.k === 1 && same.x === 0 && same.y === 0);
ok('film: turned, whole and centered (0.75, at 0,542)', side.k === 0.75 && side.x === 0 && side.y === 542 &&
   2480 * side.k <= 1860 && side.y + 1860 * side.k <= 2480);
const big = filmFit(1986, 2648, 2648, 1986);
ok('film: the bigger Scribe turned back the other way fits too', big.y === 0 && big.x >= 0 &&
   big.x + 1986 * big.k <= 2648 && 2648 * big.k <= 1986 + 1e-9);
// Record's format. What each browser's MediaRecorder said (Chrome 156, Safari: MP4 only, Firefox
// 156 and Nightly 161: VP8 only), and h264, the codec its VideoEncoder took ('' for none).
const chrome = () => true, safari = t => t === 'video/mp4', firefox = t => t === 'video/webm;codecs=vp8' || t === 'video/webm';
const thrower = () => { throw new Error('no'); };
const as = (f, h264) => { const r = recordType(f, h264 || ''); return [r.mime, r.ext, r.why, r.via, r.codec || ''].join('|'); };
ok('record: Chrome, MediaRecorder\'s MP4 of H.264', as(chrome) === 'video/mp4;codecs=avc1.42E01E|mp4||recorder|' &&
   as(chrome, H264[0]) === 'video/mp4;codecs=avc1.42E01E|mp4||recorder|');
ok('record: Safari, its plain MP4', as(safari) === 'video/mp4|mp4||recorder|');
ok('record: Firefox with H.264 in VideoEncoder, the page\'s own MP4', as(firefox, 'avc1.420033') === 'video/mp4|mp4||codecs|avc1.420033');
ok('record: Firefox without it, WebM, and the button says why', as(firefox) === 'video/webm;codecs=vp8|webm|browser|recorder|');
ok('record: no MediaRecorder at all but VideoEncoder, the page\'s MP4', as(() => false, 'avc1.42001f') === 'video/mp4|mp4||codecs|avc1.42001f');
ok('record: a browser that says nothing (or throws), its own choice, no reason claimed', as(() => false) === '|||recorder|' &&
   as(thrower) === '|||recorder|');
ok('record: Baseline H.264 first, at a level that holds both Scribes', H264[0] === 'avc1.420033' &&
   H264.every(c => /^avc1\.42/.test(c)) && filmConfig(H264[0], 1860, 2480).avc.format === 'avc');

// The MP4's boxes, read back. A box: {type, at, size, body (where its payload starts)}.
function boxes(buf, start, end) {
    const d = new DataView(buf.buffer, buf.byteOffset, buf.byteLength), out = [];
    for (let at = start; at < end;) {
        let size = d.getUint32(at), head = 8;
        if (size === 1) {
            size = d.getUint32(at + 8) * 4294967296 + d.getUint32(at + 12);
            head = 16;
        }
        if (size < head || at + size > end) return null;
        out.push({type: String.fromCharCode(...buf.subarray(at + 4, at + 8)), at, size, body: at + head});
        at += size;
    }
    return out;
}
// The box at a path of types inside buf (each a container), or null.
function find(buf, types, start = 0, end = buf.length) {
    let found = null;
    for (const t of types) {
        const list = boxes(buf, start, end);
        found = list && list.find(b => b.type === t);
        if (!found) return null;
        start = found.body + (t === 'stsd' ? 8 : 0);  // (stsd: a full box and its count, then entries)
        end = found.at + found.size;
    }
    return found;
}
const u32 = (buf, at) => new DataView(buf.buffer, buf.byteOffset).getUint32(at);
// A full box's table of u32 (or u64, size 8) after skip bytes, its count the u32 before it, each
// entry per values long.
function entries(buf, b, skip, size, per = 1) {
    const n = u32(buf, b.body + 4 + skip - 4) * per, out = [];
    for (let i = 0, at = b.body + 4 + skip; i < n; i++, at += size)
        out.push(size === 8 ? u32(buf, at) * 4294967296 + u32(buf, at + 4) : u32(buf, at));
    return out;
}
const STBL = ['moov', 'trak', 'mdia', 'minf', 'stbl'];
// The samples an MP4 file says it has: {offsets, sizes, deltas, keys}, with the track's size,
// timescale, durations and avcC.
function readMp4(file) {
    const top = boxes(file, 0, file.length);
    if (!top) return null;
    const st = t => find(file, STBL.concat(t));
    const co = st('stco') || st('co64'), stts = entries(file, st('stts'), 4, 4, 2), deltas = [];
    for (let i = 0; i < stts.length; i += 2) for (let k = 0; k < stts[i]; k++) deltas.push(stts[i + 1]);
    const tkhd = find(file, ['moov', 'trak', 'tkhd']), mdhd = find(file, ['moov', 'trak', 'mdia', 'mdhd']);
    const mvhd = find(file, ['moov', 'mvhd']), avcC = find(file, STBL.concat('stsd', 'avc1'));
    const avcCBox = boxes(file, avcC.body + 78, avcC.at + avcC.size)[0];
    const stsz = st('stsz');
    return {
        top: top.map(b => b.type), mdat: top.find(b => b.type === 'mdat'), co64: co.type === 'co64',
        offsets: entries(file, co, 4, co.type === 'co64' ? 8 : 4), sizes: entries(file, stsz, 8, 4),
        sizeAll: u32(file, stsz.body + 4), deltas, keys: entries(file, st('stss'), 4, 4),
        width: u32(file, tkhd.body + 76) / 65536, height: u32(file, tkhd.body + 80) / 65536,
        scale: u32(file, mdhd.body + 12), duration: u32(file, mdhd.body + 16),
        movieScale: u32(file, mvhd.body + 12), movieDuration: u32(file, mvhd.body + 16),
        trackDuration: u32(file, tkhd.body + 20),
        entryWidth: file[avcC.body + 24] << 8 | file[avcC.body + 25], entryHeight: file[avcC.body + 26] << 8 | file[avcC.body + 27],
        avcC: avcCBox.type === 'avcC' ? file.slice(avcCBox.body, avcCBox.at + avcCBox.size) : null,
    };
}
function mp4File(samples, avcC, w, h, endUs) {
    const b = mp4Boxes(samples, avcC, w, h, endUs), parts = [b.ftyp, b.mdat, ...samples.map(s => s.data), b.moov];
    const out = new Uint8Array(parts.reduce((n, p) => n + p.length, 0));
    parts.reduce((at, p) => { out.set(p, at); return at + p.length; }, 0);
    return out;
}
const ticks = us => Math.round(us * MP4_SCALE / 1e6);
const equal = (a, b) => a.length === b.length && a.every((v, i) => v === b[i]);
function checkMp4(name, file, samples, avcC, w, h, endUs) {
    const m = readMp4(file);
    if (!m) return ok(name + ': the boxes read back', false);
    const want = samples.map((s, i) => Math.max(1, ticks(i + 1 < samples.length ? samples[i + 1].ts : endUs) - ticks(s.ts)));
    const total = want.reduce((a, b) => a + b, 0);
    ok(name + ': ftyp, mdat, moov, nothing after', equal(m.top, ['ftyp', 'mdat', 'moov']));
    ok(name + ': each sample where stco says, its size in stsz', m.offsets.length === samples.length && m.sizeAll === 0 &&
       samples.every((s, i) => m.sizes[i] === s.data.length && equal(file.subarray(m.offsets[i], m.offsets[i] + m.sizes[i]), s.data)) &&
       m.mdat.size === 8 + samples.reduce((n, s) => n + s.data.length, 0));
    ok(name + ': each lasts until the next, the last until the end', equal(m.deltas, want) && m.duration === total &&
       m.scale === MP4_SCALE);
    ok(name + ': the movie\'s duration in ms', m.movieScale === 1000 && m.movieDuration === Math.round(total * 1000 / MP4_SCALE) &&
       m.trackDuration === m.movieDuration);
    ok(name + ': the key frames in stss', equal(m.keys, samples.flatMap((s, i) => s.key ? [i + 1] : [])));
    ok(name + ': its size and avcC as given', m.width === w && m.height === h && m.entryWidth === w && m.entryHeight === h &&
       m.avcC && equal(m.avcC, avcC));
}
const fakeAvcC = new Uint8Array([1, 0x42, 0xC0, 0x33, 0xFF, 0xE1, 0, 4, 0x67, 0x42, 0xC0, 0x33, 1, 0, 2, 0x68, 0xCE]);
const fakeSamples = [
    {ts: 0, key: true, data: new Uint8Array(10).fill(0x11)},
    {ts: 66667, key: false, data: new Uint8Array(7).fill(0x22)},
    {ts: 133333, key: false, data: new Uint8Array(3).fill(0x33)},
    {ts: 2200000, key: true, data: new Uint8Array(5).fill(0x44)},   // (after a still stretch)
    {ts: 2200010, key: false, data: new Uint8Array(1).fill(0x55)},  // (within a tick: it still lasts one)
].map(s => Object.assign(s, {size: s.data.length}));
checkMp4('mp4', mp4File(fakeSamples, fakeAvcC, 1860, 2480, 2300000), fakeSamples, fakeAvcC, 1860, 2480, 2300000);
// Past 4 GB: mdat's 64-bit size and co64 (from the sizes alone: no 4 GB is made).
{
    const big = mp4Boxes([{size: 3e9, ts: 0, key: true}, {size: 2e9, ts: 66667, key: false}], fakeAvcC, 1860, 2480, 133333);
    const md = new DataView(big.mdat.buffer);
    const co = find(big.moov, STBL.concat('co64'));
    ok('mp4 past 4 GB: mdat\'s size in 64 bits, offsets in co64', big.mdat.length === 16 && md.getUint32(0) === 1 &&
       md.getUint32(8) * 4294967296 + md.getUint32(12) === 5e9 + 16 && !!co && !find(big.moov, STBL.concat('stco')) &&
       equal(entries(big.moov, co, 4, 8), [big.ftyp.length + 16, big.ftyp.length + 16 + 3e9]));
}

// CodecRecorder on a stand-in VideoEncoder: each chunk 6 bytes (a length-prefixed NAL saying which
// frame it is), the first chunk with a decoder config, outputs a millisecond after each encode.
class FakeEncoder {
    constructor({output, error}) {
        Object.assign(this, {output, error, state: 'unconfigured', encodeQueueSize: 0, encoded: []});
        FakeEncoder.last = this;
    }
    configure(config) {
        this.config = config;
        this.state = 'configured';
        if (FakeEncoder.refuse) setTimeout(() => { this.state = 'closed'; this.error(new Error('NotSupportedError')); }, 0);
    }
    encode(frame, opts) {
        if (this.state !== 'configured') throw new Error('InvalidStateError');
        const n = this.encoded.length, key = n === 0 || !!(opts && opts.keyFrame);
        this.encoded.push({ts: frame.timestamp, key, from: frame.source});
        const data = new Uint8Array([0, 0, 0, 2, key ? 0x65 : 0x41, n]);
        this.encodeQueueSize++;
        setTimeout(() => {
            this.encodeQueueSize--;
            this.output({type: key ? 'key' : 'delta', timestamp: frame.timestamp, byteLength: data.length, copyTo: d => d.set(data)},
                n ? {} : {decoderConfig: {codec: this.config.codec, description: fakeAvcC.slice().buffer}});
        }, 1);
    }
    flush() { return new Promise(r => setTimeout(r, 10)); }
    close() { this.state = 'closed'; }
}
class FakeFrame {
    constructor(source, init) { this.source = source; this.timestamp = init.timestamp; }
    close() {}
}
async function codecChecks() {
    globalThis.VideoEncoder = FakeEncoder;
    globalThis.VideoFrame = FakeFrame;
    // A clock of its own, moved by hand: the 1/15 s wait and the 2 s between key frames are then
    // exact however busy the machine is (only the wait's timer is real).
    let clock = 1000;
    performance.now = () => clock;
    const turn = () => new Promise(r => setImmediate(r));  // (the microtasks of this one have run)
    const canvas = {width: 1860, height: 2480};
    let finished;
    const result = new Promise(r => { finished = r; });
    const rec = new CodecRecorder(canvas, 'avc1.420033', finished), enc = FakeEncoder.last;
    ok('codecs: the encoder set for the canvas\'s size, as the probe asked', enc.config.codec === 'avc1.420033' &&
       enc.config.width === 1860 && enc.config.height === 2480 && enc.config.avc.format === 'avc');
    rec.frame(); rec.frame(); rec.frame();
    ok('codecs: nothing taken while the change is still being drawn', enc.encoded.length === 0);
    await turn();
    ok('codecs: three changes in one turn, one frame, a key frame at 0', enc.encoded.length === 1 &&
       enc.encoded[0].ts === 0 && enc.encoded[0].key && enc.encoded[0].from === canvas);
    clock += 10;
    rec.frame();
    await turn();
    ok('codecs: a change sooner than 1/15 s later waits', enc.encoded.length === 1 && !!rec.timer);
    clock += 60;  // (70 ms on: due once its timer comes)
    for (let i = 0; i < 500 && enc.encoded.length < 2; i++) await new Promise(r => setTimeout(r, 10));
    ok('codecs: and then comes by itself, no key frame', enc.encoded.length === 2 && enc.encoded[1].ts === 70000 &&
       !enc.encoded[1].key);
    clock += 2100;  // (2.1 s with nothing changing)
    rec.frame();
    await turn();
    ok('codecs: a change 2 s after the last key frame is one', enc.encoded.length === 3 && enc.encoded[2].key &&
       enc.encoded[2].ts === 2170000);
    rec.frame();
    rec.stop();  // (the change still waiting is taken, not lost)
    rec.frame();
    const blob = await result;
    ok('codecs: Stop takes the change still waiting, 1/15 s on, then nothing more', enc.encoded.length === 4 &&
       !enc.encoded[3].key && enc.encoded[3].ts === 2170000 + 66667 && enc.state === 'closed');
    if (!blob) return ok('codecs: an MP4 came', false);
    const file = new Uint8Array(await blob.arrayBuffer()), m = readMp4(file);
    ok('codecs: an MP4 (video/mp4) of the four chunks, in order', blob.type === 'video/mp4' && m &&
       equal(m.top, ['ftyp', 'mdat', 'moov']) && m.sizes.length === 4 &&
       m.offsets.every((o, i) => file[o + 5] === i) && equal(m.keys, [1, 3]) && equal(m.avcC, fakeAvcC));
    ok('codecs: the last frame lasts until Stop, a frame\'s time at least', m.deltas[3] >= ticks(1e6 / 15) &&
       m.width === 1860 && m.height === 2480);
    FakeEncoder.refuse = true;
    let refused;
    const nothing = new Promise(r => { refused = r; });
    new CodecRecorder(canvas, 'avc1.420033', refused);
    ok('codecs: an encoder that refuses gives nothing to save', await nothing === null);
    FakeEncoder.refuse = false;
    delete performance.now;  // (the real one again)
    delete globalThis.VideoEncoder;
    delete globalThis.VideoFrame;
}

// A real H.264 stream (libx264, Baseline, an access unit delimiter before each frame) cut into
// WebCodecs' 'avc' chunks and avcC as a browser gives them, at uneven times (a still stretch in the
// middle), put in an MP4: ffmpeg must decode it to the very frames of the stream, at those times.
function ffmpegChecks(work) {
    let encoders = '';
    try { encoders = execFileSync('ffmpeg', ['-hide_banner', '-encoders'], {encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore']}); }
    catch (e) { /* no ffmpeg */ }
    if (!/libx264/.test(encoders)) {
        console.log('skip mp4: ffmpeg\'s decode of a real stream (no ffmpeg with libx264)');
        return;
    }
    const src = path.join(work, 'mp4-src.h264'), out = path.join(work, 'mp4-round.mp4');
    execFileSync('ffmpeg', ['-v', 'error', '-y', '-f', 'lavfi', '-i', 'testsrc2=size=372x496:rate=15', '-frames:v', '40',
        '-c:v', 'libx264', '-profile:v', 'baseline', '-g', '12', '-pix_fmt', 'yuv420p', '-x264-params', 'aud=1',
        '-f', 'h264', src]);
    const es = new Uint8Array(fs.readFileSync(src)), nals = [];
    for (let i = 0, start = -1; i <= es.length; i++) {
        const sc = i + 2 < es.length && es[i] === 0 && es[i + 1] === 0 && es[i + 2] === 1;
        if (sc || i === es.length) {
            if (start >= 0) {
                let end = i;
                while (end > start && es[end - 1] === 0 && i < es.length) end--;  // (a 4-byte start code's zero)
                nals.push(es.subarray(start, end));
            }
            if (sc) { start = i + 3; i += 2; }
        }
    }
    let sps = null, pps = null;
    const units = [];
    for (const nal of nals) {
        const t = nal[0] & 31;
        if (t === 9) units.push([]);
        else if (t === 7) sps = sps || nal;
        else if (t === 8) pps = pps || nal;
        else if ((t === 1 || t === 5) && units.length) units[units.length - 1].push(nal);
    }
    const avcC = new Uint8Array([1, sps[1], sps[2], sps[3], 0xFF, 0xE1, sps.length >> 8, sps.length & 255, ...sps,
        1, pps.length >> 8, pps.length & 255, ...pps]);
    const samples = units.filter(u => u.length).map((u, i) => {
        const data = new Uint8Array(u.reduce((n, nal) => n + 4 + nal.length, 0));
        let at = 0;
        for (const nal of u) {
            new DataView(data.buffer).setUint32(at, nal.length);
            data.set(nal, at + 4);
            at += 4 + nal.length;
        }
        const ts = Math.round(i * 1e6 / 15) + (i >= 20 ? 1500000 : 0);
        return {data, size: data.length, ts, key: u.some(nal => (nal[0] & 31) === 5)};
    });
    const endUs = samples[samples.length - 1].ts + 250000;
    fs.writeFileSync(out, mp4File(samples, avcC, 372, 496, endUs));
    const md5 = file => execFileSync('ffmpeg', ['-v', 'error', '-i', file, '-fps_mode', 'passthrough', '-f', 'framemd5', '-'],
        {encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe']})
        .split('\n').filter(l => l && l[0] !== '#').map(l => l.split(',').pop().trim());
    let a = [], b = [], err = '';
    try {
        a = md5(src);
        b = md5(out);
        const r = spawnSync('ffmpeg', ['-v', 'error', '-i', out, '-f', 'null', '-'], {encoding: 'utf8'});
        err = r.status === 0 ? r.stderr : 'ffmpeg exited ' + r.status + ': ' + r.stderr;  // (any complaint at all)
    } catch (e) { err = String(e.stderr || e); }
    ok('mp4 (ffmpeg): ' + samples.length + ' frames of a real stream decode as the stream\'s own', samples.length === 40 &&
       a.length === 40 && equal(a, b) && !err);
    let probe = {};
    try {
        probe = JSON.parse(execFileSync('ffprobe', ['-v', 'error', '-select_streams', 'v:0', '-show_entries',
            'stream=codec_name,profile,width,height,time_base:packet=pts,duration,flags', '-of', 'json', out], {encoding: 'utf8'}));
    } catch (e) { /* the check below fails */ }
    const st = (probe.streams || [])[0] || {}, pk = probe.packets || [];
    const want = samples.map((s, i) => [ticks(s.ts), (i + 1 < samples.length ? ticks(samples[i + 1].ts) : ticks(endUs)) - ticks(s.ts)]);
    ok('mp4 (ffprobe): H.264 Baseline, 372x496, each frame at its time for as long as it showed',
       st.codec_name === 'h264' && /Baseline/.test(st.profile || '') && st.width === 372 && st.height === 496 &&
       st.time_base === '1/' + MP4_SCALE && pk.length === 40 &&
       pk.every((p, i) => Number(p.pts) === want[i][0] && Number(p.duration) === want[i][1] &&
                          (p.flags[0] === 'K') === samples[i].key));
}

codecChecks().catch(e => ok('codecs: ' + e, false)).then(() => {
    try { ffmpegChecks(path.dirname(bin)); } catch (e) { ok('mp4 (ffmpeg): ' + e, false); }
    process.exit(fail);
});
