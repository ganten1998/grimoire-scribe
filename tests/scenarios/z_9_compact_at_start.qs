# The compaction z_8_left_open left (it quit with the notebook open) runs at this start, in the
# background, before the notebook is opened: its page file has lost the erased line and the pass
# over it.
# root: z_8_left_open
# rust-only
waitbg
expect bg.busy is 0
tap 326 608
wait 500
expect screen is editor
expect editor.strokes is 1
expect editor.erase_strokes is 0
expect editor.live_matches_render is 1
shot 01-compacted-at-start
pentap panel library
wait 300
expect screen is library
expect bg.busy is 0
quit
