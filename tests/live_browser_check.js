// Run the real page against small DOM/canvas stand-ins. Hold PNG decoding open to prove that
// the independent pen reader paints anyway, and only the eventual tile paint retires its ink;
// then the ink epochs in either order, the pen stream's states, Monitor mode, and how the pen
// stream retries. Then the size engine (stored sizes, keys, Ctrl+wheel), one compose a frame and
// no canvas made again for a resize that keeps the size, the rail (its sheets, fading, clicks
// that give focus back), the held eraser's renewals, and picture in picture in each tier: the
// page's own view drawing nothing while it floats.
'use strict';
const fs = require('fs'), path = require('path'), vm = require('vm');
const SOURCE = fs.readFileSync(path.join(__dirname, '../web/live.js'), 'utf8');

// A page on stand-ins. respond(url) answers its fetches (a stream by default: page.readers[url]
// gets what the test pushes); timers are kept for the test to run (page.timers), intervals too
// (page.intervals). opts: storage (an object: localStorage works, kept there), window and
// document (added to theirs), canvas (added to each canvas), video (added to each video).
function page(respond, opts = {}) {
    const canvases = [], raf = [], readers = {}, handlers = {}, timers = [], fetches = [], toggles = [], elements = [];
    const intervals = [];
    // The page's clock (performance.now): each paint is a frame 100 ms later, so a sample that came
    // before it has played out (the playout holds samples back by at most 20 ms).
    let clock = 1000;
    let fullscreen = 0, focused = null;
    function element(tag) {
        const names = new Set();
        const e = {tag, style: {}, clientWidth: 192, clientHeight: 132, scrollLeft: 0, scrollTop: 0,
            clientLeft: 0, clientTop: 0, offsetLeft: 0, offsetTop: 0, offsetHeight: 20, width: 0, height: 0,
            classList: {toggle(name, on) { toggles.push([name, on]); if (on) names.add(name); else names.delete(name); },
                add(name) { names.add(name); }, remove(name) { names.delete(name); }, contains: name => names.has(name)},
            getBoundingClientRect() { return {left: 0, top: 0, width: 100, height: 100}; }, focus() { focused = e; },
            blur() { e.blurred = (e.blurred || 0) + 1; },
            contains: other => other === e, querySelector: () => null, replaceWith() {}, insertBefore() {},
            append() {}, replaceChildren() {}, setAttribute(k, v) { this['@' + k] = v; },
            addEventListener(type, fn) { (this.listeners[type] = this.listeners[type] || []).push(fn); this[type] = fn; }, listeners: {}};
        elements.push(e);
        if (tag === 'canvas') {
            const ctx = {calls: [], getImageData(x, y, w, h) { return {data: new Uint8ClampedArray(w * h * 4)}; },
                createImageData(w, h) { return {data: new Uint8ClampedArray(w * h * 4)}; }};
            for (const op of ['save', 'restore', 'beginPath', 'arc', 'fill', 'stroke', 'moveTo', 'lineTo', 'rect', 'clip',
                'scale', 'fillRect', 'clearRect', 'drawImage', 'putImageData']) ctx[op] = (...args) => ctx.calls.push([op, ...args]);
            e.getContext = () => ctx; e.ctx = ctx; canvases.push(e);
            // (each new size of its pixels, which clears it, counted)
            let w = 0, h = 0;
            e.sized = 0;
            Object.defineProperty(e, 'width', {get: () => w, set: v => { if (v !== w) e.sized++; w = v; }});
            Object.defineProperty(e, 'height', {get: () => h, set: v => { if (v !== h) e.sized++; h = v; }});
            Object.assign(e, opts.canvas);
        }
        if (tag === 'video') Object.assign(e, opts.video);
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
    const fetch = async (url, options) => {
        fetches.push(url);
        const r = respond && respond(url, clock, options);
        if (r) return r;
        if (url === '/live/keys') return {ok: true, json: async () => []};
        return stream(url);
    };
    const store = opts.storage;
    const document = Object.assign({getElementById: id => id === 'live' ? host : null, createElement: element, body, title: 'Live',
        documentElement: html, pictureInPictureEnabled: false, querySelectorAll: () => [],
        addEventListener(name, fn) { handlers['document.' + name] = fn; }}, opts.document);
    const context = {console, Uint8Array, Uint8ClampedArray, DataView, Blob, performance: {now: () => clock}, Promise, Math, Number, Date,
        JSON, Set, Map, Object, Array, String, document,
        window: Object.assign({GRIMOIRE_LIVE_WORDS: words, fetch, ReadableStream: function () {}, createImageBitmap: () => imagePromise,
            devicePixelRatio: 1, document}, opts.window), fetch, createImageBitmap: () => imagePromise, innerHeight: 150, innerWidth: 800,
        localStorage: store ? {getItem: k => k in store ? store[k] : null, setItem(k, v) { store[k] = String(v); }}
            : {getItem() { throw Error('private storage'); }, setItem() { throw Error('private storage'); }},
        sessionStorage: {getItem() { throw Error('private storage'); }, setItem() { throw Error('private storage'); }},
        location: {replace() {}, reload() {}},
        requestAnimationFrame: fn => raf.push(fn), addEventListener: (name, fn) => { handlers[name] = fn; },
        setInterval: (fn, ms) => intervals.push({fn, ms}), setTimeout: (fn, ms) => timers.push({fn, ms}), clearTimeout() {}};
    vm.runInNewContext(SOURCE, context);
    return {canvases, raf, readers, handlers, timers, intervals, fetches, toggles, decode, elements, host, body, document,
        get focused() { return focused; }, get now() { return clock; }, get fullscreen() { return fullscreen; },
        view: canvases[3], button: label => elements.find(e => e.tag === 'button' && e.textContent === label),
        overlay: canvases[5].ctx, raw: canvases[1].ctx,
        advance(ms) { clock += ms; },
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
function batch(ack, ink, ms = 100) {
    const p = new Uint8Array(20), d = new DataView(p.buffer);
    d.setUint32(8, ack, true); d.setUint32(12, ms, true); d.setUint32(16, ink, true);
    return frame(3, p);
}
const style = (() => {
    const p = new Uint8Array(64), d = new DataView(p.buffer); p[0] = 1;
    [2, 8, 0, 1, 1, 1, 1].forEach((v, i) => d.setFloat32(4 + i * 4, v, true));
    [0, 0, 100, 100, 90, 0, 10, 20].forEach((v, i) => d.setInt32(32 + i * 4, v, true));
    return frame(8, p);
})();
function sample(id, x, flags, epoch = 1, ms = 100, reset = false) {
    const p = new Uint8Array(28), d = new DataView(p.buffer);
    d.setUint32(0, epoch, true); d.setUint32(4, Number(reset), true);
    d.setUint32(8, id, true); d.setUint32(12, ms, true);
    d.setUint16(16, x, true); d.setUint16(18, 40, true); d.setUint16(20, 32768, true);
    p[24] = 2; p[25] = flags; return frame(7, p);
}
// What the overlay shows once the frame is painted: what was drawn on it after its last clear (a
// paint can draw it more than once).
const shownCalls = o => o.calls.slice(o.calls.map(c => c[0]).lastIndexOf('clearRect') + 1);
const has = (o, op) => shownCalls(o).some(c => c[0] === op);
const inks = o => has(o, 'fill') || has(o, 'stroke');

module.exports = async function browserChecks(ok) {
    // A real pop-out moves the same live DOM, rather than making a second client/video.
    {
        const {LivePopout} = require('../web/live.js');
        const made = [];
        function node(tag) {
            const n = {tag, children: [], parent: null, style: {}, className: '',
                append(...items) { for (const item of items) { if (item.parent) item.parent.children.splice(item.parent.children.indexOf(item),1); this.children.push(item); item.parent=this; } },
                replaceWith(item) { const p=this.parent; const i=p.children.indexOf(this); if(item.parent)item.parent.children.splice(item.parent.children.indexOf(item),1);p.children[i]=item;item.parent=p;this.parent=null; },
                addEventListener(type,fn) {this[type]=fn;}, cloneNode() {return node(tag);} };
            made.push(n); return n;
        }
        const style=node('style');
        const doc={title:'Live',body:node('body'),head:node('head'),createElement:node,querySelectorAll:()=>[style]};
        const root=node('live'),canvas=node('canvas');root.append(canvas);doc.body.append(root);
        let child,changes=0,opens=0;
        const opener={open() {opens++;child={document:{head:node('head'),body:node('body')},closed:false,
            focus(){},addEventListener(type,fn){this[type]=fn;},close(){this.closed=true;if(this.pagehide)this.pagehide();}};return child;}};
        const pop=new LivePopout(opener,doc,root,{popoutActive:'Open',popoutReturn:'Back'},()=>changes++);
        ok('pop-out: moves the one live canvas and leaves only a placeholder in the main tab',pop.open() && child.document.body.children[0]===root && root.children[0]===canvas && !doc.body.children.includes(root));
        ok('pop-out: copies styles without opening another Live page or making a video',child.document.head.children.length===1 && opens===1 && !made.some(n=>n.tag==='video'));
        pop.close();
        ok('pop-out: restores the same canvas and view on close, exactly once',doc.body.children[0]===root && root.children[0]===canvas && child.closed && changes===2 && pop.win===null);
        pop.open(); child.closed=true; child.document=null; pop.close();
        ok('pop-out: an already-detached child document cannot strand the original canvas',doc.body.children[0]===root && root.children[0]===canvas && pop.win===null);
        const blocked=new LivePopout({open:()=>null},doc,root,{},()=>{});
        ok('pop-out: a blocked popup keeps the original view in place',!blocked.open() && doc.body.children[0]===root);
    }

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

    // Real readers and paints, with virtual time: old watermarks can be painted, queued, or
    // still decoding when the first new sample arrives. Exercise both sides of the u32 wrap.
    for (const minutes of [36, 40, 71, 72]) for (const order of ['painted', 'queued', 'decoding', 'reset', 'empty', 'hello', 'foreign', 'burst', 'reconnect', 'clock-paused']) {
        const q = page();
        await turn();
        const t = q.readers['/live/stream'];
        t.push(hello(1));
        await turn();
        let r = q.readers['/live/pen'];
        const old = 1000000, oldMs = 0xffff0000, gap = minutes * 60000;
        const id = (old + gap * 1000) >>> 0, ms = (oldMs + gap) >>> 0;
        t.push(batch(old, 1, oldMs));
        r.push(hello(1)); r.push(style); r.push(sample(old, 20, 20, 1, oldMs));
        await turn(); q.paint();
        if (order !== 'clock-paused') q.advance(gap);
        if (order === 'queued' || order === 'decoding') {
            if (order === 'decoding') t.push(frame(2, tile));
            t.push(batch(old, 1, oldMs));
            await turn();
        }
        if (order === 'reconnect') {
            r.end(); await turn();
            q.timers.shift().fn(); await turn();
            r = q.readers['/live/pen'];
            r.push(hello(1)); r.push(style);
        }
        if (order === 'hello') { r.push(hello(1)); r.push(style); }
        if (order === 'foreign') {
            r.push(sample(old, 80, 20, 2, oldMs, true));
            await turn();
        }
        if (order === 'empty') {
            const empty = new Uint8Array(8), d = new DataView(empty.buffer);
            d.setUint32(0, 1, true); d.setUint32(4, 1, true);
            r.push(frame(7, empty));
            await turn();
        }
        if (order === 'burst') {
            const body = new Uint8Array(48), d = new DataView(body.buffer);
            d.setUint32(0, 1, true);
            body.set(sample((old + 4000) >>> 0, 25, 16, 1, (oldMs + 4) >>> 0).subarray(13), 8);
            body.set(sample(id, 40, 20, 1, ms).subarray(13), 28);
            r.push(frame(7, body));
        } else r.push(sample(id, 40, 20, 1, ms, order === 'reset'));
        r.push(sample((id + 4000) >>> 0, 50, 16, 1, (ms + 4) >>> 0));
        await turn(); q.paint();
        const prefix = 'recovery: ' + minutes + ' minutes, ' + order;
        ok(prefix + ': fresh ink and dot paint immediately', has(q.overlay, 'fill') && has(q.overlay, 'stroke') &&
            shownCalls(q.canvases[4].ctx).some(c => c[0] === 'arc' && c[1] >= 50));
        if (order === 'decoding') { q.decode({close() {}}); await turn(); q.paint(); }
        // Repeated old ack from a layer batch must not retire new ink, even after decoding.
        t.push(batch(old, 1, oldMs));
        await turn(); q.paint();
        ok(prefix + ': stale tile acknowledgement keeps fresh ink', has(q.overlay, 'fill') && has(q.overlay, 'stroke'));
        t.push(batch(id, 1, ms));
        await turn(); q.paint();
        ok(prefix + ': current acknowledgement retires only its ink', !has(q.overlay, 'fill') && has(q.overlay, 'stroke'));
        t.push(batch((id + 4000) >>> 0, 1, (ms + 4) >>> 0));
        await turn(); q.paint();
        ok(prefix + ': next acknowledgement retires the rest', !inks(q.overlay));
    }

    // Tiles can already cover the first post-idle sample. Recovery must keep that valid ack,
    // while the dot still follows the pen and only the unacknowledged tail draws as quick ink.
    {
        const q = page(); await turn();
        const t = q.readers['/live/stream']; t.push(hello(1)); await turn();
        const r = q.readers['/live/pen'], gap = 40 * 60000, old = 1000000, id = (old + gap * 1000) >>> 0;
        t.push(batch(old, 1, 100)); r.push(hello(1)); r.push(style); r.push(sample(old, 20, 20));
        await turn(); q.paint(); q.advance(gap);
        t.push(batch(id, 1, gap + 100)); await turn(); q.paint();
        r.push(sample(id, 40, 20, 1, gap + 100)); r.push(sample(id + 4000, 50, 16, 1, gap + 104));
        await turn(); q.paint();
        ok('recovery: tiles first keep their valid ack and draw only the newer tail',
            !has(q.overlay, 'fill') && has(q.overlay, 'stroke') && has(q.canvases[4].ctx, 'arc'));
    }
    // The meter must count the first paint of an ack, not a repeated watermark's idle time.
    // After silence it discards previous observations and calibrates again, even if the local
    // clock barely advanced (the server's sample ms detects this kind of sleep).
    {
        let serverOffset = 0;
        const q = page((url, now) => url === '/live/time' ? {ok: true, json: async () => ({ms: (now + serverOffset) >>> 0})} : null);
        await turn();
        q.elements.find(e => e.textContent === 'latency').click(); await turn();
        const t = q.readers['/live/stream']; t.push(hello(1)); await turn();
        const r = q.readers['/live/pen']; r.push(hello(1)); r.push(style); r.push(sample(1000000, 20, 20, 1, 1000));
        t.push(batch(1000000, 1, 1000)); await turn(); q.paint();
        const meter = q.elements.find(e => e.tag === 'output');
        const tileAges = () => meter.textContent.split(' | ')[1];
        const first = tileAges();
        q.advance(5000); t.push(batch(1000000, 1, 1000)); await turn(); q.paint();
        ok('diagnostics: repeated tile watermark does not inflate paint latency', tileAges() === first);
        const syncs = q.fetches.filter(u => u === '/live/time').length;
        const gap = 40 * 60000, id = (1000000 + gap * 1000) >>> 0;
        serverOffset = 1000 + gap - q.now;
        r.push(sample(id, 40, 20, 1, 1000 + gap)); await turn(); q.paint();
        ok('diagnostics: recovery drops old tile ages and asks for a fresh clock calibration',
            tileAges() === 'tileAge: ...' && q.fetches.filter(u => u === '/live/time').length > syncs &&
            meter.textContent.startsWith('sampleAge: 100.0 / 100.0 / 100.0 ms'));
    }

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
    ok('layers: a batch is composed at the next animation frame, not as it is read', !drewLayer());
    p.paint();
    ok('layers: the toolbar\'s layer drawn over the page, at its place', drewLayer());
    const toolbar = p.elements.find(e => e.textContent === 'toolbar');
    const status = p.elements.find(e => e.textContent === 'statusCorner');
    ok('layers: two switches, both on by default without storage', !!toolbar && !!status &&
       toolbar['@aria-checked'] === 'true' && status['@aria-checked'] === 'true');
    p.raw.calls = [];
    toolbar.click(); p.paint();
    ok('layers: switched off, the page under the toolbar shows and the layer does not',
       toolbar['@aria-checked'] === 'false' && !drewLayer() && p.raw.calls.some(c => c[0] === 'drawImage' && c[1] !== bmp && c[6] === 10));
    p.raw.calls = [];
    toolbar.click(); p.paint();
    ok('layers: on again, the layer is back', drewLayer());
    const empty = new Uint8Array(10);
    t2.push(frame(9, empty)); t2.push(batch(0, 1));
    for (let i = 0; i < 5; i++) await turn();
    p.raw.calls = [];
    toolbar.click(); toolbar.click(); p.paint();
    ok('layers: one no longer on the screen draws nothing', !drewLayer());
    const select = p.elements.find(e => e.tag === 'select'), picker = p.elements.find(e => e.type === 'color');
    ok('dot: the size small and the first color by default, settable without storage', select.value === 'small' &&
       picker.value === '#f03c28' && (select.value = 'large', select.change(), true) && select.value === 'large');

    // The size engine, in a 100 x 100 room (the stand-in viewport less the rail's gutter) with a
    // 100 x 100 screen at one device pixel to a CSS pixel.
    {
        const store = {'grimoire.live.size': '0.5'};
        const q = page(null, {storage: store});
        await turn(); q.readers['/live/stream'].push(hello(1)); await turn(); q.paint();
        ok('size: a remembered 50% shows the screen at 50 px, averaged into 50 x 50 pixels', q.view.style.width === '50px' &&
            q.view.width === 50 && q.view.ctx.calls.some(c => c[0] === 'putImageData'));
        q.handlers.keydown({key: '0'});
        ok('size: 0 fits the page to its room, and is remembered', q.view.style.width === '100px' && store['grimoire.live.size'] === 'fit');
        q.handlers.keydown({key: '='}); q.handlers.keydown({key: '+'});
        q.view.ctx.calls = []; q.paint();
        ok('size: = and + step up the presets; above actual size the canvas keeps the Kindle\'s pixels, enlarged pixelated',
            q.view.style.width === '200px' && q.view.width === 100 && q.view.classList.contains('up') && store['grimoire.live.size'] === '2');
        q.handlers.keydown({key: '-'});
        ok('size: - steps down', q.view.style.width === '150px' && store['grimoire.live.size'] === '1.5');
        const viewport = q.elements.find(e => e.className === 'viewport'), wheel = viewport.listeners.wheel[0], stage = q.elements.find(e => e.className === 'stage');
        let prevented = 0;
        wheel({ctrlKey: true, deltaY: -50, deltaMode: 0, clientX: 50, clientY: 50, preventDefault() { prevented++; }});
        wheel({ctrlKey: false, deltaY: -50, preventDefault() { prevented++; }});
        ok('size: Ctrl+wheel (a pinch) is taken from the browser\'s page zoom and previewed as a scale; a plain wheel scrolls',
            prevented === 1 && /^scale\(/.test(stage.style.transform) && store['grimoire.live.size'] === '1.5');
        q.timers.filter(t => t.ms === 150).pop().fn();
        ok('size: the gesture commits 150 ms after its last event: larger, exact, remembered', store['grimoire.live.size'] === '2' &&
            stage.style.transform === '' && q.view.style.width === '200px');
        q.handlers.keydown({key: 'm'});
        ok('size: Monitor mode fits the page (in all of the room: no padding, no rail gutter) without forgetting the size chosen',
            q.view.style.width === '132px' && store['grimoire.live.size'] === '2');
        q.handlers.keydown({key: 'm'});
        ok('size: and the size comes back after it', q.view.style.width === '200px');
    }
    {
        const q = page(url => url === '/live/keys' ? {ok: true, json: async () => [{code: 12, mods: 0, key: 'Minus', label: 'Undo', action: 'undo'}]} : null);
        await turn(); q.readers['/live/stream'].push(hello(1)); await turn(); await turn();
        q.handlers.keydown({key: '-', code: 'Minus', preventDefault() {}});
        await turn();
        ok('size: a key live-keys.ini binds goes to the Kindle, not to the size', q.fetches.includes('/live/key') && q.view.style.width === '100px');
    }
    // One compose an animation frame, of the batches' rects together; a resize that keeps the size
    // makes no canvas again, and a burst of them is one layout.
    {
        const q = page();
        await turn();
        const t = q.readers['/live/stream'];
        t.push(hello(1)); await turn(); q.paint();
        const tileAt = (x, y) => { const b = new Uint8Array(9), d = new DataView(b.buffer); d.setUint16(0, x, true); d.setUint16(2, y, true);
            d.setUint16(4, 10, true); d.setUint16(6, 10, true); return frame(2, b); };
        t.push(tileAt(0, 0)); t.push(batch(0, 1)); t.push(tileAt(50, 50)); t.push(batch(0, 1));
        q.decode({close() {}});
        for (let i = 0; i < 6; i++) await turn();
        const composes = () => q.raw.calls.filter(c => c[0] === 'drawImage' && c[1] === q.canvases[0]);
        ok('frames: batches read are not composed until the frame', composes().length === 0);
        q.paint();
        const c = composes();
        ok('frames: two batches before one frame are one compose, of both rects', c.length === 1 && c[0].slice(2, 6).join() === '0,0,60,60');
        const sized = q.view.sized;
        q.handlers.resize(); q.handlers.resize(); q.handlers.resize();
        ok('resize: a burst of resize events asks one frame', q.raf.length === 1);
        q.paint();
        ok('resize: a layout that keeps the size makes no canvas again (which clears it)', q.view.sized === sized);
        const viewport = q.elements.find(e => e.className === 'viewport');
        viewport.clientWidth = 142;
        q.handlers.resize(); q.handlers.resize();
        q.view.ctx.calls = []; q.paint();
        ok('resize: a new size is one new canvas, scaled by the browser at once while the resize lasts', q.view.width === 50 &&
            q.view.sized === sized + 2 && q.view.ctx.calls.some(c => c[0] === 'drawImage' && c[1] === q.canvases[2]) &&
            !q.view.ctx.calls.some(c => c[0] === 'putImageData'));
        q.timers.filter(t => t.ms === 150).pop().fn(); q.paint();
        ok('resize: and the exact picture 150 ms after the last', q.view.ctx.calls.some(c => c[0] === 'putImageData'));
    }
    // The rail: its sheets, buttons that give focus back after a click, keys between its buttons,
    // and fading only when it could cover the page and nothing moves.
    {
        const q = page();
        await turn();
        q.readers['/live/stream'].push(hello(1)); q.readers['/live/stream'].push(batch(0, 1));
        await turn(); await turn(); q.paint();
        ok('rail: nothing fades, and no timer runs, at load', !q.toggles.some(t => t[0] === 'chrome-idle' && t[1]) && q.timers.length === 0 &&
            q.intervals.length === 1);
        const sizeButton = q.elements.find(e => e['@aria-controls'] === 'live-size'), sizeSheet = q.elements.find(e => e.id === 'live-size');
        const settings = q.elements.find(e => e['@aria-controls'] === 'live-settings'), settingsSheet = q.elements.find(e => e.id === 'live-settings');
        sizeButton.click({detail: 1});
        ok('rail: Size opens its sheet, keeping focus (it opens, it does not act)', !sizeSheet.hidden &&
            sizeButton['@aria-expanded'] === 'true' && !sizeButton.blurred);
        settings.click({detail: 1});
        ok('rail: one sheet at a time', sizeSheet.hidden && !settingsSheet.hidden && sizeButton['@aria-expanded'] === 'false');
        let prevented = false;
        q.host.listeners.keydown[0]({key: 'Escape', target: settings, preventDefault() { prevented = true; }});
        ok('rail: Escape closes it, focus back on its button, the key kept from the Kindle', settingsSheet.hidden &&
            settings['@aria-expanded'] === 'false' && q.focused === settings && prevented);
        sizeButton.click({detail: 1});
        q.handlers.keydown({key: 'Escape', target: q.body, preventDefault() {}});
        ok('rail: Escape closes it with the focus outside the controls too (a click gave it back)', sizeSheet.hidden &&
            sizeButton['@aria-expanded'] === 'false');
        const monitorButton = q.button('monitor');
        q.host.listeners.keydown[0]({key: 'ArrowDown', target: monitorButton, preventDefault() {}});
        ok('rail: Down moves to the next button', q.focused === q.button('full'));
        monitorButton.click({detail: 1});
        ok('rail: an action clicked with a pointer gives its focus back (no Space presses it again)', monitorButton.blurred === 1);
        monitorButton.click({detail: 0});
        ok('rail: one pressed from the keyboard keeps it', monitorButton.blurred === 1 && monitorButton['@aria-pressed'] === 'false');
        q.intervals[0].fn();
        ok('rail: at Fit it stays: it covers nothing', !q.toggles.some(t => t[0] === 'chrome-idle' && t[1]));
        q.handlers.keydown({key: 'm'});
        q.advance(3000); q.intervals[0].fn();
        ok('rail: in Monitor mode it fades after 2.5 s with nothing moving', q.toggles.filter(t => t[0] === 'chrome-idle').pop()[1] === true);
        q.handlers['document.pointermove']();
        ok('rail: a move brings it back at once', q.toggles.filter(t => t[0] === 'chrome-idle').pop()[1] === false);
    }
    // A held eraser renews its lease on a timer of its own, only while it is held.
    {
        const sent = [];
        const q = page((url, now, o) => {
            if (url === '/live/keys') return {ok: true, json: async () => [{code: 18, mods: 0, key: 'e', label: 'Eraser', action: 'hold-eraser'}]};
            if (url === '/live/key') { sent.push(o.headers['X-Grimoire-Key']); return {ok: true}; }
            return null;
        });
        await turn(); await turn();
        q.handlers.keydown({key: 'e', code: 'KeyE', preventDefault() {}});
        await turn();
        const renew = q.timers.filter(t => t.ms === 500);
        ok('keys: holding the eraser arms one 500 ms renewal', renew.length === 1 && sent.join('|') === '18,0,1');
        renew[0].fn(); await turn();
        ok('keys: which renews the lease and arms the next', sent.join('|') === '18,0,1|18,0,2' && q.timers.filter(t => t.ms === 500).length === 2);
        q.handlers.keyup({key: 'e', code: 'KeyE', preventDefault() {}}); await turn();
        q.timers.filter(t => t.ms === 500).pop().fn(); await turn();
        ok('keys: released, it renews no more', sent.join('|') === '18,0,1|18,0,2|18,0,0' && q.timers.filter(t => t.ms === 500).length === 2);
    }
    // Document Picture-in-Picture: the stage alone moves to its window, whose frames draw it; this
    // tab is asked for none, and gets the same picture back, drawn whole, when it closes.
    {
        const pipRaf = [];
        const pipBody = {className: '', style: {setProperty() {}}, classList: {toggle() {}}, append(n) { this.child = n; }};
        const pipWin = {document: {head: {append() {}}, body: pipBody, addEventListener() {}}, closed: false, innerWidth: 100, innerHeight: 100,
            devicePixelRatio: 1, listeners: {}, focus() {}, addEventListener(t, fn) { this.listeners[t] = fn; },
            requestAnimationFrame: fn => pipRaf.push(fn), close() { this.closed = true; }};
        let asked = null;
        const q = page(null, {window: {documentPictureInPicture: {requestWindow(o) { asked = o; return Promise.resolve(pipWin); }}, open() {}}});
        await turn();
        const t = q.readers['/live/stream'];
        t.push(hello(1)); await turn(); q.paint();
        const pip = q.button('pictureInPicture'), stage = q.elements.find(e => e.className === 'stage');
        ok('pip: where the browser has Document PiP the button says Picture in picture', !!pip && !pip.hidden);
        pip.click(); await turn();
        ok('pip: a window of the screen\'s shape, half the screen tall, takes the stage alone', asked && asked.width === asked.height &&
            pipBody.child === stage && / pip$/.test(pipBody.className) && pip['@aria-pressed'] === 'true');
        pipRaf.splice(0).forEach(fn => fn());
        t.push(batch(0, 1)); await turn(); await turn();
        ok('pip: frames come from the PiP window only: this tab is asked for none', q.raf.length === 0 && pipRaf.length === 1);
        pipRaf.splice(0).forEach(fn => fn());
        q.view.ctx.calls = [];
        pipWin.listeners.pagehide();
        q.paint();
        ok('pip: closed, the same picture comes back, drawn whole', pipWin.closed && pip['@aria-pressed'] === 'false' &&
            q.view.ctx.calls.some(c => c[0] === 'drawImage' && c[1] === q.canvases[2] && c.slice(2, 6).join() === '0,0,100,100'));
    }
    // Video picture in picture (no Document PiP): its own canvas's video, a frame only when the
    // picture changed, drawn by animation frames or (a hidden tab) the ticker; the page's view and
    // its quick ink draw nothing meanwhile, and the view is drawn whole when it ends.
    {
        const track = {frames: 0, requestFrame() { this.frames++; }};
        let fps = null;
        const vwin = {width: 80, height: 80, addEventListener() {}};
        const q = page(null, {
            window: {HTMLCanvasElement: {prototype: {captureStream() {}}}, HTMLVideoElement: {prototype: {requestPictureInPicture() {}}}},
            document: {pictureInPictureEnabled: true, pictureInPictureElement: null},
            canvas: {captureStream(f) { fps = f; return {getVideoTracks: () => [track]}; }},
            video: {play: () => Promise.resolve(), requestPictureInPicture: () => Promise.resolve(vwin)}});
        await turn();
        const t = q.readers['/live/stream'];
        t.push(hello(1)); await turn(); q.paint();
        const video = q.elements.find(e => e.tag === 'video'), pip = q.button('pictureInPicture');
        ok('pip (video): made at the first hello: a video of a canvas that makes frames only when asked', video && fps === 0 &&
            track.frames === 1 && !!pip && !pip.hidden);
        pip.click(); await turn(); q.paint();
        const out = q.canvases.find(c => c.width === 80 && c.ctx.calls.some(x => x[0] === 'drawImage'));
        ok('pip (video): floating, it is as big as its window\'s pixels', pip['@aria-pressed'] === 'true' && !!out && out.height === 80);
        const frames = track.frames, viewCalls = q.view.ctx.calls.length, inkCalls = q.overlay.calls.length;
        q.paint(); q.paint();
        ok('pip (video): no frame while nothing changes', track.frames === frames);
        t.push(frame(2, tile)); t.push(batch(0, 1)); q.decode({close() {}});
        for (let i = 0; i < 6; i++) await turn();
        q.timers.filter(x => x.ms === 16).pop().fn();
        ok('pip (video): a change is one frame, from the ticker as well (a hidden tab has no animation frames)', track.frames === frames + 1);
        q.paint();
        ok('pip (video): meanwhile the page\'s view and its quick ink draw nothing', track.frames === frames + 1 &&
            q.view.ctx.calls.length === viewCalls && q.overlay.calls.length === inkCalls);
        video.leavepictureinpicture();
        q.paint();
        ok('pip (video): ended, the view is drawn whole again', pip['@aria-pressed'] === 'false' &&
            q.view.ctx.calls.slice(viewCalls).some(c => c[0] === 'drawImage' && c[1] === q.canvases[2] && c.slice(2, 6).join() === '0,0,100,100'));
    }
    {
        const q = page(null, {window: {open() {}}});
        await turn();
        ok('pip: without picture in picture, the button pops the picture out', !!q.button('pip') && !q.button('pictureInPicture'));
    }
};
