# settings: status_mode=1
# rust-only
# root: z_bar_0_make
# env: GRIMOIRE_LANG=es
# env: GRIMOIRE_CHECK_FIT=1
# Phase 2's surfaces in Spanish, with every text checked for fit (PHASE2-BRIEF 13.1): the strip, Link to...,
# Find in pick mode, the long-press card with its four rows and the bar's x2 card, upright and sideways. The
# taps are the English scenarios' (z_link_1_make, z_link_3_find_pick, z_link_5_card, z_bar_10_card): the
# layout does not depend on the language.
wait 3700
tap 200 400
wait 300
expect screen is editor
expect editor.page is 1
expect editor.links is 3 3
# The strip, then Link to... (docked at the bottom)
key 38 0
wait 50
expect editor.strip is new
shot 01-strip
pen line 1000 600 1500 800 30 0.6 0.6
wait 150
expect editor.linkto is pick
expect editor.linkto.dock is bottom
shot 02-linkto
# Search: Find in pick mode; 2 offers "Enlazar con la página 2"
expect editor.ui_at.330.2370 is linkto.search
tap 330 2370
wait 100
expect editor.find is pick
tap 310 1725
wait 100
expect editor.find.1 is Enlazar con la página 2
shot 03-find-pick
tap 217 2365
wait 100
expect editor.find is 0
expect editor.linkto is pick
expect editor.ui_at.1530.2370 is linkto.cancel
tap 1530 2370
wait 100
expect toast is No se creó el enlace.
expect editor.links is 3 3
# The card for one link, to Árbol: four rows
pen leave
wait 1100
touch down 600 1100
wait 800
touch up 600 1100
wait 100
expect editor.link_card is 1
expect editor.link_card.rows is rename retarget redraw delete
expect editor.link_card.title is Árbol
shot 04-card
tap 1200 1900
wait 100
expect editor.link_card is 0
# The card for an unnamed page: Añadir la página 2 al índice...
touch down 600 1450
wait 800
touch up 600 1450
wait 100
expect editor.link_card is 1
expect editor.link_card.title is Página 2
shot 05-card-unnamed
tap 1200 1900
wait 100
expect editor.link_card is 0
# The bar's x2 row held: the card for both links
swipe 2 930 1300 0 -400
wait 300
expect editor.bar is open
expect editor.bar.1 is p3 x2
touch down 479 2218
wait 800
touch up 479 2218
wait 100
expect editor.link_card is 1
expect editor.link_card.links is 2
expect editor.link_card.rows is rename retarget delete
shot 06-card-xn
swipe 2 1300 300 0 400
wait 300
expect editor.link_card is 0
expect editor.bar is 0
# Sideways: the strip, Link to..., Find in pick mode, the card
rotate 1
wait 300
key 38 0
wait 50
expect editor.strip is new
shot 07-strip-sideways
pen line 1000 500 1400 700 30 0.6 0.6
wait 150
expect editor.linkto is pick
shot 08-linkto-sideways
expect editor.ui_at.433.1750 is linkto.search
tap 433 1750
wait 100
expect editor.find is pick
shot 09-find-pick-sideways
tap 900 300
wait 100
expect editor.find is 0
expect editor.linkto is pick
key 38 0
wait 100
expect editor.linkto is 0
pen leave
wait 1100
touch down 992 825
wait 800
touch up 992 825
wait 100
expect editor.link_card is 1
shot 10-card-sideways
tap 2200 1700
wait 100
expect editor.link_card is 0
rotate 0
wait 300
expect editor.links is 3 3
pentap panel library
waitbg
expect screen is library
