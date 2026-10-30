# rust-only
# env: GRIMOIRE_WIFI_SCRIPT=tests/fixtures/wifi/helper.sh
# env: GRIMOIRE_SIM_ADDRS=wlan0=192.168.0.20
# env: GRIMOIRE_CHECK_FIT=1
wait 200
tap 1345 95
wait 200
expect screen is share
tap share wifi
wait 300
expect wifi.open is 1
expect wifi.state is off
shot 01-wifi-off
tap share wifi.on
wait 300
expect wifi.state is connected
expect wifi.failed is 0
shot 02-wifi-connected
rotate 1
expect wifi.open is 1
shot 03-wifi-landscape
tap share wifi.off
wait 300
expect wifi.state is off
tap share wifi.done
expect screen is share
expect wifi.open is 0
tap share done
expect screen is library
quit
