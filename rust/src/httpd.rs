//! httpd.h: tiny single-threaded HTTP/1.0 share server, polled from the main loop.
//! Serves a phone/desktop-friendly HTML index of folders + notebooks with thumbnails and
//! download links, in the Kindle's style (Gothic or Classic fonts). Routes (ids are notebook
//! ids, n is 1-based):
//!   GET /                         HTML index (folders, notebooks, page links)
//!   GET /nb/<id>                  HTML notebook page with per-page PNG links
//!   GET /nb/<id>/studio           HTML studio: transparent / recolored / filtered exports
//!   GET /nb/<id>.pdf              notebook PDF
//!   GET /nb/<id>.zip              notebook zip (pdf + pngs + raw)
//!   GET /nb/<id>/<n>.png          one page PNG (?layer=ink or ?layer=paper: that layer alone;
//!                                 ?f=2..8: shrunk; headers X-Page-Size, X-Paper, X-Ink-Box)
//!   GET /nb/<id>/thumb.png        thumbnail
//!   GET /font/<name>.ttf          the page fonts (the binary's own copies)
//!   GET /folder/<name>.zip        every notebook in a folder
//!   GET /all.zip                  everything
//!   GET /api/list.json            machine-readable listing (used by tools/grimoire-pull)
//!   GET /live                     Live view (live.h): the code form, or the viewer once paired
//!   GET /live/pair?code=|t=       trades the Kindle's code (or its QR code's token) for a session
//!   GET /live/stream              the screen, for a paired browser (a viewer child answers it)
//!   GET /live/pen                 the pen's samples, for a paired browser (a pen viewer child)
//!   GET /live/time                the Kindle's now_ms(), for a paired browser's latency meter
//! The server runs while an owner keeps it (httpd_acquire): the Share screen, whose routes are every
//! one above but /live, and Live view, whose are /live and the fonts. A route no owner has answers
//! 403 (a download while only Live view runs) or 404.
//! Non-blocking sockets, up to 16 connections at once, each with its own buffer; responses are
//! built into a Buf then written in chunks from httpd_poll so the UI stays responsive. Renders
//! run in forked children, two at a time; more wait in a queue (oldest first) and are dropped if
//! their browser hangs up first.

// httpd.c: a tiny HTTP/1.0 server for the share screen. Pages, fonts and the notebook list are
// answered in-process; anything that renders (PDF, zip, PNG) is answered by a forked child, so
// a big export can neither freeze the UI nor trip the launcher's watchdog. The pages wear the
// Kindle's style (its fonts are served from the copies built into the binary), and the studio
// page recolors pages in the browser from the paper and ink layers (web/studio.js).
//
// (The C reads its strings as bytes, and so does this: a request, a path and a header are
// `&[u8]`/`Vec<u8>` here, and a C string ends at its first NUL, as `cbytes` reads one.)

// The fonts and the web pages are embedded at build time: build/gen/fonts.c and build/gen/web.c in the
// C (xxd -i), assets.rs here (include_bytes!, the same bytes).
use crate::assets::*;
use crate::live_keys::{LiveKey, LiveKeys, live_key_name};
use crate::prelude::*;
use std::cell::RefCell;
use std::ffi::CStr;
use std::net::Ipv4Addr;
use std::sync::atomic::{AtomicPtr, AtomicU32, Ordering::SeqCst};

pub const HTTPD_SHARE: u32 = 1;
pub const HTTPD_LIVE: u32 = 2;

// (The C defines MSG_NOSIGNAL as 0 where the system has none, and sets SO_NOSIGPIPE where it has
// that: the Apple libc has the one and not the other. SIGPIPE is ignored from httpd_start on.)
#[cfg(target_vendor = "apple")]
const MSG_NOSIGNAL: libc::c_int = 0;
#[cfg(not(target_vendor = "apple"))]
const MSG_NOSIGNAL: libc::c_int = libc::MSG_NOSIGNAL;

// Small, anonymous shared records let the parent answer progress while a child renders.
// No notebook contents, temporary files, or second download are needed. Retain the last 32
// records; queued/running jobs cannot be recycled, including a child not yet reaped.
const DL_EMPTY: u32 = 0;
const DL_QUEUED: u32 = 1;
const DL_PREPARING: u32 = 2;
const DL_SENDING: u32 = 3;
const DL_DONE: u32 = 4;
const DL_FAILED: u32 = 5;
const DOWNLOADS: usize = 32;

/// The part of the C's Download that the children write (state and counters): these live in the
/// shared mapping. Its id and busy flag are the parent's alone and sit in State (`download_ids`,
/// `download_busy`); a record is named by its index, which is what the C's Download pointer is.
#[repr(C)]
struct Shared {
    state: AtomicU32,
    pages: AtomicU32,
    total: AtomicU32,
    sent: AtomicU32,
    bytes: AtomicU32,
}
// (the C: _Static_assert(ATOMIC_INT_LOCK_FREE == 2, "download counters must work across fork"))
const _: () = assert!(cfg!(target_has_atomic = "32"), "download counters must work across fork");

/// The mapping download_begin makes (null before it). A lone scalar, set once by the parent.
static G_DOWNLOADS: AtomicPtr<Shared> = AtomicPtr::new(std::ptr::null_mut());

fn shared(i: usize) -> &'static Shared {
    let p = G_DOWNLOADS.load(SeqCst);
    assert!(!p.is_null() && i < DOWNLOADS);
    // SAFETY: p is the base of the DOWNLOADS records that download_begin mapped (MAP_SHARED, so a
    // forked child sees the same memory) and never unmaps, i is an index it handed out, below
    // DOWNLOADS, and a record holds only atomics, which are shared by reference.
    unsafe { &*p.add(i) }
}

/// A C string in a byte buffer: the bytes before the first NUL.
fn cbytes(s: &[u8]) -> &[u8] {
    &s[..s.iter().position(|&b| b == 0).unwrap_or(s.len())]
}

/// strstr(h, n) != NULL
fn contains(h: &[u8], n: &[u8]) -> bool {
    n.is_empty() || h.windows(n.len()).any(|w| w == n)
}

fn download_id_ok(id: &[u8]) -> bool {
    let id = cbytes(id);
    id.len() == 32 && id.iter().all(|c| c.is_ascii_hexdigit())
}

fn download_find(id: &[u8]) -> Option<usize> {
    if !G_DOWNLOADS.load(SeqCst).is_null() && download_id_ok(id) {
        let id = cbytes(id);
        return with(|s| (0..DOWNLOADS).find(|&i| s.download_ids[i].as_bytes() == id));
    }
    None
}

fn download_begin(id: &[u8]) -> Option<usize> {
    if !download_id_ok(id) || download_find(id).is_some() {
        return None;
    }
    if G_DOWNLOADS.load(SeqCst).is_null() {
        // SAFETY: an anonymous shared mapping of DOWNLOADS records, which no file or fd backs; the
        // kernel zero-fills it and it stays mapped for the life of the process.
        let p = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                DOWNLOADS * std::mem::size_of::<Shared>(),
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_SHARED | libc::MAP_ANON,
                -1,
                0,
            )
        };
        if p == libc::MAP_FAILED {
            return None;
        }
        G_DOWNLOADS.store(p as *mut Shared, SeqCst);
        for i in 0..DOWNLOADS {
            let d = shared(i);
            d.state.store(DL_EMPTY, SeqCst);
            d.pages.store(0, SeqCst);
            d.total.store(0, SeqCst);
            d.sent.store(0, SeqCst);
            d.bytes.store(0, SeqCst);
        }
    }
    for _ in 0..DOWNLOADS {
        let i = with(|s| {
            let i = (s.download_next % DOWNLOADS as u32) as usize;
            s.download_next = s.download_next.wrapping_add(1);
            i
        });
        let d = shared(i);
        let state = d.state.load(SeqCst);
        if with(|s| s.download_busy[i]) || (state != DL_EMPTY && state != DL_DONE && state != DL_FAILED) {
            continue;
        }
        with(|s| s.download_ids[i].set_bytes(cbytes(id)));
        d.pages.store(0, SeqCst);
        d.total.store(0, SeqCst);
        d.sent.store(0, SeqCst);
        d.bytes.store(0, SeqCst);
        d.state.store(DL_QUEUED, SeqCst);
        return Some(i);
    }
    None
}

// One connection: it reads its request, then writes its answer, waits for a render slot, or is
// handed to a forked child. Each has its own buffer, so a new connection is never answered from
// an older one's request, and a slow or idle one (browsers open spare connections) holds up
// nobody.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ClientState {
    Free,    // C_FREE
    Reading, // C_READING
    Writing, // C_WRITING
    Queued,  // C_QUEUED
}
struct Client {
    st: ClientState,
    fd: i32,
    req: [u8; 4096],
    reqlen: usize,
    out: Buf,
    sent: usize,
    /// last progress; for Queued, when it began waiting
    t0: u32,
    /// arrival order: queued renders start oldest first
    seq: u32,
    /// the record of the PDF it asked progress for
    download: Option<usize>,
}
impl Client {
    fn new() -> Client {
        Client {
            st: ClientState::Free,
            fd: 0,
            req: [0; 4096],
            reqlen: 0,
            out: Buf::default(),
            sent: 0,
            t0: 0,
            seq: 0,
            download: None,
        }
    }
}

const MAX_CLIENTS: usize = 16;
// Renders run in forked children, two at a time; more wait their turn (Queued). A page of
// previews asks for many at once.
const MAX_CHILDREN: usize = 2;

struct State {
    download_ids: [FixedStr<33>; DOWNLOADS],
    /// parent only: the child still owns this record
    download_busy: [bool; DOWNLOADS],
    download_next: u32,
    child_response: bool,
    child_head: bool,
    listen: i32,
    dpi: f32,
    gothic: bool,
    served: i32,
    /// HTTPD_SHARE, HTTPD_LIVE: who keeps the server up
    owners: u32,
    live_pairings: i32,
    live_keys: LiveKeys,
    /// the request being answered is a HEAD
    head: bool,
    cl: Vec<Client>,
    /// the connection a route is answering (g_cur)
    cur: usize,
    seq: u32,
    children: [libc::pid_t; MAX_CHILDREN],
    child_download: [Option<usize>; MAX_CHILDREN],
}

impl Default for State {
    fn default() -> State {
        State {
            download_ids: [FixedStr::<33>::default(); DOWNLOADS],
            download_busy: [false; DOWNLOADS],
            download_next: 0,
            child_response: false,
            child_head: false,
            listen: -1,
            dpi: 300.0,
            gothic: true,
            served: 0,
            owners: 0,
            live_pairings: 0,
            live_keys: LiveKeys::default(),
            head: false,
            cl: (0..MAX_CLIENTS).map(|_| Client::new()).collect(),
            cur: 0,
            seq: 0,
            children: [0; MAX_CHILDREN],
            child_download: [None; MAX_CHILDREN],
        }
    }
}

thread_local! {
    static S: RefCell<State> = RefCell::new(State::default());
}
fn with<R>(f: impl FnOnce(&mut State) -> R) -> R {
    S.with(|s| f(&mut s.borrow_mut()))
}

fn dpi() -> f32 {
    with(|s| s.dpi)
}
fn gothic() -> bool {
    with(|s| s.gothic)
}
fn owners() -> u32 {
    with(|s| s.owners)
}
fn cur_fd() -> i32 {
    with(|s| s.cl[s.cur].fd)
}
/// g_cur->req, as the C string it is
fn cur_req() -> Vec<u8> {
    with(|s| cbytes(&s.cl[s.cur].req[..s.cl[s.cur].reqlen]).to_vec())
}

fn reap_children() {
    for i in 0..MAX_CHILDREN {
        let pid = with(|s| s.children[i]);
        // SAFETY: waitpid on a child this module forked and has not reaped (pid > 0); WNOHANG, and
        // no status is wanted.
        if pid > 0 && unsafe { libc::waitpid(pid, std::ptr::null_mut(), libc::WNOHANG) } > 0 {
            if let Some(d) = with(|s| s.child_download[i]) {
                if shared(d).state.load(SeqCst) != DL_DONE {
                    shared(d).state.store(DL_FAILED, SeqCst);
                }
                with(|s| s.download_busy[d] = false);
            }
            with(|s| {
                s.children[i] = 0;
                s.child_download[i] = None;
            });
        }
    }
}

fn set_nonblock(fd: i32) {
    // SAFETY: fcntl on a descriptor this module owns, with no pointer arguments.
    unsafe {
        libc::fcntl(fd, libc::F_SETFL, libc::fcntl(fd, libc::F_GETFL, 0) | libc::O_NONBLOCK);
    }
}

fn httpd_start(port: i32, dpi: f32, gothic: bool) -> i32 {
    httpd_stop();
    // SAFETY: signal(2) with SIG_IGN installs no handler code; the disposition is the process's, as in the C.
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_IGN);
    }
    with(|s| {
        s.dpi = dpi;
        s.gothic = gothic;
    });
    // SAFETY: socket takes no pointers.
    let fd = unsafe { libc::socket(libc::AF_INET, libc::SOCK_STREAM, 0) };
    if fd < 0 {
        return -1;
    }
    let one: libc::c_int = 1;
    // SAFETY: setsockopt reads sizeof(int) bytes from the local `one`, on a socket we own.
    unsafe {
        libc::setsockopt(
            fd,
            libc::SOL_SOCKET,
            libc::SO_REUSEADDR,
            &one as *const libc::c_int as *const libc::c_void,
            std::mem::size_of::<libc::c_int>() as libc::socklen_t,
        );
    }
    // SAFETY: sockaddr_in is plain data; all-zero is a valid value (the C's `= {0}`).
    let mut sa: libc::sockaddr_in = unsafe { std::mem::zeroed() };
    sa.sin_family = libc::AF_INET as libc::sa_family_t;
    sa.sin_port = (port as u16).to_be();
    sa.sin_addr.s_addr = libc::INADDR_ANY.to_be();
    // SAFETY: bind reads sizeof(sockaddr_in) bytes from the local sa.
    let bound = unsafe {
        libc::bind(
            fd,
            &sa as *const libc::sockaddr_in as *const libc::sockaddr,
            std::mem::size_of::<libc::sockaddr_in>() as libc::socklen_t,
        )
    };
    // SAFETY: listen takes no pointers.
    if bound != 0 || unsafe { libc::listen(fd, 64) } != 0 {
        qlog!("httpd: bind/listen on %d failed: %s", port, &strerror(errno()));
        // SAFETY: closes the socket made above.
        unsafe { libc::close(fd) };
        return -1;
    }
    set_nonblock(fd);
    with(|s| s.listen = fd);
    qlog!("httpd: listening on port %d", port);
    0
}

/// Keeps the server up for owner (HTTPD_SHARE or HTTPD_LIVE), starting it on 0.0.0.0:port if no
/// owner had it: 0, or -1 when it could not start. gothic: the pages use the Gothic style's fonts.
pub fn httpd_acquire(owner: u32, port: i32, dpi: f32, gothic: bool) -> i32 {
    if with(|s| s.listen) < 0 && httpd_start(port, dpi, gothic) != 0 {
        return -1;
    }
    with(|s| s.owners |= owner);
    0
}

