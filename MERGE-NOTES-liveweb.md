# Merge notes: lane liveweb (Live view browser page)

Branch `claude/rc8-liveweb`, based on `codex/rc7-release-sweep` 6c5357a (2.0.0-rc.7).

## What changed

Files: `web/live.js`, `web/live.css`, `rust/src/httpd.rs` (GRIMOIRE_LIVE_WORDS only),
`tests/live_check.js`, `tests/live_browser_check.js`, `tests/live_stream.py`, docs.

1. **Size engine** (owner decision 3, LM1, LM4, LM5): Fit, Fit width, presets 25% to 200% of the
   Kindle's pixels, Ctrl/Cmd+wheel and trackpad pinch (Safari gesture events too) anchored at the
   pointer with a CSS-scale preview and an exact commit 150 ms after the last event, keys `-`
   `+`/`=` `0` (after live-keys.ini, so a binding wins), drag to pan when larger than the window,
   remembered in `localStorage` `grimoire.live.size`. Exact area-average at every size up to 100%;
   above that the canvas keeps the Kindle's pixels and the browser enlarges it with
   `image-rendering: pixelated` (LT-12: no bilinear full redraws, no 4x canvases). The page sits in
   a fixed scroll viewport, so Fit really fits (the 38 px overflow is gone) and the bar no longer
   rewraps the layout (LM2). Monitor mode forces Fit without forgetting the chosen size.
2. **Rail** (owner decision 2, Option A): slim vertical rail on the left, in the page's gutter:
   bat, live/cast status icon (with a text flag beside it while not live), Size (shows the
   percent), Picture in picture, Monitor, Full screen, Record, then View settings and Keys, which
   open as sheets beside it (one at a time; Escape, a second click or a press outside closes).
   Icons are the Kindle's own Material Design glyphs, extracted from the bundled Terminess Nerd
   Font as inline SVG masks (no new asset, no fetch). Labels stay as button text (screen readers,
   tooltips, tests). The rail never covers the page at Fit; when zoomed, at Fit width, in Monitor
   mode or in a narrow window it fades after 2.5 s without pointer movement (never while not live,
   hovered, focused or with a sheet open). Under 560 px wide the rail is horizontal at the bottom
   and sheets become bottom sheets. Up/Down/Home/End move along the rail; a pointer click gives
   focus back (LM3). View settings groups: Picture (Invert, Warmth, As on the Kindle, Transparent
   paper, now a switch), Pen (Show pen, dot colors, dot size select kept as is), Over the page
   (toolbar, clock and battery), Diagnostics (Latency, now a switch), and, only where Document PiP
   is missing, Picture in picture help.
3. **Picture in picture tiers** (owner decision 1, LT-1, LT-2): `pipTier()` picks
   - `document`: `documentPictureInPicture.requestWindow` (sized to the screen's shape, half the
     screen tall) called first in the click; only the stage moves (`LivePopout.adopt`), the
     window's own rAF and pixel ratio draw it 1:1 at its device pixels.
   - `video`: a dedicated canvas sized to the PiP window's pixels x dpr (refit on its resize),
     the picture area-averaged into it (dirty rects only), the quick ink and pen dot drawn over it,
     `captureStream(0)` + `requestFrame()` only when something changed, made at the first hello so
     the click can call `requestPictureInPicture()` at once; a Blob-URL Worker ticker drives frames
     while the tab is hidden.
   - `firefox`: Firefox has no `requestPictureInPicture` (checked 2026-10: still none; Document PiP
     shipped in Firefox 151, bug 2006594, secure contexts only). On plain http the button shows the
     video in the picture's place with a hint to use Firefox's own Picture-in-Picture button or
     context menu; frames still only on change.
   - `popout`: today's window, but with the stage alone (LT-2).
   While any tier is active the main tab's view renders nothing (canvases moved away, or video PiP
   suspends view/ink/pen drawing behind a placeholder) and is redrawn whole on close.
4. **Perf** (P2, LT-4, LT-5, LT-8, LT-11): one compose/present/paint/pen per animation frame
   (`frameSoon`/`frame`; a burst of batches is one compose of their union); a hidden tab does no
   compose or paint until it shows (a recording still composes at once); `page`, `raw`, `shown`
   are `willReadFrequently` (no GPU readbacks per batch); `areaAverage` keeps span tables and its
   buffer (and now reads exactly the source its spans cover, so a pixel is identical whatever rect
   asked for it); resize is laid out once per frame, makes no canvas again while the size holds,
   shows a GPU-scaled approximation during a burst and the exact picture 150 ms later; `Frames`
   uses a chunk list and copies each body once. No rAF requests pile up while hidden.
5. **LT-3**: hold-eraser renewals run on the Worker ticker (falls back to the active window's
   timer), armed only while a hold-eraser key is held; the latency clock re-sync runs on the active
   window's timer only while Latency is on.

## Strings (l10n: regenerate the catalog, write Spanish once)

No msgid was changed or removed. One TRANSLATORS comment changed (for "Pop out": it is now the
fallback window, not the floating one). New msgids, all in `route_live_page`:

- "Live view controls"
- "View settings"
- "Picture"
- "Pen"
- "Over the page"
- "Diagnostics"
- "Size"
- "Fit width"
- "Smaller"
- "Larger"
- "Picture in picture"
- "Point at the picture and click Firefox's Picture-in-Picture button, or right-click it and choose Picture-in-Picture."
- "For the sharpest picture in picture, let your browser treat this address as secure:"
- "Firefox: open about:config, add %s to dom.securecontext.allowlist (separate several with commas), then reload this page."
- "Chrome, Edge, Brave or Arc: open chrome://flags (edge://flags, brave://flags), find \"Insecure origins treated as secure\", add %s, enable it and relaunch the browser."
- "Safari floats a video of the page instead, which works on this address as it is."

