# settings: status_mode=1
# settings: files_card=1
# storage: 1
# generate: tests/files_place_tree.py
# Files' picture card (2.2, #146): Use as template and Use as photo copy the picture into grimoire/templates or
# grimoire/photos by the copy job, and a toast says so once it is in. A name taken there gets " 2"; the copy job
# leaves nothing else behind. A picture already in grimoire/templates offers Use as photo only.
# rust-only
wait 300
tap library files-card
wait 300
tap library files.item.documents
wait 300
tap library files.item.Sorted
wait 300
tap library files.item.c.png
wait 300
expect files.card is picture
expect files.card.rows is Use as template|Use as photo|Cancel
shot 01-picture-card
tap library files.card.template
waitbg
wait 200
expect files.card is
expect files.place is documents/Sorted
expect toast is Copied to Templates.
expect storage.names.grimoire/templates is README.txt|c.png
expect storage.size.grimoire/templates/c.png is 2000
shot 02-copied-to-templates
tap library files.item.c.png
wait 300
tap library files.card.photo
waitbg
wait 200
expect toast is Copied to Photos.
expect storage.names.grimoire/photos is README.txt|c.png
# (again into templates: the name is taken, the copy is "c 2.png")
tap library files.item.c.png
wait 300
tap library files.card.template
waitbg
wait 200
expect toast is Copied to Templates.
expect storage.names.grimoire/templates is README.txt|c 2.png|c.png
# the copy in templates offers the copy into photos only
tap library files.crumb.1
wait 300
tap library files.item.Templates
wait 300
tap library files.item.c.png
wait 300
expect files.card is picture
expect files.card.rows is Use as photo|Cancel
shot 03-in-templates
tap library files.card.cancel
wait 200
expect files.card is
quit
