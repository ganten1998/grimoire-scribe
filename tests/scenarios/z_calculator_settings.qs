# settings: status_mode=1
# rust-only
# copy: tests/fixtures/widgetcalc .
# The calculator package's settings choose its default mode and which calculators its dropdown
# offers. Changing them applies to an open frame; a mode that is turned off falls back.
wait 300
tap 965 95
wait 100
tap 1564 2254
wait 300
expect screen is editor
# Settings → Packages → Widgets → the calculator's detail page and settings.
tap panel settings
wait 400
tap panel settings.cat.5
wait 300
tap panel settings.packages
wait 400
expect editor.packages is 1 com.grimoire.calculator 1.2.0
tap panel packages.row.0
wait 300
tap panel packages.settings
wait 400
expect editor.package.0.settings.default_calculator is Simple
expect editor.package.0.settings.enable_simple is true
expect editor.package.0.settings.enable_scientific is true
expect editor.package.0.settings.enable_programming is true
shot 00-calculator-settings
# The default cycles Simple → Scientific; then Simple is turned off.
tap panel packages.setting.0
wait 200
expect editor.package.0.settings.default_calculator is Scientific
tap panel packages.setting.1
wait 200
expect editor.package.0.settings.enable_simple is false
shot 01-calculator-settings-changed
tap panel packages.close
wait 400
expect screen is editor
# The popup opens the frame on the new default, and Simple is missing from the dropdown's modes.
tap panel widgets
wait 300
tap panel widgets.com.grimoire.calculator
wait 400
expect editor.widget.0.status is scientific|0|scientific,programming
shot 02-calculator-settings-frame
# Turning off the mode a frame is showing falls back to the first enabled calculator.
tap panel settings
wait 400
tap panel settings.cat.5
wait 300
tap panel settings.packages
wait 400
tap panel packages.row.0
wait 300
tap panel packages.settings
wait 400
tap panel packages.setting.3
wait 400
expect editor.package.0.settings.enable_programming is false
tap panel packages.close
wait 400
# The open frame reads the saved settings on its next draw.
expect editor.widget.0.status is scientific|0|scientific
shot 03-calculator-settings-fallback
# Reset restores all three calculators; the open frame keeps its valid mode.
tap panel settings
wait 400
tap panel settings.cat.5
wait 300
tap panel settings.packages
wait 400
tap panel packages.row.0
wait 300
tap panel packages.settings
wait 400
tap panel packages.settings-reset
wait 400
expect editor.package.0.settings.default_calculator is Simple
expect editor.package.0.settings.enable_simple is true
expect editor.package.0.settings.enable_scientific is true
expect editor.package.0.settings.enable_programming is true
tap panel packages.close
wait 400
expect editor.widget.0.status is scientific|0|simple,scientific,programming
shot 04-calculator-settings-reset
# A new frame opens on the default again, and the old frame can be closed.
tap panel widget.0.close
wait 300
expect editor.widgets is 0
tap panel widgets
wait 300
tap panel widgets.com.grimoire.calculator
wait 400
expect editor.widget.0.status is simple|0|simple,scientific,programming
tap panel widget.0.close
wait 200
wait 1800
quit
