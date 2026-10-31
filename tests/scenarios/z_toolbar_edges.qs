# rust-only
# root: z_bar_0_make
# settings: status_mode=1
# The toolbar goes wherever it is dragged, up to the screen's very edges: its top at y 0 (no 12 px
# band above it) and its foot on the bottom edge, upright and sideways, where its buttons still take
# their taps. A drag the pen's coming near cancels leaves it where it was last drawn.
wait 300
tap 200 400
wait 300
expect editor.panel_rect is 1704 220 128 1734
drag touch 1737 260 1737 0 5
wait 300
expect editor.panel_rect is 1704 0 128 1734
expect editor.layers is ok
expect editor.panel_at.1737.30 is grip
expect editor.panel_at.1768.138 is tool-picker
tap 1768 1454
wait 300
expect editor.modal is display
tap 1464 1858
wait 300
expect editor.modal is none
# The foot on the bottom edge (2480 less the panel's 1734).
drag touch 1737 40 1737 2470 5
wait 300
expect editor.panel_rect is 1704 746 128 1734
expect editor.layers is ok
# Sideways it keeps to the edges too: dragged up, its top at y 0.
rotate 1
wait 300
expect editor.panel_rect is 1962 126 128 1734
drag touch 1995 166 1995 0 5
wait 300
expect editor.panel_rect is 1962 0 128 1734
expect editor.layers is ok
rotate 0
wait 300
expect editor.panel_rect is 1732 0 128 1734
# A drag the pen's coming near cancels (the off-screen cancel) stays where it was last drawn, never
# thrown into the corner.
pen leave
wait 1100
finger 1 down 1765 40
frame 100
finger 1 move 1765 240
frame 100
finger 1 move 1765 440
frame 100
pen hover 300 300
wait 300
expect editor.panel_rect is 1732 400 128 1734
finger 1 up 1765 440
frame
pen leave
