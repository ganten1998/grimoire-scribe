//! One storage layer (the 2.1 plan, DESIGN.md "One storage layer"; browser.md 8 and 9): the only code that
//! touches the Kindle's storage for the file browser and for every consumer of a user's file. Read-only for
//! now (lane L0): the root, the walk, Grimoire's own folders by identity, StoragePath, zones, the time-sliced
//! listing, stats and free space. Nothing reads it yet; the picker (L8) and the PDF import (L5) will.
//!
//! - One root: /mnt/us on the Kindle (when Grimoire's own folder is inside it); elsewhere GRIMOIRE_STORAGE,
//!   a hook compiled out of the Kindle build. Never "/".
//! - One walk, no links: each component opened with openat(O_DIRECTORY|O_NOFOLLOW) from the one before, the
//!   file with O_NOFOLLOW, so a link anywhere fails (O_NOFOLLOW covers the last component only; the Scribe's
//!   kernel, 4.9, has no openat2). Each directory must be on the root's device, and none may be Grimoire's
//!   notebooks (nor, for a pick, its PDFs or trash), compared by st_dev and st_ino: a path cased differently
//!   on a case-insensitive storage cannot reach them by name.
//! - Zones: lists hide by a deny-list (dot-entries, *.sdr, the system's folders, Grimoire's own); writes (not
//!   here yet) will go by an allow-list (Grimoire's import, templates, photos, fonts), so safety never depends
//!   on the deny-list being complete.
//! - Time-sliced: a listing reads names and d_type only (at most 5,000), and stats only the rows asked for,
//!   each in slices of at most 20 ms a tick with the folder kept open between ticks.
//! - Privacy: the log has counts and timings, never a name.

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
/// Grimoire's folders that hold the user's files (writes will go only here).
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
    /// Grimoire's folders for the user's files: import, templates, photos, fonts (writes, later, only here)
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
        Ok(FsStorage {
            root: root_fd,
            dev,
            grimoire: rel.and_then(|r| StoragePath::new(r.as_bytes())),
            notebooks: identity(&format!("{g}/notebooks")),
            not_picked: [identity(&format!("{g}/PDFs")), identity(&format!("{g}/trash"))].into_iter().flatten().collect(),
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
