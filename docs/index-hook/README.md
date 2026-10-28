# The Index hook: what the owner asked for, and every ruling since

Grimoire's Index and links are now "arguably the hook of this app (note-taking wise)" (the owner, 2026-10-27).
The owner wants the experience to be "smooth, well thought out, and intuitive in terms of GUI". This file is the
binding summary for anyone building it. The design files next to it hold the detail:

- `index-hook-final-plan-2026-10-27.json`: the synthesized plan. It covers the experience, gestures, the Links bar,
  link areas and the card, smart shapes, the Index map, actions, data and formats, and e-ink and performance. It
  has phases P1-P7 with steps, tests and owner decisions. It was written against the C app at 89893aa. The app is
  now Rust (see below), so read its file and function names as the Rust module of the same name.
- `index-hook-shape-gestures-2026-10-27.json`: the recorded one-finger shape gesture subsystem in depth. It covers
  capture, the integer $P matcher, recording, the trail and storage. It also covers the sideways "fit the whole page"
  setting.
- `index-hook-design-2026-10-27.json`: the raw design run: five code maps, three competing specs and two judges.
- `PLAN-V2.md` (added when ready): the plan revised for all the rulings below and for the Rust port, with the first
  implementation brief.

## The app is Rust now

The owner ordered a total rewrite, and the port is at parity on branch `claude/rust-port`. The crate is `rust/`,
with one module per former C file. `rust/PORTING.md` is the binding contract: byte-exact formats, logs, UI strings
and refresh rectangles; the display hard rules; the fork rules; the float rules; no em dashes.

The headless runner keeps the `.qs` protocol, so `tests/scenarios` run against it. The gate is `make rust-check`.
All Index work goes into the Rust crate. The C tree under `src/` is reference only.

Two features land first and must be respected:

- **Live view fast ink**: a pen sample stream to the browser, with provisional ink.
- **The clock/battery corner**: toggled by a finger TAP in a 14 mm zone in its corner of the notebook page (the top
  left unless the owner picks another: settings.ini `status_corner`; it shows on the notebook page only, never on
  the Library, a sheet or the Pages view). No gesture may claim a tap in that zone. A shape that starts there is
  still a shape. The Live view icon sits beside the panel, and every menu draws over the panel (the owner,
  2026-10-28).

## The owner's brief (verbatim)

> New menu at the bottom of "Settings" in between "Controller" and "Updates" called "Gestures" it will have the
> ability to assign anything the controller category can, and you would record a gesture to get whatever you want
> -- HOWEVER -- by default, the gesture system will serve the user navigationally. Two finger drag down now opens
> the pages menu, and from the bottom up opens the index "hotbar" which will simply show a bottom bound menu where
> users can see the index entries that are linked to in that particular page, long press to change name or delete
> any link, or tap to instantly jump -- BUT TAP OUTSIDE THE HOTBAR to confirm you want to work in the new page and
> close the hotbar. at the very bottom, a search icon (go to page) a link icon (to make a new link) and an eye icon
> that dims off by default for showing the area that is counted as tappable for a link. Next, if users want visible
> borders, they can draw them with a new smart tool system (should be more aware and less accidental input prone
> than amazon's, for drawing circles that snap, squares, lines, triangles, whatever else) on by default with a
> toggle button in the toolbar. Clicking the actual index button which will also be at the bottom now opens a more
> detailed view of the index, which has all pages linked as dots (orphans are progressively smaller or even better
> different shades/dithers or hollowed out) a la Navi org roam view or obsidian. [...] everything with a toggle or
> important setting can be assigned in shortcuts or gestures.

## Rulings and decisions (all binding)

- **R1. Making a link no longer leaves ink.**
  - A link's area is an invisible region. The bar's eye toggle shows it (off by default), and so does making the
    link.
  - A visible border is optional ordinary ink, drawn with smart shapes.
  - Existing beta.5 inked-lasso links migrate: their stroke box becomes the area, and their ink stays on the page
    as ink.
  - links.ini is new in the unreleased beta.5, so it may change.
- **R2. Long-press a link on the page.**
  - A dotted border shows its bounds.
  - A small card beside it offers Delete link and Rename. The plan also adds Change where it goes and Redraw area.
  - The notebook page stays in view: no switch to the Pages view.
  - The bar's long press uses the same card.
- **D1.** Links bar rows: one row per Index entry the page links to (two links to one entry show once, marked xN).
  Holding a row renames the entry, or deletes/retargets all its links on this page in one undoable step. Holding a
  link on the page acts on that one link.
- **D2.** While hopping, the bar keeps listing the page you started on. Tap the page to stay. X, two fingers down or
  Back return to the start.
- **D3.** Rename renames the target page's Index name everywhere. On an unnamed page it reads
  'Add page N to the Index...'.
- **D4. Gestures are recorded.** The owner: "i wanted more of a gesture system where a user can record a basic shape
  or pattern and use them anywhere".
  - A shape drawn with ONE FINGER anywhere on the page, at any zoom, is a gesture.
  - The user records a shape (2-3 samples) for any action.
  - Multi-finger swipes and taps record as those patterns too.
- **D5. Two fingers pan when the page can scroll**, zoomed in or sideways. There, the user's shapes and the panel's
  Pages/Links buttons open those menus.
  - NEW SETTING: sideways, fit the whole page (no scrolling), so the two-finger swipes work in every orientation.
  - On by default. Assignable like every toggle.
- **D6. Built-in gestures:**
  - two fingers down = Pages;
  - two fingers up = Links bar;
  - two fingers left/right = next/previous page (never adds a page; at the end, a toast).
  - The mirror rule closes menus.
- **D7.** The panel gets both new buttons: a Shapes toggle and a Links bar button.
- **D8.** One Gestures sheet for touch and pen gestures (the pen plan's pen.ini folds into gestures.ini).

## Phases (each ships as a Kindle build the owner tests)

1. Gestures (two-finger built-ins, the touch pipeline, contact-level runner input) and the Links bar (list, peek
   and commit, the Find sheet, the eye).
2. Ink-free links and the long-press card (links.ini area lines, migration, link edits in undo, ink-free link
   making).
3. Settings > Gestures with recording (the integer $P matcher, the trail, gestures.ini), and every toggle assignable.
4. Smart shapes (the recognizer and its corpus, hold-to-snap preview, one-undo).
5. The Index map (graph model and layout, the view, satellites, the tray).
6. Pen-button gestures in the same sheet.
