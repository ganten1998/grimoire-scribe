# settings: status_mode=1
# Library > Settings, the row for the pen's button: a tap moves it through its four choices
# (side_button 0 to 3: nothing, erases, highlights, draws white) and round again, upright and
# sideways, each said as a whole sentence (01-draws-white).
wait 300
tap 1535 95
wait 200
expect library.sheet is settings
expect settings.side_button is 2
tap 930 1190
wait 50
expect settings.side_button is 3
shot 01-draws-white
tap 930 1190
wait 50
expect settings.side_button is 0
tap 930 1190
wait 50
expect settings.side_button is 1
tap 930 1190
wait 50
expect settings.side_button is 2
rotate 1
wait 300
expect library.sheet is settings
tap 1240 1003
wait 50
expect settings.side_button is 3
shot 02-draws-white-landscape
tap 1240 1003
wait 50
tap 1240 1003
wait 50
tap 1240 1003
wait 50
expect settings.side_button is 2
rotate 0
wait 300
expect library.sheet is settings
quit