/// owner is done with it: the server stops once no owner is left (its downloads and connections
/// with it), and the routes only owner had stop answering at once.
pub fn httpd_release(owner: u32) {
    let none = with(|s| {
        s.owners &= !owner;
        if owner & HTTPD_LIVE != 0 { s.live_keys = LiveKeys::default(); }
        s.owners == 0
    });
    if none {
        httpd_stop();
    }
}

pub fn httpd_owners() -> u32 {
    owners()
}
/// Browsers that paired for Live view since the start (the pairing card closes when it moves on).
/// Load once per Live view session; a restart picks up edits to live-keys.ini.
pub fn httpd_live_keys_load(path: &str) {
    let map = crate::live_keys::live_keys_load(path);
    with(|s| s.live_keys.reset(map));
}
pub fn httpd_live_key_pop() -> Option<LiveKey> {
    with(|s| s.live_keys.pop(now_ms()))
}
pub fn httpd_live_key_action(key: &LiveKey) -> i32 {
    with(|s| shortcuts_lookup(&s.live_keys.map, false, key.code, key.mods))
}
pub fn httpd_live_key_held(key: &LiveKey) -> bool {
    live_on() && owners() & HTTPD_LIVE != 0 && with(|s| s.live_keys.is_held(key, now_ms()))
}
pub fn httpd_live_pairings() -> i32 {
    with(|s| s.live_pairings)
}

pub fn httpd_running() -> bool {
    with(|s| s.listen) >= 0
}
/// listening fd (for input_watch_fd), -1 if stopped
pub fn httpd_fd() -> i32 {
    with(|s| s.listen)
}
pub fn httpd_requests_served() -> i32 {
    with(|s| s.served)
}

fn client_close(i: usize) {
    if let Some(d) = with(|s| s.cl[i].download) {
        if shared(d).state.load(SeqCst) == DL_QUEUED {
            shared(d).state.store(DL_FAILED, SeqCst);
        }
    }
    let (st, fd) = with(|s| {
        let c = &mut s.cl[i];
        c.download = None;
        (c.st, c.fd)
    });
    if st != ClientState::Free && fd >= 0 {
        // SAFETY: closes the connection's descriptor, which this module owns.
        unsafe { libc::close(fd) };
    }
    with(|s| {
        let c = &mut s.cl[i];
        c.fd = -1;
        c.st = ClientState::Free;
        c.reqlen = 0;
        c.req[0] = 0;
        c.sent = 0;
        buf_free(&mut c.out);
    });
}

/// Stops it whoever has it (Grimoire leaving).
pub fn httpd_stop() {
    for i in 0..MAX_CLIENTS {
        client_close(i);
    }
    let fd = with(|s| s.listen);
    if fd >= 0 {
        // SAFETY: closes the listening socket, which this module owns.
        unsafe { libc::close(fd) };
    }
    with(|s| {
        s.listen = -1;
        s.owners = 0;
        s.live_keys = LiveKeys::default();
    });
}

// ---- helpers ----

// Text as HTML, safe in an element and in an attribute in either quotes: names, and every
// translation (a catalog is third-party input; none may add markup).
fn html_esc_n(b: &mut Buf, s: &[u8]) {
    for &ch in s {
        if ch == 0 {
            break;
        }
        match ch {
            b'<' => buf_printf!(b, "&lt;"),
            b'>' => buf_printf!(b, "&gt;"),
            b'&' => buf_printf!(b, "&amp;"),
            b'"' => buf_printf!(b, "&quot;"),
            b'\'' => buf_printf!(b, "&#39;"),
            _ => buf_append(b, &[ch]),
        };
    }
}

fn html_esc(b: &mut Buf, s: &[u8]) {
    html_esc_n(b, cbytes(s));
}

// Whether the UTF-8 character at s is a letter past ASCII: U+00C0 to U+1FFF (Latin with its
// accents, Greek, Cyrillic and the rest of the alphabets), but for the multiplication and division
// signs. Signs below (inverted marks, quotes) and above (punctuation, arrows) are not.
// (s ends in a NUL, as a C string does: a byte past the slice reads as 0.)
fn accented_letter(s: &[u8]) -> bool {
    let at = |i: usize| s.get(i).copied().unwrap_or(0);
    if at(0) == 0xC3 {
        return at(1) != 0x97 && at(1) != 0xB7;
    }
    at(0) >= 0xC4 && at(0) <= 0xE1
}

// Text in Gothic's bold (.big): the bold is a shadow one design pixel to the right, which fills
// the gaps inside an accent (the dots of an u-umlaut run into a bar, the tilde of an n-tilde turns
// into a lump), as the Kindle's overprint did before font.c spared the marks. So the letters past
// ASCII go without it, in spans of class na (studio.js does the same): legible, a little lighter
// than the rest.
fn html_esc_bold(b: &mut Buf, s: &[u8]) {
    if !gothic() {
        return html_esc(b, s);
    }
    let mut s = cbytes(s);
    while !s.is_empty() {
        let letters = accented_letter(s);
        let mut n = 0usize;
        while n < s.len() && accented_letter(&s[n..]) == letters {
            n += 1;
            while n < s.len() && (s[n] & 0xC0) == 0x80 {
                n += 1; // the rest of the character
            }
        }
        if letters {
            buf_printf!(b, "<span class=na>");
        }
        html_esc_n(b, &s[..n]);
        if letters {
            buf_printf!(b, "</span>");
        }
        s = &s[n..];
    }
}

fn url_esc(b: &mut Buf, s: &[u8]) {
    for &p in cbytes(s) {
        if p.is_ascii_lowercase() || p.is_ascii_uppercase() || p.is_ascii_digit() || b"-_.~".contains(&p) {
            buf_append(b, &[p]);
        } else {
            buf_printf!(b, "%%%02X", p);
        }
    }
}

// Also safe inside an HTML <script>: < > & are escaped, so a name cannot close the script.
fn json_str(b: &mut Buf, s: &[u8]) {
    buf_append(b, b"\"");
    for &p in cbytes(s) {
        if p == b'"' || p == b'\\' {
            buf_printf!(b, "\\%c", p as char);
        } else if p < 0x20 || p == b'<' || p == b'>' || p == b'&' {
            buf_printf!(b, "\\u%04x", p);
        } else {
            buf_append(b, &[p]);
        }
    }
    buf_append(b, b"\"");
}

fn buf_puts(b: &mut Buf, s: &str) {
    buf_append(b, s.as_bytes());
}

/// strtol(hx, NULL, 16) of the two characters after a '%'.
fn hex2(a: u8, b: u8) -> i64 {
    let s = [a, b];
    let mut i = 0;
    while i < 2 && matches!(s[i], b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r') {
        i += 1;
    }
    let mut neg = false;
    if i < 2 && (s[i] == b'+' || s[i] == b'-') {
        neg = s[i] == b'-';
        i += 1;
    }
    // (a "0x" prefix counts only with a hex digit after it, and two characters leave no room: the 0 is
    // the number then, as it is here)
    let mut v: i64 = 0;
    while i < 2 {
        match (s[i] as char).to_digit(16) {
            Some(d) => v = v * 16 + d as i64,
            None => break,
        }
        i += 1;
    }
    if neg {
        -v
    } else {
        v
    }
}

/// url_decode(s) of a C string, in a new buffer. The C decodes in place; the result is read as a C
/// string everywhere, so it ends at its first NUL (a %00, or a %xx that is not hex: strtol gives 0).
fn url_decode(s: &[u8]) -> Vec<u8> {
    let s = cbytes(s);
    let mut o: Vec<u8> = Vec::with_capacity(s.len());
    let mut i = 0;
    while i < s.len() {
        let at = |k: usize| s.get(k).copied().unwrap_or(0);
        if s[i] == b'%' && at(i + 1) != 0 && at(i + 2) != 0 {
            o.push(hex2(at(i + 1), at(i + 2)) as u8);
            i += 3;
        } else if s[i] == b'+' {
            o.push(b' ');
            i += 1;
        } else {
            o.push(s[i]);
            i += 1;
        }
    }
    cbytes(&o).to_vec()
}

// Value of `key` in a query string ("a=1&layer=ink"), decoded and cut to cap - 1 bytes (before
// the decoding, as the C copies it first); None if absent.
fn query_get(query: &[u8], key: &str, cap: usize) -> Option<Vec<u8>> {
    let key = key.as_bytes();
    let kl = key.len();
    for p in cbytes(query).split(|&b| b == b'&') {
        if p.len() < kl + 1 || &p[..kl] != key || p[kl] != b'=' {
            continue;
        }
        let v = &p[kl + 1..];
        let n = v.len().min(cap - 1);
        return Some(url_decode(&v[..n]));
    }
    None
}

fn child_send(data: &[u8], d: Option<usize>) -> bool {
    let fd = cur_fd();
    let mut sent = 0usize;
    while sent < data.len() {
        let n = (data.len() - sent).min(65536);
        // SAFETY: send reads n bytes from data[sent..], which holds at least that many.
        let w = unsafe { libc::send(fd, data[sent..].as_ptr() as *const libc::c_void, n, MSG_NOSIGNAL) };
        if w < 0 && errno() == libc::EINTR {
            continue;
        }
        if w <= 0 {
            return false;
        }
        sent += w as usize;
        if let Some(d) = d {
            shared(d).sent.store(sent as u32, SeqCst);
        }
    }
    true
}

fn download_pages(done: i32, total: i32, d: Option<usize>) {
    let Some(d) = d else { return };
    shared(d).total.store(total as u32, SeqCst);
    shared(d).pages.store(done as u32, SeqCst);
}

// The answer to g_cur. cache: a Cache-Control value, None for no-store (everything but the
// fonts); extra: more header lines, each ending in \r\n, or None.
fn respond_bytes(
    code: i32,
    ctype: &str,
    data: &[u8],
    download_name: Option<&[u8]>,
    cache: Option<&str>,
    extra: Option<&str>,
) {
    let reason = if code == 200 {
        "OK"
    } else if code == 404 {
        "Not Found"
    } else if code == 409 {
        "Conflict"
    } else if code == 429 {
        "Too Many Requests"
    } else if code == 405 {
        "Method Not Allowed"
    } else if code == 400 {
        "Bad Request"
    } else if code == 503 {
        "Service Unavailable"
    } else if code == 302 {
        "Found"
    } else if code == 401 {
        "Unauthorized"
    } else if code == 403 {
        "Forbidden"
    } else {
        "Internal Server Error"
    };
    // (the C appends to g_cur->out, empty at this point: the connection answers once)
    let mut o = with(|s| std::mem::take(&mut s.cl[s.cur].out));
    buf_printf!(
        &mut o,
        "HTTP/1.0 %d %s\r\nContent-Type: %s\r\nContent-Length: %zu\r\nConnection: close\r\n\
         Cache-Control: %s\r\n%s",
        code,
        reason,
        ctype,
        data.len(),
        cache.unwrap_or("no-store"),
        extra.unwrap_or("")
    );
    if let Some(name) = download_name {
        let mut ascii: Vec<u8> = Vec::new(); // char ascii[STORE_NAME_LEN + 8]
        for &p in cbytes(name) {
            if ascii.len() + 1 >= STORE_NAME_LEN + 8 {
                break;
            }
            ascii.push(if p < 128 && p != b'"' && p != b'\\' { p } else { b'_' });
        }
        // (ASCII only: every byte was cut to 7 bits above)
        let ascii = String::from_utf8_lossy(&ascii).into_owned();
        buf_printf!(&mut o, "Content-Disposition: attachment; filename=\"%s\"; filename*=UTF-8''", &ascii);
        url_esc(&mut o, name);
        buf_printf!(&mut o, "\r\n");
    }
    buf_printf!(&mut o, "\r\n");
    if with(|s| s.child_response) {
        // The fork owns a blocking socket. Send directly from the export buffer instead
        // of copying the whole PDF/ZIP into a second, equally large response buffer.
        let d = with(|s| s.cl[s.cur].download);
        if let Some(d) = d {
            shared(d).bytes.store(data.len() as u32, SeqCst);
            shared(d).state.store(DL_SENDING, SeqCst);
        }
        let mut ok = !o.err && child_send(&o.data, None);
        if ok && !with(|s| s.child_head) {
            ok = child_send(data, d);
        }
        if let Some(d) = d {
            shared(d).state.store(if ok && code == 200 { DL_DONE } else { DL_FAILED }, SeqCst);
        }
        buf_free(&mut o);
        return;
    }
    if !data.is_empty() {
        buf_append(&mut o, data);
    }
    with(|s| {
        let c = &mut s.cl[s.cur];
        c.out = o;
        c.st = ClientState::Writing;
        c.sent = 0;
    });
}

fn respond(code: i32, ctype: &str, body: &Buf, download_name: Option<&[u8]>) {
    respond_bytes(code, ctype, &body.data, download_name, None, None);
}

/// export_safe_name on bytes, as the C has it (export.rs's takes text, and drops the cut character a
/// long name ends in, where the C keeps its bytes): a download's file name from a name, without the
/// characters a file system or a header would mind.
fn safe_name(in_: &[u8], cap: usize) -> Vec<u8> {
    let mut o: Vec<u8> = Vec::new();
    for &ch in cbytes(in_) {
        if o.len() + 1 >= cap {
            break;
        }
        o.push(if ch < 32 || b"/\\:*?\"<>|".contains(&ch) { b'_' } else { ch });
    }
    // Trim leading/trailing spaces and dots (hidden files, Windows quirks).
    let lead = o.iter().take_while(|&&c| c == b' ' || c == b'.').count();
    o.drain(..lead);
    while o.last().is_some_and(|&c| c == b' ' || c == b'.') {
        o.pop();
    }
    if o.is_empty() {
        o = b"Untitled".to_vec();
        o.truncate(cap.saturating_sub(1));
    }
    o
}

// A plain-text answer. The ones a person can meet in a browser (a link to a notebook since
// deleted, an export that failed) come in the Kindle's language; those only a script or a
// hand-made address gets (bad request, layer is ink, paper or all) stay English.
fn respond_text(code: i32, msg: &str) {
    let mut b = Buf::default();
    buf_printf!(&mut b, "%s\n", msg);
    respond(code, "text/plain; charset=utf-8", &b, None);
    buf_free(&mut b);
}

// Grimoire's pixel bat (bat.h) as an SVG path: a rect for each run of blocks in a row, a block one
// unit, in a BAT_W x BAT_H box.
fn bat_path(b: &mut Buf) {
    for r in 0..BAT_H {
        let row = K_BAT[r as usize].as_bytes();
        for c in 0..BAT_W {
            if row[c as usize] != b'X' || (c > 0 && row[(c - 1) as usize] == b'X') {
                continue;
            }
            let mut n = 1;
            while c + n < BAT_W && row[(c + n) as usize] == b'X' {
                n += 1;
            }
            buf_printf!(b, "M%d %dh%dv1h-%dz", c, r, n, n);
        }
    }
}

