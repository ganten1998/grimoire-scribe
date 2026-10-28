# Continues live_check's script after both authenticated streams are attached.
# Make a new notebook, then capture a complete pressure ramp before any rendering catches up.
tap 965 95
wait 100
tap 1564 2254
wait 100
expect screen is editor
pen line 300 700 600 800 32 0.2 0.9
wait 100
pen hover 901 1501
wait 100
expect editor.strokes is 1
# An open tool picker must report disabled ink even for a long contact path.
pentap 1768 358
wait 100
expect editor.modal is tools
pen line 800 400 1000 450 16 0.5 0.5
wait 100
pen hover 902 1502
hold
