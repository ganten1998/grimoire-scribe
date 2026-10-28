# settings: status_mode=1
# env: GRIMOIRE_LANG=es
# Settings > Language. Automatic follows the system's language (in the simulator GRIMOIRE_LANG,
# here Spanish); a pick applies at once (the sheet comes back in it) and is saved, and
# z_language_kept starts again from what this run saved.
wait 200
expect language is es
expect settings.language is auto
tap 1535 95
wait 200
expect library.sheet is settings
shot 01-settings-automatic-spanish
tap 930 1580
wait 200
expect library.sheet is language
shot 02-languages
# English, the second row: at once, and saved
tap 930 945
wait 300
expect library.sheet is settings
expect language is en
expect settings.language is en
shot 03-settings-english
# Cancel in the list changes nothing
tap 930 1580
wait 200
tap 930 1265
wait 200
expect library.sheet is settings
expect language is en
quit
