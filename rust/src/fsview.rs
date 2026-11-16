//! One storage layer (the 2.1 plan, DESIGN.md "One storage layer"; browser.md 8 and 9): the only code that
//! touches the Kindle's storage for the file browser and for every consumer of a user's file. Lane L0 built
//! its reading side: the root, the walk, Grimoire's own folders by identity, StoragePath, zones, the
//! time-sliced listing, stats and free space; the picker (L8) and the PDF import (L5) read through it. Since
//! 2.2 (issue #117) it also writes, inside Grimoire's folders for the user's files only: a rename that never
//! replaces an entry, the trash with its record, restore, and the name rules (the copy job is
//! bgwork_copy.rs). Nothing on screen calls the writes yet.
//!
//! - One root: /mnt/us on the Kindle (when Grimoire's own folder is inside it); elsewhere GRIMOIRE_STORAGE,
//!   a hook compiled out of the Kindle build. Never "/".
//! - One walk, no links: each component opened with openat(O_DIRECTORY|O_NOFOLLOW) from the one before, the
//!   file with O_NOFOLLOW, so a link anywhere fails (O_NOFOLLOW covers the last component only; the Scribe's
//!   kernel, 4.9, has no openat2). Each directory must be on the root's device, and none may be Grimoire's
//!   notebooks (nor, for a pick, its PDFs or trash), compared by st_dev and st_ino: a path cased differently
//!   on a case-insensitive storage cannot reach them by name.
//! - Zones: lists hide by a deny-list (dot-entries, *.sdr, the system's folders, Grimoire's own); writes go
//!   by an allow-list (what is inside Grimoire's import, templates, photos and fonts), so safety never
//!   depends on the deny-list being complete.
//! - Never a hard delete, never an overwrite: delete is a rename into the trash, and every rename is
//!   renameat2(RENAME_NOREPLACE), or where the storage refuses that call (and on the Mac) renameat over a
//!   reservation of the new name made first (an O_EXCL create, or mkdirat), so nothing another writer puts
//!   there is replaced. Each write checks the item is still the one listed (FsStat::same) and that no import
//!   holds it.
//! - Time-sliced: a listing reads names and d_type only (at most 5,000), and stats only the rows asked for,
//!   each in slices of at most 20 ms a tick with the folder kept open between ticks.
//! - Privacy: the log has counts, zones and timings, never a name.

use crate::prelude::*;
use std::ffi::CString;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};

/// The Kindle's storage, the root of every StoragePath there.
pub const FS_KINDLE_ROOT: &str = "/mnt/us";
/// A StoragePath is shorter than this (a job's path, JOB_PATH_MAX in bgwork_template.rs).
pub const FS_PATH_MAX: usize = 600;
/// A listing stops here ("Showing the first 5,000.").
pub const FS_LIST_MAX: usize = 5000;
/// A slice of listing or stats: at most this long a tick.
pub const FS_SLICE_MS: u32 = 20;
/// A job that writes a lot is refused when less than this would be left after it.
pub const FS_FREE_FLOOR: u64 = 100 << 20;
/// Under this, Files says "Kindle storage nearly full."
pub const FS_FREE_LOW: u64 = 200 << 20;

/// The system's folders at the storage root, never listed or entered.
const SYSTEM: [&[u8]; 9] =
    [b"system", b"extensions", b"koreader", b"mrpackages", b"usbnet", b"libkh", b"linkss", b"linkfonts", b"kindle_hid_passthrough"];
/// Grimoire's folders that are its own business, never listed or entered.
const OWN_REFUSED: [&[u8]; 4] = [b"app", b"notebooks", b"bluetooth", b".cache"];
/// Grimoire's folders a pick never offers from (a mirror PDF, a trashed notebook).
const OWN_NOT_PICKED: [&[u8]; 2] = [b"PDFs", b"trash"];
/// Grimoire's folders that hold the user's files (writes go only to what is inside them).
const OWN_WRITABLE: [&[u8]; 4] = [b"import", b"templates", b"photos", b"fonts"];

/// The storage root for the browser: /mnt/us on the Kindle when Grimoire's own folder is inside it; elsewhere
/// GRIMOIRE_STORAGE (an absolute path, not "/"), the simulator's and the tests' hook, compiled out of the Kindle
/// build. None: no storage to browse (the browser shows only Grimoire's folders).
pub fn fsview_root() -> Option<String> {
    #[cfg(feature = "kindle")]
    {
        let r = store_root();
        r.starts_with("/mnt/us/").then(|| FS_KINDLE_ROOT.to_string())
    }
    #[cfg(not(feature = "kindle"))]
    {
        let s = std::env::var("GRIMOIRE_STORAGE").ok()?;
        (s.starts_with('/') && !s.trim_end_matches('/').is_empty()).then_some(s)
    }
}

/// A path relative to the storage root, checked: shorter than 600 bytes, no empty, "." or ".." component, no
/// leading or trailing "/", no NUL. Its names are bytes (a name need not be UTF-8). What every job and setting
/// will carry. The empty path is the root.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StoragePath(Vec<u8>);

impl StoragePath {
    pub fn new(rel: &[u8]) -> Option<StoragePath> {
        if rel.len() >= FS_PATH_MAX || rel.contains(&0) {
            return None;
        }
        if !rel.is_empty() && rel.split(|&c| c == b'/').any(|n| n.is_empty() || n == b"." || n == b"..") {
            return None;
        }
        Some(StoragePath(rel.to_vec()))
    }
    /// The root.
    pub fn root() -> StoragePath {
        StoragePath(Vec::new())
    }
    /// This folder's entry `name` (one component), checked as new is.
    pub fn join(&self, name: &[u8]) -> Option<StoragePath> {
        if name.contains(&b'/') {
            return None;
        }
        let mut v = self.0.clone();
        if !v.is_empty() {
            v.push(b'/');
        }
        v.extend_from_slice(name);
        StoragePath::new(&v)
    }
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
    /// As text for a screen or a job (a name that is not UTF-8 is shown with U+FFFD).
    pub fn to_string_lossy(&self) -> String {
        String::from_utf8_lossy(&self.0).into_owned()
    }
    pub fn is_root(&self) -> bool {
        self.0.is_empty()
    }
    pub fn components(&self) -> impl Iterator<Item = &[u8]> {
        self.0.split(|&c| c == b'/').filter(|n| !n.is_empty())
    }
    /// Its folder and its last name; None for the root.
    pub fn split_last(&self) -> Option<(StoragePath, &[u8])> {
        if self.0.is_empty() {
            return None;
        }
        Some(match self.0.iter().rposition(|&c| c == b'/') {
            Some(i) => (StoragePath(self.0[..i].to_vec()), &self.0[i + 1..]),
            None => (StoragePath::root(), &self.0[..]),
        })
    }
}

/// What may be done in a place (DESIGN.md 3.5, rule 4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FsZone {
    /// Grimoire's folders for the user's files: import, templates, photos, fonts (writes only to what is inside)
    Writable,
    /// documents, Downloads, the storage's fonts, the rest: looked at and picked from, never written
    ReadOnly,
    /// never listed or entered: the system's folders, dot-entries, *.sdr, Grimoire's own (and, for a pick,
    /// its PDFs and trash)
    Refused,
}

/// Why a walk stopped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FsErr {
    /// a component is a link
    Link,
    /// a folder on another device than the root
    OtherDevice,
    /// one of Grimoire's own folders (by identity), or a refused zone
    Refused,
    NotFound,
    /// a component that should be a folder is not
    NotDir,
    /// the end is not a regular file (a FIFO, a device, a folder)
    NotFile,
    /// any other errno
    Io(i32),
}

/// A listed entry's kind: from d_type, or from its stat when d_type said nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FsKind {
    Dir,
    File,
    /// a link (shown, never entered or opened)
    Link,
    /// a FIFO, a socket, a device
    Other,
    /// d_type said nothing: its stat will tell
    Unknown,
}

/// What a stat says, and what an action compares with the listing before it acts ("File changed.").
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FsStat {
    pub dev: u64,
    pub ino: u64,
    pub size: u64,
    pub mtime: i64,
    pub mode: u32,
}

impl FsStat {
    fn of(st: &libc::stat) -> FsStat {
        FsStat { dev: st.st_dev as u64, ino: st.st_ino as u64, size: st.st_size as u64, mtime: st.st_mtime as i64, mode: st.st_mode as u32 }
    }
    fn kind(&self) -> FsKind {
        match self.mode & libc::S_IFMT as u32 {
            m if m == libc::S_IFDIR as u32 => FsKind::Dir,
            m if m == libc::S_IFREG as u32 => FsKind::File,
            m if m == libc::S_IFLNK as u32 => FsKind::Link,
            _ => FsKind::Other,
        }
    }
    /// The same file as when it was listed: device, inode, size and time.
    pub fn same(&self, o: &FsStat) -> bool {
        (self.dev, self.ino, self.size, self.mtime) == (o.dev, o.ino, o.size, o.mtime)
    }
}

/// A listed entry: its name's bytes, its kind, and its stat once asked for (rows on screen only).
#[derive(Clone, Debug)]
pub struct FsEntry {
    pub name: Vec<u8>,
    pub kind: FsKind,
    pub stat: Option<FsStat>,
}

fn errno() -> i32 {
    std::io::Error::last_os_error().raw_os_error().unwrap_or(libc::EIO)
}

fn eq_nocase(a: &[u8], b: &[u8]) -> bool {
    a.eq_ignore_ascii_case(b)
}

