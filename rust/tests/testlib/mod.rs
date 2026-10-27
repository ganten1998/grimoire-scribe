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

/// tl_done: prints the tally; the #[test] fails if any check did.
pub fn done() {
    let (c, f) = (CHECKS.load(Ordering::Relaxed), FAILURES.load(Ordering::Relaxed));
    eprintln!("{c} checks, {f} failures");
    assert_eq!(f, 0, "{f} of {c} checks failed");
}

/// Scratch dir for tests (the Makefile sets GRIMOIRE_TEST_TMP).
pub fn tl_tmp() -> String {
    std::env::var("GRIMOIRE_TEST_TMP").unwrap_or_else(|_| "/tmp".into())
}
