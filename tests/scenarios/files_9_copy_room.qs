# settings: status_mode=1
# settings: files_card=1
# storage: 1
# generate: tests/files_place_tree.py
# env: GRIMOIRE_COPY_FREE_BYTES=104858000
# Files' picture card (2.2, #146) on a storage that would keep less than its floor (100 MB) after the copy: Use as
# template is refused at once, "Couldn't copy file. Is the storage full?", no job runs and nothing is written; the
# card is gone and Files stays where it was.
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
expect files.card.rows is Use as template|Use as photo|Cancel
tap library files.card.template
wait 300
expect toast is Couldn't copy file. Is the storage full?
expect files.card is
expect files.place is documents/Sorted
expect bg.children is 0
expect storage.names.grimoire/templates is README.txt
shot 01-storage-full
quit
