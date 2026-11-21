# settings: status_mode=1
# Toolbar clear (no confirmation), Display sheet: brightness/warmth sliders; Dark mode is Settings' row
# rust-only
wait 200
tap library add
tap library add.new-notebook
wait 100
tap 510 460
tap 1564 2254
wait 300
expect screen is editor
pen curve 300 520 40 70 0.7 5
pen line 300 700 1300 720 60 0.3 0.9
wait 300
shot 01-before-clear
expect editor.strokes is 2
pentap panel clear
wait 300
expect toast is
shot 02-cleared-by-toolbar
expect editor.strokes is 0
pentap panel undo
wait 300
expect editor.strokes is 2
pentap panel display
wait 300
shot 03-display-sheet
expect editor.modal is display
expect settings.invert is 0
expect light.intensity is 12
# A slider being dragged redraws its own row alone (the whole card was a 1.8 Mpx copy at every step),
# at most every 80 ms; the release draws the whole card.
touch down 1000 296
wait 100
touch move 1100 296
expect display.last_fast_rect is 80 236 1700 120
touch up 1100 296
wait 300
drag touch 1000 296 1500 296 5
wait 300
expect light.intensity not 12
# The Display sheet's X closes it; Dark mode is under Style in Settings > Display.
tap 1735 95
wait 300
shot 04-sheet-closed
expect editor.modal is none
tap panel settings
wait 300
expect editor.modal is settings
tap 250 422
wait 100
tap library dark
wait 500
shot 05-dark-mode
expect settings.invert is 1
tap library dark
wait 500
expect settings.invert is 0
tap library done
wait 300
expect editor.modal is none
quit
