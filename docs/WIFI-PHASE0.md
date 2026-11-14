# Wi-Fi Phase 0 probe

Run this on the Kindle over SSH, with the owner's OK, before relying on scan, add or forget on a firmware:

```sh
sh tools/kindle-wifi-probe.sh > wifi-probe.txt
```

It is read-only and masks every value. Paste the whole file below; it contains no SSID and no password.
Then fill the table.

```
probe=
model=
firmware=
wireless_enable=
wifid_enable=
profile_count=
scan_state=
wpa_passphrase=
wpa_cli=
--- properties (values removed) ---
...
--- currentEssid shape ---
...
--- profileData shape (values masked) ---
...
--- scanList shape (values masked) ---
...
```

| Check | Result | Decision |
| --- | --- | --- |
| `createProfile` is a writable hasharray and `deleteProfile` a writable string or integer `netid` on this firmware | | Add/forget can ship; a missing property keeps both out of the UI |
| `wpa_passphrase` exists and emits a 64-hex `psk` line from a passphrase on stdin | | Only the helper's direct passphrase path needs it: Grimoire derives the PSK itself, so WPA/WPA2 add ships either way |
| `profileData` uses the quoted `essid`/`psk` shape the helper reads | | Saved list works; send the masked shape if not |
| `scanList` uses quoted `essid`, `signal`, `signal_max`, `key_mgmt` and `known` values | | Nearby works; unknown security is shown but not added |
| `scan` finishes (`scanState` returns to `idle`) with the radio on | | Scan can ship; a stalled scan already fails closed after 20 s |
| With the radio off, `networks` gives `list=0` and `scan` gives `scan=off` | | The sheet's "Turn on Wi-Fi" fallbacks are the right ones |
| `currentEssid` names the connected network while `cmState` is `CONNECTED` | | The switch watcher works |
| `ensureConnection` switches to another saved network | | The current switch path works; note whether it drops SSH |
| `cmDisconnect` is writable and `cmState` leaves `CONNECTED` while `enable` stays 1 | | Tapping the connected row can use wifid's own disconnect; note whether its `cmProfile` value is needed |
| `cmConnMode manual` keeps the manager from rejoining, and `auto` brings it back | | Disconnect stays down with Wi-Fi on; a value the firmware refuses leaves the helper's other routes in charge |
| `wpa_cli` is present | | If wifid leaves the link up, its DISCONNECT is the fallback and stays down instead of rejoining at once; EAP and WPA3 add also use its control socket, so absent it fails with exit 26 (exit 28 says the supplicant does not list SAE) |

The Scribe's wifid (5.20.1) lists `w Int deleteProfile`: the helper reads that profile's `netid` from
`profileData` (or `netidValue`) and writes the integer, after the name a firmware with `w Str
deleteProfile` takes.

Security modes this release does **not** create: WEP, OWE, WPA3-Enterprise 192-bit, EAP-TTLS and EAP-TLS.
WPA2/WPA3-Enterprise PEAP/MSCHAPv2 and WPA3-Personal SAE are created through `wpa_cli`, not `createProfile`,
so they need a readable `wpa_cli` and control socket at add time (the helper's exits 26, 27 and 28 name those
steps); the enterprise profile has no CA-certificate choice yet. WPA3-Enterprise is the same EAP exchange with
optional PMF and the SHA-256 AKM; 192-bit needs EAP-TLS and is left out. Leave the rest to Kindle settings or
KOReader until a separate probe shows the exact `createProfile` arguments and a device connects.

No result means the feature stays behind the existing gates; the saved-network switch and radio controls
remain usable without these probes.
