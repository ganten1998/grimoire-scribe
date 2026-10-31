# Backlog

Work that was decided, deferred or found but not done yet. Keep this current: when an item ships,
move it to "Done" with the version; when something is deferred, add it here with who decided and
why. Newest decisions first within each section.

## Next: 2.1 (deferred by the owner, 2026-10-31)

- **Custom Paper builder.** A picker tile that builds custom versions of ruled, dot grid, graph
  paper, cross grid, isometric, hexagon and Seyes paper (spacing, line weight, margins). Needs a
  new paper namespace in the page format (docs/FORMAT.md); older versions show such pages as
  blank "Newer paper".
- **Smart-shape snapping to geometry.** A snapped shape's corners, ends and edges stick to other
  shapes, lines and points already on the page.
- **Open Amazon's own Scribe notebooks in Grimoire.** Native notebooks are folders in
  `/mnt/us/.notebooks/` holding an unencrypted SQLite file `nbk` (tables `fragments`,
  `fragment_properties`, `local_delta_fragments`, `local_action_fragments`, `capabilities`; the
  stroke data likely Amazon Ion). Verify against real notebooks on the device before trusting any
  format notes; the etienneunai fork's converter assumes a 1024-byte header these files do not
  have. Credit the fork if any of its findings hold.

## In progress

- **Live view smoothness.** rc.10 replaced the playout (smooth through stalls); still to measure on
  the device with the owner. Ideas left: a 'Wi-Fi is stalling' hint when lateness stays high,
  decode/compose/average in a Worker, 4-bit tiles on a congested link. The intermittent lag measured so far was the Kindle's 2.4 GHz link
  stalling (5 GHz was clean). When it lags: check the band (`iw dev wlan0 link`) and a
  `ping -i 0.1` from the Kindle while it happens; the page's Latency readout shows playout pinned
  at 20 ms during bursts.

## Built, not installed

- **USB file access hardening** (outside this repo, `~/KindleLab/volume`, branch `rc8`, worktree
  `~/KindleLab/volume-rc8`): a Kindle-side controller that survives crashes and logs why it stops,
  never holds the power service awake, the Mac mount behaving like a drive (no `._` files, unmounts
  when the Kindle leaves), and a read-only guard on Grimoire's live files while it runs. Install
  with the owner present: `install_mac.sh`, then `install_usb_daemon.sh` on the Kindle.

## To investigate

- **The stroke eraser on very dense pages.** Taking a stroke out of the middle still draws what was
  under it again (undo and redo of it are instant since rc.10). On a 1,090-stroke page that can take
  seconds a sweep.
- **Brushes on the Kindle.** A dense page still takes about 30 s to draw the first time (marker 23 s,
  charcoal 10 s of it). Exact speedups left: a per-dab table for core pixels (no sqrt, no powf), the
  graphite path's per-pixel divisions.
- **Times past 2^32 ms (49.7 days of uptime).** A touch gesture crossing the wrap of the frame clock
  fails (found by a unit test); the 2^31 case is fixed in rc.10.
- **The KOReader plugin as a book (owner idea, 2026-10-31).** Installed as a file KOReader's library
  shows, removed by long-press > Delete like any book, instead of a plugin folder. Needs research on
  KOReader's file browser and a precedent before building.

- **Kindle Click (Amazon's Bluetooth page turner, 2026).** Grimoire accepts any input device that
  sends ordinary key codes, so it works if the firmware exposes it as an input device while
  Grimoire runs. Test: pair it in the Kindle's settings, list `/proc/bus/input/devices`, then try
  Settings > Controller > Buttons.

## Not taken from the RC8 review (revisit when there is room)

- Live view: decode, compose and average in a Worker with OffscreenCanvas; optional 4-bit gray
  tiles when the link is congested.
- Graph: find a page in the graph and Around history; show other notebooks as anchors.
- On-screen keyboard: Backspace auto-repeat.
- Remove the retired status-bar code (`status_reserved()` is always 0) and the graph's satellite
  and tray code unused since RC5 (about 250 lines in mapview.rs).
- A tree-wide rustfmt pass over leftovers of an earlier mechanical reformat.
- Wording left as is: "Somewhere Else" (the only Title Case label), and the RC1 changelog section
  that later RCs reversed.

## Product

- Send the fork outreach note (draft on the owner's Desktop) and keep the README's "Name, logo and
  forks" section current.
- Optional pay-what-you-want supporter path ($0 allowed); not set up.
- Desktop and Windows port: paused since 2026-10-26.
- Promote a quiet 2.0.0 release candidate to 2.0.0 stable (the stable KPM feed updates every 1.2.0
  install).

## Done

- 2.0.0-rc.11: What's new under the bat (look C, picked by the owner): the owner's note (`docs/WHATSNEW.md`)
  and hand signature with the bat (`assets/signature.pgm`, from the notebook "2.0b1 sig"); opens once after an update.
- 2.0.0-rc.10: instant undo and redo on any page; dense pages open from their kept drawing; the
  heartbeat goes on through long drawing; no background redraw of the open page; faster exact
  brushes on the Kindle; Live view playout smooth through Wi-Fi stalls; gestures past 24.9 days of
  uptime.

- 2.0.0-rc.9: one-finger page turns; pen taps in the Graph; papers in three rows with cross grid,
  isometric, hexagon and Seyes; Custom music 8-line tab, presets and new default; Smallest shape
  slider; Live view picture in picture for Firefox; the toolbar kept in Live view beside the tool
  picker.
- 2.0.0-rc.8: taps no longer swallowed near the pen off the page; sliders keep their value on a
  cancelled touch; Live view rail, size control and picture in picture tiers; steadier, faster
  graph; Pages and Graph untangled.
