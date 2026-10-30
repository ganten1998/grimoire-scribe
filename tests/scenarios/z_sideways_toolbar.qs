# rust-only
# root: z_bar_0_make
# settings: status_mode=1
# settings: landscape_fit=1
# Full-page landscape uses the outside margin and retains the chosen toolbar side.
wait 300
tap 200 400
wait 300
expect editor.panel_rect is 1704 220 128 1734
rotate 1
wait 200
expect editor.panel_rect is 1962 114 128 1734
shot 01-right-margin
# Flipping between landscape orientations also retains the margin placement.
rotate 3
wait 200
expect editor.panel_rect is 1962 114 128 1734
rotate 0
wait 200
touch down 1736 260
touch move 100 260
touch up 100 260
wait 100
expect editor.panel_rect is 68 220 128 1734
rotate 1
wait 200
expect editor.panel_rect is 390 114 128 1734
shot 02-left-margin
rotate 0
wait 200
# With full-page landscape disabled, the existing saved placement remains in use.
key 17 0
expect settings.landscape_fit is 0
rotate 1
wait 200
expect editor.panel_rect is 68 114 128 1734
shot 03-width-view-unchanged
