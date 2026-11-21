# settings: status_mode=1
# storage: 1
# copy: tests/fixtures/scen-templates documents/Pics
# env: GRIMOIRE_COPY_FAIL_AFTER=100
# The Templates sheet's From a file... (2.2, #146) when the copy job's writes fail part way, as on a full storage:
# "Couldn't copy file. Is the storage full?", the sheet as it was (its tile back, no template), and nothing left in
# grimoire/templates (the copy's temporary file is gone).
# rust-only
wait 300
tap library add
tap library add.new-notebook
wait 200
tap library paper.more
wait 200
expect library.sheet is templates
tap library templates.from_file
wait 300
expect files.open is 1
tap library files.item.documents
wait 300
tap library files.item.Pics
wait 300
tap library files.item.Dots.jpg
wait 300
expect files.card.rows is Use as template|Cancel
tap library files.card.template
waitbg
wait 300
expect toast is Couldn't copy file. Is the storage full?
expect files.open is 0
expect library.sheet is templates
expect templates.copying is 0
expect templates.count is 0
expect storage.names.grimoire/templates is README.txt
shot 01-copy-failed
quit
