# rust-only
# root: z_bar_0_make
# copy: tests/fixtures/shapes .
wait 300
tap 200 400
expect settings.shapes is 1
expect editor.shape_preview is 0
pen down 1150 1100 0.6
wait 5
pen move 1148.630 1126.132 0.6
wait 5
pen move 1144.537 1151.978 0.6
wait 5
pen move 1137.764 1177.254 0.6
wait 5
pen move 1128.386 1201.684 0.6
wait 5
pen move 1116.506 1225.000 0.6
wait 5
pen move 1102.254 1246.946 0.6
wait 5
pen move 1085.786 1267.283 0.6
wait 5
pen move 1067.283 1285.786 0.6
wait 5
pen move 1046.946 1302.254 0.6
wait 5
pen move 1025.000 1316.506 0.6
wait 5
pen move 1001.684 1328.386 0.6
wait 5
pen move 977.254 1337.764 0.6
wait 5
pen move 951.978 1344.537 0.6
wait 5
pen move 926.132 1348.630 0.6
wait 5
pen move 900.000 1350.000 0.6
wait 5
pen move 873.868 1348.630 0.6
wait 5
pen move 848.022 1344.537 0.6
wait 5
pen move 822.746 1337.764 0.6
wait 5
pen move 798.316 1328.386 0.6
wait 5
pen move 775.000 1316.506 0.6
wait 5
pen move 753.054 1302.254 0.6
wait 5
pen move 732.717 1285.786 0.6
wait 5
pen move 714.214 1267.283 0.6
wait 5
pen move 697.746 1246.946 0.6
wait 5
pen move 683.494 1225.000 0.6
wait 5
pen move 671.614 1201.684 0.6
wait 5
pen move 662.236 1177.254 0.6
wait 5
pen move 655.463 1151.978 0.6
wait 5
pen move 651.370 1126.132 0.6
wait 5
pen move 650.000 1100.000 0.6
wait 5
pen move 651.370 1073.868 0.6
wait 5
pen move 655.463 1048.022 0.6
wait 5
pen move 662.236 1022.746 0.6
wait 5
pen move 671.614 998.316 0.6
wait 5
pen move 683.494 975.000 0.6
wait 5
pen move 697.746 953.054 0.6
wait 5
pen move 714.214 932.717 0.6
wait 5
pen move 732.717 914.214 0.6
wait 5
pen move 753.054 897.746 0.6
wait 5
pen move 775.000 883.494 0.6
wait 5
pen move 798.316 871.614 0.6
wait 5
pen move 822.746 862.236 0.6
wait 5
pen move 848.022 855.463 0.6
wait 5
pen move 873.868 851.370 0.6
wait 5
pen move 900.000 850.000 0.6
wait 5
pen move 926.132 851.370 0.6
wait 5
pen move 951.978 855.463 0.6
wait 5
pen move 977.254 862.236 0.6
wait 5
pen move 1001.684 871.614 0.6
wait 5
pen move 1025.000 883.494 0.6
wait 5
pen move 1046.946 897.746 0.6
wait 5
pen move 1067.283 914.214 0.6
wait 5
pen move 1085.786 932.717 0.6
wait 5
pen move 1102.254 953.054 0.6
wait 5
pen move 1116.506 975.000 0.6
wait 5
pen move 1128.386 998.316 0.6
wait 5
pen move 1137.764 1022.746 0.6
wait 5
pen move 1144.537 1048.022 0.6
wait 5
pen move 1148.630 1073.868 0.6
wait 5
pen move 1150.000 1100.000 0.6
wait 700
expect editor.shape_preview is circle
expect editor.strokes is 0
pen up 1150 1100
pen leave
wait 1000
expect editor.shape_snaps is 1
expect editor.stroke_shape is circle
expect editor.strokes is 1
expect editor.undo_kind is add
expect editor.live_matches_render is 1
shot 01-circle
tap panel undo
expect editor.strokes is 0
tap panel redo
expect editor.strokes is 1
expect editor.live_matches_render is 1
