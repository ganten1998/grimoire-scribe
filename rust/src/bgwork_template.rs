//! bgwork_template.c: templates and imports (templates.h), and the background work behind them:
//! BG_TEMPLATE (a template's picture made for a notebook, or the previews of the templates folder)
//! and BG_IMPORT_FOLDER (a folder of page pictures made into a notebook), run in a forked child by
//! bgjob.c.
//!
//! grimoire.log goes into public bug reports: the names of the owner's files and folders are never
//! logged here, only notebook ids and counts.

use crate::prelude::*;
use std::cell::RefCell;
use std::io::Read;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

/// a template file bigger than this is not taken
const TEMPLATE_MAX_BYTES: usize = 16 << 20;
/// the most read on the UI thread for a template's key
const TEMPLATE_HASH_NOW: usize = 4 << 20;
/// files of the templates folder looked at
const MAX_TEMPLATES: usize = 500;
/// import folders queued at one start (the job queue holds 16)
const IMPORT_QUEUE_MAX: usize = 8;
/// in an import folder: the notebook it becomes ("done <id>" once made)
const IMPORT_MARK: &str = ".grimoire-import";
/// starts that try a folder whose pictures stay the same
const IMPORT_TRIES: i32 = 3;
/// what a background job's path holds (bgjob.c): a longer one is cut
const JOB_PATH_MAX: usize = TEMPLATE_PATH_MAX;
/// a folder's path (< JOB_PATH_MAX) and a name in it (vfat: up to 765 bytes)
const PATH_BUF: usize = 1600;

// What import_folder came to: done, failed in a way that may pass (memory, a full disk), or
// refused (a page is no picture Grimoire takes: the same pictures fail the same way).
const IMPORT_DONE: i32 = 0;
const IMPORT_RETRY: i32 = -1;
const IMPORT_REFUSED: i32 = -2;

// The one piece of the C's file-level state: the imports this run queued.
#[derive(Default)]
struct State {
    /// imports queued by this run, oldest first (g_pending, g_npending; at most IMPORT_QUEUE_MAX)
    pending: Vec<FixedStr<STORE_ID_LEN>>,
}

thread_local! {
    static S: RefCell<State> = RefCell::new(State::default());
}

fn with<R>(f: impl FnOnce(&mut State) -> R) -> R {
    S.with(|s| f(&mut s.borrow_mut()))
}

fn root_path(sub: &str, buf: &mut String, cap: usize) {
    csnprintf!(buf, cap, "%s/%s", &store_root(), sub);
}

// ---- names ----

fn is_digit(c: u8) -> bool {
    c.is_ascii_digit()
}

// The C reads NUL-terminated strings: a name ends at its first NUL, and reading past the end finds
// the NUL.
fn cbytes(s: &str) -> &[u8] {
    let b = s.as_bytes();
    &b[..b.iter().position(|&c| c == 0).unwrap_or(b.len())]
}

/// Natural order of names: runs of digits compare by their value, other characters without regard
/// to case; equal so, the bytes decide.
pub fn natural_cmp(a: &str, b: &str) -> i32 {
    let (a, b) = (cbytes(a), cbytes(b));
    let at = |s: &[u8], i: usize| s.get(i).copied().unwrap_or(0);
    let (mut x, mut y) = (0usize, 0usize);
    while at(a, x) != 0 && at(b, y) != 0 {
        if is_digit(at(a, x)) && is_digit(at(b, y)) {
            while at(a, x) == b'0' {
                x += 1;
            }
            while at(b, y) == b'0' {
                y += 1;
            }
            let (mut xe, mut ye) = (x, y);
            while is_digit(at(a, xe)) {
                xe += 1;
            }
            while is_digit(at(b, ye)) {
                ye += 1;
            }
            if xe - x != ye - y {
                return if xe - x < ye - y { -1 } else { 1 }; // more digits: a bigger number
            }
            let c = a[x..xe].cmp(&b[y..ye]);
            if c != std::cmp::Ordering::Equal {
                return if c == std::cmp::Ordering::Less { -1 } else { 1 };
            }
            x = xe;
            y = ye;
            continue;
        }
        let (cx, cy) = (at(a, x).to_ascii_lowercase(), at(b, y).to_ascii_lowercase());
        if cx != cy {
            return if cx < cy { -1 } else { 1 };
        }
        x += 1;
        y += 1;
    }
    if at(a, x) != 0 || at(b, y) != 0 {
        return if at(a, x) != 0 { 1 } else { -1 }; // the one that ran out first goes first
    }
    match a.cmp(b) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }
}

// A picture file by its name: .png, .jpg, .jpeg or .pgm in any case, and never a dot-file.
fn picture_name(n: &str) -> bool {
    let dot = n.rfind('.');
    if n.starts_with('.') || dot.is_none() {
        return false;
    }
    let ext = &n[dot.unwrap_or(0)..];
    ext.eq_ignore_ascii_case(".png")
        || ext.eq_ignore_ascii_case(".jpg")
        || ext.eq_ignore_ascii_case(".jpeg")
        || ext.eq_ignore_ascii_case(".pgm")
}

fn is_dir(path: &str) -> bool {
    Path::new(path).is_dir()
}

fn exists(path: &str) -> bool {
    Path::new(path).exists()
}

// The entries of dir that are picture files (pictures) or folders (!pictures), in natural order,
// at most max. 0, or -1 without the folder (errno says why).
// (A name that is not UTF-8 is left out: the job queue and the screens take paths as text, so such
// a file could not be used anyway; a Kindle's storage writes UTF-8 names.)
fn list_names(dir: &str, pictures: bool, max: usize, out: &mut Vec<String>) -> i32 {
    out.clear();
    let Ok(rd) = std::fs::read_dir(dir) else { return -1 };
    for e in rd {
        if out.len() >= max {
            break;
        }
        let Ok(e) = e else { break };
        let name = e.file_name();
        let Some(name) = name.to_str() else { continue };
        if name.starts_with('.') {
            continue;
        }
        if pictures && !picture_name(name) {
            continue;
        }
        let mut path = String::new();
        csnprintf!(&mut path, PATH_BUF, "%s/%s", dir, name);
        if is_dir(&path) == pictures {
            continue;
        }
        out.push(name.to_string());
    }
    if !out.is_empty() {
        out.sort_by(|a, b| natural_cmp(a, b).cmp(&0));
    }
    0
}

// A name cut to fit its field at the start of a character, never inside one.
fn copy_name<const N: usize>(dst: &mut FixedStr<N>, src: &str) {
    dst.set_cut(src);
}

// A whole picture file of at most max bytes, or None (missing, unreadable, or bigger: errno EFBIG).
fn read_source(path: &str, max: usize) -> Option<Vec<u8>> {
    bgimage_read(path, max)
}

// The first `max` bytes of a file (all of it if it is shorter), or None.
fn read_head(path: &str, max: usize) -> Option<Vec<u8>> {
    let f = std::fs::File::open(path).ok()?;
    let mut buf: Vec<u8> = Vec::new();
    buf.try_reserve_exact(max).ok()?;
    // (fread gives what it read before a failure)
    let _ = f.take(max as u64).read_to_end(&mut buf);
    Some(buf)
}

// Whether the file at path, of at most max bytes, is a picture Grimoire takes (bgimage_check), from
// its header: in its first 64 KB, or else (metadata before it: an ICC profile, a JPEG's
// thumbnail) anywhere in the file, read whole for that. 0 with its size, or -1 and why.
fn check_file(path: &str, max: usize, w: &mut i32, h: &mut i32, why: &mut &'static str) -> i32 {
    *why = "the file does not read";
    let Ok(st) = std::fs::metadata(path) else { return -1 };
    if !st.is_file() {
        return -1;
    }
    if st.len() > max as u64 {
        *why = "too big a file";
        return -1;
    }
    let want: usize = 64 << 10;
    let head = read_head(path, want);
    let more = head.as_ref().is_some_and(|h| h.len() == want);
    let mut rc = match &head {
        Some(hd) => bgimage_check(hd, st.len() as usize, w, h, why),
        None => -1,
    };
    if rc != 0 && more {
        if let Some(all) = read_source(path, max) {
            rc = bgimage_check(&all, all.len(), w, h, why);
        }
    }
    rc
}

fn cpath(path: &str) -> Option<std::ffi::CString> {
    std::ffi::CString::new(path).ok()
}

// A small file written whole (write, fsync, rename), its path never logged: it is in the owner's
// own folder, whose name is theirs.
fn write_quiet(path: &str, text: &str) -> i32 {
    let mut tmp = String::new();
    csnprintf!(&mut tmp, PATH_BUF + 64, "%s.tmp", path);
    let Some(ctmp) = cpath(&tmp) else { return -1 };
    // SAFETY: open reads the NUL-terminated path we own; the descriptor it returns is ours alone.
    let fd = unsafe { libc::open(ctmp.as_ptr(), libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC, 0o644 as libc::c_uint) };
    if fd < 0 {
        return -1;
    }
    let len = text.len();
    // SAFETY: write reads len bytes from text, which holds that many; fd is the open descriptor.
    let wrote = unsafe { libc::write(fd, text.as_ptr() as *const libc::c_void, len) };
    // SAFETY: fsync only takes the open descriptor.
    let mut ok = wrote == len as isize && unsafe { libc::fsync(fd) } == 0;
    // SAFETY: close takes the descriptor this function opened and does not use again.
    ok = unsafe { libc::close(fd) } == 0 && ok;
    if !ok || std::fs::rename(&tmp, path).is_err() {
        let _ = std::fs::remove_file(&tmp);
        return -1;
    }
    0
}

