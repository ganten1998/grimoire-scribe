#!/usr/bin/env node
// live_check.js: the browser's frame reader (web/live.js, Frames) on a stream the Kindle sent
// (tests/live_stream.py keeps its bytes and the frames it read): cut into pieces of every size,
// from one byte up, the frames must come out the same, in order. Then the page's pure parts: the
// front light's color, the pen frame and its mark, where a turned screen goes in a recording, and
// Record's format for what a browser records.
//   usage: node tests/live_check.js <stream.bin> <stream.txt>
'use strict';
const fs = require('fs');
const path = require('path');
const { Frames, lightColor, readPen, penMark, filmFit, recordType } = require(path.join(__dirname, '..', 'web', 'live.js'));

const [bin, txt] = process.argv.slice(2);
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
// The pen frame, little-endian, and away as null.
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
// Record's format. What each browser said (Chrome 156, Safari: MP4 only, Firefox 156: VP8 only).
const chrome = () => true, safari = t => t === 'video/mp4', firefox = t => t === 'video/webm;codecs=vp8' || t === 'video/webm';
const thrower = () => { throw new Error('no'); };
const as = (f, clear) => { const r = recordType(f, clear); return [r.mime, r.ext, r.why].join('|'); };
ok('record: Chrome, an MP4 of H.264', as(chrome) === 'video/mp4;codecs=avc1.42E01E|mp4|');
ok('record: Safari, its plain MP4', as(safari) === 'video/mp4|mp4|');
ok('record: Firefox, WebM, and the button says why', as(firefox) === 'video/webm;codecs=vp8|webm|browser');
ok('record: Transparent paper, WebM where it can (it keeps the paper clear)', as(chrome, true) === 'video/webm;codecs=vp8|webm|clear' &&
   as(firefox, true) === 'video/webm;codecs=vp8|webm|clear');
ok('record: Transparent paper without WebM, an MP4 (the page draws the paper in)', as(safari, true) === 'video/mp4|mp4|');
ok('record: a browser that says nothing (or throws), its own choice, no reason claimed', as(() => false) === '||' &&
   as(thrower) === '||');
process.exit(fail);
