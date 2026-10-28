# Beta.5 readiness

The owner has put public releases on hold. No tag, published release or KPM feed update is authorized.
This file tracks work toward a testable candidate. Read the actual branch and deploy evidence before
assuming an older handoff state still applies.

## Current baseline

- Baseline: 72fafda, installed on the owner's Kindle as BUILD_ID 2aa3bb94ff09.
- Device install evidence: `~/KindleLab/build/deploy-beta5-72fafda/DEVICE-RESULT.json`.
- Integration work: `codex/beta5-barriers`, isolated from the baseline checkout.
- All installs require the full checks, a fresh verified backup, an inactive Grimoire process,
  unchanged notebook/settings data and a matching installed binary checksum.

## Work under way

- Default notebook status bar with notebook/page/section, links, tool, clock/battery and connection
  indicators; Corner and Off alternatives, page fitted below the strip, both orientations.
- Authenticated Live view keys with a separate `live-keys.ini` map and a help panel.
- Recovery after long pen silence and a review of timing/acknowledgement rollover.
- Gesture-new-page in the Buttons sheet; touch evidence that remains available after long use.
- Index phase 2: ink-free links, with migration and undo safety reviewed separately before integration.

## Open evidence questions

- The intermittent Live view stalls have not been explained. The earlier ping tests show bursts on
  the Kindle-to-Mac path, but stop/start comparisons also changed the radio's power-save state.
  They do not isolate the stream, radio setting, router, Bluetooth coexistence or host scheduling.
  The packet-rate comparison weakens a simple packet-rate explanation; it does not rule out all
  transport or scheduling interactions. No automatic radio workaround belongs in a candidate on
  these measurements alone. 72fafda already removed forced power-save changes.
- The palm test produced sparse application logs. These logs do not prove whether hardware,
  the kernel driver or Grimoire's recognizer suppressed the fingertip. The always-visible status
  bar removes reliance on a finger tap while writing; the rejection source remains to be measured.
- Gesture threshold tuning is requested for a later build, per the owner's notes.
- Old lasso strokes remain ordinary ink. New link creation must not deposit ink. Removing old
  ordinary strokes automatically requires an explicit policy and reliable provenance.

## Acceptance still needed

The owner must inspect the status bar on the Kindle, both orientations, and accept or reject the
new default. Check page alignment, writing near the top edge, pan/zoom, long names, menu transitions,
Live view layers and keyboard actions. This acceptance is separate from permission to publish.
