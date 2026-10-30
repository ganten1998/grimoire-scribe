# settings: status_mode=1
# rust-only
# A button the first finger pressed on the bar or the Find sheet, and a second finger took over (the first
# finger's cancel, then the gesture): let go with a Fast refresh as it was pressed, not a Ui one, and nothing
# is tapped.
# root: z_bar_0_make
# settings: shortcuts_enabled=1
# copy: tests/fixtures/index-keys .
wait 300
tap 200 400
wait 300
swipe 2 930 1300 0 -400
wait 300
expect editor.bar is open
expect display.presents not -1
finger 1 down 479 2218
frame 40
expect display.presents is fast 40 2156 878 124
finger 2 down 1400 1200
frame 20
expect display.presents is fast 40 2156 878 124
finger 1 up 479 2218
finger 2 up 1400 1200
frame
expect touch.contacts is 0
expect editor.bar is open
expect editor.bar.peek is 0
# the Find sheet: a row of it
tap panel bar.close
key 34 0
wait 300
expect editor.find is go
tap 487 1725
expect editor.find.1 is Go to page 3
expect display.presents not -1
finger 1 down 930 1390
frame 40
expect display.presents is fast 36 1328 1788 124
finger 2 down 1400 700
frame 20
expect display.presents is fast 36 1328 1788 124
finger 1 up 930 1390
finger 2 up 1400 700
frame
expect touch.contacts is 0
expect editor.find is go
expect editor.page is 1
