// The real keyboard controller: focus, exact modifiers, repeat suppression, bounded asynchronous
// delivery, release after focus changes, and the held eraser's renewable lease.
'use strict';
const {LiveKeyboard, liveKeyCode, keyEditing} = require('../web/live.js');
const turn = () => new Promise(resolve => setImmediate(resolve));
module.exports = async function (ok) {
    const bindings = [{code: 44, mods: 1, action: 'undo'}, {code: 18, mods: 0, action: 'hold-eraser'}];
    const sent = [];
    let errors = 0;
    const k = new LiveKeyboard(bindings, async data => sent.push(data), () => errors++);
    const event = extra => ({key: 'z', code: 'KeyZ', ctrlKey: true, target: {tagName: 'CANVAS'},
        preventDefault() { this.prevented = true; }, ...extra});
    for (const target of [{tagName: 'INPUT'}, {tagName: 'SELECT'}, {tagName: 'TEXTAREA'},
        {isContentEditable: true}, {closest: () => ({})}]) {
        ok('keys: editable focus is ignored', !k.down(event({target})) && keyEditing(target));
    }
    ok('keys: exact modifiers, composition and AltGr are ignored', !k.down(event({shiftKey: true})) &&
        !k.down(event({isComposing: true})) && !k.down(event({getModifierState: () => true})));
    const first = event();
    ok('keys: mapped shortcut takes browser default', k.down(first) && first.prevented);
    for (let i = 0; i < 100; i++) k.down(event({repeat: true}));
    k.down(event());
    await turn();
    ok('keys: repeat events and duplicate downs send once', sent.length === 1 && sent[0].join() === '44,1,1');
    // Modifiers can be released first; focus can move to an input before keyup.
    k.up(event({ctrlKey: false, target: {tagName: 'INPUT'}}));
    await turn();
    ok('keys: keyup uses the original modifiers even after focus changes', sent[1].join() === '44,1,0');
    k.down(event({key: 'e', code: 'KeyE', ctrlKey: false}));
    k.renew();
    k.release();
    await turn();
    ok('keys: eraser renews without repeating and blur releases it', sent.slice(2).map(a => a.join()).join('|') ===
        '18,0,1|18,0,2|18,0,0' && k.held.size === 0);
    ok('keys: logical international letters, navigation, media and physical code bindings',
        liveKeyCode({key: 'Z', code: 'KeyY'}) === 44 && liveKeyCode({key: 'ArrowRight'}) === 106 &&
        liveKeyCode({key: 'MediaPlayPause'}) === 164 && liveKeyCode({key: ';', code: 'Semicolon'}) === 39 &&
        liveKeyCode({key: 'F12', code: 'F12'}) === 88 && liveKeyCode({key: 'é', code: 'KeyE'}) === 0);
    let finish;
    const slow = new LiveKeyboard(bindings, () => new Promise(resolve => { finish = resolve; }), () => {});
    for (let i = 0; i < 100; i++) slow.enqueue(44, 1, 1);
    ok('keys: slow network has one request and 32 waiting at most', slow.busy && slow.pending.length === 32);
    slow.pending.length = 0; finish(); await turn();
    const broken = new LiveKeyboard(bindings, async () => { throw Error('offline'); }, () => errors++);
    broken.down(event());
    broken.enqueue(18, 0, 1);
    await turn();
    ok('keys: unknown delivery is never retried; queue and holds are cleared', errors === 1 &&
        !broken.busy && !broken.pending.length && !broken.held.size);
};
