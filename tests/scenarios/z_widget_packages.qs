# settings: status_mode=1
# rust-only
# copy: tests/fixtures/widgetpkg .
# The widget package manager: an installed package appears in Packages → Widgets and in the widget
# popup, a frame opens from it, Remove asks first, and Install from repository... opens the repository
# keyboard.
wait 300
# From the Library: Settings → Packages → Widgets, then the X closes Settings back to the Library.
tap library settings
wait 400
tap library settings.cat.5
wait 300
tap library settings.packages
wait 400
expect editor.packages.open is 1
tap library packages.close
wait 300
expect screen is library
# Open a notebook for the editor's own Settings and the widget frames.
tap library add
tap library add.new-notebook
wait 100
tap 1564 2254
wait 300
expect screen is editor
# Settings → Packages → Widgets (not the popup): the category and its Widgets row.
tap panel settings
wait 400
tap panel settings.cat.5
wait 300
shot 00-settings-packages
tap panel settings.packages
wait 400
expect editor.packages.open is 1
expect editor.packages is 1 com.example.timer 1.0.0
expect editor.package.com.example.timer is 1.0.0
shot 01-packages
# A tap that finds no control (the empty part of the page) draws and refreshes nothing:
# the page is not redrawn for a contact that changed nothing.
expect display.presents not -1
tap 900 1500
wait 120
expect display.presents is
# The installed package is in the popup too, and opens as a frame.
tap panel packages.close
wait 300
tap panel widgets
wait 300
tap panel widgets.com.example.timer
wait 400
expect editor.widgets is 1 com.example.timer
shot 02-package-widget
tap panel widget.com.example.timer.close
wait 300
# Its detail page, then Remove with its question. This time the popup's Manage widgets... opens
# the same Settings page.
tap panel widgets
wait 200
tap panel widgets.manage
wait 400
tap panel packages.row.com.example.timer
wait 300
shot 03-package-detail
# The package's own settings: manifest-defined rows, saved as they are tapped, and reset together.
tap panel packages.settings
wait 300
expect editor.package.com.example.timer.settings.show_seconds is true
expect editor.package.com.example.timer.settings.layout is compact
shot 03a-package-settings
tap panel packages.setting.show_seconds
wait 200
expect editor.package.com.example.timer.settings.show_seconds is false
tap panel packages.setting.layout
wait 200
expect editor.package.com.example.timer.settings.layout is large
shot 03b-package-settings-changed
tap panel packages.settings-reset
wait 200
expect editor.package.com.example.timer.settings.show_seconds is true
expect editor.package.com.example.timer.settings.layout is compact
tap panel packages.back
wait 300
tap panel packages.remove
wait 300
shot 04-remove-question
tap panel packages.remove-yes
wait 400
expect editor.packages is 0
expect editor.packages.open is 1
shot 05-package-removed
tap panel packages.install
wait 400
shot 06-install-keyboard
# The repository is typed on the one global keyboard: a key refreshes its own line and key on the
# fast waveform, never the page under it (the page is not drawn again until Done or Cancel).
expect keyboard.draws not -1
expect display.presents not -1
tap 160 1787
wait 100
expect keyboard.draws is 2
expect display.presents has fast
expect display.presents not has full
expect display.presents not has ui
shot 07-install-keyboard-typed
quit
