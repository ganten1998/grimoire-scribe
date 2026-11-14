// forkcase: one case of a test, run in a forked child. For cases that change what is process-wide (the
// descriptor table, the limits, the environment, the session's broken and no-worker flags): the child makes
// the change, runs the case and ends, and the test harness's own process never sees it.
//
//   mod forkcase;
//   let (code, log) = forkcase::in_child(&format!("{dir}/log"), || { ...; 0 });
#![allow(dead_code)]

use std::io::Write;

/// One line to the case's log (the child's stderr, which `in_child` sent to the log file). Written to the
/// descriptor itself: the harness's output capture, which the child inherits a copy of, never takes it.
pub fn note(s: &str) {
    let _ = std::io::stderr().write_all(format!("{s}\n").as_bytes());
}

/// Runs `f` in a forked child and returns its exit code and the text of `log`. The child's stderr goes to the
/// file `log` (the log of the lib and of any process it starts), `f` runs inside `catch_unwind`, and the child
/// ends with `_exit` on every path (the code `f` returned, or 99 for a panic, whose message is in the log), so
/// a failed check never unwinds into a copy of the test harness. A child a signal ended gives 128 plus the
/// signal. The parent waits for the child's end; the 180 s bound only guards against a hang (the child is
/// then killed and the code is 98). Every environment change a case needs is made inside `f`, never in the
/// parent.
pub fn in_child(log: &str, f: impl FnOnce() -> i32) -> (i32, String) {
    let clog = std::ffi::CString::new(log).expect("a path without NUL");
    // SAFETY: fork. The child sends its stderr to the log, runs the case and ends by _exit on every path.
    let pid = unsafe { libc::fork() };
    assert!(pid >= 0, "fork failed");
    if pid == 0 {
        // SAFETY: open and dup2 on a path and descriptors this function owns; _exit ends the child at once.
        unsafe {
            let fd = libc::open(clog.as_ptr(), libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC | libc::O_APPEND, 0o644);
            if fd < 0 || libc::dup2(fd, 2) != 2 {
                libc::_exit(97);
            }
            if fd != 2 {
                libc::close(fd);
            }
        }
        let code = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
            Ok(code) => code,
            Err(p) => {
                let msg = p.downcast_ref::<&str>().map(|s| s.to_string()).or_else(|| p.downcast_ref::<String>().cloned()).unwrap_or_default();
                note(&format!("the case panicked: {msg}"));
                99
            }
        };
        // SAFETY: _exit ends the child at once, running nothing of the harness it is a copy of.
        unsafe { libc::_exit(code) };
    }
    let mut st = 0;
    let mut code = None;
    let t0 = std::time::Instant::now();
    while code.is_none() {
        // SAFETY: waitpid on the child just forked; WNOHANG.
        let r = unsafe { libc::waitpid(pid, &mut st, libc::WNOHANG) };
        if r == pid {
            code = Some(if libc::WIFEXITED(st) { libc::WEXITSTATUS(st) } else { 128 + libc::WTERMSIG(st) });
        } else if r < 0 {
            code = Some(96);
        } else if t0.elapsed().as_secs() >= 180 {
            // SAFETY: kill and waitpid on the child just forked.
            unsafe {
                libc::kill(pid, libc::SIGKILL);
                libc::waitpid(pid, &mut st, 0);
            }
            code = Some(98);
        } else {
            // SAFETY: usleep has no preconditions.
            unsafe { libc::usleep(2000) };
        }
    }
    (code.unwrap_or(96), String::from_utf8_lossy(&std::fs::read(log).unwrap_or_default()).into_owned())
}
