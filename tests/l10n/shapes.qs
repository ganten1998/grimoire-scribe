# Smart Shapes controls in every language/style, without English text assertions.
wait 200
tap 965 95
wait 100
tap 1564 2254
wait 200
tap panel shapes
expect editor.shapes_menu is 1
shot 01-defaults
tap panel shapes.quads
tap panel shapes.round
tap panel shapes.hold
shot 02-squares-circles-slow
tap panel shapes.quads
tap panel shapes.round
tap panel shapes.hold
tap panel shapes.links
tap panel shapes.on
shot 03-uneven-as-drawn-quick-off
tap panel shapes.done
rotate 1
wait 200
tap panel shapes
expect editor.shapes_menu is 1
shot 04-landscape-options
tap panel shapes.quads
tap panel shapes.round
tap panel shapes.hold
tap panel shapes.links
tap panel shapes.on
shot 05-landscape-defaults
tap panel shapes.done