// Makes what a rename did to the entries of dir durable, as write_file_atomic does after its own
// (best effort).
fn sync_dir(dir: &str) {
    let Some(c) = cpath(dir) else { return };
    // SAFETY: open reads the NUL-terminated path we own.
    let fd = unsafe { libc::open(c.as_ptr(), libc::O_RDONLY) };
    if fd < 0 {
        return;
    }
    // SAFETY: fsync and close take the descriptor opened above, used nowhere else.
    unsafe {
        libc::fsync(fd);
        libc::close(fd);
    }
}

// ---- memory, in the children ----
//
// A ceiling on a child's address space: what it holds now, plus what one decode may take
// (BG_DECODE_BUDGET) and room for the page picture it makes. Past it an allocation fails, and the
// job with it (an import keeps its folder), before the Kindle runs out of memory and its kernel
// picks a process to kill. The decode budget keeps stb_image within it already: this catches the
// rest. Linux only. (The C also stays out of the sanitizers' builds, whose shadow memory is address
// space too; a Rust build has none.)

struct MemCap {
    set: bool,
    was: libc::rlimit,
}

#[cfg(target_os = "linux")]
fn cap_memory() -> MemCap {
    let mut m = MemCap { set: false, was: libc::rlimit { rlim_cur: 0, rlim_max: 0 } };
    cap_memory_linux(&mut m);
    m
}

#[cfg(not(target_os = "linux"))]
fn cap_memory() -> MemCap {
    MemCap { set: false, was: libc::rlimit { rlim_cur: 0, rlim_max: 0 } }
}

#[cfg(target_os = "linux")]
fn cap_memory_linux(m: &mut MemCap) {
    // (the first number of /proc/self/statm: the pages of address space the process holds)
    let Ok(statm) = std::fs::read("/proc/self/statm") else { return };
    let digits: Vec<u8> = statm.iter().skip_while(|c| c.is_ascii_whitespace()).take_while(|c| c.is_ascii_digit()).copied().collect();
    let Some(pages) = std::str::from_utf8(&digits).ok().and_then(|d| d.parse::<u64>().ok()) else { return };
    // SAFETY: sysconf only reads a configuration value.
    let psz = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
    // SAFETY: getrlimit writes one rlimit into m.was, which we own.
    if psz <= 0 || unsafe { libc::getrlimit(libc::RLIMIT_AS, &mut m.was) } != 0 {
        return;
    }
    let cap: libc::rlim_t = (pages as libc::rlim_t) * (psz as libc::rlim_t) + (BG_DECODE_BUDGET as libc::rlim_t) + ((128 as libc::rlim_t) << 20);
    if m.was.rlim_cur != libc::RLIM_INFINITY && m.was.rlim_cur <= cap {
        return; // as low already
    }
    if m.was.rlim_max != libc::RLIM_INFINITY && cap > m.was.rlim_max {
        return;
    }
    let r = libc::rlimit { rlim_cur: cap, rlim_max: m.was.rlim_max };
    // SAFETY: setrlimit reads one rlimit from r.
    m.set = unsafe { libc::setrlimit(libc::RLIMIT_AS, &r) } == 0;
}

fn uncap_memory(m: &MemCap) {
    if m.set {
        // SAFETY: setrlimit reads one rlimit from m.was, which getrlimit filled in.
        unsafe { libc::setrlimit(libc::RLIMIT_AS, &m.was) };
    }
}

// ---- the folders ----

// A folder's README.txt, written once when Grimoire makes the folder (in the language in use then)
// for the owner to read on a computer: a title and paragraphs, a translated string each.
fn write_readme(dir: &str, parts: &[&str], what: &str) {
    let mut path = String::new();
    csnprintf!(&mut path, 700, "%s/README.txt", dir);
    if exists(&path) {
        return;
    }
    let n = parts.len();
    let mut text = String::new();
    for (i, part) in parts.iter().enumerate() {
        text.push_str(part);
        text.push_str(if i + 1 < n { "\n\n" } else { "\n" });
    }
    if text.len() >= 4096 || write_quiet(&path, &text) != 0 {
        qlog!("%s: could not write %s/README.txt", what, what);
    }
}

/// At startup: grimoire/templates/, grimoire/photos/ and grimoire/import/, each with a README.txt,
/// when missing.
pub fn templates_init() {
    let mut dir = String::new();
    root_path("templates", &mut dir, 600);
    if mkdir_p(&dir) == 0 {
        // TRANSLATORS: grimoire/templates/README.txt, a plain text file Grimoire writes once (in
        // the language in use then) for the owner to read on a computer: its title, then its two
        // paragraphs. No room limit.
        let parts = [
            tr("Templates for Grimoire"),
            tr("Copy pictures here to write on them as paper: PNG or JPG files, upright, in the shape of a Kindle \
Scribe page (3:4). 1860 x 2480 pixels is best. Light lines on white work best on e-ink: dark areas \
hide your writing."),
            tr("Copy them in over USB while Grimoire is closed. A notebook keeps its own copy of each template it \
uses, so changing or deleting a file here never changes pages already written on it. Grimoire keeps \
its previews of these files in the .cache folder."),
        ];
        write_readme(&dir, &parts, "templates");
    }
    root_path("photos", &mut dir, 600);
    if mkdir_p(&dir) == 0 {
        // TRANSLATORS: grimoire/photos/README.txt, written like the templates one: its title, then its
        // two paragraphs. "Paper", "Photos..." and "Pages" name Grimoire's own sheets and buttons:
        // as they are translated there.
        let parts = [
            tr("Photos for Grimoire"),
            tr("Copy photos here to write on them: JPG or PNG files, as your phone or camera saved them. In a \
notebook, open Paper and pick Photos...: the photo becomes the open page's paper, fitted to the \
page and turned upright. For a page of its own, add a new page in Pages first."),
            tr("Copy them in over USB while Grimoire is closed. A notebook keeps its own copy of each photo it \
uses, so changing or deleting a file here never changes pages already written on it. Grimoire keeps \
its previews of these files in the .cache folder."),
        ];
        write_readme(&dir, &parts, "photos");
    }
    root_path("import", &mut dir, 600);
    if mkdir_p(&dir) == 0 {
        // TRANSLATORS: grimoire/import/README.txt, written like the templates one: its title, then
        // its two paragraphs. "done" is the name of a folder: keep it as it is.
        let parts = [
            tr("Importing pages into Grimoire"),
            tr("To write on pages you already have, such as a PDF saved as pictures, put a folder of PNG or JPG \
files here, one picture a page, and start Grimoire. The folder's name becomes the notebook's name, \
and the pages follow the file names in order (page-2 comes before page-10). Gray pictures import \
faster and take less room than colour ones."),
            tr("Grimoire imports each folder in the background and then moves it into the done folder. Copy folders \
in over USB while Grimoire is closed."),
        ];
        write_readme(&dir, &parts, "import");
    }
}

// ---- template previews ----

/// previews are 1/6 of the page, as the paper sheets' tiles
const THUMB_DIV: i32 = 6;

// The preview of a template file: .cache/<hash of its name, size and time>.pgm (with .key, the
// hash of its bytes, and .bad for one that does not read), so a file replaced by another gets a
// preview of its own.
fn preview_path(dir: &str, name: &str, ext: &str, buf: &mut String, cap: usize) -> bool {
    let mut path = String::new();
    csnprintf!(&mut path, PATH_BUF, "%s/%s", dir, name);
    let Ok(st) = std::fs::metadata(&path) else { return false };
    let mut h: u64 = 14695981039346656037; // FNV-1a
    let mut tail = String::new();
    csnprintf!(&mut tail, 64, "/%lld/%lld", st.len() as i64, st.mtime());
    for s in name.bytes().chain(tail.bytes()) {
        h = (h ^ s as u64).wrapping_mul(1099511628211);
    }
    csnprintf!(buf, cap, "%s/.cache/%016llx.%s", dir, h, ext);
    true
}

// The same for a template given by its path.
fn preview_of(path: &str, ext: &str, buf: &mut String, cap: usize) -> bool {
    // (a path the 600-byte copy would cut is none)
    if path.len() >= TEMPLATE_PATH_MAX {
        return false;
    }
    let Some(slash) = path.rfind('/') else { return false };
    preview_path(&path[..slash], &path[slash + 1..], ext, buf, cap)
}

/// Whether path names a file in grimoire/photos/ (a background job tells a photo by it).
pub fn photo_path(path: &str) -> bool {
    let mut dir = String::new();
    root_path("photos", &mut dir, 600);
    let n = dir.len();
    let p = cbytes(path);
    p.starts_with(dir.as_bytes()) && p.get(n) == Some(&b'/') && p.get(n + 1).is_some() && !p[n + 1..].contains(&b'/')
}

// The pictures of folder `which` (templates or photos) in the data root, for template_scan.
fn scan_folder(which: &str, out: &mut Vec<Template>, dpi: f32) -> i32 {
    out.clear();
    let mut dir = String::new();
    root_path(which, &mut dir, 600);
    let mut names = Vec::new();
    if list_names(&dir, true, MAX_TEMPLATES, &mut names) != 0 {
        return -1;
    }
    let mut t: Vec<Template> = Vec::new();
    let mut missing = false;
    let mut toolong = 0;
    for name in &names {
        if dir.len() + 1 + name.len() >= TEMPLATE_PATH_MAX {
            // (it would be cut on its way to a job)
            toolong += 1;
            continue;
        }
        let mut tp = Template::default();
        let mut path = String::new();
        csnprintf!(&mut path, TEMPLATE_PATH_MAX, "%s/%s", &dir, name);
        tp.path.set(&path);
        let mut base = String::new();
        strlcpy_q(&mut base, name, STORE_NAME_LEN * 2);
        if let Some(dot) = base.rfind('.') {
            base.truncate(dot);
        }
        copy_name(&mut tp.name, &base);
        let (mut pv, mut kp) = (String::new(), String::new());
        'flags: {
            if !preview_path(&dir, name, "pgm", &mut pv, PATH_BUF) || !preview_path(&dir, name, "key", &mut kp, PATH_BUF) {
                break 'flags;
            }
            if exists(&pv) {
                tp.ready = true;
                if !exists(&kp) {
                    missing = true;
                }
                break 'flags;
            }
            preview_path(&dir, name, "bad", &mut pv, PATH_BUF);
            if exists(&pv) {
                tp.bad = true;
            } else {
                missing = true;
            }
        }
        t.push(tp);
    }
    if toolong != 0 {
        qlog!("%s: %d file(s) with too long a name left out", which, toolong);
    }
    if missing {
        bg_queue(BgKind::Template, Some(""), Some(&dir), dpi, false);
    }
    *out = t;
    0
}

