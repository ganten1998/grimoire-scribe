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
const { Frames, lightColor, readPen, penMark, drawMark, filmFit, recordType, H264, filmConfig, filmChoices, annexbNals,
    annexbSample, avcCOf, mp4Boxes,
        MP4_SCALE, CodecRecorder } = require(path.join(__dirname, '..', 'web', 'live.js'));

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
// Its mark: at the pixel's center, scaled; a see-through dot hovering, a smaller solid one touching.
// drawMark on a stand-in context: the red filled at the mark's alpha, inside a light edge and a dark
// one drawn whole; the box whole pixels around all it drew (and the antialiased edge), so taking it
// away leaves nothing; and the context left as it was (film's picture is drawn after it, never
// see-through).
function drawn(m) {
    const ops = [], kept = [];
    const ctx = {globalAlpha: 1, lineWidth: 1, fillStyle: '#000', strokeStyle: '#000', at: null,
        save() { kept.push([this.globalAlpha, this.lineWidth, this.fillStyle, this.strokeStyle]); },
        restore() { [this.globalAlpha, this.lineWidth, this.fillStyle, this.strokeStyle] = kept.pop(); },
        beginPath() { this.at = null; },
        arc(x, y, r) { this.at = {x, y, r}; },
        fill() { ops.push({fill: this.fillStyle, alpha: this.globalAlpha, ...this.at, reach: this.at.r}); },
        stroke() {
            ops.push({stroke: this.strokeStyle, alpha: this.globalAlpha, ...this.at, reach: this.at.r + this.lineWidth / 2});
        }};
    drawMark(ctx, m);
    const fills = ops.filter(o => o.fill), edges = ops.filter(o => o.stroke);
    return {ops, fills, edges, left: !kept.length && ctx.globalAlpha === 1 && ctx.lineWidth === 1 &&
        ctx.fillStyle === '#000' && ctx.strokeStyle === '#000'};
}
function covers(m) {
    const reach = Math.max(...drawn(m).ops.map(o => o.reach));
    return Number.isInteger(m.box.x) && Number.isInteger(m.box.y) && Number.isInteger(m.box.w) &&
        Number.isInteger(m.box.h) && m.box.x <= m.x - reach - 1 && m.box.y <= m.y - reach - 1 &&
        m.box.x + m.box.w >= m.x + reach + 1 && m.box.y + m.box.h >= m.y + reach + 1;
}
const hover = penMark({x: 100, y: 200, state: 1}, 0.25, 2), touch = penMark({x: 100, y: 200, state: 2}, 0.25, 2);
ok('mark: at the pixel\'s center, scaled (25.125, 50.125)', hover.x === 25.125 && hover.y === 50.125);
ok('mark: a see-through dot hovering, a smaller solid one touching', hover.alpha > 0.3 && hover.alpha < 0.8 &&
   touch.alpha === 1 && touch.r < hover.r);
