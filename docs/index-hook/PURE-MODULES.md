# The Index hook's pure modules: status

Branch `claude/rust-index-pure-p4zqq5`. Three pure modules, host-tested, with no screen code and no editor or app
wiring: `rust/src/fingershape.rs`, `rust/src/shape.rs` and `rust/src/graph.rs`, each with a `mod` line and a prelude
entry in `lib.rs`.

This is an interim status, pushed early because the cloud credit is nearly gone. Each module was built by one
implementer, then reviewed adversarially against its spec and PORTING.md, then fixed; review rounds are still running.
The final version of this file will list each module's full public API and what the screen phases must call.

## State at this push

| module | spec | tests (this machine, Linux x86_64) | review |
|---|---|---|---|
| fingershape | index-hook-shape-gestures-2026-10-27.json | Phase 3 focused test and full ignored corpus/bench | Phase 3 merge; review items 1-4; slip-bank pad 10 |
| shape | final plan, plan.smart_shapes | `test_shape`: ok (984 checks) | round 1 fix partly merged (1 high, 3 medium found) |
| graph | final plan, plan.graph_view | `test_graph`: ok (445 checks, 18 cases) | round 1 fix partly merged (1 high, 3 medium found) |

The merged tree builds with no warnings (`cargo build`).

### fingershape

- Merged in Phase 3 from `claude/rust-index-pure-p4zqq5` commit `7cbcf0f`; the synthetic corpus is in
  `rust/src/fingershape_synth.rs`.
- Integer $P over 32 points, rotation tried at -10, 0 and +10 degrees in fixed point, per-gesture thresholds from the
  samples' spread, the margin and the structure guard. Exact scores remain integer-only.
- Recording consistency now requires every new sample to be within 60 of every earlier sample.
- Runtime matching also rejects open paths with one monotonic turn, using six coarse chord directions. Near-closed
  paths, paths with opposing turns, and paths turning more than once remain eligible.
- Slip-bank pad: 10. The pad-0 baseline kept five shapes and had 24 hairpin slips; pad 10 kept seven and had 7.
  Pad 10 preserves more two-turn/loop shapes while the runtime turn guard closes the remaining single-turn gap.
- Measured baseline before the runtime turn guard (12,000 tries per slip kind):

  | pad | kept | use | recording | wrong | swipe | thumb arc | slide | hairpin | drift | scrub | scribble |
  |---:|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
  | 0 | N, Z, S, triangle, spiral | 960/1000 | 1000/1000 | 0 | 3256/0 | 5951/0 | 9444/4 | 11651/24 | 6829/0 | 183/0 | 11223/57 |
  | 10 | Z, S, circle, alpha, M, triangle, W | 1324/1400 | 1400/1400 | 0 | 3256/0 | 5951/0 | 9444/4 | 11651/7 | 6829/1 | 183/0 | 11223/68 |

- Corpus gates are set to at least 92% use acceptance, at least 97% recording acceptance, zero wrong actions in at
  least 6,200 matcher comparisons, zero swipe/thumb-arc/scrub fires, at most two slide and two hairpin fires, and at
  most one drift fire per 1,000. The full test records the final counts.
- The slip-bank check, capture gates and `gestures.ini` value codec remain as reviewed. Trail drawing, line framing,
  pen-near handling and late-second-finger handling stay in the UI/gesturemap/touchgest layers.
