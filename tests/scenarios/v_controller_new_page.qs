# rust-only
# settings: shortcuts_enabled=1
# settings: mirror_pdf=0
# The last Pages action stays separate from sideways fit and Done in both orientations. Learn, replace,
# forget and relearn it sideways, then use the saved button to control actual two-finger page turns.
# This scenario also runs with GRIMOIRE_CHECK_FIT=1 in English/Spanish and Gothic/Classic.
wait 150
tap 1535 95
wait 100
tap library controller
wait 100
tap 930 1215
wait 100
tap library buttons.group.pages
wait 100
expect library.buttons_group is pages
tap 930 1536
wait 100
expect library.learning is landscape-fit
tap 930 1536
wait 100
tap 930 1696
wait 100
expect library.learning is gesture-new-page
shot 01-upright-learning
key 324 0 1
wait 100
expect pads.gesture-new-page is 324
rotate 1
wait 100
expect library.buttons_group is pages
shot 02-sideways-pages
tap 1240 1398
wait 100
expect library.learning is landscape-fit
tap 1240 1398
wait 100
tap 1240 1556
wait 100
expect library.learning is gesture-new-page
shot 03-sideways-learning
key 325 0 1
wait 100
expect library.learning is
expect pads.gesture-new-page is 325
touch down 1240 1556
wait 700
touch up 1240 1556
wait 100
expect library.learning is
expect pads.gesture-new-page is
tap 1240 1556
wait 100
expect library.learning is gesture-new-page
key 326 0 1
wait 100
expect pads.gesture-new-page is 326
shot 04-sideways-learned
tap 1240 1714
wait 100
expect library.buttons_group is
tap library done
wait 100
expect library.sheet is shortcuts
tap 1240 1488
wait 100
expect library.sheet is none
rotate 0
wait 100
tap 965 95
wait 100
tap 1564 2254
wait 200
expect screen is editor
rotate 1
wait 200
expect settings.gesture_new_page is 1
key 326 0 1
wait 100
expect settings.gesture_new_page is 0
wait 1100
swipe 2 1300 1000 -400 0
expect editor.pages is 1
key 326 0 1
wait 100
expect settings.gesture_new_page is 1
wait 1100
swipe 2 1300 1000 -400 0
expect editor.pages is 2
expect editor.page is 2
quit