const unit = penMark({x: 0, y: 0, state: 1}, 1, 1);
ok('mark: its unit scales it (2 CSS px to the pixel: twice the size)', hover.r === 2 * unit.r && hover.edge === 2 * unit.edge);
ok('mark: drawn as a dot, the red at its alpha, a light edge and a dark one whole, all around its place',
   [hover, touch].every(m => {
       const d = drawn(m);
       return d.fills.length === 1 && d.fills[0].fill === '#f03c28' && d.fills[0].alpha === m.alpha &&
           d.fills[0].r === m.r && d.edges.length === 2 && d.edges.every(e => e.alpha === 1 && e.r > m.r) &&
           d.edges.some(e => /255, *255, *255/.test(e.stroke)) && d.edges.some(e => /\(0, *0, *0,/.test(e.stroke)) &&
           d.ops.every(o => o.x === m.x && o.y === m.y);
   }));
ok('mark: the context left as it was', drawn(hover).left && drawn(touch).left);
ok('mark: the box covers all it draws, in whole pixels', [hover, touch, penMark({x: 1859, y: 2479, state: 1}, 1, 3),
    penMark({x: 3, y: 7, state: 2}, 0.3071, 1.5, 0, 542)].every(covers));
ok('mark: placed in a film at its offset', penMark({x: 0, y: 0, state: 2}, 0.75, 1, 0, 542).y === 542.375);
ok('mark: none while away', penMark(null, 1, 1) === null && penMark({x: 1, y: 1, state: 0}, 1, 1) === null);
// The dot's settings: smaller than it was by default (hovering 7, touching 5 CSS px then), any color.
const { DOT_SIZES, DOT_COLORS, dotColor, dotSize } = require(path.join(__dirname, '..', 'web', 'live.js'));
const big = penMark({x: 10, y: 10, state: 1}, 1, 1, 0, 0, {color: '#123abc', size: 'large'});
ok('dot: small by default, smaller than it used to be', unit.r === DOT_SIZES.small[0] && unit.r < 7 &&
   penMark({x: 0, y: 0, state: 2}, 1, 1).r === DOT_SIZES.small[1] && DOT_SIZES.small[1] < 5 && DOT_SIZES.medium.join() === '7,5');
ok('dot: its color and size as chosen, its box still covering it', big.r === DOT_SIZES.large[0] && big.color === '#123abc' &&
   covers(big) && drawn(big).fills[0].fill === '#123abc');
ok('dot: remembered values read back safely', dotColor('#ABCDEF') === '#abcdef' && dotColor('red') === DOT_COLORS[0] &&
   dotColor(null) === DOT_COLORS[0] && dotSize('large') === 'large' && dotSize('toString') === 'small' && dotSize(null) === 'small');
// A recording keeps its size: the same screen fills it, a turned one goes in whole, centered.
const same = filmFit(1860, 2480, 1860, 2480), side = filmFit(2480, 1860, 1860, 2480);
ok('film: the same size, as it is', same.k === 1 && same.x === 0 && same.y === 0);
ok('film: turned, whole and centered (0.75, at 0,542)', side.k === 0.75 && side.x === 0 && side.y === 542 &&
   2480 * side.k <= 1860 && side.y + 1860 * side.k <= 2480);
const bigger = filmFit(1986, 2648, 2648, 1986);
ok('film: the bigger Scribe turned back the other way fits too', bigger.y === 0 && bigger.x >= 0 &&
   bigger.x + 1986 * bigger.k <= 2648 && 2648 * bigger.k <= 1986 + 1e-9);
// Record's format. What each browser's MediaRecorder said (Chrome 156, Safari: MP4 only, Firefox
// 156 and Nightly 161: VP8 only), and h264, the codec its VideoEncoder took ('' for none).
const chrome = () => true, safari = t => t === 'video/mp4', firefox = t => t === 'video/webm;codecs=vp8' || t === 'video/webm';
const thrower = () => { throw new Error('no'); };
const film = codec => ({codec, width: 1860, height: 2480, format: 'avc', hw: ''});
const as = (f, codec) => { const r = recordType(f, codec ? film(codec) : null); return [r.mime, r.ext, r.why, r.via, r.codec || ''].join('|'); };
ok('record: Chrome, MediaRecorder\'s MP4 of H.264', as(chrome) === 'video/mp4;codecs=avc1.42E01E|mp4||recorder|' &&
   as(chrome, H264[0]) === 'video/mp4;codecs=avc1.42E01E|mp4||recorder|');
ok('record: Safari, its plain MP4', as(safari) === 'video/mp4|mp4||recorder|');
ok('record: Firefox with H.264 in VideoEncoder, the page\'s own MP4', as(firefox, 'avc1.420033') === 'video/mp4|mp4||codecs|avc1.420033' &&
   recordType(firefox, film('avc1.42E033')).film.width === 1860);
ok('record: Firefox without it, WebM, and the button says why', as(firefox) === 'video/webm;codecs=vp8|webm|browser|recorder|');
ok('record: no MediaRecorder at all but VideoEncoder, the page\'s MP4', as(() => false, 'avc1.42001f') === 'video/mp4|mp4||codecs|avc1.42001f');
ok('record: a browser that says nothing (or throws), its own choice, no reason claimed', as(() => false) === '|||recorder|' &&
   as(thrower) === '|||recorder|');
ok('record: Constrained Baseline H.264 first, at a level that holds both Scribes, Baseline only', H264[0] === 'avc1.42E033' &&
   H264.every(c => /^avc1\.42/.test(c)) && filmConfig(H264[0], 1860, 2480).avc.format === 'avc' &&
   filmConfig(H264[0], 1860, 2480, 'annexb', 'prefer-software').hardwareAcceleration === 'prefer-software' &&
   !('hardwareAcceleration' in filmConfig(H264[0], 2, 2)));
// What the probe asks VideoEncoder for a Scribe's screen: its own size first, both forms, the
// browser's choice of hardware first; then smaller even sizes that a hardware encoder takes.
const choices = filmChoices(1860, 2480);
ok('record: the probe asks the screen\'s size first, in both forms, then smaller even sizes',
   choices[0].width === 1860 && choices[0].height === 2480 && choices[0].format === 'avc' && choices[0].hw === '' &&
   choices.some(c => c.format === 'annexb' && c.width === 1860) &&
   choices.some(c => c.height === 2304 && c.width % 2 === 0 && Math.abs(c.width / c.height - 1860 / 2480) < 0.01) &&
   choices.some(c => Math.max(c.width, c.height) === 1920) &&
   choices.every(c => c.width % 2 === 0 && c.height % 2 === 0 && Math.max(c.width, c.height) <= 2480) &&
   choices.findIndex(c => c.height === 2304) > choices.findIndex(c => c.hw === 'prefer-hardware'));
// Annex B (Firefox's encoder may give only it): the parameter sets go to avcC, the pictures become
// length-prefixed samples.
const sps = [0x67, 0x42, 0xC0, 0x33, 0xAA], pps = [0x68, 0xCE, 0x3C], idr = [0x65, 0x88, 0x84, 0x00, 0x00, 0x03, 0x01];
const annexb = Uint8Array.from([0, 0, 0, 1, 0x09, 0xF0, 0, 0, 0, 1, ...sps, 0, 0, 1, ...pps, 0, 0, 0, 1, ...idr]);
const nals = annexbNals(annexb), sample = annexbSample(annexb);
ok('record: Annex B read as its NAL units, start codes of 3 and 4 bytes', nals.length === 4 && nals[1].join() === sps.join() &&
   nals[2].join() === pps.join() && nals[3].join() === idr.join());
ok('record: an Annex B chunk as a length-prefixed sample, its SPS and PPS kept for avcC',
   sample.sample.join() === [0, 0, 0, idr.length, ...idr].join() && sample.sps.join() === sps.join() && sample.pps.join() === pps.join());
const avcc = avcCOf(Uint8Array.from(sps), Uint8Array.from(pps));
ok('record: avcC of one SPS and one PPS', avcc.join() === [1, 0x42, 0xC0, 0x33, 0xFF, 0xE1, 0, 5, ...sps, 1, 0, 3, ...pps].join());

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

// Fast ink exercises the same model and draw calls the page uses, with no GUI or timing sleeps.
const {readStyle, readSamples, FastInk, Playout, inkWidth, PaintAges, shade, INK_LEVEL, areaAverage} = require('../web/live.js');
const styleBytes = new Uint8Array(64), sd = new DataView(styleBytes.buffer);
styleBytes[0] = 1; styleBytes[1] = 2;
[1, 9, .1, .9, 2, .2, .8].forEach((v, i) => sd.setFloat32(4 + i * 4, v, true));
[0, 0, 1860, 2480, 1700, 0, 160, 1800].forEach((v, i) => sd.setInt32(32 + i * 4, v, true));
const inkStyle = readStyle(styleBytes);
ok('fast: style widths, pressure curve and UI exclusion read back', inkStyle.enabled && inkStyle.min === 1 &&
    inkStyle.max === 9 && inkStyle.gamma === 2 && inkStyle.exclude[0] === 1700);
ok('fast: pressure changes width and opacity', inkWidth(inkStyle, {pressure: 0}).width === 1 &&
    inkWidth(inkStyle, {pressure: 1}).width === 9 && inkWidth(inkStyle, {pressure: .5}).width > 2);
// Samples are known by their identity, the kernel's microseconds: several in one millisecond keep
// their order, and the tiles' watermark names one.
const makeSample = (id, x, state = 2, flags = 16) => ({id, x, y: 700, state, flags, pressure: .5, ms: 100});
const fastInk = new FastInk();
fastInk.accept(makeSample(1000, 300, 2, 20), inkStyle);
fastInk.accept(makeSample(1250, 400), inkStyle);
fastInk.accept(makeSample(1500, 500), inkStyle);
ok('fast: down and every move retained, even in the same millisecond', fastInk.segments.length === 3);
fastInk.reconcile(1250);
ok('fast: only what the tiles hold retires', fastInk.segments.length === 1 && fastInk.segments[0].to.id === 1500 &&
    fastInk.segments[0].from.id === 1250);
fastInk.reconcile(1500);
fastInk.accept(makeSample(1750, 600), inkStyle);
ok('fast: reconciliation keeps the join to authoritative ink', fastInk.segments.length === 1 && fastInk.segments[0].from.id === 1500);
fastInk.reconcile(9000);
fastInk.accept(makeSample(2000, 610), inkStyle);
ok('fast: late pen packets cannot redraw ink already in the tiles', fastInk.segments.length === 0);
ok('fast: a sample older than the last is refused', fastInk.accept(makeSample(1900, 600), inkStyle) === false &&
    fastInk.accept(makeSample(2000, 610, 0, 0), inkStyle) === true);
// The identities wrap at 2^32 us: 40 minutes on, a sample reads as older than the last one, unless a
// silence that long starts the line again.
const away = new FastInk(), later = (5000 + 40 * 60e6) >>> 0;
away.heard(0);
away.accept(makeSample(5000, 300, 1, 0), inkStyle);
ok('fast: 40 minutes away reads as older without the silence', away.accept(makeSample(later, 310, 1, 0), inkStyle) === false);
ok('fast: a silence starts the line again', away.heard(40 * 60e3) === true &&
    away.accept(makeSample(later, 310, 1, 0), inkStyle) === true && away.pen && away.pen.x === 310 && away.heard(40 * 60e3 + 5) === false);
// Exact half/full-cycle edges too: acknowledge with both stamps, including ms wrapping.
for (const gapUs of [2 ** 31 - 1, 2 ** 31, 2 ** 31 + 1, 2 ** 32 - 1, 2 ** 32, 2 ** 32 + 1]) {
    const model = new FastInk(), old = 5000, oldMs = 0xffff0000;
    const id = (old + gapUs) >>> 0, ms = (oldMs + Math.floor(gapUs / 1000)) >>> 0;
    model.heard(1000, oldMs);
    model.accept({...makeSample(old, 300, 2, 20), ms: oldMs}, inkStyle);
    model.reconcile(old, oldMs);
    const recovered = model.heard(1001, ms);  // this computer's clock barely advanced
    model.reconcile(old, oldMs);  // a reset/reconnect can restore the painted watermark
    model.accept({...makeSample(id, 400, 2, 20), ms}, inkStyle);
    model.reconcile(old, oldMs);  // or a layer batch can repeat it
    ok('recovery: exact ' + gapUs + ' us gap keeps new ink and restarts time', recovered &&
        model.segments.length === 1 && model.pen.t === 0 && model.pen.x === 400);
    model.reconcile(id, ms);
    model.reconcile(old, oldMs);  // a stale watermark must never move the ack backwards
    model.accept({...makeSample((id + 4000) >>> 0, 420), ms: (ms + 4) >>> 0}, inkStyle);
    ok('recovery: exact ' + gapUs + ' us gap acknowledges new ink and keeps its join',
        model.ack === id && model.segments.length === 1 && model.segments[0].from.id === id && model.pen.t === 4);
}
fastInk.reset();
fastInk.accept(makeSample(10, 1750, 2, 20), inkStyle);
fastInk.accept(makeSample(11, 400), inkStyle);
ok('fast: a UI tap moving onto paper never starts a stroke', fastInk.segments.length === 0);
fastInk.accept(makeSample(12, 400, 1, 0), inkStyle);
fastInk.accept(makeSample(13, 400, 2, 20), {...inkStyle, enabled: false});
fastInk.accept(makeSample(14, 500), inkStyle);
ok('fast: a contact that began over a menu stays disabled', fastInk.segments.length === 0);
fastInk.accept(makeSample(15, 400, 2, 20), inkStyle);
fastInk.accept(makeSample(17, 600, 2, 16 | 8), inkStyle);
ok('fast: a gap never draws a bridge across lost ink', fastInk.segments.length === 1 && !fastInk.active);
fastInk.accept(makeSample(18, 650, 2, 20), {...inkStyle, whole: true});
ok('fast: whole-stroke eraser waits for authoritative tiles', fastInk.segments.length === 1 && !fastInk.active);
fastInk.reset();
fastInk.line.reset();
fastInk.accept(makeSample(0xfffffc00, 300, 2, 20), inkStyle);
fastInk.accept(makeSample(0xfffffe00, 400), inkStyle);
fastInk.accept(makeSample(0x100, 500, 2, 20), inkStyle);
fastInk.reconcile(0xfffffe00);
ok('fast: reconciliation survives the identities\' wrap', fastInk.segments.length === 1 && fastInk.segments[0].to.id === 0x100);
ok('fast: the timeline runs on across the wrap, in milliseconds', Math.abs(fastInk.segments[0].to.t - 1.28) < 1e-9);
// The playout: samples that came in a burst show at their own pace, a few milliseconds behind.
{
    const ink2 = new FastInk(), po = new Playout();
    let now = 5000;
    // 20 samples 4 ms apart on the Kindle; the first five on time, then a burst of the rest 30 ms late.
    for (let i = 0; i < 20; i++) {
        const sm = makeSample(100000 + i * 4000, 300 + i * 10, 2, i ? 16 : 20);
        ink2.accept(sm, inkStyle);
        po.arrive(sm.t, i < 5 ? now + i * 4 : now + 76 + 30);
    }
    ok('playout: the offset is the quickest path, the delay at most PLAYOUT_MAX', po.offset === now && po.delay <= 160 && po.delay > 0);
    // (at a time the clock has reached the samples: it runs po.delay behind them)
    const until = po.clock(now + 50 + po.delay);
    const at = ink2.penAt(until);
    ok('playout: the dot glides between samples', at && at.state === 2 && at.x > 300 && Math.abs((at.x - 300) / 10 * 4 - until) < 1e-6);
    ok('playout: more is still to play, so another frame is asked', ink2.pending(until) && !ink2.pending(1e9));
    const lines = [];
    const rec = new Proxy({}, {get: (_, key) => (...args) => lines.push([key, ...args]), set: () => true});
    ink2.draw(rec, 1, {invert: false, clear: false, light: [255, 255, 255]}, null, until);
    const tos = lines.filter(c => c[0] === 'lineTo');
    ok('playout: the line drawn as far as the clock, its last segment cut where it falls',
       tos.length === Math.floor(until / 4) + 1 && Math.abs(tos[tos.length - 1][1] - at.x) < 1e-6);
    ok('playout: none past a lift (no glide from touching to hovering)', (() => {
        const i3 = new FastInk();
        i3.accept(makeSample(1000, 100, 2, 20), inkStyle);
        i3.accept(makeSample(5000, 200, 1, 0), inkStyle);
        const p = i3.penAt(2);
        return p.x === 100 && p.state === 2 && i3.penAt(4).state === 1 && i3.penAt(-1) === null;
    })());
    ok('playout: a dot away is no dot', (() => {
        const i4 = new FastInk();
        i4.accept(makeSample(1000, 100, 1, 0), inkStyle);
        i4.accept(makeSample(2000, 100, 0, 0), inkStyle);
        return i4.penAt(0).state === 1 && i4.penAt(1) === null;
    })());
    const p2 = new Playout();
    ok('playout: before any sample everything shows', p2.clock(123) === Infinity);
    // The dot's motion at 60 frames a second, from a pen moving 1 px every 4 ms whose samples reach
    // the page in bursts every 20 ms (Wi-Fi's way), with and without the playout: the step from
    // one frame to the next should be the pen's own 4.2 px.
    const ink3 = new FastInk(), po3 = new Playout(), arrivals = [];
    for (let i = 0; i < 1000; i++) arrivals.push({i, at: Math.ceil((i * 4 + 3) / 20) * 20});
    let next = 0, newest = null;
    const plain = [], played = [], delays = [];
    for (let now = 100; now < 3900; now += 1000 / 60) {
        while (next < arrivals.length && arrivals[next].at <= now) {
            const sm = makeSample(1e6 + arrivals[next].i * 4000, 100 + arrivals[next].i, 1, 0);
            ink3.accept(sm, inkStyle);
            po3.arrive(sm.t, arrivals[next].at);
            newest = sm;
            next++;
        }
        plain.push(newest ? newest.x : 0);
        const at = ink3.penAt(po3.clock(now));
        played.push(at ? at.x : 0);
        delays.push(po3.delay);
    }
    const steps = xs => xs.slice(1).map((x, k) => x - xs[k]).slice(60);  // (after the first second)
    const spread = xs => { const m = xs.reduce((a, b) => a + b, 0) / xs.length; return Math.sqrt(xs.reduce((a, b) => a + (b - m) * (b - m), 0) / xs.length); };
    const sp = spread(steps(plain)), sq = spread(steps(played)), most = Math.max(...steps(played).map(Math.abs));
    console.log('     playout: the dot\'s steps a frame (the pen moves 4.2 px), samples in 20 ms bursts: ' +
        'drawn as they come sd ' + sp.toFixed(2) + ' px, max ' + Math.max(...steps(plain)).toFixed(1) +
        ' px; played out sd ' + sq.toFixed(2) + ' px, max ' + most.toFixed(1) + ' px, added delay ' +
        delays[delays.length - 1].toFixed(1) + ' ms');
    ok('playout: bursts every 20 ms play out as a glide (the steps\' spread a tenth of drawing them as they come or less)',
       sq < sp / 10 && most < 5 && delays[delays.length - 1] <= 30);
    // A Wi-Fi stall: the pen moves on (1 px every 4 ms) but nothing reaches the page for 250 ms, then
    // it all comes at once. Drawn as it comes the dot jumps 60 px; played out the clock never goes back
    // and the dot moves on a little quicker for a moment (2.0.0-rc.10).
    const ink5 = new FastInk(), po5 = new Playout();
    const arr5 = [];
    for (let i = 0; i < 1500; i++) {
        const sent = i * 4 + 3;
        arr5.push({i, at: sent >= 2000 && sent < 2250 ? 2250 : sent});
    }
    let n5 = 0, newest5 = null, lastUntil = -Infinity, back = false;
    const plain5 = [], played5 = [], delays5 = [];
    for (let now = 100; now < 5900; now += 1000 / 60) {
        while (n5 < arr5.length && arr5[n5].at <= now) {
            const sm = makeSample(1e6 + arr5[n5].i * 4000, 100 + arr5[n5].i, 1, 0);
            ink5.accept(sm, inkStyle);
            po5.arrive(sm.t, arr5[n5].at);
            newest5 = sm;
            n5++;
        }
        const until = po5.clock(now);
        if (until < lastUntil) back = true;
        lastUntil = until;
        plain5.push(newest5 ? newest5.x : 0);
        const at = ink5.penAt(until);
        played5.push(at ? at.x : 0);
        delays5.push(po5.delay);
    }
    const jump = xs => Math.max(...xs.slice(1).map((x, k) => x - xs[k]).slice(60));
    console.log('     playout: a 250 ms stall: drawn as it comes the dot jumps ' + jump(plain5).toFixed(1) +
        ' px a frame; played out at most ' + jump(played5).toFixed(1) + ' px (the pen moves 4.2 px a frame), delay ' +
        Math.max(...delays5).toFixed(0) + ' ms at most, ' + delays5[delays5.length - 1].toFixed(1) + ' ms at the end');
    ok('playout: after a stall the clock never goes back, and the dot catches up in steps under 3x its own',
       !back && jump(plain5) > 40 && jump(played5) < 3 * 4.2 + 0.5);
    ok('playout: the delay comes back down once the link is clean again', delays5[delays5.length - 1] < 25);
    // The pen away for a second (no samples), then a new stroke: it shows within its delay, not after a
    // glide across the empty second.
    const po6 = new Playout();
    for (let i = 0; i < 100; i++) po6.arrive(1000 + i * 4, 5000 + i * 4);
    for (let now = 5000; now < 5400; now += 16) po6.clock(now);
    po6.arrive(1000 + 99 * 4 + 1000, 5000 + 99 * 4 + 1000);
    const resumed = po6.clock(5000 + 99 * 4 + 1000 + 16);
    ok('playout: after a gap the next sample shows within its delay', resumed >= 1000 + 99 * 4 + 1000 - 1);
}
let commands = [];
const ctx = new Proxy({}, {get: (_, key) => (...args) => commands.push([key, ...args]),
    set: (_, key, value) => { commands.push([key, value]); return true; }});
const plain = {invert: false, clear: false, light: [255, 255, 255]};
fastInk.draw(ctx, .5, plain);
ok('fast: overlay clips to page and away from panel, then draws a round mark', commands.some(c => c[0] === 'clip' && c[1] === 'evenodd') &&
    commands.some(c => c[0] === 'arc') && commands.some(c => c[0] === 'fill'));
// The quick line's colors are repaint()'s: paper black while inverted, else the light; ink light
// while inverted, else black; with transparent paper, ink as opaque as it is dark and no paper.
const warm = [255, 200, 150];
ok('fast colors: ink black, and the paper the light, as the picture shows them', shade(0, plain).join() === '0,0,0,1' &&
    shade(255, {...plain, light: warm}).join() === '255,200,150,1');
ok('fast colors: inverted, the paper black and the ink the light', shade(255, {...plain, invert: true}).join() === '0,0,0,1' &&
    shade(0, {...plain, invert: true, light: warm}).join() === '255,200,150,1');
ok('fast colors: transparent paper shows no paper, and ink as dark as it is', shade(255, {...plain, clear: true})[3] === 0 &&
    shade(0, {...plain, clear: true}).join() === '0,0,0,1' && shade(0, {...plain, clear: true, invert: true, light: warm}).join() === '255,200,150,1');
ok('fast colors: the highlighter is its own gray, not black', INK_LEVEL[4] === 110 && shade(INK_LEVEL[4], plain)[0] === 110);
// A see-through run is drawn whole on the layer, then laid over the overlay once at its opacity: its
// segments never pile up toward black. One that is not see-through goes straight on the overlay.
function drawWith(ink, look) {
    const main = [], scratch = [];
    const rec = list => new Proxy({}, {get: (_, key) => (...args) => list.push([key, ...args]),
        set: (_, key, value) => { list.push([key, value]); return true; }});
    const layer = {canvas: {width: 10, height: 10}, ctx: rec(scratch)};
    ink.draw(rec(main), 1, look, layer);
    return {main, scratch, layer};
}
const highlighter = {...inkStyle, tool: 4, min: 35, max: 35, opacityMin: .5, opacityMax: .75};
const band = new FastInk();
for (let i = 1; i <= 40; i++) band.accept(makeSample(i, 300 + i * 3, 2, i === 1 ? 20 : 16), highlighter);
let got = drawWith(band, plain);
const composite = got.main.filter(c => c[0] === 'drawImage');
ok('fast: a highlighter stroke goes on the overlay once, at its opacity, in its gray', composite.length === 1 &&
    composite[0][1] === got.layer.canvas &&
    got.main.some(c => c[0] === 'globalAlpha' && Math.abs(c[1] - inkWidth(highlighter, {pressure: .5}).opacity) < 1e-6) &&
    !got.main.some(c => c[0] === 'stroke') && got.scratch.filter(c => c[0] === 'stroke').length === 39 &&
    got.scratch.some(c => c[0] === 'strokeStyle' && c[1] === 'rgb(110,110,110)'));
const opaque = new FastInk(), solid = {...inkStyle, opacityMin: 1, opacityMax: 1};
opaque.accept(makeSample(1, 300, 2, 20), solid);
opaque.accept(makeSample(2, 320), solid);
got = drawWith(opaque, plain);
ok('fast: an opaque stroke is drawn straight on the overlay', !got.main.some(c => c[0] === 'drawImage') &&
    got.main.some(c => c[0] === 'stroke') && got.scratch.length === 0);
const white = new FastInk(), whiteInk = {...solid, white: true};
white.accept(makeSample(1, 300, 2, 20), whiteInk);
white.accept(makeSample(2, 320), whiteInk);
got = drawWith(white, {...plain, invert: true});
ok('fast: white ink inverted is the black paper', got.main.some(c => c[0] === 'strokeStyle' && c[1] === 'rgb(0,0,0)'));
got = drawWith(white, {...plain, clear: true});
ok('fast: white ink on transparent paper takes the quick ink under it away',
    got.main.some(c => c[0] === 'globalCompositeOperation' && c[1] === 'destination-out'));
fastInk.reset();
for (let i = 1; i <= 3000; i++) fastInk.accept(makeSample(i, i % 1000, 2, i === 1 ? 20 : 16), inkStyle);
ok('fast: disconnected tile transport cannot grow memory indefinitely', fastInk.segments.length === 2048);
// The exact area average (the page view, Picture in picture, a smaller recording): a 1 px line keeps
// its weight wherever it falls, and only the pixels a changed rect reaches are made.
{
    const W = 30, H = 10, src = new Uint8ClampedArray(W * H * 4).fill(255);
    for (let y = 0; y < H; y++) { const i = (y * W + 13) * 4; src[i] = src[i + 1] = src[i + 2] = 0; }  // a black column at x 13
    const reads = [];
    const read = (x, y, w, h) => {
        reads.push([x, y, w, h]);
        const out = new Uint8ClampedArray(w * h * 4);
        for (let r = 0; r < h; r++) out.set(src.subarray(((y + r) * W + x) * 4, ((y + r) * W + x + w) * 4), r * w * 4);
        return out;
    };
    const all = areaAverage(read, W, H, 10, 10, {x: 0, y: 0, w: W, h: H}, null);
    const col = x => all.data[(5 * 10 + x) * 4];
    ok('area: a 1 px line at a third of the size is a third as dark, in one column', all.w === 10 && all.h === 10 &&
       Math.abs(col(4) - 170) <= 1 && col(3) === 255 && col(5) === 255);
    reads.length = 0;
    const part = areaAverage(read, W, H, 10, 10, {x: 12, y: 2, w: 2, h: 1}, null);
    ok('area: a changed rect makes only the pixels it reaches, from the source it needs', part.x === 4 && part.w === 1 &&
       part.y === 2 && part.h === 1 && reads.length === 1 && reads[0].join() === '12,2,3,1' && Math.abs(part.data[0] - 170) <= 1);
    const clearSrc = new Uint8ClampedArray(4 * 4).fill(0);
    clearSrc.set([0, 0, 0, 255], 0);
    const flat = areaAverage(() => clearSrc, 2, 2, 1, 1, {x: 0, y: 0, w: 2, h: 2}, [200, 100, 50]);
    ok('area: over paper where the film has no transparency', flat.data[3] === 255 && Math.abs(flat.data[0] - 150) <= 1 &&
       Math.abs(flat.data[1] - 75) <= 1);
}
// The span tables are kept between calls: the same pixels as the spans worked out afresh each
// call (the reference below, rc.7's), to within a rounding, and every pixel the same whatever rect
// asked for it (a changed rect's pixels are the whole picture's).
{
    function reference(read, sw, sh, dw, dh, d) {
        const kx = sw / dw, ky = sh / dh;
        const vx0 = Math.max(0, Math.floor(d.x / kx)), vy0 = Math.max(0, Math.floor(d.y / ky));
        const vx1 = Math.min(dw, Math.ceil((d.x + d.w) / kx)), vy1 = Math.min(dh, Math.ceil((d.y + d.h) / ky));
        if (vx1 <= vx0 || vy1 <= vy0) return null;
        const sx0 = Math.floor(vx0 * kx), sy0 = Math.floor(vy0 * ky);
        const rw = Math.min(sw, Math.ceil(vx1 * kx)) - sx0, rh = Math.min(sh, Math.ceil(vy1 * ky)) - sy0, ow = vx1 - vx0, oh = vy1 - vy0;
        const src = read(sx0, sy0, rw, rh), out = new Float64Array(ow * oh * 4);
        const spans = (v0, k, s0, limit, n) => Array.from({length: n}, (_, i) => {
            const a = (v0 + i) * k - s0, b = Math.min(a + k, limit), cols = [];
            for (let s = Math.floor(a); s < b; s++) cols.push(s, (Math.min(b, s + 1) - Math.max(a, s)) / (b - a));
            return cols;
        });
        const xs = spans(vx0, kx, sx0, rw, ow), ys = spans(vy0, ky, sy0, rh, oh);
        for (let o = 0; o < oh; o++) for (let c = 0; c < ow; c++) {
            const acc = [0, 0, 0, 0];
            for (let j = 0; j < ys[o].length; j += 2) for (let i = 0; i < xs[c].length; i += 2) {
                const q = ((ys[o][j]) * rw + xs[c][i]) * 4, w = ys[o][j + 1] * xs[c][i + 1] * src[q + 3];
                for (let ch = 0; ch < 3; ch++) acc[ch] += src[q + ch] * w;
                acc[3] += w;
            }
            const q = (o * ow + c) * 4;
            for (let ch = 0; ch < 3; ch++) out[q + ch] = acc[3] > 0 ? acc[ch] / acc[3] : 0;
            out[q + 3] = acc[3];
        }
        return {x: vx0, y: vy0, w: ow, h: oh, data: out};
    }
    let seed = 11, worst = 0, mismatched = 0;
    const rnd = () => (seed = (seed * 1103515245 + 12345) & 0x7fffffff) / 0x7fffffff;
    for (let t = 0; t < 300; t++) {
        const sw = 1 + Math.floor(rnd() * 120), sh = 1 + Math.floor(rnd() * 120);
        const dw = 1 + Math.floor(rnd() * sw), dh = 1 + Math.floor(rnd() * sh);
        const px = new Uint8ClampedArray(sw * sh * 4).map(() => rnd() < 0.5 ? 255 : Math.floor(rnd() * 256));
        const read = (x, y, w, h) => {
            const o = new Uint8ClampedArray(w * h * 4);
            for (let r = 0; r < h; r++) o.set(px.subarray(((y + r) * sw + x) * 4, ((y + r) * sw + x + w) * 4), r * w * 4);
            return o;
        };
        const x = Math.floor(rnd() * sw), y = Math.floor(rnd() * sh);
        const d = {x, y, w: 1 + Math.floor(rnd() * (sw - x)), h: 1 + Math.floor(rnd() * (sh - y))};
        const got = areaAverage(read, sw, sh, dw, dh, d, null), want = reference(read, sw, sh, dw, dh, d);
        const whole = areaAverage(read, sw, sh, dw, dh, {x: 0, y: 0, w: sw, h: sh}, null);
        for (let r = 0; r < got.h; r++) for (let c = 0; c < got.w; c++) for (let ch = 0; ch < 4; ch++) {
            const g = got.data[(r * got.w + c) * 4 + ch];
            if (got.data[(r * got.w + c) * 4 + 3] || ch === 3) worst = Math.max(worst, Math.abs(g - want.data[(r * want.w + c) * 4 + ch]));
            if (g !== whole.data[((got.y + r) * whole.w + got.x + c) * 4 + ch]) mismatched++;
        }
    }
    ok('area: kept span tables give the pixels worked out afresh, to within a rounding (' + worst.toFixed(2) + ')', worst < 1.01);
    ok('area: a pixel is the same whatever rect asked for it', mismatched === 0);
}
// The chunk list: a big frame cut into many chunks comes out whole, once; the bytes still waiting
// read back.
{
    const big = new Uint8Array(200005), d = new DataView(big.buffer);
    d.setUint32(0, 200001, true); big[4] = 2;
    for (let i = 5; i < big.length; i++) big[i] = i & 255;
    const f = new Frames(), got = [];
    for (let at = 0; at < big.length; at += 997) got.push(...f.push(big.subarray(at, at + 997)));
    ok('frames: a 200 KB frame in 997-byte chunks comes once, its body whole and its own bytes', got.length === 1 &&
       got[0].body.length === 200000 && got[0].body.byteOffset === 0 && got[0].body.buffer.byteLength === 200000 &&
       got[0].body.every((v, i) => v === ((i + 5) & 255)) && f.buf.length === 0 && f.parts.length === 0);
    f.push(new Uint8Array([3, 0, 0, 0, 4, 9]));
    ok('frames: a frame cut short waits, its bytes kept', f.buf.join() === '3,0,0,0,4,9');
    ok('frames: and is read when the rest comes', f.push(new Uint8Array([8]))[0].body.join() === '9,8' && f.size === 0);
}
// The size engine's pure parts.
{
    const {liveSize, sizeValue, viewSize, zoomStep, anchorScroll, ZOOMS, pipTier} = require('../web/live.js');
    ok('size: what the browser remembers: Fit for anything else, kept to 25%..200%',
       liveSize(null).mode === 'fit' && liveSize('fit').mode === 'fit' && liveSize('junk').mode === 'fit' && liveSize('-1').mode === 'fit' &&
       liveSize('width').mode === 'width' && liveSize('0.5').k === 0.5 && liveSize('9').k === 2 && liveSize('0.01').k === 0.25 &&
       sizeValue(liveSize('0.75')) === '0.75' && sizeValue({mode: 'width'}) === 'width');
    const vs = (size, room, tall, ratio, rw = 1860, rh = 2480) => viewSize(size, room, tall, rw, rh, ratio);
    const fit = vs({mode: 'fit'}, 1188, 768, 2);
    ok('size: Fit, the whole page in the room (' + fit.cssW + ' x ' + fit.cssH + ' CSS, ' + fit.w + ' x ' + fit.h + ' px)',
       fit.cssH === 768 && fit.cssW === 576 && fit.w === 1152 && fit.h === 1536 && !fit.up && Math.abs(fit.k - 1152 / 1860) < 1e-9);
    const width = vs({mode: 'width'}, 1188, 768, 1);
    ok('size: Fit width, the page as wide as the room, taller than it', width.cssW === 1188 && width.cssH === 1584);
    const half = vs({mode: 'scale', k: 0.5}, 100, 100, 1.25), actual = vs({mode: 'scale', k: 1}, 100, 100, 1.25);
    ok('size: 50% and actual size at a 1.25 pixel ratio: whole device pixels, one per Kindle pixel at 100%',
       half.w === 930 && half.cssW === 744 && actual.w === 1860 && actual.cssW === 1488 && !actual.up);
    const twice = vs({mode: 'scale', k: 2}, 100, 100, 2);
    ok('size: 200% keeps the Kindle\'s pixels in the canvas, enlarged by the browser', twice.up && twice.w === 1860 &&
       twice.cssW === 1860 && twice.k === 2);
    const land = vs({mode: 'fit'}, 1000, 400, 1, 2480, 1860);
    ok('size: a turned screen fits by its height', land.cssH === 400 && land.cssW === 533);
    ok('size: the presets step up and down from any size, at the ends the ends',
       zoomStep(0.48, 1) === 0.5 && zoomStep(0.48, -1) === 1 / 3 && zoomStep(1, 1) === 1.5 && zoomStep(2, 1) === 2 &&
       zoomStep(0.25, -1) === 0.25 && zoomStep(0.5, -1) === 1 / 3 && ZOOMS.length === 8);
    const a = anchorScroll({left: 100, top: 50, width: 200, height: 300}, {left: 50, top: 0, width: 400, height: 600}, 150, 200);
    ok('size: zooming about the pointer keeps the point under it', a.dx === 0 && a.dy === 100);
    const win = extra => Object.assign({open() {}, document: {}}, extra);
    const capture = {HTMLCanvasElement: {prototype: {captureStream() {}}}};
    ok('pip: each browser\'s tier', pipTier(win({documentPictureInPicture: {requestWindow() {}}})) === 'document' &&
       pipTier(win(Object.assign({document: {pictureInPictureEnabled: true}, HTMLVideoElement: {prototype: {requestPictureInPicture() {}}}}, capture))) === 'video' &&
       pipTier(win(Object.assign({navigator: {userAgent: 'Mozilla/5.0 (Macintosh) Gecko/20100101 Firefox/152.0'}}, capture))) === 'firefox' &&
       pipTier(win({})) === 'popout' && pipTier({document: {}}) === '');
}
const clock = new PaintAges();
clock.sync(1000, 1020, 0xfffffffa);
clock.sync(1030, 1034, 14);
clock.sync(1040, 1080, 42);
ok('clock: smallest RTT selected and unsigned clock wrap handled', clock.best === 4 && clock.age(1050, 4) === 28);
clock.add(clock.samples, 1050, 4);
clock.add(clock.samples, 1060, 4);
clock.add(clock.samples, 1070, 4);
ok('clock: p50, p90, max come from observed ages', clock.stats(clock.samples).join() === '38,38,48');
clock.reset();
ok('clock: recovery forgets old calibration and observations', clock.best === Infinity &&
    clock.age(2000, 100) === null && clock.stats(clock.samples) === null && clock.stats(clock.tiles) === null);
clock.sync(2000, 2004, 1000);
clock.add(clock.samples, 2012, 1000);
ok('clock: recovery measures from the new calibration only', clock.stats(clock.samples).join() === '10,10,10');
const fastPath = path.join(path.dirname(bin), 'fast.bin');
if (fs.existsSync(fastPath)) {
    let actualStyle = null, count = 0, sawTilt = false;
    for (const frame of new Frames().push(new Uint8Array(fs.readFileSync(fastPath)))) {
        if (frame.type === 8) actualStyle = readStyle(frame.body);
        if (frame.type === 7) for (const s of readSamples(frame.body).samples) {
            count++; sawTilt ||= Number.isFinite(s.tiltX) && Number.isFinite(s.tiltY);
            if (s.flags & 16) ok('fast stream: drawable sample has enabled style', actualStyle && actualStyle.enabled);
        }
    }
    ok('fast: browser reads the real headless sample stream', count >= 50 && sawTilt);
}

require('./live_keys_check.js')(ok).then(() => require('./live_browser_check.js')(ok)).then(codecChecks).catch(e => ok('codecs: ' + e, false)).then(() => {
    try { ffmpegChecks(path.dirname(bin)); } catch (e) { ok('mp4 (ffmpeg): ' + e, false); }
    process.exit(fail);
});
