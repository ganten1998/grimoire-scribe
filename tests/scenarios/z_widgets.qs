# settings: status_mode=1
# rust-only
# copy: tests/fixtures/widgetcalc .
# The installed calculator package: the toolbar's Widgets button opens the catalog popup; its row
# opens a frame; the frame draws its own keypad through the generic WebAssembly host. Controls are
# the package's hit regions, reached as widget.0.hit.N, and the status line it reports is read with
# editor.widget.0.status. The title's minimize button shrinks it and a tap on the title restores it;
# the title bar drags; the close button removes it.
wait 300
tap library add
tap library add.new-notebook
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
expect editor.widget.0.state is normal
expect editor.widget.0.status is simple|0|simple,scientific,programming
shot 02-calculator-open
# 1 + 2 = 3
tap panel widget.0.hit.12
tap panel widget.0.hit.15
tap panel widget.0.hit.13
tap panel widget.0.hit.19
wait 300
expect editor.widget.0.status is simple|3|simple,scientific,programming
shot 03-calculator-three
# 3 x 4 = 12
tap panel widget.0.hit.7
tap panel widget.0.hit.8
tap panel widget.0.hit.19
wait 300
expect editor.widget.0.status is simple|12|simple,scientific,programming
shot 04-calculator-twelve
# 1 / 0 = Error, and AC starts again.
tap panel widget.0.hit.0
tap panel widget.0.hit.12
tap panel widget.0.hit.3
tap panel widget.0.hit.16
tap panel widget.0.hit.19
wait 300
expect editor.widget.0.status is simple|Error|simple,scientific,programming
shot 05-calculator-error
tap panel widget.0.hit.0
wait 200
expect editor.widget.0.status is simple|0|simple,scientific,programming
# The mode dropdown; the scientific keypad evaluates an expression.
tap panel widget.0.hit.100
wait 200
shot 06-calculator-modes
tap panel widget.0.hit.102
wait 300
expect editor.widget.0.status is scientific|0|simple,scientific,programming
# sin(30) = 0.5
tap panel widget.0.hit.5
tap panel widget.0.hit.27
tap panel widget.0.hit.28
tap panel widget.0.hit.24
wait 300
expect editor.widget.0.status is scientific|0.5|simple,scientific,programming
shot 07-calculator-scientific
# The programmer keypad converts bases and opens the unit converter.
tap panel widget.0.hit.100
wait 200
tap panel widget.0.hit.103
wait 300
expect editor.widget.0.status is programming|0|simple,scientific,programming
tap panel widget.0.hit.24
tap panel widget.0.hit.27
wait 200
expect editor.widget.0.status is programming|10|simple,scientific,programming
tap panel widget.0.hit.1
wait 200
expect editor.widget.0.status is programming|0xA|simple,scientific,programming
tap panel widget.0.hit.2
wait 200
expect editor.widget.0.status is programming|10|simple,scientific,programming
shot 08-calculator-programming
tap panel widget.0.hit.31
wait 300
shot 09-calculator-units
tap panel widget.0.hit.202
wait 300
shot 10-calculator-units-category
tap panel widget.0.hit.210
wait 300
# Back to the simple keypad.
tap panel widget.0.hit.100
wait 200
tap panel widget.0.hit.101
wait 300
expect editor.widget.0.status is simple|0|simple,scientific,programming
# Minimize, restore, and drag by the title.
tap panel widget.0.min
wait 300
expect editor.widget.0.state is minimized
shot 11-calculator-minimized
tap panel widget.0.title
wait 300
expect editor.widget.0.state is normal
drag touch 1546 60 1100 500 8
wait 300
expect editor.widget.0.state is normal
expect editor.owner is none
shot 12-calculator-dragged
tap panel widget.0.close
wait 300
expect editor.widgets is 0
shot 13-calculator-closed
wait 1800
quit
