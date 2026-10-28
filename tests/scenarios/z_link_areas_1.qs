# rust-only
# The eye: dotted, numbered boxes over the page where a finger's tap follows a link. The link-areas
# action turns it on and off; the boxes are the links' ink padded to a finger's size, three of them
# here, tagged 1 2 1 (the bar's entries: page 3 twice, page 2). They are on the screen only, so a
# stroke across one still renders as the page does, and the setting is kept through a restart. The
# first time it comes on a toast says what the boxes are. (The bar's Areas button, which does the same,
# is in z_bar_1_list.)
# root: z_bar_0_make
wait 300
tap 200 400
wait 300
expect screen is editor
expect editor.page is 1
expect editor.areas is 0
expect settings.link_areas is 0
expect settings.hints is 0
key 30 0
wait 200
expect settings.link_areas is 1
expect editor.areas is 3
expect settings.hints is 4
expect toast is Dotted boxes show where a finger tap follows a link. They never print.
shot 01-areas
wait 4300
expect toast is
# off and on again: one refresh each, over the boxes alone, and no second tip
expect display.presents not -1
key 30 0
wait 100
expect settings.link_areas is 0
expect editor.areas is 0
expect display.presents is ui 297 997 606 906
key 30 0
wait 100
expect settings.link_areas is 1
expect editor.areas is 3
expect toast is
expect display.presents is ui 297 997 606 906
# a stroke across a box: the page canvas is what a fresh render draws (the boxes are not in it)
pen line 200 1050 1000 1100 30 0.5 0.5
expect editor.strokes is 4
expect editor.areas is 3
expect editor.live_matches_render is 1
shot 02-areas-after-a-stroke
# zoomed in, the boxes follow the page
pinch 930 1200 300 900 6
expect editor.zoomed is 1
expect editor.areas is 3
shot 03-areas-zoomed
# sideways
rotate 1
wait 1200
expect editor.zoomed is 0
expect editor.areas is 3
shot 04-areas-sideways
rotate 0
wait 1200
# kept through a restart
pentap panel library
waitbg
tap 200 400
wait 400
expect screen is editor
expect settings.link_areas is 1
expect editor.areas is 3
