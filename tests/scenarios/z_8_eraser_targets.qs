# Eraser practice has real letter strokes and a separate black block. Empty paper does not
# erase anything; whole strokes removes only touched targets, precise keeps the untouched ink.
# env: GRIMOIRE_CHECK_FIT=1
wait 100
tap 1535 95
tap 930 446
expect screen is penset
expect settings.eraser_mode is 0
tap 1723 289
expect penset.pad_letters is 4
expect penset.pad_block_rect is 1250 584 530 310
expect penset.pad_ink is 1
expect penset.pad_white is 0
shot 01-whole-strokes-ready
# A blank part of the pad does not clear the sample.
pentap 700 540
expect penset.pad_letters is 4
# Touch one T, leaving the E, S, other T and block alone.
pentap 730 740
expect penset.pad_letters is 3
expect penset.pad_ink_near.730.750 is 0
expect penset.pad_ink_near.805.740 is 1
expect penset.pad_ink_near.1400.740 is 1
shot 02-one-letter-removed
tap 1708 498
expect penset.pad_letters is 4
expect penset.pad_white is 0
# A fast sweep whose endpoints miss the letters still crosses and removes them.
pen down 640 740 1.0
expect penset.pad_letters is 4
pen move 1190 740 1.0
expect penset.pad_letters is 0
pen up 1190 740
expect penset.pad_ink_near.1400.740 is 1
pentap 1400 740
expect penset.pad_ink is 0
expect penset.pad_lighter is 0
shot 03-whole-targets-erased
tap 1708 498
# Its bounding box overlaps the block, but the diagonal goes past the upper-left corner.
pen line 640 900 1380 400 8 0.7 0.7
expect penset.pad_white is 0
tap 1708 498
tap 1559 1440
expect settings.eraser_mode is 1
expect penset.pad_letters is 4
pen line 730 700 730 790 12 0.7 0.7
expect penset.pad_ink_near.730.740 is 0
expect penset.pad_ink_near.730.639 is 1
expect penset.pad_ink_near.805.740 is 1
pen line 1450 650 1450 830 12 0.7 0.7
expect penset.pad_white is 1
expect penset.pad_ink_near.1450.740 is 0
expect penset.pad_ink_near.1700.740 is 1
expect penset.pad_lighter is 0
shot 04-precise-text-and-block
# Reset pad restores both targets; Reset tool also restores whole-stroke mode.
tap 1708 498
expect settings.eraser_mode is 1
expect penset.pad_letters is 4
expect penset.pad_white is 0
tap 1630 95
expect settings.eraser_mode is 0
expect penset.pad_letters is 4
rotate 1
expect penset.pad_block_rect is 1560 584 840 310
expect penset.pad_letters is 4
pentap 885 740
expect penset.pad_letters is 3
shot 05-landscape-whole-stroke
tap 2089 1440
expect settings.eraser_mode is 1
expect penset.pad_letters is 4
pen line 1780 650 1780 830 12 1.0 1.0
expect penset.pad_white is 1
expect penset.pad_lighter is 0
shot 06-landscape-precise
rotate 0
expect penset.pad_letters is 4
expect penset.pad_white is 0
tap 125 95
expect screen is library
quit
