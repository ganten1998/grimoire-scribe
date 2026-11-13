// test_sec_pdf_drip.rs (security review 2026-11, finding "the page watchdog restarts at every record"): the writer
// gives a worker PAGE_WATCHDOG_MS for its NEXT record, and a page may come in as many bands as it has rows. A worker
// the PDF took over can sleep in futex (on the allow-list, no CPU, so RLIMIT_CPU never ends it) and send a one-row
// band just inside the watchdog: a 2480-row page then holds the writer, and with it the one background slot that
// exports, mirrors and compaction wait on, for about 20 hours, and a resumed import does it again.
//
// The stand-in worker here sends a 10 x 40 page as 40 one-row bands, one every 200 ms, under a 500 ms watchdog: it
// is never silent for a watchdog, yet the page takes 8 s (16 watchdogs). The page the writer asked for is one band
// (`R 1 10 40 1`), so the bands themselves are already outside what it asked.
#![cfg(feature = "sim")]
use grimoire::pdfwork::*;
use grimoire::sandbox::*;

#[test]
fn a_page_that_drips_one_row_bands_is_ended() {
    use std::os::unix::fs::PermissionsExt;
    let base = std::env::var("GRIMOIRE_TEST_TMP").map(std::path::PathBuf::from).unwrap_or_else(|_| std::env::temp_dir());
    let dir = base.join(format!("grimoire-sec-drip-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let script = dir.join("worker.sh");
    std::fs::write(
        &script,
        format!(
            "#!/bin/sh\nPATH=/usr/bin:/bin\nprintf 'B {}\\nS 1\\n' >&4\nIFS= read -r line <&5\nprintf 'P 1\\nK\\n' >&4\nIFS= read -r line <&5\n\
             y=0\nwhile [ $y -lt 40 ]; do\n  sleep 0.2\n  printf 'G 1 %d 10 1 1\\n' $y >&4\n  printf '\\377\\377\\377\\377\\377\\377\\377\\377\\377\\377' >&4\n  y=$((y+1))\ndone\n\
             printf 'D 1\\n' >&4\nsleep 5\n",
            build_id()
        ),
    )
    .unwrap();
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    std::env::set_var("GRIMOIRE_WORK_EXE", &script);
    std::env::set_var("GRIMOIRE_PDF_WATCHDOG_MS", "500");
    // (any file stands for the PDF: the stand-in never reads it)
    let src = std::fs::File::open(&script).unwrap();
    let fd = std::os::fd::AsRawFd::as_raw_fd(&src);
    let limits = Limits { as_bytes: 1 << 30, cpu_secs: 60, mapped: false, file_len: 0 };
    let mut s = Session::start(fd, 100, &limits).expect("the stand-in shakes hands");
    assert_eq!(s.probe(), Ok(1));
    let t0 = std::time::Instant::now();
    let out = s.render(1, 10, 40, 1);
    let ms = t0.elapsed().as_millis();
    let what = match &out {
        PageOutcome::Page(_) => "a page".to_string(),
        o => format!("{o:?}"),
    };
    eprintln!("  the dripped page: {what} after {ms} ms (watchdog 500 ms)");
    assert!(!matches!(out, PageOutcome::Page(_)), "a page of 40 one-row bands, asked for as one band, was taken after {ms} ms");
    assert!(ms < 3_000, "the writer waited {ms} ms on one page");
    let _ = std::fs::remove_dir_all(&dir);
}