Translator notes: `%s` is filled in by the page's JavaScript (the Kindle's host name for Firefox,
its origin with port for Chromium); keep it, and keep `about:config`,
`dom.securecontext.allowlist`, `chrome://flags`, `edge://flags`, `brave://flags` and
"Insecure origins treated as secure" verbatim (browser UI names). Existing words reused:
"Fit", "Actual size" (tooltip of 100%), "Pop out", "Live view is open in another window. Keep
this tab open.", "Bring Live view back". Percentages use `Intl.NumberFormat` (Spanish gets
"48 %"), no string.

## Tests run (all in this worktree)

- `node tests/live_check.js build/live-check/stream.bin ...` (via `GRIMOIRE_TEST_PORT=8864
  tests/live_check.sh`): LIVE CHECK PASSED, including the new checks: kept span tables vs a
  from-scratch reference (within 1.0 of a rounding; partial rects identical to the whole picture),
  a 200 KB frame in 997-byte chunks, cut-short frames, liveSize/viewSize/zoomStep/anchorScroll
  tables (DPR 1, 1.25, 2; portrait and landscape), pipTier per browser, and the served page's words
  parse as JSON (live_stream.py).
- Browser stand-ins (`tests/live_browser_check.js`, run by live_check.js): 241 ok, 0 failed.
  Updated: the layer checks now paint a frame (compose is per frame). New: remembered size, 0/=/+/-
  keys, a bound `-` goes to the Kindle, Ctrl+wheel preview and commit, Monitor keeps the size; two
  batches one compose of their union; a resize burst is one frame, an unchanged size makes no
  canvas, a new size is one canvas with the approximation then the exact picture; rail sheets,
  Escape (inside and outside the rail), Down key, pointer clicks blur, keyboard clicks keep focus,
  fade only when it could cover the page, a move brings it back, no timer at load; eraser renewals
  armed only while held; Document PiP moves only the stage, frames only from the PiP window, close
  redraws the view whole; video PiP `captureStream(0)`, a frame only on change, from the ticker
  too, view and ink untouched meanwhile, whole redraw on leave; pop-out fallback label.
- `tests/live_keys_check.js`: ok.
- Rust: `cargo test --features sim --test test_httpd` ok; `--lib assets` ok; `make headless` ok.
- Scenarios (`scen.py`, port 8864): live_1_card, y_live_chip, y_live_ink, h_live_vs_settled,
  v_gestures_4h_button2_live: 5/5 passed.
- Real browser: headless Chrome against a harness page (the real live.css/live.js, recorded
  stream from live_check.sh, a fake fetch) for Fit, Fit width, zoom steps to 200%, the Size and
  View settings sheets, the narrow layout, Monitor mode, the video tier's placeholder and frames,
  the Firefox tier's in-page video, and the help block. Not committed (scratch only).
- Not run: the full `make check` (integrator).

## Left undone, and why

- No device or real PiP window was exercised: headless browsers cannot open PiP windows without
  a user gesture. Needs the owner's check (steps below).
- Firefox's own PiP toggle on a MediaStream video (the `firefox` tier) and Safari floating a
  2 px transparent video (the `video` tier keeps the video in the page, invisible) are unverified
  on real browsers.
- Proposal P3 (gray single-channel buffers and a Worker for repaint/average) was not done; the
  brief asked for P2's quick wins.
- The rail is on the left only (no setting for the side); the idle cursor in Monitor mode is
  unchanged; the dot size stays a select. These were open questions in the proposals, not
  requirements.
- The C tree's `src/httpd.c` words were not touched (it already lacked rc.6's pop-out words).

## Owner test steps

1. Install, open Live view in Firefox Nightly on the plain IP. The rail sits left of the page and
   the page fits the window with no scrollbar. Hover each icon for its name.
2. Size: open it, try Fit width, 50%, 100%, 200%; Ctrl/Cmd+scroll and a trackpad pinch over the
   page; `-`, `+` and `0`. At 200% drag the page with the mouse. Reload: the size is remembered.
   At 100% or below thin lines keep their weight; above 100% pixels are crisp squares.
3. View settings: check every switch still works (Invert, Warmth, As on the Kindle, Transparent
   paper, Show pen, dot colors and size, Show toolbar, Show clock and battery, Latency). The
   Picture in picture group explains the secure-address steps.
4. Picture in picture on the plain IP in Firefox: the page turns into a video with a hint; use
   Firefox's PiP button on it. Write on the Kindle: ink and the dot move in the floating window,
   also with another app in front. Bring Live view back.
5. Add the Kindle's IP to `dom.securecontext.allowlist` in about:config, reload, press Picture in
   picture: a floating window with just the page, sharp at its size, writing as smooth as the tab.
   The tab shows only "Live view is open in another window". Resize the floating window, press keys
   (M, F, live-keys bindings, a held eraser) in it, close it: the page returns to the tab at once.
6. Repeat 4 and 5 in Chrome (plain IP: a floating video; with the chrome://flags entry: the
   document window) and in Safari (a floating video).
7. Monitor mode (M): the page fills the window, the rail fades after a moment and returns when the
   mouse moves; leaving Monitor mode restores your size.
8. Make the window narrow (under 560 px): the rail moves to the bottom, sheets open above it.
9. Record still works (MP4 in Firefox and Chrome), with the record icon turning into a stop icon.
