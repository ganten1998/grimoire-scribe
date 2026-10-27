# Keyboard and controller shortcuts

A controller (or a keyboard) is express keys for the hand that does not hold the pen: pages,
tools, undo and redo, the eraser and white ink, in the notebook editor. It opens and closes the
editor's menus, but does not move around inside them (but for next and previous in the tool picker,
below): tap there.

Enable **Library > Settings > Controller > Enable shortcuts**. This writes starter mappings to
`/mnt/us/grimoire/shortcuts.ini` only if that file does not exist. Edit it over USB/SSH and choose
**Reload shortcuts.ini**; that row says how many mappings loaded and how many lines were refused.
The sheet shows the controller or keyboard Grimoire reads now (its own name), or that another app
holds it (see below), and the last key or controller button code. Disable shortcuts to close the
optional input devices again.

## Controller buttons

**Settings > Controller > Buttons...** shows what each button does and learns new ones. Its list
has four groups, as the editor's panel groups its buttons: **Editing** (undo, redo, the eraser,
Erase while held, the eraser's mode, white ink, clear page), **Tools** (each of the nine pens in
the tool picker's order, previous / next tool, and previous / next in the picker), **Pages**
(previous / next page, first / last page, new page, fit page, Pages, Index) and **Menus** (the tool
picker, Paper, Display, Pen feel, fold the panel, the Library, close menu). Open a group, tap a
row, then press the button; Done goes back up to the list. One button per action: a button that did something else does this now (a
toast says which action lost it), and the action's other buttons stop doing it. The previous /
next and first / last rows learn two presses, the first one's and then the second's. Ten seconds
without a press leave the row as it was; a second tap on it stops it too. The buttons learned are written to `/mnt/us/grimoire/controller.ini` (atomically,
every time). While that file is there, the controller's buttons come from it alone and the `pad.`
lines of `shortcuts.ini` are set aside; keyboard (`key.`) lines always come from `shortcuts.ini`.
Hold a row (0.6 s) to take its buttons away, that row's alone (both actions of a previous / next
or first / last row): it reads "none", a toast says so, and `controller.ini` is written without
them, so it stays that way. **Reset buttons** deletes `controller.ini`, every row at once.

An Xbox-layout controller's triggers (8BitDo's Xbox mode, which the Kindle reads through Linux's
generic driver as "Xbox Bluetooth Gamepad") are no buttons there but axes, ABS_Z and ABS_RZ beside
a right stick on RX and RY, or ABS_BRAKE and ABS_GAS. Grimoire reads them as buttons named LT and
RT (`pad.LT`, `pad.RT`): pressed past half their travel, let go at a quarter or under. A right
stick is never taken for them: Z and RZ count only beside RX and RY, when they are narrower than
the sticks or rest at an end while the sticks rest in the middle. The generic driver numbers a
controller's buttons from BTN_A, so L2 and R2 (312 and 313) are its ninth and tenth buttons, not
its triggers: on that 8BitDo most likely its stick presses, on a Switch-mode one - and +.

