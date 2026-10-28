# The Kindle layer's review, before the first device install

The final review of the Rust port's Kindle layer (branch `claude/rust-kindle-review-wol8f6`, from
`claude/rust-port` at 436aba1), read line by line against the C tree in `src/`, in a Linux x86_64
cloud session with no device.

How it was done: two rounds of parallel reviewers, one per area, each finding then checked by two
independent skeptics (one trying to refute it from the code, one tracing a real path to it on the
device), and a completeness critic after each round that picked the gaps and read them itself.

- Round 1: input (input_evdev.rs), display (display_fbink.rs, the shim), every fork site and the
  process model, fast ink and the clock corner, light, power, sleep and wake, and a crate-wide panic
  sweep.
- Round 2: pixel indexing from coordinates at the panel's edges, main-thread latency (every call
  that can block, and where the C pumps), untrusted bytes from the browser and the helpers,
  RefCell re-entry through work_pump and the input hooks, and code that runs differently in the
  release build (no overflow checks, panic=abort).
- The fixes were then reviewed again, the same way.

**No blocker was found.** Neither round found an MXCFB_WAIT_FOR_UPDATE_SUBMISSION, a second waiter
on a marker, a main-loop block of ~100 ms the C does not have, a fork child that could find a lock
or the allocator held, or a panic that device input can reach. What was found, and what was done
about it, follows.

## Fixed

Each fix is its own commit.

| # | Finding | Severity | Fix |
|---|---|---|---|
| 1 | **The detached exit child could write into a PDF or a page** (bgjob.rs bg_detach_rest). It kept Grimoire's SIGTERM/INT/HUP handler, which writes one byte to the signal pipe's write end. child_setup closes that fd, and the jobs' output files take the lowest free numbers next, so a stop sent to the child (a shutdown's TERM to everyone) wrote 0x01 into the file being written. Also in the C. | MINOR, data | The child ignores the three signals before anything can reuse the number. It already went on through a stop (the handler only set a flag nobody read), so its behavior is unchanged. |
| 2 | **page_paper allocated and copied a whole page at an erase pen-down** (editor.rs). The first erase or white-ink stroke after every page turn on an image paper rebuilt the paper into a fresh page-size canvas (allocation plus a white fill), then Rc::from copied it into a second allocation. That is about 9 MB of allocation and two page-size passes the C does not make, with no pump, as the stroke begins, and the copy's allocation could not fail (an abort where the C refuses the stroke). | MINOR (latency at pen-down) | The shared paper is `Rc<Vec<u8>>`. page_paper takes the buffer back when nothing holds it and draws again in place, as the C does, and `Rc::new` moves a Vec without copying it. render_layers' copy of the erase strokes' paper is moved the same way, instead of being copied a second time. |
| 3 | **read_file took one read call and one copy per 4 KB** (util.rs). That is on the page-load path, which does not pump. The C's fread into a doubling buffer makes a handful of calls. | MINOR (latency) | `read_to_end`, straight into the buffer (EINTR retried, and what was read is kept on an error, as before). |
| 4 | **A lipc battery reading could stay a zombie with an open pipe** (status.rs, light_kindle.rs; Rust-only code). Only status_tick reaped it, after its "corner off or busy" return. | MINOR | A new `sys_battery_reap` (a no-op on the host and in the simulator) runs first in every status_tick. It never waits, except on a shell it has just killed. |
| 5 | **The corner showed the battery from before a sleep for up to 10 s after the wake** (status.rs). CLOCK_MONOTONIC stops in suspend, and status_reset left read_at alone. | MINOR | status_reset clears read_at. |
| 6 | **status_cover built a full layout on every fast ink frame** (status.rs): the wall clock, localtime_r, a format and a measure, even for a stroke at the bottom of the page. | MINOR (pen path cost) | The dirty rect is first tested against the band the indicator always stays in (its height depends only on the size), and against where it was drawn. |

## Not fixed: owner's decisions or no device effect

- **light_get's two synchronous lipc popens** (light_kindle.rs:199). When the startup read failed,
  every Display sheet open, Live view start and browser light read runs two `lipc-get-prop` popens
  (about 0.1 to 0.2 s, no timeout) on the main loop. This is the same as the C (light_kindle.c:92).
  Changing it changes both trees: the owner decides. The device test below checks that the startup
  read succeeds.
- **Live view's tile viewer reads w and h without a seqlock** (live.rs:1143, as live.c:350). Two
  turns within nanoseconds of each other could give it a torn size, and only that viewer child
  would die. Not reachable in practice; the same in the C.
