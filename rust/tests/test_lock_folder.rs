// Not a port of a C test: GRIMOIRE_LOCKS (util::lock_file_path), the folder for lock files when the data
// folder is on a file system without advisory locks. One test, because the variable is the process's:
// - unset or empty, a lock file's path is its natural one, unchanged (the Kindle);
// - set, the lock is `<folder>/<hash>.lock`: one file per natural path, the same at every call, the folder made
//   when missing, and the natural path is left alone;
// - the import lock goes there: nothing is made in import/, a second holder is refused in the folder, and the
//   lock is free again once the first lets go.
use grimoire::pdfimport::{lock_fd, lock_release_if_idle, lock_take, Lock};
use grimoire::prelude::*;
use std::os::fd::AsRawFd;

fn tmp(name: &str) -> std::path::PathBuf {
    let base = std::env::var("GRIMOIRE_TEST_TMP").map(std::path::PathBuf::from).unwrap_or_else(|_| std::env::temp_dir());
    base.join(format!("grimoire-lock-folder-{}-{name}", std::process::id()))
}

#[test]
fn lock_files_go_to_the_named_folder() {
    let root = tmp("root");
    let locks = tmp("locks");
    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&locks);
    std::fs::create_dir_all(root.join("import")).unwrap();
    let root_s = root.to_str().unwrap();
    let locks_s = locks.to_str().unwrap();
    let natural = format!("{root_s}/import/.lock");

    // unset, and empty: the natural path
    std::env::remove_var("GRIMOIRE_LOCKS");
    assert_eq!(lock_file_path(&natural), natural);
    std::env::set_var("GRIMOIRE_LOCKS", "");
    assert_eq!(lock_file_path(&natural), natural);

    // set: inside the folder (made now), one name per natural path, the same every time
    std::env::set_var("GRIMOIRE_LOCKS", format!("{locks_s}/"));
    assert!(!locks.exists());
    let there = lock_file_path(&natural);
    assert!(locks.is_dir(), "the folder is made when missing");
    assert_eq!(std::path::Path::new(&there).parent().unwrap(), locks.as_path());
    assert!(there.ends_with(".lock") && there.len() == locks_s.len() + 1 + 16 + 5, "{there}");
    assert_eq!(lock_file_path(&natural), there);
    assert_ne!(lock_file_path(&format!("{root_s}/notebooks/a/graph.ini.lock")), there);

    // the import lock is taken there, and import/ gets no lock file
    assert_eq!(store_init(root_s), 0);
    assert_eq!(lock_take(), Lock::Fresh);
    assert!(lock_fd() >= 0);
    assert!(std::path::Path::new(&there).is_file());
    assert!(!root.join("import/.lock").exists());
    // (another open file description, as another process would have)
    let other = std::fs::OpenOptions::new().read(true).write(true).open(&there).unwrap();
    // SAFETY: flock on a descriptor this test owns.
    assert_ne!(unsafe { libc::flock(other.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) }, 0, "the lock is held in the folder");
    assert_eq!(lock_take(), Lock::Held);
    lock_release_if_idle();
    assert_eq!(lock_fd(), -1);
    // SAFETY: as above.
    assert_eq!(unsafe { libc::flock(other.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) }, 0, "and free once let go");
    drop(other);

    std::env::remove_var("GRIMOIRE_LOCKS");
    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&locks);
}
