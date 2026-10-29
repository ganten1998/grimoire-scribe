//! nav.h: a notebook's section names (nav.ini) and links (links.ini), kept apart from page ink. Page
//! ids, not page positions, keep them attached when pages move or notes are added.

use crate::prelude::*;

pub const NAV_NAME_LEN: usize = 96;
pub const NAV_FILE_MAX: usize = 256 * 1024;
pub const NAV_LINKS_FILE_MAX: usize = 512 * 1024;
pub const NAV_AREA_MAX: i32 = 6000;
pub const NAV_UNKNOWN_MAX: usize = 16 * 1024;
/// in a notebook
pub const NAV_LINKS_MAX: i32 = 4000;
pub const NAV_LINKS_PER_PAGE: i32 = 64;
pub const NAV_TARGET_LEN: usize = 16;
/// nav_link_page: the link goes to the Index
pub const NAV_TO_INDEX: i32 = -2;
/// a link's box is at least this a side, to tap or click
pub const NAV_LINK_MIN_MM: f32 = 7.0;

#[derive(Clone, Copy, Debug, Default)]
pub struct NavName {
    pub pid: u32,
    pub text: FixedStr<NAV_NAME_LEN>,
}
/// A link's target and optional invisible area live in links.ini, apart from page ink.
/// Without an area it uses its seed's ink box (beta.5); unknown targets survive unchanged.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NavLink {
    pub pid: u32,
    pub seed: u32,
    pub target: FixedStr<NAV_TARGET_LEN>,
    pub area: Rect,
    pub has_area: bool,
}
#[derive(Clone, Debug, Default)]
pub struct Nav {
    /// sorted by page id (the C's names, count, capacity)
    pub names: Vec<NavName>,
    /// nav.ini's lines this version does not read, as read (bytes: carried, never interpreted)
    pub unknown: Vec<u8>,
    /// false after an unreadable or oversized nav.ini: never overwrite it
    pub writable: bool,
    /// sorted by page id, then seed (the C's links, nlinks, links_cap)
    pub links: Vec<NavLink>,
    /// links.ini's lines this version does not read, carried as nav.ini's are
    pub links_unknown: Vec<u8>,
    /// the same for links.ini
    pub links_writable: bool,
    /// links.ini is there (else it is written only once there is a link)
    pub links_file: bool,
    /// Orphan area lines, logged only after a successful save.
    pub areas_dropped: i32,
}

pub fn nav_init(nav: &mut Nav) {
    *nav = Nav::default();
    nav.writable = true;
    nav.links_writable = true;
}
pub fn nav_free(nav: &mut Nav) {
    *nav = Nav::default();
}

