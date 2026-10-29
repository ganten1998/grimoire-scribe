# Keyboard and controller shortcuts

A controller (or a keyboard) is express keys for the hand that does not hold the pen: pages,
tools, undo and redo, the eraser and white ink, links and page names, in the notebook editor. It
opens and closes the editor's menus, but does not move around inside them (but for next and
previous in the tool picker, below): tap there.

Enable **Library > Settings > Controller > Enable shortcuts**. This writes starter mappings to
`/mnt/us/grimoire/shortcuts.ini` only if that file does not exist. Edit it over USB/SSH and choose
**Reload shortcuts.ini**; that row says how many mappings loaded and how many lines were refused.
The sheet shows the controller or keyboard Grimoire reads now (its own name), or that another app
holds it (see below), and the last key or controller button code. Disable shortcuts to close the
optional input devices again.

## Controller buttons

**Settings > Controller > Buttons...** shows what each button does and learns new ones. Its list
has seven registry groups: **Editing** (undo, redo, erase and clear), **Tools** (pens and tool
steps), **Pages** (page movement, zoom, scroll and sideways behavior), **Index** (links, finding
and naming), **Menus** (surfaces, panel and status), **Screen** (light, warmth, rotation and
appearance) and **System** (gesture switch, language, sleep after and update channel).
The registry table below gives the exact membership and source restrictions. Open a group,
tap a row, then press the button; Done goes back up to the list. One button per action: a button
that did something else does this now (a toast says which action lost it), and the action's other
buttons stop doing it. The previous / next and first / last rows learn two presses, the first one's
and then the second's. Ten seconds without a press leave the row as it was; a second tap on it stops
it too. The buttons learned are written to `/mnt/us/grimoire/controller.ini` (atomically, every
time). While that file is there, the controller's buttons come from it alone and the `pad.` lines of
`shortcuts.ini` are set aside; keyboard (`key.`) lines always come from `shortcuts.ini`. Hold a row
(0.6 s) to take its buttons away, that row's alone (both actions of a previous / next or first /
last row): it reads "none", a toast says so, and `controller.ini` is written without them, so it
stays that way. **Reset buttons** deletes `controller.ini`, every row at once.

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
  its Pages or its Index tab), `landscape-fit` (sideways, the whole page or the page's width, in turn:
  the Display sheet's "Sideways, show the whole page" row, with a toast; upright only the toast, as it
  applies when the Kindle is sideways), `add-link` (Add a link) and `name-page` (Name the page), both
  below, and `gesture-new-page` ("Swiping past the last page adds a page": turns that behavior on
  or off, as the Display sheet's checkbox does).
- **Index**: `links` (the Links bar: the links on the open page, one row for each page they lead to; it
  opens and closes with its own button, and two fingers up also open it, two fingers down close it),
  `go-to-page` (Go to page or name: a keyboard over the page, the same sheet the Pages view's Go to page
  opens, closed again by its own button), `go-back` (Back to the page before: the page a link, the Pages
  view or Find left, the newest first, up to 32, forgotten when the notebook closes; a page turn leaves
  nothing on that list, the page on show is never the one it goes back to, and with none a toast says so) and `link-areas` (dotted boxes over the page where a
  finger tap follows a link, on and off, remembered; the Links bar's Areas button does the same).
- **Menus**: `tool-picker`, `paper`, `display`, `pen-feel`, `fold-panel` (folds the panel, or
  unfolds it), `library` (saves and closes the notebook), `close-menu`, `status-toggle` (the
  small clock and battery on a notebook page on and off, as a finger tap in its corner does) and
  `status-corner` (the next of the page's four corners for it, clockwise from the top left, as the Display
  sheet's corner cell does; docs/KINDLE.md, The clock and battery corner). With no notebook page in front
  they change the setting and say so with a toast, since it shows on a page only.

`previous-tool` and `next-tool` step through the pens on the editor's panel alone: the tool button's
pen and the pinned pens, top to bottom as the panel shows them, from the last round to the first and
back. Pressed again and again they keep to the panel, so the highlight never jumps into the tool
picker and back. With fewer than two pens on the panel they step to the tool picker's next or
previous pen in its order instead, and that pen goes to the tool button. With none pinned that walks
all nine, from the highlighter round to the pencil and back. With only the tool button's own pen
pinned (the button shows a grid), the first press brings the picker's next or previous pen onto the
tool button, and from then on they go between those two, as with any two pens on the panel. The
eraser is not among them: from the eraser they step from the pen it goes back to (with the fountain
pen on the tool button and the highlighter and the fineliner pinned: highlighter, eraser, next tool:
the fineliner); the eraser's own button goes back to that pen itself. They do nothing while the tool
picker is open.

`picker-previous` and `picker-next` step through the tool picker's nine pens in its order. While
the picker is open (not while it pins pens) they move its choice and leave it open. With it closed
they switch tools through all nine, from the eraser as above (marker, eraser, next in the picker:
the fountain pen): a pinned pen is picked as a tap on its pin picks it (the tool button keeps its
pen), another goes to the tool button. The starter mappings give them no key or button.

`add-link` starts a link on the open page in one press, as the Index's Add a link does, without
going through the Pages view: a strip along the bottom asks you to draw around what links, the next
pen stroke on the page draws a dashed lasso around the link's area (it leaves no ink, whatever the
tool), and Link to... then asks where it goes (a new page, The Index, a recent page, an Index entry,
Search, or Choose from the pages...). It does not wait for ink: the stroke that follows draws the area
only. Pressed again while the strip or Link to... is up, it is not made (a toast says so), as Close
menu and their Cancel do. On a
read-only page, where the notebook's `links.ini` does not read, and on a page that has 64 links, it
does nothing but say so. `name-page` opens the Index's naming
keyboard for the open page, holding its name: Done saves it (an empty name takes it out of the
Index), Cancel keeps the old one, and either goes back to the page, not to the Index; the Pages
view then opens again on the tab it was left on. Where the notebook's `nav.ini` does not read it
does nothing (Name this page is greyed out then). Neither acts while a menu is open, and the
starter mappings give them no key or button.

A menu's button opens its menu as the panel's button does, and closes it when pressed again; with
another menu in front it closes that one and opens its own. `pages` and `index` turn the Pages
view to their tab, and close it on their own. `close-menu` closes whatever menu is in front: a link
being placed is not made, as Back there does, nor one still waiting for its box. `paper` owns the
paper sheet, its Templates and Photos sheet and Custom music. On Pen feel, `pen-feel` and
`close-menu` go back. No menu button acts while the Pages view's keyboard is open, so typing is
never lost. Existing shortcut files are never rewritten. Versions 1.2.0 and 2.0.0-beta.1 to beta.4
refuse the actions added in 2.0.0-beta.5 (`hold-eraser`, `eraser-mode`, `clear-page`, `first-page`,
`last-page`, `new-page`, `fit-page`, `pages`, `tool-picker`, `paper`, `display`, `pen-feel`,
`fold-panel`, `library`, `close-menu`, `picker-previous`, `picker-next`, `add-link`,
`name-page`, `status-toggle`, `status-corner`, `links`, `go-to-page`, `go-back`, `link-areas`, `landscape-fit`, `gesture-new-page`) and count
them in Reload shortcuts.ini; their other lines still work.

Two-finger gestures are bound to the same actions in `gestures.ini` (docs/FORMAT.md) and need no Enable
shortcuts: the Settings switch for them is `gestures` in settings.ini. Their built-ins are two fingers
down `pages`, up `links`, left `next-page`, right `previous-page`, and a page turned left on the last page
adds a page, as the next-page key does (`gesture_new_page=0` in settings.ini, the Display sheet's "Swiping past the
last page adds a page" row, or the `gesture-new-page` action turns that off: the swipe then only says it is the last page). A line `touch.2.up=nothing` turns a built-in off; the file is
read at start and when shortcuts are reloaded.

### Pen-button gestures

Settings > Gestures records the pen button as well as fingers. Tap an action, click the
button while hovering, or hold it / drag with it, then choose Use. The card shows the
recognized name at 64 px, with Use, Try again and Cancel. A normal nib tap operates those
buttons; recording draws no page ink. An occupied binding names its current action before
Use moves it, then a toast names the action that lost it. Shape recording keeps its separate
first-Use conflict check and second-Use confirmation when that check discovers a clash.

The five slots are click (`tap`), double-click, triple-click, hold and drag. A click lasts at
most 450 ms without nib contact. When double or triple is bound, a single click waits 350 ms
after release. Without either, it acts at release. For double/triple, each next press must be
within 350 ms of the previous release; recognition resolves 350 ms after the final release.
The built-in triple-click and a 1.2 s hovering hold both clear the page; clearing the Clear
page row removes both. Leaving pen proximity cancels an unfinished click sequence.

Erase while held is pen-only in this sheet. Record a hold or drag for it: it erases from the
button's press to its release, with no 1.2 s wait; short clicks can still have their own action.
Pan with the pen and Zoom with the pen are Pages rows that accept only a button drag.
A bound drag performs its action once and leaves no ink, including in Live view. Pan moves a
scrollable page; zoom follows vertical motion around the starting point (up zooms in,
1.25 times per 10 mm, within the zoom limits). An unbound drag retains Settings' side-button
ink choice: nothing, eraser, highlighter or white. The Settings row changes only that choice.
Pen Next page never adds a page at the end of a notebook.

Rows show the first available source in touch, shape, pen order, then `+N` additional sources.
Each action/source counts once even if several slots use it; paired rows summarize both actions.
A two-slot panel skips three-finger bindings. A second button is shown only on hardware that
reports it; unavailable Button2 and Eraser bindings remain in the file for compatible versions.

### gestures.ini by hand

`gestures.ini` sits beside `shortcuts.ini` in `/mnt/us/grimoire/`. Settings > Gestures now
writes it. With no file the built-ins above apply. It is an overlay: only changed slots are
written, `nothing` clears a built-in, and Reset removes the file and restores the defaults.
The exact header, carry limits, shape codec and atomic write contract are in [FORMAT.md](FORMAT.md).

```ini
# two fingers up make a new page; two fingers down do nothing
touch.2.up=new-page
touch.2.down=nothing
# a two-finger tap undoes
touch.2.tap=undo
# one-finger swipes are also assignable
touch.1.right=go-back
# the pen button's double-click opens Links
pen.Button.double=links
# disable both clear-page built-ins
pen.Button.triple=nothing
pen.Button.hold=nothing
# holding the button while moving the nib pans instead of drawing
pen.Button.drag=pan
```

`touch.1.up/down/left/right` are valid; `touch.1.tap` stays reserved for taps and links.
`touch.2.*` and `touch.3.*` accept up/down/left/right/tap. Three-finger patterns require at
least three reported slots; a two-slot panel neither shows three-finger rows nor promotes a
partial three-finger attempt into a two-finger action. `shape.ACTION` stores two or three
recorded samples; it is best created with the record card, not hand-written.

The provisional `TG_TUNE` accepts a 16 mm swipe within 1200 ms, with each finger travelling
at least 8 mm, and a two-finger tap within 400 ms moving at most 6 mm. These are implementation
defaults, not owner measurements; see [the pending study](index-hook/TOUCH-STUDY.md). Two fingers
pan while zoomed in. Nearby writing, palms and extra fingers suppress actions; a recent, distant
pen hover can allow a swipe, and a pinch can begin straight after writing. A brief lost finger
in a pinch/pan has an 80 ms re-grab grace period within 15 mm.

With a menu or sheet in front, its own bound gesture closes it, as can its opposite; other
actions follow the surface's input rules. With the Links bar up, two fingers down always close
it, back to the page it began on. Finger shapes can act over the Links bar without becoming ink,
links or undo entries. Clock-corner taps keep their existing behavior; moving into a shape can
still arm capture there. Opening Settings > Gestures reloads the map, as does Controller >
Reload shortcuts.ini. Unsupported source/action combinations and malformed lines are refused
and counted; unknown future lines are carried within the limits in FORMAT.md.

Key names include letters, digits, Left/Right/Up/Down, PageUp/PageDown, Space, Enter, Tab,
Escape, VolumeUp/VolumeDown, PlayPause, NextSong/PreviousSong, Home, End, Back and `codeNN` (Linux
evdev number 1–767). Modifiers Ctrl, Shift, Alt, Meta match exactly. Gamepad buttons are named
South, East, North, West, L1, R1, L2, R2, Select, Start, Up, Down, Left, Right, LT, RT (analog
triggers, above), or numbered 256–767. Later duplicate bindings replace earlier ones. Up to 64 bindings and 64 KiB of
configuration are accepted. An existing empty file deliberately disables mappings.

Shortcuts act only in the idle notebook editor, apart from `status-toggle` and `status-corner`, which
act on every screen while Grimoire is awake. They do nothing while drawing, pinching,
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

## Action registry (through phase 6)

`rust/src/actions.rs` is the shared registry for parsing, dispatch and both learning sheets.
Names below are the persistent INI values; numeric ids are internal. The new actions use ids
56..77, preserving 54 (`gesture-new-page`) and 55 (`status-mode`); later actions retain 78..81,
and pen pan/zoom use 82/83. Groups are Editing, Tools,
Pages, Index, Menus, Screen and System. `tool:pencil` remains a compatibility entry even though
the current Tools learning list starts with the mechanical pencil.

Toggle changes a boolean; Menu opens/closes a surface; Step moves one increment; Cycle advances
through choices; OneShot runs once; Momentary lasts until release. Touch patterns and shapes
have no release, so `hold-eraser` accepts key, pad and only the pen's hold/drag slots. Calibration
is deliberately outside the assignable registry. `shortcuts` (Enable shortcuts) and
`bluetooth-controller` accept touch, shape and pen; keys/pads cannot turn off their own input.
Every registered action accepts PEN. Pan and zoom accept only a pen drag.

| Persistent action name | Kind | Group | Sources |
| --- | --- | --- | --- |
| `tool:pencil` | Cycle | Tools | Key, pad, touch, shape, pen |
| `tool:mech` | Cycle | Tools | Key, pad, touch, shape, pen |
| `tool:ballpoint` | Cycle | Tools | Key, pad, touch, shape, pen |
| `tool:eraser` | Cycle | Editing | Key, pad, touch, shape, pen |
| `tool:highlighter` | Cycle | Tools | Key, pad, touch, shape, pen |
| `tool:fountain` | Cycle | Tools | Key, pad, touch, shape, pen |
| `tool:fineliner` | Cycle | Tools | Key, pad, touch, shape, pen |
| `tool:marker` | Cycle | Tools | Key, pad, touch, shape, pen |
| `tool:charcoal` | Cycle | Tools | Key, pad, touch, shape, pen |
| `tool:calligraphy` | Cycle | Tools | Key, pad, touch, shape, pen |
| `undo` | OneShot | Editing | Key, pad, touch, shape, pen |
| `redo` | OneShot | Editing | Key, pad, touch, shape, pen |
| `previous-page` | Step | Pages | Key, pad, touch, shape, pen |
| `next-page` | Step | Pages | Key, pad, touch, shape, pen |
| `previous-tool` | Step | Tools | Key, pad, touch, shape, pen |
| `next-tool` | Step | Tools | Key, pad, touch, shape, pen |
| `white` | Toggle | Editing | Key, pad, touch, shape, pen |
| `index` | Menu | Index | Key, pad, touch, shape, pen |
| `hold-eraser` | Momentary | Editing | Key, pad, pen (hold/drag) |
| `eraser-mode` | Toggle | Editing | Key, pad, touch, shape, pen |
| `clear-page` | OneShot | Editing | Key, pad, touch, shape, pen |
| `first-page` | Step | Pages | Key, pad, touch, shape, pen |
| `last-page` | Step | Pages | Key, pad, touch, shape, pen |
| `new-page` | OneShot | Pages | Key, pad, touch, shape, pen |
| `fit-page` | OneShot | Pages | Key, pad, touch, shape, pen |
| `pages` | Menu | Pages | Key, pad, touch, shape, pen |
| `tool-picker` | Menu | Menus | Key, pad, touch, shape, pen |
| `paper` | Menu | Menus | Key, pad, touch, shape, pen |
| `display` | Menu | Menus | Key, pad, touch, shape, pen |
| `pen-feel` | Menu | Menus | Key, pad, touch, shape, pen |
| `fold-panel` | Toggle | Menus | Key, pad, touch, shape, pen |
| `library` | Menu | Menus | Key, pad, touch, shape, pen |
| `close-menu` | OneShot | Menus | Key, pad, touch, shape, pen |
| `picker-previous` | Step | Tools | Key, pad, touch, shape, pen |
| `picker-next` | Step | Tools | Key, pad, touch, shape, pen |
| `add-link` | Menu | Index | Key, pad, touch, shape, pen |
| `name-page` | Menu | Index | Key, pad, touch, shape, pen |
| `status-toggle` | Toggle | Menus | Key, pad, touch, shape, pen |
| `links` | Menu | Index | Key, pad, touch, shape, pen |
| `go-to-page` | Menu | Index | Key, pad, touch, shape, pen |
| `go-back` | OneShot | Index | Key, pad, touch, shape, pen |
| `link-areas` | Toggle | Index | Key, pad, touch, shape, pen |
| `landscape-fit` | Toggle | Pages | Key, pad, touch, shape, pen |
| `status-corner` | Cycle | Menus | Key, pad, touch, shape, pen |
| `gesture-new-page` | Toggle | Pages | Key, pad, touch, shape, pen |
| `status-mode` | Cycle | Menus | Key, pad, touch, shape, pen |
| `zoom-out` | Step | Pages | Key, pad, touch, shape, pen |
| `zoom-in` | Step | Pages | Key, pad, touch, shape, pen |
| `scroll-up` | Step | Pages | Key, pad, touch, shape, pen |
| `scroll-down` | Step | Pages | Key, pad, touch, shape, pen |
| `invert` | Toggle | Screen | Key, pad, touch, shape, pen |
| `light-down` | Step | Screen | Key, pad, touch, shape, pen |
| `light-up` | Step | Screen | Key, pad, touch, shape, pen |
| `warmth-down` | Step | Screen | Key, pad, touch, shape, pen |
| `warmth-up` | Step | Screen | Key, pad, touch, shape, pen |
| `rotation-lock` | Toggle | Screen | Key, pad, touch, shape, pen |
| `pinch-zoom` | Toggle | Screen | Key, pad, touch, shape, pen |
| `theme` | Cycle | Screen | Key, pad, touch, shape, pen |
| `side-button` | Cycle | Screen | Key, pad, touch, shape, pen |
| `mirror-pdf` | Toggle | Screen | Key, pad, touch, shape, pen |
| `sleep-screen` | Cycle | Screen | Key, pad, touch, shape, pen |
| `sleep` | OneShot | Screen | Key, pad, touch, shape, pen |
| `gestures` | Toggle | System | Key, pad, touch, shape, pen |
| `shortcuts` | Toggle | System | Touch, shape, pen |
| `bluetooth-controller` | Toggle | System | Touch, shape, pen |
| `language` | Cycle | System | Key, pad, touch, shape, pen |
| `sleep-after` | Cycle | System | Key, pad, touch, shape, pen |
| `update-channel` | Cycle | System | Key, pad, touch, shape, pen |
| `shapes` | Toggle | Editing | Key, pad, touch, shape, pen |
| `shape-hold` | Cycle | Editing | Key, pad, touch, shape, pen |
| `shape-last` | OneShot | Editing | Key, pad, touch, shape, pen |
| `index-map` | Menu | Index | Key, pad, touch, shape, pen |
| `pan` | Momentary | Pages | Pen (drag only) |
| `zoom` | Momentary | Pages | Pen (drag only) |

Gestures do not need Enable shortcuts. `settings.gestures` gates touch patterns and finger
shapes; pinch/pan retain their own setting. Recording preserves unrelated key and pad bindings.
Each action has at most one touch pattern and one finger shape. Assigning an occupied pattern
or an equivalent shape moves that source to the new action; the record card names the losing
action before Use. Hold a Gestures row to clear its touch, shape and pen bindings together (both members of a pair). Clear,
learn and reassignment save a copy atomically before replacing the active map. Failed saves
leave the live map unchanged; an unreadable file can be replaced only with Reset.

`rotation-lock` stores 1 plus the current rotation, or 0 to follow the Kindle. `sleep-screen`
cycles four existing choices: page, pinned page, Kindle screensaver, KOReader wallpaper (value
3 already existed). The light and warmth actions use cached controls; no input handler waits
for a device read. Theme, language and invert deliberately request a Full refresh.

## Live view keys

While a browser shows Live view and its page has focus, the keyboard of that computer runs
Grimoire's shortcut actions on the Kindle, as a keyboard paired with the Kindle would. The keys
are in their own file, `live-keys.ini` beside `shortcuts.ini`, in the same `key.<key>=<action>`
lines. Grimoire writes it from the built-in keyboard mappings the first time Live view starts,
and reads it at each start: edit it, then stop and start Live view. The page's **Keys** button
lists the keys in use.

Only a browser paired for Live view can send keys, and only while Live view runs. A key held
down sends one press (no repeats); keys are ignored while a field in the page has focus.
Holding a key bound to `hold-eraser` keeps the eraser while it is held, and lets go if the
browser goes quiet for a second and a half.

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

`status-mode` cycles Bar, Corner and Off. `status-toggle` toggles the selected bar or corner.