/// The templates in grimoire/templates/, in the natural order of their names (page-2 before
/// page-10), and which have their previews made (kept in grimoire/templates/.cache/); none is read
/// here: a sheet reads those it shows with template_thumb, a page of tiles at a time. Previews still
/// missing are made in the background (one BG_TEMPLATE job with no notebook): scan again when
/// bg_poll_kind reports a BG_TEMPLATE done. A file whose path is longer than TEMPLATE_PATH_MAX is
/// left out. (template_list_free empties a list.) 0, or -1 without the folder.
pub fn template_scan(out: &mut Vec<Template>, dpi: f32) -> i32 {
    scan_folder("templates", out, dpi)
}

// ---- photos ----
//
// grimoire/photos/*.png|jpg|jpeg|pgm: photos the owner copies onto the Kindle over USB, each to be a
// page's paper: the page's own picture (a document page, Image papers: a new page after it gets the
// notebook's paper), fitted to the page and turned upright by its EXIF orientation, previews too.
// The same scan, previews and sheet as templates.

/// As template_scan, for grimoire/photos/.
pub fn photos_scan(out: &mut Vec<Template>, dpi: f32) -> i32 {
    scan_folder("photos", out, dpi)
}

/// Template t's preview (read once, then kept in t), or NULL while it is not made (or t is bad).
pub fn template_thumb(t: &mut Template) -> Option<&Canvas> {
    if !t.thumb.px.is_empty() {
        return Some(&t.thumb);
    }
    if !t.ready {
        return None;
    }
    let mut pv = String::new();
    if !preview_of(t.path.as_str(), "pgm", &mut pv, PATH_BUF) || store_read_pgm(&pv, &mut t.thumb) != 0 {
        t.ready = false; // gone since the scan (the file changed): a scan makes it again
        return None;
    }
    Some(&t.thumb)
}

pub fn template_list_free(t: &mut Vec<Template>) {
    for tp in t.iter_mut() {
        canvas_free(&mut tp.thumb);
    }
    t.clear();
}

/// What template_scan found of each of the n templates brought up to date (a preview made since,
/// a file found bad), from the cache's files alone, queueing nothing: a sheet fills in its tiles
/// with it while the previews job runs. How many of them changed.
pub fn template_refresh(t: &mut [Template]) -> i32 {
    let mut changed = 0;
    for tp in t.iter_mut() {
        let (mut pv, mut other) = (String::new(), String::new());
        if tp.ready || tp.bad || !preview_of(tp.path.as_str(), "pgm", &mut pv, PATH_BUF) {
            continue;
        }
        let ext = pv.len() - 3; // the three letters after the dot: the other files' too
        if exists(&pv) {
            tp.ready = true; // (as template_scan has it: the preview's key is written first)
            changed += 1;
            continue;
        }
        csnprintf!(&mut other, PATH_BUF, "%.*sbad", ext as i32, &pv);
        if exists(&other) {
            tp.bad = true;
            changed += 1;
        }
    }
    changed
}

// In the child: the previews the templates folder (or the photos folder) lacks, made at the page size
// of this Kindle, with the hash of each file's bytes beside its preview (a template's key comes from
// it, and so the UI never reads a big file to know it); previews of files no longer there are
// removed. A photo's preview is turned upright, as its picture will be.
fn make_previews(dir: &str) -> i32 {
    let mut photos = String::new();
    root_path("photos", &mut photos, 600);
    let photo = dir == photos;
    let mut cache = String::new();
    csnprintf!(&mut cache, 700, "%s/.cache", dir);
    if mkdir_p(&cache) != 0 {
        return -1;
    }
    let mut names = Vec::new();
    if list_names(dir, true, MAX_TEMPLATES, &mut names) != 0 {
        return -1;
    }
    let nn = names.len();
    let (mut pw, mut ph) = (0, 0);
    let (mut made, mut bad, mut later) = (0, 0, 0);
    store_page_size(&mut pw, &mut ph);
    // (the hash part of each file's preview name, "" for a file that has none)
    let mut keep: Vec<String> = vec![String::new(); nn];
    for i in 0..nn {
        let (mut pv, mut kp, mut bp, mut path) = (String::new(), String::new(), String::new(), String::new());
        if !preview_path(dir, &names[i], "pgm", &mut pv, PATH_BUF) {
            continue;
        }
        preview_path(dir, &names[i], "key", &mut kp, PATH_BUF);
        preview_path(dir, &names[i], "bad", &mut bp, PATH_BUF);
        let file = pv.rfind('/').map_or(&pv[..], |s| &pv[s + 1..]);
        strlcpy_q(&mut keep[i], file, 32);
        keep[i].truncate(16); // the hash, without the extension
        if (exists(&pv) && exists(&kp)) || exists(&bp) {
            continue;
        }
        csnprintf!(&mut path, PATH_BUF, "%s/%s", dir, &names[i]);
        let data = read_source(&path, TEMPLATE_MAX_BYTES);
        // A file too big is no template, and neither is one whose bytes are no picture Grimoire
        // takes. One that does not read now (no memory for it, a read error) may read at a later
        // scan: it is not marked bad.
        let too_big = data.is_none() && errno() == libc::EFBIG;
        let mut why: &'static str = if data.is_some() {
            "out of memory"
        } else if too_big {
            "a file over 16 MB"
        } else {
            "the file does not read"
        };
        let mut src = BgPicture::default();
        let mut fit = BgPicture::default();
        let mut full = Canvas::default();
        let mut small = Canvas::default();
        let bytes: &[u8] = data.as_deref().unwrap_or(&[]);
        let mut dec = if data.is_some() {
            bgimage_decode(bytes, &mut src, &mut why)
        } else if too_big {
            -1
        } else {
            -2
        };
        if dec == 0 && photo && bgimage_orient(&mut src, bgimage_exif_orientation(bytes)) != 0 {
            dec = -2;
        }
        let ok = dec == 0
            && bgimage_normalize(&src, pw, ph, &mut fit) == 0
            && bgimage_luma(&fit, &mut full) == 0
            && canvas_init(&mut small, pw / THUMB_DIV, ph / THUMB_DIV) == 0;
        if ok {
            canvas_downsample_keep_lines(&mut small, &full, 0.6);
            let key = cformat!("%016llx %d %d\n", bgimage_content_hash(bytes), src.w, src.h);
            // (the key first: a preview has one)
            let wrote = write_quiet(&kp, &key) == 0 && store_write_pgm(&pv, &small) == 0;
            made += wrote as i32;
        } else if dec == -1 {
            qlog!(
                "%s: file %d of %d is not a picture Grimoire can use (%s)",
                if photo { "photos" } else { "templates" },
                i + 1,
                nn,
                why
            );
            write_quiet(&bp, "");
            bad += 1;
        } else {
            later += 1; // out of memory, or the file did not read: the next scan tries again
        }
        bgpicture_free(&mut src);
        bgpicture_free(&mut fit);
        canvas_free(&mut full);
        canvas_free(&mut small);
    }
    // Previews of files that are gone.
    if let Ok(rd) = std::fs::read_dir(&cache) {
        use std::os::unix::ffi::OsStrExt;
        for e in rd {
            let Ok(e) = e else { break };
            let name = e.file_name();
            if name.as_bytes().first() == Some(&b'.') {
                continue;
            }
            let kept = keep.iter().any(|k| !k.is_empty() && name.as_bytes().starts_with(&k.as_bytes()[..16.min(k.len())]));
            if kept {
                continue;
            }
            let _ = std::fs::remove_file(Path::new(&cache).join(&name));
        }
    }
    qlog!(
        "%s: %d preview(s) made, %d file(s) that are no picture Grimoire can use, %d left for a later scan \
(out of memory, or a file that did not read)",
        if photo { "photos" } else { "templates" },
        made,
        bad,
        later
    );
    0
}

// ---- templates into notebooks ----

// sscanf's "%16llx" on the text at *pos (the C reads NUL-terminated text): leading white space is
// skipped, then an optional sign and 0x, and at most 16 characters of the field in all.
fn scan_hex16(t: &[u8], pos: &mut usize) -> Option<u64> {
    let at = |i: usize| t.get(i).copied().unwrap_or(0);
    let mut i = *pos;
    while matches!(at(i), b' ' | b'\t' | b'\n' | 0x0B | 0x0C | b'\r') {
        i += 1;
    }
    let start = i;
    let mut neg = false;
    if at(i) == b'+' || at(i) == b'-' {
        neg = at(i) == b'-';
        i += 1;
    }
    if at(i) == b'0' && (at(i + 1) | 32) == b'x' && (at(i + 2) as char).is_ascii_hexdigit() {
        i += 2;
    }
    let mut v: u64 = 0;
    let mut any = false;
    while i - start < 16 && (at(i) as char).is_ascii_hexdigit() {
        v = v.saturating_mul(16).saturating_add((at(i) as char).to_digit(16).unwrap_or(0) as u64);
        any = true;
        i += 1;
    }
    if !any {
        return None;
    }
    *pos = i;
    Some(if neg { v.wrapping_neg() } else { v })
}

