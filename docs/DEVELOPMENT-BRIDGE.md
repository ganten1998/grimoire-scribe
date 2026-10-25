# Kindle development bridge and HID follow-up

This development branch builds on RC3, including the firmware 5.20 launch-guard repair. The release candidate remains separate; these additions
are not installed by a normal Grimoire ZIP/KPM installation.

## Independent SSH

`tools/install-kindle-ssh.py` explicitly installs a separate listener using the owner's existing
KOReader Dropbear, authorized public keys and Ed25519 host identity. It requires an already
trusted bootstrap connection and never reads the Mac's private key contents:

```sh
python3 tools/install-kindle-ssh.py KINDLE_IP --port 2222 \
  --identity /path/to/private-key --known-hosts /path/to/known_hosts --start
```

Stop an existing bridge before refreshing its binary. The installer refuses unrelated files
at its destination and stages the Library wrapper beside Documents before renaming it.
`KindleLab SSH` in the Kindle Library toggles the server. Over an authenticated shell:

```sh
sh /mnt/us/kindle-lab/ssh/bridge.sh status
sh /mnt/us/kindle-lab/ssh/bridge.sh start
sh /mnt/us/kindle-lab/ssh/bridge.sh stop
```

The listener uses **port 2223**, key authentication only, with local/remote forwarding disabled.
It copies its binary and credentials to `/var/local/kindle-lab-ssh`, with directory mode 700 and
credential mode 600. It uses a distinct executable name/PID, validates process ownership before
stopping, serializes lifecycle commands and removes only its tagged firewall rules. A firewall
failure stops its listener and rolls back its rules. It never kills every Dropbear process.
Existing authenticated sessions may finish after stopping the listener; no new ones can connect.

No boot service is installed. Start it explicitly after reboot. It persists across app exits,
but the Kindle must still have awake networking; it does not prevent normal sleep or keep Wi-Fi
awake. This is a developer shell with the owner's root privileges, not an unauthenticated remote
control endpoint. Verify the new port's public host identity against the already trusted one
before accepting it on the computer. The binary is copied on-device, not bundled in this repo.

Removal: stop it, then remove only its Library item, `/mnt/us/kindle-lab/ssh` and
`/var/local/kindle-lab-ssh`. Do not remove other SSH keys or KOReader's SSH directory.

### Actual Scribe evidence, October 25, 2026

Scribe 2024 / firmware 5.19.6 / KOReader v2026.07.2. The copied Dropbear v2026.92 authenticates
with the existing lab key, rejects an unrelated key, and has the same verified public host
identity. Fresh connections succeeded while Grimoire was open through Tools > Grimoire and
again after Back to KOReader. Eight samples observed the Grimoire launcher during the handover;
the independent listener PID stayed unchanged. A separate normal KOReader restart also passed.
The old KOReader listener used `-n` (passwordless); after testing the secure bridge, that listener
and its two original firewall rules were stopped/removed without killing client sessions. Its
saved passwordless preference was not edited under the running reader. Before using KOReader's
own SSH server again, choose its key-only option. Autostart was not enabled in its saved settings.

The original listener also daemonized under PID 1; it was not proven to stop on every handover.
The standalone bridge establishes explicit ownership, independent credentials and user controls,
rather than relying on the assumption that KOReader's server must die with the app.

Host lifecycle tests cover key-only flags, protected credentials, duplicate starts, missing keys,
foreign PID/directory preservation and firewall rollback. The live test preserves normal app data.

## External HID input

Grimoire now recognizes sparse keyboard/page-turn/media interfaces as well as full keyboards,
non-base gamepad buttons, digital D-pads and standard `ABS_HAT0X/Y` ranges of -1 through 1.
Pen/touch interfaces, mice, bare accelerometers and power-only devices are excluded. External
power/sleep keys never become the app's native power button.

Press state suppresses duplicate down/repeat events. Attachment, native-PIN return and
`SYN_DROPPED` resynchronize from the actual device state; a held control must release before it
acts again. Failed state reads close the optional device for rediscovery. Pen/touch and power
frames are queued before optional keys from the same poll so drawing and sleep gates take effect.
This path never grabs keyboards exclusively or logs key values. Native PIN remains unchanged.

Additional mappings (existing files are never rewritten):

```ini
pad.Up=previous-tool
pad.Down=next-tool
pad.Left=previous-page
pad.Right=next-page
pad.South=undo
pad.East=tool:eraser
key.VolumeDown=next-page
key.VolumeUp=previous-page
```

`pad.Up/Down/Left/Right` use Linux codes 544–547, also emitted by standard HAT0 directions.
Named buttons include South/East/North/West, L1/R1/L2/R2 and Select/Start. Media key names include
VolumeUp/Down, PlayPause, NextSong and PreviousSong. New defaults include D-pad mappings; custom
and existing empty files retain the owner's choices. Analog sticks/triggers are not converted
into actions. Nonstandard hat ranges are ignored.

