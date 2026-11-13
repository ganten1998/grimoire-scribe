# Lasso select, copy, cut, paste, move, and scoped erasing (2.0.0-beta.2, lane B)

Status: design, for one builder. Base: `claude/b2-lasso` at e691c43 (`rust/src` there is the
2.0.0-beta.1 tree plus docs). This replaces items 3 to 6 of lane B in `docs/beta2/BRIEFS.md`
where it says so (section 12), following the owner's words of 2026-11-02:

> "why can't we have our cake and eat it too? changing the way the eraser works would be worth it
> if we could do lasso copy paste without anything unexpected for the user."
>
> "copypasting a clearly linked item should do what the user expects - copies the link and
> contents exactly."

It is the synthesis of three competing designs (scoped backdrops with group tags, bake to
geometry with gap runs, scoped floors with a marker), judged against the code. Section 14 records
what was checked and which claims were refuted.

## 1. The idea

An erase stroke (tool 3: white ink or the precise eraser) lightens every pixel it covers toward
the paper under that pixel, never past it (`canvas_restore_px`, `restore_span`). The restore
target is a buffer handed to the brush: `brush_set_erase(b, paper, x0, y0, stride)`. Today that
buffer is always a copy of the paper (`render_layers`, page.rs:330-401).

**Scoped erasing:** an erase stroke may instead restore toward its **floor**: the page as drawn
just before a marked point in the stroke order. Same brush, same blend, same texture byte and hash
bits, same parameters. Only the target buffer differs. Over paper the floor *is* the paper, so a
scoped erase stroke draws exactly what an ordinary one draws. Over earlier ink it takes away only
the ink drawn after the floor, and lands exactly on the ink under it, never lighter.

That gives the lasso what it needs:

- Erasing belongs to the ink it erased. When selected ink moves or is copied, the erase strokes
  that touched it go with it as copies. Where ink lies under the pasted group, those copies are
  scoped to the group's floor, so the destination ink is never lightened.
- The original erase strokes stay where they are. Over the emptied spot they restore paper to
  paper, so they leave no ghost; other ink they erased stays erased.
- Nothing is composited and no ink is separated from paper, so the rejected ink-canvas approach
  (about 95% of inked rule pixels changed) never comes up.
- Live erasing does not change at all. New erase strokes are ordinary (floor = paper). The eraser
  looks and works exactly as before; erase strokes only gain an optional scope for when they
  travel. That is the cake, eaten.

Scoping is used only when it is needed: when ink lies under a group's carried erasure at the
destination. Otherwise the copies stay ordinary erase strokes and every version from 1.1 on shows
the page exactly.

## 2. What the user sees (the expectation matrix)

Shared facts:

- Moves and pastes go by whole page pixels. Ballpoint, fineliner, fountain pen, marker,
  calligraphy and highlighter strokes come out byte-identical, except a rare single gray level
  where an f32 coordinate crosses a power of two (tests count it).
- Pencil, mechanical pencil, charcoal and white-pencil grain belongs to the paper: `tooth_at`
  samples a 512 px tile at absolute page position, charcoal dust hashes absolute x and y. A moved
  pencil word shows the tooth of its new spot, like a real pencil line. Shape, width, darkness
  and erased gaps are the same. (Pixel-identical for offsets that are multiples of 512 px.)
- Seeds are kept when free on the destination page (always so for a move on its own page), which
  keeps per-dab jitter. See 6.6.

| # | Case | What the user sees |
|---|---|---|
| 1 | A word partly erased with the precise eraser, lassoed and moved | **New place:** the same word with the same gaps, arcs and soft rims, on top of what is there. Ink under its gaps is not touched. **Old place:** nothing of the word, no erasure ghost. Other ink the same eraser strokes took away stays erased (the originals stay). One undo step. |
| 2 | The same word copied and pasted | Identical look. Destination ink under the paste is **not lightened** anywhere: if any lies under the carried erasure, those copies are scoped. The source page does not change; Copy is no undo step. The paste lands selected, ready to drag. |
| 3 | White ink drawn over someone else's black ink; only the white is lassoed | **Decided: white ink is erasing, not an object.** A lasso around only white ink or eraser marks selects nothing: toast "Nothing to select here: white ink and eraser marks move with the ink they cover." The black ink stays covered. Lassoing the black ink brings the white-out with it (it moves still whited out, looking the same). The correction workflow (white out a word, write the fix over it, lasso the fix, move it) moves the fix alone: the white-out came before the fix in stroke order, so it is not carried; the mistake stays hidden at the old place and nothing at the new place is whited out. Justification in 2.1. |
| 4 | Ink and the eraser marks inside the lasso, pasted into a notebook of another page size | Pasted **1:1** in page pixels: every Scribe is 300 dpi and widths and rules are in mm, so the writing keeps its physical size and still fits the destination's rules. It goes where it was if that spot is on screen and on the page, else in the middle of the view, clamped onto the page. Only a group larger than the destination page is scaled down uniformly (points, link areas and outlines; widths stay in mm, as in `scale_page`). Eraser marks that erased selected ink come along and erase only the pasted ink, toward the destination's own paper and ink. Marks that erased only unselected ink stay behind (they are invisible). |
| 5 | A linked item lassoed (ink plus link area or outline, or a beta.5 ink-box link) | The link comes with the selection (rules in 6.2). The action bar shows "1 link" (or n links) and the selection draws the carried link areas dashed, so the user sees it before acting. **Copy/paste:** a new link on the destination page with the same target, area and outline moved by the same pixel offset (scaled with the group if it was scaled), and a fresh seed. Across notebooks the target is rewritten with `nav_target_across(target, src_nb, dst_nb)`. **Move on the page:** the same link (same seed), area and outline moved, in the same undo step. **Cut/Delete:** the link goes with its ink. A carried beta.5 ink link's strokes take the link's new seed, so beta.5 builds still find its box. |
| 6 | Undo/redo of each | Move, Paste, Cut and Delete are one step each, link changes included. Copy is no step. Undo and redo swap the step's canvas patch: instant on the 1,090-stroke page. |
| 7 | A lasso-touched page in older versions | Section 4. Pages never touched by the lasso look and save exactly as today in every version. Lasso pages without scoping (the common case) are exact in 1.1, 1.2.0 and 2.0.0-beta.1. A scoped group shows there with its erasure also lightening the destination ink under its gaps; nothing is lost on their re-save and beta.2 shows it right again. 1.0.0-beta draws no erasure (as on every page) and its re-save zeroes the scope. |
| 8 | Pencil-textured partial erasing (white pencil, texture byte), and erase strokes crossing selected and unselected ink | The moved or pasted word carries a copy of each such stroke (same texture byte, seed, parameters and points), scoped if needed, so its partial erasing is identical. The original stays, so the unselected neighbour stays partly erased exactly as before (except where the moved ink used to overlap it, which is now uncovered as expected). |
| 9 | Drawing or erasing over a pasted group later | New ink goes after the group: normal. New erase strokes are ordinary and take away group ink and anything under it, as anywhere. The stroke eraser takes group strokes away (erase strokes and floor markers stay, as erase strokes always do), deciding visibility through the floor-aware renderer. Lassoing the group again, alone or together with the ink under it, keeps every floor, so it moves exactly again. |

