# settings: status_mode=1
# settings: link_areas=1
# copy: tests/fixtures/pdf/weblinks Documents
# pdfs: 1
# rust-only
# A fill brings back the web links of the pages it adds (2.2, issue #174). The web links PDF picked in place (the
# runner's pdfimport) makes its notebook; page 2 (an address on two lines) is deleted in the Pages view; the same
# PDF picked again fills the notebook with that page, whose two links come back with it, and the other pages' web
# links are not added twice. The eye (Link areas on) draws the web links' boxes with the document links'.
pdfimport z_weblinks_3_fill/Documents/Links.pdf
waitbg
wait 300
expect library.items is 1
tap library card.1
wait 700
expect screen is editor
expect editor.page is 1
expect editor.weblinks is 1 4
# (page 1's boxes: the tab and the address)
expect editor.doc_areas is 2
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
pdfimport z_weblinks_3_fill/Documents/Links.pdf
waitfor toast is Added 1 book page.
waitbg
wait 300
expect library.items is 1
tap library card.1
wait 700
expect screen is editor
expect editor.pages is 3
expect editor.weblinks is 1 4
pentap panel next
wait 400
expect editor.page is 2
expect editor.weblinks is 2 4
expect editor.weblink.1 is 160 720 1520 160 https://example.org/a/long/address/that/goes/on/two/lines
expect editor.weblink.2 is 160 880 640 160 https://example.org/a/long/address/that/goes/on/two/lines
expect editor.doc_areas is 2
tap 480 960
wait 400
shot 01-filled-card
expect editor.link_card.web is https://example.org/a/long/address/that/goes/on/two/lines
expect editor.link_card.qr is 1
tap panel card.close
wait 300
pentap panel library
wait 600
expect screen is library
quit