// The bat in a page, in the text's color (so it turns with a dark or inverted page), its blocks
// whole design pixels (.bat, below). Decorative: the title beside it says the name.
fn bat_svg(b: &mut Buf) {
    buf_printf!(b, "<svg class=bat viewBox='0 0 %d %d' aria-hidden=true><path d='", BAT_W, BAT_H);
    bat_path(b);
    buf_puts(b, "'/></svg>");
}

// The pages are dark (the Kindle's own pictures in them stay as the Kindle draws them: thumbnails,
// pages, Live view's screen), in the colors of :root, which web/live.css and web/studio.css use too.
// The pages wear the Kindle's style. Classic is Terminess throughout. Gothic sets titles in
// Jacquard 24 and text in Jersey 10, pixel faces whose outlines sit on a grid of design pixels
// (30 and 75 font units): sized so one design pixel is a whole number of device pixels (--px
// for text, --pxs for small print, --tp/--tp1 for titles, set from the screen's pixel ratio)
// they stay sharp, and bold is the Kindle's overprint, one design pixel to the right.
const K_CSS_COMMON: &str = concat!(
    ":root{--fg:#eee;--bg:#181818;--soft:#555;--quiet:#aaa;--well:#242424;--field:#000;color-scheme:dark}",
    "body{margin:24px;color:var(--fg);background:var(--bg)}",
    "h1{margin:0 0 4px}h2{margin:28px 0 10px;border-bottom:2px solid var(--fg)}",
    "a{color:var(--fg)}.grid{display:flex;flex-wrap:wrap;gap:18px}",
    ".card{width:200px;border:2px solid var(--fg);border-radius:10px;padding:10px}",
    ".card img{width:100%;border:1px solid var(--soft);background:#fff}",
    ".name{margin:6px 0 2px;word-break:break-word}.meta{color:var(--quiet)}",
    ".links a{margin-right:10px}",
    ".pages{display:flex;flex-wrap:wrap;gap:12px}.pages figure{margin:0}",
    ".pages img{width:280px;border:1px solid var(--soft);background:#fff;display:block}.pages figcaption{margin-top:4px}",
    ".big{display:inline-block;border:2px solid var(--fg);border-radius:8px;padding:8px 14px;text-decoration:none;",
    "margin:0 8px 8px 0}",
    ".bat{width:calc(var(--bp,2px)*43);height:calc(var(--bp,2px)*14);fill:currentColor;shape-rendering:crispEdges;",
    "flex:none}h1.brand{display:flex;align-items:center;gap:.35em}"
);
const K_CSS_CLASSIC: &str = concat!(
    "body{font-family:'Grimoire Text','Terminess Nerd Font',ui-monospace,Menlo,monospace;font-size:16px}",
    "h1{font-size:28px}h2{font-size:20px}h1,h2,.name,.big,b,strong{font-weight:bold}.meta{font-size:13px}"
);
const K_CSS_GOTHIC: &str = concat!(
    ":root{--px:1.5px;--pxs:1px;--tp:1px;--tp1:1.5px;--bp:var(--tp1)}",
    "body{font-family:'Grimoire Text',ui-monospace,monospace;font-size:calc(var(--px)*18.6667);",
    "line-height:calc(var(--px)*20);font-synthesis:none}",
    ".meta{font-size:calc(var(--pxs)*18.6667);line-height:calc(var(--pxs)*20)}",
    "h1,h2{font-family:'Grimoire Title',serif;font-weight:normal}",
    "h1{font-size:calc(var(--tp1)*43);line-height:calc(var(--tp1)*43)}",
    "h2{font-size:calc(var(--tp)*43);line-height:calc(var(--tp)*43)}",
    ".name{font-weight:normal}.big,b,strong{font-weight:normal;text-shadow:var(--px) 0 currentColor}",
    ".na{text-shadow:none}" // (html_esc_bold)
);
// One design pixel per theme size, rounded to whole device pixels (re-run on zoom).
const K_JS_PIXELS: &str = concat!(
    "<script>(function(){function f(){var d=window.devicePixelRatio||1,s=document.documentElement.style;",
    "function k(n,t){s.setProperty(n,Math.max(1,Math.round(t*d))/d+'px')}",
    "k('--px',1.25);k('--pxs',0.9);k('--tp',1);k('--tp1',1.5)}f();addEventListener('resize',f)})()</script>"
);

// The pages are in the Kindle's language (i18n.h): the lang attribute says which.
fn page_head(b: &mut Buf, title: &[u8]) {
    buf_printf!(
        b,
        "<!doctype html><html lang=%s><head><meta charset=utf-8>\
         <meta name=viewport content='width=device-width,initial-scale=1'><title>",
        i18n_current()
    );
    html_esc(b, title);
    buf_printf!(
        b,
        "</title><link rel=icon type=image/svg+xml href='/icon.svg?v=%s'>\
         <link rel=apple-touch-icon href='/apple-touch-icon.png?v=%s'>",
        GRIMOIRE_VERSION,
        GRIMOIRE_VERSION
    );
    let gothic = gothic();
    if gothic {
        buf_puts(b, K_JS_PIXELS);
        buf_printf!(
            b,
            "<style>@font-face{font-family:'Grimoire Title';src:url(/font/jacquard24.ttf?v=%s) \
             format('truetype');font-display:swap}@font-face{font-family:'Grimoire Text';\
             src:url(/font/jersey10.ttf?v=%s) format('truetype');font-display:swap}",
            GRIMOIRE_VERSION,
            GRIMOIRE_VERSION
        );
        buf_puts(b, K_CSS_GOTHIC);
    } else {
        buf_printf!(
            b,
            "<style>@font-face{font-family:'Grimoire Text';src:url(/font/terminess.ttf?v=%s) \
             format('truetype');font-weight:400;font-display:swap}@font-face{font-family:'Grimoire Text';\
             src:url(/font/terminess-bold.ttf?v=%s) format('truetype');font-weight:700;font-display:swap}",
            GRIMOIRE_VERSION,
            GRIMOIRE_VERSION
        );
        buf_puts(b, K_CSS_CLASSIC);
    }
    buf_puts(b, K_CSS_COMMON);
    buf_printf!(b, "</style></head><body class=%s>", if gothic { "gothic" } else { "classic" });
}

// The fonts the pages use: the binary's own copies, cached by the browser for a year (the
// version in the URL changes when they could).
fn route_font(name: &[u8]) {
    let fonts: [(&[u8], &[u8]); 4] = [
        (b"jacquard24.ttf", FONT_JACQUARD24),
        (b"jersey10.ttf", FONT_JERSEY10),
        (b"terminess.ttf", FONT_TERMINESS_REGULAR),
        (b"terminess-bold.ttf", FONT_TERMINESS_BOLD),
    ];
    let name = cbytes(name);
    for (n, data) in fonts {
        if name == n {
            return respond_bytes(200, "font/ttf", data, None, Some("public, max-age=31536000, immutable"), None);
        }
    }
    // TRANSLATORS: a plain-text answer in the browser: an address the share server has nothing at.
    respond_text(404, tr("not found"));
}

fn nb_card(b: &mut Buf, nb: &NotebookInfo) {
    buf_printf!(
        b,
        "<div class=card><a href='/nb/%s'><img loading=lazy src='/nb/%s/thumb.png' alt=''></a>\
         <div class=name>",
        &nb.id,
        &nb.id
    );
    html_esc(b, nb.name.as_bytes());
    let mut when = String::new();
    let mut pages = String::new();
    format_time_short(nb.modified, &mut when, 64);
    // TRANSLATORS: on the share index, under a notebook: its pages, when it last changed and its
    // paper, then links to it as a page of pictures and to the studio (Effects).
    csnprintf!(&mut pages, 64, ng_("%d page", "%d pages", nb.npages() as i64), nb.npages());
    buf_printf!(b, "</div><div class=meta>");
    html_esc(b, pages.as_bytes());
    buf_printf!(b, " &middot; ");
    html_esc(b, when.as_bytes());
    buf_printf!(b, " &middot; ");
    html_esc(b, paper_label(nb.paper).as_bytes());
    buf_printf!(
        b,
        "</div><div class=links><a href='/nb/%s.pdf'>PDF</a><a href='/nb/%s.zip'>ZIP</a><a href='/nb/%s'>",
        &nb.id,
        &nb.id,
        &nb.id
    );
    html_esc(b, c_("share link", "pages").as_bytes());
    buf_printf!(b, "</a><a href='/nb/%s/studio'>", &nb.id);
    html_esc(b, c_("share link", "effects").as_bytes());
    buf_printf!(b, "</a></div></div>");
}

fn download_footer(b: &mut Buf) {
    let words = [
        // TRANSLATORS: PDF download status in Share. {done}/{total} are page counts;
        // {percent} includes the percent sign: sent to the browser, not saved to its disk.
        tr("Preparing PDF..."),
        tr("Preparing PDF: {done} / {total} pages"),
        tr("Sending PDF: {percent}"),
        tr("Sent to your browser. Check Downloads."),
        tr("PDF download failed. Please try again."),
        tr("Download status unavailable. Check your browser's Downloads and the Kindle's Wi-Fi."),
        tr("Dismiss"),
        tr("Waiting for an export slot..."),
    ];
    buf_printf!(b, "<div id=download-status></div><script>window.GRIMOIRE_DOWNLOAD_WORDS=[");
    for (i, w) in words.iter().enumerate() {
        if i > 0 {
            buf_printf!(b, ",");
        }
        json_str(b, w.as_bytes());
    }
    buf_printf!(b, "];</script><script>");
    buf_append(b, WEB_DOWNLOAD_JS);
    buf_printf!(b, "</script></body></html>");
}

fn route_index() {
    let mut nbs: Vec<NotebookInfo> = Vec::new();
    let mut folders: Vec<String> = Vec::new();
    store_list(&mut nbs);
    store_list_folders(&mut folders);
    let n = nbs.len() as i32;
    let mut b = Buf::default();
    page_head(&mut b, b"Grimoire");
    let mut line = String::new();
    // TRANSLATORS: the share index, the page a browser opens at the Share screen's address.
    csnprintf!(&mut line, 160, ng_("%d notebook on your Kindle", "%d notebooks on your Kindle", n as i64), n);
    buf_puts(&mut b, "<h1 class=brand>");
    bat_svg(&mut b);
    buf_puts(&mut b, "Grimoire</h1><p class=meta>");
    html_esc(&mut b, line.as_bytes());
    buf_printf!(&mut b, "</p><p><a class=big href='/all.zip'>");
    // TRANSLATORS: the share index's big link to a zip of every notebook.
    html_esc_bold(&mut b, tr("Download everything (zip)").as_bytes());
    buf_printf!(&mut b, "</a></p>");
    for f in -1..folders.len() as i32 {
        let folder: &str = if f < 0 { "" } else { &folders[f as usize] };
        let mut count = 0;
        for nb in &nbs {
            count += (nb.folder.as_bytes() == folder.as_bytes()) as i32;
        }
        if f < 0 && count == 0 {
            continue;
        }
        buf_printf!(&mut b, "<h2>");
        // TRANSLATORS: the share index's heading over the notebooks in no folder.
        html_esc(&mut b, if f < 0 { tr("Library") } else { folder }.as_bytes());
        if f >= 0 {
            buf_printf!(&mut b, " <a class=meta href='/folder/");
            url_esc(&mut b, folder.as_bytes());
            buf_printf!(&mut b, ".zip'>(zip)</a>");
        }
        buf_printf!(&mut b, "</h2><div class=grid>");
        for nb in &nbs {
            if nb.folder.as_bytes() == folder.as_bytes() {
                nb_card(&mut b, nb);
            }
        }
        if count == 0 {
            buf_printf!(&mut b, "<p class=meta>");
            // TRANSLATORS: on the share index, under a folder with no notebooks.
            html_esc(&mut b, c_("folder", "empty").as_bytes());
            buf_printf!(&mut b, "</p>");
        }
        buf_printf!(&mut b, "</div>");
    }
    buf_printf!(&mut b, "<p class=meta style='margin-top:40px'>Grimoire %s</p>", GRIMOIRE_VERSION);
    download_footer(&mut b);
    respond(200, "text/html; charset=utf-8", &b, None);
    buf_free(&mut b);
}

/// strtol(s, &end, 10) of a C string: the C `long` is wider here than on the Kindle, and the one
/// caller clamps what it gets, so saturating at the i64 range reads the same.
fn strtol10(s: &[u8]) -> i64 {
    let s = cbytes(s);
    let mut i = 0;
    while i < s.len() && matches!(s[i], b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r') {
        i += 1;
    }
    let mut neg = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        neg = s[i] == b'-';
        i += 1;
    }
    let mut v: i64 = 0;
    while i < s.len() && s[i].is_ascii_digit() {
        v = v.saturating_mul(10).saturating_add((s[i] - b'0') as i64);
        i += 1;
    }
    if neg {
        -v
    } else {
        v
    }
}

fn route_notebook_html(nb: &NotebookInfo, query: &[u8]) {
    let wanted: i64 = match query_get(query, "p", 32) {
        Some(value) => strtol10(&value),
        None => 1,
    };
    let npages = nb.npages();
    let mut current: i32 = if wanted < 1 {
        1
    } else if wanted > npages as i64 {
        npages
    } else {
        wanted as i32
    };
    if current < 1 {
        current = 1;
    }
    let mut b = Buf::default();
    page_head(&mut b, nb.name.as_bytes());
    buf_printf!(
        &mut b,
        "<nav aria-label='Pages' style='position:sticky;top:0;background:var(--bg,#fff);padding:12px;z-index:1'>\
         <form method=get style='display:flex;flex-wrap:wrap;align-items:center;justify-content:center;gap:12px'>"
    );
    if current > 1 {
        buf_printf!(&mut b, "<a class=big href='?p=%d' rel=prev>&larr;</a>", current - 1);
    }
    buf_printf!(&mut b, "<label for=page-number>");
    html_esc(&mut b, tr("Page").as_bytes());
    buf_printf!(
        &mut b,
        "</label><input id=page-number name=p type=number inputmode=numeric min=1 max=%d value=%d \
         style='width:6ch;font:inherit;padding:8px;text-align:center' required onchange='this.form.submit()'><span>/ %d</span><button type=submit>",
        npages,
        current,
        npages
    );
    html_esc(&mut b, tr("Go").as_bytes());
    buf_printf!(&mut b, "</button>");
    if current < npages {
        buf_printf!(&mut b, "<a class=big href='?p=%d' rel=next>&rarr;</a>", current + 1);
    }
    buf_printf!(&mut b, "</form></nav>");
    buf_printf!(&mut b, "<p><a href='/'>&larr; ");
    // TRANSLATORS: a notebook's share page: back to the index, then its downloads (the zip holds
    // the PDF, a PNG of each page and the notebook's own files), the studio, and its pages.
    html_esc(&mut b, tr("all notebooks").as_bytes());
    buf_printf!(&mut b, "</a></p><h1>");
    html_esc(&mut b, nb.name.as_bytes());
    buf_printf!(&mut b, "</h1><p><a class=big href='/nb/%s.pdf'>PDF</a><a class=big href='/nb/%s.zip'>", &nb.id, &nb.id);
    html_esc_bold(&mut b, tr("ZIP (PDF + PNG pages + raw)").as_bytes());
    buf_printf!(&mut b, "</a><a class=big href='/nb/%s/studio'>", &nb.id);
    html_esc_bold(&mut b, tr("Effects (transparent, colors, gradients, filters)").as_bytes());
    buf_printf!(&mut b, "</a></p>");
    buf_printf!(
        &mut b,
        "<figure style='margin:16px 0;text-align:center'><a href='/nb/%s/%d.png'>\
         <img style='max-width:100%%;height:auto' src='/nb/%s/%d.png?f=2' alt='",
        &nb.id,
        current,
        &nb.id,
        current
    );
    let mut page_label = String::new();
    csnprintf!(&mut page_label, 64, tr("Page %d"), current);
    html_esc(&mut b, page_label.as_bytes());
    buf_printf!(&mut b, "'></a><figcaption>");
    html_esc(&mut b, page_label.as_bytes());
    buf_printf!(&mut b, " &middot; <a href='/nb/%s/studio?p=%d'>", &nb.id, current);
    html_esc(&mut b, c_("share link", "effects").as_bytes());
    buf_printf!(&mut b, "</a></figcaption></figure>");
    download_footer(&mut b);
    respond(200, "text/html; charset=utf-8", &b, None);
    buf_free(&mut b);
}

