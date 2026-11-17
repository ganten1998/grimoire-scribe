//! Send a PDF from the Share page (2.2, issue #175): what the share server's one upload route stands on, apart from
//! its connections (httpd.rs has those): the rules a request is held to, the name the Kindle shows for it, and the one
//! temporary file its bytes wait in until the Kindle's answer.
//!
//! The request is `POST /upload` with the PDF's bytes as its body (a Content-Length, never chunked), the file's name
//! percent-encoded in `X-Grimoire-Name` and the Share page's token in `X-Grimoire-Token`. Before the user's yes nothing
//! of the body is read but its first five bytes (`%PDF-`), and nothing is written but the temporary file
//! (`grimoire/.cache/share-upload.part`, made afresh with O_CREAT|O_EXCL|O_NOFOLLOW in a folder reached without
//! following a link). A `Part` dropped without `commit` removes that file, so every way out leaves nothing; one a
//! crash left is removed at the next start (`upload_sweep`). Import renames it into `grimoire/import/` under the
//! cleaned name with fsview's rename that never replaces (" 2", " 3"... where the name is taken), where the import
//! takes it as any PDF copied in. The name the client sends is never a path: it is shown, and names that file.

use crate::fsview::{fsview_name_check, numbered, rename_noreplace, sync_dir, FS_NAME_MAX};
use crate::prelude::*;
use std::ffi::CString;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};

/// The biggest PDF the Share page may send: the import's limit for one PDF on every Kindle (the one it reads whole
/// where the private mapping is refused; a Kindle that maps PDFs takes bigger ones copied in over USB).
pub const UPLOAD_MAX: u64 = crate::sandbox::s1::PDF_MAX_READ_WHOLE;
/// A body that sends nothing for this long is refused, and so is one not whole after `UPLOAD_BODY_MS`.
pub const UPLOAD_STALL_MS: u32 = 30_000;
pub const UPLOAD_BODY_MS: u32 = 600_000;
/// The Kindle asks this long; then the bytes go, as after Cancel.
pub const UPLOAD_ASK_MS: u32 = 120_000;
/// After a refusal sent while the browser may still be sending, what it sends is read and dropped this long, so that
/// it reads the refusal (a close with bytes unread would reset the connection under it).
pub const UPLOAD_DRAIN_MS: u32 = 5_000;
/// After a send ends any way but Import (Cancel, no answer, the browser gone, a second send, a body refused), no
/// new send is taken for this long: longer than the toast "Not imported." (3 s), so that a question that went is
/// seen to go before another can come, and short enough to send again at once after a Cancel by mistake.
pub const UPLOAD_QUIET_MS: u32 = 5_000;
/// The name of a PDF whose own name is not one a file in Grimoire's folders may have.
pub const UPLOAD_FALLBACK: &str = "Untitled.pdf";
/// The temporary file's name, in `grimoire/.cache` (a folder Files never lists or enters).
pub const PART_NAME: &str = "share-upload.part";
/// The first bytes of every PDF the import takes, checked before the Kindle asks.
pub const PDF_MAGIC: &[u8; 5] = b"%PDF-";

/// The name the Kindle shows and the file in import/ takes: the client's, percent-decoded, when it is text that
/// fsview's rules let a file of Grimoire's folders be named (not empty, at most 255 bytes, none of / \ : * ? " < > |,
/// no control character, no space at either end, no dot at either end), with no character that turns or hides text,
/// ending in `.pdf` in any case; any other name, or none, is `UPLOAD_FALLBACK`. Never a path.
pub fn upload_name(raw: Option<&[u8]>) -> String {
    raw.and_then(percent_decoded).filter(|n| name_ok(n)).unwrap_or_else(|| UPLOAD_FALLBACK.to_string())
}

