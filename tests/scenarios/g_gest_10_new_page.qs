# rust-only
# env: TZ=UTC
# settings: mirror_pdf=0
# settings: shortcuts_enabled=1
# Two fingers left on the last page add a page, by default, as the next-page key does (2.0.0-beta.5 first said a
# gesture never adds one; the owner asked for the reverse). settings.ini gesture_new_page, a row of the Display
# sheet under Sideways and the gesture-new-page action turn it off: then the swipe only says it is the last page.
wait 300
tap 965 95
wait 100
tap 1564 2254
wait 200
expect screen is editor
expect settings.gesture_new_page is 1
expect editor.pages is 1
wait 1100
swipe 2 1300 1200 -400 0
expect editor.pages is 2
expect editor.page is 2
wait 1700
swipe 2 1300 1200 -400 0
expect editor.pages is 3
expect editor.page is 3
# the Display sheet's row turns it off, and on again
pentap panel display
wait 300
expect editor.modal is display
tap 500 1076
wait 300
expect settings.gesture_new_page is 0
shot 01-display-row-off
tap 500 1076
wait 300
expect settings.gesture_new_page is 1
tap 500 1076
wait 300
expect settings.gesture_new_page is 0
pentap panel display
wait 300
expect editor.modal is none
# off: nothing is added, and the toast says where it stopped
wait 1100
swipe 2 1300 1200 -400 0
expect toast is This is the last page.
expect editor.pages is 3
expect editor.page is 3
# a key's next page still adds one
key 106 0
wait 300
expect editor.pages is 4
quit
