# rust-only
# generate: tests/add_pages_fixture.py
# env: GRIMOIRE_CHECK_FIT=1 GRIMOIRE_LANG=es
# settings: shortcuts_enabled=1
# copy: tests/fixtures/new-pages-key .
# The toolbar's count pad at the limit, in Spanish (owner, 2026-11-03): a notebook holds 2000 pages, so the pad
# takes at most what is left, and a digit past it leaves the count there with a toast saying how many fit, as the
# Pages view's pad does. Once full, the new-pages key (the + held opens the pad through the same path) says so
# instead of opening the pad, and so does the Pages view's New page tapped. (Page 1 here: the panel's page count
# "1995 / 2000" does not fit its chip, docs/BACKLOG.md.) Then a small notebook: the toolbar's toast in the singular, and New page tapped
# in the Pages view, which stays, the tile marked new. Every text fits without being cut short.
wait 300
tap library folder.Void
tap library notebook.F
expect editor.pages is 1995
expect editor.page is 1
key 49 0
wait 200
expect editor.addpad is 1
expect editor.keyboard is Páginas que añadir
expect editor.addpad.done is Añadir 1 página
tap panel addpad.key.9
wait 100
expect editor.text is 5
expect toast is A este cuaderno le caben 5 páginas más.
expect editor.addpad.done is Añadir 5 páginas
shot 01-clamped
tap panel addpad.key.done
wait 300
expect editor.addpad is 0
expect editor.pages is 2000
expect editor.page is 1
expect toast is Se añadieron 5 páginas tras esta, desde la página 2. Se deshacen en Páginas.
shot 02-added
# Full: the new-pages key says so and opens no pad
key 49 0
wait 200
expect editor.addpad is 0
expect toast is Este cuaderno está lleno: tiene 2000 páginas.
# The Pages view's New page tapped in the full notebook: the same, the view staying
tap panel pages
waitbg
wait 200
tap panel pages.new
wait 300
expect editor.modal is pages
expect editor.pages is 2000
expect toast is Este cuaderno está lleno: tiene 2000 páginas.
# The small notebook: one page from the toolbar's pad, in the singular, the editor staying on page 2
tap panel pages.back
wait 200
tap panel library
waitbg
tap library notebook.S
wait 300
key 37 0
wait 300
expect editor.page is 2
touch down 1768 1330
wait 700
touch up 1768 1330
wait 100
expect editor.addpad.done is Añadir 1 página
tap panel addpad.key.done
wait 300
expect editor.page_ids is 1,2,3
expect editor.page is 2
expect toast is Se añadió 1 página tras esta: la página 3. Se deshace en Páginas.
shot 03-one
# New page tapped in the Pages view: one page after the open one, the view staying, its tile marked new
tap panel pages
wait 300
expect editor.tile.3 is Página 3 (nueva)
tap panel pages.new
wait 300
expect editor.modal is pages
expect editor.page_ids is 1,2,4,3
expect editor.page is 2
expect toast is Se añadió 1 página: es la página 3.
expect editor.tile.2 is Página 2 (abierta)
expect editor.tile.3 is Página 3 (nueva)
expect editor.tile.4 is Página 4
shot 04-pages-new
quit
