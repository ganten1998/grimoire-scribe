# settings: status_mode=1
# settings: invert=1
# root: z_weblinks_1_card
# rust-only
# A web link under Invert, held, and after a page deleted (2.2, issue #174), from z_weblinks_1_card's notebook. With
# Invert on, the card's code is drawn as its negative (dark on light on the panel), module for module the code of
# the address. A finger held on a web link arms Select, as on a tab: no card; a tap after it still opens the card.
# Page 2 deleted in the Pages view: its two web links go from what the editor shows, but weblinks.ini keeps their
# lines until its next write; reopened, the notebook drops them again (the links of a page no longer in it).
wait 300
tap 326 608
wait 700
expect screen is editor
expect settings.invert is 1
expect editor.page is 2
pentap panel previous
wait 400
expect editor.page is 1
expect editor.weblinks is 1 4
tap 680 480
wait 400
shot 01-inverted
expect editor.link_card is 1
expect editor.link_card.web is https://example.org/grimoire
expect editor.link_card.qr is 1
tap panel card.close
wait 300
expect editor.link_card is 0
# held: Select arms, no card (the pen gone first: a hold while the pen is near arms nothing)
pen leave
wait 1500
touch down 680 480
wait 800
expect editor.link_card is 0
expect editor.select is lasso
touch up 680 480
wait 100
tap 1500 2300
wait 300
expect editor.link_card is 0
tap 680 480
wait 400
expect editor.link_card is 1
tap panel card.close
wait 300
# page 2 deleted, the notebook closed and opened again: its web links are not shown
pentap panel pages
wait 400
expect editor.modal is pages
touch down 930 551
wait 700
expect editor.pages.context is 2
touch up 930 551
tap panel pages.delete
wait 400
expect editor.pages is 2
expect editor.weblinks is 1 2
tap panel pages.back
wait 400
pentap panel library
wait 600
expect screen is library
tap 326 608
wait 700
expect screen is editor
expect editor.pages is 2
expect editor.weblinks is 1 2
pentap panel next
wait 400
expect editor.weblinks is 1 2
expect editor.weblink.1 is 160 400 1040 160 http://example.com/notes
pentap panel library
wait 600
expect screen is library
quit