// The studio's words in the language in use (window.GRIMOIRE_L10N), from the catalog's context
// "studio" (make l10n puts web/studio.js's there): {"English": "translation"}, through json_str
// (safe inside the script). A plural comes in its form for the notebook's page count, the one
// count the studio words (studio.js, tPages). English: {}, and studio.js keeps its own words.
// (The walk's callback is a closure over the buffer, the count and the pages.)

// The studio: the page's controls, the notebook as JSON, and web/studio.{css,js} inline. The
// browser does the image work from /nb/<id>/<n>.png?layer=ink|paper.
fn route_studio(nb: &NotebookInfo) {
    let mut b = Buf::default();
    page_head(&mut b, nb.name.as_bytes());
    buf_printf!(&mut b, "<p><a href='/nb/%s'>&larr; ", &nb.id);
    html_esc(&mut b, nb.name.as_bytes());
    buf_printf!(&mut b, "</a></p><h1>");
    html_esc(&mut b, nb.name.as_bytes());
    buf_printf!(&mut b, "</h1><div id=studio><p class=meta>");
    // TRANSLATORS: the studio's page while its script starts, or in a browser without JavaScript.
    html_esc(&mut b, tr("Loading the studio...").as_bytes());
    buf_printf!(&mut b, "</p></div><noscript><p>");
    html_esc(&mut b, tr("The studio needs JavaScript.").as_bytes());
    buf_printf!(&mut b, "</p></noscript><script>window.GRIMOIRE_NB={\"id\":");
    json_str(&mut b, nb.id.as_bytes());
    buf_printf!(&mut b, ",\"name\":");
    json_str(&mut b, nb.name.as_bytes());
    buf_printf!(&mut b, ",\"pages\":%d,\"dpi\":%.0f,\"version\":", nb.npages(), dpi());
    json_str(&mut b, GRIMOIRE_VERSION.as_bytes());
    buf_printf!(&mut b, "};window.GRIMOIRE_L10N={");
    let pages = nb.npages() as i64;
    let mut n = 0;
    i18n_walk(i18n_current(), &mut |e: &I18nEntry| {
        if e.ctx.as_bytes() != b"studio" {
            return;
        }
        let s = match e.id_plural {
            Some(plural) => i18n_getnc("studio", e.id, plural, pages),
            None => e.str,
        };
        if s.is_empty() {
            return;
        }
        if n > 0 {
            buf_printf!(&mut b, ",");
        }
        n += 1;
        json_str(&mut b, e.id.as_bytes());
        buf_printf!(&mut b, ":");
        json_str(&mut b, s.as_bytes());
    });
    buf_printf!(&mut b, "};</script><style>");
    buf_append(&mut b, WEB_STUDIO_CSS);
    buf_printf!(&mut b, "</style><script>");
    buf_append(&mut b, WEB_STUDIO_JS);
    buf_printf!(&mut b, "</script></body></html>");
    respond(200, "text/html; charset=utf-8", &b, None);
    buf_free(&mut b);
}

fn route_list_json() {
    let mut nbs: Vec<NotebookInfo> = Vec::new();
    let mut folders: Vec<String> = Vec::new();
    store_list(&mut nbs);
    store_list_folders(&mut folders);
    let mut b = Buf::default();
    buf_printf!(&mut b, "{\"device\":\"Grimoire\",\"version\":");
    json_str(&mut b, GRIMOIRE_VERSION.as_bytes());
    buf_printf!(&mut b, ",\"notebooks\":[");
    for (i, nb) in nbs.iter().enumerate() {
        buf_printf!(&mut b, "%s{\"id\":", if i > 0 { "," } else { "" });
        json_str(&mut b, nb.id.as_bytes());
        buf_printf!(&mut b, ",\"name\":");
        json_str(&mut b, nb.name.as_bytes());
        buf_printf!(&mut b, ",\"folder\":");
        json_str(&mut b, nb.folder.as_bytes());
        buf_printf!(&mut b, ",\"pages\":%d,\"paper\":", nb.npages());
        json_str(&mut b, paper_name(nb.paper).as_bytes());
        buf_printf!(
            &mut b,
            ",\"modified\":%lld,\"pdf\":\"/nb/%s.pdf\",\"zip\":\"/nb/%s.zip\",\"pngs\":[",
            nb.modified,
            &nb.id,
            &nb.id
        );
        for p in 0..nb.npages() {
            buf_printf!(&mut b, "%s\"/nb/%s/%d.png\"", if p > 0 { "," } else { "" }, &nb.id, p + 1);
        }
        buf_printf!(&mut b, "]}");
    }
    buf_printf!(&mut b, "],\"folders\":[");
    for (f, folder) in folders.iter().enumerate() {
        if f > 0 {
            buf_printf!(&mut b, ",");
        }
        json_str(&mut b, folder.as_bytes());
    }
    buf_printf!(&mut b, "]}\n");
    respond(200, "application/json", &b, None);
    buf_free(&mut b);
}

// ---- Live view ----

// A request header's value (the first line naming it), or None. (None too for a value of cap
// bytes or more.)
fn header_get(req: &[u8], name: &str, cap: usize) -> Option<Vec<u8>> {
    let name = name.as_bytes();
    let nl = name.len();
    let req = cbytes(req);
    let mut from = 0;
    while let Some(k) = req[from..].iter().position(|&b| b == b'\n') {
        let mut h = &req[from + k + 1..];
        from += k + 1;
        if h.len() < nl || !h[..nl].eq_ignore_ascii_case(name) || h.get(nl) != Some(&b':') {
            continue;
        }
        h = &h[nl + 1..];
        while !h.is_empty() && (h[0] == b' ' || h[0] == b'\t') {
            h = &h[1..];
        }
        let n = h.iter().position(|&b| b == b'\r' || b == b'\n').unwrap_or(h.len());
        if n >= cap {
            return None;
        }
        return Some(h[..n].to_vec());
    }
    None
}

fn session_scopes() -> u32 {
    match header_get(&cur_req(), "Cookie", 1024) {
        // (a NUL ends the C string; pair_cookie_scopes reads text, and what is not UTF-8 cannot be a
        // token anyway)
        Some(cookie) => pair_cookie_scopes(&String::from_utf8_lossy(cbytes(&cookie))),
        None => 0,
    }
}

fn now_s() -> i64 {
    (now_ms() / 1000) as i64
}

fn redirect(to: &str, cookie: Option<&str>) {
    let mut hdr = String::new();
    csnprintf!(
        &mut hdr,
        256,
        "Location: %s\r\n%s%s%s",
        to,
        if cookie.is_some() { "Set-Cookie: " } else { "" },
        cookie.unwrap_or(""),
        if cookie.is_some() { "\r\n" } else { "" }
    );
    respond_bytes(302, "text/plain; charset=utf-8", &[], None, None, Some(&hdr));
}

fn live_word(b: &mut Buf, n: &mut i32, key: &str, text: &str) {
    buf_printf!(b, "%s\"%s\":", if *n > 0 { "," } else { "" }, key);
    *n += 1;
    json_str(b, text.as_bytes());
}

// The page: the code form until this browser has paired, then the viewer (web/live.{css,js}).
fn route_live_page(query: &[u8]) {
    let mut b = Buf::default();
    // TRANSLATORS: Live view's name: the Kindle's screen watched in a computer's browser. The
    // browser page's title, and the pairing card's on the Kindle.
    page_head(&mut b, tr("Live view").as_bytes());
    if session_scopes() & PAIR_LIVE != 0 {
        buf_printf!(&mut b, "<div id=live><p class=meta>");
        // TRANSLATORS: Live view's page in the browser while its script starts.
        html_esc(&mut b, tr("Connecting...").as_bytes());
        buf_printf!(&mut b, "</p></div><noscript><p>");
        // TRANSLATORS: Live view's page in a browser without JavaScript.
        html_esc(&mut b, tr("Live view needs JavaScript.").as_bytes());
        buf_printf!(&mut b, "</p></noscript><script>window.GRIMOIRE_LIVE_WORDS={");
        let mut n = 0;
        // TRANSLATORS: Live view's status in the browser, and its buttons (the browser's own words
        // for these are fine: they are not on the Kindle).
        live_word(&mut b, &mut n, "connecting", tr("Connecting..."));
        live_word(&mut b, &mut n, "reconnecting", tr("Reconnecting..."));
        live_word(&mut b, &mut n, "live", tr("Live"));
        live_word(&mut b, &mut n, "asleep", tr("The Kindle is asleep"));
        live_word(&mut b, &mut n, "pairing", tr("Pairing on the Kindle..."));
        live_word(&mut b, &mut n, "ended", tr("Live view is off on the Kindle"));
        live_word(&mut b, &mut n, "fit", tr("Fit"));
        live_word(&mut b, &mut n, "actual", tr("Actual size"));
        live_word(&mut b, &mut n, "clear", tr("Transparent paper"));
        // TRANSLATORS: Live view's switch in the browser: the picture white on black. It follows
        // Invert colors on the Kindle until it is switched; then "As on the Kindle" goes back to that.
        live_word(&mut b, &mut n, "invert", tr("Invert colors"));
        live_word(&mut b, &mut n, "follow", tr("As on the Kindle"));
        // TRANSLATORS: Live view's slider in the browser: the picture's light from white to warm,
        // following the Kindle's front light warmth until it is moved ("As on the Kindle" again).
        live_word(&mut b, &mut n, "warmth", tr("Warmth"));
        // TRANSLATORS: Live view's switch in the browser: where the Kindle's pen is, drawn over the
        // picture (a dot, see-through while it hovers over the screen, solid while it touches), in
        // recordings too.
        live_word(&mut b, &mut n, "pen", tr("Show pen"));
        // TRANSLATORS: Live view's controls in the browser for the pen's dot: its color (a few named
        // ones, and any other from the browser's color picker) and its size.
        live_word(&mut b, &mut n, "dotColor", tr("Pen dot color"));
        live_word(&mut b, &mut n, "dotRed", tr("Red"));
        live_word(&mut b, &mut n, "dotBlue", tr("Blue"));
        live_word(&mut b, &mut n, "dotGreen", tr("Green"));
        live_word(&mut b, &mut n, "dotOrange", tr("Orange"));
        live_word(&mut b, &mut n, "dotPurple", tr("Purple"));
        live_word(&mut b, &mut n, "dotCustom", tr("Other color"));
        live_word(&mut b, &mut n, "dotSize", tr("Pen dot size"));
        // TRANSLATORS: the pen dot's sizes in Live view's browser page.
        live_word(&mut b, &mut n, "small", tr("Small"));
        live_word(&mut b, &mut n, "medium", tr("Medium"));
        live_word(&mut b, &mut n, "large", tr("Large"));
        // TRANSLATORS: Live view's switches in the browser: the notebook's toolbar, and the clock and
        // battery corner, each drawn over the page in the browser or not (the Kindle is unchanged).
        live_word(&mut b, &mut n, "toolbar", tr("Show toolbar"));
        live_word(&mut b, &mut n, "statusCorner", tr("Show clock and battery"));
        // TRANSLATORS: Live view's button in the browser that fits the page to the window and tucks the
        // controls away, for drawing on the Kindle while looking at the computer; M is its key.
        live_word(&mut b, &mut n, "monitor", tr("Monitor mode (M)"));
        // TRANSLATORS: Live view's button in the browser that shows or hides its latency readout.
        live_word(&mut b, &mut n, "latency", tr("Latency"));
        // TRANSLATORS: the latency readout: how late the pen's quick line shows in the browser, from
        // when the Kindle read the pen to when the browser draws it.
        live_word(&mut b, &mut n, "sampleAge", tr("Pen age at paint"));
        // TRANSLATORS: the latency readout: how late Grimoire's exact image of the page (the real ink
        // that replaces the quick line) shows in the browser, from when the Kindle read the pen.
        live_word(&mut b, &mut n, "tileAge", tr("Page image age at paint"));
        // TRANSLATORS: the latency readout's three numbers for each line: the median, the 90th
        // percentile and the largest of the latest measurements.
        live_word(&mut b, &mut n, "stats", tr("p50 / p90 / max"));
        // TRANSLATORS: the latency readout: how far off its times may be, the Kindle's clock and the
        // computer's being compared over the network.
        live_word(&mut b, &mut n, "clockError", tr("Clock uncertainty"));
        // TRANSLATORS: the latency readout: how long the browser holds the pen's samples back so that
        // a burst of them draws as a smooth line instead of a jump.
        live_word(&mut b, &mut n, "playout", tr("Playout delay"));
        live_word(&mut b, &mut n, "record", tr("Record"));
        // TRANSLATORS: Live view's Record button where the browser can make no MP4 at all (WebM is the
        // other video format), and its tooltip saying why.
        live_word(&mut b, &mut n, "recordWebm", tr("Record (WebM)"));
        // TRANSLATORS: Live view's Record button where the browser records an MP4.
        live_word(&mut b, &mut n, "recordMp4", tr("Record (MP4)"));
        live_word(&mut b, &mut n, "webmBrowser", tr("This browser cannot record MP4, so the video is WebM."));
        live_word(&mut b, &mut n, "stop", tr("Stop recording"));
        live_word(&mut b, &mut n, "full", tr("Full screen"));
        // TRANSLATORS: Live view's button in the browser: the screen in a small window that floats over
        // the computer's other windows (the browser's own name for it is fine).
        live_word(&mut b, &mut n, "pip", tr("Pop out"));
        live_word(&mut b, &mut n, "popoutActive", tr("Live view is open in another window. Keep this tab open."));
        live_word(&mut b, &mut n, "popoutReturn", tr("Bring Live view back"));
        live_word(&mut b, &mut n, "keys", tr("Keys"));
        live_word(&mut b, &mut n, "keysHint", tr("Shortcuts control Grimoire while this page has focus. Edit live-keys.ini, then restart Live view."));
        live_word(&mut b, &mut n, "keysEmpty", tr("No Live view keys are assigned."));
        live_word(&mut b, &mut n, "keysError", tr("Could not send keys. Release the keys and try again."));
        live_word(&mut b, &mut n, "old", tr("This browser cannot show Live view."));
        buf_printf!(&mut b, "};</script><template id=live-bat>");
        bat_svg(&mut b);
        buf_printf!(&mut b, "</template><style>");
        buf_append(&mut b, WEB_LIVE_CSS);
        buf_printf!(&mut b, "</style><script>");
        buf_append(&mut b, WEB_LIVE_JS);
        buf_printf!(&mut b, "</script></body></html>");
        respond(200, "text/html; charset=utf-8", &b, None);
        buf_free(&mut b);
        return;
    }
    let e = query_get(query, "e", 16).unwrap_or_default();
    buf_printf!(&mut b, "<style>");
    buf_append(&mut b, WEB_LIVE_CSS);
    buf_printf!(&mut b, "</style><h1 class=brand>");
    bat_svg(&mut b);
    html_esc(&mut b, tr("Live view").as_bytes());
    buf_printf!(&mut b, "</h1>");
    if owners() & HTTPD_LIVE == 0 {
        buf_printf!(&mut b, "<p>");
        // TRANSLATORS: Live view's page while it is off on the Kindle.
        html_esc(&mut b, tr("Live view is off. On the Kindle, open Share and tap Live view.").as_bytes());
        buf_printf!(&mut b, "</p></body></html>");
        respond(200, "text/html; charset=utf-8", &b, None);
        buf_free(&mut b);
        return;
    }
    buf_printf!(&mut b, "<form class=code action='/live/pair' method=get><p><label>");
    // TRANSLATORS: Live view's code form in the browser: the six digits the Kindle shows.
    html_esc(&mut b, tr("Type the code shown on the Kindle:").as_bytes());
    buf_printf!(
        &mut b,
        "<br><input name=code inputmode=numeric autocomplete=one-time-code maxlength=7 required \
         autofocus></label><button>"
    );
    // TRANSLATORS: the code form's button: start watching the Kindle's screen.
    html_esc(&mut b, tr("Watch").as_bytes());
    buf_printf!(&mut b, "</button></p>");
    if e == b"wrong" || e == b"locked" {
        buf_printf!(&mut b, "<p class=err role=alert>");
        html_esc(
            &mut b,
            if e == b"wrong" {
                // TRANSLATORS: the code form after a wrong code, and after five (the Kindle shows a new one).
                tr("That code is not the one on the Kindle.")
            } else {
                tr("Too many tries. Wait 30 seconds, then type the new code.")
            }
            .as_bytes(),
        );
        buf_printf!(&mut b, "</p>");
    }
    buf_printf!(&mut b, "</form></body></html>");
    respond(200, "text/html; charset=utf-8", &b, None);
    buf_free(&mut b);
}

