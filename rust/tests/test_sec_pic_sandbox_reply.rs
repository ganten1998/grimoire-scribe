// test_sec_pic_sandbox_reply.rs (security review 2026-11, finding "picture fallback after the bytes"): a picture
// worker that answers `E sandbox <step>` AFTER it was given a picture's bytes (what a worker that the picture took
// over can say: write is on the allow-list) must not make the root writer decode that picture itself, nor any
// later one. The design allows the 2.0 fallback only when no worker can be had before a byte is read (D21).
//
// The worker here is a stand-in for a compromised one: a shell script that gives the right handshake (B and S 1,
// what the real worker sends before it reads anything), then answers the picture with `E sandbox chroot`. Its own
// binary: the fallback is process-wide.
#![cfg(feature = "sim")]
mod testlib;
use grimoire::picwork::*;
use grimoire::prelude::*;
use testlib::*;

fn compromised_worker(dir: &str) -> String {
    use std::os::unix::fs::PermissionsExt;
    let path = format!("{dir}/worker.sh");
    let script = format!(
        "#!/bin/sh\nPATH=/usr/bin:/bin\nprintf 'B %s\\n' '{}' >&4\nprintf 'S 1\\n' >&4\nIFS= read -r line <&5\nprintf 'E sandbox chroot\\n' >&4\nsleep 10\n",
        grimoire::sandbox::build_id()
    );
    std::fs::write(&path, script).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path
}

fn a_late_sandbox_reply_is_a_lie() {
    let dir = format!("{}/sec-picsandbox-{}", tl_tmp(), std::process::id());
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::env::remove_var("GRIMOIRE_WORK_FAIL_STEP");
    std::env::set_var("GRIMOIRE_WORK_EXE", compromised_worker(&dir));
    let src = BgPicture { px: (0..50 * 60).map(|i| (i % 250) as u8).collect(), w: 50, h: 60, ch: 1 };
    let path = format!("{dir}/a.png");
    check!(bgimage_write_png(&path, &src) == 0);
    let bytes = std::fs::read(&path).unwrap();
    let mut pool = PicPool::new();
    let out = pool.decode(&bytes, false, 300, 400);
    eprintln!("  decode after a late `E sandbox`: {}", match &out {
        PicOut::Pic { .. } => "Pic (the root writer decoded the picture the worker had been given)".to_string(),
        o => format!("{o:?}"),
    });
    // the picture was handed to a worker that then claimed a failed sandbox step: it is refused, never decoded here
    check!(!matches!(out, PicOut::Pic { .. }));
    check!(matches!(out, PicOut::Dead(_)));
    // and the process does not go on to decode every later picture itself
    check!(!in_fallback());
    let head = pool.check(&bytes, bytes.len());
    eprintln!("  header check after it: {head:?}");
    check!(!matches!(head, PicOut::Info { .. }));
    let _ = std::fs::remove_dir_all(&dir);
}

// A worker that dies on a picture, and then no worker can be had (the next one does not answer its handshake): the
// picture the dead worker was given is not decoded in the writer either (the later pictures, never given to a
// worker, may still fall back as D21 has it).
fn a_picture_a_worker_died_on_never_falls_back() {
    use std::os::unix::fs::PermissionsExt;
    let dir = format!("{}/sec-picretry-{}", tl_tmp(), std::process::id());
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let count = format!("{dir}/started");
    let path = format!("{dir}/worker.sh");
    let script = format!(
        "#!/bin/sh\nPATH=/usr/bin:/bin\nif [ -e '{count}' ]; then exit 0; fi\n: > '{count}'\nprintf 'B %s\\n' '{}' >&4\nprintf 'S 1\\n' >&4\nIFS= read -r line <&5\nsleep 1\nexit 0\n",
        grimoire::sandbox::build_id()
    );
    std::fs::write(&path, script).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    std::env::set_var("GRIMOIRE_WORK_EXE", &path);
    let src = BgPicture { px: (0..40 * 30).map(|i| (i % 200) as u8).collect(), w: 40, h: 30, ch: 1 };
    let png = format!("{dir}/b.png");
    check!(bgimage_write_png(&png, &src) == 0);
    let bytes = std::fs::read(&png).unwrap();
    let mut pool = PicPool::new();
    let out = pool.decode(&bytes, false, 300, 400);
    eprintln!("  decode after the worker died and no other could be had: {}", match &out {
        PicOut::Pic { .. } => "Pic (the root writer decoded the picture the dead worker had been given)".to_string(),
        o => format!("{o:?}"),
    });
    check!(!matches!(out, PicOut::Pic { .. }));
    check!(matches!(out, PicOut::Dead(_)));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_sec_pic_sandbox_reply() {
    // (this order: the second leaves the process in the fallback, which the first must not find)
    run!(a_late_sandbox_reply_is_a_lie);
    run!(a_picture_a_worker_died_on_never_falls_back);
    done();
}
