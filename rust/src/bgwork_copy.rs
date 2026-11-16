//! A file copied into one of Grimoire's folders for the user's files (BgKind::CopyFile, 2.2, issue #117): what a
//! picture's Use as template and Use as photo in Files, and the From a file... tiles of the Templates and Photos
//! sheets, ask for (issue #146; the Font sheet's Add a font... comes later). The request (UI thread) checks the
//! source is still the plain file listed, that it is no bigger than its folder takes (CopyTo::max_bytes: a bigger
//! one could never be used there), that the storage keeps its floor after it (fsview_room_for), and queues the
//! job. The job (a background child) opens
//! the source again through fsview's walk (no link, one device, never Grimoire's notebooks, PDFs or trash),
//! checks it is still that file, no bigger than that, and that there is room, copies it to a temporary name in the folder
//! (`.grimoire-copy-<pid>.part`), checks the source did not change while it was read, syncs, and renames it to
//! the source's name with a rename that replaces nothing (" 2", " 3"... when taken). On any failure the
//! temporary file goes: nothing is left behind. The log has the folder, sizes and times, never a name. What the
//! screen says of a copy that was not made is copy_refusal's: the sentence for its cause where one exists
//! (CopyFail, which the job's exit status carries), else the one for a full storage.

use crate::fsview::*;
use crate::prelude::*;
use std::ffi::CString;
use std::os::fd::{AsRawFd, FromRawFd};

/// Where a copy goes: one of Grimoire's folders for the user's files.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CopyTo {
    Templates,
    Photos,
    Fonts,
}

impl CopyTo {
    /// Its folder's name in Grimoire's folder (the job's id).
    pub fn folder(self) -> &'static str {
        match self {
            CopyTo::Templates => "templates",
            CopyTo::Photos => "photos",
            CopyTo::Fonts => "fonts",
        }
    }
    /// The folder named so (a job's id, as bg_poll_job reports it), None for any other.
    pub fn of(folder: &str) -> Option<CopyTo> {
        [CopyTo::Templates, CopyTo::Photos, CopyTo::Fonts].into_iter().find(|t| t.folder() == folder)
    }
    /// The largest file the folder's own job takes: the template job's limit for templates and photos, the font
    /// check's for fonts. A bigger file is never copied in, as nothing there could use it.
    pub fn max_bytes(self) -> u64 {
        match self {
            CopyTo::Templates | CopyTo::Photos => crate::bgwork_template::TEMPLATE_MAX_BYTES as u64,
            CopyTo::Fonts => crate::fontcheck::FONT_FILE_MAX as u64,
        }
    }
}

/// Why a copy was not made, as far as a sentence the screens have says it: at the request (`of_request`) or in the
/// job, which ends with its number as exit status (`of_exit` reads it back from bg_last_exit; bgjob.rs lets these
/// numbers through and makes 1 of any other failure).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CopyFail {
    /// the floor, a write that failed, or a cause with no sentence of its own (grimoire.log says which)
    Other = 1,
    /// the file is bigger than its folder takes (CopyTo::max_bytes)
    TooBig = 2,
}

impl CopyFail {
    /// From copy_request's refusal.
    pub fn of_request(e: FsWriteErr) -> CopyFail {
        match e {
            FsWriteErr::TooBig => CopyFail::TooBig,
            _ => CopyFail::Other,
        }
    }
    /// From the exit status of a copy job that failed.
    pub fn of_exit(code: i32) -> CopyFail {
        match code {
            2 => CopyFail::TooBig,
            _ => CopyFail::Other,
        }
    }
}

/// What the screen says of a copy into `to` that was not made, refused at the request or failed in the job (a
/// toast).
pub fn copy_refusal(why: CopyFail, to: CopyTo) -> String {
    match why {
        // TRANSLATORS: (also) a toast (one line of about 1000 px at 44 px in bold): the picture picked for Use as
        // template, Use as photo or a sheet's From a file... is bigger than Grimoire takes as a template or a photo,
        // so it is not copied; %d is 16.
        CopyFail::TooBig => cformat!(tr("Too big (over %d MB)."), (to.max_bytes() >> 20) as i32),
        CopyFail::Other => copy_failed().to_string(),
    }
}

/// What a copy that could not be made says when no other sentence names its cause (the floor is the likely one).
pub fn copy_failed() -> &'static str {
    // TRANSLATORS: a toast (about 1700 px at 44 px in bold, or two lines): a picture could not be copied into
    // Grimoire's templates or photos (Use as template, Use as photo, a sheet's From a file...); the storage keeps
    // 100 MB free, so the likely cause is that it is nearly full.
    tr("Couldn't copy file. Is the storage full?")
}

