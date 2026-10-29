# settings: status_mode=1
# rust-only
# The pen wins over a finger on a link: its stroke starts (it used to be dropped as one gesture at a
# time), and the finger's session ends with no follow when it lifts.
# settings: shortcuts_enabled=1
# settings: mirror_pdf=0
# copy: tests/fixtures/link-keys .
wait 300
tap 965 95
wait 100
tap 1564 2254
wait 200
pentap panel next
wait 100
pentap panel previous
wait 100
expect editor.pages is 2
key 38 0
pen line 300 1000 900 1200 30 0.6 0.6
# Link to... opens: Choose from the pages... picks in the Pages view
expect editor.ui_at.930.2370 is linkto.pages
tap 930 2370
expect editor.picking is new
tap 703 340
expect editor.links is 1 1
# (the lasso that made the link left no ink)
expect editor.strokes is 0
tap 600 1500
expect editor.page is 1
# a finger rests on the link, the pen writes elsewhere, the finger lifts
touch down 600 1100
pen line 300 1700 900 1800 30 0.6 0.6
touch up 600 1100
expect editor.strokes is 1
expect editor.page is 1
# the same finger without the pen follows it
tap 600 1100
expect editor.page is 2
