# settings: status_mode=1
# settings: shortcuts_enabled=1
# copy: tests/fixtures/keys-2-1-starter .
# rust-only
# The size keys from the starter map (2.2). A shortcuts.ini that is byte for byte the starter 2.0.0-beta.2 to 2.1
# wrote becomes this build's at the start (no toast), so Shift+[ and Shift+] are size-smaller and size-larger on a
# page. They step the pen in use through its five quick sizes as a tap on that chip of its card sets them (a
# setting: no history step); each step says the size in a short toast (1.2 s), which a step while it shows
# replaces; at the smallest and the largest the size stays and the toast still says it, however often the key
# comes. [ and ] still step tools. The eraser steps through its own chips. With Select mode on (the lasso, which has
# no size) nothing happens and nothing shows.
wait 300
expect toast is
key 49 1
wait 100
expect library.sheet is new
key 28 0
wait 300
expect screen is editor
expect editor.tool is pencil
expect settings.pencil.size_max_mm is 0.3500
# Shift+]: the next chip up, said in a toast; no history step
key 27 2
wait 100
expect settings.pencil.size_max_mm is 0.5000
expect settings.pencil.size_min_mm is 0.1900
expect toast is Size: 0.5 mm
expect editor.undo_kind is
key 27 2
wait 100
expect toast is Size: 0.7 mm
key 27 2
wait 100
expect settings.pencil.size_max_mm is 1.0000
expect toast is Size: 1 mm
# Pressed again and again at the largest: the size stays, the toast still says it
key 27 2
wait 50
key 27 2
wait 50
key 27 2
wait 100
expect settings.pencil.size_max_mm is 1.0000
expect toast is Size: 1 mm
# Shift+[ down to the smallest and past it
key 26 2
wait 50
key 26 2
wait 50
key 26 2
wait 50
key 26 2
wait 50
key 26 2
wait 50
key 26 2
wait 100
expect settings.pencil.size_max_mm is 0.2500
expect toast is Size: 0.25 mm
# The toast goes after its 1.2 s
wait 1500
expect toast is
# [ and ] are still the tools'
key 27 0
wait 100
expect editor.tool not pencil
key 26 0
wait 100
expect editor.tool is pencil
# The eraser in use: its own chips (8 mm, then 12, the largest)
key 18 0
wait 100
expect editor.tool is eraser
key 27 2
wait 100
expect settings.eraser.size_max_mm is 12.0000
expect toast is Size: 12 mm
key 27 2
wait 100
expect toast is Size: 12 mm
key 18 0
wait 100
expect editor.tool is pencil
# Select mode: nothing, and no toast
key 31 0
wait 1500
expect editor.select is armed
expect toast is
key 27 2
wait 100
key 26 2
wait 100
expect toast is
expect settings.pencil.size_max_mm is 0.2500
expect settings.eraser.size_max_mm is 12.0000
key 1 0
wait 100
expect editor.select is off
key 27 2
wait 100
expect settings.pencil.size_max_mm is 0.3500
expect toast is Size: 0.35 mm
