# rust-only
# generate: tests/lasso_dense_fixture.py
# A move and a paste on a 1,000-stroke page (docs/beta2/LASSO-DESIGN.md 8): the times are in grimoire.log
# ("lasso: move n=... src_ms=... dst_ms=..." and "lasso: paste ..."), and the page drawn live equals a
# fresh drawing after each, undo and redo included.
wait 300
tap 326 608
wait 2000
expect screen is editor
expect editor.strokes is 1000
tap panel tool-picker
tap panel picker.select
wait 100
expect editor.select is armed
pen down 700 1000
pen move 1000 990
pen move 1300 1000
pen move 1310 1200
pen move 1300 1400
pen move 1000 1410
pen move 700 1400
pen move 690 1200
pen up 700 1010
wait 100
expect editor.select is selected
pen down 1000 1200
pen move 1000 1400
pen move 1100 1600
pen up 1100 1700
wait 100
expect editor.undo_kind is lasso
expect editor.live_matches_render is 1
tap panel select.copy
wait 100
tap panel select.done
pentap panel undo
wait 100
expect editor.live_matches_render is 1
pentap panel redo
wait 100
expect editor.live_matches_render is 1
tap panel tool-picker
tap panel picker.select
wait 100
tap panel select.paste
wait 100
expect editor.undo_kind is lasso
expect editor.live_matches_render is 1
pentap panel undo
wait 100
expect editor.live_matches_render is 1
quit
