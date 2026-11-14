// test_sec_spawn_fds.rs (hardening round 2026-11): a worker is started with descriptors 0 to 5 in place whatever
// the writer's own descriptors are, and not at all when one cannot be put in place.
//
// Each case runs in a forked child (forkcase): it rearranges the process's own descriptors and limits, which
// the test harness must never see. GRIMOIRE_SANDBOX is left as the environment has it.
#![cfg(feature = "sim")]
mod forkcase;
mod pdffix;
mod testlib;
use grimoire::pdfwork::*;
use grimoire::sandbox::*;
use testlib::*;

fn scratch(name: &str) -> String {
    let dir = format!("{}/sec-spawn-fds-{}-{name}", tl_tmp(), std::process::id());
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Closes every descriptor from 3 to 1023 but `keep`.
fn close_from_3_but(keep: i32) {
    for fd in 3..1024 {
        if fd != keep {
            // SAFETY: closing descriptors of this forked child, which uses none of them.
            unsafe { libc::close(fd) };
        }
    }
}

// The writer has descriptors 0 and 1 closed and the PDF on 20: the worker still reads the PDF on 3, takes its
// commands on 5 and answers on 4.
fn a_writer_with_two_standard_descriptors_closed_gives_the_worker_all_six() {
    let dir = scratch("six");
    let (code, log) = forkcase::in_child(&format!("{dir}/log"), || {
        use std::os::fd::IntoRawFd;
        std::env::remove_var("GRIMOIRE_WORK_FAIL_STEP");
        std::env::set_var("GRIMOIRE_WORK_EXE", env!("CARGO_BIN_EXE_grimoire-headless"));
        let bytes = pdffix::planner(2);
        let pdf = format!("{dir}/a.pdf");
        std::fs::write(&pdf, &bytes).unwrap();
        let len = bytes.len() as u64;
        let fd = std::fs::File::open(&pdf).unwrap().into_raw_fd();
        // SAFETY: descriptor calls on this forked child's own table.
        unsafe {
            if fd != 20 {
                assert_eq!(libc::dup2(fd, 20), 20);
                libc::close(fd);
            }
            close_from_3_but(20);
            libc::close(0);
            libc::close(1);
        }
        let limits = Limits { as_bytes: worker_as_bytes(len, false), cpu_secs: 60, mapped: true, file_len: len };
        let mut s = match Session::start(20, len, &limits) {
            Ok(s) => s,
            Err(e) => {
                forkcase::note(&format!("the session did not start: {e:?}"));
                return 2;
            }
        };
        let pages = s.probe();
        forkcase::note(&format!("the probe: {pages:?}"));
        if pages != Ok(2) {
            return 3;
        }
        s.end();
        0
    });
    eprintln!("  exit code {code}; the case's log:\n{}", log.trim_end());
    check!(code == 0);
    let _ = std::fs::remove_dir_all(&dir);
}

// The writer's table is full above the five numbers a start takes before its fork, so the child can park
// nothing above the numbers a worker's descriptors take. No worker is started: the stand-in never runs, and
// the file the writer holds on descriptor 4 gets no byte.
fn a_spawn_whose_descriptors_cannot_be_arranged_starts_no_worker() {
    use std::os::unix::fs::PermissionsExt;
    let dir = scratch("none");
    let (started, canary, script) = (format!("{dir}/started"), format!("{dir}/canary"), format!("{dir}/worker.sh"));
    std::fs::write(&script, format!("#!/bin/sh\nPATH=/usr/bin:/bin\n: > '{started}'\nprintf 'B %s\\nS 1\\n' '{}' >&4\n", build_id())).unwrap();
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    let (code, log) = forkcase::in_child(&format!("{dir}/log"), || {
        use std::os::fd::IntoRawFd;
        std::env::remove_var("GRIMOIRE_WORK_FAIL_STEP");
        std::env::set_var("GRIMOIRE_WORK_EXE", &script);
        close_from_3_but(-1);
        // (3 and 4 are free now: the two opens take them in this order)
        let any = std::fs::File::open(&script).unwrap().into_raw_fd();
        let can = std::fs::File::create(&canary).unwrap().into_raw_fd();
        assert_eq!((any, can), (3, 4));
        // SAFETY: descriptor and limit calls on this forked child's own table.
        unsafe {
            // (std opens close-on-exec: the canary must cross an exec to show a start that should not have been)
            assert_eq!(libc::fcntl(can, libc::F_SETFD, 0), 0);
            let mut lim: libc::rlimit = std::mem::zeroed();
            assert_eq!(libc::getrlimit(libc::RLIMIT_NOFILE, &mut lim), 0);
            lim.rlim_cur = 64;
            assert_eq!(libc::setrlimit(libc::RLIMIT_NOFILE, &lim), 0);
            while libc::open(c"/dev/null".as_ptr(), libc::O_RDONLY) >= 0 {}
            for fd in 5..10 {
                libc::close(fd);
            }
        }
        let mut w = match Worker::spawn(WorkKind::Picture, None, &grimoire::picwork::limits()) {
            Ok(w) => w,
            Err(e) => {
                forkcase::note(&format!("the spawn: {e:?}"));
                return 2;
            }
        };
        // (the bounds only guard against a hang: the line ends at the end of the pipe, finish at the child's end)
        let line = w.read_line(512, mono_ms() + 120_000);
        let st = w.finish(60_000);
        let (ran, wrote) = (std::fs::metadata(&started).is_ok(), std::fs::metadata(&canary).map(|m| m.len()).unwrap_or(u64::MAX));
        forkcase::note(&format!(
            "the first line: {:?}; wait status {st:#x}; the stand-in ran: {ran}; bytes in the writer's own file on descriptor 4: {wrote}",
            line.as_ref().map(|l| String::from_utf8_lossy(l).into_owned())
        ));
        if line != Err(ReadErr::End) {
            return 3;
        }
        if !(libc::WIFEXITED(st) && libc::WEXITSTATUS(st) == 127) {
            return 4;
        }
        if ran {
            return 5;
        }
        if wrote != 0 {
            return 6;
        }
        0
    });
    eprintln!("  exit code {code}; the case's log:\n{}", log.trim_end());
    check!(code == 0);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_sec_spawn_fds() {
    run!(a_writer_with_two_standard_descriptors_closed_gives_the_worker_all_six);
    run!(a_spawn_whose_descriptors_cannot_be_arranged_starts_no_worker);
    done();
}
