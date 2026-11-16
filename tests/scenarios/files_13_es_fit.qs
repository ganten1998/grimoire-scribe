# settings: status_mode=1
# settings: files_card=1
# settings: shortcuts_enabled=1
# storage: 1
# generate: tests/files_trash_tree.py
# env: GRIMOIRE_LANG=es
# env: GRIMOIRE_CHECK_FIT=1
# Files' rename, delete and restore (2.2, issue #144) in Spanish, every text checked for fit (files_14_es_fit_classic
# runs the same in Classic): a template's card with Renombrar and Borrar, the rename keyboard and a refusal of each
# kind's words over it, the Delete question, the Trash place's rows with their sizes, an entry's card with Restaurar, and the toasts of
# each act; upright, then the card and the keyboard sideways (Esc, with the keys on, its Cancel). The taps are
# files_8_rename's and files_9_trash's.
# rust-only
wait 300
tap library files-card
wait 300
waitfor files.sub.Trash is grimoire/trash, 7 KB
shot 01-root
tap library files.item.Templates
wait 300
tap library files.item.Grid.png
wait 300
expect files.card.rows is Renombrar|Borrar|Cancelar
shot 02-card
tap library files.card.rename
wait 300
expect files.keyboard is 1
# each kind of refusal's words: a character, a space, a dot, the extension; then a name taken
tap 570 2365
tap 664 2045
tap 570 2365
tap 1685 2365
wait 200
expect toast is El nombre no puede contener :
shot 03-char
tap 1679 2205
key 107 0
tap 1018 2365
tap 1685 2365
wait 200
expect toast is El nombre no puede empezar ni terminar con un espacio.
shot 04-space
tap 1679 2205
tap 1466 2365
tap 1685 2365
wait 200
expect toast is El nombre no puede empezar ni terminar con un punto.
shot 05-dot
tap 1679 2205
tap 1679 2205
tap 1679 2205
tap 1679 2205
tap 1679 2205
tap 1685 2365
wait 200
expect toast is Mantén la extensión .png.
shot 06-extension
key 30 1
tap 175 2205
tap 1637 2045
tap 1372 1885
tap 1278 2205
tap 487 1885
tap 398 2045
tap 1466 2365
tap 1726 1885
tap 1278 2205
tap 929 2045
expect files.text is Lines.png
tap 1685 2365
wait 200
expect toast is Ese nombre ya existe.
shot 07-taken
tap 217 2365
wait 300
expect files.keyboard is 0
# Delete, then the Trash place and Restore
tap library files.item.Grid.png
wait 300
tap library files.card.trash
wait 500
expect files.card.details is ¿Borrar «Grid.png»?
expect files.card.rows is Borrar (lo mueve a grimoire/trash)|Conservarlo
shot 08-asked
tap library files.card.delete
wait 300
expect toast is Movido a grimoire/trash
shot 08-moved
tap library files.crumb.1
wait 300
tap library files.item.Trash
wait 300
waitfor files.row.1 is Grid.png
waitfor files.sub.1 has 1 KB,
waitfor files.sub.2 has 5 KB,
waitfor files.sub.3 has 2 KB,
shot 09-trash
tap library files.row.1
wait 300
expect files.card.rows is Restaurar|Cancelar
shot 10-trash-card
tap library files.card.restore
wait 300
expect toast is Restaurado en grimoire/templates.
shot 11-restored
# sideways: the trash card, then a template's card and the keyboard
rotate 1
wait 400
tap library files.row.1
wait 300
expect files.card is trashed
shot 12-trash-card-sideways
tap library files.card.cancel
wait 200
tap library files.crumb.1
wait 300
tap library files.item.Templates
wait 300
tap library files.item.Lines.png
wait 300
expect files.card is picture
shot 13-card-sideways
tap library files.card.trash
wait 500
expect files.card is delete
shot 13-asked-sideways
tap library files.card.cancel
wait 300
expect files.card is
tap library files.item.Lines.png
wait 300
tap library files.card.rename
wait 300
expect files.keyboard is 1
shot 14-keyboard-sideways
key 1 0
wait 300
expect files.keyboard is 0
quit
