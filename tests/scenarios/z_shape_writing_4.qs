# rust-only
# root: z_bar_0_make
# The maintainer's handwriting (assets/eraser-sample.qpg) through Smart shapes' hold, part 4 of 4:
# strokes 23 to 32 of its 32, each held still 700 ms before its lift, as written (real time: a
# pause inside a stroke fires the hold there too, as on the Kindle). None may snap. The four parts
# are one replay cut in four so that they run at once: with a 700 ms hold no stroke starts within
# SHAPE_BURST_MS of the last one's last sample, so none is in a writing burst and a stroke's hold does
# not depend on the strokes before it (each part starts from the same page and settings).
wait 300
tap 200 400
expect settings.shapes is 1
expect editor.shape_snaps is 0
pen strokes assets/eraser-sample.qpg 10 700 22
pen leave
wait 1000
expect editor.shape_snaps is 0
expect editor.strokes is 10
expect editor.live_matches_render is 1
shot 01-handwriting