// Bodyless POST, with the event in a bounded non-simple header. Cross-origin scripts need a
// preflight, which this server refuses. Never accept a key on GET/HEAD or a form's simple POST.
fn route_live_key(req: &[u8]) {
    if !host_ok(req) { return respond_text(400, "bad host"); }
    if owners() & HTTPD_LIVE == 0 || !live_on() { return respond_text(404, "live view is off"); }
    if session_scopes() & PAIR_LIVE == 0 { return respond_text(401, "pair first"); }
    if header_get(req, "Transfer-Encoding", 64).is_some()
        || header_get(req, "Content-Length", 32).is_some_and(|v| v != b"0") {
        return respond_text(400, "no body allowed");
    }
    let Some(event) = header_get(req, "X-Grimoire-Key", 32) else { return respond_text(400, "key header required") };
    let Some(values) = parse_live_key(&event) else { return respond_text(400, "bad key") };
    let cookie = header_get(req, "Cookie", 1024).unwrap_or_default();
    let rc = with(|s| s.live_keys.submit(String::from_utf8_lossy(&cookie).into_owned(), values[0], values[1], values[2], now_ms()));
    respond_text(rc, if rc == 200 { "ok" } else { "key refused" });
}
fn parse_live_key(event: &[u8]) -> Option<[i32; 3]> {
    let text = std::str::from_utf8(event).ok()?;
    let mut parts = text.split(',');
    let mut values = [0; 3];
    for value in &mut values {
        let part = parts.next()?;
        if part.is_empty() || !part.bytes().all(|b| b.is_ascii_digit()) { return None; }
        *value = part.parse().ok()?;
    }
    if parts.next().is_some() { return None; }
    Some(values)
}

fn route_live(path: &[u8], query: &[u8]) {
    if path == b"/live" || path == b"/live/" {
        return route_live_page(query);
    }
    if owners() & HTTPD_LIVE == 0 {
        return respond_text(404, tr("not found"));
    }
    if path == b"/live/pair" {
        // The code as typed (spaces dropped: "482 913"), or the QR code's token.
        let cred = match query_get(query, "code", 48) {
            Some(c) => c,
            None => query_get(query, "t", 48).unwrap_or_default(),
        };
        let mut clean: Vec<u8> = Vec::new(); // char clean[48]
        for &p in cbytes(&cred) {
            if clean.len() + 1 >= 48 {
                break;
            }
            if p != b' ' && p != b'-' {
                clean.push(p);
            }
        }
        let mut tok = String::new();
        let mut cookie = String::new();
        // (the credential is compared as bytes against digits and hex, and the text a lossy
        // conversion would make of a stray byte matches neither)
        let rc = pair_redeem(&String::from_utf8_lossy(&clean), now_s(), &mut tok);
        if rc == PAIR_OK && pair_cookie_header(&tok, &mut cookie, 128) == 0 {
            with(|s| s.live_pairings += 1);
            qlog!("live: a browser paired");
            return redirect("/live", Some(&cookie));
        }
        qlog!("live: a pairing was refused (%s)", if rc == PAIR_LOCKED { "locked" } else { "wrong code" });
        return redirect(if rc == PAIR_LOCKED { "/live?e=locked" } else { "/live?e=wrong" }, None);
    }
    if path == b"/live/keys" {
        if session_scopes() & PAIR_LIVE == 0 { return respond_text(401, "pair first"); }
        let map = with(|s| s.live_keys.map.clone());
        let mut b = Buf::default();
        buf_puts(&mut b, "[");
        for (i, key) in map.bindings.iter().take(map.count as usize).enumerate() {
            buf_printf!(&mut b, "%s{\"code\":%d,\"mods\":%d,\"key\":", if i == 0 { "" } else { "," }, key.code, key.mods);
            json_str(&mut b, live_key_name(key.code).as_bytes());
            buf_puts(&mut b, ",\"action\":");
            json_str(&mut b, shortcut_action_name(key.action).unwrap_or("").as_bytes());
            buf_puts(&mut b, ",\"label\":");
            json_str(&mut b, crate::actions::action_label(key.action).as_bytes());
            buf_puts(&mut b, "}");
        }
        buf_puts(&mut b, "]");
        respond(200, "application/json", &b, None);
        return;
    }
    if path == b"/live/key" { return respond_text(405, "POST only"); }
    if path == b"/live/time" {
        if session_scopes() & PAIR_LIVE == 0 {
            return respond_text(401, "pair first");
        }
        let mut b = Buf::default();
        buf_printf!(&mut b, "{\"ms\":%u}", now_ms());
        respond(200, "application/json", &b, None);
        buf_free(&mut b);
        return;
    }
    if path == b"/live/stream" || path == b"/live/pen" {
        if session_scopes() & PAIR_LIVE == 0 {
            return respond_text(401, "pair first");
        }
        if with(|s| s.head || s.child_response) {
            return respond_text(400, "GET only");
        }
        if !live_on() {
            return respond_text(503, "live view is off");
        }
        let taken = if path == b"/live/pen" { live_take_pen_viewer(cur_fd()) } else { live_take_viewer(cur_fd()) };
        if !taken {
            // TRANSLATORS: Live view's answer to a third computer (two can watch at once).
            return respond_text(503, tr("Two computers are watching already."));
        }
        qlog!("live: a viewer started (%d watching)", live_viewers());
        client_close(with(|s| s.cur)); // the viewer child owns the connection now
        return;
    }
    respond_text(404, tr("not found"));
}

// The browser's icon for the pages: the bat in a square, black, or light where the browser is dark.
fn route_icon_svg() {
    let mut b = Buf::default();
    buf_printf!(
        &mut b,
        "<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 %g %d %d' shape-rendering='crispEdges'>\
         <style>path{fill:#111}@media (prefers-color-scheme:dark){path{fill:#eee}}</style><path d='",
        -((BAT_W - BAT_H) as f64) / 2.0,
        BAT_W,
        BAT_W
    );
    bat_path(&mut b);
    buf_puts(&mut b, "'/></svg>");
    respond_bytes(200, "image/svg+xml", &b.data, None, Some("public, max-age=31536000, immutable"), None);
    buf_free(&mut b);
}

/// stbi_write_png_to_mem on one gray plane: the PNG's bytes.
fn png_to_mem(c: &Canvas) -> Option<Vec<u8>> {
    let mut len: libc::c_int = 0;
    // SAFETY: px holds h rows of stride bytes (canvas_init makes w * h with stride w), which stb
    // reads for w x h one-component pixels; len receives the size of the block it returns.
    let p = unsafe { crate::ffi_stb::stbi_write_png_to_mem(c.px.as_ptr(), c.stride, c.w, c.h, 1, &mut len) };
    if p.is_null() {
        return None;
    }
    // SAFETY: p points at len bytes that stb wrote; they are copied out and the block, which stb
    // took from the C library's malloc, is freed once.
    let png = unsafe { std::slice::from_raw_parts(p, len as usize).to_vec() };
    // SAFETY: see above; nothing uses p after this.
    unsafe { libc::free(p as *mut libc::c_void) };
    Some(png)
}

// The icon a phone keeps for the pages (an iPhone's home screen or bookmarks): the bat in blocks of
// 4 px, black on white, 180 px square.
fn route_touch_icon() {
    const S: i32 = 180;
    const K: i32 = 4;
    let mut c = Canvas::default();
    if canvas_init(&mut c, S, S) != 0 {
        return respond_text(500, "no memory");
    }
    draw_bat(&mut c, (S - BAT_W * K) / 2, (S - BAT_H * K) / 2, K);
    let png = png_to_mem(&c);
    canvas_free(&mut c);
    let Some(png) = png else { return respond_text(500, "encode failed") };
    respond_bytes(200, "image/png", &png, None, Some("public, max-age=31536000, immutable"), None);
}

/// sscanf(s, "/%d.png%n", &page, &end) as the Kindle's libc (musl) does it: Some((page, end)) when
/// the whole format matched (the C also wants one conversion and a nonzero end). %d skips white
/// space, takes a sign and decimal digits, and stores the low 32 bits of what it read.
fn scan_page_png(s: &[u8]) -> Option<(i32, usize)> {
    let s = cbytes(s);
    if s.first() != Some(&b'/') {
        return None;
    }
    let mut i = 1;
    while i < s.len() && matches!(s[i], b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r') {
        i += 1;
    }
    let mut neg = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        neg = s[i] == b'-';
        i += 1;
    }
    let start = i;
    let mut v: u64 = 0;
    while i < s.len() && s[i].is_ascii_digit() {
        v = v.saturating_mul(10).saturating_add((s[i] - b'0') as u64);
        i += 1;
    }
    if i == start || !s[i..].starts_with(b".png") {
        return None;
    }
    let page = if neg { (v as u32).wrapping_neg() } else { v as u32 } as i32;
    Some((page, i + 4))
}

/// atoi(s)
fn atoi(s: &[u8]) -> i32 {
    strtol10(s).clamp(i32::MIN as i64, i32::MAX as i64) as i32
}

