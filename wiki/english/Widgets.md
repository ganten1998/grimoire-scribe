# Widgets

Widgets are small windows that sit over a notebook: a calculator, a timer, a clock. From 2.2 a
widget is a package you install, so the list can grow without updating Grimoire itself.

> **The calculator is the only widget that runs in 2.2.** A WebAssembly widget can be installed and
> appears in the catalog, but until Grimoire has a WebAssembly runtime its frame shows its name,
> version, author and description instead of running. The calculator's code is built into
> Grimoire, but it is still installed as a package.

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
come back after a restart. The calculator's current number belongs to its frame: minimizing and
restoring it keeps the number.

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

- Installing is a trust decision. There is no review, no signature and no sandbox yet.
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
`editor:overlay` and `storage:widget-local` capabilities, and it cannot add native code: the only
native widget entry is the calculator.