// sscanf's "%d" on the text at *pos: white space, a sign, digits; a value past the range comes out
// as the C library's conversion cuts it.
fn scan_int(t: &[u8], pos: &mut usize) -> Option<i32> {
    let at = |i: usize| t.get(i).copied().unwrap_or(0);
    let mut i = *pos;
    while matches!(at(i), b' ' | b'\t' | b'\n' | 0x0B | 0x0C | b'\r') {
        i += 1;
    }
    let mut neg = false;
    if at(i) == b'+' || at(i) == b'-' {
        neg = at(i) == b'-';
        i += 1;
    }
    let mut v: u64 = 0;
    let mut any = false;
    while at(i).is_ascii_digit() {
        v = v.saturating_mul(10).saturating_add((at(i) - b'0') as u64);
        any = true;
        i += 1;
    }
    if !any {
        return None;
    }
    *pos = i;
    Some((if neg { v.wrapping_neg() } else { v }) as i32)
}

// What the previews job found of a template file as it is now (its name, size and time): the hash
// of its bytes and its size in pixels. 0, or -1 when it has not seen it.
fn cached_key(path: &str, hash: &mut u64, w: &mut i32, h: &mut i32) -> i32 {
    let mut kp = String::new();
    if !preview_of(path, "key", &mut kp, PATH_BUF) {
        return -1;
    }
    let Some(t) = read_file(&kp) else { return -1 };
    // sscanf(t, "%16llx %d %d", &v, w, h) == 3
    let t = &t[..t.iter().position(|&c| c == 0).unwrap_or(t.len())];
    let mut pos = 0;
    let (Some(v), Some(tw), Some(th)) = (scan_hex16(t, &mut pos), scan_int(t, &mut pos), scan_int(t, &mut pos)) else {
        return -1;
    };
    *w = tw;
    *h = th;
    *hash = v;
    0
}

// A template's key, or a photo's (its own kind: bgimage.h).
fn picture_key(photo: bool, content: u64, len: usize, pw: i32, ph: i32) -> u64 {
    if photo {
        bgimage_photo_key(content, len, pw, ph)
    } else {
        bgimage_template_key(content, len, pw, ph)
    }
}

fn picture_paper(t: &Template, nb: &NotebookInfo, ix: &BgIndex, kind: &mut PaperKind, photo: bool) -> i32 {
    let mut content = 0u64;
    let (mut w, mut h, mut pw, mut ph) = (0, 0, 0, 0);
    let mut id = 0u32;
    // The key template_install gives it (the same hash, length and page size), looked up.
    let Ok(st) = std::fs::metadata(t.path.as_str()) else { return -1 };
    if !st.is_file() || cached_key(t.path.as_str(), &mut content, &mut w, &mut h) != 0 {
        return -1;
    }
    store_notebook_page_size(nb, &mut pw, &mut ph);
    if !bgindex_find(ix, picture_key(photo, content, st.len() as usize, pw, ph), &mut id) {
        return -1;
    }
    *kind = paper_image(if photo { PAPER_ROLE_DOCUMENT } else { PAPER_ROLE_TEMPLATE }, id);
    0
}

/// The paper template t already is in notebook nb, by the notebook's bg/index.ini (ix, from
/// bgindex_load): 0 with *kind, or -1 when it was never put there, or its preview is not made yet
/// (its key comes from the hash the preview keeps). Reads nothing big, installs nothing.
pub fn template_paper(t: &Template, nb: &NotebookInfo, ix: &BgIndex, kind: &mut PaperKind) -> i32 {
    picture_paper(t, nb, ix, kind, false)
}

/// As template_paper, for a photo.
pub fn photo_paper(t: &Template, nb: &NotebookInfo, ix: &BgIndex, kind: &mut PaperKind) -> i32 {
    picture_paper(t, nb, ix, kind, true)
}

/// Puts the template at path into notebook nb, to be a page's paper: *kind is the paper a page takes
/// for it, known at once. Its picture is made for the notebook's page size in the background
/// (BG_TEMPLATE), and a page on it draws white until then (bgimage.h: the editor redraws the page
/// once that job is done, after bgimage_retry_missing). The id in *kind comes from a hash of the
/// file's bytes and the page size, so installing the same file again gives the same paper, and makes
/// its picture again if it went missing. 0, or -1 (*why, for the log: not a picture Grimoire can
/// use, a file over 16 MB, not readable, or one over 4 MB whose preview is not made yet). The hash
/// is the one the previews job keeps beside each preview, so nothing big is read on the UI thread:
/// only a template of at most 4 MB with no preview yet is read here (whole, once).
pub fn template_install(nb: &NotebookInfo, path: &str, kind: &mut PaperKind, why: &mut &'static str) -> i32 {
    install_picture(nb, path, kind, why, false)
}

/// As template_install, for a photo: *kind a document page's paper on its picture, made upright.
pub fn photo_install(nb: &NotebookInfo, path: &str, kind: &mut PaperKind, why: &mut &'static str) -> i32 {
    install_picture(nb, path, kind, why, true)
}

fn install_picture(nb: &NotebookInfo, path: &str, kind: &mut PaperKind, why: &mut &'static str, photo: bool) -> i32 {
    *why = "the file does not read";
    // On the UI thread: its key, from the hash of its bytes and the page size; the picture is made
    // in the background. The previews job, which reads every template whole anyway, keeps the hash
    // and whether the file is a picture Grimoire takes; only a small file it has not seen yet is
    // read here (whole, once).
    if path.len() >= JOB_PATH_MAX {
        *why = "too long a path for a background job";
        return -1;
    }
    let Ok(st) = std::fs::metadata(path) else { return -1 };
    if !st.is_file() {
        return -1;
    }
    if st.len() > TEMPLATE_MAX_BYTES as u64 {
        *why = "a file over 16 MB";
        return -1;
    }
    let (mut w, mut h, mut pw, mut ph) = (0, 0, 0, 0);
    let mut content = 0u64;
    let mut len = st.len() as usize;
    if cached_key(path, &mut content, &mut w, &mut h) != 0 {
        if st.len() > TEMPLATE_HASH_NOW as u64 {
            *why = "its preview is not made yet (a file over 4 MB is not read here)";
            return -1;
        }
        let data = read_source(path, TEMPLATE_HASH_NOW);
        let ok = match &data {
            Some(d) => {
                len = d.len();
                bgimage_check(d, len, &mut w, &mut h, why) == 0
            }
            None => false,
        };
        if ok {
            content = bgimage_content_hash(data.as_deref().unwrap_or(&[]));
        }
        if !ok {
            return -1;
        }
    }
    store_notebook_page_size(nb, &mut pw, &mut ph);
    let key = picture_key(photo, content, len, pw, ph);
    let (mut dir, mut file) = (String::new(), String::new());
    *why = "the notebook cannot be written";
    if bgimage_dir(nb.id.as_str(), &mut dir, 700) != 0 || mkdir_p(&dir) != 0 {
        return -1;
    }
    let mut ix = BgIndex::default();
    let mut id = 0u32;
    let mut added = false;
    if bgindex_load(&dir, &mut ix) != 0 {
        return -1;
    }
    let mut rc = bgindex_claim(&mut ix, &dir, key, &mut id, &mut added);
    if rc == 0 && added {
        rc = bgindex_save(&dir, &ix);
    }
    bgindex_free(&mut ix);
    if rc != 0 {
        return -1;
    }
    *kind = paper_image(if photo { PAPER_ROLE_DOCUMENT } else { PAPER_ROLE_TEMPLATE }, id);
    bgimage_file(&dir, id, &mut file, 800);
    if !exists(&file) {
        qlog!(
            "%s: %s: bg/%06x.png to be made (%dx%d picture for a %dx%d page)",
            if photo { "photos" } else { "templates" },
            &nb.id,
            id,
            w,
            h,
            pw,
            ph
        );
        bg_queue(BgKind::Template, Some(nb.id.as_str()), Some(path), store_dpi(), false);
    }
    0
}

/// New notebook on a template: notebook nb, which store_create just made, goes onto the template at
/// path. The template is installed into it (above: the first page draws white until its picture is
/// made), then the notebook's own paper and its first page go onto it (store_start_on). 0, or -1
/// and *why (for the log) when that could not be done: then the notebook stays on the paper it was
/// made on.
pub fn template_start_notebook(nb: &mut NotebookInfo, path: &str, why: &mut &'static str) -> i32 {
    let mut k = PaperKind::default();
    if template_install(nb, path, &mut k, why) != 0 {
        return -1;
    }
    if store_start_on(nb, k) == 0 {
        return 0;
    }
    *why = "the notebook cannot be written";
    -1
}

// The notebook's card (thumb.pgm, of its first page) when that page is on picture pic, drawn again
// once the picture is made: the editor drew it on white paper if the notebook was left before (a
// first page written on in the seconds a picture takes on the Kindle, or a page KOReader sent,
// looked at and left while its picture was fitted: a document page, whose picture is made here
// as a template's is). The editor may have the notebook open still, and write the card itself as
// this draws it (leaving page 1, which it saves first, or closing): then the card is drawn again
// from page 1 as it is now, up to three times.
fn redraw_card(nb: &NotebookInfo, pic: u32, dpi: f32) {
    let (mut pw, mut ph) = (0, 0);
    store_notebook_page_size(nb, &mut pw, &mut ph);
    // A job queued behind another is forked after the editor drew the page white: what it found
    // missing then is remembered here too (bgimage.h), and would draw the card white again.
    bgimage_forget(Some(nb.id.as_str()));
    for _tries in 0..3 {
        let was = store_read_thumb_file(nb.id.as_str());
        let mut p: Option<Box<Page>> = None;
        let mut t = Canvas::default();
        let mut rc = 0; // (page 1 not on the picture: nothing to draw)
        if store_read_page(nb, 0, &mut p) != 0 {
            rc = -1;
        } else if p.as_ref().is_some_and(|pg| paper_is_image(pg.paper) && paper_image_id(pg.paper) == pic) {
            rc = if canvas_init(&mut t, pw / 3, ph / 3) == 0 { 0 } else { -1 };
            if rc == 0 {
                if let Some(pg) = p.as_deref() {
                    page_render_thumb(&mut t, pg, dpi);
                }
                rc = store_replace_thumb_if_same(nb.id.as_str(), &t, was.as_deref().unwrap_or(&[]));
                canvas_free(&mut t);
            }
        }
        if let Some(pg) = p {
            page_free(pg);
        }
        if rc < 0 {
            qlog!("templates: %s: its card could not be drawn again", &nb.id);
        }
        if rc != 1 {
            return;
        }
    }
    qlog!("templates: %s: its card changed each time it was drawn again: the editor's stays", &nb.id);
}