Other rules:

- Read-only pages: Select and Copy work; Cut, Delete, Move and Paste are refused with the existing
  read-only message.
- A paste that would pass 64 links on a page or 4000 in a notebook, or that carries links while
  links.ini does not read or write, is refused whole with a toast. Nothing is pasted in part.
- An `index` link has no cross-notebook target syntax (`nav_external_target` needs `p<id>`).
  Pasted into another notebook, its ink pastes and the link is left out with the toast "A link to
  the Index stays in its notebook." (owner question 3; lane C may add a syntax).

### 2.1 Why white-out is not an object

- Grimoire presents white ink as erasing: README "With White, every pen draws white with its own
  texture, taking ink away while the paper's lines stay." It restores ruled lines, which white
  paint would cover.
- It is invisible over paper and the editor does not even keep a white stroke that changed no
  pixel (FORMAT, Erase strokes). On e-ink there is no outline to show where the "object" is.
- Moving it alone would uncover ink at the old place (often a mistake the writer meant to hide)
  and wipe ink at the new place. Both are the "unexpected" the owner rules out.
- Uncovering stays Undo. An explicit "Remove erasure" action on a selection, and white-out as an
  object, go to the BACKLOG as owner options (owner question 1).

## 3. Data and format changes

No header version bump, no tool id >= 10, no chunk in notebook pages, no links.ini syntax change.
The erase blend, texture byte and hash bits are unchanged (the frozen invariant holds: no
existing page draws differently).

### 3.1 ext[1] (the reserved "layer" byte) gets its meaning

Registry row: **ext[1] = layer. On an erase stroke: 0 = the page (restores toward the paper);
1 to 255 = scoped to that layer's floor (2.0.0-beta.2). On ink: always 0; other values reserved.**

- **Floor marker.** A tool-3 stroke with exactly one point at (-8192, -8192), pressure 0, tilt 0,
  t_ms 0, the eraser's default parameters (in range), seed 0 (round tip), and ext[1] = g. It is
  off every page (pages are at most 1986 x 2648) and inside every reader's -10000..30000 clamp, so
  it draws nothing in any version. Recognised as: tool 3, one point, x <= -5000 and y <= -5000.
- **Floor of layer g:** the canvas as drawn up to, not including, the first floor marker with
  ext[1] = g, at its position in the stroke order whether or not it is drawn or skipped.
- **Scoped erase stroke:** an erase stroke with ext[1] = g that comes after layer g's marker
  restores toward the floor of g instead of the paper. With no marker of g before it, it restores
  toward the paper (an ordinary erase stroke). A later duplicate marker of g is ignored.
- Ink always keeps ext[1] = 0, so deleting any ink never moves a floor. The marker is the
  sentinel nothing deletes: the lasso never selects it, the 1.1+ stroke eraser never removes an
  erase stroke ("Erase strokes stay", editor.rs:3630; the floors design reports the same in
  the editor.c of v1.1.0-beta.1, v1.2.0 and v2.0.0-beta.1, which the builder re-checks), 1.0.0-beta's stroke eraser cannot reach a point off the page, and every
  compactor refuses pages with ext[1] != 0 (bgwork_compact.rs:368; v1.2.0 bgwork_compact.c:301;
  v2.0.0-beta.1 :303).
- **Writers** take a layer g unused by any stroke on the page and by any stroke a history step of
  that page owns (so undo and redo never meet a reused layer). All 255 in use: the paste or move
  is refused with "This page holds too many moved groups: paste on another page." (practically
  unreachable: layers are used only when needed; reclaiming dead layers is a BACKLOG item).
- Nesting needs no table: each scoped erase stroke names its own floor; a group re-lifted with
  two floors gets two markers and two layers.
