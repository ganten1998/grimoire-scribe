// test_picwork_fallback.rs: with the sandbox unavailable, pictures fall back to the writer's own decode and say
// so in the log (D21). Its own binary: the fallback holds for the rest of the process.
#![cfg(feature = "sim")]
mod testlib;
use grimoire::picwork::*;
use grimoire::prelude::*;
use testlib::*;

fn fallback_decodes_and_says_so() {
    std::env::set_var("GRIMOIRE_WORK_EXE", env!("CARGO_BIN_EXE_grimoire-headless"));
    std::env::set_var("GRIMOIRE_WORK_FAIL_STEP", "seccomp");
    let dir = format!("{}/picfallback-{}", tl_tmp(), std::process::id());
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let log = format!("{dir}/captured.log");
    let src = BgPicture { px: (0..50 * 60).map(|i| (i % 250) as u8).collect(), w: 50, h: 60, ch: 1 };
    let path = format!("{dir}/a.png");
    check!(bgimage_write_png(&path, &src) == 0);
    let bytes = std::fs::read(&path).unwrap();
    // stderr is the log: capture it for the run
    use std::io::Write;
    let _ = std::io::stderr().flush();
    // SAFETY: dup, open, dup2 and close on descriptors this test owns.
    let saved = unsafe {
        let saved = libc::dup(2);
        let c = std::ffi::CString::new(log.as_str()).unwrap();
        let fd = libc::open(c.as_ptr(), libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC, 0o644 as libc::c_uint);
        libc::dup2(fd, 2);
        libc::close(fd);
        saved
    };
    let mut pool = PicPool::new();
    let a = pool.decode(&bytes, false, 300, 400);
    let b = pool.decode(&bytes, false, 300, 400);
    let c = pool.check(&bytes, bytes.len());
    // SAFETY: dup2 and close on the descriptor saved above.
    unsafe {
        libc::dup2(saved, 2);
        libc::close(saved);
    }
    check!(matches!(a, PicOut::Pic { src_w: 50, src_h: 60, .. }));
    check!(matches!(b, PicOut::Pic { .. }));
    check!(matches!(c, PicOut::Info { w: 50, h: 60 }));
    check!(in_fallback());
    let text = std::fs::read_to_string(&log).unwrap_or_default();
    eprint!("{text}");
    check!(text.contains("pictures decode in the writer"));
    check!(text.matches("pictures decode in the writer").count() == 1); // (once)
    // a hostile picture is refused in the fallback too
    check!(matches!(pool.decode(b"no picture", false, 300, 400), PicOut::Bad(_)));
}

#[test]
fn test_picwork_fallback() {
    run!(fallback_decodes_and_says_so);
    done();
}
