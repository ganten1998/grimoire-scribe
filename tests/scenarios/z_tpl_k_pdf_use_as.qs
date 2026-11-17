# settings: status_mode=1
# settings: files_card=1
# storage: 1
# copy: tests/fixtures/pdf/pick documents/Plans
# Files' PDF card (2.2, #172): Use as template copies the PDF into grimoire/templates by the copy job, as a picture's
# card does, and a toast says so. New notebook > More layouts... then shows its pages, a tile each, and the PDF itself
# stays where it was, never imported.
# rust-only
wait 300
tap library files-card
wait 300
tap library files.item.documents
wait 300
tap library files.item.Plans
wait 300
tap library files.item.Planner.pdf
wait 300
expect files.card is pdf
expect files.card.rows is Import as notebook|Use as template|Cancel
shot 01-pdf-card
tap library files.card.template
waitbg
wait 200
expect files.card is
expect toast is Copied to Templates.
expect storage.names.grimoire/templates is Planner.pdf|README.txt
expect storage.names.documents/Plans is Planner.pdf
tap library files.back
wait 300
tap library files.back
wait 300
tap library files.back
wait 300
expect files.open is 0
tap library new
wait 200
tap library paper.more
wait 200
expect library.sheet is templates
waitbg
wait 300
waitbg
wait 300
expect templates.count is 2
expect templates.ready is 2
expect templates.names is Planner p. 1|Planner p. 2
shot 02-its-pages
# (the Files card alone: no notebook was made of the PDF)
expect library.items is 1
quit
