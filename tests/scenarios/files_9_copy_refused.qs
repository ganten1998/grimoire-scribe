# settings: status_mode=1
# settings: files_card=1
# storage: 1
# generate: tests/files_copy_refused.py
# A copy that is refused when it is asked for (2.2, #146). A picture bigger than the template job takes (16 MB): Use
# as template and Use as photo in Files say "Too big (over 16 MB).", no job runs, nothing is written and nothing says
# it was copied. The Templates sheet's From a file... says the same and leaves the picker up for another picture.
# rust-only
wait 300
tap library files-card
wait 300
tap library files.item.documents
wait 300
tap library files.item.Big
wait 300
tap library files.item.huge.png
wait 300
expect files.card is picture
expect files.card.rows is Use as template|Use as photo|Cancel
tap library files.card.template
wait 300
expect toast is Too big (over 16 MB).
expect files.card is
expect files.place is documents/Big
expect bg.children is 0
expect storage.names.grimoire/templates is README.txt
shot 01-too-big-for-a-template
# (the toast gone, so the next one is the photo's own)
wait 3200
expect toast is
tap library files.item.huge.png
wait 300
tap library files.card.photo
wait 300
expect toast is Too big (over 16 MB).
expect bg.children is 0
expect storage.names.grimoire/photos is README.txt
wait 3200
expect toast is
tap library files.back
wait 300
tap library files.back
wait 300
tap library files.back
wait 300
expect files.open is 0
# the Templates sheet's From a file...: refused the same way, the picker still up
tap library new
wait 200
tap library paper.more
wait 200
expect library.sheet is templates
tap library templates.from_file
wait 300
tap library files.item.documents
wait 300
tap library files.item.Big
wait 300
tap library files.item.huge.png
wait 300
expect files.card.rows is Use as template|Cancel
tap library files.card.template
wait 300
expect toast is Too big (over 16 MB).
expect files.open is 1
expect files.card is
expect files.place is documents/Big
expect bg.children is 0
shot 02-too-big-from-the-tile
tap library files.back
wait 300
tap library files.back
wait 300
tap library files.back
wait 300
expect files.open is 0
expect library.sheet is templates
expect templates.copying is 0
expect templates.count is 0
expect storage.names.grimoire/templates is README.txt
quit