- No released version writes ext[1] != 0 (1.0.0-beta writes 0 0 0; later versions write ext as
  read and set only ext[2]'s low bits), so every existing page has layer 0 everywhere.

### 3.2 The clipboard: `root/clipboard.qpg` (new, read only by 2.0.0-beta.2 on)

- A version-1 page file: w and h = the source page size, paper 4 (Blank). Its strokes are the
  lifted group L in order (6.3), at source coordinates, with floors encoded exactly as on a page
  (markers and layer bytes, clipboard-local numbers).
- After the strokes, one **critical** chunk `CLIP` (registered: used only in root/clipboard.qpg):
  a format byte (1); the source notebook id and page id (length-prefixed); the group box x, y, w, h
  (i32, source px, link areas included); then the carried links as `link.N=`, `area.N=`,
  `outline.N=` lines in links.ini syntax without page ids, source coordinates, original seeds.
- One file, written atomically (tmp, fsync, rename), replaced whole by each Copy and Cut. No torn
  pair is possible. (RC review, 2026-11-04: it lasts one session; every start of Grimoire deletes it,
  FORMAT.md, The clipboard.)
- A clipboard that is missing, does not read, has a bad or cut-off `CLIP`, or more than 20,000
  strokes or 16 MiB: Paste shows "Couldn't read clipboard." and changes nothing. If writing
  it fails during Cut, nothing is cut.
- Older versions never look in the data root for it (the Library lists folders with
  `notebook.ini`; the builder confirms release Library scans ignore root files).

### 3.3 links.ini

No syntax change. Carried links are ordinary `link.`/`area.`/`outline.` lines.

### 3.4 Kept drawings and previews (caches, not data)

2.0.0-beta.1 already keeps dense-page drawings (`store_save_page_render`) under the tag
`{stamp}|{w}x{h}|{fingerprint}`, and its fingerprint equals beta.2's for unlayered pages. It would
therefore keep a flat drawing of a layered page that beta.2 would trust. So:

- `store_save_page_render` / `store_load_page_render` / `store_page_render_kept` and the preview
  stamp take a `layered` flag (the editor has the page loaded at both points, editor.rs:2705 and
  :2560). Layered pages are tagged with a `|L` suffix and only a `|L` tag is trusted for them.
  Older versions never write `|L`, so their drawings of layered pages are redrawn.
- Unlayered pages keep their exact tags and `page_render_fingerprint` is unchanged, so no existing
  kept drawing or preview is retired.

### 3.5 Compaction

beta.2's `holds_newer_data` already refuses pages with ext[1] != 0, and keeps doing so: a layered
page stays listed in `erased.txt` and is not compacted, as in 1.1 to 2.0.0-beta.1. Teaching
`page_droppable` floors (it is render-diff based, so mostly passing floors through the Pieces
path and keeping markers) is a BACKLOG item. A Move or Cut lists the source page in `erased.txt`
when original erase strokes stay behind, so compaction drops those that now change nothing (on
unlayered pages).

### 3.6 FORMAT.md edits (the builder makes them)

- Stroke record row and Registry ext[1]: the layer meaning above; "nothing else in them is
  interpreted yet" updated.
- Erase strokes: a subsection "Scoped erase strokes and floor markers" (the floor rule, the
  marker, robustness rules, nesting).
- Frozen invariants: the floor rule and the marker shape, once shipped.
- Chunk tags table: `CLIP | critical | the clipboard's source and links, only in
  root/clipboard.qpg | 2.0.0-beta.2`.
- Files: `root/clipboard.qpg`.
- Compaction: layered pages are left as they are by every version, this one included for now.
- Links: the seed rule for pasted ink and links (6.6).
- Moving between versions: the rows of section 4.

## 4. Per-version compatibility

| What the lasso wrote | 1.0.0-beta, rc.1 (and 1.0.0-beta.2 to 1.0.x) | 1.1, 1.2.0, 2.0.0-beta.1 | 2.0.0-beta.2 |
|---|---|---|---|
| Ink moved, pasted or cut, no erasure carried | ordinary strokes; a 1.0.0-beta re-save zeroes pad bytes as ever | exact | exact |
| Carried erasure, **plain** (no ink under it at the destination: the usual paste into space) | erase strokes are not drawn (as on every page), so erased gaps show; kept on re-save | **exact** | exact |
| Carried erasure, **scoped** (layer byte + marker) | not drawn; a 1.0.0-beta re-save **zeroes the layer bytes**: back in beta.2 that erasure is ordinary and also lightens the destination ink under the group's gaps, for good (every stroke is still in the file). 1.0.0-beta.2 to 1.0.x keep the bytes. | drawn flat: the erasure **also lightens the destination ink under its gaps**, on their screen only. Layer bytes and marker **kept** byte for byte on re-save; their compaction leaves the page alone; their pens, eraser and undo work normally (new strokes are layer 0, after the group) | exact |
| Floor marker | draws nothing (erase stroke, off page); kept, ext zeroed; afterwards a dead stroke a later compaction may drop | draws nothing; kept | draws nothing |
| The source of a move or cut (original erase strokes left in place) | as for any page with erase strokes | exact | exact |
| Carried links | links.ini never read | read (areas, outlines and `n:` targets all exist in 2.0.0-beta.1; 1.1 and 1.2.0 never read links.ini) | read |
| `root/clipboard.qpg` | never read | never read | read |
| Kept drawing or preview an older version made of a layered page | n/a | trusted by that version | not trusted (`|L`), redrawn |
| PDF mirror page an older version wrote of a layered page | | shows that version's look | corrected when beta.2 next writes the notebook's PDF |
| **A page never touched by the lasso** | **as today** | **as today** | **as today** |

The 1.0.0-beta zeroing follows existing precedent (a highlighter's vertical tip, ext[2] = 1, is
likewise lost on a 1.0.0-beta re-save) and affects only scoped groups: pasted erased ink that
overlaps existing ink.

## 5. Untouched pages render and save identically (the proof)

1. **Input.** No released or current version writes ext[1] != 0 (checked: `git grep ext\[1\]` at
   v1.2.0 and v2.0.0-beta.1 finds only the compaction reads; rust/src has no writer). So every
   existing page has no marker and no scoped erase stroke.
2. **Renderer.** `render_layers` gains one pre-scan, `layered = strokes.any(is_floor_marker with
   ext[1] != 0)` (one byte test per stroke). When false, the floor list is empty and the loop runs
   today's code unchanged: same clip, same paper copy `u`, same `stroke_brush_begin`, same masks
   and pieces. brush.rs and canvas.rs are not edited at all; the scoped path only hands another
   buffer to the existing `brush_set_erase`.
3. **Writer.** Unchanged. A page with no new data is still written byte for byte as 1.0.0-beta
   writes it (test_format against tests/beta_format.h). The lasso never writes a page it did not
   edit.
4. **Caches.** `page_render_fingerprint` is unchanged (asserted constant) and unlayered pages keep
   their tags, so no kept drawing or preview is retired.
5. **Tests.** test_golden (1.0.0-beta renders) and the erase goldens pass unchanged. A new test
   renders every fixture page plus 200 seeded random unlayered pages (ink, precise and white
   erase strokes, image paper; whole, tile-masked, ink layer only, and through the OOM split) with
   a frozen copy of today's `render_layers` and with the new one, and requires equal bytes. A
   second test forces the floor path on (one floor at index 0 whose snapshot is the paper copy) on
   the same pages and requires equal bytes, so the generalisation itself is proven, not only
   bypassed.
6. **Older versions** run their own unchanged code on untouched pages.

## 6. Algorithms

### 6.1 Select mode

- A mode, not a tool id: a lasso icon in the panel and tool picker, and a bindable action
  `select` (actions, gesture map, Settings > Controller > Buttons, pen shortcuts, so a pen button
  can hold it).
- `SelState { Off, Armed, Lasso, Selected, Dragging }` beside `LmState`. It reuses `lasso_keep`
  (LASSO_MAX 4096), `lasso_screen_pt`, `draw_lasso` and `canvas_dither_poly`; no ink, no tool id.
- Selected: a dithered box around the selection and its carried link areas (drawn dashed), and an
  action bar: Copy, Cut, Delete, Paste (when the clipboard holds something), "n links". A bar button
  shows a pressed look at the down (one fast refresh of its rect, as a panel button's, kept by any draw
  made while it is held) and acts at the lift when that is still on it and no drag away, so a button
  that changes nothing visible (Copy) still answers the tap (issue #50); a cancelled contact puts it back
  as it was and acts nothing. A pen or
  finger down inside the box drags; only the outline moves (DU), clamped to the page; the strokes
  move at release. A tap outside deselects; choosing a pen leaves Select mode. A long press on
  empty page in Select mode pastes there.
- The finger's way in (owner ruling 2026-11-02: no panel button for Select): a finger held still on the
  page off a link for LINK_HOLD_MS (press_move's slop, the link card's hold) arms a lasso, a small dashed
  ring at the finger its cue (one DU, cleared with the lasso). Drawn on without lifting it is the lasso
  above, its lift selecting with the same rule, caps and bar; lifted still it pastes centred there (the
  long press, decided at the lift, nothing with an empty clipboard). Off, it enters Select mode quietly
  (no bar, the panel untouched) and the mode ends once nothing is selected. No surprises: a contact
  flagged palm-sized (TOUCH_F_BROAD) never arms (touchgest ends a palm or a resting thumb with a cancel),
  nor does a hold with the pen in range from its down on (pen_near, any pen event, or a leave under
  FHOLD_PEN_MS before the down); a pen event after the arm drops the lasso and its ring, the lift ignored.
- Any other edit, a page turn, Undo or Redo ends the selection.

### 6.2 Enclosure (`lasso.rs`, pure)

- **Polygon test.** The lasso points, closed. A crossing table: for each integer row in the
  lasso's box, the x of each edge crossing with its winding sign, sorted. Inside = non-zero
  winding (forgiving for loopy lassos). About 20k entries at most; one binary search per test.
- **Candidates:** ink strokes (tools 0 to 9 except 3) whose bbox meets the lasso box. Erase
  strokes and markers are never candidates.
- **Visible spine** (so a word whose tail was erased is judged by what the user sees). Sample each
  candidate's spine at its points and segment midpoints (segments longer than 1 mm subdivided).
  A sample p is hidden if an erase stroke j after it, in scope (floor of j before the ink stroke),
  bbox-overlapping, passes within `rcore_j - r_i(p)` of p, where `rcore = r(1 - rim) - 0.5` from
  `brush_dab` and `look_of` at the nearest point of j, and `r_i` is the ink radius there. An
  estimate for selection only, never for rendering.
- **Rule.** Let V be the visible spine length. If V >= 1.5 mm: selected if at least half of V
  (weighted by length) is inside. If V < 1.5 mm and the stroke has visible samples (i-dots,
  accents, short crossbars): selected if the centroid of its visible samples is inside. If nothing
  is visible: judged on its whole spine by the same half rule (it is invisible both ways; carrying
  it keeps the old place clean). Tune the 50% and 1.5 mm on a handwriting corpus from the
  fixtures and rust/tests/shape_corpus.
- **Links.** A link goes with the selection when:
  - its area (or its outline polygon, when it has one) has at least half of an 8 x 8 sample grid
    inside the lasso; or
  - every ink stroke inside its area is selected (at least one); or
  - for a beta.5 link without an area: every stroke with its seed is selected (all or nothing:
    if at least half its ink length is selected, all of its strokes join the selection).
- **Nothing selected** (no ink, no link): the toast of row 3.
- Cost on the dense page: about 300k samples x a binary search, plus the hidden test only near
  erase strokes: 20 to 40 ms estimated.

### 6.3 Lift: building the group L

Inputs: page P, selected ink S (in page order).

- **Floor index of an erase stroke e:** `f(e) = 0` (the page start) if e is unscoped, else the
  index of its layer's marker. e affects ink with index in `(f(e), idx(e))`.
- **Carried erasure:**

  ```
  C = { e erase stroke, not a marker :
        exists s in S with f(e) < idx(s) < idx(e),
        bbox(e) overlaps bbox(s),
        and their dab tile masks (stroke_mark_tiles, 16 px tiles) intersect }
  ```

  Carrying too many is harmless: a carried erase stroke over no group ink restores pixels already
  at its floor, so it changes nothing.
- **L** = S ∪ C in source order, all copied. The originals stay on P (Copy), and for Move/Cut only
  S leaves P; every erase stroke stays.
- **Floors of L.** Each distinct `f(e)` over C is one floor φ. Its position in L is before the
  first L element whose source index is greater than f (floor 0 is L's start). Each carried e is
  tagged with its floor. In the source, e affected selected ink in (f, idx(e)); in L it affects
  L's ink between its floor and itself: the same set. Nested floors work the same way.
- **Copy and Cut prune dead ink** from the clipboard: run `page_droppable` on a temporary page of
  L over blank paper (the group's box only); selected ink it marks as entirely erased is left out
  (it is invisible, and erased content should not travel to another notebook). Move keeps it (it
  is invisible either way, and skipping the test keeps moves fast). If the test would exceed
  300 ms (more than about 200 strokes), it is skipped and logged.

### 6.4 Place: appending L at the end of the destination

`place(P, L, dx, dy, scale)`:

1. Translate (and scale, only for an oversized group) every point; markers stay at (-8192,
   -8192). Recompute bboxes with `stroke_append` (as `scale_page` does). Assign seeds (6.6).
2. Walk L in order over the page canvas C (valid, equal to P rendered; for a move, the source
   already removed):
   - **At each floor position φ:** `R_φ` = union of the bboxes of φ's erase strokes, clipped to
     the page. Copy C over `R_φ` into `F_φ` (a memcpy). If `F_φ` equals the destination paper on
     every pixel of those strokes' dab tiles (dry run, `stroke_mark_tiles`), φ is **plain**: its
     strokes keep layer 0 and no marker is written. Otherwise take a free layer g, emit a marker
     with layer g before φ's first element, and give φ's erase strokes layer g. When the page's
     template picture is missing (the paper cannot be compared), every floor is scoped. Scoped is
     always correct in beta.2; plain is only for older-version fidelity.
   - **Draw each stroke onto C:** ink as ever; plain erase strokes with the paper buffer; scoped
     erase strokes with `brush_set_erase(F_φ, R_φ.x, R_φ.y, R_φ.w)`, clipped to `R_φ`.
3. `page_add` the output at the end (indices n .. n+m-1) and record the history step.
4. A test asserts C after `place` equals a full `page_render` of the new page (whole and tiles).

Why plain stays exact: where `F_φ` is the paper, restoring toward the floor and toward the paper
give the same bytes. Later edits never put ink under the group: new strokes are appended after
it, history is linear (nothing older than the paste comes back without undoing it first), and a
later move takes ink to the end.

Why the end of the order (and not the moved strokes' old places, as the brief's "reuses Replace"
says): at its old index, a moved word would be erased by erase strokes drawn later at its new
place, and sit under ink drawn later there. At the end it sits on top, like any object put down.

### 6.5 Renderer: floors in `render_layers` (page.rs)

When the pre-scan finds a marker with ext[1] != 0:

1. **Pre-pass.** For each layer g with a marker and scoped erase strokes that are drawn and meet
   the region r (and the mask), `u_g` = union of their bboxes ∩ r. Reserve each snapshot buffer
   before drawing the paper, together with the paper copy; out of memory splits r in two exactly
   as the paper copy does today (page.rs:347).
2. **Main loop.** Before the skip checks for stroke i: if i is layer g's marker and `u_g` is not
   empty, copy the canvas over `u_g` into `F_g` (rows of `c.px`). Inside a tile-masked render the
   canvas outside the marked tiles holds old pixels, but no dab reads or writes there (the existing
   argument for the paper copy).
3. A scoped stroke draws clipped to `u_g` with `brush_set_erase(F_g, u_g.x, u_g.y, u_g.w)`.
   Unscoped erase strokes and ink run unchanged.
4. **Ink layer only** (`?layer=ink`): ordinary erase strokes restore toward white as today; floors
   are snapshots of the ink canvas, which is coherent.
5. **Pieces path** (compaction): the same `brush_set_erase` swap, wired and tested though
   unreachable while layered pages are not compacted.
6. **`page_render_from(c, p, region, dpi, first, paper)`**: draws strokes `first..n` onto a canvas
   that already holds `0..first`, with the same floor logic. Paste and move use it (via `place`) to
   draw only the group.
7. **C tree:** mirror the floor branch in src/page.c with a c-test case. `make check` does not gate
   the C tree; if it does not fit the budget, record it in the BACKLOG.

Audit every consumer of erase strokes (`rg stroke_is_erase rust/src`) for a marker that must not
be translated onto the page, scaled onto it, hit, linked or dropped: selection, Copy page (lane A
copies pages verbatim, markers included), `scale_page`, link boxes, export, grimoire-pull. Each
gets a test.

### 6.6 Seeds and links

- **Ink:** keep the seed if no other stroke on the destination page (the originals excepted in a
  move) and no link there uses it; else `nav_new_seed`. Free seeds keep the look; a clash re-rolls
  only jitter and dust.
- **Erase copies and markers:** keep their seeds exactly (texture byte, hash bits). Links never
  key on erase strokes (`link_box` skips them).
- **Links on paste:** a fresh seed from `nav_new_seed` (unique against every ink and link seed of
  the destination page and every seed given in the same paste). The ink strokes of a carried
  beta.5 ink link take that seed. A stroke that carries a link seed whose link does *not* travel
  gets a fresh seed (it leaves that link).
- **Links on move within a page:** same seed, area and outline moved by (dx, dy).
- **Targets:** unchanged within a notebook. Across notebooks:
  `nav_target_across(target, src_nb, dst_nb)`: `p<id>` becomes `n:<src>:p<id>`, `n:<dst>:p<id>`
  becomes `p<id>`, anything else is unchanged; `index` is left out with a toast (2). The function
  does not exist at e691c43 (checked on every lane): whichever of lanes A and B lands first adds
  it to nav.rs with its unit tests and the other rebases onto it.
- Areas and outlines are translated, scaled with the group when it is scaled, rounded outward,
  clamped to the page and to 6000, and validated with `area_valid` / `outline.within`.
- Limits (64 per page, 4000 per notebook) and links.ini writability are checked before anything
  changes.

### 6.7 Paste position

- Where the group was, if that spot is on screen and on the page; else centred in the view.
- On the page it was copied from, while the originals are still there, offset 3 mm right and down
  so it reads as a copy.
- A long press pastes centred on the press point.
- The group box (link areas included) is clamped onto the page; scaled down only if it does not
  fit (row 4).

## 7. Undo and history

New in-memory kind `ActKind::Lasso` (history is not stored on disk):

- `removed`: the selected ink with ascending original indices, ids, owned while the step is done.
- `added`: ids of the placed strokes, contiguous from index k, owned while undone.
- `link: Option<LinkEdit>` (records before and after).
- `patch`.

Apply: undo = take off `added`, then put back `removed`; redo = the reverse (`take_off`,
`put_back`, `find_stroke` reused as they are). `bbox_of` covers both sets for a trimmed patch.

| Gesture | removed | added | links |
|---|---|---|---|
| Move | the selected ink | L at the end | areas and outlines moved: (before, after) |
| Paste | none | L at the end | (None, new link) records |
| Cut, Delete | the selected ink | none | (link, None) for carried links |
| Copy | not a step | | |

- The editor brackets each gesture in `step_begin` / `step_end` (old tiles ∪ new box), so one
  patch serves the step and undo/redo swap it (instant). `PATCH_CAP` trimming applies as ever; a
  trimmed step redraws through `draw_page_without` / `render_layers`, which are floor-aware.
- links.ini is saved inside the step. If that save fails, the page change is rolled back and a
  toast explains, so ink and links never disagree.
- **`undo_redo_link` must be generalised.** Today (editor.rs:5290) it swaps no patch and redraws
  Clear steps with `render_page_full`. For a Lasso step with links it must swap the patch exactly
  like `undo_redo`, and its "clearing" rollback becomes "the step has strokes" (a failed links.ini
  save reverses the ink change and keeps the step).
- A cross-page move is Cut on page A plus Paste on page B: two gestures, two steps in two
  per-page histories.
- Layer allocation skips layers owned by any history step of the page (3.1).

### 7.1 Drawing at commit

- **Old place:** blit the `under` buffer if valid, else `draw_page_without` over the removed
  strokes' tiles (the known dense-page slow path).
- **New place:** `place` draws only L onto the page canvas (`page_render_from` logic).
- **`under` buffer** (from the scoped-backdrop design): after a commit, keep the page canvas
  without the group over the group's box. While the selection is exactly the last strokes and
  nothing else changed, the next drag of the same selection (and a drag right after a paste) is a
  memcpy plus the group's own draw: instant even on the dense page.

## 8. Performance on the Kindle (estimates; the builder measures and logs them)

Scale: a dense page redraws in about 1.2 s; 128 points cost 3 to 15 ms depending on the tool.

| Operation | Light page | 1,090-stroke page |
|---|---|---|
| Lasso lift, enclosure with visible spine | < 10 ms | 20 to 40 ms |
| Carry analysis (bbox prefilter, dry-run tile masks) | < 5 ms | 50 to 300 ms with many erase strokes |
| Plainness test and floor snapshot | < 2 ms | < 2 ms |
| Copy (lift, dead-ink pruning, clipboard write with fsync) | < 60 ms for a word | 0.3 to 0.6 s for a large selection |
| Paste (draw only the group) | 10 to 100 ms for a word | same |
| First move (old place redraw) | < 100 ms | 0.5 to 2.5 s for a word (the BACKLOG slow path: rc.11 measured about 2.6 s for a page-crossing marker stroke's tiles) |
| Later drags of the same selection, paste then drag | memcpy + group draw | same |
| Undo/redo (patch swap) | < 20 ms | < 20 ms |
| Renderer on unlayered pages | one byte test per stroke | one byte test per stroke |
| Renderer on layered pages | + one memcpy per floor in view (a word's floor is about 100 KB) | same |

Log line per gesture: `lasso: <op> n=%d carried=%d scoped=%d src_ms=%u dst_ms=%u`. Numbers are
reported, not hidden; `work_pump` keeps the pens live during long redraws.

## 9. Risks

- **Older versions and scoped groups.** 1.1, 1.2.0 and 2.0.0-beta.1 show a scoped group's erasure
  also lightening the destination ink under its gaps (screen only; kept on re-save; beta.2 shows
  it right). A 1.0.0-beta re-save zeroes the layer bytes, after which that erasure lightens the
  ink under it in beta.2 for good (no stroke is deleted; a later compaction could then drop
  destination ink that became fully erased). Limited by construction to pasted erased ink that
  overlaps existing ink. Documented in Moving between versions.
- **Grain.** Textured tools take the tooth of their new spot (absolute tooth, absolute dust). Same
  strokes, not the same pixels. A per-group tooth origin would need per-group data: BACKLOG.
- **Dense-page moves** are slow at the old place on the first drag (0.5 to 2.5 s estimated).
- **render_layers is the hottest code.** Guarded by the bypass, the frozen-copy identity test, the
  forced-path test, place-equals-render, tile/OOM/ink-layer equality tests and
  `editor.live_matches_render` in every lasso scenario.
- **Markers are visible to other code** (third-party readers, grimoire-pull, lane A's page copy).
  Each consumer in rust/src gets a test that a marker is never moved onto the page or dropped.
- **Enclosure tuning** (half the visible spine, 1.5 mm) surprises users more than anything else
  if wrong. Tune on real handwriting; the visible-spine estimate can misjudge a stroke grazed by
  an eraser's soft rim.
- **Layer exhaustion and dead floors.** Not reclaimed until compaction learns layers; layered
  pages keep erased ink in their files longer (privacy note in FORMAT).
- **f32 translation** may change a rare pixel by one gray level; tests count it.
- **Lane coordination:** `nav_target_across` (lanes A and B), lane A's page copy must carry
  markers verbatim, lane C may define an index target across notebooks.
- **C tree drift** if src/page.c is not mirrored (outside `make check`).

## 10. Test plan

**Unit tests (`lasso.rs`, pure)**
1. Crossing table against an exact point-in-polygon test; non-zero winding on figure-8 lassos.
2. Enclosure on a handwriting corpus: dots and crossbars come along; a long underline crossing the
   lasso does not; a cursive line whose tail was erased is selected by its visible part.
3. Link selection: area half inside, outline-masked grid, all ink in the area selected, beta.5 all
   or nothing.
4. Carry analysis: erase before or after the ink, outside the floor, nested floors, white ink that
   erased only unselected ink (not carried), white ink crossing both (carried and kept).
5. Seeds: kept when free, fresh on ink or link clash, beta.5 seed sharing, erase seeds kept.
6. `nav_target_across` cases; `index` left out; translation, scale-to-fit, clamping.
7. Layer allocation, including layers owned by history steps; refusal at 255.

**Render (`page.rs`)**
8. Frozen-copy identity: every fixture page plus 200 random unlayered pages, whole, tile-masked,
   ink layer only, OOM split: equal bytes. Fingerprint constant unchanged. Goldens unchanged.
9. Forced floor path equals the old path on the same pages.
10. Scoped semantics: a scoped group over blank paper equals the flat render; over ink, destination
    pixels under fully erased group pixels equal the floor exactly and are never lighter than it
    anywhere.
11. `place` canvas equals a full re-render (plain, scoped, nested, masked); `page_render_from`
    equals the full render; `page_render_tiles` equals `page_render` on layered pages.
12. Move oracle (Blank paper, offset a multiple of 1024 px, coordinates on 1/16 px so translation
    is exact): the destination over empty paper equals the L-only render translated (graphite
    included); the source region equals P rendered without S; where the L-only render is paper,
    destination pixels are unchanged.
13. Marker robustness: missing marker (ordinary), duplicate marker (first wins), scoped stroke
    before its marker (ordinary), marker scaled by `scale_page` stays off page.

**Format and store**
14. A layered page round-trips byte for byte through the current reader/writer; through
    tests/beta_format (1.0.0-beta) it keeps the marker and every point with ext zeroed.
15. `holds_newer_data` refuses layered pages; an unlayered lasso page is compacted normally.
16. Cache tags: `|L` for layered pages; an old-style tag on a layered page is not trusted;
    unlayered tags unchanged.
17. Clipboard: round trip; truncated file, bad `CLIP`, over-cap refused with no change; a failed
    write cuts nothing; gone after a restart (RC review, 2026-11-04); cross-notebook targets rewritten.

**History**
18. Lasso undo/redo for Move, Paste, Cut, Delete: exact stroke identities and order, the patch
    swap equals a re-render, link records restored, a failed links.ini save rolls back, one step
    per gesture, Copy pushes nothing.

**Scenarios (`tests/scenarios/*.qs`, headless, `expect editor.live_matches_render is 1` after
each step)**
- `lasso_select_move`; `lasso_move_erased` (precise eraser crossing the word and a neighbour: the
  neighbour stays erased, pixel hashes at both places); `lasso_paste_over_ink` (row 2, pixel
  expect: destination not lightened); `lasso_whiteout_only` (toast, nothing changes);
  `lasso_correction` (white-out, fix written over it, fix moved alone); `lasso_cut_paste_pages`;
  `lasso_paste_other_size` (1860 x 2480 into 1986 x 2648); `lasso_link_copy_across` (links.ini
  checked, a tap follows the rewritten target); `lasso_link_move` (same seed, area moved);
  `lasso_readonly`; undo/redo after each; `lasso_dense_timing` (the 1,090-stroke fixture; the log
  line asserted present); Spanish strings via `make l10n-check`.

**Revert checks** (the new tests must fail without the change)
- Force every floor plain: row 2 tests fail.
- Drop the floor in `render_layers`: place-equals-render and the layered scenarios fail.
- Drop link carrying: link tests fail.
- Disable the visible spine: the erased-tail enclosure test fails.

**Gate:** `GRIMOIRE_TEST_PORT=<port> make -j15 check 2>&1 | tee build/gate-claude-b2-lasso.log`,
reported honestly with failing names; a known flaky scenario named, rerun once, reported.

## 11. Implementation plan (one builder, about 6 to 7 days, about 2,800 lines with 1,400 of tests)

Each step committed with its targeted tests.

1. **Renderer floors (1 d).** `is_floor_marker`, marker constructor, the pre-scan, snapshot
   buffers in the pre-pass and OOM split, the loop hook, Pieces wiring, `page_render_from`. Tests
   8 to 11 and 13, including the frozen copy of today's `render_layers` (kept in the test module).
2. **`lasso.rs` pure (1.25 d).** Crossing table, visible spine, enclosure, link selection, carry
   analysis and floors, seed plan, translate/scale/clamp. Tests 1 to 7.
3. **`lift` / `place` (0.75 d).** On Page + Canvas, no UI: floors plain or scoped, dead-ink
   pruning for Copy/Cut. Tests 11, 12.
4. **History (0.5 d).** `ActKind::Lasso`, `history_push_lasso`, layer ownership query. Test 18.
5. **Clipboard (0.5 d).** `clipboard.rs`: write/read root/clipboard.qpg with `CLIP`, refusals.
   Test 17.
6. **nav (0.25 d).** `nav_target_across` if lane A has not landed it; carried link records.
7. **Store (0.25 d).** `layered` flag on kept drawings and previews; compaction refusal test.
   Tests 14 to 16.
8. **Editor (1.5 d).** SelState, panel icon, `select` action (actions, gesture map, controller,
   pen shortcuts), box and bar with "n links", outline drag, commit through `step_begin` /
   `step_end`, `under` buffer, paste position, read-only gating, `undo_redo_link` generalisation,
   timing log, toasts through `tr()` with Spanish in po/es.po (`make l10n`, `make l10n-check`).
9. **C tree (0.25 d, or BACKLOG).** src/page.c floor branch and a c-test case.
10. **Docs (0.25 d).** FORMAT.md (3.6), DESIGN.md (Select mode, scoped erasing, placement),
    SHORTCUTS.md (`select`), TESTING.md (new scenarios), BACKLOG.md: move lane B to shipped; add
    deferred items (below) with "decided by the lane B design, 2026-11-02, owner to confirm".
11. **Scenarios, revert checks, then one full gate** at the tip.

Deferred to the BACKLOG:
- floor-aware compaction and dead-layer reclaim;
- a per-group tooth origin (pixel-identical grain after a move);
- "Remove erasure" on a selection, and white-out as an object (owner options);
- a faster old-place redraw on dense pages;
- an index target across notebooks (with lane C);
- a dragged-ink preview instead of the bare outline.

## 12. Where this departs from the beta.2 brief

- **Item 3** (move reuses Replace, keeping stroke order): moved strokes go to the end (6.4).
- **Item 4** (scale like `scale_page`; always fresh seeds): 1:1 paste, scaled only to fit; seeds
  kept when free, fresh on clash (6.6).
- **Item 5** (carried erase strokes selected like ink and lightening destination ink): replaced by
  scoped erasing; erase strokes are never selected as objects. Owner's "nothing unexpected".
- **Item 6** (copy carries ink only): links are copied with target, area and outline. Owner's
  "copies the link and contents exactly".

## 13. Owner questions (none blocks building)

1. White ink lassoed alone selects nothing (erasure semantics, 2.1). Alternative: white ink that
   covers only unselected ink becomes a movable object that uncovers at its old place.
2. Paste into another page size is 1:1, scaled only to fit.
3. An `index` link pasted into another notebook is left out with a toast, until a target syntax
   exists.
4. Cut and Delete take a carried link with its ink (shown as "n links" on the bar).

## 14. How the three candidate designs were judged

Checked in the code at e691c43 and at the release tags:

- `brush_set_erase` takes any restore buffer, and `render_layers` builds one paper copy `u` and
  clips erase strokes to it (page.rs:330-439): the floor generalisation fits without touching
  brush.rs or canvas.rs. Restore is lighten-only and never past its target (canvas.rs:685), so a
  floor can never lighten destination ink.
- No writer of ext[1] exists; compaction refuses ext[1] != 0 in beta.2, 1.2.0 and 2.0.0-beta.1.
- The 1.1+ stroke eraser never removes erase strokes (editor.rs:3630).
- `nav_target_across` does not exist; `nav_external_target` has no index form.
- `undo_redo_link` swaps no patch and redraws Clear in full (editor.rs:5290): it must be
  generalised (a finding of the floors design).
- 2.0.0-beta.1 keeps dense-page drawings under the same fingerprint, so a layered page needs its
  own tag suffix that the loader checks with the page in hand.

**Scoped backdrops with group tags** (score 7.5): exact and close to this design. Ink and erase
strokes of a group share a tag in contiguous runs, with a `grp1` safe chunk for nesting. Its claim
that deleting a group's first stroke keeps the backdrop is correct. Weaker: a page-level chunk and
a parent table for nesting, a bbox-based need test (scopes more than needed), more partition and
flattening logic. Grafted: the unified history kind, the single-file clipboard, the forced-path
identity test, `page_render_from` and the `under` buffer, layer allocation that skips
history-owned strokes.

**Bake to geometry with gap runs** (score 6): no format meaning added. Its core trick is verified
exact in the current brush (a segment over 20000 px is skipped after `brush_dab` has already set
the radius, the dab index and the nib, brush.rs:628 and stamp_as, identical at all four tags).
Refuted or weak: "front" placement changes deposit order, which is not commutative
(`canvas_deposit_px` is a lerp): pencil and charcoal over pencil and every anti-aliased rim
differ, not only highlighters; the bake fallback is approximate (round caps, lost soft and
textured erasing), a visible surprise; gap-run strokes have bboxes reaching (30000, 30000), so
every version, beta.2 included, replays them on every redraw and the paper copy grows to the
whole region; it depends on a corrupt-data guard being frozen forever; 1.0.0-beta's stroke eraser
may hit the invisible lines to the hubs; 9 to 12 days. Grafted: the visible-spine selection,
dead-ink pruning for the clipboard, the correction-workflow argument for white-out, rollback of
the ink change when links.ini fails to save.

**Scoped floors with a marker** (score 8, the base): exact, no chunk, nesting falls out of
per-stroke floors, pixel-based plainness test, ink never tagged, and it found the
`undo_redo_link` gap. Changed here: its white-out-as-object rule contradicts how Grimoire
presents white ink and can uncover a hidden mistake; its two-file clipboard became one file; its
dismissal of contiguous ink tags was mistaken (correct only for its own untagged-ink model), which
does not affect the choice.

Contributor content: none was read for this design, and no injected instructions were seen.
