# The runner's own check: every expect line below holds, so the run must exit with status 0 (see
# the scenarios rule in the Makefile). It asks every probe key, one of each family, at least once
# (docs/SIM.md lists them).
wait 200
expect running is 1
expect watching is 0
expect screen is library
expect screen not editor
expect screen has lib
expect asleep is 0
expect rotation is 0
expect size is 1860x2480
expect toast is
expect toast.rect is
expect settings.theme is gothic
expect settings.invert is 0
expect settings.mirror_pdf is 1
expect settings.follow_rotation is 1
expect settings.pinch_zoom is 1
expect settings.side_button is 2
expect settings.sleep_mode is 0
expect settings.sleep_minutes is 10
expect settings.pen.valid is 0
expect settings.touch.valid is 0
expect settings.pencil.size_max_mm has .
expect settings.ballpoint.pressure_gamma has .
expect light.intensity is 12
expect light.warmth is 8
expect library.sheet is none
expect library.folder is
expect library.items is 0
expect library.page is 1
expect library.pages is 1
expect library.text is
expect library.music is
expect editor.page is 0
expect editor.pages is 0
expect editor.modal is none
expect editor.strokes is 0
expect editor.paper is none
expect editor.music is
expect editor.ticked is 0
expect editor.unsaved is 0
# New notebook (medium rule is preselected), then Create
tap 965 95
expect library.sheet is new
expect library.paper is 1
tap 1564 2254
expect screen is editor
expect library.sheet is none
expect editor.page is 1
expect editor.pages is 1
expect editor.tool is pencil
expect editor.zoom is 1.00
expect editor.paper is 1
expect editor.panel is expanded
expect editor.modal is none
expect editor.ticked is 0
expect editor.music is
pen line 300 700 1200 700 40 0.5 0.5
expect editor.strokes is 1
expect editor.unsaved is 1
# The tool picker, then the mechanical pencil from it
pentap panel tool-picker
expect editor.modal is tools
pentap 1188 395
expect editor.modal is none
expect editor.tool is mech
# A second page
pentap panel next
expect editor.page is 2
expect editor.pages is 2
expect editor.strokes is 0
# Sleep and wake
sleep
expect asleep is 1
wake
expect asleep is 0
# Sideways and back
rotate 1
expect rotation is 1
expect size is 2480x1860
rotate 0
expect rotation is 0
expect size is 1860x2480
# Semantic panel taps stay valid as new buttons are added.
pentap panel library
expect screen is library
expect toast is Saved
expect library.items is 1
quit
# The app has quit: "running" is the one key with an answer now
expect running is 0
expect running not 1
