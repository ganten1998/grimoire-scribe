# Changelog

## 2.1.0 (unreleased)

- A cleared page, or a page given a new paper, template or photo, no longer shows its old picture (2.0.0 and
  2.0.1). A page that keeps its drawing so that it opens at once (a dense page, or one that took a second or more
  to draw) was drawn from that drawing again after the change, though it was made before it: a cleared page still
  showed all its old ink (with Undo and Clear greyed out, as it had none), a new paper showed the old one, and the
  save 2 seconds later kept that picture, so it came back at every open, in the Pages view's preview, and on the
  Library's card when it was page 1. The page itself was always saved right (a cleared page with no strokes), and
  PDF exports drew it right. A kept drawing now stands for a page only while nothing on it has changed since it
  was saved; a changed page is drawn from its strokes.

- The Links bar learning scenario waits for its incoming rows with the headless runner's new `waitfor`,
  instead of guessing that every runner will check 120 notebooks within 9 seconds. Normal application
  behavior is unchanged. `waitfor KEY OP VALUE` takes the keys and checks of `expect` and lets timers run as
  `wait` does until its check holds, then reads on at once: the main loop goes round every 30 ms, as in a
  `wait` or on the Kindle at rest, and with `GRIMOIRE_SIM_CLOCK=script` app time moves on by what it idled,
  so it can wait on what a timer does (a toast going). It fails after 60 s of real time, and at once for an
  unknown key, a malformed check or a key whose check starts its count again (`display.presents`,
  `main.longest_unpumped_ms`, `pump.calls`, `keyboard.draws`, `fshape.fired`), which it leaves unread for an
  `expect`. A `touch` contact held across it is stamped by the clock again after it, as across a `waitbg`.
- The slow runner the scenario failed on is pinned: `GRIMOIRE_SIM_BACKLINKS_STEP_MS` (headless runner only)
  makes each tick of the backlinks check that much longer, and the scenario runs with 60 ms, so its check
  takes over 11 s however fast the machine is (the old 9 s wait fails every run, the `waitfor` passes). Its
  run with every wait 400 ms late (`make scenarios-late`) does not slow the check: it covers the waits around
  it, not the slow runner.
- More fonts in Classic (the owner, 2026-11-04: "Custom font support/More fonts in the classic theme"). In
  Classic the Style row in Settings has a Font chip at its right end ("Font: Terminess"); the rest of the
  row still changes the style, and Gothic, whose look is its pixel type, has no chip. It opens the Font
  sheet: Terminess (still the default), Atkinson Hyperlegible Next, Noto Sans and Literata, each named in
  itself, the one in use with a check, and on top a sample of the one highlighted (its name, a sentence in
  the language in use, digits and accents). A tap highlights a font and refreshes only the sample and the
  two rows; Done applies it with a full refresh, as a style change does; Cancel, Back and Esc leave the
  font as it was. Every screen's text follows, and so do the share pages in the browser from the next page
  they load, with Live view on too (a style change made while Live view is on now reaches them the same
  way); pages, paper labels, the clock, PDFs, pictures and the icons never do. Each font is drawn a little
  bigger than the size asked so that its letters come out about Terminess's size (Atkinson 1.10, Noto Sans
  1.065, Literata 1.15), and every screen fits in each in English and Spanish, upright and sideways.
  Letters a font lacks (Greek, Cyrillic, a few of Latin Extended-A in Atkinson) come from Terminess, and
  Japanese from DotGothic16, as before. Saved as `font=` in settings.ini (`terminess`, `atkinson`,
  `noto-sans`, `literata`); a value this version does not have draws as Terminess and stays in the file
  until a font is picked. The three families are Google Fonts' releases (Atkinson Hyperlegible Next 2.001,
  Noto Sans 2.015, Literata 3.103 at its 12 pt optical size), each a static Regular and Bold cut down to
  Latin, Latin-1, Latin Extended-A and the general punctuation: 238 KB for the six, with their OFL texts
  in `app/licenses` and `make fonts-subset` making them again (`tools/fonts_classic.py`,
  assets/README.md). The Kindle binary grows by about 0.26 MB (5,840,984 to 6,102,208 bytes) and the zip
  by about 0.13 MB; package-check now fails a binary of 10.5 MB or more and a zip of 5.5 MB or more
  (decision D23).
