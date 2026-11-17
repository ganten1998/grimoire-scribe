// Send a PDF to the Kindle from the Share page (issue #175). The file goes as the body of one POST, its name in a
// header beside the page's token; the Kindle asks before it keeps anything, and its answer is this request's reply.
// The browser sends the file as it is: no byte of it passes through this script.
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
    host.append(button, input, status);
    let busy = false;
    button.addEventListener('click', function () {
        if (!busy) input.click();
    });
    function sending(done, total) {
        status.textContent = w.sending.replace('{percent}', Math.min(100, Math.floor(done * 100 / Math.max(1, total))) + '%');
    }
    function finish(code) {
        busy = false;
        button.disabled = false;
        status.textContent = code === 200 ? w.sent : code === 413 ? w.big : code === 409 ? w.busy :
            code === 401 || code === 403 ? w.reload : code === 507 ? w.full : code === 503 ? w.wait : w.no;
    }
    input.addEventListener('change', function () {
        const file = input.files && input.files[0];
        input.value = '';
        if (!file || busy) return;
        if (file.size > cfg.max) {
            status.textContent = w.big;
            return;
        }
        busy = true;
        button.disabled = true;
        sending(0, 1);
        const x = new XMLHttpRequest();
        x.open('POST', '/upload');
        x.setRequestHeader('Content-Type', 'application/pdf');
        x.setRequestHeader('X-Grimoire-Token', cfg.token);
        x.setRequestHeader('X-Grimoire-Name', encodeURIComponent(file.name));
        x.upload.onprogress = function (e) {
            if (e.lengthComputable) sending(e.loaded, e.total);
        };
        x.upload.onload = function () {
            status.textContent = w.confirm;
        };
        x.onload = function () {
            finish(x.status);
        };
        x.onerror = x.onabort = x.ontimeout = function () {
            finish(0);
        };
        x.send(file);
    });
})();