The sheet names the buttons as the controller connected labels them. A Switch-mode controller
(8BitDo's Switch mode, or a Pro Controller, which the Kindle reads through Linux's generic driver
as "Pro Controller") numbers its buttons in the Pro Controller's own order: B 304, A 305, Y 306,
X 307, L 308, R 309, ZL 310, ZR 311, - 312, + 313, the stick presses LS 314 and RS 315, Home 316
and Capture 317. The sheet shows those labels; the files keep the numbers, or the convention's
names for the ones that have one (`pad.South` is its B). With the starter mappings B is undo, A
the eraser, X redo, L white ink, ZL / ZR previous / next tool, the stick presses previous / next
page, and the D-pad (an arcade stick's lever) pages and tools; Y, R, - and + do nothing until a
row learns them.

## shortcuts.ini

The default keyboard uses 1–9 for the nine drawing tools in the tool picker's order (left to right,
then down: pencil, mechanical pencil, charcoal, ballpoint, fineliner, marker, fountain pen,
calligraphy pen, highlighter), E for eraser, W for white ink,
Ctrl+Z / Ctrl+Y for undo/redo, left/right for pages, and PageUp/PageDown for tool cycling.
Gamepad defaults use the Linux gamepad convention: South undo, East eraser, North redo, West white,
L1/R1 previous/next tool, Select/Start previous/next page, and the D-pad (a hat or four buttons,
both read the same) Left/Right for pages and Up/Down for tools. These are starting points:
controller modes number their buttons differently (a Switch-mode controller's A and B are not where
the convention puts them), which the Buttons sheet is for. An 8BitDo in keyboard mode uses `key.`
bindings. A `shortcuts.ini` that binds none of the D-pad's four directions (every one written
before 2.0.0-beta.5, which read no D-pad) gets the D-pad's starter lines as well, in memory: the
file is not changed, and Reload counts them among its mappings.

```ini
key.1=tool:pencil
key.Ctrl+z=undo
key.Shift+Left=previous-page
key.code106=next-page
key.Ctrl+i=index
pad.East=tool:eraser
pad.311=next-tool
pad.Up=previous-tool
pad.R2=picker-next
```

Actions, in the Buttons sheet's groups:

