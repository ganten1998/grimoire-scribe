# rust-only
# settings: shortcuts_enabled=1
# env: GRIMOIRE_WIFI_SCRIPT=tests/fixtures/wifi/helper.sh
# env: GRIMOIRE_CHECK_FIT=1
# Settings > Wi-Fi (2.1.0-beta.2): nearby networks first (the sheet scans as soon as the radio is on),
# with Rescan, Other... and Saved networks above them. The scenario helper (tests/fixtures/wifi/helper.sh)
# has fifteen saved networks, listed only while the radio is on, and Far away refusing a switch.
# make l10n-check runs it in every language and style, for the fit check.
wait 200
tap library settings
wait 100
expect library.sheet is settings
shot 01-settings
# Settings' General category holds Wi-Fi (the branch's settings menu, not the old bottom row).
expect library.ui_at.1000.630 is wifi
rotate 1
wait 200
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
# Turn on: it connects to Home, reads the saved networks, and starts the nearby scan by itself.
tap library wifi.radio
wait 800
expect wifi.state is connected
expect wifi.saved is 15
expect wifi.current is Home
expect wifi.scanning is 0
expect wifi.scan.failed is 0
expect wifi.near is 5
shot 03-nearby-auto
# Rescan is the first control above the list; it runs another scan and leaves the five synthetic networks.
tap library wifi.scan
wait 800
expect wifi.scanning is 0
expect wifi.near is 5
# The unsaved Open network is added with no password.
tap library wifi.near.5
wait 800
expect wifi.saved is 16
expect wifi.current is Open
shot 03b-added
# Saved networks: the newly added Open network is the last row; its trash button forgets it.
tap library wifi.tab.saved
wait 300
shot 03c-saved
drag touch 930 1700 930 850 6
wait 100
shot 03c-scrolled
tap library wifi.delete.16
wait 800
expect wifi.saved is 15
expect wifi.current is Home
# Back to the top of Saved for the switch tests.
tap library wifi.tab.saved
wait 100
tap library wifi.tab.saved
wait 100
shot 03d-forgotten
# A tap on the second row (Studio 5 GHz) switches to it: Connecting to..., then connected once a read says so.
tap library wifi.net.2
wait 300
expect wifi.join is pending
shot 04-connecting
wait 2500
expect wifi.join is connected
expect wifi.current is Studio 5 GHz
shot 05-switched
# Tapping the connected saved network again disconnects it and leaves Wi-Fi on.
tap library wifi.net.2
wait 800
expect wifi.state is on
expect wifi.current is
shot 05a-disconnected
# Saved networks still knows it: the same row reconnects to Studio.
tap library wifi.net.2
wait 2500
expect wifi.current is Studio 5 GHz
shot 05b-reconnected
rotate 1
wait 200
shot 05-switched-sideways
rotate 0
wait 200
# The saved list scrolls: drag it up to Far away, which refuses: Could not connect to Far away.
drag touch 930 1700 930 850 6
wait 100
shot 06-scrolled
tap library wifi.net.15
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
# Esc goes back to Settings; the sheet opened again forgets the refused switch and opens on Nearby.
key 1 0
wait 100
expect library.sheet is settings
tap library wifi
wait 300
expect wifi.join is
expect wifi.current is Studio 5 GHz
# Other...: its keyboard stays over the Wi-Fi page. Type a hidden name, then choose its security.
tap library wifi.other
wait 100
expect library.sheet is keyboard
expect library.text is
# H i d d e n on the letters page.
tap 1106 2045
tap 1382 1885
tap 567 2045
tap 567 2045
tap 477 1885
tap 1242 2205
expect library.text is hidden
shot 09-other-name
tap 1643 2365
wait 300expect library.sheet is wifi
shot 10-other-security
# WPA/WPA2 takes a masked password over the same page; 12345678, then Done.
tap library wifi.other.wpa
wait 100
expect library.sheet is keyboard
tap 115 1725
tap 296 1725
tap 477 1725
tap 658 1725
tap 839 1725
tap 1020 1725
tap 1201 1725
tap 1382 1725
shot 11-other-password
tap 1643 2365
wait 800
expect wifi.saved is 16
expect wifi.current is hidden
shot 12-other-added
tap library wifi.radio
wait 300
expect wifi.state is off
expect wifi.current is
shot 13-turned-off
# The sheet's top-right X, which closes Settings with it.
tap 1730 90
wait 100
expect library.sheet is none
quit