/// The zone of a path, given where Grimoire's own folder is (relative to the root). Names compare without
/// regard to ASCII case, as a case-insensitive storage finds them.
fn zone_of(grimoire: Option<&StoragePath>, p: &StoragePath, pick: bool) -> FsZone {
    let comps: Vec<&[u8]> = p.components().collect();
    if comps.iter().any(|n| n.first() == Some(&b'.') || (n.len() >= 4 && eq_nocase(&n[n.len() - 4..], b".sdr"))) {
        return FsZone::Refused;
    }
    if comps.first().is_some_and(|n| SYSTEM.iter().any(|s| eq_nocase(n, s))) {
        return FsZone::Refused;
    }
    if let Some(g) = grimoire {
        let gc: Vec<&[u8]> = g.components().collect();
        if !gc.is_empty() && comps.len() > gc.len() && gc.iter().zip(&comps).all(|(a, b)| eq_nocase(a, b)) {
            let n = comps[gc.len()];
            if OWN_REFUSED.iter().any(|s| eq_nocase(n, s)) || (pick && OWN_NOT_PICKED.iter().any(|s| eq_nocase(n, s))) {
                return FsZone::Refused;
            }
            if OWN_WRITABLE.iter().any(|s| eq_nocase(n, s)) {
                return FsZone::Writable;
            }
            // (its settings and lists, in Grimoire's folder itself)
            if comps.len() == gc.len() + 1 && n.len() >= 4 && eq_nocase(&n[n.len() - 4..], b".ini") {
                return FsZone::Refused;
            }
        }
    }
    FsZone::ReadOnly
}

/// The storage, opened once: the root's descriptor and device, and the identity of Grimoire's own folders.
pub struct FsStorage {
    root: OwnedFd,
    dev: u64,
    /// Grimoire's folder relative to the root, when it is inside it
    grimoire: Option<StoragePath>,
    /// (dev, ino) of Grimoire's notebooks, and of its PDFs and trash (refused to a pick)
    notebooks: Option<(u64, u64)>,
    not_picked: Vec<(u64, u64)>,
    /// (dev, ino) of Grimoire's trash, where the writes put what is deleted
    trash: Option<(u64, u64)>,
}

fn open_dir_at(dirfd: i32, name: &[u8]) -> Result<OwnedFd, FsErr> {
    let c = CString::new(name).map_err(|_| FsErr::NotFound)?;
    // SAFETY: c is a NUL-terminated name; the descriptor returned is owned by the OwnedFd.
    let fd = unsafe { libc::openat(dirfd, c.as_ptr(), libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC) };
    if fd >= 0 {
        // SAFETY: fd was just opened and is owned by no one else.
        return Ok(unsafe { OwnedFd::from_raw_fd(fd) });
    }
    Err(classify(dirfd, &c, errno()))
}

/// Why opening `name` in `dirfd` failed: a link (whatever errno the system chose for it), else by errno.
fn classify(dirfd: i32, name: &CString, e: i32) -> FsErr {
    // SAFETY: st is written by fstatat before it is read; name is NUL-terminated.
    let mut st: libc::stat = unsafe { std::mem::zeroed() };
    let r = unsafe { libc::fstatat(dirfd, name.as_ptr(), &mut st, libc::AT_SYMLINK_NOFOLLOW) };
    if r == 0 && (st.st_mode & libc::S_IFMT) == libc::S_IFLNK {
        return FsErr::Link;
    }
    match e {
        libc::ELOOP => FsErr::Link,
        libc::ENOENT => FsErr::NotFound,
        libc::ENOTDIR => FsErr::NotDir,
        _ => FsErr::Io(e),
    }
}

fn fstat_fd(fd: i32) -> Result<FsStat, FsErr> {
    // SAFETY: st is written by fstat before it is read.
    let mut st: libc::stat = unsafe { std::mem::zeroed() };
    if unsafe { libc::fstat(fd, &mut st) } != 0 {
        return Err(FsErr::Io(errno()));
    }
    Ok(FsStat::of(&st))
}

/// (dev, ino) of a folder of Grimoire's own, by its path (its own root, which it trusts).
fn identity(path: &str) -> Option<(u64, u64)> {
    let c = CString::new(path).ok()?;
    // SAFETY: c is NUL-terminated; st is written by stat before it is read.
    let mut st: libc::stat = unsafe { std::mem::zeroed() };
    (unsafe { libc::stat(c.as_ptr(), &mut st) } == 0).then_some((st.st_dev as u64, st.st_ino as u64))
}

impl FsStorage {
    /// Opens the storage at `root` (O_DIRECTORY|O_NOFOLLOW), with Grimoire's own folder at `grimoire` (its store
    /// root, an absolute path: inside `root` or not), whose notebooks, PDFs and trash it notes by identity.
    pub fn open(root: &str, grimoire: &str) -> Result<FsStorage, FsErr> {
        let c = CString::new(root).map_err(|_| FsErr::NotFound)?;
        // SAFETY: c is NUL-terminated; the descriptor is owned by the OwnedFd.
        let fd = unsafe { libc::open(c.as_ptr(), libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC) };
        if fd < 0 {
            return Err(if errno() == libc::ELOOP { FsErr::Link } else { FsErr::Io(errno()) });
        }
        // SAFETY: fd was just opened and is owned by no one else.
        let root_fd = unsafe { OwnedFd::from_raw_fd(fd) };
        let dev = fstat_fd(root_fd.as_raw_fd())?.dev;
        let base = root.trim_end_matches('/');
        let rel = grimoire.strip_prefix(base).and_then(|r| r.strip_prefix('/')).map(|r| r.trim_end_matches('/'));
        let g = grimoire.trim_end_matches('/');
        let trash = identity(&format!("{g}/trash"));
        Ok(FsStorage {
            root: root_fd,
            dev,
            grimoire: rel.and_then(|r| StoragePath::new(r.as_bytes())),
            notebooks: identity(&format!("{g}/notebooks")),
            not_picked: [identity(&format!("{g}/PDFs")), trash].into_iter().flatten().collect(),
            trash,
        })
    }

    /// Grimoire's own folder relative to the root, when it is inside it.
    pub fn grimoire_rel(&self) -> Option<&StoragePath> {
        self.grimoire.as_ref()
    }

    /// The zone of `p` (a pick also refuses Grimoire's PDFs and trash).
    pub fn zone(&self, p: &StoragePath, pick: bool) -> FsZone {
        zone_of(self.grimoire.as_ref(), p, pick)
    }

    /// The folder at `p`, opened one component at a time from the root, never through a link, each on the
    /// root's device and none of Grimoire's own folders; a refused zone is not entered.
    pub fn open_dir(&self, p: &StoragePath, pick: bool) -> Result<OwnedFd, FsErr> {
        if self.zone(p, pick) == FsZone::Refused {
            return Err(FsErr::Refused);
        }
        let mut fd = open_dir_at(self.root.as_raw_fd(), b".")?;
        for name in p.components() {
            let next = open_dir_at(fd.as_raw_fd(), name)?;
            let st = fstat_fd(next.as_raw_fd())?;
            if st.dev != self.dev {
                return Err(FsErr::OtherDevice);
            }
            let id = Some((st.dev, st.ino));
            if id == self.notebooks || (pick && self.not_picked.iter().any(|&o| Some(o) == id)) {
                return Err(FsErr::Refused);
            }
            fd = next;
        }
        Ok(fd)
    }

    /// The regular file at `p`, for reading: its folder by the walk, the file O_NOFOLLOW (and O_NONBLOCK, so a
    /// FIFO never waits for a writer), on the root's device.
    pub fn open_file(&self, p: &StoragePath, pick: bool) -> Result<OwnedFd, FsErr> {
        let (folder, name) = p.split_last().ok_or(FsErr::NotFile)?;
        if self.zone(p, pick) == FsZone::Refused {
            return Err(FsErr::Refused);
        }
        let dir = self.open_dir(&folder, pick)?;
        let c = CString::new(name).map_err(|_| FsErr::NotFound)?;
        // SAFETY: c is NUL-terminated; the descriptor is owned by the OwnedFd.
        let fd = unsafe { libc::openat(dir.as_raw_fd(), c.as_ptr(), libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC) };
        if fd < 0 {
            return Err(classify(dir.as_raw_fd(), &c, errno()));
        }
        // SAFETY: fd was just opened and is owned by no one else.
        let file = unsafe { OwnedFd::from_raw_fd(fd) };
        let st = fstat_fd(file.as_raw_fd())?;
        if st.kind() != FsKind::File {
            return Err(FsErr::NotFile);
        }
        if st.dev != self.dev {
            return Err(FsErr::OtherDevice);
        }
        // (reads block again, as a file's do)
        // SAFETY: fcntl on a descriptor this function owns.
        unsafe {
            let fl = libc::fcntl(file.as_raw_fd(), libc::F_GETFL);
            libc::fcntl(file.as_raw_fd(), libc::F_SETFL, fl & !libc::O_NONBLOCK);
        }
        Ok(file)
    }

    /// The entry at `p` as it is now (a link is its own stat, never followed): what an action compares with
    /// the listing before it acts.
    pub fn stat(&self, p: &StoragePath, pick: bool) -> Result<FsStat, FsErr> {
        let Some((folder, name)) = p.split_last() else {
            return fstat_fd(self.root.as_raw_fd());
        };
        let dir = self.open_dir(&folder, pick)?;
        let c = CString::new(name).map_err(|_| FsErr::NotFound)?;
        stat_at(dir.as_raw_fd(), &c)
    }

    /// The size of the storage in bytes, None when it cannot tell.
    pub fn total_bytes(&self) -> Option<u64> {
        // SAFETY: sv is written by fstatvfs before it is read.
        let mut sv: libc::statvfs = unsafe { std::mem::zeroed() };
        (unsafe { libc::fstatvfs(self.root.as_raw_fd(), &mut sv) } == 0).then(|| sv.f_blocks as u64 * sv.f_frsize as u64)
    }

    /// The bytes free on the storage for this user (statvfs's f_bavail), None when it cannot tell.
    pub fn free_bytes(&self) -> Option<u64> {
        // SAFETY: sv is written by fstatvfs before it is read.
        let mut sv: libc::statvfs = unsafe { std::mem::zeroed() };
        (unsafe { libc::fstatvfs(self.root.as_raw_fd(), &mut sv) } == 0).then(|| sv.f_bavail as u64 * sv.f_frsize as u64)
    }
}

fn stat_at(dirfd: i32, name: &CString) -> Result<FsStat, FsErr> {
    // SAFETY: st is written by fstatat before it is read; name is NUL-terminated.
    let mut st: libc::stat = unsafe { std::mem::zeroed() };
    if unsafe { libc::fstatat(dirfd, name.as_ptr(), &mut st, libc::AT_SYMLINK_NOFOLLOW) } != 0 {
        return Err(if errno() == libc::ENOENT { FsErr::NotFound } else { FsErr::Io(errno()) });
    }
    Ok(FsStat::of(&st))
}

