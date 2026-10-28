# settings: status_mode=1
# rust-only
# Sideways, show the whole page: the Display sheet's row turns it off and on (a Full refresh when
# sideways, only a toast upright), the page rests whole and centred until it is zoomed or moved, and a
# turn lands on it.
# settings: shortcuts_enabled=1
# settings: mirror_pdf=0
# copy: tests/fixtures/index-keys .
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
rotate 1
wait 1200
expect editor.zoom is 1.00
expect editor.scale is 0.75
expect editor.zoomed is 0
# two fingers swipe: Pages opens and closes
swipe 2 1240 700 0 400
expect editor.modal is pages
swipe 2 1240 1300 0 -400
expect editor.modal is none
# pinched in it is zoomed, and a page turn rests it again
pinch 1240 930 300 900 6
wait 400
expect editor.zoomed is 1
pentap panel next
wait 300
expect editor.zoomed is 0
expect editor.page is 2
pentap panel previous
wait 300
# pinched in, fit-page puts it back whole
pinch 1240 930 300 900 6
wait 400
expect editor.zoomed is 1
key 33 0
wait 300
expect editor.zoomed is 0
expect editor.scale is 0.75
# landscape-fit (a key or a controller's button) is the Display row: the width at once, even pinched in
pinch 1240 930 300 900 6
wait 400
key 17 0
wait 400
expect settings.landscape_fit is 0
expect editor.scale is 1.33
expect editor.zoomed is 0
expect toast is Sideways fills the width.
key 17 0
wait 400
expect settings.landscape_fit is 1
expect editor.scale is 0.75
expect toast is Sideways shows the whole page.
# the Display sheet's row: the width again, at once, even pinched in
pinch 1240 930 300 900 6
wait 400
pentap panel display
wait 300
expect editor.modal is display
expect display.presents not clear
tap 1240 779
wait 400
expect settings.landscape_fit is 0
expect editor.zoom is 1.00
expect editor.scale is 1.33
expect editor.zoomed is 0
expect toast is Sideways fills the width.
expect display.presents has full
# and the whole page again
tap 1240 779
wait 400
expect settings.landscape_fit is 1
expect editor.scale is 0.75
expect editor.zoomed is 0
expect toast is Sideways shows the whole page.
# off again, left off for the next run, which starts the app over
tap 1240 779
wait 400
expect settings.landscape_fit is 0
tap 1462 1860
wait 300
expect editor.modal is none
# upright the setting applies to a later turn only: the toast and the card's new checkbox, and no refresh
# of the page
rotate 0
wait 1200
expect editor.scale is 1.00
pentap panel display
wait 300
expect display.presents not clear
tap 930 986
wait 300
expect settings.landscape_fit is 1
expect toast is Applies when the Kindle is sideways.
expect display.presents is ui 180 326 1500 2034
tap 930 986
wait 300
expect settings.landscape_fit is 0
expect toast is Applies when the Kindle is sideways.
expect display.presents is ui 180 326 1500 2034
