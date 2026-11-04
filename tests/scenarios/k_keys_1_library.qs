# settings: status_mode=1
# settings: shortcuts_enabled=1
# settings: mirror_pdf=0
# generate: tests/keys_fixture.py --books 8
# rust-only
# The keyboard scheme in the Library, on a keyboard
# paired with the Kindle (no shortcuts.ini: the starter map): Ctrl+N New notebook, Ctrl+F a new folder's name,
# O Settings, S Share, Ctrl+S saves, Left and Right the grid's screens, Up and Down nothing, and Esc: a sheet
# back one level, a folder up to the top, and at the top "Exit Grimoire?" (Enter the highlighted answer, Yes
# at first; Left and Right move it; Esc is No).
wait 200
expect library.items is 1
expect library.folder is
# Ctrl+N: the New notebook sheet; Esc closes it (its Cancel)
key 49 1
wait 100
expect library.sheet is new
key 1 0
wait 100
expect library.sheet is none
expect library.items is 1
# Ctrl+F: the new folder's name keyboard; Esc is its Cancel (no folder)
key 33 1
wait 100
expect library.sheet is keyboard
key 1 0
wait 100
expect library.sheet is none
expect library.items is 1
# O: Settings; its Display sheet; Esc back to Settings, then closed
key 24 0
wait 100
expect library.sheet is settings
tap library display
wait 200
expect library.sheet is display
key 1 0
wait 100
expect library.sheet is settings
key 1 0
wait 100
expect library.sheet is none
# S: Share; Esc back to the Library
key 31 0
wait 300
expect screen is share
key 1 0
wait 300
expect screen is library
expect library.sheet is none
# Ctrl+S: anything waiting written (library.rs's unit test reads it back), with no toast
key 31 1
wait 100
expect toast is
# Into the folder: Left and Right turn the grid's screens (as its arrows); Up and Down do nothing
tap library folder.Shelf
wait 300
expect library.folder is Shelf
expect library.pages is 2
expect library.page is 1
key 106 0
wait 100
expect library.page is 2
key 106 0
wait 100
expect library.page is 2
key 103 0
key 108 0
wait 100
expect library.page is 2
expect library.sheet is none
key 105 0
wait 100
expect library.page is 1
# Esc in a folder: up to the top level, as its Back
key 1 0
wait 300
expect library.folder is
expect library.items is 1
expect library.sheet is none
# Esc at the top: Exit Grimoire?, Yes highlighted; Esc is No
key 1 0
wait 100
expect library.sheet is exit
expect library.exit is yes
shot 01-exit
key 1 0
wait 100
expect library.sheet is none
expect running is 1
# Right moves the highlight to No (other keys do nothing behind it); Enter presses No
key 1 0
wait 100
key 106 0
wait 100
expect library.exit is no
key 49 1
key 24 0
wait 100
expect library.sheet is exit
key 28 0
wait 100
expect library.sheet is none
expect running is 1
# Its No by a tap
key 1 0
wait 100
tap library exit.no
wait 100
expect library.sheet is none
expect running is 1
# Enter on Yes leaves, as the Exit button
key 1 0
wait 100
key 105 0
wait 100
expect library.exit is no
key 106 0
wait 100
expect library.exit is yes
key 28 0
wait 200
expect running is 0
