# Phase 3 touch study: provisional defaults, owner evidence pending

As of 2026-10-29 no owner trace has been supplied for this implementation. `TG_TUNE` uses the
brief's provisional defaults; the table below is a source snapshot, not a measured optimum.
No owner success rates, negative-fire counts or percentile measurements are available. The
finger-shape freeze gate remains **unverified on owner data**. Page shape support implemented
before that evidence is a deviation from the brief's gate, not a claim that it passed.

## Evidence and replay

`tests/fixtures/touch/synthetic.txt` and the synthetic shape corpus are explicitly synthetic
regression evidence. Their outcomes test implementation behavior; they do not demonstrate
97% real-hand swipe acceptance, zero palm/reading fires or 95% owner-shape acceptance. This
record reports no test run or measured rate from those inputs.

Collect [the owner script](../KINDLE.md#the-two-finger-study), blocks A–L at natural speed and
pressure, hands off for 5 s between blocks. Store a reviewed trace as
`tests/fixtures/touch/owner-<date>.txt` with build/date/block order in its README. Retain the
original `grimoire.log` measurement lines. `tgstudy` splits at gaps of at least 4 s; check its
block sizes before drawing conclusions. Trace pen events have no coordinates: replay treats
the far-pen test as met and judges block H on timing only. Real hover distance needs the owner
checks; it cannot be reconstructed from this trace.

```sh
# Host only; TRACE and BLOCKS.txt must be supplied from the reviewed study.
tgstudy TRACE --tune v1 --blocks BLOCKS.txt
tgstudy TRACE --tune new --blocks BLOCKS.txt
tgstudy TRACE --tune new --set swipe_mm=16 --sweep cos_min=0.7:0.8:0.05 --blocks BLOCKS.txt
```

## Current tuning (not frozen from owner measurements)

Lengths with `_mm` are millimetres; `_ms` are milliseconds. `commit_px` is the existing
300 dpi reference-pixel gate. Ratios and cosines are unitless. A disabled old rule is shown
with its actual zero/false value. `palm_major` is the raw touch-major threshold.

| Field | `TG_TUNE_V1` | Provisional `TG_TUNE` | Owner measurement / freeze |
| --- | --- | --- | --- |
| `land_ms` | `150` | `150` | Pending owner trace |
| `palm_ms` | `400` | `400` | Pending owner trace |
| `pen_window_ms` | `1000` | `1000` | Pending owner trace |
| `pen_gone_ms` | `300` | `300` | Pending owner trace |
| `far_mm` | `40.0` | `40.0` | Pending owner trace |
| `rest_ms` | `300` | `300` | Pending owner trace |
| `rest_mm` | `3.0` | `3.0` | Pending owner trace |
| `commit_spread` | `0.10` | `0.10` | Pending owner trace |
| `commit_px` | `70.0` | `70.0` | Pending owner trace |
| `zoom_ratio` | `0.6` | `0.4` | Pending owner trace |
| `zoom_opposite_mm` | `0.0` | `2.0` | Pending owner trace |
| `zoom_upgrade` | `0.0` | `0.15` | Pending owner trace |
| `zoom_after_pen` | `false` | `true` | Pending owner trace |
| `grace_ms` | `0` | `80` | Pending owner trace |
| `regrab_mm` | `0.0` | `15.0` | Pending owner trace |
| `tan_axis` | `0.577_350_3` | `0.577_350_3` | Pending owner trace |
| `swipe_mm` | `20.0` | `16.0` | Pending owner trace |
| `finger_mm` | `12.0` | `8.0` | Pending owner trace |
| `cos_min` | `0.8` | `0.7` | Pending owner trace |
| `finger_ratio` | `2.0` | `3.0` | Pending owner trace |
| `spread_max` | `0.25` | `0.50` | Pending owner trace |
| `spread_opposite` | `false` | `true` | Pending owner trace |
| `swipe_ms` | `1000` | `1200` | Pending owner trace |
| `follow_ms` | `0` | `250` | Pending owner trace |
| `tap_lift_ms` | `150` | `200` | Pending owner trace |
| `tap_ms` | `300` | `400` | Pending owner trace |
| `tap_mm` | `5.0` | `6.0` | Pending owner trace |
| `apart_min_mm` | `8.0` | `8.0` | Pending owner trace |
| `apart_max_mm` | `70.0` | `80.0` | Pending owner trace |
| `palm_major` | `i32::MAX` | `i32::MAX` | Pending owner trace |
| `hover_far_mm` | `0.0` | `60.0` | Pending owner trace |
| `hover_age_ms` | `0` | `50` | Pending owner trace |

`palm_major` stays `i32::MAX`: broad-contact size remains log-only until every fingertip in
A–I/L can be separated from every broad contact in J by at least 20%. Do not choose a size
threshold from a synthetic contact's fabricated major value.

## Decision rules for the eventual freeze

| Blocks | Required owner result | Measured result |
| --- | --- | --- |
| A–D, I | At least 97% intended swipe fires, zero wrong direction | Pending |
| E | At least 95% `2.tap` | Pending |
| F | At least 95% sessions classed ZOOM | Pending |
| G | Every drag pans | Pending |
| J, K | Zero pattern fires and zero finger-shape fires | Pending |
| L, three-sample shapes | At least 95% held-out acceptance, zero wrong shape | Pending |

For each loosened field, keep the most permissive value satisfying zero negative fires and
zero wrong direction with all other new fields enabled, bounded by the brief's provisional
table unless a measurement justifies more. Use the intended measure's 2nd/98th percentile
within that safe range. Report counts and denominators, not a rounded percentage alone.
Keep a new rule only if disabling it loses at least two percentage points of intended fires,
or it fixes a named failure. Re-grab may stay on as harmless when the trace has no matching
lift/re-land within 100 ms and 15 mm; record that absence rather than inventing benefit.

For the shape freeze, prepare eight stress shapes from L (Z, N, S, circle, alpha, triangle,
plus two synthetic stress shapes) and replay J/K for zero fires. With three recorded samples,
L's remaining examples must meet the acceptance target above, with zero wrong. The planned
`s_shape_9_traces` replay must use the reviewed owner evidence. If it fails, report it to the
owner before accepting page shape support; two-finger recognition, the registry and recording
can still proceed. Until those files and results exist, thresholds remain provisional.
