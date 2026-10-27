// The exporter (tools/grimoire-export.c): make check runs it once over a scenario's data root
// (grimoire-export --force <root> <out>); this runs it over a data root of its own and through its
// other paths, as the C tool takes them: a notebook in a folder, the incremental skip (a PDF newer
// than the notebook's last change is left alone), --png and --zip, and the usage line with exit
// status 2.
mod testlib;
use grimoire::prelude::*;
use std::process::Command;
use testlib::*;

/// The exporter's exit status, stdout and stderr.
fn exporter(args: &[&str]) -> (i32, String, String) {
    let o = Command::new(env!("CARGO_BIN_EXE_grimoire-export")).args(args).output().expect("grimoire-export runs");
    (o.status.code().unwrap_or(-1), String::from_utf8_lossy(&o.stdout).into_owned(), String::from_utf8_lossy(&o.stderr).into_owned())
}

fn starts_with(path: &str, magic: &[u8]) -> bool {
    std::fs::read(path).map(|b| b.starts_with(magic)).unwrap_or(false)
}

fn exports_a_data_root() {
    let base = format!("{}/export-tool", tl_tmp());
    let _ = std::fs::remove_dir_all(&base);
    let (root, out) = (format!("{base}/root"), format!("{base}/out"));
    check!(mkdir_p(&root) == 0);
    check!(store_init(&root) == 0);
    let (mut a, mut b) = (NotebookInfo::default(), NotebookInfo::default());
    check!(store_create("Lecture notes", "", PAPER_NARROW, &mut a) == 0);
    check!(store_create("Songs", "Music", PAPER_MUSIC, &mut b) == 0);

    let (rc, so, se) = exporter(&["--force", &root, &out]);
    check!(rc == 0);
    check!(se.is_empty());
    check!(so.contains("  Lecture notes (1 page)\n"));
    check!(so.contains("  Music/Songs (1 page)\n"));
    check!(so.ends_with(&format!("grimoire-export: 2 exported, 0 unchanged, 0 failed -> {out}\n")));
    check!(starts_with(&format!("{out}/Lecture notes.pdf"), b"%PDF"));
    check!(starts_with(&format!("{out}/Music/Songs.pdf"), b"%PDF"));

    // Again without --force: both PDFs are newer than their notebooks.
    let (rc, so, _) = exporter(&[&root, &out]);
    check!(rc == 0);
    check!(so == format!("grimoire-export: 0 exported, 2 unchanged, 0 failed -> {out}\n"));

    let (rc, _, _) = exporter(&["--png", "--zip", "--force", &root, &out]);
    check!(rc == 0);
    check!(starts_with(&format!("{out}/Lecture notes/page-001.png"), b"\x89PNG\r\n\x1a\n"));
    check!(starts_with(&format!("{out}/Music/Songs/page-001.png"), b"\x89PNG\r\n\x1a\n"));
    check!(starts_with(&format!("{out}/Lecture notes.zip"), b"PK\x03\x04"));
    check!(starts_with(&format!("{out}/Music/Songs.zip"), b"PK\x03\x04"));
}

fn usage() {
    let line = "usage: grimoire-export [--png] [--zip] [--force] <grimoire-root> <out-dir>\n";
    for args in [&[][..], &["only-a-root"][..], &["-h", "a", "b"][..], &["a", "b", "--help"][..]] {
        let (rc, so, se) = exporter(args);
        check!(rc == 2);
        check!(so.is_empty());
        check!(se == line);
    }
}

#[test]
fn test_export_tool() {
    run!(exports_a_data_root);
    run!(usage);
    done();
}