- A wake no longer leaves parts of the sleep picture (the KOReader wallpaper) on the screen until a redraw (the
  owner's Scribe, 2026-11-05). The framebuffer held the Library whole: the Kindle draws on the panel on its way
  out of the screensaver after Grimoire's first refresh, and the second push was a no-flash one, which drives
  only the pixels the panel's controller thinks changed. That push is now a full refresh, 2.5 s after the wake
  whatever the sleep picture: one flash, and nothing of the sleep picture can stay.
- What's new's Read all lists every version's notes, newest first (the owner, 2026-11-06: "wire read all button
  to a list of each UNIQUE release note, and allow users to easily browse the changes by version - some of them
  are important"). A row for each: the version (the one running marked This version), its date and the notes'
  first line. A tap on a row shows that version's notes whole (on one screen, below), with Back to the list;
  Dismiss closes What's new from the list and from a version's notes, as from the card. Notes that are word for word
  the same are one row naming every version that has them. On a keyboard Enter opens (the card's Read all, then
  the notes of the version running), Esc goes back a step (a version's notes to the list, the list to the card,
  and the card closes), and Left and Right turn the pages. Each step on the sheet refreshes the sheet's place
  alone. The card is as before, this version's notes. The notes are a file for each version now,
  `docs/whatsnew/VERSION.md` (2.0.0's and 2.0.0-beta.1's as they shipped; `docs/WHATSNEW.md` is gone;
  2.0.0-beta.2 showed beta.1's notes and has no file), built in with the dates of this changelog's headings.
  The release checks want this version's file, with notes in it, and not the release before's notes again
  (docs/RELEASING.md, The bump commit).
- Read all shows each version's notes whole on one screen (the owner, 2026-11-07: "The read all doesn't need
  text that big -- either smaller text for all past messages, or fit all (which i lean toward)", and "The note
  reading itself isn't polished enough"). Notes too long for the sheet are first set closer at the size they
  had (44 px): the line spacing down to 1.15 and the gaps between items to 60%. Only if that is not enough does
  the text get smaller, in half pixels, to the largest size that fits at the closest spacing, and the room that
  leaves goes back to the spacing, up to the usual look, so the notes fill their sheet evenly. The title, the
  version line and the buttons keep their sizes, and there are no page arrows. 2.0.0's notes, which took two
  pages in Gothic and sideways in Classic, fit on one: at 44 px in Gothic upright and in Classic's Atkinson,
  Noto Sans and Literata sideways, at 41.5 px in Terminess sideways, and sideways in Gothic, which has no room
  for its large type, in its next size down. The text gets no smaller than 34 px, the size the card sets the
  same notes in: notes too long even for that would take pages at it (none do). Bullets keep the card's
  proportion to the text, so they are a little larger beside 44 px text than before. A word too long for a
  line (a link) is broken across lines rather than cut short, and Japanese or Chinese breaks between
  characters. Read all's sheets, the list's too, never end across the Library's page buttons (they reach
  down to the screen's margin instead, so Page 1 of 1 is never cut through) nor start across its top bar's
  buttons (such a sheet starts at the margin above them instead, as the tallest does). The What's new card
  ends above the page buttons too: sideways it had covered the left one and the P of Page 1 of 1, and now
  shows 2.0.0's nine bullets there, its two paragraphs left to Read all.

## 2.0.0 (2026-11-05)

- The first stable release of Grimoire 2.0: 2.0.0-beta.2 with the fixes below, now on the stable channel too
  (KPM 2.0.9999; a preview subscriber is offered it as well). Fixes come as 2.0.1, 2.0.2 and so on, new features
  as 2.1 and up. Since beta.2 the release workflow publishes on its own again (its publish job had no status
  function in its condition, so a skipped job upstream skipped it), and README's Building from source describes
  the Rust build.
- What's new after every update, and no Don't show this again (the owner's rule of 2026-11-05: "no don't show
  this again, just a "Dismiss" and every update with changes by default has at least a basic what's new"). The
  first start after an update opens the notes once, even where the box of the 2.0.0 betas was ticked; the card
  and Read all close with Dismiss (or Back, or Esc), and the bat in the Library's top bar opens them again.
  `whatsnew_hide` stays in settings.ini as it was, for an older Grimoire that reads the file, and this version
  no longer looks at it. Every release now comes with its What's new: the release checks fail when
  docs/WHATSNEW.md has not changed since the release before (docs/RELEASING.md, The bump commit).
- The What's new card ends at the last note that fits whole above the signature, and leaves the rest to Read
  all (the card had cut the last paragraph of 2.0.0's notes mid-sentence: "please consider donating to"). A
  first note too long for the card on its own is cut at the end of a line.
- Dragging a selection follows the pen however much it holds (the owner's report of 2026-11-05: "the copy paste
  lasso function is horrifically slow on drag with anything containing colored in or multiple strokes"). The
  drag drew its whole band again at every pen sample, the bar and the selection's dotted halo included, so a
  selection of shading or of many strokes took about 22 ms a sample on the Scribe, while the pen reports one
  every 2 to 3 ms, and the outline fell behind the pen. Now a pen sample only notes where the pen is, and the
  outline is drawn at the newest place once the panel has finished the last fast refresh (about five times a
  second at room temperature), or at once when the pen stops: it steps along with the pen and never trails
  behind it. The bar goes as the box starts to move (one fast refresh of its place, which shows the page in
  black-and-white dots until you lift) and comes back under the box when you lift; after that only the outline's
  strips are refreshed, never the box's inside, and nothing of the ink is drawn while it moves, so a selection of
  colored-in hexagons drags as lightly as three words. The box moves once the pen has gone 1 mm (a finger a
  little more), so a tap inside the selection that wobbles a pixel no longer moves it with an undo step, and a
  sleep in the middle of a drag cancels it. The lift is as before: the strokes move to where the pen lifted, one
  undo step, with pencil, mechanical pencil and charcoal grain drawn afresh on the moved part, and links move
  with them. Live view shows the drag as the Kindle does.
- Live view keeps the toolbar in the browser while a menu is up on the Kindle (issue #16, reported by nebuff).
  The toolbar's layer was taken away whenever the Display sheet, the Paper sheet or another of the notebook's
  menus opened, so the browser showed the page where the Kindle still showed the toolbar beside the menu, until
  the menu closed. Now the layer holds what the Kindle shows where the toolbar is: the toolbar beside a menu,
  and a menu's part over it where the menu covers it, whether a setting changes in the menu or not. Settings
  and its Display sheet were already shown as the Kindle shows them, and still are.

## 2.0.0-beta.2 (2026-11-05)

- A smaller download: the zip is 2.9 MB (beta.1's was 7.1 MB) and the app 5.8 MB (8.8 MB). Terminess keeps
  Terminus's own text and only the Material Design icons Grimoire uses: Grimoire's own text looks as before,
  and the box drawing, block elements, heart, lightning, menu sign, power symbols and heavy angle brackets
  that Nerd Fonts had added are gone. The icons' Apache-2.0 notice ships beside the OFL text, and the share
  pages serve the fonts' licenses at `/font/licenses.txt` (`assets/README.md`). Grimoire updates its copy in
  KOReader's `fonts/Grimoire` at its next start.
- KOReader's battery symbol is back in its dropdown and reading footer with the Terminess menus (the owner's
  Scribe showed a "JS" badge in its place, 2026-11-04). The menus use Grimoire's own copy of Terminess in
  `koreader/fonts/Grimoire` whenever it is there, before any other Terminess or Terminus in the Kindle's fonts
  folders; a full Nerd Font, whose icons sit where KOReader's battery symbols are (U+E781 is a JavaScript
  badge), is used only when no other font will do. `koreader/crash.log` names the font the menus use and why
  each other was passed over. A start of Grimoire replaces an earlier Grimoire copy of the Terminess patch in
  KOReader's `patches` (1.1.0-beta.2's, 1.2.0's or 2.0.0-beta.1's, byte for byte) with this one, and
  grimoire.log says so; a copy someone changed is kept, and a patch that is not applied stays off.
- Quieter toasts (the owner's review of every one): confirmations of what the screen already shows are gone,
  and the rest are terse ("Couldn't save page name.", "Couldn't save links. Page unchanged."). A finger shape
  names its action only when the action did something: none for Next page on the last page, Previous page on
  page 1, Clear page on an empty page, or Undo and Redo with nothing to undo or redo. Zoom in at the most zoom,
  Zoom out at the least and a scroll past the first or last page no longer draw the screen again.
- The lasso's clipboard lasts one session: every start of Grimoire clears it, and a paste no longer logs a
  false "read-only page" line.
- Add Japanese: every screen, the share pages and the Effects studio, with a DotGothic16 pixel fallback for kana and kanji (subset to the kana and the common kanji, `assets/README.md`). `po/ja.po`, `po/ja.mo` and `po/ja-glossary.md`; the KOReader plugin's own words follow KOReader's Japanese setting.
- A Japanese keyboard, as a phone has it: with Grimoire in Japanese the keyboard's accents key is
  あ, and it opens a flick kana pad, twelve keys in three columns (あ か さ / た な は / ま や ら /
  わ ゛小 ー). Tap a key for its row's first kana, swipe left, up, right or down for the rest (や
  keeps 「 and 」 on its sides, ゆ above and よ below), ー types straight, and abc goes back to the
  letters. One modifier key cycles the kana typed last: か が か, は ば ぱ は, つ づ っ つ, や ゃ や,
  so dakuten, handakuten and the small kana are one key. カ types the pad in katakana (ノート,
  メモ) and あ back in hiragana; 、 holds 、。？！・ on its flicks. Every keyboard has it, so
  notebook, folder, page and link names all take kana. Every other language keeps áé there.
- Caps on: the keyboard's shift key has three states, a tap each (the owner's "next toggle shift in typing
  toggles off shift for next char then shift on which makes longer caps strings possible"). One tap gives the
  next letter its capital, as before; a second tap is caps on, every letter a capital through digits, spaces,
  delete and the symbols and accents pages, for names such as NASA or a run of initials; a third tap turns it
  off. Shift for one letter shows the pressed shift key, as before; caps on shows the same key with a bar under
  its label. Every keyboard opens with shift off, so caps on ends when the keyboard closes. Every keyboard has
  it: names, Find, link names and the rest.
- Delete held down deletes one character after another (the owner's "hold del long enough in keyboards to
  quick delete"), on every keyboard: names (notebooks, folders, pages, links), Find, the page number pads and the
  count pad for new pages. A tap still deletes one, at the touch-down; kept on the key, it deletes the next after
  0.4 s ("low ish delay but not too low"), then one every 65 ms, about 15 a second ("repeat rate high so it feels
  snappy", "not too high"), the same rate all the way, until the finger or the pen lifts or slides off the key.
  It never runs ahead of the screen: while the panel is still showing the last delete, the next waits and then
  takes the ones that came due together (three at most), so nothing is deleted after the lift that was not
  already on its way to the screen. It stops with the text gone, and on the count pad the touch-down clears the
  opening 1 as before, with nothing to repeat. The link card's Rename keyboard now also gets the gray clean-up
  refresh the other keyboards get once typing pauses. A hardware keyboard's held Backspace still deletes one digit
  on a count pad.
- One keyboard scheme for a keyboard paired with the Kindle (the owner's scheme; docs/SHORTCUTS.md). A key
  may do one thing in each place (the Library, a notebook page, the Pages view, the Graph, Pen feel): in the
  Library Ctrl+N a new notebook, Ctrl+F a new folder, S Share, O Settings, Left and Right its screens; on a page
  0 and E the eraser, Ctrl+L Add a link, R the page's name, Space what two fingers up do (the Links bar), - and
  = (+) zoom, Ctrl+S saves now everywhere. Esc is one action, `back`, eating one layer a press (a field's
  Cancel, a dialog, a selection, a menu one level at a time, a link being placed, then back to the Library, up
  from a folder), and at the Library's top it asks "Exit Grimoire?" (Enter the highlighted answer, Yes at
  first; Left and Right move it; Esc No). Up and Down do nothing but in the Graph; the count pad no longer
  takes them. PageUp and PageDown turn pages (they stepped tools; [ and ] do that), and an Apple keyboard's
  Cmd works as Ctrl. A shortcuts.ini (or live-keys.ini) that is byte for byte a starter map an earlier
  version wrote gets the new keys at the next start (grimoire.log says so); one someone edited is kept and means
  what it meant. Older versions refuse the new action names one line each.
- Settings > Shortcuts and Settings > Pair, in place of the Controller sheet (the owner's "\"Shortcuts\" is
  better now ... a button occupying the previous spot called \"Pair\""). Settings' top rows are Display, Pen
  sensitivity, Shortcuts and Calibrate pen and touch, in that order, and the bottom row's Pair (where
  Controller was) holds Bluetooth controller, Pair a controller... and what is connected. Shortcuts holds
  Enable shortcuts, Buttons... and Reload shortcuts.ini, and each keyboard's or controller's own shortcuts:
  its Device row goes through those connected and those saved, and **Own shortcuts for this device** gives
  the one shown a profile, `grimoire/shortcuts/<device>.ini` (a copy of shortcuts.ini to start from), used
  whenever it is connected; Buttons... then learns its buttons beside it, and takes a press only from a
  controller that uses the buttons it shows (a toast names another; a keyboard's profile leaves it on the
  default's, controller.ini). Turned off, the device uses the
  default again and the profile is kept for when it is turned on. A device is its name and vendor:product
  numbers (two of the same model share a profile). A device with none, and anyone who never makes one, uses
  shortcuts.ini and controller.ini as before, and they are never rewritten by this; gestures, the pen's
  button and finger shapes are not per device (docs/SHORTCUTS.md, Device profiles). The starter
  shortcuts.ini names Settings > Shortcuts; an unedited one from an earlier beta.2 build becomes it at the
  next start, as earlier starters do.
- Pen feel on a keyboard: 1 to 9, 0 and E pick that pen's tab.
- The Pages view on a keyboard: Ctrl+A ticks every page (the new `select-all` action), Ctrl+N opens the number
  pad for new pages, Esc closes a held page's card alone (Pages staying open, also with a shortcuts file that
  keeps Esc as `close-menu`), and Esc stops a drag with nothing moved.
- The number pad for new pages (New page held in Pages, the toolbar's + held, Ctrl+N) starts at 1: Add 1 page
  (or Enter) at once adds one page, the first digit typed replaces the 1, and - and + count from it. A keyboard's
  Up and Down keys no longer change the count (- and + do).
- From the owner's Scribe test of the beta.2 builds: a drag in Pages starts after a shorter hold, 300 ms, half
  the 600 ms a held page's card takes, so the page is picked up and moved with no card drawn and taken down
  first (two refreshes fewer). The page is picked up as the finger leaves it: a slow tap that rolls a little on
  the page is still a tap (it ticks the page, and never moves ticked pages), a page held still opens its card as
  before, and a quick swipe from a page still turns the screen. A drag lifted back in its own place takes its
  black bar off with one small refresh instead of drawing Pages again. The number pad for new pages shows its opening 1 as plain black text at the size of its
  title, with no caret after it (it was a large white 1 in a black box, which read as a 0 on the Scribe).
- Go to page... in Pages (and the `go-to-page` key there) opens the page number or name sheet over Pages,
  which stays behind it, instead of closing Pages first: Cancel, a tap outside it or the key again give Pages
  back as it was, on the same screen with the same pages ticked (with the page previews made while it was open,
  on every page shown), and a row picked opens that page.
- The Graph's view button reads **View: All** or **View: Adjacent** (it said "Around", and "Fit: ..." after a
  pan), on the Kindle too: a tap always switches to the other view, fitted, and the selected page's card says
  **Adjacent** where it said "Around it" (the owner's "View: All and View: Adjacent ... so the wording isn't
  jumbled"). graph.ini still writes `mode=around`.
- The Graph's keys, on a keyboard paired with the Kindle: the arrows pan it (a sixth of the view a press; the
  only place Up and Down do anything), - and + (and Ctrl with them) zoom by 1.25 about its centre, Ctrl+0 fits
  the view again, V switches View: All and View: Adjacent, Ctrl+A selects every page drawn (each drawn
  selected), Enter opens the selected page, and Esc clears the selection, then closes it. New actions
  `graph-view`, `pan-left`, `pan-right`, `pan-up`, `pan-down` (docs/SHORTCUTS.md); a shortcuts.ini that binds
  none of them gets the arrows and V for the Graph in memory, its own lines unchanged.

- Live view shows Grimoire's toasts (the owner, testing on the Scribe with a browser watching: "Enable
  toasts on the live view display. they don't show up now."). Each one comes as the Kindle shows it, in
  the same place with the same look, over the page, the toolbar and the clock corner; it moves when it
  moves on the Kindle (a sheet or the keyboard opening under it, the Kindle turned) and goes when it goes
  there (its time up, a dismiss, another toast in its place, the Kindle asleep). It always shows, with no
  setting, and the toolbar and clock switches leave it alone: it comes with transparency, so with the
  toolbar or the clock switched off its round ends show the page there, never bits of what is hidden.
  Nothing is sent while the pairing card is up or the Kindle sleeps, as before, and a toast from then
  never shows afterwards unless the Kindle still shows it. The toast is a third layer of the stream, kept
  on the Kindle in a buffer of about 1.1 MB at most (`live.rs`); a browser page from before it shows no
  toast and works as before.
- Live view's picture in picture no longer costs more the larger its window. In a large PiP window the
  pen showed 15 to 23 ms later than on the page (the Latency readout's pen age at paint): every frame
  redrew the whole picture, up to the Kindle's 4.6 million pixels, the capture copied them all, and with
  the tab shown it did so about 120 times a second against the page's 60. Now a frame draws only what
  changed, into a picture kept in memory (no copy back from the GPU), and animation frames pace it as
  they pace the page (the background timer stands in only when they stop). The picture is the same,
  pixel for pixel; in headless Firefox Nightly PiP's pen age is within 2 ms of the page's at every size.
- Live view's picture in picture keeps up in Firefox Nightly. With the Live view tab hidden behind
  the floating picture (or behind another tab), Nightly 161 runs a hidden tab's timers once a second,
  so the floating video changed once a second and the ink came up to a second late. Its frames are
  now drawn as the Kindle's data arrives: in headless Nightly a batch of ink is handed to the video 1 ms
  after it arrives at the median and 2 ms at p90 (it was 0.45 s and 0.9 s), and the video changes 59
  times a second while you write instead of once. Visible tabs, other browsers and the pop-out work
  as before (Chrome's hidden PiP gets the same quicker frames).
- New page in the Pages view adds pages in a batch: tap + for one page at once; hold it (or, with a
  keyboard attached, tap it with Shift held) for a number pad that asks how many, with - and + under the
  digits and an "Add N pages" button. The pages are added in one go where New page puts one, and the bottom
  row's Undo takes the batch back while those pages are still blank. Taps made while a large batch is written
  are ignored. A big batch (from 51 pages, about 2 seconds or more on a Kindle) says "Adding N pages..." until
  its pages are added, and so does its Redo; a smaller one adds them with nothing said. A tap on + no longer opens
  the new page: Pages stays open, the page's tile says "(new)", and Undo takes it back as a batch of one. New
  pages take the paper of the page they go after: the last page ticked, else the open page.
- The toolbar's + (its next-page arrow on the last page) held, or tapped with a keyboard's modifier key held, opens the
  same number pad over the page: the pages go after the page you are on, on its paper, which stays open, and the
  Undo in Pages takes them back. A tap on it still adds one
  page and opens it, as swiping past the last page does. The new `new-pages` action opens the pad from a key
  or a controller button.
- Seyes rule paper is true to size: it is drawn at the panel's own density (302.9 dpi on the Scribe 2,
  from the display driver's size) instead of the round 300, so a line pitch measures 2 mm and a square
  8 mm, as in a French school notebook. Pages already on Seyes get the corrected lines too (up to about
  2 mm lower at the bottom of the page). Its lines are darker and heavier, so they read sharp on the Scribe's
  screen: the faint lines and the 8 mm verticals 2 px (the ruled papers' weight), the strong lines and the
  margin 3 px, each in one of the panel's 16 grays so none shows as its neighbour.
- Lasso select, copy, cut, paste and move. Select (in the tool picker, or the `select` action on a key,
  controller button, gesture or pen button) turns the pen into a dashed lasso; what it goes around is
  selected, with the erasing that belongs to it and its links. A bar under it has Copy, Cut, Delete, Paste
  and Done; drag inside its box to move it (only the outline moves until you lift, then it is one undo
  step). The clipboard is a file: page turns and other notebooks keep it, and each start of Grimoire clears it.
  While Select is on, the toolbar's tool button shows the lasso. Hold the pen still on empty page to paste
  there. There is no Select button on the panel: hold a finger still on the page (off a link) until a small
  dashed ring shows, about 0.6 s, then draw around ink without lifting to select it, with the same bar; or
  lift it without moving to paste the clipboard there. A finger held on a link still opens its card, and
  swipes, taps and links work as before. A palm, or a thumb resting on the page while the pen is in range,
  never selects or pastes: the pen coming drops the lasso. Bound to a pen button's hold or drag, `select` works while the button is held:
  draw the lasso with the button down; let go with nothing selected and the pen writes again, or keep the
  selection for the bar until you tap outside it. A paste on the page it came from lands 3 mm off only
  while the originals are still there, so Cut and Paste puts it back where it was.
- The lasso shows what it selects while you draw it, not only when you lift: ink the lasso would take if
  you lifted now gets a dotted halo, which comes and goes as the lasso goes round (pen or finger
  alike), and what you lift with is exactly what the halo showed. The halo stays on the selection after
  the lift. It refreshes a few small areas at most about eight times a second, and the lift redraws them
  all in grays once; on the 1,090-stroke stress page it costs well under a millisecond a step on a Mac.
- Exact-area lasso ("pixel lassoing"): the bar has a mode button, **Whole strokes** (the default) or
  **Exact area**. In exact area the lasso cuts the strokes it crosses along its line and takes only the
  ink inside it: the halo covers just the inside part while you draw, with a short tick where each cut
  will be, and the selection is the inside parts. A short mark (an i-dot, an accent, under 1.5 mm) is
  never cut: it is taken whole when its middle is inside, as its halo shows. Nothing changes on the page until you move, cut or
  delete; then the parts left behind stay exactly as they were drawn (the cut ends are the pen's own
  round ends), and one undo puts the strokes back whole. Copy leaves the page untouched. Tapping the
  mode button with a selection up judges the same lasso again in the other mode. With a keyboard paired
  with the Kindle: hold Shift (or Alt) as you start a lasso to use the other mode for it, or bind
  `select-exact` to a key. The mode lasts until Grimoire restarts. Pencil, mechanical pencil and charcoal grain is drawn
  afresh on the moved part (it differs slightly in single pixels); a calligraphy stroke whose tilt
  widens or lightens it, and ink carrying a link of 2.0.0-beta.5, are taken whole as before. The pieces
  are ordinary strokes every older version reads.
- Erasing travels with the ink it erased: a word partly erased with the precise eraser or white ink moves
  and pastes with the same gaps, and never lightens the ink it lands on (scoped erase strokes; the eraser
  itself works exactly as before). White ink lassoed alone selects nothing: it moves with the ink it covers.
- A linked item copies with its link: the same target, area and outline, a new link where it is pasted;
  in another notebook its target points back to the first notebook's page. A move takes the link along.
- Links between notebooks, polished. A page's Links bar lists **Linked from**: the pages of this notebook and of other notebooks whose links come to it (tap one to look at it, or to follow it into its notebook). A held page's card in Pages says how many pages link to it. What other notebooks link to is kept in a small cache, `backlinks.ini`, rebuilt a little at a time while nothing is touched, never holding up the page; deleting it loses nothing.
- When the Graph shows one notebook alone (a library with too many linked pages to draw at once), other notebooks' pages that it links to, or that link here, are boxes at its edge, grouped by notebook, with an arrow for each way the links go; a tap opens that page. No dot moves for them.
- The Graph opens on **All notebooks**: every linked page of every notebook, links within and across notebooks its lines, each notebook's pages gathered in an island with the notebook's name over it, as Obsidian's global graph and org-roam-ui draw a library. A tap on another notebook's page shows its notebook in the card; Open goes there, and Back comes back. There is no notebook picker: the Graph is always every notebook's (the owner's ruling; View: All / Around is as before). What every notebook says is kept in a small cache, `graph-index.ini`, filled a little at a time while nothing is touched (the panel says "Reading notebooks" the first time), and the library's layout in `graph-all.ini`; a new notebook joins beside the others without moving them. Deleting either loses nothing. Pages with no link are never drawn there (the owner's ruling); the panel counts them.
- The Graph is a place of its own: two fingers down over the Library (or the key bound to `index-map`, for a keyboard) open the Graph of all notebooks with no notebook open. It has no List or Pages tab; tapping a page twice opens its notebook there, Back (or the Library button) from that notebook comes back to the Graph as it was left, and Back from the Graph returns to the Library. A notebook's or a folder's menu in the Library has **Leave out of the Graph** (and **Show in the Graph**): a notebook left out has no page in the Graph of all notebooks, links to it are not drawn, and a folder's setting covers everything inside it, folders and notebooks moved in later included. Kept in `notebook.ini` (`graph=0`) and `graph-omit.ini`; older versions keep the first and ignore the second.
- The Graph from the Library, the reviewer's fixes (2.0.0-beta.2 graph lane): two fingers spreading over it zoom it as over the editor's Graph (a spread that was not a clean swipe did nothing); a notebook that does not open says so instead of the Graph coming back silently; a library with too many linked pages for every notebook opens the notebooks' browser to pick one instead of nothing; Leave out of the Graph (or a move, or the trash) right after Grimoire starts shows in the first Graph opened; a left-out folder that vanished with its last notebook does not leave the next folder of that name out; and the Links bar's chip reads "Volver al grafo" in Spanish.
- A link to a page or notebook that was deleted shows as such: a dashed chip that says Page gone or Notebook gone, a dashed mark on its page in the Graph (which counts them), and a tap on it (on the page, the bar or the Graph's List) opens the link's card to change where it goes or delete it, instead of a toast and nothing.
- Back out of another notebook returns to the page the link was followed from at the zoom and place it had, and that notebook's own Back goes on where it left off.
- Pages: move pages. Hold a page (or tick pages and use the bottom row) and pick Move to...: type the page they go before, or one past the last page for the end. Ticked pages stay together in their order, the open page stays open, and links, names and book pages follow their pages.
- Pages: a move can be undone. Right after Move to... (or a drag), with nothing ticked, the bottom row's Undo (in Move to...'s place) puts the pages back in the order they had. It stays until the notebook's pages change again (a page added, deleted, copied or moved; a rename keeps it) or the notebook closes. The Undo on the page still undoes only that page's writing.
- Pages: Redo, and the undo and redo keys. Right after the bottom row's Undo of a move or a batch of new pages, Redo (beside it, in Copy to notebook...'s place, nothing ticked) does it again: the pages in the exact order the move left, or the same blank pages back at the same places. One step each, until the notebook's pages change otherwise. While Pages is open, the `undo` and `redo` actions (keys, controller buttons, gestures) are Pages' Undo and Redo; on the page they stay the page's own. The two histories never mix.
- Pages: drag to reorder. Hold a page a moment (300 ms, before its card would open), then move off it without lifting: a black bar shows the gap the page (or the ticked pages, when it is one of them) goes into, and nothing else moves on the screen; lift to drop it there. Hold it over the top bar or the bottom row to turn to the previous or next screen. Lifting it back in its own place, or over the bars, moves nothing. Held still (600 ms), it opens its card, and a move from there drags it too.
- Pages: Copy to notebook... copies a page, or the ticked pages, into another notebook: pick it among your folders, then a page there (the copies go before it) or At the end. The copies keep their ink byte for byte, their picture paper (templates, photos, book pages), their names and their links: a link between two pages copied together joins the two copies, a link to a page left behind leads back into the first notebook, and links to the Index are left out. Pages from a Scribe of the other size are scaled as they open there.
- Pages: each preview shows as soon as it is ready, the light pages first and a dense page last, with an hourglass on the pages still to come (also in Copy to notebook... and Link to...); a screen turned to while previews are made asks for its own at once (made beside the others when the Kindle has the cores for a second background process, else right after them). Previews and dense pages reuse a kept drawing instead of drawing the page again (making previews keeps one only of a dense page, a page file of 512 KB or more, so ordinary pages save nothing extra), and opening a page of 256 KB or more that must be drawn says Loading page... until it shows.
- Bluetooth controllers: the helper in `grimoire/bluetooth` is stopped as Grimoire leaves even when it was started again outside Grimoire or Grimoire lost its record of it (before, such a helper kept running, holding the radio, while the Kindle's own Bluetooth started again beside it). Its mouse cursor overlay goes with it when the helper has to be killed, and turning Bluetooth controller on takes such a helper back instead of calling it someone else's. A helper someone else started elsewhere is still never stopped, and one unpacked in another folder is now recognised while it starts, so Grimoire does not start a second one.
- Fewer leftovers on the screen after moving the toolbar or erasing (issue #11, reported by nebuff). Moving the toolbar ends with one quick flash of just the strip it passed through, which wipes the faint copies of it that the fast refresh used while dragging leaves behind (and a drag that went round no longer leaves the page between its ends speckled). Erasing (either eraser) and moving a selection keep count of how much they wiped with the fast refresh; small erases never flash, but once more than a quarter of the screen has been wiped, the screen flashes once where the erasing was, after the pen leaves, instead of waiting for a page turn or a trip to the Library. A selection dragged round and back also no longer leaves the page it crossed speckled.
- The release checks no longer fail on GitHub's macOS runner: two scenarios timed a press and a finger shape by the real time between their lines, which a loaded machine stretches. The test runner now stamps a scripted touch by the script's own clock, and `make check` runs those scenarios again with every wait 400 ms late.
- Tests and CI only: the scenarios run several at once (tests/scenario_runner.py; `SCEN_JOBS`, longest first by
  tests/scenario-times.txt), and so do the language runs: the scripted checks take about two minutes on the
  maintainer's Mac instead of half an hour. The handwriting replay is four scenarios that run side by side, the
  English Gothic run of the all-screens tour is made once instead of twice, and 23 scenarios that count
  refreshes hold the clock corner still (a minute turning over mid-count failed them). Pull requests' CI is
  split into jobs that run at once (a build, the unit tests' two halves, six scenario shards, the languages, the
  host checks, the package) behind one `gate` check, skips the two clean release builds, and runs only a short
  check for a documentation-only change.
- Fix Live view's picture in picture in Firefox 159 and later (and Nightly): Firefox now has the video picture-in-picture API, so Live view took the path for Chromium and Safari, whose frame requests Firefox does not have (it asks the capture stream, not its track). The floated picture stayed on its first frame and a refusal did nothing. Frames are now asked of whichever has them, and if Firefox will not float the video, it goes in the page for Firefox's own Picture-in-Picture button as before.

## 2.0.0-beta.1 (2026-11-01)

The first public beta of Grimoire 2.0, the same app as 2.0.0-beta.0, which tested the KPM pipeline:
installed with `;kpm install grimoire` from the preview feed, it unpacked, opened from KOReader and kept
every notebook. Everything in 2.0.0-beta.0 and the candidates below.

## 2.0.0-beta.0 (2026-10-31)

The first public beta of Grimoire 2.0, the rewrite in Rust: everything in the candidates below
(2.0.0-rc.1 to rc.12), tested on a Kindle Scribe, plus:

- Link outlines are a grainy dithered band (a 1 px pattern read as a plain gray line on the Kindle).
- A link's lasso snaps with every Smart shapes setting, Corner roundness and Smallest shape too.

## 2.0.0-rc.12 (candidate, folded into 2.0.0-beta.0)

- Folders can move: a folder's menu has Move folder, which takes it, with everything in it, to the top level or into another folder (never into itself or a folder inside it). New folder... works there too, and its notebooks' mirror PDFs follow. Renaming or moving a folder no longer leaves empty mirror directories behind.
- The keyboard has an apostrophe beside the space bar, and a symbols page (the #+= key): ! @ # $ % ^ & * ( ) ? , " : ; _ + = / ~ [ ] < > and the euro, pound and degree signs, with the digits on it too. Letters stay where they were.
- Pages: hold a page for Rename, and Delete moved from the bottom row to the held page's menu (it deletes the ticked pages when the held one is ticked, else that page). The bottom row's arrows are gone: one finger flicked sideways turns the screen, as on the notebook's page, and so do the page-turn keys and controller buttons while Pages is up.
- In the Graph, a pen's tap selects a node as a finger's does; a second tap opens it (a pen's tap often reached the Kindle as two and opened at once).
- Smallest shape now sets the real size floor, 3 to 40 mm (10 by default, as before): lowered, small circles, squares and triangles snap (it could only ever raise a fixed 10 mm floor). With Round shapes: As drawn, long and egg-shaped ovals snap to clean ellipses; with Four sides: Uneven allowed, parallelograms (slanted, turned, thin or steep) and trapeziums snap with their corners where they were drawn.
- Add Corner roundness to the Smart shapes menu: it rounds the corners of snapped rectangles, squares, four-sided shapes and triangles (Sharp by default). The menu's sliders answer where they are first touched, a tap anywhere on one sets it, and a touch cut short by the pen coming near keeps its value.
- Link outlines (the eye's boxes, the area waiting for its target, the lasso and the held link's border) are a slightly thicker dithered band instead of fine dots, so e-ink shows them clearly.
- The stroke eraser is much faster on dense pages: taking a stroke off draws the page again only where that stroke was. On a 1,090-stroke page a marker stroke across the page goes in about 3 seconds instead of 34; sweeps through many strokes are 2.5 to 4 times faster.
- A pen or finger lift the Kindle missed (for example while a dense page kept Grimoire busy) no longer swallows your next stroke or tap, and pressing the pen's button for a drag while drawing a link's lasso no longer leaves the lasso stuck. grimoire.log now says why a finger on the page did nothing, follows each step of making a link, and counts events dropped while Grimoire was busy.
- The tool picker shows Black or White as picked at once (it kept the old choice until opened again).
- The clock and battery corner is medium by default (settings saved before keep their size).
- Live view: a Brightness slider under Warmth dims the picture in this browser only, and the Kindle's warmth no longer moves the Warmth slider while you drag it.

## 2.0.0-rc.11 (candidate, folded into 2.0.0-beta.0)

- Add What's new, under the bat: Grimoire's bat now stands left of its name in the Library (in place of the emblem, and in Classic too), and a tap on either opens a card with a note from the author, signed by hand with the bat beside the signature, Read all for the whole note in larger type, and Don't show this again. The first start after an update opens it by itself once, unless that box is ticked.

## 2.0.0-rc.10 (candidate, folded into 2.0.0-beta.0)

- Undo and redo are a copy of what the step changed, however dense the page: each step keeps the tiles of the page it touched. On a page of 1,090 strokes an undo took 4 to 11 s, until the launcher stopped Grimoire as hung.
- Pages that take a second or more to draw keep their drawing, so they open at once. A very dense page took 36 s to draw on the Kindle, longer than the launcher waits, so its notebook would not open; long drawing now keeps the launcher's heartbeat going too.
- The page being written on no longer gets drawn again in the background for its Pages preview at every pause (on a dense page that kept a core busy for 20 s at a time); its preview comes from the page already on screen.
- Pens draw the same pixels faster on the Kindle (about 14% on a dense page): no float division a pixel for the paper's tooth, opacity shares remembered, round dabs trimmed to their circle.
- Live view's pen plays out smoothly through Wi-Fi stalls: its clock follows the pen exactly on a clean link (a few milliseconds behind), and after a stall it moves on a little quicker for a moment instead of freezing and jumping. Drawn ink no longer steps back for a frame when the delay changes.
- Two-finger gestures, and recording them, worked again on a device or computer up more than 24.9 days (the time since boot passed 2^31 ms and a pen never seen counted as one that had just left).

## 2.0.0-rc.9 (candidate, folded into 2.0.0-beta.0)

- Turn the page with one finger: swipe left for the next page and right for the previous one, as on the Kindle's own books (two fingers still work, and Settings > Gestures can rebind either).
- Take pen taps in the Graph: a dot or Back tapped with the pen is a tap even when the tip slides a millimetre or two, instead of a small pan.
- Arrange the paper pickers in three rows of six: the rules without and then with a margin; Cornell notes, dot grid, graph paper, sheet music, guitar tab and blank; then cross grid, isometric, hexagon, Seyes rule and Custom music. More layouts... (the templates) and, in a notebook, Photos... are buttons beside Cancel.
- Add four papers: cross grid (5 mm reticle), isometric (5 mm triangles), hexagon (4 mm sides) and Seyes rule. Older versions show pages on them as blank "Newer paper", keeping their ink.
- Custom music gains an 8-line tab part and a Start from row (grand staff and 4 to 7 string bass tabs, which leave the main grid), and new layouts start as treble, bass and 6-line tab.
- Add a Smallest shape slider to the Smart shapes menu: a held stroke smaller than it stays as drawn (Any by default, as before).
- Keep the toolbar in Live view while the tool picker (or its Pin mode, or the Smart shapes menu) is open beside it, as on the Kindle.
- Keep Live view's picture in picture playing on a still page, so Firefox offers its own Picture-in-Picture button and other browsers never show a black window.

## 2.0.0-rc.8 (candidate, folded into 2.0.0-beta.0)

- Answer every finger tap on buttons, sheets, the keyboard, the Links bar and the toolbar at once, even while the pen hovers or has just lifted. The pen's palm window now applies only over the notebook page, and a finger landing well away from the pen counts there too.
- Keep a slider's value when its touch is cancelled (the pen arriving or a second finger). Brightness, warmth, Sleep after and the pen feel sliders no longer jump to their minimum.
- Let the on-screen keyboard take a key pressed while the previous finger is still lifting, count a slightly rolled tap that stays on its button, and answer slow presses on the Links bar.
- Show a pressed look at once on Display, Paper, Pages and the tool picker, and share the toolbar's column between its buttons so a finger between two takes the nearest.
- Let the toolbar be dragged right up to the screen's top edge (and bottom).
- Start every link lasso fresh, so a "Too small" try with a snapped shape can no longer break the next link.
- Save pages without rebuilding their Pages preview on the spot, repaint only the changed tiles in Pages, and share decoded previews with the link destination picker.
- Let a Wi-Fi change finish when Grimoire exits, and keep a Turn on/off tap queued behind a failed status check.
- Picture in picture really floats: the browser's own always-on-top window where it allows one (see the page's tip for making the Kindle's address count as secure), else a video of the page that includes the quick ink and only draws when something changes, else a pop-out window. While the picture is away the page's own view stops drawing.
- Choose the Live view size: Fit, Fit width or 25 to 200 percent, with Ctrl or Cmd and the scroll wheel, a trackpad pinch, or the + - and 0 keys, remembered in the browser.
- Send only the Live view tiles that changed, drop tiles a newer picture has replaced, and keep the pen stream ahead of tiles on a busy Wi-Fi link.
- Make the Pages action open Pages, as the toolbar page count does. Keys, controller buttons, pen gestures and finger gestures you bound to Pages now open Pages instead of the Graph. The Buttons and Gestures sheets list the Graph once, as Graph (`index-map`); a saved `index` binding still opens the Graph. Two fingers down still open the Graph by default.
- Open the same Go to page sheet from the Links bar as from the go-to-page action and Pages: one keyboard over the notebook that takes a page number or part of a page name.
- Drop the numbered discs from Links bar rows, since link areas no longer carry numbers; each row starts with its name.
- Keep the graph's shape when links and pages change: a new link moves only its two pages and their neighbours, a few new pages are placed beside theirs, and a larger change is laid out again from the current positions. Naming an unlinked page or adding plain pages moves nothing. Unlinked pages no longer take hidden room, so maps are tighter with larger dots.
- View now switches between All and Around. All always fits the whole linked graph; after a pan or pinch the button reads Fit and first fits the current view again. Background layouts are fitted when they finish unless the view was moved.
- Lay out graphs faster by stopping once their shape settles, write graph.ini only when it changes, and draw labels and pinch previews with less work. Links to the same page of another notebook count once in a page's card.
- Rework Live view's browser controls into a slim rail beside the page, with View settings and Keys opening beside it; it fades only where it could cover the page. Add a Size control: Fit, Fit width and 25% to 200%, with Ctrl/Cmd+scroll, trackpad pinch and the - + 0 keys, remembered, area-averaged at every size up to 100% and pixelated above. Make Picture in picture float where the browser can: Document Picture-in-Picture moves only the page and draws it at that window's pixels, video PiP (Safari, Chromium and Firefox on plain HTTP) carries the quick ink and pen dot and sends frames only on change, else a pop-out; the main tab draws nothing meanwhile. View settings explains how to let Firefox and Chromium treat the Kindle's address as secure. Compose once per animation frame, skip work while hidden, avoid GPU readbacks and coalesce resizes.

## 2.0.0-rc.7 (candidate, folded into 2.0.0-beta.0)

- Preserve final touch movement delivered with finger lift-off, widen the second-finger landing window and accept deliberate two-finger swipes up to 2.5 seconds. Keep pen, palm, pinch and pan guards.
- Show link areas as fine dotted outlines without number badges. Preserve snapped circles, ellipses, triangles and quadrilaterals through saving, copying, redraw and undo; taps follow their actual shapes.
- Inherit the current page's layout for linked New page and Pages > New. Fresh notebooks default to Narrow, no margin.
- Shorten the Library exit button to KOReader and give it the same width as neighboring buttons. Fit notebook/folder names in smaller text on up to two lines.
- Set fresh/reset fountain-pen and highlighter defaults to the owner's captured settings: fountain 0.25–1 mm, fully opaque; highlighter 5 mm, 25–75% opacity. Preserve saved preferences and existing ink.
- Keep KOReader's native font fallback for battery status and reading footers in the optional Terminess patch, preventing unrelated Nerd Font glyphs and faded charging icons. Allow explicit upgrades of the exact RC6 patch; preserve locally modified copies.

## 2.0.0-rc.6 (candidate, folded into 2.0.0-beta.0)

- Open Pages directly from the toolbar page count. Add compact New/Copy/Delete icons, an explicit Multi-select mode, page borders and a distinct inner selection border.
- Preload nearby page previews while idle, reuse decoded previews in Pages, and preserve valid disk previews across lossless background ink compaction.
- Keep Name page and the swipe-up menu's numeric Go to page dialog over the notebook. Fix the Smart shapes heading font in Classic and preview clock size/corner changes live in Display; the default remains enabled, small and top left.
- Remove the pen-proximity swipe notification and redundant KOReader Wi-Fi startup/manual controls from the Grimoire plugin. Use KOReader's network menu for connections.
- Replace video Picture in picture with a real pop-out that moves the existing Live canvases and controls into one window, preserving the same streams and watcher. Closing the pop-out returns them to the original tab.
- Reset stale pen, touch and gesture state after every wake, including wallpaper sleep and a missed sleep announcement; finish interrupted ink before accepting fresh input.
- Close Live view's raw pen reader while asleep and reopen it after wake, even when its device path is unchanged. Fall back to the main input stream on device hangup/EOF. Explicitly leave hardware handwriting mode at sleep and reassert normal display mode at startup and wake.
- Reclaim ended Live viewers before accepting reconnects and bound stale Wi-Fi sockets with TCP keepalive and an acknowledgement timeout on Kindle.

## 2.0.0-rc.5 (candidate, folded into 2.0.0-beta.0)

- Show only linked pages in the graph, including when the current or a named page has no links. Remove Unlinked and its tiny tray; exclude deleted cross-notebook destinations from the graph while retaining saved links for repair.
- Replace graph footer controls with a compact panel: View cycles All / Around / Fit, with node, orphan and link counts plus current-notebook and library-wide page totals. Remove zoom and Tidy up buttons; retain pinch zoom and automatically lay out changed graphs.
- Make Pages a 3×3 grid of full-page cached previews without empty framed containers or checkboxes. Tap selects, double tap opens, and holding a tile offers Open or Add a link from that page.
- Stop retrying unavailable page previews after every redraw; retry on reopening or explicit cache invalidation. Keep background preview work bounded.
- Update Areas immediately on both the current page and a linked-page preview without committing the preview. Clear the full notebook picker after link creation, including failed saves.
- Move Pen feel into the toolbar's drawing-tools group. Open main Settings and its nested sheets over the notebook, preserving the current page instead of showing the Library underneath.

## 2.0.0-rc.4 (candidate, folded into 2.0.0-beta.0)

- Add Wi-Fi controls to Share and Live view, with separate radio/connection status and On/Off buttons. Connect to saved networks without returning to the Kindle home screen.
- Bound the standalone Wi-Fi helper, keep it outside the drawing loop, verify radio state after changes, and leave saved networks and credentials untouched.
- The KOReader Grimoire plugin turns on Wi-Fi once when KOReader starts, using KOReader's own Kindle backend. Tools > Grimoire offers manual Wi-Fi control and a startup opt-out. Manual off stays off for the rest of that KOReader process.

## 2.0.0-rc.3 (candidate, folded into 2.0.0-beta.0)

- Unify the Index under Back, Graph, List and Pages. Move graph view options to the bottom; show existing links in List without redundant footer controls.
- Replace the upward panel's Add link and Graph buttons with page status and naming; add a main Settings cog in the notebook toolbar.
- Replace new Index link destinations with Somewhere Else: browse folders and choose a page in another notebook. Stable notebook/page IDs survive renames and moves; Graph and List separate notebook destinations.
- Cache small page previews for page management and link destinations; generate missing visible previews in bounded background jobs, with stale-cache checks.

## 2.0.0-rc.2 (candidate, folded into 2.0.0-beta.0)

- Replace the notebook status bar with a small enabled top-left clock corner; remove the Display mode selector.
- The toolbar link-plus button creates a link directly; swipe up with two fingers for the Links bar.
- Limit pinned pens to three. Smart Shapes has its own menu for On/Off, hold speed, four-sided and round-shape preferences, and ink-free link-area snapping (enabled by default).
- Clear page removes its link areas as well as ink, with one combined Undo/Redo, including otherwise blank pages. Failed link saves preserve the page. Clear gestures are ignored while making a link.
- Pen-button Clear page defaults to hold only. Recording still recognizes multiple clicks when no multi-click action is bound.
- KOReader startup uses the orientation sensor after restoring book/browser settings, including saved rotation 0, just as returning from Grimoire does. Accelerometer ignore and lock settings remain honored.

## 2.0.0-rc.1 (candidate, folded into 2.0.0-beta.0)

- **Links without ink:** draw around an area to select it, then choose a page in the same notebook
  or The Index. The selection disappears; a visible border is ordinary ink drawn separately.
  Hold a link to rename its destination, change its target, redraw its area or remove it. Link
  changes have undo/redo. Existing beta.5 lasso strokes remain ordinary ink, including after
  deleting their link; the page file format is unchanged.
- **Record your gestures:** Settings > Gestures assigns touch patterns and recorded finger shapes
  to actions. Recording checks consistency and shows a conflict before replacing a binding.
  Three-finger patterns appear only on panels that support three contacts. Real-device tuning
  and false-trigger measurements remain part of candidate acceptance.
- **Smart shapes:** hold the pen still at the end of a line, circle, ellipse, triangle or box to
  preview a clean shape. Lift to keep it, or move on to continue the original stroke. Recognition
  starts enabled, has a toolbar switch and hold-time choices, and supports shaping the last
  stroke with one undoable replacement. Erasing, white ink and link selection are excluded.
- **The Index map:** the Links bar's Index button and the list's Graph button show the notebook's
  linked pages, named sections and unlinked pages. Select a dot for its page card; pan, pinch,
  switch All/Around, rename a page or open it. Large layouts run in the background. Incoming-link
  counts appear in the list and a Links-bar chip opens Around for the current page. Links to
  The Index retain their named-page-list meaning.
- **Pen-button gestures:** assign clicks, double/triple clicks, holds and button-held drags.
  Pan and zoom drags leave no ink; momentary erasing starts on button press. Triple click and
  hold still clear the page by default; double click has no built-in assignment. A second
  button is offered only when the pen reports it.
- **Nested notebook folders:** create subfolders inside folders, move notebooks to nested
  destinations and rename a whole folder subtree. Back goes up one level. Folder ZIP exports
  include descendants and retain their hierarchy.
- **Toolbar beside a landscape page:** with Whole page sideways enabled, rotating places the
  toolbar just outside the page on its chosen side with a small gap. Flipping between landscape
  orientations retains that placement. The width-filling setting keeps its existing behavior.

- **Keyboard shortcuts in Live view:** with the Live view page focused in a browser, that
  computer's keys run Grimoire's shortcut actions on the Kindle. They come from their own file,
  `live-keys.ini`, written from the built-in keyboard mappings the first time Live view starts; the
  page's Keys button lists them. Only a browser paired for Live view can send keys.
- **Buttons sheet:** "Swiping past the last page adds a page" and the status display (bar, corner
  or off) can be given a button, in the Pages and Menus groups.
- **The touch log keeps going on a long day:** its 2000-line limit now applies per minute, not per
  run, so touches late in the day still leave their lines in `grimoire.log`.
- **Notebook status bar:** the new default keeps the notebook, page and section, link counts, tool,
  clock, battery, screen sharing and controller visible above the page. Upright and sideways, the
  page fits below it. Display lets you select Bar, Corner or Off, also available as `status-mode`.
- **Live view after a long pen silence:** quick ink and the pen dot no longer freeze once the pen
  has been away for over 35 minutes.
- **KOReader comes back turned as you hold the Kindle:** leaving Grimoire for KOReader no longer
  lands in KOReader's default orientation until you pick one by hand. Back from Grimoire, the
  KOReader plugin reads the orientation sensor once and turns KOReader to match (upside down,
  upright or sideways), honouring KOReader's own accelerometer settings: nothing when it ignores
  the accelerometer, and only a flip within portrait or landscape when its orientation lock is on.
  Taps still land where they are drawn (update the plugin: Library > Settings > KOReader > Install
  / update Grimoire plugin).

## 2.0.0-beta.5 (2026-10-28)

The first release written in Rust. It is what is new since 2.0.0-beta.3: 2.0.0-beta.4 was a
performance candidate that was never published, so its changes (the last section below) are in this
release too.

- **Grimoire is now written in Rust**: the whole app has been rewritten, screen for screen and file
  for file, from the C it was written in until now. Using it does not change: the same screens,
  pens, papers, shortcuts and settings. The port itself changes no file: your notebooks, templates,
  settings and everything else under `grimoire/` are read and written as before, so there is nothing
  to convert (the new features below add a few files of their own, each described in its entry).
  The point is the versions to come: Rust makes a whole family of crashes (reading or writing past
  the end of a buffer, using memory that was already freed) much harder to write, in everything
  except the small C parts the app still links and that are unchanged: the screen library FBInk,
  the font and image code and the QR code. The maintainer compared it with the C version on a
  Kindle Scribe (2024) before this release. If something looks or behaves differently from
  2.0.0-beta.3 and is not in this list, it is a mistake in the port: please report it.

- **The Index from a button or a key**: five new actions for Buttons... (a new Index group, and
  Whole page sideways in Pages) and for shortcuts.ini: Links on this page opens and closes the Links
  bar; Go to page (Find: a keyboard over the page that takes a page number or part of a page's name,
  accents and capitals ignored, so "arb" finds "Árbol") opens and closes the same keyboard as the
  bar's Go to; Back to the page before returns to the page a link, the Pages view or Find left (up
  to 32, newest first, forgotten when the notebook closes; a toast says when there is none); Show
  link areas is the bar's eye, which draws the dotted, numbered boxes where each link's ink is, on
  and off, remembered; and Whole page sideways is the Display row of the same name.
  Removing or changing a link now redraws the boxes at once, so a deleted link's box no longer
  stays on the page.

- **Draw while looking at your computer**: Live view now sends pen movement on its own connection,
  so the browser can show a quick pressure-sensitive line while the exact page image catches up.
  Menu taps leave no preview ink. Monitor mode (M) fits the page and tucks the controls away; its
  choice is remembered in that browser. An optional Latency readout shows estimated pen and image
  delays. Textured nibs and erasing still settle to Grimoire's exact rendering.

- **Live view, smoother and easier to read**: the pen's line and its dot in the browser no longer
  skip. The browser used to get the pen's movement only when Grimoire's main loop read it, so a heavy
  tool such as a textured brush sent it in bursts; now each browser reads the Kindle's pen sensor
  itself the moment it has a point, and plays the points back at their own times (the Latency
  readout has a Playout delay). The dot has five colours and a colour picker, and three sizes
  (small is the new default, smaller than before; medium is the old size), remembered in that
  browser. The toolbar and the clock and battery corner are layers over the page, each with a
  switch (on by default, remembered). Picture in picture draws at its own window's pixels, so ruled
  lines and thin strokes keep their weight there too. Recording makes an MP4 in Firefox now, where
  its encoder can, and the button says Record (MP4) or Record (WebM) so you know which you get.
  Live view's words are in English and Spanish. A menu or a keyboard on the Kindle takes the
  panel out of the browser's copy while it is open, and the page under the panel arrives in the
  same push as the rest of the screen. At actual size, full screen and Monitor mode scroll to the
  page's far edges. After the pen had been away for over 35 minutes, the browser's dot froze and the
  quick line stopped until 71 minutes had passed: a long silence now starts them again.

- **A Kindle reboot while writing, fixed**: with the developer switch `ink-metrics` on, Grimoire
  followed some ink refreshes through the Kindle's display driver, and a race in the driver could
  then crash the Kindle, which rebooted itself. Grimoire no longer asks the driver for those
  timings. Going to sleep and closing Grimoire could also wait 5 seconds longer than needed for
  the last refresh; they no longer do.
- **No more skips while writing with shortcuts or a controller on**: with Settings > Controller's
  shortcuts on, every 5 seconds of unbroken writing a line could skip: it jumped ahead, then drew
  a short piece backwards, and a stretch of it was lost. Grimoire's check for keyboards and
  controllers that connect held everything up for about a third of a second, long enough for the
  Kindle to throw pen movement away. The check now waits until the pen has left the screen, and
  no longer opens the pen, touch and other devices Grimoire already uses, so it takes next to no
  time. If the Kindle ever does drop pen movement, the line now goes on straight from where it was,
  without the backwards spur; a stroke that ended in the dropped moment (the pen lifted, or turned
  to its eraser end) is not joined to the next by a stray line; and saved pages keep each point's
  real time. grimoire.log says how long any hold-up took.
- **The pen's pressure and tilt kept through slow moments**: turning to a page with a lot of
  writing, opening Paper for the first time in a notebook, going back to the Library, opening the
  Display sheet and dragging its light sliders held Grimoire up for a tenth of a second to more than
  a second, long enough for the Kindle to throw away the pen's pressure and tilt while it hovered
  (26 times in one session), and a stroke begun in such a moment could lose its start. Grimoire now
  keeps reading the pen while it draws a page, a paper, a thumbnail or the screen, and while it
  saves. The front light's levels are read once when Grimoire starts instead of at every Display
  sheet, and a light slider redraws only its own row while it moves. Each point of a stroke also
  takes the pressure and tilt of its own moment, where a stretch read at once used to share one.
  grimoire.log names what held it up.
- **The Index's keyboard types as fast as the Library's**: naming a page (and Go to page's number
  pad) lagged further behind with every key. Each key refreshed most of the keyboard at once, so
  the next key waited on the screen for it, and the pen resting on a key drew the whole keyboard
  again many times over. Now only the text and the key refresh, the keyboard is drawn once a key,
  and a moment after typing stops the keys are cleaned up, exactly as on the Library's keyboard.
  Naming a page right after writing on it no longer leaves the toolbar drawn over the keyboard,
  and a notice stays above the keys (Go to page's "Enter a page number" covered its 0 and del).

- **A small clock and battery on the notebook page**: tap a corner of the page with a finger (the top
  left, or the corner you pick in Settings > Display) and a small box shows the time, a battery drawn
  in pixel art that shows the charge, with its percent beside it, and a bolt over the battery while it
  charges; tap it again to hide it. It shows on the notebook page and nowhere else: not on the
  Library, Settings or any other screen, and not while a sheet, a menu, the Links bar, Find or the
  Pages view is in front of the page. Settings > Display, beside Done, has its size (small, medium
  or large; the battery grows with it) and its corner (top left, top right, bottom right, bottom
  left): the tap zone follows it, a 14 mm square and the box's own place with 4 mm around it, shown or
  not (half the taps to bring it back went where it had been, outside the square). One tap is one toggle, at once: a fingertip that
  rolls inside the corner still taps, slow presses too (up to 0.8 second), nothing makes the next tap wait, and the box is refreshed as the
  finger lifts, even when a pen left the screen without a word. A key or a controller button can turn
  it on and off or move it to the next corner (Buttons... > Menus > Clock and battery, and Clock and
  battery corner). It changes each minute and when the battery or its charging changes, with one
  small refresh of its own, and holds back while the pen is near or you are writing, so it never
  touches a stroke. It is only ever on the screen: never in a page, a PDF or a picture, Live view
  or the sleep screen. Older versions leave the three new lines in settings.ini alone, and count
  `status-toggle` and `status-corner` among the shortcuts they do not know.

- **Live view's icon beside the panel**: while a computer watches with Live view, the small icon that
  says so is no longer in the top right corner of a notebook page, where it covered the page: it sits
  beside the panel, level with its header, and moves with it when you drag or fold the panel. On the
  other screens it stays in the top right corner.

- **Menus over the panel**: the panel is only ever over the notebook page. The Links bar, Find and its
  keyboard, Display, Paper, the tool picker and the Pages view are all drawn over it, however the page
  or the panel is redrawn, so a button of the panel can no longer show through a menu or a keyboard.
  In Live view the browser's copy of the panel goes while one of them is open.

- **Two-finger gestures and the Links bar**: on a notebook page, two fingers down open Pages and two
  fingers up open the Links bar, left turns to the next page and right to the previous one (a page
  turned left on the last one adds a page, as the next-page key does; a row in Display, under
  Sideways, turns that off). The bar is a card along the bottom with one row for each page
  the open page's links go to, numbered as the page's dotted link areas are (Areas in the bar, or
  Buttons... > Index, show them over the page, never in a PDF): tap a row to look at that page, tap
  the page to stay on it, or X to go back where you were. Go to opens a keyboard that takes a page
  number or part of a page's name from the Index, and Back to the page before returns. The panel has
  a Links button after Next. The pen always wins: with it near the screen or just lifted, a gesture
  does not fire, and a resting palm fires nothing. Sideways now shows the whole page (Display >
  "Sideways, show the whole page" turns it off), so two-finger swipes work there too. A new
  gestures.ini beside shortcuts.ini changes what each gesture does (docs/SHORTCUTS.md). After the
  update, the first notebook you open and the first turn sideways say what changed, once. Older
  versions keep the four new settings.ini lines and never read gestures.ini.

- **Updates from Grimoire**: Library > Settings > Updates checks Grimoire's feed through KPM (betas
  and releases on a beta, releases only on a release; a tap changes it) and installs a newer one:
  Grimoire closes, KPM installs it with its progress on the screen, and Grimoire starts again. A
  failed install is reported at the next start. Hotfixes go out the same way: docs/KPM.md,
  Hotfixes. Settings' bottom row now has Controller and Updates side by side.

- **Controllers**: Settings > Controller (was Shortcuts) shows the controller or keyboard connected,
  and Buttons... learns which controller button does what: tap a row, press the button
  (grimoire/controller.ini). A short toast says when one connects or goes, and when another app
  holds it so Grimoire gets nothing. The D-pad (or a hat, as an arcade stick's lever) turns pages
  and tools by default, also with a shortcuts.ini from an earlier version, which binds no D-pad (the
  file is not changed). Buttons... names a Switch-mode controller's buttons as printed on it (B, A,
  Y, X, L, R, ZL, ZR, -, +) instead of by number or the Linux names. Next page on the last page adds
  a page, as the editor's + button does (it used to stop there).
- **More for a controller's buttons**: every tool on its own, Erase while held (the pen erases
  while the button is down), the eraser's mode, clear page, first, last and new page, fit page,
  and the editor's menus: the tool picker, Paper, Pages, Index, Display, Pen feel, fold the panel,
  the Library and Close menu. A menu's button opens it and closes it again, or swaps the menu in
  front for its own. Buttons... groups them under Editing, Tools, Pages and Menus, and a toast says
  when a button learned there leaves another action. Older versions refuse the new names in
  shortcuts.ini and count them; their other lines still work.
- **Next and previous tool stay on the panel**: they now go through the pens on the panel only, the
  tool button's pen and the pinned ones, top to bottom and round again, so pressing next over and
  over never sends the highlight into the tool picker and back. With nothing pinned they go through
  the tool picker's nine pens in its order; with only the tool button's own pen pinned, the first
  press brings the picker's next pen onto the tool button, and from then on they go between those
  two. From the eraser they go on from the pen it goes back to, instead of starting again at the top
  or the bottom. A new shortcuts.ini numbers keys 1 to 9 in the picker's order too (3 charcoal, 4
  ballpoint, 7 fountain pen); an existing one is never changed.
- **Next and previous in the tool picker**: two new actions for a button or a key (picker-previous
  and picker-next, a row of their own in Buttons... under Tools) move the open tool picker's choice
  and leave it open; with it closed they switch tools through all nine pens. Next and previous tool
  leave the open picker alone.
- **Links and page names from a button**: two new actions for a button or a key (add-link and
  name-page, rows of their own in Buttons... under Pages). Add a link starts a link on the open page
  in one press, without going through the Pages view and its Index: draw around what links, then
  tap where it goes. Pressed again before you draw, the link is not made. Name the page opens the
  keyboard for the open page's name and goes back to the page after Done or Cancel.
- **Bluetooth controllers**: with the kindle-hid-passthrough helper unpacked in grimoire/bluetooth
  (only its files: its installer adds a boot-time button mapper that takes controllers away),
  Bluetooth controller starts it with Grimoire and Pair a controller... finds and pairs one. It runs
  only while Grimoire does: it stops when Grimoire leaves, and the Kindle's own Bluetooth comes back.
- **LT and RT on Xbox-layout controllers**: on 8BitDo's Xbox mode (and other controllers whose
  triggers are analog) LT and RT did nothing, because Linux reads them as axes, not buttons.
  Grimoire now reads each as a button: pressed past half its travel, let go near its rest, once each
  way however the hand wavers. Buttons... learns them and names them LT and RT. A right stick is
  never taken for one.
- **Hold a row in Buttons... to clear it**: holding a row for a moment takes its buttons away, that
  row's alone; it reads none and a toast says so. Reset buttons still clears every row.

- **Highlights keep the tip they were drawn with**
  ([#5](https://github.com/ganten1998/grimoire-scribe/issues/5)): each stroke now records its
  tip, so highlights drawn with the round tip before 1.2.0 look as they did again, in the editor
  and in exports, and new highlights keep the vertical tip. Highlights drawn with the vertical tip
  in 1.2.0 to 2.0.0-beta.4 recorded no tip and show round too. Later tip changes will never restyle
  old pages: docs/FORMAT.md, Tip variants.
- **Photo pages**: copy JPG or PNG photos into `grimoire/photos` over USB, then in a notebook pick
  Paper > Photos...: the photo becomes the page's paper, fitted to the page and turned upright the
  way your phone or camera took it, ready to write on. A page added after it gets the notebook's
  own paper.
- **KOReader no longer hangs after Grimoire with Keep alive on**: KOReader came back with the Kindle
  still kept awake but its Keep alive shown off, and five idle minutes later its auto-suspend timer
  spun without end, so KOReader stopped answering. The plugin now turns Keep alive back on when
  KOReader starts on a Kindle that is kept awake (update the plugin: Library > Settings >
  KOReader > Install / update Grimoire plugin).
- **KOReader no longer comes back upside down**: with the Kindle turned while in Grimoire, KOReader
  could come back drawn upside down with every tap landing on the opposite side, its menu out of
  reach. The plugin now sets KOReader's rotation right as it starts on a Kindle Scribe.
- **Open a whole book in Grimoire**: in a PDF, DjVu or comic book in KOReader, Tools > Grimoire >
  Open this book in Grimoire, or Open with... > Grimoire in KOReader's file browser, puts every page
  into the book's notebook at once (KOReader draws them first, with a progress message a tap stops)
  and opens the page you were reading. Pages already in the notebook keep what you wrote on them;
  Annotate this page uses the same notebook. Needs the KOReader plugin from this release.
- **Links between pages**: in the Pages view's Index tab, Add a link, draw around what links, and
  tap the page it goes to (or The Index). A finger's tap on the ink you drew follows it; holding a
  finger on it changes where it goes or removes it. The links are your own ink, so erasing or
  undoing it takes a link away, and older versions show the ink. They are kept in the notebook's
  `links.ini`, apart from the pages: docs/FORMAT.md, Links.
- **Live view**: Library > Share > Live view mirrors Grimoire's screen in a computer's browser on
  the same Wi-Fi, as it changes: open the address on the card and type its one-use code, or scan the
  QR code with a phone. Fit or actual size, full screen, transparent paper and recording in the
  browser (an MP4 in Chrome, Edge, Safari and Firefox where the browser can encode one, and WebM
  where it cannot and for transparent paper, which an MP4 cannot keep clear), the pen shown where it is (a see-through dot
  while it hovers, solid while it touches; Show pen turns it off), an Invert colors switch and a
  Warmth slider: the picture follows Invert colors (the whole page goes black) and the front light's
  warmth (its light turns amber) on the Kindle until you change them, and As on the Kindle goes back
  to following. Picture in picture floats it over your other windows, still live while you work
  elsewhere. Ruled lines and thin strokes keep an even weight at any window size (the browser's own
  scaling made some vanish). The pen never waits for it (each browser gets its own background
  process that sends only what changed), the code never shows on the stream, downloads still need
  Share open, and it turns off with Stop, after 10 minutes with nobody watching, or when Grimoire
  closes.
- **Dark share pages with Grimoire's bat**: the pages a browser opens from Share and Live view are
  dark (your pages, thumbnails and Live view's screen show as the Kindle draws them), with the
  pixel bat by the title and as the browser tab's and the phone's icon.
- **The notebook PDF has a table of contents and links**: named sections become the PDF's outline
  (the bookmarks a PDF reader lists), and links drawn to pages are links in the PDF.

## 2.0.0-beta.4 (candidate, folded into 2.0.0-beta.5)

- **Live ink on gray pages goes straight to black**: on a gray PDF page or template, most
  pixels under a new stroke used to flash white first and turn black a whole display waveform
  later, a white halo and extra lag that white pages never had. Live frames now only move a
  pixel toward its new value, with the same average darkness. Black-and-white pages are
  refreshed exactly as before.
- **PDF download progress**: Share shows completed pages while preparing a notebook PDF,
  then the transfer percentage. Files still go through the browser's normal download
  handling. Interrupted or failed downloads show a message instead of a stuck indicator.
- **Less memory while sharing**: PDF, ZIP and image downloads send from their existing
  export buffer, without building a second full copy for the HTTP response.
- **Keep ink moving during a backlog**: input batches yield to the display and timers after
  8 ms or 64 events. Queued pen samples stay in order; none are discarded by this limit.
- **Less brush work**: skip pixels already darker than a pen's ink, reuse the opacity of
  smooth dab interiors, and skip unused tilt calculations. Pen settings and rendered ink
  are unchanged. Measured CPU improvements and their limits are in docs/INK-PERFORMANCE.md.
- **Faster ink with the driver's pen mode**: live ink now goes out in the Kindle display
  driver's pen mode, which starts a frame on the panel within about 1 ms. Plain refreshes that
  overlap ink still being drawn waited in the driver for up to 300 ms, so quick circles and note
  heads fell behind the pen. A stroke takes pen mode only when no other refresh can still be on
  the panel, as the driver requires. A `pen-mode` file with `off` turns it off.
- **Ballpoint follows the nib closer**: its default smoothing is 0.50 instead of 1.00, which
  made the line trail the pen by 6 to 13 ms at writing speed (now about 2). A ballpoint still at
  the old default moves to the new one; a smoothing you set yourself stays.
- **Grays settle when the pen leaves**: the grayscale cleanup of fresh ink waits until the pen
  leaves the page (or 1.5 s with it hovering), so the next word no longer runs into it.
- **Developer latency report**: optional timing logs measure framebuffer copying and refresh
  submission, and now each live ink frame from the pen's sample to the refresh ioctl, without
  input coordinates. docs/INK-PERFORMANCE.md also lists the panel's own
  waveform timing: on gray paper DU first drives a pixel toward white, so ink there starts up to
  59 ms later than on white paper whatever Grimoire does.

## 2.0.0-beta.3 (2026-10-26)

- **Empty folders leave the PDF mirror too**: removing or renaming an empty Library folder
  also removes its empty directory under grimoire/PDFs. Directories with files, symlinks, and
  directories another logical folder still uses are kept.

- **Retuned pen defaults**: use the latest settings tuned on the Kindle for new installs and
  Pen feel > Reset tool. Existing saved profiles and old stroke parameters are kept.
- **Handwritten eraser practice**: the eraser tab starts with the maintainer's blackletter
  “erase me!” sample, centered in the pad. Whole strokes removes the strokes it touches;
  Precise erases only the part under the tip. Reset pad restores the sample. Whole strokes
  remains the default, and Reset tool on the eraser tab restores that mode too.
- **Fine slider control**: hold a Pen feel slider briefly, then move above or below its track
  to reduce sensitivity. Return within a knob-height band around the track to catch up and
  follow your finger or pen directly again. Release away from the track to keep a fine value.
- **Consistent tool selection**: from the eraser or another pen, tapping the top pen button
  selects its pen; tapping it again opens the picker. A grid button still opens directly.

## 1.2.0 (2026-10-25)

This release includes the changes previously tested as 2.0.0-beta.1 and beta.2. The 2.0 beta
remains a historical prerelease; this release continues the stable 1.x series.

- **Vertical highlighter**: a fixed vertical tip replaces the round tip. This also changes how
  older highlights render when a notebook is opened or exported; preserving the old shape is
  tracked in [issue #5](https://github.com/ganten1998/grimoire-scribe/issues/5).
- **Clear writing area**: Index jumps and pinch zoom add no floating Back or reset-zoom buttons.
  Pinch inward to return to the fitted page view.

- **Journal index**: name pages as section starts from the new Index tab beside Pages. Tap a
  section to open it. Names follow their pages through insertions and deletions, survive
  reopening, and travel in the raw notebook ZIP. The optional `index` shortcut works with custom
  keyboard or controller mappings.
- **New page inside a notebook**: in Pages, add a blank note after the open page, or after the
  last ticked page. It uses the notebook's paper, including when the preceding page is an
  imported PDF page. The first page stays the notebook's cover.
- **Copy page**: tick one page in Pages and copy it right after the original, with its ink and
  paper intact. An image page keeps its picture. Later edits to either page stay on that page.
- **Book-page labels**: pages annotated from KOReader show their source page number in Pages,
  so inserted notes do not make the book's page numbers ambiguous.

## 2.0.0-beta.1 (2026-10-25)

The quality release: what is new since 1.1.0-beta.2. The release candidates 1.1.0-rc.1 to rc.3
(below) are folded into it; there is no 1.1.0. docs/TESTING.md says what was checked on a Kindle.

- **Writing over PDF and image paper**: fast pen updates keep unchanged gray paper intact,
  reducing the temporary shadow around a stroke. Writing over gray images can still feel slower
  than writing over black-and-white paper; further pen-latency work is ongoing.
- **Native PIN checked on firmware 5.20.1.0.2**: from a Kindle Library launch, the Scribe 2024
  showed its own keypad and returned to writing after unlock, upright and sideways.
- **Retuned pen defaults**: the pencil, mechanical pencil, ballpoint, highlighter, fountain pen
  and calligraphy pen start with the settings tuned on the Kindle. Pen feel > Reset tool uses
  the same values. Existing saved pen settings and the ink already on your pages stay as they are.

- **Install with KPM**: the release has a `.kpkg` for Kindle Package Manager on `kindlehf`
  Scribes, built from the same app as the zip. It adds the two Library items, keeps your
  notebooks and settings through install, update and removal, and installs no KOReader plugin or
  patches. KPM numbers this release 2.0.1001. The preview feed starts with this release, and the
  stable feed with the first stable release, 2.0.0; docs/KPM.md says how to add a feed.
- **KOReader integration is your choice**: a new install, or a start from the Library, no longer
  adds Grimoire's plugin to KOReader. Add it with Library > Settings > KOReader > Install / update
  Grimoire plugin, or copy it in by hand. A plugin already installed keeps being updated; once you
  delete it, or install KOReader again, it stays out until you add it again. The plugin is
  refreshed when Grimoire is opened from the Kindle Library, so after updating, open Grimoire once
  from the Library before using Tools > Grimoire.
- **Opening Grimoire from KOReader works on firmware 5.20.1.0.2**: KOReader keeps a key-only input
  device open there, and Grimoire refused to start. A held device with no pen or touch axes no
  longer stops it; a held pen or touch screen, or a device whose kind cannot be read, still does.
- **Keyboard and controller shortcuts**: Library > Settings > Shortcuts > Enable shortcuts writes
  starter mappings to `grimoire/shortcuts.ini` (only when the file does not exist): tools,
  undo/redo, white ink and moving between pages. Edit the file, then Reload shortcuts.ini. The
  sheet shows the last key or button code, to help map a controller. Shortcuts act only in a
  notebook that is idle: not while drawing, with a sheet open, asleep or at the passcode keypad.
  While shortcuts are enabled, key and controller presses keep the Kindle awake, as the pen does.
  Grimoire adds no Bluetooth pairing and maps no analog sticks; 8BitDo controllers are not
  confirmed yet. docs/SHORTCUTS.md has the details.
- **Terminess menus in KOReader**: the optional patch picks a matching regular and bold font,
  uses a real bold, and leaves a gap between the dropdown menu's battery icon and its percentage.
  Library > Settings > KOReader updates an unchanged earlier Grimoire patch; a patch you changed
  is left alone.
- **Updates wait for Grimoire to close**: while Grimoire is open, unpacking a new `Grimoire.sh`
  and resetting the pen calibration are refused before they touch the app or your settings. A
  notebook PDF still being written in the background does not block a later start.
- **"Annotate this page in Grimoire" refuses a notebook's own PDF** from `grimoire/PDFs`, which
  would make a notebook of a notebook: open that notebook in Grimoire instead. A copy of the PDF
  kept somewhere else is annotated like any other book.
- **Gothic Library**: a small gap between the header ornament and the notebook cards, upright and
  sideways.
- **No calibration tip at startup**: Grimoire opens without that reminder.
- **No KOReader DPI patch**: the DPI experiment tried in the release candidates is gone, and
  KOReader keeps its own scaling.
- **Known issue**: on firmware 5.20.1, KOReader can draw an incomplete first screen, also when it
  comes back from Grimoire. This is a KOReader issue
  ([koreader issue 16283](https://github.com/koreader/koreader/issues/16283)), not fixed by
  Grimoire.
- **For developers**: `make dist` collects the installer zip, the optional patches zip, the KPM
  package and its feeds, checksums and the source revision in `dist/Grimoire-VERSION-COMMIT/`,
  and never overwrites a different build. The release workflow runs the full checks, needs
  ShellCheck and Luacheck, and publishes only from a published GitHub release.

## 1.1.0-rc.3 (candidate, folded into 2.0.0-beta.1)

- Fix KOReader handover on firmware 5.20.1.0.2: a retained key-only input device no longer
  prevents Grimoire from starting. Retained pen/touch devices and unknown input capabilities
  still block the handover; input device numbers are not hardcoded.
- Collect installable builds under `dist/` with version, source commit and checksums. Preserve
  existing different builds and keep Downloads out of the build delivery path.

## 1.1.0-rc.2 (candidate, folded into 2.0.0-beta.1)

- Remove the calibration startup tip. Grimoire opens without that reminder.
- Add a small gap between the Gothic header ornament and Library thumbnails, in both orientations.
- Refuse manual unpack/update and calibration resets while Grimoire is running, before touching
  its cached executable or settings. Background PDF exports do not block later starts.
- Keep KPM's temporary installer bookkeeping out of `grimoire/`, so a first launch can still
  migrate notebooks from the old Quire name. Require shell/Lua linters in the release workflow.
- Remove the experimental KOReader DPI patch and its Settings control. KOReader keeps its native
  scaling policy and the DPI setting already chosen in it. The Terminess font/battery-spacing fix
  remains.
- Retain the corrected KPM installer for the real Documents folder and the driver-owned Share
  test lifetime. KPM version 1.1.5002 sorts after 1.1.0-rc.1 (1.1.5001).

## 1.1.0-rc.1 (local candidate, folded into 2.0.0-beta.1)

- **KPM:** a kindlehf package built from the same installer as the ZIP, with Library launchers,
  install/upgrade/remove hooks and notebook/settings preservation. Fresh installs change no
  KOReader files. The release workflow validates artifacts before publication and maintains
  separate preview/stable feeds. KPM's numeric version 1.1.5001 represents this RC.
- **Explicit KOReader integration:** fresh Library starts no longer install the Grimoire plugin.
  Enable it in Settings > KOReader or copy it manually. An already installed plugin stays updated;
  deleting it or reinstalling KOReader requires opting in again. User patches remain separate.
- **Keyboard/controller shortcuts:** opt-in mappings for tools, undo/redo, white ink and existing
  pages, edited in `grimoire/shortcuts.ini` and reloaded in Settings > Shortcuts. The sheet shows
  the last button code. Drawing, open sheets and sleep/native PIN block shortcut actions. No
  Bluetooth pairing or analog-axis mapping is added; physical 8BitDo compatibility is unconfirmed.
- **Terminess menus:** deterministic regular/bold font-family selection, real bold registration
  and an explicit gap between dropdown battery symbols and percentages. A byte-exact prior
  Grimoire patch can be explicitly upgraded; modified patches stay untouched.
- **Optional Scribe DPI patch:** preserves the existing 320 layout and scales Auto/other presets/
  custom values proportionally. Loads before widgets cache sizes, leaves other devices alone,
  and retains KOReader's restart requirement. This changes KOReader's sizing policy only when
  the separate patch is enabled.
- **Installer follow-up:** stage Library wrappers inside Documents before the final rename.
  The Kindle blocks cross-directory moves into that folder; the real migration from the
  1.1.0-beta.2 launchers now passes. Regression coverage includes rollback after a failed second move.
  The share test keeps its server alive until downloads finish instead of racing a 25-second timer.
- **Validation:** real Scribe/KPM 0.2.2 isolated install, unpack, ARM version and uninstall passed;
  repository beta-app → RC upgrade and the local-file uninstall/reinstall fallback also passed.
  KPM 0.2.2 rejects a direct local-file upgrade; see docs/KPM.md and docs/TESTING.md. Original launchers and the
  running KOReader process stayed unchanged. Lua patch tests and the
  installed KOReader DPI menu code passed on the Kindle. The battery spacing and DPI behavior
  were confirmed by eye on that Kindle. Physical controller input and multi-device handover remain
  candidate checks.

## 1.1.0-beta.2 (2026-10-24)

- **KOReader compatibility:** discovery includes `extensions/koreader` and `documents/koreader`,
  handover accepts an intervening launcher process, and a KOReader launcher that leaves the Kindle
  UI running gets a Grimoire-owned pause/restore. Grimoire failures now reach the plugin instead of
  always reporting success. The plugin retains KOReader's framework service during Grimoire on
  no-framework starts, so it does not discard the service needed for power events.
- **Fast Pages:** a numbered grid loads no previews or other page files. **Go to page...** opens
  a number pad. A regression scenario uses 120 pages and checks that only explicitly opened pages
  enter memory. Selection and deletion still work.
- **Browser reader:** one page at a time; a sticky page counter and number field at the top jump
  directly to a page. Arrows, direct PNG links and Effects for that page remain available.
- **Native Kindle PIN:** Library starts keep the passcode flag enabled throughout. Sleep releases
  input and resumes the native keypad; only the unlock event returns input to Grimoire. Timers
  cannot bypass it. The framework stays running for the keypad, and PIN takes priority over sleep
  pictures. KOReader's own launcher still disables PIN for its session; hosted Grimoire inherits
  that policy. This new native handover needs device confirmation across firmware versions.
- **KOReader wallpaper:** Display's fourth sleep choice uses a custom image, random image or book
  cover exported by the plugin at handover. A missing image falls back to the page. The native PIN
  screen takes priority; KOReader's message/status widgets are not exported.
- **Manage KOReader in Grimoire:** Library > Settings > KOReader installs/updates/removes the
  plugin and toggles the three patches uploaded with beta.1. The three Lua scripts are reused
  unchanged; Terminess fonts, their OFL and manual instructions are included. Different patches
  at the same filename are preserved. Restart KOReader to apply changes. Both release zip assets
  include the optional scripts.
- **Carry-over fixes:** retain all book-title characters (show a box for glyphs the UI font lacks),
  refresh a card when its annotation picture arrives, put `source.ini` in raw exports, return to
  the tool button's own pen on its first tap, and wait for jobs rather than fixed delays in tests.
  Populated Templates and KOReader controls join the language/layout checks.

## 1.1.0-beta.1 (2026-10-20)

- **White ink and a precise eraser**: the tool picker (tap the pen button on the panel) has a new
  bottom row, **Black | White**. With White, every pen draws white with its own texture and
  settings, taking ink away while the paper's lines, staves, grid and dots stay; the pen in use
  shows a hollow icon on the panel (collapsed too) so it never looks broken on white paper.
  **Pen feel > the eraser's tab > Eraser: Whole strokes | Precise** picks the eraser that takes
  whole strokes, as before, or a precise one that takes away only what it touches. Library >
  Settings' pen-button row has a fourth choice: holding the pen button draws white. The
  whole-strokes eraser no longer takes a stroke when it touches only a part that white ink or the
  precise eraser already took away. When a notebook closes, ink erased entirely (every pixel of
  it) is taken out of its page files in the background, so zips and grimoire-pull copies no longer
  carry it; a stroke erased only in part stays whole in the file. Opening the notebook stops that
  at once.
- **Pinned pens**: pin up to four pens to the panel: tap the pen button, then **Pin** at the top
  right of the picker, tap pens to pin or unpin them, then Done. Each gets its own button under the
  pen button, and the panel still fits sideways at full size. Tapping the eraser again goes back
  to the pen used just before it, a pinned one too.
- **Templates**: copy PNG or JPG pictures into `grimoire/templates/` over USB while Grimoire is
  closed (upright, 3:4, 1860 x 2480 is best, light lines on white), then pick one with **More
  papers...**, the last tile of the editor's Paper sheet and of New notebook. Each notebook keeps
  its own copy, so changing or deleting a file there never changes pages already written. The page
  shows white paper for a few seconds while that copy is made.
- **A folder of page pictures becomes a notebook**: put a folder of PNG or JPG pages in
  `grimoire/import/<Name>/` (a PDF saved as pictures, for example with
  `pdftoppm -gray -png -r 200 file.pdf page`) and start Grimoire. It makes the notebook <Name> in
  the background, each picture a page's paper, says "Imported a notebook of N pages.", and moves
  the folder to `grimoire/import/done/`. PDFs are not read on the Kindle.
- **Annotate this page, from KOReader**: in a book, Tools > Grimoire > **Annotate this page in
  Grimoire** (also a gesture action) opens the page you are reading in Grimoire, ready to write
  on. Grimoire keeps one notebook per book, named after its title, with the pages you annotate in
  the book's order; annotating the same page again opens it with what you wrote. Library > Back
  to KOReader returns to the same book and page. A PDF's page is the whole page; an EPUB's is the
  screen as KOReader lays it out.
- **Sheets stay open when the Kindle turns or wakes** in a notebook too (the tool picker, Paper,
  Custom music, Display, Templates), as they do in the Library.
- **What you wait on comes first**: a PDF you asked for and a template's picture are made before
  background compaction and mirror PDFs.
- **Grimoire in Spanish**: every screen, the share pages and the Effects studio now come in
  Spanish too. A new **Settings > Language** row offers Automatic (named with the language it gives:
  "Automatic (English)"), English and Español. Automatic, the default, follows the language of the
  Kindle's own Settings, so a Kindle set to Spanish shows Grimoire in Spanish from the first start
  after the update; Ajustes > Idioma > English (Settings > Language, in Spanish) switches back. A
  choice applies at once and is kept in `settings.ini` (`language=`). In Spanish the
  keyboard's letters page has ñ after l, and a new notebook is named "Cuaderno 20 oct". Pages never
  change with the language: paper looks the same in every one (Cornell notes keep their English
  headings), and so do PDFs, PNGs and thumbnails. grimoire.log stays in English and says which
  language is in use and why (`i18n: language es (auto, from the Kindle's locale es_ES.UTF-8)`).
- **Text that fits**: a notice too long for one line takes two instead of being cut short, and a
  notice is never cut inside a character. The Library's top-bar buttons widen together for a longer
  word, the zoom chip widens for its text, and a paper's name too long for its tile, in New
  notebook or in the editor's paper sheet, goes on two lines. The Library card's line ("3 pages,
  Oct 10, 14:32") is no longer cut sideways in Classic: it takes a smaller size where the style has
  one, else leaves out the time. Custom music's part buttons draw their names at one size, and the
  Settings rows keep their text inside the sheet.
- **Share pages in Grimoire's language**: the share index, a notebook's page and the Effects
  studio are in the language Grimoire is in (their `<html lang>` says which), dates included ("10
  oct, 14:32"), and so are the messages a browser can show (a notebook that is gone, an export
  that failed). `/api/list.json` and the download file names do not change: a notebook's `paper`
  stays the English name. In the Gothic style, bold links keep accented letters legible. The
  studio's export buttons say "page" for a one-page notebook ("PNGs, all 1 page (zip)").
- **Open Grimoire from KOReader**: with KOReader installed, the next start of Grimoire from the
  Library adds **Tools > Grimoire** to KOReader: **Open Grimoire**, **Notebook PDFs** (KOReader's
  file browser in `grimoire/PDFs`) and **Reset pen calibration**, plus an Open Grimoire action for
  KOReader's gestures. Open Grimoire closes KOReader, runs Grimoire in the same session (the
  Kindle's home screen never shows), and when Grimoire ends KOReader starts again, with the book
  that was open. Opened so, the Library's Exit reads **Back to KOReader**, and holding the power
  button 4 s goes back too. The plugin speaks KOReader's language (English or Spanish). It is
  optional, and the zip's new "KOReader plugin (optional)" folder holds a copy for a hand install,
  never a top-level `koreader` folder. Each start from the Library keeps the installed plugin up to
  date, and `grimoire/.koplugin-installed` holds the build it came from; deleting
  `koreader/plugins/grimoire.koplugin` while KOReader stays installed keeps the plugin out (the file
  then reads `removed`). A start that finds no KOReader forgets that, so a KOReader installed later
  gets the plugin. *Execute shell script* on `Grimoire.sh` in KOReader still refuses with status 3.
  - Hosted by KOReader, Grimoire leaves the passcode and the pause of the Kindle's screens to
    KOReader, which keeps both for its whole session (Grimoire only stops again what KOReader's
    exit woke). The Kindle's own screensaver as the sleep screen needs those screens, so the page
    shows instead. Only a KOReader started by `koreader.sh` that paused the Kindle's screens (its
    Library item, KUAL) can open Grimoire. When the hand-over does not run Grimoire, KOReader says
    "Grimoire could not start. See grimoire/grimoire.log.": grimoire.log has the launcher's reason,
    and for a failure inside KOReader, KOReader's `crash.log` has a `Grimoire:` line.
  - For tools: `grimoire/app/grimoire.sh --hosted koreader` (0 Grimoire ran, 1 another Grimoire is
    running, 3 refused), and `Grimoire.sh --hosted koreader`, which unpacks first and then answers
    with that status, or exits 1 when it cannot unpack the app and 2 while the old Quire app still
    runs (docs/KINDLE.md). `hosted` is in the Capabilities lines: `guard unpack-only hosted` in
    `Grimoire.sh` and `app/CAPABILITIES`, `guard hosted` in `app/grimoire.sh`.
- **Notebooks keep their page size**: the 2022 and 2024 Scribes have a 1860x2480 screen, the
  Scribe 3 and the Scribe Colorsoft a 1986x2648 one, and a notebook now keeps the page size it
  was made with (`page_w`, `page_h` in `notebook.ini`). On a Scribe of the other size its pages
  are shown fitted to the screen, the writing on its lines; its new pages, thumbnails and every
  export (PDFs, PNGs and the share pages' pictures, a missing page's blank one included) have the
  notebook's size. Pages an earlier version stretched to another Scribe's screen come back to the
  notebook's size. A `notebook.ini` without a size takes the size most of its page files have, a
  tie going to the first page's. Custom music's "Systems per page" counts the page's own height
  (a 1986x2648 page fits more).
- **Calibration follows the panel**: `settings.ini` records the panel each calibration was made on
  (`pen.panel`, `touch.panel`), and a `grimoire` folder copied to a Scribe of the other size has
  its pen and touch calibration scaled to the new panel at startup. A record gone stale
  (1.0.0-beta.2 keeps the line through a calibration of its own) is caught by the calibration's
  own scale, on either size.
- **Scribe Colorsoft**: on its color screen, a quick refresh other than the pen's own ink now waits
  until the driver has read the frame (`MXCFB_WAIT_FOR_MDP_COMPLETION`), which should stop buttons
  showing half old or striped while the panel is dragged or the page pinched. Writing, erasing,
  the pen feel pad and the calibration's check line never wait. A driver that refuses the wait, or
  takes over a second, is not asked again that session.
- **For developers**: `make check` runs `make l10n-check`: the committed `po/*.mo` must be what
  msgfmt makes of `po/*.po`, and `tests/scenarios/y_all_screens.qs` visits every screen and sheet
  in every language and style with the fit check (`GRIMOIRE_CHECK_FIT=1`), which fails on any text
  cut short (`fit[es]: ...`); its shots in `build/shots/l10n/<lang>-<style>/` show translators every
  screen. `make l10n` (GNU gettext) collects the strings into `po/grimoire.pot` and updates and
  compiles each language. `docs/TRANSLATING.md` says how to improve a translation or add one.
  `GRIMOIRE_LANG` picks the simulator's language, `GRIMOIRE_L10N_DIR` reads a catalog from a
  folder, and `GRIMOIRE_HOST=koreader` starts it as KOReader's plugin does; the scenarios run in
  English. New checks cover the language, the hosted start, page sizes and live ink (`language`,
  `host`, `asleep.mode`, `editor.page_size`, `editor.scale`, `display.last_fast` and more, in
  docs/SIM.md).
  `tests/share_check.sh` also checks the pages in Spanish and from a catalog full of markup, and
  the studio's Copy command (with node). `tests/launcher_check.sh` runs hosted starts and their
  refusals and installs the plugin; `tests/koplugin_check.sh` runs the plugin against stand-ins for
  KOReader (with luajit). The test builds stop at the first undefined behaviour.

## 1.0.0-beta.2 (not released on its own: all of it is in 1.1.0-beta.1)

- **Hold the power button to leave**: hold it 4 seconds to save and leave Grimoire from any
  screen, asleep or awake, and get the Kindle back. On a Kindle that stays locked after a press, a
  second press does the same. When the page cannot be saved (the Kindle's storage is full), the
  hold never says "Saved.": awake, the first hold stays in Grimoire with "Could not save this page.
  Hold power 4 s again to leave anyway.", and a second within a minute leaves with "Leaving
  Grimoire without this page." From the sleep screen, and on a Kindle that stays locked, Grimoire
  leaves at once, and grimoire.log says the page could not be saved. Leaving straight from a
  notebook this way while another notebook's PDF is still being written no longer skips the open
  notebook's PDF.
- **Kindles with a passcode**: a Device Passcode is now paused while Grimoire is open and turned
  back on when you leave, as KOReader does. With a passcode, the Kindle holds back its wake until
  the passcode is typed, but its keypad cannot take a touch while Grimoire holds the touch screen:
  Grimoire's page came back over a Kindle that stayed locked, and leaving painted the old screen
  over the passcode keypad (a tester on 5.20.1 could only get out by restarting). Grimoire says so
  the first time ("Your Kindle passcode is paused while Grimoire is open."). If the Kindle restarts
  while Grimoire is open, the passcode stays off: Grimoire then shows "Your Kindle passcode was
  left off. Turn it back on in Kindle Settings." (in place of its other start notices) at that
  start and the next two while the passcode is still off. When the launcher cannot note the pause
  (the Kindle's storage is full or read-only), it does not pause the passcode.
- **Sleep and wake fixes**: the Kindle no longer falls asleep again right after waking. Three pen
  side-button clicks while asleep no longer clear the page, and a pinch while asleep no longer
  zooms it. A press right after another no longer requests sleep halfway through a long press.
  Grimoire no longer draws over a Kindle that is still locked or asleep after a press, and no
  longer sends a power-button request to a Kindle already in its screensaver. The sleep request is
  ordered (preventScreenSaver 0, then powerButton 1), and "Sleep after: never" is restored after
  each wake. Leaving Grimoire while the Kindle shows its screensaver or passcode prompt no longer
  paints the old screen over it.
- **Notebooks move between versions**: Grimoire now keeps what a newer version writes and does not
  understand (ink data, papers, extra page data, unknown `settings.ini` and `notebook.ini` lines)
  and writes it back unchanged. A page from a newer version opens read-only ("This page was made
  by a newer Grimoire. It is read-only here.") instead of being set aside as damaged, and pages
  1.0.0-beta set aside come back when their notebook is opened, if they are still in its page
  list. Papers from a newer version show blank as "Newer paper" and are kept; a page added after a
  page that holds one imported page's picture (1.1's document pages) gets the notebook's own paper,
  as 1.1 does, never that picture. A setting value from
  a newer version that this one lacks (for example `side_button=3`) works here as the nearest value
  it has. It stays in the file until a save finds the setting changed; from then on the setting is
  saved as you set it, even if you set it back. A switch holding another value than 0 or 1 (a
  newer version's third state, such as `invert=2`) is used as on or off and stays in
  `settings.ini` as it was until the setting is changed. A notebook or folder name longer than 95
  bytes is cut before the character that would cross the limit, never inside an accented letter.
  `docs/FORMAT.md` documents the file format, and the README's "Moving between versions" says what
  older versions do with newer notebooks.
- **A QR code on the Share screen**: point a phone's camera at it to open the share page without
  typing the address. It holds the Kindle's Wi-Fi address; other addresses (USBNetwork's, say) are
  listed under it. Share also has a landscape layout, so the downloads counter no longer covers a
  help line or the third address and text no longer runs past the card. Turning the Kindle or
  waking it on the Share screen no longer restarts the share server, so queued downloads and pages
  being sent are no longer dropped.
- **Share fixes**: asleep on the Share screen, a phone or computer trying to connect no longer
  keeps Grimoire's processor busy until the wake; the connection waits and is answered after the
  wake. With no page to show, the Share screen's sleep screen no longer shows the QR code and the
  address: it shows the "Share over Wi-Fi" card with "Asleep" and "Wake the Kindle to share
  again." The share page's link to a folder's zip works for names up to the 95-byte limit (a long
  accented name got "not found").
- **Accents on the keyboard**: a new **áé** key beside the space bar opens a page with á é í ó ú,
  à è ì ò ù, â ê î ô û, ä ë ï ö ü, ñ ç ã õ ß œ æ ø å and ¿ ¡ « », and the middle dot of Catalan
  l·l. Shift gives the capitals and **abc** goes back; backspace removes a whole character.
  Renaming keeps accents: the keyboard used to drop every non-ASCII byte from the name it opened,
  so "Música" came back as "Msica". Folders whose names begin with an accented letter now sort
  with that letter (Álgebra among the A's), not after Z.
- **Gothic bold keeps accents legible**: bold labels, toasts, menus, Settings rows and keyboard keys
  keep the two dots of ü ö ä ë, the tilde of ñ ã õ, the hat of â ê ô û and the ring of å, and the
  carons, double acutes and ring of Czech, Slovak and Hungarian (č š ž ř ň ě ď ť ő ű ů) keep their
  gap. The bold overprint used to fill them in (ü looked like ū). In bold, ß keeps the notch that
  tells it from B, and ø and Ø keep their counters open around the slash. In every Gothic weight
  the inverted exclamation mark now hangs below the line, as type usually sets it; it used to look
  exactly like an i. ASCII text and the Classic style do not change.
- **KOReader**: Grimoire no longer starts from inside KOReader, where it got neither pen nor touch
  and its exit woke the Kindle UI underneath KOReader; KOReader shows status 3 instead. A Library
  tap with a KOReader left running says so on the screen. For the KOReader plugin in 1.1:
  `Grimoire.sh --unpack-only` with documented exit statuses, and Capabilities lines in
  `Grimoire.sh` and `grimoire/app/grimoire.sh` (also `grimoire/app/CAPABILITIES`).
- **Smaller fixes**: a tap on Undo or Clear right after a stroke now works at once (it was ignored
  for about half a second). Turning the Kindle, or waking it, no longer closes an open Library
  sheet (Settings, New notebook, the keyboard...). In the Library, a notice that is up when the
  keyboard or a tall sheet opens (New notebook; Settings and Custom music sideways) moves above
  its keys or buttons instead of covering them. Sideways, the New notebook sheet shows "Narrow, no
  margin" and its neighbours on two lines instead of cutting them short. The paper pickers and
  Library cards no longer show a paper they do not know as the 7-string bass tab. A page that
  cannot be saved (the storage is full) is tried again every 2 s instead of at every turn of the
  main loop. The notice for a damaged page set aside now fits the screen in the Gothic style.
  `settings.ini` is no longer written through a fixed 8 KB buffer that was never checked.
- **Effects preview**: the transparency checkerboard covers only the page, which now has a thin
  outline on a gray surround, so you can see where the page ends.
- **Other Kindles**: the screen guard works on 32 bpp panels (Scribe Colorsoft); Grimoire closes
  cleanly, giving the Kindle its screen back, if the panel takes no refresh at all; the dpi of a
  device FBInk does not know comes from the panel's physical size.
- **A more useful grimoire.log**: it starts with diagnostics for bug reports (the model code, never
  the serial number; memory, display and input details; the sleep settings; the Kindle's language,
  whether a passcode is set, the power state, Special Offers and KOReader's version) and ends with
  the power state at exit. None of these lines identify the Kindle or its network. The Share
  screen logs which interface its address is on and the port, never the address itself, and a
  folder's zip download is logged without the folder's name. A `watcher-multi` switch (an empty
  file in the `grimoire` folder) listens to the power service the way KOReader does, for a device
  test.
- **Licenses**: the app ships the licenses of FBInk, stb, musl, zig's compiler-rt and the QR code
  generator, and the notices of cutef8 and Bjoern Hoehrmann's UTF-8 decoder (both compiled in with
  FBInk), with the GPL and the font licenses, in `grimoire/app/licenses`.
- **For developers**: `make check` builds and inspects the zip and runs both launchers against
  stand-ins for the Kindle's tools. The scripted scenarios check what the app holds after each
  step (`expect` lines) and fail the build when a check fails or a script line is misspelt. After
  the Kindle turns, the simulator maps input through the calibration the way the Kindle does, so
  script taps land where they say on calibrated data too. `tests/test_golden.c` holds renders
  recorded from 1.0.0-beta that every later version must reproduce, and `tests/test_format.c`
  checks pages against a frozen copy of 1.0.0-beta's page reader. The runner's `power held` and
  `storage full`, its `watching`, `toast.rect` and `editor.unsaved` checks and a scenario's
  `# pdfs: N` line cover the hold, a full storage, Share asleep, notices over sheets and the PDFs
  written at exit (docs/SIM.md).

## 1.0.0-beta (2026-10-18)

- **A logo, and a bat on the cover**: "Grimoire" in the pixel blackletter with a pixel bat over
  it; the same bat now sits in the window of the Library cover, in place of the diamond.
- **Effects in Share over Wi-Fi**: a studio in the browser makes new pictures of any page:
  transparent, white, colored or gradient backgrounds; paper lines shown, hidden or recolored;
  ink in a color or a gradient across the writing (a Signature preset: transparent, gradient,
  trimmed); tones (gray, duotone, invert), warmth, tint, brightness and contrast; a crop to the
  ink; presets from Sepia to Blueprint. It downloads a PNG, a zip of PNGs or a PDF, and shows the
  ImageMagick command that makes the same picture.
- **Share over Wi-Fi fixes**: a browser asking for several things at once (the page previews of
  a notebook) could get the wrong file or a dropped connection, because the server answered a
  new connection from the previous request before the new one had arrived. Each connection now
  has its own buffer, downloads beyond the two the Kindle renders at once wait their turn
  instead of being refused, an idle connection no longer holds up the others, and notebook page
  previews load at a third of the size.
- **Share pages in your style**: the browser pages use Grimoire's own fonts, Jacquard 24 titles
  and Jersey 10 text in the Gothic style (kept pixel-sharp), Terminess in Classic.
- **Lighter notebook names**: in the Gothic style, notebook and folder names on the Library
  cards use the pixel font's regular weight instead of its bold, which turned a name in
  capitals into a solid black bar.

## 1.0.0-rc.1 (2026-10-17)

First public pre-release, and a new name: **Quire is now Grimoire**.

- **Gothic style**: crisp pixel type like an old console game (Jacquard 24 pixel blackletter
  titles, Jersey 10 text, drawn as exact pixel blocks), carved-stone frames, an arcade of
  pointed arches, lozenge checkboxes and sliders, a new Library cover. Settings > Style switches
  to the plain Classic look.
- **Two new pens**: **charcoal** (coarse grain, crumbly edges, loose dust, broad and soft on
  its side) and a **calligraphy pen**, a broad-edge nib held at 90 degrees to the pen's real
  lean, so a right hand leaning to the bottom right writes classic 45 degree italic and a left
  hand gets the mirror image. The tool picker is now a 3 x 3 grid of nine pens.
- **Moving from Quire**: the first launch moves `quire/` to `grimoire/` and removes the old
  launcher; nothing to do by hand.
- **Safer storage**: a page that cannot be read is set aside (`*.damaged-<time>`) instead of
  being overwritten by the blank shown in its place; a damaged notebook index rebuilds its page
  list from the page files; a save that hits a full disk fails loudly and keeps the page in
  memory instead of dropping it; notebooks created in the same second can no longer collide.
- **Share over Wi-Fi**: PDF, zip and PNG downloads render in a separate process, so a big
  download no longer freezes the Kindle or trips the watchdog; requests for foreign hostnames
  are refused (DNS rebinding); the firewall follows a custom port.
- **Sleep and launch fixes**: waking from sleep no longer sends you to the Kindle home screen;
  a leftover background export can no longer stop Grimoire from launching; a relaunch waits for
  the previous session to finish handing the screen back; the power-service watcher exits with
  Grimoire even after a crash.
- **Other Kindles**: a clear notice when no pen is found; the warmth slider is hidden on Kindles
  without warm light.

Earlier (as Quire, private): notebooks and folders, 18 papers plus a custom music builder, seven
tunable pens, the floating panel, pages view, pinch zoom, rotation, sleep screens, Wi-Fi / USB /
SSH export.
