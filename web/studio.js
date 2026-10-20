// studio.js: the share page's image studio, served inline at /nb/<id>/studio (httpd.c,
// route_studio). It rebuilds a page from its two layers, /nb/<id>/<n>.png?layer=paper and
// ?layer=ink: a background (white, transparent, a color or a gradient), the paper's lines and
// the ink each in their own color (the ink also in a gradient across the writing or the page),
// then a tone, warmth, a tint and brightness/contrast, a crop to the ink and a size. Every control is one ImageMagick operation:
// the page prints the magick command that makes the same picture from the two layer files, and
// the pixel math below is ImageMagick 7's for exactly those operations (tests/studio_check.js
// runs both and compares). PNG, zip and PDF are written here too, so a transparent edge keeps
// its exact color (a canvas would round it). The core is plain functions; node runs it as well.
(function () {
    'use strict';

    // ---------- settings ----------

    const DEFAULTS = Object.freeze({
        bg: 'white', bgColor: '#f4ecd8', bgFrom: '#fbe9d0', bgTo: '#e2bf8f', bgAngle: 180,
        lines: 'show', lineColor: '#000000', lineOpacity: 100,
        ink: 'black', inkColor: '#1d3557', inkFrom: '#fc466b', inkTo: '#3f5efb', inkAngle: 90, inkSpan: 'writing',
        tone: 'none', duoDark: '#3d2817', duoLight: '#f3e5c8',
        warmth: 0, tint: '#ff9b40', tintAmount: 0, brightness: 0, contrast: 0,
        crop: 'page', margin: 60, scale: 100,
    });

    // Marks a word for translation where it is a key as well: a preset's name is its key (Original
    // keeps no crop or size); only the page shows it, translated (t, below).
    const N_ = s => s;
    const PRESETS = [
        // TRANSLATORS: the studio's presets, each a whole look, on buttons.
        [N_('Original'), {}],
        [N_('Transparent'), { bg: 'none', lines: 'hide' }],
        [N_('Signature'), { bg: 'none', lines: 'hide', ink: 'gradient', crop: 'trim', margin: 40 }],
        [N_('Warm'), { warmth: 45 }],
        [N_('Sepia'), { tone: 'duotone' }],
        [N_('Parchment'), { bg: 'gradient', bgFrom: '#f7e9c6', bgTo: '#e4cc98', ink: 'color', inkColor: '#3b2a1a',
            lineColor: '#8b6b3d', lineOpacity: 45, warmth: 15 }],
        [N_('Blueprint'), { bg: 'color', bgColor: '#1f4e8c', ink: 'color', inkColor: '#ffffff', lineColor: '#ffffff',
            lineOpacity: 30 }],
        [N_('Chalkboard'), { bg: 'color', bgColor: '#2e3b33', ink: 'color', inkColor: '#f4f1e8', lineColor: '#ffffff',
            lineOpacity: 15 }],
        [N_('Night'), { tone: 'invert' }],
        [N_('Sunset ink'), { bg: 'none', lines: 'hide', ink: 'gradient', inkFrom: '#f12711', inkTo: '#f5af19', inkAngle: 135,
            crop: 'trim' }],
        [N_('Aurora'), { bg: 'gradient', bgFrom: '#0f2027', bgTo: '#2c5364', bgAngle: 160, lines: 'hide', ink: 'gradient',
            inkFrom: '#a8ff78', inkTo: '#78ffd6', inkAngle: 90 }],
    ];

    const CHOICES = {
        bg: ['white', 'none', 'color', 'gradient'], lines: ['show', 'hide'], ink: ['black', 'color', 'gradient'],
        inkSpan: ['writing', 'page'],
        tone: ['none', 'gray', 'duotone', 'invert'], crop: ['page', 'trim'], scale: [100, 50, 25],
    };
    const RANGES = {
        bgAngle: [0, 360], lineOpacity: [0, 100], inkAngle: [0, 360], warmth: [-100, 100], tintAmount: [0, 100],
        brightness: [-100, 100], contrast: [-90, 90], margin: [0, 600],
    };

    // Any stored or hand-made settings object made safe: known keys, valid colors, numbers in range.
    function normalize(raw) {
        const p = Object.assign({}, DEFAULTS);
        if (!raw || typeof raw !== 'object') return p;
        for (const k of Object.keys(DEFAULTS)) {
            const v = raw[k];
            if (v === undefined) continue;
            if (CHOICES[k]) {
                const hit = CHOICES[k].find(c => String(c) === String(v));
                if (hit !== undefined) p[k] = hit;
            } else if (RANGES[k]) {
                const n = Math.round(+v);
                if (Number.isFinite(n)) p[k] = Math.min(RANGES[k][1], Math.max(RANGES[k][0], n));
            } else if (typeof v === 'string' && /^#[0-9a-fA-F]{6}$/.test(v)) {
                p[k] = v.toLowerCase();
            }
        }
        return p;
    }

    // ---------- the math (ImageMagick 7's, operation by operation) ----------

    const clamp01 = v => (v < 0 ? 0 : v > 1 ? 1 : v);
    function rgb(hex) {
        const n = parseInt(hex.slice(1), 16);
        return [((n >> 16) & 255) / 255, ((n >> 8) & 255) / 255, (n & 255) / 255];
    }
    const num = v => String(+(+v).toFixed(4));

    // gradient:angle on a w x h image: the CSS linear-gradient line through the center
    // (GradientImage), as t = a*x + b*y + c at pixel (x, y), clamped to 0..1 by the caller.
    function gradLine(angle, w, h) {
        const r = ((angle - 90) * Math.PI) / 180, s = Math.sin(r), c = Math.cos(r);
        const d = Math.abs((w - 1) * c) + Math.abs((h - 1) * s);
        const x1 = 0.5 * (w - 1 - d * c), y1 = 0.5 * (h - 1 - d * s), px = d * c, py = d * s;
        const l2 = px * px + py * py;
        if (l2 < 1e-12) return { a: 0, b: 0, c: 0 };
        return { a: px / l2, b: py / l2, c: -(x1 * px + y1 * py) / l2 };
    }

    // Warmth as channel gains (-evaluate multiply): warm dims green a little and blue more, so
    // white paper turns cream while black ink stays black; cool is the mirror image.
    function warmthGains(w) {
        const k = Math.min(100, Math.abs(w)) / 100;
        const strong = +(1 - 0.25 * k).toFixed(4), mild = +(1 - 0.08 * k).toFixed(4);
        return w > 0 ? [1, mild, strong] : w < 0 ? [strong, mild, 1] : [1, 1, 1];
    }

    // -brightness-contrast BxC (BrightnessContrastImage): out = slope * in + intercept.
    function bcLine(b, c) {
        const slope = c < 0 ? 0.01 * c + 1 : 100 / (100 - c);
        return { slope, icpt: (0.01 * b - 0.5) * slope + 0.5 };
    }

    // The ink's box at full size, {x, y, w, h} or null: from the Kindle (X-Ink-Box) when it sent
    // one, else found here in a full-size ink layer (pixels darker than 250, as the Kindle does).
    function inkBox(L) {
        if (L.box !== undefined) return L.box;
        const W = L.w, H = L.h, ink = L.ink;
        let x0 = W, y0 = H, x1 = -1, y1 = -1;
        for (let y = 0; y < H; y++) {
            const row = y * W;
            let first = -1, last = -1;
            for (let x = 0; x < W; x++) {
                if (ink[row + x] < 250) {
                    if (first < 0) first = x;
                    last = x;
                }
            }
            if (first < 0) continue;
            if (first < x0) x0 = first;
            if (last > x1) x1 = last;
            if (y < y0) y0 = y;
            y1 = y;
        }
        return (L.box = x1 < 0 ? null : { x: x0, y: y0, w: x1 - x0 + 1, h: y1 - y0 + 1 });
    }

    // The ink gradient's own box: the writing's, or the whole page.
    function inkGradBox(p, L) {
        const b = p.inkSpan === 'writing' ? inkBox(L) : null;
        return b || { x: 0, y: 0, w: L.pw || L.w, h: L.ph || L.h };
    }

    // One pixel of the result for layers L (gray Uint8Arrays ink and paper, w x h; a shrunken
    // preview has f > 1 and the full page size in pw x ph): straight RGBA in 0..1 into o[0..3].
    function shader(p, L) {
        const f = L.f || 1, W = L.w, ink = L.ink, paper = p.lines === 'show' ? L.paper : null;
        const pw = L.pw || L.w, ph = L.ph || L.h, mid = (f - 1) / 2;
        // A preview pixel stands for f x f page pixels: gradients are sampled at their center.
        // Coefficients of t over this layer's own pixels, for a gradient laid on box b.
        const line = (angle, b) => {
            const g = gradLine(angle, b.w, b.h);
            return [g.a * f, g.b * f, g.c + (g.a + g.b) * mid - g.a * b.x - g.b * b.y];
        };
        const bgMode = p.bg, BG = rgb(p.bgColor), BF = rgb(p.bgFrom), BT = rgb(p.bgTo);
        const bl = line(p.bgAngle, { x: 0, y: 0, w: pw, h: ph });
        const LC = rgb(p.lineColor), lo = clamp01(p.lineOpacity / 100);
        const inkGrad = p.ink === 'gradient', KC = p.ink === 'color' ? rgb(p.inkColor) : [0, 0, 0];
        const KF = rgb(p.inkFrom), KT = rgb(p.inkTo), kb = inkGrad ? inkGradBox(p, L) : null;
        const kl = inkGrad ? line(p.inkAngle, kb) : null;
        // Outside its box the gradient image is padded with the first color (-extent).
        const kx0 = kb && (kb.x - mid) / f, kx1 = kb && (kb.x + kb.w - mid) / f;
        const ky0 = kb && (kb.y - mid) / f, ky1 = kb && (kb.y + kb.h - mid) / f;
        const tone = p.tone, DD = rgb(p.duoDark), DL = rgb(p.duoLight);
        const WG = warmthGains(p.warmth), warm = WG[0] !== 1 || WG[1] !== 1 || WG[2] !== 1;
        const ta = clamp01(p.tintAmount / 100), TC = rgb(p.tint);
        const bcOn = p.brightness !== 0 || p.contrast !== 0, BC = bcLine(p.brightness, p.contrast);
        return function (x, y, o) {
            const i = y * W + x;
            let r, g, b, a;
            if (bgMode === 'white') r = g = b = a = 1;
            else if (bgMode === 'none') r = g = b = a = 0;
            else if (bgMode === 'color') (r = BG[0]), (g = BG[1]), (b = BG[2]), (a = 1);
            else {
                const t = clamp01(bl[0] * x + bl[1] * y + bl[2]);
                r = BF[0] + (BT[0] - BF[0]) * t;
                g = BF[1] + (BT[1] - BF[1]) * t;
                b = BF[2] + (BT[2] - BF[2]) * t;
                a = 1;
            }
            // Each layer's darkness is its alpha (-negate -alpha shape), composited over.
            if (paper) {
                const sa = ((255 - paper[i]) / 255) * lo;
                if (sa > 0) {
                    const m = a * (1 - sa), na = sa + m;
                    r = (LC[0] * sa + r * m) / na;
                    g = (LC[1] * sa + g * m) / na;
                    b = (LC[2] * sa + b * m) / na;
                    a = na;
                }
            }
            const sa = (255 - ink[i]) / 255;
            if (sa > 0) {
                let cr = KC[0], cg = KC[1], cb = KC[2];
                if (inkGrad) {
                    const t = x < kx0 || x >= kx1 || y < ky0 || y >= ky1 ? 0 : clamp01(kl[0] * x + kl[1] * y + kl[2]);
                    cr = KF[0] + (KT[0] - KF[0]) * t;
                    cg = KF[1] + (KT[1] - KF[1]) * t;
                    cb = KF[2] + (KT[2] - KF[2]) * t;
                }
                const m = a * (1 - sa), na = sa + m;
                r = (cr * sa + r * m) / na;
                g = (cg * sa + g * m) / na;
                b = (cb * sa + b * m) / na;
                a = na;
            }
            if (a <= 0) {
                o[0] = o[1] = o[2] = o[3] = 0;
                return;
            }
            if (tone === 'invert') (r = 1 - r), (g = 1 - g), (b = 1 - b);
            else if (tone !== 'none') {
                const Y = 0.212656 * r + 0.715158 * g + 0.072186 * b;  // Rec709Luma
                if (tone === 'gray') r = g = b = Y;
                else (r = DD[0] + (DL[0] - DD[0]) * Y), (g = DD[1] + (DL[1] - DD[1]) * Y), (b = DD[2] + (DL[2] - DD[2]) * Y);
            }
            if (warm) (r *= WG[0]), (g *= WG[1]), (b *= WG[2]);
            if (ta > 0) (r = r * (1 - ta) + TC[0] * ta), (g = g * (1 - ta) + TC[1] * ta), (b = b * (1 - ta) + TC[2] * ta);
            if (bcOn) {
                r = clamp01(BC.slope * r + BC.icpt);
                g = clamp01(BC.slope * g + BC.icpt);
                b = clamp01(BC.slope * b + BC.icpt);
            }
            o[0] = r;
            o[1] = g;
            o[2] = b;
            o[3] = a;
        };
    }

    // The region to export, at full size, in whole steps of the size reduction (so -scale is an
    // exact block average): the page, or the ink's box grown by the margin.
    function exportBox(p, L, step) {
        const W = L.pw || L.w, H = L.ph || L.h, page = { x: 0, y: 0, w: W - (W % step), h: H - (H % step) };
        if (p.crop !== 'trim') return page;
        const b = inkBox(L);
        if (!b) return Object.assign(page, { empty: true });
        const m = p.margin;
        const span = (a, z, n) => {
            a = Math.max(0, a - m);
            z = Math.min(n, z + m);
            let short = (step - ((z - a) % step)) % step;
            const grow = Math.min(short, n - z);
            z += grow;
            short -= grow;
            a -= Math.min(short, a);
            if ((z - a) % step) z -= (z - a) % step;
            return [a, z];
        };
        const [ax, zx] = span(b.x, b.x + b.w, W), [ay, zy] = span(b.y, b.y + b.h, H);
        return { x: ax, y: ay, w: zx - ax, h: zy - ay };
    }

    // The picture: box of the layers (default: all of them), shrunk by step (1, 2 or 4) the way
    // -scale does (alpha-weighted block average). 8-bit straight RGBA plus what it contains.
    function render(p, L, box, step) {
        box = box || { x: 0, y: 0, w: L.w, h: L.h };
        step = step || 1;
        const px = shader(p, L), t = new Float64Array(4);
        const ow = Math.floor(box.w / step), oh = Math.floor(box.h / step), n = step * step;
        const out = new Uint8ClampedArray(ow * oh * 4);
        let alpha = false, gray = true;
        for (let y = 0; y < oh; y++) {
            for (let x = 0; x < ow; x++) {
                let r, g, b, a;
                if (step === 1) {
                    px(box.x + x, box.y + y, t);
                    (r = t[0]), (g = t[1]), (b = t[2]), (a = t[3]);
                } else {
                    let sr = 0, sg = 0, sb = 0, sa = 0;
                    for (let j = 0; j < step; j++) {
                        for (let i = 0; i < step; i++) {
                            px(box.x + x * step + i, box.y + y * step + j, t);
                            sr += t[0] * t[3];
                            sg += t[1] * t[3];
                            sb += t[2] * t[3];
                            sa += t[3];
                        }
                    }
                    a = sa / n;
                    if (sa > 0) (r = sr / sa), (g = sg / sa), (b = sb / sa);
                    else r = g = b = 0;
                }
                const k = (y * ow + x) * 4, A = Math.round(a * 255);
                if (A === 0) {
                    alpha = true;
                    continue;  // stays 0, 0, 0, 0
                }
                const R = Math.round(r * 255), G = Math.round(g * 255), B = Math.round(b * 255);
                out[k] = R;
                out[k + 1] = G;
                out[k + 2] = B;
                out[k + 3] = A;
                if (A !== 255) alpha = true;
                if (R !== G || G !== B) gray = false;
            }
        }
        return { w: ow, h: oh, rgba: out, alpha, gray };
    }

    // Shell word: bare when safe, else single-quoted.
    const sh = s => (/^[A-Za-z0-9._\/+:=@%^,-]+$/.test(s) ? s : "'" + String(s).replace(/'/g, "'\\''") + "'");

    // The magick command (ImageMagick 7, macOS/Linux shell) that makes the same picture from the
    // two layer files. ctx: {w, h} page size, box, step, inkGrad (the ink gradient's box), ink,
    // paper and out file names.
    function imCommand(p, ctx) {
        const size = ctx.w + 'x' + ctx.h, L = [];
        if (p.bg === 'gradient')
            L.push(`magick -size ${size} -define gradient:angle=${p.bgAngle} gradient:${sh(p.bgFrom + '-' + p.bgTo)}`);
        else L.push(`magick -size ${size} xc:${p.bg === 'color' ? sh(p.bgColor) : p.bg === 'none' ? 'none' : 'white'}`);
        if (p.lines === 'show') {
            const fade = p.lineOpacity < 100 ? ` -channel A -evaluate multiply ${num(p.lineOpacity / 100)} +channel` : '';
            L.push(`\\( ${sh(ctx.paper)} -negate -background ${sh(p.lineColor)} -alpha shape${fade} \\) -compose over -composite`);
        }
        const kb = ctx.inkGrad;
        if (p.ink === 'gradient' && kb && (kb.x || kb.y || kb.w !== ctx.w || kb.h !== ctx.h))
            L.push(`\\( -size ${kb.w}x${kb.h} -define gradient:angle=${p.inkAngle} gradient:${sh(p.inkFrom + '-' + p.inkTo)}` +
                   ` -background ${sh(p.inkFrom)} -compose over -extent ${size}-${kb.x}-${kb.y}` +
                   ` \\( ${sh(ctx.ink)} -negate \\) -alpha off -compose copy-opacity -composite \\) -compose over -composite`);
        else if (p.ink === 'gradient')
            L.push(`\\( -size ${size} -define gradient:angle=${p.inkAngle} gradient:${sh(p.inkFrom + '-' + p.inkTo)}` +
                   ` \\( ${sh(ctx.ink)} -negate \\) -alpha off -compose copy-opacity -composite \\) -compose over -composite`);
        else
            L.push(`\\( ${sh(ctx.ink)} -negate -background ${sh(p.ink === 'color' ? p.inkColor : '#000000')}` +
                   ` -alpha shape \\) -compose over -composite`);
        if (p.tone === 'gray') L.push('-grayscale Rec709Luma -colorspace sRGB');
        else if (p.tone === 'duotone')
            L.push(`-grayscale Rec709Luma -channel RGB +level-colors ${sh(p.duoDark + ',' + p.duoLight)} +channel`);
        else if (p.tone === 'invert') L.push('-channel RGB -negate +channel');
        if (p.warmth) {
            const g = warmthGains(p.warmth), parts = [];
            ['R', 'G', 'B'].forEach((c, i) => g[i] !== 1 && parts.push(`-channel ${c} -evaluate multiply ${num(g[i])}`));
            L.push(parts.join(' ') + ' +channel');
        }
        if (p.tintAmount > 0) L.push(`-fill ${sh(p.tint)} -colorize ${p.tintAmount}%`);
        if (p.brightness || p.contrast) L.push(`-channel RGB -brightness-contrast ${p.brightness}x${p.contrast} +channel -clamp`);
        const b = ctx.box;
        if (b && (b.x || b.y || b.w !== ctx.w || b.h !== ctx.h)) L.push(`-crop ${b.w}x${b.h}+${b.x}+${b.y} +repage`);
        if (ctx.step > 1) L.push(`-scale ${100 / ctx.step}%`);
        L.push(`-depth 8 ${sh(ctx.out)}`);
        return L.join(' \\\n  ');
    }

    // ---------- files: PNG in and out, zip, PDF ----------

    const CRC = new Uint32Array(256);
    for (let i = 0; i < 256; i++) {
        let c = i;
        for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
        CRC[i] = c >>> 0;
    }
    function crc32(bytes, crc) {
        let c = crc === undefined ? 0xffffffff : crc;
        for (let i = 0; i < bytes.length; i++) c = CRC[(c ^ bytes[i]) & 255] ^ (c >>> 8);
        return c;
    }
    const crcDone = c => (c ^ 0xffffffff) >>> 0;

    function concat(parts) {
        let n = 0;
        for (const q of parts) n += q.length;
        const out = new Uint8Array(n);
        let o = 0;
        for (const q of parts) out.set(q, o), (o += q.length);
        return out;
    }

    async function pipe(bytes, stream) {
        const s = new Blob([bytes]).stream().pipeThrough(stream);
        return new Uint8Array(await new Response(s).arrayBuffer());
    }
    // zlib (RFC 1950), as PNG and PDF want it. Without CompressionStream (browsers before 2023)
    // the data goes in stored blocks: valid, just not smaller.
    async function deflate(bytes) {
        if (typeof CompressionStream === 'function') return pipe(bytes, new CompressionStream('deflate'));
        const parts = [new Uint8Array([0x78, 0x01])];
        for (let o = 0; o < bytes.length || o === 0; o += 65535) {
            const n = Math.min(65535, bytes.length - o), last = o + n >= bytes.length;
            parts.push(new Uint8Array([last ? 1 : 0, n & 255, n >> 8, ~n & 255, (~n >> 8) & 255]), bytes.subarray(o, o + n));
            if (last) break;
        }
        let s1 = 1, s2 = 0;
        for (let i = 0; i < bytes.length; i++) (s1 = (s1 + bytes[i]) % 65521), (s2 = (s2 + s1) % 65521);
        const ad = ((s2 << 16) | s1) >>> 0;
        parts.push(new Uint8Array([ad >>> 24, (ad >> 16) & 255, (ad >> 8) & 255, ad & 255]));
        return concat(parts);
    }

    // An 8-bit, non-interlaced PNG (what the Kindle sends) to its first channel: the layers are
    // gray. Falls back to the browser's decoder where DecompressionStream is missing.
    async function decodeGray(bytes) {
        if (typeof DecompressionStream !== 'function') return decodeGrayCanvas(bytes);
        const dv = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
        if (bytes.length < 33 || dv.getUint32(0) !== 0x89504e47) throw new Error('not a PNG');
        let pos = 8, w = 0, h = 0, depth = 0, type = 0, lace = 0;
        const idat = [];
        while (pos + 8 <= bytes.length) {
            const len = dv.getUint32(pos), tag = String.fromCharCode(...bytes.subarray(pos + 4, pos + 8));
            if (tag === 'IHDR') {
                (w = dv.getUint32(pos + 8)), (h = dv.getUint32(pos + 12));
                (depth = bytes[pos + 16]), (type = bytes[pos + 17]), (lace = bytes[pos + 20]);
            } else if (tag === 'IDAT') idat.push(bytes.subarray(pos + 8, pos + 8 + len));
            else if (tag === 'IEND') break;
            pos += 12 + len;
        }
        const ch = { 0: 1, 2: 3, 4: 2, 6: 4 }[type];
        if (depth !== 8 || lace || !ch || !w || !h) return decodeGrayCanvas(bytes);
        const raw = await pipe(concat(idat), new DecompressionStream('deflate'));
        const stride = w * ch;
        if (raw.length < (stride + 1) * h) throw new Error('short PNG');
        const out = new Uint8Array(w * h);
        let prev = new Uint8Array(stride);
        for (let y = 0; y < h; y++) {
            const ft = raw[y * (stride + 1)], row = raw.subarray(y * (stride + 1) + 1, (y + 1) * (stride + 1));
            for (let i = 0; i < stride; i++) {
                const left = i >= ch ? row[i - ch] : 0, up = prev[i], ul = i >= ch ? prev[i - ch] : 0;
                let v = row[i];
                if (ft === 1) v += left;
                else if (ft === 2) v += up;
                else if (ft === 3) v += (left + up) >> 1;
                else if (ft === 4) {
                    const q = left + up - ul, pa = Math.abs(q - left), pb = Math.abs(q - up), pc = Math.abs(q - ul);
                    v += pa <= pb && pa <= pc ? left : pb <= pc ? up : ul;
                }
                row[i] = v & 255;
            }
            for (let x = 0; x < w; x++) out[y * w + x] = row[x * ch];
            prev = row;
        }
        return { w, h, data: out };
    }

    async function decodeGrayCanvas(bytes) {
        const bmp = await createImageBitmap(new Blob([bytes], { type: 'image/png' }));
        const c = document.createElement('canvas');
        (c.width = bmp.width), (c.height = bmp.height);
        const g = c.getContext('2d');
        g.drawImage(bmp, 0, 0);
        const d = g.getImageData(0, 0, c.width, c.height).data, out = new Uint8Array(c.width * c.height);
        for (let i = 0; i < out.length; i++) out[i] = d[i * 4];
        return { w: c.width, h: c.height, data: out };
    }

    // render() output as the smallest fitting PNG: gray or RGB, alpha only if used, an adaptive
    // filter per row, and the print size (dpi) in pHYs.
    async function encodePNG(img, dpi) {
        const ch = img.gray ? (img.alpha ? 2 : 1) : img.alpha ? 4 : 3;
        const type = img.gray ? (img.alpha ? 4 : 0) : img.alpha ? 6 : 2;
        const W = img.w, H = img.h, S = W * ch, src = img.rgba;
        const raw = new Uint8Array((S + 1) * H);
        let line = new Uint8Array(S), prev = new Uint8Array(S);
        const cand = [0, 1, 2, 3, 4].map(() => new Uint8Array(S));
        for (let y = 0; y < H; y++) {
            for (let x = 0, k = y * W * 4; x < W; x++, k += 4) {
                const o = x * ch;
                if (ch === 1) line[o] = src[k];
                else if (ch === 2) (line[o] = src[k]), (line[o + 1] = src[k + 3]);
                else {
                    (line[o] = src[k]), (line[o + 1] = src[k + 1]), (line[o + 2] = src[k + 2]);
                    if (ch === 4) line[o + 3] = src[k + 3];
                }
            }
            let best = 0, bestCost = Infinity;
            for (let f = 0; f < 5; f++) {
                const c = cand[f];
                let cost = 0;
                for (let i = 0; i < S; i++) {
                    const left = i >= ch ? line[i - ch] : 0, up = prev[i], ul = i >= ch ? prev[i - ch] : 0;
                    let pred = 0;
                    if (f === 1) pred = left;
                    else if (f === 2) pred = up;
                    else if (f === 3) pred = (left + up) >> 1;
                    else if (f === 4) {
                        const q = left + up - ul, pa = Math.abs(q - left), pb = Math.abs(q - up), pc = Math.abs(q - ul);
                        pred = pa <= pb && pa <= pc ? left : pb <= pc ? up : ul;
                    }
                    const v = (line[i] - pred) & 255;
                    c[i] = v;
                    cost += v < 128 ? v : 256 - v;
                }
                if (cost < bestCost) (bestCost = cost), (best = f);
            }
            raw[y * (S + 1)] = best;
            raw.set(cand[best], y * (S + 1) + 1);
            [prev, line] = [line, prev];
        }
        const chunk = (tag, data) => {
            const head = new Uint8Array(8), dv = new DataView(head.buffer);
            dv.setUint32(0, data.length);
            for (let i = 0; i < 4; i++) head[4 + i] = tag.charCodeAt(i);
            const tail = new Uint8Array(4);
            new DataView(tail.buffer).setUint32(0, crcDone(crc32(data, crc32(head.subarray(4)))));
            return [head, data, tail];
        };
        const ihdr = new Uint8Array(13), iv = new DataView(ihdr.buffer);
        iv.setUint32(0, W), iv.setUint32(4, H), (ihdr[8] = 8), (ihdr[9] = type);
        const phys = new Uint8Array(9), pv = new DataView(phys.buffer), ppm = Math.round((dpi || 300) / 0.0254);
        pv.setUint32(0, ppm), pv.setUint32(4, ppm), (phys[8] = 1);
        return concat([new Uint8Array([137, 80, 78, 71, 13, 10, 26, 10]), ...chunk('IHDR', ihdr), ...chunk('pHYs', phys),
                       ...chunk('IDAT', await deflate(raw)), ...chunk('IEND', new Uint8Array(0))]);
    }

    // A zip of stored files (PNGs are compressed already), UTF-8 names.
    function zipFiles(files) {
        const enc = new TextEncoder(), now = new Date(), parts = [], central = [];
        const time = (now.getHours() << 11) | (now.getMinutes() << 5) | (now.getSeconds() >> 1);
        const date = ((now.getFullYear() - 1980) << 9) | ((now.getMonth() + 1) << 5) | now.getDate();
        let off = 0;
        for (const f of files) {
            const name = enc.encode(f.name), crc = crcDone(crc32(f.data)), n = f.data.length;
            const lh = new DataView(new ArrayBuffer(30));
            lh.setUint32(0, 0x04034b50, true), lh.setUint16(4, 20, true), lh.setUint16(6, 0x0800, true);
            lh.setUint16(10, time, true), lh.setUint16(12, date, true), lh.setUint32(14, crc, true);
            lh.setUint32(18, n, true), lh.setUint32(22, n, true), lh.setUint16(26, name.length, true);
            parts.push(new Uint8Array(lh.buffer), name, f.data);
            const ch = new DataView(new ArrayBuffer(46));
            ch.setUint32(0, 0x02014b50, true), ch.setUint16(4, 0x0314, true), ch.setUint16(6, 20, true);
            ch.setUint16(8, 0x0800, true), ch.setUint16(12, time, true), ch.setUint16(14, date, true);
            ch.setUint32(16, crc, true), ch.setUint32(20, n, true), ch.setUint32(24, n, true);
            ch.setUint16(28, name.length, true), ch.setUint32(38, 0o100644 << 16, true), ch.setUint32(42, off, true);
            central.push(new Uint8Array(ch.buffer), name);
            off += 30 + name.length + n;
        }
        let size = 0;
        for (const c of central) size += c.length;
        const end = new DataView(new ArrayBuffer(22));
        end.setUint32(0, 0x06054b50, true), end.setUint16(8, files.length, true), end.setUint16(10, files.length, true);
        end.setUint32(12, size, true), end.setUint32(16, off, true);
        return new Blob([...parts, ...central, new Uint8Array(end.buffer)], { type: 'application/zip' });
    }

    // One PDF page's image, compressed: gray or RGB, plus a soft mask when it has transparency.
    async function pdfImage(img, dpi) {
        const n = img.w * img.h, ch = img.gray ? 1 : 3, pix = new Uint8Array(n * ch);
        const mask = img.alpha ? new Uint8Array(n) : null, s = img.rgba;
        for (let i = 0; i < n; i++) {
            if (ch === 1) pix[i] = s[i * 4];
            else (pix[i * 3] = s[i * 4]), (pix[i * 3 + 1] = s[i * 4 + 1]), (pix[i * 3 + 2] = s[i * 4 + 2]);
            if (mask) mask[i] = s[i * 4 + 3];
        }
        return { w: img.w, h: img.h, gray: img.gray, dpi, pix: await deflate(pix), mask: mask && (await deflate(mask)) };
    }

    function pdfText(s) {  // UTF-16BE hex string with BOM
        let out = '<FEFF';
        for (const ch of s) {
            const c = ch.codePointAt(0);
            if (c > 0xffff) {
                const v = c - 0x10000;
                out += (0xd800 + (v >> 10)).toString(16).padStart(4, '0') + (0xdc00 + (v & 1023)).toString(16).padStart(4, '0');
            } else out += c.toString(16).padStart(4, '0');
        }
        return out.toUpperCase() + '>';
    }

    // pdfImage() pages into a PDF, each page the image's print size.
    function pdfFile(images, title, version) {
        const enc = new TextEncoder(), parts = [], xref = [];
        let off = 0;
        const put = x => {
            const b = typeof x === 'string' ? enc.encode(x) : x;
            parts.push(b);
            off += b.length;
        };
        const obj = (num, dict, stream) => {
            xref[num] = off;
            put(`${num} 0 obj\n${dict}\n`);
            if (stream) put('stream\n'), put(stream), put('\nendstream\n');
            put('endobj\n');
        };
        put(new Uint8Array([0x25, 0x50, 0x44, 0x46, 0x2d, 0x31, 0x2e, 0x34, 0x0a, 0x25, 0xe2, 0xe3, 0xcf, 0xd3, 0x0a]));
        const ids = [];
        let next = 4;
        for (const im of images) {
            ids.push({ page: next, content: next + 1, image: next + 2, mask: im.mask ? next + 3 : 0 });
            next += im.mask ? 4 : 3;
        }
        obj(1, '<< /Type /Catalog /Pages 2 0 R >>');
        obj(2, `<< /Type /Pages /Count ${images.length} /Kids [${ids.map(i => `${i.page} 0 R`).join(' ')}] >>`);
        obj(3, `<< /Title ${pdfText(title)} /Creator (Grimoire ${version} studio) /Producer (Grimoire) >>`);
        images.forEach((im, k) => {
            const id = ids[k], w = ((im.w * 72) / im.dpi).toFixed(2), h = ((im.h * 72) / im.dpi).toFixed(2);
            obj(id.page, `<< /Type /Page /Parent 2 0 R /MediaBox [0 0 ${w} ${h}] /Contents ${id.content} 0 R ` +
                         `/Resources << /XObject << /Im0 ${id.image} 0 R >> >> >>`);
            const content = enc.encode(`q ${w} 0 0 ${h} 0 0 cm /Im0 Do Q`);
            obj(id.content, `<< /Length ${content.length} >>`, content);
            obj(id.image, `<< /Type /XObject /Subtype /Image /Width ${im.w} /Height ${im.h} /ColorSpace ` +
                          `/Device${im.gray ? 'Gray' : 'RGB'} /BitsPerComponent 8 /Filter /FlateDecode` +
                          `${id.mask ? ` /SMask ${id.mask} 0 R` : ''} /Length ${im.pix.length} >>`, im.pix);
            if (id.mask)
                obj(id.mask, `<< /Type /XObject /Subtype /Image /Width ${im.w} /Height ${im.h} /ColorSpace /DeviceGray ` +
                             `/BitsPerComponent 8 /Filter /FlateDecode /Length ${im.mask.length} >>`, im.mask);
        });
        const start = off;
        let table = `xref\n0 ${next}\n0000000000 65535 f \n`;
        for (let i = 1; i < next; i++) table += String(xref[i]).padStart(10, '0') + ' 00000 n \n';
        put(table + `trailer\n<< /Size ${next} /Root 1 0 R /Info 3 0 R >>\nstartxref\n${start}\n%%EOF\n`);
        return new Blob(parts, { type: 'application/pdf' });
    }

    // Everything imCommand needs besides the settings, for layers L.
    const commandContext = (p, L, box, step, names) => Object.assign({ w: L.pw || L.w, h: L.ph || L.h, box, step,
        inkGrad: p.ink === 'gradient' ? inkGradBox(p, L) : null }, names);

    const core = { DEFAULTS, PRESETS, normalize, render, inkBox, exportBox, imCommand, commandContext, decodeGray,
                   encodePNG, zipFiles, pdfImage, pdfFile, gradLine, warmthGains, bcLine };
    if (typeof module !== 'undefined' && module.exports) module.exports = core;
    if (typeof document === 'undefined' || !window.GRIMOIRE_NB) return;

    // ---------- the page ----------

    const NB = window.GRIMOIRE_NB, NP = Math.max(1, NB.pages | 0), DPI = +NB.dpi || 300;
    const ID = encodeURIComponent(NB.id);
    // The page's words, in the Kindle's language: httpd.c sends the translations, by their English
    // (window.GRIMOIRE_L10N; none in English), and a word without one stays English. Only what is
    // shown goes through t: keys, file names and the magick command never do. A value goes in at
    // %d or %s (%1$d, %2$s... where a language puts them in another order), %% is a percent sign.
    const L10N = window.GRIMOIRE_L10N || {};
    const has = k => Object.prototype.hasOwnProperty.call(L10N, k) && typeof L10N[k] === 'string';
    const fill = (s, args) => {
        let next = 0;
        return s.replace(/%(?:(\d+)\$)?([ds%])/g, (m, pos, kind) => (kind === '%' ? '%' : String(args[pos ? pos - 1 : next++])));
    };
    const t = (s, ...args) => fill(has(s) ? L10N[s] : s, args);
    // A count of this notebook's pages in words: the Kindle sends a plural in its form for that
    // count, the only one the studio words.
    const tPages = (one, many) => fill(has(one) ? L10N[one] : NP === 1 ? one : many, [NP]);
    const root = document.getElementById('studio');
    const store = {
        get() {
            try {
                return JSON.parse(localStorage.getItem('grimoire-studio') || 'null');
            } catch (e) {
                return null;
            }
        },
        set(p) {
            try {
                localStorage.setItem('grimoire-studio', JSON.stringify(p));
            } catch (e) { /* private window: settings just are not remembered */ }
        },
    };
    let P = normalize(store.get());
    let pageNo = Math.min(NP, Math.max(1, parseInt(new URLSearchParams(location.search).get('p'), 10) || 1));
    let busy = false;

    // An element's text. Gothic's bold labels (studio.css) are a shadow one design pixel to the
    // right, which fills the gaps inside accents, so letters past ASCII go in spans without it
    // (class na, as on the share pages: httpd.c); signs keep it.
    const ACCENTED = /[^\P{L}\x00-\x7f]+/gu;
    function setText(e, text) {
        e.textContent = '';
        const s = String(text);
        let at = 0;
        for (const m of s.matchAll(ACCENTED)) {
            if (m.index > at) e.append(s.slice(at, m.index));
            const span = document.createElement('span');
            span.className = 'na';
            span.textContent = m[0];
            e.append(span);
            at = m.index + m[0].length;
        }
        if (at < s.length) e.append(s.slice(at));
    }
    const el = (tag, attrs, ...kids) => {
        const e = document.createElement(tag);
        for (const [k, v] of Object.entries(attrs || {})) {
            if (k === 'on') for (const [ev, fn] of Object.entries(v)) e.addEventListener(ev, fn);
            else if (k === 'text') setText(e, v);
            else e.setAttribute(k, v);
        }
        for (const kid of kids) if (kid) e.append(kid);
        return e;
    };
    const safeName = s => (s || 'Untitled').replace(/[\u0000-\u001f\/\\:*?"<>|]/g, '_').replace(/^[ .]+|[ .]+$/g, '') || 'Untitled';
    const base = safeName(NB.name), pad3 = n => String(n).padStart(3, '0');
    const aborted = () => new DOMException('The page was left', 'AbortError');
    const sleep = (ms, signal) => new Promise((ok, no) => {
        const t = setTimeout(ok, ms);
        if (signal) signal.addEventListener('abort', () => (clearTimeout(t), no(aborted())), { once: true });
    });

    // ---- layers from the Kindle ----
    // A page is two layers, its ink (?layer=ink) and its paper (?layer=paper). Previews take them
    // shrunk 2x (?f=2: a quarter of the Kindle's encoding and of the Wi-Fi); exports and small
    // crops take them full size. The ink's headers give the page size, the paper kind (one paper
    // layer serves every page with that paper) and the ink's box. The Kindle renders two at a
    // time and queues the rest; requests for a page you flipped past are aborted, so it drops them.

    // One layer, decoded; retried through a dropped connection or a broken transfer.
    async function fetchLayer(url, signal) {
        let last = null;
        for (let attempt = 0; attempt < 4; attempt++) {
            if (attempt) await sleep(500 * attempt, signal);
            const ctrl = new AbortController(), stop = () => ctrl.abort();
            signal.addEventListener('abort', stop);
            const timer = setTimeout(stop, 120000);  // the Kindle's queue never takes that long
            try {
                const r = await fetch(url, { cache: 'no-store', signal: ctrl.signal });
                if (r.status >= 400 && r.status < 500) {
                    // TRANSLATORS: the studio could not get a page from the Kindle: its answer (an
                    // HTTP status, 404) for that address.
                    throw Object.assign(new Error(t('The Kindle answered %d for %s', r.status, url)), { final: true });
                }
                if (!r.ok) throw new Error(`HTTP ${r.status}`);
                const img = await decodeGray(new Uint8Array(await r.arrayBuffer()));
                const size = /^(\d+)x(\d+)$/.exec(r.headers.get('X-Page-Size') || '');
                const ib = r.headers.get('X-Ink-Box'), box = /^(\d+),(\d+),(\d+),(\d+)$/.exec(ib || '');
                return Object.assign(img, {
                    pw: size ? +size[1] : img.w, ph: size ? +size[2] : img.h, paper: r.headers.get('X-Paper') || '?',
                    box: box ? { x: +box[1], y: +box[2], w: +box[3], h: +box[4] } : ib === 'none' ? null : undefined,
                });
            } catch (e) {
                if (signal.aborted) throw aborted();
                if (e.final) throw e;
                last = e;
            } finally {
                clearTimeout(timer);
                signal.removeEventListener('abort', stop);
            }
        }
        // TRANSLATORS: %s is what went wrong, in the browser's words.
        throw new Error(t('The Kindle stopped answering (%s). Is the Share screen still open?', last && last.message));
    }

    // Decoded layers, the most recently used kept up to about 80 MB.
    const cache = new Map();
    let cachedBytes = 0;
    function remember(key, v) {
        cache.set(key, v);
        cachedBytes += v.data.length;
        for (const [k, old] of cache) {
            if (cachedBytes <= 80e6 || k === key) break;
            cache.delete(k);
            cachedBytes -= old.data.length;
        }
    }
    // Layers on their way, one request each, tagged with the page that wants them; keep = an export.
    const flying = new Map();
    function layer(key, url, page, keep) {
        const hit = cache.get(key);
        if (hit) {
            cache.delete(key);
            cache.set(key, hit);  // most recent last
            return Promise.resolve(hit);
        }
        let job = flying.get(key);
        if (job) {
            job.keep = job.keep || keep;
            return job.promise;
        }
        job = { ctrl: new AbortController(), page, keep };
        job.promise = fetchLayer(url, job.ctrl.signal)
            .then(v => (remember(key, v), v))
            .finally(() => flying.get(key) === job && flying.delete(key));
        flying.set(key, job);
        return job.promise;
    }
    function dropAllBut(page) {
        for (const [key, job] of flying)
            if (!job.keep && job.page !== page) job.ctrl.abort(), flying.delete(key);
    }
    // Page n's layers shrunk f times (1 or 2).
    async function pageLayers(n, f, keep) {
        const ink = await layer(`ink ${n} ${f}`, `/nb/${ID}/${n}.png?layer=ink&f=${f}`, n, keep);
        const paper = await layer(`paper ${ink.paper} ${ink.pw}x${ink.ph} ${f}`, `/nb/${ID}/${n}.png?layer=paper&f=${f}`,
                                  n, keep);
        return { w: ink.w, h: ink.h, f, pw: ink.pw, ph: ink.ph, ink: ink.data, box: ink.box,
                 paper: paper.w === ink.w && paper.h === ink.h ? paper.data : null };
    }

    // ---- controls ----
    const seg = (key, label, opts, when) => ({ kind: 'seg', key, label, opts, when });
    const color = (key, label, when) => ({ kind: 'color', key, label, when });
    const range = (key, label, unit, when) => ({ kind: 'range', key, label, unit, when });
    // TRANSLATORS: the Warmth slider's value: cooler (blue) or warmer (orange), by how much.
    const shown = (key, v, unit) => (key === 'warmth' ? (v < 0 ? t('cool %d', -v) : v > 0 ? t('warm %d', v) : '0') : v + unit);
    const inkGrad = p => p.ink === 'gradient';
    // The panel: a fieldset per section, a row per control; the first value of each choice is its
    // key (settings, the command), the second its label.
    const PANEL = [
        // TRANSLATORS: the studio's controls: a heading per section, then the labels of its rows and
        // of the choices on its buttons. Ink is the writing; the paper lines are the page's ruling.
        [t('Background'), [seg('bg', '', [['white', t('White')], ['none', t('Transparent')], ['color', t('Color')],
            ['gradient', t('Gradient')]]),
            color('bgColor', t('Color'), p => p.bg === 'color'), color('bgFrom', t('From'), p => p.bg === 'gradient'),
            color('bgTo', t('To'), p => p.bg === 'gradient'), range('bgAngle', t('Angle'), '°', p => p.bg === 'gradient')]],
        [t('Paper lines'), [seg('lines', '', [['show', t('Show')], ['hide', t('Hide')]]),
            color('lineColor', t('Color'), p => p.lines === 'show'),
            range('lineOpacity', t('Strength'), '%', p => p.lines === 'show')]],
        [t('Ink (your writing)'), [seg('ink', '', [['black', t('As written')], ['color', t('Color')], ['gradient', t('Gradient')]]),
            color('inkColor', t('Color'), p => p.ink === 'color'), color('inkFrom', t('From'), inkGrad),
            color('inkTo', t('To'), inkGrad), range('inkAngle', t('Angle'), '°', inkGrad),
            seg('inkSpan', t('Across'), [['writing', t('The writing')], ['page', t('The page')]], inkGrad)]],
        [t('Look'), [seg('tone', t('Tone'), [['none', t('None')], ['gray', t('Gray')], ['duotone', t('Duotone')],
            ['invert', t('Invert')]]),
            color('duoDark', t('Shadows'), p => p.tone === 'duotone'), color('duoLight', t('Highlights'), p => p.tone === 'duotone'),
            range('warmth', t('Warmth'), ''), color('tint', t('Tint')), range('tintAmount', t('Tint amount'), '%'),
            range('brightness', t('Brightness'), ''), range('contrast', t('Contrast'), '')]],
        [t('Crop and size'), [seg('crop', t('Crop'), [['page', t('Whole page')], ['trim', t('Trim to ink')]]),
            range('margin', t('Margin'), ' px', p => p.crop === 'trim'),
            seg('scale', t('Size'), [[100, '100%'], [50, '50%'], [25, '25%']])]],
    ];

    const panel = el('div', { class: 'st-panel' });
    // TRANSLATORS: names for a screen reader: the picture, the page number's box, the arrows.
    const canvas = el('canvas', { width: 1, height: 1, 'aria-label': t('Preview') });
    const veil = el('div', { class: 'st-veil' });
    const stage = el('div', { class: 'st-stage st-loading' }, canvas, veil);
    const pageInput = el('input', { type: 'number', class: 'st-pnum', min: 1, max: NP, value: pageNo, 'aria-label': t('Page'),
        on: { change: e => go(+e.target.value) } });
    const info = el('span', { class: 'st-info meta' });
    const msg = el('p', { class: 'st-msg meta', role: 'status' });
    const nav = d => el('button', { class: 'st-btn', type: 'button', 'aria-label': d < 0 ? t('Previous page') : t('Next page'),
        text: d < 0 ? '‹' : '›', on: { click: () => go(pageNo + d) } });
    // TRANSLATORS: over the studio's picture: %s is the box with the page's number, %d the pages.
    const [pageBefore, pageAfter] = t('Page %s of %d', '\u0000', NP).split('\u0000');
    const view = el('div', { class: 'st-view' },
        el('div', { class: 'st-nav' }, nav(-1), el('span', { class: 'st-page' }, pageBefore, pageInput, pageAfter), nav(1), info),
        stage, msg);

    const sync = [];  // functions that bring a control up to date with P
    const presets = el('div', { class: 'st-presets' });
    // A preset is a whole look; it keeps your crop and size (Original resets those too).
    for (const [name, over] of PRESETS)
        presets.append(el('button', { class: 'st-btn', type: 'button', text: t(name), on: { click: () => {
            const keep = name === 'Original' ? {} : { crop: P.crop, margin: P.margin, scale: P.scale };
            P = normalize(Object.assign({}, DEFAULTS, keep, over));
            changed();
        } } }));
    panel.append(el('fieldset', {}, el('legend', { text: t('Presets') }), presets));
    for (const [title, rows] of PANEL) {
        const fs = el('fieldset', {}, el('legend', { text: title }));
        for (const c of rows) {
            const row = el('div', { class: 'st-row' });
            const id = 'st-' + c.key;
            if (c.label) row.append(el('label', { for: id, text: c.label }));
            if (c.kind === 'seg') {
                const box = el('div', { class: 'st-seg', role: 'group', id, 'aria-label': c.label || title });
                const btns = c.opts.map(([v, text]) => {
                    const b = el('button', { type: 'button', text, on: { click: () => ((P[c.key] = v), changed()) } });
                    box.append(b);
                    return [v, b];
                });
                row.append(box);
                sync.push(() => btns.forEach(([v, b]) => b.setAttribute('aria-pressed', String(P[c.key] === v))));
            } else if (c.kind === 'color') {
                const inp = el('input', { type: 'color', id, on: { input: e => ((P[c.key] = e.target.value), changed()) } });
                const hex = el('span', { class: 'st-val meta' });
                row.append(inp, hex);
                sync.push(() => ((inp.value = P[c.key]), (hex.textContent = P[c.key])));
            } else {
                const [lo, hi] = RANGES[c.key];
                const inp = el('input', { type: 'range', id, min: lo, max: hi, step: 1,
                    on: { input: e => ((P[c.key] = +e.target.value), changed()) } });
                const val = el('span', { class: 'st-val' });
                row.append(inp, val);
                sync.push(() => ((inp.value = P[c.key]), (val.textContent = shown(c.key, P[c.key], c.unit))));
            }
            if (c.when) sync.push(() => row.classList.toggle('st-hide', !c.when(P)));
            fs.append(row);
        }
        panel.append(fs);
    }

    const progress = el('p', { class: 'st-msg meta', role: 'status' });
    const exportBtn = (text, kind, primary) => el('button', { class: 'st-btn' + (primary ? ' primary' : ''), type: 'button',
        text, on: { click: () => exportAs(kind) } });
    // TRANSLATORS: the studio's downloads: this page as a PNG, every page as PNGs in a zip, or as
    // one PDF (%d is how many pages the notebook has).
    panel.append(el('fieldset', {}, el('legend', { text: t('Export') }),
        el('div', { class: 'st-presets' }, exportBtn(t('PNG, this page'), 'png', true),
           exportBtn(tPages('PNGs, all %d page (zip)', 'PNGs, all %d pages (zip)'), 'zip'),
           exportBtn(tPages('PDF, all %d page', 'PDF, all %d pages'), 'pdf')), progress));

    const cmd = el('pre', { class: 'st-cmd' });
    // TRANSLATORS: copies the ImageMagick command below the picture's settings.
    const copyBtn = el('button', { class: 'st-btn', type: 'button', text: t('Copy command'), on: { click: copyCommand } });
    const inkLink = el('a', {}), paperLink = el('a', {});
    panel.append(el('fieldset', {}, el('legend', { text: 'ImageMagick' }),
        // TRANSLATORS: over the command that makes the same picture with ImageMagick, and the links
        // to the page's two layers (its ink, its paper) it is made from.
        el('p', { class: 'meta', text: t('The same picture from the page’s two layers, with ImageMagick 7 in a ' +
                  'macOS or Linux terminal. Download the layers into one folder and run it there:') }),
        el('p', {}, inkLink, ' · ', paperLink), cmd, copyBtn));

    root.textContent = '';
    root.append(el('div', { class: 'st' }, view, panel));

    // ---- preview ----
    let current = null;  // the page on screen: {n, L: {1, 2} its layers by shrink, asked: {1, 2}}
    let queued = false, navTimer = 0, nextTimer = 0;
    function changed() {
        store.set(P);
        for (const s of sync) s();
        if (!queued) (queued = true), requestAnimationFrame(draw);
    }
    function names(n) {
        const stem = `${base}-page-${pad3(n)}`;
        return { ink: `${stem}-ink.png`, paper: `${stem}-paper.png`, out: `${stem}.png` };
    }
    function say(text, retry) {
        msg.textContent = text;
        if (retry) msg.append(' ', el('button', { class: 'st-btn', type: 'button', text: t('Try again'), on: { click: retry } }));
    }
    function draw() {
        queued = false;
        const n = pageNo, nm = names(n);
        inkLink.href = `/nb/${ID}/${n}.png?layer=ink`;
        inkLink.download = inkLink.textContent = nm.ink;
        paperLink.href = `/nb/${ID}/${n}.png?layer=paper`;
        paperLink.download = paperLink.textContent = nm.paper;
        if (!current || current.n !== n) return;
        const step = 100 / P.scale, box = exportBox(P, current.L[2] || current.L[1], step);
        // Small crops are previewed at full size (fetched when first needed), the rest shrunk 2x.
        const want = Math.max(box.w, box.h) <= 1300 ? 1 : 2;
        if (!current.L[want] && !current.asked[want]) {
            current.asked[want] = true;
            const here = current;
            pageLayers(n, want).then(L => ((here.L[want] = L), current === here && changed())).catch(() => {});
        }
        const L = current.L[want] || current.L[3 - want], f = L.f;
        cmd.textContent = imCommand(P, commandContext(P, L, box, step, nm));
        const sb = { x: Math.floor(box.x / f), y: Math.floor(box.y / f), w: Math.max(1, Math.floor(box.w / f)),
                     h: Math.max(1, Math.floor(box.h / f)) };
        const img = render(P, L, sb, 1);
        (canvas.width = img.w), (canvas.height = img.h);
        canvas.getContext('2d').putImageData(new ImageData(img.rgba, img.w, img.h), 0, 0);
        stage.classList.toggle('alpha', img.alpha);
        // TRANSLATORS: the picture's size in pixels, whether it has transparent parts, and whether
        // Trim to ink found no ink (then the whole page is kept).
        const pw = box.w / step, ph = box.h / step;
        info.textContent = img.alpha && box.empty ? t('%d × %d px, transparent, no ink to trim to', pw, ph)
                           : img.alpha ? t('%d × %d px, transparent', pw, ph)
                           : box.empty ? t('%d × %d px, no ink to trim to', pw, ph)
                           : t('%d × %d px', pw, ph);
    }
    // Flipping through pages only fetches the one you stop on; the others' requests are dropped.
    function go(n) {
        n = Math.min(NP, Math.max(1, Math.round(n) || 1));
        pageNo = n;
        pageInput.value = n;
        history.replaceState(null, '', `?p=${n}`);
        dropAllBut(n);
        clearTimeout(navTimer);
        clearTimeout(nextTimer);
        msg.textContent = '';
        if (current && current.n === n) {
            stage.classList.remove('st-loading');
            return changed();
        }
        veil.textContent = t('Loading page %d...', n);
        stage.classList.add('st-loading');
        changed();
        navTimer = setTimeout(() => open(n), 150);
    }
    async function open(n) {
        try {
            const L = await pageLayers(n, 2);
            if (n !== pageNo) return;
            current = { n, L: { 1: null, 2: L }, asked: { 2: true } };
            stage.classList.remove('st-loading');
            changed();
            if (n < NP) nextTimer = setTimeout(() => pageLayers(n + 1, 2).catch(() => {}), 600);  // likely next
        } catch (e) {
            if (n !== pageNo || e.name === 'AbortError') return;
            veil.textContent = t('Not loaded');
            say(e.message, () => go(n));
        }
    }
    document.addEventListener('keydown', e => {
        if (e.metaKey || e.ctrlKey || e.altKey || (e.target.closest && e.target.closest('input, textarea, select'))) return;
        if (e.key === 'ArrowLeft') go(pageNo - 1);
        else if (e.key === 'ArrowRight') go(pageNo + 1);
        else return;
        e.preventDefault();
    });

    // ---- export ----
    function save(blob, name) {
        const a = el('a', { href: URL.createObjectURL(blob), download: name });
        document.body.append(a);
        a.click();
        setTimeout(() => (URL.revokeObjectURL(a.href), a.remove()), 10000);
    }
    async function exportAs(kind) {
        if (busy) return;
        busy = true;
        panel.classList.add('st-busy');
        const pages = kind === 'png' ? [pageNo] : Array.from({ length: NP }, (_, i) => i + 1);
        const p = Object.assign({}, P), step = 100 / p.scale, dpi = DPI / step, files = [];
        try {
            for (const n of pages) {
                // TRANSLATORS: an export's progress, page by page.
                progress.textContent = pages.length > 1 ? t('Page %d of %d...', n, NP) : t('Working...');
                const L = await pageLayers(n, 1, true);
                const img = render(p, L, exportBox(p, L, step), step);
                if (kind === 'pdf') files.push(await pdfImage(img, dpi));
                else files.push({ name: kind === 'zip' ? `${base}/page-${pad3(n)}.png` : names(n).out, data: await encodePNG(img, dpi) });
            }
            if (kind === 'png') save(new Blob([files[0].data], { type: 'image/png' }), files[0].name);
            else if (kind === 'zip') save(zipFiles(files), `${base} (effects).zip`);
            else save(pdfFile(files, NB.name, NB.version || ''), `${base} (effects).pdf`);
            progress.textContent = t('Done.');
        } catch (e) {
            progress.textContent = e.message || String(e);
        } finally {
            busy = false;
            panel.classList.remove('st-busy');
        }
    }

    async function copyCommand() {
        const text = cmd.textContent;
        try {
            await navigator.clipboard.writeText(text);  // needs https or localhost
        } catch (e) {
            const t = el('textarea', {});
            t.value = text;
            document.body.append(t);
            t.select();
            let ok = false;
            try {
                ok = document.execCommand('copy');
            } catch (e2) { /* reported below */ }
            t.remove();
            if (!ok) {
                getSelection().selectAllChildren(cmd);  // the browser would not copy: select it for Cmd/Ctrl+C
                setText(copyBtn, t('Selected: press Cmd/Ctrl+C'));
                setTimeout(() => setText(copyBtn, t('Copy command')), 3000);
                return;
            }
        }
        setText(copyBtn, t('Copied'));
        setTimeout(() => setText(copyBtn, t('Copy command')), 1500);
    }

    go(pageNo);
})();