- **The first frame's readback at (100,191) and (100,600)** has no bounds check (display_fbink.rs
  ~1163, as the C). Every Kindle panel is larger than that.
- **The framebuffer mapping is held as `&'static mut [u8]`** (display_fbink.rs:76), while the Kindle
  UI also writes /dev/fb0 and the screen guard reads it back. In Rust's model, that exclusive
  reference is unsound. It does not miscompile today: every access goes through the State's
  RefCell, with opaque FFI calls and syscalls between a write and a read. A raw pointer with
  volatile reads would remove the question. That is a refactor for after the device test, not a
  pre-install fix.
- **The simulator's display_pen_mode_on** clears at stroke end. On the device it stays on until the
  next grayscale refresh (documented in display.rs). The scenarios therefore never exercise "pen
  mode still on" holding the clock corner back. On the device the editor's settle always follows a
  stroke, so the corner waits about a second at most.
- **Render and Live view children have no catch_unwind** (httpd.rs, live.rs). In the dev profile a
  panic there would unwind into the parent's code inside the child. The device build aborts on a
  panic, so a child just dies and is reaped. This only affects host tests.

The two refuted findings are also recorded. The first claimed the watcher shell inherits an ignored
SIGPIPE: mainloop restores the default first thing. The second claimed the quiet thread's spawn
failure logs a stale errno: true, but only text in a log line that has never been seen.

## What was checked and found right

