// test_sec_records_cap.rs (hardening round 2026-11): the limit on a worker's record lines (RECORDS_MAX_BYTES,
// pdfwork.rs) is counted for each worker. A worker whose lines pass it is ended, and the next worker of the same
// job starts its count afresh: what the first sent is not held against it.
//
// The stand-in workers here shake hands, answer the probe with 3 pages, and answer LINKS with a file of link
// records the test wrote: one file is over the cap, the other well under it. Every line is one the grammar and
// the page count take, and both files stay under the cap on links (LINKS_MAX), so the count of bytes alone
// decides. No time is measured.
#![cfg(feature = "sim")]
use grimoire::pdfwork::*;
use grimoire::sandbox::*;

/// `n` links of a 3-page PDF, each with its quad, as a worker sends them.
fn link_records(n: usize) -> String {
    let mut t = String::new();
    for i in 0..n {
        t.push_str(&format!("L 1 2 100000 100000 100000 100000 {i}\n"));
        t.push_str("Q 1000000 1000000 1000000 1000000 1000000 1000000 1000000 1000000\n");
    }
    t
}

#[test]
fn a_workers_records_end_at_the_cap_and_the_next_worker_starts_afresh() {
    use std::os::unix::fs::PermissionsExt;
    let base = std::env::var("GRIMOIRE_TEST_TMP").map(std::path::PathBuf::from).unwrap_or_else(|_| std::env::temp_dir());
    let dir = base.join(format!("grimoire-sec-records-cap-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let (over, under) = (45_000usize, 30_000usize);
    let (a, b) = (link_records(over), link_records(under));
    assert!(a.len() > RECORDS_MAX_BYTES, "the first worker's records ({} bytes) are not over the cap", a.len());
    // (64 bytes: more than the page count, the left-out counts and the two closing lines come to)
    assert!(b.len() + 64 < RECORDS_MAX_BYTES, "the second worker's records ({} bytes) are not under the cap", b.len());
    assert!(over < LINKS_MAX && under < LINKS_MAX, "the cap on links would end a worker before the cap on bytes");
    let stand_in = |name: &str, records: &str| {
        let file = dir.join(format!("{name}.records"));
        std::fs::write(&file, records).unwrap();
        let file = file.to_str().expect("the scratch folder's path is text").to_string();
        assert!(!file.contains('\''), "the scratch folder's path holds a quote: {file}");
        let script = dir.join(format!("{name}.sh"));
        std::fs::write(
            &script,
            format!(
                "#!/bin/sh\nPATH=/usr/bin:/bin\nprintf 'B {}\\nS 1\\n' >&4\nIFS= read -r line <&5\nprintf 'P 3\\nK\\n' >&4\nIFS= read -r line <&5\n\
                 cat '{file}' >&4\nprintf 'F 0 0 0 0\\nK\\n' >&4\n",
                build_id()
            ),
        )
        .unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        script
    };
    let (script_a, script_b) = (stand_in("over", &a), stand_in("under", &b));
    // (a slow machine must not turn either outcome into a stall)
    std::env::set_var("GRIMOIRE_PDF_WATCHDOG_MS", "120000");
    // (any file stands for the PDF: the stand-ins never read it)
    let src = std::fs::File::open(&script_a).unwrap();
    let fd = std::os::fd::AsRawFd::as_raw_fd(&src);
    let limits = Limits { as_bytes: 1 << 30, cpu_secs: 60, mapped: false, file_len: 0 };

    // the first worker: ended at the cap
    std::env::set_var("GRIMOIRE_WORK_EXE", &script_a);
    let mut first = Session::start(fd, 100, &limits).expect("the first stand-in shakes hands");
    assert_eq!(first.probe(), Ok(3));
    assert_eq!(first.links(600, 800).err(), Some("too many records"));
    drop(first);

    // the next worker of the job: its own count, from nothing
    std::env::set_var("GRIMOIRE_WORK_EXE", &script_b);
    let mut next = Session::start(fd, 100, &limits).expect("the second stand-in shakes hands");
    assert_eq!(next.probe(), Ok(3));
    let got = next.links(600, 800).expect("a worker under the cap is taken, whatever the one before it sent");
    assert_eq!(got.links.len(), under);
    assert!(got.links.iter().all(|(_, quad)| quad.is_some()), "a link came without its quad");
    assert_eq!((got.links[0].0.index, got.links[under - 1].0.index), (0, under as u32 - 1));
    next.end();
    let _ = std::fs::remove_dir_all(&dir);
}
