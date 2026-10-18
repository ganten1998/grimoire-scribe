#!/usr/bin/env node
// studio_check.js: the share studio's picture against ImageMagick's. For each setting below,
// web/studio.js renders the page from its two layers, the magick command the studio prints is
// run on the same two files, and the pixels must agree; the studio's PNG, zip and PDF writers
// are checked on the way. Needs node 18+ and ImageMagick 7 (tests/share_check.sh skips it
// without them).
//   usage: node tests/studio_check.js <ink.png> <paper.png> <work dir> [<X-Ink-Box from the Kindle>]
'use strict';
const fs = require('fs');
const path = require('path');
const { execFileSync } = require('child_process');
const S = require(path.join(__dirname, '..', 'web', 'studio.js'));

const [inkPath, paperPath, work, kindleBox] = process.argv.slice(2);
if (!work) {
    console.error('usage: studio_check.js <ink.png> <paper.png> <work dir>');
    process.exit(2);
}

const CASES = [
    ['original', {}],
    ['transparent', { bg: 'none', lines: 'hide' }],
    ['colors', { bg: 'color', bgColor: '#20304a', ink: 'color', inkColor: '#ffd166', lineColor: '#88ccff', lineOpacity: 40 }],
    ['gradients', { bg: 'gradient', bgFrom: '#0f2027', bgTo: '#2c5364', bgAngle: 160, ink: 'gradient', inkFrom: '#a8ff78',
        inkTo: '#78ffd6', inkAngle: 90 }],
    ['gray warm', { tone: 'gray', warmth: 60, ink: 'color', inkColor: '#c0392b' }],
    ['duotone tint', { tone: 'duotone', tint: '#3366ff', tintAmount: 25 }],
    ['invert bc', { tone: 'invert', brightness: 10, contrast: 30 }],
    ['cool low contrast', { warmth: -50, brightness: -20, contrast: -40, bg: 'gradient', bgAngle: 33 }],
    ['sticker 50%', { bg: 'none', lines: 'hide', ink: 'gradient', crop: 'trim', margin: 37, scale: 50, inkAngle: 200 }],
    ['gradient across the page', { bg: 'none', ink: 'gradient', inkSpan: 'page', inkAngle: 135 }],
    ['writing gradient, whole page', { ink: 'gradient', inkAngle: 20, lineColor: '#3355aa' }],
    ['quarter', { bg: 'gradient', bgAngle: 270, scale: 25, lineOpacity: 60, warmth: 30 }],
    ['trim opaque', { crop: 'trim', margin: 0, tintAmount: 10 }],
    ...S.PRESETS.map(([name, over]) => ['preset ' + name, over]),
];

function magickRGBA(file, w, h) {
    const buf = execFileSync('magick', [file, '-depth', '8', 'rgba:-'], { maxBuffer: 1 << 28 });
    if (buf.length !== w * h * 4) throw new Error(`${file}: ${buf.length} bytes, expected ${w}x${h} RGBA`);
    return buf;
}

// Alpha within 1, color within 2 where alpha >= 1/4 (below that a color level is not visible),
// premultiplied color within 2 everywhere.
function compare(a, b) {
    let maxA = 0, maxC = 0, maxP = 0, sum = 0;
    for (let i = 0; i < a.length; i += 4) {
        const aa = a[i + 3], ba = b[i + 3];
        maxA = Math.max(maxA, Math.abs(aa - ba));
        for (let c = 0; c < 3; c++) {
            const d = Math.abs(a[i + c] - b[i + c]);
            if (aa >= 64 && ba >= 64) maxC = Math.max(maxC, d);
            const pd = Math.abs((a[i + c] * aa) / 255 - (b[i + c] * ba) / 255);
            maxP = Math.max(maxP, pd);
            sum += pd;
        }
    }
    return { maxA, maxC, maxP: +maxP.toFixed(2), mean: +(sum / (a.length * 0.75)).toFixed(4),
             ok: maxA <= 1 && maxC <= 2 && maxP <= 2 };
}

