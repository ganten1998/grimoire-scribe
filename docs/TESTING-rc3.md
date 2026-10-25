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

The owner also reported missing areas on KOReader's very first draw after a device reboot,
independently of Grimoire. The virtual node was being classified as an external keyboard;
KOReader's externalkeyboard plugin then broadcasts PhysicalKeyboardConnected, causing the
file browser to reinitialize after its initial paint. An explicit device-local repair filters
only `kindle-input-pt` from keyboard discovery/hotplug and closes its unused descriptor.
Real external keyboards retain their discovery, mappings and capability handling. This follows
the known upstream issue https://github.com/koreader/koreader/issues/16283.

The owner confirmed a complete first screen after restarting KOReader with that filter.
They also confirmed that opening Grimoire and returning to KOReader now draws the complete
screen without opening/dismissing its menu, after the handover-only workaround was removed.
Fourteen isolated checks exercise the patched plugin's actual discovery/hotplug paths, native
capability preservation, real keyboards, mixed devices and discarded-fd ownership; device
LuaJIT parsing passed. The original and reproducible patch are retained in the owner's KindleLab
kit. This KOReader repair is not bundled or installed by Grimoire. An earlier handover-only
repaint workaround did not address the fresh-start problem and has been removed. Normal
page/menu refresh settings remain unchanged.

RC3 was installed through KPM 0.2.4 (package version 1.1.5003), with notebook-file hashes,
settings and running KOReader PIDs verified unchanged. Source-only HID experiments remain
separate. Package/launcher, KPM and KOReader integration checks cover the candidate repair.

Build delivery uses `make dist`, with the owner's shared primary-project dist directory.
The bundle includes installer and optional patch ZIPs, KPM package/manifests, source identity
and checksums. The physically tested RC2 is retained separately. Source-only HID work from
PR #2 is not included in this release-candidate repair.
