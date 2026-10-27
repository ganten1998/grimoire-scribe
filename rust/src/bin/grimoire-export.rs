//! tools/grimoire-export.c: render a Grimoire data root (e.g. rsynced off the Kindle) into PDFs / PNGs
//! on a computer, with exactly the same renderer the Kindle uses.
//!
//!   grimoire-export [--png] [--zip] [--force] <grimoire-root> <out-dir>
//!
//! Writes <out>/<folder>/<name>.pdf (and <name>/page-NNN.png with --png, <name>.zip with --zip).
//! Incremental: a notebook is skipped when its PDF is newer than its last modification.
use grimoire::prelude::*;
use std::io::Write;
use std::os::unix::fs::MetadataExt;

/// fprintf(stderr, ...): the C's stderr is unbuffered, so is Rust's.
fn eprint_c(text: &str) {
    let _ = std::io::stderr().write_all(text.as_bytes());
}

/// printf(...)
fn print_c(text: &str) {
    let _ = std::io::stdout().write_all(text.as_bytes());
}

fn write_out(path: &str, b: &Buf) -> i32 {
    if write_file_atomic(path, &b.data) != 0 {
        eprint_c(&cformat!("grimoire-export: cannot write %s\n", path));
        return -1;
    }
    0
}

fn main() {
    // (args_os: the C takes any bytes as an argument, where std::env::args would panic on one that
    // is not UTF-8)
    let args = std::env::args_os().map(|a| a.to_string_lossy().into_owned()).collect();
    std::process::exit(run(args));
}

fn run(args: Vec<String>) -> i32 {
    let (mut png, mut zip, mut force) = (false, false, false);
    let (mut root, mut out): (Option<&str>, Option<&str>) = (None, None);
    let mut argc = args.len();
    let mut i = 1;
    while i < argc {
        let a = args[i].as_str();
        if a == "--png" {
            png = true;
        } else if a == "--zip" {
            zip = true;
        } else if a == "--force" {
            force = true;
        } else if a == "-h" || a == "--help" {
            root = None;
            out = None;
            argc = 0;
        } else if root.is_none() {
            root = Some(a);
        } else if out.is_none() {
            out = Some(a);
        }
        i += 1;
    }
    let (Some(root), Some(out)) = (root, out) else {
        eprint_c("usage: grimoire-export [--png] [--zip] [--force] <grimoire-root> <out-dir>\n");
        return 2;
    };
    if store_init(root) != 0 || mkdir_p(out) != 0 {
        return 1;
    }
    let dpi: f32 = 300.0;
    let mut nbs: Vec<NotebookInfo> = Vec::new();
    if store_list(&mut nbs) != 0 {
        eprint_c(&cformat!("grimoire-export: no notebooks under %s/notebooks\n", root));
        return 1;
    }
    let (mut done, mut skipped, mut failed) = (0, 0, 0);
    for nb in &nbs {
        let (mut sf, mut sn, mut dir, mut path) = (String::new(), String::new(), String::new(), String::new());
        export_safe_name(nb.name.as_str(), &mut sn, STORE_NAME_LEN);
        if !nb.folder.is_empty() {
            export_safe_name(nb.folder.as_str(), &mut sf, STORE_NAME_LEN);
            csnprintf!(&mut dir, 1200, "%s/%s", out, &sf);
        } else {
            csnprintf!(&mut dir, 1200, "%s", out);
        }
        mkdir_p(&dir);
        csnprintf!(&mut path, 1400, "%s/%s.pdf", &dir, &sn);
        if !force {
            if let Ok(st) = std::fs::metadata(&path) {
                if st.mtime() >= nb.modified {
                    skipped += 1;
                    continue;
                }
            }
        }
        let mut b = Buf::default();
        let mut rc = export_notebook_pdf(nb, dpi, &mut b);
        if rc == 0 {
            rc = write_out(&path, &b);
        }
        buf_free(&mut b);
        if rc == 0 && png {
            let mut pdir = String::new();
            csnprintf!(&mut pdir, 1300, "%s/%s", &dir, &sn);
            mkdir_p(&pdir);
            let mut p = 0;
            while p < nb.npages() && rc == 0 {
                let mut pb = Buf::default();
                rc = export_page_png(nb, p, dpi, &mut pb);
                csnprintf!(&mut path, 1400, "%s/page-%03d.png", &pdir, p + 1);
                if rc == 0 {
                    rc = write_out(&path, &pb);
                }
                buf_free(&mut pb);
                p += 1;
            }
        }
        if rc == 0 && zip {
            let mut zb = Buf::default();
            {
                let mut z = zip_begin(&mut zb);
                rc = export_notebook_zip(nb, dpi, &mut z, "");
                zip_end(&mut z);
            }
            csnprintf!(&mut path, 1400, "%s/%s.zip", &dir, &sn);
            if rc == 0 {
                rc = write_out(&path, &zb);
            }
            buf_free(&mut zb);
        }
        if rc == 0 {
            let has_folder = !nb.folder.is_empty();
            print_c(&cformat!(
                "  %s%s%s (%d page%s)\n",
                if has_folder { nb.folder.as_str() } else { "" },
                if has_folder { "/" } else { "" },
                &nb.name,
                nb.npages(),
                if nb.npages() == 1 { "" } else { "s" }
            ));
            done += 1;
        } else {
            eprint_c(&cformat!("grimoire-export: failed: %s\n", &nb.name));
            failed += 1;
        }
    }
    print_c(&cformat!("grimoire-export: %d exported, %d unchanged, %d failed -> %s\n", done, skipped, failed, out));
    if failed != 0 {
        1
    } else {
        0
    }
}
