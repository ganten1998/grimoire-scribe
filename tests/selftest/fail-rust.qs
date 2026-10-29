# The Rust build's own check, the other way round: every line below that is not a comment, a wait
# or a tap fails, so the run must report each of them by its line number and exit with status 3
# (see the scenarios rule in the Makefile). make c-check skips it (SCEN_SKIP_RUST_ONLY).
wait 200
expect status.zone is 0 0 0 0
expect status.shown is 1
expect gestures.pages is touch.2.up
expect touch.contacts is 9
expect settings.gestures is 0
expect settings.hints is 4
expect notebook.last_page is 7
expect main.longest_unpumped_ms is 99999
expect editor.panel_at.1768.594 is undo
expect editor.scrolls is 1 1
expect editor.bar is open
expect editor.bar.entries is 3
expect editor.areas is 2
expect editor.strip is new
expect editor.lasso is 9
expect editor.linkto is pick
expect editor.link_card is 1
pentap panel nosuchbutton

expect fshape.state is drawing

expect editor.map is graph
expect editor.shape_preview is circle
