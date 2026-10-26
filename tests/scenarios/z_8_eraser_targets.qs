# The maintainer's handwriting stays centered and erasable as its original 32 strokes.
# env: GRIMOIRE_CHECK_FIT=1
wait 100
tap 1535 95
tap 930 446
tap 1723 289
expect settings.eraser_mode is 0
expect penset.pad_strokes is 32
expect penset.pad_ink is 1
expect penset.pad_lighter is 0
expect penset.pad_sample_rect is 696 564 1047 240
shot 01-handwriting-ready
# Blank paper does not clear the practice ink.
pentap 700 500
expect penset.pad_strokes is 32
# A whole-stroke hit removes just the touched pieces of the actual handwriting.
pentap 1000 700
expect penset.pad_strokes not 32
expect penset.pad_strokes not 0
expect penset.pad_ink is 1
shot 02-whole-stroke-hit
tap 1708 498
expect penset.pad_strokes is 32
# Sweeps cover the lettering without needing to land on each small mark.
pen line 640 570 1790 570 1 1.0 1.0
pen line 640 630 1790 630 1 1.0 1.0
pen line 640 690 1790 690 1 1.0 1.0
pen line 640 750 1790 750 1 1.0 1.0
pen line 640 810 1790 810 1 1.0 1.0
pen line 640 870 1790 870 1 1.0 1.0
expect penset.pad_strokes is 0
expect penset.pad_ink is 0
expect penset.pad_lighter is 0
shot 03-whole-sample-erased
tap 1708 498
tap 1559 1440
expect settings.eraser_mode is 1
expect penset.pad_strokes is 32
pen line 1000 580 1000 790 12 0.7 0.7
expect penset.pad_strokes is 32
expect penset.pad_ink is 1
expect penset.pad_lighter is 0
shot 04-precise-handwriting
# A full precise wipe removes the pixels, not the underlying stored practice strokes.
pen line 640 570 1790 570 60 1.0 1.0
pen line 640 630 1790 630 60 1.0 1.0
pen line 640 690 1790 690 60 1.0 1.0
pen line 640 750 1790 750 60 1.0 1.0
pen line 640 810 1790 810 60 1.0 1.0
pen line 640 870 1790 870 60 1.0 1.0
expect penset.pad_strokes is 32
expect penset.pad_ink is 0
expect penset.pad_lighter is 0
tap 1708 498
expect penset.pad_ink is 1
tap 1630 95
expect settings.eraser_mode is 0
expect penset.pad_strokes is 32
rotate 1
expect penset.pad_strokes is 32
expect penset.pad_sample_rect is 1006 564 1047 240
shot 05-landscape-centered
pentap 1530 700
expect penset.pad_strokes not 32
expect penset.pad_strokes not 0
tap 2318 498
expect penset.pad_strokes is 32
rotate 0
expect penset.pad_strokes is 32
tap 125 95
expect screen is library
quit