(async () => {
    fs.mkdirSync(work, { recursive: true });
    fs.copyFileSync(inkPath, path.join(work, 'ink.png'));
    fs.copyFileSync(paperPath, path.join(work, 'paper.png'));
    const ink = await S.decodeGray(new Uint8Array(fs.readFileSync(inkPath)));
    const paper = await S.decodeGray(new Uint8Array(fs.readFileSync(paperPath)));
    const L = { w: ink.w, h: ink.h, ink: ink.data, paper: paper.data };
    // The decoder against ImageMagick's reading of the same file.
    const ref = magickRGBA(inkPath, L.w, L.h);
    let bad = 0;
    for (let i = 0; i < L.w * L.h; i++) bad += ref[i * 4] !== L.ink[i];
    let fail = bad ? 1 : 0;
    console.log(`${bad ? 'FAIL' : 'ok  '} PNG decoder (${bad} pixels differ)`);
    if (kindleBox) {
        const b = S.inkBox(L), mine = b ? `${b.x},${b.y},${b.w},${b.h}` : 'none';
        const same = mine === kindleBox.trim();
        if (!same) fail = 1;
        console.log(`${same ? 'ok  ' : 'FAIL'} ink box: Kindle ${kindleBox.trim()}, studio ${mine}`);
    }

    let pdfPages = [];
    for (const [name, over] of CASES) {
        const p = S.normalize(Object.assign({}, S.DEFAULTS, over));
        const step = 100 / p.scale, box = S.exportBox(p, L, step);
        const t0 = Date.now();
        const img = S.render(p, L, box, step);
        const ms = Date.now() - t0;
        const out = 'out-' + name.replace(/\W+/g, '_') + '.png';
        const cmd = S.imCommand(p, S.commandContext(p, L, box, step, { ink: 'ink.png', paper: 'paper.png', out }));
        execFileSync('/bin/sh', ['-c', cmd], { cwd: work });
        const im = magickRGBA(path.join(work, out), img.w, img.h);
        const r = compare(img.rgba, im);
        // The studio's own PNG must hold exactly its pixels.
        const png = await S.encodePNG(img, 300 / step);
        fs.writeFileSync(path.join(work, 'studio-' + out), png);
        const back = magickRGBA(path.join(work, 'studio-' + out), img.w, img.h);
        let lossy = 0;
        for (let i = 0; i < back.length; i++) lossy += back[i] !== img.rgba[i];
        const ok = r.ok && !lossy;
        if (!ok) fail = 1;
        console.log(`${ok ? 'ok  ' : 'FAIL'} ${name}: ${img.w}x${img.h}${img.alpha ? ' alpha' : ''}${img.gray ? ' gray' : ''}` +
                    ` | vs magick: alpha ${r.maxA}, color ${r.maxC}, premul ${r.maxP} (mean ${r.mean})` +
                    ` | png ${png.length >> 10} KiB${lossy ? `, ${lossy} bytes differ` : ''} | ${ms} ms`);
        if (pdfPages.length < 3) pdfPages.push(await S.pdfImage(img, 300 / step));
    }

    // The zip and the PDF: valid enough for unzip, and every xref offset lands on its object.
    const zip = Buffer.from(await S.zipFiles([{ name: 'Notes/page-001.png', data: fs.readFileSync(path.join(work, 'ink.png')) },
                                             { name: 'Notes/päge-002.png', data: new Uint8Array(10) }]).arrayBuffer());
    fs.writeFileSync(path.join(work, 'test.zip'), zip);
    let zipOk = true;
    try {
        execFileSync('unzip', ['-tq', path.join(work, 'test.zip')], { stdio: 'pipe' });
    } catch (e) {
        zipOk = false;
    }
    console.log(`${zipOk ? 'ok  ' : 'FAIL'} zip`);
    if (!zipOk) fail = 1;
    const pdf = Buffer.from(await S.pdfFile(pdfPages, 'Notes été \u{1F600}', 'test').arrayBuffer());
    fs.writeFileSync(path.join(work, 'test.pdf'), pdf);
    const text = pdf.toString('latin1');
    const xref = text.slice(text.lastIndexOf('\nxref\n') + 1);
    const offs = [...xref.matchAll(/^(\d{10}) 00000 n $/gm)].map(m => +m[1]);
    const startxref = +text.match(/startxref\n(\d+)/)[1];
    const pdfOk = offs.length > 0 && offs.every((o, i) => text.startsWith(`${i + 1} 0 obj`, o)) &&
                  text.startsWith('xref', startxref) && (text.match(/\/Type \/Page /g) || []).length === pdfPages.length;
    console.log(`${pdfOk ? 'ok  ' : 'FAIL'} pdf (${offs.length} objects, ${pdfPages.length} pages)`);
    if (!pdfOk) fail = 1;
    console.log(fail ? 'STUDIO CHECK FAILED' : 'STUDIO CHECK PASSED');
    process.exit(fail);
})().catch(e => {
    console.error(e);
    process.exit(1);
});
