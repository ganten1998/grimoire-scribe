// The Share page's Send a PDF box (web/upload.js, issue #175) without a browser: what it sends, and what it says for
// each answer the server gives.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const vm = require('node:vm');
const source = fs.readFileSync('web/upload.js', 'utf8');
const words = {send: 'send', sending: 'sending {percent}', confirm: 'confirm', sent: 'sent', no: 'no', busy: 'busy',
    reload: 'reload', wait: 'wait', big: 'big', full: 'full'};

function setup({supported = true, box = true} = {}) {
    const elements = [], sent = [];
    class Element {
        constructor(tag) { this.tag = tag; this.children = []; this.attrs = {}; this.style = {}; elements.push(this); }
        append(...children) { this.children.push(...children); }
        setAttribute(k, v) { this.attrs[k] = v; }
        addEventListener(k, f) { this['on' + k] = f; }
        click() { this.clicked = (this.clicked || 0) + 1; }
    }
    const host = new Element('div');
    class XMLHttpRequest {
        constructor() { this.headers = {}; this.upload = {}; sent.push(this); }
        open(method, url) { this.method = method; this.url = url; }
        setRequestHeader(k, v) { this.headers[k] = v; }
        send(body) { this.body = body; }
    }
    const window = {GRIMOIRE_UPLOAD: {token: '0123456789abcdef0123456789abcdef', max: 1000, words}};
    if (supported) window.XMLHttpRequest = XMLHttpRequest;
    vm.runInNewContext(source, {window, document: {getElementById: () => box ? host : null, createElement: t => new Element(t)},
        XMLHttpRequest, encodeURIComponent, Math});
    const [button, input, status] = host.children;
    function pick(file) {
        input.files = [file];
        input.value = 'C:\\fakepath\\' + file.name;
        input.onchange();
        return sent[sent.length - 1];
    }
    return {host, button, input, status, sent, pick};
}

const s = setup();
assert.equal(s.button.tag, 'button');
assert.equal(s.button.textContent, 'send');
assert.equal(s.input.type, 'file');
assert.equal(s.input.accept, 'application/pdf,.pdf');
assert.equal(s.input.hidden, true);
assert.equal(s.status.attrs.role, 'status');
s.button.onclick();
assert.equal(s.input.clicked, 1, 'the button opens the file picker');
// too big: refused here, nothing sent
s.pick({name: 'big.pdf', size: 1001});
assert.equal(s.sent.length, 0);
assert.equal(s.status.textContent, 'big');
// a PDF: one POST of the file as it is, its name percent-encoded beside the token
const file = {name: '2027 Planner & notes/é.pdf', size: 1000};
const x = s.pick(file);
assert.equal(x.method, 'POST');
assert.equal(x.url, '/upload');
assert.equal(x.body, file, 'the file itself is the body');
assert.equal(x.headers['X-Grimoire-Token'], '0123456789abcdef0123456789abcdef');
assert.equal(x.headers['X-Grimoire-Name'], '2027%20Planner%20%26%20notes%2F%C3%A9.pdf');
assert.equal(x.headers['Content-Type'], 'application/pdf');
assert.equal(s.input.value, '', 'the pick is let go: the same file can be sent again');
assert.equal(s.button.disabled, true);
assert.equal(s.status.textContent, 'sending 0%');
// a second pick while one is under way sends nothing
s.pick({name: 'other.pdf', size: 10});
assert.equal(s.sent.length, 1);
s.button.onclick();
assert.equal(s.input.clicked, 1);
x.upload.onprogress({lengthComputable: true, loaded: 420, total: 1000});
assert.equal(s.status.textContent, 'sending 42%');
x.upload.onload();
assert.equal(s.status.textContent, 'confirm');
x.status = 200;
x.onload();
assert.equal(s.status.textContent, 'sent');
assert.equal(s.button.disabled, false);
// every other answer, in the page's words
for (const [code, said] of [[410, 'no'], [408, 'no'], [415, 'no'], [500, 'no'], [409, 'busy'], [401, 'reload'],
    [403, 'reload'], [413, 'big'], [507, 'full'], [503, 'wait'], [400, 'no']]) {
    const y = s.pick({name: 'a.pdf', size: 10});
    y.status = code;
    y.onload();
    assert.equal(s.status.textContent, said, String(code));
}
const z = s.pick({name: 'a.pdf', size: 10});
z.onerror();
assert.equal(s.status.textContent, 'no', 'a connection lost');
assert.equal(s.button.disabled, false);
// no box without the page's words, or where the browser has no XMLHttpRequest
assert.equal(setup({supported: false}).host.children.length, 0);
assert.equal(setup({box: false}).host.children.length, 0);
console.log('Upload UI: the picker, one POST of the file with its name and token, progress, every answer passed');