In brief (the reviewers' full lists are long):

- **Input.** The ioctl numbers were recomputed for armv7 (EVIOCGRAB 0x40044590, EVIOCSCLOCKID
  0x400445a0, EVIOCGABS 0x80184540+abs, and the rest). `input_event` is 16 bytes, and reads are
  counted by its size. Every bitmap size matches the C's. EVIOCSCLOCKID is the first ioctl after
  every open. The grabs, and their release and retake on the native PIN handover, match the C,
  rollback on failure included. The SYN_DROPPED resync on both pens and on HID matches. The key
  scan gating is kept whole: stat first, never open or close a held node, and only while the pen,
  touch and pinch are idle, after 1500 ms of quiet. The LT/RT trigger axes and hats are bounded,
  with no division by a zero span. The power key hold uses kernel stamps and wrapping arithmetic.
  The watcher child forks without allocating and ends in `_exit(127)`. The signal pipe is
  async-signal-safe. Every u32 time compare wraps.
- **Display.** No MXCFB_WAIT_FOR_UPDATE_SUBMISSION exists anywhere in rust/ or the shim. Each marker
  has one waiter: the quiet thread gets the gray FIFO and the newest DU, pen_leave waits only on
  pen-mode markers, and display_wait goes through the quiet thread's books with a 6 s bound. The
  quiet thread uses only its Mutex/Condvar, the ioctl and sleep (no allocation, no log). Pen mode,
  live ink, the refresh modes, the rect clipping, orientation and flip, the readback bounds, the
  first-frame retry and the colour panel's MDP wait all match the C. The only main-thread waits are
  the C's.
- **Fork sites** (bgjob, httpd renders, Live view viewers and the pen child, bluetooth, update, the
  light shells, the power watcher). No RefCell borrow is held across any fork. Every child ends in
  `_exit`, or execs and then `_exit(127)`, with every exec string made before the fork. Signal
  masks are held across the bgjob fork and reset in the child. Every child is reaped by its pid,
  never with waitpid(-1), and no std::process::Command is used on the device. The shared tables are
  all AtomicU32 in MAP_SHARED memory.
- **Fast ink.** The sample ring is a correct per-slot seqlock: stamp 0, then the payload, then the
  stamp, then the head, all SeqCst, so dmb on armv7. It has no 64-bit or plain shared fields. seq
  wraps and skips 0, and lapping is detected. The pen child allocates nothing, sets PDEATHSIG and
  checks getppid. The per-batch wake is at most one non-blocking 1-byte write per pen viewer per
  read of the pens, with deduplicated wakes, so it cannot block the main loop.
- **The clock corner.** status_flush is held while the pen is near, while editor_busy and while
  pen mode is on. It sends a single GL16 with no display wait. Time formatting is safe until 2038
  (32-bit time_t). Battery parsing cannot panic.
- **Light, power, sleep and wake.** These match the C: the lipc commands byte for byte, run_line's
  timeouts and kill, editor_suspend's save before the sleep screen, the mode 2 handover, the wake
  regrab, the autosleep and the fallbacks. write_file_atomic fsyncs, and keeps the good file on an
  error, as util.c does.
- **Panics.** Every evdev read is sliced by the count it returned. Touch slots are clamped. Every
  division by a device range is guarded. The httpd request parsing, pairing, JSON, bluetooth,
  update, KOReader and annotate readers are bounded. The page reader caps counts by the bytes left.
  The canvas, brush, font and editor drawing is clipped before any index, so a pen at the panel's
  edge or off it cannot index out of range. The editor's `open_page().unwrap()` calls cannot fail,
  because store_load_page gives a blank page for a damaged file. There are no debug_assert or
  cfg(debug_assertions) guards in device code.
- **Re-entry.** Every thread_local was enumerated. No borrow is held across work_pump or a hook. The
  hooks reach only app.rs's Cells and short live.rs borrows. The pump has the C's re-entry guard and
  pid check.

## Tests on this machine (Linux x86_64, root)

- `make rust-kindle`: builds. The Kindle clippy (`cargo-zigbuild clippy --release --target
  armv7-unknown-linux-musleabihf --features kindle`) and the host clippy (`--features sim --tests`)
  are clean.
- `GRIMOIRE_TEST_PORT=8850 make rust-scenarios`: passes, every scenario and the runner's self-test.
- `make rust-test`: fails here in test_annotate only (8 checks at lines 808 to 864). Those checks
  make a folder or file unwritable with chmod 0555/0000 to stand for full storage, and root
  ignores those modes. This is host-specific; the expectations are left as they are. Because
  cargo stops at the first failing binary, the whole suite was also run with `--no-fail-fast`
  (results below).

TEST-RESULTS

## What remains for the device

Nothing in this review can stand in for the device on the following points:

- the e-ink driver's behavior: the quiet thread, pen mode, the clock corner's GL16 next to fast ink
  frames, the first frame and the screen guard;
- real pen timing: SYN_DROPPED under load (the log's `pen:` resync lines), the key scan's gating
  while writing, the pen-down cost on an image paper after the page_paper fix;
- the battery read: sysfs or the lipc fallback, whichever this Kindle has;
- sleep and wake with the native PIN, and the handover of the input devices;
- Live view and fast ink over the owner's LAN;
- the authoritative gate (`make rust-check`) on the owner's Mac.

## Device test checklist

Watch `grimoire.log` (over ssh from the owner's session) while doing these.

1. **Start.** The first frame comes up whole, with no flash loop. The log shows the input devices
   found (pen, raw digitizer, touch, power key), EVIOCSCLOCKID taking effect, and a valid light
   intensity from the startup read (not -1).
2. **Write.** Fill a page fast, with long strokes, including at all four edges and corners. No
   gaps, no `SYN_DROPPED`/resync lines in normal writing, and no heartbeat warnings.
3. **Erase on a template.** Open a notebook on an image paper and turn the page. The first eraser
   stroke and the first white-ink stroke start without a gap at their beginning. Undo and redo.
4. **Clock corner.** Turn it on with a tap in the corner. Write near it and lift the pen: it updates
   within about a second of the settle, and never in the middle of a stroke. Write at the bottom of
   the page while a minute changes. Turn it off with a tap.
5. **Battery.** The percent shows, and the bolt shows while charging. If this Kindle reads the
   battery through lipc, turn the corner off and on a few times, then check that no `sh` zombies
   stay (`ps`).
6. **Sleep and wake.** Press power to sleep: the sleep screen comes up and the native PIN works.
   Wake: Grimoire takes the pen and touch back, the corner shows a fresh battery, and the page is
   whole. Do it again after a long sleep.
7. **Power key hold.** A 4 s hold does what it should, and a short press sleeps.
8. **Rotate.** Turn the Kindle each way. Writing still lands under the pen, and the flip is right.
9. **Controller.** If a game controller is paired: the buttons, the hats, and the LT/RT triggers
   as the shortcuts are set.
10. **Live view and fast ink.** Pair a browser, open Live view, and write. Strokes stream with no
    slowdown on the Kindle. Close the browser tab: the viewer children go (`ps`).
11. **Exports in the background.** Export a notebook to PDF and keep writing. Exit Grimoire with an
    export queued: the PDF finishes, and is whole, after the exit.
12. **Light.** The Display sheet's light and warmth sliders work, with no pause on opening.
13. **Quit.** Quit from the menu: everything is saved, and the Kindle UI comes back.
