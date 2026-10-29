# rust-only
# root: z_bar_0_make
# copy: tests/fixtures/shapes .
wait 300
tap 200 400
expect settings.shapes is 1
expect editor.shape_preview is 0
key 31 0
expect settings.shapes is 0
key 35 0
expect settings.shape_hold_ms is 900
pen down 500 900 0.6
wait 100
pen move 1100 900 0.6
wait 100
pen move 1100 1400 0.6
wait 100
pen move 500 1400 0.6
wait 100
pen move 500 900 0.6
pen up 500 900
pen leave
wait 1000
expect editor.strokes is 1
expect editor.stroke_points is 5
key 47 0
expect editor.undo_kind is replace
expect editor.shape_snaps is 1
expect editor.strokes is 1
expect editor.live_matches_render is 1
shot 01-shape-last
tap panel undo
expect editor.stroke_points is 5
expect editor.live_matches_render is 1
tap panel redo
expect editor.undo_kind is replace
expect editor.live_matches_render is 1
key 50 0
expect editor.map is graph
key 50 0
expect editor.map is 0
