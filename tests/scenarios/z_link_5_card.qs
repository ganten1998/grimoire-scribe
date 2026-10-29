# rust-only
# settings: status_on=0
# root: z_bar_0_make
# env: GRIMOIRE_CHECK_FIT=1
# The long-press card (PHASE2 8): a finger held on a link shows its heavy border by four DU strips, then the card
# beside it by one GL16, over the page (no Pages view, no full refresh). Rename names the target page in the Index
# and the card stays; a tap outside closes it and follows nothing; Change where it goes... is Link to... without
# Choose from the pages..., the current target marked; Redraw the area takes the next pen stroke as the new area;
# an unnamed target's Rename row adds the page to the Index; the pen on its rows works as a finger; a pen stroke
# outside closes it and draws nothing.
wait 300
tap 200 400
wait 300
expect editor.strokes is 0
expect display.presents not -1
touch down 600 1100
wait 800
expect editor.link_card is 1
expect editor.modal is card
expect editor.link_card.title is Árbol
expect editor.link_card.rows is rename retarget redraw delete
expect editor.link_card.border is 1
expect editor.link_card.links is 1
expect editor.link_card.rect is 289 1235 780 666
expect display.presents has fast 289 989 622 11, fast 289 1200 622 11, fast 289 1000 11 200, fast 900 1000 11 200, ui 289 1235 780 666
touch up 600 1100
expect editor.page is 1
expect editor.link_card is 1
shot 01-card
# Rename: its keyboard at the bottom (the border is high on the page), the name there to change
expect editor.ui_at.679.1428 is card.rename
tap 679 1428
wait 100
expect editor.keyboard is Name in the Index
expect editor.text is Árbol
expect editor.kb_dock is bottom
expect editor.link_card is 1
expect editor.ui_at.679.1428 is
shot 02-rename
# five dels, then shift, c, a, p, and Done
tap 1679 2205
tap 1679 2205
tap 1679 2205
tap 1679 2205
tap 1679 2205
tap 175 2205
tap 747 2205
tap 221 2045
tap 1726 1885
expect editor.text is Cap
tap 1643 2365
wait 100
expect toast is Page 3 is in the Index as Cap.
expect editor.link_card.title is Cap
expect editor.link_card is 1
expect editor.keyboard is
# A tap outside: the card closes, and the link under the tap is not followed
tap 1200 1900
expect editor.link_card is 0
expect editor.link_card.border is 0
expect editor.page is 1
# The clock corner keeps its tap with the card up (rule 12): a tap there toggles the clock and the card stays
touch down 600 1100
wait 800
touch up 600 1100
expect editor.link_card is 1
expect status.front is 1
tap 60 60
wait 100
expect settings.status_on is 1
expect editor.link_card is 1
tap 60 60
wait 100
expect settings.status_on is 0
expect editor.link_card is 1
tap 1200 1900
expect editor.link_card is 0
# Change where it goes...: Link to... without Choose from the pages..., Cap marked; The Index
touch down 600 1100
wait 800
touch up 600 1100
expect editor.ui_at.679.1558 is card.retarget
tap 679 1558
wait 100
expect editor.linkto is pick
expect editor.modal is linkto
expect editor.link_card is 0
# and with Link to... up: the clock toggles, nothing is cancelled
tap 60 60
wait 100
expect settings.status_on is 1
expect editor.linkto is pick
tap 60 60
wait 100
expect settings.status_on is 0
expect editor.linkto is pick
expect editor.link_card.border is 1
expect editor.ui_at.930.2370 is
expect editor.ui_at.480.2370 is linkto.search
expect editor.ui_at.1380.2370 is linkto.cancel
expect editor.linkto.3 is p3 current
shot 03-retarget
expect editor.ui_at.1381.2078 is linkto.chip.2
tap 1381 2078
wait 100
expect toast is Linked to the Index.
expect editor.linkto is 0
expect editor.link_card.border is 0
expect editor.undo_kind is link
expect editor.links is 3 3
pen leave
wait 1100
tap 600 1100
expect editor.modal is pages
expect editor.pages_tab is index
tap 125 95
expect editor.modal is none
pentap panel undo
wait 300
expect toast is Link change undone.
pen leave
wait 1100
tap 600 1100
expect editor.page is 3
pentap panel previous
pentap panel previous
expect editor.page is 1
# Change where it goes..., then Cancel: nothing changes, and no toast
pen leave
wait 1100
touch down 600 1100
wait 800
touch up 600 1100
tap 679 1558
wait 100
tap 1380 2370
wait 100
expect editor.linkto is 0
expect editor.link_card.border is 0
expect toast is
# An unnamed target (page 2): Add page 2 to the Index...; named Two
touch down 600 1450
wait 800
touch up 600 1450
expect editor.link_card.title is Page 2
expect editor.link_card.rows is rename retarget redraw delete
expect editor.link_card.rect is 289 1535 780 666
shot 04-card-unnamed
# low on the screen, so its keyboard docks at the top (the keys 1307 px higher): shift, t, w, o, Done
expect editor.ui_at.679.1728 is card.rename
tap 679 1728
wait 100
expect editor.keyboard is Name in the Index
expect editor.text is
expect editor.kb_dock is top
tap 175 898
tap 841 578
tap 310 578
tap 1549 578
expect editor.text is Two
tap 1643 1058
wait 100
expect toast is Page 2 is in the Index as Two.
expect editor.index_rows is 3
expect editor.link_card.title is Two
tap 1200 600
expect editor.link_card is 0
# Redraw the area: the strip asks for it, the old border stays until the new area is drawn
touch down 600 1450
wait 800
touch up 600 1450
expect editor.ui_at.679.1988 is card.redraw
tap 679 1988
wait 100
expect editor.link_card is 0
expect editor.strip is redraw
expect editor.link_card.border is 1
shot 05-redraw
pen line 1000 1000 1400 1100 30 0.6 0.6
expect editor.strip is 0
expect editor.link_card.border is 0
expect toast is New area saved. Undo brings the old one back.
expect editor.strokes is 0
expect editor.links is 3 3
expect editor.undo_kind is link
# (links.ini lists a page's links by seed, and seeds are made from the clock: the areas are checked by where a
# finger's tap follows, not by editor.link.N)
pen leave
wait 1100
tap 600 1450
expect editor.page is 1
tap 1200 1050
expect editor.page is 2
pentap panel previous
expect editor.page is 1
pentap panel undo
wait 300
expect toast is Link change undone.
pen leave
wait 1100
tap 1200 1050
expect editor.page is 1
tap 600 1450
expect editor.page is 2
pentap panel previous
expect editor.page is 1
# The pen on the card's rows: Delete, then Undo
pen leave
wait 1100
touch down 600 1450
wait 800
touch up 600 1450
pentap 679 2118
expect editor.links is 2 2
expect toast is Link deleted. Undo brings it back.
pentap panel undo
wait 300
expect editor.links is 3 3
# Where the card goes: links near the right edge, at the bottom and at the top (each to the Index)
key 38 0
pen line 1250 600 1600 700 30 0.6 0.6
tap 1381 2078
wait 100
expect editor.links is 4 4
key 38 0
pen line 300 2250 900 2350 30 0.6 0.6
expect editor.linkto.dock is top
expect editor.linkto.rect is 20 173 1820 736
expect editor.ui_at.1381.387 is linkto.chip.2
tap 1381 387
wait 100
key 38 0
pen line 600 250 1200 350 30 0.6 0.6
tap 1381 2078
wait 100
expect editor.links is 6 6
pen leave
wait 1100
touch down 1425 650
wait 800
touch up 1425 650
expect editor.link_card.rect is 435 589 780 666
shot 06-card-right
tap 100 1600
expect editor.link_card is 0
touch down 600 2300
wait 800
touch up 600 2300
expect editor.link_card.rect is 935 1924 780 536
shot 07-card-bottom
tap 1200 1000
expect editor.link_card is 0
touch down 900 300
wait 800
touch up 900 300
expect editor.link_card.rect is 589 385 780 666
shot 08-card-top
tap 1200 1600
expect editor.link_card is 0
rotate 1
wait 300
expect editor.link_card is 0
# sideways (the whole page, 0.75 of its size): beside a link high on the page, and above one low on it
touch down 992 825
wait 800
touch up 992 825
expect editor.link_card is 1
expect editor.link_card.rect is 757 935 780 666
shot 09-card-sideways
tap 2200 1700
expect editor.link_card is 0
touch down 992 1725
wait 800
touch up 992 1725
expect editor.link_card is 1
expect editor.link_card.rect is 757 1117 780 536
shot 10-card-sideways-low
tap 2200 300
rotate 0
wait 300
# A pen stroke outside closes the card and draws nothing
touch down 600 1100
wait 800
touch up 600 1100
expect editor.link_card is 1
pen line 1300 1600 1500 1800 30 0.6 0.6
expect editor.link_card is 0
expect editor.strokes is 0
expect display.ink is 0
