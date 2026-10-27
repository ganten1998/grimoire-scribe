//! bgwork_compact.c: BG_COMPACT_PAGES, taking the ink erased entirely out of a notebook's page files
//! (compact.h).
//!
//! It runs in bgjob.c's background child: page_droppable can take tens of seconds on a heavily
//! erased page on the Kindle, never something for the UI thread. For each page listed in the
//! notebook's erased.txt it reads the page file, and when page_droppable finds strokes that change
//! nothing (ink that later erase strokes restore entirely, erase strokes left with nothing to
//! change) it writes the page without them, through a temporary file and a rename, and only if the
//! file still holds what it read (an older version, or a copy from a computer, may have written it
//! since). A stroke erased only in part stays whole, with the erase strokes over it. A page that is
//! done (compacted, or with nothing to leave out) goes off the list, and the list goes once it is
//! empty. A page it cannot judge stays listed: one that does not read, one of another size than its
//! notebook's, and one holding data this version reads past (a newer version's), which only the
//! version that wrote it can judge.
//!
//! Stopping it is safe at any point. bg_cancel_notebook (SIGTERM) runs before the notebook opens.
//! The pass first puts SIGTERM, SIGINT and SIGHUP back to their default action (a fork keeps
//! Grimoire's handlers, and on the Kindle input_evdev.c's only note the signal, so the pass would
//! run on and Grimoire would wait for all of it), and they wait while a file is being replaced, so
//! a stop never leaves a temporary file behind: each page is its old file or its new one. It stops
//! by itself when Grimoire goes (its parent), so a Grimoire started again can open the notebook at
//! once: the list keeps the pages for next time.
//!
//! Compaction never changes how a page looks (page_droppable checks the page with its paper and its
//! ink layer), so the notebook's PDF mirror comes out the same whether it runs before compaction or
//! after it. The two never run at once anyway (bgjob.c runs one child at a time, and keeps
//! compaction out of what it hands to the child that outlives Grimoire), and a page file is replaced
//! in one rename, so any reader (the mirror, the share server) sees the old file or the new one.

use crate::prelude::*;

const LIST_NAME: &str = "erased.txt";
/// ids read from one list (a notebook has at most STORE_MAX_PAGES pages)
const LIST_MAX: usize = 4096;

/// Tests only (NULL otherwise): called with the page file's path once the pass has written a page's
/// new file, just before it checks that the page file still holds what it read and renames the new
/// file over it, with the stops held (SIGTERM waits). A test writes the page there as another writer
/// might, or asks the pass to stop.
pub static COMPACT_TEST_BEFORE_REPLACE: std::sync::Mutex<Option<fn(&str)>> = std::sync::Mutex::new(None);

/// A C string for a path, cut where the C would see its NUL.
fn cstr(s: &str) -> std::ffi::CString {
    let end = s.bytes().position(|b| b == 0).unwrap_or(s.len());
    std::ffi::CString::new(&s.as_bytes()[..end]).unwrap_or_default()
}

fn list_path(id: &str, buf: &mut String, cap: usize) -> i32 {
    let mut dir = String::new();
    if store_notebook_dir(id, &mut dir, 600) != 0 {
        return -1;
    }
    csnprintf!(buf, cap, "%s/%s", &dir, LIST_NAME);
    0
}

// The whole of the list file at `path`: None with *err -1 when there is no such file, -2 when it is
// there but could not be read to its end (a read error, no memory).
fn list_text(path: &str, err: &mut i32) -> Option<Vec<u8>> {
    use std::io::Read;
    let mut f = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(e) => {
            *err = if e.raw_os_error() == Some(libc::ENOENT) { -1 } else { -2 };
            return None;
        }
    };
    let mut t: Vec<u8> = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        match f.read(&mut chunk) {
            Ok(0) => break,
            Ok(r) => {
                if t.try_reserve(r).is_err() {
                    *err = -2;
                    return None;
                }
                t.extend_from_slice(&chunk[..r]);
            }
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => {
                *err = -2;
                return None;
            }
        }
    }
    Some(t)
}

