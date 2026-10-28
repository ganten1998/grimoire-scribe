# settings: status_mode=1
# settings: shortcuts_enabled=1
# env: GRIMOIRE_PASSCODE=enabled
# Raw evdev button injection tests app gating, not Bluetooth or evdev hardware discovery.
# rust-only
wait 150
expect settings.shortcuts_enabled is 1
expect native_pin is 1
tap 965 95
wait 100
tap 510 460
tap 1564 2254
wait 200
expect editor.tool is pencil
key 7 0
wait 60
expect editor.tool is marker
pen line 300 700 600 720 8 0.6 0.7
wait 80
expect editor.strokes is 1
key 44 1
wait 80
expect editor.strokes is 0
key 21 1
wait 80
expect editor.strokes is 1
key 304 0 1
wait 80
expect editor.strokes is 0
key 307 0 1
wait 80
expect editor.strokes is 1
# Extra modifiers don't accidentally match a bare key.
key 2 1
wait 60
expect editor.tool is marker
key 18 0
wait 60
expect editor.tool is eraser
# From the eraser, previous and next tool step from the pen it goes back to (the marker).
key 310 0 1
wait 60
expect editor.tool is fineliner
key 305 0 1
key 311 0 1
wait 60
expect editor.tool is marker
# Past the picker's ninth pen (the highlighter, key 9) next tool is its first again.
key 10 0
key 311 0 1
wait 60
expect editor.tool is pencil
# Never switch tools during a live stroke.
pen down 300 900 0.7
key 7 0
wait 60
expect editor.tool is pencil
pen up 500 950
wait 60
key 7 0
wait 60
expect editor.tool is marker
key 17 0
wait 60
expect settings.white is 1
key 308 0 1
wait 60
expect settings.white is 0
# Next page on the last page adds a page, as the panel's + does; previous on the first stays.
key 106 0
wait 120
expect editor.pages is 2
expect editor.page is 2
key 314 0 1
wait 80
expect editor.page is 1
key 314 0 1
wait 80
expect editor.page is 1
expect editor.pages is 2
key 315 0 1
wait 80
expect editor.page is 2
expect editor.pages is 2
# Native D-pad and decoded HAT directions share the default pad bindings.
key 546 0 1
wait 80
expect editor.page is 1
key 547 0 1
wait 80
expect editor.page is 2
# The controller's next page (Start) at the end adds one too.
key 315 0 1
wait 120
expect editor.pages is 3
expect editor.page is 3
key 546 0 1
wait 80
expect editor.page is 2
# An open tools sheet owns input; keyboard shortcuts cannot act through it.
pentap panel tool-picker
wait 80
expect editor.modal is tools
key 2 0
wait 80
expect editor.tool is marker
tap 100 200
wait 80
# Native PIN ignores injected keys and remains asleep until trusted wake.
sleep
wait 80
key 2 0
wait 80
expect asleep is 1
key 546 0 1
wait 60
expect editor.page is 2
expect asleep.mode is 2
expect input.yielded is 1
expect editor.tool is marker
wake
wait 100
key 2 0
wait 80
expect editor.tool is pencil
shot shortcuts-editor
quit
