//! Your own fonts (2.1.0-beta.2, lane L10; DESIGN.md "Your own fonts"): everything around the check. Rust only.
//!
//! - the folder: grimoire/fonts/, made at start with a README.txt. `.ttf`, `.otf`, `.ttc` and `.otc` files in
//!   it and in its folders one level down, at most 200, read through fsview's walk (no link is followed).
//! - the cache: grimoire/fonts/.cache/<key>.pgm (the font's name drawn in itself), .no (the check refused the
//!   file), .checking (a check under way), .slow (a check that took too long, once) and .bad (the check's worker
//!   died on it). <key> is the FNV-1a hash of the file's path under the folder, its size and its time, as a
//!   template's preview is keyed, so a file replaced by another is checked afresh. Written by the background
//!   job only; deleting it loses nothing, and has every font checked again.
//! - the verdicts (`VerdictDir`): <key>.font (fontcheck.rs), what the check found of a file. Where the build has
//!   a folder of its own outside the storage (the Kindle: /var/local/grimoire; elsewhere GRIMOIRE_STATE), they
//!   are kept there, in a folder for each data root, and nothing under grimoire/fonts decides whether a font is
//!   read. A build without such a folder keeps them in the cache, beside the previews. Either way a verdict
//!   counts only with its companion in the cache: a pass with its .pgm, a refusal with its .no.
//! - the job (`bg_font_check`, BgKind::FontCheck): the writer. It reads each unchecked file whole, hands the
//!   bytes to the sandboxed worker, validates what comes back and writes the verdict. It parses nothing.
//! - the UI's side: the list the Font sheet shows (`userfonts_scan`), the load of a font that passed
//!   (`userfonts_apply`: the bytes read must hash to the verdict's, or the file is checked again), the memory
//!   caps, and the crash guard.
//! - the crash guard: grimoire/.font-inuse holds the key of a font from before its first use in a session
//!   until a screen drawn with it has reached the panel. Found at a start, the last start never got that far:
//!   one strike, and this start stays in Terminess; the start after tries the font once more; a second strike
//!   keeps Terminess until the file changes. It is not a cache: the UI writes it.
//!
//! grimoire.log goes into public bug reports: no file name and no font's family is ever logged here, only
//! counts, sizes, timings and reasons.

use crate::fontcheck::*;
use crate::fsview::*;
use crate::prelude::*;
use std::cell::RefCell;
use std::collections::HashSet;
use std::ffi::CString;
use std::io::Read;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::fs::MetadataExt;
use std::rc::Rc;

/// A font's path under grimoire/fonts is at most this long (settings.ini keeps it: font=file:<path>).
pub const USERFONT_REL_MAX: usize = 200;
/// Below this much available memory a custom font is not loaded (it is logged, and Terminess draws).
pub const USERFONT_MEM_FLOOR: u64 = 100 << 20;
/// A preview file is at most this long (the name at the text size, 20 sizes wide).
const PREVIEW_MAX_BYTES: u64 = 4 << 20;
/// Folders of the fonts folder looked into.
const FOLDERS_MAX: usize = 64;

// ---- where things are ----

fn fonts_dir() -> String {
    format!("{}/fonts", store_root())
}
fn cache_dir() -> String {
    format!("{}/fonts/.cache", store_root())
}
fn cache_file(key: u64, ext: &str) -> String {
    format!("{}/fonts/.cache/{:016x}.{}", store_root(), key, ext)
}
fn guard_path() -> String {
    format!("{}/.font-inuse", store_root())
}

/// At startup: grimoire/fonts/ with its README.txt, when missing (as templates_init makes the other folders).
pub fn userfonts_init() {
    let dir = fonts_dir();
    if mkdir_p(&dir) != 0 {
        return;
    }
    let parts = [
        // TRANSLATORS: grimoire/fonts/README.txt, a plain text file Grimoire writes once (in the language in
        // use then) for the owner to read on a computer: its title, then its two paragraphs. No room limit.
        tr("Fonts for Grimoire"),
        // TRANSLATORS: grimoire/fonts/README.txt's first paragraph. "Settings", "Font", "Your fonts" and
        // "Classic" name Grimoire's own sheet, chip, heading and style: as they are translated there. ".ttf"
        // and ".otf" are kinds of file: keep them. Terminess is the built-in font's name: kept.
        tr("Copy font files here to read Grimoire in a font of your own: .ttf or .otf files. In Settings, with the \
Classic style, tap Font and pick it under Your fonts. Your font draws Grimoire's own words (menus, names, the \
keyboard); your pages, their PDFs and the pages you share never change with it. A letter your font lacks \
comes from Terminess."),
        // TRANSLATORS: grimoire/fonts/README.txt's second paragraph. ".cache" is a folder's name: keep it.
        tr("Copy them in over USB while Grimoire is closed. Grimoire checks each font in the background before it \
uses it, and keeps what it found in the .cache folder. Your fonts stay on this Kindle: use fonts you have the \
right to use. If a font ever goes wrong, delete its file here and Grimoire is back in Terminess."),
    ];
    crate::bgwork_template::write_readme(&dir, &parts, "fonts");
}

// ---- the files ----

/// What a font file is by its name: a font Grimoire may take, a web font it does not ("Use .ttf or .otf."),
/// or no font at all.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NameKind {
    Font,
    Web,
    Other,
}

fn name_kind(name: &str) -> NameKind {
    let Some(dot) = name.rfind('.') else { return NameKind::Other };
    if name.starts_with('.') {
        return NameKind::Other;
    }
    let ext = name[dot + 1..].to_ascii_lowercase();
    match ext.as_str() {
        "ttf" | "otf" | "ttc" | "otc" => NameKind::Font,
        "woff" | "woff2" => NameKind::Web,
        _ => NameKind::Other,
    }
}

/// Whether a path under the fonts folder is one settings.ini and a job can carry: text without control
/// characters, at most two names, short enough.
pub fn userfont_rel_ok(rel: &str) -> bool {
    !rel.is_empty()
        && rel.len() <= USERFONT_REL_MAX
        && rel.split('/').count() <= 2
        && !rel.chars().any(|c| c.is_control())
        && rel.split('/').all(|n| !n.is_empty() && !n.starts_with('.') && n != "..")
        && name_kind(rel.rsplit('/').next().unwrap_or("")) != NameKind::Other
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Listed {
    rel: String,
    size: u64,
    mtime: i64,
    key: u64,
}

/// A file's key: the FNV-1a hash of its path under the folder, its size and its time.
fn key_of(rel: &str, size: u64, mtime: i64) -> u64 {
    let mut h: u64 = 14695981039346656037;
    let tail = format!("/{}/{}", size as i64, mtime);
    for b in rel.bytes().chain(tail.bytes()) {
        h = (h ^ b as u64).wrapping_mul(1099511628211);
    }
    h
}

fn storage() -> Option<FsStorage> {
    FsStorage::open(&fonts_dir(), &store_root()).ok()
}

/// The names of one folder of the fonts folder: its font files (with their stats), and its folders.
fn list_folder(st: &FsStorage, at: &StoragePath, files: &mut Vec<Listed>, folders: Option<&mut Vec<String>>, left_out: &mut usize) {
    let Ok(mut l) = FsListing::start(st, at, false) else { return };
    while !l.step(1000) {}
    let n = l.entries.len();
    l.stat(0..n, u32::MAX);
    *left_out += l.too_long;
    let mut dirs = Vec::new();
    for e in &l.entries {
        let Ok(name) = std::str::from_utf8(&e.name) else {
            *left_out += 1;
            continue;
        };
        let rel = if at.is_root() { name.to_string() } else { format!("{}/{}", at.to_string_lossy(), name) };
        match (e.kind, e.stat) {
            (FsKind::Dir, _) if at.is_root() => dirs.push(name.to_string()),
            (FsKind::File, Some(s)) if name_kind(name) != NameKind::Other => {
                if userfont_rel_ok(&rel) {
                    files.push(Listed { key: key_of(&rel, s.size, s.mtime), rel, size: s.size, mtime: s.mtime });
                } else {
                    *left_out += 1;
                }
            }
            _ => {}
        }
    }
    if let Some(f) = folders {
        *f = dirs;
    }
}

/// The font files of grimoire/fonts and of its folders one level down, in name order, at most
/// FONT_FILES_MAX; and how many were left out (too many, or a name that cannot be carried).
fn list_fonts() -> (Vec<Listed>, usize) {
    let Some(st) = storage() else { return (Vec::new(), 0) };
    let (mut files, mut folders, mut left_out) = (Vec::new(), Vec::new(), 0);
    list_folder(&st, &StoragePath::root(), &mut files, Some(&mut folders), &mut left_out);
    folders.sort_by(|a, b| name_cmp(a, b));
    for d in folders.iter().take(FOLDERS_MAX) {
        if files.len() > FONT_FILES_MAX * 4 {
            break;
        }
        if let Some(sp) = StoragePath::new(d.as_bytes()) {
            list_folder(&st, &sp, &mut files, None, &mut left_out);
        }
    }
    files.sort_by(|a, b| name_cmp(&a.rel, &b.rel));
    if files.len() > FONT_FILES_MAX {
        left_out += files.len() - FONT_FILES_MAX;
        files.truncate(FONT_FILES_MAX);
    }
    (files, left_out)
}

/// A whole font file through the walk (no link followed), at most FONT_FILE_MAX bytes: None when it does not
/// read, is no regular file, or is not the file that was listed (its size or time changed since).
fn read_listed(st: &FsStorage, l: &Listed) -> Option<Vec<u8>> {
    if l.size == 0 || l.size > FONT_FILE_MAX as u64 {
        return None;
    }
    let sp = StoragePath::new(l.rel.as_bytes())?;
    let before = st.stat(&sp, false).ok()?;
    if before.size != l.size || before.mtime != l.mtime {
        return None;
    }
    let fd = st.open_file(&sp, false).ok()?;
    let mut f = std::fs::File::from(fd);
    let mut buf: Vec<u8> = Vec::new();
    buf.try_reserve_exact(l.size as usize + 1).ok()?;
    f.by_ref().take(l.size + 1).read_to_end(&mut buf).ok()?;
    let after = st.stat(&sp, false).ok()?;
    (buf.len() as u64 == l.size && after.same(&before)).then_some(buf)
}

/// A small file of the cache, at most `max` bytes (a longer one is not Grimoire's).
fn read_small(path: &str, max: u64) -> Option<Vec<u8>> {
    let md = std::fs::symlink_metadata(path).ok()?;
    if !md.is_file() || md.len() > max {
        return None;
    }
    let mut buf = Vec::new();
    std::fs::File::open(path).ok()?.take(max + 1).read_to_end(&mut buf).ok()?;
    (buf.len() as u64 <= max).then_some(buf)
}

// ---- where the check's results are kept ----

/// Grimoire's own folder on the Kindle, outside the storage: a constant, which no environment variable moves.
#[cfg(feature = "kindle")]
const STATE_BASE: &str = "/var/local/grimoire";
/// The room that folder's file system must have free before a font is checked: bytes, and files (inodes).
/// Placeholders until the Scribe's figures are in (docs/TESTING.md).
const STATE_FREE_FLOOR: u64 = 4 << 20;
const STATE_FILES_FLOOR: u64 = 512;
/// The folders of other data roots looked at in a job, and how long a `root` file is at most.
const STATE_ROOTS_MAX: usize = 64;
const STATE_ROOT_BYTES: usize = 1024;

const NOT_THERE: &str = "it is not there";
const NOT_MADE: &str = "it cannot be made";
const NOT_OWN: &str = "it is not Grimoire's own";

/// The folder of Grimoire's own that the verdicts are kept under: the Kindle's always; in any other build the
/// absolute path GRIMOIRE_STATE names. None: this build has no such folder, and the verdicts stay beside the
/// previews in fonts/.cache, by design (a platform kept outside this crate may hold its data on a file system
/// without owners and modes).
fn state_base() -> Option<String> {
    #[cfg(feature = "kindle")]
    {
        Some(STATE_BASE.to_string())
    }
    #[cfg(not(feature = "kindle"))]
    {
        std::env::var("GRIMOIRE_STATE").ok().filter(|p| p.starts_with('/'))
    }
}

/// A data root's folder in Grimoire's own: the FNV-1a 64 hash of its path without trailing slashes, 16 hex
/// digits. Each data root has one, so a second root on one device keeps its verdicts apart.
fn root_tag(root: &str) -> String {
    let mut h: u64 = 14695981039346656037;
    for b in root.trim_end_matches('/').bytes() {
        h = (h ^ b as u64).wrapping_mul(1099511628211);
    }
    format!("{h:016x}")
}

/// Whether a file system with this much free (bytes, and files of `files` in all) has room for what a check
/// finds. One that counts no files (`files` 0) is not refused for it.
fn enough_room(free_bytes: u64, files: u64, files_free: u64) -> bool {
    free_bytes >= STATE_FREE_FLOOR && (files == 0 || files_free >= STATE_FILES_FLOOR)
}

/// Whether the folder behind a descriptor is this user's and no one else may write it.
fn own_dir(fd: &OwnedFd) -> bool {
    // SAFETY: st is written by fstat before it is read; geteuid takes no arguments.
    unsafe {
        let mut st: libc::stat = std::mem::zeroed();
        libc::fstat(fd.as_raw_fd(), &mut st) == 0 && (st.st_mode & libc::S_IFMT) == libc::S_IFDIR && st.st_uid == libc::geteuid() && st.st_mode & 0o022 == 0
    }
}

/// The folder `name` of the folder `at`, by descriptor: made first when `make` (mode 0700; one that is there is
/// fine), opened without following a link, and held to `own_dir`. Or why not.
fn own_dir_at(at: &OwnedFd, name: &str, make: bool) -> Result<OwnedFd, &'static str> {
    let c = CString::new(name).map_err(|_| NOT_MADE)?;
    // SAFETY: mkdirat and openat on a descriptor we hold and a NUL-terminated name; the descriptor openat gives
    // is owned from here on.
    unsafe {
        if make && libc::mkdirat(at.as_raw_fd(), c.as_ptr(), 0o700 as libc::mode_t) != 0 && errno() != libc::EEXIST {
            return Err(NOT_MADE);
        }
        let fd = libc::openat(at.as_raw_fd(), c.as_ptr(), libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC);
        if fd < 0 {
            return Err(if errno() == libc::ENOENT { NOT_THERE } else { NOT_OWN });
        }
        let fd = OwnedFd::from_raw_fd(fd);
        if own_dir(&fd) {
            Ok(fd)
        } else {
            Err(NOT_OWN)
        }
    }
}

