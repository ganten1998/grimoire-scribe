# Changelog

## 1.0.0-rc.1 (2026-10-17)

First public pre-release, and a new name: **Quire is now Grimoire**.

- **Gothic style**: crisp pixel type like an old console game (Jacquard 24 pixel blackletter
  titles, Jersey 10 text, drawn as exact pixel blocks), carved-stone frames, an arcade of
  pointed arches, lozenge checkboxes and sliders, a new Library cover. Settings > Style switches
  to the plain Classic look.
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