fn route(path_in: &[u8]) {
    let path_in = cbytes(path_in);
    let mut query: Vec<u8> = Vec::new();
    let mut raw = path_in;
    if let Some(q) = path_in.iter().position(|&b| b == b'?') {
        // strlcpy_q(query, q + 1, sizeof query)
        let rest = &path_in[q + 1..];
        query = rest[..rest.len().min(127)].to_vec();
        raw = &path_in[..q];
    }
    let path = url_decode(raw);
    let path = &path[..];
    if contains(path, b"..") {
        return respond_text(400, "bad path");
    }
    if path.starts_with(b"/live") && (path.len() == 5 || path[5] == b'/') {
        return route_live(path, &query);
    }
    if path.starts_with(b"/font/") {
        return route_font(&path[6..]);
    }
    if path == b"/icon.svg" {
        return route_icon_svg();
    }
    if path == b"/apple-touch-icon.png" || path == b"/apple-touch-icon-precomposed.png" {
        return route_touch_icon();
    }
    if owners() & HTTPD_SHARE == 0 {
        // TRANSLATORS: the answer in a browser to a download (or the notebooks' page) while only Live view
        // keeps the server up: the Share screen must be open for those.
        return respond_text(403, tr("Open Share on the Kindle to download."));
    }
    if path == b"/" || path == b"/index.html" {
        return route_index();
    }
    if path == b"/api/list.json" {
        return route_list_json();
    }
    if path.starts_with(b"/api/download/") {
        let Some(d) = download_find(&path[14..]) else { return respond_text(404, "unknown download") };
        let mut b = Buf::default();
        let sh = shared(d);
        buf_printf!(
            &mut b,
            "{\"state\":%u,\"pages\":%u,\"total\":%u,\"sent\":%u,\"bytes\":%u}",
            sh.state.load(SeqCst),
            sh.pages.load(SeqCst),
            sh.total.load(SeqCst),
            sh.sent.load(SeqCst),
            sh.bytes.load(SeqCst)
        );
        respond(200, "application/json", &b, None);
        buf_free(&mut b);
        return;
    }
    if path == b"/all.zip" {
        let mut b = Buf::default();
        let rc = export_folder_zip("", dpi(), &mut b);
        if rc == 0 {
            respond(200, "application/zip", &b, Some(b"Grimoire.zip"));
        } else {
            respond_text(500, tr("export failed"));
        }
        buf_free(&mut b);
        return;
    }
    if path.starts_with(b"/folder/") {
        // char folder[STORE_NAME_LEN + 8]: the longest folder name, and ".zip" after it
        let rest = &path[8..];
        let folder = &rest[..rest.len().min(STORE_NAME_LEN + 8 - 1)];
        let l = folder.len();
        if l < 5 || &folder[l - 4..] != b".zip" {
            return respond_text(404, tr("not found"));
        }
        let folder = &folder[..l - 4];
        if l - 4 >= STORE_NAME_LEN {
            return respond_text(404, tr("not found")); // longer than any folder
        }
        let mut b = Buf::default();
        let mut fname = safe_name(folder, STORE_NAME_LEN);
        fname.extend_from_slice(b".zip");
        fname.truncate(STORE_NAME_LEN + 8 - 1);
        // (the folders there are are text: one that is not matches none)
        let folder = String::from_utf8_lossy(folder).into_owned();
        if export_folder_zip(&folder, dpi(), &mut b) == 0 {
            respond(200, "application/zip", &b, Some(&fname));
        } else {
            respond_text(500, tr("export failed"));
        }
        buf_free(&mut b);
        return;
    }
    if !path.starts_with(b"/nb/") {
        return respond_text(404, tr("not found"));
    }
    // /nb/<id>[.pdf|.zip|/<n>.png[?layer=ink|paper]|/thumb.png|/studio]
    let mut rest = &path[4..];
    let il = rest.iter().position(|&b| b == b'.' || b == b'/').unwrap_or(rest.len());
    if il == 0 || il >= STORE_ID_LEN {
        return respond_text(404, tr("not found"));
    }
    let id = String::from_utf8_lossy(&rest[..il]).into_owned();
    rest = &rest[il..];
    let mut nb = Box::<NotebookInfo>::default();
    if store_load_info(&id, &mut nb) != 0 {
        return respond_text(404, tr("no such notebook"));
    }
    let nb = &*nb;
    let safe = safe_name(nb.name.as_bytes(), STORE_NAME_LEN);
    let fname: Vec<u8>;
    let mut b = Buf::default();
    let dpi = dpi();
    if rest.is_empty() || rest == b"/" {
        route_notebook_html(nb, &query);
    } else if rest == b"/studio" {
        route_studio(nb);
    } else if rest == b".pdf" {
        fname = [&safe[..], b".pdf"].concat();
        let d = with(|s| s.cl[s.cur].download);
        if export_notebook_pdf_progress(nb, dpi, &mut b, Some(&mut |done: i32, total: i32| download_pages(done, total, d))) == 0 {
            respond(200, "application/pdf", &b, Some(&fname));
        } else {
            respond_text(500, tr("export failed"));
        }
    } else if rest == b".zip" {
        let mut z = zip_begin(&mut b);
        let mut rc = export_notebook_zip(nb, dpi, &mut z, "");
        if zip_end(&mut z) != 0 {
            rc = -1;
        }
        fname = [&safe[..], b".zip"].concat();
        if rc == 0 {
            respond(200, "application/zip", &b, Some(&fname));
        } else {
            respond_text(500, tr("export failed"));
        }
    } else if rest == b"/thumb.png" {
        let mut t = Canvas::default();
        if store_load_thumb(nb.id.as_str(), &mut t) == 0 {
            let png = png_to_mem(&t);
            canvas_free(&mut t);
            if let Some(png) = png {
                buf_append(&mut b, &png);
                respond(200, "image/png", &b, None);
            } else {
                respond_text(500, "encode failed");
            }
        } else if export_page_png(nb, 0, dpi, &mut b) == 0 {
            respond(200, "image/png", &b, None); // no thumbnail yet: first page at full size
        } else {
            respond_text(404, "no thumbnail");
        }
    } else if let Some((page, _)) = scan_page_png(rest).filter(|&(p, end)| end == rest.len() && p >= 1 && p <= nb.npages()) {
        // ?layer=ink: the strokes alone on white; ?layer=paper: the empty page; ?f=2..8: shrunk
        // f times (previews). For the studio, headers carry the full page size, the paper kind
        // and, for the ink, the box it covers at full size.
        let layer = query_get(&query, "layer", 16).unwrap_or_else(|| b"all".to_vec());
        let fs = query_get(&query, "f", 8).unwrap_or_else(|| b"1".to_vec());
        let layers = if layer == b"ink" {
            PAGE_LAYER_INK
        } else if layer == b"paper" {
            PAGE_LAYER_PAPER
        } else if layer == b"all" {
            PAGE_LAYER_ALL
        } else {
            0
        };
        let f = atoi(&fs);
        let mut pr = PageRender::default();
        if layers == 0 || !(1..=8).contains(&f) {
            respond_text(400, "layer is ink, paper or all; f is 1 to 8");
        } else if export_page_png_ex(nb, page - 1, dpi, layers, f, Some(&mut pr), &mut b) == 0 {
            let mut hdr = String::new();
            let n = csnprintf!(&mut hdr, 160, "X-Page-Size: %dx%d\r\nX-Paper: %u\r\n", pr.w, pr.h, pr.paper);
            if layers == PAGE_LAYER_INK && n > 0 && (n as usize) < 160 {
                let mut more = String::new();
                let room = 160 - n as usize;
                if rect_is_empty(pr.ink) {
                    csnprintf!(&mut more, room, "X-Ink-Box: none\r\n");
                } else {
                    csnprintf!(&mut more, room, "X-Ink-Box: %d,%d,%d,%d\r\n", pr.ink.x, pr.ink.y, pr.ink.w, pr.ink.h);
                }
                hdr.push_str(&more);
            }
            respond_bytes(200, "image/png", &b.data, None, None, Some(&hdr));
        } else {
            respond_text(500, tr("export failed"));
        }
    } else {
        respond_text(404, tr("not found"));
    }
    buf_free(&mut b);
}

// The Host header must name the Kindle by address (or be absent). A page on another site that
// rebinds its own hostname to the Kindle's address sends that hostname, and is refused.
fn host_ok(req: &[u8]) -> bool {
    let req = cbytes(req);
    let mut from = 0;
    while let Some(k) = req[from..].iter().position(|&b| b == b'\n') {
        let mut h = &req[from + k + 1..];
        from += k + 1;
        if h.len() < 5 || !h[..5].eq_ignore_ascii_case(b"Host:") {
            continue;
        }
        h = &h[5..];
        while !h.is_empty() && (h[0] == b' ' || h[0] == b'\t') {
            h = &h[1..];
        }
        let n = h.iter().position(|&b| b == b'\r' || b == b'\n').unwrap_or(h.len());
        if n == 0 || n >= 128 {
            return false;
        }
        let host = cbytes(&h[..n]);
        if host.first() == Some(&b'[') {
            return true; // IPv6 literal
        }
        let host = match host.iter().position(|&b| b == b':') {
            Some(colon) => &host[..colon],
            None => host,
        };
        if host == b"localhost" {
            return true;
        }
        return inet_pton4(host);
    }
    true
}

fn heavy(path: &[u8]) -> bool {
    if path.starts_with(b"/all.zip") || path.starts_with(b"/folder/") {
        return true;
    }
    if !path.starts_with(b"/nb/") {
        return false;
    }
    contains(path, b".pdf") || contains(path, b".zip") || (contains(path, b".png") && !contains(path, b"thumb.png"))
}

// Milliseconds since t (signed, so a t taken a moment after `now` is not 49 days ago).
fn ms_since(now: u32, t: u32) -> i32 {
    now.wrapping_sub(t) as i32
}

fn keep_headers(out: &mut Buf) {
    // HEAD: the answer without its body
    if let Some(end) = out.data.windows(4).position(|w| w == b"\r\n\r\n") {
        out.data.truncate(end + 4);
    }
}