/// A folder by its path (links in the path are followed unless `nofollow`, which holds for its last name).
fn open_dir(path: &str, nofollow: bool) -> Option<OwnedFd> {
    let c = CString::new(path).ok()?;
    let flags = libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | if nofollow { libc::O_NOFOLLOW } else { 0 };
    // SAFETY: open reads the NUL-terminated path we own; the descriptor is owned from here on.
    unsafe {
        let fd = libc::open(c.as_ptr(), flags);
        (fd >= 0).then(|| OwnedFd::from_raw_fd(fd))
    }
}

/// The names in a folder we hold a descriptor of (never `.` or `..`), at most `cap` of them.
fn names_at(dir: &OwnedFd, cap: usize) -> Vec<String> {
    let mut out = Vec::new();
    // SAFETY: the duplicate is the stream's from fdopendir on (closedir closes it, never `dir`); each entry is
    // read before the next readdir; d_name is NUL-terminated.
    unsafe {
        let fd = libc::fcntl(dir.as_raw_fd(), libc::F_DUPFD_CLOEXEC, 0);
        if fd < 0 {
            return out;
        }
        let d = libc::fdopendir(fd);
        if d.is_null() {
            libc::close(fd);
            return out;
        }
        // (the duplicate shares the folder's place with every earlier look)
        libc::rewinddir(d);
        while out.len() < cap {
            let e = libc::readdir(d);
            if e.is_null() {
                break;
            }
            if let Ok(n) = std::ffi::CStr::from_ptr((*e).d_name.as_ptr()).to_str() {
                if n != "." && n != ".." {
                    out.push(n.to_string());
                }
            }
        }
        libc::closedir(d);
    }
    out
}

/// A small file of a folder we hold a descriptor of, at most `max` bytes, never through a link: None when it is
/// no regular file, is longer, does not read, or (`own`) is not this user's.
fn read_at(dir: &OwnedFd, name: &str, max: usize, own: bool) -> Option<Vec<u8>> {
    let c = CString::new(name).ok()?;
    // SAFETY: openat on a descriptor we hold and a NUL-terminated name; the descriptor it gives is the File's.
    let f = unsafe {
        let fd = libc::openat(dir.as_raw_fd(), c.as_ptr(), libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC);
        if fd < 0 {
            return None;
        }
        std::fs::File::from_raw_fd(fd)
    };
    let md = f.metadata().ok()?;
    // SAFETY: geteuid takes no arguments.
    if !md.is_file() || md.len() > max as u64 || (own && md.uid() != unsafe { libc::geteuid() }) {
        return None;
    }
    let mut buf = Vec::new();
    f.take(max as u64 + 1).read_to_end(&mut buf).ok()?;
    (buf.len() <= max).then_some(buf)
}

/// Writes the file `name` of a folder we hold a descriptor of, whole: `<name>.tmp` is opened as
/// util::open_rewrite_nofollow opens a file (never through a link; a regular file with one name, and only then
/// emptied), made with `mode` when missing, written, synced and renamed onto `name`. False when a step fails:
/// the `.tmp` is then unlinked and `name` is as it was.
fn write_at(dir: &OwnedFd, name: &str, data: &[u8], mode: u32) -> bool {
    use std::io::Write;
    let (Ok(to), Ok(tmp)) = (CString::new(name), CString::new(format!("{name}.tmp"))) else { return false };
    let d = dir.as_raw_fd();
    // SAFETY: openat, fstat, ftruncate, fsync, renameat and unlinkat on a descriptor we hold, NUL-terminated
    // names and a struct we own; the descriptor openat gives is the File's, closed before the rename.
    unsafe {
        let fd = libc::openat(d, tmp.as_ptr(), libc::O_WRONLY | libc::O_CREAT | libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC, mode as libc::c_uint);
        let mut ok = fd >= 0;
        if ok {
            let mut f = std::fs::File::from_raw_fd(fd);
            let mut st: libc::stat = std::mem::zeroed();
            ok = libc::fstat(fd, &mut st) == 0 && (st.st_mode & libc::S_IFMT) == libc::S_IFREG && st.st_nlink == 1 && libc::ftruncate(fd, 0) == 0;
            ok = ok && f.write_all(data).is_ok() && libc::fsync(fd) == 0;
        }
        if ok && libc::renameat(d, tmp.as_ptr(), d, to.as_ptr()) == 0 {
            return true;
        }
        libc::unlinkat(d, tmp.as_ptr(), 0);
        false
    }
}

fn unlink_at(dir: &OwnedFd, name: &str, folder: bool) -> bool {
    let Ok(c) = CString::new(name) else { return false };
    // SAFETY: unlinkat on a descriptor we hold and a NUL-terminated name.
    unsafe { libc::unlinkat(dir.as_raw_fd(), c.as_ptr(), if folder { libc::AT_REMOVEDIR } else { 0 }) == 0 }
}

/// The folder the verdicts are in, held by descriptor: every verdict is read and written from it, never by a
/// path. Two modes, one code path: Grimoire's own folder where the build has one (`state_base`), the cache
/// where it has none.
struct VerdictDir {
    fd: OwnedFd,
    /// in Grimoire's own folder: that folder, and this data root's in it (the job's upkeep works by them)
    own: Option<(OwnedFd, OwnedFd)>,
}

