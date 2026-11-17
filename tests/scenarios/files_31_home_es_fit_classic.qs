# settings: status_mode=1
# settings: files_card=1
# settings: shortcuts_enabled=1
# settings: home=files/documents/Nowhere
# storage: 1
# generate: tests/files_home_tree.py
# env: GRIMOIRE_LANG=es
# env: GRIMOIRE_CHECK_FIT=1
# settings: theme=classic
# Home (2.2, issue #160) in Spanish and Classic, every text checked for fit, as files_30_home_es_fit does in Gothic,
# with the same taps: the toast of a home not found at the start, a folder's card in Files with Usar como inicio, the
# Go to sheet with Inicio: and Usar como inicio, then with Restablecer el inicio a Cuadernos, upright and sideways,
# Exit Grimoire? over Files at home, and a Library folder's menu with each of its two home rows.
# rust-only
wait 300
expect toast is No se encontró la carpeta de inicio.
shot 01-not-found
tap library files-card
wait 300
tap library files.item.documents
wait 300
expect files.place is documents
tap library files.more.1
wait 300
expect files.card is folder
expect files.card.rows has |Usar como inicio|Cancelar
shot 02-folder-card
tap library files.card.cancel
wait 300
tap library files.item.Sorted
wait 300
tap library files.crumb.3
wait 300
expect files.card.rows has Inicio: Cuadernos|
expect files.card.rows has |Usar como inicio|Cancelar
shot 03-go-to
tap library files.card.sethome
wait 300
expect settings.home is files/documents/Sorted
tap library files.crumb.3
wait 300
expect files.card.rows has Inicio: Sorted|
expect files.card.rows has |Restablecer el inicio a Cuadernos|Cancelar
shot 04-go-to-at-home
tap library files.card.cancel
wait 300
key 1 0
wait 300
expect library.sheet is exit
shot 05-exit
key 1 0
wait 300
rotate 1
wait 400
tap library files.crumb.3
wait 300
expect files.card is goto
shot 06-go-to-sideways
tap library files.card.cancel
wait 300
rotate 0
wait 400
tap library files.crumb.1
wait 300
tap library files.back
wait 300
expect files.open is 0
tap library folder-menu.School
wait 300
shot 07-folder-menu
tap library fmore.home
wait 300
expect settings.home is notebooks/School
tap library folder-menu.School
wait 300
shot 08-folder-menu-home
tap library done
wait 300
quit
