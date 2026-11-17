**English** | [日本語](Building-from-Source-ja) | [Polski](Building-from-Source-pl)

# Building from Source

The app is Rust: one crate in `rust/` (`rust/Cargo.toml`; there is no `Cargo.toml` at the top),
built and checked by the top-level Makefile. Run cargo from inside `rust/`, or use make;
`rust/PORTING.md` has the cargo commands and the rules for the code.

## What it needs

The versions are the release workflow's pins, in `.github/workflows/release.yml`:

- **Rust 1.97.1** through rustup, with the Kindle's target, and the Makefile pointed at it:

  ```sh
  rustup toolchain install 1.97.1 --profile minimal --target armv7-unknown-linux-musleabihf
  export RUST_BIN="$(dirname "$(rustup which --toolchain 1.97.1 rustc)")"
  ```

  Without `RUST_BIN` the Makefile puts rustup's `stable` toolchain for Apple Silicon first on the
  PATH (it moves with each Rust release, and has no Kindle target unless you add one); on an Intel
  Mac or on Linux that folder does not exist, so make runs whichever `cargo` is first on your
  PATH.
- **Zig 0.16.0** (ziglang.org) and **cargo-zigbuild 0.23.0**
  (`cargo install cargo-zigbuild --version 0.23.0 --locked`), for the Kindle build. A newer zig
  passes the checks, but only the pinned one makes the release's bytes.
- A C compiler (`clang`; gcc on Linux), make, curl and unzip: `build.rs` compiles the vendored C.
- **Python 3.12+** (the standard library only), **Node.js 18+** and **LuaJIT**: the checks run
  them.
- Optional, each skipped with a message when missing: shellcheck, luacheck, dash and busybox (the
  launchers and the KOReader plugin), ImageMagick 7's `magick` (the Effects studio's picture
  check), and GNU gettext's `msgfmt` (the languages' `.mo` comparison; `make l10n` needs it).
  `make sim` needs SDL3 and pkg-config (on a Mac, `brew install sdl3 pkgconf`).

## Make targets

```sh
make check     # everything that can be checked without a Kindle: unit tests, scripted UI
               # scenarios with state checks, every language on every screen, the share server,
               # Live view, Bluetooth, the exporter, the ARM build, the zip with its launchers and
               # KOReader plugin, two clean builds compared byte for byte, and the KPM package;
               # ends with CHECK PASSED
make test      # the unit tests (rust/tests), on the host and in the headless build
make scenarios # tests/scenarios/*.qs on the headless runner (SCENARIOS=... for some)
make kindle    # build/kindle/grimoire: static ARMv7 hard-float (musl) binary
make dist      # complete installable bundle in dist/Grimoire-VERSION-COMMIT/
make package   # build/dist/Grimoire.zip
make kpm-package # build/kpm/*.kpkg and preview/stable repository manifests
make tools     # build/host/grimoire-export (used by tools/grimoire-pull)
make tablet    # build/host/scribe-tablet: the pen as a Mac tablet for osu!
make l10n      # po/grimoire.pot from the sources, every po/<lang>.po updated and compiled
make sim       # the C tree's simulator: an SDL3 window, the app before the Rust port
```

On a 15-core Mac, `make -j15 check SCEN_JOBS=8` takes about 7 minutes once the build cache is
warm; a first build, or a smaller or busier machine, takes longer. `build/` grows to a few GB, and
more over time as cargo keeps older builds in `build/cargo`. Two checkouts can run it at the same
time on different ports (`GRIMOIRE_TEST_PORT`). The whole of `make check` runs on macOS. On Linux
the unit tests and the scenarios run (with two known failures, in `docs/BACKLOG.md`); the rest has
not been tried there, and the share check needs macOS's `sips`.

Use `make dist` for builds to keep or install. It collects the installer ZIP, optional KOReader
patch ZIP, KPM package and manifests, checksums, and source revision in `dist/`. Version/commit
folders keep different builds separate, and the collector refuses to overwrite different files.
`DIST=/path/to/dist` selects a shared output folder across worktrees. Build intermediates remain
under `build/`. Collecting a build does not publish it.

## Development docs

- `docs/DESIGN.md` describes the architecture, the refresh model, the brush model, the languages
  and the file formats
- `docs/FORMAT.md` the files byte by byte
- `docs/KINDLE.md` how it fits into the Kindle and KOReader
- `docs/TESTING.md` device testing and the CI layout
- `docs/SIM.md` the simulator and the headless scenario runner
- `docs/TRANSLATING.md` how to improve a translation or add one
- `docs/TABLET.md` the Mac tool that makes the pen a tablet, over SSH