/// A job that writes `need` bytes leaves the floor (100 MB) free.
pub fn fsview_room_for(free: u64, need: u64) -> bool {
    free.saturating_sub(need) >= FS_FREE_FLOOR && free >= need
}

/// Files says "Kindle storage nearly full." under 200 MB.
pub fn fsview_nearly_full(free: u64) -> bool {
    free < FS_FREE_LOW
}

/// A folder being listed a slice at a time: names and kinds (d_type, no stat) up to 5,000, the deny-list's
/// entries and links left out; its folder kept open between ticks for the stats of the rows on screen.
pub struct FsListing {
    dir: *mut libc::DIR,
    pub path: StoragePath,
    pub entries: Vec<FsEntry>,
    /// entries left out: the deny-list's, links, and names that would make a path of 600 bytes or more
    pub hidden: usize,
    /// of those, the names whose path would be too long ("Path too long.")
    pub too_long: usize,
    /// stopped at FS_LIST_MAX
    pub capped: bool,
    /// the names of the links left out (counted in hidden too), not in a refused zone, at most FS_LIST_MAX: Files'
    /// Show all lists them as "link, not followed" (2.2), never entered or opened
    pub links: Vec<Vec<u8>>,
    pub done: bool,
    grimoire: Option<StoragePath>,
    pick: bool,
    spent_ms: u32,
}

impl Drop for FsListing {
    fn drop(&mut self) {
        if !self.dir.is_null() {
            // SAFETY: dir came from fdopendir and is closed once, here.
            unsafe { libc::closedir(self.dir) };
        }
    }
}

impl FsListing {
    /// Opens the folder `p` by the walk; nothing is read yet (step reads).
    pub fn start(st: &FsStorage, p: &StoragePath, pick: bool) -> Result<FsListing, FsErr> {
        let fd = st.open_dir(p, pick)?;
        let raw = std::os::fd::IntoRawFd::into_raw_fd(fd);
        // SAFETY: raw is an open folder descriptor; fdopendir takes it over (closedir closes it).
        let dir = unsafe { libc::fdopendir(raw) };
        if dir.is_null() {
            let e = errno();
            // SAFETY: fdopendir failed, so raw is still ours to close.
            unsafe { libc::close(raw) };
            return Err(FsErr::Io(e));
        }
        Ok(FsListing {
            dir,
            path: p.clone(),
            entries: Vec::new(),
            hidden: 0,
            too_long: 0,
            capped: false,
            links: Vec::new(),
            done: false,
            grimoire: st.grimoire.clone(),
            pick,
            spent_ms: 0,
        })
    }

    /// Reads entries for at most `budget_ms` (FS_SLICE_MS a tick); true once the folder is read.
    pub fn step(&mut self, budget_ms: u32) -> bool {
        self.step_clocked(budget_ms, &mut real_now_ms)
    }

    /// The same by `clock` (the tests' own).
    pub fn step_clocked(&mut self, budget_ms: u32, clock: &mut dyn FnMut() -> u32) -> bool {
        if self.done {
            return true;
        }
        let t0 = clock();
        loop {
            if self.entries.len() >= FS_LIST_MAX {
                self.capped = true;
                self.finish();
                break;
            }
            // SAFETY: dir is open; readdir's entry is read before the next call.
            let e = unsafe { libc::readdir(self.dir) };
            if e.is_null() {
                self.finish();
                break;
            }
            // SAFETY: e points at a dirent whose d_name is NUL-terminated.
            let (name, dtype) = unsafe { (std::ffi::CStr::from_ptr((*e).d_name.as_ptr()).to_bytes().to_vec(), (*e).d_type) };
            if name != b"." && name != b".." {
                self.take(name, dtype);
            }
            if clock().wrapping_sub(t0) >= budget_ms {
                break;
            }
        }
        self.spent_ms = self.spent_ms.wrapping_add(clock().wrapping_sub(t0));
        self.done
    }

    fn take(&mut self, name: Vec<u8>, dtype: u8) {
        let kind = match dtype {
            libc::DT_DIR => FsKind::Dir,
            libc::DT_REG => FsKind::File,
            libc::DT_LNK => FsKind::Link,
            libc::DT_UNKNOWN => FsKind::Unknown,
            _ => FsKind::Other,
        };
        let joined = self.path.join(&name);
        if joined.is_none() {
            self.too_long += 1;
        }
        let refused = joined.is_none_or(|p| zone_of(self.grimoire.as_ref(), &p, self.pick) == FsZone::Refused);
        if kind == FsKind::Link || refused {
            self.hidden += 1;
            if kind == FsKind::Link && !refused && self.links.len() < FS_LIST_MAX {
                self.links.push(name);
            }
            return;
        }
        self.entries.push(FsEntry { name, kind, stat: None });
    }

    fn finish(&mut self) {
        self.done = true;
        // (counts and timings only: never a name)
        qlog!("files: listed %d entries (%d usable) in %u ms", (self.entries.len() + self.hidden) as i32, self.entries.len() as i32, self.spent_ms);
    }

    /// Stats the entries in `range` that have none yet (fstatat on the open folder, no link followed), for at
    /// most `budget_ms`; true once each has one. A kind d_type left unknown comes from it.
    pub fn stat(&mut self, range: std::ops::Range<usize>, budget_ms: u32) -> bool {
        self.stat_clocked(range, budget_ms, &mut real_now_ms)
    }

    pub fn stat_clocked(&mut self, range: std::ops::Range<usize>, budget_ms: u32, clock: &mut dyn FnMut() -> u32) -> bool {
        let t0 = clock();
        let end = range.end.min(self.entries.len());
        // SAFETY: dir is open while self lives.
        let dirfd = unsafe { libc::dirfd(self.dir) };
        for i in range.start..end {
            if self.entries[i].stat.is_some() {
                continue;
            }
            if clock().wrapping_sub(t0) >= budget_ms {
                return false;
            }
            let Ok(c) = CString::new(self.entries[i].name.clone()) else { continue };
            if let Ok(st) = stat_at(dirfd, &c) {
                let e = &mut self.entries[i];
                e.kind = st.kind();
                e.stat = Some(st);
            }
        }
        true
    }
}

// ---- writes (2.2, issue #117): only inside Grimoire's folders for the user's files ----

/// The record beside a trashed file or folder: `trash/files-<YYYYMMDD-HHMMSS>-<4 hex>/.grimoire-trash`.
pub const TRASH_RECORD: &str = ".grimoire-trash";
/// A record longer than this is not read (it holds one path under 600 bytes and a time).
pub const TRASH_RECORD_MAX: usize = 4096;
/// A name is at most this many bytes (browser.md 8.8).
pub const FS_NAME_MAX: usize = 255;
/// The folder a trashed file or folder goes into begins so: `files-<YYYYMMDD-HHMMSS>-<4 hex>`.
const TRASH_FILES: &[u8] = b"files-";
/// Where a restore finds its name taken: " 2" up to this; every one taken, it refuses.
const NUMBERED_MAX: u32 = 99;

/// Why a write was refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FsWriteErr {
    /// outside what is inside Grimoire's writable folders, one of Grimoire's own names there, a page in the
    /// trash, or an entry the trash does not know
    Refused,
    /// the walk to it failed (a link, another device, a folder that is not there...)
    Walk(FsErr),
    /// it is no longer the item listed: device, inode, size or time ("File changed. Try again.")
    Changed,
    /// an import holds it ("Importing now.")
    Busy,
    /// the new name breaks a rule
    Name(NameRule),
    /// an entry of that name is there: nothing is ever replaced
    Exists,
    /// the path would be 600 bytes or more, or a name longer than 255
    TooLong,
    /// restore: the folder it came from is gone
    FolderGone,
    /// restore: its record is not there or does not read (strictly)
    BadRecord,
    /// copy: less than the floor would be left
    NoRoom,
    /// copy: the file is bigger than the folder it would go into takes ("Too big (over %d MB).")
    TooBig,
    /// a call failed with this errno
    Io(i32),
}

/// The rule a new name breaks (`fsview_name_check`): browser.md 8.8, the strictest of vfat and MTP.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NameRule {
    Empty,
    /// longer than 255 bytes
    TooLong,
    /// one of / \ : * ? " < > |, the first in the name ("Name can't contain %s")
    Char(u8),
    /// a control character
    Control,
    LeadingSpace,
    TrailingSpace,
    TrailingDot,
    /// a name beginning with a dot is hidden: Files never lists it
    LeadingDot,
    /// a file's extension changed or went (the keyboard edits the stem)
    Extension,
}

/// The rules a new name keeps, as one function: not empty, at most 255 bytes, none of / \ : * ? " < > |, no
/// control character, no leading or trailing space, no trailing dot, no leading dot, and for a file (`dir`
/// false) the extension `old` has, in any case. The first rule broken.
pub fn fsview_name_check(old: &[u8], new: &[u8], dir: bool) -> Result<(), NameRule> {
    let (Some(&first), Some(&last)) = (new.first(), new.last()) else { return Err(NameRule::Empty) };
    if new.len() > FS_NAME_MAX {
        return Err(NameRule::TooLong);
    }
    if let Some(&c) = new.iter().find(|c| b"/\\:*?\"<>|".contains(c)) {
        return Err(NameRule::Char(c));
    }
    if new.iter().any(|&c| c < 0x20 || c == 0x7f) {
        return Err(NameRule::Control);
    }
    if first == b' ' {
        return Err(NameRule::LeadingSpace);
    }
    if last == b' ' {
        return Err(NameRule::TrailingSpace);
    }
    if last == b'.' {
        return Err(NameRule::TrailingDot);
    }
    if first == b'.' {
        return Err(NameRule::LeadingDot);
    }
    if let (false, Some(ext)) = (dir, extension(old)) {
        if !extension(new).is_some_and(|e| e.eq_ignore_ascii_case(ext)) {
            return Err(NameRule::Extension);
        }
    }
    Ok(())
}

/// A name's extension: what follows its last dot, when that dot is neither its first byte nor its last.
fn extension(name: &[u8]) -> Option<&[u8]> {
    let i = name.iter().rposition(|&c| c == b'.')?;
    (i > 0 && i + 1 < name.len()).then(|| &name[i + 1..])
}

