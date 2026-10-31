# settings: status_mode=1
# rust-only
# A lift that never came (on the Kindle, lost from a full input queue while the page was busy) no longer
# swallows the next contact of that input: a down from the input that still owns a gesture ends that
# gesture as a cancel (grimoire.log: "its lift was lost") and starts its own. A finger whose lift was lost
# follows nothing, and the next finger's tap on a link follows it; a stroke whose lift was lost is kept and
# the next is a stroke of its own; a lasso whose lift was lost goes back to the strip, and the next lasso
# alone makes the area. (Before, the second down was ignored, one gesture at a time, and its moves and lift
# went on with the old gesture: the tap followed nothing, the strokes were one, the lasso's box both.)
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
expect editor.page is 1
key 38 0
pen line 300 1000 900 1200 30 0.6 0.6
pen leave
expect editor.ui_at.930.2370 is linkto.pages
tap 930 2370
wait 100
tap 703 340
wait 200
expect editor.links is 1 1
expect editor.owner is none
# a finger on the page, its lift lost; the next finger taps the link
touch down 1100 1700
expect editor.owner is link finger
touch down 600 1100
expect editor.owner is link finger
touch up 600 1100
wait 300
expect editor.page is 2
expect editor.owner is none
tap panel previous
wait 300
expect editor.page is 1
# a stroke, its lift lost; the next stroke
pen down 300 1700 0.6
pen move 700 1750 0.6
expect editor.owner is draw pen
pen down 300 1900 0.6
pen move 700 1950 0.6
pen up 700 1950
pen leave
wait 200
expect editor.strokes is 2
expect editor.owner is none
# a lasso, its lift lost: back to the strip; the next lasso makes the area alone
key 38 0
pen down 300 1300
pen move 900 1300
pen move 900 1500
expect editor.owner is lasso pen
pen down 300 1700
expect editor.strip is new
expect editor.owner is lasso pen
pen move 900 1700
pen move 900 2000
pen move 300 2000
pen up 300 1700
pen leave
wait 200
expect editor.linkto is pick
expect editor.link_area is 300 1700 600 300
expect editor.strokes is 2
expect editor.owner is none
quit
