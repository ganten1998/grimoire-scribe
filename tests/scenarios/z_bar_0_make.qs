# settings: status_mode=1
# rust-only
# The notebook the Links bar's scenarios start from (# root: z_bar_0_make): five pages, three links drawn on
# page 1 with the pen (to page 3, to page 2 and to page 3 again, from the top of the page down), then page 3
# named Arbol (with its accent) and page 5 Arena. The bar lists two entries: page 3 twice, then page 2. (The
# links come first: once a page is named the Pages view picks from its Index tab.)
# settings: shortcuts_enabled=1
# settings: mirror_pdf=0
# copy: tests/fixtures/index-keys .
wait 300
tap 965 95
wait 100
tap 1564 2254
wait 200
pentap panel next
pentap panel next
pentap panel next
pentap panel next
expect editor.pages is 5
pentap panel previous
pentap panel previous
pentap panel previous
pentap panel previous
expect editor.page is 1
# Three links with the pen: each a box drawn around what links, then its page picked
key 38 0
expect editor.link_mode is 1
pen line 300 1000 900 1200 30 0.6 0.6
# Link to... opens: Choose from the pages... picks in the Pages view
expect editor.ui_at.930.2370 is linkto.pages
tap 930 2370
expect editor.picking is new
expect editor.pages_tab is pages
tap 1155 340
expect editor.links is 1 1
key 38 0
pen line 300 1400 900 1500 30 0.6 0.6
# Link to... opens: Choose from the pages... picks in the Pages view
expect editor.ui_at.930.679 is linkto.pages
tap 930 679
expect editor.picking is new
tap 703 340
expect editor.links is 2 2
key 38 0
pen line 300 1800 900 1900 30 0.6 0.6
# Link to... opens: Choose from the pages... picks in the Pages view
expect editor.ui_at.930.679 is linkto.pages
tap 930 679
expect editor.picking is new
tap 1155 340
expect editor.links is 3 3
expect editor.page is 1
# Page 3 is Arbol with an acute accent on its A: shift, the accents page, a acute, the letters page, r, b, o, l
pentap panel next
pentap panel next
expect editor.page is 3
key 49 0
wait 200
expect editor.keyboard is Page name
tap 175 2205
tap 696 2365
tap 133 1725
tap 696 2365
tap 664 1885
tap 1101 2205
tap 1549 1885
tap 1637 2045
expect editor.text is Árbol
tap 1643 2365
wait 200
expect editor.page_name is Árbol
# Page 5 is Arena: shift, a, r, e, n, a
pentap panel next
pentap panel next
expect editor.page is 5
key 49 0
wait 200
tap 175 2205
tap 221 2045
tap 664 1885
tap 487 1885
tap 1278 2205
tap 221 2045
expect editor.text is Arena
tap 1643 2365
wait 200
expect editor.page_name is Arena
expect editor.index_rows is 2
# Back on page 1, where the notebook opens again
pentap panel previous
pentap panel previous
pentap panel previous
pentap panel previous
expect editor.page is 1
pentap panel library
waitbg
expect screen is library