/// What the copy reads and writes at a time.
const COPY_CHUNK: usize = 64 << 10;
/// The temporary name begins and ends so (`.grimoire-copy-<pid>.part`): a dot-name, which no listing shows.
const TMP_HEAD: &str = ".grimoire-copy-";
const TMP_TAIL: &str = ".part";
/// A temporary file untouched this long is a dead copy's, whatever runs under its process id now (after a
/// restart that number often belongs to another process): a live copy writes it every 64 KiB.
const TMP_STALE_SECS: i64 = 3600;

/// The job's path: the source as listed ("<dev>.<ino>.<size>.<mtime>:") and its storage path, None when the path
/// is not text or the whole would not fit the job's 600 bytes.
fn job_path(src: &StoragePath, listed: &FsStat) -> Option<String> {
    let s = std::str::from_utf8(src.as_bytes()).ok()?;
    let p = format!("{}.{}.{}.{}:{}", listed.dev, listed.ino, listed.size, listed.mtime, s);
    (p.len() < FS_PATH_MAX).then_some(p)
}

fn job_parse(path: &str) -> Option<(FsStat, StoragePath)> {
    let (head, src) = path.split_once(':')?;
    let f: Vec<&str> = head.split('.').collect();
    let [dev, ino, size, mtime] = f[..] else { return None };
    let st = FsStat { dev: dev.parse().ok()?, ino: ino.parse().ok()?, size: size.parse().ok()?, mtime: mtime.parse().ok()?, mode: 0 };
    Some((st, StoragePath::new(src.as_bytes()).filter(|p| !p.is_root())?))
}

#[cfg(any(test, feature = "sim"))]
static SEAM_FREE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(u64::MAX);
#[cfg(any(test, feature = "sim"))]
static SEAM_FAIL_AFTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(u64::MAX);

/// Tests (never the Kindle's build): the bytes free the copy sees (None: the storage's own), and a write that fails
/// as on a full storage (ENOSPC) once this many bytes are written (None: none). A job forked after takes them too.
/// The headless runner's scenarios set the same through `GRIMOIRE_COPY_FREE_BYTES` and `GRIMOIRE_COPY_FAIL_AFTER`
/// (docs/SIM.md), which these override.
#[cfg(any(test, feature = "sim"))]
pub fn copy_seams(free: Option<u64>, fail_after: Option<u64>) {
    SEAM_FREE.store(free.unwrap_or(u64::MAX), std::sync::atomic::Ordering::Relaxed);
    SEAM_FAIL_AFTER.store(fail_after.unwrap_or(u64::MAX), std::sync::atomic::Ordering::Relaxed);
}

// (a seam's value: the test's, else the runner's variable, else none)
#[cfg(any(test, feature = "sim"))]
fn seam(set: &std::sync::atomic::AtomicU64, var: &str) -> Option<u64> {
    let v = set.load(std::sync::atomic::Ordering::Relaxed);
    if v != u64::MAX {
        return Some(v);
    }
    std::env::var(var).ok().and_then(|v| v.parse::<u64>().ok())
}

fn free_bytes(st: &FsStorage) -> Option<u64> {
    #[cfg(any(test, feature = "sim"))]
    if let Some(f) = seam(&SEAM_FREE, "GRIMOIRE_COPY_FREE_BYTES") {
        return Some(f);
    }
    st.free_bytes()
}

fn fail_after() -> u64 {
    #[cfg(any(test, feature = "sim"))]
    {
        seam(&SEAM_FAIL_AFTER, "GRIMOIRE_COPY_FAIL_AFTER").unwrap_or(u64::MAX)
    }
    #[cfg(not(any(test, feature = "sim")))]
    {
        u64::MAX
    }
}

