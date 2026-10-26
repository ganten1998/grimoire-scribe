// Native-download behavior and status transitions without a browser or notebook data.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const vm = require('node:vm');
const {webcrypto} = require('node:crypto');
const source = fs.readFileSync('web/download.js', 'utf8');
const words = ['preparing', '{done}/{total}', '{percent}', 'sent', 'failed', 'unavailable', 'dismiss', 'queued'];

function setup({card = true, supported = true} = {}) {
    const elements = [], timers = new Map(), responses = [];
    let timerId = 0, click;
    class Element {
        constructor(tag) { this.tag = tag; this.children = []; this.attrs = {}; this.style = {}; elements.push(this); }
        append(...children) { this.children.push(...children); }
        setAttribute(k, v) { this.attrs[k] = v; }
        removeAttribute(k) { delete this.attrs[k]; }
        addEventListener(k, f) { this[k] = f; }
        insertAdjacentElement(where, e) { assert.equal(where, 'afterend'); this.after = e; }
        scrollIntoView() { this.scrolled = true; }
        remove() { this.removed = true; }
    }
    const host = new Element('div'), body = new Element('body'), controls = new Element('p');
    const notebook = new Element('article');
    notebook.querySelector = () => ({textContent: '<Notes & music>'});
    const link = {
        href: 'http://127.0.0.1:18789/nb/test-1.pdf', target: '', textContent: 'PDF',
        hasAttribute: () => false, closest: s => s === 'a' ? link : s === '.card' ? card ? notebook : null : controls,
    };
    const document = {
        body, getElementById: () => host, createElement: tag => new Element(tag),
        querySelector: () => ({textContent: 'Notebook'}), addEventListener: (_, f) => { click = f; },
    };
    const fetch = async url => {
        assert.match(url, /^\/api\/download\/[0-9a-f]{32}$/);
        const d = responses.shift();
        if (!d) throw Error('offline');
        return {ok: true, json: async () => d};
    };
    vm.runInNewContext(source, {
        window: {GRIMOIRE_DOWNLOAD_WORDS: words, fetch: supported ? fetch : null, crypto: webcrypto, AbortController},
        document, location: {href: link.href, origin: 'http://127.0.0.1:18789'}, crypto: webcrypto, URL, AbortController,
        Uint8Array, fetch, setTimeout: (fn, ms) => { timers.set(++timerId, {fn, ms}); return timerId; },
        clearTimeout: id => timers.delete(id),
    });
    function tap(extra = {}) {
        const e = {target: link, button: 0, preventDefault() { this.prevented = true; }, ...extra};
        click(e);
        return e;
    }
    async function poll(d) {
        responses.push(d);
        const [id, t] = [...timers].sort((a, b) => a[1].ms - b[1].ms)[0];
        timers.delete(id);
        await t.fn();
    }
    return {tap, poll, elements, timers, controls, host, body, get click() { return click; },
        get box() { return elements.find(e => e.tag === 'section'); },
        get text() { return elements.find(e => e.attrs.role === 'status'); },
        get bar() { return elements.find(e => e.tag === 'progress'); }};
}

(async () => {
    const s = setup();
    assert.equal(s.tap({ctrlKey: true}).prevented, undefined);
    assert.equal(s.tap({button: 1}).prevented, undefined);
    assert.equal(s.tap().prevented, true);
    assert.equal(s.box.children[0].textContent, '<Notes & music> · PDF');
    assert.ok(s.box.scrolled);
    assert.match(s.body.children[0].src, /\/nb\/test-1.pdf\?progress=[0-9a-f]{32}$/);
    s.tap();
    assert.equal(s.body.children.length, 1, 'repeated click must not restart an export');
    await s.poll({state: 1});
    assert.equal(s.text.textContent, 'queued');
    await s.poll({state: 2, pages: 3, total: 99});
    assert.equal(s.text.textContent, '3/99');
    assert.equal(s.bar.value, 3);
    assert.equal(s.bar.max, 99);
    await s.poll({state: 3, bytes: 1000, sent: 420});
    assert.equal(s.text.textContent, '42%');
    await s.poll({state: 4});
    assert.equal(s.text.textContent, 'sent');
    assert.equal(s.bar.hidden, true);
    assert.equal(s.timers.size, 0);
    assert.equal(s.body.children[0].removed, undefined, 'do not cancel the native download');
    s.tap();
    assert.equal(s.body.children.length, 2, 'a completed export can be requested again');

    const failed = setup({card: false});
    failed.tap();
    assert.equal(failed.controls.after, failed.box, 'reader status belongs beside its download controls');
    await failed.poll({state: 5});
    assert.equal(failed.text.textContent, 'failed');
    const offline = setup();
    offline.tap();
    for (let i = 0; i < 5; i++) await offline.poll(null);
    assert.equal(offline.text.textContent, 'unavailable');
    assert.equal(offline.timers.size, 0);
    assert.equal(setup({supported: false}).click, undefined, 'unsupported browsers keep ordinary links');
    console.log('Download UI: native transfer, duplicate clicks, placement, progress, failure and offline passed');
})().catch(e => { console.error(e); process.exitCode = 1; });
