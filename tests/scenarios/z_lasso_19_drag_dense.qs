# settings: mirror_pdf=0
# rust-only
# generate: tests/lasso_drag_fixture.py
# A drag of a heavy selection (2.1.0; the owner, 2026-11-05: "the copy paste lasso function is horrifically slow
# on drag with anything containing colored in or multiple strokes"): three hexagons coloured in with 12 Pencil
# strokes of 9,852 points, his page's statistics (generated, never his drawing), copied and pasted 3 mm off over
# the originals as on his Scribe, then dragged. A pen sample only records the place; the editor's tick shows the
# newest place (at most every 120 ms, or once the pen pauses): 60 samples in one go are one show. The first move
# takes the bar away (its rect a DU of its own) and the outline's strips with it; each show puts back the old
# strips from the drag's copy of the screen and draws the new ones, each strip a DU of its own: never the bar,
# never the box's inside, and the selection's ink is walked (its halo drawn) only where the first move takes
# the outline away, its four strips. The old code composed and refreshed the box with the bar, 1680 x 365 px,
# at every sample, its halo drawn each time. The lift is as before: the strokes moved by the newest place, one
# undo step, one grayscale refresh; the page drawn live equals a fresh drawing, and undo puts it back exactly.
clock 1700000000
wait 300
tap 326 608
wait 2000
expect screen is editor
expect editor.strokes is 12
tap panel tool-picker
tap panel picker.select
wait 100
expect editor.select is armed
pen down 440 440
pen move 580 430
pen move 720 440
pen move 730 570
pen move 720 700
pen move 580 710
pen move 440 700
pen move 430 570
pen up 440 450
wait 200
expect editor.select is selected
expect editor.selected is 12
expect editor.sel_box is 482 482 193 186
tap panel select.copy
wait 200
tap panel select.paste
wait 500
expect editor.strokes is 24
expect editor.selected is 12
expect editor.sel_box is 517 517 193 186
expect editor.page_sig_keep is kept
# (the launch repaints the whole screen 1.2 s and 3.5 s after it starts: past both before counting)
wait 800
expect display.presents not -1
# 60 samples at once, 1.75 px right and 0.25 px up each: one turn of the main loop, one show of the newest place.
# The bar goes, the outline's strips and the selection's four resize handles where it was, and the outline at
# 105 -15 with its handles: thirteen DU, none of them the box's inside.
# (The first six are within the drag's 1 mm slop, 11.8 px: they move nothing, so 54 samples changed the place.)
pen down 612.00 605.00 0.5
pen move 613.75 604.75 0.5
pen move 615.50 604.50 0.5
pen move 617.25 604.25 0.5
pen move 619.00 604.00 0.5
pen move 620.75 603.75 0.5
pen move 622.50 603.50 0.5
pen move 624.25 603.25 0.5
pen move 626.00 603.00 0.5
pen move 627.75 602.75 0.5
pen move 629.50 602.50 0.5
pen move 631.25 602.25 0.5
pen move 633.00 602.00 0.5
pen move 634.75 601.75 0.5
pen move 636.50 601.50 0.5
pen move 638.25 601.25 0.5
pen move 640.00 601.00 0.5
pen move 641.75 600.75 0.5
pen move 643.50 600.50 0.5
pen move 645.25 600.25 0.5
pen move 647.00 600.00 0.5
pen move 648.75 599.75 0.5
pen move 650.50 599.50 0.5
pen move 652.25 599.25 0.5
pen move 654.00 599.00 0.5
pen move 655.75 598.75 0.5
pen move 657.50 598.50 0.5
pen move 659.25 598.25 0.5
pen move 661.00 598.00 0.5
pen move 662.75 597.75 0.5
pen move 664.50 597.50 0.5
pen move 666.25 597.25 0.5
pen move 668.00 597.00 0.5
pen move 669.75 596.75 0.5
pen move 671.50 596.50 0.5
pen move 673.25 596.25 0.5
pen move 675.00 596.00 0.5
pen move 676.75 595.75 0.5
pen move 678.50 595.50 0.5
pen move 680.25 595.25 0.5
pen move 682.00 595.00 0.5
pen move 683.75 594.75 0.5
pen move 685.50 594.50 0.5
pen move 687.25 594.25 0.5
pen move 689.00 594.00 0.5
pen move 690.75 593.75 0.5
pen move 692.50 593.50 0.5
pen move 694.25 593.25 0.5
pen move 696.00 593.00 0.5
pen move 697.75 592.75 0.5
pen move 699.50 592.50 0.5
pen move 701.25 592.25 0.5
pen move 703.00 592.00 0.5
pen move 704.75 591.75 0.5
pen move 706.50 591.50 0.5
pen move 708.25 591.25 0.5
pen move 710.00 591.00 0.5
pen move 711.75 590.75 0.5
pen move 713.50 590.50 0.5
pen move 715.25 590.25 0.5
pen move 717.00 590.00 0.5
wait 200
expect editor.select is dragging
expect editor.drag_samples is 54
expect editor.drag_shows is 1
expect editor.drag_shown is 105 -15
expect editor.drag_copy is 1
expect editor.drag_inked is 6
expect display.presents is fast 10 733 1680 130, fast 510 510 207 8, fast 510 702 207 8, fast 510 518 8 184, fast 709 518 8 184, fast 503 503 28 28, fast 696 503 28 28, fast 503 689 28 28, and 5 more
# the screen is what its layers make of it: the outline at the place shown, no bar (this composes the whole
# screen again, the halo with it: one walk more)
expect editor.layers is ok
expect editor.drag_inked is 7
shot 01-dragging
# 60 more: one show, eight strips (the old place's four, the new place's four), the ink not walked
pen move 718.75 589.75 0.5
pen move 720.50 589.50 0.5
pen move 722.25 589.25 0.5
pen move 724.00 589.00 0.5
pen move 725.75 588.75 0.5
pen move 727.50 588.50 0.5
pen move 729.25 588.25 0.5
pen move 731.00 588.00 0.5
pen move 732.75 587.75 0.5
pen move 734.50 587.50 0.5
pen move 736.25 587.25 0.5
pen move 738.00 587.00 0.5
pen move 739.75 586.75 0.5
pen move 741.50 586.50 0.5
pen move 743.25 586.25 0.5
pen move 745.00 586.00 0.5
pen move 746.75 585.75 0.5
pen move 748.50 585.50 0.5
pen move 750.25 585.25 0.5
pen move 752.00 585.00 0.5
pen move 753.75 584.75 0.5
pen move 755.50 584.50 0.5
pen move 757.25 584.25 0.5
pen move 759.00 584.00 0.5
pen move 760.75 583.75 0.5
pen move 762.50 583.50 0.5
pen move 764.25 583.25 0.5
pen move 766.00 583.00 0.5
pen move 767.75 582.75 0.5
pen move 769.50 582.50 0.5
pen move 771.25 582.25 0.5
pen move 773.00 582.00 0.5
pen move 774.75 581.75 0.5
pen move 776.50 581.50 0.5
pen move 778.25 581.25 0.5
pen move 780.00 581.00 0.5
pen move 781.75 580.75 0.5
pen move 783.50 580.50 0.5
pen move 785.25 580.25 0.5
pen move 787.00 580.00 0.5
pen move 788.75 579.75 0.5
pen move 790.50 579.50 0.5
pen move 792.25 579.25 0.5
pen move 794.00 579.00 0.5
pen move 795.75 578.75 0.5
pen move 797.50 578.50 0.5
pen move 799.25 578.25 0.5
pen move 801.00 578.00 0.5
pen move 802.75 577.75 0.5
pen move 804.50 577.50 0.5
pen move 806.25 577.25 0.5
pen move 808.00 577.00 0.5
pen move 809.75 576.75 0.5
pen move 811.50 576.50 0.5
pen move 813.25 576.25 0.5
pen move 815.00 576.00 0.5
pen move 816.75 575.75 0.5
pen move 818.50 575.50 0.5
pen move 820.25 575.25 0.5
pen move 822.00 575.00 0.5
wait 200
expect editor.drag_samples is 114
expect editor.drag_shows is 2
expect editor.drag_shown is 210 -30
expect editor.drag_inked is 7
expect display.presents is fast 615 495 207 8, fast 615 687 207 8, fast 615 503 8 184, fast 814 503 8 184, fast 720 480 207 8, fast 720 672 207 8, fast 720 488 8 184, fast 919 488 8 184
# then as the pen reports on the Kindle, a sample every few ms (200 of them): the first is due at once (its old
# and new strips overlap, so each side is one DU), the rest every 120 ms or so as the machine's time goes; the
# ink is never walked
pen move 823.75 574.75 0.5
wait 4
pen move 825.50 574.50 0.5
wait 4
pen move 827.25 574.25 0.5
wait 4
pen move 829.00 574.00 0.5
wait 4
pen move 830.75 573.75 0.5
wait 4
pen move 832.50 573.50 0.5
wait 4
pen move 834.25 573.25 0.5
wait 4
pen move 836.00 573.00 0.5
wait 4
pen move 837.75 572.75 0.5
wait 4
pen move 839.50 572.50 0.5
wait 4
pen move 841.25 572.25 0.5
wait 4
pen move 843.00 572.00 0.5
wait 4
pen move 844.75 571.75 0.5
wait 4
pen move 846.50 571.50 0.5
wait 4
pen move 848.25 571.25 0.5
wait 4
pen move 850.00 571.00 0.5
wait 4
pen move 851.75 570.75 0.5
wait 4
pen move 853.50 570.50 0.5
wait 4
pen move 855.25 570.25 0.5
wait 4
pen move 857.00 570.00 0.5
wait 4
pen move 858.75 569.75 0.5
wait 4
pen move 860.50 569.50 0.5
wait 4
pen move 862.25 569.25 0.5
wait 4
pen move 864.00 569.00 0.5
wait 4
pen move 865.75 568.75 0.5
wait 4
pen move 867.50 568.50 0.5
wait 4
pen move 869.25 568.25 0.5
wait 4
pen move 871.00 568.00 0.5
wait 4
pen move 872.75 567.75 0.5
wait 4
pen move 874.50 567.50 0.5
wait 4
pen move 876.25 567.25 0.5
wait 4
pen move 878.00 567.00 0.5
wait 4
pen move 879.75 566.75 0.5
wait 4
pen move 881.50 566.50 0.5
wait 4
pen move 883.25 566.25 0.5
wait 4
pen move 885.00 566.00 0.5
wait 4
pen move 886.75 565.75 0.5
wait 4
pen move 888.50 565.50 0.5
wait 4
pen move 890.25 565.25 0.5
wait 4
pen move 892.00 565.00 0.5
wait 4
pen move 893.75 564.75 0.5
wait 4
pen move 895.50 564.50 0.5
wait 4
pen move 897.25 564.25 0.5
wait 4
pen move 899.00 564.00 0.5
wait 4
pen move 900.75 563.75 0.5
wait 4
pen move 902.50 563.50 0.5
wait 4
pen move 904.25 563.25 0.5
wait 4
pen move 906.00 563.00 0.5
wait 4
pen move 907.75 562.75 0.5
wait 4
pen move 909.50 562.50 0.5
wait 4
pen move 911.25 562.25 0.5
wait 4
pen move 913.00 562.00 0.5
wait 4
pen move 914.75 561.75 0.5
wait 4
pen move 916.50 561.50 0.5
wait 4
pen move 918.25 561.25 0.5
wait 4
pen move 920.00 561.00 0.5
wait 4
pen move 921.75 560.75 0.5
wait 4
pen move 923.50 560.50 0.5
wait 4
pen move 925.25 560.25 0.5
wait 4
pen move 927.00 560.00 0.5
wait 4
pen move 928.75 559.75 0.5
wait 4
pen move 930.50 559.50 0.5
wait 4
pen move 932.25 559.25 0.5
wait 4
pen move 934.00 559.00 0.5
wait 4
pen move 935.75 558.75 0.5
wait 4
pen move 937.50 558.50 0.5
wait 4
pen move 939.25 558.25 0.5
wait 4
pen move 941.00 558.00 0.5
wait 4
pen move 942.75 557.75 0.5
wait 4
pen move 944.50 557.50 0.5
wait 4
pen move 946.25 557.25 0.5
wait 4
pen move 948.00 557.00 0.5
wait 4
pen move 949.75 556.75 0.5
wait 4
pen move 951.50 556.50 0.5
wait 4
pen move 953.25 556.25 0.5
wait 4
pen move 955.00 556.00 0.5
wait 4
pen move 956.75 555.75 0.5
wait 4
pen move 958.50 555.50 0.5
wait 4
pen move 960.25 555.25 0.5
wait 4
pen move 962.00 555.00 0.5
wait 4
pen move 963.75 554.75 0.5
wait 4
pen move 965.50 554.50 0.5
wait 4
pen move 967.25 554.25 0.5
wait 4
pen move 969.00 554.00 0.5
wait 4
pen move 970.75 553.75 0.5
wait 4
pen move 972.50 553.50 0.5
wait 4
pen move 974.25 553.25 0.5
wait 4
pen move 976.00 553.00 0.5
wait 4
pen move 977.75 552.75 0.5
wait 4
pen move 979.50 552.50 0.5
wait 4
pen move 981.25 552.25 0.5
wait 4
pen move 983.00 552.00 0.5
wait 4
pen move 984.75 551.75 0.5
wait 4
pen move 986.50 551.50 0.5
wait 4
pen move 988.25 551.25 0.5
wait 4
pen move 990.00 551.00 0.5
wait 4
pen move 991.75 550.75 0.5
wait 4
pen move 993.50 550.50 0.5
wait 4
pen move 995.25 550.25 0.5
wait 4
pen move 997.00 550.00 0.5
wait 4
pen move 998.75 549.75 0.5
wait 4
pen move 1000.50 549.50 0.5
wait 4
pen move 1002.25 549.25 0.5
wait 4
pen move 1004.00 549.00 0.5
wait 4
pen move 1005.75 548.75 0.5
wait 4
pen move 1007.50 548.50 0.5
wait 4
pen move 1009.25 548.25 0.5
wait 4
pen move 1011.00 548.00 0.5
wait 4
pen move 1012.75 547.75 0.5
wait 4
pen move 1014.50 547.50 0.5
wait 4
pen move 1016.25 547.25 0.5
wait 4
pen move 1018.00 547.00 0.5
wait 4
pen move 1019.75 546.75 0.5
wait 4
pen move 1021.50 546.50 0.5
wait 4
pen move 1023.25 546.25 0.5
wait 4
pen move 1025.00 546.00 0.5
wait 4
pen move 1026.75 545.75 0.5
wait 4
pen move 1028.50 545.50 0.5
wait 4
pen move 1030.25 545.25 0.5
wait 4
pen move 1032.00 545.00 0.5
wait 4
pen move 1033.75 544.75 0.5
wait 4
pen move 1035.50 544.50 0.5
wait 4
pen move 1037.25 544.25 0.5
wait 4
pen move 1039.00 544.00 0.5
wait 4
pen move 1040.75 543.75 0.5
wait 4
pen move 1042.50 543.50 0.5
wait 4
pen move 1044.25 543.25 0.5
wait 4
pen move 1046.00 543.00 0.5
wait 4
pen move 1047.75 542.75 0.5
wait 4
pen move 1049.50 542.50 0.5
wait 4
pen move 1051.25 542.25 0.5
wait 4
pen move 1053.00 542.00 0.5
wait 4
pen move 1054.75 541.75 0.5
wait 4
pen move 1056.50 541.50 0.5
wait 4
pen move 1058.25 541.25 0.5
wait 4
pen move 1060.00 541.00 0.5
wait 4
pen move 1061.75 540.75 0.5
wait 4
pen move 1063.50 540.50 0.5
wait 4
pen move 1065.25 540.25 0.5
wait 4
pen move 1067.00 540.00 0.5
wait 4
pen move 1068.75 539.75 0.5
wait 4
pen move 1070.50 539.50 0.5
wait 4
pen move 1072.25 539.25 0.5
wait 4
pen move 1074.00 539.00 0.5
wait 4
pen move 1075.75 538.75 0.5
wait 4
pen move 1077.50 538.50 0.5
wait 4
pen move 1079.25 538.25 0.5
wait 4
pen move 1081.00 538.00 0.5
wait 4
pen move 1082.75 537.75 0.5
wait 4
pen move 1084.50 537.50 0.5
wait 4
pen move 1086.25 537.25 0.5
wait 4
pen move 1088.00 537.00 0.5
wait 4
pen move 1089.75 536.75 0.5
wait 4
pen move 1091.50 536.50 0.5
wait 4
pen move 1093.25 536.25 0.5
wait 4
pen move 1095.00 536.00 0.5
wait 4
pen move 1096.75 535.75 0.5
wait 4
pen move 1098.50 535.50 0.5
wait 4
pen move 1100.25 535.25 0.5
wait 4
pen move 1102.00 535.00 0.5
wait 4
pen move 1103.75 534.75 0.5
wait 4
pen move 1105.50 534.50 0.5
wait 4
pen move 1107.25 534.25 0.5
wait 4
pen move 1109.00 534.00 0.5
wait 4
pen move 1110.75 533.75 0.5
wait 4
pen move 1112.50 533.50 0.5
wait 4
pen move 1114.25 533.25 0.5
wait 4
pen move 1116.00 533.00 0.5
wait 4
pen move 1117.75 532.75 0.5
wait 4
pen move 1119.50 532.50 0.5
wait 4
pen move 1121.25 532.25 0.5
wait 4
pen move 1123.00 532.00 0.5
wait 4
pen move 1124.75 531.75 0.5
wait 4
pen move 1126.50 531.50 0.5
wait 4
pen move 1128.25 531.25 0.5
wait 4
pen move 1130.00 531.00 0.5
wait 4
pen move 1131.75 530.75 0.5
wait 4
pen move 1133.50 530.50 0.5
wait 4
pen move 1135.25 530.25 0.5
wait 4
pen move 1137.00 530.00 0.5
wait 4
pen move 1138.75 529.75 0.5
wait 4
pen move 1140.50 529.50 0.5
wait 4
pen move 1142.25 529.25 0.5
wait 4
pen move 1144.00 529.00 0.5
wait 4
pen move 1145.75 528.75 0.5
wait 4
pen move 1147.50 528.50 0.5
wait 4
pen move 1149.25 528.25 0.5
wait 4
pen move 1151.00 528.00 0.5
wait 4
pen move 1152.75 527.75 0.5
wait 4
pen move 1154.50 527.50 0.5
wait 4
pen move 1156.25 527.25 0.5
wait 4
pen move 1158.00 527.00 0.5
wait 4
pen move 1159.75 526.75 0.5
wait 4
pen move 1161.50 526.50 0.5
wait 4
pen move 1163.25 526.25 0.5
wait 4
pen move 1165.00 526.00 0.5
wait 4
pen move 1166.75 525.75 0.5
wait 4
pen move 1168.50 525.50 0.5
wait 4
pen move 1170.25 525.25 0.5
wait 4
pen move 1172.00 525.00 0.5
wait 4
wait 200
expect editor.drag_samples is 314
expect editor.drag_shown is 560 -80
expect editor.drag_inked is 7
expect display.presents has fast 720 480 209 8, fast 720 672 209 8, fast 720 488 10 184, fast 919 488 10 184
expect editor.layers is ok
shot 02-dragged
pen up 1172.00 525.00
wait 300
# the lift as before: moved by the newest place, one undo step, one grayscale refresh (the whole way, the bar
# back under the box and the panel), the copy of the screen gone
expect editor.select is selected
expect editor.strokes is 24
expect editor.undo_kind is lasso
expect editor.sel_box is 1077 437 193 186
expect editor.drag_copy is 0
expect display.presents is ui 10 220 1822 1734
expect editor.live_matches_render is 1
expect editor.layers is ok
shot 03-moved
# one undo puts the page back exactly as the paste left it
tap panel select.done
wait 100
expect editor.select is off
pentap panel undo
wait 100
expect editor.strokes is 24
expect editor.page_sig_same is 1
expect editor.live_matches_render is 1
pentap panel redo
wait 100
expect editor.page_sig_same is 0
expect editor.live_matches_render is 1
quit