impl VerdictDir {
    /// Opens the folder, or says why not. In Grimoire's own folder the walk goes name by name from the base's
    /// parent, which is the system's: the base, this data root's folder (`root_tag`) and its `fonts`, each made
    /// when `make` (the job alone asks; the UI never writes there), opened without following a link, and held to
    /// be this user's with no write bit for anyone else. In the cache: the folder made when `make`, as the job
    /// always made it, then opened; no owner and no mode is asked of it.
    fn open(make: bool) -> Result<VerdictDir, &'static str> {
        let Some(base) = state_base() else {
            let cache = cache_dir();
            if make && (!no_link_below(&store_root(), &cache) || mkdir_p(&cache) != 0) {
                return Err(NOT_MADE);
            }
            return Ok(VerdictDir { fd: open_dir(&cache, true).ok_or(NOT_THERE)?, own: None });
        };
        let base = base.trim_end_matches('/');
        let Some((parent, name)) = base.rsplit_once('/').filter(|(_, n)| !n.is_empty() && *n != "." && *n != "..") else { return Err(NOT_MADE) };
        let parent = open_dir(if parent.is_empty() { "/" } else { parent }, false).ok_or(NOT_THERE)?;
        // (never util::mkdir_p here: it takes any name that is there as made, and makes folders others can read)
        let base = own_dir_at(&parent, name, make)?;
        let tag = own_dir_at(&base, &root_tag(&store_root()), make)?;
        let fd = own_dir_at(&tag, "fonts", make)?;
        Ok(VerdictDir { fd, own: Some((base, tag)) })
    }

    /// Whether this is Grimoire's own folder (not the cache).
    fn private(&self) -> bool {
        self.own.is_some()
    }

    /// The verdict kept under `key`, read strictly (in Grimoire's own folder it must be this user's file too).
    fn read(&self, key: u64) -> Option<Verdict> {
        Verdict::parse(&read_at(&self.fd, &format!("{key:016x}.font"), VERDICT_MAX, self.private())?)
    }

    fn write(&self, key: u64, text: &[u8]) -> bool {
        write_at(&self.fd, &format!("{key:016x}.font"), text, if self.private() { 0o600 } else { 0o644 })
    }

    fn remove(&self, key: u64) {
        unlink_at(&self.fd, &format!("{key:016x}.font"), false);
    }

    /// The names in the folder (capped as `cache_names` is).
    fn names(&self) -> HashSet<String> {
        names_at(&self.fd, 4 * FONT_FILES_MAX + 64).into_iter().collect()
    }

    /// What is there for files that are gone (a name whose part before the first dot is not in `keep`) goes,
    /// and so does a `.tmp` a cut-off write left.
    fn sweep(&self, keep: &HashSet<String>) {
        for n in self.names() {
            if !keep.contains(n.split('.').next().unwrap_or("")) || n.ends_with(".tmp") {
                unlink_at(&self.fd, &n, false);
            }
        }
    }

    /// Whether what a check finds can be kept: the cache is not asked (a write there says so itself);
    /// Grimoire's own folder must have `enough_room`. A file system that does not answer counts as roomy.
    /// Tests (not the Kindle's build) set the free bytes with GRIMOIRE_STATE_FREE_BYTES.
    fn room(&self) -> bool {
        if !self.private() {
            return true;
        }
        // SAFETY: sv is written by fstatvfs before it is read.
        let mut sv: libc::statvfs = unsafe { std::mem::zeroed() };
        if unsafe { libc::fstatvfs(self.fd.as_raw_fd(), &mut sv) } != 0 {
            return true;
        }
        #[allow(unused_mut)]
        let mut free = (sv.f_bavail as u64).saturating_mul(sv.f_frsize as u64);
        #[cfg(not(feature = "kindle"))]
        if let Some(v) = std::env::var("GRIMOIRE_STATE_FREE_BYTES").ok().and_then(|v| v.parse::<u64>().ok()) {
            free = v;
        }
        enough_room(free, sv.f_files as u64, sv.f_favail as u64)
    }
}

/// The verdict kept for the file of this key and size. It counts only with its companion in fonts/.cache: a
/// pass with its preview (`<key>.pgm`), a refusal with `<key>.no`. So deleting grimoire/fonts/.cache has every
/// font checked again wherever the verdicts are kept, and a companion alone never passes a font.
fn verdict_for(vd: &VerdictDir, key: u64, size: u64) -> Option<Verdict> {
    let v = vd.read(key).filter(|v| v.size == size)?;
    let companion = cache_file(key, if matches!(v.what, VerdictKind::Passed(_)) { "pgm" } else { "no" });
    std::fs::symlink_metadata(companion).is_ok_and(|m| m.is_file()).then_some(v)
}

/// The names of the verdicts kept (none when their folder cannot be opened: every font is then "checking",
/// and the job says why).
fn verdict_names() -> HashSet<String> {
    VerdictDir::open(false).map(|vd| vd.names()).unwrap_or_default()
}

/// The job's upkeep of Grimoire's own folder: this data root's folder names its root (the file `root`: the
/// path and a newline), and the folder of a data root that is gone is removed, so the folder holds what the
/// roots that exist need and no more. Another root's folder goes only when its `root` reads (this user's
/// file, an absolute path, no control character), nothing is at that path, and the folder holds nothing but
/// what Grimoire put there; every step is by descriptor, never by a path and never through a link.
fn own_upkeep(base: &OwnedFd, tag: &OwnedFd) {
    let root = store_root();
    let root = root.trim_end_matches('/');
    let line = format!("{root}\n");
    if read_at(tag, "root", STATE_ROOT_BYTES, true).as_deref() != Some(line.as_bytes()) {
        write_at(tag, "root", line.as_bytes(), 0o600);
    }
    let mine = root_tag(root);
    let mut gone = 0;
    for name in names_at(base, STATE_ROOTS_MAX) {
        if name == mine || name.len() != 16 || !name.bytes().all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c)) {
            continue;
        }
        let Ok(other) = own_dir_at(base, &name, false) else { continue };
        let Some(text) = read_at(&other, "root", STATE_ROOT_BYTES, true).and_then(|b| String::from_utf8(b).ok()) else { continue };
        let path = text.strip_suffix('\n').unwrap_or(&text);
        if !path.starts_with('/') || path.chars().any(|c| c.is_control()) {
            continue;
        }
        if !matches!(std::fs::symlink_metadata(path), Err(e) if e.raw_os_error() == Some(libc::ENOENT)) {
            continue;
        }
        let inside = names_at(&other, 8);
        if inside.iter().any(|n| !matches!(n.as_str(), "root" | "root.tmp" | "fonts")) {
            continue;
        }
        if inside.iter().any(|n| n == "fonts") {
            let Ok(fonts) = own_dir_at(&other, "fonts", false) else { continue };
            for f in names_at(&fonts, 4 * FONT_FILES_MAX + 64) {
                unlink_at(&fonts, &f, false);
            }
            drop(fonts);
            if !unlink_at(&other, "fonts", true) {
                continue;
            }
        }
        unlink_at(&other, "root.tmp", false);
        unlink_at(&other, "root", false);
        drop(other);
        if unlink_at(base, &name, true) {
            gone += 1;
        }
    }
    if gone > 0 {
        qlog!("fonts: %d data folder(s) are gone: what was kept for them is removed from Grimoire's own folder", gone);
    }
}

// ---- the crash guard: grimoire/.font-inuse ----

/// One font's mark in the guard: its key, the starts that did not finish their first screen with it, and
/// whether it is in use now and has not yet been seen on a screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GuardEntry {
    pub key: u64,
    pub strikes: u32,
    pub armed: bool,
}

/// Marks kept at most (the font in use, its bold, the Font sheet's sample, and the struck ones).
const GUARD_MAX: usize = 8;
/// Strikes after which a font stays out until its file changes.
pub const GUARD_STRIKES: u32 = 2;

/// The guard file's text.
pub fn guard_text(e: &[GuardEntry]) -> String {
    let mut s = String::from("grimoire-font-guard 1\n");
    for g in e {
        s.push_str(&format!("{:016x} {} {}\n", g.key, g.strikes, g.armed as i32));
    }
    s
}

/// Reads the guard file's bytes, strictly; None for anything that is not one.
pub fn guard_parse(text: &[u8]) -> Option<Vec<GuardEntry>> {
    if text.len() > 1024 {
        return None;
    }
    let text = std::str::from_utf8(text).ok()?;
    let mut lines = text.split('\n');
    if lines.next()? != "grimoire-font-guard 1" {
        return None;
    }
    let mut out = Vec::new();
    for l in lines {
        if l.is_empty() {
            continue;
        }
        let p: Vec<&str> = l.split(' ').collect();
        if p.len() != 3 || p[0].len() != 16 || !p[0].bytes().all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c)) {
            return None;
        }
        let strikes = match p[1] {
            "0" => 0,
            "1" => 1,
            "2" => 2,
            _ => return None,
        };
        let armed = match p[2] {
            "0" => false,
            "1" => true,
            _ => return None,
        };
        if out.len() == GUARD_MAX {
            return None;
        }
        out.push(GuardEntry { key: u64::from_str_radix(p[0], 16).ok()?, strikes, armed });
    }
    Some(out)
}

/// What a start makes of the marks it finds: a mark still armed means the last start with that font never
/// finished its first screen, which is a strike. Returns the keys struck now: this start draws none of them
/// (when the font in use is one, this start is in Terminess).
pub fn guard_start(e: &mut Vec<GuardEntry>) -> Vec<u64> {
    let mut struck = Vec::new();
    for g in e.iter_mut() {
        if g.armed {
            g.armed = false;
            g.strikes = (g.strikes + 1).min(GUARD_STRIKES);
            struck.push(g.key);
        }
    }
    e.retain(|g| g.strikes > 0);
    struck
}

/// Before a font's first use in a session: its mark is armed (kept with its strikes, if it has any).
pub fn guard_arm(e: &mut Vec<GuardEntry>, key: u64) {
    match e.iter_mut().find(|g| g.key == key) {
        Some(g) => g.armed = true,
        None => {
            if e.len() >= GUARD_MAX {
                // (the oldest mark that is not in use gives way)
                if let Some(i) = e.iter().position(|g| !g.armed) {
                    e.remove(i);
                }
            }
            if e.len() < GUARD_MAX {
                e.push(GuardEntry { key, strikes: 0, armed: true });
            }
        }
    }
}

/// After a screen drawn with the armed fonts reached the panel: their marks go, strikes and all (a font that
/// got through its second try is as good as new).
pub fn guard_settle(e: &mut Vec<GuardEntry>) {
    e.retain(|g| !g.armed);
}

/// Whether a font stays out: two strikes, until its file changes (which gives it another key).
pub fn guard_refuses(e: &[GuardEntry], key: u64) -> bool {
    e.iter().any(|g| g.key == key && g.strikes >= GUARD_STRIKES)
}

