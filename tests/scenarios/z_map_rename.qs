# rust-only
# root: z_bar_0_make
# env: GRIMOIRE_CHECK_FIT=1
wait 300
tap 200 400
key 37 0
tap 1590 2370
tap map-node 4
expect editor.map.selected is 4
tap 1540 2380
expect editor.keyboard is Name in the Index
expect editor.text is
tap 221 2045
expect editor.text is a
tap 1643 2365
expect editor.index_rows is 3
expect toast is Page 4 is in the Index as a.
expect editor.map is graph
shot 01-renamed
# Pan with a pen, preserving notebook ink.
drag pen 900 900 1200 1100 20
expect editor.strokes is 0
# Open page 3, whose incoming chip counts distinct source pages despite two links.
tap map-node 3
tap map-node 3
expect editor.page is 3
key 37 0
expect editor.bar.incoming is 1
shot 02-incoming

tap editor bar.incoming
expect editor.map is graph
expect editor.map.mode is around
tap 100 95
expect editor.map is 0
