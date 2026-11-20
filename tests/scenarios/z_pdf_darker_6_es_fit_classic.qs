# settings: status_mode=1
# settings: theme=classic
# env: GRIMOIRE_LANG=es
# env: GRIMOIRE_CHECK_FIT=1
# Darker lines in imported PDFs (2.2, issue #173) in Spanish and Classic, every text checked for fit
# (as z_pdf_darker_5_es_fit does in Gothic): Settings > System with the row at each of its three levels, and the row's list,
# upright and sideways.
# rust-only
tap library settings
wait 100
tap library settings.cat.6
wait 100
shot 01-row-off
tap library darker-lines
wait 100
expect library.sheet is darker-lines
shot 02-list
tap library darker-lines.2
wait 100
expect settings.pdf_gamma is 2.0
shot 03-row-strong
tap library darker-lines
wait 100
tap library darker-lines.1
wait 100
expect settings.pdf_gamma is 1.5
shot 04-row-medium
rotate 1
wait 400
shot 05-row-sideways
tap library darker-lines
wait 100
expect library.sheet is darker-lines
shot 06-list-sideways
tap library darker-lines.back
wait 100
rotate 0
wait 400
tap library done
wait 100
quit
