// test_sec_tmp_link.rs (security review 2026-11, finding "a planted <name>.tmp link is written through"): every
// file Grimoire saves on the storage goes through write_file_atomic (util.rs): `<name>.tmp`, fsync, rename. The
// temporary was opened O_CREAT|O_TRUNC, following a link: a `<name>.tmp` that is a link to any file made root
// truncate that file and write the save into it (import/.attempts, the marks, notebook.ini, settings.ini and the
// rest), and the rename then put the link in the file's place. Planting the link needs a way to make one on
// /mnt/us (another program on the Kindle, a shell; never MTP).
use grimoire::prelude::*;

#[test]
fn a_save_never_writes_through_a_planted_tmp_link() {
    let base = std::env::var("GRIMOIRE_TEST_TMP").map(std::path::PathBuf::from).unwrap_or_else(|_| std::env::temp_dir());
    let top = base.join(format!("grimoire-sec-tmp-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&top);
    std::fs::create_dir_all(top.join("store")).unwrap();
    let victim = top.join("passwd");
    std::fs::write(&victim, b"root:x:0:0:root:/root:/bin/sh\n").unwrap();
    let path = top.join("store/.attempts");
    std::os::unix::fs::symlink(&victim, top.join("store/.attempts.tmp")).unwrap();
    let rc = write_file_atomic(path.to_str().unwrap(), b"# Grimoire import attempts\n");
    let v = std::fs::read(&victim).unwrap();
    eprintln!("  save: {rc}; the linked file now holds {:?}", String::from_utf8_lossy(&v));
    assert_eq!(v, b"root:x:0:0:root:/root:/bin/sh\n", "the save truncated and wrote the file its .tmp link points at");
    // the save is refused (a link stands where its temporary goes) or made in a file of its own: never through it
    assert_eq!(rc, -1);
    assert!(std::fs::symlink_metadata(&path).is_err(), "the link was renamed into the saved file's place");
    // and a temporary that is a FIFO does not hold the save, nor does one hard-linked to another file get cut
    std::fs::remove_file(top.join("store/.attempts.tmp")).unwrap();
    std::fs::hard_link(&victim, top.join("store/.attempts.tmp")).unwrap();
    assert_eq!(write_file_atomic(path.to_str().unwrap(), b"x\n"), -1);
    assert_eq!(std::fs::read(&victim).unwrap(), b"root:x:0:0:root:/root:/bin/sh\n", "a hard-linked temporary was cut");
    std::fs::remove_file(top.join("store/.attempts.tmp")).unwrap();
    let fifo = std::ffi::CString::new(top.join("store/.attempts.tmp").to_str().unwrap()).unwrap();
    // SAFETY: mkfifo reads the NUL-terminated path we own.
    assert_eq!(unsafe { libc::mkfifo(fifo.as_ptr(), 0o644) }, 0);
    let t0 = std::time::Instant::now();
    assert_eq!(write_file_atomic(path.to_str().unwrap(), b"z\n"), -1);
    assert!(t0.elapsed().as_secs() < 2, "a FIFO held the save");
    std::fs::remove_file(top.join("store/.attempts.tmp")).unwrap();
    // (a stale temporary of its own is written over, as before)
    std::fs::write(top.join("store/.attempts.tmp"), b"stale").unwrap();
    assert_eq!(write_file_atomic(path.to_str().unwrap(), b"y\n"), 0);
    assert_eq!(std::fs::read(&path).unwrap(), b"y\n");
    let _ = std::fs::remove_dir_all(&top);
}
