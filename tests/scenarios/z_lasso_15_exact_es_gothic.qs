# settings: status_mode=1
# settings: mirror_pdf=0
# settings: language=es
# settings: theme=gothic
# env: GRIMOIRE_CHECK_FIT=1
# rust-only
# Select mode's bar in Spanish (gothic), with the fit check on: the mode button's two names (Trazos enteros,
# Área exacta) and the bar under a selection with the clipboard full (its widest) all fit; a run that cuts any
# text short fails.
wait 300
tap 965 95
wait 100
tap 1564 2254
wait 200
expect screen is editor
pentap panel tool-picker
pentap 872 561
wait 100
pen line 300 700 1300 700 40 0.6
pen leave
wait 100
tap panel tool-picker
wait 100
tap panel picker.select
wait 100
expect editor.select is armed
shot 01-armed
tap panel select.mode
wait 100
expect editor.sel_exact is 1
shot 02-armed-exact
pen down 700 600
pen move 1500 600
pen move 1500 800
pen move 700 800
pen up 700 650
wait 100
expect editor.select is selected
tap panel select.copy
wait 100
shot 03-selected-exact
tap panel select.mode
wait 100
expect editor.sel_exact is 0
shot 04-selected-whole
quit
