// grimoire.log's editor lines (#119), read from the headless runner's stderr (grimoire.log is the app's stderr on
// the Kindle): "Loading page..." with the page's id, its file's size and why, when the notice shows, and a line for
// each undo with what it undid in a few words (never the ink itself), one of each a frame at most.
//
//   cargo test --features sim --test test_editor_log -- --test-threads=1
#![cfg(feature = "sim")]

use grimoire::prelude::*;
use std::path::{Path, PathBuf};
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_grimoire-headless");

/// A fresh scratch directory (the Makefile's test rules set GRIMOIRE_TEST_TMP).
fn scratch(name: &str) -> PathBuf {
    let base = std::env::var("GRIMOIRE_TEST_TMP").map(PathBuf::from).unwrap_or_else(|_| std::env::temp_dir());
    let d = base.join(format!("test_editor_log-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// The editor's lines of the runner's stderr, without their "HH:MM:SS.mmm " stamps.
fn editor_lines(stderr: &[u8]) -> Vec<String> {
    String::from_utf8_lossy(stderr)
        .lines()
        .filter_map(|l| l.get(13..).filter(|rest| rest.starts_with("editor: ")).map(str::to_string))
        .collect()
}

#[test]
fn loading_page_and_undo_are_logged() {
    let d = scratch("log");
    let root = d.join("root");
    assert_eq!(store_init(root.to_str().unwrap()), 0);
    // A notebook whose one page takes seconds to draw on a Kindle (store_page_slow: a file of 256 KB or more) and
    // has no kept drawing: opening it says "Loading page...". Its ink fills the top half; the bottom is free.
    let mut nb = NotebookInfo::default();
    assert_eq!(store_create("Slow", "", PAPER_BLANK, &mut nb), 0);
    let mut page = page_new(nb.page_w, nb.page_h, PAPER_BLANK);
    let mut prm = ToolParams::default();
    tool_params_default(TOOL_BALLPOINT, &mut prm);
    for k in 0..160u32 {
        let mut ink = stroke_new(TOOL_BALLPOINT, &prm, 0x900 + k);
        for i in 0..100 {
            let t = i as f32 / 99.0;
            let y = 40.0 + 8.0 * k as f32 + 6.0 * (t * 17.0).sin();
            stroke_append(&mut ink, &PenSample { x: 60.0 + 880.0 * t, y, pressure: 0.6, t_ms: i * 5, ..Default::default() }, 300.0);
        }
        page_add(&mut page, ink);
    }
    assert_eq!(store_save_page(&nb, 0, &page), 0);
    let pid = nb.page_ids[0];
    let kb = store_page_file_bytes(&nb, pid).unwrap().div_ceil(1024);
    assert!(store_page_slow(&nb, pid) && kb >= 256, "{kb} KB");
    std::fs::write(root.join("settings.ini"), "shortcuts_enabled=1\nmirror_pdf=0\nstatus_mode=1\n").unwrap();
    // The notebook opened; a stroke, undone; Clear page, undone; a lasso move, undone; two strokes, and two undos
    // in one batch (Ctrl+Z twice: one frame), which say one line.
    let script = d.join("log.qs");
    let id = nb.id.as_str();
    std::fs::write(
        &script,
        format!(
            "wait 300\ntap library notebook.{id}\nwait 300\nexpect editor.loading_toasts is 1\n\
             pen line 300 2000 900 2000 20 0.6 0.6\nwait 100\nexpect editor.undo_kind is add\n\
             pentap panel undo\nwait 100\nexpect editor.redo_kind is add\n\
             pentap panel clear\nwait 300\nexpect editor.strokes is 0\npentap panel undo\nwait 300\n\
             expect editor.strokes is 160\nexpect editor.redo_kind is clear\n\
             pen line 300 2000 900 2000 20 0.6 0.6\nwait 100\n\
             tap panel tool-picker\nwait 100\ntap panel picker.select\nwait 100\nexpect editor.select is armed\n\
             pen down 250 1900\npen move 950 1900\npen move 950 2100\npen move 250 2100\npen move 240 2000\n\
             pen up 250 1910\nwait 100\nexpect editor.selected is 1\n\
             pen down 600 2000\npen move 600 2100\npen move 600 2200\npen up 600 2200\nwait 100\n\
             expect editor.undo_kind is lasso\ntap panel select.done\nwait 100\n\
             pentap panel undo\nwait 100\nexpect editor.redo_kind is lasso\n\
             pen line 300 2300 900 2300 20 0.6 0.6\nwait 100\npen line 300 2350 900 2350 20 0.6 0.6\nwait 100\n\
             expect editor.strokes is 163\nkey 44 1\nkey 44 1\nwait 100\nexpect editor.strokes is 161\nquit\n"
        ),
    )
    .unwrap();
    let mut c = Command::new(BIN);
    c.current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap());
    for (k, _) in std::env::vars() {
        if k.starts_with("GRIMOIRE_") || k == "SIM_TOUCH_SLOTS" || k == "TOUCH_TUNE" || k.starts_with("FAKE_UPDATE_") {
            c.env_remove(k);
        }
    }
    c.env("GRIMOIRE_LANG", "en").env("GRIMOIRE_ROOT", &root).env("GRIMOIRE_SCRIPT", &script);
    let o = c.output().unwrap();
    let log = String::from_utf8_lossy(&o.stderr).into_owned();
    assert_eq!(o.status.code(), Some(0), "{log}");
    let want = [
        format!("editor: \"Loading page...\" for page id {pid}: its file is {kb} KB, with no current kept drawing"),
        format!("editor: undo of a stroke on page id {pid}"),
        format!("editor: undo of a page clear (160 strokes) on page id {pid}"),
        format!("editor: undo of a lasso step (1 stroke off the page, 1 on) on page id {pid}"),
        format!("editor: undo of a stroke on page id {pid}"),
    ];
    assert_eq!(editor_lines(&o.stderr), want, "{log}");
    let _ = std::fs::remove_dir_all(d);
}
