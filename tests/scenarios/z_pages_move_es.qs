# rust-only
# generate: tests/move_pages_fixture.py
# env: GRIMOIRE_CHECK_FIT=1 GRIMOIRE_LANG=es
# Move to... and Copy to notebook... in Spanish: the held page's card, the number pad's title and hint,
# the picker's line and At the end, the toasts, and the bottom row's Undo and Redo and their toasts, each
# without a text cut short.
wait 300
tap library folder.Void
tap library notebook.A
tap panel pages
waitbg
wait 200
tap panel pages.multi
tap panel pages.page.2
tap panel pages.page.3
touch down 930 551
wait 700
touch up 930 551
expect editor.pages.context is 2
shot 01-card
tap panel pages.move-to
wait 100
expect editor.keyboard is Mover antes de la página
expect toast has del 1 al 5, o 6 para el final.
shot 02-pad
tap panel pages.key.done
wait 200
expect editor.page_ids is 1,4,5,2,3
expect toast has Se movieron 2 páginas: ahora empiezan en la página 4.
# The bottom row's Undo (2.0.0-beta.2): refused on a full storage, then done
storage full
tap panel pages.undo-move
wait 200
expect toast is No se pudo deshacer el movimiento.
expect editor.page_ids is 1,4,5,2,3
storage ok
tap panel pages.undo-move
wait 200
expect toast is Movimiento deshecho: las páginas vuelven a su orden.
expect editor.page_ids is 1,2,3,4,5
shot 02b-undone
# Its Redo beside it (2.0.0-beta.2): refused on a full storage, then done, and undone again
storage full
tap panel pages.redo
wait 200
expect toast is No se pudo rehacer.
expect editor.page_ids is 1,2,3,4,5
storage ok
tap panel pages.redo
wait 200
expect toast is Movimiento rehecho: las páginas están donde las moviste.
expect editor.page_ids is 1,4,5,2,3
tap panel pages.undo-move
wait 200
# A new page's Undo and Redo, and its toasts
tap panel pages.new
wait 300
expect editor.page_ids is 1,6,2,3,4,5
tap panel pages.undo-add
wait 200
tap panel pages.redo
wait 200
expect toast is Rehecho: 1 página nueva otra vez, la página 2.
expect editor.page_ids is 1,6,2,3,4,5
shot 02c-redone
tap panel pages.undo-add
wait 200
expect editor.page_ids is 1,2,3,4,5
tap panel pages.page.1
tap panel pages.copy-to-notebook
wait 200
shot 03-picker
tap panel elsewhere.book.B
waitbg
wait 200
shot 04-picker-pages
tap panel elsewhere.end
wait 200
expect toast has Se copió 1 página a Destination B.
shot 05-copied
quit
