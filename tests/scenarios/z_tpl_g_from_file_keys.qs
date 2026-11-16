# settings: status_mode=1
# settings: shortcuts_enabled=1
# storage: 1
# copy: tests/fixtures/pictures documents/Many
# copy: tests/fixtures/photos documents/Many
# copy: tests/fixtures/scen-templates documents/Many
# The picture pick over a notebook's page (2.2, #146), its keys. While it is up the page's keys do nothing (undo, a
# new page), the page keys turn the pick's screens and not the notebook's pages, and Esc goes up a level and, at the
# pick's root, back to the Templates sheet. With the pick gone the keys are the page's again.
# rust-only
wait 300
tap 965 95
wait 100
tap 1564 2254
wait 400
expect screen is editor
pen line 300 640 1300 700 40 0.6 0.6
wait 300
expect editor.strokes is 1
pentap panel paper
wait 200
pentap panel paper.more
wait 300
pentap panel templates.from_file
wait 300
expect editor.modal is files
expect files.place is root
# undo (Ctrl+Z) and a new page (Ctrl+N) do nothing under the pick
key 44 1
wait 300
expect editor.strokes is 1
key 49 1
wait 300
expect editor.pages is 1
expect editor.modal is files
# PageDown and PageUp turn the pick's screens
pentap panel files.item.documents
wait 300
pentap panel files.item.Many
wait 300
expect files.rows is 17
expect files.pages is 2
expect files.page is 1
key 109 0
wait 300
expect files.page is 2
expect editor.page is 1
shot 01-the-second-screen
key 104 0
wait 300
expect files.page is 1
# Esc: up a level, then at the root back to the sheet
key 1 0
wait 300
expect files.place is documents
expect editor.modal is files
key 1 0
wait 300
expect files.place is root
expect editor.modal is files
key 1 0
wait 300
expect files.open is 0
expect editor.modal is templates
expect editor.strokes is 1
expect editor.pages is 1
# the pick gone and the sheets closed (Esc: the Templates sheet, then Paper), undo is the page's again
key 1 0
wait 300
expect editor.modal is paper
key 1 0
wait 300
expect editor.modal is none
key 44 1
wait 300
expect editor.strokes is 0
quit
