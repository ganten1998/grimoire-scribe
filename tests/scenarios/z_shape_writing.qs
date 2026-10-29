# rust-only
# root: z_bar_0_make
wait 300
tap 200 400
expect settings.shapes is 1
expect editor.shape_snaps is 0
pen strokes assets/eraser-sample.qpg 32 700
pen leave
wait 1000
expect editor.shape_snaps is 0
expect editor.strokes is 32
expect editor.live_matches_render is 1
shot 01-handwriting
