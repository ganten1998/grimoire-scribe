# settings: status_mode=1
# settings: shortcuts_enabled=1
# settings: mirror_pdf=0
# copy: tests/fixtures/keys-old-kindle .
# rust-only
# A Kindle that had shortcuts on keeps the starter shortcuts.ini 2.0.0-beta.1 wrote, byte for byte (nobody
# edited it): at the start it becomes this build's starter map (grimoire.log says so; no toast). Then Pen feel's keys: 1 to 9 that pen's tab (the picker's order), 0 and E the eraser's,
# Esc back to where it came from.
wait 300
expect toast is
# Esc asks Exit Grimoire? (the new map)
key 1 0
wait 100
expect library.sheet is exit
key 1 0
wait 100
expect library.sheet is none
# Pen feel from Settings
tap library settings
wait 200
tap library pen-feel
wait 300
expect screen is penset
key 4 0
wait 200
expect penset.tab is charcoal
key 11 0
wait 200
expect penset.tab is eraser
key 2 0
wait 200
expect penset.tab is pencil
key 18 0
wait 200
expect penset.tab is eraser
key 10 0
wait 200
expect penset.tab is highlighter
# Ctrl+S (no toast: pensettings.rs's unit test reads the file back); Esc: back
key 31 1
wait 100
expect toast is
key 1 0
wait 300
expect screen is library
