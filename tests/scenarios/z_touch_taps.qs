# settings: status_mode=1
# rust-only
# root: z_bar_0_make
# settings: shortcuts_enabled=1
# copy: tests/fixtures/index-keys .
# Buttons are not wrongly rejected: a tap is a tap however the fingertip rolled while it stayed on
# what it pressed, a slow press on a button with no hold is its tap, a press shows pressed at once
# where it did not, and a finger landing between the panel's buttons takes the nearest one.
wait 300
tap 200 400
wait 300
expect screen is editor
expect editor.page is 1
# A finger rolled 3.4 mm on a link still follows it (within 5 mm, as the clock's corner).
touch down 600 1100
touch move 640 1100
touch up 640 1100
wait 300
expect editor.page is 3
tap panel previous
tap panel previous
expect editor.page is 1
# The Links bar: a slow press on its X (the X has no hold) closes it.
swipe 2 930 1300 0 -400
wait 300
expect editor.bar is open
touch down 1761 2046
wait 800
touch up 1761 2046
wait 300
expect editor.bar is 0
# The Display sheet: a tap rolled 40 px on Invert colors acts, and its Done shows pressed at once.
tap panel display
wait 300
expect editor.modal is display
touch down 400 832
touch move 440 845
touch up 440 845
wait 500
expect settings.invert is 1
tap 400 832
wait 500
expect settings.invert is 0
expect display.presents not -1
touch down 1464 1858
expect display.presents is fast 1280 1788 360 140
touch up 1464 1858
wait 300
expect editor.modal is none
# The panel: a finger in the gap between two buttons, or in the frame's margin beside one, takes
# the nearest button; a greyed-out button (Previous, on page 1) still takes nothing.
expect editor.panel_at.1768.517 is eraser
expect editor.panel_at.1768.522 is shapes
expect editor.panel_at.1710.1674 is display
expect editor.panel_at.1768.1190 is
tap 1710 1674
wait 300
expect editor.modal is display
tap 1464 1858
wait 300
expect editor.modal is none
