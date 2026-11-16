# settings: status_mode=1
# storage: 1
# copy: tests/fixtures/scen-templates documents/Pics
# copy: tests/fixtures/photos documents/Camera
# The editor's Paper sheet > More papers... and Photos... (2.2, #146): their From a file... tile opens the picture
# pick over the page, hosted by the editor, so the notebook stays open as it was (its page, a stroke on it and its
# undo). Back at the picker's root goes back to the sheet. Use as template copies the picture in and brings the sheet
# back, where the new template's tile fills in once its preview is made and becomes the page's paper; two undos then
# take back the paper and the stroke drawn before. The page's idle save runs under the picker, and a turn and a wake
# keep it up. The same from Photos... with a photo.
# rust-only
wait 300
tap 965 95
wait 100
tap 1564 2254
wait 400
expect screen is editor
expect editor.paper is 5
pen line 300 640 1300 700 40 0.6 0.6
wait 300
expect editor.strokes is 1
pentap panel paper
wait 200
pentap panel paper.more
wait 300
expect editor.modal is templates
expect templates.count is 0
pentap panel templates.from_file
wait 300
expect editor.modal is files
expect files.open is 1
expect files.mode is pick
expect files.order is documents|All storage|Show Files in Library
shot 01-picker-over-the-page
# the page's idle save runs under the picker, as under any sheet (the stroke was drawn a moment before)
wait 2500
expect editor.unsaved is 0
# a turn and a wake keep the picker over the page
rotate 1
wait 400
expect editor.modal is files
expect files.open is 1
rotate 0
wait 400
sleep
wait 200
wake
wait 500
expect editor.modal is files
expect files.place is root
pentap panel files.back
wait 300
expect files.open is 0
expect editor.modal is templates
expect templates.count is 0
pentap panel templates.from_file
wait 300
pentap panel files.item.documents
wait 300
pentap panel files.item.Pics
wait 300
expect files.crumbs is Files > Documents > Pics
pentap panel files.item.Lined.png
wait 300
expect files.card is picture
expect files.card.rows is Use as template|Cancel
pentap panel files.card.template
wait 100
expect files.open is 0
expect editor.modal is templates
waitbg
wait 300
waitbg
wait 300
expect templates.copying is 0
expect templates.count is 1
expect templates.ready is 1
expect templates.names is Lined
expect toast is
shot 02-the-new-template
pentap 232 452
wait 300
expect editor.modal is none
expect editor.page is 1
expect editor.strokes is 1
expect editor.paper_hex is 0x50f4d83c
pentap panel undo
wait 300
expect editor.paper is 5
expect editor.strokes is 1
pentap panel undo
wait 300
expect editor.strokes is 0
# Photos...: the same, a photo
pentap panel paper
wait 200
pentap panel paper.photos
wait 300
expect editor.modal is templates
expect templates.count is 0
pentap panel templates.from_file
wait 300
expect editor.modal is files
pentap panel files.item.documents
wait 300
pentap panel files.item.Camera
wait 300
pentap panel files.item.IMG_0002.png
wait 300
expect files.card.rows is Use as photo|Cancel
pentap panel files.card.photo
wait 100
expect editor.modal is templates
waitbg
wait 300
waitbg
wait 300
expect templates.count is 1
expect templates.ready is 1
expect templates.names is IMG_0002
expect storage.names.grimoire/photos is .cache|IMG_0002.png|README.txt
shot 03-the-new-photo
pentap 232 452
wait 300
expect editor.modal is none
expect annotate.role is document
quit
