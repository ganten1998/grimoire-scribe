# settings: status_mode=1
# settings: shortcuts_enabled=1
# settings: mirror_pdf=0
# copy: tests/fixtures/keys-old-kindle .
# env: GRIMOIRE_LANG=es GRIMOIRE_CHECK_FIT=1
# rust-only
# The keyboard scheme's new words in Spanish, every one fitting (GRIMOIRE_CHECK_FIT: a text cut short fails the
# run): the toast for an earlier starter map replaced, Ctrl+S's, and Exit Grimoire? with its two answers.
wait 300
expect toast is Atajos de teclado: las teclas iniciales nuevas están en shortcuts.ini.
shot 01-upgraded
key 31 1
wait 100
expect toast is Guardado
key 1 0
wait 100
expect library.sheet is exit
shot 02-exit
key 106 0
wait 100
expect library.exit is no
key 28 0
wait 100
expect library.sheet is none
expect running is 1