// The page ids notebook `id` lists, each once, in the order listed: 0 (out may be empty); -1 when it
// has no list (or `id` cannot be a notebook's); -2 when the list is there but could not be read
// whole, so nothing may be written over it. A line that is not a page id is skipped.
fn list_read(id: &str, out: &mut Vec<u32>) -> i32 {
    out.clear();
    let mut path = String::new();
    if list_path(id, &mut path, 700) != 0 {
        return -1;
    }
    let mut err = 0;
    let Some(text) = list_text(&path, &mut err) else { return err };
    let mut ids: Vec<u32> = Vec::new();
    if ids.try_reserve_exact(LIST_MAX).is_err() {
        return -2;
    }
    // (The C reads the text as a string: it ends at a NUL byte, if the file has one.)
    let text = &text[..text.iter().position(|&b| b == 0).unwrap_or(text.len())];
    let mut l = 0;
    while l < text.len() && ids.len() < LIST_MAX {
        let nl = text[l..].iter().position(|&b| b == b'\n').map(|p| l + p);
        let mut line = &text[l..nl.unwrap_or(text.len())];
        while let Some(&b) = line.first() {
            if b == b' ' || b == b'\t' {
                line = &line[1..];
            } else {
                break;
            }
        }
        // strtoul: a value that does not fit comes back as the largest, which is refused below too
        let mut v: u64 = 0;
        let mut digits = 0;
        while digits < line.len() && line[digits].is_ascii_digit() {
            v = v.saturating_mul(10).saturating_add((line[digits] - b'0') as u64);
            digits += 1;
        }
        let after = line.get(digits).copied();
        if v > 0 && v < 0x7fffffff && matches!(after, None | Some(b'\r') | Some(b' ')) {
            let seen = ids.contains(&(v as u32));
            if !seen {
                ids.push(v as u32);
            }
        }
        let Some(nl) = nl else { break };
        l = nl + 1;
    }
    *out = ids;
    0
}

// Writes notebook `id`'s list (ids), or removes it when there are none. 0, or -1.
fn list_write(id: &str, ids: &[u32]) -> i32 {
    let mut path = String::new();
    if list_path(id, &mut path, 700) != 0 {
        return -1;
    }
    if ids.is_empty() {
        return match std::fs::remove_file(&path) {
            Ok(()) => 0,
            Err(e) if e.raw_os_error() == Some(libc::ENOENT) => 0,
            Err(_) => -1,
        };
    }
    // (The C sizes its buffer as 128 + 12 per id and checks every append against it: the header is
    // 80 bytes and an id with its newline at most 11, so the checks cannot fail and are not here.)
    let mut t = String::with_capacity(128 + ids.len() * 12);
    t.push_str("# Grimoire: pages that may still hold erased ink, until compaction takes it out\n");
    for &i in ids {
        t.push_str(&cformat!("%u\n", i));
    }
    write_file_atomic(&path, t.as_bytes())
}

/// Lists page `page_id` of notebook `id` in its erased.txt (if it is not there yet): the editor
/// calls it when it saves a page with an erase stroke it kept this session. 0, or -1 if the list
/// could not be written, or is there but could not be read whole (then it is left as it is, and
/// the editor tries again at its next save).
pub fn compact_note_page(id: &str, page_id: u32) -> i32 {
    let mut ids: Vec<u32> = Vec::new();
    let rd = list_read(id, &mut ids);
    if rd == -2 {
        return -1; // a list that does not read now is never written over: tried again later
    }
    if ids.contains(&page_id) {
        return 0;
    }
    let mut rc = -1;
    if ids.len() < LIST_MAX && ids.try_reserve(1).is_ok() {
        ids.push(page_id);
        rc = list_write(id, &ids);
    }
    rc
}

/// Whether notebook `id` may have pages listed: true for a list that is there but does not read now
/// (the background pass tries it again).
pub fn compact_listed(id: &str) -> bool {
    let mut ids: Vec<u32> = Vec::new();
    let rd = list_read(id, &mut ids);
    if rd == -2 {
        return true;
    }
    if rd != 0 {
        return false;
    }
    !ids.is_empty()
}

/// At startup: queues the compaction of every notebook with pages listed (an earlier run left while
/// the notebook was open, or a compaction was stopped before it was done). Returns how many.
pub fn compact_queue_listed(dpi: f32) -> i32 {
    let mut dir = String::new();
    csnprintf!(&mut dir, 600, "%s/notebooks", &store_root());
    let Ok(d) = std::fs::read_dir(&dir) else { return 0 };
    let mut n = 0;
    for e in d.flatten() {
        // (A name that is not UTF-8 cannot be a notebook's id.)
        let Some(name) = e.file_name().to_str().map(|s| s.to_string()) else { continue };
        // (compact_listed is false for a name that cannot be a notebook id)
        if name.starts_with('.') || !compact_listed(&name) {
            continue;
        }
        bg_queue(BgKind::CompactPages, Some(&name), None, dpi, false);
        n += 1;
    }
    if n != 0 {
        qlog!("compact: %d notebook(s) with erased ink left to take out from an earlier run", n);
    }
    n
}