#[derive(Default)]
struct State {
    /// the guard's marks, read once (at the first use)
    guard: Option<Vec<GuardEntry>>,
    /// the keys struck at this start: not drawn in this session
    struck: Vec<u64>,
    /// the keys armed and not yet seen on a screen, and font_own_drawn() when they were armed
    armed: Vec<u64>,
    armed_at: u32,
    /// the keys that reached a screen in this session: not armed again
    settled: Vec<u64>,
    /// no worker can be had in this session: no more fonts are checked, and the unchecked say so
    broken: bool,
    /// the font in use: its path under the folder, and its file's key
    in_use: Option<(String, u64)>,
    /// its family's name as the check read it (the file's own name when it gave none): Settings' Font chip
    in_use_name: String,
    /// the font settings.ini asks for could not be loaded yet (its check is not done): tried again when a
    /// check ends
    waiting: bool,
    /// a check left files for later (still being copied): the folder is looked at again then
    later_ms: Option<u32>,
    /// the sample the Font sheet holds: its path
    sample: Option<String>,
    /// the guard keys of the files the font in use and the sample were loaded from (a font let go before any
    /// screen showed it gives its mark back)
    in_use_keys: Vec<u64>,
    sample_keys: Vec<u64>,
    /// why the font asked for was not used, as last logged (said once, not at every look)
    said: Option<LoadErr>,
    /// checks in a row that ended with no verdict for the font asked for (3: files left for later, 1: failed,
    /// or 0 with the font still unchecked), and when the next check may be queued (backoff); at MAX_FAILS the
    /// session stops checking, as with no worker
    fails: u32,
    retry_ms: Option<u32>,
    /// the keys whose verdict did not read, queued for a new check once (the sheet would else ask again)
    rechecked: Vec<u64>,
    /// the dpi the last check was asked for (a recheck queued from a row has none of its own)
    dpi: f32,
    /// userfonts_job_done is applying the font: no check is queued by that (it decides what comes next)
    holding: bool,
}

/// Checks in a row that end with no verdict before the session stops asking (H8: no check forked 29 times a
/// second), and how long to wait after the first, second, third and fourth.
const MAX_FAILS: u32 = 5;
const RETRY_MS: [u32; 4] = [3_000, 15_000, 60_000, 60_000];
/// How long after one watchdog timeout a font is checked again (a timeout right after a wake is the clock's
/// jump, not a slow font): one retry, then the verdict stands.
const SLOW_RETRY_S: i64 = 60;

thread_local! {
    static S: RefCell<State> = RefCell::new(State::default());
}

fn with<R>(f: impl FnOnce(&mut State) -> R) -> R {
    S.with(|s| f(&mut s.borrow_mut()))
}

fn guard_load(st: &mut State) -> &mut Vec<GuardEntry> {
    if st.guard.is_none() {
        let path = guard_path();
        let read = read_small(&path, 1024);
        let mut marks = match read.as_deref().map(guard_parse) {
            Some(Some(m)) => m,
            Some(None) => {
                qlog!("font: the guard file does not read; starting it afresh");
                Vec::new()
            }
            None => Vec::new(),
        };
        st.struck = guard_start(&mut marks);
        if !st.struck.is_empty() {
            qlog!(
                "font: the last start did not finish its first screen with your font (%d mark(s)): not drawn at this start",
                st.struck.len() as i32
            );
        }
        if read.is_some() {
            guard_save(&marks);
        }
        st.guard = Some(marks);
    }
    st.guard.as_mut().unwrap()
}

/// Writes the marks (none: the file goes). False when the storage would not take them.
fn guard_save(marks: &[GuardEntry]) -> bool {
    let path = guard_path();
    if marks.is_empty() {
        return match std::fs::remove_file(&path) {
            Ok(()) => true,
            Err(e) => e.kind() == std::io::ErrorKind::NotFound,
        };
    }
    write_file_atomic(&path, guard_text(marks).as_bytes()) == 0
}

/// Whether the guard keeps this font out: two strikes, or struck at this very start.
fn guard_out(key: u64) -> bool {
    with(|st| {
        let refused = guard_refuses(guard_load(st), key);
        refused || st.struck.contains(&key)
    })
}

/// Arms the guard for these fonts before their first use in this session. False when the mark could not be
/// written: the font is then not used.
fn guard_arm_keys(keys: &[u64]) -> bool {
    with(|st| {
        let new: Vec<u64> = keys.iter().copied().filter(|k| !st.settled.contains(k) && !st.armed.contains(k)).collect();
        if new.is_empty() {
            return true;
        }
        let marks = guard_load(st);
        let before = marks.clone();
        for &k in &new {
            guard_arm(marks, k);
        }
        if !new.iter().all(|k| marks.iter().any(|g| g.key == *k && g.armed)) || !guard_save(marks) {
            *marks = before;
            return false;
        }
        if st.armed.is_empty() {
            st.armed_at = font_own_drawn();
        }
        st.armed.extend(new);
        true
    })
}

/// A font that was armed and then not used after all (it did not load): its mark is let go, strikes kept.
fn guard_disarm_keys(keys: &[u64]) {
    with(|st| {
        if !keys.iter().any(|k| st.armed.contains(k)) {
            return;
        }
        st.armed.retain(|k| !keys.contains(k));
        let marks = guard_load(st);
        for g in marks.iter_mut().filter(|g| keys.contains(&g.key)) {
            g.armed = false;
        }
        marks.retain(|g| g.armed || g.strikes > 0);
        let marks = marks.clone();
        guard_save(&marks);
    });
}

/// After the panel was given a screen: if a custom font drew on it since its mark was armed, the mark goes.
/// (Called after every present: it costs a comparison when nothing is armed.) A mark the storage would not let
/// go stays armed in memory too, and is tried again after the next screen a custom font draws on.
pub fn userfonts_screen_drawn() {
    with(|st| {
        if st.armed.is_empty() || font_own_drawn() == st.armed_at {
            return;
        }
        let marks = guard_load(st);
        let before = marks.clone();
        guard_settle(marks);
        if !guard_save(marks) {
            *marks = before;
            st.armed_at = font_own_drawn();
            return;
        }
        let armed = std::mem::take(&mut st.armed);
        st.settled.extend(armed);
    });
}

// ---- the list the Font sheet shows ----

/// Why a row cannot be used: the Font sheet says it on the row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FontWhy {
    /// "Unreadable font."
    Unreadable,
    /// "Too big (over 24 MB)."
    TooBig,
    /// "No Latin letters."
    NoLatin,
    /// "Use .ttf or .otf."
    Format,
    /// "Took too long to read."
    Slow,
    /// "File missing." (the font settings.ini names, gone from the folder)
    Missing,
}

