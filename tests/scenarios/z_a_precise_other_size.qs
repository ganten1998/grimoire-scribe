# settings: status_mode=1
# The precise eraser on a notebook from a Scribe of another size (b_library's 1860x2480 pages on a
# 1986x2648 screen, fitted to it): the eraser keeps its size on the screen, fitted and zoomed in,
# and restores the paper at the notebook's page size, not the screen's.
# root: b_library
# env: GRIMOIRE_SIM_SIZE=1986x2648
# settings: eraser_mode=1
# settings: mirror_pdf=0
# rust-only
wait 300
expect size is 1986x2648
pentap 347 636
wait 300
expect library.folder is School
tap 347 636
wait 500
expect screen is editor
expect editor.page_size is 1860x2480
expect editor.scale is 1.07
expect editor.strokes is 2
pen line 300 1500 1500 1560 80 0.6 0.9
wait 300
expect editor.strokes is 3
eraser line 700 1400 760 1700 20
wait 700
shot 01-erased-fitted
expect editor.strokes is 4
expect editor.erase_strokes is 1
expect editor.stroke_screen_mm is 7.00
expect editor.stroke_look is round tip
expect editor.stroke_settings is eraser
expect editor.paper_cache is 1
expect editor.live_matches_render is 1
pinch 900 1500 300 750 6
wait 300
expect editor.zoomed is 1
eraser line 900 1300 960 1800 30
wait 700
shot 02-erased-zoomed
expect editor.strokes is 5
expect editor.stroke_screen_mm is 7.00
expect editor.stroke_look is round tip
expect editor.stroke_settings is eraser
expect editor.live_matches_render is 1
pentap panel library
wait 1500
expect screen is library
expect bg.busy is 0
quit
