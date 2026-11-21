# settings: status_mode=1
# Library > Settings > Pen, the row for the pen's button: it opens its four choices, and a tap on one picks it
# and goes back to Settings (since the new Settings menu, #115; side_button 0 to 3: nothing, erases,
# highlights, draws white), through all four and round again, upright and sideways, the row saying each
# as a whole sentence (01-draws-white).
wait 300
tap library settings
wait 200
expect library.sheet is settings
tap library settings.cat.3
wait 100
expect settings.side_button is 2
tap library pen-button
wait 50
tap library pen-button.3
wait 50
expect settings.side_button is 3
shot 01-draws-white
tap library pen-button
wait 50
tap library pen-button.0
wait 50
expect settings.side_button is 0
tap library pen-button
wait 50
tap library pen-button.1
wait 50
expect settings.side_button is 1
tap library pen-button
wait 50
tap library pen-button.2
wait 50
expect settings.side_button is 2
rotate 1
wait 300
expect library.sheet is settings
tap library pen-button
wait 50
tap library pen-button.3
wait 50
expect settings.side_button is 3
shot 02-draws-white-landscape
tap library pen-button
wait 50
tap library pen-button.0
wait 50
tap library pen-button
wait 50
tap library pen-button.1
wait 50
tap library pen-button
wait 50
tap library pen-button.2
wait 50
expect settings.side_button is 2
rotate 0
wait 300
expect library.sheet is settings
quit
