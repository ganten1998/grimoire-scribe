# rust-only
# settings: status_mode=1
# settings: mirror_pdf=0
# settings: shortcuts_enabled=1
# copy: tests/fixtures/index-keys .
wait 200
tap 1535 95
wait 100
tap library gestures
wait 100
expect library.sheet is gestures
tap library gestures.group.index
wait 100
expect library.gestures_group is index
tap library gestures.row.1
wait 100
expect library.gestures.learning is links
shape Z 930 1150 45 600
shape Z 930 1150 45 600
tap library record.use
wait 100
expect library.record is 0
expect gestures.shape.links is 2
tap library done
wait 100
tap library done
wait 100
expect library.sheet is settings
tap library done
wait 100
tap 965 95
wait 100
tap 1564 2254
wait 200
pentap panel next
pentap panel next
pentap panel previous
pentap panel previous
key 38 0
pen line 300 1000 900 1200 20 0.6 0.6
expect editor.ui_at.930.2370 is linkto.pages
tap 930 2370
tap 1155 340
key 38 0
pen line 300 1400 900 1500 20 0.6 0.6
expect editor.ui_at.930.679 is linkto.pages
tap 930 679
tap 703 340
pen leave
wait 1100
expect editor.page is 1
key 37 0
wait 100
tap 479 2218
expect editor.bar is peek
expect editor.bar.peek is 3
wait 1100
expect display.presents not clear
shape S 930 1150 45 600
expect fshape.last has reject
expect display.presents has fast
wait 1000
expect display.presents has ui
expect editor.bar is peek
expect editor.page is 1
expect fshape.fired is 0
shape Z 930 1150 45 600
expect editor.bar is 0
expect editor.page is 1
expect fshape.fired is 1
key 37 0
tap 479 2218
expect editor.bar is peek
tap 600 1100
expect editor.bar is 0
expect editor.page is 3
