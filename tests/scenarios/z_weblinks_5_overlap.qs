# settings: status_mode=1
# settings: status_on=0
# settings: shortcuts_enabled=1
# settings: mirror_pdf=0
# env: GRIMOIRE_IMPORT_STABLE=fast GRIMOIRE_PDF_SLICE_PAGES=2
# copy: tests/fixtures/pdf/weblinks_overlap import
# copy: tests/fixtures/index-keys .
# pdfs: 0
# rust-only
# What a finger's tap opens where a web link meets Grimoire's own links (2.2, issue #174): the user's links (their
# touch area grown to 7 mm too) and the document's page links (a tab's finger margin too) win over a web link's
# area; the clock's corner is the clock's over a web link; elsewhere the web link's card opens. Page 1 of the
# fixture has a small tab to page 3 under a web link over the whole page; page 2 the same tab alone.
waitbg
wait 300
expect library.items is 1
tap library card.1
wait 700
expect screen is editor
expect editor.page is 1
expect editor.doclink.1 is p3 80 840 40 40
expect editor.weblink.1 is 0 0 1860 2480 https://whole.example/
# a tap in the tab's finger margin, outside its own 40 px box: the tab, not the web link under it
tap 65 860
wait 400
expect editor.link_card is 0
expect editor.page is 3
pentap panel previous
pentap panel previous
wait 400
expect editor.page is 1
# the tab's own box
tap 100 860
wait 400
expect editor.page is 3
pentap panel previous
pentap panel previous
wait 400
expect editor.page is 1
# the clock's corner: the clock's, over the web link too
tap 70 70
wait 400
expect editor.link_card is 0
expect settings.status_on is 1
tap 70 70
wait 400
expect settings.status_on is 0
expect editor.link_card is 0
# elsewhere on the page: the web link's card
tap 900 1200
wait 400
expect editor.link_card is 1
expect editor.link_card.web is https://whole.example/
tap panel card.close
wait 300
expect editor.link_card is 0
# a thin link of the user's drawn over the web link, to page 2 (30 px tall: a finger's 7 mm, 83 px, is its touch
# area): a tap in that grown area, outside its own box, follows it
key 38 0
expect editor.link_mode is 1
pen line 1000 1500 1200 1530 30 0.6 0.6
expect editor.link_area is 1000 1500 200 30
tap panel linkto.pages
expect editor.picking is new
tap panel pages.page.2
wait 300
expect editor.links is 1 1
pen leave
wait 1100
tap 1100 1550
wait 400
expect editor.link_card is 0
expect editor.page is 2
# page 2: the same tab, no web link: its finger margin follows it as on page 1
tap 65 860
wait 400
expect editor.page is 3
pentap panel library
wait 600
expect screen is library
quit
