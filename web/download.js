// Native PDF downloads with preparation/transfer status. No PDF bytes pass through JavaScript.
(function () {
    'use strict';
    const host = document.getElementById('download-status');
    const words = window.GRIMOIRE_DOWNLOAD_WORDS;
    if (!host || !words || !window.fetch || !window.AbortController || !window.crypto || !window.crypto.getRandomValues) return;
    const active = new Map();
    document.addEventListener('click', function (event) {
        const link = event.target.closest('a');
        if (!link || event.defaultPrevented || event.button !== 0 || event.ctrlKey || event.metaKey ||
            event.shiftKey || event.altKey || link.target || link.hasAttribute('download')) return;
        const url = new URL(link.href, location.href);
        if (url.origin !== location.origin || !/^\/nb\/[a-zA-Z0-9_-]+\.pdf$/.test(url.pathname)) return;
        event.preventDefault();
        if (active.has(url.pathname)) {
            active.get(url.pathname).scrollIntoView({block: 'nearest'});
            return;
        }
        const id = Array.from(crypto.getRandomValues(new Uint8Array(16)), n => n.toString(16).padStart(2, '0')).join('');
        const box = document.createElement('section');
        box.style.cssText = 'border:1px solid currentColor;padding:16px;margin:16px 0';
        const title = document.createElement('strong');
        const card = link.closest('.card');
        title.textContent = (card && card.querySelector('.name') || document.querySelector('h1') || link).textContent + ' · PDF';
        const text = document.createElement('p');
        text.setAttribute('role', 'status');
        text.setAttribute('aria-live', 'polite');
        text.id = 'download-' + id;
        text.textContent = words[0];
        const bar = document.createElement('progress');
        bar.style.cssText = 'width:100%;accent-color:currentColor';
        bar.setAttribute('aria-describedby', text.id);
        bar.setAttribute('aria-label', title.textContent);
        const dismiss = document.createElement('button');
        dismiss.textContent = words[6];
        dismiss.hidden = true;
        dismiss.addEventListener('click', () => box.remove());
        box.append(title, text, bar, dismiss);
        // Put feedback next to the clicked notebook, including in a long Library.
        const controls = link.closest('p');
        if (card) card.append(box);
        else if (controls) controls.insertAdjacentElement('afterend', box);
        else host.append(box);
        box.scrollIntoView({block: 'nearest'});
        active.set(url.pathname, box);
        const frame = document.createElement('iframe');
        frame.hidden = true;
        frame.title = 'PDF';
        document.body.append(frame);
        url.searchParams.set('progress', id);
        frame.src = url.href;
        let failures = 0, finished = false, timer;
        function finish(message) {
            if (finished) return;
            finished = true;
            clearTimeout(timer);
            text.textContent = message;
            bar.hidden = true;
            dismiss.hidden = false;
            active.delete(url.pathname);
            // Keep the frame: removing it can cancel a native download still draining its socket.
        }
        async function poll() {
            const abort = new AbortController();
            const timeout = setTimeout(() => abort.abort(), 5000);
            try {
                const response = await fetch('/api/download/' + id, {cache: 'no-store', signal: abort.signal});
                if (!response.ok) throw new Error('status');
                const d = await response.json();
                failures = 0;
                if (d.state === 4) return finish(words[3]);
                if (d.state === 5) return finish(words[4]);
                if (d.state === 3) {
                    bar.max = Math.max(1, d.bytes);
                    bar.value = d.sent;
                    text.textContent = words[2].replace('{percent}', Math.min(100, Math.floor(d.sent * 100 / bar.max)));
                } else if (d.state === 2 && d.total > 0) {
                    bar.max = d.total;
                    bar.value = d.pages;
                    text.textContent = words[1].replace('{done}', d.pages).replace('{total}', d.total);
                } else {
                    bar.removeAttribute('value');
                    text.textContent = d.state === 1 ? words[7] : words[0];
                }
            } catch (_) {
                if (++failures >= 5) return finish(words[5]);
            } finally { clearTimeout(timeout); }
            if (!finished) timer = setTimeout(poll, 750);
        }
        timer = setTimeout(poll, 300);
    });
})();