/// `%XX` (two hex digits) as its byte, every other byte as it is; None for a `%` without two hex digits after it or
/// for what is not UTF-8 once decoded.
fn percent_decoded(raw: &[u8]) -> Option<String> {
    let hex = |c: Option<&u8>| c.and_then(|&c| (c as char).to_digit(16));
    let mut o = Vec::with_capacity(raw.len());
    let mut i = 0;
    while i < raw.len() {
        if raw[i] == b'%' {
            let (Some(a), Some(b)) = (hex(raw.get(i + 1)), hex(raw.get(i + 2))) else { return None };
            o.push((a * 16 + b) as u8);
            i += 3;
        } else {
            o.push(raw[i]);
            i += 1;
        }
    }
    String::from_utf8(o).ok()
}

fn name_ok(n: &str) -> bool {
    let b = n.as_bytes();
    fsview_name_check(b, b, false).is_ok()
        && !n.chars().any(|c| c.is_control() || turns_or_hides(c))
        && b.len() > 4
        && b[b.len() - 4..].eq_ignore_ascii_case(b".pdf")
}

/// Characters that change which way text runs or show nothing (a name that read one way on the Kindle and named
/// another file would mislead the question).
fn turns_or_hides(c: char) -> bool {
    matches!(c, '\u{200B}'..='\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2060}'..='\u{206F}' | '\u{FEFF}')
}

/// Whether the token a request carries is the page's: the same length and bytes, compared in a time that does not
/// depend on where they differ. An empty token is never one.
pub fn same_token(given: &[u8], page: &[u8]) -> bool {
    if given.len() != page.len() || page.is_empty() {
        return false;
    }
    std::hint::black_box(given.iter().zip(page).fold(0u8, |acc, (x, y)| acc | (x ^ y))) == 0
}

/// A new token for the Share page: 32 hex digits of the system's randomness, None when it has none (then no page
/// offers to send, and every upload is refused).
pub fn new_token() -> Option<String> {
    let mut b = [0u8; 16];
    crate::util::random_fill(&mut b).then(|| b.iter().map(|x| format!("{x:02x}")).collect())
}

/// A new send code: four decimal digits of the system's randomness, each of the 10,000 as likely, None when it has
/// none. Not a password: it ties the question on the Kindle to the page that sent the PDF. The server draws it, so no
/// sender chooses it; it is given only in that send's own reply and shown only under that send's question.
pub fn new_code() -> Option<String> {
    loop {
        let mut b = [0u8; 2];
        if !crate::util::random_fill(&mut b) {
            return None;
        }
        // (60,000 is 6 times 10,000: a value above it is drawn again, so that no code is likelier than another)
        let v = u16::from_le_bytes(b);
        if v < 60_000 {
            return Some(format!("{:04}", v % 10_000));
        }
    }
}

/// Whether a request comes from the Share page as far as a browser says: an Origin header, when there is one, is
/// `http://` and the Host header's very text (cases aside), and Sec-Fetch-Site, when there is one, is `same-origin`.
/// A browser sends both with the page's script's POST; a page of another site cannot make either say so.
pub fn same_origin(origin: Option<&[u8]>, host: Option<&[u8]>, site: Option<&[u8]>) -> bool {
    if site.is_some_and(|s| s.trim_ascii() != b"same-origin") {
        return false;
    }
    let Some(o) = origin.map(|o| o.trim_ascii()) else { return true };
    let Some(h) = host.map(|h| h.trim_ascii()).filter(|h| !h.is_empty()) else { return false };
    o.len() == 7 + h.len() && o[..7].eq_ignore_ascii_case(b"http://") && o[7..].eq_ignore_ascii_case(h)
}

/// A Content-Length value: decimal digits only (at most 12), None for anything else.
pub fn body_length(v: Option<&[u8]>) -> Option<u64> {
    let v = v?.trim_ascii();
    if v.is_empty() || v.len() > 12 || !v.iter().all(u8::is_ascii_digit) {
        return None;
    }
    std::str::from_utf8(v).ok()?.parse().ok()
}

