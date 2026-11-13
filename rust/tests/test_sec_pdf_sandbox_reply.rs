// test_sec_pdf_sandbox_reply.rs (security review 2026-11, finding "a PDF worker's late `E sandbox`"): a PDF worker
// that answers the probe with `E sandbox <step>` has already parsed the PDF (worker_run opens it with hayro before
// it reads its first command), so the line can come from a worker the PDF took over. It must count as that worker's
// lie, not as a sandbox that refused a step: the latter marks the whole session broken (no PDF or font is read
// any more, and every picture of the session is then decoded in the root writer, picwork::in_fallback) and counts
// no try, so the same file does it again at every start.
//
// The writer is this process (pdfimport::run_slice, as test_pdfimport.rs runs it); the worker is a stand-in that
// gives the real handshake (what a real worker sends before it reads a byte of the PDF) and then the late line.
#![cfg(feature = "sim")]
mod pdffix;
use grimoire::pdfimport::*;
use grimoire::prelude::*;

fn setup(name: &str) -> String {
    for h in ["GRIMOIRE_WORK_FAIL_STEP", "GRIMOIRE_SANDBOX", "GRIMOIRE_IMPORT_STABLE"] {
        std::env::remove_var(h);
    }
    let base = std::env::var("GRIMOIRE_TEST_TMP").map(std::path::PathBuf::from).unwrap_or_else(|_| std::env::temp_dir());
    let storage = base.join(format!("grimoire-sec-pdf-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&storage);
    let root = storage.join("grimoire");
    std::fs::create_dir_all(root.join("import")).unwrap();
    std::env::set_var("GRIMOIRE_STORAGE", &storage);
    assert_eq!(store_init(root.to_str().unwrap()), 0);
    store_set_device(300.0, 600, 800);
    ui_reset();
    root.to_str().unwrap().to_string()
}

#[test]
fn a_sandbox_record_after_the_handshake_is_the_workers_lie() {
    use std::os::unix::fs::PermissionsExt;
    let root = setup("late");
    let w = format!("{root}/worker.sh");
    std::fs::write(
        &w,
        format!(
            "#!/bin/sh\nPATH=/usr/bin:/bin\nprintf 'B {}\\nS 1\\n' >&4\nIFS= read -r line <&5\nprintf 'E sandbox chroot\\n' >&4\nsleep 5\n",
            grimoire::sandbox::build_id()
        ),
    )
    .unwrap();
    std::fs::set_permissions(&w, std::fs::Permissions::from_mode(0o755)).unwrap();
    std::env::set_var("GRIMOIRE_WORK_EXE", &w);
    std::fs::write(format!("{root}/import/Planner.pdf"), pdffix::planner(2)).unwrap();
    let mut id = String::new();
    store_make_id(&mut id);
    let end = run_slice(&id, "grimoire/import/Planner.pdf");
    let broken = sandbox_broken_in_progress();
    eprintln!("  slice end {end:?}; the session told its sandbox is broken: {broken}");
    assert!(!broken, "a worker that had parsed the PDF marked the session's sandbox broken");
    assert_ne!(end, SliceEnd::Sandbox, "a worker that had parsed the PDF was taken for a refusing sandbox (no try counted)");
    assert!(matches!(end, SliceEnd::Failed(..)), "{end:?}");
}
