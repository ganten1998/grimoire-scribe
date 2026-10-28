# The Rust build's own check: every expect line below holds, so the run must exit with status 0
# (see the scenarios rule in the Makefile). It asks what only the Rust build answers, one of each
# family at least once (docs/SIM.md lists them). make c-check skips it (SCEN_SKIP_RUST_ONLY).
wait 200
expect running is 1
expect status.zone is 0 0 142 142
expect status.shown is 0
expect gestures.pages is touch.2.down
expect gestures.next-page is touch.2.left
expect gestures.undo is
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
