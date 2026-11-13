//! A 2000-page notebook's mirror takes no more than a 64 MB margin of memory (lane L9). Its own test
//! binary, so the process's peak is the mirror's: the PDF here is about 200 MB, which one buffer would
//! hold whole. On Linux the child also carries the real RLIMIT_AS cap the mirror gets.

use grimoire::prelude::*;

const PAGES: usize = 2000;

fn peak_bytes() -> u64 {
    let mut ru: libc::rusage = unsafe { std::mem::zeroed() };
    // SAFETY: getrusage writes one rusage into ru.
    unsafe { libc::getrusage(libc::RUSAGE_SELF, &mut ru) };
    if cfg!(target_vendor = "apple") {
        ru.ru_maxrss as u64
    } else {
        ru.ru_maxrss as u64 * 1024
    }
}

#[test]
fn a_two_thousand_page_mirror_completes_within_a_64_mb_margin() {
    let root = std::path::PathBuf::from(std::env::var("GRIMOIRE_TEST_TMP").unwrap_or_else(|_| std::env::temp_dir().to_string_lossy().into_owned()))
        .join(format!("pdfbig-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    assert_eq!(store_init(root.to_str().unwrap()), 0);
    store_set_device(300.0, 300, 300);
    let mut nb = NotebookInfo::default();
    assert_eq!(store_create("Big", "", PAPER_NARROW_NM, &mut nb), 0);
    // one noise picture every page has as its paper: a stream of about 90 KB a page
    let mut bg = String::new();
    bgimage_dir(nb.id.as_str(), &mut bg, 700);
    assert_eq!(mkdir_p(&bg), 0);
    let mut seed = 12345u32;
    let px: Vec<u8> = (0..300 * 300)
        .map(|_| {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            (seed >> 24) as u8
        })
        .collect();
    let mut f = String::new();
    bgimage_file(&bg, 0x20, &mut f, 800);
    assert_eq!(bgimage_write_png(&f, &BgPicture { px, w: 300, h: 300, ch: 1 }), 0);
    let paper = paper_image(PAPER_ROLE_DOCUMENT, 0x20);
    let mut undo = None;
    assert_eq!(store_add_pages(&mut nb, 1, PAGES as i32 - 1, paper, &mut undo), 0);
    let nb = {
        let mut b = NotebookInfo::default();
        assert_eq!(store_load_info(nb.id.as_str(), &mut b), 0);
        b
    };
    assert_eq!(nb.npages() as usize, PAGES);
    for i in 0..PAGES {
        assert_eq!(store_save_page(&nb, i as i32, &page_new(300, 300, paper)), 0);
    }
    let dir = format!("{}/PDFs", root.display());
    // the mirror in a forked child, as bgjob runs it: its peak above where it started
    // SAFETY: fork; the child only exports and _exits.
    let pid = unsafe { libc::fork() };
    if pid == 0 {
        #[cfg(target_os = "linux")]
        child_caps_apply(64 << 20);
        let start = peak_bytes();
        let rc = export_mirror_pdf(&nb, 300.0, &dir);
        let grew = peak_bytes().saturating_sub(start);
        // SAFETY: _exit ends the child at once.
        unsafe { libc::_exit(if rc != 0 { 2 } else if grew > 64 << 20 { 3 } else { 0 }) };
    }
    let mut st = 0;
    // SAFETY: waitpid on the child just forked.
    unsafe { libc::waitpid(pid, &mut st, 0) };
    assert!(libc::WIFEXITED(st), "the mirror child ended");
    assert_eq!(libc::WEXITSTATUS(st), 0, "the mirror completed (2: failed, 3: grew past 64 MB)");
    let mut path = String::new();
    export_mirror_path(&nb, &dir, &mut path, 1000);
    let len = std::fs::metadata(&path).unwrap().len();
    assert!(len > 100 << 20, "the PDF is far bigger than the margin: {len}");
    // and the second one draws nothing
    let before = pdf_pages_rendered();
    assert_eq!(export_mirror_pdf(&nb, 300.0, &dir), 0);
    assert_eq!(pdf_pages_rendered() - before, 0);
    let _ = std::fs::remove_dir_all(&root);
}
