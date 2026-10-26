# A queued burst must appear on screen before pen-up, retaining every sample. The old
# unbounded main loop processed all 258 events before a single display refresh.
wait 300
tap 965 95
wait 200
tap 510 460
wait 100
tap 1564 2254
wait 400
expect screen is editor
expect display.ink_frames is 0
drag pen 300 700 1500 900 256
expect editor.strokes is 1
expect editor.stroke_points is 257
expect display.ink_frames not 0
expect display.ink_frames not 1
expect display.ink_frames not 2
expect display.ink_frames not 3
expect editor.live_matches_render is 1
wait 700
expect display.ink is 0
shot ink-backlog