- **Editing**: `undo`, `redo`, `tool:eraser` (pressed again, it goes back to the pen used before
  it), `hold-eraser` (Erase while held: while its key or button is down, a stroke the pen starts
  erases as its eraser end does, by the eraser's mode; one under way when it is let go keeps
  erasing), `eraser-mode` (Whole strokes and Precise in turn, which a toast says), `white` (white
  ink on and off), `clear-page` (as the panel's broom: no question, one undo step).
- **Tools**: `tool:pencil`, `tool:mech`, `tool:charcoal`, `tool:ballpoint`, `tool:fineliner`,
  `tool:marker`, `tool:fountain`, `tool:calligraphy`, `tool:highlighter`, `previous-tool`,
  `next-tool`, `picker-previous`, `picker-next` (the Buttons sheet's Previous / next in the picker).
- **Pages**: `previous-page`, `next-page`, `first-page`, `last-page`, `new-page` (a page on the
  notebook's own paper right after the open one, which opens, as the Pages view's New page does),
  `fit-page` (the page's whole width again after a pinch), `pages` and `index` (the Pages view on
  its Pages or its Index tab).
- **Menus**: `tool-picker`, `paper`, `display`, `pen-feel`, `fold-panel` (folds the panel, or
  unfolds it), `library` (saves and closes the notebook), `close-menu`.

`previous-tool` and `next-tool` step through the pens on the editor's panel alone: the tool
button's pen and the pinned pens, top to bottom as the panel shows them, from the last round to the
first and back. Pressed again and again they keep to the panel, so the highlight never jumps into
the tool picker and back. With fewer than two pens on the panel (none pinned, or only the tool
button's own pen) they step through the tool picker's nine pens in its order instead, from the
highlighter round to the pencil and back. The eraser is not among them: from the eraser they step
from the pen it goes back to (with the fountain pen on the tool button and the highlighter and the
fineliner pinned: highlighter, eraser, next tool: the fineliner); the eraser's own button goes back
to that pen itself. They do nothing while the tool picker is open.

`picker-previous` and `picker-next` step through the tool picker's nine pens in its order. While
the picker is open (not while it pins pens) they move its choice and leave it open. With it closed
they switch tools through all nine, from the eraser as above (marker, eraser, next in the picker:
the fountain pen): a pinned pen is picked as a tap on its pin picks it (the tool button keeps its
pen), another goes to the tool button. The starter mappings give them no key or button.

A menu's button opens its menu as the panel's button does, and closes it when pressed again; with
another menu in front it closes that one and opens its own. `pages` and `index` turn the Pages
view to their tab, and close it on their own. `close-menu` closes whatever menu is in front: a link
being placed is not made, as Back there does, nor one still waiting for its box. `paper` owns the
paper sheet, its Templates and Photos sheet and Custom music. On Pen feel, `pen-feel` and
`close-menu` go back. No menu button acts while the Pages view's keyboard is open, so typing is
never lost. Existing shortcut files are never rewritten. Versions 1.2.0 and 2.0.0-beta.1 to beta.4
refuse the actions added in 2.0.0-beta.5 (`hold-eraser`, `eraser-mode`, `clear-page`, `first-page`,
`last-page`, `new-page`, `fit-page`, `pages`, `tool-picker`, `paper`, `display`, `pen-feel`,
`fold-panel`, `library`, `close-menu`, `picker-previous`, `picker-next`) and count them in Reload
shortcuts.ini; their other lines still work.

Key names include letters, digits, Left/Right/Up/Down, PageUp/PageDown, Space, Enter, Tab,
Escape, VolumeUp/VolumeDown, PlayPause, NextSong/PreviousSong, Home, End, Back and `codeNN` (Linux
evdev number 1–767). Modifiers Ctrl, Shift, Alt, Meta match exactly. Gamepad buttons are named
South, East, North, West, L1, R1, L2, R2, Select, Start, Up, Down, Left, Right, LT, RT (analog
triggers, above), or numbered 256–767. Later duplicate bindings replace earlier ones. Up to 64 bindings and 64 KiB of
configuration are accepted. An existing empty file deliberately disables mappings.

Shortcuts act only in the idle notebook editor. They do nothing while drawing, pinching,
sleeping or displaying a sheet, except the menus' buttons and Close menu (above), Erase while held,
whose press counts even during a stroke (from the next one), and next and previous in the picker,
which move the open tool picker's choice. Next page on the last
page adds a page, as the editor's + button there does: on the open page's paper, or the notebook's
own after an imported, book or photo page. Presses act once; repeats do not trigger actions, and
a release only ends Erase while held (so does the controller going, or the Kindle going to sleep). While shortcuts are enabled, a key or
button press keeps the Kindle awake as the pen and touch do: "Sleep after" counts from the last
one, on any screen. Native PIN input is discarded and never logged. The app logs attachment
paths, not key presses or device names. The visible last-input display belongs to the current
unlocked Grimoire session.

Grimoire watches up to four keyboards and controllers that the Kindle shows as
`/dev/input/event*`, checks for new ones every five seconds (never while the pen is near the
screen or a finger is on it: one that connects while you write is noticed once the pen has been
away 1.5 seconds), and never grabs them. A short toast
says when one connects or disconnects, at most once each way in 30 seconds, so a link that keeps
dropping does not flash the screen. A controller that is switched off is noticed only when
Bluetooth gives up on its link, and on a link the controller started (as it does when it comes
back), the controller decides how long that takes: an 8BitDo in Xbox mode was let go only when it
was switched on again, so its disconnect toast came just before the connect one. Grimoire hears of
it within tens of milliseconds of the Bluetooth helper (below), and neither can tell sooner: an
idle controller sends nothing, so it looks the same as one switched off. When another app holds a
controller exclusively (a button mapper), Grimoire cannot get its presses: the toast and the
Controller sheet say so. Analog sticks are not read; analog triggers are, as LT and RT (above).

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
Scripts inject presses (`key CODE MODS PAD`), releases (`keyup CODE MODS PAD`), an Xbox-layout
controller's axes (`axis CODE VALUE`, its triggers read as LT and RT) and devices (`hid attach pad|key|held NAME`,
`hid detach NAME`), and `tests/bt_check.sh` runs the pairing sheet against a stand-in for the
helper's API (`tests/fake_bthelper.py`). None of this proves Bluetooth, controller discovery or
real button numbering on a Kindle.