/// A time limit of the upload's (one of the constants above), which the simulator's runs and the tests may shorten
/// with `GRIMOIRE_UPLOAD_<NAME>_MS` (STALL, BODY, ASK, DRAIN, QUIET); never the Kindle's build.
pub fn limit_ms(name: &str, ms: u32) -> u32 {
    #[cfg(feature = "sim")]
    if let Some(v) = std::env::var(format!("GRIMOIRE_UPLOAD_{name}_MS")).ok().and_then(|v| v.parse::<u32>().ok()) {
        return v;
    }
    let _ = name;
    ms
}

/// The bytes free on the storage of Grimoire's folder (the simulator's runs and the tests may say instead, with
/// `GRIMOIRE_UPLOAD_FREE_BYTES`; never the Kindle's build).
pub fn free_bytes() -> Option<u64> {
    #[cfg(feature = "sim")]
    if let Some(v) = std::env::var("GRIMOIRE_UPLOAD_FREE_BYTES").ok().and_then(|v| v.parse::<u64>().ok()) {
        return Some(v);
    }
    crate::pdfimport::storage_space().map(|(_, free)| free)
}

fn errno() -> i32 {
    std::io::Error::last_os_error().raw_os_error().unwrap_or(libc::EIO)
}

/// Grimoire's own folder, open (the root everything below is reached from without following a link).
fn root_dir() -> Result<OwnedFd, i32> {
    let c = CString::new(store_root()).map_err(|_| libc::EINVAL)?;
    // SAFETY: open reads the NUL-terminated path; the descriptor is owned by the OwnedFd.
    let fd = unsafe { libc::open(c.as_ptr(), libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC) };
    if fd < 0 {
        return Err(errno());
    }
    // SAFETY: fd was just opened and is owned by no one else.
    Ok(unsafe { OwnedFd::from_raw_fd(fd) })
}

/// `name` in the folder open at `dirfd`, as a folder and never through a link (O_DIRECTORY|O_NOFOLLOW); made first
/// where `make` says and it is missing.
fn sub_dir(dirfd: i32, name: &str, make: bool) -> Result<OwnedFd, i32> {
    let c = CString::new(name).map_err(|_| libc::EINVAL)?;
    for _ in 0..2 {
        // SAFETY: openat on our descriptor and a NUL-terminated name; the descriptor is owned by the OwnedFd.
        let fd = unsafe { libc::openat(dirfd, c.as_ptr(), libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC) };
        if fd >= 0 {
            // SAFETY: fd was just opened and is owned by no one else.
            return Ok(unsafe { OwnedFd::from_raw_fd(fd) });
        }
        let e = errno();
        if e != libc::ENOENT || !make {
            return Err(e);
        }
        // SAFETY: mkdirat on our descriptor and a NUL-terminated name (a folder made by another meanwhile is fine).
        unsafe { libc::mkdirat(dirfd, c.as_ptr(), 0o755) };
    }
    Err(libc::ENOENT)
}

fn part_cname() -> CString {
    CString::new(PART_NAME).unwrap_or_default()
}

/// The temporary file an upload's bytes wait in until the Kindle's answer. Dropped without `commit`, it is removed.
pub struct Part {
    dir: OwnedFd,
    file: Option<std::fs::File>,
    kept: bool,
}

impl Part {
    /// Made afresh in `grimoire/.cache` (made where missing, never through a link): what a dead run left at the name
    /// goes first (the entry itself, never what it may point at), then O_CREAT|O_EXCL|O_NOFOLLOW, mode 0600.
    pub fn create() -> Result<Part, i32> {
        let root = root_dir()?;
        let dir = sub_dir(root.as_raw_fd(), ".cache", true)?;
        let name = part_cname();
        // SAFETY: unlinkat on our descriptor and a NUL-terminated name (a folder there fails, and so does the open).
        unsafe { libc::unlinkat(dir.as_raw_fd(), name.as_ptr(), 0) };
        // SAFETY: openat on our descriptor and a NUL-terminated name; the descriptor is owned by the File.
        let fd = unsafe {
            libc::openat(dir.as_raw_fd(), name.as_ptr(), libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_NOFOLLOW | libc::O_CLOEXEC, 0o600 as libc::c_uint)
        };
        if fd < 0 {
            return Err(errno());
        }
        // SAFETY: fd was just opened and is owned by no one else.
        let file = unsafe { std::fs::File::from_raw_fd(fd) };
        Ok(Part { dir, file: Some(file), kept: false })
    }

