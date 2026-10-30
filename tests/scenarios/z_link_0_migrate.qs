# The owner's beta.5 links migrate (PHASE2-BRIEF 5): tests/fixtures/beta5-links, made by the phase 1
# inked flow. Page 1 has links A (to page 3), B (to page 2), C (to the Index, its ring hidden whole by
# the precise eraser) and D (to page 4, its ring undone: dormant); page 4 a small ring to page 3 inside
# a big one to page 2. Opening the notebook migrates page 1 at once (each inked link gets the unpadded
# box of its ink as its area, the ink stays; D stays as it is), the idle ticks migrate page 4 without
# opening it, and links.ini is written after a pause. Then a link follows its area: ring A erased whole
# still goes to page 3, ring C (hidden) still to the Index, and inside page 4's small ring the small
# one wins (beta.5 followed the big one). Last, Copy page copies page 1's links with their areas. links.ini orders a page's links by seed: on page 1 link 1 is
# C, 2 is A, 3 is D and 4 is B.
# copy: tests/fixtures/beta5-links .
# settings: mirror_pdf=0
# settings: status_mode=1
# rust-only
wait 300
tap 200 400
wait 200
expect screen is editor
expect editor.page is 1
expect editor.pages is 4
expect editor.strokes is 4
expect editor.erase_strokes is 1
expect editor.links is 4 3
expect editor.link.2 is p3
expect editor.link.2.area is 300 1000 600 200
expect editor.link.1 is index
expect editor.link.1.area is 1000 1000 500 200
expect editor.link.3 is p4
expect editor.link.3.area is
expect editor.link.4 is p2
expect editor.link.4.area is 300 1400 600 100
expect editor.links_dirty is 1
# The idle ticks migrate page 4 without opening it, reading it with the pens pumped (z_link_0b_reopen
# finds its areas saved)
expect main.longest_unpumped_ms not -1
wait 1500
expect editor.migrate_left is 0
expect main.longest_unpumped_ms is ok
expect editor.page is 1
# ... and links.ini is written after a pause (a migrated page is not a changed page)
wait 2500
expect editor.links_dirty is 0
# Ring A erased whole with the stroke eraser: the link still follows its area
pentap panel eraser
expect editor.tool is eraser
pen line 600 950 600 1250 20 0.6 0.6
wait 300
expect editor.strokes is 3
expect editor.links is 4 3
pentap panel eraser
pen leave
wait 1100
tap 600 1100
expect editor.page is 3
pentap panel previous
pentap panel previous
expect editor.page is 1
# Ring C, hidden by the precise eraser: its link to the Index still works
tap 1250 1100
expect editor.modal is map
expect editor.map.list is 0
tap 125 95
expect editor.modal is none
expect editor.page is 1
# Page 4 (never opened before): inside the small ring the small link wins
pentap panel next
pentap panel next
pentap panel next
expect editor.page is 4
expect editor.links is 2 2
expect editor.link.1 is p2
expect editor.link.2 is p3
tap 750 1050
expect editor.page is 3
# Back on page 1
pentap panel previous
pentap panel previous
expect editor.page is 1
# Copy page keeps links (PHASE2-BRIEF 9.2): page 1 ticked alone is copied as page 2, which opens with the
# same links (same seeds, so the same N), the same areas, and D still dormant. Here last, as the copy moves
# the later pages one on (z_link_0b_reopen opens it again).
pentap panel pages
tap panel pages.graph
wait 300
expect editor.modal is map
tap panel map.pages
wait 100
expect editor.pages_tab is pages
tap 396 296
wait 100
expect editor.ticked is 1
tap panel pages.copy
wait 300
expect editor.modal is none
expect editor.page is 2
expect editor.pages is 5
expect toast has a copy of page 1
expect editor.strokes is 3
expect editor.links is 4 3
expect editor.link.1 is index
expect editor.link.1.area is 1000 1000 500 200
expect editor.link.2 is p3
expect editor.link.2.area is 300 1000 600 200
expect editor.link.3 is p4
expect editor.link.3.area is
expect editor.link.4 is p2
expect editor.link.4.area is 300 1400 600 100
expect editor.links_dirty is 0
expect editor.undo_kind is
# the copy's ring A area still goes to page 3, now the fourth page (links name pages, not places)
pen leave
wait 1100
tap 600 1100
expect editor.page is 4
pentap panel previous
pentap panel previous
expect editor.page is 2
pentap panel previous
expect editor.page is 1
pentap panel library
waitbg
expect screen is library
