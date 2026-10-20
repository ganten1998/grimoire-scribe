# The data b_library made on an 1860x2480 Scribe, opened on a 1986x2648 one (the Scribe 3 and the
# Scribe Colorsoft): its notebooks keep their page size, fitted to the screen, ink lands under the
# pen, new pages get the notebook's size, and a notebook made here gets the screen's. The pen and
# touch calibration b_library made on its 1860x2480 panel is scaled to this one: pen taps land.
# root: b_library
# env: GRIMOIRE_SIM_SIZE=1986x2648
wait 300
shot 01-library
expect size is 1986x2648
expect settings.pen.valid is 1
expect settings.pen.panel is 1986x2648
expect settings.touch.panel is 1986x2648
# The School folder with the pen, its notebook (sheet music, two strokes) with a finger
pentap 347 636
wait 300
expect library.folder is School
tap 347 636
wait 500
shot 02-fitted
expect screen is editor
expect editor.page_size is 1860x2480
expect editor.scale is 1.07
expect editor.zoom is 1.00
expect editor.paper is 3
expect editor.strokes is 2
# Ink through the fitted view
pen line 300 1500 1500 1560 80 0.3 0.9
pen curve 420 1850 40 60 0.6 3
wait 700
shot 03-written
expect editor.strokes is 4
# The eraser end takes the curve again
eraser line 380 1850 560 1850 20
wait 700
expect editor.strokes is 3
pen curve 420 1850 40 60 0.6 3
wait 700
expect editor.strokes is 4
# A new page (the panel's + with the pen): the notebook's size, not the screen's
pentap 1894 1114
wait 500
expect editor.page is 2
expect editor.pages is 2
expect editor.page_size is 1860x2480
expect editor.paper is 3
pen line 300 700 1500 760 60 0.4 0.8
wait 700
shot 04-new-page
expect editor.strokes is 1
# The Pages view: both pages, the notebook's shape
pentap 1894 1026
wait 400
shot 05-pages
expect editor.modal is pages
tap 125 95
wait 400
expect editor.modal is none
expect editor.page is 2
# The sleep screen: the page fitted to the screen
sleep
wait 300
shot 06-sleep
expect asleep is 1
wake
wait 1700
expect asleep is 0
expect editor.scale is 1.07
# The pinned sleep page: page 2 pinned (Display, with the pen), then asleep on page 1
pentap 1894 1350
wait 300
expect editor.modal is display
tap 563 1368
expect toast is This page is now the sleep screen
expect settings.sleep_mode is 1
wait 300
tap 1523 1680
wait 300
expect editor.modal is none
pentap 1894 938
wait 400
expect editor.page is 1
sleep
wait 300
shot 07-sleep-pinned-page-2
expect asleep is 1
wake
wait 1700
expect asleep is 0
expect editor.page is 1
# Sideways: the page's width fits the screen's
rotate 1
wait 500
shot 08-sideways
expect size is 2648x1986
expect editor.page_size is 1860x2480
expect editor.scale is 1.42
expect editor.zoom is 1.00
rotate 0
wait 500
expect size is 1986x2648
expect editor.scale is 1.07
# Library with the pen; back to the top level with a finger
pentap 1894 1566
wait 500
expect screen is library
tap 125 95
wait 300
expect library.folder is
# A notebook made here: this screen's size, shown at 100%
tap 1091 95
wait 300
expect library.sheet is new
tap 1663 2306
wait 500
expect screen is editor
expect editor.page_size is 1986x2648
expect editor.scale is 1.00
expect editor.zoom is 1.00
pen line 300 900 1600 900 60 0.5 0.9
pen curve 400 1200 45 70 0.6 4
wait 700
shot 09-made-here
expect editor.strokes is 2
pentap 1894 1566
wait 500
shot 10-library
expect screen is library
expect library.items is 3
quit
