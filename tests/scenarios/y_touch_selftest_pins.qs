# settings: status_mode=1
# rust-only
# tap panel NAME with three pens pinned under the tool button and, sideways, its metrics scaled
# to fit: Undo, Redo and the pinned pens are found where they are drawn. A legacy four-pin
# setting loads only its first three pins.
# settings: mirror_pdf=0
# settings: panel_tools=highlighter,fineliner,marker,charcoal
wait 300
tap 965 95
wait 100
tap 510 460
tap 1564 2254
wait 400
expect screen is editor
expect editor.pins is highlighter,fineliner,marker
pen line 300 700 1300 720 20 0.6 0.6
tap panel undo
expect editor.strokes is 0
pentap panel tool:marker
wait 200
expect editor.tool is marker
pentap panel eraser
wait 200
expect editor.tool is eraser
pentap panel tool:marker
wait 200
expect editor.tool is marker
# sideways
rotate 1
wait 400
pen line 300 700 1300 720 20 0.6 0.6
tap panel undo
expect editor.strokes is 0
pentap panel tool:fineliner
wait 200
expect editor.tool is fineliner
pentap panel tool-picker
wait 200
expect editor.tool is pencil
pentap panel tool-picker
wait 300
expect editor.modal is tools
