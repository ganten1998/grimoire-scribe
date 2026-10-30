# rust-only
# settings: mirror_pdf=0
# settings: link_areas=1
# settings: shape_quads=0
# env: GRIMOIRE_CHECK_FIT=1
wait 200
tap library new
expect library.paper is 5
tap library create
expect editor.paper is 5
# Change only the current page, then create a linked page.
pentap panel paper
pentap 511 1035
expect editor.modal is none
expect editor.paper is 10
pentap panel links
pen down 780.000 750.000 0.6
pen move 779.315 765.688 0.6
pen move 777.265 781.257 0.6
pen move 773.867 796.587 0.6
pen move 769.145 811.564 0.6
pen move 763.135 826.071 0.6
pen move 755.885 840.000 0.6
pen move 747.447 853.244 0.6
pen move 737.888 865.702 0.6
pen move 727.279 877.279 0.6
pen move 715.702 887.888 0.6
pen move 703.244 897.447 0.6
pen move 690.000 905.885 0.6
pen move 676.071 913.135 0.6
pen move 661.564 919.145 0.6
pen move 646.587 923.867 0.6
pen move 631.257 927.265 0.6
pen move 615.688 929.315 0.6
pen move 600.000 930.000 0.6
pen move 584.312 929.315 0.6
pen move 568.743 927.265 0.6
pen move 553.413 923.867 0.6
pen move 538.436 919.145 0.6
pen move 523.929 913.135 0.6
pen move 510.000 905.885 0.6
pen move 496.756 897.447 0.6
pen move 484.298 887.888 0.6
pen move 472.721 877.279 0.6
pen move 462.112 865.702 0.6
pen move 452.553 853.244 0.6
pen move 444.115 840.000 0.6
pen move 436.865 826.071 0.6
pen move 430.855 811.564 0.6
pen move 426.133 796.587 0.6
pen move 422.735 781.257 0.6
pen move 420.685 765.688 0.6
pen move 420.000 750.000 0.6
pen move 420.685 734.312 0.6
pen move 422.735 718.743 0.6
pen move 426.133 703.413 0.6
pen move 430.855 688.436 0.6
pen move 436.865 673.929 0.6
pen move 444.115 660.000 0.6
pen move 452.553 646.756 0.6
pen move 462.112 634.298 0.6
pen move 472.721 622.721 0.6
pen move 484.298 612.112 0.6
pen move 496.756 602.553 0.6
pen move 510.000 594.115 0.6
pen move 523.929 586.865 0.6
pen move 538.436 580.855 0.6
pen move 553.413 576.133 0.6
pen move 568.743 572.735 0.6
pen move 584.312 570.685 0.6
pen move 600.000 570.000 0.6
pen move 615.688 570.685 0.6
pen move 631.257 572.735 0.6
pen move 646.587 576.133 0.6
pen move 661.564 580.855 0.6
pen move 676.071 586.865 0.6
pen move 690.000 594.115 0.6
pen move 703.244 602.553 0.6
pen move 715.702 612.112 0.6
pen move 727.279 622.721 0.6
pen move 737.888 634.298 0.6
pen move 747.447 646.756 0.6
pen move 755.885 660.000 0.6
pen move 763.135 673.929 0.6
pen move 769.145 688.436 0.6
pen move 773.867 703.413 0.6
pen move 777.265 718.743 0.6
pen move 779.315 734.312 0.6
pen move 780.000 750.000 0.6
wait 700
expect editor.link_shape is circle
expect editor.strokes is 0
shot 01-circle-snap
pen up
pen leave
expect editor.linkto is pick
tap panel linkto.chip.1
expect editor.keyboard is Name the new page
tap 1643 2365
expect editor.pages is 2
expect editor.modal is none
expect editor.strokes is 0
wait 1100
expect editor.page is 1
expect editor.links is 1 1
# The old bounding-box corner must not follow a circular link.
tap 430 580
expect editor.page is 1
tap 600 750
expect editor.page is 2
expect editor.paper is 10
pentap panel previous
pentap panel links
pen down 1520.000 750.000 0.6
pen move 1518.973 763.073 0.6
pen move 1515.898 776.047 0.6
pen move 1510.800 788.823 0.6
pen move 1503.717 801.303 0.6
pen move 1494.703 813.393 0.6
pen move 1483.827 825.000 0.6
pen move 1471.171 836.036 0.6
pen move 1456.832 846.418 0.6
pen move 1440.919 856.066 0.6
pen move 1423.553 864.907 0.6
pen move 1404.866 872.873 0.6
pen move 1385.000 879.904 0.6
pen move 1364.107 885.946 0.6
pen move 1342.345 890.954 0.6
pen move 1319.881 894.889 0.6
pen move 1296.885 897.721 0.6
pen move 1273.532 899.429 0.6
pen move 1250.000 900.000 0.6
pen move 1226.468 899.429 0.6
pen move 1203.115 897.721 0.6
pen move 1180.119 894.889 0.6
pen move 1157.655 890.954 0.6
pen move 1135.893 885.946 0.6
pen move 1115.000 879.904 0.6
pen move 1095.134 872.873 0.6
pen move 1076.447 864.907 0.6
pen move 1059.081 856.066 0.6
pen move 1043.168 846.418 0.6
pen move 1028.829 836.036 0.6
pen move 1016.173 825.000 0.6
pen move 1005.297 813.393 0.6
pen move 996.283 801.303 0.6
pen move 989.200 788.823 0.6
pen move 984.102 776.047 0.6
pen move 981.027 763.073 0.6
pen move 980.000 750.000 0.6
pen move 981.027 736.927 0.6
pen move 984.102 723.953 0.6
pen move 989.200 711.177 0.6
pen move 996.283 698.697 0.6
pen move 1005.297 686.607 0.6
pen move 1016.173 675.000 0.6
pen move 1028.829 663.964 0.6
pen move 1043.168 653.582 0.6
pen move 1059.081 643.934 0.6
pen move 1076.447 635.093 0.6
pen move 1095.134 627.127 0.6
pen move 1115.000 620.096 0.6
pen move 1135.893 614.054 0.6
pen move 1157.655 609.046 0.6
pen move 1180.119 605.111 0.6
pen move 1203.115 602.279 0.6
pen move 1226.468 600.571 0.6
pen move 1250.000 600.000 0.6
pen move 1273.532 600.571 0.6
pen move 1296.885 602.279 0.6
pen move 1319.881 605.111 0.6
pen move 1342.345 609.046 0.6
pen move 1364.107 614.054 0.6
pen move 1385.000 620.096 0.6
pen move 1404.866 627.127 0.6
pen move 1423.553 635.093 0.6
pen move 1440.919 643.934 0.6
pen move 1456.832 653.582 0.6
pen move 1471.171 663.964 0.6
pen move 1483.827 675.000 0.6
pen move 1494.703 686.607 0.6
pen move 1503.717 698.697 0.6
pen move 1510.800 711.177 0.6
pen move 1515.898 723.953 0.6
pen move 1518.973 736.927 0.6
pen move 1520.000 750.000 0.6
wait 700
expect editor.link_shape is ellipse
expect editor.strokes is 0
shot 02-ellipse-snap
pen up
pen leave
expect editor.linkto is pick
tap panel linkto.pages
tap panel pages.page.2
expect editor.modal is none
expect editor.strokes is 0
wait 1100
pentap panel links
pen down 500.000 1250.000 0.6
pen move 525.000 1291.667 0.6
pen move 550.000 1333.333 0.6
pen move 575.000 1375.000 0.6
pen move 600.000 1416.667 0.6
pen move 625.000 1458.333 0.6
pen move 650.000 1500.000 0.6
pen move 675.000 1541.667 0.6
pen move 700.000 1583.333 0.6
pen move 725.000 1625.000 0.6
pen move 750.000 1666.667 0.6
pen move 775.000 1708.333 0.6
pen move 800.000 1750.000 0.6
pen move 750.000 1750.000 0.6
pen move 700.000 1750.000 0.6
pen move 650.000 1750.000 0.6
pen move 600.000 1750.000 0.6
pen move 550.000 1750.000 0.6
pen move 500.000 1750.000 0.6
pen move 450.000 1750.000 0.6
pen move 400.000 1750.000 0.6
pen move 350.000 1750.000 0.6
pen move 300.000 1750.000 0.6
pen move 250.000 1750.000 0.6
pen move 200.000 1750.000 0.6
pen move 225.000 1708.333 0.6
pen move 250.000 1666.667 0.6
pen move 275.000 1625.000 0.6
pen move 300.000 1583.333 0.6
pen move 325.000 1541.667 0.6
pen move 350.000 1500.000 0.6
pen move 375.000 1458.333 0.6
pen move 400.000 1416.667 0.6
pen move 425.000 1375.000 0.6
pen move 450.000 1333.333 0.6
pen move 475.000 1291.667 0.6
pen move 500.000 1250.000 0.6
wait 700
expect editor.link_shape is triangle
expect editor.strokes is 0
shot 03-triangle-snap
pen up
pen leave
expect editor.linkto is pick
tap panel linkto.pages
tap panel pages.page.2
expect editor.modal is none
expect editor.strokes is 0
wait 1100
pentap panel links
pen down 1100.000 1250.000 0.6
pen move 1125.000 1250.000 0.6
pen move 1150.000 1250.000 0.6
pen move 1175.000 1250.000 0.6
pen move 1200.000 1250.000 0.6
pen move 1225.000 1250.000 0.6
pen move 1250.000 1250.000 0.6
pen move 1275.000 1250.000 0.6
pen move 1300.000 1250.000 0.6
pen move 1325.000 1250.000 0.6
pen move 1350.000 1250.000 0.6
pen move 1375.000 1250.000 0.6
pen move 1400.000 1250.000 0.6
pen move 1412.500 1291.667 0.6
pen move 1425.000 1333.333 0.6
pen move 1437.500 1375.000 0.6
pen move 1450.000 1416.667 0.6
pen move 1462.500 1458.333 0.6
pen move 1475.000 1500.000 0.6
pen move 1487.500 1541.667 0.6
pen move 1500.000 1583.333 0.6
pen move 1512.500 1625.000 0.6
pen move 1525.000 1666.667 0.6
pen move 1537.500 1708.333 0.6
pen move 1550.000 1750.000 0.6
pen move 1500.000 1750.000 0.6
pen move 1450.000 1750.000 0.6
pen move 1400.000 1750.000 0.6
pen move 1350.000 1750.000 0.6
pen move 1300.000 1750.000 0.6
pen move 1250.000 1750.000 0.6
pen move 1200.000 1750.000 0.6
pen move 1150.000 1750.000 0.6
pen move 1100.000 1750.000 0.6
pen move 1050.000 1750.000 0.6
pen move 1000.000 1750.000 0.6
pen move 950.000 1750.000 0.6
pen move 962.500 1708.333 0.6
pen move 975.000 1666.667 0.6
pen move 987.500 1625.000 0.6
pen move 1000.000 1583.333 0.6
pen move 1012.500 1541.667 0.6
pen move 1025.000 1500.000 0.6
pen move 1037.500 1458.333 0.6
pen move 1050.000 1416.667 0.6
pen move 1062.500 1375.000 0.6
pen move 1075.000 1333.333 0.6
pen move 1087.500 1291.667 0.6
pen move 1100.000 1250.000 0.6
wait 700
expect editor.link_shape is quad
expect editor.strokes is 0
shot 04-quad-snap
pen up
pen leave
expect editor.linkto is pick
tap panel linkto.pages
tap panel pages.page.2
expect editor.modal is none
expect editor.strokes is 0
wait 1100
expect editor.links is 4 4
shot 05-unnumbered-shape-areas
pentap panel undo
expect editor.links is 3 3
pentap panel redo
expect editor.links is 4 4
pentap panel clear
expect editor.links is 0 0
pentap panel undo
expect editor.links is 4 4
expect editor.screen_composed is 1
pentap panel library
waitbg
tap 200 400
expect editor.links is 4 4
expect editor.paper is 10
tap 430 580
expect editor.page is 1
tap 600 750
expect editor.page is 2
expect editor.paper is 10
quit
