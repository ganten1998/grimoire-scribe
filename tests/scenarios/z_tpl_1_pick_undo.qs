# settings: status_mode=1
# The editor's Paper sheet > More papers... > a template: the page goes onto it as onto any paper,
# one undo step, and its picture (made for this notebook in the background) is drawn once it is
# there. The precise eraser over ink on it brings the picture back, undo and redo of the change are
# exact, a page added after it gets it too, the Paper sheet's More papers tile shows it and the
# Templates sheet selects it, and both pages keep it when the notebook is opened again.
# copy: tests/fixtures/scen-templates templates
# settings: eraser_mode=1
# rust-only
wait 300
tap 965 95
wait 100
tap 1564 2254
wait 400
expect screen is editor
expect editor.paper is 1
pen line 300 640 1300 700 40 0.6 0.6
wait 300
expect editor.strokes is 1
pentap panel paper
wait 200
expect editor.modal is paper
pentap 232 1750
wait 300
expect editor.modal is templates
expect templates.count is 2
expect templates.selected is
# (the previews of the two, made in the background meanwhile: waitbg waits for background jobs as
# long as the machine takes)
waitbg
wait 300
expect templates.ready is 2
shot 01-templates
pentap 511 452
wait 300
expect editor.modal is none
expect editor.paper_hex is 0x50f4d83c
waitbg
wait 300
expect editor.picture is shown
expect editor.live_matches_render is 1
shot 02-on-lined
eraser line 790 560 810 780 15
wait 500
expect editor.erase_strokes is 1
expect editor.live_matches_render is 1
shot 03-erased-to-the-picture
# Undone (the erase stroke, then the paper): medium rule again; then both redone.
pentap panel undo
wait 200
pentap panel undo
wait 300
expect editor.paper is 1
expect editor.erase_strokes is 0
expect editor.live_matches_render is 1
shot 04-undone
pentap panel redo
wait 300
expect editor.paper_hex is 0x50f4d83c
expect editor.picture is shown
expect editor.live_matches_render is 1
pentap panel redo
wait 300
expect editor.erase_strokes is 1
expect editor.live_matches_render is 1
# A page added after it.
pentap panel next
wait 400
expect editor.page is 2
expect editor.paper_hex is 0x50f4d83c
expect editor.picture is shown
pentap panel paper
wait 200
shot 05-paper-sheet-on-a-template
pentap 232 1750
wait 300
expect templates.selected is Lined
shot 06-lined-selected
pentap 300 2123
wait 200
expect editor.modal is paper
tap 1562 2125
wait 200
expect editor.modal is none
pentap panel library
wait 600
expect screen is library
tap 326 608
wait 600
expect editor.page is 2
expect editor.paper_hex is 0x50f4d83c
expect editor.picture is shown
pentap panel previous
wait 400
expect editor.page is 1
expect editor.paper_hex is 0x50f4d83c
expect editor.erase_strokes is 1
expect editor.live_matches_render is 1
quit
