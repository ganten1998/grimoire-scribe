# env: GRIMOIRE_SIM_ADDRS=en0=192.168.0.20
# rust-only
# Live view's chip (it says a computer is watching): on a notebook page it sits BESIDE THE PANEL and moves with it,
# not in the top right corner: left of the panel, level with its header, or right of it where the panel is at the
# screen's left edge; it goes while a menu, sheet, bar or the Pages view is in front (they draw over the panel);
# the other screens (Share, Library) keep it in the top right corner.
wait 300
tap 1345 95
wait 300
tap 318 2100
wait 300
expect live.on is 1
tap 1538 2100
wait 300
expect screen is library
expect live.chip is 1814 4 42 42
tap 965 95
wait 100
tap 510 460
tap 1564 2254
wait 400
expect screen is editor
expect live.on is 1
# Beside the panel (the panel is 128 wide and 1518 tall at x 1704, y 220, its header 80 tall): left of it, level with the header.
expect editor.panel_rect is 1704 220 128 1518
expect live.chip is 1654 239 42 42
shot 01-beside-the-panel
# It follows the panel: dragged to the middle of the page, the chip comes along; dragged to the left edge it is on
# the panel's right.
drag touch 1737 260 933 760 5
wait 400
expect editor.panel_rect is 900 720 128 1518
expect live.chip is 850 739 42 42
drag touch 933 760 33 760 5
wait 400
expect editor.panel_rect is 0 720 128 1518
expect live.chip is 136 739 42 42
shot 02-left-edge
# Folded, the panel is one box and the chip is level with it.
tap panel collapse
wait 400
expect editor.panel is collapsed
expect live.chip is 136 759 42 42
shot 03-folded
tap panel expand
wait 400
# A menu in front: no chip while it is up, and it is back when it closes.
tap panel display
wait 400
expect editor.modal is display
expect live.chip is
tap 70 70
wait 400
expect editor.modal is none
expect live.chip is 136 739 42 42
# The Pages view covers everything: none there either.
tap panel pages
wait 500
expect editor.modal is pages
expect live.chip is
tap 100 100
wait 500
expect editor.modal is none
expect live.chip is 136 739 42 42
# Back on the Library it is in the top right corner again, and gone with Live view.
tap panel library
wait 600
expect screen is library
expect live.chip is 1814 4 42 42
tap 1345 95
wait 300
tap 318 2100
wait 300
expect live.pairing is 1
tap 318 2100
wait 300
expect screen is library
expect live.on is 0
expect live.chip is
quit
