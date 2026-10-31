# Lane touch: merge notes

Branch `claude/rc8-touch`, from `codex/rc7-release-sweep` 6c5357a (2.0.0-rc.7). The owner's concern:
buttons always work and are not wrongly rejected, sliders and the keyboard included.

## What changed (one commit each, in order)

1. **T2, cancelled slider** (`dispmenu.rs`, `pensettings.rs`, `editor.rs`, `library.rs`). A touch
   the recognizer cancels (the pen coming near, a late second finger, a palm) ends with a TouchUp
   at x = y = -10000. The Display sheet's brightness, warmth and Sleep after sliders read that as a
   value (light 0, Sleep after "never", saved); Pen feel's sliders moved by the fine-motion gain.
   New `dispmenu_cancel` keeps, applies and saves the current value; Pen feel repaints and saves
   without reading the cancel. The audit found one more consumer with the same bug: a panel drag
   that is cancelled jumped into the screen's corner. It now stays where it was last drawn. (Map
   view already restores its drag on a cancel; every other press consumer calls press_move with
   the cancel's place, so no tap fires.)
2. **T1 + T3 + T9, recognizer** (`touchgest.rs`, `editor.rs` touch_context, `library.rs`,
   `bin/tgstudy.rs`). OWNER APPROVED T1.
   - `TgView` gains `ink` (the notebook's page in front with no sheet, keyboard, bar, picker or
     menu; set only by the editor), `controls: [Rect; 2]` (the panel, and the link strip while
     linking) and `keys` (the open keyboard's area).
   - One finger is held as the writing hand only when it lands on the ink and off the controls
     (`pen_hand`). There it still counts when far from the pen by the session rules: a fresh hover
     sample >= hover_far_mm (60) away, or the pen out of proximity >= pen_gone_ms (300) and the
     finger >= far_mm (40) from pen.last. A live press is cancelled by the pen only when the pen
     touches or a fresh hover sample is near it (`pen_reaches`; with hovers unread, as in
     TG_TUNE_V1, the landing rule again). The resting-palm rule (Ph::Wait) is untouched (T4 was
     refuted).
   - The pen's ARRIVAL still cancels whatever finger is down (input_evdev.rs / sim_sdl.rs
     `touchgest_cancel`), as before; T2 makes that harmless for sliders.
   - T3: with a finger down on the keys, a second finger landing on the keys is the next key: the
     first gets its TouchUp where it is and waits to lift (locked), the second gets a TouchDown.
     Only on the keys, so a press on a Find row taken over by a second finger is still cancelled
     (z_bar_3b). The Library now pushes keyboard_open and keys (`library_touch_view`, pushed after
     each Library event and at enter; the editor's Library-settings branch uses it too).
   - T9: a single finger's final movement reported with its tracking-ID release now gives a
     TouchMove there before the TouchUp.
   - tgstudy sets `ink: true` (its traces are of the page), so its replay is unchanged.
3. **T5 + T7, taps** (`ui.rs`, `status.rs`, `linkbar.rs`, `editor.rs`). `press_is_tap` is the
   lenient rule the panel and Library already used: begins and ends in the hit's rect, however far
   it rolled between (`moved` is kept for drags and holds). The shared 30 px slop is NOT raised (the
   skeptic's caution: it also gates drags). A finger tap on a page link may roll 5 mm
   (`TAP_SLOP_MM`, now shared with the clock corner's `SLOP_MM`). Links bar: only a List chip treats
   a press past 600 ms as no tap; X, Prev/Next, Areas, Go to, Back act on a slow press.
4. **T8, pressed look** (`app.rs` new `app_press_release`, `dispmenu.rs`, `tplsheet.rs`,
   `pagesview.rs`, `editor.rs` tool picker). The Display sheet's buttons and rows, the Templates
   sheet, the Pages view's controls (not its tiles: their tap redraws at once) and the tool picker
   now invert on press with app_press_feedback, and take it off at the lift before acting. Each
   surface's draw forgets the press, so a redraw in between never leaves an inverted button.
   Not done: Map view (Graph) has no pressed look yet (its drag and selection redraws need care).
5. **Proposal, panel tiling** (`editor.rs`). draw_panel records every button it draws
   (`panel_cells`, greyed out too). `panel_hit_at`: a finger landing in the panel but outside every
   button (gap, divider, frame margin) takes the nearest button (rows first). A greyed-out button
   still takes nothing; hit rects and the pressed look are unchanged. Used by ui_pointer (landing
   and lift) and the `editor.panel_at` probe.
6. **OWNER REPORT, toolbar top padding** (`editor.rs` panel_rect_at, `panel.rs`). The 12 px
   `PANEL_MARGIN` was applied as a position clamp above and below the panel. It is gone from the
   clamp: the panel reaches y 0 and the bottom edge, upright and sideways (the sides had no margin).
   `panel_fit` still sizes the panel with that room, so no button changes size. The Live view
   panel layer follows the rect as before (nothing margin-specific there). `editor.panel_fits` now
   checks inside the screen.
7. A test-harness commit: `test_sheets_probe` opens the display (needed by the new pressed look
   under the sim feature).

## User-facing strings

None added or changed. No po/ edits needed.

## Tests run

- `cargo test` (host) and `cargo test --features sim`, both `--test-threads=1` as `make test`
  does: all pass (after the commits above).
- New unit tests in `rust/tests/test_touchgest.rs`: `the_pen_window_is_the_inks_alone`,
  `one_finger_far_from_the_pen`, `one_finger_lifts_where_it_ended`,
  `keyboard_roll_over_types_both_keys`. Existing ones adjusted where they assumed the old one-finger
  rule (the rig now models the page: `ink: true`; two pen-last points placed near the finger).
  `ui.rs` / `test_ui.rs` press tests updated to the lenient rule (plus a cancel case).
  `editor.rs` panel test and `sim_sdl.rs` pen-lines test updated.
- New scenarios, each failing on RC7 (checked against a base build of 6c5357a) and passing now:
  - `tests/scenarios/z_touch_rejections.qs` (the proposal's touch-rejection suite through
    touchgest): Library tap with a far hover and 600 ms after a leave; keyboard roll-over "as";
    Pen feel slider kept through a late second finger; panel tap with the pen hovering 20 mm away;
    Display brightness kept through the pen's arrival; on the ink a near finger held, a far one let
    through.
  - `tests/scenarios/z_touch_taps.qs`: a link followed after a 40 px roll; a slow press on the
    Links bar X; a 40 px rolled tap on Invert colors; Done's pressed look; panel gap/margin taps.
  - `tests/scenarios/z_toolbar_edges.qs`: panel dragged to y 0 and the bottom edge upright and
    sideways, taps there, and a cancelled drag staying put.
- Retargeted (legitimately, the 12 px bottom clamp is gone, so a panel pinned to the bottom sits
  12 px lower): `y_menus_over_panel`, `z_sideways_toolbar`, `z_tool_2_pins`,
  `z_tool_7_pins_colorsoft`, `z_tool_d_picker_middle`. `s_fit_1`: the Display row's tap now
  presents its pressed look (fast row rect) before the card's Ui.
- Scenario sweep: see the results section at the end.

## Left undone, and why

- The pen's arrival (entering proximity) still cancels a finger already down anywhere, including on
  the panel or a slider (input_evdev.rs / sim_sdl.rs). Changing it touches the "PenDown never meets
  a pinch" rule in the readers; T2 makes the slider case harmless. A finger already on a toolbar
  button as the pen comes into range loses that tap.
- Map view (Graph) pressed look (T8) not added.
- The clock corner is part of the ink surface (as fs_can_arm says); a corner tap right after writing
  near it is still held unless the far rules let it through.
- status.rs clears its pen_near on any TouchDown it sees ("a finger let through is not the writing
  hand"); off the ink a finger is now let through with the pen near. The next hover sample sets it
  again, so at most one corner refresh may go out early. Left as is.
- No CHANGELOG entry (the integrator writes the RC8 notes).

## Owner-facing test steps (on the Scribe)

1. Write a stroke, then at once tap Undo on the toolbar with a finger: it undoes the first time.
   Same with the pen hovering near the toolbar.
2. In the Library, hold the pen over the page and tap New folder (or any button) with a finger: it
   opens the first time. Same right after lifting the pen away.
3. On the page, rest the writing hand while writing: no links followed, nothing pressed (unchanged).
   With the pen hovering at one side, a finger tap on a link far away (6 cm or more) follows it.
4. Display sheet: drag brightness, then bring the pen near while still dragging: the light stays
   where you dragged it (it used to go dark). Same with Sleep after (it used to become "never").
5. Type a name on the keyboard quickly with two fingers or thumbs so presses overlap: no letters
   go missing.
6. Tap a toolbar button slightly off its edge, between two buttons or at the panel's border: the
   nearest button acts. Greyed-out buttons still do nothing.
7. Press firmly and roll a little on a sheet button (Display, Pages controls, Templates): it acts,
   and shows pressed at once.
8. Links bar: press X (or Prev/Next) slowly, about a second: it acts.
9. Drag the toolbar by its grip to the very top of the screen: it touches the top edge (no gap),
   and to the bottom: it touches the bottom edge. Sideways too. Its buttons work there.
