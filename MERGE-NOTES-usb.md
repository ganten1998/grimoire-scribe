# Lane usb: merge notes

The persistent USB connection (Kindle storage at `/Volumes/Kindle` through the libmtp and WebDAV
bridge), and the repo pieces that touch it. Base: `codex/rc7-release-sweep` 6c5357a.

## What changed

### In this repo (branch `claude/rc8-usb`)

- `862dcdb` **bluetooth.sh waits for real on the Kindle (usb-12).** Busybox `sleep` there takes
  whole seconds only, so every `sleep 0.2` failed at once ("invalid number '0.2'", 530 lines in
  one log): the lock was taken as stale in milliseconds, the helper got no 12 s detach grace
  before its KILL, the start wait spun. A `nap` helper tries `usleep 200000`, then `sleep 0.2`,
  then `sleep 1` counted as five fifths, so the 20 s lock, 10 s start and 12 s stop waits keep
  their length on any shell.
- `1199ac2` **grimoire.sh flags a running Grimoire (usb-11, the owner's decision).** It writes
  `/mnt/us/grimoire/.running` (`pid=<launcher> build=<BUILD_ID> since=<epoch>`) right after its
  traps are set and removes it at the top of `restore()`, which every exit path the traps cover
  runs (normal exit, watchdog kill, INT/TERM/HUP, update hand-off, hosted). A refused second
  start (lock held) exits before the traps and leaves the running Grimoire's flag alone. A flag
  left by a power loss or a SIGKILL is removed by the Kindle's USB controller (its launcher pid is
  gone).
