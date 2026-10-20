#!/usr/bin/env node
// studio_copy_check.js: the share studio's "Copy command" button on a plain-http page (the share
// server's: no navigator.clipboard there, it needs https), against a stand-in DOM. Where the
// browser copies through document.execCommand the button says so; where it will not (execCommand
// returns false, as iOS Safari can), the command is selected for Cmd/Ctrl+C and the button says
// that, in the page's language (window.GRIMOIRE_L10N), and nothing throws. Needs node 18+
// (tests/share_check.sh skips it without node).
//   usage: node tests/studio_copy_check.js
'use strict';
const path = require('path');
const STUDIO = path.join(__dirname, '..', 'web', 'studio.js');

function El(tag, buttons) {
    this.tag = tag;
    this.kids = [];
    this.attrs = {};
    this._text = '';
    this.className = '';
    this.listeners = {};
    this.classList = { add() {}, remove() {}, toggle() {} };
    this.style = {};
    if (tag === 'button') buttons.push(this);
}
El.prototype = {
    append(...k) { for (const x of k) this.kids.push(typeof x === 'string' ? { text: x } : x); },
    setAttribute(k, v) { this.attrs[k] = v; },
    addEventListener(ev, fn) { this.listeners[ev] = fn; },
    getContext() { return { putImageData() {}, drawImage() {}, clearRect() {} }; },
    set textContent(v) { this._text = String(v); this.kids = []; },
    get textContent() { return this._text + this.kids.map(k => (k.text !== undefined ? k.text : k.textContent)).join(''); },
    closest() { return null; },
    select() {},
    remove() {},
};

// Loads the studio's page code once more, as a browser would on opening the page: the copy button
// once it is there, and what the page did meanwhile.
function openStudio(execResult, l10n) {
    const buttons = [], log = { selected: false, errors: [] };
    global.window = { GRIMOIRE_NB: { id: 'nb1', name: 'Notebook', pages: 3, dpi: 300, version: 'x' }, GRIMOIRE_L10N: l10n };
    global.document = {
        createElement: t => new El(t, buttons),
        getElementById: () => new El('div', buttons),
        addEventListener() {},
        body: new El('body', buttons),
        execCommand: () => execResult,
    };
    global.getSelection = () => ({ selectAllChildren() { log.selected = true; } });
    Object.defineProperty(globalThis, 'navigator', { value: {}, configurable: true, writable: true });  // no clipboard
    global.localStorage = { getItem() { return null; }, setItem() {} };
    global.location = { search: '' };
    global.history = { replaceState() {} };
    global.requestAnimationFrame = f => setTimeout(f, 0);
    global.fetch = () => Promise.reject(new Error('no network here'));
    global.ImageData = function () {};
    delete require.cache[require.resolve(STUDIO)];
    require(STUDIO);
    return { buttons, log };
}

let failed = 0;
function check(what, ok, got) {
    console.log(`${ok ? 'ok  ' : 'FAIL'} ${what}${ok ? '' : ` (got ${JSON.stringify(got)})`}`);
    if (!ok) failed++;
}

process.on('unhandledRejection', () => {});  // the stand-in has no network: the preview's fetches fail

const cases = [
    { name: 'the browser copies', exec: true, l10n: {}, want: 'Copied', selected: false },
    { name: 'the browser will not copy: the command is selected', exec: false, l10n: {},
      want: 'Selected: press Cmd/Ctrl+C', selected: true },
    { name: 'the same in another language', exec: false,
      l10n: { 'Copy command': '[copy command]', 'Selected: press Cmd/Ctrl+C': '[selected]', Copied: '[copied]' },
      want: '[selected]', selected: true, label: '[copy command]' },
];

(async () => {
    for (const c of cases) {
        const { buttons, log } = openStudio(c.exec, c.l10n);
        await new Promise(r => setTimeout(r, 50));
        const b = buttons.find(x => x.textContent === (c.label || 'Copy command'));
        check(`${c.name}: the button is there`, !!b, buttons.map(x => x.textContent));
        if (!b) continue;
        let threw = null;
        try {
            await b.listeners.click();
        } catch (e) {
            threw = String(e);
        }
        check(`${c.name}: nothing throws`, threw === null, threw);
        check(`${c.name}: the button says "${c.want}"`, b.textContent === c.want, b.textContent);
        check(`${c.name}: ${c.selected ? 'the command is selected' : 'nothing is selected'}`, log.selected === c.selected,
              log.selected);
    }
    console.log(failed ? `studio copy: ${failed} failed` : 'studio copy: all passed');
    process.exit(failed ? 1 : 0);  // (the buttons' 1.5 s and 3 s timers would hold the process)
})();
