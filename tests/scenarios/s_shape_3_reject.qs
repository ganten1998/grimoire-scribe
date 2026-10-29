# rust-only
# settings: status_mode=1
# settings: mirror_pdf=0
# root: s_shape_1_record
wait 200
tap 965 95
wait 100
tap 1564 2254
wait 200
expect screen is editor
expect fshape.fired is 0
expect display.presents not clear
shape S 930 1150 45 600
expect fshape.last has reject
expect editor.strokes is 0
expect fshape.fired is 0
expect display.presents has fast
wait 1000
expect display.presents has ui
expect settings.hints is 128
expect toast is That is not one of your finger shapes. Settings > Gestures lists them.
wait 4200
shape S 930 1150 45 600
expect fshape.fired is 0
expect settings.hints is 128
expect toast is
