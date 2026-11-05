# settings: mirror_pdf=0
# rust-only
# generate: tests/lasso_drag_fixture.py
# env: GRIMOIRE_SIM_ADDRS=en0=192.168.0.20
# A heavy selection dragged with Live view on (2.1.0, beside z_lasso_19_drag_dense, which has it off): every
# change a drag makes on the screen goes out to Live view's frame with its present, the bar taken away, each
# strip of the outline put back and drawn, so what a browser is sent is what the Kindle shows (live.frame.stale
# 0, the panel left out: the frame holds the page under it); the drag's copy of the screen never takes Live
# view's composing without the panel, and the lift is as with it off.
clock 1700000000
wait 300
tap 1345 95
wait 300
expect screen is share
tap 318 2100
wait 300
expect live.on is 1
tap 1538 2100
wait 300
expect screen is library
expect live.on is 1
tap 326 608
wait 2000
expect screen is editor
expect editor.strokes is 12
expect live.frame.stale is 0
tap panel tool-picker
tap panel picker.select
wait 100
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
expect editor.selected is 12
tap panel select.copy
wait 200
tap panel select.paste
wait 500
expect editor.strokes is 24
expect editor.sel_box is 517 517 193 186
expect live.frame.stale is 0
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
expect editor.drag_shown is 105 -15
expect editor.drag_shows is 1
expect live.frame.stale is 0
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
expect editor.drag_shown is 210 -30
expect editor.drag_shows is 2
expect live.frame.stale is 0
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
wait 200
expect editor.drag_shown is 385 -55
expect editor.drag_inked is 4
expect live.frame.stale is 0
expect editor.layers is ok
pen up 997.00 550.00
wait 300
expect editor.undo_kind is lasso
expect editor.sel_box is 902 462 193 186
expect editor.live_matches_render is 1
expect live.frame.stale is 0
quit
