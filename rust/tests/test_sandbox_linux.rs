// test_sandbox_linux.rs: the safe loader's real steps (2.1.0-beta.2, DESIGN.md 3.7 and 4.2; areas/pdf.md 3.1).
// They need Linux and root (namespaces, chroot, setuid, seccomp), so they run in the CI job `sandbox`
// (.github/workflows/release.yml: sudo, GRIMOIRE_SANDBOX=require) and are skipped anywhere else, with a line
// saying so. THEY HAVE NOT BEEN RUN ON THIS MAC OR ON A KINDLE: the first CI run is their first run.
#![cfg(all(target_os = "linux", feature = "sim"))]
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
    let maps = std::fs::read_to_string(format!("/proc/{}/maps", w.pid)).unwrap();
    assert!(!maps.lines().any(|l| l.split_whitespace().nth(1).is_some_and(|p| p.ends_with('s') && p.starts_with("rw"))), "a shared writable mapping:\n{maps}");
    let exe = std::fs::read_link(format!("/proc/{}/exe", w.pid)).unwrap();
    assert!(exe.to_string_lossy().contains("grimoire-headless"));
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
