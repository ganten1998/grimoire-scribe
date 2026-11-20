# Writing and publishing a widget package

Widgets are small frames over the notebook: the calculator, a timer, anything a package adds. The
editor's **Widgets** button opens the catalog; a tap on a row opens a frame that can be dragged by
its title bar, minimized, resized from its bottom-right grip and closed. Its place and size are kept
in `widgets.ini` at the data root, so it comes back where it was.

This page is the developer's guide: the repository, the manifest, the artifact, testing and
publishing. The exact field rules and the installed layout are in
[FORMAT.md](FORMAT.md#widget-packages-22-issue-200); this page explains the workflow around them.

> **In 2.2 the package manager installs widgets, but a WASM widget does not run yet.** The
> manifest, size and SHA-256 checks, install, update, remove, the frame and a package's Settings page
> are in place. There is no WASM host in this build: a `spec.runtime: wasm` package is installed,
> gets a frame, and the body
> of that frame shows the package's name, version, author and description instead of running its
> artifact. The only widget implementation that runs is the native `calculator`, whose code Grimoire
> carries itself; it reaches the catalog through a package like any other widget. A package cannot
> add native code.
>
> A package can be written and published now, but say in its README that it needs a Grimoire with
> the WASM runtime. Do not ship one as working until the host is there.

## The repository

A package is one repository with the manifest at its root:

```
my-timer/
├── package.yaml
├── timer.bin
├── LICENSE
└── README.md
```

`package.yaml` is required and must be at the root. `timer.bin` is whatever single file
`artifact.path` names; the name and extension are yours. `LICENSE` and `README.md` are optional.

An install copies only two files to the Kindle: `package.yaml` and the artifact. The license is not
copied; **View license** reads it from `license_url` or from the repository when the reader asks.
The README never reaches the device. The files a widget writes at runtime belong in
`packages/widgets/<id>/state/`, which survives an update and a removal (see
[State](#state)).

The repository must be reachable without credentials. Grimoire fetches with `curl`; there is no
token, no private-repository support and no zip or tarball distribution.

## The address

Settings → Packages → Widgets accepts two forms:

| address | what is fetched |
|---|---|
| `user/repo` | GitHub: `https://api.github.com/repos/user/repo` gives the default branch, then `https://raw.githubusercontent.com/user/repo/<branch>/package.yaml` (the fallback when the API cannot be reached is `main`) |
| `https://host/path/package.yaml` | that manifest directly; any HTTPS host works |

Only `https://` is fetched, and the address may be at most 1024 characters. The GitHub API is
unauthenticated, so a check is subject to GitHub's rate limit.

The artifact's URL is the manifest's directory plus `artifact.path`, unless `releases` has an entry
for the manifest's version, in which case that entry's `url` is used. This is why a repository can
keep its source and its built artifact in one tree, or attach the artifact to a GitHub release and
point at it.

An update re-fetches the stored install address (`source_url`). A package installed as
`user/repo` is updated through GitHub; one installed from a full `.yaml` address is updated from
that address.

## The manifest

A complete example:

```yaml
name: Timer
id: com.example.timer
type: widget
version: 1.0.0
author: Jane Doe
description: A small timer with one countdown.
license: MIT
license_url: https://github.com/janedoe/timer/blob/main/LICENSE
min_grimoire_version: 2.2.0
platforms: [all]
repository: https://github.com/janedoe/timer
capabilities: [storage:widget-local]
settings:
  - key: show_seconds
    label: Show seconds
    type: toggle
    default: true
  - key: layout
    label: Layout
    type: choice
    default: compact
    choices: [compact, large]
artifact:
  path: timer.bin
  sha256: 9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08
  size: 4
releases:
  - version: 1.0.0
    url: https://github.com/janedoe/timer/releases/download/v1.0.0/timer.bin
    sha256: 9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08
spec:
  runtime: wasm
  entry: _start
  icon: timer
  minimizable: true
  size:
    mode: fixed
    default: [220, 160]
```

| field | |
|---|---|
| `name` | required; the row and the frame's title |
| `id` | required; the package's permanent identity. Lowercase letters, digits, dots and hyphens only, at most 128, no leading or trailing dot and no `..`. The install, update and remove paths all use it, so never change it between versions |
| `type` | must be `widget` |
| `version` | required; `MAJOR.MINOR.PATCH`, with an optional prerelease suffix. The updater orders `-beta.N` and `-rc.N`; another suffix is accepted but is not ordered as newer. No leading zeroes in a number. An update is installed only when this sorts above the installed version, so bump it for every published change |
| `author` | optional; shown in the detail page and the widget's frame metadata |
| `description` | optional; shown in the list, the detail page and the frame |
| `license` | optional; the SPDX name or a short name. When its text is found, the install page asks the reader to agree before installing |
| `license_url` | optional; an HTTPS page or raw file with the license text. Without it, `LICENSE`, `LICENSE.md`, `LICENSE.txt`, `COPYING` and `COPYING.md` beside the manifest are tried, in that order |
| `min_grimoire_version` | optional; the same `MAJOR.MINOR.PATCH` form, with an optional `-beta.N` or `-rc.N` suffix. The install is refused on an older Grimoire |
| `platforms` | optional; `all` (the default when the key is omitted), `kindle-scribe-2022`, `kindle-scribe-2024`, or a list of them. An explicit empty list is refused. `all` matches every device |
| `repository` | optional; the human-facing project address. A GitHub address also gets **View QR code** in the detail page |
| `capabilities` | optional; only `editor:overlay` and `storage:widget-local` are accepted. Declare only what the widget needs. The host does not enforce them yet because it is not there |
| `settings` | optional; up to 32 settings the package declares for its own Settings page. Each has a `key`, a `label`, a `type` (`toggle` or `choice`), and a `default`; a choice also has 2 to 16 `choices`. An optional `icon` names one of the icons above |
| `artifact.path` | required; a safe relative path, at most 240 bytes: no leading `/`, no backslash, no empty, `.` or `..` component, and neither `package.yaml` nor anything under `state/` |
| `artifact.sha256` | required; 64 hexadecimal characters, over the artifact's bytes |
| `artifact.size` | required; the artifact's size in bytes, at least 1 and at most 32 MiB. It is used even when `releases` supplies the URL and digest |
| `releases` | optional; `version`, `url` (use HTTPS) and `sha256` per entry. Only the entry whose `version` equals the manifest's is used |
| `spec.runtime` | required; `wasm` or `native` |
| `spec.entry` | for `native`, must be `calculator`; any other native entry is refused. For `wasm`, kept for the runtime; this build does not call it |
| `spec.icon` | optional; one of Grimoire's icons: `calculator`, `package`, `clock`, `timer`/`hourglass`, `music`, `notebook`, `pdf`, `image`/`photo`, `files`/`folder`, `search`, `download`, `language`, `gear`/`settings`, `update`, `paper`/`page`, `library`, `link`, `live`, `gamepad`, `eye`. A `md-` prefix is accepted. An unknown or omitted icon is the widget grid, or the calculator's icon for the native calculator |
| `spec.minimizable` | optional, default `true`. Accepted for forward compatibility; this build always draws the frame's minimize button |
| `spec.size.mode` | `fixed` (default) or `resizable` |
| `spec.size.default` | optional; `[width, height]` at 300 dpi, default `[220, 160]` |
| `spec.size.min`, `spec.size.max` | `resizable` only; each side must be at least 120x80 at 300 dpi and at most 70% of the editor canvas either way (about 1302x1736 on a 1860x2480 Scribe). `min` may not exceed `max`. A `fixed` widget uses `default` for all three |

Unknown top-level keys are ignored, so a newer manifest can carry fields an older Grimoire does not
know; unknown keys inside `spec` are ignored too. Values of the known fields are checked where
they affect the install: a missing name, a bad id, version, path, digest or size, an unknown
platform, capability or runtime is refused. An unknown icon falls back to the widget grid.

Limits: the manifest may be at most 256 KiB, the artifact at most 32 MiB, at most 64 packages may
be installed, and at most 8 widget frames may be open at once. A `wasm` artifact is one file; a
release asset is a single file, never an archive.

### Sizes

`spec.size` values are pixels at 300 dpi, the Scribe's panel. Grimoire scales them by the panel's
dpi when it draws the frame, so the widget is the same physical size on every device. The floor is
120x80 at 300 dpi; the ceiling is 70% of the editor canvas in either direction. A frame is placed
at the top right of the editor, cascading down as more open, and is clamped onto the screen after
a rotation or a window-size change.

`mode: resizable` gives the bottom-right grip and clamps to `min` and `max`; `mode: fixed` omits the
grip and keeps the frame at `default`.

### Settings

A package can declare a Settings page of its own. When `settings` is not empty, the package's
detail page gets a **Settings...** button above **Check for updates**; opening it lists one row per
setting. A `toggle` row has a switch. A `choice` row shows the current value after its label, and
tapping it moves to the next choice. There is no separate Save button: a tap writes the new value
immediately, and **Reset all to defaults** puts every value back.

```yaml
settings:
  - key: show_seconds
    label: Show seconds
    type: toggle
    default: true
    icon: timer
  - key: accuracy
    label: Accuracy
    type: choice
    default: seconds
    choices: [seconds, tenths]
```

| setting field | |
|---|---|
| `key` | required; at most 64 characters of lowercase letters, digits, `.`, `-` and `_`; no leading or trailing dot and no `..`. It is the key the widget reads and must be unique within the package |
| `label` | required; the row's text, at most 160 bytes |
| `type` | required; `toggle` or `choice` |
| `default` | optional; `true` or `false` for a toggle (default `true`), or one of the choices for a choice (default the first) |
| `choices` | a choice needs 2 to 16 unique values, each at most 64 bytes; the row shows the chosen value |
| `icon` | optional; an icon name as in `spec.icon`; the gear when omitted |

Values are saved in `packages/widgets/<id>/state/settings.json`, beside the widget's own state, and
an update or a remove that keeps state carries them along. The file holds strings, keyed by the
setting's `key`; a missing or invalid value reads as its default. That path is reserved for the
host: a widget should keep its other data in another file under `state/`.

The file is the contract for the package's runtime: a future WASM host reads it as the widget's
settings map. This build has no WASM host, so a WASM package cannot read it yet; the page and the
saved values are ready for the host. A native widget can read the same file, but the calculator
does not declare settings yet.

## The artifact

The artifact is an opaque file to this build: there is no signature and no inspection beyond the
size and SHA-256 check. The installer downloads it (at most 32 MiB, with a 45-second timeout),
checks the length and digest, and only then writes anything. A mismatch refuses the install with
nothing changed.

For a WASM widget the same bytes are meant to work on every supported Scribe; there is no
Kindle-specific build and nothing to cross-compile for armhf. Build a self-contained WebAssembly
module with whatever toolchain you like and name it at `artifact.path`. Keep it to one file. A
module that imports nothing is the safest shape: there is no host ABI to import from yet.

Because the host is not in this build, an artifact cannot be exercised on the Kindle yet. Do not
work around that by pointing `spec.runtime` at `native`: the only accepted native entry is the
calculator, and a package cannot ship executable code for the host.

`calculator.bin` in the fixture is 27 bytes and is not executable; it exists to exercise the
package path. The installer never executes or parses an artifact.

## Packaging

Build the artifact first. Then, from a Grimoire checkout:

```sh
tools/widget-pack.sh path/to/package path/to/built.bin
```

The script copies the built file to the manifest's `artifact.path`, writes `artifact.size` and
`artifact.sha256`, and verifies the result. Run it without the second argument to check an existing
pair:

```sh
tools/widget-pack.sh path/to/package
```

In the manifest it only touches those two numbers. Bump `version` yourself, and run it again after
every rebuild.

The manifest must use the block form of `artifact:` with `path`, `sha256` and `size` present; the
script refuses an inline mapping rather than rewriting YAML it cannot see.

A release entry has its own `sha256` and URL. The script does not update it: if `releases` has an
entry for the current version, update that entry by hand (or leave it out and let the artifact be
served from the repository tree). The `artifact.size` must still describe the file at the release
URL.

### Versioning

`version` is the package's version, not the artifact's. Bump it whenever the artifact or the
manifest changes: an update is installed only when it sorts above the installed version. A
prerelease such as `1.2.0-beta.1` sorts below `1.2.0`. Use `-beta.N` or `-rc.N` for one: another
suffix passes the manifest check but the updater does not order it as newer.

Tags and GitHub releases are optional. If you use them, the release entry's `url` must be the
artifact file itself, and its `sha256` and the manifest's `artifact.size` must describe that file.
A release page URL is not an artifact URL.

### License

Set `license` to the license you actually publish under. When the text is found, the install page
shows it and asks the reader to agree before the download starts. The text is read from
`license_url`, or from one of the conventional file names beside the manifest. If only the name is declared and no file is found, the
install still proceeds; a declared `license_url` that does not answer stops it. Keep the license
file in the repository even when you set `license_url`, so the source tree is self-contained.

## Testing

There is no interactive Rust window. The Rust build is exercised by the headless runner, which can
take PNG screenshots of what it drew. The C simulator (`make sim`) predates widgets and does not
have them. The fixtures and scenarios under `tests/` are the working examples:

- `tests/fixtures/widgetcalc/` is an installed native calculator package (`packages/index.json` plus
  `packages/widgets/com.grimoire.calculator/`).
- `tests/fixtures/widgetpkg/` is a minimal WASM package, used for the install-path checks.
- `tests/scenarios/z_widgets.qs` opens the catalog, opens the calculator, minimizes, restores,
  drags and closes it.
- `tests/scenarios/z_widget_packages.qs` drives install, update, license and remove through the
  package manager.

A scenario can copy a fixture into a fresh data root:

```
# copy: tests/fixtures/widgetcalc .
```

To try your own package without publishing it, make a fixture with the installed layout and add its
entry to `packages/index.json`:

```sh
mkdir -p /tmp/mypkg/packages/widgets
cp -r path/to/my-timer /tmp/mypkg/packages/widgets/com.example.timer
# then add the entry to /tmp/mypkg/packages/index.json (see FORMAT.md)
```

The entry's `path` is relative to the data root (`packages/widgets/<id>/<artifact.path>`), `sha` is
the installed artifact's digest, and `source_url` is the address an update should re-fetch. Then
run a scenario with `# copy: /tmp/mypkg .` (or copy it to `GRIMOIRE_ROOT` yourself) and inspect the
shots:

```sh
make scenarios SCENARIOS="tests/scenarios/z_widgets.qs" SCEN_JOBS=1
```

The runner writes screenshots under `build/shots/<scenario>/`. `make headless` builds the runner.
Useful environment for widget checks:

| variable | |
|---|---|
| `GRIMOIRE_ROOT=DIR` | the data root (default `./grimoire-data` off the Kindle) |
| `GRIMOIRE_WIDGET_PLATFORM=kindle-scribe-2022` | stands in for a device's model, for `platforms` checks; off the Kindle it is otherwise `unknown` |
| `GRIMOIRE_SCRIPT=FILE` | the scenario to run in a direct `grimoire-headless` invocation |
| `GRIMOIRE_SHOTS=DIR` | where screenshots go in a direct invocation |

The platform values are `all`, `kindle-scribe-2022` and `kindle-scribe-2024`. On a Kindle the model
comes from `/proc/device-tree/model`; the environment variable is the simulator's and a test's way
to stand in for it.

The scenario scripts cannot name a row longer than 32 bytes, so `packages.row.com.grimoire.calculator`
is not a valid panel name. Tap its coordinates instead, as `z_widget_packages.qs` does.

## Publishing

1. Put `package.yaml` and the artifact at the repository root and push them to a public
   repository. GitHub's default branch is used for `user/repo`; a direct raw URL works on any
   branch.
2. Check the install from a clean data root: Settings → Packages → Widgets → **Install from
   repository...**, enter `user/repo`, read the trust page and the license, and install.
3. Open the widget from the catalog, move and resize it, close it and reopen it. If it has a
   license, check **View license**. On a GitHub package, check **View QR code**.
4. Publish a new version by changing the artifact and `version`, re-running the packaging script,
   and pushing. Existing readers use **Check for updates** or **Update all**.
5. Do not move or rewrite a published version. If the artifact was wrong, publish a new version.

Removal deletes the installed manifest, artifact and frame, and keeps `state/`. An update keeps
the old directory as `<id>.previous` until the new one is in place; a failed write rolls back.

## State

A package that needs to remember something across launches writes it under
`packages/widgets/<id>/state/` (the WASM capability for this is `storage:widget-local`). That folder
is not part of the artifact and is never overwritten by an update. Removing the package keeps it,
so reinstalling restores the widget's data. Nothing else of the package remains after a removal.

Keep state small and treat it as the reader's data: it is not backed up, not uploaded, and not
shared between packages. There is no state migration helper; a widget that changes its format must
read the old one and write the new one itself.

## Trust and safety

Installing is an explicit trust decision: the trust page names the package and its author, and the
install only starts after it is accepted. Grimoire checks that the manifest is well formed, that
the platform matches, and that the artifact's length and SHA-256 match the manifest, but it does
not review the code and cannot tell a good widget from a malicious one. Install from sources you
trust, and read the manifest before installing.

The manifest and artifact are fetched over HTTPS. The SHA-256 protects against a truncated or
corrupted download; it is not a signature, and it comes from the same repository as the artifact.
There is no package signing and no revocation list.

A widget is meant to be confined to its frame and its state folder. Until the WASM host is built
there is no runtime to confine, which is the main reason not to treat a current WASM package as a
working widget.

## Checklist

- [ ] `package.yaml` at the repository root, `type: widget`
- [ ] a stable `id`, a new `version` for this publish
- [ ] an artifact at `artifact.path`, one file, at most 32 MiB
- [ ] `artifact.size` and `artifact.sha256` match it (`tools/widget-pack.sh`)
- [ ] `license` and a license file or `license_url`
- [ ] `spec.runtime` and `spec.entry` are ones this build accepts
- [ ] `spec.size` within 120x80 and 70% of the canvas
- [ ] `platforms` correct, `min_grimoire_version` honest
- [ ] any `settings` keys, labels, defaults and choices are final; changing a key loses its saved value
- [ ] tested from a clean data root: install, open, move, close, update, remove; open Settings and toggle each row
- [ ] README says a WASM widget needs a Grimoire with the runtime
