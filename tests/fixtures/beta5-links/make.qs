# Generator for the committed beta.5 links fixture (PHASE2-BRIEF 1.3). Not a scenario: it ran once with
# the phase 1 headless runner at 72fafda (claude/rust-index-p1), whose inked flow writes links.ini byte
# for byte as the beta.5 device does. See README.md beside it for how it was run.
# One notebook of 4 pages. On page 1, with the pen, each link a box drawn around what links and then its
# page picked (links.ini orders a page's links by seed, so editor.link.N is not checked here):
#   A: to page 3; B: to page 2;
#   C: to the Index, then its ring covered whole by the precise eraser (its stroke stays, hidden);
#   D: to page 4, then its ring undone (a dormant link: no ink).
# On page 4 a small ring linked to page 3, then a big ring around it linked to page 2 (beta.5 follows the
# big one inside the small one; smallest wins follows the small one). Last, page 3 is named Arbol with
# its acute accent (after the links: once a page is named the Pages view picks from its Index tab). The
# run ends with quit while the notebook is open, so the closing compaction never takes C's erased ring
# out of the page file (erased.txt, the list it leaves, is not part of the fixture).
# settings: shortcuts_enabled=1
# settings: mirror_pdf=0
# settings: eraser_mode=1
# copy: tests/fixtures/index-keys .
# rust-only
wait 300
tap 965 95
wait 100
tap 1564 2254
wait 200
pentap panel next
pentap panel next
pentap panel next
expect editor.pages is 4
pentap panel previous
pentap panel previous
pentap panel previous
expect editor.page is 1
# A: to page 3
key 38 0
expect editor.link_mode is 1
pen line 300 1000 900 1200 30 0.6 0.6
expect editor.picking is new
expect editor.pages_tab is pages
tap 1155 340
expect editor.links is 1 1
# B: to page 2
key 38 0
pen line 300 1400 900 1500 30 0.6 0.6
expect editor.picking is new
tap 703 340
expect editor.links is 2 2
# C: to the Index, then its ring erased whole by the precise eraser (the eraser end)
key 38 0
pen line 1000 1000 1500 1200 30 0.6 0.6
expect editor.picking is new
tap 1250 95
expect editor.links is 3 3
expect editor.strokes is 3
eraser line 980 992 1520 1208 40
wait 700
expect editor.strokes is 4
expect editor.erase_strokes is 1
expect editor.links is 3 3
shot 01-c-erased
# D: to page 4, then its ring undone: the link stays, dormant
key 38 0
pen line 1000 1400 1500 1500 30 0.6 0.6
expect editor.picking is new
tap 1607 340
expect editor.links is 4 4
pentap panel undo
wait 300
expect editor.links is 4 3
expect editor.page is 1
shot 02-page-1
# Page 4: a small ring to page 3, then a big ring around it to page 2
pentap panel next
pentap panel next
pentap panel next
expect editor.page is 4
key 38 0
pen line 600 1000 900 1100 30 0.6 0.6
expect editor.picking is new
tap 1155 340
expect editor.links is 1 1
key 38 0
pen line 300 800 1500 1500 30 0.6 0.6
expect editor.picking is new
tap 703 340
expect editor.links is 2 2
shot 03-page-4
# beta.5 follows the big ring inside the small one
tap 750 1050
expect editor.page is 2
# Page 3 is Arbol with an acute accent on its A: shift, the accents page, a acute, the letters page, r, b, o, l
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
expect editor.index_rows is 1
# Back on page 1, where the notebook opens again; quit with it open (no compaction)
pentap panel previous
pentap panel previous
expect editor.page is 1
expect editor.links is 4 3
wait 500
quit