- `f521c1b` **usb-stage.py (lib-10, usb-8).** Writes with MKCOL and PUT to the bridge's token
  URL instead of `mkdir` and `shutil.copyfile` through the Finder mount (which left a `._` file
  beside every staged file), keeps the fresh read-back verification, no longer needs the mount,
  runs on Python 3.9 (the Mac's `/usr/bin/python3`; `hashlib.file_digest` was 3.11+), and adds
  `--prune-keep N` and `--clean-sidecars`. README's USB staging paragraph says so. New
  `make usb-stage-check` (added to `make check`) runs `tests/test_usb_stage.py`.

### In `~/KindleLab/volume` (outside the repo)

The directory was not a git repository (it sat inside the HOME dotfiles repo). It now is:
`git init`, the pre-rc8 sources and built helpers committed first (`1410672`, tagged `pre-rc8`;
the same files are also copied to `~/KindleLab/volume/backup-pre-rc8/`), then the rc8 work on
branch `rc8`.

**The rc8 work is in a linked worktree, `/Users/ganten/KindleLab/volume-rc8`, not edited in the
live folder.** Reason: the running bridge spawns a fresh USB worker on every reconnect, and with
the `spawn` start method that worker re-imports `kindle_volume.py` and reloads
`libkindlemtp.dylib` from disk. Editing them in place would have changed the running bridge at its
next reconnect, against the hard limit. The live folder stays on `main` = `pre-rc8` until the
owner runs `install_mac.sh`, which stops the bridge, fast-forwards `main` to `rc8` and restarts
it. `git -C ~/KindleLab/volume log main..rc8` (or `diff pre-rc8 rc8`) is the reviewable diff.

Commits on `rc8`:

- `b401675` **mtp_bridge.c and netfs_mount.c (usb-7, usb-15).** `kv_open` enumerates with libusb
  and hands libmtp a filled-in raw device (libmtp's probe for unknown devices, the Kindle being
  one, skipped it whenever its kernel-driver check failed). Distinct results: absent (-3), several
  Kindles (-2), other serial (-5), on USB but not serving files (-7), claimed by another Mac app
  (-8). New `kv_present` (enumeration only, nothing opened), `kv_read` into a descriptor with a
  progress callback, `kv_upload` with progress, `kv_size` (GetObjectInfo). `netfs-mount` takes
  credentials from the environment and an optional mount point. `build_helpers.sh` builds both
  (recorded in `provenance.json`).
- `9dcaf53` **kindle_volume.py (usb-6, 8, 9, 10, 11, 13, 14, 15).**
  - Unmounts `/Volumes/Kindle` 15 s after the Kindle is gone (`diskutil unmount`, then
    `umount -f`), never while an upload is being written; remounts on return. `status.json`'s
    `mount` means mounted and connected; a leftover mount shows as `stale_mount`.
  - Backoff 5 s doubling to 60 s while the reason stays the same; a libusb presence check every 2 s
    while disconnected retries at once when a Kindle appears (in place of an IOKit notification).
    Waiting out the backoff no longer pushes it further. Repeated 5xx DAV log lines are limited to
    one per minute per kind. `status.json` gains `reason`, `device_present`, `mtp_ready`,
    `claimed_by_other`, `kindle_controller` (usb-status and its heartbeat age),
    `grimoire_running`, `leftovers`, `pending_writes`, `retry_in_s`, `mount_url_has_token`.
  - AppleDouble `._*`, `.DS_Store`, `.Spotlight-V100`, `.Trashes`, `.fseventsd`,
    `.TemporaryItems` are kept on the Mac in `state/sidecars/` (PUT, GET, MKCOL, MOVE, DELETE all
    work, so Finder succeeds) and never sent over MTP; ones already on the Kindle are hidden from
    listings. A Kindle file cannot be moved onto a sidecar name or back (403). Deleting a folder
    takes its hidden device-side `._` files with it. `X-KindleLab-Raw: 1` shows the Kindle's own
    names, for maintenance. (WebDAVFS has no mount option that stops AppleDouble files; this layer
    plus `DSDontWriteNetworkStores` is the fix.)
  - The read-only guard: PUT, MKCOL, DELETE, MOVE (source and destination) and COPY (destination)
    under `/grimoire/` return **423 Locked** with a log line while `/grimoire/.running` exists on
    the Kindle (checked at most every 5 s), except `grimoire/photos/`, `grimoire/templates/`,
    `grimoire/import/`, `grimoire/PDFs/` and Mac metadata. Raw maintenance requests are guarded
    too. Reads are always allowed.
  - GET streams while the MTP download fills a capped LRU cache (`state/cache/`, 512 MB,
    `KINDLE_VOLUME_CACHE_MB`; a file over half the cap is streamed and not kept). Ranges and
    repeat GETs come from the cache, keyed by object id, size and mtime, dropped on
    upload/delete/move. libmtp allows one operation at a time per device, so transfers still
    serialize, but `stat` and `list` answer from the last listing while a transfer holds the
    session instead of freezing Finder. The 120 s cap is gone: a worker is judged stuck only after
    60 s with no progress (the transfer callbacks are its heartbeat).
  - WebDAVFS's empty create is held as a placeholder for up to 30 s (no USB upload) until the
    real PUT; uploads are verified by the size the Kindle records (`KINDLE_VOLUME_VERIFY=full`
    keeps the full read-back); every replacement is journaled in `state/journal/` before it
    starts and put right on the next connection (a backup with no destination is renamed back,
    staged copies deleted, a completed replacement's backup deleted). A short upload is deleted
    on the Kindle, the original kept.
  - A strange listing (invalid or duplicate names) skips those entries and logs them once; a
    non-empty rmdir is a 409. Neither ends the session.
  - The Finder share is `/Kindle` with Digest auth (user `kindle`, password = the token), so
    `mount` shows `http://127.0.0.1:18763/Kindle/` and no token; the token URL stays for tools
    and is the fallback should the Digest mount ever fail.
  - libmtp's stderr goes to `state/libmtp.log` (256 KB cap); `volume.log` is capped at 2 MB
    (copied to `volume.log.1`, only when launchd's descriptor is O_APPEND).
- `5d778c1` **The Kindle controller (usb-1 to usb-5).** `run.sh` supervises `usb_daemon.lua`:
  waits up to 60 s for `/mnt/us`, runs the luajit the installer copies to
  `/var/local/kindle-lab-usb/bin` (KOReader's as fallback), logs every exit to `daemon.log` and
  restarts with a 2 s to 30 s backoff. The upstart job execs `run.sh` with `respawn limit 10
  300`. Each tick runs under `xpcall` (traceback logged, loop continues). The env's
  `lipc.set_int` ignores `com.lab126.powerd/preventScreenSaver`, so the controller never owns
  keep-awake (vendor core byte-identical). volumd is read from `/proc/<pid>/stat` (pidof once,
  missing volumd looked for at most every 15 s); the cable from sysfs with `io.open` (two
  directory listings, then none). `usb-status` is written on change plus a `since=... beat=...`
  heartbeat a minute. A pulled cable (no USB supply or UDC `not attached`) stops the session
  after 3 s; a suspended bus (sleeping Mac) does not. A timeout with no host backs off 30, 60,
  120, 240, 300 s; a replug or a host on the bus re-arms at once. A stale `grimoire/.running` is
  removed. `install_usb_daemon.sh install|rollback|uninstall` is idempotent, checks
  `SHA256SUMS`, keeps v1 in `/var/local/kindle-lab-usb/rollback` on the first install;
  `deploy_usb_daemon.py` sends the bundle and runs it in one SSH session (`--dry-run` builds and
  shows it without connecting).
- `a88956f` **_meta.lua (usb-16):** the deprecated `name` key is gone.
- `13709c5` **install_mac.sh, manage.py, README.** `install_mac.sh` runs the offline tests in a
  scratch checkout, stops the bridge, unmounts, fast-forwards the live folder to `rc8`, runs
  `manage.py install-mac` (which now also sets `DSDontWriteNetworkStores`) and waits for a
  connection; `install_mac.sh rollback` returns to `pre-rc8`. `manage.py clean PATH... [--delete]`
  lists (removes) Mac metadata and `KindleLab(Upload|Backup)-*` copies already on the Kindle, and
  refuses to delete while uploads are still journaled.
- `6ab964d`, `f1d6a85`, `eb23cf1`: a file that changed size on the Kindle while read is a 409,
  not a session teardown, and `usb-status` is read as it is now (it changes size every write);
  test tidying.

## l10n

No user-facing strings changed (no `tr()` text, no app UI). Nothing for the catalog.

## Tests run and results

- `python3 -I tests/test_bluetooth_sh.py`: 8 tests OK (3 new). The 3 new ones fail on the old
  bluetooth.sh (checked against `git show HEAD~:kindle/app/bluetooth.sh`).
- `make package` then `tests/launcher_check.sh build/dist/Grimoire.zip`: LAUNCHER CHECK PASSED
  (sh and dash), shellcheck clean; new checks: the flag names the launcher and the build while
  Grimoire runs, goes with the restore (normal, watchdog hang and stale, hosted runs), and a
  refused second start leaves the running Grimoire's flag.
- `python3 -m unittest discover -s tests -p 'test_usb_stage.py'`: 5 tests OK, including a full
  stage under `/usr/bin/python3` 3.9.6. Also run by hand against the rc8 bridge app itself (fake
  Kindle behind it) under both Pythons: stage, `--clean-sidecars`, `--prune-keep 1` all worked,
  no `._` object reached the fake Kindle.
- KindleLab: `venv/bin/python -m unittest test_kindle_volume`: 35 tests OK (sidecars kept on
  the Mac and hidden, guard 423s and its exceptions, placeholders, streamed and ranged GETs from
  one download, Digest share and token share, backoff, open-error reasons, strange listings,
  mount parsing, unmount rule, controller heartbeat, cache during a transfer, attach retry,
  progress timeout, upload journal recovery, size verification, `manage.py clean`).
- `luajit test_usb_daemon.lua`: 29 checks OK (keep-awake never set through a real core
  start/stop, volumd and cable without forks, status on change plus heartbeat, charger backoff
  31 61 121 241 301 301 and replug re-arm, unplug after 3 s but not on a suspended bus, stale
  flag removal, a failing tick logged and survived, log cap).
- `venv/bin/python webdavfs_check.py`: PASSED. A real macOS WebDAVFS mount (scratch folder, port
  18799, fake Kindle): Digest mount works with the password from the environment, the `mount`
  line has no token, a `cp` of a quarantined file plus `mkdir`, `mv`, `rm`, `rmdir` reach the
  Kindle as one upload each, and the `._` files stay in `state/sidecars`.
- Scenarios: no Rust or web code changed, so no `.qs` scenario touches this lane's code; none run.
- Not run: `make check` (the integrator's), anything on the Kindle or against the live bridge.

## Left undone, and why

- **Nothing is installed or deployed.** The live bridge, its LaunchAgent and the Kindle were not
  touched (hard limits). The owner runs `install_mac.sh` and `deploy_usb_daemon.py` (below).
- **The five "while Grimoire is closed" help strings** (`tplsheet.rs:355`, `:359`;
  `bgwork_template.rs:358`, `:374`, `:391`) are unchanged. The guard keeps photos and templates
  writable while Grimoire runs, but Grimoire does not rescan those folders when files arrive, so
  the advice still holds in practice. Changing them (and adding a rescan) is the owner's call.
- **Whether the Kindle's MTP responder sees `.running`** created on the device while a USB
  session is open is unverified. If it does not, the guard fails open (writes allowed) until the
  next session. Owner test 6 checks it.
- **IOKit attach notifications** were replaced by a 2 s libusb enumeration in the bridge process
  (cheap, no extra helper or pyobjc).
- **Truly parallel reads** are not possible: libmtp serializes operations on one device. Listings
  answer from cache during transfers instead.
- **Existing `._` files and old staged bundles on the Kindle** are not deleted (owner's call):
  `manage.py clean /kindle-lab /grimoire-preview-kpm` lists them, `--delete` removes them;
  `tools/usb-stage.py --clean-sidecars --prune-keep 3` handles `kindle-lab/grimoire-dev`.
- The controller still starts on `started lab126` (proven on this Kindle); `run.sh` waits for
  `/mnt/us` instead of switching to a userstore job.

## Owner test plan

Install first (with me, Kindle on Wi-Fi for SSH, cable in):

1. Before anything, on the Kindle over SSH (read-only): `initctl status kindlelab-usb; grep -i
   kindlelab /var/log/messages | tail; cat /mnt/us/kindle-lab/volume/usb-status; ls -l
   /mnt/us/kindle-lab/volume/usb-status`. A job in stop/waiting with no reboot since the morning
   confirms the old controller died (usb-1).
2. Mac: `sh ~/KindleLab/volume-rc8/install_mac.sh`. It runs the offline tests, restarts the bridge
   on rc8 and prints `manage.py status`. Check `mount | grep 18763` shows `/Kindle/` and no token.
   Rollback if needed: `sh ~/KindleLab/volume/install_mac.sh rollback`.
3. Kindle: `cd ~/KindleLab/volume && venv/bin/python deploy_usb_daemon.py install --dry-run`, then
   `... install`. Then on the Kindle: `initctl status kindlelab-usb` (start/running, process is
   `sh run.sh`), `tail /var/local/kindle-lab-usb/daemon.log`, `ls /var/local/kindle-lab-usb/bin`
   (luajit copied). Rollback: `deploy_usb_daemon.py rollback`; removal: `... uninstall`.
4. Install the rc8 Grimoire build (for the `.running` flag and the bluetooth.sh fix).

Then:

5. **Cable in with Grimoire already open** (standalone from Kindle home): plug in. Within about
   30 s `/Volumes/Kindle` appears and `manage.py status` says `connected: true`,
   `kindle_controller.state: connected` with a small `beat_age_s`.
6. **Grimoire's data is protected:** with Grimoire open, `cat /Volumes/Kindle/grimoire/.running`
   shows `pid=... build=...` and `manage.py status` says `grimoire_running: true`. `cp x.png
   /Volumes/Kindle/grimoire/photos/` works; `touch /Volumes/Kindle/grimoire/settings.ini` fails
   (Finder: in use) and volume.log says `Refused PUT /grimoire/settings.ini`. Quit Grimoire, wait
   10 s: the same touch works. If `.running` does not show while Grimoire runs, the responder
   does not see new device files mid-session: tell me (the guard then needs another channel).
7. **Grimoire launched with the cable already in:** from Kindle home with Finder on
   `/Volumes/Kindle`, open Grimoire. Expect a 10 to 25 s outage, then the volume back. Quit
   Grimoire, then on the Kindle `lipc-get-prop -i com.lab126.powerd preventScreenSaver` must print
   0 (usb-2), and the stock UI must auto-sleep as usual.
8. **Hosted path:** KOReader with the cable in, wait for `/Volumes/Kindle`, open Grimoire from
   KOReader, list `/Volumes/Kindle/grimoire`, back to KOReader, list again. The mount should never
   drop for long.
9. **Unplug and replug with Grimoire open:** pull the cable. Within about 3 s
   `daemon.log` says `USB cable or host gone` and usb-status reads `unplugged`, then `no cable`;
   on the Mac `/Volumes/Kindle` disappears about 15 s later and `manage.py status` shows
   `reason: absent`. Plug back in and time how long until `/Volumes/Kindle` lists again (expect a
   few seconds after the Kindle side connects).
10. **Sleep and wake:** cable in, Grimoire open, sleep with the power button, wait 1 min, wake,
    `ls /Volumes/Kindle`. Note whether the Kindle slept with USB attached. Also sleep the Mac with
    the Kindle connected: the Kindle should keep its session (suspended bus is not an unplug).
11. **Wall charger:** Grimoire on a wall charger for 10 min. `daemon.log` should show tries
    backing off (`next try in 30 s`, 60, 120, 240, 300), not a cycle every 30 s.
12. **Mac reboot** with the Kindle plugged in and Grimoire open: after login `/Volumes/Kindle`
    appears without action; `launchctl print gui/$(id -u)/com.ganten.kindle-volume` shows it
    running.
13. **Controller crash recovery:** on the Kindle, `kill $(pidof luajit)` (the controller, not
    KOReader's: check `ps` first). `daemon.log` shows the exit and a restart 2 s later; the job
    stays start/running.
14. **Sidecars:** copy a PNG with Finder into `/Volumes/Kindle/kindle-lab/`; `manage.py clean
    /kindle-lab` must not list a new `._` file for it (old ones from before rc8 may be listed;
    `--delete` removes them). `ls ~/KindleLab/volume/state/sidecars/kindle-lab/` shows the
    `._` file kept on the Mac.
15. **Big file:** copy a file over 200 MB to the Kindle and back; Finder stays browsable while it
    copies and nothing reconnects (volume.log has no `no progress` line).
16. **Staging:** `python3 tools/usb-stage.py <bundle> --receipt /tmp/r.json --prune-keep 3`, and
    once with `/usr/bin/python3`; then `manage.py clean /kindle-lab` lists no `._` beside it.
