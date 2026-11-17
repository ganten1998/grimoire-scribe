# settings: status_mode=1
# env: GRIMOIRE_IMPORT_STABLE=fast GRIMOIRE_PDF_SLICE_PAGES=2
# copy: tests/fixtures/pdf/weblinks import
# pdfs: 0
# rust-only
# A PDF with web links in grimoire/import/ (2.2, issue #174) becomes a notebook whose web links are kept. A finger's
# tap on one shows its card beside it: the host as its title, the address as the PDF has it, a QR code of that
# address (module for module the code qr.rs makes of it) and Done, which closes it, as a tap outside the card does.
# Nothing opens the address. The page's tab is a document link as before; a refused address (javascript:) is no
# link at all; an address the PDF wrote with spaces around it is kept without them; page 2's address on two lines
# is a link on each line, one address. The fixture's page is a quarter of the Scribe's: one point is 4 px.
waitbg
wait 300
expect notice is
expect toast is
expect library.items is 1
tap 326 608
wait 700
expect screen is editor
expect editor.pages is 3
expect editor.page is 1
# (page 1: the address and the tab; the notebook: four web links on three pages)
expect editor.weblinks is 1 4
expect editor.weblink.1 is 160 400 1040 160 https://example.org/grimoire
expect editor.weblink.2 is
expect editor.doclinks is 1 1
expect editor.doclink.1 is p3 1520 320 320 160
expect editor.link_card is 0
# the tap on the address: its card, the page staying
tap 680 480
wait 400
shot 01-card
expect editor.link_card is 1
expect editor.link_card.web is https://example.org/grimoire
expect editor.link_card.title is example.org
expect editor.link_card.rows is close
expect editor.link_card.qr is 1
expect editor.link_card.border is 1
expect editor.page is 1
# Done closes it
tap panel card.close
wait 300
expect editor.link_card is 0
expect editor.link_card.border is 0
expect editor.page is 1
# the refused address is no link: a tap on it does nothing
tap 680 880
wait 300
expect editor.link_card is 0
expect editor.page is 1
# the tab goes to page 3, whose address the PDF had with spaces around it
tap 1680 400
wait 400
expect editor.page is 3
expect editor.weblink.1 is 160 400 1040 160 http://example.com/notes
tap 680 480
wait 400
expect editor.link_card.web is http://example.com/notes
expect editor.link_card.title is example.com
expect editor.link_card.qr is 1
# a tap outside the card closes it, and does nothing else
tap 1500 2300
wait 300
expect editor.link_card is 0
expect editor.page is 3
# page 2: an address on two lines, a link on each, one address; the second line's tap shows it whole
pentap panel previous
wait 400
expect editor.page is 2
expect editor.weblinks is 2 4
expect editor.weblink.1 is 160 720 1520 160 https://example.org/a/long/address/that/goes/on/two/lines
expect editor.weblink.2 is 160 880 640 160 https://example.org/a/long/address/that/goes/on/two/lines
tap 480 960
wait 400
shot 02-two-lines
expect editor.link_card.web is https://example.org/a/long/address/that/goes/on/two/lines
expect editor.link_card.title is example.org
expect editor.link_card.qr is 1
tap panel card.close
wait 300
expect editor.link_card is 0
# the refused mailto: is no link either
tap 680 1600
wait 300
expect editor.link_card is 0
pentap panel library
wait 600
expect screen is library
quit