impl FontWhy {
    /// its name for the runner's probes
    pub fn name(self) -> &'static str {
        match self {
            FontWhy::Unreadable => "unreadable",
            FontWhy::TooBig => "toobig",
            FontWhy::NoLatin => "nolatin",
            FontWhy::Format => "format",
            FontWhy::Slow => "slow",
            FontWhy::Missing => "missing",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UserFontState {
    /// not checked yet: the background job has it, or will
    Checking,
    /// checked: its verdict is there, not read yet (userfont_resolve reads it)
    Checked,
    /// it passed: what the check learned, and the hash its bytes must have
    Ready(Rc<FaceFacts>, [u8; 32]),
    No(FontWhy),
}

/// A font file of the folder, as the Font sheet lists it.
#[derive(Clone, Debug)]
pub struct UserFont {
    /// its path under grimoire/fonts
    pub rel: String,
    pub key: u64,
    pub size: u64,
    pub state: UserFontState,
    /// the check's preview (the font's name in itself), once userfont_preview read it
    preview: Canvas,
    preview_tried: bool,
}

impl UserFont {
    /// The file's name without its folder and its extension, as a row shows it.
    pub fn base(&self) -> &str {
        let name = self.rel.rsplit('/').next().unwrap_or(&self.rel);
        name.rfind('.').map_or(name, |d| &name[..d])
    }
    /// What the row's state is called by the runner's probes.
    pub fn state_name(&self) -> &'static str {
        match &self.state {
            UserFontState::Checking => "checking",
            UserFontState::Checked | UserFontState::Ready(..) => "ready",
            UserFontState::No(w) => w.name(),
        }
    }
    pub fn usable(&self) -> bool {
        matches!(self.state, UserFontState::Ready(..))
    }
    /// The family's name as the check read it (the file's own name when it gave none).
    pub fn family(&self) -> String {
        match &self.state {
            UserFontState::Ready(f, _) if !f.family.is_empty() => f.family.clone(),
            _ => self.base().to_string(),
        }
    }
}

/// The names in the cache folder: which keys have a preview or a `.no`, and which are marked bad.
fn cache_names() -> std::collections::HashSet<String> {
    let mut names = std::collections::HashSet::new();
    if let Ok(rd) = std::fs::read_dir(cache_dir()) {
        for e in rd.flatten().take(4 * FONT_FILES_MAX + 64) {
            if let Some(n) = e.file_name().to_str() {
                names.insert(n.to_string());
            }
        }
    }
    names
}

/// Whether a key has a verdict by the names alone: its `.font` among the verdicts kept, and its companion (the
/// preview, or `.no`) in the cache.
fn checked_by_names(key: u64, verdicts: &HashSet<String>, cache: &HashSet<String>) -> bool {
    verdicts.contains(&format!("{key:016x}.font")) && (cache.contains(&format!("{key:016x}.pgm")) || cache.contains(&format!("{key:016x}.no")))
}

fn state_from(l: &Listed, verdicts: &HashSet<String>, cache: &HashSet<String>) -> UserFontState {
    if name_kind(l.rel.rsplit('/').next().unwrap_or("")) == NameKind::Web {
        return UserFontState::No(FontWhy::Format);
    }
    if l.size > FONT_FILE_MAX as u64 {
        return UserFontState::No(FontWhy::TooBig);
    }
    if l.size == 0 || guard_out(l.key) {
        return UserFontState::No(FontWhy::Unreadable);
    }
    if checked_by_names(l.key, verdicts, cache) {
        return UserFontState::Checked;
    }
    if cache.contains(&format!("{:016x}.bad", l.key)) || with(|st| st.broken) {
        return UserFontState::No(FontWhy::Unreadable);
    }
    UserFontState::Checking
}

/// Reads a checked font's verdict, once: what the check found, or why not. A verdict that does not read (or is
/// of another file) leaves the font to be checked again.
pub fn userfont_resolve(f: &mut UserFont) {
    if f.state != UserFontState::Checked {
        return;
    }
    f.state = match VerdictDir::open(false).ok().and_then(|vd| verdict_for(&vd, f.key, f.size)) {
        Some(v) => match v.what {
            VerdictKind::Passed(facts) => UserFontState::Ready(Rc::new(facts), v.hash),
            VerdictKind::Refused(Refuse::Unreadable) => UserFontState::No(FontWhy::Unreadable),
            VerdictKind::Refused(Refuse::NoLatin) => UserFontState::No(FontWhy::NoLatin),
            VerdictKind::Refused(Refuse::Format) => UserFontState::No(FontWhy::Format),
            VerdictKind::Slow => UserFontState::No(FontWhy::Slow),
        },
        None => UserFontState::Checking,
    };
    // A verdict that does not read (a line a later version added, a cut file) is checked again, once: the row
    // would else say "Checking..." for good and the open sheet refresh at every look.
    if f.state == UserFontState::Checking && !with(|st| st.rechecked.contains(&f.key)) {
        let dpi = with(|st| {
            st.rechecked.push(f.key);
            st.dpi
        });
        queue_check("", dpi);
    }
}

fn queue_check(arg: &str, dpi: f32) {
    with(|st| st.dpi = dpi);
    // (one look at the folder at a time: a check that runs or waits already serves; a file it was too early
    // for is found by the look after it)
    // (and not before the retry time of a check that ended with no verdict: userfonts_tick queues it then)
    if with(|st| st.broken || st.retry_ms.is_some() || st.holding)
        || (arg.is_empty() && bg_pending(BgKind::FontCheck, ""))
    {
        return;
    }
    // (the first check of a look is what the user waits on; the rest of its fonts are one job each, behind
    // exports, previews and the slices of a PDF import: userfonts_job_done)
    bg_queue(BgKind::FontCheck, Some(""), Some(arg), dpi, arg != "next");
}

/// A check ended with no verdict for what was wanted: the next one waits (backoff), and a few in a row stop
/// the checks for this session.
fn note_failure(why: &str) {
    with(|st| {
        st.fails += 1;
        if st.fails >= MAX_FAILS {
            st.retry_ms = None;
            if !std::mem::replace(&mut st.broken, true) {
                qlog!("fonts: the check ended %d times with no verdict (%s): no font is checked until Grimoire starts again", st.fails as i32, why);
            }
        } else {
            let wait = RETRY_MS[(st.fails as usize - 1).min(RETRY_MS.len() - 1)];
            st.retry_ms = Some(now_ms().wrapping_add(wait).max(1));
            qlog!("fonts: the check ended with no verdict (%s): tried again in %d s", why, (wait / 1000) as i32);
        }
    });
}

/// The retry time of a check that left a font with no verdict has come: the check is queued again. The Library
/// asks every tick (a comparison when nothing waits).
pub fn userfonts_tick(a: &mut App, now: u32) {
    let due = with(|st| match st.retry_ms {
        Some(t) if now.wrapping_sub(t) < 0x8000_0000 => {
            st.retry_ms = None;
            true
        }
        _ => false,
    });
    if due {
        queue_check("", a.dpi);
    }
}

/// The font files of grimoire/fonts, in name order, each with what the cache says of it: nothing big is read
/// here (a verdict is read when its row is shown, userfont_resolve). Files not checked yet are checked in the
/// background (one FontCheck job for all of them): look again with userfonts_refresh while it runs.
pub fn userfonts_scan(dpi: f32) -> Vec<UserFont> {
    let (files, left_out) = list_fonts();
    if left_out != 0 {
        qlog!("fonts: %d file(s) left out (too many, or a name too long to keep)", left_out as i32);
    }
    // The guard forgets the fonts that are gone or changed (their strikes were another file's).
    with(|st| {
        let marks = guard_load(st);
        let n = marks.len();
        marks.retain(|g| g.armed || files.iter().any(|l| l.key == g.key));
        if marks.len() != n {
            let marks = marks.clone();
            guard_save(&marks);
        }
    });
    let (verdicts, cache) = (verdict_names(), cache_names());
    let list: Vec<UserFont> = files
        .iter()
        .map(|l| UserFont { rel: l.rel.clone(), key: l.key, size: l.size, state: state_from(l, &verdicts, &cache), preview: Canvas::default(), preview_tried: false })
        .collect();
    if list.iter().any(|f| f.state == UserFontState::Checking) {
        queue_check("", dpi);
    }
    list
}

/// What the scan found of each font brought up to date from the cache's files alone (a check that ended
/// since), queueing nothing. How many rows changed.
pub fn userfonts_refresh(list: &mut [UserFont]) -> usize {
    if !list.iter().any(|f| f.state == UserFontState::Checking) {
        return 0;
    }
    let (verdicts, cache) = (verdict_names(), cache_names());
    let broken = with(|st| st.broken);
    let mut changed = 0;
    let rechecked = with(|st| st.rechecked.clone());
    for f in list.iter_mut().filter(|f| f.state == UserFontState::Checking) {
        // (a verdict that did not read waits for its new check: the old file is not "a check that ended")
        if checked_by_names(f.key, &verdicts, &cache) && !rechecked.contains(&f.key) {
            f.state = UserFontState::Checked;
            changed += 1;
        } else if cache.contains(&format!("{:016x}.bad", f.key)) || broken {
            f.state = UserFontState::No(FontWhy::Unreadable);
            changed += 1;
        }
    }
    changed
}

/// How many fonts of the list are still being checked ("Checking 3 fonts...").
pub fn userfonts_checking(list: &[UserFont]) -> usize {
    list.iter().filter(|f| f.state == UserFontState::Checking).count()
}

/// The check's preview of a font that passed (its name drawn in itself, black on white), read once and kept.
pub fn userfont_preview(f: &mut UserFont) -> Option<&Canvas> {
    if !f.preview.px.is_empty() {
        return Some(&f.preview);
    }
    if f.preview_tried || !f.usable() {
        return None;
    }
    f.preview_tried = true;
    let path = cache_file(f.key, "pgm");
    let small = std::fs::symlink_metadata(&path).is_ok_and(|m| m.is_file() && m.len() <= PREVIEW_MAX_BYTES);
    if !small || store_read_pgm(&path, &mut f.preview) != 0 || f.preview.h > 600 || f.preview.w > 8000 {
        f.preview = Canvas::default();
        return None;
    }
    Some(&f.preview)
}

// ---- loading a font that passed ----

/// Why a font was not loaded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LoadErr {
    /// no such file in the folder
    Missing,
    /// not checked yet, or its file changed since: a check is queued
    Checking,
    /// the check refused it (or the guard keeps it out)
    Refused,
    /// too little memory free, or the loaded fonts' cap
    Memory,
    /// the guard's mark could not be written, or the bytes no longer read as a font
    Failed,
}

/// The memory the Kindle has available, in bytes (None where it cannot tell: the host). Tests (not the Kindle's
/// build) set it with GRIMOIRE_FONT_MEM_AVAILABLE.
fn mem_available() -> Option<u64> {
    #[cfg(not(feature = "kindle"))]
    if let Some(v) = std::env::var("GRIMOIRE_FONT_MEM_AVAILABLE").ok().and_then(|v| v.parse::<u64>().ok()) {
        return Some(v);
    }
    let t = std::fs::read_to_string("/proc/meminfo").ok()?;
    let l = t.lines().find(|l| l.starts_with("MemAvailable:"))?;
    Some(l.split_whitespace().nth(1)?.parse::<u64>().ok()? * 1024)
}

struct Loaded {
    data: Rc<Vec<u8>>,
    facts: Rc<FaceFacts>,
    key: u64,
}

/// One checked file's bytes: read whole through the walk and held only if they hash to the verdict's.
fn load_file(st: &FsStorage, rel: &str, dpi: f32, room: usize) -> Result<Loaded, LoadErr> {
    if !userfont_rel_ok(rel) {
        return Err(LoadErr::Missing);
    }
    let sp = StoragePath::new(rel.as_bytes()).ok_or(LoadErr::Missing)?;
    let s = st.stat(&sp, false).map_err(|_| LoadErr::Missing)?;
    let l = Listed { rel: rel.to_string(), size: s.size, mtime: s.mtime, key: key_of(rel, s.size, s.mtime) };
    if name_kind(rel.rsplit('/').next().unwrap_or("")) != NameKind::Font || l.size == 0 || l.size > FONT_FILE_MAX as u64 || guard_out(l.key) {
        return Err(LoadErr::Refused);
    }
    let Some(v) = VerdictDir::open(false).ok().and_then(|vd| verdict_for(&vd, l.key, l.size)) else {
        let bad = std::path::Path::new(&cache_file(l.key, "bad")).exists();
        if bad {
            return Err(LoadErr::Refused);
        }
        queue_check("", dpi);
        return Err(LoadErr::Checking);
    };
    let VerdictKind::Passed(facts) = v.what else { return Err(LoadErr::Refused) };
    if l.size as usize > room {
        return Err(LoadErr::Memory);
    }
    let data = read_listed(st, &l).ok_or(LoadErr::Missing)?;
    if font_hash(&data) != v.hash {
        // Another file under the same name, size and time: what the check passed is not what is here.
        qlog!("font: a font file is not the one that was checked: checking it again");
        queue_check(&format!("recheck:{:016x}", l.key), dpi);
        return Err(LoadErr::Checking);
    }
    Ok(Loaded { data: Rc::new(data), facts: Rc::new(facts), key: l.key })
}

/// The Bold file that goes with a font: in its folder, of the same family, upright, of weight 600 or more,
/// itself checked and passed, the one nearest 700. None for a font that is itself that heavy.
fn pair_of(rel: &str, facts: &FaceFacts) -> Option<String> {
    if facts.weight >= 600 || facts.family.is_empty() {
        return None;
    }
    let folder = |r: &str| r.rfind('/').map_or(String::new(), |i| r[..i].to_string());
    let (files, _) = list_fonts();
    let vd = VerdictDir::open(false).ok()?;
    let mut best: Option<(i32, String)> = None;
    for l in files.iter().filter(|l| l.rel != rel && folder(&l.rel) == folder(rel) && l.size <= FONT_FILE_MAX as u64) {
        let Some(Verdict { what: VerdictKind::Passed(f), .. }) = verdict_for(&vd, l.key, l.size) else { continue };
        if f.italic || f.weight < 600 || !f.family.eq_ignore_ascii_case(&facts.family) {
            continue;
        }
        let d = (f.weight - 700).abs();
        if best.as_ref().is_none_or(|b| d < b.0) {
            best = Some((d, l.rel.clone()));
        }
    }
    best.map(|b| b.1)
}