    /// Writes `b` at the file's end: Err(errno) when the storage refuses (ENOSPC when it is full).
    pub fn write(&mut self, b: &[u8]) -> Result<(), i32> {
        use std::io::Write;
        let f = self.file.as_mut().ok_or(libc::EBADF)?;
        f.write_all(b).map_err(|e| e.raw_os_error().unwrap_or(libc::EIO))
    }

    /// The bytes are all in: the file is closed (the question then waits with nothing open).
    pub fn close(&mut self) {
        self.file = None;
    }

    /// The user's yes: the file renamed into `grimoire/import/` (made where missing, reached without following a
    /// link) as `name`, or " 2", " 3"... before its extension where taken, never over a file. The name it took, or
    /// Err(errno); on Err the file goes as on any other way out.
    pub fn commit(mut self, name: &str) -> Result<String, i32> {
        self.file = None;
        let root = root_dir()?;
        let import = sub_dir(root.as_raw_fd(), "import", true)?;
        let part = part_cname();
        for k in 1..=99u32 {
            let want = if k == 1 { name.as_bytes().to_vec() } else { numbered(name.as_bytes(), k, false) };
            if want.len() > FS_NAME_MAX {
                return Err(libc::ENAMETOOLONG);
            }
            let c = CString::new(want.clone()).map_err(|_| libc::EINVAL)?;
            match rename_noreplace(self.dir.as_raw_fd(), &part, import.as_raw_fd(), &c) {
                Ok(()) => {
                    self.kept = true;
                    sync_dir(import.as_raw_fd());
                    return Ok(String::from_utf8_lossy(&want).into_owned());
                }
                Err(libc::EEXIST) => continue,
                Err(e) => return Err(e),
            }
        }
        Err(libc::EEXIST)
    }
}

impl Drop for Part {
    fn drop(&mut self) {
        self.file = None;
        if !self.kept {
            let name = part_cname();
            // SAFETY: unlinkat on our descriptor and a NUL-terminated name: the temporary file, whatever it holds.
            unsafe { libc::unlinkat(self.dir.as_raw_fd(), name.as_ptr(), 0) };
        }
    }
}

/// At the start: a temporary file a crash left (a run that ended while the Kindle asked) goes. True when one was there.
pub fn upload_sweep() -> bool {
    let Ok(root) = root_dir() else { return false };
    let Ok(dir) = sub_dir(root.as_raw_fd(), ".cache", false) else { return false };
    let name = part_cname();
    // SAFETY: unlinkat on our descriptor and a NUL-terminated name.
    unsafe { libc::unlinkat(dir.as_raw_fd(), name.as_ptr(), 0) == 0 }
}