/// Asks for the file at `src` (as listed: `listed`) to be copied into Grimoire's folder `to`, in the background
/// (BgKind::CopyFile, reported through bg_poll as a job the user waits on). Refused now when the source is in a
/// place a pick never takes from (Grimoire's own folders), is no longer the item listed, is no plain file, is
/// bigger than the folder takes (`to.max_bytes()`), or would leave less than the floor free (100 MB after it); or
/// when its path is not text or too long for a job.
pub fn copy_request(st: &FsStorage, src: &StoragePath, listed: &FsStat, to: CopyTo) -> Result<(), FsWriteErr> {
    if src.is_root() || st.zone(src, true) == FsZone::Refused {
        return Err(FsWriteErr::Refused);
    }
    let now = st.stat(src, true).map_err(|e| if e == FsErr::NotFound { FsWriteErr::Changed } else { FsWriteErr::Walk(e) })?;
    if !now.same(listed) {
        return Err(FsWriteErr::Changed);
    }
    if now.mode & libc::S_IFMT as u32 != libc::S_IFREG as u32 {
        return Err(FsWriteErr::Walk(FsErr::NotFile));
    }
    // (said before the room: a file too big is refused however much room there is)
    if now.size > to.max_bytes() {
        return Err(FsWriteErr::TooBig);
    }
    let free = free_bytes(st).ok_or(FsWriteErr::Io(libc::EIO))?;
    if !fsview_room_for(free, now.size) {
        return Err(FsWriteErr::NoRoom);
    }
    let path = job_path(src, &now).ok_or(FsWriteErr::TooLong)?;
    bg_queue(BgKind::CopyFile, Some(to.folder()), Some(&path), 0.0, true);
    Ok(())
}

/// BgKind::CopyFile, in the background child: 0 when the file is in its folder; when not, a CopyFail's number
/// where the cause has one (the job's exit status), else -1 (the log says why, and nothing of it is left there).
pub fn bg_copy_file(to: &str, path: &str) -> i32 {
    let t0 = now_ms();
    match copy(to, path) {
        Ok(n) => {
            qlog!("bg: a file copied into %s: %u KB in %d ms", to, (n >> 10) as u32, now_ms().wrapping_sub(t0) as i32);
            0
        }
        Err(Fail(why, words)) => {
            qlog!("bg: a file not copied into %s: %s", to, &words);
            if why == CopyFail::Other {
                -1
            } else {
                why as i32
            }
        }
    }
}

// Why the job failed: what the screen can say of it, and the log's words.
struct Fail(CopyFail, String);
impl From<&str> for Fail {
    fn from(words: &str) -> Fail {
        Fail(CopyFail::Other, words.into())
    }
}
impl From<String> for Fail {
    fn from(words: String) -> Fail {
        Fail(CopyFail::Other, words)
    }
}

fn walk_why(e: FsErr) -> String {
    match e {
        FsErr::Link => "a link on its way".into(),
        FsErr::OtherDevice => "another device".into(),
        FsErr::Refused => "a place it is not taken from".into(),
        FsErr::NotFound => "it is not there".into(),
        FsErr::NotDir => "a folder on its way is not one".into(),
        FsErr::NotFile => "it is no plain file".into(),
        FsErr::Io(n) => strerror(n),
    }
}

fn copy(to: &str, path: &str) -> Result<u64, Fail> {
    let to = CopyTo::of(to).ok_or("not one of Grimoire's folders")?;
    let (listed, src) = job_parse(path).ok_or("a request that does not read")?;
    let root = crate::pdfimport::storage_root().ok_or("no storage")?;
    let st = FsStorage::open(&root, &store_root()).map_err(walk_why)?;
    let g = st.grimoire_rel().ok_or("Grimoire's folder is not in the storage")?.clone();
    let dest = g.join(to.folder().as_bytes()).ok_or("a path too long")?;
    let (_, name) = src.split_last().ok_or("no source")?;
    if st.zone(&dest.join(name).ok_or("a path too long")?, false) != FsZone::Writable {
        return Err("not one of Grimoire's folders".into());
    }
    // the source: the walk again, a plain file, the one listed, no bigger than the folder takes
    let file = st.open_file(&src, true).map_err(walk_why)?;
    let mut f = std::fs::File::from(file);
    let at = file_stat(&f)?;
    if !at.same(&listed) {
        return Err("it changed since it was listed".into());
    }
    if at.size > to.max_bytes() {
        return Err(Fail(CopyFail::TooBig, format!("it is over {} MB", to.max_bytes() >> 20)));
    }
    if !free_bytes(&st).is_some_and(|free| fsview_room_for(free, at.size)) {
        return Err("no room: the storage would keep less than 100 MB".into());
    }
    let dir = match st.open_dir(&dest, false) {
        Ok(d) => d,
        Err(FsErr::NotFound) => {
            // (the folder itself is Grimoire's: made where it is missing)
            let gfd = st.open_dir(&g, false).map_err(walk_why)?;
            let c = CString::new(to.folder()).map_err(|_| "a name")?;
            // SAFETY: mkdirat on our descriptor and a NUL-terminated name.
            unsafe { libc::mkdirat(gfd.as_raw_fd(), c.as_ptr(), 0o755) };
            st.open_dir(&dest, false).map_err(walk_why)?
        }
        Err(e) => return Err(walk_why(e).into()),
    };
    let d = dir.as_raw_fd();
    sweep_stale(d);
    let tmp = CString::new(format!("{TMP_HEAD}{}{TMP_TAIL}", std::process::id())).map_err(|_| "a name")?;
    let out = create_tmp(d, &tmp)?;
    let r = fill(&mut f, out, &listed).and_then(|n| {
        // the source's name, or " 2", " 3"... before its extension where taken (a file: never a folder's form)
        for k in 1..=99u32 {
            let want = if k == 1 { name.to_vec() } else { numbered(name, k, false) };
            if want.len() > FS_NAME_MAX || dest.join(&want).is_none() {
                return Err("a name too long".into());
            }
            let cw = CString::new(want).map_err(|_| String::from("a name"))?;
            match rename_noreplace(d, &tmp, d, &cw) {
                Ok(()) => return Ok(n),
                Err(libc::EEXIST) => continue,
                Err(e) => return Err(strerror(e)),
            }
        }
        Err("every name up to \" 99\" is taken".into())
    });
    if r.is_err() {
        // SAFETY: unlinkat on our descriptor and a NUL-terminated name: the temporary file, whatever it holds.
        unsafe { libc::unlinkat(d, tmp.as_ptr(), 0) };
    }
    sync_dir(d);
    Ok(r?)
}

