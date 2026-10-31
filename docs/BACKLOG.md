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

- **Undo on dense pages (rc.10).** An undo redraws the whole region under the removed stroke: 4 to
  11 s on the Kindle on a 1,090-stroke page, until the launcher's watchdog killed the app
  (evidence `~/KindleLab/evidence/rc8-crash-20261031-0503/`). Fix: keep the pixels each stroke
  covered when it finished, so undo is a copy; a long redraw must also keep the watchdog fed.
- **Live view smoothness.** The intermittent lag measured so far was the Kindle's 2.4 GHz link
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

- 2.0.0-rc.9: one-finger page turns; pen taps in the Graph; papers in three rows with cross grid,
  isometric, hexagon and Seyes; Custom music 8-line tab, presets and new default; Smallest shape
  slider; Live view picture in picture for Firefox; the toolbar kept in Live view beside the tool
  picker.
- 2.0.0-rc.8: taps no longer swallowed near the pen off the page; sliders keep their value on a
  cancelled touch; Live view rail, size control and picture in picture tiers; steadier, faster
  graph; Pages and Graph untangled.
