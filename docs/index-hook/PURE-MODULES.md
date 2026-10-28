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
| fingershape | index-hook-shape-gestures-2026-10-27.json | `test_fingershape`: ok (37 cases; 3 long corpus and bench runs `#[ignore]`d) | round 1 fixed; round 2 found 3 medium, 2 low |
| shape | final plan, plan.smart_shapes | `test_shape`: ok (984 checks) | round 1 fix partly merged (1 high, 3 medium found) |
| graph | final plan, plan.graph_view | `test_graph`: ok (445 checks, 18 cases) | round 1 fix partly merged (1 high, 3 medium found) |

The merged tree builds with no warnings (`cargo build`).

### fingershape

- Done:
  - integer $P over 32 points, rotation tried at -10, 0 and +10 degrees in fixed point, per-gesture thresholds from the
    samples' spread, the margin and the structure guard;
  - recording checks with refusal enums: inconsistent samples, too close to another action, the slip bank;
  - the capture gates: size, path, duration, pause, speed, chord, closure, tangle, swipe;
  - the gestures.ini value codec;
  - a synthetic corpus (`rust/tests/fingershape_synth`).
- Integer math only, so scores are identical on every architecture. The tests assert exact scores, among them V
  against ^, a circle drawn either way, scribbles and unknown shapes.
- Open from review 2 (medium):
  - the slip bank refuses at T_g + 10, where the spec says T_g;
  - recording consistency lets the spread grow past 60;
  - the corpus slip gates are weaker than the spec's.
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
