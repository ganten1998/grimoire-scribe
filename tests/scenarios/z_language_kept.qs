# root: y_language
# env: GRIMOIRE_LANG=es
# The language picked stays picked when Grimoire starts again (Automatic would be Spanish here),
# and Automatic gives the choice back to the system.
wait 200
expect settings.language is en
expect language is en
tap 1535 95
wait 200
tap 930 1580
wait 200
expect library.sheet is language
tap 930 785
wait 300
expect library.sheet is settings
expect settings.language is auto
expect language is es
shot 01-settings-automatic-again
quit
