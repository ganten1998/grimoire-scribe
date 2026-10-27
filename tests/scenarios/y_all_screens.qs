# env: GRIMOIRE_CHECK_FIT=1
# env: GRIMOIRE_SIM_ADDRS=wlan0=192.168.0.20
# Every screen and sheet, upright and sideways, for the fit check: make l10n-check runs this in
# every language, in Gothic and in Classic, with GRIMOIRE_CHECK_FIT=1, and a run that cuts any
# text short to fit fails (its shots, build/shots/l10n/<lang>-<style>/, show translators every
# screen); make scenarios runs it in English and Gothic. Its taps land in both styles, and it
# checks only what does not depend on the language. It leaves every setting as it found it.
wait 100
expect screen is library
expect library.items is 0
expect library.hint is 1
shot 01-library-empty
# Settings: every row. The style row twice (the other style's sheet in between), the mirror,
# turning and pinching rows twice, the pen button row through its four texts.
tap 1535 95
wait 50
expect library.sheet is settings
shot 02-settings
tap 746 1930
wait 50
expect library.sheet is shortcuts
shot 02-shortcuts-disabled
tap 930 575
wait 50
expect settings.shortcuts_enabled is 1
key 305 0 1
wait 50
shot 02-shortcuts-input
hid attach held Mapper Pad
wait 50
shot 02-shortcuts-held
hid detach Mapper Pad
tap 930 1215
wait 50
expect library.sheet is buttons
shot 02-buttons
tap 930 682
wait 50
expect library.buttons_group is editing
shot 02-buttons-editing
tap 930 1056
wait 50
expect library.learning is hold-eraser
shot 02-buttons-learning-editing
tap 930 1056
wait 50
tap 930 1696
wait 50
tap 930 842
wait 50
expect library.buttons_group is tools
shot 02-buttons-tools
tap 930 1475
wait 50
expect library.learning is tool:calligraphy
shot 02-buttons-learning
tap 930 1475
wait 50
expect library.learning is
tap 930 1955
wait 50
expect library.learning is picker-previous
shot 02-buttons-learning-picker
tap 930 1955
wait 50
expect library.learning is
tap 930 2115
wait 50
expect library.buttons_group is
tap 930 1002
wait 50
expect library.buttons_group is pages
shot 02-buttons-pages
tap 930 629
wait 50
expect library.learning is previous-page
shot 02-buttons-learning-pages
tap 930 629
wait 50
tap 930 1589
wait 50
tap 930 1162
wait 50
expect library.buttons_group is menus
shot 02-buttons-menus
tap 930 1696
wait 50
expect library.buttons_group is
tap 930 1482
wait 50
expect library.sheet is shortcuts
tap 930 1375
wait 50
tap 930 575
wait 50
expect settings.shortcuts_enabled is 0
tap 930 1695
wait 50
expect library.sheet is none
tap 1535 95
wait 50
tap 1152 1930
wait 50
expect library.sheet is updates
shot 02-updates-no-kpm
tap 930 1375
wait 50
expect library.sheet is none
tap 1535 95
wait 50
expect library.sheet is settings
tap 500 1930
wait 50
expect library.sheet is koreader
shot 02-koreader-controls
tap 930 1450
wait 50
expect library.sheet is none
tap 1535 95
wait 50
expect library.sheet is settings
tap 930 930
wait 100
shot 03-settings-other-style
tap 930 930
wait 100
tap 930 1060
wait 50
tap 930 1060
wait 50
tap 930 1190
wait 50
shot 04-settings-pen-button-3
tap 930 1190
wait 50
shot 04-settings-pen-button-0
tap 930 1190
wait 50
shot 05-settings-pen-button-1
tap 930 1190
wait 50
tap 930 1320
wait 50
tap 930 1320
wait 50
tap 930 1450
wait 50
tap 930 1450
wait 50
expect library.sheet is settings
expect settings.side_button is 2
expect settings.mirror_pdf is 1
expect settings.follow_rotation is 1
expect settings.pinch_zoom is 1
# Language: the list, Automatic (which changes nothing here), the list again, Cancel. The list is
# shorter than Settings: the empty Library's lines are not drawn under it, where it would cut them.
tap 930 1580
wait 50
expect library.sheet is language
expect library.hint is 0
shot 06-language
tap 930 785
wait 100
expect library.sheet is settings
expect settings.language is auto
tap 930 1580
wait 50
expect library.sheet is language
tap 930 1265
wait 50
expect library.sheet is settings
# Display, with its sleep rows: each of the four sleep screens.
tap 930 780
wait 50
expect library.sheet is display
shot 07-display
tap 930 1215
wait 50
shot 08-display-sleep-1
tap 930 1215
wait 50
shot 09-display-sleep-2
tap 930 1215
wait 50
expect settings.sleep_mode is 3
shot display-koreader-wallpaper
tap 930 1215
wait 50
expect settings.sleep_mode is 0
tap 1462 1533
wait 50
expect library.sheet is settings
# Pen feel: every tab (the pad drawn on in the pencil's and the calligraphy pen's: the live readout),
# Reset tool.
tap 930 460
wait 100
expect screen is penset
shot 10-penfeel
tap 130 289
wait 50
shot 11-penfeel-tab-0
pen line 700 600 1500 700 40 0.2 1.0
wait 100
shot 11-penfeel-tab-0-drawn
tap 307 289
wait 50
shot 11-penfeel-tab-1
tap 484 289
wait 50
shot 11-penfeel-tab-2
tap 661 289
wait 50
shot 11-penfeel-tab-3
tap 838 289
wait 50
shot 11-penfeel-tab-4
tap 1015 289
wait 50
shot 11-penfeel-tab-5
tap 1192 289
wait 50
shot 11-penfeel-tab-6
tap 1369 289
wait 50
shot 11-penfeel-tab-7
pen line 700 600 1500 700 40 0.2 1.0
wait 100
shot 11-penfeel-tab-7-drawn
tap 1546 289
wait 50
shot 11-penfeel-tab-8
tap 1723 289
wait 50
shot 11-penfeel-tab-9
tap 1559 1440
wait 50
expect settings.eraser_mode is 1
shot 11-penfeel-tab-9-precise
tap 1039 1440
wait 50
expect settings.eraser_mode is 0
tap 1630 95
wait 50
shot 12-penfeel-reset
expect screen is penset
tap 125 95
wait 100
expect screen is library
# Calibration: taps that make no sense (all in one place), then the corners with the center far
# off, then right; touch skipped with the pen; the check, then Cancel (keep the old calibration).
tap 1535 95
wait 50
tap 930 620
wait 100
expect screen is calib
shot 13-calib-pen
pentap 930 1240
pentap 930 1240
pentap 930 1240
pentap 930 1240
pentap 930 1240
wait 50
shot 14-calib-no-sense
pentap 223 298
pentap 1637 298
pentap 1637 2182
pentap 223 2182
pentap 930 1700
wait 50
shot 15-calib-center-off
pentap 223 298
pentap 1637 298
pentap 1637 2182
pentap 223 2182
pentap 930 1240
wait 50
shot 16-calib-touch
expect screen is calib
pentap 930 1785
wait 50
shot 17-calib-check
expect screen is calib
tap 930 1985
wait 100
expect screen is library
expect settings.pen.valid is 0
# Share (a made-up address under make l10n-check), then its Live view card and Stop.
tap 1345 95
wait 300
expect screen is share
shot 18-share
tap 318 2100
wait 300
expect live.pairing is 1
shot 18-live-card
tap 318 2100
wait 100
expect screen is library
expect live.on is 0
# New notebook, Custom music and its Cancel, More papers (the Templates sheet with no templates
# yet: how to add some) and its Back, the name's keyboard with its accents page and its Cancel,
# then Create.
tap 965 95
wait 50
expect library.sheet is new
shot 19-new
tap 1625 1330
wait 100
expect library.sheet is music
shot 20-music
tap 290 1935
wait 50
expect library.sheet is new
tap 300 1650
wait 100
expect library.sheet is templates
expect templates.count is 0
shot 20-templates-empty-new
tap 300 2123
wait 50
expect library.sheet is new
tap 1600 1975
wait 50
expect library.sheet is keyboard
expect library.hint is 1
shot 21-keyboard
tap 696 2365
wait 50
shot 22-keyboard-accents
tap 696 2365
wait 50
tap 217 2365
wait 50
expect library.sheet is new
tap 1564 2254
wait 150
expect screen is editor
# The editor: the panel, the tool picker (White, then Pin mode: four pens pinned, a fifth refused,
# all unpinned; Pen feel with white ink; Black), the paper sheet and its Templates sheet (with no
# templates yet), the Display sheet (Pin this page, then the sleep screen back as it was), two more
# pages, the Pages view with Select all and Delete (3), the unobstructed page after a pinch, Clear.
pen curve 300 600 40 70 0.7 4
wait 100
shot 23-editor
pentap 1768 358
wait 50
expect editor.modal is tools
shot 24-tools
pentap 1425 878
wait 50
expect settings.white is 1
expect editor.white_mark is 1
shot 24-tools-white
pentap 1600 273
wait 50
expect editor.pin_mode is 1
shot 24-tools-pin-mode
pentap 1504 727
wait 50
pentap 1188 561
wait 50
pentap 1504 561
wait 50
pentap 1188 727
wait 50
expect editor.pins is highlighter,fineliner,marker,calligraphy
pentap 872 395
wait 50
expect editor.pins is highlighter,fineliner,marker,calligraphy
shot 24-tools-pin-cap
pentap 1504 727
wait 50
pentap 1188 561
wait 50
pentap 1504 561
wait 50
pentap 1188 727
wait 50
expect editor.pins is
pentap 1600 273
wait 50
expect editor.pin_mode is 0
pentap 300 1800
wait 50
expect editor.modal is none
pentap 1768 1458
wait 100
expect screen is penset
shot 24-penfeel-white
tap 125 95
wait 100
expect screen is editor
pentap 1768 358
wait 50
pentap 951 878
wait 50
expect settings.white is 0
pentap 872 561
wait 50
expect editor.modal is none
pentap 1768 1242
wait 100
expect editor.modal is paper
shot 25-paper
tap 232 1750
wait 100
expect editor.modal is templates
expect templates.count is 0
shot 25-templates-empty
tap 300 2123
wait 100
expect editor.modal is paper
# photos: the Photos sheet with no photos yet, then Back to the Paper sheet
tap 511 1750
wait 100
expect editor.modal is templates
expect templates.count is 0
shot 25-photos-empty
tap 300 2123
wait 100
expect editor.modal is paper
tap 1562 2125
wait 100
expect editor.modal is none
pentap 1768 1350
wait 100
expect editor.modal is display
shot 26-display-editor
tap 500 1305
wait 100
shot 27-display-pinned
expect settings.sleep_mode is 1
tap 930 1165
wait 50
tap 930 1165
wait 50
expect settings.sleep_mode is 3
shot display-koreader-wallpaper
tap 930 1165
wait 50
expect settings.sleep_mode is 0
tap 1462 1626
wait 100
expect editor.modal is none
pentap 1768 1114
wait 50
pentap 1768 1114
wait 50
expect editor.pages is 3
pentap 1768 1026
wait 150
expect editor.modal is pages
shot 28-pages
tap 850 95
shot 28-index-empty
tap 1400 95
shot 28-index-name-keyboard
tap 133 1725
tap 1643 2365
expect editor.index_rows is 1
shot 28-index-named
tap 125 95
pentap 1768 938
pentap 1768 1026
tap 900 300
shot 28-index-jump
# a link: Add a link, its box, where it goes picked (the Index), then a finger held on it
pentap 1768 1026
wait 150
tap 1410 2395
wait 50
expect editor.link_mode is 1
pen line 300 1000 900 1200 30 0.6 0.6
wait 150
expect editor.picking is new
shot 28-link-pick-new
tap 1400 95
wait 100
expect editor.link.1 is index
touch down 600 1100
wait 800
touch up 600 1100
wait 100
expect editor.picking is had
shot 28-link-pick-had
tap 125 95
wait 100
expect editor.modal is none
pentap 1768 938
pentap 1768 1114
pentap 1768 1026
tap 1400 95
tap 1679 2205
tap 1643 2365
expect editor.index_rows is 0
tap 430 95
tap 1250 95
wait 50
expect editor.ticked is 3
shot 29-pages-all
tap 1630 95
wait 150
shot 30-pages-deleted
expect editor.pages is 1
tap 125 95
wait 100
expect editor.modal is none
pen curve 300 600 40 70 0.7 4
pinch 900 900 300 700 6
wait 100
expect editor.zoom not 1.00
shot 31-zoom
tap 930 74
expect editor.zoom not 1.00
pinch 900 900 700 200 6
wait 100
expect editor.zoom is 1.00
pentap 1768 810
wait 100
expect editor.strokes is 0
shot 32-cleared
pentap 1768 1114
wait 50
pentap 1768 1114
wait 50
expect editor.pages is 3
pentap 1768 1566
wait 200
expect screen is library
# The Library: a notebook's menu, Save PDF now (its notice), Delete and Keep it; a folder
# from the top bar ("A", on the keyboard), the notebook moved into it, the folder's menu while it
# holds a notebook, inside it and back; an empty folder ("B"): inside it, its menu, its Rename
# keyboard and Cancel, then Remove folder.
shot 33-library-card
tap 548 1052
wait 50
expect library.sheet is more
shot 34-more
tap 930 1155
wait 50
expect library.sheet is none
shot 35-saving-pdf
tap 548 1052
wait 50
tap 930 1315
wait 50
expect library.sheet is delete
shot 37-delete
tap 930 1050
wait 50
expect library.sheet is none
expect library.items is 1
tap 1155 95
wait 50
expect library.sheet is keyboard
tap 175 2205
tap 221 2045
tap 1643 2365
wait 100
expect library.sheet is none
expect library.items is 2
tap 1151 1052
wait 50
expect library.sheet is more
tap 930 995
wait 50
expect library.sheet is move
shot 38-move
tap 930 945
wait 100
expect library.sheet is none
expect library.items is 1
tap 548 1052
wait 50
expect library.sheet is folder-more
shot 39-folder-more-holding
tap 930 1155
wait 50
expect library.sheet is none
tap 326 608
wait 100
expect library.folder is A
expect library.items is 1
shot 40-in-folder
tap 125 95
wait 100
expect library.folder is
tap 1155 95
wait 50
tap 175 2205
tap 1101 2205
tap 1643 2365
wait 100
expect library.items is 2
tap 929 608
wait 100
expect library.folder is B
expect library.items is 0
expect library.hint is 1
shot 41-empty-folder
tap 125 95
wait 100
tap 1151 1052
wait 50
expect library.sheet is folder-more
shot 42-folder-more-empty
tap 930 835
wait 50
expect library.sheet is keyboard
shot 43-rename-folder
tap 217 2365
wait 50
expect library.sheet is none
tap 1151 1052
wait 50
tap 930 995
wait 100
expect library.sheet is none
expect library.items is 1
# Sideways: the Library, Settings, New notebook (its labels on two lines) and its Templates sheet,
# the notebook's card (its date line), the tool picker, the paper sheet and its Templates sheet, its
# Pages.
rotate 1
wait 200
expect rotation is 1
shot 44-library-landscape
tap 2155 95
wait 100
expect library.sheet is settings
shot 45-settings-landscape
tap 1062 1695
wait 50
expect library.sheet is shortcuts
tap 1240 368
wait 50
expect settings.shortcuts_enabled is 1
shot 45-controller-landscape
tap 1240 1008
wait 50
expect library.sheet is buttons
shot 45-buttons-landscape
tap 1240 636
wait 50
expect library.buttons_group is tools
shot 45-buttons-tools-landscape
tap 1240 1699
wait 50
expect library.buttons_group is
tap 1240 1276
wait 50
expect library.sheet is shortcuts
tap 1240 368
wait 50
expect settings.shortcuts_enabled is 0
tap 1240 1488
wait 50
expect library.sheet is none
tap 2155 95
wait 100
expect library.sheet is settings
tap 1240 1003
wait 50
expect settings.side_button is 3
shot 45-settings-pen-button-3-landscape
tap 1240 1003
wait 50
tap 1240 1003
wait 50
tap 1240 1003
wait 50
expect settings.side_button is 2
tap 1908 1695
wait 100
expect library.sheet is none
tap 1585 95
wait 100
expect library.sheet is new
shot 46-new-landscape
tap 1462 1116
wait 100
expect library.sheet is templates
shot 46-templates-empty-new-landscape
tap 300 1694
wait 50
expect library.sheet is new
tap 570 1700
wait 50
expect library.sheet is none
tap 267 530
wait 100
expect library.folder is A
shot 47-in-folder-landscape
tap 267 530
wait 200
expect screen is editor
shot 48-editor-landscape
pentap 2388 358
wait 50
expect editor.modal is tools
shot 48-tools-landscape
pentap 2220 273
wait 50
expect editor.pin_mode is 1
shot 48-tools-pin-mode-landscape
pentap 2220 273
wait 50
expect editor.pin_mode is 0
pentap 1492 561
wait 50
expect editor.modal is none
pentap 2388 1244
wait 150
expect editor.modal is paper
shot 48-paper-landscape
tap 1544 1300
wait 100
expect editor.modal is templates
shot 48-templates-empty-landscape
tap 300 1694
wait 100
expect editor.modal is paper
# photos, sideways
tap 1848 1300
wait 100
expect editor.modal is templates
expect templates.count is 0
shot 48-photos-empty-landscape
tap 300 1694
wait 100
expect editor.modal is paper
tap 2182 1696
wait 100
expect editor.modal is none
pentap 2388 1028
wait 150
expect editor.modal is pages
shot 49-pages-landscape
tap 1300 95
shot 49-index-landscape
tap 2100 95
shot 49-index-keyboard-landscape
tap 280 1745
tap 500 95
tap 125 95
wait 100
expect editor.modal is none
# a link, sideways: Add a link, its box, its page picked, a finger held on it
pentap 2388 1028
wait 150
tap 1300 95
tap 1760 1775
wait 50
expect editor.link_mode is 1
pen line 400 800 1000 1000 30 0.6 0.6
wait 150
expect editor.picking is new
shot 49-link-pick-new-landscape
tap 227 340
wait 100
expect editor.links is 1 1
touch down 700 900
wait 800
touch up 700 900
wait 100
expect editor.picking is had
shot 49-link-pick-had-landscape
tap 125 95
wait 100
expect editor.modal is none
pentap 2388 1560
wait 200
expect screen is library
# Sideways, the rest: in folder A the notebook's menu, its Delete and Move to; at the top level the
# folder's menu, Settings' Display sheet (its last sleep screen) and language list, Pen feel (the
# calligraphy pen drawn with), calibration (each step, then Cancel), Share, and New notebook's
# keyboard (both pages) and Custom music.
expect library.folder is A
tap 431 896
wait 50
expect library.sheet is more
shot 50-more-landscape
tap 1240 1110
wait 50
expect library.sheet is delete
shot 51-delete-landscape
tap 1240 845
wait 50
expect library.sheet is none
expect library.items is 1
tap 431 896
wait 50
tap 1240 790
wait 50
expect library.sheet is move
shot 52-move-landscape
tap 1240 1055
wait 50
expect library.sheet is none
tap 125 95
wait 100
expect library.folder is
tap 431 896
wait 50
expect library.sheet is folder-more
shot 53-folder-more-landscape
tap 1240 950
wait 50
expect library.sheet is none
tap 2155 95
wait 100
expect library.sheet is settings
tap 1240 590
wait 50
expect library.sheet is display
tap 1240 1005
wait 50
tap 1240 1005
wait 50
shot 54-display-landscape
tap 1240 1005
wait 50
expect settings.sleep_mode is 3
shot display-koreader-wallpaper
tap 1240 1005
wait 50
expect settings.sleep_mode is 0
tap 1770 1325
wait 50
expect library.sheet is settings
tap 1240 1385
wait 50
expect library.sheet is language
shot 55-language-landscape
tap 1240 1055
wait 50
expect library.sheet is settings
tap 1240 260
wait 100
expect screen is penset
tap 1834 289
wait 50
pen line 700 600 1500 700 40 0.2 1.0
wait 100
shot 56-penfeel-landscape
tap 2312 289
wait 50
shot 56-penfeel-eraser-landscape
tap 125 95
wait 100
expect screen is library
tap 2155 95
wait 50
tap 1240 425
wait 100
expect screen is calib
shot 57-calib-pen-landscape
pentap 1240 930
pentap 1240 930
pentap 1240 930
pentap 1240 930
pentap 1240 930
wait 50
shot 58-calib-no-sense-landscape
pentap 298 223
pentap 2182 223
pentap 2182 1637
pentap 298 1637
pentap 1240 1300
wait 50
shot 59-calib-center-off-landscape
pentap 298 223
pentap 2182 223
pentap 2182 1637
pentap 298 1637
pentap 1240 930
wait 50
shot 60-calib-touch-landscape
expect screen is calib
pentap 1240 1475
wait 50
shot 61-calib-check-landscape
tap 1240 1675
wait 100
expect screen is library
expect settings.pen.valid is 0
tap 1965 95
wait 300
expect screen is share
shot 62-share-landscape
tap 318 1485
wait 300
expect live.pairing is 1
shot 62-live-card-landscape
tap 318 1485
wait 100
expect screen is library
expect live.on is 0
tap 1585 95
wait 50
expect library.sheet is new
tap 1950 1424
wait 50
expect library.sheet is keyboard
shot 63-keyboard-landscape
tap 928 1745
wait 50
shot 64-keyboard-accents-landscape
tap 928 1745
wait 50
tap 279 1745
wait 50
expect library.sheet is new
tap 1240 1116
wait 100
expect library.sheet is music
shot 65-music-landscape
tap 598 1693
wait 50
expect library.sheet is new
tap 570 1700
wait 50
expect library.sheet is none
rotate 0
wait 200
expect rotation is 0
quit
