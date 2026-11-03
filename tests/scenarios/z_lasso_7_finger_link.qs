# rust-only
# settings: status_on=0
# settings: gesture_new_page=0
# root: z_bar_0_make
# The finger hold that arms the lasso (z_lasso_6) leaves every other finger as it was: held on a link, a
# finger opens the link's card and never a lasso; a tap on a link follows it; one finger flicked sideways and
# two fingers swiped turn the page; a finger that moves before the hold time arms nothing. Held off a link
# with an empty clipboard and lifted where it was, a finger does nothing at all (the ring it showed goes,
# no toast, no undo step).
wait 300
tap 200 400
wait 300
expect screen is editor
expect editor.page is 1
expect editor.strokes is 0
# held on a link: its card, never the lasso
touch down 600 1100
wait 800
expect editor.link_card is 1
expect editor.modal is card
expect editor.select is off
touch up 600 1100
wait 100
expect editor.link_card is 1
expect editor.select is off
tap 1300 700
wait 100
expect editor.link_card is 0
expect editor.page is 1
# a tap on the link follows it
tap 600 1100
wait 300
expect editor.page is 3
expect editor.select is off
# one finger flicked right, then left, turns the page; two fingers too
fpath 500 1500 1400 1480 250
wait 400
expect editor.page is 2
fpath 1400 1500 500 1520 250
wait 400
expect editor.page is 3
swipe 2 1300 1200 -400 0
wait 400
expect editor.page is 4
swipe 2 600 1200 400 0
wait 400
expect editor.page is 3
expect editor.select is off
# moved before the hold time: no lasso, however long it stays down after
touch down 1300 700
touch move 1300 900
wait 800
expect editor.select is off
expect editor.owner is link finger
touch up 1300 900
wait 100
expect editor.owner is none
expect editor.select is off
# held off a link, no clipboard: the lasso arms, and lifted where it was nothing happens
expect editor.clipboard is 0
touch down 1300 700
wait 800
expect editor.select is lasso
touch up 1300 700
wait 100
expect editor.select is off
expect editor.owner is none
expect editor.strokes is 0
expect editor.undo_kind is
expect toast is
expect editor.page is 3
quit
