# rust-only
# settings: status_mode=1
# env: GRIMOIRE_SIM_ADDRS=en0=192.168.0.20
# Live view's toolbar under the editor's menus (2.0.0, issue #16, reported by nebuff: "Toolbar disappears in live
# view when a popup menu (settings, display settings) is opened then comes back after closing menu"): the browser
# shows the toolbar wherever the Kindle does. The panel's layer (live.panel: what a browser is sent) stays where
# the panel is while a menu, sheet or bar is up beside it or over it, holding the screen there as the Kindle shows
# it, the menu's part over the panel too; the browser's picture there (live.stale: the tiles with the layers over
# them, every layer switched on) is the Kindle's, to a unit of gray, before a setting changes in the menu, after
# it, and after the menu closes. Settings over the notebook (and its own Display sheet) leave the frame the whole
# screen, the panel's edge beside the sheet in the tiles and no layer, as before; the layer is back as it closes.
# The Kindle's refreshes are what they were: the layer is the stream's alone.
# (past the two launch repaints, at 1.2 s and 3.5 s, which refresh the whole screen: the stretch counted at the
# Display sheet's X ended about 3.4 s after the start, so the 3.5 s one fell inside it, before it or after it as
# the machine's speed had it, and the check wanted it last, issue #134)
# (the clock corner held still: its minute's refresh would land among the counted ones)
clock 1700000000
wait 3600
wait 300
tap library add
tap library add.share
wait 300
tap share live
wait 300
expect live.on is 1
tap share done
wait 300
expect screen is library
tap library add
tap library add.new-notebook
wait 100
tap 510 460
tap 1564 2254
wait 400
expect screen is editor
expect editor.panel_rect is 1704 220 128 1842
expect live.panel is 1704 220 128 1842
expect live.stale.1704.220.128.1842 is 0
expect display.presents not -1
# The Display sheet beside the panel, nothing changed: the toolbar stays in the browser.
tap panel display
wait 400
expect editor.modal is display
# (the Kindle's refreshes as before: the pressed button, then the sheet with the button let go)
expect display.presents is fast 1718 1732 100 100, ui 0 0 1860 2480
expect live.panel is 1704 220 128 1842
expect live.stale.1704.220.128.1842 is 0
# The brightness dragged, a box ticked: it stays.
expect light.intensity is 12
drag touch 900 296 1100 296 3
wait 400
expect light.intensity is 7
expect live.panel is 1704 220 128 1842
expect live.stale.1704.220.128.1842 is 0
tap panel display.newpage
wait 400
expect settings.gesture_new_page is 0
expect live.panel is 1704 220 128 1842
expect live.stale.1704.220.128.1842 is 0
tap panel display.done
wait 400
expect editor.modal is none
# (the sheet again after the drag, the New page row pressed and the sheet again, the X pressed and the screen
# without the sheet)
expect display.presents is ui 0 0 1860 2480, fast 80 704 1700 120, ui 0 0 1860 2480, fast 1660 20 150 150, ui 0 0 1860 2480
expect live.panel is 1704 220 128 1842
expect live.stale.1704.220.128.1842 is 0
# Another sheet beside it (Paper), and closed again.
tap panel paper
wait 400
expect editor.modal is paper
expect live.panel is 1704 220 128 1842
expect live.stale.1704.220.128.1842 is 0
tap 100 2400
wait 400
expect editor.modal is none
expect live.stale.1704.220.128.1842 is 0
# The panel dragged into the middle, where the Display sheet covers its top: the layer holds the sheet there and
# the toolbar below it, as the Kindle shows them, while a setting changes and after the sheet closes.
drag touch 1737 260 933 760 5
wait 400
expect editor.panel_rect is 900 638 128 1842
expect live.panel is 900 638 128 1842
expect live.stale.900.638.128.1842 is 0
tap panel display
wait 400
expect editor.modal is display
expect live.panel is 900 638 128 1842
expect live.stale.900.638.128.1842 is 0
shot 02-display-over-the-panel
drag touch 1100 296 900 296 3
wait 400
expect light.intensity is 2
expect live.stale.900.638.128.1842 is 0
tap panel display.newpage
wait 400
expect settings.gesture_new_page is 1
expect live.stale.900.638.128.1842 is 0
tap panel display.done
wait 400
expect editor.modal is none
expect live.panel is 900 638 128 1842
expect live.stale.900.638.128.1842 is 0
drag touch 933 660 1737 243 5
wait 400
expect editor.panel_rect is 1704 220 128 1842
expect live.stale.1704.220.128.1842 is 0
# Settings over the notebook: the frame is the whole screen and the browser shows what the Kindle does while a
# setting changes there and in its Display sheet; closed, the toolbar's layer is back, in the style just picked.
tap panel settings
wait 600
expect editor.modal is settings
expect live.panel is
expect live.stale.1704.220.128.1842 is 0
tap 250 422
wait 100
tap library rotate
wait 600
expect settings.follow_rotation is 0
expect live.stale.1704.220.128.1842 is 0
tap library display
wait 600
expect live.stale.1704.220.128.1842 is 0
drag touch 900 296 1100 296 3
wait 400
expect light.intensity is 7
expect live.stale.1704.220.128.1842 is 0
tap library display.back
wait 600
expect editor.modal is settings
tap library dark
wait 600
expect settings.invert is 1
expect live.stale.1704.220.128.1842 is 0
tap library dark
wait 600
expect settings.invert is 0
tap library rotate
wait 600
expect settings.follow_rotation is 1
tap library theme
wait 600
expect settings.theme is classic
expect live.stale.1704.220.128.1842 is 0
tap library done
wait 600
expect editor.modal is none
expect live.panel is 1704 220 128 1842
expect live.stale.1704.220.128.1842 is 0
shot 03-classic-after-settings
quit
