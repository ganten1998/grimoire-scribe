# rust-only
# settings: shortcuts_enabled=1
# env: GRIMOIRE_WIFI_SCRIPT=tests/fixtures/wifi/helper.sh
# env: GRIMOIRE_CHECK_FIT=1
# Settings > Wi-Fi (2.1.0-beta.2): the radio, the network connected to and the Kindle's saved networks, from
# the scenario helper (tests/fixtures/wifi/helper.sh: nine saved networks, listed only while the radio is on,
# Far away refusing a switch). make l10n-check runs it in every language and style, for the fit check.
wait 200
tap 1535 95
wait 100
expect library.sheet is settings
shot 01-settings
# Settings' bottom row: KOReader, Pair, Wi-Fi, Gestures, Updates, Done, at the tap points docs/SIM.md gives.
expect library.ui_at.580.1930 is pair-menu
expect library.ui_at.770.1930 is wifi
expect library.ui_at.1000.1930 is gestures
expect library.ui_at.1200.1930 is updates
expect library.ui_at.1550.1930 is done
rotate 1
wait 200
expect library.ui_at.863.1695 is pair-menu
expect library.ui_at.1070.1695 is wifi
expect library.ui_at.1320.1695 is gestures
expect library.ui_at.1560.1695 is updates
expect library.ui_at.1820.1695 is done
shot 01-settings-sideways
rotate 0
wait 200
tap library wifi
wait 300
expect library.sheet is wifi
expect wifi.state is off
expect wifi.saved is unread
expect wifi.current is
shot 02-off
# Turn on: connected to Home; the saved networks are read once the radio is on.
tap library wifi.radio
wait 300
expect wifi.state is connected
wait 300
expect wifi.saved is 9
expect wifi.current is Home
shot 03-connected
# A tap on the second row (Studio 5 GHz) switches to it: Connecting to..., then connected once a read says so.
tap library wifi.net.2
wait 300
expect wifi.join is pending
shot 04-connecting
wait 2500
expect wifi.join is connected
expect wifi.current is Studio 5 GHz
shot 05-switched
rotate 1
wait 200
shot 05-switched-sideways
rotate 0
wait 200
# The second page, with Far away, which refuses: Could not connect to Far away.
tap library wifi.more
wait 100
shot 06-page-2
tap library wifi.net.2
wait 600
expect wifi.join is failed
expect wifi.current is Studio 5 GHz
shot 07-refused
rotate 1
wait 200
expect library.sheet is wifi
shot 08-sideways
rotate 0
wait 200
# Esc goes back to Settings; the sheet opened again forgets the refused switch.
key 1 0
wait 100
expect library.sheet is settings
tap library wifi
wait 300
expect wifi.join is
expect wifi.current is Studio 5 GHz
tap library wifi.radio
wait 300
expect wifi.state is off
expect wifi.current is
shot 09-turned-off
tap library done
wait 100
expect library.sheet is none
quit