- Not here, by design: the trail drawing, the gestures.ini line framing (gesturemap's job), "pen near" and "late second
  finger" (touchgest's).

### shape

- Done:
  - line with angle snap; the closed-shape prerequisites; rectangle, square, quad, triangle, ellipse and circle;
  - dot-grid and graph paper snapping, and the ruled-paper horizontal snap;
  - the burst gate as a parameter;
  - output points per STORAGE, and `shape_stroke`, which keeps the raw stroke's tool, params, seed and ext;
  - v1.5 arrow, polyline and arc behind `ShapeParams::extras` (off by default).
- Noisy positives, 200 per kind: 100% for every kind but square, 98%.
- Negatives: no snap on 1900 synthetic ones (cursive, letters, digits, CJK-like, scribbles) or on the owner's
  eraser-sample strokes at written size.
- Round 1 findings being fixed:
  - a grid snap of one line end undid the angle snap (high);
  - the closure gate measure;
  - the end-of-stroke dwell in the corpus;
  - the noise level of the corpus.

### graph

- Done:
  - the model from `NotebookInfo`, `Nav` and `NavLink`: the pid table, the three bands, degrees, edges deduplicated with
    direction bits and multiplicity, and The Index as badges;
  - the place_disk seed, Barnes-Hut relaxation with Navi's constants and a pump every 2 steps;
  - satellites on phyllotaxis rings, incremental placement, and the Around layout;
  - graph.ini read and write: atomic, unknown lines carried, size caps, a corrupt file refused and never overwritten;
  - the bgjob work function.
- Round 1 findings being fixed:
  - incremental drift on maps of 600 or more nodes (high);
  - a gen mismatch was not acted on;
  - the open page could keep a stale seed position;
  - satellites depended on which page was open.
- INTERFACE CHANGE proposed: `bgjob::BgKind` gains `GraphLayout = 4`, for big and Tidy-up layouts in the niced child.
  Not applied: it is another module's.

## Host-specific results (the base's own, not from these modules)

`make rust-test` on this machine (root, x86_64) fails in tests that also fail on the base commit:

- `test_annotate` (8 checks): its storage-full cases use `chmod 0555`, which root ignores.
- `test_brush` (359 checks): the brush probe fixture was recorded with Apple's libm.
- `test_bgimage:579`, `test_sheets_probe:1057` and the lib test `sim_sdl::tests::sim_probe_matches_the_c`: recorded
  float results that differ in the last bit, because `fmla` fuses only on aarch64.

Their expectations are unchanged; they are for the Mac gate.

## Left

- Finish the review and fix rounds, then merge them here.
- Run the full `make rust-test` and `make rust-kindle` on the final tree.
- Write the final version of this file.


## Phase 3 fingershape review closure

Imported only fingershape and its synthetic corpus from `7cbcf0f`; the corpus lives in
`rust/src/fingershape_synth.rs`. Recording consistency now requires every earlier sample to
be within 60. The regression includes a third sample near sample 2 but too far from sample 1.

A signed-turn structure guard rejects open single-turn paths that an unordered cloud can confuse
with S or a triangle. Six coarse chords suppress jitter; up to five degrees of reverse wobble is
not a deliberate second turn. A loop closes within one seventh of its diagonal. Scores and cloud
ranking remain integer and unchanged. The diagnostic artificial V/caret bank (which bypasses
recording refusal) now has fewer noisy fires; exact distance snapshots remain 22 and 14.

The final deterministic pad study uses 700 draws per retained shape per regime and 12,000 of
each slip kind. Counts below are measurements, not owner trace evidence:

| Pad | Use acceptance | Recording acceptance | Wrong actions | Swipe | Thumb arc | Slide | Hairpin |
|---|---|---|---|---|---|---|---|
| 0 | 3318 / 3500 | 3500 / 3500 | 0 | 0 / 12000 | 0 / 12000 | 2 / 12000 | 9 / 12000 |
| 10 | 4669 / 4900 | 4900 / 4900 | 0 | 0 / 12000 | 0 / 12000 | 2 / 12000 | 0 / 12000 |

Pad 10 remains: pad 0 exceeds the single-turn slip budget. The final full gate also uses 500
tries per retained shape, with 3328 / 3500 use acceptance, 3500 / 3500 recording acceptance,
zero wrong actions in 6986 matcher comparisons, zero swipe/thumb/scrub fires and two slide plus
zero hairpin fires in 12,000 of each kind. Its assertions require at least 92% use and 97%
recording acceptance, zero wrong actions in at least 6200 comparisons, and at most two single-turn
fires combined. These limits were strengthened from the imported corpus, never relaxed.

Evidence: `index-p3-pad-study.log` preserves the original pre-guard pad comparison;
`index-p3-pad-final.log` records the final comparison; `index-p3-corpus-full.log` records the
final full run. No owner traces have been supplied; the device freeze remains provisional.