// In the child: the picture template_install gave an id to, made for the notebook's page size.
fn make_template(id: &str, src: &str, dpi: f32) -> i32 {
    let mut nb = NotebookInfo::default();
    if store_load_info(id, &mut nb) != 0 {
        qlog!("templates: notebook %s does not open", id);
        return -1;
    }
    let (mut pw, mut ph) = (0, 0);
    store_notebook_page_size(&nb, &mut pw, &mut ph);
    let Some(data) = read_source(src, TEMPLATE_MAX_BYTES) else {
        qlog!("templates: %s: the template file is gone", id);
        return -1;
    };
    let len = data.len();
    let (mut dir, mut file) = (String::new(), String::new());
    let mut ix = BgIndex::default();
    let mut pic = 0u32;
    let photo = photo_path(src); // (upright first, and a key of its own kind)
    let key = picture_key(photo, bgimage_content_hash(&data), len, pw, ph);
    let known = bgimage_dir(id, &mut dir, 700) == 0 && bgindex_load(&dir, &mut ix) == 0 && bgindex_find(&ix, key, &mut pic);
    bgindex_free(&mut ix);
    if !known {
        qlog!("templates: %s: no id was given to this template (it changed since it was picked?)", id);
        return -1;
    }
    bgimage_file(&dir, pic, &mut file, 800);
    let mut rc = 0;
    if !exists(&file) {
        let mut p = BgPicture::default();
        let mut fit = BgPicture::default();
        let mut why: &'static str = "out of memory";
        rc = if bgimage_decode(&data, &mut p, &mut why) == 0
            && (!photo || bgimage_orient(&mut p, bgimage_exif_orientation(&data)) == 0)
            && bgimage_normalize(&p, pw, ph, &mut fit) == 0
            && bgimage_write_png(&file, &fit) == 0
        {
            0
        } else {
            -1
        };
        if rc != 0 {
            qlog!("templates: %s: bg/%06x.png could not be made (%s)", id, pic, why);
        } else {
            qlog!("templates: %s: bg/%06x.png made (%dx%d, %s)", id, pic, pw, ph, if fit.ch == 3 { "colour" } else { "gray" });
        }
        bgpicture_free(&mut p);
        bgpicture_free(&mut fit);
        if rc == 0 {
            redraw_card(&nb, pic, dpi);
        }
    }
    rc
}

// ---- a page KOReader sent (annotate.h) ----

// Whether src is notebook id's own bg/<6 hex digits>.source.png: a page picture KOReader's plugin
// sent that is not of the notebook's page size, made into the page's picture here as a template's
// is (its id comes from the same key), then removed, made or not: it is no picture of the notebook
// (bg/ keeps only those), and the next request for its page brings another.
fn annotate_source(id: &str, src: &str) -> bool {
    let mut dir = String::new();
    if bgimage_dir(id, &mut dir, 700) != 0 {
        return false;
    }
    let n = dir.len();
    let s = cbytes(src);
    if !s.starts_with(dir.as_bytes()) || s.get(n) != Some(&b'/') {
        return false;
    }
    let f = &s[n + 1..];
    if f.len() != 17 || &f[6..] != b".source.png" {
        return false;
    }
    f[..6].iter().all(|c| c.is_ascii_hexdigit())
}

/// bgjob.h: the work of a BG_TEMPLATE job, in the child; 0 = done.
pub fn bg_template(id: &str, src: &str, dpi: f32) -> i32 {
    let cap = cap_memory();
    let rc = if id.is_empty() { make_previews(src) } else { make_template(id, src, dpi) }; // no notebook: the folder's previews
    uncap_memory(&cap);
    if !id.is_empty() && annotate_source(id, src) {
        let _ = std::fs::remove_file(src); // a page KOReader sent: done with (above)
    }
    rc
}

// ---- imports ----

// Only one import runs at a time, in whichever process (a run's last jobs go on in a detached child
// after Grimoire closes): grimoire/import/.lock, held while it works. A start that finds it held
// leaves the import folders for the next start. Err(errno) (EWOULDBLOCK: it is held), or the
// descriptor that holds the lock.
fn import_lock(wait: bool) -> Result<i32, i32> {
    let mut path = String::new();
    root_path("import/.lock", &mut path, 700);
    let Some(c) = cpath(&path) else { return Err(libc::ENOENT) };
    // SAFETY: open reads the NUL-terminated path we own; the descriptor it returns is ours alone.
    let fd = unsafe { libc::open(c.as_ptr(), libc::O_RDWR | libc::O_CREAT, 0o644 as libc::c_uint) };
    if fd < 0 {
        return Err(errno());
    }
    // SAFETY: flock only takes the open descriptor.
    while unsafe { libc::flock(fd, libc::LOCK_EX | if wait { 0 } else { libc::LOCK_NB }) } != 0 {
        let err = errno();
        if err == libc::EINTR {
            continue;
        }
        // SAFETY: close takes the descriptor opened above, which is not used again.
        unsafe { libc::close(fd) };
        return Err(err);
    }
    Ok(fd)
}

// Removes a folder an import was building (notebooks/.import-<id>/: its files and its bg/).
fn remove_work<P: AsRef<Path>>(work: P) {
    let work = work.as_ref();
    for pass in 0..2 {
        let dir = if pass == 0 { work.join("bg") } else { work.to_path_buf() };
        if let Ok(rd) = std::fs::read_dir(&dir) {
            for e in rd {
                let Ok(e) = e else { break };
                let _ = std::fs::remove_file(dir.join(e.file_name()));
            }
        }
        let _ = std::fs::remove_dir(&dir);
    }
}

// The notebook id written in an import folder's mark, or false (none, or "done").
fn read_mark(src: &str, id: &mut String, done: &mut bool) -> bool {
    let mut path = String::new();
    csnprintf!(&mut path, PATH_BUF, "%s/%s", src, IMPORT_MARK);
    let t = read_file(&path);
    *done = t.as_ref().is_some_and(|t| t.starts_with(b"done"));
    let mut ok = false;
    if let Some(t) = &t {
        if !*done {
            // (the text up to the first line end, or NUL)
            let end = t.iter().position(|&c| c == b'\r' || c == b'\n' || c == 0).unwrap_or(t.len());
            let t = &t[..end];
            let mut dir = String::new();
            ok = t.len() < STORE_ID_LEN
                && std::str::from_utf8(t).is_ok_and(|s| store_notebook_dir(s, &mut dir, 700) == 0);
            if ok {
                strlcpy_q(id, std::str::from_utf8(t).unwrap_or(""), STORE_ID_LEN);
            }
        }
    }
    ok
}

fn write_mark(src: &str, text: &str) -> bool {
    let mut path = String::new();
    csnprintf!(&mut path, PATH_BUF, "%s/%s", src, IMPORT_MARK);
    write_quiet(&path, text) == 0
}

// An import that did not happen leaves the folder as the owner made it.
fn drop_mark(src: &str) {
    let mut path = String::new();
    csnprintf!(&mut path, PATH_BUF, "%s/%s", src, IMPORT_MARK);
    let _ = std::fs::remove_file(&path);
}

// ---- import attempts: import/.attempts ----
//
// A line "<name> <pictures> <tries>" for each folder still in import/ that an import tried: a hash
// of its name and one of its pictures (their names, sizes and times; 16 hex digits each), and how
// many starts tried it with those pictures (TRIES_REFUSED once a page was no picture Grimoire
// takes). A folder refused, or tried IMPORT_TRIES times (a failure that might have passed, or an
// import cut off each time), waits until its pictures change: it costs one try, not one at every
// start, and its failure shows once.
const ATTEMPTS_MAX: usize = 64;
const TRIES_REFUSED: i32 = 99;

#[derive(Clone, Copy)]
struct Attempt {
    name: u64,
    pictures: u64,
    tries: i32,
}

const ATTEMPT_TRY: i32 = 0;
const ATTEMPT_REFUSED: i32 = 1;
const ATTEMPT_DONE: i32 = 2;

const FNV0: u64 = 14695981039346656037;
fn fnv(mut h: u64, d: &[u8]) -> u64 {
    for &b in d {
        h = (h ^ b as u64).wrapping_mul(1099511628211);
    }
    h
}

fn folder_name_hash(src: &str) -> u64 {
    let name = src.rfind('/').map_or(src, |s| &src[s + 1..]);
    fnv(FNV0, name.as_bytes())
}

// What a folder's pictures (names, in order) are: their names, sizes and times.
fn pictures_hash(src: &str, names: &[String]) -> u64 {
    let n = names.len() as i32;
    let mut h = fnv(FNV0, &n.to_ne_bytes());
    for name in names {
        let mut path = String::new();
        csnprintf!(&mut path, PATH_BUF, "%s/%s", src, name);
        let mut v: [i64; 2] = [-1, -1];
        if let Ok(st) = std::fs::metadata(&path) {
            v[0] = st.len() as i64;
            v[1] = st.mtime();
        }
        // (the name's NUL too, then the two numbers as they lie in memory)
        h = fnv(fnv(h, name.as_bytes()), &[0]);
        h = fnv(h, &v[0].to_ne_bytes());
        h = fnv(h, &v[1].to_ne_bytes());
    }
    h
}

