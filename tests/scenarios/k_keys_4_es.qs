# settings: status_mode=1
# settings: shortcuts_enabled=1
# settings: mirror_pdf=0
# copy: tests/fixtures/keys-old-kindle .
# env: GRIMOIRE_LANG=es GRIMOIRE_CHECK_FIT=1
# rust-only
# The keyboard scheme's new words in Spanish, every one fitting (GRIMOIRE_CHECK_FIT: a text cut short fails the
# run): Exit Grimoire? with its two answers. An earlier starter map replaced and Ctrl+S say nothing (their toasts
# went in 2.0.0-beta.2).
wait 300
expect toast is
shot 01-upgraded
key 31 1
wait 100
expect toast is
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
