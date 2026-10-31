# rust-only
# root: z_bar_0_make
# settings: status_mode=1
# settings: shapes=1
# settings: shape_min_mm=30
# Smallest shape (rc.9, 2.0.0-rc.12 makes it count): at 30 mm, a held circle 17 mm across stays as drawn and one
# 51 mm across snaps.
wait 300
tap 200 400
wait 300
pen leave
wait 1100
expect settings.shape_min_mm is 30
pen down 700.000 1000.000 0.6 0 0
pen move 698.769 1015.643 0.6 0 0
pen move 695.106 1030.902 0.6 0 0
pen move 689.101 1045.399 0.6 0 0
pen move 680.902 1058.779 0.6 0 0
pen move 670.711 1070.711 0.6 0 0
pen move 658.779 1080.902 0.6 0 0
pen move 645.399 1089.101 0.6 0 0
pen move 630.902 1095.106 0.6 0 0
pen move 615.643 1098.769 0.6 0 0
pen move 600.000 1100.000 0.6 0 0
pen move 584.357 1098.769 0.6 0 0
pen move 569.098 1095.106 0.6 0 0
pen move 554.601 1089.101 0.6 0 0
pen move 541.221 1080.902 0.6 0 0
pen move 529.289 1070.711 0.6 0 0
pen move 519.098 1058.779 0.6 0 0
pen move 510.899 1045.399 0.6 0 0
pen move 504.894 1030.902 0.6 0 0
pen move 501.231 1015.643 0.6 0 0
pen move 500.000 1000.000 0.6 0 0
pen move 501.231 984.357 0.6 0 0
pen move 504.894 969.098 0.6 0 0
pen move 510.899 954.601 0.6 0 0
pen move 519.098 941.221 0.6 0 0
pen move 529.289 929.289 0.6 0 0
pen move 541.221 919.098 0.6 0 0
pen move 554.601 910.899 0.6 0 0
pen move 569.098 904.894 0.6 0 0
pen move 584.357 901.231 0.6 0 0
pen move 600.000 900.000 0.6 0 0
pen move 615.643 901.231 0.6 0 0
pen move 630.902 904.894 0.6 0 0
pen move 645.399 910.899 0.6 0 0
pen move 658.779 919.098 0.6 0 0
pen move 670.711 929.289 0.6 0 0
pen move 680.902 941.221 0.6 0 0
pen move 689.101 954.601 0.6 0 0
pen move 695.106 969.098 0.6 0 0
pen move 698.769 984.357 0.6 0 0
pen move 700.000 1000.000 0.6 0 0
wait 700
expect editor.shape_preview is 0
pen up
pen leave
wait 1100
expect editor.stroke_shape is none
pen down 1300.000 1700.000 0.6 0 0
pen move 1296.307 1746.930 0.6 0 0
pen move 1285.317 1792.705 0.6 0 0
pen move 1267.302 1836.197 0.6 0 0
pen move 1242.705 1876.336 0.6 0 0
pen move 1212.132 1912.132 0.6 0 0
pen move 1176.336 1942.705 0.6 0 0
pen move 1136.197 1967.302 0.6 0 0
pen move 1092.705 1985.317 0.6 0 0
pen move 1046.930 1996.307 0.6 0 0
pen move 1000.000 2000.000 0.6 0 0
pen move 953.070 1996.307 0.6 0 0
pen move 907.295 1985.317 0.6 0 0
pen move 863.803 1967.302 0.6 0 0
pen move 823.664 1942.705 0.6 0 0
pen move 787.868 1912.132 0.6 0 0
pen move 757.295 1876.336 0.6 0 0
pen move 732.698 1836.197 0.6 0 0
pen move 714.683 1792.705 0.6 0 0
pen move 703.693 1746.930 0.6 0 0
pen move 700.000 1700.000 0.6 0 0
pen move 703.693 1653.070 0.6 0 0
pen move 714.683 1607.295 0.6 0 0
pen move 732.698 1563.803 0.6 0 0
pen move 757.295 1523.664 0.6 0 0
pen move 787.868 1487.868 0.6 0 0
pen move 823.664 1457.295 0.6 0 0
pen move 863.803 1432.698 0.6 0 0
pen move 907.295 1414.683 0.6 0 0
pen move 953.070 1403.693 0.6 0 0
pen move 1000.000 1400.000 0.6 0 0
pen move 1046.930 1403.693 0.6 0 0
pen move 1092.705 1414.683 0.6 0 0
pen move 1136.197 1432.698 0.6 0 0
pen move 1176.336 1457.295 0.6 0 0
pen move 1212.132 1487.868 0.6 0 0
pen move 1242.705 1523.664 0.6 0 0
pen move 1267.302 1563.803 0.6 0 0
pen move 1285.317 1607.295 0.6 0 0
pen move 1296.307 1653.070 0.6 0 0
pen move 1300.000 1700.000 0.6 0 0
wait 700
expect editor.shape_preview is circle
pen up
pen leave
wait 1100
quit
