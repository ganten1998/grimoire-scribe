# The Rust build's own check: every expect line below holds, so the run must exit with status 0
# (see the scenarios rule in the Makefile). It asks what only the Rust build answers, one of each
# family at least once (docs/SIM.md lists them). make c-check skips it (SCEN_SKIP_RUST_ONLY).
wait 200
expect running is 1
expect status.zone is 0 0 165 165
expect status.shown is 0
expect gestures.pages is touch.2.down
expect gestures.next-page is touch.2.left
expect gestures.undo is
expect settings.gestures is 1
expect settings.landscape_fit is 1
expect settings.hints is 0
expect settings.link_areas is 0
expect notebook.last_page is 0
expect main.longest_unpumped_ms not -1
expect touch.contacts is 0
expect touch.class is none
finger 1 down 930 1800
frame 20
expect touch.contacts is 1
expect touch.class is one
finger 1 up 930 1800
frame
expect touch.contacts is 0
swipe 2 930 1200 0 400
expect touch.class is none
expect touch.suppressed is 0
expect screen is library
expect library.sheet is none
expect main.longest_unpumped_ms is ok
# the panel's names: a stroke makes Undo a button, found by name and at a point
tap 965 95
wait 100
tap 510 460
tap 1564 2254
wait 400
expect screen is editor
expect editor.panel_at.1768.358 is tool-picker
expect editor.scrolls is 0 0
expect editor.bar is 0
expect editor.bar.entries is 0
expect editor.bar.rect is
expect editor.strip is 0
expect editor.lasso is 0
expect editor.link_area is
expect editor.links_dirty is 0
expect editor.ui_at.900.900 is
expect editor.linkto is 0
expect editor.linkto.rect is
expect editor.link_card is 0
expect editor.link_card.rect is
expect editor.link_card.border is 0
expect editor.recent is
expect editor.kb_dock is
expect editor.areas is 0
expect editor.panel_at.1768.594 is
expect editor.panel_at.1768.1242 is links
pen line 300 700 1300 720 20 0.6 0.6
expect editor.panel_at.1768.594 is undo
expect editor.undo_kind is add
tap panel undo
expect editor.strokes is 0
expect editor.undo_kind is
expect editor.redo_kind is add

expect fshape.state is idle
expect gestures.shape_count is 0
expect library.record is 0
