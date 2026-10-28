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
