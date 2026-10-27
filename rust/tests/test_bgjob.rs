// test_bgjob.c: the background queue (src/bgjob.h): which job runs next, and which one a full
// queue lets go. The jobs here end at once (no such notebook, file or folder), which is enough:
// bg_poll_kind says the kind of each as it ends, and the result of each the user waits on.
mod testlib;
use grimoire::prelude::*;
use testlib::*;

#[derive(Clone, Copy, Default)]
struct Ended {
    kind: i32,
    result: i32,
}

// Runs the queue dry: the jobs in the order they ended (at most e.len() of them).
fn drain(e: &mut [Ended]) -> usize {
    let cap = e.len();
    let mut n = 0;
    let mut ms = 0;
    while bg_busy() && ms < 30000 {
        let mut k = -1;
        let r = bg_poll_kind(Some(&mut k));
        if k >= 0 {
            if n < cap {
                e[n] = Ended { kind: k, result: r };
            }
            n += 1;
        } else {
            sleep_ms(2);
            ms += 2;
        }
    }
    n
}

const BG_MIRROR_PDF: i32 = BgKind::MirrorPdf as i32;
const BG_COMPACT_PAGES: i32 = BgKind::CompactPages as i32;
const BG_TEMPLATE: i32 = BgKind::Template as i32;
const BG_IMPORT_FOLDER: i32 = BgKind::ImportFolder as i32;

// Something the user waits on first (a template's picture, an export they asked for, in the order
// they came), then the mirror PDFs, then imports, and compaction last: the first job queued starts
// at once whatever it is.
fn what_the_user_waits_on_goes_first() {
    check!(!bg_busy());
    bg_queue(BgKind::CompactPages, Some("nb-a"), None, 300.0, false);
    bg_queue(BgKind::CompactPages, Some("nb-b"), None, 300.0, false);
    bg_queue(BgKind::ImportFolder, Some(""), Some("/nonexistent/import/Book"), 300.0, false);
    bg_mirror_pdf("nb-c", 300.0, "/nonexistent/PDFs", false);
    bg_queue(BgKind::Template, Some("nb-d"), Some("/nonexistent/t.png"), 300.0, false);
    bg_mirror_pdf("nb-e", 300.0, "/nonexistent/PDFs", true);
    let mut e = [Ended::default(); 8];
    check!(drain(&mut e) == 6);
    check!(e[0].kind == BG_COMPACT_PAGES && e[1].kind == BG_TEMPLATE);
    check!(e[2].kind == BG_MIRROR_PDF && e[2].result == -1); // the export asked for (it fails here)
    check!(e[3].kind == BG_MIRROR_PDF && e[3].result == 0); // the mirror, which reports nothing
    check!(e[4].kind == BG_IMPORT_FOLDER && e[5].kind == BG_COMPACT_PAGES);
}

// A full queue lets its oldest compaction go (the pages stay listed for the next one), not the
// oldest job: here every mirror runs and the compaction does not.
fn a_full_queue_lets_a_compaction_go_first() {
    check!(!bg_busy());
    bg_mirror_pdf("running", 300.0, "/nonexistent/PDFs", false);
    for i in 0..8 {
        let id = cformat!("m%d", i);
        bg_mirror_pdf(&id, 300.0, "/nonexistent/PDFs", false);
    }
    bg_queue(BgKind::CompactPages, Some("nb-old"), None, 300.0, false);
    for i in 8..16 {
        // the 16th queued job fills the queue, the 17th finds it full
        let id = cformat!("m%d", i);
        bg_mirror_pdf(&id, 300.0, "/nonexistent/PDFs", false);
    }
    let mut e = [Ended::default(); 20];
    let n = drain(&mut e);
    check!(n == 17);
    let mut compactions = 0;
    for x in e.iter().take(n) {
        compactions += (x.kind == BG_COMPACT_PAGES) as i32;
    }
    check!(compactions == 0);
}

#[test]
fn test_bgjob() {
    let root = format!("{}/bgjob-{}", tl_tmp(), std::process::id());
    let _ = std::fs::remove_dir_all(&root);
    check!(store_init(&root) == 0);
    run!(what_the_user_waits_on_goes_first);
    run!(a_full_queue_lets_a_compaction_go_first);
    let _ = std::fs::remove_dir_all(&root);
    done();
}
