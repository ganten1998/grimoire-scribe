# rust-only
# root: z_bar_0_make
# settings: status_mode=1
# settings: shapes=1
wait 300
tap 200 400
wait 300
pen leave
wait 1100
expect editor.strokes is 0
pen down 1220.000 1000.000 0.6 0 0
pen move 1219.163 1019.174 0.6 0 0
pen move 1216.658 1038.203 0.6 0 0
pen move 1212.504 1056.940 0.6 0 0
pen move 1206.732 1075.244 0.6 0 0
pen move 1199.388 1092.976 0.6 0 0
pen move 1190.526 1110.000 0.6 0 0
pen move 1180.213 1126.187 0.6 0 0
pen move 1168.530 1141.413 0.6 0 0
pen move 1155.563 1155.563 0.6 0 0
pen move 1141.413 1168.530 0.6 0 0
pen move 1126.187 1180.213 0.6 0 0
pen move 1110.000 1190.526 0.6 0 0
pen move 1092.976 1199.388 0.6 0 0
pen move 1075.244 1206.732 0.6 0 0
pen move 1056.940 1212.504 0.6 0 0
pen move 1038.203 1216.658 0.6 0 0
pen move 1019.174 1219.163 0.6 0 0
pen move 1000.000 1220.000 0.6 0 0
pen move 980.826 1219.163 0.6 0 0
pen move 961.797 1216.658 0.6 0 0
pen move 943.060 1212.504 0.6 0 0
pen move 924.756 1206.732 0.6 0 0
pen move 907.024 1199.388 0.6 0 0
pen move 890.000 1190.526 0.6 0 0
pen move 873.813 1180.213 0.6 0 0
pen move 858.587 1168.530 0.6 0 0
pen move 844.437 1155.563 0.6 0 0
pen move 831.470 1141.413 0.6 0 0
pen move 819.787 1126.187 0.6 0 0
pen move 809.474 1110.000 0.6 0 0
pen move 800.612 1092.976 0.6 0 0
pen move 793.268 1075.244 0.6 0 0
pen move 787.496 1056.940 0.6 0 0
pen move 783.342 1038.203 0.6 0 0
pen move 780.837 1019.174 0.6 0 0
pen move 780.000 1000.000 0.6 0 0
pen move 780.837 980.826 0.6 0 0
pen move 783.342 961.797 0.6 0 0
pen move 787.496 943.060 0.6 0 0
pen move 793.268 924.756 0.6 0 0
pen move 800.612 907.024 0.6 0 0
pen move 809.474 890.000 0.6 0 0
pen move 819.787 873.813 0.6 0 0
pen move 831.470 858.587 0.6 0 0
pen move 844.437 844.437 0.6 0 0
pen move 858.587 831.470 0.6 0 0
pen move 873.813 819.787 0.6 0 0
pen move 890.000 809.474 0.6 0 0
pen move 907.024 800.612 0.6 0 0
pen move 924.756 793.268 0.6 0 0
pen move 943.060 787.496 0.6 0 0
pen move 961.797 783.342 0.6 0 0
pen move 980.826 780.837 0.6 0 0
pen move 1000.000 780.000 0.6 0 0
pen move 1019.174 780.837 0.6 0 0
pen move 1038.203 783.342 0.6 0 0
pen move 1056.940 787.496 0.6 0 0
pen move 1075.244 793.268 0.6 0 0
pen move 1092.976 800.612 0.6 0 0
pen move 1110.000 809.474 0.6 0 0
pen move 1126.187 819.787 0.6 0 0
pen move 1141.413 831.470 0.6 0 0
pen move 1155.563 844.437 0.6 0 0
pen move 1168.530 858.587 0.6 0 0
pen move 1180.213 873.813 0.6 0 0
pen move 1190.526 890.000 0.6 0 0
pen move 1199.388 907.024 0.6 0 0
pen move 1206.732 924.756 0.6 0 0
pen move 1212.504 943.060 0.6 0 0
pen move 1216.658 961.797 0.6 0 0
pen move 1219.163 980.826 0.6 0 0
pen move 1220.000 1000.000 0.6 0 0
wait 700
expect editor.shape_preview is circle
pen move 1255 1000 0.6 0 0
expect editor.shape_preview is 0
expect editor.stroke_shape is none
pen up
pen leave
wait 1100
expect editor.strokes is 1
expect editor.live_matches_render is 1