/// `name` with " n" before a file's extension ("a 2.png"), or at the end of a folder's name ("Work 2").
pub(crate) fn numbered(name: &[u8], n: u32, dir: bool) -> Vec<u8> {
    let at = if dir { None } else { extension(name).map(|e| name.len() - e.len() - 1) };
    let (stem, ext) = name.split_at(at.unwrap_or(name.len()));
    [stem, format!(" {n}").as_bytes(), ext].concat()
}

/// Whether two paths are one, or one is inside the other (components compared without regard to ASCII case,
/// as a case-insensitive storage finds them).
fn related(a: &[u8], b: &[u8]) -> bool {
    let (x, y): (Vec<&[u8]>, Vec<&[u8]>) = (a.split(|&c| c == b'/').collect(), b.split(|&c| c == b'/').collect());
    x.iter().zip(&y).all(|(p, q)| eq_nocase(p, q))
}

/// Pure: whether an import holds `p` (a storage path's bytes). `held`: the paths the imports queued or running
/// work on (a PDF, a folder of pictures); one of them is `p`, or inside `p`, or `p` is inside one. `detached`: a
/// previous run's import still holds the lock, and what it works on is not known here, so all of `import` (Grimoire's
/// import folder) counts as held.
fn holds(p: &[u8], held: &[Vec<u8>], detached: bool, import: Option<&[u8]>) -> bool {
    held.iter().any(|h| !h.is_empty() && related(p, h)) || (detached && import.is_some_and(|i| related(p, i)))
}

/// What a trash entry is, by its name alone (no stat).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrashKind {
    /// a file or folder of Grimoire's folders, in `files-<stamp>-<4 hex>/` with its record (2.2)
    File,
    /// a notebook, `<id>-<time>` (store_delete, every version)
    Notebook,
    /// a page, `<id>-p<page id>-<time>` (store_delete_page): listed only, its place in the notebook is kept nowhere
    Page,
    /// a name of none of these forms
    Other,
}

/// One entry of the trash, read from its name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrashEntry {
    /// its name in the trash
    pub name: Vec<u8>,
    pub kind: TrashKind,
    /// when it went there, unix seconds, from its name (0 for Other)
    pub time: i64,
    /// a notebook's id (a page's: its notebook's)
    pub id: String,
    /// a page's id
    pub page: u32,
}

fn digits(s: &[u8]) -> bool {
    !s.is_empty() && s.iter().all(u8::is_ascii_digit)
}

/// A notebook id as the store takes it (store.rs's rule: letters, digits, '-' and '_', under 24 bytes).
fn notebook_id_ok(id: &str) -> bool {
    store_notebook_dir(id, &mut String::new(), FS_PATH_MAX) == 0
}

/// The unix time of a local `YYYYMMDD-HHMMSS` (a trash wrapper's stamp), None for no such time.
#[allow(deprecated)] // (libc's time_t on 32-bit musl, as util.rs's local_tm)
fn stamp_time(s: &[u8]) -> Option<i64> {
    if s.len() != 15 || s[8] != b'-' || !digits(&s[..8]) || !digits(&s[9..]) {
        return None;
    }
    let n = |a: usize, b: usize| std::str::from_utf8(&s[a..b]).ok().and_then(|t| t.parse::<i32>().ok());
    let (y, mo, d, h, mi, se) = (n(0, 4)?, n(4, 6)?, n(6, 8)?, n(9, 11)?, n(11, 13)?, n(13, 15)?);
    if !(1..=12).contains(&mo) || !(1..=31).contains(&d) || h > 23 || mi > 59 || se > 60 {
        return None;
    }
    // SAFETY: tm is plain data, zeroed then filled; mktime reads and normalizes our local.
    unsafe {
        let mut tm: libc::tm = std::mem::zeroed();
        tm.tm_year = y - 1900;
        tm.tm_mon = mo - 1;
        tm.tm_mday = d;
        tm.tm_hour = h;
        tm.tm_min = mi;
        tm.tm_sec = se;
        tm.tm_isdst = -1;
        let t = libc::mktime(&mut tm);
        (t != -1).then_some(t as i64)
    }
}

/// A trash entry from its name: `files-<YYYYMMDD-HHMMSS>-<4 hex>` (a file or folder, 2.2), `<id>-<time>` (a
/// notebook) or `<id>-p<page id>-<time>` (a page), else Other.
fn trash_parse(name: &[u8]) -> TrashEntry {
    let other = TrashEntry { name: name.to_vec(), kind: TrashKind::Other, time: 0, id: String::new(), page: 0 };
    if let Some(rest) = name.strip_prefix(TRASH_FILES) {
        let hex = |s: &[u8]| s.len() == 4 && s.iter().all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(c));
        return match (rest.len() == 20 && rest[15] == b'-' && hex(&rest[16..])).then(|| stamp_time(&rest[..15])).flatten() {
            Some(time) => TrashEntry { kind: TrashKind::File, time, ..other },
            None => other,
        };
    }
    let Some((head, t)) = std::str::from_utf8(name).ok().and_then(|s| s.rsplit_once('-')) else { return other };
    let Some(time) = (digits(t.as_bytes()) && t.len() <= 18).then(|| t.parse::<i64>().ok()).flatten() else { return other };
    if let Some((id, p)) = head.rsplit_once('-') {
        let page = p.strip_prefix('p').filter(|d| digits(d.as_bytes()) && d.len() <= 9).and_then(|d| d.parse::<u32>().ok());
        if let Some(page) = page.filter(|&n| n > 0 && notebook_id_ok(id)) {
            return TrashEntry { kind: TrashKind::Page, time, id: id.to_string(), page, ..other };
        }
    }
    if notebook_id_ok(head) {
        return TrashEntry { kind: TrashKind::Notebook, time, id: head.to_string(), ..other };
    }
    other
}

/// The trash's entries from a listing of it (FsListing over `FsStorage::trash_dir`, in its slices), each read
/// from its name alone, newest first (those of no known form last), names in byte order where times are equal.
pub fn fsview_trash_entries(entries: &[FsEntry]) -> Vec<TrashEntry> {
    let mut v: Vec<TrashEntry> = entries.iter().map(|e| trash_parse(&e.name)).collect();
    v.sort_by(|a, b| ((a.kind == TrashKind::Other), std::cmp::Reverse(a.time), &a.name).cmp(&((b.kind == TrashKind::Other), std::cmp::Reverse(b.time), &b.name)));
    v
}

/// A trashed file's or folder's record: where it was (a storage path) and when it went (unix seconds).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrashRecord {
    pub from: StoragePath,
    pub time: i64,
}

fn trash_record_text(from: &StoragePath, time: i64) -> Vec<u8> {
    let mut t = b"# Grimoire: where this went from, for Restore (docs/FORMAT.md, The trash)\nfrom=".to_vec();
    t.extend_from_slice(from.as_bytes());
    t.extend_from_slice(format!("\ntime={time}\n").as_bytes());
    t
}

/// A record read strictly: at most TRASH_RECORD_MAX bytes, every line ended by a line feed (no carriage return),
/// a line that begins with '#' passed over, `from=` (a storage path, not the root) and `time=` (digits) once each,
/// and no other line. None for anything else.
fn trash_record_parse(t: &[u8]) -> Option<TrashRecord> {
    if t.len() > TRASH_RECORD_MAX || t.last() != Some(&b'\n') || t.contains(&b'\r') || t.contains(&0) {
        return None;
    }
    let (mut from, mut time) = (None, None);
    for line in t[..t.len() - 1].split(|&c| c == b'\n') {
        if line.first() == Some(&b'#') {
            continue;
        }
        if let Some(v) = line.strip_prefix(b"from=") {
            if from.is_some() {
                return None;
            }
            from = Some(StoragePath::new(v).filter(|p| !p.is_root())?);
        } else if let Some(v) = line.strip_prefix(b"time=") {
            if time.is_some() || !digits(v) || v.len() > 18 {
                return None;
            }
            time = Some(std::str::from_utf8(v).ok()?.parse::<i64>().ok()?);
        } else {
            return None;
        }
    }
    Some(TrashRecord { from: from?, time: time? })
}

#[cfg(any(test, feature = "sim"))]
static FORCE_FALLBACK: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Tests (never the Kindle's build): every rename takes the fallback (the new name reserved, then renameat over
/// the reservation), as on a storage that refuses renameat2's flag, so a run on Linux tries it too.
#[cfg(any(test, feature = "sim"))]
pub fn fsview_rename_fallback_force(on: bool) {
    FORCE_FALLBACK.store(on, std::sync::atomic::Ordering::Relaxed);
}

#[cfg(target_os = "linux")]
fn fallback_forced() -> bool {
    #[cfg(any(test, feature = "sim"))]
    if FORCE_FALLBACK.load(std::sync::atomic::Ordering::Relaxed) {
        return true;
    }
    false
}

/// What a test runs in the fallback between its check and its renameat (fsview_rename_seam).
#[cfg(any(test, feature = "sim"))]
type RenameSeam = Box<dyn FnOnce() + Send>;
#[cfg(any(test, feature = "sim"))]
static RENAME_SEAM: std::sync::Mutex<Option<RenameSeam>> = std::sync::Mutex::new(None);

/// Tests (never the Kindle's build): `f` runs once, in the next fallback rename, between its check (the new name
/// reserved, or looked at for a folder of the trash's own) and its renameat: what another writer does at that
/// moment.
#[cfg(any(test, feature = "sim"))]
pub fn fsview_rename_seam(f: Option<RenameSeam>) {
    *RENAME_SEAM.lock().unwrap_or_else(|e| e.into_inner()) = f;
}

fn rename_seam() {
    #[cfg(any(test, feature = "sim"))]
    {
        let f = RENAME_SEAM.lock().unwrap_or_else(|e| e.into_inner()).take();
        if let Some(f) = f {
            f();
        }
    }
}

/// The moments a test can stop a write at, as a crash would (fsview_stop_at): nothing after it runs, no undo.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FsStop {
    /// a rename through a temporary name, between its two renames
    RenameHalfway = 1,
    /// the trash: its record written, the item not moved yet
    TrashRecorded = 2,
}