/// sscanf(req, "%7s %599s", method, path) when it converts both: the first word (cut at 7 bytes)
/// and the next (cut at 599), where a word the cut leaves part of goes on in the next.
fn scan_request_line(req: &[u8]) -> Option<(Vec<u8>, Vec<u8>)> {
    let s = cbytes(req);
    let space = |b: u8| matches!(b, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r');
    let mut i = 0;
    while i < s.len() && space(s[i]) {
        i += 1;
    }
    let a = i;
    while i < s.len() && !space(s[i]) && i - a < 7 {
        i += 1;
    }
    if i == a {
        return None;
    }
    let method = s[a..i].to_vec();
    while i < s.len() && space(s[i]) {
        i += 1;
    }
    let b = i;
    while i < s.len() && !space(s[i]) && i - b < 599 {
        i += 1;
    }
    if i == b {
        return None;
    }
    Some((method, s[b..i].to_vec()))
}

// Child side of a heavy request: render, send everything (blocking), exit.
fn serve_in_child(c: usize, method: &[u8], path: &[u8]) -> ! {
    work_pump_set(None); // the pens are Grimoire's to read (common.h), never a child's
    input_forget_signal_pipe(); // (its numbers are closed below, and the render's files take them)
    let keep = with(|s| s.cl[c].fd);
    for fd in 3..1024 {
        if fd != keep {
            // SAFETY: closes descriptors the child inherited and does not use (an fd that is not open
            // fails harmlessly).
            unsafe { libc::close(fd) };
        }
    }
    // SAFETY: nice only changes this process's priority.
    if unsafe { libc::nice(5) } < 0 {
        // best effort
    }
    #[cfg(feature = "sim")]
    {
        use std::os::unix::ffi::OsStrExt;
        if let Some(slow) = std::env::var_os("GRIMOIRE_HTTPD_DELAY_MS") {
            // tests: renders as slow as the Kindle's
            sleep_ms(atoi(slow.as_bytes()));
        }
    }
    // SAFETY: fcntl on the connection's descriptor, which the child owns now, with no pointers.
    unsafe {
        libc::fcntl(keep, libc::F_SETFL, libc::fcntl(keep, libc::F_GETFL, 0) & !libc::O_NONBLOCK);
    }
    // a reader that stalls for a minute is gone
    let tv = libc::timeval { tv_sec: 60, tv_usec: 0 };
    // SAFETY: setsockopt reads a timeval from the local tv, on the connection's socket.
    unsafe {
        libc::setsockopt(
            keep,
            libc::SOL_SOCKET,
            libc::SO_SNDTIMEO,
            &tv as *const libc::timeval as *const libc::c_void,
            std::mem::size_of::<libc::timeval>() as libc::socklen_t,
        );
    }
    with(|s| {
        s.cur = c;
        s.child_response = true;
        s.child_head = method == b"HEAD";
    });
    route(path);
    // SAFETY: shutdown of the connection's write side, then _exit, which ends the child at once and
    // runs none of the parent's exit handlers or flushes (PORTING.md, The process model).
    unsafe {
        libc::shutdown(keep, libc::SHUT_WR);
        libc::_exit(0)
    }
}

fn handle_request(c: usize) {
    with(|s| {
        s.cur = c;
        s.head = false;
    });
    let req = cur_req();
    let Some((method, path)) = scan_request_line(&req) else { return respond_text(400, "bad request") };
    let head = method == b"HEAD";
    with(|s| s.head = head);
    if method == b"POST" && path == b"/live/key" {
        return route_live_key(&req);
    }
    if method != b"GET" && method != b"HEAD" {
        return respond_text(405, "GET only");
    }
    if !host_ok(&req) {
        qlog!("httpd: refused a request for another host");
        // TRANSLATORS: the answer to a browser that asked for the Kindle by another name than its
        // address (as a page on another site would).
        return respond_text(400, tr("use the address shown on the Kindle"));
    }
    // grimoire.log goes to public bug reports: a folder's name (the user's own words) stays out,
    // however the path is spelled, and so does anything that is not a plain path (a request in
    // proxy form carries this Kindle's address).
    let shown = url_decode(&path);
    // Nor does Live view's code or token, nor its query at all.
    let live = shown.starts_with(b"/live") && (shown.len() == 5 || shown[5] == b'/' || shown[5] == b'?');
    let bare = &path[..path.iter().position(|&b| b == b'?').unwrap_or(path.len())];
    // (the log is text: a byte that is not UTF-8 shows as U+FFFD there)
    let logged_path: &[u8] = if path.first() != Some(&b'/') {
        b"(not a path)"
    } else if shown.starts_with(b"/folder/") {
        b"/folder/<name>.zip"
    } else if live {
        bare
    } else {
        &path
    };
    let logged = String::from_utf8_lossy(logged_path).into_owned();
    qlog!("httpd: %s %s", &String::from_utf8_lossy(&method).into_owned(), &logged);
    if heavy(&path) && owners() & HTTPD_SHARE == 0 {
        route(&path); // (refused: no render for it)
        return;
    }
    if heavy(&path) {
        let query = path.iter().position(|&b| b == b'?');
        let ext = path.windows(4).position(|w| w == b".pdf");
        if method == b"GET" && path.starts_with(b"/nb/") {
            if let (Some(ext), Some(query)) = (ext, query) {
                let after = path.get(ext + 4).copied().unwrap_or(0);
                if after == 0 || after == b'?' {
                    if let Some(token) = query_get(&path[query + 1..], "progress", 64) {
                        let d = download_begin(&token);
                        with(|s| s.cl[c].download = d);
                        if d.is_none() {
                            return respond_text(400, "invalid or busy download identifier");
                        }
                    }
                }
            }
        }
        with(|s| s.cl[c].st = ClientState::Queued); // start_renders() takes it from here; t0 = when the request came in
        return;
    }
    route(&path);
    if method == b"HEAD" {
        with(|s| keep_headers(&mut s.cl[c].out));
    }
    with(|s| s.served += 1);
}

// True once the browser has hung up (the studio drops pages you flipped past).
fn peer_gone(fd: i32) -> bool {
    let mut b = 0u8;
    // SAFETY: recv writes at most one byte into the local b; MSG_PEEK leaves it queued.
    let r = unsafe { libc::recv(fd, &mut b as *mut u8 as *mut libc::c_void, 1, libc::MSG_PEEK | libc::MSG_DONTWAIT) };
    let e = errno();
    r == 0 || (r < 0 && e != libc::EAGAIN && e != libc::EWOULDBLOCK && e != libc::EINTR)
}

// Queued renders start oldest first while a child slot is free; one whose browser has gone is
// dropped before it costs a render.
fn start_renders() {
    loop {
        let mut next: Option<usize> = None;
        for i in 0..MAX_CLIENTS {
            let (st, fd, seq) = with(|s| (s.cl[i].st, s.cl[i].fd, s.cl[i].seq));
            if st != ClientState::Queued {
                continue;
            }
            if peer_gone(fd) {
                qlog!("httpd: dropped a render the browser no longer wants");
                client_close(i);
            } else if next.is_none() || seq < with(|s| s.cl[next.unwrap()].seq) {
                next = Some(i);
            }
        }
        let mut slot: i32 = -1;
        for i in 0..MAX_CHILDREN {
            if with(|s| s.children[i]) <= 0 {
                slot = i as i32;
            }
        }
        let Some(next) = next else { return };
        if slot < 0 {
            return;
        }
        let req = with(|s| cbytes(&s.cl[next].req[..s.cl[next].reqlen]).to_vec());
        let (method, path) = scan_request_line(&req).unwrap_or_default();
        with(|s| s.served += 1);
        let d = with(|s| s.cl[next].download);
        if let Some(d) = d {
            with(|s| s.download_busy[d] = true);
            shared(d).state.store(DL_PREPARING, SeqCst);
        }
        // SAFETY: fork; the child (pid 0) never returns from serve_in_child, which ends in _exit. The
        // quiet thread (display_fbink) holds no allocator or other lock across its waits, so the child
        // may allocate (PORTING.md, The process model).
        let pid = unsafe { libc::fork() };
        if pid == 0 {
            serve_in_child(next, &method, &path);
        }
        if pid > 0 {
            with(|s| {
                s.children[slot as usize] = pid;
                s.child_download[slot as usize] = d;
            });
            client_close(next); // the child owns the connection now
            continue;
        }
        qlog!("httpd: fork failed (%s), exporting in-process", &strerror(errno()));
        with(|s| s.cur = next);
        route(&path);
        if let Some(d) = d {
            with(|s| s.download_busy[d] = false);
            shared(d).state.store(DL_FAILED, SeqCst);
        }
        if method == b"HEAD" {
            with(|s| keep_headers(&mut s.cl[next].out));
        }
    }
}

// New connections while there is room (the rest wait in the listen backlog). With every slot
// taken and someone waiting, the oldest connection that has sent nothing for 3 s makes room.
fn accept_new(now: u32) {
    loop {
        let mut c: Option<usize> = None;
        let mut idle: Option<usize> = None;
        with(|s| {
            for i in 0..MAX_CLIENTS {
                let k = &s.cl[i];
                if k.st == ClientState::Free {
                    c = c.or(Some(i));
                } else if k.st == ClientState::Reading
                    && k.reqlen == 0
                    && ms_since(now, k.t0) > 3000
                    && idle.is_none_or(|j| k.t0 < s.cl[j].t0)
                {
                    idle = Some(i);
                }
            }
        });
        let listen = with(|s| s.listen);
        let c = match c {
            Some(c) => c,
            None => {
                let Some(idle) = idle else { return };
                let mut pf = libc::pollfd { fd: listen, events: libc::POLLIN, revents: 0 };
                // SAFETY: poll reads and writes the one pollfd of the local pf.
                if unsafe { libc::poll(&mut pf, 1, 0) } <= 0 {
                    return;
                }
                client_close(idle);
                idle
            }
        };
        // SAFETY: accept on the listening socket we own, asking for no peer address.
        let fd = unsafe { libc::accept(listen, std::ptr::null_mut(), std::ptr::null_mut()) };
        if fd < 0 {
            return;
        }
        set_nonblock(fd);
        #[cfg(target_vendor = "apple")]
        {
            let one: libc::c_int = 1;
            // SAFETY: setsockopt reads sizeof(int) bytes from the local `one`, on the new socket.
            unsafe {
                libc::setsockopt(
                    fd,
                    libc::SOL_SOCKET,
                    libc::SO_NOSIGPIPE,
                    &one as *const libc::c_int as *const libc::c_void,
                    std::mem::size_of::<libc::c_int>() as libc::socklen_t,
                );
            }
        }
        with(|s| {
            s.seq = s.seq.wrapping_add(1);
            let seq = s.seq;
            s.cl[c] = Client { st: ClientState::Reading, fd, t0: now, seq, ..Client::new() };
        });
    }
}

fn client_read(c: usize, now: u32) {
    if ms_since(now, with(|s| s.cl[c].t0)) > 30000 {
        return client_close(c); // said nothing (more) for 30 s
    }
    let (r, e) = with(|s| {
        let k = &mut s.cl[c];
        let room = k.req.len() - 1 - k.reqlen;
        // SAFETY: recv writes at most room bytes into req from reqlen on, and reqlen + room is one
        // short of the array's end.
        let r = unsafe { libc::recv(k.fd, k.req[k.reqlen..].as_mut_ptr() as *mut libc::c_void, room, 0) };
        (r, errno())
    });
    if r == 0 || (r < 0 && e != libc::EAGAIN && e != libc::EWOULDBLOCK && e != libc::EINTR) {
        return client_close(c);
    }
    if r < 0 {
        return; // the request has not arrived yet
    }
    let (done, full) = with(|s| {
        let k = &mut s.cl[c];
        k.reqlen += r as usize;
        k.req[k.reqlen] = 0;
        k.t0 = now;
        let req = cbytes(&k.req[..k.reqlen]);
        (contains(req, b"\r\n\r\n") || contains(req, b"\n\n"), k.reqlen >= k.req.len() - 1)
    });
    if done {
        handle_request(c);
    } else if full {
        with(|s| s.cur = c);
        respond_text(400, "request too large");
    }
}

fn client_write(c: usize, now: u32) {
    if ms_since(now, with(|s| s.cl[c].t0)) > 30000 {
        return client_close(c); // the reader stalled
    }
    let (w, e) = with(|s| {
        let k = &s.cl[c];
        let left = k.out.data.len() - k.sent;
        let chunk = left.min(256 * 1024);
        if chunk == 0 {
            return (0, 0);
        }
        // SAFETY: send reads chunk bytes from out.data[sent..], which holds left >= chunk of them.
        let w = unsafe { libc::send(k.fd, k.out.data[k.sent..].as_ptr() as *const libc::c_void, chunk, MSG_NOSIGNAL) };
        (w, errno())
    });
    if w < 0 && e != libc::EAGAIN && e != libc::EWOULDBLOCK && e != libc::EINTR {
        return client_close(c);
    }
    let finished = with(|s| {
        let k = &mut s.cl[c];
        if w > 0 {
            k.sent += w as usize;
            k.t0 = now; // progress resets the stall timer
        }
        k.sent >= k.out.data.len()
    });
    if finished {
        client_close(c);
    }
}

/// Accept / read / write some work; returns quickly. Call every loop iteration while running.
pub fn httpd_poll() {
    if with(|s| s.listen) < 0 {
        return;
    }
    reap_children();
    let now = now_ms();
    accept_new(now);
    for i in 0..MAX_CLIENTS {
        if with(|s| s.cl[i].st) == ClientState::Reading {
            client_read(i, now);
        }
        if with(|s| s.cl[i].st) == ClientState::Writing {
            client_write(i, now);
        }
        let (st, t0) = with(|s| (s.cl[i].st, s.cl[i].t0));
        if st == ClientState::Queued && ms_since(now, t0) > 120000 {
            client_close(i); // waited two minutes: gone
        }
    }
    start_renders();
}

const MAX_ADDRS: usize = 32;

/// inet_pton(AF_INET, text, ...) == 1: four decimal numbers of one to three digits, 255 at most, no
/// leading zero but a lone 0 (the C library's rule, which std's parser applies too).
fn inet_pton4(text: &[u8]) -> bool {
    std::str::from_utf8(cbytes(text)).is_ok_and(|t| t.parse::<Ipv4Addr>().is_ok())
}

#[cfg(feature = "sim")]
// Tests: GRIMOIRE_SIM_ADDRS="wlan0=192.168.0.20,usb0=192.168.15.244" stands in for the machine's
// own interfaces ("" for none), so the Share screen can be laid out with any set of addresses.
fn sim_addrs(spec: &[u8], out: &mut [NetAddr], max: i32) -> i32 {
    // char buf[512]
    let spec = cbytes(spec);
    let spec = &spec[..spec.len().min(511)];
    let mut n = 0i32;
    // strtok_r(buf, ", ", ...): tokens between runs of commas and spaces
    for t in spec.split(|&b| b == b',' || b == b' ').filter(|t| !t.is_empty()) {
        if n >= max {
            break;
        }
        let Some(eq) = t.iter().position(|&b| b == b'=') else { continue };
        if eq == 0 {
            continue;
        }
        let ip = &t[eq + 1..];
        if !inet_pton4(ip) {
            continue;
        }
        out[n as usize].ifname.set_bytes(&t[..eq]);
        out[n as usize].ip.set_bytes(ip);
        n += 1;
    }
    n
}

// Every UP, non-loopback IPv4 address, in the order the system lists its interfaces.
fn collect_addrs(out: &mut [NetAddr], max: i32) -> i32 {
    #[cfg(feature = "sim")]
    {
        use std::os::unix::ffi::OsStrExt;
        if let Some(spec) = std::env::var_os("GRIMOIRE_SIM_ADDRS") {
            return sim_addrs(spec.as_bytes(), out, max);
        }
    }
    let mut ifs: *mut libc::ifaddrs = std::ptr::null_mut();
    let mut n = 0i32;
    // SAFETY: getifaddrs stores the head of a list it allocates into the local ifs.
    if unsafe { libc::getifaddrs(&mut ifs) } != 0 {
        return 0;
    }
    let mut i = ifs;
    while !i.is_null() && n < max {
        // SAFETY: i is a node of the list getifaddrs returned, which stays valid until freeifaddrs.
        let ifa = unsafe { &*i };
        i = ifa.ifa_next;
        if ifa.ifa_addr.is_null() {
            continue;
        }
        // SAFETY: ifa_addr is not null and points at a sockaddr, whose family field says what follows.
        if unsafe { (*ifa.ifa_addr).sa_family } as i32 != libc::AF_INET {
            continue;
        }
        if ifa.ifa_flags & libc::IFF_UP as u32 == 0 || ifa.ifa_flags & libc::IFF_LOOPBACK as u32 != 0 {
            continue;
        }
        // SAFETY: the family is AF_INET, so ifa_addr points at a sockaddr_in (read unaligned to ask
        // nothing of the allocation).
        let sa: libc::sockaddr_in = unsafe { std::ptr::read_unaligned(ifa.ifa_addr as *const libc::sockaddr_in) };
        // (inet_ntop of an IPv4 address is its four bytes in decimal, with dots; s_addr is in network order)
        let ip = Ipv4Addr::from(u32::from_be(sa.sin_addr.s_addr)).to_string();
        out[n as usize].ip.set(&ip);
        let name = if ifa.ifa_name.is_null() {
            &[][..]
        } else {
            // SAFETY: ifa_name is a NUL-terminated string owned by the list.
            unsafe { CStr::from_ptr(ifa.ifa_name) }.to_bytes()
        };
        out[n as usize].ifname.set_bytes(name);
        // usb0 on a Kindle is the USBNetwork link (192.168.15.x); still useful, keep it.
        n += 1;
    }
    // SAFETY: frees the list getifaddrs allocated, once; nothing points into it after this.
    unsafe { libc::freeifaddrs(ifs) };
    n
}

/// Best-effort list of this device's IPv4 addresses (non-loopback), "a.b.c.d" each.
pub fn net_ipv4_addrs(out: &mut Vec<String>, max: i32) -> i32 {
    let mut all = [NetAddr::default(); MAX_ADDRS];
    let n = collect_addrs(&mut all, if max < MAX_ADDRS as i32 { max } else { MAX_ADDRS as i32 });
    out.clear();
    for a in all.iter().take(n.max(0) as usize) {
        out.push(a.ip.as_str().to_string());
    }
    n
}

/// The same addresses with the interface each is on, Wi-Fi first: wlan*, then en* and eth*, then
/// the rest, usb* (USBNetwork) last. The launcher opens the share port on wlan0 only.
#[derive(Clone, Copy, Debug, Default)]
pub struct NetAddr {
    /// "a.b.c.d"
    pub ip: FixedStr<16>,
    /// "wlan0"
    pub ifname: FixedStr<16>,
}

// Wi-Fi is what a phone can reach (and the only interface the launcher opens the port on).
fn if_rank(name: &[u8]) -> i32 {
    if name.starts_with(b"wlan") {
        return 0;
    }
    if name.starts_with(b"en") || name.starts_with(b"eth") {
        return 1;
    }
    if name.starts_with(b"usb") {
        return 3;
    }
    2
}

pub fn net_ipv4_list(out: &mut [NetAddr]) -> i32 {
    let max = out.len() as i32;
    let mut all = [NetAddr::default(); MAX_ADDRS];
    let mut n = collect_addrs(&mut all, MAX_ADDRS as i32); // all of them: the wlan0 one may come last
    for i in 1..n as usize {
        // stable: the system's order within a rank
        let t = all[i];
        let mut j = i;
        while j > 0 && if_rank(all[j - 1].ifname.as_bytes()) > if_rank(t.ifname.as_bytes()) {
            all[j] = all[j - 1];
            j -= 1;
        }
        all[j] = t;
    }
    if n > max {
        n = if max < 0 { 0 } else { max };
    }
    out[..n as usize].copy_from_slice(&all[..n as usize]);
    n
}

#[cfg(test)]
mod tests {
    use super::*;

    fn esc(s: &[u8]) -> String {
        let mut b = Buf::default();
        html_esc(&mut b, s);
        String::from_utf8(b.data).unwrap()
    }
    fn bold(s: &[u8]) -> String {
        let mut b = Buf::default();
        html_esc_bold(&mut b, s);
        String::from_utf8(b.data).unwrap()
    }
    fn json(s: &[u8]) -> String {
        let mut b = Buf::default();
        json_str(&mut b, s);
        String::from_utf8(b.data).unwrap()
    }

    #[test]
    fn live_keys_header_is_exact_and_bounded() {
        assert_eq!(parse_live_key(b"44,1,1"), Some([44, 1, 1]));
        for bad in [b"1,2".as_slice(), b"1,2,3,4", b"+1,0,1", b"1, 0,1", b"1,0,1x", b"1,0,", b"999999999999,0,1"] {
            assert_eq!(parse_live_key(bad), None);
        }
    }

    #[test]
    fn text_is_escaped_for_html_and_for_json() {
        assert_eq!(esc(b"<a href=\"x\">&'"), "&lt;a href=&quot;x&quot;&gt;&amp;&#39;");
        assert_eq!(esc(b"ab\0cd"), "ab");
        assert_eq!(json(b"a\"b\\c<d>&e\n\x01"), "\"a\\\"b\\\\c\\u003cd\\u003e\\u0026e\\u000a\\u0001\"");
        let mut b = Buf::default();
        url_esc(&mut b, "a b/\u{e9}-_.~".as_bytes());
        assert_eq!(b.data, b"a%20b%2F%C3%A9-_.~");
    }

    #[test]
    fn gothic_bold_leaves_accents_without_the_shadow() {
        // the letters past ASCII (U+00C0 to U+1FFF but the signs x and /) go in spans of class na
        assert_eq!(bold("\u{dc}ber".as_bytes()), "<span class=na>\u{dc}</span>ber");
        assert_eq!(bold("caf\u{e9} x".as_bytes()), "caf<span class=na>\u{e9}</span> x");
        assert_eq!(bold("a \u{d7} b \u{f7} c".as_bytes()), "a \u{d7} b \u{f7} c");
        assert_eq!(bold("\u{3b1}\u{3b2}\u{3b3} abc".as_bytes()), "<span class=na>\u{3b1}\u{3b2}\u{3b3}</span> abc");
        assert_eq!(bold("\u{2014} <b>".as_bytes()), "\u{2014} &lt;b&gt;");
        assert!(accented_letter("\u{c0}".as_bytes()) && !accented_letter("\u{bf}".as_bytes()) && !accented_letter(b"a"));
        assert!(accented_letter("\u{1fff}".as_bytes()) && !accented_letter("\u{2000}".as_bytes()));
        // a cut character at the end reads as the C reads it: its continuation byte is 0
        assert!(accented_letter(b"\xc3") && !accented_letter(b""));
        // the Classic style has no shadow to spare
        with(|s| s.gothic = false);
        assert_eq!(bold("\u{dc}ber".as_bytes()), "\u{dc}ber");
        with(|s| s.gothic = true);
    }

    #[test]
    fn urls_decode_as_strtol_reads_them() {
        assert_eq!(url_decode(b"a%20b+c"), b"a b c");
        assert_eq!(url_decode(b"%41%4a%4A"), b"AJJ");
        assert_eq!(url_decode(b"%2e%2e"), b"..");
        // too short to be an escape: kept as it is
        assert_eq!(url_decode(b"%"), b"%");
        assert_eq!(url_decode(b"100%"), b"100%");
        assert_eq!(url_decode(b"a%4"), b"a%4");
        // strtol reads a sign and white space, and 0 for no digits: a NUL, which ends the C string
        assert_eq!(url_decode(b"ab%zzcd"), b"ab");
        assert_eq!(url_decode(b"ab%00cd"), b"ab");
        assert_eq!(url_decode(b"a%0xb"), b"a");
        assert_eq!(url_decode(b"% 1x"), b"\x01x");
        assert_eq!(url_decode(b"%+fx"), b"\x0fx");
        assert_eq!(url_decode(b"%-1x"), b"\xffx");
        assert_eq!(url_decode(b"%-fx"), b"\xf1x");
        assert_eq!(url_decode(b"%ff"), b"\xff");
        assert_eq!(url_decode(b"a\0b"), b"a");
    }

    #[test]
    fn query_values_are_cut_then_decoded() {
        let q = b"a=1&layer=ink&f=2";
        assert_eq!(query_get(q, "layer", 16).unwrap(), b"ink");
        assert_eq!(query_get(q, "a", 16).unwrap(), b"1");
        assert_eq!(query_get(q, "f", 16).unwrap(), b"2");
        assert_eq!(query_get(q, "g", 16), None);
        assert_eq!(query_get(b"layer=in%6b", "layer", 16).unwrap(), b"ink");
        // the first of two wins, a longer key or one without = is another key
        assert_eq!(query_get(b"p=2&p=3", "p", 32).unwrap(), b"2");
        assert_eq!(query_get(b"layerx=1&layer=2", "layer", 32).unwrap(), b"2");
        assert_eq!(query_get(b"layer&f=1", "layer", 32), None);
        assert_eq!(query_get(b"x=1&&f=2", "f", 8).unwrap(), b"2");
        assert_eq!(query_get(b"f=", "f", 8).unwrap(), b"");
        // cap - 1 bytes of it, before the decoding (a cut escape stays as it is)
        assert_eq!(query_get(b"p=123456", "p", 4).unwrap(), b"123");
        assert_eq!(query_get(b"p=1%32", "p", 4).unwrap(), b"1%3");
        assert_eq!(query_get(b"", "p", 4), None);
    }

    #[test]
    fn the_request_line_is_read_as_sscanf_reads_it() {
        let m = |s: &[u8]| scan_request_line(s).map(|(a, b)| (String::from_utf8(a).unwrap(), String::from_utf8(b).unwrap()));
        assert_eq!(m(b"GET /x HTTP/1.0\r\n\r\n"), Some(("GET".into(), "/x".into())));
        assert_eq!(m(b"  GET \t /api/list.json   HTTP/1.0"), Some(("GET".into(), "/api/list.json".into())));
        assert_eq!(m(b"\r\nGET / x"), Some(("GET".into(), "/".into())));
        // a first word of more than 7 bytes goes on as the second
        assert_eq!(m(b"GETTTTTTTT /x"), Some(("GETTTTT".into(), "TTT".into())));
        assert_eq!(m(b"GET"), None);
        assert_eq!(m(b"GET \r\n\r\n"), None);
        assert_eq!(m(b""), None);
        assert_eq!(m(b"GET /a\0b HTTP/1.0"), Some(("GET".into(), "/a".into())));
        let long = format!("GET /{} HTTP/1.0", "a".repeat(700));
        assert_eq!(m(long.as_bytes()).unwrap().1.len(), 599);
    }

    #[test]
    fn a_page_number_is_scanned_as_percent_d() {
        assert_eq!(scan_page_png(b"/1.png"), Some((1, 6)));
        assert_eq!(scan_page_png(b"/ 12.png"), Some((12, 8)));
        assert_eq!(scan_page_png(b"/+3.png"), Some((3, 7)));
        assert_eq!(scan_page_png(b"/-1.png"), Some((-1, 7)));
        assert_eq!(scan_page_png(b"/007.png"), Some((7, 8)));
        // musl keeps the low 32 bits of what it read
        assert_eq!(scan_page_png(b"/4294967297.png"), Some((1, 15)));
        // the format must match to its end; what follows is the caller's to refuse
        assert_eq!(scan_page_png(b"/1.pngx"), Some((1, 6)));
        for no in [&b"/.png"[..], b"1.png", b"/1png", b"/1.PNG", b"/1.pn", b"/x.png", b"/", b"", b"/+.png", b"/1,png"] {
            assert_eq!(scan_page_png(no), None, "{:?}", String::from_utf8_lossy(no));
        }
    }

    #[test]
    fn numbers_are_read_as_strtol_and_atoi_read_them() {
        assert_eq!(strtol10(b"12"), 12);
        assert_eq!(strtol10(b"  -7x"), -7);
        assert_eq!(strtol10(b"+5"), 5);
        assert_eq!(strtol10(b"x5"), 0);
        assert_eq!(strtol10(b""), 0);
        assert_eq!(strtol10(b"99999999999999999999999"), i64::MAX);
        assert_eq!(strtol10(b"-99999999999999999999999"), -i64::MAX);
        assert_eq!(atoi(b"3"), 3);
        assert_eq!(atoi(b"99999999999"), i32::MAX);
        assert_eq!(atoi(b"abc"), 0);
        assert_eq!(atoi(b"2abc"), 2);
    }

    #[test]
    fn headers_are_found_by_name_after_the_request_line() {
        let req = b"GET / HTTP/1.0\r\nCookie: a=1\r\nhost:\t10.0.0.1 \r\nX-Cookie: no\r\n\r\n";
        assert_eq!(header_get(req, "Cookie", 1024).unwrap(), b"a=1");
        assert_eq!(header_get(req, "HOST", 1024).unwrap(), b"10.0.0.1 ");
        assert_eq!(header_get(req, "Accept", 1024), None);
        // the request line is not a header, and a value of cap bytes or more is refused
        assert_eq!(header_get(b"Cookie: x\r\n\r\n", "Cookie", 1024), None);
        assert_eq!(header_get(b"GET /\r\nCookie: abcd\r\n\r\n", "Cookie", 4), None);
        assert_eq!(header_get(b"GET /\r\nCookie: abc\r\n\r\n", "Cookie", 4).unwrap(), b"abc");
        assert_eq!(header_get(b"GET /\nCookie:x\n\n", "Cookie", 4).unwrap(), b"x");
        assert_eq!(header_get(b"GET /\r\nCookie : x\r\n\r\n", "Cookie", 8), None);
    }

    #[test]
    fn only_the_kindles_own_address_is_a_host() {
        let ok = |h: &str| host_ok(format!("GET / HTTP/1.0\r\nHost: {h}\r\n\r\n").as_bytes());
        assert!(host_ok(b"GET / HTTP/1.0\r\n\r\n"));
        assert!(ok("192.168.0.20:8765") && ok("192.168.0.20") && ok("localhost") && ok("localhost:80"));
        assert!(ok("[::1]:8765") && ok("[fe80::1]"));
        assert!(!ok("evil.example.com") && !ok("evil.example.com:8765") && !ok("LOCALHOST.") && !ok("1.2.3") && !ok("256.1.1.1") && !ok("1.2.3.4.5"));
        assert!(!ok("") && !ok(&"1".repeat(128)) && !ok("::1"));
        assert!(host_ok(b"GET / HTTP/1.0\nhOsT:\t10.0.0.1\n\n"));
        // the first Host header decides
        assert!(host_ok(b"GET / HTTP/1.0\r\nHost: 1.2.3.4\r\nHost: evil.com\r\n\r\n"));
        assert!(!host_ok(b"GET / HTTP/1.0\r\nHost: evil.com\r\nHost: 1.2.3.4\r\n\r\n"));
        assert!(!host_ok(b"GET / HTTP/1.0\r\nHost : 1.2.3.4\r\nHost: evil.com\r\n\r\n"));
    }

    #[test]
    fn renders_are_the_requests_that_take_a_child() {
        for yes in ["/all.zip", "/all.zip?x", "/folder/x", "/folder/", "/nb/id.pdf", "/nb/id.zip", "/nb/id/1.png", "/nb/id/1.png?f=2", "/nb/id.pdf?progress=1"] {
            assert!(heavy(yes.as_bytes()), "{yes}");
        }
        for no in ["/", "/nb/id", "/nb/id/studio", "/nb/id/thumb.png", "/nb/id/studio?p=2", "/font/x.zip", "/api/list.json", "/live", "/icon.svg", "/ALL.ZIP"] {
            assert!(!heavy(no.as_bytes()), "{no}");
        }
    }

    #[test]
    fn a_head_keeps_its_headers() {
        let mut b = Buf::default();
        buf_append(&mut b, b"HTTP/1.0 200 OK\r\nA: b\r\n\r\nbody\r\n\r\nmore");
        keep_headers(&mut b);
        assert_eq!(b.data, b"HTTP/1.0 200 OK\r\nA: b\r\n\r\n");
        let mut c = Buf::default();
        buf_append(&mut c, b"no end");
        keep_headers(&mut c);
        assert_eq!(c.data, b"no end");
    }

    #[test]
    fn times_are_milliseconds_since_even_across_the_wrap() {
        assert_eq!(ms_since(1000, 400), 600);
        assert_eq!(ms_since(5, u32::MAX - 4), 10);
        assert_eq!(ms_since(100, 101), -1);
    }

    #[test]
    fn a_download_record_is_made_once_and_recycled_when_done() {
        let id = |n: u32| format!("{n:032x}");
        assert!(download_begin(b"short").is_none());
        assert!(download_begin(format!("{}g", "a".repeat(31)).as_bytes()).is_none());
        assert!(download_find(b"a").is_none());
        let first = download_begin(id(1).as_bytes()).unwrap();
        assert_eq!(shared(first).state.load(SeqCst), DL_QUEUED);
        assert_eq!(download_find(id(1).as_bytes()), Some(first));
        // the same identifier again is refused (a finished one included)
        assert!(download_begin(id(1).as_bytes()).is_none());
        shared(first).state.store(DL_DONE, SeqCst);
        assert!(download_begin(id(1).as_bytes()).is_none() && download_find(id(1).as_bytes()) == Some(first));
        // 31 more queued ones fill the table (the first, back to queued); none can be recycled
        shared(first).state.store(DL_QUEUED, SeqCst);
        let mut ours = vec![first];
        for n in 2..=32 {
            ours.push(download_begin(id(n).as_bytes()).unwrap());
        }
        assert_eq!(ours.iter().collect::<std::collections::HashSet<_>>().len(), 32);
        assert!(download_begin(id(33).as_bytes()).is_none());
        // a record the child still owns (busy) is kept, whatever its state; a finished one goes
        shared(ours[5]).state.store(DL_FAILED, SeqCst);
        with(|s| s.download_busy[ours[5]] = true);
        assert!(download_begin(id(33).as_bytes()).is_none());
        with(|s| s.download_busy[ours[5]] = false);
        assert_eq!(download_begin(id(33).as_bytes()), Some(ours[5]));
        assert_eq!(download_find(id(6).as_bytes()), None);
        assert_eq!(shared(ours[5]).state.load(SeqCst), DL_QUEUED);
        for i in 0..DOWNLOADS {
            shared(i).state.store(DL_EMPTY, SeqCst);
        }
    }

    #[test]
    fn download_names_are_safe_names_on_bytes() {
        assert_eq!(safe_name(b"a/b:c*d?e\"f<g>h|i\\j", 96), b"a_b_c_d_e_f_g_h_i_j");
        assert_eq!(safe_name(b"  .hidden. ", 96), b"hidden");
        assert_eq!(safe_name(b"tab\there", 96), b"tab_here");
        assert_eq!(safe_name(b"", 96), b"Untitled");
        assert_eq!(safe_name(b" . ", 96), b"Untitled");
        // bytes stay bytes: a cut character, and what is not UTF-8 at all
        assert_eq!(safe_name(b"\xff\xc3", 96), b"\xff\xc3");
        assert_eq!(safe_name(&[b'a'; 200], 96).len(), 95);
        assert_eq!(safe_name(&[b'a'; 200], 8), b"aaaaaaa");
        assert_eq!(safe_name(b"", 4), b"Unt");
        // the same as export.rs's on text
        for name in ["Plain", "  x.", "A/B", "<b>\"q\"</b>", "Caf\u{e9} \u{fc}ber", "...", "", "\u{65e5}\u{672c}\u{8a9e}"] {
            let mut theirs = String::new();
            export_safe_name(name, &mut theirs, 96);
            assert_eq!(safe_name(name.as_bytes(), 96), theirs.as_bytes(), "{name}");
        }
    }

    #[test]
    fn interfaces_rank_wi_fi_first_and_usb_last() {
        assert_eq!([&b"wlan0"[..], b"eth0", b"en0", b"tun0", b"usb0", b"wlp3s0"].map(if_rank), [0, 1, 1, 2, 3, 2]);
    }

    #[test]
    fn an_ipv4_address_is_four_numbers() {
        for yes in ["0.0.0.0", "1.2.3.4", "255.255.255.255", "192.168.0.20", "10.0.0.5"] {
            assert!(inet_pton4(yes.as_bytes()), "{yes}");
        }
        for no in ["", "1.2.3", "1.2.3.4.5", "256.1.1.1", "1.2.3.04", "01.2.3.4", "1..3.4", "1.2.3.4 ", " 1.2.3.4", "a.b.c.d", "1.2.3.4/24", "::1", "1.2.3.-4"] {
            assert!(!inet_pton4(no.as_bytes()), "{no}");
        }
        assert!(inet_pton4(b"1.2.3.4\0junk"));
    }

    #[cfg(feature = "sim")]
    #[test]
    fn simulated_addresses_are_parsed_from_the_spec() {
        let mut out = [NetAddr::default(); 4];
        assert_eq!(sim_addrs(b"wlan0=192.168.0.20,usb0=192.168.15.244", &mut out, 4), 2);
        assert_eq!((out[0].ifname.as_str(), out[0].ip.as_str()), ("wlan0", "192.168.0.20"));
        assert_eq!((out[1].ifname.as_str(), out[1].ip.as_str()), ("usb0", "192.168.15.244"));
        assert_eq!(sim_addrs(b"a=1.1.1.1 b=2.2.2.2,, c=3.3.3.3", &mut out, 2), 2);
        assert_eq!(sim_addrs(b"a=1.1.1.1", &mut out, 0), 0);
        assert_eq!(sim_addrs(b"", &mut out, 4), 0);
        assert_eq!(sim_addrs(b"=1.1.1.1,x,y=,z=1.1.1,w=1.1.1.1", &mut out, 4), 1);
        assert_eq!(out[0].ifname.as_str(), "w");
    }
}
