# rust-only
# The setting is kept across a restart: s_fit_1 left it off, and the notebook it left comes up sideways
# with the page's width across. Settings > Display, in the Library, turns it on: the editor lays the
# page out whole when it is next entered.
# root: s_fit_1
wait 300
expect settings.landscape_fit is 0
expect screen is library
tap 1535 95
wait 200
tap 930 780
wait 200
expect library.sheet is display
tap 930 1032
wait 300
expect settings.landscape_fit is 1
expect toast is Applies when the Kindle is sideways.
tap 930 1032
wait 300
expect settings.landscape_fit is 0
tap 1460 1626
wait 300
expect library.sheet is settings
tap 1557 1930
wait 300
expect library.sheet is none
tap 200 400
wait 400
expect screen is editor
rotate 1
wait 1200
expect editor.zoom is 1.00
expect editor.scale is 1.33
pentap panel library
wait 500
expect screen is library
rotate 0
wait 800
tap 1535 95
wait 200
tap 930 780
wait 200
tap 930 1032
wait 300
expect settings.landscape_fit is 1
tap 1460 1626
wait 300
tap 1557 1930
wait 300
tap 200 400
wait 400
expect screen is editor
expect editor.scale is 1.00
rotate 1
wait 1200
expect editor.zoom is 1.00
expect editor.scale is 0.75
