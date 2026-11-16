# rust-only
# generate: tests/add_pages_fixture.py
# env: GRIMOIRE_CHECK_FIT=1 GRIMOIRE_LANG=es
# settings: shortcuts_enabled=1
# copy: tests/fixtures/new-pages-key .
# The toolbar's count pad at the limit, in Spanish (owner, 2026-11-03): a notebook holds 2000 pages, so the pad
# takes at most what is left, and a digit past it leaves the count there, which shows how many fit (no toast), as
# the Pages view's pad does. Once full, the new-pages key (the + held opens the pad through the same path) says so
# instead of opening the pad, and so does the Pages view's New page tapped. The panel's page chip shows the page
# alone on every page of a notebook where the page and the count would not fit it on some page (#119): "1" on page
# 1 of 1995 pages and of 2000, "2000" on the last page, upright and sideways. Then a small notebook, whose chip
# shows the page and the count as before ("2 / 2", then "2 / 3"): the toolbar's pad in the singular, and New page
# tapped in the Pages view, which stays, the tile marked new. Every text fits without being cut short.
wait 300
tap library folder.Void
tap library notebook.F
expect editor.pages is 1995
expect editor.page is 1
expect editor.page_chip is 1
key 49 0
wait 200
expect editor.addpad is 1
expect editor.keyboard is Páginas que añadir
expect editor.addpad.done is Añadir 1 página
tap panel addpad.key.9
wait 100
expect editor.text is 5
expect toast is
expect editor.addpad.done is Añadir 5 páginas
shot 01-clamped
tap panel addpad.key.done
wait 300
expect editor.addpad is 0
expect editor.pages is 2000
expect editor.page is 1
expect editor.page_chip is 1
expect toast is
shot 02-added
# The last page: the page alone on the chip, upright and sideways
key 37 0
wait 300
expect editor.page is 2000
expect editor.page_chip is 2000
rotate 1
wait 300
expect editor.page_chip is 2000
shot 02b-last-sideways
rotate 0
wait 300
expect editor.page_chip is 2000
# Full: the new-pages key says so and opens no pad
key 49 0
wait 200
expect editor.addpad is 0
expect toast is Cuaderno lleno con 2000 páginas.
# The Pages view's New page tapped in the full notebook: the same, the view staying
tap panel pages
waitbg
wait 200
tap panel pages.new
wait 300
expect editor.modal is pages
expect editor.pages is 2000
expect toast is Cuaderno lleno con 2000 páginas.
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
expect editor.page_chip is 2 / 2
touch down 1768 1330
wait 700
touch up 1768 1330
wait 100
expect editor.addpad.done is Añadir 1 página
tap panel addpad.key.done
wait 300
expect editor.page_ids is 1,2,3
expect editor.page is 2
expect editor.page_chip is 2 / 3
expect toast is
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
expect toast is
expect editor.tile.2 is Página 2 (abierta)
expect editor.tile.3 is Página 3 (nueva)
expect editor.tile.4 is Página 4
shot 04-pages-new
quit