fn file_stat(f: &std::fs::File) -> Result<FsStat, String> {
    use std::os::unix::fs::MetadataExt;
    let m = f.metadata().map_err(|e| strerror(e.raw_os_error().unwrap_or(libc::EIO)))?;
    Ok(FsStat { dev: m.dev(), ino: m.ino(), size: m.size(), mtime: m.mtime(), mode: m.mode() })
}

/// The temporary file, made afresh (O_CREAT|O_EXCL|O_NOFOLLOW): one a dead run of the same process id left goes
/// first, the entry itself and never what it may point at.
fn create_tmp(d: i32, tmp: &CString) -> Result<std::fs::File, String> {
    for _ in 0..2 {
        // SAFETY: openat on our descriptor and a NUL-terminated name; the descriptor is owned by the File.
        let fd = unsafe { libc::openat(d, tmp.as_ptr(), libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC, 0o644 as libc::c_uint) };
        if fd >= 0 {
            // SAFETY: fd was just opened and is owned by no one else.
            return Ok(unsafe { std::fs::File::from_raw_fd(fd) });
        }
        let e = errno();
        if e != libc::EEXIST {
            return Err(strerror(e));
        }
        // SAFETY: unlinkat on our descriptor and a NUL-terminated name (a folder there fails, and so does the open).
        unsafe { libc::unlinkat(d, tmp.as_ptr(), 0) };
    }
    Err(strerror(libc::EEXIST))
}

/// Copies the source into `out` and syncs it: the bytes copied, or why not. The source must end as it was listed
/// (size and time, as read at the end), and have given exactly its size.
fn fill(src: &mut std::fs::File, out: std::fs::File, listed: &FsStat) -> Result<u64, String> {
    use std::io::{Read, Write};
    let mut out = out;
    let mut buf = Vec::new();
    buf.try_reserve_exact(COPY_CHUNK).map_err(|_| "no memory")?;
    buf.resize(COPY_CHUNK, 0);
    let limit = fail_after();
    let mut n: u64 = 0;
    loop {
        let k = match src.read(&mut buf) {
            Ok(0) => break,
            Ok(k) => k,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(strerror(e.raw_os_error().unwrap_or(libc::EIO))),
        };
        if n.saturating_add(k as u64) > limit {
            return Err(strerror(libc::ENOSPC));
        }
        out.write_all(&buf[..k]).map_err(|e| strerror(e.raw_os_error().unwrap_or(libc::EIO)))?;
        n += k as u64;
        if n > listed.size {
            return Err("it changed while it was copied".into());
        }
    }
    let end = file_stat(src)?;
    if n != listed.size || !end.same(listed) {
        return Err("it changed while it was copied".into());
    }
    out.sync_all().map_err(|e| strerror(e.raw_os_error().unwrap_or(libc::EIO)))?;
    Ok(n)
}

