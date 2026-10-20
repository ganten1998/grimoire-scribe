# Back on an 1860x2480 Scribe with what y_pagesize_1_ksc left. The notebook made on the 1986x2648
# screen keeps its size: the whole page fitted to this smaller screen (no zoom chip), sideways its
# width, and pinching out shows all of it. The notebook made here is at 100% again, with what was
# written on the bigger screen. The calibration, recorded as the 1986x2648 panel's by then, is
# scaled back: pen taps land.
# root: y_pagesize_1_ksc
wait 300
shot 01-library
expect size is 1860x2480
expect settings.pen.valid is 1
expect settings.pen.panel is 1860x2480
expect settings.touch.panel is 1860x2480
expect library.items is 3
# The notebook made on the 1986x2648 screen (the newest, after the School folder), with the pen
pentap 929 608
wait 500
shot 02-fitted
expect screen is editor
expect editor.page_size is 1986x2648
expect editor.scale is 0.94
expect editor.zoom is 1.00
expect editor.strokes is 2
pen line 250 1400 1450 1450 80 0.3 0.9
wait 700
shot 03-written
expect editor.strokes is 3
# Sideways the page's width fits the screen's; pinching out shows the whole page
rotate 1
wait 500
expect size is 2480x1860
expect editor.scale is 1.25
expect editor.zoom is 1.00
pinch 1240 930 900 300 8
wait 500
shot 04-sideways-whole
expect editor.scale is 0.70
expect editor.zoom is 0.56
rotate 0
wait 500
expect editor.scale is 0.94
expect editor.zoom is 1.00
# A new page (the panel's + with the pen): the notebook's size
pentap 1768 1114
wait 500
expect editor.pages is 2
expect editor.page is 2
expect editor.page_size is 1986x2648
# Library with the pen. The notebook made here, in School: 100% again, on the page left open
pentap 1768 1566
wait 500
expect screen is library
tap 326 608
wait 300
expect library.folder is School
pentap 326 608
wait 500
shot 05-own-size
expect screen is editor
expect editor.page_size is 1860x2480
expect editor.scale is 1.00
expect editor.pages is 2
expect editor.page is 1
expect editor.strokes is 4
pentap 1768 1566
wait 500
expect screen is library
quit