fn folder_pictures_hash(src: &str) -> u64 {
    let mut names = Vec::new();
    if list_names(src, true, STORE_MAX_PAGES + 1, &mut names) != 0 {
        return 0;
    }
    pictures_hash(src, &names)
}

fn attempts_load() -> Vec<Attempt> {
    let mut path = String::new();
    root_path("import/.attempts", &mut path, 700);
    let mut a: Vec<Attempt> = Vec::new();
    let Some(t) = read_file(&path) else { return a };
    let t = &t[..t.iter().position(|&c| c == 0).unwrap_or(t.len())];
    // (strtok_r on "\n": empty lines are not lines)
    for line in t.split(|&c| c == b'\n').filter(|l| !l.is_empty()) {
        if a.len() >= ATTEMPTS_MAX {
            break;
        }
        // sscanf(line, "%16llx %16llx %d", &nh, &ph, &tries) == 3 && tries > 0
        let mut pos = 0;
        let (Some(nh), Some(ph), Some(tries)) = (scan_hex16(line, &mut pos), scan_hex16(line, &mut pos), scan_int(line, &mut pos)) else {
            continue;
        };
        if tries > 0 {
            a.push(Attempt { name: nh, pictures: ph, tries });
        }
    }
    a
}

fn attempts_save(a: &[Attempt]) {
    let mut path = String::new();
    root_path("import/.attempts", &mut path, 700);
    if a.is_empty() {
        let _ = std::fs::remove_file(&path);
        return;
    }
    let mut text = cformat!("# Grimoire: import folders tried (docs/FORMAT.md)\n");
    for e in a {
        text.push_str(&cformat!("%016llx %016llx %d\n", e.name, e.pictures, e.tries));
    }
    if write_file_atomic(&path, text.as_bytes()) != 0 {
        qlog!("import: could not write import/.attempts");
    }
}

// In the child, under the lock: the folder at src (with these pictures) was tried once more, or was
// refused, or was imported (its record goes).
fn attempts_note(src: &str, pictures: u64, what: i32) {
    let mut a = attempts_load();
    let mut at: Option<usize> = None;
    let nh = folder_name_hash(src);
    for (i, e) in a.iter().enumerate() {
        if e.name == nh {
            at = Some(i);
        }
    }
    if what == ATTEMPT_DONE {
        let Some(at) = at else { return };
        a.swap_remove(at);
    } else if let Some(at) = at {
        a[at].tries = if what == ATTEMPT_REFUSED {
            TRIES_REFUSED
        } else if a[at].pictures != pictures {
            1
        } else if a[at].tries < TRIES_REFUSED - 1 {
            a[at].tries + 1
        } else {
            a[at].tries
        };
        a[at].pictures = pictures;
    } else {
        if a.len() == ATTEMPTS_MAX {
            a.remove(0); // the oldest record goes
        }
        a.push(Attempt { name: nh, pictures, tries: if what == ATTEMPT_REFUSED { TRIES_REFUSED } else { 1 } });
    }
    attempts_save(&a);
}

// What an import was building when it was cut off (notebooks/.import-<id>/). One holding its
// notebook.ini is whole (that file is written last, each one before it made durable): it goes into
// place, and its folder moves when its id comes up (the folder's mark names it). Any other is
// removed. Whether there were any; with `act`, deals with them all.
fn import_leftovers(act: bool) -> bool {
    use std::os::unix::ffi::OsStrExt;
    let mut notebooks = String::new();
    root_path("notebooks", &mut notebooks, 600);
    let mut any = false;
    let mut placed = false;
    if let Ok(rd) = std::fs::read_dir(&notebooks) {
        for e in rd {
            let Ok(e) = e else { break };
            let name = e.file_name();
            if !name.as_bytes().starts_with(b".import-") {
                continue;
            }
            any = true;
            if !act {
                break;
            }
            let Some(name) = name.to_str() else {
                // (no id the store takes: it is removed)
                remove_work(Path::new(&notebooks).join(&name));
                qlog!("import: removed what a cut-off import left (%s)", &name.to_string_lossy()[8..]);
                continue;
            };
            let id = &name[8..];
            let (mut work, mut ini, mut final_) = (String::new(), String::new(), String::new());
            csnprintf!(&mut work, 1000, "%s/%s", &notebooks, name);
            csnprintf!(&mut ini, 1100, "%s/notebook.ini", &work);
            if exists(&ini)
                && store_notebook_dir(id, &mut final_, 700) == 0
                && !exists(&final_)
                && std::fs::rename(&work, &final_).is_ok()
            {
                placed = true;
                qlog!("import: put in place the notebook a cut-off import had finished (%s)", id);
                continue;
            }
            remove_work(&work);
            qlog!("import: removed what a cut-off import left (%s)", id);
        }
    }
    if placed {
        sync_dir(&notebooks);
    }
    any
}

/// At startup: every folder in grimoire/import/ (but done/ and dot-folders) holding page pictures
/// is queued to become a notebook (BG_IMPORT_FOLDER), at most 8 a start. What an import that was cut
/// off left in notebooks/ (.import-<id>/) is put in place when it was whole, else removed. A folder
/// tried 3 times, or refused (a page that is no picture Grimoire takes), waits until its pictures
/// change (import/.attempts, docs/FORMAT.md). Returns how many were queued.
pub fn import_scan(dpi: f32) -> i32 {
    let mut dir = String::new();
    root_path("import", &mut dir, 600);
    let mut names = Vec::new();
    if list_names(&dir, false, 1000, &mut names) != 0 {
        names.clear();
    }
    let folders = names.iter().filter(|n| n.as_str() != "done").count();
    if folders == 0 && !import_leftovers(false) {
        // nothing to do (and no lock file made for it)
        return 0;
    }
    let lock = match import_lock(false) {
        Ok(fd) => fd,
        Err(err) => {
            if err == libc::EWOULDBLOCK {
                qlog!("import: an import from an earlier run is still going; the import folders wait for the next start");
            } else {
                qlog!("import: the import lock cannot be taken (%s); the import folders wait for the next start", &strerror(err));
            }
            return 0;
        }
    };
    // Leftovers of an import that was cut off (Grimoire or the Kindle stopped mid-way): no import
    // runs now (the lock), so they are no one's.
    import_leftovers(true);
    let mut att = attempts_load();
    let natt = att.len();
    let mut seen = [false; ATTEMPTS_MAX];
    let (mut queued, mut empty, mut waiting, mut toolong) = (0, 0, 0, 0);
    for name in &names {
        if name == "done" {
            continue;
        }
        let mut src = String::new();
        let mut id = String::new();
        csnprintf!(&mut src, PATH_BUF, "%s/%s", &dir, name);
        let nh = folder_name_hash(&src);
        let mut at: Option<usize> = None;
        for k in 0..natt {
            if att[k].name == nh {
                at = Some(k);
                seen[k] = true;
            }
        }
        if with(|s| s.pending.len()) >= IMPORT_QUEUE_MAX {
            continue; // (the next start takes it)
        }
        if src.len() >= JOB_PATH_MAX {
            // it would be cut on its way to the job
            toolong += 1;
            continue;
        }
        let mut pics = Vec::new();
        if list_names(&src, true, 1, &mut pics) != 0 {
            continue;
        }
        let np = pics.len();
        let mut done = false;
        let have = read_mark(&src, &mut id, &mut done);
        if np == 0 || done {
            empty += (np == 0) as i32;
            continue;
        }
        if at.is_some_and(|at| att[at].tries >= IMPORT_TRIES && att[at].pictures == folder_pictures_hash(&src)) {
            waiting += 1;
            continue;
        }
        // A mark left by a run that was cut off names the notebook the folder was becoming: that id
        // again, so a notebook it made (its folder not moved yet) is finished, never made twice.
        if !have {
            store_make_id(&mut id);
            let text = cformat!("%s\n", &id);
            if !write_mark(&src, &text) {
                qlog!("import: an import folder cannot be written to; left as it is");
                continue;
            }
        }
        bg_queue(BgKind::ImportFolder, Some(&id), Some(&src), dpi, true);
        with(|s| s.pending.push(FixedStr::from_str(&id)));
        queued += 1;
    }
    // the records of folders no longer here (imported, or taken away) go
    let mut kept = 0;
    for k in 0..natt {
        if seen[k] {
            att[kept] = att[k];
            kept += 1;
        }
    }
    if kept != natt {
        attempts_save(&att[..kept]);
    }
    // SAFETY: close takes the descriptor import_lock opened, which is not used again.
    unsafe { libc::close(lock) };
    if queued != 0 || empty != 0 || waiting != 0 || toolong != 0 {
        qlog!(
            "import: %d folder(s) queued; left as they are: %d without page pictures, %d that failed and have not \
changed since, %d with too long a name",
            queued,
            empty,
            waiting,
            toolong
        );
    }
    queued
}

/// When bg_poll_kind reports a BG_IMPORT_FOLDER job done (ok: it succeeded): 1 with the new
/// notebook's page count in *pages once an import made its notebook, -1 when one failed, 0 when
/// nothing came of it (another run had imported that folder).
pub fn import_finished(ok: bool, pages: &mut i32) -> i32 {
    // The imports run in the order they were queued: the oldest pending one has just finished,
    // unless the job queue dropped some (then any later one that made its notebook did).
    let n = with(|s| s.pending.len());
    let mut nb = NotebookInfo::default();
    for i in 0..n {
        let id = with(|s| s.pending[i]);
        if (i > 0 && !ok) || store_load_info(id.as_str(), &mut nb) != 0 {
            continue;
        }
        *pages = nb.npages();
        with(|s| s.pending.remove(i));
        return 1;
    }
    if ok || n == 0 {
        return 0;
    }
    with(|s| s.pending.remove(0)); // the oldest failed
    -1
}

