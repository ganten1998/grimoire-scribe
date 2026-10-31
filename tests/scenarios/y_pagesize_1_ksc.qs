# settings: status_mode=1
# The data b_library made on an 1860x2480 Scribe, opened on a 1986x2648 one (the Scribe 3 and the
# Scribe Colorsoft): its notebooks keep their page size, fitted to the screen (no zoom chip), their
# ink where it was written, new ink lands under the pen, new pages get the notebook's size, Custom
# music counts systems for the notebook's page, and a notebook made here gets the screen's size.
# The pen and touch calibration b_library made on its 1860x2480 panel is scaled to this one: pen
# taps land.
# root: b_library
# env: GRIMOIRE_SIM_SIZE=1986x2648
# settings: landscape_fit=0
# rust-only
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
expect editor.zoomed is 0
expect editor.paper is 3
expect editor.strokes is 2
# b_library's last stroke, where it was written on the 1860x2480 screen (not scaled to this one)
expect editor.stroke_start is 300 520
# Ink through the fitted view: screen (300, 1500) is page (281, 1405)
pen line 300 1500 1500 1560 80 0.3 0.9
expect editor.stroke_start is 281 1405
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
# Custom music (the panel's Paper): systems per page for the notebook's 1860x2480 page, 6 of the
# layout (7 would fit on this screen's 2648 px); + twice stops at 6. Cancel leaves the page be.
pentap panel paper
wait 300
expect editor.modal is paper
pentap panel paper.custom
wait 300
shot 04-custom-music
expect editor.modal is music
expect editor.music is 3: treble bass tab6
expect editor.music_max is 4
pentap panel music.more
wait 200
expect editor.music is 4: treble bass tab6
pentap panel music.cancel
wait 300
expect editor.modal is none
expect editor.paper is 3
expect editor.strokes is 4
# A new page (the panel's + with the pen): the notebook's size, not the screen's
pentap panel next
wait 500
expect editor.page is 2
expect editor.pages is 2
expect editor.page_size is 1860x2480
expect editor.paper is 3
pen line 300 700 1500 760 60 0.4 0.8
wait 700
shot 05-new-page
expect editor.strokes is 1
# The Pages view: both pages, the notebook's shape
pentap panel pages
wait 400
shot 06-pages
expect editor.modal is pages
tap 125 95
wait 400
expect editor.modal is none
expect editor.page is 2
# The sleep screen: the page fitted to the screen
sleep
wait 300
shot 07-sleep
expect asleep is 1
wake
wait 1700
expect asleep is 0
expect editor.scale is 1.07
# The pinned sleep page: page 2 pinned (Display, with the pen), then asleep on page 1
pentap panel display
wait 300
expect editor.modal is display
tap 563 1611
expect toast is This page is now the sleep screen
expect settings.sleep_mode is 1
wait 300
tap 1523 1923
wait 300
expect editor.modal is none
pentap panel previous
wait 400
expect editor.page is 1
sleep
wait 300
shot 08-sleep-pinned-page-2
expect asleep is 1
wake
wait 1700
expect asleep is 0
expect editor.page is 1
# Sideways: the page's width fits the screen's
rotate 1
wait 500
shot 09-sideways
expect size is 2648x1986
expect editor.page_size is 1860x2480
expect editor.scale is 1.42
expect editor.zoom is 1.00
expect editor.zoomed is 0
rotate 0
wait 500
expect size is 1986x2648
expect editor.scale is 1.07
expect editor.zoomed is 0
# Library with the pen; back to the top level with a finger
pentap panel library
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
expect editor.zoomed is 0
pen line 300 900 1600 900 60 0.5 0.9
expect editor.stroke_start is 300 900
pen curve 400 1200 45 70 0.6 4
wait 700
shot 10-made-here
expect editor.strokes is 2
pentap panel library
wait 500
shot 11-library
expect screen is library
expect library.items is 3
quit
