# settings: status_mode=1
# rust-only
# copy: tests/fixtures/widgetcalc .
# The installed calculator widget: the toolbar's Widgets button opens the catalog popup; its row
# opens a frame; the keypad calculates; the title's minimize button shrinks it and a tap on the
# title restores it; the title bar drags; the close button removes it.
wait 300
tap 965 95
wait 100
tap 1564 2254
wait 300
expect screen is editor
expect editor.widgets is 0
expect editor.widgets.open is 0
tap panel widgets
wait 300
expect editor.widgets.open is 1
shot 01-widgets-popup
tap panel widgets.com.grimoire.calculator
wait 400
expect editor.widgets.open is 0
expect editor.widgets is 1 com.grimoire.calculator
expect editor.calculator.state is normal
expect editor.calculator.display is 0
shot 02-calculator-open
# 1 + 2 = 3
tap panel calculator.key.12
tap panel calculator.key.15
tap panel calculator.key.13
tap panel calculator.key.19
wait 300
expect editor.calculator.display is 3
shot 03-calculator-three
# 3 x 4 = 12
tap panel calculator.key.7
tap panel calculator.key.8
tap panel calculator.key.19
wait 300
expect editor.calculator.display is 12
shot 04-calculator-twelve
# 1 / 0 = Error, and AC starts again.
tap panel calculator.key.0
tap panel calculator.key.12
tap panel calculator.key.3
tap panel calculator.key.16
tap panel calculator.key.19
wait 300
expect editor.calculator.display is Error
shot 05-calculator-error
tap panel calculator.key.0
wait 200
expect editor.calculator.display is 0
# Minimize, restore, and drag by the title.
tap panel calculator.min
wait 300
expect editor.calculator.state is minimized
shot 06-calculator-minimized
tap panel calculator.title
wait 300
expect editor.calculator.state is normal
drag touch 1546 60 1100 500 8
wait 300
expect editor.calculator.state is normal
expect editor.owner is none
shot 07-calculator-dragged
tap panel calculator.close
wait 300
expect editor.widgets is 0
shot 08-calculator-closed
wait 1800
quit