### Optional KOReader profile

`kindle/lab/kindlelabhid.koplugin` is a separate, disabled-by-default developer plugin. Copy it
manually to KOReader's plugins directory and restart. Enable **Tools > KindleLab HID buttons**.
It adds only absent mappings: south=Press, east=Back, select=Menu, shoulders=page back/forward,
and digital D-pad=directional navigation. Disabling it removes only entries it still owns;
existing KOReader or other-plugin mappings are preserved. It opens no devices, starts no Bluetooth
service and installs no user patches. Standard hats are decoded directly by Grimoire; this small
KOReader profile covers digital button events only, not a claim of general axis support.

The Kindle must expose usable `/dev/input/event*` devices first. Bluetooth transport/pairing
remains separate. The earlier Arcade Stick X-mode attempt discovered the controller but did not
establish usable input. A Bluetooth helper's "paired" message alone is not proof of an encrypted
connection, HID descriptor setup, an input node, or working app controls. Controller and firmware
coverage need physical tests; no universal 8BitDo claim is made.

### Actual Arcade Stick test, October 25, 2026

The owner's 8BitDo Arcade Stick 80FE connected using **S + BT**, identifying as **Pro Controller**
(`E4:17:D8:85:67:1A`). The isolated upstream kindle-hid-passthrough v3.18.0 runtime exposed
`/dev/input/event6` with a 170-byte HID descriptor. Raw face-button events were received.
With its LS/DP/RS selector on **DP**, a read-only ARM probe compiled against this branch's
`src/hidkeys.c` decoded all four directions (544, 545, 546, 547) and a face button (306).
This tests the production decoder on real input without replacing the installed RC2 or touching
a notebook. It does not establish end-to-end app control or KOReader gamepad support. The
optional digital mapping profile alone does not add KOReader gamepad discovery or axis routing.
After restarting the isolated transport, tapping Home reconnected the saved pairing and recreated
the input device without pairing again. The transport was isolated under `/mnt/us/kindle-lab`; its broad installer, boot service, udev
rules, mapper and KOReader plugin were not installed.

## Validation

`make bridge-check` runs the shell lifecycle and optional KOReader profile checks. Portable
`test_hidkeys` covers capabilities, modifiers, duplicates, D-pads/hats and state loss/PIN resync.
The shortcut scenario covers mapped page directions and their native-PIN gate. ARM and headless
builds verify integration; simulation does not establish Bluetooth pairing or real event routing.
Full release checks remain `make -j4 check`.

## USB and firmware research boundaries

The Scribe's live configuration uses `mtpgadget/ffs.mtp`. Its shipped `tizen-mtp` binary
(SHA256 `2fbe2aa27234142f426e0adb2527b0a16ee4b27fbd06f8e23eb96f136f007198`) was inspected
with REA/Ghidra: procedure 0x2b1d4 opens the MTP FunctionFS data/status endpoints. Evidence IDs
`ev_45ca72718fb3687077158052a24b76d8d9d7b66d507d845882c8fd83d39a758c` and
`ev_db5ba29455be84aa9ec6b7bd90264a337181727f0e997b2b9a8c03bbcfacbd48`; the private lab's
`evidence/usb-mtp-2026-10-25.json` preserves the full records and limitations.

Mass-storage and Ethernet gadget modules are present, but their existence does not establish
that switching is safe. The shipped MTP script unloads them when claiming USB. An eventual
Finder-compatible transfer mode could use USB networking and a file-sharing protocol. A true
mass-storage mode would need exclusive storage ownership and a clean handoff from the Kindle;
never expose a writable backing filesystem while it remains mounted by the device. No USB
configuration, firmware, update-blocking or reboot service was changed by this investigation.

The official Véra selector lists Scribe 2 versions 5.20.1 and 5.20.1.0.2 as of this investigation.
That establishes an available jailbreak route, not guaranteed survival of an installed stack or
Grimoire/KOReader compatibility after updating. The owner's device remains on 5.19.6; OTA binaries
are renamed to `.bck` and a persistence bridge is present. Exact target/hotfix verification and
backups are required before a separately requested firmware update.

Sources: [KOReader SSH](https://github.com/koreader/koreader/wiki/SSH),
[Dropbear patches](https://github.com/koreader/koreader-base/tree/master/thirdparty/dropbear),
[Véra selector](https://kindlemodding.org/vera/),
[Hotfix](https://github.com/KindleModding/Hotfix),
[modern jailbreak next steps](https://kindlemodding.org/jailbreaking/whats-next/).
