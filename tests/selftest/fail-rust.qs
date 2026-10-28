# The Rust build's own check, the other way round: every line below that is not a comment, a wait
# or a tap fails, so the run must report each of them by its line number and exit with status 3
# (see the scenarios rule in the Makefile). make c-check skips it (SCEN_SKIP_RUST_ONLY).
wait 200
expect status.zone is 0 0 0 0
expect status.shown is 1
expect gestures.pages is touch.2.up
expect touch.contacts is 9
expect main.longest_unpumped_ms is 99999
