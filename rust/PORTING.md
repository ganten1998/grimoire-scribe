# Porting Grimoire to Rust

The owner's decision: the exact same app as the C one on the Kindle, the Index design included,
in Rust. This crate is that port, one module per C file. The C tree (`../src`) stays the
reference until the switch, and every rule below serves one aim: a user, a test or a file cannot
tell the two builds apart.

Read this whole file before translating anything. It is the contract between the groups that
translate in parallel: the skeleton's types and signatures are shared by everyone, and changing
one is a cross-group event (see "Interface changes").

## Building

Homebrew's rustc shadows rustup's on the owner's Mac. Put rustup's toolchain first:

```
export PATH=~/.rustup/toolchains/stable-aarch64-apple-darwin/bin:$PATH   # rustc 1.97.1
cd rust
cargo build                                    # host: the library, grimoire-export, mkicon, mklogo
cargo build --features sim                     # adds grimoire-headless, the scripted runner
cargo test -- --test-threads=1                 # the ported unit tests (one at a time: see Tests)
cargo test --features sim -- --test-threads=1  # again in the headless build: the runner's own tests too
cargo zigbuild --release --target armv7-unknown-linux-musleabihf --features kindle --bin grimoire
```

Run cargo from inside `rust/`: `.cargo/config.toml` (the Kindle target's flags: cortex-a7,
static, kindle/no-comment.ld) is found from the working directory, not from the manifest.

From the top level, `make` does the same with the target dir out of the tree (`build/cargo`;
build/ is a symlink out of Dropbox in the owner's checkout, and a cargo target inside Dropbox
breaks). Since the switch (wave 4) the app's rules build the Rust binaries, where the C ones went:

