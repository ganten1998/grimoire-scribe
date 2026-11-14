// testlib: tests/testlib.h for the Rust ports of the C unit tests. Each tests/test_<name>.c becomes
// rust/tests/test_<name>.rs, one test binary, as each C test is its own binary: its TEST functions
// become plain fns, and one #[test] runs them in the C's RUN order with run!(), so the order a test
// file relies on (a store set up by the first) holds, and the output is the C's ("- name" per test,
// "  FAIL file:line: cond" per failed check, "N checks, M failures"). A failed check does not stop
// the run, as in C; the #[test] fails at the end (done()). Process-wide state (the store's root, the
// language) is the C's too, so the Makefile runs the tests one at a time (--test-threads=1).
//
//   mod testlib;
//   use testlib::*;
//   fn qr_modules() { check!(ui_qr_modules("http://192.168.0.20:8765") == 37); }
//   #[test]
//   fn test_qr() { run!(qr_modules); done(); }
#![allow(dead_code, unused_macros)]

use std::sync::atomic::{AtomicU32, Ordering};

pub static CHECKS: AtomicU32 = AtomicU32::new(0);
pub static FAILURES: AtomicU32 = AtomicU32::new(0);

#[macro_export]
macro_rules! check {
    ($cond:expr) => {{
        $crate::testlib::CHECKS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        // (the condition as a bool first: a comparison with a NaN is false, and fails the check)
        let ok: bool = $cond;
        if !ok {
            $crate::testlib::FAILURES.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            eprintln!("  FAIL {}:{}: {}", file!(), line!(), stringify!($cond));
        }
    }};
}

#[macro_export]
macro_rules! check_near {
    ($a:expr, $b:expr, $eps:expr) => {
        $crate::check!((($a) as f64 - ($b) as f64).abs() <= ($eps) as f64)
    };
}

#[macro_export]
macro_rules! run {
    ($name:ident) => {{
        eprintln!("- {}", stringify!($name));
        $name();
    }};
}

/// The tool sizes (size_min_mm, size_max_mm) the defaults had before 2.1's quick sizes moved them onto the pen-width
/// series: what the recorded goldens (the C build's hashes) and the pixel tolerances measured on them were made with.
pub const PRE_21_SIZES: [(grimoire::prelude::ToolKind, f32, f32); 7] = {
    use grimoire::prelude::*;
    [
        (TOOL_PENCIL, 0.12, 0.32),
        (TOOL_CHARCOAL, 0.30, 0.55),
        (TOOL_BALLPOINT, 0.30, 0.48),
        (TOOL_FINELINER, 0.28, 0.28),
        (TOOL_CALLIGRAPHY, 1.75, 1.75),
        (TOOL_HIGHLIGHTER, 5.0, 5.0),
        (TOOL_ERASER, 2.5, 7.0),
    ]
};

/// Tool `t`'s default parameters with the pre-2.1 sizes (a test that is not about the sizes pins them, so its
/// recorded hashes and measured tolerances stay what they were).
pub fn tool_params_default_pre21(t: grimoire::prelude::ToolKind, p: &mut grimoire::prelude::ToolParams) {
    grimoire::prelude::tool_params_default(t, p);
    // (a tool out of range gets the pencil's, as tool_params_default does)
    let known = (0..grimoire::prelude::TOOL_COUNT as i32).contains(&t.0);
    let key = if known { t } else { grimoire::prelude::TOOL_PENCIL };
    if let Some(&(_, lo, hi)) = PRE_21_SIZES.iter().find(|(k, _, _)| *k == key) {
        p.size_min_mm = lo;
        p.size_max_mm = hi;
    }
}

/// Every tool of `s` with the pre-2.1 sizes (after settings_default).
pub fn pin_pre21_sizes(s: &mut grimoire::prelude::Settings) {
    for &(t, lo, hi) in PRE_21_SIZES.iter() {
        s.tools[t.0 as usize].size_min_mm = lo;
        s.tools[t.0 as usize].size_max_mm = hi;
    }
}

/// tl_done: prints the tally; the #[test] fails if any check did.
pub fn done() {
    let (c, f) = (CHECKS.load(Ordering::Relaxed), FAILURES.load(Ordering::Relaxed));
    eprintln!("{c} checks, {f} failures");
    assert_eq!(f, 0, "{f} of {c} checks failed");
}

/// Scratch dir for tests (the Makefile sets GRIMOIRE_TEST_TMP). Without it, a grimoire-test folder in
/// the system's temporary directory, made here: the C's "/tmp" put the tests' `<tmp>/../out` at /out,
/// which a plain `cargo test` cannot write.
pub fn tl_tmp() -> String {
    std::env::var("GRIMOIRE_TEST_TMP").unwrap_or_else(|_| {
        let dir = std::env::temp_dir().join("grimoire-test");
        let _ = std::fs::create_dir_all(&dir);
        dir.to_string_lossy().into_owned()
    })
}

/// Runs `f`, which must not wait on the FIFO at `fifo`. A passing run waits for nothing. Only if `f` is still
/// running after 30 s does a helper thread open the FIFO (both ends at once, which never waits), hold it a
/// moment and close it, so that `f` returns; the test then fails.
pub fn never_waits<R>(fifo: &std::path::Path, f: impl FnOnce() -> R) -> R {
    let (r, waited) = waits_on(fifo, f);
    assert!(!waited, "the call waited on the FIFO at {}", fifo.display());
    r
}

/// never_waits without its assertion: what `f` returned, and whether it had to be let go (it waited on the
/// FIFO). For a test that checks several calls and names each one that waited.
pub fn waits_on<R>(fifo: &std::path::Path, f: impl FnOnce() -> R) -> (R, bool) {
    use std::os::unix::ffi::OsStrExt;
    use std::sync::atomic::AtomicBool;
    use std::sync::mpsc::RecvTimeoutError;
    use std::sync::Arc;
    let path = std::ffi::CString::new(fifo.as_os_str().as_bytes()).unwrap();
    let rescued = Arc::new(AtomicBool::new(false));
    let (tx, rx) = std::sync::mpsc::channel::<()>();
    let flag = rescued.clone();
    let helper = std::thread::spawn(move || {
        // (a disconnect is `f` panicking: no rescue; only the time running out is one)
        if rx.recv_timeout(std::time::Duration::from_secs(30)) == Err(RecvTimeoutError::Timeout) {
            flag.store(true, Ordering::SeqCst);
            // SAFETY: open reads the NUL-terminated path we own; the descriptor is closed below.
            let fd = unsafe { libc::open(path.as_ptr(), libc::O_RDWR | libc::O_NONBLOCK) };
            std::thread::sleep(std::time::Duration::from_millis(200));
            if fd >= 0 {
                // SAFETY: closing the descriptor opened above.
                unsafe { libc::close(fd) };
            }
        }
    });
    // (on the test's own thread: the store's state is the thread's)
    let r = f();
    let _ = tx.send(());
    let _ = helper.join();
    (r, rescued.load(Ordering::SeqCst))
}

/// Checks of one test that all run: a failed one is printed and does not hide the next, and the test fails at
/// the end naming each (done). For a test whose parts each pin a change of their own. (check! and done()
/// above count for the whole binary, which holds one test; this is for a binary of several.)
#[derive(Default)]
pub struct Checks(Vec<String>);
impl Checks {
    pub fn that(&mut self, ok: bool, what: &str) {
        if !ok {
            eprintln!("  FAIL: {what}");
            self.0.push(what.to_string());
        }
    }
    pub fn done(self) {
        assert!(self.0.is_empty(), "{} check(s) failed: {:?}", self.0.len(), self.0);
    }
}