/// A face of bytes that passed the check, the guard armed for its file before skrifa reads a byte of it here
/// (the mark is let go again if the face cannot be made). None: the mark could not be written, or the bytes
/// do not read as the check read them.
fn guarded_face(l: &Loaded, bold: bool) -> Option<SafeFace> {
    if !guard_arm_keys(&[l.key]) {
        qlog!("font: the guard's mark could not be written: your font is not used");
        return None;
    }
    let face = SafeFace::new(l.data.clone(), l.facts.clone(), bold);
    if face.is_none() && !bold {
        guard_disarm_keys(&[l.key]);
    }
    face
}

/// Loads the font at `rel` (a path under grimoire/fonts) for drawing: its regular face, and its bold by its
/// weight axis, else by `bold_rel` (settings.ini's font_bold) or the Bold file of its family, else none (bold
/// is then the regular overprinted). `held` is what other loaded fonts hold already: all together stay under
/// FONT_LOADED_MAX, the bold file giving way first. The crash guard is armed for each file before it is read
/// as a font. Returns the font, its files' keys, and how its bold is made.
fn load_font(rel: &str, bold_rel: &str, dpi: f32, held: usize) -> Result<(OwnFont, Vec<u64>, &'static str), LoadErr> {
    let st = storage().ok_or(LoadErr::Missing)?;
    if mem_available().is_some_and(|m| m < USERFONT_MEM_FLOOR) {
        qlog!("font: under %d MB of memory available: your font is not loaded", (USERFONT_MEM_FLOOR >> 20) as i32);
        return Err(LoadErr::Memory);
    }
    let t0 = now_ms();
    let reg = load_file(&st, rel, dpi, FONT_LOADED_MAX.saturating_sub(held))?;
    let mut keys = vec![reg.key];
    let regular = guarded_face(&reg, false).ok_or(LoadErr::Failed)?;
    let mut how = "overprint";
    let mut bold = None;
    let mut bytes = reg.data.len();
    let explicit = userfont_rel_ok(bold_rel) && bold_rel != rel;
    if reg.facts.bold_at != 0 && !explicit {
        bold = guarded_face(&reg, true);
        if bold.is_some() {
            how = "its weight axis";
        }
    }
    if bold.is_none() {
        let pair = if explicit { Some(bold_rel.to_string()) } else { pair_of(rel, &reg.facts) };
        if let Some(p) = pair {
            match load_file(&st, &p, dpi, FONT_LOADED_MAX.saturating_sub(held + bytes)) {
                Ok(b) => {
                    if let Some(face) = guarded_face(&b, false) {
                        bytes += b.data.len();
                        keys.push(b.key);
                        bold = Some(face);
                        how = "a Bold file";
                    }
                }
                Err(LoadErr::Memory) => qlog!("font: the Bold file would pass %d MB of fonts loaded: bold is overprinted", (FONT_LOADED_MAX >> 20) as i32),
                Err(_) => {}
            }
        }
    }
    qlog!(
        "font: custom font loaded: %d KB in %u ms, bold by %s, %d glyph(s) from Terminess, VmRSS %d kB",
        (bytes / 1024) as i32,
        now_ms().wrapping_sub(t0),
        how,
        reg.facts.skip.len() as i32,
        vm_rss_kb()
    );
    Ok((OwnFont { regular, bold }, keys, how))
}

/// Tests (the headless and host builds): loads the font at `rel` as the UI would and lets it go again; how its
/// bold is made ("its weight axis", "a Bold file", "overprint"), or why it did not load.
#[cfg(not(feature = "kindle"))]
pub fn userfonts_try_load(rel: &str, bold_rel: &str) -> Result<&'static str, LoadErr> {
    load_font(rel, bold_rel, 300.0, 0).map(|l| {
        guard_disarm_keys(&l.1);
        l.2
    })
}

// This process's resident memory in kB (Linux's VmRSS line), 0 where it is not known.
fn vm_rss_kb() -> i32 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| s.lines().find_map(|l| l.strip_prefix("VmRSS:").and_then(|v| v.trim().trim_end_matches("kB").trim().parse::<i32>().ok())))
        .unwrap_or(0)
}

/// What userfonts_apply came to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Applied {
    /// no custom font is asked for (or Gothic shows): the family draws
    None,
    /// the custom font is in use
    Own,
    /// it is asked for and not usable now: Terminess draws, and the setting stays
    Fallback(LoadErr),
}

/// Makes Classic's text what settings.ini asks: the custom font of font=file:..., loaded if it passed its
/// check (and armed in the guard), else the family. Gothic never loads one. At startup, when the style or the
/// font changes, and when a check ends. True when what draws the text changed (the screen wants a redraw).
pub fn userfonts_apply(a: &mut App) -> bool {
    let was = with(|st| st.in_use.clone());
    let r = apply(a);
    with(|st| {
        st.waiting = matches!(r, Applied::Fallback(LoadErr::Checking));
        st.in_use != was
    })
}

fn apply(a: &mut App) -> Applied {
    let rel = a.settings.font_file.as_str().to_string();
    if rel.is_empty() || a.settings.theme != UiTheme::Classic as i32 {
        if font_own() {
            font_set_own(None);
        }
        let keys = with(|st| {
            st.in_use = None;
            std::mem::take(&mut st.in_use_keys)
        });
        guard_disarm_keys(&keys);
        return Applied::None;
    }
    // The same file as the one in use: nothing to do.
    if font_own() {
        if let (Some(st), Some(sp)) = (storage(), StoragePath::new(rel.as_bytes())) {
            if let Ok(s) = st.stat(&sp, false) {
                if with(|x| x.in_use.as_ref().is_some_and(|(r, k)| *r == rel && *k == key_of(&rel, s.size, s.mtime))) {
                    return Applied::Own;
                }
            }
        }
    }
    // (A font struck at this start is out for the session, load_file's guard_out: Terminess draws.)
    font_set_sample(None);
    font_set_own(None);
    let keys = with(|st| {
        st.in_use = None;
        st.sample = None;
        let mut k = std::mem::take(&mut st.in_use_keys);
        k.append(&mut st.sample_keys);
        k
    });
    guard_disarm_keys(&keys);
    match load_font(&rel, a.settings.font_bold.as_str(), a.dpi, 0) {
        Ok((font, keys, _)) => {
            let name = match font.regular.facts().family.as_str() {
                "" => rel.rsplit('/').next().unwrap_or("").rsplit_once('.').map_or(rel.as_str(), |n| n.0).to_string(),
                family => family.to_string(),
            };
            font_set_family(0);
            font_set_own(Some(font));
            with(|st| {
                st.in_use = Some((rel, keys[0]));
                st.in_use_keys = keys;
                st.in_use_name = name;
                st.said = None;
            });
            Applied::Own
        }
        Err(e) => {
            if with(|st| st.said.replace(e)) != Some(e) {
                qlog!("font: your font is not used (%s): Terminess", load_err_name(e));
            }
            Applied::Fallback(e)
        }
    }
}

fn load_err_name(e: LoadErr) -> &'static str {
    match e {
        LoadErr::Missing => "its file is missing",
        LoadErr::Checking => "it is being checked",
        LoadErr::Refused => "it did not pass its check",
        LoadErr::Memory => "too little memory",
        LoadErr::Failed => "it could not be loaded",
    }
}

/// The Font sheet's sample: the font at `rel` held beside the one in use, for font_with_sample (None: the
/// sample goes). True when it is there to draw. The font in use, picked again, is its own sample.
pub fn userfonts_sample(a: &mut App, rel: Option<&str>) -> bool {
    if rel.is_some() && with(|st| st.sample.as_deref() == rel) {
        return true;
    }
    // (the sample held before goes, and its mark with it if no screen ever showed it)
    if with(|st| st.sample.take().is_some()) {
        font_set_sample(None);
    }
    let keys = with(|st| std::mem::take(&mut st.sample_keys));
    guard_disarm_keys(&keys);
    let Some(rel) = rel else { return false };
    match load_font(rel, "", a.dpi, font_own_bytes()) {
        Ok((font, keys, _)) => {
            font_set_sample(Some(font));
            with(|st| {
                st.sample = Some(rel.to_string());
                st.sample_keys = keys;
            });
            true
        }
        Err(_) => false,
    }
}

/// Whether the custom font in use is the one at `rel`.
pub fn userfonts_in_use(rel: &str) -> bool {
    font_own() && with(|st| st.in_use.as_ref().is_some_and(|(r, _)| r == rel))
}

/// The family name of the custom font in use (None while a family draws): Settings' chip shows it.
pub fn userfonts_in_use_name() -> Option<String> {
    (font_own() && with(|st| st.in_use.is_some())).then(|| with(|st| st.in_use_name.clone()))
}

/// Whether the Font sheet's sample is the font at `rel`, loaded.
pub fn userfonts_sample_is(rel: &str) -> bool {
    with(|st| st.sample.as_deref() == Some(rel))
}

/// The row of a font settings.ini names whose file is no longer in the folder ("File missing.").
pub fn userfont_missing(rel: &str) -> UserFont {
    UserFont { rel: rel.to_string(), key: 0, size: 0, state: UserFontState::No(FontWhy::Missing), preview: Canvas::default(), preview_tried: true }
}

