# rust-only
# generate: tests/add_pages_fixture.py
# env: GRIMOIRE_CHECK_FIT=1 GRIMOIRE_LANG=es
# New page's batch at the limit, in Spanish (owner, 2026-11-03): a notebook holds 2000 pages, so the count pad
# takes at most what is left; a digit or + past it leaves the count there, which shows how many fit (no toast).
# Done adds them all, and then New page held says the notebook is full instead of opening the pad. The pad's
# title, Done and that toast fit without a text cut short.
wait 300
tap library folder.Void
tap library notebook.F
expect editor.pages is 1995
tap panel pages
waitbg
wait 200
touch down 394 2395
wait 700
touch up 394 2395
wait 100
expect editor.pages.adding is 1
expect editor.pages.keyboard is Páginas que añadir
expect editor.pages.text is 1
expect editor.pages.add.done is Añadir 1 página
tap panel pages.key.1
wait 100
expect editor.pages.add.done is Añadir 1 página
tap panel pages.key.2
wait 100
expect editor.pages.text is 5
expect toast is
expect editor.pages.add.done is Añadir 5 páginas
shot 01-clamped
tap panel pages.key.inc
wait 100
expect editor.pages.text is 5
tap panel pages.key.dec
tap panel pages.key.dec
tap panel pages.key.dec
tap panel pages.key.dec
tap panel pages.key.dec
wait 100
expect editor.pages.text is 1
tap panel pages.key.del
wait 100
expect editor.pages.text is
expect editor.pages.add.done.on is 0
tap panel pages.key.9
tap panel pages.key.done
wait 300
expect editor.pages.adding is 0
expect editor.pages is 2000
expect toast is
expect editor.pages.undo.kind is add
shot 02-added
# Full: held, New page says so and opens no pad
touch down 394 2395
wait 700
touch up 394 2395
wait 100
expect editor.pages.adding is 0
expect toast is Cuaderno lleno con 2000 páginas.
expect editor.pages is 2000
# Undo takes the five back
tap panel pages.undo-add
wait 300
expect editor.pages is 1995
expect toast is
# The small notebook: the pad's own 1, Done at once, is one page, in the singular
tap panel pages.back
wait 200
tap panel library
waitbg
tap library notebook.S
tap panel pages
wait 200
touch down 394 2395
wait 700
touch up 394 2395
wait 100
expect editor.pages.add.done is Añadir 1 página
tap panel pages.key.done
wait 300
expect editor.page_ids is 1,3,2
expect toast is
quit
