# settings: status_mode=1
# rust-only
# Making a link, then the fingers (the owner's rc.11 report: no finger input after a link until a sleep and
# wake; the strip and a half-drawn lasso staying up). Fingers here go through touchgest (finger, frame and
# fpath), as the Kindle's do, with the pen hovering as it does while a link is made. A link picked with
# fingers leaves nothing owned and the fingers turning pages; with the pen still near, a finger on the page
# is the pen's writing hand (touch.hand), by design, and the same tap follows the link once the pen has gone.
# A held lasso that snaps, a lasso a tap outside cancels, and a lasso the pen's button takes over for a drag
# (it goes back to the strip: before, it stayed half drawn and the next stroke was ink) each leave the
# fingers working.
# settings: shortcuts_enabled=1
# settings: mirror_pdf=0
# settings: gesture_new_page=0
# copy: tests/fixtures/link-keys .
# copy: tests/fixtures/penbutton/pan .
wait 300
tap 965 95
wait 100
tap 1564 2254
wait 200
pentap panel next
wait 100
pentap panel previous
wait 100
expect editor.page is 1
expect editor.pages is 2
# one finger turns the pages before any link
wait 1100
fpath 1400 1500 500 1520 250
wait 400
expect editor.page is 2
fpath 500 1500 1400 1480 250
wait 400
expect editor.page is 1
# A: the lasso with the pen hovering before and after it; fingers pick Choose from the pages... and page 2
key 38 0
expect editor.strip is new
pen hover 300 1100
pen down 300 1000
pen move 900 1000
pen move 900 1200
pen move 300 1200
pen move 305 1010
pen up 305 1010
pen hover 320 1050
wait 200
expect editor.owner is none
expect editor.linkto is pick
expect editor.ui_at.930.2370 is linkto.pages
finger 1 down 930 2370
frame 60
finger 1 up 930 2370
frame
wait 200
expect editor.picking is new
finger 1 down 703 340
frame 60
finger 1 up 703 340
frame
wait 300
expect editor.links is 1 1
expect editor.linkto is 0
expect editor.owner is none
# the pen still near the page: a finger on the link is the pen's hand, and nothing follows
pen hover 320 1050
finger 1 down 600 1100
frame 60
finger 1 up 600 1100
frame
wait 300
expect touch.hand is near
expect editor.page is 1
expect editor.owner is none
# the pen gone: the same tap follows the link, and a swipe comes back
pen leave
wait 1100
finger 1 down 600 1100
frame 60
finger 1 up 600 1100
frame
wait 300
expect touch.hand is
expect editor.page is 2
fpath 500 1500 1400 1480 250
wait 400
expect editor.page is 1
# B: a lasso held still to snap a rectangle, picked from Link to...'s recent pages (docked at the top)
key 38 0
pen down 300 1700
pen move 900 1700
pen move 900 2000
pen move 300 2000
pen move 305 1710
wait 900
pen up 305 1710
wait 100
pen leave
wait 200
expect editor.linkto.3 is recent p2
expect editor.ui_at.479.527 is linkto.chip.3
tap 479 527
wait 200
expect editor.links is 2 2
expect editor.owner is none
wait 1100
fpath 1400 1500 500 1520 250
wait 400
expect editor.page is 2
fpath 500 1500 1400 1480 250
wait 400
expect editor.page is 1
finger 1 down 600 1850
frame 60
finger 1 up 600 1850
frame
wait 300
expect editor.page is 2
tap panel previous
wait 300
expect editor.page is 1
# C: a lasso, then a finger tap outside Link to...: no link, and the fingers still turn pages
key 38 0
pen line 300 1300 900 1500 30 0.6 0.6
pen leave
wait 200
expect editor.linkto is pick
tap 1100 900
wait 200
expect toast is No link made.
expect editor.links is 2 2
expect editor.owner is none
wait 1100
fpath 1400 1500 500 1520 250
wait 400
expect editor.page is 2
fpath 500 1500 1400 1480 250
wait 400
expect editor.page is 1
# D: the pen's button pressed in the middle of the lasso takes it over for a drag (bound to Pan): the
# lasso goes, the strip stays for another, and the next stroke is the lasso again, never ink
key 38 0
pen hover 300 1000
pen down 300 1000
pen move 900 1000
pen move 900 1200
side on
pen move 300 1200
expect editor.lasso is 0
expect editor.strip is new
pen move 305 1010
pen up 305 1010
side off
pen leave
wait 200
expect editor.owner is none
expect editor.strip is new
expect editor.strokes is 0
pen line 300 1300 900 1500 30 0.6 0.6
pen leave
wait 200
expect editor.strokes is 0
expect editor.strip is 0
expect editor.linkto is pick
tap 1100 900
wait 200
expect toast is No link made.
wait 1100
fpath 1400 1500 500 1520 250
wait 400
expect editor.page is 2
quit