// Moves a folder that became notebook id to import/done/<Name>-<time>; one that cannot move is
// marked done in place, so no later start imports it again.
fn finish_source(src: &str, id: &str) {
    let (mut import, mut done, mut to) = (String::new(), String::new(), String::new());
    root_path("import", &mut import, 600);
    root_path("import/done", &mut done, 700);
    let name = src.rfind('/').map_or(src, |s| &src[s + 1..]);
    let tm = local_tm(wall_time());
    let mut stamp = String::new();
    csnprintf!(
        &mut stamp,
        32,
        "%04d%02d%02d-%02d%02d%02d",
        tm.tm_year + 1900,
        tm.tm_mon + 1,
        tm.tm_mday,
        tm.tm_hour,
        tm.tm_min,
        tm.tm_sec
    );
    let mut moved = false;
    let mut err = 0;
    if mkdir_p(&done) != 0 {
        err = errno();
    } else {
        let mut k = 1;
        while k < 100 && !moved {
            if k == 1 {
                csnprintf!(&mut to, PATH_BUF + 64, "%s/%s-%s", &done, name, &stamp);
            } else {
                csnprintf!(&mut to, PATH_BUF + 64, "%s/%s-%s-%d", &done, name, &stamp, k);
            }
            if exists(&to) {
                k += 1;
                continue;
            }
            match std::fs::rename(src, &to) {
                Ok(()) => moved = true,
                Err(e) => {
                    err = e.raw_os_error().unwrap_or(0);
                    break;
                }
            }
            k += 1;
        }
    }
    if moved {
        sync_dir(&done);
        sync_dir(&import);
        drop_mark(&to);
        return;
    }
    let text = cformat!("done %s\n", id);
    write_mark(src, &text);
    qlog!("import: %s: its folder could not move to import/done (%s); marked as imported", id, &strerror(err));
}

// Every page's header (a millisecond or so a page), before any picture is made: a folder with a
// page that is no picture Grimoire takes fails at once, whatever its place. IMPORT_REFUSED, or 0.
fn check_pages(id: &str, src: &str, names: &[String]) -> i32 {
    let n = names.len();
    if n == 0 || n > STORE_MAX_PAGES {
        qlog!("import: %s: %s page pictures; nothing imported", id, if n != 0 { "more than 2000" } else { "no" });
        return IMPORT_REFUSED;
    }
    for (i, name) in names.iter().enumerate() {
        let mut path = String::new();
        let (mut w, mut h) = (0, 0);
        let mut why: &'static str = "";
        csnprintf!(&mut path, PATH_BUF, "%s/%s", src, name);
        if check_file(&path, BG_SOURCE_MAX_BYTES, &mut w, &mut h, &mut why) != 0 {
            qlog!("import: %s: page %d of %d: %s; nothing imported", id, i + 1, n, why);
            return IMPORT_REFUSED;
        }
    }
    0
}

// The notebook itself, built in notebooks/.import-<id>/ (which the Library never lists) and renamed
// into place only once whole; anything going wrong removes it. IMPORT_DONE (its folder moved to
// import/done), IMPORT_RETRY or IMPORT_REFUSED.
fn build_notebook(id: &str, src: &str, final_: &str, names: &[String]) -> i32 {
    let n = names.len();
    let mut work = String::new();
    let mut bgdir = String::new();
    csnprintf!(&mut work, 700, "%s/notebooks/.import-%s", &store_root(), id);
    csnprintf!(&mut bgdir, 800, "%s/bg", &work);
    remove_work(&work);
    let (mut pw, mut ph) = (0, 0);
    store_page_size(&mut pw, &mut ph);
    let mut ix = BgIndex::default();
    let mut pids: Vec<u32> = Vec::new();
    let mut papers: Vec<PaperKind> = Vec::new();
    let mut nb = NotebookInfo::default();
    let rc = 'out: {
        if pids.try_reserve_exact(n).is_err() || papers.try_reserve_exact(n).is_err() {
            qlog!("import: %s: out of memory; nothing imported", id);
            break 'out IMPORT_RETRY;
        }
        if mkdir_p(&bgdir) != 0 {
            qlog!("import: %s: notebooks/ cannot be written (%s); nothing imported", id, &strerror(errno()));
            break 'out IMPORT_RETRY;
        }
        qlog!("import: %s: %d page(s), for %dx%d pages", id, n, pw, ph);
        for (i, name) in names.iter().enumerate() {
            let mut path = String::new();
            csnprintf!(&mut path, PATH_BUF, "%s/%s", src, name);
            let data = read_source(&path, BG_SOURCE_MAX_BYTES);
            let mut p = BgPicture::default();
            let mut fit = BgPicture::default();
            let mut why: &'static str = "the file does not read";
            let mut fail = IMPORT_RETRY;
            if let Some(d) = &data {
                let dec = bgimage_decode(d, &mut p, &mut why);
                fail = if dec == 0 {
                    0
                } else if dec == -2 {
                    IMPORT_RETRY
                } else {
                    IMPORT_REFUSED
                };
            }
            drop(data);
            if fail == 0 && bgimage_normalize(&p, pw, ph, &mut fit) != 0 {
                why = "out of memory";
                fail = IMPORT_RETRY;
            }
            bgpicture_free(&mut p);
            let mut pic = 0u32;
            let mut added = false;
            if fail == 0 && bgindex_claim(&mut ix, &bgdir, bgimage_key(&fit), &mut pic, &mut added) != 0 {
                why = "out of memory";
                fail = IMPORT_RETRY;
            }
            if fail == 0 && added {
                let mut file = String::new();
                bgimage_file(&bgdir, pic, &mut file, 900);
                if bgimage_write_png(&file, &fit) != 0 {
                    why = "a picture could not be written";
                    fail = IMPORT_RETRY;
                }
            }
            if fail == 0 && i == 0 {
                // the Library's thumbnail: page 1 as page_render_thumb makes it
                let mut full = Canvas::default();
                let mut thumb = Canvas::default();
                let mut tp = String::new();
                csnprintf!(&mut tp, 900, "%s/thumb.pgm", &work);
                if bgimage_luma(&fit, &mut full) == 0 && canvas_init(&mut thumb, pw / 3, ph / 3) == 0 {
                    canvas_downsample(&mut thumb, &full);
                    store_write_pgm(&tp, &thumb);
                }
                canvas_free(&mut full);
                canvas_free(&mut thumb);
            }
            bgpicture_free(&mut fit);
            if fail == 0 {
                pids.push(i as u32 + 1);
                papers.push(paper_image(PAPER_ROLE_DOCUMENT, pic));
                let pg = page_new(pw, ph, papers[i]);
                if store_write_page_in(&work, pids[i], &pg) != 0 {
                    why = "a page could not be written";
                    fail = IMPORT_RETRY;
                }
                page_free(pg);
            }
            if fail != 0 {
                qlog!("import: %s: page %d of %d: %s; nothing imported", id, i + 1, n, why);
                break 'out fail;
            }
            if (i + 1) % 10 == 0 && i + 1 < n {
                qlog!("import: %s: %d of %d pages", id, i + 1, n);
            }
        }
        // The notebook around them: its pages are document pages, and pages added to it are blank.
        nb.id.set(id);
        let name = src.rfind('/').map_or(src, |s| &src[s + 1..]);
        copy_name(&mut nb.name, name); // (a long one cut between characters)
        nb.paper = PAPER_BLANK;
        nb.created = wall_time();
        nb.modified = nb.created;
        nb.page_ids = pids.clone();
        nb.next_page_id = n as u32 + 1;
        nb.page_w = pw;
        nb.page_h = ph;
        // notebook.ini last: a folder holding it is whole (import_leftovers).
        if bgindex_save(&bgdir, &ix) != 0 || store_write_pages_list_in(&bgdir, &pids, &papers) != 0 || store_write_info_in(&work, &nb) != 0 {
            qlog!("import: %s: its notebook files could not be written; nothing imported", id);
            break 'out IMPORT_RETRY;
        }
        if let Err(e) = std::fs::rename(&work, final_) {
            qlog!("import: %s: could not move the notebook into place (%s); nothing imported", id, &strerror(e.raw_os_error().unwrap_or(0)));
            break 'out IMPORT_RETRY;
        }
        let mut notebooks = String::new();
        root_path("notebooks", &mut notebooks, 600);
        sync_dir(&notebooks); // the notebook in place for good before its folder leaves import/
        qlog!("import: notebook %s made of %d page(s), %d picture(s)", id, n, ix.e.len());
        finish_source(src, id);
        IMPORT_DONE
    };
    if rc != IMPORT_DONE {
        remove_work(&work);
    }
    bgindex_free(&mut ix);
    rc
}

// The import itself, under the lock. A failed one leaves no notebook, and the folder as it was.
fn import_folder(src: &str) -> i32 {
    if !is_dir(src) {
        qlog!("import: the folder is gone (an earlier run imported it)");
        return IMPORT_DONE;
    }
    let mut id = String::new();
    let mut final_ = String::new();
    let mut done = false;
    if !read_mark(src, &mut id, &mut done) {
        if done {
            return IMPORT_DONE;
        }
        store_make_id(&mut id);
    }
    if store_notebook_dir(&id, &mut final_, 600) != 0 {
        return IMPORT_RETRY;
    }
    if is_dir(&final_) {
        // made by a run cut off before its folder moved
        qlog!("import: %s: made by an earlier run; its folder moves now", &id);
        finish_source(src, &id);
        attempts_note(src, 0, ATTEMPT_DONE);
        return IMPORT_DONE;
    }
    let mut names = Vec::new();
    if list_names(src, true, STORE_MAX_PAGES + 1, &mut names) != 0 {
        qlog!("import: %s: its folder does not read (%s)", &id, &strerror(errno()));
        return IMPORT_RETRY;
    }
    let pictures = pictures_hash(src, &names);
    attempts_note(src, pictures, ATTEMPT_TRY); // (counted first: a try cut off counts too)
    let mut rc = check_pages(&id, src, &names);
    if rc == 0 {
        rc = build_notebook(&id, src, &final_, &names);
    }
    if rc == IMPORT_DONE {
        attempts_note(src, pictures, ATTEMPT_DONE);
    } else {
        if rc == IMPORT_REFUSED {
            attempts_note(src, pictures, ATTEMPT_REFUSED);
        }
        drop_mark(src);
    }
    rc
}