/// A FontCheck job ended (bg_last_exit's code: 0 done, 3 some files were still being copied, 4 no worker could
/// be had, 5 one font was checked and more are left, 6 what a check finds cannot be kept, anything else it
/// died). The font settings.ini asks for is
/// tried again if it was waiting for its check, but only after a job that ended well (0, or 5 for the next
/// one): a check that ends with no verdict for it is asked again after a retry time that grows, and a few in a
/// row stop the checks for the session (userfonts_tick). Returns whether what draws the text changed.
pub fn userfonts_job_done(a: &mut App, code: i32) -> bool {
    with(|st| st.rechecked.clear());
    match code {
        0 => {}
        3 => {
            with(|st| st.later_ms = Some(now_ms().wrapping_add(3000)));
            note_failure("some files are still being copied");
        }
        4 => {
            if !with(|st| std::mem::replace(&mut st.broken, true)) {
                qlog!("fonts: no worker can be had (see the lines above): no font is checked until Grimoire starts again");
            }
        }
        6 => {
            // (Grimoire's own folder cannot be used: as with no worker, nothing is marked and the session
            // checks no more fonts; the fonts that have their verdicts go on working)
            if !with(|st| std::mem::replace(&mut st.broken, true)) {
                qlog!("fonts: what the check finds cannot be kept (see the line above): no font is checked until Grimoire starts again");
            }
        }
        5 => {
            // (one font a job, so that an export or the next slice of a PDF import is not kept waiting for ten
            // minutes: the rest are the next job, which ranks behind them)
            with(|st| st.fails = 0);
            queue_check("next", a.dpi);
        }
        _ => {
            // The writer itself died on a font: the file it was on is marked, and the rest is checked.
            let mut marked = 0;
            for n in cache_names() {
                if let Some(key) = n.strip_suffix(".checking") {
                    let dir = cache_dir();
                    if std::fs::rename(format!("{dir}/{n}"), format!("{dir}/{key}.bad")).is_ok() {
                        marked += 1;
                    }
                }
            }
            qlog!("fonts: the check ended badly; %d file(s) marked unreadable", marked);
            if marked > 0 {
                with(|st| st.fails = 0);
                queue_check("", a.dpi);
            } else {
                note_failure("it failed before any verdict (is the storage, or Grimoire's own folder, full or read-only?)");
            }
        }
    }
    let mut changed = false;
    if with(|st| st.waiting) && a.scr == ScreenId::Library {
        with(|st| st.holding = true);
        changed = userfonts_apply(a);
        with(|st| st.holding = false);
        // A job that ended well and the font still has no verdict: it is not in what the job walks (past the
        // files it lists) or is not settled: no endless asking.
        if code == 0 && with(|st| st.waiting) {
            note_failure("the font asked for has no verdict after a check that ended well");
        }
    }
    if code == 0 && !with(|st| st.waiting) {
        with(|st| st.fails = 0);
    }
    changed
}

/// Whether a check left files for later and it is time to look again (the Font sheet asks while it is up).
pub fn userfonts_look_again(now: u32) -> bool {
    with(|st| match st.later_ms {
        Some(t) if now.wrapping_sub(t) < 0x8000_0000 => {
            st.later_ms = None;
            true
        }
        _ => false,
    })
}

/// Whether the font settings.ini asks for waits for its check (the Library tries it again when it comes in
/// front).
pub fn userfonts_waiting() -> bool {
    with(|st| st.waiting)
}

/// The guard's marks as text, for the runner's probe: "<key> <strikes> <armed>" each, joined by commas.
pub fn userfonts_guard_probe() -> String {
    with(|st| guard_load(st).iter().map(|g| format!("{:016x} {} {}", g.key, g.strikes, g.armed as i32)).collect::<Vec<_>>().join(","))
}

// ---- the job: the writer ----

/// The files of a scan held still long enough (DESIGN.md 3.8): unchanged across two looks at least 2 s apart
/// and at least 5 s old. Tests (not the Kindle's build) skip the wait with GRIMOIRE_IMPORT_STABLE=fast.
fn fast_stable() -> bool {
    #[cfg(not(feature = "kindle"))]
    {
        std::env::var("GRIMOIRE_IMPORT_STABLE").is_ok_and(|v| v == "fast")
    }
    #[cfg(feature = "kindle")]
    {
        false
    }
}

/// How long a font waits for its second try after a first timeout. Tests (not the Kindle's build) that set the
/// watchdog short on purpose do not wait the minute.
fn slow_retry_s() -> i64 {
    #[cfg(not(feature = "kindle"))]
    if std::env::var_os("GRIMOIRE_FONT_WATCHDOG_MS").is_some() {
        return 0;
    }
    SLOW_RETRY_S
}

fn write_cache(path: &str, data: &[u8]) -> bool {
    write_file_atomic(path, data) == 0
}

