# The stroke eraser (eraser_mode=0, as before 1.1) on the page z_4_erase_zoom_rotate left, where
# erase strokes cross the ink: it takes the whole ink strokes it touches and never an erase stroke
# (taking one away would bring back the ink it covers); undo puts the ink back; live equals a fresh
# render.
# root: z_4_erase_zoom_rotate
# settings: eraser_mode=0
# settings: landscape_fit=0
# rust-only
wait 300
expect settings.eraser_mode is 0
tap 326 608
wait 500
expect screen is editor
expect editor.strokes is 5
expect editor.erase_strokes is 3
# The eraser end along the first erase stroke's path: both ink strokes go, the erase strokes stay
eraser line 380 500 460 690 12
wait 700
shot 01-ink-gone-erase-strokes-kept
expect editor.strokes is 3
expect editor.erase_strokes is 3
expect editor.live_matches_render is 1
pentap panel undo
wait 300
expect editor.strokes is 5
expect editor.live_matches_render is 1
pentap panel library
wait 500
expect screen is library
quit
