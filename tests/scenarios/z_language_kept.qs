# settings: status_mode=1
# root: y_language
# env: GRIMOIRE_LANG=es
# The language picked stays picked when Grimoire starts again (Automatic would be Spanish here),
# and Automatic gives the choice back to the system.
wait 200
expect settings.language is en
expect language is en
tap library settings
wait 200
tap library settings.cat.0
wait 100
tap library language
wait 200
expect library.sheet is language
tap library language.auto
wait 300
expect library.sheet is settings
expect settings.language is auto
expect language is es
shot 01-settings-automatic-again
quit
