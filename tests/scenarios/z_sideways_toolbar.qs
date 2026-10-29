# rust-only
# root: z_bar_0_make
# settings: status_mode=1
# settings: landscape_fit=1
# Full-page landscape uses the outside margin and retains the chosen toolbar side.
wait 300
tap 200 400
wait 300
expect editor.panel_rect is 1704 220 128 1626
rotate 1
wait 200
expect editor.panel_rect is 1962 220 128 1626
shot 01-right-margin
rotate 0
wait 200
touch down 1736 260
touch move 100 260
touch up 100 260
wait 100
expect editor.panel_rect is 68 220 128 1626
rotate 1
wait 200
expect editor.panel_rect is 390 220 128 1626
shot 02-left-margin
rotate 0
wait 200
# With full-page landscape disabled, the existing saved placement remains in use.
key 17 0
expect settings.landscape_fit is 0
rotate 1
wait 200
expect editor.panel_rect is 68 220 128 1626
shot 03-width-view-unchanged