fn slot(n: &Nav, pid: u32) -> usize {
    let mut lo = 0;
    let mut hi = n.names.len();
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if n.names[mid].pid < pid {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    lo
}

// The name page pid has (its bytes: a name read from a file need not be UTF-8), if it has one.
fn name_of(n: &Nav, pid: u32) -> Option<&FixedStr<NAV_NAME_LEN>> {
    let at = slot(n, pid);
    if at < n.names.len() && n.names[at].pid == pid {
        Some(&n.names[at].text)
    } else {
        None
    }
}

/// empty when unnamed
pub fn nav_name(nav: &Nav, pid: u32) -> &str {
    match name_of(nav, pid) {
        Some(t) => t.as_str(),
        None => "",
    }
}

// i18n_copy on bytes (names come from a file and need not be UTF-8): src up to its first NUL, at
// most cap - 1 bytes of it, and what the cut leaves of a last character that is not whole dropped
// (the C's strlcpy_q, then its check of the last lead byte).
fn i18n_copy_bytes(src: &[u8], cap: usize) -> Vec<u8> {
    if cap == 0 {
        return Vec::new();
    }
    let src = &src[..src.iter().position(|&b| b == 0).unwrap_or(src.len())];
    let mut dst = src[..src.len().min(cap - 1)].to_vec();
    let mut n = dst.len();
    // The last character copied, if it is not whole (cut here, or before: by snprintf).
    let mut lead = n;
    while lead > 0 && (dst[lead - 1] & 0xC0) == 0x80 {
        lead -= 1;
    }
    if lead > 0 {
        let c = dst[lead - 1];
        let len = if c >= 0xF0 {
            4
        } else if c >= 0xE0 {
            3
        } else if c >= 0xC0 {
            2
        } else {
            1
        };
        if n - (lead - 1) < len {
            n = lead - 1;
        }
    }
    dst.truncate(n);
    dst
}

// nav_set_name on bytes: nav_load's lines are bytes from a file.
fn set_name_bytes(n: &mut Nav, pid: u32, name: &[u8]) -> i32 {
    if !n.writable || pid == 0 {
        return -1;
    }
    // Trim before cutting, so leading whitespace does not use up the name's byte budget.
    let mut at_name = 0;
    while at_name < name.len() && name[at_name] != 0 && (name[at_name] <= 32 || name[at_name] == 127) {
        at_name += 1;
    }
    let mut text = i18n_copy_bytes(&name[at_name..], NAV_NAME_LEN);
    for c in text.iter_mut() {
        if *c < 32 || *c == 127 {
            *c = b' ';
        }
    }
    while text.last() == Some(&b' ') {
        text.pop();
    }
    let at = slot(n, pid);
    let found = at < n.names.len() && n.names[at].pid == pid;
    if text.is_empty() {
        if found {
            n.names.remove(at);
        }
        return 0;
    }
    if !found {
        if n.names.len() >= STORE_MAX_PAGES {
            return -1;
        }
        if n.names.len() == n.names.capacity() {
            let cap = clampi(if n.names.capacity() > 0 { n.names.capacity() as i32 * 2 } else { 16 }, 16, STORE_MAX_PAGES as i32) as usize;
            if n.names.try_reserve_exact(cap - n.names.len()).is_err() {
                return -1;
            }
        }
        n.names.insert(at, NavName::default());
    }
    n.names[at].pid = pid;
    n.names[at].text.set_bytes(&text);
    0
}

/// A single line, trimmed, at most 95 bytes, cut between UTF-8 characters. Empty removes a name.
pub fn nav_set_name(nav: &mut Nav, pid: u32, name: &str) -> i32 {
    set_name_bytes(nav, pid, name.as_bytes())
}

// The path of one of the notebook's files.
fn path_for(id: &str, file: &str) -> Option<String> {
    let mut dir = String::new();
    if store_notebook_dir(id, &mut dir, 600) != 0 {
        return None;
    }
    let mut path = String::new();
    let len = csnprintf!(&mut path, 700, "%s/%s", &dir, file);
    if len < 0 || len as usize >= 700 {
        None
    } else {
        Some(path)
    }
}

// name.<pid>=<name>
fn name_key(line: &[u8]) -> Option<(u32, &[u8])> {
    let rest = line.strip_prefix(b"name.")?;
    let digits = rest.iter().take_while(|c| c.is_ascii_digit()).count();
    if digits == 0 {
        return None;
    }
    let mut id: u64 = 0;
    for &c in &rest[..digits] {
        id = id * 10 + (c - b'0') as u64;
        if id > u32::MAX as u64 {
            return None;
        }
    }
    if rest.get(digits) != Some(&b'=') || id == 0 {
        return None;
    }
    Some((id as u32, &rest[digits + 1..]))
}

// A sidecar's text (None when there is no file): Ok, or Err when it is there and does not read, is
// not a regular file, is a symlink, is over NAV_FILE_MAX or holds a NUL.
fn read_sidecar(path: &str, cap: usize) -> Result<Option<Vec<u8>>, ()> {
    let Ok(cpath) = std::ffi::CString::new(path) else { return Err(()) };
    // SAFETY: open of a NUL-terminated path we own; the descriptor is closed on every path below.
    let fd = unsafe { libc::open(cpath.as_ptr(), libc::O_RDONLY | libc::O_NONBLOCK | libc::O_NOFOLLOW) };
    if fd < 0 {
        // SAFETY: lstat writes one stat into our zeroed local, from a NUL-terminated path.
        let absent = errno() == libc::ENOENT && {
            let mut st: libc::stat = unsafe { std::mem::zeroed() };
            unsafe { libc::lstat(cpath.as_ptr(), &mut st) != 0 && errno() == libc::ENOENT }
        };
        return if absent { Ok(None) } else { Err(()) };
    }
    let r = read_open_sidecar(fd, cap);
    // SAFETY: fd is the descriptor opened above, closed once.
    unsafe { libc::close(fd) };
    r.map(Some)
}

fn read_open_sidecar(fd: libc::c_int, cap: usize) -> Result<Vec<u8>, ()> {
    // SAFETY: fstat writes one stat into our zeroed local, for the open descriptor.
    let mut st: libc::stat = unsafe { std::mem::zeroed() };
    if unsafe { libc::fstat(fd, &mut st) } != 0 || (st.st_mode & libc::S_IFMT) != libc::S_IFREG || st.st_size < 0 || st.st_size > cap as _ {
        return Err(());
    }
    let mut buf: Vec<u8> = Vec::new();
    if buf.try_reserve_exact(cap + 1).is_err() {
        return Err(());
    }
    let mut got_len = 0usize;
    buf.resize(cap + 1, 0);
    loop {
        // SAFETY: read into the part of buf after got_len, with the room that is left there.
        let got = unsafe { libc::read(fd, buf[got_len..].as_mut_ptr() as *mut libc::c_void, cap + 1 - got_len) };
        if got < 0 {
            if errno() == libc::EINTR {
                continue;
            }
            return Err(());
        }
        if got == 0 {
            break;
        }
        got_len += got as usize;
        if got_len > cap {
            return Err(());
        }
    }
    buf.truncate(got_len);
    if buf.contains(&0) {
        return Err(()); // never rewrite a file with a hidden binary tail
    }
    Ok(buf)
}

// Keeps a line this version does not read, to write back: -1 past the limits.
fn carry(buf: &mut Vec<u8>, line: &[u8]) -> i32 {
    if line.len() > 255 || buf.len() + line.len() + 1 > NAV_UNKNOWN_MAX {
        return -1;
    }
    if buf.try_reserve(line.len() + 1).is_err() {
        return -1;
    }
    buf.extend_from_slice(line);
    buf.push(b'\n');
    0
}

// Calls line_fn for each line of text (its end cut off, one carriage return too) until it fails.
fn each_line(text: &[u8], mut line_fn: impl FnMut(&mut Nav, &[u8]) -> i32, n: &mut Nav) -> i32 {
    let mut at = 0;
    while at < text.len() {
        let end = text[at..].iter().position(|&c| c == b'\n').map(|k| at + k);
        let mut line = &text[at..end.unwrap_or(text.len())];
        if line.last() == Some(&b'\r') {
            line = &line[..line.len() - 1];
        }
        if line_fn(n, line) != 0 {
            return -1;
        }
        match end {
            Some(e) => at = e + 1,
            None => break,
        }
    }
    0
}

fn name_line(n: &mut Nav, line: &[u8]) -> i32 {
    if let Some((pid, value)) = name_key(line) {
        return set_name_bytes(n, pid, value);
    }
    if !line.is_empty() && line[0] != b'#' {
        return carry(&mut n.unknown, line);
    }
    0
}

// link.<pid>=<seed, 1 to 8 hex digits>,<target, 1 to 15 bytes from ! to ~>
fn link_parse(line: &[u8]) -> Option<(u32, u32, &str)> {
    let rest = line.strip_prefix(b"link.")?;
    let mut p = 0;
    let mut id: u64 = 0;
    if !rest.first().is_some_and(|c| c.is_ascii_digit()) {
        return None;
    }
    while p < rest.len() && rest[p].is_ascii_digit() {
        id = id * 10 + (rest[p] - b'0') as u64;
        p += 1;
        if id > u32::MAX as u64 {
            return None;
        }
    }
    let eq = rest.get(p) == Some(&b'=');
    p += 1;
    if !eq || id == 0 {
        return None;
    }
    let mut seed: u32 = 0;
    let mut digits = 0;
    while digits < 9 {
        let v = match rest.get(p) {
            Some(c @ b'0'..=b'9') => (c - b'0') as u32,
            Some(c @ b'a'..=b'f') => (c - b'a') as u32 + 10,
            Some(c @ b'A'..=b'F') => (c - b'A') as u32 + 10,
            _ => break,
        };
        seed = seed << 4 | v;
        digits += 1;
        p += 1;
    }
    let comma = rest.get(p) == Some(&b',');
    p += 1;
    if digits == 0 || digits > 8 || !comma {
        return None;
    }
    let target = &rest[p..];
    if target.is_empty() || target.len() >= NAV_TARGET_LEN {
        return None;
    }
    if target.iter().any(|&c| !(b'!'..=b'~').contains(&c)) {
        return None;
    }
    // (all from ! to ~: ASCII)
    Some((id as u32, seed, std::str::from_utf8(target).ok()?))
}

// Parse the shared id/seed grammar through link_parse, then require four bounded decimals.
fn area_parse(line: &[u8]) -> Option<(u32, u32, Rect)> {
    let rest = line.strip_prefix(b"area.")?;
    let comma = rest.iter().position(|&c| c == b',')?;
    let mut key = b"link.".to_vec();
    key.extend_from_slice(&rest[..comma]);
    key.extend_from_slice(b",index");
    let (pid, seed, _) = link_parse(&key)?;
    let mut fields = rest[comma + 1..].split(|&c| c == b',');
    let mut v = [0i32; 4];
    for n in &mut v {
        let f = fields.next()?;
        if f.is_empty() || f.len() > 4 || !f.iter().all(u8::is_ascii_digit) {
            return None;
        }
        for &c in f {
            *n = *n * 10 + (c - b'0') as i32;
        }
    }
    let r = Rect { x: v[0], y: v[1], w: v[2], h: v[3] };
    (fields.next().is_none() && area_valid(r)).then_some((pid, seed, r))
}

fn area_valid(r: Rect) -> bool {
    r.x >= 0 && r.y >= 0 && r.w > 0 && r.h > 0
        && r.x as i64 + r.w as i64 <= NAV_AREA_MAX as i64
        && r.y as i64 + r.h as i64 <= NAV_AREA_MAX as i64
}

fn link_line(n: &mut Nav, line: &[u8], areas: &mut Vec<(u32, u32, Rect)>) -> i32 {
    if let Some((pid, seed, target)) = link_parse(line) {
        return link_put(n, pid, seed, target);
    }
    if let Some(area) = area_parse(line) {
        // Count every valid line, including duplicates, to bound work on hostile files.
        if areas.len() >= NAV_LINKS_MAX as usize || areas.try_reserve(1).is_err() {
            return -1;
        }
        areas.push(area);
        return 0;
    }
    if !line.is_empty() && line[0] != b'#' {
        return carry(&mut n.links_unknown, line);
    }
    0
}

/// nav must be initialized. Missing is an empty writable index; other read failures hide the
/// names and refuse edits. links.ini the same, on its own: either can fail with the other read.
/// Unknown lines of both are kept. -1 when either failed.
pub fn nav_load(id: &str, nav: &mut Nav) -> i32 {
    let mut fresh = Nav::default();
    nav_init(&mut fresh);
    let mut text: Option<Vec<u8>> = None;
    let names_ok = (|| {
        let path = path_for(id, "nav.ini")?;
        text = read_sidecar(&path, NAV_FILE_MAX).ok()?;
        match &text {
            Some(t) => (each_line(t, name_line, &mut fresh) == 0).then_some(()),
            None => Some(()),
        }
    })()
    .is_some();
    text = None;
    if !names_ok {
        fresh.names = Vec::new();
        fresh.unknown = Vec::new();
        fresh.writable = false;
        qlog!("index: page names could not be read; left unchanged");
    }
    let mut areas = Vec::new();
    let links_ok = (|| {
        let path = path_for(id, "links.ini")?;
        text = read_sidecar(&path, NAV_LINKS_FILE_MAX).ok()?;
        match &text {
            Some(t) => (each_line(t, |n, line| link_line(n, line, &mut areas), &mut fresh) == 0).then_some(()),
            None => Some(()),
        }
    })()
    .is_some();
    fresh.links_file = text.is_some();
    if links_ok {
        // File order makes the last area win, even when it precedes its link.
        for (pid, seed, area) in areas {
            let at = link_slot(&fresh, pid, seed);
            if let Some(l) = fresh.links.get_mut(at).filter(|l| l.pid == pid && l.seed == seed) {
                l.area = area;
                l.has_area = true;
            } else {
                fresh.areas_dropped += 1;
            }
        }
    }
    if !links_ok {
        fresh.links = Vec::new();
        fresh.links_unknown = Vec::new();
        fresh.links_writable = false;
        fresh.links_file = true;
        qlog!("index: links could not be read; left unchanged");
    }
    nav_free(nav);
    *nav = fresh;
    if names_ok && links_ok {
        0
    } else {
        -1
    }
}

fn has_page(nb: &NotebookInfo, pid: u32) -> bool {
    nb.page_ids.contains(&pid)
}

/// Atomic. Names of deleted pages are pruned only after a successful save; unknown lines stay.
pub fn nav_save(id: &str, nav: &mut Nav, nb: &NotebookInfo) -> i32 {
    if id.as_bytes() != nb.id.as_bytes() || !nav.writable || nb.npages() < 0 || nb.npages() > STORE_MAX_PAGES as i32 {
        return -1;
    }
    let Some(path) = path_for(id, "nav.ini") else { return -1 };
    let mut text: Vec<u8> = Vec::new();
    text.extend_from_slice(b"# Grimoire: page names\n");
    for name in &nav.names {
        if !has_page(nb, name.pid) {
            continue;
        }
        // name.%u=%s\n
        let mut line = cformat!("name.%u=", name.pid).into_bytes();
        line.extend_from_slice(name.text.as_bytes());
        line.push(b'\n');
        if line.len() > NAV_FILE_MAX - text.len() {
            return -1;
        }
        text.extend_from_slice(&line);
    }
    if nav.unknown.len() > NAV_FILE_MAX - text.len() {
        return -1;
    }
    text.extend_from_slice(&nav.unknown);
    let rc = write_file_atomic(&path, &text);
    if rc == 0 {
        nav.names.retain(|n| has_page(nb, n.pid));
    }
    rc
}

/// Named pages in notebook order; returns the full count, writes at most cap page indices.
pub fn nav_index(nav: &Nav, nb: &NotebookInfo, indices: &mut [i32]) -> i32 {
    let mut count = 0;
    for i in 0..nb.page_ids.len() {
        if name_of(nav, nb.page_ids[i]).is_some_and(|t| !t.is_empty()) {
            if (count as usize) < indices.len() {
                indices[count as usize] = i as i32;
            }
            count += 1;
        }
    }
    count
}

/// page index, or -1
pub fn nav_section_of(nav: &Nav, nb: &NotebookInfo, index: i32) -> i32 {
    let mut i = clampi(index, -1, nb.npages() - 1);
    while i >= 0 {
        if name_of(nav, nb.page_ids[i as usize]).is_some_and(|t| !t.is_empty()) {
            return i;
        }
        i -= 1;
    }
    -1
}

// ---- links ----

// Where (pid, seed) is or would go in n->links.
fn link_slot(n: &Nav, pid: u32, seed: u32) -> usize {
    let mut lo = 0;
    let mut hi = n.links.len();
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        let l = &n.links[mid];
        if l.pid < pid || (l.pid == pid && l.seed < seed) {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    lo
}

fn link_put(n: &mut Nav, pid: u32, seed: u32, target: &str) -> i32 {
    let tl = target.len();
    if pid == 0 || tl == 0 || tl >= NAV_TARGET_LEN {
        return -1;
    }
    let at = link_slot(n, pid, seed);
    if !(at < n.links.len() && n.links[at].pid == pid && n.links[at].seed == seed) {
        if n.links.len() >= NAV_LINKS_MAX as usize {
            return -1;
        }
        if n.links.len() == n.links.capacity() {
            let cap = clampi(if n.links.capacity() > 0 { n.links.capacity() as i32 * 2 } else { 16 }, 16, NAV_LINKS_MAX) as usize;
            if n.links.try_reserve_exact(cap - n.links.len()).is_err() {
                return -1;
            }
        }
        n.links.insert(at, NavLink { pid, seed, ..Default::default() });
    }
    n.links[at].target.set(target);
    0
}

// A page target's page id: "p<id>", the id from 1 to UINT32_MAX in decimal without leading zeros.
fn target_page(t: &[u8]) -> u32 {
    let c1 = t.get(1).copied().unwrap_or(0);
    if t.first() != Some(&b'p') || !(b'1'..=b'9').contains(&c1) {
        return 0;
    }
    let mut id: u64 = 0;
    for &c in &t[1..] {
        if !c.is_ascii_digit() {
            return 0;
        }
        id = id * 10 + (c - b'0') as u64;
        if id > u32::MAX as u64 {
            return 0;
        }
    }
    id as u32
}

/// Adds the link of the strokes with `seed` on page pid, or changes its target. target: "index" or
/// "p<page id>". -1 when it cannot (not writable, a bad target, the limits above).
pub fn nav_link_set(nav: &mut Nav, pid: u32, seed: u32, target: &str) -> i32 {
    if !nav.links_writable || (target != "index" && target_page(target.as_bytes()) == 0) {
        return -1;
    }
    let mut first = 0;
    let count = nav_links_of(nav, pid, &mut first);
    let mut found = false;
    for i in first..first + count {
        found |= nav.links[i as usize].seed == seed;
    }
    if !found && count >= NAV_LINKS_PER_PAGE {
        return -1;
    }
    link_put(nav, pid, seed, target)
}

/// Returns the complete link, including a target from a newer version.
pub fn nav_link_get(nav: &Nav, pid: u32, seed: u32) -> Option<NavLink> {
    nav.links.get(link_slot(nav, pid, seed)).filter(|l| l.pid == pid && l.seed == seed).copied()
}

pub fn nav_link_set_area(nav: &mut Nav, pid: u32, seed: u32, area: Option<Rect>) -> i32 {
    if !nav.links_writable || area.is_some_and(|r| !area_valid(r)) {
        return -1;
    }
    let at = link_slot(nav, pid, seed);
    let Some(l) = nav.links.get_mut(at).filter(|l| l.pid == pid && l.seed == seed) else { return -1 };
    l.area = area.unwrap_or(RECT_EMPTY);
    l.has_area = area.is_some();
    0
}

/// Restores an exact record. Unknown targets must survive undo as they survive a load.
pub fn nav_link_put(nav: &mut Nav, l: &NavLink) -> i32 {
    if !nav.links_writable || (l.has_area && !area_valid(l.area))
        || l.target.is_empty() || !l.target.as_bytes().iter().all(|c| (b'!'..=b'~').contains(c)) {
        return -1;
    }
    if nav_link_get(nav, l.pid, l.seed).is_none() && nav_links_of(nav, l.pid, &mut 0) >= NAV_LINKS_PER_PAGE {
        return -1;
    }
    if link_put(nav, l.pid, l.seed, l.target.as_str()) != 0 {
        return -1;
    }
    let at = link_slot(nav, l.pid, l.seed);
    nav.links[at] = *l;
    0
}

pub fn nav_link_remove(nav: &mut Nav, pid: u32, seed: u32) {
    if !nav.links_writable {
        return;
    }
    let at = link_slot(nav, pid, seed);
    if at >= nav.links.len() || nav.links[at].pid != pid || nav.links[at].seed != seed {
        return;
    }
    nav.links.remove(at);
}

/// Page pid's links: how many, and the first in nav->links (*first).
pub fn nav_links_of(nav: &Nav, pid: u32, first: &mut i32) -> i32 {
    let at = link_slot(nav, pid, 0);
    let mut end = at;
    while end < nav.links.len() && nav.links[end].pid == pid {
        end += 1;
    }
    *first = at as i32;
    (end - at) as i32
}

fn page_index(nb: &NotebookInfo, pid: u32) -> i32 {
    for i in 0..nb.page_ids.len() {
        if nb.page_ids[i] == pid {
            return i as i32;
        }
    }
    -1
}

/// Where a link goes in nb: a page index, NAV_TO_INDEX, or -1 (a page gone, a target this version
/// does not know).
pub fn nav_link_page(l: &NavLink, nb: &NotebookInfo) -> i32 {
    if l.target == "index" {
        return NAV_TO_INDEX;
    }
    let to = target_page(l.target.as_bytes());
    if to != 0 {
        page_index(nb, to)
    } else {
        -1
    }
}

// The box of the points of page p's ink strokes with this seed, at least min_px a side around its
// centre, and the place in the page of the last of them (-1, and an empty box, for none).
fn link_box(p: &Page, seed: u32, min_px: f32, box_: &mut Rect) -> i32 {
    let mut x0 = 0.0f32;
    let mut y0 = 0.0f32;
    let mut x1 = 0.0f32;
    let mut y1 = 0.0f32;
    let mut last: i32 = -1;
    for i in 0..p.strokes.len() {
        let s = &p.strokes[i];
        if s.seed != seed || stroke_is_erase(s) {
            continue;
        }
        for k in 0..s.pts.len() {
            let x = s.pts[k].x;
            let y = s.pts[k].y;
            if last < 0 && k == 0 {
                x0 = x;
                x1 = x;
                y0 = y;
                y1 = y;
            }
            if x < x0 {
                x0 = x;
            }
            if x > x1 {
                x1 = x;
            }
            if y < y0 {
                y0 = y;
            }
            if y > y1 {
                y1 = y;
            }
        }
        if !s.pts.is_empty() {
            last = i as i32;
        }
    }
    if last < 0 {
        *box_ = RECT_EMPTY;
        return -1;
    }
    let cx = (x0 + x1) * 0.5;
    let cy = (y0 + y1) * 0.5;
    if x1 - x0 < min_px {
        x0 = fmla(-min_px, 0.5, cx);
        x1 = fmla(min_px, 0.5, cx);
    }
    if y1 - y0 < min_px {
        y0 = fmla(-min_px, 0.5, cy);
        y1 = fmla(min_px, 0.5, cy);
    }
    let ix = x0.floor() as i32;
    let iy = y0.floor() as i32;
    *box_ = Rect { x: ix, y: iy, w: x1.ceil() as i32 - ix, h: y1.ceil() as i32 - iy };
    last
}

/// The boxes of page p's links (p is page pid): for each link whose strokes are there, the box of
/// their points (the ink strokes only), made at least min_px a side around its centre. Writes at most
/// cap links and their boxes; returns how many it wrote.
pub fn nav_link_areas(nav: &Nav, p: &Page, pid: u32, min_px: f32, links: &mut [NavLink], areas: &mut [Rect]) -> i32 {
    // (the C's cap: how many of each the caller has room for)
    let cap = links.len().min(areas.len()) as i32;
    let mut first = 0;
    let count = nav_links_of(nav, pid, &mut first);
    let mut wrote = 0;
    let mut i = first;
    while i < first + count && wrote < cap {
        let mut box_ = Rect::default();
        if link_box(p, nav.links[i as usize].seed, min_px, &mut box_) >= 0 {
            links[wrote as usize] = nav.links[i as usize];
            areas[wrote as usize] = box_;
            wrote += 1;
        }
        i += 1;
    }
    wrote
}

/// The link whose box holds (x, y) on page p, the one drawn last when boxes overlap. False for none.
pub fn nav_link_at(nav: &Nav, p: &Page, pid: u32, x: f32, y: f32, min_px: f32, out: &mut NavLink) -> bool {
    let mut first = 0;
    let count = nav_links_of(nav, pid, &mut first);
    let mut best: i32 = -1;
    let mut best_last: i32 = -1;
    for i in first..first + count {
        let mut box_ = Rect::default();
        let last = link_box(p, nav.links[i as usize].seed, min_px, &mut box_);
        if last <= best_last {
            continue;
        }
        if x >= box_.x as f32 && x < (box_.x + box_.w) as f32 && y >= box_.y as f32 && y < (box_.y + box_.h) as f32 {
            best = i;
            best_last = last;
        }
    }
    if best < 0 {
        return false;
    }
    *out = nav.links[best as usize];
    true
}

/// The seed for a new ink stroke on page p (page pid): want, or the next after it that no ink stroke on
/// p and no link of pid has, so a link's box never takes in a stroke drawn after it.
pub fn nav_new_seed(nav: &Nav, p: &Page, pid: u32, mut want: u32) -> u32 {
    let mut first = 0;
    let count = nav_links_of(nav, pid, &mut first);
    for _tries in 0..64 {
        let mut taken = false;
        let mut i = first;
        while i < first + count && !taken {
            taken = nav.links[i as usize].seed == want;
            i += 1;
        }
        let mut i = 0;
        while i < p.strokes.len() && !taken {
            taken = p.strokes[i].seed == want && !stroke_is_erase(&p.strokes[i]);
            i += 1;
        }
        if !taken {
            break;
        }
        want = want.wrapping_mul(2654435761).wrapping_add(0x9e3779b9);
    }
    want
}

// A link stays while its page is in the notebook and it goes to the Index, a page of it, or a place
// this version does not know.
fn link_kept(l: &NavLink, nb: &NotebookInfo) -> bool {
    let to = target_page(l.target.as_bytes());
    has_page(nb, l.pid) && (to == 0 || has_page(nb, to))
}

/// Writes links.ini, leaving out the links of pages no longer in nb and those going to such pages.
/// -1 when not links_writable or the write failed (then nothing is left out).
pub fn nav_save_links(id: &str, nav: &mut Nav, nb: &NotebookInfo) -> i32 {
    if id.as_bytes() != nb.id.as_bytes() || !nav.links_writable || nb.npages() < 0 || nb.npages() > STORE_MAX_PAGES as i32 {
        return -1;
    }
    let Some(path) = path_for(id, "links.ini") else { return -1 };
    let kept = nav.links.iter().filter(|l| link_kept(l, nb)).count();
    if !nav.links_file && kept == 0 && nav.links_unknown.is_empty() {
        return 0;
    }
    let mut text: Vec<u8> = Vec::new();
    text.extend_from_slice(b"# Grimoire: links drawn on pages\n");
    for l in &nav.links {
        if !link_kept(l, nb) {
            continue;
        }
        let line = cformat!("link.%u=%08x,%s\n", l.pid, l.seed, l.target.as_str());
        if line.len() > NAV_LINKS_FILE_MAX - text.len() {
            return -1;
        }
        text.extend_from_slice(line.as_bytes());
        if l.has_area {
            if !area_valid(l.area) { return -1; }
            let r = l.area;
            let area = cformat!("area.%u=%08x,%d,%d,%d,%d\n", l.pid, l.seed, r.x, r.y, r.w, r.h);
            if area.len() > NAV_LINKS_FILE_MAX - text.len() { return -1; }
            text.extend_from_slice(area.as_bytes());
        }
    }
    if nav.links_unknown.len() > NAV_LINKS_FILE_MAX - text.len() {
        return -1;
    }
    text.extend_from_slice(&nav.links_unknown);
    let rc = write_file_atomic(&path, &text);
    if rc == 0 {
        if nav.areas_dropped > 0 {
            qlog!("index: %d link area(s) with no link left out", nav.areas_dropped);
            nav.areas_dropped = 0;
        }
        nav.links_file = true;
        nav.links.retain(|l| link_kept(l, nb));
    }
    rc
}
