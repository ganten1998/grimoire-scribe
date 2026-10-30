# settings: status_mode=1
# Every sheet of the editor stays open through a turn and a sleep with its wake, laid out again for
# the screen it is on, and works there: the tool picker in Pin mode, the paper sheet, Custom music,
# the Display sheet and the Pages view (editor.modal). As the Library does with its sheets.
# settings: mirror_pdf=0
# rust-only
wait 300
tap 965 95
wait 100
tap 510 460
tap 1564 2254
wait 400
expect screen is editor
pen line 300 700 1300 700 60 0.7 0.7
wait 300
# The tool picker in Pin mode: turned, it pins and unpins there; turned back, then asleep and
# awake, still in Pin mode; Done, then a pen picked closes it
pentap panel tool-picker
wait 200
pentap editor picker.pin-mode
wait 200
expect editor.modal is tools
expect editor.pin_mode is 1
rotate 1
wait 300
expect size is 2480x1860
expect editor.modal is tools
expect editor.pin_mode is 1
shot 01-pin-mode-sideways
pentap editor picker.tool:fineliner
wait 200
expect editor.pins is fineliner
expect editor.modal is tools
pentap editor picker.tool:fineliner
wait 200
expect editor.pins is
rotate 0
wait 300
expect editor.modal is tools
expect editor.pin_mode is 1
sleep
wait 300
expect asleep is 1
wake
wait 300
expect asleep is 0
expect editor.modal is tools
expect editor.pin_mode is 1
shot 02-pin-mode-awake
pentap editor picker.pin-mode
wait 200
expect editor.pin_mode is 0
expect editor.modal is tools
pentap editor picker.tool:marker
wait 200
expect editor.modal is none
expect editor.tool is marker
# The paper sheet: turned, a paper picked there (dot grid); again, asleep and awake, grand staff
pentap panel paper
wait 200
expect editor.modal is paper
rotate 1
wait 300
expect editor.modal is paper
shot 03-paper-sideways
tap 325 843
wait 300
expect editor.modal is none
expect editor.paper is 10
rotate 0
wait 300
pentap panel paper
wait 200
expect editor.modal is paper
sleep
wait 300
wake
wait 300
expect editor.modal is paper
tap 1348 1035
wait 300
expect editor.modal is none
expect editor.paper is 8
# Custom music (from the paper sheet): turned, its Cancel there; again, asleep and awake, a bass
# staff added, then Cancel
pentap panel paper
wait 200
pentap 1622 1452
wait 200
expect editor.modal is music
rotate 1
wait 300
expect editor.modal is music
shot 04-music-sideways
tap 598 1693
wait 200
expect editor.modal is none
rotate 0
wait 300
pentap panel paper
wait 200
pentap 1622 1452
wait 200
expect editor.modal is music
expect editor.music is 5: treble tab6
sleep
wait 300
wake
wait 300
expect editor.modal is music
pentap 579 1279
wait 200
expect editor.music is 4: treble tab6 bass
tap 290 1935
wait 200
expect editor.modal is none
expect editor.paper is 8
# The Display sheet: turned, its sleep screen row there (and back), asleep and awake, Done
pentap panel display
wait 200
expect editor.modal is display
rotate 1
wait 300
expect editor.modal is display
shot 05-display-sideways
tap 1240 1199
wait 100
expect settings.sleep_mode is 1
tap 1240 1199
wait 100
tap 1240 1199
wait 100
tap 1240 1199
wait 100
expect settings.sleep_mode is 0
rotate 0
wait 300
expect editor.modal is display
sleep
wait 300
wake
wait 300
expect editor.modal is display
tap 1462 1860
wait 200
expect editor.modal is none
# The Pages view: turned, every page ticked there; turned back, asleep and awake, still ticked; Back
pentap panel next
wait 300
expect editor.pages is 2
pentap panel pages
tap panel pages.graph
wait 300
expect editor.map is graph
pentap panel map.pages
expect editor.modal is pages
rotate 1
wait 300
expect editor.modal is pages
tap panel pages.all
wait 100
expect editor.ticked is 2
shot 06-pages-sideways
rotate 0
wait 300
expect editor.modal is pages
expect editor.ticked is 2
sleep
wait 300
wake
wait 300
expect editor.modal is pages
expect editor.ticked is 2
tap panel pages.back
wait 300
expect editor.modal is none
expect editor.pages is 2
pentap panel library
wait 500
expect screen is library
quit
