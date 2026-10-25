# 1.1.0-rc.3 validation

The owner's Scribe 2024 now runs firmware 5.20.1.0.2, with KOReader v2026.07.2. Its new
`kindle-input-pt` virtual device exposes keyboard capabilities and no absolute axes. KOReader
retains its file descriptor during handover, after releasing the actual pen and touch devices.
RC2's launcher rejected every retained input descriptor, so the app never started.

The launcher now permits a retained input descriptor only when sysfs positively reports a
zero absolute-axis bitmap. It checks every descriptor: a keyboard cannot hide a retained pen
or touch device later in the list. Missing, empty, whitespace-only and malformed bitmaps remain
a refusal. Regression cases exercise these paths under sh and dash alongside the existing
ancestry, session, native PIN, installer and recovery checks.

The launcher-only hotfix was installed with the original saved under the device's
`kindle-lab/fw520-launch-fix/`. The owner confirmed opening Grimoire and returning to KOReader.
The log records the first frame, successful pen/touch acquisition and clean status-0 return
on 5.20.1.0.2. It also records the changed pen node from event5 to event6; discovery remains
dynamic. The native RC2 executable worked after the launch guard was corrected.

The owner then reported a missing screen strip after KOReader reinitialization, restored by
opening/dismissing its menu or a full refresh. A successful handover now leaves a one-use return
receipt. On the next KOReader start the plugin consumes it and schedules one complete repaint
after 250 ms, through KOReader's own dirty/refresh queue. Ordinary startup and page-turn refresh
policy remain unchanged. Plugin tests check deferred full repaint and receipt consumption.
Physical validation of this repaint is recorded after deployment below.

Build delivery uses `make dist`, with the owner's shared primary-project dist directory.
The bundle includes installer and optional patch ZIPs, KPM package/manifests, source identity
and checksums. The physically tested RC2 is retained separately. Source-only HID work from
PR #2 is not included in this release-candidate repair.