// ---- the background pass ----

fn stop_signals() -> libc::sigset_t {
    // SAFETY: sigset_t is plain data; sigemptyset initializes it and sigaddset adds valid signals.
    unsafe {
        let mut s: libc::sigset_t = std::mem::zeroed();
        libc::sigemptyset(&mut s);
        libc::sigaddset(&mut s, libc::SIGTERM);
        libc::sigaddset(&mut s, libc::SIGINT);
        libc::sigaddset(&mut s, libc::SIGHUP);
        s
    }
}

// The signals that stop the pass, at their default action and not blocked: the child of a fork
// keeps Grimoire's handlers, and on the Kindle (input_evdev.c) those only note the signal for
// Grimoire's main loop, so bg_cancel_notebook's SIGTERM, or the one PR_SET_PDEATHSIG sends, would
// not stop the pass, and Grimoire would wait in bg_cancel_notebook for all of it.
fn stops_by_default() {
    // SAFETY: sigaction is plain data, zeroed and then set to SIG_DFL with an empty mask; the three
    // calls install it for signals this process may handle, and sigprocmask unblocks the same three
    // in the calling (only) thread of the forked child.
    unsafe {
        let mut sa: libc::sigaction = std::mem::zeroed();
        sa.sa_sigaction = libc::SIG_DFL;
        libc::sigemptyset(&mut sa.sa_mask);
        libc::sigaction(libc::SIGTERM, &sa, std::ptr::null_mut());
        libc::sigaction(libc::SIGINT, &sa, std::ptr::null_mut());
        libc::sigaction(libc::SIGHUP, &sa, std::ptr::null_mut());
        let s = stop_signals();
        libc::sigprocmask(libc::SIG_UNBLOCK, &s, std::ptr::null_mut());
    }
}

// The stops wait while a file is replaced (bg_cancel_notebook's stop then lands after it). Returns
// the mask to put back (allow_stops).
fn hold_stops() -> libc::sigset_t {
    let s = stop_signals();
    // SAFETY: old is plain data that sigprocmask fills with the mask it replaces; s is initialized.
    unsafe {
        let mut old: libc::sigset_t = std::mem::zeroed();
        libc::sigprocmask(libc::SIG_BLOCK, &s, &mut old);
        old
    }
}
fn allow_stops(old: &libc::sigset_t) {
    // SAFETY: old is the mask hold_stops got back from sigprocmask.
    unsafe { libc::sigprocmask(libc::SIG_SETMASK, old, std::ptr::null_mut()) };
}

// Grimoire (the parent the pass started under) is gone: stop before writing anything more.
fn parent_gone(parent: libc::pid_t) -> bool {
    // SAFETY: getppid has no preconditions.
    let p = unsafe { libc::getppid() };
    p != parent || p == 1
}

// Grimoire gone: stop (its list keeps the pages)
#[cfg(target_os = "linux")]
fn stop_with_parent() {
    // SAFETY: prctl(PR_SET_PDEATHSIG, sig) only sets the signal this process gets when its parent
    // dies.
    unsafe { libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM as libc::c_ulong) };
}
#[cfg(not(target_os = "linux"))]
fn stop_with_parent() {}

