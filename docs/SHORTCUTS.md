# Keyboard and controller shortcuts

A controller (or a keyboard) is express keys for the hand that does not hold the pen: pages,
tools, undo and redo, the eraser and white ink, in the notebook editor. It does not drive menus.

Enable **Library > Settings > Controller > Enable shortcuts**. This writes starter mappings to
`/mnt/us/grimoire/shortcuts.ini` only if that file does not exist. Edit it over USB/SSH and choose
**Reload shortcuts.ini**; that row says how many mappings loaded and how many lines were refused.
The sheet shows the controller or keyboard Grimoire reads now (its own name), or that another app
holds it (see below), and the last key or controller button code. Disable shortcuts to close the
optional input devices again.

## Controller buttons

**Settings > Controller > Buttons...** shows what each button does and learns new ones: tap a row,
then press the button. One button per action: a button that did something else does this now, and
the action's other buttons stop doing it. **Pages** and **Tools** learn two presses, the previous
one's and then the next one's. Ten seconds without a press leave the row as it was; a second tap on
it stops it too. The buttons learned are written to `/mnt/us/grimoire/controller.ini` (atomically,
every time). While that file is there, the controller's buttons come from it alone and the `pad.`
lines of `shortcuts.ini` are set aside; keyboard (`key.`) lines always come from `shortcuts.ini`.
**Reset buttons** deletes `controller.ini`.

## shortcuts.ini

The default keyboard uses 1–9 for the nine drawing tools, E for eraser, W for white ink,
Ctrl+Z / Ctrl+Y for undo/redo, left/right for existing pages, and PageUp/PageDown for tool cycling.
Gamepad defaults use the Linux gamepad convention: South undo, East eraser, North redo, West white,
L1/R1 previous/next tool, Select/Start previous/next page, and the D-pad (a hat or four buttons,
both read the same) Left/Right for pages and Up/Down for tools. These are starting points:
controller modes number their buttons differently (a Switch-mode controller's A and B are not where
the convention puts them), which the Buttons sheet is for. An 8BitDo in keyboard mode uses `key.`
bindings.

```ini
key.1=tool:pencil
key.Ctrl+z=undo
key.Shift+Left=previous-page
key.code106=next-page
key.Ctrl+i=index
pad.East=tool:eraser
pad.311=next-tool
pad.Up=previous-tool
```

Actions: `tool:pencil`, `tool:mech`, `tool:ballpoint`, `tool:fountain`, `tool:fineliner`,
`tool:marker`, `tool:charcoal`, `tool:calligraphy`, `tool:highlighter`, `tool:eraser`, `undo`,
`redo`, `white`, `previous-page`, `next-page`, `previous-tool`, `next-tool`, `index`.
The optional custom Index mapping opens the section list (and closes it when it is already open).
Existing shortcut files are never rewritten.

Key names include letters, digits, Left/Right/Up/Down, PageUp/PageDown, Space, Enter, Tab,
Escape, VolumeUp/VolumeDown, PlayPause, NextSong/PreviousSong, Home, End, Back and `codeNN` (Linux
evdev number 1–767). Modifiers Ctrl, Shift, Alt, Meta match exactly. Gamepad buttons are named
South, East, North, West, L1, R1, L2, R2, Select, Start, Up, Down, Left, Right, or numbered
256–767. Later duplicate bindings replace earlier ones. Up to 64 bindings and 64 KiB of
configuration are accepted. An existing empty file deliberately disables mappings.

Shortcuts act only in the idle notebook editor. They do nothing while drawing, pinching,
sleeping or displaying a sheet, except Index can toggle the Pages/Index view while its keyboard
is closed. Page navigation never creates a new page at the end. Presses
act once; releases and repeats do not trigger actions. While shortcuts are enabled, a key or
button press keeps the Kindle awake as the pen and touch do: "Sleep after" counts from the last
one, on any screen. Native PIN input is discarded and never logged. The app logs attachment
paths, not key presses or device names. The visible last-input display belongs to the current
unlocked Grimoire session.

Grimoire watches up to four keyboards and controllers that the Kindle shows as
`/dev/input/event*`, checks for new ones every five seconds, and never grabs them. A short toast
says when one connects or disconnects, at most once each way in 30 seconds, so a link that keeps
dropping does not flash the screen. When another app holds a controller exclusively (a button
mapper), Grimoire cannot get its presses: the toast and the Controller sheet say so. Analog sticks
are not read.

## Bluetooth

The Kindle's own Bluetooth pairs audio, not controllers. A third-party helper,
[kindle-hid-passthrough](https://github.com/zampierilucas/kindle-hid-passthrough) (GPL-3.0), is a
Bluetooth host of its own that makes paired controllers and keyboards into `/dev/input` devices.
Grimoire can run it for you, only while Grimoire runs:

1. Download its release archive and unpack **only the program and its files** into
   `/mnt/us/grimoire/bluetooth/` over USB, so that
   `/mnt/us/grimoire/bluetooth/kindle-hid-passthrough` is there. Do **not** run its installer or
   install its KPM package: those add a button mapper that starts at every boot and grabs
   controllers away from every app, Grimoire included, and write to the system partition.
   (An existing install in `/mnt/us/kindle_hid_passthrough` is used too, as it is.)
2. **Settings > Controller > Bluetooth controller** starts it now, and from then on a moment after
   Grimoire starts. The Kindle's own Bluetooth jobs stop while it runs.
3. **Pair a controller...** searches for 15 seconds (put the controller in pairing mode first) and
   lists what it finds; tap one to pair it. A controller paired before reconnects by itself (often
   with a press on its home button). **Forget** unpairs one.

Grimoire stops the helper when Bluetooth controller (or shortcuts) is turned off, starts it again
when shortcuts come back on, and the launcher stops it as
Grimoire leaves, crash or not; the Kindle's own Bluetooth jobs that ran before are started again.
A helper that someone else started (KOReader's, say) is used as it is and never stopped. Grimoire
never installs anything for it, never turns on its start at boot, and turns off its media remote
(a phone pairing with the Kindle as a speaker) in `grimoire/bluetooth`. Pairing accepts any device
in pairing mode without a code (what the helper does): pair where nobody else is pairing.

The helper keeps its paired devices and their keys in its own folder, visible over USB. Its
releases change often; this was written against 3.18. Tested setup: none yet on firmware
5.20.1.0.2 (an 8BitDo Arcade Stick in Switch mode paired and reported its buttons on 5.19.6).

## In the simulator

In the SDL simulator, enabling shortcuts repurposes the number/arrows/E keys from pressure/tilt/
eraser simulation. P and S remain reserved for simulated power/pen-side-button; Escape quits.
Scripts inject presses (`key CODE MODS PAD`) and devices (`hid attach pad|key|held NAME`,
`hid detach NAME`), and `tests/bt_check.sh` runs the pairing sheet against a stand-in for the
helper's API (`tests/fake_bthelper.py`). None of this proves Bluetooth, controller discovery or
real button numbering on a Kindle.
