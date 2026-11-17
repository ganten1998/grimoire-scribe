// Send a PDF to the Kindle from the Share page (issue #175). The file goes as the body of one POST, its name in a
// header beside the page's token; the Kindle asks before it keeps anything. A send it asks about is answered 200:
// first a line with the send's code, which this page shows while the Kindle shows it under its question, then a
// last line with the answer. Any other status is a refusal before the Kindle asked.
// The browser sends the file as it is: no byte of it passes through this script. The server serves this file
// without its comment lines and indents (httpd.rs, script_bare): keep each comment on a line of its own.
(function () {
    'use strict';
    const host = document.getElementById('upload');
    const cfg = window.GRIMOIRE_UPLOAD;
    if (!host || !cfg || !window.XMLHttpRequest) return;
    const w = cfg.words;
    const button = document.createElement('button');
    button.type = 'button';
    button.className = 'big';
    button.style.cssText = 'font:inherit;color:inherit;background:none;cursor:pointer';
    button.textContent = w.send;
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = 'application/pdf,.pdf';
    input.hidden = true;
    const status = document.createElement('p');
    status.className = 'meta';
    status.setAttribute('role', 'status');
    status.setAttribute('aria-live', 'polite');
    const code = document.createElement('p');
    code.style.cssText = 'font-size:2.5em;font-weight:bold;letter-spacing:.1em;margin:.2em 0';
    host.append(button, input, status, code);
    let busy = false;
    button.addEventListener('click', function () {
        if (!busy) input.click();
    });
    function sending(done, total) {
        status.textContent = w.sending.replace('{percent}', Math.min(100, Math.floor(done * 100 / Math.max(1, total))) + '%');
    }
    // The send's code, once its line has come.
    function coded(x) {
        const m = /^code (\d{4})\n/.exec(x.responseText || '');
        if (m) code.textContent = w.code.replace('%s', m[1]);
    }
    // The answer: a refusal's status, or the last line of a send the Kindle asked about. The code goes with the send.
    function finish(c, text) {
        busy = false;
        button.disabled = false;
        code.textContent = '';
        const said = c === 200 ? text.trim().split('\n').pop() : '';
        status.textContent = said === 'imported' ? w.sent : said === 'another PDF was sent' || c === 409 ? w.busy :
            said === 'no room' || c === 507 ? w.full : c === 413 ? w.big : c === 401 || c === 403 ? w.reload :
            c === 503 ? w.wait : w.no;
    }
    input.addEventListener('change', function () {
        const file = input.files && input.files[0];
        input.value = '';
        if (!file || busy) return;
        if (file.size > cfg.max) {
            status.textContent = w.big;
            return;
        }
        // A lone half of a UTF-16 pair, which a file's name can hold, is a `_` (encodeURIComponent throws on one).
        const name = file.name.replace(/[\uD800-\uDBFF][\uDC00-\uDFFF]|[\uD800-\uDFFF]/g, function (m) {
            return m.length > 1 ? m : '_';
        });
        busy = true;
        button.disabled = true;
        code.textContent = '';
        sending(0, 1);
        const x = new XMLHttpRequest();
        x.open('POST', '/upload');
        x.setRequestHeader('Content-Type', 'application/pdf');
        x.setRequestHeader('X-Grimoire-Token', cfg.token);
        x.setRequestHeader('X-Grimoire-Name', encodeURIComponent(name));
        x.upload.onprogress = function (e) {
            if (e.lengthComputable) sending(e.loaded, e.total);
        };
        x.upload.onload = function () {
            status.textContent = w.confirm;
        };
        x.onprogress = x.onreadystatechange = function () {
            coded(x);
        };
        x.onload = function () {
            finish(x.status, x.responseText || '');
        };
        x.onerror = x.onabort = x.ontimeout = function () {
            finish(0, '');
        };
        x.send(file);
    });
})();
