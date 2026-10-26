#!/usr/bin/env node
// live_check.js: the browser's frame reader (web/live.js, Frames) on a stream the Kindle sent
// (tests/live_stream.py keeps its bytes and the frames it read): cut into pieces of every size,
// from one byte up, the frames must come out the same, in order.
//   usage: node tests/live_check.js <stream.bin> <stream.txt>
'use strict';
const fs = require('fs');
const path = require('path');
const { Frames, lightColor } = require(path.join(__dirname, '..', 'web', 'live.js'));

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
process.exit(fail);
