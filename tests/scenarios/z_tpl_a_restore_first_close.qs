# settings: status_mode=1
# The notebook 1.0.0-beta wrote on (as in z_tpl_4), opened on page 2 and closed once, page 1 never
# visited: the pages it gets back as it opens are what this close writes the notebook's PDF and
# card for. (z_tpl_4 visits page 1 later, which would write them too.)
# copy: tests/fixtures/scen-tpl-restore .
# pdfs: 1
# rust-only
wait 500
expect screen is library
expect library.card.1.luma is 255
tap 326 608
wait 600
expect screen is editor
expect editor.page is 2
expect editor.unsaved is 0
pentap panel library
wait 600
expect screen is library
expect library.card.1.luma is 236
quit