#[cfg(any(test, feature = "sim"))]
static STOP_AT: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);

/// Tests (never the Kindle's build): the next writes stop at `at` (None: nowhere), returning Io(EINTR).
#[cfg(any(test, feature = "sim"))]
pub fn fsview_stop_at(at: Option<FsStop>) {
    STOP_AT.store(at.map_or(0, |s| s as u8), std::sync::atomic::Ordering::Relaxed);
}

fn stopped(at: FsStop) -> bool {
    #[cfg(any(test, feature = "sim"))]
    if STOP_AT.load(std::sync::atomic::Ordering::Relaxed) == at as u8 {
        return true;
    }
    let _ = at;
    false
}

/// Renames `old` in `ofd` to `new` in `nfd`, never over an entry: renameat2(RENAME_NOREPLACE) on Linux; where the
/// storage refuses the flag or the kernel has no such call (EINVAL, ENOSYS), and on the Mac, the fallback
/// (`rename_fallback`) reserves `new` first, atomically, and renames over its own reservation only. Err(errno):
/// EEXIST when taken.
pub(crate) fn rename_noreplace(ofd: i32, old: &CString, nfd: i32, new: &CString) -> Result<(), i32> {
    rename_at(ofd, old, nfd, new, true)
}

fn rename_at(ofd: i32, old: &CString, nfd: i32, new: &CString, reserve: bool) -> Result<(), i32> {
    #[cfg(target_os = "linux")]
    if !fallback_forced() {
        // SAFETY: renameat2 through syscall (a C library may lack the wrapper) on our descriptors and two
        // NUL-terminated names.
        let r = unsafe { libc::syscall(libc::SYS_renameat2, ofd, old.as_ptr(), nfd, new.as_ptr(), libc::RENAME_NOREPLACE) };
        if r == 0 {
            return Ok(());
        }
        let e = errno();
        if e != libc::EINVAL && e != libc::ENOSYS {
            return Err(e);
        }
        static NOTED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
        if !NOTED.swap(true, std::sync::atomic::Ordering::Relaxed) {
            qlog!("files: the storage refused the no-replace rename (%s); a rename reserves the name first", &strerror(e));
        }
    }
    rename_fallback(ofd, old, nfd, new, reserve)
}

/// The fallback. Grimoire is not the only writer while it runs (files come over USB, the copy job renames into the
/// same folders), so a look for the name and then renameat could replace what came in between. Instead the new
/// name is reserved, atomically: an O_EXCL create for a file, mkdirat for a folder, either of which fails when
/// anything is there. Just before renameat over it, the reservation must still be its own (the same device and
/// inode, a file still empty; a folder the kernel will not replace once something is in it). On any failure the
/// reservation goes, only while it is still its own. `reserve` false: a folder the caller made a moment ago (the
/// trash's own, under a random name), where a look is enough and a crash can leave no empty file beside the item.
fn rename_fallback(ofd: i32, old: &CString, nfd: i32, new: &CString, reserve: bool) -> Result<(), i32> {
    // SAFETY: a stat is plain data; zeroed, then written by fstatat before it is read.
    let mut st: libc::stat = unsafe { std::mem::zeroed() };
    if !reserve {
        // SAFETY: fstatat on our descriptor and a NUL-terminated name, into st.
        if unsafe { libc::fstatat(nfd, new.as_ptr(), &mut st, libc::AT_SYMLINK_NOFOLLOW) } == 0 {
            return Err(libc::EEXIST);
        }
        let e = errno();
        if e != libc::ENOENT {
            return Err(e);
        }
        rename_seam();
        // SAFETY: renameat on our descriptors and two NUL-terminated names.
        return if unsafe { libc::renameat(ofd, old.as_ptr(), nfd, new.as_ptr()) } == 0 { Ok(()) } else { Err(errno()) };
    }
    // SAFETY: as above, for the item.
    if unsafe { libc::fstatat(ofd, old.as_ptr(), &mut st, libc::AT_SYMLINK_NOFOLLOW) } != 0 {
        return Err(errno());
    }
    let dir = st.st_mode & libc::S_IFMT == libc::S_IFDIR;
    let mine = reserve_name(nfd, new, dir)?;
    rename_seam();
    if !still_reserved(nfd, new, mine, dir) {
        return Err(libc::EEXIST);
    }
    // SAFETY: renameat on our descriptors and two NUL-terminated names.
    if unsafe { libc::renameat(ofd, old.as_ptr(), nfd, new.as_ptr()) } == 0 {
        return Ok(());
    }
    let e = errno();
    if still_reserved(nfd, new, mine, dir) {
        unlink_at(nfd, new, if dir { libc::AT_REMOVEDIR } else { 0 });
    }
    Err(e)
}

/// Makes `new` in `nfd`, a folder or an empty file, failing when anything is there: its device and inode.
fn reserve_name(nfd: i32, new: &CString, dir: bool) -> Result<(u64, u64), i32> {
    // SAFETY: a stat is plain data; zeroed, then written before it is read.
    let mut st: libc::stat = unsafe { std::mem::zeroed() };
    if dir {
        // SAFETY: mkdirat on our descriptor and a NUL-terminated name.
        if unsafe { libc::mkdirat(nfd, new.as_ptr(), 0o755) } != 0 {
            return Err(errno());
        }
        // SAFETY: fstatat on our descriptor and a NUL-terminated name, into st.
        if unsafe { libc::fstatat(nfd, new.as_ptr(), &mut st, libc::AT_SYMLINK_NOFOLLOW) } != 0 {
            return Err(errno());
        }
    } else {
        // SAFETY: openat on our descriptor and a NUL-terminated name; the descriptor is closed here, after fstat
        // (closed before the rename, so no storage renames over a file still open).
        let fd = unsafe { libc::openat(nfd, new.as_ptr(), libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC, 0o600 as libc::c_uint) };
        if fd < 0 {
            return Err(errno());
        }
        // SAFETY: fstat on the descriptor just opened, into st.
        let r = unsafe { libc::fstat(fd, &mut st) };
        let e = errno();
        // SAFETY: closes the descriptor this function opened, once.
        unsafe { libc::close(fd) };
        if r != 0 {
            unlink_at(nfd, new, 0);
            return Err(e);
        }
    }
    Ok((st.st_dev as u64, st.st_ino as u64))
}

/// Whether `new` in `nfd` is still the reservation `mine`: the same device and inode, a folder, or a file nothing
/// has written into.
fn still_reserved(nfd: i32, new: &CString, mine: (u64, u64), dir: bool) -> bool {
    // SAFETY: a stat is plain data; zeroed, then written by fstatat before it is read.
    let mut st: libc::stat = unsafe { std::mem::zeroed() };
    // SAFETY: fstatat on our descriptor and a NUL-terminated name, into st.
    if unsafe { libc::fstatat(nfd, new.as_ptr(), &mut st, libc::AT_SYMLINK_NOFOLLOW) } != 0 {
        return false;
    }
    let kind = st.st_mode & libc::S_IFMT;
    (st.st_dev as u64, st.st_ino as u64) == mine && if dir { kind == libc::S_IFDIR } else { kind == libc::S_IFREG && st.st_size == 0 }
}

fn cname(name: &[u8]) -> Result<CString, FsWriteErr> {
    CString::new(name).map_err(|_| FsWriteErr::Refused)
}

/// Four hex digits from the system's randomness (the clock and a count where there is none).
fn hex4() -> String {
    let mut b = [0u8; 2];
    if !crate::util::random_fill(&mut b) {
        static COUNT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let x = real_now_ms() ^ std::process::id().rotate_left(8) ^ COUNT.fetch_add(0x9e37, std::sync::atomic::Ordering::Relaxed);
        b = [(x >> 8) as u8, x as u8];
    }
    format!("{:02x}{:02x}", b[0], b[1])
}

/// Best effort: what was renamed in or out of the folder open at `fd` is on the storage. The pens are read after
/// it (work_pump, as util's atomic write does between its steps: on the Kindle each fsync waits 15-25 ms); in a
/// background child that does nothing.
pub(crate) fn sync_dir(fd: i32) {
    // SAFETY: fsync on a descriptor the caller owns.
    unsafe { libc::fsync(fd) };
    crate::util::work_pump();
}

/// Opens `name` in `dirfd` as a folder (O_DIRECTORY|O_NOFOLLOW).
fn open_sub(dirfd: i32, name: &CString) -> Result<OwnedFd, i32> {
    // SAFETY: openat on our descriptor and a NUL-terminated name; the descriptor returned is owned by the OwnedFd.
    let fd = unsafe { libc::openat(dirfd, name.as_ptr(), libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC) };
    if fd < 0 {
        return Err(errno());
    }
    // SAFETY: fd was just opened and is owned by no one else.
    Ok(unsafe { OwnedFd::from_raw_fd(fd) })
}

fn unlink_at(dirfd: i32, name: &CString, flags: i32) {
    // SAFETY: unlinkat on our descriptor and a NUL-terminated name.
    unsafe { libc::unlinkat(dirfd, name.as_ptr(), flags) };
}

/// Writes a trashed item's record into the new folder open at `wfd`: created there (O_CREAT|O_EXCL|O_NOFOLLOW, the
/// descriptor form of create_fresh_nofollow: the folder is new, so nothing stands at the name), written whole and
/// synced, or removed again.
fn write_record(wfd: i32, from: &StoragePath, time: i64) -> Result<(), i32> {
    let c = CString::new(TRASH_RECORD).map_err(|_| libc::EINVAL)?;
    // SAFETY: openat on our descriptor and a NUL-terminated name; the descriptor is closed below.
    let fd = unsafe { libc::openat(wfd, c.as_ptr(), libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC, 0o644 as libc::c_uint) };
    if fd < 0 {
        return Err(errno());
    }
    // SAFETY: fd was just opened and is owned by no one else; the File closes it.
    let mut f = unsafe { std::fs::File::from_raw_fd(fd) };
    use std::io::Write;
    let r = f.write_all(&trash_record_text(from, time)).and_then(|_| f.sync_all());
    drop(f);
    crate::util::work_pump();
    r.map_err(|e| {
        unlink_at(wfd, &c, 0);
        e.raw_os_error().unwrap_or(libc::EIO)
    })
}