// Replaces `path` with `data` through a temporary file, but only if the file still holds `was`
// (checked once the new file is written, just before the rename): 0 replaced, 1 the file is not
// the one read any more (it stays as it is), -1 could not write.
fn replace_if_same(dir: &str, path: &str, was: &[u8], data: &[u8]) -> i32 {
    let mut tmp = String::new();
    csnprintf!(&mut tmp, 760, "%s.compact.tmp", path);
    #[cfg(feature = "sim")]
    if crate::util::SIM_STORAGE_FULL.load(std::sync::atomic::Ordering::Relaxed) {
        return -1;
    }
    let ctmp = cstr(&tmp);
    // SAFETY: open reads the NUL-terminated path we own.
    let fd = unsafe { libc::open(ctmp.as_ptr(), libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC, 0o644 as libc::c_uint) };
    if fd < 0 {
        return -1;
    }
    let mut p = data;
    let mut ok = true;
    while !p.is_empty() && ok {
        // SAFETY: write reads at most p.len() bytes from our slice into the open fd.
        let w = unsafe { libc::write(fd, p.as_ptr() as *const libc::c_void, p.len()) };
        if w < 0 && errno() == libc::EINTR {
            continue;
        }
        ok = w > 0;
        if ok {
            p = &p[w as usize..];
        }
    }
    // SAFETY: fsync and close on the fd opened above, closed once.
    ok = ok && unsafe { libc::fsync(fd) } == 0;
    // SAFETY: as above.
    ok = unsafe { libc::close(fd) } == 0 && ok;
    if !ok {
        // SAFETY: unlink reads the NUL-terminated path we own.
        unsafe { libc::unlink(ctmp.as_ptr()) };
        return -1;
    }
    let hook = *COMPACT_TEST_BEFORE_REPLACE.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(f) = hook {
        f(path);
    }
    let now = read_file(path);
    let same = now.as_deref() == Some(was);
    if !same {
        // SAFETY: as the unlink above.
        unsafe { libc::unlink(ctmp.as_ptr()) };
        return 1;
    }
    let cpath = cstr(path);
    // SAFETY: rename reads two NUL-terminated paths we own.
    if unsafe { libc::rename(ctmp.as_ptr(), cpath.as_ptr()) } != 0 {
        // SAFETY: as the unlink above.
        unsafe { libc::unlink(ctmp.as_ptr()) };
        return -1;
    }
    let cdir = cstr(dir);
    // SAFETY: best effort, the rename itself durable: open reads a path we own, and fsync and close
    // work on the fd it returned.
    unsafe {
        let dfd = libc::open(cdir.as_ptr(), libc::O_RDONLY);
        if dfd >= 0 {
            libc::fsync(dfd);
            libc::close(dfd);
        }
    }
    0
}

// Whether page p holds data this version reads past (docs/FORMAT.md, Compaction): ext bytes on a
// stroke (a newer version's colour, layer or flags, or a tip variant this version lacks; one it
// knows is fine), an erase stroke whose texture this version
// lacks (drawn here as the round tip), chunks after the strokes, or a paper it does not know (drawn
// here blank). The version that wrote it may draw the page otherwise, so how it looks here cannot
// tell what may be left out of it.
fn holds_newer_data(p: &Page) -> bool {
    if !p.tail.is_empty() || !paper_is_known(p.paper) {
        return true;
    }
    for s in &p.strokes {
        // A tip variant this version knows (ext[2]'s low bits) is no newer data; any other ext bit is.
        if s.ext[0] != 0 || s.ext[1] != 0 || (s.ext[2] & 0xF0) != 0 || !brush_tip_known(stroke_render_tool(s), stroke_tip(s)) {
            return true;
        }
        // A texture this version has: 0 (the round tip) or a drawing tool + 1, what erase_seed
        // writes for the tool stroke_render_tool reads out of it.
        let top = s.seed >> 24;
        if stroke_is_erase(s) && top != 0 && erase_seed(stroke_render_tool(s), 0) >> 24 != top {
            return true;
        }
    }
    false
}

/// off the list / stays listed for next time
#[derive(Clone, Copy, PartialEq, Eq)]
enum PageDone {
    Done,  // PAGE_DONE
    Again, // PAGE_AGAIN
}

