// Run the real page against small DOM/canvas stand-ins. Hold PNG decoding open to prove that
// the independent pen reader paints anyway, and only the eventual tile paint retires its ink;
// then the ink epochs in either order, the pen stream's states, Monitor mode, and how the pen
// stream retries.
'use strict';
const fs = require('fs'), path = require('path'), vm = require('vm');
const SOURCE = fs.readFileSync(path.join(__dirname, '../web/live.js'), 'utf8');

// A page on stand-ins. respond(url) answers its fetches (a stream by default: page.readers[url]
// gets what the test pushes); timers are kept for the test to run (page.timers).
function page(respond) {
    const canvases = [], raf = [], readers = {}, handlers = {}, timers = [], fetches = [], toggles = [], elements = [];
    // The page's clock (performance.now): each paint is a frame 100 ms later, so a sample that came
    // before it has played out (the playout holds samples back by at most 20 ms).
    let clock = 1000;
    let fullscreen = 0;
    function element(tag) {
        const e = {tag, style: {}, classList: {toggle(name, on) { toggles.push([name, on]); }}, clientWidth: 100,
            clientLeft: 0, clientTop: 0, offsetLeft: 0, offsetTop: 0, offsetHeight: 20, width: 0, height: 0,
            append() {}, replaceChildren() {}, setAttribute(k, v) { this['@' + k] = v; }, addEventListener(type, fn) { this[type] = fn; }};
        elements.push(e);
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
    const html = element('html');
    html.requestFullscreen = () => { fullscreen++; return Promise.resolve(); };
    const words = new Proxy({}, {get: (_, key) => String(key)});
    let decode;
    const imagePromise = new Promise(resolve => { decode = resolve; });
    function stream(url) {
        return {ok: true, status: 200, body: {getReader() {
            const pending = [], chunks = [];
            const reader = {read() { return chunks.length ? Promise.resolve(chunks.shift()) : new Promise(r => pending.push(r)); },
                push(value) { const item = {value, done: false}; if (pending.length) pending.shift()(item); else chunks.push(item); },
                end() { const item = {value: undefined, done: true}; if (pending.length) pending.shift()(item); else chunks.push(item); }};
            readers[url] = reader; return reader;
        }}};
    }
    const fetch = async url => { fetches.push(url); const r = respond && respond(url); return r || stream(url); };
    const context = {console, Uint8Array, Uint8ClampedArray, DataView, Blob, performance: {now: () => clock}, Promise, Math, Number, Date,
        JSON, Set, Object, Array, String,
        document: {getElementById: id => id === 'live' ? host : null, createElement: element, body,
            documentElement: html, pictureInPictureEnabled: false},
        window: {GRIMOIRE_LIVE_WORDS: words, fetch, ReadableStream: function () {}, createImageBitmap: () => imagePromise,
            devicePixelRatio: 1}, fetch, createImageBitmap: () => imagePromise, innerHeight: 150,
        localStorage: {getItem() { throw Error('private storage'); }, setItem() { throw Error('private storage'); }},
        sessionStorage: {getItem() { throw Error('private storage'); }, setItem() { throw Error('private storage'); }},
        location: {replace() {}, reload() {}},
        requestAnimationFrame: fn => raf.push(fn), addEventListener: (name, fn) => { handlers[name] = fn; },
        setInterval() {}, setTimeout: (fn, ms) => timers.push({fn, ms}), clearTimeout() {}};
    vm.runInNewContext(SOURCE, context);
    return {canvases, raf, readers, handlers, timers, fetches, toggles, decode, elements, get fullscreen() { return fullscreen; },
        overlay: canvases[5].ctx, raw: canvases[1].ctx,
        paint() { clock += 100; const batch = raf.splice(0); for (const fn of batch) fn(clock); }};
}
const turn = () => new Promise(resolve => setImmediate(resolve));
const frame = (type, payload) => {
    const out = new Uint8Array(payload.length + 5), d = new DataView(out.buffer);
    d.setUint32(0, payload.length + 1, true); out[4] = type; out.set(payload, 5); return out;
};
function hello(epoch) {
    const p = new Uint8Array(12), d = new DataView(p.buffer);
    [3, 100, 100, 64].forEach((v, i) => d.setUint16(i * 2, v, true)); d.setUint32(8, epoch, true);
    return frame(1, p);
}
function batch(ack, ink) {
    const p = new Uint8Array(20), d = new DataView(p.buffer);
    d.setUint32(8, ack, true); d.setUint32(12, 100, true); d.setUint32(16, ink, true);
    return frame(3, p);
}
const style = (() => {
    const p = new Uint8Array(64), d = new DataView(p.buffer); p[0] = 1;
    [2, 8, 0, 1, 1, 1, 1].forEach((v, i) => d.setFloat32(4 + i * 4, v, true));
    [0, 0, 100, 100, 90, 0, 10, 20].forEach((v, i) => d.setInt32(32 + i * 4, v, true));
    return frame(8, p);
})();
function sample(id, x, flags, epoch = 1) {
    const p = new Uint8Array(28), d = new DataView(p.buffer);
    d.setUint32(0, epoch, true); d.setUint32(8, id, true); d.setUint32(12, 100, true);
    d.setUint16(16, x, true); d.setUint16(18, 40, true); d.setUint16(20, 32768, true);
    p[24] = 2; p[25] = flags; return frame(7, p);
}
// What the overlay shows once the frame is painted: what was drawn on it after its last clear (a
// paint can draw it more than once).
const shownCalls = o => o.calls.slice(o.calls.map(c => c[0]).lastIndexOf('clearRect') + 1);
const has = (o, op) => shownCalls(o).some(c => c[0] === op);
const inks = o => has(o, 'fill') || has(o, 'stroke');

module.exports = async function browserChecks(ok) {
    let p = page();
    await turn();
    const tiles = p.readers['/live/stream'], pen = () => p.readers['/live/pen'];
    const before = p.fetches.includes('/live/pen');
    tiles.push(hello(1));
    await turn();
    ok('page: the pen stream waits for the tile stream to say hello', !before && p.fetches.includes('/live/pen'));
    tiles.push(batch(0, 1));
    pen().push(hello(1)); pen().push(style); pen().push(sample(1, 20, 20));
    await turn(); p.paint();
    ok('page: starts with unavailable localStorage and paints fast ink', has(p.overlay, 'fill'));
    const tile = new Uint8Array(9), td = new DataView(tile.buffer);
    td.setUint16(4, 100, true); td.setUint16(6, 100, true);
    tiles.push(frame(2, tile)); tiles.push(batch(1, 1));
    pen().push(sample(2, 30, 16));
    await turn(); p.overlay.calls = []; p.paint();
    ok('page: pen paints while tile decoding is still pending', has(p.overlay, 'fill') && has(p.overlay, 'stroke'));
    p.decode({close() {}}); await turn(); p.overlay.calls = []; p.paint();
    ok('page: tile paint retires its dot but keeps the newer segment', !has(p.overlay, 'fill') && has(p.overlay, 'stroke'));
    // A page turned: the pen's hello of the new ink epoch first. Its ink waits for a picture of it.
    pen().push(hello(2)); pen().push(style); pen().push(sample(3, 50, 20, 2)); pen().push(sample(4, 60, 16, 2));
    await turn(); p.overlay.calls = []; p.paint();
    ok('page: a new ink epoch shows no ink over the old picture', !inks(p.overlay));
    tiles.push(batch(0, 2));
    await turn(); p.overlay.calls = []; p.paint();
    ok('page: then the picture in that epoch comes, and the ink with it', inks(p.overlay));
    // The other order: the picture's batch of the next epoch first. The old ink goes at once.
    tiles.push(batch(0, 3));
    await turn(); p.overlay.calls = []; p.paint();
    ok('page: a picture of a newer ink epoch takes the old ink away', !inks(p.overlay));
    pen().push(hello(3)); pen().push(style); pen().push(sample(5, 70, 20, 3));
    await turn(); p.overlay.calls = []; p.paint();
    ok('page: and the pen\'s hello of it brings the new ink', inks(p.overlay));
    // Asleep, as the pen stream says it: no ink, no pen; awake again, nothing comes back by itself.
    const asleep = frame(5, new Uint8Array([1, 0, 255]));
    pen().push(asleep);
    await turn(); p.overlay.calls = []; p.paint();
    ok('page: a state but awake on the pen stream takes the ink away', !inks(p.overlay));
    p.handlers.keydown({key: 'm'});
    ok('page: M turns Monitor mode on, and asks for full screen on that key press',
        p.toggles.some(t => t[0] === 'monitor' && t[1] === true) && p.fullscreen === 1);
    p.handlers.keydown({key: 'm'});
    ok('page: M again turns it off', p.toggles.filter(t => t[0] === 'monitor').pop()[1] === false);

    // The pen stream on a Kindle without it (404, and Live view off): it waits for the next tile
    // hello, never polls; a failure backs off as the tile stream does, from 500 ms to 8 s.
    let answer = {ok: false, status: 404};
    p = page(url => url === '/live/pen' ? answer : null);
    await turn();
    p.readers['/live/stream'].push(hello(1));
    for (let i = 0; i < 5; i++) await turn();
    const asked = () => p.fetches.filter(u => u === '/live/pen').length;
    ok('page: a 404 for the pen stream stops it, with no timer', asked() === 1 && p.timers.length === 0);
    answer = {ok: false, status: 500};
    p.readers['/live/stream'].push(hello(1));
    for (let i = 0; i < 5; i++) await turn();
    const delays = [];
    for (let i = 0; i < 6; i++) {
        const t = p.timers.shift();
        if (!t) break;
        delays.push(t.ms);
        t.fn();
        for (let k = 0; k < 5; k++) await turn();
    }
    ok('page: the next tile hello starts it again; failures back off to 8 s (' + delays + ')', asked() === 8 &&
        delays.join() === '500,1000,2000,4000,8000,8000');

    // The layers (frame 9): the toolbar drawn over the page once its batch is painted, gone from the
    // picture when its switch is off (the page under it shows), back when on; the switches and the
    // dot's settings start from their defaults where the browser keeps nothing.
    p = page();
    await turn();
    const t2 = p.readers['/live/stream'];
    t2.push(hello(1));
    const layer = new Uint8Array(14), ld = new DataView(layer.buffer);
    layer[0] = 0; ld.setUint16(2, 10, true); ld.setUint16(4, 20, true); ld.setUint16(6, 30, true); ld.setUint16(8, 40, true);
    t2.push(frame(9, layer)); t2.push(batch(0, 1));
    const bmp = {close() {}, tag: 'toolbar'};
    p.decode(bmp);
    for (let i = 0; i < 5; i++) await turn();
    const drewLayer = () => p.raw.calls.some(c => c[0] === 'drawImage' && c[1] === bmp && c[6] === 10 && c[7] === 20);
    ok('layers: the toolbar\'s layer drawn over the page, at its place', drewLayer());
    const toolbar = p.elements.find(e => e.textContent === 'toolbar');
    const status = p.elements.find(e => e.textContent === 'statusCorner');
    ok('layers: two switches, both on by default without storage', !!toolbar && !!status &&
       toolbar['@aria-checked'] === 'true' && status['@aria-checked'] === 'true');
    p.raw.calls = [];
    toolbar.click();
    ok('layers: switched off, the page under the toolbar shows and the layer does not',
       toolbar['@aria-checked'] === 'false' && !drewLayer() && p.raw.calls.some(c => c[0] === 'drawImage' && c[1] !== bmp && c[6] === 10));
    p.raw.calls = [];
    toolbar.click();
    ok('layers: on again, the layer is back', drewLayer());
    const empty = new Uint8Array(10);
    t2.push(frame(9, empty)); t2.push(batch(0, 1));
    for (let i = 0; i < 5; i++) await turn();
    p.raw.calls = [];
    toolbar.click(); toolbar.click();
    ok('layers: one no longer on the screen draws nothing', !drewLayer());
    const select = p.elements.find(e => e.tag === 'select'), picker = p.elements.find(e => e.type === 'color');
    ok('dot: the size small and the first color by default, settable without storage', select.value === 'small' &&
       picker.value === '#f03c28' && (select.value = 'large', select.change(), true) && select.value === 'large');
};
