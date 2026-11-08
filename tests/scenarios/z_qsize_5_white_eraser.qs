# settings: status_mode=1
# settings: shortcuts_enabled=1
# settings: panel_tools=fineliner
# settings: eraser_mode=1
# rust-only
# Quick sizes with white ink and the eraser: white ink shares its pen's sizes (the card's samples are white on
# black), the next white stroke has the picked width; the eraser's card shows its two modes (Precise picked
# here) and the precise eraser at its 12 mm chip, zoomed to 2, still reads 12 mm on the screen (it keeps its size
# on the screen at any zoom: 6 page mm).
wait 3600
tap 965 95
wait 200
tap library paper.1
tap 1564 2254
wait 400
expect screen is editor
pen curve 300 520 40 70 0.7 5
wait 300
pen leave
# White ink on (the tool picker's White)
pentap panel tool-picker
wait 50
pentap 1425 878
wait 50
pentap 300 1800
wait 100
pen leave
expect settings.white is 1
touch down 1768 466
wait 700
touch up 1768 466
wait 200
expect editor.quicksize is fineliner
expect editor.quicksize.white is 1
shot 01-white-card
tap panel quicksize.3
wait 200
expect settings.fineliner.size_max_mm is 0.6000
pen line 300 520 1300 540 40 0.7 0.7
wait 400
pen leave
expect editor.stroke_look is white fineliner
expect editor.stroke_screen_mm is 0.60
# The eraser's card: Precise is the mode shown; Whole strokes taps through to settings and the card stays.
touch down 1768 574
wait 700
touch up 1768 574
wait 200
expect editor.quicksize is eraser
expect editor.quicksize.mode is 1
expect editor.quicksize.white is 0
tap panel quicksize.whole
wait 200
expect settings.eraser_mode is 0
expect editor.quicksize is eraser
expect editor.quicksize.mode is 0
tap panel quicksize.precise
wait 200
expect settings.eraser_mode is 1
tap panel quicksize.3
wait 200
expect settings.eraser.size_max_mm is 12.0000
expect editor.quicksize is 0
pinch 600 600 300 750 6
wait 700
expect editor.zoomed is 1
eraser line 900 300 1000 1100 30
wait 700
expect editor.stroke_screen_mm is 12.00
expect editor.stroke_look is round tip