/// Reads the record in the folder open at `wfd`: a plain file at the name (O_NOFOLLOW, and O_NONBLOCK so nothing
/// else ever waits), at most TRASH_RECORD_MAX bytes (the descriptor form of open_read_nofollow and its limit), read
/// strictly.
fn read_record(wfd: i32) -> Option<TrashRecord> {
    use std::io::Read;
    let c = CString::new(TRASH_RECORD).ok()?;
    // SAFETY: openat on our descriptor and a NUL-terminated name; the descriptor is owned by the File.
    let fd = unsafe { libc::openat(wfd, c.as_ptr(), libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_NOCTTY | libc::O_CLOEXEC) };
    if fd < 0 {
        return None;
    }
    // SAFETY: fd was just opened and is owned by no one else.
    let f = unsafe { std::fs::File::from_raw_fd(fd) };
    let md = f.metadata().ok()?;
    if !md.is_file() || md.len() > TRASH_RECORD_MAX as u64 {
        return None;
    }
    let mut t = Vec::new();
    f.take(TRASH_RECORD_MAX as u64 + 1).read_to_end(&mut t).ok()?;
    trash_record_parse(&t)
}

/// What a restore put back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Restored {
    /// a file or folder, at this storage path now
    File(StoragePath),
    /// a notebook, as `notebooks/<id>` again
    Notebook(String),
    /// nothing: the entry held nothing but its record (a trash cut off before its item moved, so the item never
    /// left its folder) or nothing at all, and is gone from the trash
    Nothing,
}

/// Whether the folder open at `wfd` holds nothing but a trash record (or nothing at all). False at any other entry,
/// or when it cannot be read.
fn holds_only_record(wfd: i32) -> bool {
    // SAFETY: dup of our descriptor; fdopendir takes the duplicate and closedir closes it; each entry's name is read
    // before the next readdir.
    unsafe {
        let d2 = libc::dup(wfd);
        if d2 < 0 {
            return false;
        }
        let dir = libc::fdopendir(d2);
        if dir.is_null() {
            libc::close(d2);
            return false;
        }
        let mut only = true;
        loop {
            crate::util::set_errno(0);
            let e = libc::readdir(dir);
            if e.is_null() {
                // (the end, or a read that failed: then nothing is known, and nothing is removed)
                only = only && errno() == 0;
                break;
            }
            let n = std::ffi::CStr::from_ptr((*e).d_name.as_ptr()).to_bytes();
            if n != b"." && n != b".." && n != TRASH_RECORD.as_bytes() {
                only = false;
                break;
            }
        }
        libc::closedir(dir);
        only
    }
}

/// An errno of a rename as a refusal: taken, gone since it was looked at, or another failure.
fn rename_err(e: i32) -> FsWriteErr {
    match e {
        libc::EEXIST | libc::ENOTEMPTY => FsWriteErr::Exists,
        libc::ENOENT => FsWriteErr::Changed,
        _ => FsWriteErr::Io(e),
    }
}

fn log_failed(what: &str, e: FsWriteErr) -> FsWriteErr {
    if let FsWriteErr::Io(n) = e {
        qlog!("files: %s failed: %s", what, &strerror(n));
    }
    e
}