/// Temporary files of copies that died with their process (a killed child, a restart): gone, so nothing stays
/// behind. One whose process still runs (a detached child finishing a copy at exit) is left to it, unless nothing
/// has written it for an hour (TMP_STALE_SECS: its process id is someone else's now).
fn sweep_stale(d: i32) {
    // SAFETY: dup of our descriptor; fdopendir takes the duplicate, closedir closes it; readdir's entries are read
    // before the next call.
    let names: Vec<CString> = unsafe {
        let d2 = libc::dup(d);
        if d2 < 0 {
            return;
        }
        let dir = libc::fdopendir(d2);
        if dir.is_null() {
            libc::close(d2);
            return;
        }
        let mut v = Vec::new();
        loop {
            let e = libc::readdir(dir);
            if e.is_null() {
                break;
            }
            let n = std::ffi::CStr::from_ptr((*e).d_name.as_ptr());
            if n.to_bytes().starts_with(TMP_HEAD.as_bytes()) && v.len() < 64 {
                v.push(n.to_owned());
            }
        }
        libc::closedir(dir);
        v
    };
    let now = wall_time();
    for n in names {
        let pid = std::str::from_utf8(n.to_bytes())
            .ok()
            .and_then(|s| s.strip_prefix(TMP_HEAD)?.strip_suffix(TMP_TAIL)?.parse::<i32>().ok())
            .filter(|&p| p > 0);
        // SAFETY: kill with signal 0 only asks whether the process exists.
        let dead = |p: i32| unsafe { libc::kill(p, 0) } != 0 && errno() == libc::ESRCH;
        let untouched = || {
            // SAFETY: a stat is plain data, written by fstatat (no link followed) before it is read.
            let mut st: libc::stat = unsafe { std::mem::zeroed() };
            // SAFETY: fstatat on our descriptor and a NUL-terminated name, into st.
            let r = unsafe { libc::fstatat(d, n.as_ptr(), &mut st, libc::AT_SYMLINK_NOFOLLOW) };
            r == 0 && now - (st.st_mtime as i64) > TMP_STALE_SECS
        };
        if pid.is_some_and(|p| p != std::process::id() as i32 && (dead(p) || untouched())) {
            // SAFETY: unlinkat on our descriptor and a NUL-terminated name (the entry itself, never followed).
            unsafe { libc::unlinkat(d, n.as_ptr(), 0) };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The job's path carries the source as listed and its storage path, and reads back; one that would not fit the
    // job's 600 bytes, or is not text, is not made; one that does not read is refused.
    #[test]
    fn the_jobs_path() {
        let st = FsStat { dev: 7, ino: 12345, size: 99, mtime: -3, mode: 0 };
        let src = StoragePath::new(b"documents/a:b.png").unwrap();
        let p = job_path(&src, &st).unwrap();
        assert_eq!(p, "7.12345.99.-3:documents/a:b.png");
        let (back, s) = job_parse(&p).unwrap();
        assert!(back.same(&st) && s == src);
        let long = StoragePath::new(&[b'a'; 590]).unwrap();
        assert!(job_path(&long, &st).is_none());
        assert!(job_path(&StoragePath::new(b"caf\xe9.png").unwrap(), &st).is_none());
        for bad in ["", "1.2.3:a", "1.2.3.4.5:a", "x.2.3.4:a", "1.2.3.4:", "1.2.3.4:/etc/passwd", "1.2.3.4:a/../b"] {
            assert!(job_parse(bad).is_none(), "{bad}");
        }
        assert_eq!(CopyTo::of("photos"), Some(CopyTo::Photos));
        assert_eq!(CopyTo::of("notebooks"), None);
    }

    // What the screen says of a copy that was not made: the sentence of its cause, read from the request's refusal
    // or from the job's exit status; the folder's own limit in the sentence for a file too big.
    #[test]
    fn what_a_refused_copy_says() {
        let full = "Couldn't copy file. Is the storage full?";
        assert_eq!(copy_refusal(CopyFail::of_request(FsWriteErr::TooBig), CopyTo::Templates), "Too big (over 16 MB).");
        assert_eq!(copy_refusal(CopyFail::of_request(FsWriteErr::TooBig), CopyTo::Photos), "Too big (over 16 MB).");
        assert_eq!(copy_refusal(CopyFail::of_request(FsWriteErr::TooBig), CopyTo::Fonts), "Too big (over 24 MB).");
        for e in [FsWriteErr::NoRoom, FsWriteErr::Refused, FsWriteErr::Io(libc::EIO), FsWriteErr::Walk(FsErr::NotFile)] {
            assert_eq!(copy_refusal(CopyFail::of_request(e), CopyTo::Templates), full, "{e:?}");
        }
        // the job's exit status, as bg_last_exit gives it: the cause's number, 1 (or a child that died) for the rest
        assert_eq!(CopyFail::of_exit(CopyFail::TooBig as i32), CopyFail::TooBig);
        for code in [1, -1, 0, 9] {
            assert_eq!(copy_refusal(CopyFail::of_exit(code), CopyTo::Photos), full, "{code}");
        }
    }
}
