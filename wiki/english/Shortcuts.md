**English** | [日本語](Shortcuts-ja) | [Polski](Shortcuts-pl)

# Keyboard and Controller Shortcuts

A controller (or a keyboard) is express keys for the hand that does not hold the pen: pages,
tools, undo and redo, the eraser and white ink, links and page names, in the notebook editor. It
opens and closes the editor's menus, but does not move around inside them (but for next and
previous in the tool picker): tap there.

Enable **Library > Settings > Shortcuts > Enable shortcuts**. This writes starter mappings to
`/mnt/us/grimoire/shortcuts.ini` only if that file does not exist. Edit it over USB/SSH and choose
**Reload shortcuts.ini**; that row says how many mappings loaded and how many lines were refused.
The sheet lists the keyboards and controllers, and shows the last key or controller button code.
Disable shortcuts to close the optional input devices again.

Settings' top rows are **Display**, **Pen sensitivity**, **Shortcuts** and **Calibrate pen and
touch**, in that order; the bottom row's **Pair** opens the Bluetooth part: Bluetooth controller,
Pair a controller... and a line naming what is connected.

## The keyboard scheme

One scheme for a keyboard paired with the Kindle. An Apple keyboard's Cmd reports as Meta on the
Kindle, and a key with Meta held and no line for it is looked up again with Ctrl in Meta's place,
so Cmd+Z is Undo there too. Shortcuts stay opt-in.

A key may mean one thing in each place: **the Library** (and the Share screen, where only Esc
acts), **a notebook page** (whatever menu is over it), **the Pages view**, **the Graph** and **Pen
feel**.

| place | keys |
|---|---|
| everywhere | Esc `back`, Ctrl+S `save` |
| the Library | Ctrl+N `new-notebook`, Ctrl+F `new-folder`, S `share`, O `settings`, Left/Right its screens, G the Graph |
| a notebook page | 1 to 9 the pens, 0 and E the eraser, W white ink, Ctrl+Z / Ctrl+Shift+Z / Ctrl+Y undo and redo, Left/Right and PageUp/PageDown pages, Home/End, [ and ] tools, Shift+[ and Shift+] the pen's size, Ctrl+N `new-page`, Ctrl+F `go-to-page`, Ctrl+L `add-link`, R `name-page`, Space `two-finger-up`, - and = zoom, Ctrl+0 the whole page, G L P T S, X held erases |
| the Pages view | Left/Right its screens, Ctrl+N, Ctrl+F, Ctrl+A `select-all`, Ctrl+Z and Ctrl+Shift+Z its own Undo and Redo, P, G |
| the Graph | the arrows `pan-*`, - and +, Ctrl+0, V `graph-view`, Ctrl+A, G |
| Pen feel | 1 to 9 that pen's tab, 0 and E the eraser's |
| a keyboard up | Left/Right the caret a letter, Home/End to the start and end (with Shift they select), Ctrl or Cmd with A, C, X and V select all, copy, cut and paste the typed text; these come before any shortcut and cannot be rebound; not on the count pad |

**Esc** eats one layer a press: a field or pad, a dialog or card, a drag, a selection, a menu or
sheet, a link being placed, then back to the Library from a notebook, back from Pen feel and
Share, up to the top from a folder, and at the Library's top **Exit Grimoire?**. **Space** does
what two fingers swiped up do there. **Ctrl+S** writes the page and its notebook now. **R** opens
the page's naming keyboard.

## Device profiles

Each keyboard or controller, USB or Bluetooth, can have shortcuts of its own, saved and used
again whenever it is connected. One that has none uses the default: `shortcuts.ini` and
`controller.ini`.

**Settings > Shortcuts** shows **Device**, **Own shortcuts for this device**, **Buttons...**,
**Reload shortcuts.ini**, the last input and Done. Turn **Own shortcuts for this device** on: the
first time, from a copy of the default, so it does what it did until you change it. Turned off,
the device uses the default again and its profile is kept. Files live in
`/mnt/us/grimoire/shortcuts/`, named by the device's slug (its name in lower case, runs of other
characters as `-`, then vendor and product numbers in hex: `8bitdo-pro-2-2dc8-6006`). Up to 32
profiles are read.

## Controller buttons

**Settings > Shortcuts > Buttons...** shows what each button does and learns new ones. Its list
has seven groups: **Editing**, **Tools**, **Pages**, **Index**, **Menus**, **Screen** and
**System**. Open a group, tap a row, then press the button; Done goes back up. One button per
action: a button that did something else does this now (a toast says which action lost it), and
the action's other buttons stop doing it. The previous / next and first / last rows learn two
presses, the first one's and then the second's. Ten seconds without a press leave the row as it
was. Hold a row (0.6 s) to take its buttons away; **Reset buttons** deletes `controller.ini`.

An Xbox-layout controller's triggers are read as buttons named LT and RT. A Switch-mode controller
numbers its buttons in the Pro Controller's own order (B, A, Y, X, L, R, ZL, ZR, -, +, LS, RS,
Home, Capture); the sheet shows those labels and the files keep the numbers.

`/mnt/us/grimoire/controller.ini` holds the learned buttons; while it is there, the controller's
buttons come from it alone and the `pad.` lines of `shortcuts.ini` are set aside. Keyboard (`key.`)
lines always come from `shortcuts.ini`.

## shortcuts.ini

The default keyboard maps 1–9 to the nine drawing tools in the tool picker's order (pencil,
mechanical pencil, charcoal, ballpoint, fineliner, marker, fountain pen, calligraphy pen,
highlighter), 0 and E to the eraser, W to white ink, Ctrl+Z / Ctrl+Shift+Z / Ctrl+Y to undo and
redo, left/right and PageUp/PageDown to pages, [ and ] to tool cycling, and Shift+[ and Shift+] to
the pen's size.

Gamepad defaults use the Linux gamepad convention: South undo, East eraser, North redo, West
white, L1/R1 previous/next tool, Select/Start previous/next page, and the D-pad Left/Right for
pages and Up/Down for tools. These are starting points; **Buttons...** is for the rest.

```ini
key.1=tool:pencil
key.Ctrl+z=undo
key.Shift+Left=previous-page
key.code106=next-page
key.Ctrl+i=index
pad.East=tool:eraser
```

Older versions refuse the newer action names (`save`, `back`, `two-finger-up`, `new-notebook`,
`new-folder`, `share`, `settings`, and the other 2.0.0-beta.2 ones), one line each.
