# Holding and moving away gives exact, continuous adjustments, without an up-event jump.
# env: GRIMOIRE_CHECK_FIT=1
wait 100
tap 1535 95
tap 930 446
expect screen is penset
touch down 830 1172
expect penset.param.size_min_mm is 0.2800
wait 400
touch move 830 1332
expect penset.param.size_min_mm is 0.2800
touch move 855 1332
expect penset.param.size_min_mm is 0.3000
touch move 867 1332
expect penset.param.size_min_mm is 0.3100
wait 120
expect penset.slider_pending is 0
shot 01-fine-value-painted
touch move 855 1332
touch up 855 1332
expect penset.param.size_min_mm is 0.3000
expect penset.slider_pending is 0
expect penset.param.opacity_min is 0.8500
# Releasing ends fine mode; a new press uses its own origin and timer.
pen down 830 1172 0.5
expect penset.param.size_min_mm is 0.2800
wait 400
pen move 830 1332 0.5
pen move 855 1332 0.5
pen up 855 1332
expect penset.param.size_min_mm is 0.3000
expect penset.param.opacity_min is 0.8500
tap 125 95
tap 930 446
expect penset.param.size_min_mm is 0.3000
shot 02-fine-value-saved
tap 125 95
quit
