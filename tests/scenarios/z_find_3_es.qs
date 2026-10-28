# rust-only
# The Find sheet in Spanish, upright and sideways, with its rows, under the fit check: its title, the rows, the
# line that says there is no such page.
# root: z_bar_0_make
# env: GRIMOIRE_LANG=es GRIMOIRE_CHECK_FIT=1
wait 300
tap 200 400
wait 300
key 34 0
wait 300
expect editor.find is go
expect editor.keyboard is Número o nombre de página
tap 221 2045
tap 664 1885
expect editor.find.rows is 2
shot 01-find-es-upright
tap 1679 2205
tap 1679 2205
tap 1549 1725
expect editor.find.1 is No hay página 9 (5 páginas).
shot 02-find-es-no-page
rotate 1
wait 1200
expect editor.find is go
expect editor.find.rows is 1
shot 03-find-es-sideways