impl FsStorage {
    /// Which of Grimoire's folders for the user's files `p` is inside ("import", "templates", "photos", "fonts"),
    /// None when it may not be written: the allow-list. Only what is inside one of them, never the folder itself,
    /// nor `import/done` (where imports file their originals), nor a dot-name or anything else the zones refuse.
    fn write_zone(&self, p: &StoragePath) -> Option<&'static str> {
        let g = self.grimoire.as_ref()?;
        if zone_of(Some(g), p, false) != FsZone::Writable {
            return None;
        }
        let gl = g.components().count();
        let comps: Vec<&[u8]> = p.components().collect();
        if comps.len() < gl + 2 || (comps.len() == gl + 2 && eq_nocase(comps[gl], b"import") && eq_nocase(comps[gl + 1], b"done")) {
            return None;
        }
        ["import", "templates", "photos", "fonts"].into_iter().find(|w| eq_nocase(comps[gl], w.as_bytes()))
    }

    /// Whether an import holds `p` (browser.md 8.14, "Importing now."): a PDF import or a picture import queued or
    /// running in this process works on it, inside it, or on the folder it is in (the job table's paths, and the
    /// PDF import's own list); or a previous run's import still holds the lock (pdfimport's detached state), and
    /// then everything in Grimoire's import folder counts as held, since what that one works on is not known here.
    pub fn busy(&self, p: &StoragePath) -> bool {
        if std::str::from_utf8(p.as_bytes()).is_ok_and(crate::pdfimport::is_queued) {
            return true;
        }
        let g = self.grimoire.as_ref().map(|g| g.as_bytes().to_vec());
        let own = store_root();
        let held: Vec<Vec<u8>> = bg_job_paths(&[BgKind::PdfImport, BgKind::ImportFolder])
            .into_iter()
            .filter_map(|j| match j.strip_prefix(own.trim_end_matches('/')).and_then(|r| r.strip_prefix('/')) {
                // (a folder of import/ a scan queued: its absolute path, under Grimoire's own folder)
                Some(rel) if j.starts_with('/') => g.as_ref().map(|g| [&g[..], b"/", rel.as_bytes()].concat()),
                None if j.starts_with('/') => None,
                // (a PDF, a folder picked in place: a storage path)
                _ => Some(j.into_bytes()),
            })
            .collect();
        let import = g.map(|g| [&g[..], b"/import"].concat());
        holds(p.as_bytes(), &held, crate::pdfimport::detached_running(), import.as_deref())
    }

    /// Grimoire's trash as a storage path (to list it with FsListing), None when Grimoire's folder is not inside
    /// the root.
    pub fn trash_dir(&self) -> Option<StoragePath> {
        self.grimoire.as_ref()?.join(b"trash")
    }

    /// The trash, opened by the walk and known by its identity.
    fn trash_fd(&self) -> Result<OwnedFd, FsWriteErr> {
        let p = self.trash_dir().ok_or(FsWriteErr::Refused)?;
        let fd = self.open_dir(&p, false).map_err(FsWriteErr::Walk)?;
        let st = fstat_fd(fd.as_raw_fd()).map_err(FsWriteErr::Walk)?;
        if self.trash != Some((st.dev, st.ino)) {
            return Err(FsWriteErr::Refused);
        }
        Ok(fd)
    }

    /// Grimoire's notebooks folder, opened from its own folder (by the walk) and known by its identity: the one
    /// place a restore writes outside the allow-list, and only a notebook's folder, by its id.
    fn notebooks_fd(&self) -> Result<OwnedFd, FsWriteErr> {
        let g = self.grimoire.as_ref().ok_or(FsWriteErr::Refused)?;
        let gfd = self.open_dir(g, false).map_err(FsWriteErr::Walk)?;
        let fd = open_sub(gfd.as_raw_fd(), &cname(b"notebooks")?).map_err(|e| FsWriteErr::Walk(if e == libc::ENOENT { FsErr::NotFound } else { FsErr::Io(e) }))?;
        let st = fstat_fd(fd.as_raw_fd()).map_err(FsWriteErr::Walk)?;
        if self.notebooks != Some((st.dev, st.ino)) {
            return Err(FsWriteErr::Refused);
        }
        Ok(fd)
    }

    /// Renames the file or folder at `p` (as listed: `listed`) to `new_name` in the same folder, never over an
    /// entry: inside Grimoire's folders for the user's files only, not while an import holds it, only while it is
    /// the item listed, and only to a name the rules allow (`fsview_name_check`). A rename that changes only the
    /// case, or that the storage says would land on the item itself (a storage that folds case), goes through a
    /// temporary name in the same folder, the new name numbered (" 2"; TooLong when that passes 255 bytes), and is
    /// undone when its second step fails. Its new path.
    pub fn rename(&self, p: &StoragePath, listed: &FsStat, new_name: &[u8]) -> Result<StoragePath, FsWriteErr> {
        let zone = self.write_zone(p).ok_or(FsWriteErr::Refused)?;
        let (folder, old) = p.split_last().ok_or(FsWriteErr::Refused)?;
        if self.busy(p) {
            return Err(FsWriteErr::Busy);
        }
        let dir = self.open_dir(&folder, false).map_err(FsWriteErr::Walk)?;
        let d = dir.as_raw_fd();
        let c_old = cname(old)?;
        let now = stat_at(d, &c_old).map_err(|e| if e == FsErr::NotFound { FsWriteErr::Changed } else { FsWriteErr::Walk(e) })?;
        if !now.same(listed) {
            return Err(FsWriteErr::Changed);
        }
        let dir_item = match now.kind() {
            FsKind::Dir => true,
            FsKind::File => false,
            _ => return Err(FsWriteErr::Refused),
        };
        fsview_name_check(old, new_name, dir_item).map_err(FsWriteErr::Name)?;
        let to = folder.join(new_name).ok_or(FsWriteErr::TooLong)?;
        if self.write_zone(&to).is_none() {
            return Err(FsWriteErr::Refused);
        }
        if new_name == old {
            return Ok(to);
        }
        let c_new = cname(new_name)?;
        let folded = new_name.eq_ignore_ascii_case(old);
        let direct = if folded { Err(libc::EEXIST) } else { rename_noreplace(d, &c_old, d, &c_new) };
        match direct {
            Ok(()) => {}
            Err(libc::EEXIST) if folded || stat_at(d, &c_new).is_ok_and(|s| (s.dev, s.ino) == (now.dev, now.ino)) => {
                // The temporary name is the new one numbered (" 2", before a file's extension): a name every
                // listing shows, so a rename cut off half way (a crash, an undo that fails) leaves the item in
                // sight under nearly the name asked for, never hidden with its name kept nowhere.
                let mut parked = None;
                for n in 2..=NUMBERED_MAX {
                    let t = numbered(new_name, n, dir_item);
                    if t.len() > FS_NAME_MAX {
                        return Err(FsWriteErr::TooLong);
                    }
                    let ct = cname(&t)?;
                    match rename_noreplace(d, &c_old, d, &ct) {
                        Ok(()) => {
                            parked = Some(ct);
                            break;
                        }
                        Err(libc::EEXIST) => continue,
                        Err(e) => return Err(log_failed("a rename", rename_err(e))),
                    }
                }
                let tmp = parked.ok_or(FsWriteErr::Exists)?;
                if stopped(FsStop::RenameHalfway) {
                    return Err(FsWriteErr::Io(libc::EINTR));
                }
                if let Err(e) = rename_noreplace(d, &tmp, d, &c_new) {
                    if rename_noreplace(d, &tmp, d, &c_old).is_err() {
                        qlog!("files: a rename through a temporary name could not be undone; the item has the new name numbered, in %s", zone);
                    }
                    return Err(log_failed("a rename", rename_err(e)));
                }
            }
            Err(e) => return Err(log_failed("a rename", rename_err(e))),
        }
        sync_dir(d);
        qlog!("files: an item of %s was renamed", zone);
        Ok(to)
    }

    /// Moves the file or folder at `p` (as listed: `listed`) to the trash, never deleting it: one rename into a new
    /// folder `trash/files-<YYYYMMDD-HHMMSS>-<4 hex>/`, beside its record (`.grimoire-trash`: where it was and
    /// when), written and synced first. Inside Grimoire's folders for the user's files only, not while an import
    /// holds it, only while it is the item listed. Nothing is left behind when it fails. Cut off between its record
    /// and its rename (a crash), the item never left: the trash lists the entry, and Restore removes it
    /// (`Restored::Nothing`). The new entry's name in the trash.
    pub fn trash(&self, p: &StoragePath, listed: &FsStat) -> Result<Vec<u8>, FsWriteErr> {
        let zone = self.write_zone(p).ok_or(FsWriteErr::Refused)?;
        let (folder, name) = p.split_last().ok_or(FsWriteErr::Refused)?;
        // (a record is lines: a path with a line break in it cannot be kept, so it is not trashed)
        if p.as_bytes().iter().any(|&c| c == b'\n' || c == b'\r') {
            return Err(FsWriteErr::Refused);
        }
        if self.busy(p) {
            return Err(FsWriteErr::Busy);
        }
        let dir = self.open_dir(&folder, false).map_err(FsWriteErr::Walk)?;
        let c = cname(name)?;
        let now = stat_at(dir.as_raw_fd(), &c).map_err(|e| if e == FsErr::NotFound { FsWriteErr::Changed } else { FsWriteErr::Walk(e) })?;
        if !now.same(listed) {
            return Err(FsWriteErr::Changed);
        }
        if !matches!(now.kind(), FsKind::File | FsKind::Dir) || now.dev != self.dev {
            return Err(FsWriteErr::Refused);
        }
        let trash = self.trash_fd()?;
        let t = trash.as_raw_fd();
        // a folder of its own, at a name nothing has
        let mut made = None;
        for _ in 0..8 {
            let tm = local_tm(wall_time());
            let w = format!(
                "files-{:04}{:02}{:02}-{:02}{:02}{:02}-{}",
                tm.tm_year + 1900,
                tm.tm_mon + 1,
                tm.tm_mday,
                tm.tm_hour,
                tm.tm_min,
                tm.tm_sec,
                hex4()
            );
            let cw = cname(w.as_bytes())?;
            // SAFETY: mkdirat on our descriptor and a NUL-terminated name.
            if unsafe { libc::mkdirat(t, cw.as_ptr(), 0o755) } == 0 {
                made = Some((w, cw));
                break;
            }
            let e = errno();
            if e != libc::EEXIST {
                return Err(log_failed("the trash", FsWriteErr::Io(e)));
            }
        }
        let Some((w, cw)) = made else { return Err(log_failed("the trash", FsWriteErr::Io(libc::EEXIST))) };
        let undo = |wfd: Option<i32>| {
            if let (Some(wfd), Ok(rc)) = (wfd, cname(TRASH_RECORD.as_bytes())) {
                unlink_at(wfd, &rc, 0);
            }
            unlink_at(t, &cw, libc::AT_REMOVEDIR);
        };
        let wrapper = match open_sub(t, &cw) {
            Ok(fd) => fd,
            Err(e) => {
                undo(None);
                return Err(log_failed("the trash", FsWriteErr::Io(e)));
            }
        };
        let wfd = wrapper.as_raw_fd();
        if let Err(e) = write_record(wfd, p, wall_time()) {
            undo(None);
            return Err(log_failed("the trash's record", FsWriteErr::Io(e)));
        }
        if stopped(FsStop::TrashRecorded) {
            return Err(FsWriteErr::Io(libc::EINTR));
        }
        // (into the folder made above, holding only the record: no reservation is needed there)
        if let Err(e) = rename_at(dir.as_raw_fd(), &c, wfd, &c, false) {
            undo(Some(wfd));
            return Err(log_failed("the trash", rename_err(e)));
        }
        sync_dir(wfd);
        sync_dir(dir.as_raw_fd());
        sync_dir(t);
        qlog!("files: an item of %s went to the trash", zone);
        Ok(w.into_bytes())
    }

    /// The record of the trash entry `entry` (a file's or folder's, `files-...`), for a row on screen: where it
    /// came from and when. BadRecord when there is none, or it does not read strictly.
    pub fn trash_record(&self, entry: &[u8]) -> Result<TrashRecord, FsWriteErr> {
        if trash_parse(entry).kind != TrashKind::File {
            return Err(FsWriteErr::Refused);
        }
        let trash = self.trash_fd()?;
        let w = open_sub(trash.as_raw_fd(), &cname(entry)?).map_err(|_| FsWriteErr::BadRecord)?;
        read_record(w.as_raw_fd()).ok_or(FsWriteErr::BadRecord)
    }

    /// Puts the trash entry `entry` back. A file or folder (`files-...`) goes back where its record says it was,
    /// when that folder is still there, inside Grimoire's folders for the user's files and not held by an import; a
    /// name taken there gets " 2", " 3" and so on (before a file's extension). A notebook goes back to `notebooks/<id>`, from the name
    /// store_delete gives it (`<id>-<time>`) or from a record whose `from=` is that folder; refused when the id is
    /// taken. A page is not restored here (its place in its notebook was never kept). Its folder in the trash goes
    /// once empty: the record, then the folder, the only removal in these writes. A `files-...` entry that holds
    /// nothing but its record, or nothing (a trash cut off before its item moved), is removed: `Restored::Nothing`.
    pub fn restore(&self, entry: &[u8]) -> Result<Restored, FsWriteErr> {
        if entry.is_empty() || entry.contains(&b'/') || entry == b"." || entry == b".." {
            return Err(FsWriteErr::Refused);
        }
        let e = trash_parse(entry);
        let trash = self.trash_fd()?;
        let t = trash.as_raw_fd();
        let ce = cname(entry)?;
        match e.kind {
            TrashKind::Notebook => {
                let st = stat_at(t, &ce).map_err(FsWriteErr::Walk)?;
                if st.kind() != FsKind::Dir {
                    return Err(FsWriteErr::Refused);
                }
                self.restore_notebook(t, &ce, &e.id)?;
                Ok(Restored::Notebook(e.id))
            }
            TrashKind::File => {
                let w = open_sub(t, &ce).map_err(|_| FsWriteErr::BadRecord)?;
                let wfd = w.as_raw_fd();
                if holds_only_record(wfd) {
                    // A trash cut off before its item moved (a crash after the folder or the record): the item never
                    // left, so there is nothing to put back. The record, then the emptied folder, go.
                    unlink_at(wfd, &cname(TRASH_RECORD.as_bytes())?, 0);
                    drop(w);
                    // SAFETY: unlinkat on our descriptor and a NUL-terminated name (a folder: removed only empty).
                    if unsafe { libc::unlinkat(t, ce.as_ptr(), libc::AT_REMOVEDIR) } != 0 {
                        return Err(FsWriteErr::BadRecord);
                    }
                    sync_dir(t);
                    qlog!("files: a trash entry that held nothing was removed");
                    return Ok(Restored::Nothing);
                }
                let rec = read_record(wfd).ok_or(FsWriteErr::BadRecord)?;
                let (folder, name) = rec.from.split_last().ok_or(FsWriteErr::BadRecord)?;
                // where it goes first: a notebook's folder, or inside Grimoire's folders for the user's files
                let zone = match self.is_notebook_path(&rec.from) {
                    true => None,
                    false => Some(self.write_zone(&rec.from).ok_or(FsWriteErr::Refused)?),
                };
                // then the item, at the name the record gives it
                let cn = cname(name)?;
                let st = stat_at(wfd, &cn).map_err(|_| FsWriteErr::BadRecord)?;
                let dir_item = match st.kind() {
                    FsKind::Dir => true,
                    FsKind::File => false,
                    _ => return Err(FsWriteErr::Refused),
                };
                let done = if let Some(zone) = zone {
                    // (never into what an import holds: a picture put back into a folder being imported)
                    if self.busy(&rec.from) {
                        return Err(FsWriteErr::Busy);
                    }
                    let dest = match self.open_dir(&folder, false) {
                        Ok(fd) => fd,
                        Err(FsErr::NotFound | FsErr::NotDir) => return Err(FsWriteErr::FolderGone),
                        Err(e) => return Err(FsWriteErr::Walk(e)),
                    };
                    let mut back = None;
                    for n in 1..=NUMBERED_MAX {
                        let want = if n == 1 { name.to_vec() } else { numbered(name, n, dir_item) };
                        let to = folder.join(&want).filter(|_| want.len() <= FS_NAME_MAX).ok_or(FsWriteErr::TooLong)?;
                        match rename_noreplace(wfd, &cn, dest.as_raw_fd(), &cname(&want)?) {
                            Ok(()) => {
                                back = Some(to);
                                break;
                            }
                            Err(libc::EEXIST) => continue,
                            Err(e) => return Err(log_failed("a restore", rename_err(e))),
                        }
                    }
                    let to = back.ok_or(FsWriteErr::Exists)?;
                    sync_dir(dest.as_raw_fd());
                    qlog!("files: an item went back to %s", zone);
                    Restored::File(to)
                } else {
                    let id = std::str::from_utf8(name).ok().filter(|id| dir_item && notebook_id_ok(id)).ok_or(FsWriteErr::Refused)?;
                    self.restore_notebook(wfd, &cn, id)?;
                    Restored::Notebook(id.to_string())
                };
                // its folder in the trash, now holding the record alone
                unlink_at(wfd, &cname(TRASH_RECORD.as_bytes())?, 0);
                drop(w);
                unlink_at(t, &ce, libc::AT_REMOVEDIR);
                sync_dir(t);
                Ok(done)
            }
            TrashKind::Page | TrashKind::Other => Err(FsWriteErr::Refused),
        }
    }

    /// Whether `p` is `<Grimoire's folder>/notebooks/<one name>`.
    fn is_notebook_path(&self, p: &StoragePath) -> bool {
        let Some(g) = self.grimoire.as_ref() else { return false };
        let (gc, pc): (Vec<&[u8]>, Vec<&[u8]>) = (g.components().collect(), p.components().collect());
        pc.len() == gc.len() + 2 && gc.iter().zip(&pc).all(|(a, b)| eq_nocase(a, b)) && eq_nocase(pc[gc.len()], b"notebooks")
    }

    /// Renames the notebook folder `name` in `from_fd` to `notebooks/<id>`, never over a notebook there.
    fn restore_notebook(&self, from_fd: i32, name: &CString, id: &str) -> Result<(), FsWriteErr> {
        if !notebook_id_ok(id) {
            return Err(FsWriteErr::Refused);
        }
        let nb = self.notebooks_fd()?;
        rename_noreplace(from_fd, name, nb.as_raw_fd(), &cname(id.as_bytes())?).map_err(|e| log_failed("a notebook's restore", rename_err(e)))?;
        sync_dir(nb.as_raw_fd());
        qlog!("files: notebook %s came back from the trash", id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The name rules, each refused on its own and the first broken named; a file keeps its extension in any case,
    // a folder has none to keep.
    #[test]
    fn the_name_rules() {
        let ok = |old: &[u8], new: &[u8], dir: bool| fsview_name_check(old, new, dir);
        assert_eq!(ok(b"a.png", b"b.png", false), Ok(()));
        assert_eq!(ok(b"a.png", b"Holiday 2026.PNG", false), Ok(()));
        assert_eq!(ok(b"a.png", b"caf\xc3\xa9.png", false), Ok(()));
        assert_eq!(ok(b"a.png", b"", false), Err(NameRule::Empty));
        let long = [vec![b'x'; 252], b".png".to_vec()].concat();
        assert_eq!(ok(b"a.png", &long, false), Err(NameRule::TooLong));
        assert_eq!(ok(b"a.png", &long[1..], false), Ok(()));
        for c in b"/\\:*?\"<>|" {
            assert_eq!(ok(b"a.png", &[b'x', *c, b'.', b'p', b'n', b'g'], false), Err(NameRule::Char(*c)));
        }
        assert_eq!(ok(b"a.png", b"a:b*c.png", false), Err(NameRule::Char(b':')));
        assert_eq!(ok(b"a.png", b"a\tb.png", false), Err(NameRule::Control));
        assert_eq!(ok(b"a.png", b"a\x7f.png", false), Err(NameRule::Control));
        assert_eq!(ok(b"a.png", b" a.png", false), Err(NameRule::LeadingSpace));
        assert_eq!(ok(b"Work", b"Work ", true), Err(NameRule::TrailingSpace));
        assert_eq!(ok(b"Work", b"Work.", true), Err(NameRule::TrailingDot));
        assert_eq!(ok(b"Work", b".Work", true), Err(NameRule::LeadingDot));
        assert_eq!(ok(b"a.png", b"a.jpg", false), Err(NameRule::Extension));
        assert_eq!(ok(b"a.png", b"a", false), Err(NameRule::Extension));
        assert_eq!(ok(b"a.png", b"png", false), Err(NameRule::Extension));
        // a file with no extension may take any name; a folder keeps none
        assert_eq!(ok(b"notes", b"notes 2", false), Ok(()));
        assert_eq!(ok(b"Work.2026", b"School", true), Ok(()));
    }

    // " 2" goes before a file's extension and at the end of a folder's name.
    #[test]
    fn numbered_names() {
        assert_eq!(numbered(b"a.png", 2, false), b"a 2.png");
        assert_eq!(numbered(b"notes", 3, false), b"notes 3");
        assert_eq!(numbered(b"Work.2026", 2, true), b"Work.2026 2");
        assert_eq!(numbered(b"archive.tar.gz", 2, false), b"archive.tar 2.gz");
    }

    // The trash's names: a file's folder (2.2), a notebook and a page (every version), and anything else.
    #[test]
    fn trash_names() {
        let tm = local_tm(1_700_000_000);
        let stamp = format!("{:04}{:02}{:02}-{:02}{:02}{:02}", tm.tm_year + 1900, tm.tm_mon + 1, tm.tm_mday, tm.tm_hour, tm.tm_min, tm.tm_sec);
        let f = trash_parse(format!("files-{stamp}-0a9f").as_bytes());
        assert_eq!((f.kind, f.time), (TrashKind::File, 1_700_000_000));
        let n = trash_parse(b"20261115-101010-ab12-1763200000");
        assert_eq!((n.kind, n.time, n.id.as_str()), (TrashKind::Notebook, 1_763_200_000, "20261115-101010-ab12"));
        let p = trash_parse(b"20261115-101010-ab12-p7-1763200001");
        assert_eq!((p.kind, p.time, p.id.as_str(), p.page), (TrashKind::Page, 1_763_200_001, "20261115-101010-ab12", 7));
        for bad in [
            &b"files-20261115-101010-0A9F"[..],
            b"files-20261115-101010-0a9",
            b"files-20261315-101010-0a9f",
            b"files-2026111-1010100-0a9f",
            b"files-",
            b"notes.txt",
            b"20261115-101010-ab12-",
            b"a b-123",
            b"-123",
            b"20261115-101010-ab12-x",
        ] {
            assert_eq!(trash_parse(bad).kind, TrashKind::Other, "{}", String::from_utf8_lossy(bad));
        }
        // a page's id 0 is no page: the name reads as a notebook's id with "-p0" in it
        assert_eq!(trash_parse(b"abc-p0-12").kind, TrashKind::Notebook);
        // newest first, the unknown last
        let e = |n: &[u8]| FsEntry { name: n.to_vec(), kind: FsKind::Dir, stat: None };
        let list = fsview_trash_entries(&[e(b"zzz"), e(b"a-100"), e(format!("files-{stamp}-0001").as_bytes()), e(b"a-p2-200")]);
        let order: Vec<&[u8]> = list.iter().map(|t| &t.name[..]).collect();
        assert_eq!(order, vec![format!("files-{stamp}-0001").as_bytes(), b"a-p2-200", b"a-100", b"zzz"]);
    }

    // The record, read strictly: what the writer writes reads back; a missing or doubled key, another line, a
    // carriage return, no last line feed, a path that is no storage path, a time that is no number, or more than
    // 4 KiB, each refused.
    #[test]
    fn the_record_reads_strictly() {
        let from = StoragePath::new(b"grimoire/templates/a b.png").unwrap();
        let t = trash_record_text(&from, 1_763_200_000);
        assert_eq!(trash_record_parse(&t), Some(TrashRecord { from: from.clone(), time: 1_763_200_000 }));
        assert_eq!(trash_record_parse(b"time=5\nfrom=grimoire/import/x.pdf\n").map(|r| r.time), Some(5));
        for bad in [
            &b""[..],
            b"from=grimoire/import/x.pdf\n",
            b"time=5\n",
            b"from=grimoire/import/x.pdf\ntime=5",
            b"from=grimoire/import/x.pdf\r\ntime=5\n",
            b"from=grimoire/import/x.pdf\ntime=5\ntime=6\n",
            b"from=a\nfrom=b\ntime=5\n",
            b"from=grimoire/import/x.pdf\ntime=5\nkind=file\n",
            b"from=grimoire/import/x.pdf\n\ntime=5\n",
            b"from=/etc/passwd\ntime=5\n",
            b"from=grimoire/../etc\ntime=5\n",
            b"from=\ntime=5\n",
            b"from=grimoire/import/x.pdf\ntime=-5\n",
            b"from=grimoire/import/x.pdf\ntime=5s\n",
            b"from=grimoire/import/x.pdf\ntime=\n",
            b"from=grimoire/im\0port\ntime=5\n",
        ] {
            assert_eq!(trash_record_parse(bad), None, "{:?}", String::from_utf8_lossy(bad));
        }
        let big = [b"# ".to_vec(), vec![b'x'; TRASH_RECORD_MAX], b"\nfrom=a\ntime=1\n".to_vec()].concat();
        assert_eq!(trash_record_parse(&big), None);
    }

    // What an import holds: the paths it works on, what is inside them and the folders they are in; any case of
    // the names; and everything in the import folder while a previous run's import holds the lock.
    #[test]
    fn what_an_import_holds() {
        let held = vec![b"grimoire/import/Book.pdf".to_vec(), b"grimoire/import/Trip".to_vec()];
        let imp: &[u8] = b"grimoire/import";
        assert!(holds(b"grimoire/import/Book.pdf", &held, false, Some(imp)));
        assert!(holds(b"grimoire/import/book.PDF", &held, false, Some(imp)));
        assert!(holds(b"grimoire/import/Trip/p1.png", &held, false, Some(imp)));
        assert!(holds(b"grimoire/import/Trip", &held, false, Some(imp)));
        assert!(!holds(b"grimoire/import/Other.pdf", &held, false, Some(imp)));
        assert!(!holds(b"grimoire/import/Trip 2", &held, false, Some(imp)));
        assert!(!holds(b"grimoire/templates/a.png", &held, false, Some(imp)));
        let work = vec![b"grimoire/import/Work/A.pdf".to_vec()];
        assert!(holds(b"grimoire/import/Work", &work, false, Some(imp)));
        assert!(!holds(b"grimoire/import/Other.pdf", &[], false, Some(imp)));
        assert!(holds(b"grimoire/import/Other.pdf", &[], true, Some(imp)));
        assert!(!holds(b"grimoire/templates/a.png", &[], true, Some(imp)));
    }
}
