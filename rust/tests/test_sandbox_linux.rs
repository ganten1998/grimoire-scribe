// test_sandbox_linux.rs: the safe loader's real steps (2.1.0-beta.2, DESIGN.md 3.7 and 4.2; areas/pdf.md 3.1).
// They need Linux and root (namespaces, chroot, setuid, seccomp), so they run in the CI job `sandbox`
// (.github/workflows/release.yml: sudo, GRIMOIRE_SANDBOX=require) and are skipped anywhere else, with a line
// saying so. THEY HAVE NOT BEEN RUN ON THIS MAC OR ON A KINDLE: the first CI run is their first run.
#![cfg(all(target_os = "linux", feature = "sim"))]
mod fontfix;
mod pdffix;
use grimoire::pdfwork::*;
use grimoire::sandbox::*;
use std::os::fd::AsRawFd;
use std::sync::Mutex;

static LOCK: Mutex<()> = Mutex::new(());

fn root() -> bool {
    // SAFETY: geteuid has no preconditions.
    let r = unsafe { libc::geteuid() } == 0;
    if !r {
        eprintln!("skipped: the sandbox's real steps need root (CI runs them with sudo)");
    }
    r
}

fn setup() -> (std::sync::MutexGuard<'static, ()>, std::fs::File, u64) {
    let g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    for h in ["GRIMOIRE_WORK_FAIL_STEP", "GRIMOIRE_PDF_TEST_TRAP_PAGE"] {
        std::env::remove_var(h);
    }
    std::env::set_var("GRIMOIRE_SANDBOX", "require");
    std::env::set_var("GRIMOIRE_WORK_EXE", env!("CARGO_BIN_EXE_grimoire-headless"));
    let dir = std::env::temp_dir().join(format!("grimoire-sandbox-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let p = dir.join("a.pdf");
    let bytes = pdffix::planner(2);
    std::fs::write(&p, &bytes).unwrap();
    (g, std::fs::File::open(&p).unwrap(), bytes.len() as u64)
}

fn limits(len: u64) -> Limits {
    Limits { as_bytes: worker_as_bytes(len, false), cpu_secs: 60, mapped: true, file_len: len }
}

#[test]
fn a_planner_renders_in_the_real_sandbox() {
    if !root() {
        return;
    }
    let (_g, f, len) = setup();
    let mut s = Session::start(f.as_raw_fd(), len, &limits(len)).expect("the worker starts sandboxed");
    assert!(s.sandboxed, "the steps ran");
    assert_eq!(s.probe(), Ok(2));
    match s.render(1, 600, 800, 1) {
        PageOutcome::Page(p) => assert_eq!((p.w, p.h, p.ch), (600, 800, 1)),
        other => panic!("{other:?}"),
    }
    s.end();
}

#[test]
fn a_forced_failure_of_each_step_parses_nothing() {
    if !root() {
        return;
    }
    for step in ["oom", "namespaces", "chroot", "uid", "privileges", "pdeathsig", "rlimits", "seccomp"] {
        let (_g, f, len) = setup();
        std::env::set_var("GRIMOIRE_WORK_FAIL_STEP", step);
        let r = Session::start(f.as_raw_fd(), len, &limits(len));
        assert_eq!(r.err(), Some(StartErr::Sandbox(step.to_string())), "{step}");
        // not a byte of the source was read: the descriptor's offset is where it began
        // SAFETY: lseek on the descriptor we own.
        assert_eq!(unsafe { libc::lseek(f.as_raw_fd(), 0, libc::SEEK_CUR) }, 0, "{step}");
    }
}

#[test]
fn a_call_outside_the_list_kills_the_worker_and_says_which() {
    if !root() {
        return;
    }
    let (_g, f, len) = setup();
    let mut s = Session::start(f.as_raw_fd(), len, &limits(len)).unwrap();
    assert_eq!(s.probe(), Ok(2));
    assert!(s.hook("trap", 1));
    match s.render(1, 600, 800, 1) {
        PageOutcome::Died(Death::Sigsys(n)) => assert_eq!(n as i64, libc::SYS_getpid as i64),
        other => panic!("{other:?}"),
    }
}

#[test]
fn the_workers_maps_hold_nothing_of_the_writer() {
    if !root() {
        return;
    }
    let (_g, f, len) = setup();
    let mut w = Worker::spawn(WorkKind::Pdf, Some(f.as_raw_fd()), &limits(len)).unwrap();
    // Worker::spawn returns right after its fork: until the child's execve, /proc/<pid>/exe and maps are still
    // this test's own binary (a copy of the writer). The handshake comes after the exec and the steps, so the
    // worker is read only once it has said `S 1`.
    let deadline = mono_ms() + 15_000;
    let hello = w.read_line(512, deadline).expect("its build line");
    assert!(hello.starts_with(b"B "), "{:?}", String::from_utf8_lossy(&hello));
    assert_eq!(String::from_utf8_lossy(&w.read_line(512, deadline).expect("its sandbox line")), "S 1");
    let maps = std::fs::read_to_string(format!("/proc/{}/maps", w.pid)).unwrap();
    assert!(!maps.lines().any(|l| l.split_whitespace().nth(1).is_some_and(|p| p.ends_with('s') && p.starts_with("rw"))), "a shared writable mapping:\n{maps}");
    assert!(!maps.lines().any(|l| l.split_whitespace().nth(1).is_some_and(|p| p.contains('w') && p.contains('x'))), "a mapping both writable and executable:\n{maps}");
    let me = std::fs::canonicalize(std::env::current_exe().unwrap()).unwrap();
    assert!(!maps.contains(me.to_string_lossy().as_ref()), "the writer's own binary ({me:?}) is mapped in the worker:\n{maps}");
    // /proc/<pid>/exe is the file the worker was exec'd from: GRIMOIRE_WORK_EXE outside the Kindle build
    // (sandbox.rs work_exe), its links resolved, as this process sees it (the worker's chroot does not change
    // what it reads), with " (deleted)" after it when the file was replaced since (a rebuild during the run).
    let want = std::fs::canonicalize(std::env::var_os("GRIMOIRE_WORK_EXE").expect("setup sets it")).unwrap();
    let exe = std::fs::read_link(format!("/proc/{}/exe", w.pid)).unwrap();
    let seen = exe.to_string_lossy();
    let path = seen.strip_suffix(" (deleted)").unwrap_or(&seen);
    assert_eq!(
        std::path::Path::new(path),
        want.as_path(),
        "/proc/{}/exe is {seen:?}, not the GRIMOIRE_WORK_EXE this test set ({want:?}): before the worker's execve it is \
         the test binary ({me:?})",
        w.pid
    );
    w.kill_and_reap();
}

#[test]
fn killing_the_writer_kills_the_worker() {
    if !root() {
        return;
    }
    let (_g, f, len) = setup();
    let mut p = [0i32; 2];
    // SAFETY: a pipe, and a fork whose child only spawns a worker, reports its pid and sleeps.
    unsafe {
        libc::pipe(p.as_mut_ptr());
        let writer = libc::fork();
        if writer == 0 {
            let w = Worker::spawn(WorkKind::Pdf, Some(f.as_raw_fd()), &limits(len)).unwrap();
            let pid = w.pid.to_string();
            libc::write(p[1], pid.as_ptr() as *const libc::c_void, pid.len());
            std::mem::forget(w);
            libc::sleep(60);
            libc::_exit(0);
        }
        libc::close(p[1]);
        let mut buf = [0u8; 16];
        let n = libc::read(p[0], buf.as_mut_ptr() as *mut libc::c_void, 16);
        let worker: i32 = std::str::from_utf8(&buf[..n as usize]).unwrap().parse().unwrap();
        libc::kill(writer, libc::SIGKILL);
        let mut st = 0;
        libc::waitpid(writer, &mut st, 0);
        let mut gone = false;
        for _ in 0..100 {
            if libc::kill(worker, 0) != 0 {
                gone = true;
                break;
            }
            libc::usleep(50_000);
        }
        assert!(gone, "the worker outlived its writer");
    }
}

#[test]
fn with_every_step_but_the_filter_no_descriptor_can_be_made() {
    if !root() {
        return;
    }
    // In a forked child: the steps in order (chroot to an empty directory, the uid, no new privileges, the
    // limits), then close(0): socket() and open() both fail, whatever the process closed.
    let dir = format!("/tmp/grimoire-nofile-{}", std::process::id());
    std::fs::create_dir_all(&dir).unwrap();
    // SAFETY: a fork whose child runs the steps and reports through its exit status only.
    unsafe {
        let pid = libc::fork();
        if pid == 0 {
            let a = WorkArgs { kind: WorkKind::Pdf, writer_pid: libc::getppid(), limits: Limits { as_bytes: 1 << 30, cpu_secs: 30, mapped: true, file_len: 0 }, jail: Some(dir.clone()), fail_step: None };
            let mut steps = standard_steps(&a);
            steps.retain(|s| s.name != "seccomp" && s.name != "pdeathsig");
            if apply_steps(&mut steps, true, |_, _, _| {}).is_err() {
                libc::_exit(10);
            }
            libc::close(0);
            let s = libc::socket(libc::AF_INET, libc::SOCK_STREAM, 0);
            let o = libc::open(c"/".as_ptr(), libc::O_RDONLY);
            libc::_exit(if s < 0 && o < 0 { 0 } else { 11 });
        }
        let mut st = 0;
        libc::waitpid(pid, &mut st, 0);
        assert!(libc::WIFEXITED(st) && libc::WEXITSTATUS(st) == 0, "child status {st}");
    }
    let _ = std::fs::remove_dir(&dir);
}

#[test]
fn the_filter_is_the_one_the_tests_evaluate() {
    // the filter the worker installs is built by build_filter over this architecture's list: the host
    // tests evaluate that very function, so here it is enough that the list is what they evaluated
    assert_eq!(allow_list(), allow_list_for(this_arch()));
}

// Pictures (lane L7): decoded in the real sandbox, a trap ends the worker and refuses the picture (never a
// fallback to the writer's own decode), and a forced failed step falls back and says so.
#[test]
fn pictures_decode_in_the_real_sandbox() {
    use grimoire::picwork::*;
    use grimoire::prelude::*;
    if !root() {
        return;
    }
    let (_g, _f, _len) = setup();
    std::env::remove_var("GRIMOIRE_PIC_TEST_TRAP");
    let px: Vec<u8> = (0..80 * 60).map(|i| (i % 250) as u8).collect();
    let src = BgPicture { px, w: 80, h: 60, ch: 1 };
    let path = std::env::temp_dir().join(format!("grimoire-sandbox-{}/p.png", std::process::id()));
    assert_eq!(bgimage_write_png(path.to_str().unwrap(), &src), 0);
    let bytes = std::fs::read(&path).unwrap();
    let mut pool = PicPool::new();
    assert!(matches!(pool.decode(&bytes, false, 300, 400), PicOut::Pic { src_w: 80, src_h: 60, .. }));
    assert!(!in_fallback(), "the sandbox held");
    std::env::set_var("GRIMOIRE_PIC_TEST_TRAP", "2");
    let mut pool = PicPool::new();
    assert!(matches!(pool.decode(&bytes, false, 300, 400), PicOut::Pic { .. }));
    assert!(matches!(pool.decode(&bytes, false, 300, 400), PicOut::Dead(_)));
    std::env::remove_var("GRIMOIRE_PIC_TEST_TRAP");
    assert!(!in_fallback());
}

// Fonts (lane L10): checked in the real sandbox, with every call the check makes on the allow-list (the walk of
// every glyph, the string fit through the embedded fonts, the CPU clock that ends a slice); a trap ends the
// worker and the font is marked (never checked by the writer itself).
#[test]
fn a_font_is_checked_in_the_real_sandbox() {
    use grimoire::fontcheck::*;
    if !root() {
        return;
    }
    let (_g, _f, _len) = setup();
    for h in ["GRIMOIRE_FONT_TEST_TRAP", "GRIMOIRE_FONT_SLICE_MS"] {
        std::env::remove_var(h);
    }
    let font = fontfix::ttf(&fontfix::test_font_many(3000));
    match check_font(&font, 44) {
        FontOut::Passed { facts, w, h, .. } => {
            assert_eq!(facts.mapped, 3095);
            assert!(facts.skip.is_empty() && w > 100 && h == 63);
        }
        other => panic!("{other:?}"),
    }
    // in slices (a fresh sandboxed worker each), and a CFF font, and one the check refuses
    std::env::set_var("GRIMOIRE_FONT_SLICE_MS", "2");
    assert!(matches!(check_font(&font, 44), FontOut::Passed { .. }));
    std::env::remove_var("GRIMOIRE_FONT_SLICE_MS");
    assert!(matches!(check_font(&fontfix::otf(&fontfix::test_font("Regular", 400), 2, false), 44), FontOut::Passed { .. }));
    assert!(matches!(check_font(&fontfix::ttf(&fontfix::cyclic_composite(true)), 44), FontOut::Refused(Refuse::Unreadable)));
    assert!(matches!(check_font(&fontfix::woff2(), 44), FontOut::Refused(Refuse::Format)));
    // a call outside the list: the worker is ended, and the font is not passed
    std::env::set_var("GRIMOIRE_FONT_TEST_TRAP", "40");
    assert!(matches!(check_font(&font, 44), FontOut::Dead(_)));
    std::env::remove_var("GRIMOIRE_FONT_TEST_TRAP");
    assert!(!session_broken(), "a trap is the worker's end, not the sandbox's");
    // (a forced failed step, which checks nothing and leaves the session without workers, is test_fontcheck.rs's:
    // it would take the workers away from the tests after this one)
}

// The worker's clean end (the 2.1.0-beta.2 fix round): `--work` ends with _exit, so a worker told to stop under
// the filter makes no call outside the list on its way out (the runtime's own exit lets go of the main thread's
// signal stack with sigaltstack, 186 on the Kindle, which would read as `E sigsys 186` at every clean end).
#[test]
fn a_workers_clean_end_makes_no_call_outside_the_list() {
    if !root() {
        return;
    }
    let (_g, _f, _len) = setup();
    let mut w = Worker::spawn(WorkKind::Picture, None, &grimoire::picwork::limits()).expect("a picture worker starts sandboxed");
    let deadline = mono_ms() + 15_000;
    let hello = w.read_line(512, deadline).expect("its build line");
    assert!(hello.starts_with(b"B "), "{:?}", String::from_utf8_lossy(&hello));
    assert_eq!(w.read_line(512, deadline).expect("its sandbox line"), b"S 1");
    assert!(w.send("Q\n"));
    let st = w.finish(5000);
    assert!(libc::WIFEXITED(st) && libc::WEXITSTATUS(st) == 0, "a clean end, not a signal (SIGSYS is 31): status {st:#x}");
}

// ---- the hardening round (2026-11): futex, mmap and mprotect are judged by an argument ----
// These need the filter and no jail, so they also run without root where a worker loads the filter without it.

/// As `setup`, for the tests that need the filter only: a process that is not root asks for no sandbox (no
/// jail is needed, and a Linux worker still loads the filter without root).
fn setup_filter() -> (std::sync::MutexGuard<'static, ()>, std::fs::File, u64) {
    let r = setup();
    // SAFETY: geteuid has no preconditions.
    if unsafe { libc::geteuid() } != 0 {
        std::env::remove_var("GRIMOIRE_SANDBOX");
    }
    r
}

/// Whether a worker runs under the filter here: a session whose `trap` hook makes a call outside the list
/// must end with the filter's report. As root it does, or the test fails; without root a host that loads no
/// filter is skipped, with a line saying so.
fn filter_is_on(f: &std::fs::File, len: u64) -> bool {
    let on = matches!(page_after_hook(f, len, "trap"), PageOutcome::Died(Death::Sigsys(n)) if n as i64 == libc::SYS_getpid as i64);
    if !on {
        // SAFETY: geteuid has no preconditions.
        assert!(unsafe { libc::geteuid() } != 0, "as root a worker runs under the filter");
        eprintln!("skipped: this host loads no filter without root");
    }
    on
}

/// How page 1 of the planner ends in a worker told to run `hook` when it reaches that page.
fn page_after_hook(f: &std::fs::File, len: u64, hook: &str) -> PageOutcome {
    let mut s = Session::start(f.as_raw_fd(), len, &limits(len)).expect("a worker starts");
    assert_eq!(s.probe(), Ok(2));
    assert!(s.hook(hook, 1));
    s.render(1, 600, 800, 1)
}

/// A file of `bytes` in the tests' folder (`setup` made it), opened afresh.
fn file_of(name: &str, bytes: &[u8]) -> (std::fs::File, u64) {
    let p = std::env::temp_dir().join(format!("grimoire-sandbox-{}/{name}", std::process::id()));
    std::fs::write(&p, bytes).unwrap();
    (std::fs::File::open(&p).unwrap(), bytes.len() as u64)
}

// A futex operation outside the three a worker can make (a requeue between two words of its own) ends the worker,
// and the report names futex.
#[test]
fn a_futex_call_outside_the_list_kills_the_worker() {
    let (_g, f, len) = setup_filter();
    if !filter_is_on(&f, len) {
        return;
    }
    match page_after_hook(&f, len, "futexbad") {
        PageOutcome::Died(Death::Sigsys(n)) => assert_eq!(n, this_arch().futex),
        other => panic!("{other:?}"),
    }
}

// A mapping asked executable, by mmap or by mprotect, ends the worker, and the report names the call.
#[test]
fn an_executable_mapping_kills_the_worker() {
    let (_g, f, len) = setup_filter();
    if !filter_is_on(&f, len) {
        return;
    }
    for (hook, call) in [("execmap", this_arch().mmap), ("execprot", this_arch().mprotect)] {
        match page_after_hook(&f, len, hook) {
            PageOutcome::Died(Death::Sigsys(n)) => assert_eq!(n, call, "{hook}"),
            other => panic!("{hook}: {other:?}"),
        }
    }
}

// (the guard: the three operations one thread can issue, and the standard library's own wake, pass)
#[test]
fn the_futex_calls_a_worker_can_make_pass() {
    let (_g, f, len) = setup_filter();
    if !filter_is_on(&f, len) {
        return;
    }
    match page_after_hook(&f, len, "futexok") {
        PageOutcome::Page(p) => assert_eq!((p.w, p.h, p.ch), (600, 800, 1)),
        other => panic!("{other:?}"),
    }
}

// (the guard: a panic that unwinds under the filter makes no call outside the list, and is reported as one)
#[test]
fn a_panic_under_the_filter_is_reported_as_a_panic() {
    let (_g, f, len) = setup_filter();
    if !filter_is_on(&f, len) {
        return;
    }
    match page_after_hook(&f, len, "panic") {
        PageOutcome::Died(Death::Panic) => {}
        other => panic!("{other:?}"),
    }
}

// (the guard: the work no other test runs under the real filter. A colour page in bands, the link walk, a PDF
// read whole instead of mapped, a PDF with a password, and JPEG pictures, one turned upright.)
#[test]
fn the_work_a_worker_does_passes_the_filter() {
    use grimoire::picwork::{in_fallback, PicOut, PicPool};
    let (_g, f, len) = setup_filter();
    if !filter_is_on(&f, len) {
        return;
    }
    let (cf, clen) = file_of("colour.pdf", &pdffix::colour());
    let mut s = Session::start(cf.as_raw_fd(), clen, &limits(clen)).expect("a worker starts");
    assert_eq!(s.probe(), Ok(3));
    match s.render(2, 600, 800, 4) {
        PageOutcome::Page(p) => assert_eq!((p.w, p.h, p.ch), (600, 800, 3)),
        other => panic!("the colour page in bands: {other:?}"),
    }
    s.end();
    let (tf, tlen) = file_of("tabs.pdf", &pdffix::tabs());
    let mut s = Session::start(tf.as_raw_fd(), tlen, &limits(tlen)).expect("a worker starts");
    assert_eq!(s.probe(), Ok(4));
    match s.links(600, 800) {
        Ok(got) => assert!(!got.links.is_empty(), "the walk found no link"),
        Err(why) => panic!("the link walk: {why}"),
    }
    s.end();
    let (rf, rlen) = file_of("read.pdf", &pdffix::planner(2));
    let mut s = Session::start(rf.as_raw_fd(), rlen, &Limits { mapped: false, ..limits(rlen) }).expect("a worker starts");
    assert_eq!(s.probe(), Ok(2));
    assert!(matches!(s.render(1, 600, 800, 1), PageOutcome::Page(_)), "a PDF read whole");
    s.end();
    let (lf, llen) = file_of("locked.pdf", &pdffix::encrypted_planner(2, "secret", "owner"));
    let mut s = Session::start(lf.as_raw_fd(), llen, &limits(llen)).expect("a worker starts");
    assert_eq!(s.probe(), Err(StartErr::Password));
    drop(s);
    let fix = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/bgimage");
    let mut pool = PicPool::new();
    let jpeg = std::fs::read(format!("{fix}/base_420.jpg")).unwrap();
    assert!(matches!(pool.decode(&jpeg, false, 300, 400), PicOut::Pic { src_w: 67, src_h: 39, .. }));
    // (24 x 16 with orientation 6: upright it is 16 x 24)
    let turned = std::fs::read(format!("{fix}/exif_le_6.jpg")).unwrap();
    assert!(matches!(pool.decode(&turned, true, 300, 400), PicOut::Pic { src_w: 16, src_h: 24, .. }));
    assert!(!in_fallback(), "the pictures were decoded by a worker");
}

// (the guard: sandbox.rs types its numbers by hand, and this job is where they are first held to libc's)
#[test]
fn the_hand_typed_numbers_match_libc() {
    let n = this_arch();
    assert_eq!(
        [n.read, n.write, n.munmap, n.mremap, n.madvise, n.mprotect, n.brk, n.futex, n.clock_gettime, n.rt_sigreturn, n.exit, n.exit_group],
        [
            libc::SYS_read as u32,
            libc::SYS_write as u32,
            libc::SYS_munmap as u32,
            libc::SYS_mremap as u32,
            libc::SYS_madvise as u32,
            libc::SYS_mprotect as u32,
            libc::SYS_brk as u32,
            libc::SYS_futex as u32,
            libc::SYS_clock_gettime as u32,
            libc::SYS_rt_sigreturn as u32,
            libc::SYS_exit as u32,
            libc::SYS_exit_group as u32
        ]
    );
    #[cfg(target_arch = "arm")]
    assert_eq!(n.mmap, libc::SYS_mmap2 as u32);
    #[cfg(not(target_arch = "arm"))]
    assert_eq!(n.mmap, libc::SYS_mmap as u32);
    assert_eq!(
        n.reach_out,
        [libc::SYS_socket as u32, libc::SYS_execve as u32, libc::SYS_kill as u32, libc::SYS_ptrace as u32, libc::SYS_mount as u32, libc::SYS_chroot as u32]
    );
    assert!(n.enosys.contains(&(libc::SYS_gettid as u32)));
    assert_eq!(
        [FUTEX_WAIT, FUTEX_WAKE, FUTEX_WAIT_BITSET, FUTEX_PRIVATE_FLAG, PROT_EXEC],
        [libc::FUTEX_WAIT as u32, libc::FUTEX_WAKE as u32, libc::FUTEX_WAIT_BITSET as u32, libc::FUTEX_PRIVATE_FLAG as u32, libc::PROT_EXEC as u32]
    );
    assert_eq!(
        FUTEX_ALLOWED,
        [
            (libc::FUTEX_WAIT | libc::FUTEX_PRIVATE_FLAG) as u32,
            (libc::FUTEX_WAKE | libc::FUTEX_PRIVATE_FLAG) as u32,
            (libc::FUTEX_WAIT_BITSET | libc::FUTEX_PRIVATE_FLAG) as u32
        ]
    );
    // where the filter reads an argument: the low half of args[i] in the kernel's own struct
    assert_eq!(arg_lo(0) as usize, std::mem::offset_of!(libc::seccomp_data, args));
    assert_eq!(arg_lo(2) as usize, std::mem::offset_of!(libc::seccomp_data, args) + 2 * std::mem::size_of::<u64>());
}

// The mirror child's cap at the Scribe's page size (H3): a notebook of colour pages, each on a photo-like
// picture of its own (an imported PDF's), and two on one picture (a colour template's), mirrored in a forked
// child under the very RLIMIT_AS a mirror child gets (child_margin(BgKind::MirrorPdf) above its size at the
// fork). No root needed: the cap is the child's own soft limit. rust/tests/test_pdf_colour_memory.rs measures
// the same pages on the Mac.
#[test]
fn a_scribe_sized_colour_page_mirrors_under_the_mirror_childs_cap() {
    use grimoire::prelude::*;
    let _g = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let (w, h) = (1860i32, 2480i32);
    let dir = std::env::temp_dir().join(format!("grimoire-sandbox-colour-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    assert_eq!(store_init(dir.to_str().unwrap()), 0);
    store_set_device(300.0, w, h);
    let mut nb = NotebookInfo::default();
    assert_eq!(store_create("Colour", "", PAPER_BLANK, &mut nb), 0);
    for i in 1..4 {
        assert_eq!(store_add_page(&mut nb, i, PAPER_BLANK), 0);
    }
    let mut bg = String::new();
    bgimage_dir(nb.id.as_str(), &mut bg, 700);
    assert_eq!(mkdir_p(&bg), 0);
    for i in 0..4 {
        let id = 0x30 + (i as u32).min(2);
        if i <= 2 {
            let mut seed = 777u32 + i as u32;
            let mut px = vec![0u8; (w * h * 3) as usize];
            for y in 0..h {
                for x in 0..w {
                    let at = ((y * w + x) * 3) as usize;
                    for (c, base) in [(x * 200 / w) as u8, (y * 200 / h) as u8, ((x + y) * 100 / (w + h)) as u8].into_iter().enumerate() {
                        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                        px[at + c] = base.wrapping_add(((seed >> 24) as u8) & 31);
                    }
                }
            }
            let mut f = String::new();
            bgimage_file(&bg, id, &mut f, 800);
            assert_eq!(bgimage_write_png(&f, &BgPicture { px, w, h, ch: 3 }), 0);
        }
        assert_eq!(store_save_page(&nb, i, &page_new(w, h, paper_image(PAPER_ROLE_DOCUMENT, id))), 0);
    }
    let mut b = NotebookInfo::default();
    assert_eq!(store_load_info(nb.id.as_str(), &mut b), 0);
    let out = format!("{}/PDFs", dir.display());
    // SAFETY: fork; the child only takes the caps, exports and _exits.
    let pid = unsafe { libc::fork() };
    if pid == 0 {
        let took = child_caps_apply(child_margin(BgKind::MirrorPdf));
        let rc = export_mirror_pdf(&b, 300.0, &out);
        // SAFETY: _exit ends the child at once.
        unsafe { libc::_exit(if took.as_bytes.is_none() { 3 } else if rc == 0 { 0 } else { 2 }) };
    }
    let mut st = 0;
    // SAFETY: waitpid on the child just forked.
    unsafe { libc::waitpid(pid, &mut st, 0) };
    assert!(libc::WIFEXITED(st), "the mirror child ended by itself: status {st:#x}");
    assert_eq!(libc::WEXITSTATUS(st), 0, "the mirror completed under its cap (2: it failed, 3: no cap was taken)");
    let mut path = String::new();
    export_mirror_path(&b, &out, &mut path, 1000);
    let pdf = std::fs::read(&path).unwrap();
    assert_eq!(String::from_utf8_lossy(&pdf).matches("/DeviceRGB").count(), 4, "every page in colour");
    let _ = std::fs::remove_dir_all(&dir);
}
