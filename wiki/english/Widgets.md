# Widgets

Widgets are small windows that sit over a notebook: a calculator, a timer, a clock. From 2.2 a
widget is a package you install, so the list can grow without updating Grimoire itself.

A widget's code is part of its package, not part of Grimoire. Each one is a small WebAssembly
module that draws its own window and remembers its own state; Grimoire only gives it a frame,
input and the files it may use. That is why any installed package can add a calculator, a timer, a
sticky note or something else without a new Grimoire release.

The catalog only lists installed packages, so install the ones you want from their repositories.

## Opening and moving a widget

Open a notebook and tap the **Widgets** button on the toolbar. The popup beside the panel lists the
installed widgets:

- Tap a row to open its widget, or to raise it if it is already open.
- Drag the widget by its title bar to move it.
- **−** minimizes it to a small square; tap the square to bring it back at the same size.
- The grip at the bottom right resizes a resizable widget.
- **×** closes it. Closing a widget does not remove the package.

Up to eight widgets can be open at once. Their places and sizes are remembered per device, so they
come back after a restart. A widget's own saved state comes back too: the calculator keeps its
current number through a minimize, a close and a restart.

## The calculator's modes

The calculator has three modes, chosen from the bar at the top of its window:

- **Simple** is the four-function keypad with percent, sign and backspace.
- **Scientific** reads an expression: parentheses, `sin`, `cos` and `tan` in degrees, `ln`, `log`,
  square root, powers, π and e. Type the whole expression, then tap `=`.
- **Programming** converts between decimal, hexadecimal, binary and octal, and has bitwise
  AND/OR/XOR/NOT, shifts, MOD and power. Its **Un** key opens a converter for length, mass, data
  and temperature.

Each mode keeps its own screen while you switch. A package can turn modes off; the ones that are
on are the ones the dropdown lists.

## Installing a widget

Open **Library > Settings > Packages > Widgets**, or tap **Manage widgets...** at the bottom of the
Widgets popup. There is no built-in directory to browse: a widget's project page gives the address
to enter.

1. Tap **Install from repository...**
2. Enter a GitHub repository as `user/repo`, or an `https://` address ending in `.yaml`, and tap
   **Done** on the keyboard.
3. Read the trust page. It names the widget, its author and its version. Grimoire does not review,
   vet or verify widget code, so install only from authors you trust, then tap **I trust this
   author, continue**.
4. If Grimoire finds the package's license text, read it and tap **I have read and agree**.

Grimoire downloads the manifest and then the widget, checks its size and SHA-256, and only then
writes anything. The Kindle needs Wi-Fi, and the address must be `https://`. A package can support
both Scribe models or just one.

The detail page shows the author, and the license and repository when the package declares them.
**View QR code** shows a GitHub package's address as a QR code for a phone.

## Changing a widget's settings

If a package came with settings, its detail page has a **Settings...** button. It lists that
widget's own switches and choices, such as which features are on or how it looks. Tap a switch to
turn it on or off; tap a choice to move to the next value. Each tap is saved at once, and **Reset
all to defaults** puts every setting back.

The calculator uses its settings to choose which calculator a new window opens with and which ones
the mode bar offers. If you turn off the calculator a window is showing, that window falls back to
the first one still on; turning every calculator off leaves Simple on.

## Updating

- **Check for updates** on a package's detail page checks that package.
- **Update all** at the top of the list checks every package that has a repository to update from,
  one after another.

Only a newer version is installed. Grimoire keeps the old version until the new one is in place so a
failed write can be rolled back. A package's own saved state is kept across an update.

## Removing

Open the package's detail page and tap **Remove...**, then **Remove** to confirm. Its widget is
closed and its files are deleted; your widget layout is kept. The package's own saved state
(`packages/widgets/<id>/state/`) is also kept, so installing the package again brings it back.

## Safety and privacy

- Installing is a trust decision. There is no review and no signature.
- A widget runs as WebAssembly with no network, filesystem or process access, and with a fixed
  instruction budget, so a broken widget cannot freeze Grimoire. It can draw in its frame, read
  its settings and read and write its own saved state.
- The size and SHA-256 checks catch a truncated or corrupted download. They are not a signature:
  they come from the same repository as the widget.
- Packages are fetched over HTTPS with `curl`. Grimoire has no account, cloud or telemetry, and
  the installer sends nothing except those requests.
- A widget's saved state stays on the Kindle under `packages/widgets/<id>/state/`, with the rest of
  your Grimoire data.

## Making your own

The developer guide is in the Grimoire repository:
[Writing and publishing a widget package](https://github.com/ganten1998/grimoire-scribe/blob/main/docs/WIDGETS.md).
It covers the `package.yaml` manifest, the artifact and its checksum, local testing with the
headless runner, and publishing. The helper
[`tools/widget-pack.sh`](https://github.com/ganten1998/grimoire-scribe/blob/main/tools/widget-pack.sh)
copies a built artifact into place and writes its size and SHA-256.

A package chooses a size, an icon and optionally a repository and license. It can declare only the
`editor:overlay` and `storage:widget-local` capabilities. Its module exports `widget_draw` and
`widget_tap` and draws through a small, generic host ABI; the guide lists every import and export.
Grimoire has no built-in widget implementations, so a package never needs changes to Grimoire's
source.
