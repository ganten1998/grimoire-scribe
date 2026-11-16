# settings: status_mode=1
# storage: 1
# copy: tests/fixtures/scen-templates documents/Pics
# New notebook > More papers... with no templates yet (2.2, #146): the sheet's From a file... tile opens the picker on
# pictures (no Import place; a folder lists its folders and pictures only), and a picture's card offers Use as
# template, which asks the copy job and brings the sheet back. Once the copy is in, the folder is read again: the new
# template's tile fills in when its preview is made, and is picked as New notebook's paper. Back at the picker's root
# goes back to the sheet.
# rust-only
wait 300
tap library new
wait 200
expect library.sheet is new
tap library paper.more
wait 200
expect library.sheet is templates
expect templates.count is 0
expect templates.copying is 0
shot 01-no-templates-yet
tap library templates.from_file
wait 300
expect files.open is 1
expect files.mode is pick
expect files.place is root
expect files.order is documents|All storage|Show Files in Library
shot 02-picker
tap library files.item.documents
wait 300
tap library files.item.Pics
wait 300
expect files.order is Dots.jpg|Lined.png
tap library files.item.Lined.png
wait 300
expect files.card is picture
expect files.card.rows is Use as template|Cancel
shot 03-card
tap library files.card.template
wait 100
expect files.open is 0
expect library.sheet is templates
# (the copy, then the preview the sheet asks for once it sees the copy in)
waitbg
wait 300
waitbg
wait 300
expect templates.copying is 0
expect templates.count is 1
expect templates.ready is 1
expect templates.names is Lined
expect storage.names.grimoire/templates is .cache|Lined.png|README.txt
expect toast is
shot 04-the-new-template
tap 232 452
wait 200
expect library.sheet is new
expect library.template is Lined
tap library paper.more
wait 200
expect templates.selected is Lined
tap library templates.from_file
wait 300
expect files.open is 1
tap library files.back
wait 300
expect files.open is 0
expect library.sheet is templates
expect templates.count is 1
quit