| target | does |
|---|---|
| `make headless` | `build/host/grimoire-headless` |
| `make tools` | `build/host/grimoire-export` |
| `make kindle` | `build/kindle/grimoire` (static armhf, no machine paths in it) |
| `make test` | cargo test, one test at a time, `GRIMOIRE_TEST_TMP` set: on the host, then with `--features sim` |
| `make scenarios` | every tests/scenarios/*.qs and the runner's self-test |
| `make l10n-check` | every screen in every language, with the fit check |
| `make rust-check` | the app's gate: unit tests, scenarios, languages, mirror cleanup, share, Live view, Bluetooth, the exporter, the ARM build |
| `make package` | the Kindle zip, in `build/dist` |
| `make check` | `rust-check` plus the package, launcher, KPM and patch checks |

The port's first names (`rust-headless`, `rust-tools`, `rust-kindle`, `rust-test`,
`rust-scenarios`, `rust-l10n-check`, `rust-package`) still work as aliases. Each binary's rule runs
cargo every time and copies the binary only when its bytes changed. `tests/*.sh`, the scenario
runner and the Python and JS checks run unchanged.

The C tree builds into `build/c` under `c-headless`, `c-test`, `c-tools`, `c-kindle`, `c-icon` and
`c-logo`; `make c-check` and `make c-package` run the same checks and packaging against the C
binaries (`BUILD=build/c`). Never run a GUI binary as a check, and never touch the device from a
check.

The unit tests read `GRIMOIRE_TEST_TMP` as the C's do (test_store writes to its `../out`): a bare
`cargo test` without it fails test_store's export checks in both languages. Run them through
`make test`, or set it to a writable directory whose parent is writable too.

## Variants

The C build's `-D` switches are cargo features:

| C | Rust | platform modules |
|---|---|---|
| `GRIMOIRE_KINDLE` | `--features kindle` (armv7 Linux only) | display_fbink, input_evdev, light_kindle |
| `GRIMOIRE_SIM` (headless) | `--features sim` | sim_sdl (the headless runner) |
| neither (host tools, unit tests) | no feature | platform_none |

`#ifdef GRIMOIRE_SIM` becomes `#[cfg(feature = "sim")]`, `#ifdef GRIMOIRE_KINDLE` becomes
`#[cfg(feature = "kindle")]`. display.rs, input.rs and light.rs hold their headers' types and
documentation, and forward every function to the build's platform module (`platform::...`), so
the platform's signatures are checked against the header. Unlike the C's host build, every module
compiles in every build (platform_none.rs answers for the missing platform), so `cargo build` on
the host type-checks the whole app.

## The map

Every C file has a module of the same name; a header and its .c are one module.

| C | Rust | notes |
|---|---|---|
| common.h + util.c | util.rs (also `crate::common`) | fully implemented: Rect, clamps, fmla, clocks, qlog!, files, FixedStr, strerror |
| main.c | mainloop.rs | `main.rs` would read as a binary to every Rust tool; src/bin/*.rs call it |
| sim_sdl.c | sim_sdl.rs | the headless runner only (see "Not ported") |
| templates.h + bgwork_template.c | templates.rs (types) + bgwork_template.rs (functions) | templates.rs re-exports them |
| compact.h + bgwork_compact.c | compact.rs + bgwork_compact.rs | the same |
| app.h | app.rs | re-exports the screens' entry points from library.rs, editor.rs, ... |
| display.h, input.h, light.h | display.rs, input.rs, light.rs | types + forwarders to the platform |
| inkmode.h, panel.h, bat.h | inkmode.rs, panel.rs, bat.rs | header-only logic, implemented |
| erasersample.h, build/gen/*.c | erasersample.rs, assets.rs | include_bytes! of the same files (the bytes xxd -i embeds); build.rs writes the catalogs' table |
| stb_impl.c, third_party/* | build.rs (cc), ffi_stb.rs, ffi_qr.rs, ffi_fbink.rs, csrc/ | see "Vendored C" |
| tools/grimoire-export.c, mkicon.c, mklogo.c | src/bin/grimoire-export.rs, mkicon.rs, mklogo.rs | |
| tests/test_*.c, tests/testlib.h | tests/test_*.rs, tests/testlib/mod.rs | see "Tests" |

The other files (tests/*.sh, *.py, *.js, *.lua, tests/scenarios, kindle/, web/, po/, tools/*.py,
docs/) are shared: the Rust build uses them as they are.

### Not ported

- The SDL3 window simulator (`grimoire-sim`, GRIMOIRE_SIM_SDL): a development aid no check runs.
  Port it later only if the owner asks.
- tools/bench-ink.c, tools/du-model.c: one-off measurement programs outside the Makefile.
- tools/qrdecode.swift, tools/*.py: not C; used as they are.

## The rules

### 1. Same behavior, byte for byte

Everything observable stays exactly as the C makes it:

- **Files**: page files (QPG1), notebook.ini, settings.ini (key order, number formatting, the lines
  kept from newer versions), nav.ini, links.ini, source.ini, bg/index.ini, bg/pages.ini, erased.txt,
  import/.attempts, thumbnails (PGM), sleep.pgm, controller.ini, shortcuts.ini. docs/FORMAT.md is
  byte-exact; the Rust must write the same bytes for the same state, and read every file the C
  reads, including the damaged and newer-version cases.
- **Exports**: PDF, PNG and zip bytes (the same stb_image_write and zlib settings, the same
  object order and numbers), the share server's HTML, JSON and headers, Live view's stream frames.
- **Log lines**: every qlog line's text (tests grep grimoire.log). Use `qlog!` with the C's format
  string, verbatim.
- **UI strings**: every string a screen shows, byte for byte, so po/*.mo keep working (the msgids
  are the English text), and the same TRANSLATORS comments above them.
- **The panel**: every refresh's rectangle and mode, in order (scenarios check
  `display.presents`), every pixel of every screen (shots and golden renders).
- **The runner**: the headless runner's stderr messages ("expect failed (line N): ...", "shot: ...",
  "refreshes: ...", "headless: ..."), its exit statuses (3 for a failed expect or fit check), and
  every `app_probe` key and value (docs/SIM.md).
- **Timing**: every timeout, debounce and interval constant, in the same clock (now_ms is
  CLOCK_MONOTONIC milliseconds as a u32 that wraps; compare with `wrapping_sub` as the C does with
  unsigned arithmetic).
- **Exit**: the process's exit status in every path.

### 2. No new behavior

Translate; do not improve. No new features, no "fixes", no reordering of work, no extra logging,
no extra validation, no different error text. A C bug is ported as it is and reported in your
result (file, line, what happens). The one exception is a difference the language forces; each
known one is listed under "Decided differences", and a new one is reported.

### 3. Comments

The C's comments are the design record: carry every explanatory comment over, next to the code it
explains (header comments are already in the skeleton as `///` docs). Write comments as the C does:
why, not what, in plain English. No em dashes in comments or docs. A comment that names a C
function names its Rust twin (they have the same names). "NULL" in a carried comment reads as
`None`; leave the word unless the sentence becomes wrong.

### 4. unsafe

Only for: FFI calls (stb, qrcodegen, the FBInk shim, libc), ioctls, fork/exec/_exit/waitpid,
signals and sigprocmask, mmap'ed memory (the framebuffer, Live view's shared frame, httpd's shared
download table) and raw fds. Every `unsafe` block gets a `// SAFETY:` comment saying why it is
sound. Never use unsafe to get around the borrow checker (no raw pointers to Rust data, no
transmute of Rust types, no `static mut` of Rust state): restructure instead, as below.

### 5. The display's hard rules

From src/display_fbink.c and ~/.claude/projects/-Users-ganten/memory/kindle-hwtcon-marker-race.md
(the Kindle Scribe 2 rebooted on its watchdog: a kernel oops in Amazon's e-ink driver):

- **Never** issue `MXCFB_WAIT_FOR_UPDATE_SUBMISSION`. A submission waiter woken by the driver's
  first signal frees the marker before its second signal: the kernel panics.
- **Never** put a second waiter on a marker another thread waits on: each signal wakes one waiter,
  and the other sits out the driver's 5 s timeout. display_wait waits on the quiet thread's books
  when that thread took the last marker, exactly as display_fbink.c does.
- The waits run on the quiet thread's own fd, as in the C; `grim_fbink_wait_for_complete` is the
  only wait the shim offers, plus `MXCFB_WAIT_FOR_MDP_COMPLETION` (an ioctl in display_fbink.rs)
  for colour panels, on a signal of its own.
- Pen mode, the live ink rule (live_ink_mark, live_ink_frame) and the colour panel's waits follow
  the C line for line.

### 6. The process model

The C's processes stay as they are, made with libc (never std::process::Command, which changes
signal dispositions, fds and the environment in ways the C does not):

- **bgjob**: one niced child at a time (fork, no exec) renders PDFs, compacts pages, makes
  pictures; `bg_detach_rest` hands the queue to a detached child at exit. The stop signals are held
  across the fork and put back to their defaults in the child, as bgjob.c does.
- **The share server**: render children (two at a time) and Live view's viewer children (two at
  most), forked by httpd and live; the shared download table and frame are `mmap(MAP_SHARED |
  MAP_ANON)`, read and written with atomics where the C uses volatile or C11 atomics.
- **Scripts**: bluetooth.sh, update.sh, lipc through `/bin/sh -c` (light_kindle: posix_spawn and
  popen), the watcher in input_evdev: fork, close what the C closes, reset what the C resets,
  `execl`, `_exit(127)`.
- In a child, end with `libc::_exit`, never `std::process::exit` or a return from main (they run
  the parent's exit handlers and flush its buffers, which the C's `_exit` avoids).
- **The quiet thread** (display_fbink's) does nothing a fork could find half done: it only waits on
  the driver and keeps books under its mutex. In Rust that means no allocation (no Vec growth,
  String, format!, qlog!), no other lock, no std I/O; only its Mutex/Condvar, the ioctl and
  sleeping. A bgjob child forked while it runs must find malloc and every other lock free.
- No other threads. std::thread is for the quiet thread only.

### 7. Global state

C file-level `static` state becomes module-private state, touched by the main thread only (and by
forked children, which get their own copy):

```rust
thread_local! {
    static S: RefCell<State> = RefCell::new(State::default());
}
fn with<R>(f: impl FnOnce(&mut State) -> R) -> R { S.with(|s| f(&mut s.borrow_mut())) }
```

Keep each borrow short: never call another module, `work_pump()`, or a callback while holding it
(work_pump reads the pens, which can reach other modules; a nested borrow panics where the C would
simply re-enter). State the C shares with the quiet thread uses `Mutex`/`Condvar`, as the C's
pthread mutex and cond do; a flag the C reads from a signal handler or another process uses an
atomic. `static` with an atomic is fine for a lone scalar. Tests may set the C's test hooks
(`STORE_TEST_BEFORE_CARD_REPLACE`, `COMPACT_TEST_BEFORE_REPLACE`) through their Mutex.

### 8. The App

There is one App per process, owned by mainloop.rs (boxed), passed as `a: &mut App` as the C
passes `App *a`. There is **no global pointer to it**. The C keeps two:

- `g_probe_app` (app.c), for app_probe: the runner's probe point is handed the App instead.
  mainloop.rs calls `sim_sdl::input_poll_app(ev, timeout, &mut app)` in the headless build, and
  `app_probe(a, key, out, cap)` takes the App.
- `g_live_app` (app.c), for the hover hook (input_hover_hook calls back from inside input
  reading, possibly from work_pump in the middle of app work): the hook reads an atomic that app.rs
  keeps equal to "Live view is on and the app is awake" instead of the App. Fast ink's live_capture
  (input_sample_hook's, called from the same places) reads app.rs's LiveCap instead: Live view on,
  asleep and input_yielded, kept equal to the App's wherever they change, and the rest of what the
  C's live_capture reads of it (the screen, the book's state, editor_live_style for an event of
  each eraser and side button state), made again after every app_event and app_tick and as Live
  view starts.

The screens keep their state in `a.lib`, `a.ed`, `a.ps`, `a.cal`, `a.share`
(`Option<Box<XState>>`). The C passes `App *a` and pointers into the App's own state together
everywhere (a sheet inside the editor's state, and the App). In Rust:

- **Short borrows**: take what you need from `a.ed` into locals, drop the borrow, call `app_*`, take
  it again. A helper `fn ed(a: &mut App) -> &mut EditorState { a.ed.as_deref_mut().unwrap() }` per
  screen module keeps it readable.
- **Disjoint fields**: where a hot path needs the screen state and the screen canvas at once,
  destructure: `let App { screen, ed, .. } = a; let ed = ed.as_deref_mut().unwrap();`.
- **Sheets and keyboards**: a modal stored in a screen's state (TplSheet, MusicBuild, DispMenu,
  PagesView, Keyboard) is taken out for a call that also needs the App, then put back:
  `let mut kb = std::mem::take(&mut ed(a).kb); let r = app_keyboard_pointer(a, &mut kb, ph, ev); ed(a).kb = kb;`.
  The modal's own functions never reach back into the host's state, so nothing sees it missing.
- **Re-entry**: app_switch from inside a screen's event calls the old screen's leave and the new
  one's enter. Never hold a borrow of `a.lib` (or take the state out) across a call that can
  switch screens or redraw (app_switch, app_redraw, editor_open_notebook, ...).

### 9. Ownership: the C's pointers

The skeleton already settles these; follow the same patterns elsewhere:

| C | Rust |
|---|---|
| owned array + n + cap (`Stroke **strokes`, `PenSample *pts`, `NavName *names`, `BgIndexEntry *e`) | `Vec<T>` (the count is `len()`; NotebookInfo::npages() for npages) |
| owned object (`Page *`, `Stroke *`) | `Box<T>`; `*_free` takes the Box |
| `T **out` that the callee allocates | `&mut Option<Box<T>>`, or `&mut Vec<T>` for arrays |
| strokes found by pointer (history.c) | `StrokeId` (the boxed stroke's address, compared, never dereferenced); `Action.owned` holds the strokes while the action owns them |
| a struct pointing into its host's data (PagesView's nb and nav) | passed to each call that reads them (pagesview_draw, _pointer, _probe) |
| the same where the data does not change while it lives (TplSheet's nb) | a copy made when it opens |
| a buffer a struct reads while the caller keeps it (BrushState's paper) | `Option<Rc<[u8]>>`, shared |
| a writer holding its output (`ZipWriter.out`) | `ZipWriter<'a> { out: &'a mut Buf }`, made by zip_begin |
| `void (*fn)(...)` | `fn(...)` / `Option<fn(...)>` |
| callback + `void *ctx` | `&mut dyn FnMut(...)` (the closure carries the context) |
| a borrowed pointer into the framebuffer or a mapping | a slice made once from the mapping (unsafe, with SAFETY), then safe indexing |

### 10. Memory

The C checks malloc's result where the size comes from data (a page canvas, a picture, a file) and
takes a failure path (a logged error, -1). Keep those paths: `Vec::try_reserve_exact` (canvas_init
does) and return the C's failure. Small fixed allocations may just allocate.

## Conventions

### Names

The C's names, unchanged: functions, statics (as module-private items), struct fields, constants.
File-local helper functions keep their names and stay private. Rust keywords as names take a
trailing underscore (`type_`, `box_`, `where_`, `in_`). Each module starts with
`use crate::prelude::*;`, the C's one namespace, so a call reads as the call it translates.

### Types

| C | Rust |
|---|---|
| `int`, `unsigned`, `uint32_t`, ... | `i32`, `u32`, `u32`, ... |
| `long` | `i64` (it is 32 bits on the Kindle: where a value can pass 2^31, keep the C's wraparound with `as i32` at the same point) |
| `size_t`, `ptrdiff_t` | `usize`, `isize` |
| `float`, `double` | `f32`, `f64` |
| `bool` | `bool` |
| `char name[N]` field | `FixedStr<N>` (util.rs): the same capacity, the same cut, the same bytes |
| `const char *` parameter | `&str`; `Option<&str>` where the C allows NULL; `&[u8]` for bytes from outside (files, sockets, the radio) that need not be UTF-8 |
| `char *out, size_t cap` | `out: &mut String, cap: usize`: out holds what the C leaves in a buffer of cap bytes (at most cap - 1 bytes) |
| `char out[N]` parameter | `&mut String` holding at most N - 1 bytes |
| returned `const char *` into a table or the catalog | `&'static str`; into the callee's state: `String` or a borrow |
| `const uint8_t *data, size_t len` | `&[u8]` (the length is the slice's) |
| `T *items, int n` | `&[T]` / `&mut [T]` |
| `Rect`, `Canvas`, `InEvent`, ... | the same names; Copy where the C copies them by value |

Enums:

- A C enum whose values come from files or arithmetic (ToolKind, PaperKind, MusicPart) is a newtype
  (`ToolKind(pub i32)`) with the C's names as module-level constants (`TOOL_PENCIL`), so an unknown
  value from a newer version survives, as in the C.
- A closed C enum is a Rust enum (`#[repr(i32)]`, the C's values), variants in CamelCase with the C
  name beside each (`InType::PenDown // IN_PEN_DOWN`). Its `_COUNT` is `X::COUNT`. `x as i32` gives
  the C value.
- An anonymous enum of constants is constants.

### Integers

Rust panics on overflow in debug builds and the tests run them: where the C relies on unsigned
wraparound (times as u32, hashes, CRCs, fixed-point) write `wrapping_*`; where C's int arithmetic
cannot overflow for real inputs, plain operators are right. `x as i32` from a float truncates
toward zero and saturates (NaN gives 0), which is what the C build does on ARM (VCVT); keep the
C's explicit rounding (`+ 0.5f`, `lrintf`, `floorf`) as it is. Integer division and `%` truncate
toward zero in both languages.

### Floating point

Golden renders, PDF images and thumbnails depend on every float operation. Three rules:

1. **Precision**: in C, `float` arithmetic with a `double` operand (a literal without `f`, a
   `double` variable, a math function without the `f` suffix) is done in double and rounded back
   at the assignment. Do the same: `((x as f64) * 0.5) as f32` where the C writes `x * 0.5`.
   `sqrtf`, `floorf`, `powf`, `sinf` are `f32::sqrt`, `f32::floor`, `f32::powf`, `f32::sin`; their
   double forms are the `f64` methods. Both call the same libm as the C on each target.
2. **Contraction**: clang compiles the C with `-ffp-contract=on` (its default, and the Makefile's
   test flags say so: tests/test_golden.c's renders were recorded so): `a * b + c` within one
   expression is contracted (`llvm.fmuladd`), and the backend makes one fused multiply-add of it
   where it judges that faster. On ARM64, the host the tests and golden renders run on, it does
   (fmadd). On the Kindle's Cortex-A7 (zig cc, `-mcpu=cortex_a7`) it does not: the ARM backend
   lowers it to a separate vmul and vadd, and the C Kindle binary has no vfma (checked in the
   assembly of brush, canvas, calib_fit, font, paper, pensync and ui). Rust never contracts, and
   `mul_add` always fuses (vfma on the Cortex-A7). So never write `mul_add`: where the C has
   `a * b + c` (or `c + a * b`, `a * b - c`, `c - a * b`) in one expression of float code, write
   `fmla(a, b, c)` (util.rs: `mul_add` on aarch64, the plain form elsewhere; f32 and f64), with
   `fmla(-a, b, c)` for `c - a * b` and `fmla(a, b, -c)` for `a * b - c`. util::lerpf is one.
   canvas_deposit_px and canvas_restore_px show it. When two products meet (`a * b + c * d`),
   clang fuses the left one (`fmla(a, b, c * d)`): check against the golden tests (test_golden,
   test_ink, test_erase, test_pagesize) and the disassembly of the C build when they differ.
3. **Formatting**: Rust's `{:.N}` rounds exactly as printf's `%.Nf`; cformat! handles `%g` and `%e`
   as printf does.

### Text

- **printf**: keep the C's format strings verbatim with `cformat!` (to a String), `csnprintf!`
  (snprintf into `&mut String` with a cap, returning the untruncated length), `qlog!` (a log line)
  and `buf_printf!` (into a Buf). Never rewrite a format that reaches a file, a log, the screen or
  the network as a Rust `format!`.
- **Translations**: `_()` is `tr()`, `N_()` is `n_()`, `C_()` is `c_()`, `NC_()` is `nc_()`, `NG_()`
  is `ng_()`. The English is the msgid, byte for byte, including its printf conversions; keep each
  `// TRANSLATORS:` comment on the line above, as xgettext reads it. A translation is a format:
  `cformat!(tr("Saved %s"), name)` (cformat! takes positional conversions, `%2$d %1$s`).
- **UTF-8**: the C treats text as bytes. `FixedStr` keeps the bytes; `as_str()` reads them up to the
  first invalid sequence (a field the C cut inside a character), `as_bytes()` gives them all: write
  `as_bytes()` to files. Never pass data that is written back through `from_utf8_lossy` (it changes
  bytes). `strlcpy_q` and `FixedStr::set` cut at bytes like the C; `i18n_copy` and `set_cut` cut
  between characters like the C's i18n_copy. A String cannot hold a cut character: where a C
  snprintf cuts inside one and the result only reaches the screen, the Rust drops the partial
  character (the C's broken byte draws nothing anyway); where it reaches a file, keep bytes.

### Errors

Keep the C's return codes (0, -1, the documented others) and its out-parameters: callers and
tests check them, and a Result would change which paths exist. `errno` text comes from
`util::strerror(util::errno())` (the C library's words; Rust's io::Error adds " (os error N)").

## Vendored C

build.rs compiles, with the Makefile's flags (`-std=gnu11 -D_GNU_SOURCE`, `-mcpu=cortex-a7` on the
Kindle): `../src/stb_impl.c` as it is (stb_truetype, stb_image_write), `csrc/stb_image_impl.c`
(stb_image with bgimage.c's exact STBI_* configuration, its allocator routed to
`bgimage::stb_malloc` and friends through ffi_stb.rs), qrcodegen, and on the Kindle FBInk
(`FBINK_FOR_KINDLE FBINK_MINIMAL FBINK_VERSION="master-886f25f"`, warnings off) with
`csrc/fbink_shim.c`, which keeps FBInk's large structs on the C side and hands display_fbink.rs the
fields display_fbink.c reads (`GrimFbState`). Declare FFI only in ffi_stb.rs, ffi_qr.rs and
ffi_fbink.rs, and wrap each call in a safe function in the module that uses it. Nothing of
Grimoire's own logic goes into C.

## Tests

- **Unit tests**: each `tests/test_X.c` becomes `rust/tests/test_X.rs`, one test binary as in C,
  with `tests/testlib/mod.rs` (`check!`, `check_near!`, `run!`, `done()`, `tl_tmp()`): its TEST
  functions as plain fns, one `#[test]` running them in the C's RUN order, the same checks with the
  same values. `tests/beta_format.h` (the 1.0.0-beta writer the format tests read back) becomes
  `tests/beta_format/mod.rs`. Tests of a module's private helpers may go into the module as
  `#[cfg(test)] mod tests`. Run them one at a time (`--test-threads=1`, as make rust-test does):
  they keep the C's process-wide state.
- **Golden renders**: tests/test_golden.c's recorded hashes are the arbiter of the render path
  (with test_ink, test_erase, test_pagesize). A mismatch is a translation error until shown
  otherwise (see Floating point).
- **Scenarios**: `make rust-scenarios` runs all of tests/scenarios/*.qs and the runner's self-test
  through the C's rule; `make rust-l10n-check` every screen in every language with the fit check.
- **The gate**: `make rust-check` (and `make check` for the release). The C's checks keep working
  as `make c-check` until the owner retires the C tree.

## Interface changes

The skeleton's public types and signatures are the contract between the groups. If yours cannot
be implemented faithfully as declared (a missing parameter, a field the C has that the skeleton
dropped, a borrow that cannot work):

1. Implement your module against the skeleton as far as it goes, with the smallest change.
2. Change only your own files. Never edit another group's module, even to fix a call.
3. Report it in your result, one block per change:
   `INTERFACE CHANGE: module::item: old signature -> new signature. Why: ... Callers: modules that
   call it (grep).`
   The next wave's architect applies it across the crate before translation continues.

Do not "fix" another module's signature to suit yours, and do not add `pub` items another module
should have: report them.

### Calling conventions settled in wave 2

Where a C caller passes NULL, a count-only call or a pointer comparison, the translated callee
expects this (editor, export, pagesview, pensettings, library and the other later modules follow it):

- `nav_index(nav, nb, indices)`: the C's `(NULL, 0)` counting call is `&mut []`; the cap is
  `indices.len()`. `nav_link_areas(.., links, areas)`: the cap is the shorter slice; the C's
  count-only call (`NULL, NULL, NAV_LINKS_PER_PAGE`, editor.c) passes two local arrays of
  `NAV_LINKS_PER_PAGE`. `nav_links_of`'s `first` and `nav_link_at`'s `out`: a throwaway local for
  the C's NULL.
- `history_push_add(h, index, &s)`: a stroke's identity is its Box's address, which survives the
  move into the page. Where the C compares `page->strokes[page->n - 1] == s` (editor.c), take
  `stroke_id(&s)` before `page_add` moves the Box and compare it with the last stroke's.
  `history_push_erase`: `indices.len() == removed.len()`. `History::default()` has no limit; only
  `history_init` sets 500, as the C's memset and init.
- `page_remove` panics on an index out of range (every C caller passes a valid one).
  `page_droppable(p, dpi, drop)`: `drop.len() >= p.strokes.len()`.
- `page_serialize(p, out)` replaces `out` on success and leaves it as it was on the read-only
  refusal (-1). `store_load_info(id, out)`: the C let `id` alias `out->id`; copy the id first.
  `store_replace_thumb_if_same(.., was)`: an empty `was` is the C's NULL (no card was read).
- `bgimage_read` returns `None` with the C's errno set (ENOENT, EFBIG, EIO, ENOMEM, or open's or
  read's own): read `util::errno()` right after it, as the C callers do.
- `buf_printf!` is in the prelude with the other printf macros (httpd.rs formats into a Buf).
- Module state is thread-local: a `#[test]` sets up ui, fonts, the language and the store on its
  own thread (one `#[test]` per C binary does it once).

## Decided differences

The only known ways the Rust build may differ, each deliberate:

- **SIGPIPE**: Rust's runtime ignores SIGPIPE before main; mainloop::main restores the default
  first thing, as a C program starts (httpd ignores it when the server starts, as in the C).
- **O_CLOEXEC**: std::fs opens files close-on-exec; the C's fopen does not. Script children close
  every fd above 2 anyway (bluetooth.c), so nothing changes for them; where an fd must survive an
  exec, open it with libc as the C does.
- **time_t**: the libc crate's time_t is 32-bit on armv7 musl (zig's musl C code uses 64-bit). Only
  wall-clock seconds pass through it (wall_time, localtime_r); it overflows in 2038, not before.
- **Panics**: a todo!() or a bounds check that fails ends the process where the C would read out of
  bounds. The release profile aborts on panic (no unwinding).
- **Fast ink's style in a pump**: a pen sample that work_pump reads in the middle of a screen's
  event takes its ink style from LiveCap (The App), which is the editor as that event began; the
  C's live_capture reads the editor as it is at that moment. They differ only where the handler of a
  pen move or of a pen-down on the page (the events whose pumps may preview at all) changes the
  tool, the view or the panel and then pumps.

Report any other difference you find yourself making; it is a bug until the owner decides
otherwise.

## Packaging and the switch

The switch was made in wave 4: `make package`, `make dist` and the KPM rules ship the Rust binary
(`make c-package` builds the zip with the C one, in `build/c/dist`).

1. `make kindle` builds the Rust binary into `$(BUILD)/kindle/grimoire`, and `headless`/`tools`
   (and `icon`/`logo`) the Rust ones; the C rules have `c-*` names until the C tree is retired.
2. The package's licenses/ add the Rust standard library's notice (`rust-std-LICENSE.txt`, the
   toolchain's COPYRIGHT-library.html as text) and the libc crate's (`rust-libc-LICENSE.txt`);
   FBInk's, stb's, qrcodegen's, musl's and zig's compiler-rt notices stay (they are still linked).
   tests/package_check.sh checks both. It still reads WATCH_EVENTS from src/input_evdev.c (the
   Rust's `macro_rules! WATCH_EVENTS` in input_evdev.rs holds the same text); point it at the
   Rust when the C tree is retired.
3. tests/repro_check.sh builds the Rust binary twice from clean, each with its own cargo target
   (the remap flags in the Makefile and kindle/no-comment.ld keep the bytes independent of the
   machine), and compares.
4. The l10n rule extracts the msgids from rust/src/*.rs with the keywords
   `-ktr -kn_ -kc_:1c,2 -knc_:1c,2 -kng_:1,2`, through xgettext's C parser (`--language=C`):
   gettext 1.0's Rust parser drops the `#, c-format` marks msgfmt --check needs. po/grimoire.pot
   must come out the same, apart from the source references; it does not yet (see the wave 4
   report: i18n.rs's own tests get extracted, some TRANSLATORS comments sit a line too high after
   rustfmt, and two groups of strings come in another order), so `make l10n`'s output is not
   committed until it does.
5. `make check` is `make rust-check` plus the package, launcher, KPM and patch checks. The release
   workflow (.github/workflows/release.yml, `make -j4 check`) now needs rustup's stable toolchain,
   the armv7-unknown-linux-musleabihf target and cargo-zigbuild on its runner.

## Finishing

When every module is translated: remove the skeleton's crate-level `allow`s (lib.rs, src/bin/*),
fix every warning, and make `make rust-check` and `cargo clippy` pass. Delete nothing of the C
tree in the port's branch: the owner retires it.

Wave 4 translated the last modules (input_evdev, display_fbink, the host tools) and removed the
skeleton's allows. lib.rs keeps three clippy allows on purpose, each with its reason there
(too_many_arguments, needless_range_loop, unnecessary_cast), and platform_none keeps
unused_variables and ptr_arg for its stubs. The host, sim and Kindle builds and their clippy runs
are warning-free.