/// Where the temporary file is (for the tests and the runner's checks).
pub fn part_path() -> String {
    format!("{}/.cache/{}", store_root(), PART_NAME)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_is_shown_only_when_a_file_may_have_it() {
        let n = |s: &str| upload_name(Some(s.as_bytes()));
        assert_eq!(n("2027%20Planner.pdf"), "2027 Planner.pdf");
        assert_eq!(n("Caf%C3%A9%20%C3%BCber.PDF"), "Caf\u{e9} \u{fc}ber.PDF");
        assert_eq!(n("plain.pdf"), "plain.pdf");
        // a path, a control byte, a name hidden or with an edge of space or dot, another kind of file: the fallback
        for bad in [
            "a%2Fb.pdf", "..%2F..%2Fetc%2Fpasswd.pdf", "a%5Cb.pdf", "..", "..pdf", ".pdf", "%2E.pdf", "x%00.pdf", "x%0A.pdf",
            "x%09y.pdf", "x%7F.pdf", "%C2%85x.pdf", "%20x.pdf", "x.pdf%20", "x.pdf.", "notes", "notes.txt", "notes.pdfx",
            "a%3Ab.pdf", "a*b.pdf", "a%22b.pdf", "a%3Cb.pdf", "a%7Cb.pdf", "a%3Fb.pdf", "x%E2%80%AEfdp.exe.pdf",
            "x%E2%80%8B.pdf", "%EF%BB%BFx.pdf", "", "%", "%4", "%zz.pdf", "%FF.pdf", "%C3.pdf",
        ] {
            assert_eq!(n(bad), UPLOAD_FALLBACK, "{bad}");
        }
        // raw bytes that are not UTF-8, and none at all
        assert_eq!(upload_name(Some(b"\xff\xfe.pdf")), UPLOAD_FALLBACK);
        assert_eq!(upload_name(None), UPLOAD_FALLBACK);
        // 255 bytes at most
        let long = format!("{}.pdf", "a".repeat(251));
        assert_eq!(n(&long), long);
        assert_eq!(n(&format!("{}.pdf", "a".repeat(252))), UPLOAD_FALLBACK);
        // the fallback keeps the rules itself
        assert!(name_ok(UPLOAD_FALLBACK));
    }

    #[test]
    fn tokens_are_compared_whole() {
        let t = b"0123456789abcdef0123456789abcdef";
        assert!(same_token(t, t));
        assert!(!same_token(b"0123456789abcdef0123456789abcdee", t));
        assert!(!same_token(b"0123456789abcdef0123456789abcde", t));
        assert!(!same_token(b"", b""));
        assert!(!same_token(t, b""));
        let a = new_token().unwrap();
        assert_eq!(a.len(), 32);
        assert!(a.bytes().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(a, new_token().unwrap());
    }

    #[test]
    fn a_code_is_four_digits_of_chance() {
        let codes: Vec<String> = (0..2000).map(|_| new_code().unwrap()).collect();
        assert!(codes.iter().all(|c| c.len() == 4 && c.bytes().all(|d| d.is_ascii_digit())));
        // (2000 draws of 10,000 values: about 1820 different ones; fewer than 1500 would mean they are not drawn evenly)
        let mut seen = codes.clone();
        seen.sort();
        seen.dedup();
        assert!(seen.len() > 1500, "{}", seen.len());
        assert!(codes.iter().any(|c| c.starts_with('0')));
    }

    #[test]
    fn only_the_page_itself_is_its_origin() {
        let h = Some(&b"192.168.0.20:8765"[..]);
        assert!(same_origin(None, h, None));
        assert!(same_origin(Some(b"http://192.168.0.20:8765"), h, Some(b"same-origin")));
        assert!(same_origin(Some(b"HTTP://192.168.0.20:8765"), h, None));
        assert!(!same_origin(Some(b"http://evil.example"), h, None));
        assert!(!same_origin(Some(b"https://192.168.0.20:8765"), h, None));
        assert!(!same_origin(Some(b"http://192.168.0.20:876"), h, None));
        assert!(!same_origin(Some(b"http://192.168.0.20:8765.evil"), h, None));
        assert!(!same_origin(Some(b"null"), h, None));
        assert!(!same_origin(Some(b"http://192.168.0.20:8765"), None, None));
        assert!(!same_origin(None, h, Some(b"cross-site")));
        assert!(!same_origin(None, h, Some(b"same-site")));
        assert!(!same_origin(None, h, Some(b"none")));
    }

    #[test]
    fn a_length_is_digits() {
        assert_eq!(body_length(Some(b"4028")), Some(4028));
        assert_eq!(body_length(Some(b" 12 ")), Some(12));
        assert_eq!(body_length(Some(b"0")), Some(0));
        for bad in [&b""[..], b"-1", b"+5", b"1e3", b"0x10", b"12 34", b"1234567890123", b"\xff"] {
            assert_eq!(body_length(Some(bad)), None, "{:?}", bad);
        }
        assert_eq!(body_length(None), None);
    }
}
