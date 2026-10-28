// Run the real page against small DOM/canvas stand-ins. Hold PNG decoding open to prove that
// the independent pen reader paints anyway, and only the eventual tile paint retires its ink.
'use strict';
const fs = require('fs'), path = require('path'), vm = require('vm');
module.exports = async function browserChecks(ok) {
    const canvases = [], raf = [], readers = {}, handlers = {};
    function element(tag) {
        const e = {tag, style: {}, classList: {toggle() {}}, clientWidth: 100, clientLeft: 0, clientTop: 0,
            offsetLeft: 0, offsetTop: 0, offsetHeight: 20, width: 0, height: 0,
            append() {}, replaceChildren() {}, setAttribute() {}, addEventListener(type, fn) { this[type] = fn; }};
        if (tag === 'canvas') {
            const ctx = {calls: [], getImageData(x, y, w, h) { return {data: new Uint8ClampedArray(w * h * 4)}; },
                createImageData(w, h) { return {data: new Uint8ClampedArray(w * h * 4)}; }};
            for (const op of ['save', 'restore', 'beginPath', 'arc', 'fill', 'stroke', 'moveTo', 'lineTo', 'rect', 'clip',
                'scale', 'fillRect', 'clearRect', 'drawImage', 'putImageData']) ctx[op] = (...args) => ctx.calls.push([op, ...args]);
            e.getContext = () => ctx; e.ctx = ctx; canvases.push(e);
        }
        return e;
    }
    const host = element('div'), body = element('body'); body.style.setProperty = () => {};
    const words = new Proxy({}, {get: (_, key) => String(key)});
    let decode;
    const imagePromise = new Promise(resolve => { decode = resolve; });
    const fetch = async url => ({ok: true, status: 200, body: {getReader() {
        const pending = [], chunks = [];
        const reader = {read() { return chunks.length ? Promise.resolve(chunks.shift()) : new Promise(r => pending.push(r)); },
            push(value) { const item = {value, done: false}; if (pending.length) pending.shift()(item); else chunks.push(item); }};
        readers[url] = reader; return reader;
    }}});
    const context = {console, Uint8Array, Uint8ClampedArray, DataView, Blob, performance, Promise, Math, Number,
        document: {getElementById: id => id === 'live' ? host : null, createElement: element, body,
            documentElement: element('html'), pictureInPictureEnabled: false},
        window: {GRIMOIRE_LIVE_WORDS: words, fetch, ReadableStream: function () {}, createImageBitmap: () => imagePromise,
            devicePixelRatio: 1}, fetch, createImageBitmap: () => imagePromise, innerHeight: 150,
        localStorage: {getItem() { throw Error('private storage'); }, setItem() { throw Error('private storage'); }},
        requestAnimationFrame: fn => raf.push(fn), addEventListener: (name, fn) => { handlers[name] = fn; },
        setInterval() {}, setTimeout() {}, clearTimeout() {}};
    vm.runInNewContext(fs.readFileSync(path.join(__dirname, '../web/live.js'), 'utf8'), context);
    const turn = () => new Promise(resolve => setImmediate(resolve));
    const paint = () => { const batch = raf.splice(0); for (const fn of batch) fn(performance.now()); };
    const frame = (type, payload) => {
        const out = new Uint8Array(payload.length + 5), d = new DataView(out.buffer);
        d.setUint32(0, payload.length + 1, true); out[4] = type; out.set(payload, 5); return out;
    };
    const hello = new Uint8Array(12), hd = new DataView(hello.buffer);
    [2, 100, 100, 64].forEach((v, i) => hd.setUint16(i * 2, v, true)); hd.setUint32(8, 1, true);
    const style = new Uint8Array(64), sd = new DataView(style.buffer); style[0] = 1;
    [2, 8, 0, 1, 1, 1, 1].forEach((v, i) => sd.setFloat32(4 + i * 4, v, true));
    [0, 0, 100, 100, 90, 0, 10, 20].forEach((v, i) => sd.setInt32(32 + i * 4, v, true));
    function sample(seq, x, flags) {
        const p = new Uint8Array(28), d = new DataView(p.buffer);
        d.setUint32(0, 1, true); d.setUint32(8, seq, true); d.setUint32(12, 100, true);
        d.setUint16(16, x, true); d.setUint16(18, 40, true); d.setUint16(20, 32768, true);
        p[24] = 2; p[25] = flags; return frame(7, p);
    }
    await turn();
    readers['/live/stream'].push(frame(1, hello)); readers['/live/pen'].push(frame(1, hello));
    readers['/live/pen'].push(frame(8, style)); readers['/live/pen'].push(sample(1, 20, 20));
    await turn(); paint();
    const overlay = canvases[4].ctx;
    ok('page: starts with unavailable localStorage and paints fast ink', overlay.calls.some(c => c[0] === 'fill'));
    const tile = new Uint8Array(9), td = new DataView(tile.buffer);
    td.setUint16(4, 100, true); td.setUint16(6, 100, true);
    const batch = new Uint8Array(16), bd = new DataView(batch.buffer);
    bd.setUint32(8, 1, true); bd.setUint32(12, 100, true);
    readers['/live/stream'].push(frame(2, tile)); readers['/live/stream'].push(frame(3, batch));
    readers['/live/pen'].push(sample(2, 30, 16));
    await turn(); overlay.calls = []; paint();
    ok('page: pen paints while tile decoding is still pending', overlay.calls.some(c => c[0] === 'fill') &&
        overlay.calls.some(c => c[0] === 'stroke'));
    decode({close() {}}); await turn(); overlay.calls = []; paint();
    ok('page: tile paint retires its dot but keeps the newer segment', !overlay.calls.some(c => c[0] === 'fill') &&
        overlay.calls.some(c => c[0] === 'stroke'));
    handlers.keydown({key: 'm'});
    ok('page: monitor shortcut works without storage or fullscreen support', !!handlers.fullscreenchange);
};
