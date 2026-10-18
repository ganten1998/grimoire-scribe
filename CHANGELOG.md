# Changelog

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