/// BgKind::FontCheck, in the background child: every font file of grimoire/fonts that has no verdict yet is
/// read whole and checked by the sandboxed worker (fontcheck.rs), one after another, its preview (or its
/// `.no`) written to the cache and its verdict to where verdicts are kept (`VerdictDir`). `arg` is "" or
/// "recheck:<key>" (that file's verdict is stale: it is checked again). `dpi` sizes the preview. Returns 0
/// done, 3 some files wait (still being copied, or no memory now), 4 no worker can be had (nothing is marked),
/// 6 what a check finds cannot be kept (Grimoire's own folder cannot be used, or has too little room: nothing
/// is checked and nothing is marked), -1 it could not start.
pub fn bg_font_check(arg: &str, dpi: f32) -> i32 {
    let cache = cache_dir();
    // (its files are written and stale entries deleted there as root: never through a planted link)
    if !no_link_below(&store_root(), &cache) || mkdir_p(&cache) != 0 {
        return -1;
    }
    let vd = match VerdictDir::open(true) {
        Ok(vd) => vd,
        Err(why) if state_base().is_some() => {
            // (before any worker starts and before any mark is written; the folder's path is never logged)
            qlog!("fonts: what the check finds cannot be kept (%s); nothing is marked", why);
            return 6;
        }
        Err(_) => return -1,
    };
    if let Some((base, tag)) = &vd.own {
        own_upkeep(base, tag);
    }
    let Some(st) = storage() else { return -1 };
    let (files, _) = list_fonts();
    let t0 = real_now_ms();
    // What is kept for files that are gone (or changed) goes; so do marks an earlier run left.
    if let Some(key) = arg.strip_prefix("recheck:").filter(|k| k.len() == 16 && k.bytes().all(|c| c.is_ascii_hexdigit())).and_then(|k| u64::from_str_radix(k, 16).ok()) {
        vd.remove(key);
        for ext in ["pgm", "no", "bad", "slow"] {
            let _ = std::fs::remove_file(cache_file(key, ext));
        }
    }
    let keep: std::collections::HashSet<String> = files.iter().map(|l| format!("{:016x}", l.key)).collect();
    for n in cache_names() {
        let stem = n.split('.').next().unwrap_or("");
        if n.starts_with('.') {
            continue;
        }
        if !keep.contains(stem) || n.ends_with(".checking") || n.ends_with(".tmp") {
            let _ = std::fs::remove_file(format!("{cache}/{n}"));
        }
    }
    if vd.private() {
        vd.sweep(&keep);
    }
    let px = (44.0 * dpi / 300.0).round().clamp(8.0, 400.0) as i32;
    // Whether a file still needs a verdict (one that a first timeout left for a retry counts: it waits).
    let needs = |l: &Listed| {
        name_kind(l.rel.rsplit('/').next().unwrap_or("")) == NameKind::Font
            && l.size != 0
            && l.size <= FONT_FILE_MAX as u64
            && !std::path::Path::new(&cache_file(l.key, "bad")).exists()
            && verdict_for(&vd, l.key, l.size).is_none()
    };
    let mut later = 0;
    let wall = wall_time();
    let n = files.len();
    for (i, l) in files.iter().enumerate() {
        if name_kind(l.rel.rsplit('/').next().unwrap_or("")) != NameKind::Font || l.size == 0 || l.size > FONT_FILE_MAX as u64 {
            continue;
        }
        let (pp, np, bp, cp) = (cache_file(l.key, "pgm"), cache_file(l.key, "no"), cache_file(l.key, "bad"), cache_file(l.key, "checking"));
        if std::path::Path::new(&bp).exists() || verdict_for(&vd, l.key, l.size).is_some() {
            continue;
        }
        // A file whose check timed out once waits a minute, then is checked again (the clock may have jumped).
        let slow_mark = cache_file(l.key, "slow");
        if let Ok(m) = std::fs::metadata(&slow_mark) {
            let age = m.modified().ok().and_then(|t| std::time::SystemTime::now().duration_since(t).ok()).map_or(0, |d| d.as_secs() as i64);
            if age < slow_retry_s() {
                later += 1;
                continue;
            }
        }
        // A file still being copied waits: young, or changed between two looks.
        if !fast_stable() {
            if wall - l.mtime < crate::pdfimport::STABLE_AGE_S {
                later += 1;
                continue;
            }
            let since = real_now_ms().wrapping_sub(t0);
            if since < crate::pdfimport::STABLE_GAP_MS {
                sleep_ms((crate::pdfimport::STABLE_GAP_MS - since) as i32);
            }
        }
        // Nothing is checked whose verdict could not be kept (fonts that have theirs go on loading: reading
        // needs no room).
        if !vd.room() {
            qlog!("fonts: what the check finds cannot be kept (too little room); nothing is marked");
            return 6;
        }
        if !write_cache(&cp, b"") {
            return -1;
        }
        let Some(data) = read_listed(&st, l) else {
            let _ = std::fs::remove_file(&cp);
            later += 1;
            continue;
        };
        let hash = font_hash(&data);
        let t = real_now_ms();
        let out = check_font(&data, px);
        let ms = real_now_ms().wrapping_sub(t);
        // A verdict is kept with its companion in the cache, written first (verdict_for): a pass after its
        // preview, a refusal after its `.no`. A mark a first timeout left goes with any verdict.
        let keep_verdict = |what: VerdictKind| {
            let pass = matches!(what, VerdictKind::Passed(_));
            let kept = (pass || write_cache(&np, b"")) && vd.write(l.key, Verdict { hash, size: l.size, what }.to_text().as_bytes());
            if kept {
                let _ = std::fs::remove_file(&slow_mark);
                let _ = std::fs::remove_file(if pass { &np } else { &pp });
            }
            kept
        };
        let mut wrote = true;
        match out {
            FontOut::Passed { facts, preview, w, h } => {
                let mut c = Canvas::default();
                if canvas_init(&mut c, w, h) != 0 {
                    // (a verdict counts only with its preview: none can be made now, so none is written)
                    qlog!("fonts: file %d of %d left for later (out of memory)", i as i32 + 1, n as i32);
                    later += 1;
                } else {
                    for y in 0..h as usize {
                        c.px[y * c.stride as usize..y * c.stride as usize + w as usize].copy_from_slice(&preview[y * w as usize..(y + 1) * w as usize]);
                    }
                    qlog!(
                        "fonts: file %d of %d passed: %d KB, %d glyphs, %d from Terminess, checked in %u ms",
                        i as i32 + 1,
                        n as i32,
                        (l.size / 1024) as i32,
                        facts.mapped,
                        facts.skip.len() as i32,
                        ms
                    );
                    wrote = store_write_pgm(&pp, &c) == 0 && keep_verdict(VerdictKind::Passed(facts));
                }
            }
            FontOut::Refused(r) => {
                qlog!("fonts: file %d of %d refused (%s)", i as i32 + 1, n as i32, match r {
                    Refuse::Unreadable => "unreadable",
                    Refuse::NoLatin => "no Latin letters",
                    Refuse::Format => "a web font",
                });
                wrote = keep_verdict(VerdictKind::Refused(r));
            }
            FontOut::Slow => {
                if std::path::Path::new(&slow_mark).exists() {
                    qlog!("fonts: file %d of %d took too long to read again (%u ms)", i as i32 + 1, n as i32, ms);
                    wrote = keep_verdict(VerdictKind::Slow);
                    let _ = std::fs::remove_file(&slow_mark);
                } else {
                    // (a timeout may be the clock's jump after a wake: no verdict yet, one more try later)
                    qlog!("fonts: file %d of %d took too long to read (%u ms): left for a second try", i as i32 + 1, n as i32, ms);
                    wrote = write_cache(&slow_mark, b"");
                    later += 1;
                }
            }
            FontOut::Dead(why) => {
                qlog!("fonts: file %d of %d: the check's worker ended (%s): marked unreadable", i as i32 + 1, n as i32, why);
                let _ = std::fs::rename(&cp, &bp);
                if files[i + 1..].iter().any(|l| needs(l)) {
                    return 5;
                }
                continue;
            }
            FontOut::Retry(why) => {
                qlog!("fonts: file %d of %d left for later (%s)", i as i32 + 1, n as i32, why);
                later += 1;
            }
            FontOut::NoWorker(why) => {
                qlog!("fonts: no worker for the check (%s); nothing is marked", why);
                let _ = std::fs::remove_file(&cp);
                return 4;
            }
        }
        let _ = std::fs::remove_file(&cp);
        if !wrote {
            qlog!("fonts: what the check found could not be written. Is the storage, or Grimoire's own folder, full?");
            return -1;
        }
        // One font a job: the others are the next one's, which ranks behind an export or a PDF's slice.
        if files[i + 1..].iter().any(|l| needs(l)) {
            qlog!("fonts: file %d of %d done; more are left for the next job", i as i32 + 1, n as i32);
            return 5;
        }
    }
    // The folder's tally, from the verdicts kept: this job checked one font at most (the fix round's
    // M19), so its own counts would read "1 passed" after three fonts.
    let (mut passed, mut refused, mut dead, mut waiting, mut other, mut all) = (0, 0, 0, 0, 0, 0);
    for l in files.iter().filter(|l| name_kind(l.rel.rsplit('/').next().unwrap_or("")) == NameKind::Font) {
        all += 1;
        if l.size == 0 || l.size > FONT_FILE_MAX as u64 {
            other += 1;
        } else if std::path::Path::new(&cache_file(l.key, "bad")).exists() {
            dead += 1;
        } else {
            match verdict_for(&vd, l.key, l.size).map(|v| v.what) {
                Some(VerdictKind::Passed(_)) => passed += 1,
                Some(_) => refused += 1,
                None => waiting += 1,
            }
        }
    }
    qlog!(
        "fonts: in the folder, %d passed, %d refused, %d the worker died on, %d left for later, %d too big or empty (%d font file(s) in all)",
        passed,
        refused,
        dead,
        waiting,
        other,
        all
    );
    if later > 0 {
        3
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_tell_fonts_from_the_rest() {
        assert_eq!(name_kind("Inter.ttf"), NameKind::Font);
        assert_eq!(name_kind("Inter.OTF"), NameKind::Font);
        assert_eq!(name_kind("Fonts.ttc"), NameKind::Font);
        assert_eq!(name_kind("Inter.woff2"), NameKind::Web);
        assert_eq!(name_kind("Inter.WOFF"), NameKind::Web);
        assert_eq!(name_kind("README.txt"), NameKind::Other);
        assert_eq!(name_kind(".hidden.ttf"), NameKind::Other);
        assert_eq!(name_kind("ttf"), NameKind::Other);
        assert!(userfont_rel_ok("Inter.ttf") && userfont_rel_ok("Inter/Inter-Bold.otf"));
        for bad in ["", "a/b/c.ttf", "../x.ttf", "a/../x.ttf", ".cache/x.ttf", "x.txt", "a\nb.ttf", "/x.ttf", "x.ttf/"] {
            assert!(!userfont_rel_ok(bad), "{bad}");
        }
        assert!(!userfont_rel_ok(&format!("{}.ttf", "n".repeat(USERFONT_REL_MAX))));
    }

    #[test]
    fn a_file_replaced_gets_another_key() {
        let k = key_of("Inter.ttf", 1000, 1_700_000_000);
        assert_eq!(k, key_of("Inter.ttf", 1000, 1_700_000_000));
        assert_ne!(k, key_of("Inter.ttf", 1001, 1_700_000_000));
        assert_ne!(k, key_of("Inter.ttf", 1000, 1_700_000_001));
        assert_ne!(k, key_of("Inter2.ttf", 1000, 1_700_000_000));
    }

    // The guard, start by start: a font armed and never seen on a screen is struck at the next start, tried once
    // more at the one after, and after a second strike stays out until its file changes (another key).
    #[test]
    fn two_strikes_keep_a_font_out_until_its_file_changes() {
        let (font, other) = (0xAAAA_u64, 0xBBBB_u64);
        let mut marks: Vec<GuardEntry> = Vec::new();
        // start 1: armed, then the first screen never ends (no settle)
        assert!(guard_start(&mut marks).is_empty());
        guard_arm(&mut marks, font);
        assert_eq!(marks, vec![GuardEntry { key: font, strikes: 0, armed: true }]);
        let on_disk = guard_text(&marks);
        // start 2: a strike, Terminess for this start; the font is not refused for good
        let mut marks = guard_parse(on_disk.as_bytes()).unwrap();
        assert_eq!(guard_start(&mut marks), vec![font]);
        assert_eq!(marks, vec![GuardEntry { key: font, strikes: 1, armed: false }]);
        assert!(!guard_refuses(&marks, font));
        // start 3: tried once more; this time the screen is drawn: as good as new
        let mut again = marks.clone();
        assert!(guard_start(&mut again).is_empty());
        guard_arm(&mut again, font);
        assert_eq!(again, vec![GuardEntry { key: font, strikes: 1, armed: true }]);
        let mut fine = again.clone();
        guard_settle(&mut fine);
        assert!(fine.is_empty());
        // start 3, the other way: it crashes again; start 4 finds the second strike
        let mut marks = guard_parse(guard_text(&again).as_bytes()).unwrap();
        assert_eq!(guard_start(&mut marks), vec![font]);
        assert!(guard_refuses(&marks, font));
        // every start after keeps it out, and a start changes nothing more
        let mut later = marks.clone();
        assert!(guard_start(&mut later).is_empty());
        assert_eq!(later, marks);
        assert!(guard_refuses(&later, font));
        // the file changed: another key, which nothing refuses
        assert!(!guard_refuses(&later, other));
        // a power cut during a first screen costs one start in Terminess, not the font
        let mut cut = vec![GuardEntry { key: other, strikes: 0, armed: true }];
        guard_start(&mut cut);
        assert!(!guard_refuses(&cut, other));
    }

    #[test]
    fn the_guard_file_parses_strictly() {
        let marks = vec![GuardEntry { key: 0x1234, strikes: 2, armed: false }, GuardEntry { key: u64::MAX, strikes: 0, armed: true }];
        assert_eq!(guard_parse(guard_text(&marks).as_bytes()), Some(marks));
        assert_eq!(guard_parse(b"grimoire-font-guard 1\n"), Some(Vec::new()));
        for bad in [
            "",
            "grimoire-font-guard 2\n",
            "grimoire-font-guard 1\n1234 1 0\n",
            "grimoire-font-guard 1\n0000000000001234 3 0\n",
            "grimoire-font-guard 1\n0000000000001234 1 2\n",
            "grimoire-font-guard 1\n0000000000001234 1\n",
            "grimoire-font-guard 1\n000000000000123G 1 0\n",
        ] {
            assert_eq!(guard_parse(bad.as_bytes()), None, "{bad}");
        }
        // more marks than it ever writes
        let many: Vec<GuardEntry> = (0..9).map(|k| GuardEntry { key: k, strikes: 1, armed: false }).collect();
        assert_eq!(guard_parse(guard_text(&many).as_bytes()), None);
        // the ninth font armed takes the place of the oldest one not in use
        let mut full: Vec<GuardEntry> = (0..8).map(|k| GuardEntry { key: k, strikes: 2, armed: false }).collect();
        guard_arm(&mut full, 99);
        assert_eq!(full.len(), 8);
        assert!(full.iter().any(|g| g.key == 99 && g.armed) && !full.iter().any(|g| g.key == 0));
    }

    #[test]
    fn room_below_the_floor_is_not_enough() {
        let (bytes, files) = (STATE_FREE_FLOOR, STATE_FILES_FLOOR);
        assert!(enough_room(bytes, 100_000, files));
        assert!(enough_room(u64::MAX, u64::MAX, u64::MAX));
        // under the byte floor, whatever the files
        assert!(!enough_room(bytes - 1, 100_000, 100_000));
        assert!(!enough_room(0, 0, 0));
        // under the file floor
        assert!(!enough_room(bytes, 100_000, files - 1));
        assert!(!enough_room(u64::MAX, 100_000, 0));
        // a file system that counts no files is not refused for having none free
        assert!(enough_room(bytes, 0, 0));
    }

    #[test]
    fn a_data_roots_tag_ignores_a_trailing_slash() {
        let t = root_tag("/mnt/us/grimoire");
        assert_eq!(t.len(), 16);
        assert!(t.bytes().all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c)));
        assert_eq!(t, root_tag("/mnt/us/grimoire/"));
        assert_eq!(t, root_tag("/mnt/us/grimoire//"));
        assert_ne!(t, root_tag("/mnt/us/grimoire-dev"));
        assert_ne!(t, root_tag("/mnt/us/Grimoire"));
        // (FNV-1a 64 of the path's bytes: the offset basis for no bytes at all)
        assert_eq!(root_tag(""), "cbf29ce484222325");
        assert_eq!(root_tag("a"), "af63dc4c8601ec8c");
    }

    #[test]
    fn no_em_dash() {
        assert!(!include_str!("userfonts.rs").contains('\u{2014}'));
    }
}
