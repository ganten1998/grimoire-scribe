# settings: status_mode=1
# settings: files_card=1
# storage: 1
# generate: tests/files_trash_tree.py
# Restore a notebook the Library deleted (2.2, issue #144): Alpha's menu, Delete, its confirmation ("Moved to
# grimoire/trash"); then Files > Trash lists it first, by its name, and its card's Restore puts it back
# ("Restored to Notebooks.") and the Library under Files has it again at once. The trash's older notebook, also
# named Beta as the Beta at the Library's top, comes back as "Beta 2", in its place by its date.
# rust-only
wait 300
expect library.items is 4
expect library.card.3.name is Alpha
tap library notebook-menu.20261101-000000-0001
wait 200
expect library.sheet is more
tap library more.delete
wait 500
expect library.sheet is delete
tap library delete.yes
wait 300
expect toast is Moved to grimoire/trash
expect library.items is 3
tap library files-card
wait 300
tap library files.item.Trash
wait 300
expect files.rows is 3
waitfor files.row.1 is Alpha
tap library files.row.1
wait 300
expect files.card is trashed
expect files.card.rows is Restore|Cancel
shot 01-card
tap library files.card.restore
wait 300
expect toast is Restored to Notebooks.
expect library.items is 4
expect files.rows is 2
waitfor files.row.1 is Beta
tap library files.row.1
wait 300
tap library files.card.restore
wait 300
expect toast is Restored to Notebooks.
expect library.items is 5
expect files.rows is 1
expect files.row.1 is Alpha
expect files.kind.1 is trashed
shot 02-trash
tap library files.back
wait 300
tap library files.back
wait 300
expect files.open is 0
expect library.items is 5
expect library.card.2.name is Beta 2
expect library.card.3.name is Beta
expect library.card.4.name is Alpha
shot 03-library
quit