/// bgjob.h: the work of a BG_IMPORT_FOLDER job, in the child; 0 = done.
pub fn bg_import_folder(path: &str, dpi: f32) -> i32 {
    let _ = dpi;
    let lock = match import_lock(true) {
        Ok(fd) => fd,
        Err(err) => {
            qlog!("import: no lock (%s)", &strerror(err));
            return -1;
        }
    };
    let cap = cap_memory();
    let rc = import_folder(path);
    uncap_memory(&cap);
    // SAFETY: close takes the descriptor import_lock opened, which is not used again.
    unsafe { libc::close(lock) };
    if rc == IMPORT_DONE {
        0
    } else {
        -1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A scratch folder of this test's own.
    fn scratch(name: &str) -> String {
        let dir = std::env::temp_dir().join(format!("grimoire-bgwork-template-{}-{}", std::process::id(), name));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir.to_str().unwrap().to_string()
    }

    #[test]
    fn natural_order() {
        assert!(natural_cmp("page-2", "page-10") < 0 && natural_cmp("page-10", "page-2") > 0);
        assert!(natural_cmp("page-9.png", "page-10.png") < 0);
        assert!(natural_cmp("Page-3", "page-4") < 0 && natural_cmp("a", "B") < 0);
        assert!(natural_cmp("page-002", "page-3") < 0 && natural_cmp("page-02", "page-2") != 0);
        assert!(natural_cmp("x", "x1") < 0 && natural_cmp("x1y2", "x1y10") < 0 && natural_cmp("same", "same") == 0);
        assert!(natural_cmp("scan 1 of 12", "scan 12 of 12") < 0);
        assert!(natural_cmp("100", "99") > 0 && natural_cmp("0", "00") != 0);
        // equal but for case: the bytes decide ('B' is before 'b')
        assert_eq!(natural_cmp("B", "b"), -1);
        assert_eq!(natural_cmp("b", "B"), 1);
        assert_eq!(natural_cmp("", ""), 0);
        assert_eq!(natural_cmp("", "a"), -1);
    }

    #[test]
    fn picture_names() {
        for n in ["a.png", "a.PNG", "a.jpg", "a.JPeG", "a.pgm", "a.b.jpeg", "page 1.Png"] {
            assert!(picture_name(n), "{n}");
        }
        for n in [".hidden.png", "a", "a.gif", "a.png.txt", ".png", "png", "a.pn"] {
            assert!(!picture_name(n), "{n}");
        }
    }

    #[test]
    fn names_are_cut_between_characters() {
        let mut f = FixedStr::<STORE_NAME_LEN>::new();
        let name: String = "\u{e9}".repeat(50); // 100 bytes
        copy_name(&mut f, &name);
        assert_eq!(f.len(), 94);
        assert_eq!(f.as_bytes(), &name.as_bytes()[..94]);
        copy_name(&mut f, "short");
        assert_eq!(f, "short");
    }

    #[test]
    fn sscanf_of_the_key_and_attempt_lines() {
        let one = |t: &str| {
            let t = t.as_bytes();
            let mut pos = 0;
            Some((scan_hex16(t, &mut pos)?, scan_int(t, &mut pos)?, scan_int(t, &mut pos)?))
        };
        assert_eq!(one("00000000000000ff 600 800\n"), Some((0xff, 600, 800)));
        assert_eq!(one("DEADBEEFcafef00d  7\t-3"), Some((0xdeadbeefcafef00d, 7, -3)));
        assert_eq!(one("0x10 1 2"), Some((0x10, 1, 2)));
        assert_eq!(one("0x 1 2"), None); // no digit after the prefix: the 0 is the number, the x is not one
        assert_eq!(one(""), None);
        assert_eq!(one("zz 1 2"), None);
        assert_eq!(one("ff 1"), None);
        assert_eq!(one("ff x 2"), None);
        // the field stops at 16 characters: the 17th starts the next conversion
        let mut pos = 0;
        let t = b"0123456789abcdef1 2";
        assert_eq!(scan_hex16(t, &mut pos), Some(0x0123456789abcdef));
        assert_eq!(scan_hex16(t, &mut pos), Some(1));
        assert_eq!(scan_int(t, &mut pos), Some(2));
        // a number past the range is cut to 32 bits, as the C library stores it
        let mut pos = 0;
        assert_eq!(scan_int(b"4294967297", &mut pos), Some(1));
    }

    #[test]
    fn previews_are_named_by_name_size_and_time() {
        let dir = scratch("preview");
        std::fs::write(format!("{dir}/a.png"), b"12345").unwrap();
        let (mut pgm, mut key, mut again) = (String::new(), String::new(), String::new());
        assert!(preview_path(&dir, "a.png", "pgm", &mut pgm, PATH_BUF));
        assert!(preview_path(&dir, "a.png", "key", &mut key, PATH_BUF));
        assert!(pgm.starts_with(&format!("{dir}/.cache/")) && pgm.ends_with(".pgm") && key.ends_with(".key"));
        assert_eq!(pgm.len(), dir.len() + "/.cache/".len() + 16 + ".pgm".len());
        assert_eq!(&pgm[..pgm.len() - 3], &key[..key.len() - 3]);
        // FNV-1a over the name, then "/size/mtime"
        let mtime = std::fs::metadata(format!("{dir}/a.png")).unwrap().mtime();
        let mut h: u64 = 14695981039346656037;
        for b in format!("a.png/5/{mtime}").bytes() {
            h = (h ^ b as u64).wrapping_mul(1099511628211);
        }
        assert_eq!(pgm, format!("{dir}/.cache/{h:016x}.pgm"));
        // by path: the same
        assert!(preview_of(&format!("{dir}/a.png"), "pgm", &mut again, PATH_BUF));
        assert_eq!(again, pgm);
        // a file changed in size is another preview; a missing one has none
        std::fs::write(format!("{dir}/a.png"), b"123456").unwrap();
        assert!(preview_path(&dir, "a.png", "pgm", &mut again, PATH_BUF));
        assert_ne!(again, pgm);
        assert!(!preview_path(&dir, "nope.png", "pgm", &mut again, PATH_BUF));
        assert!(!preview_of("no-slash.png", "pgm", &mut again, PATH_BUF));
        assert!(!preview_of(&format!("{dir}/{}", "x".repeat(600)), "pgm", &mut again, PATH_BUF));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn listing_pictures_and_folders() {
        let dir = scratch("list");
        for f in ["page-10.png", "page-2.PNG", "Page-1.jpg", "notes.txt", ".hidden.png", "b.jpeg", "c.pgm", "d.gif"] {
            std::fs::write(format!("{dir}/{f}"), b"x").unwrap();
        }
        for d in ["zeta", "alpha-10", "alpha-9", ".dot", "done.png"] {
            std::fs::create_dir(format!("{dir}/{d}")).unwrap();
        }
        let mut names = Vec::new();
        assert_eq!(list_names(&dir, true, 100, &mut names), 0);
        assert_eq!(names, ["b.jpeg", "c.pgm", "Page-1.jpg", "page-2.PNG", "page-10.png"]);
        assert_eq!(list_names(&dir, false, 100, &mut names), 0);
        assert_eq!(names, ["alpha-9", "alpha-10", "done.png", "zeta"]);
        // at most max, taken in the folder's order, then sorted
        assert_eq!(list_names(&dir, true, 2, &mut names), 0);
        assert_eq!(names.len(), 2);
        assert_eq!(list_names(&format!("{dir}/missing"), true, 2, &mut names), -1);
        assert!(names.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn small_files_and_folders() {
        let dir = scratch("quiet");
        let path = format!("{dir}/note.txt");
        assert_eq!(write_quiet(&path, "hello\n"), 0);
        assert_eq!(std::fs::read(&path).unwrap(), b"hello\n");
        assert!(!exists(&format!("{path}.tmp")));
        assert_eq!(write_quiet(&path, ""), 0);
        assert_eq!(std::fs::read(&path).unwrap(), b"");
        assert_eq!(write_quiet(&format!("{dir}/no/such/dir/x"), "x"), -1);
        // remove_work: a folder with a bg/ of files, and no other folder inside
        std::fs::create_dir_all(format!("{dir}/work/bg")).unwrap();
        std::fs::write(format!("{dir}/work/p1.qpg"), b"a").unwrap();
        std::fs::write(format!("{dir}/work/bg/000001.png"), b"b").unwrap();
        remove_work(format!("{dir}/work"));
        assert!(!exists(&format!("{dir}/work")));
        // check_file reads the size of what is no file
        let (mut w, mut h) = (0, 0);
        let mut why = "";
        assert_eq!(check_file(&format!("{dir}/work"), 100, &mut w, &mut h, &mut why), -1);
        assert_eq!(why, "the file does not read");
        std::fs::write(&path, vec![0u8; 101]).unwrap();
        assert_eq!(check_file(&path, 100, &mut w, &mut h, &mut why), -1);
        assert_eq!(why, "too big a file");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn hashes_of_names() {
        // FNV-1a of "Name", the last part of the path
        let mut h: u64 = 14695981039346656037;
        for b in b"Name" {
            h = (h ^ *b as u64).wrapping_mul(1099511628211);
        }
        assert_eq!(folder_name_hash("/mnt/us/grimoire/import/Name"), h);
        assert_eq!(folder_name_hash("Name"), h);
        assert_ne!(folder_name_hash("/a/Name2"), h);
        assert_eq!(fnv(FNV0, b""), FNV0);
    }
}
