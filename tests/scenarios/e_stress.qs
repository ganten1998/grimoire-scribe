# settings: status_mode=1
# stress: many strokes, big erases, deep undo/redo, pages, paper changes
# rust-only
wait 200
tap 965 95
wait 100
tap library paper.1
tap 1564 2254
wait 300
expect screen is editor
expect editor.paper is 1
pentap panel tool-picker
pentap 872 395
expect editor.tool is pencil
pen line 150 300 1400 320 40 0.20 0.9 0 0
pen line 150 345 1360 365 40 0.35 0.9 20 0
pen line 150 390 1320 410 40 0.50 0.9 40 0
pen line 150 435 1280 455 40 0.65 0.9 60 0
pen line 150 480 1240 500 40 0.80 0.9 0 0
pentap panel tool-picker
pentap 872 561
expect editor.tool is ballpoint
pen line 150 525 1200 545 40 0.20 0.9 20 0
pen line 150 570 1160 590 40 0.35 0.9 40 0
pen line 150 615 1400 635 40 0.50 0.9 60 0
pen line 150 660 1360 680 40 0.65 0.9 0 0
pen line 150 705 1320 725 40 0.80 0.9 20 0
pentap panel tool-picker
pentap 1188 395
expect editor.tool is mech
pen line 150 750 1280 770 40 0.20 0.9 40 0
pen line 150 795 1240 815 40 0.35 0.9 60 0
pen line 150 840 1200 860 40 0.50 0.9 0 0
pen line 150 885 1160 905 40 0.65 0.9 20 0
pen line 150 930 1400 950 40 0.80 0.9 40 0
pentap panel tool-picker
pentap 872 395
pen line 150 975 1360 995 40 0.20 0.9 60 0
pen line 150 1020 1320 1040 40 0.35 0.9 0 0
pen line 150 1065 1280 1085 40 0.50 0.9 20 0
pen line 150 1110 1240 1130 40 0.65 0.9 40 0
pen line 150 1155 1200 1175 40 0.80 0.9 60 0
pentap panel tool-picker
pentap 872 561
pen line 150 1200 1160 1220 40 0.20 0.9 0 0
pen line 150 1245 1400 1265 40 0.35 0.9 20 0
pen line 150 1290 1360 1310 40 0.50 0.9 40 0
pen line 150 1335 1320 1355 40 0.65 0.9 60 0
pen line 150 1380 1280 1400 40 0.80 0.9 0 0
pentap panel tool-picker
pentap 1188 395
pen line 150 1425 1240 1445 40 0.20 0.9 20 0
pen line 150 1470 1200 1490 40 0.35 0.9 40 0
pen line 150 1515 1160 1535 40 0.50 0.9 60 0
pen line 150 1560 1400 1580 40 0.65 0.9 0 0
pen line 150 1605 1360 1625 40 0.80 0.9 20 0
pentap panel tool-picker
pentap 872 395
pen line 150 1650 1320 1670 40 0.20 0.9 40 0
pen line 150 1695 1280 1715 40 0.35 0.9 60 0
pen line 150 1740 1240 1760 40 0.50 0.9 0 0
pen line 150 1785 1200 1805 40 0.65 0.9 20 0
pen line 150 1830 1160 1850 40 0.80 0.9 40 0
pentap panel tool-picker
pentap 872 561
pen line 150 1875 1400 1895 40 0.20 0.9 60 0
pen line 150 1920 1360 1940 40 0.35 0.9 0 0
pen line 150 1965 1320 1985 40 0.50 0.9 20 0
pen line 150 2010 1280 2030 40 0.65 0.9 40 0
pen line 150 2055 1240 2075 40 0.80 0.9 60 0
pentap panel tool-picker
pentap 1188 395
pen line 150 2100 1200 2120 40 0.20 0.9 0 0
pen line 150 2145 1160 2165 40 0.35 0.9 20 0
pen line 150 2190 1400 2210 40 0.50 0.9 40 0
pen line 150 2235 1360 2255 40 0.65 0.9 60 0
pen line 150 2280 1320 2300 40 0.80 0.9 0 0
wait 200
expect editor.strokes is 45
eraser line 300 250 320 2400 60
eraser line 550 250 570 2400 60
eraser line 800 250 820 2400 60
eraser line 1050 250 1070 2400 60
wait 200
shot 01-after-erase
expect editor.strokes is 0
pentap panel undo
pentap panel undo
pentap panel undo
pentap panel undo
pentap panel undo
pentap panel undo
pentap panel undo
pentap panel undo
pentap panel undo
pentap panel undo
pentap panel undo
pentap panel undo
wait 200
shot 02-undone
expect editor.strokes is 34
pentap panel redo
pentap panel redo
pentap panel redo
pentap panel redo
pentap panel redo
pentap panel redo
wait 200
shot 03-redone
expect editor.strokes is 40
pen line 200 200 1300 2300 80 0.5 0.5
pentap panel next
wait 150
expect editor.page is 2
expect editor.pages is 2
expect editor.strokes is 0
pen curve 300 600 40 60 0.6 3
pentap panel next
wait 150
expect editor.page is 3
pen curve 300 700 40 60 0.6 3
pentap panel next
wait 150
expect editor.page is 4
pen curve 300 800 40 60 0.6 3
pentap panel next
wait 150
expect editor.page is 5
expect editor.pages is 5
pen curve 300 900 40 60 0.6 3
pentap panel previous
wait 100
expect editor.page is 4
expect editor.strokes is 1
pentap panel previous
wait 100
pentap panel previous
wait 100
pentap panel previous
wait 100
expect editor.page is 1
expect editor.strokes is 41
pentap panel paper
wait 200
shot 04-paper-sheet
expect editor.modal is paper
pentap panel paper.3
wait 300
shot 05-paper-music
expect editor.modal is none
expect editor.paper is 3
pentap panel undo
wait 300
shot 06-paper-undone
expect editor.paper is 1
expect editor.strokes is 41
pentap panel collapse
wait 100
expect editor.panel is collapsed
tap panel expand
wait 100
expect editor.panel is expanded
pentap panel pen-feel
wait 200
expect screen is penset
tap 125 95
wait 200
expect screen is editor
pentap panel library
wait 600
shot 07-library
expect screen is library
expect library.items is 1
quit