// One listed page of nb: its file without the strokes that change nothing, if there are any.
fn compact_page(nb: &NotebookInfo, dir: &str, pid: u32, parent: libc::pid_t, left_out: &mut i32) -> PageDone {
    let mut path = String::new();
    csnprintf!(&mut path, 700, "%s/p%u.qpg", dir, pid);
    let Some(was) = read_file(&path) else {
        return PageDone::Done; // no file (never written, or gone): no ink in it
    };
    let mut p: Option<Box<Page>> = None;
    let mut res = PageDone::Done;
    let (mut w, mut h) = (0, 0);
    store_notebook_page_size(nb, &mut w, &mut h);
    if page_deserialize(&was, &mut p) != 0 || p.is_none() {
        // (The editor sets it aside when it opens it; a page that reads again is looked at then.)
        qlog!("compact: %s p%u does not read; left listed", &nb.id, pid);
        res = PageDone::Again;
    } else if let Some(pg) = p.as_deref_mut() {
        if pg.readonly || holds_newer_data(pg) {
            // Never judged by how this version draws it: a newer version's page, or one with data this
            // version reads past, stays as it is, and listed for the version that wrote it.
            qlog!("compact: %s p%u %s; left as it is, listed", &nb.id, pid, if pg.readonly { "is read-only" } else { "holds data of a newer version" });
            res = PageDone::Again;
        } else if pg.w != w || pg.h != h {
            // One of another size (an older version wrote it on another Scribe) goes back to the
            // notebook's size once the editor writes it: compacted after that, listed until then.
            qlog!("compact: %s p%u is of another size; left listed", &nb.id, pid);
            res = PageDone::Again;
        } else {
            let erase = pg.strokes.iter().filter(|s| stroke_is_erase(s)).count() as i32;
            let mut drop: Vec<bool> = Vec::new();
            let have_drop = erase != 0 && drop.try_reserve_exact(pg.strokes.len()).is_ok();
            if have_drop {
                drop.resize(pg.strokes.len(), false);
            }
            let t0 = now_ms();
            // At the dpi the reader built the strokes' reach with (store_dpi), the editor's.
            let mut n = if have_drop { page_droppable(pg, store_dpi(), &mut drop) } else { 0 };
            let ms = now_ms().wrapping_sub(t0);
            if erase != 0 && !have_drop {
                n = -1;
            }
            if n < 0 {
                qlog!("compact: %s p%u: out of memory; tried again next time", &nb.id, pid);
                res = PageDone::Again;
            } else if n == 0 {
                if erase != 0 {
                    qlog!("compact: %s p%u: nothing to leave out (%d strokes, %d erase), %u ms", &nb.id, pid, pg.strokes.len() as i32, erase, ms);
                }
            } else {
                let total = pg.strokes.len() as i32;
                let mut gone_erase = 0;
                let strokes = std::mem::take(&mut pg.strokes);
                for (i, s) in strokes.into_iter().enumerate() {
                    if !drop[i] {
                        pg.strokes.push(s);
                        continue;
                    }
                    gone_erase += stroke_is_erase(&s) as i32;
                    stroke_free(s);
                }
                let mut data: Vec<u8> = Vec::new();
                let mut rc = -1;
                if page_serialize(pg, &mut data) == 0 {
                    let old = hold_stops();
                    rc = if parent_gone(parent) { 2 } else { replace_if_same(dir, &path, &was, &data) };
                    allow_stops(&old);
                }
                if rc == 0 {
                    qlog!("compact: %s p%u: %d of %d strokes left out (%d erase), %u ms", &nb.id, pid, n, total, gone_erase, ms);
                    *left_out += n;
                } else {
                    qlog!(
                        "compact: %s p%u: %s; tried again next time",
                        &nb.id,
                        pid,
                        if rc == 1 {
                            "the file changed meanwhile"
                        } else if rc == 2 {
                            "Grimoire has gone"
                        } else {
                            "could not write it"
                        }
                    );
                    res = PageDone::Again;
                }
            }
        }
    }
    if let Some(pg) = p {
        page_free(pg);
    }
    res
}

/// bgjob.h: the work of a BG_COMPACT_PAGES job, in the child; 0 = done.
pub fn bg_compact_pages(id: &str, dpi: f32) -> i32 {
    let _ = dpi; // the pages are checked at store_dpi(), the dpi their strokes were read at
    stops_by_default();
    // SAFETY: getppid has no preconditions.
    let parent = unsafe { libc::getppid() };
    stop_with_parent();
    let mut ids: Vec<u32> = Vec::new();
    let rd = list_read(id, &mut ids);
    if rd == -1 {
        return 0; // nothing listed
    }
    if rd != 0 {
        qlog!("compact: %s: its list does not read; left as it is", id);
        return -1;
    }
    let mut nb = NotebookInfo::default();
    let mut dir = String::new();
    if store_load_info(id, &mut nb) != 0 || store_notebook_dir(id, &mut dir, 600) != 0 {
        qlog!("compact: %s cannot be read; its list stays", id);
        return -1;
    }
    let listed = ids.len() as i32;
    let mut left_out = 0;
    if ids.is_empty() {
        // a list with no page ids in it lists nothing: it goes
        let old = hold_stops();
        list_write(id, &ids);
        allow_stops(&old);
    }
    let mut i = 0;
    while i < ids.len() && !parent_gone(parent) {
        let in_book = nb.page_ids.contains(&ids[i]);
        let done = if in_book { compact_page(&nb, &dir, ids[i], parent, &mut left_out) } else { PageDone::Done };
        if done == PageDone::Again {
            i += 1;
            continue;
        }
        ids.remove(i);
        let old = hold_stops();
        if !parent_gone(parent) && list_write(id, &ids) != 0 {
            qlog!("compact: %s: could not update its list", id);
        }
        allow_stops(&old);
    }
    if listed != 0 {
        qlog!("compact: %s: %d of %d listed page(s) done, %d stroke(s) left out", id, listed - ids.len() as i32, listed, left_out);
    }
    0
}
