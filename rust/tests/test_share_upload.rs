// test_share_upload: Send a PDF from the Share page (2.2, issue #175), at the level of its requests: the share server
// is run inside this process and spoken to over a socket on localhost, as a browser on the Wi-Fi would (and as one
// that means harm would). Nothing sent is kept before the Kindle's yes (httpd_upload_answer stands in for the
// Share screen's Import and Cancel here; share.rs's tests and the scenarios have the screen): each refusal and each
// way out leaves no temporary file, no question and nothing new in import/, and the log never holds a name or a token.
//
// One process, one thread: the client end of each exchange is a non-blocking socket written and read between
// httpd_poll calls.
//
// A sim-feature test: after a send ends, however it ends, the server takes no new one for a few seconds
// (UPLOAD_QUIET_MS), and the cases here send again at once, with that time made 0 (GRIMOIRE_UPLOAD_QUIET_MS, as the
// other time limits are shortened, which only the simulator's builds read); one case holds the quiet time itself.
#![cfg(feature = "sim")]
mod testlib;
use grimoire::prelude::*;
use grimoire::upload::*;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use testlib::*;

static ROOT: OnceLock<String> = OnceLock::new();
static PORT: OnceLock<i32> = OnceLock::new();
/// Every token a page carried in this run, and every send's code (the log and the files are checked for none of them
/// at the end).
static TOKENS: Mutex<Vec<String>> = Mutex::new(Vec::new());
static CODES: Mutex<Vec<String>> = Mutex::new(Vec::new());

fn port() -> i32 {
    *PORT.get().unwrap()
}
fn root() -> &'static str {
    ROOT.get().unwrap()
}
fn free_port() -> i32 {
    let l = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    l.local_addr().unwrap().port() as i32
}

/// A PDF's bytes, n of them: its header, filler and its end.
fn pdf(n: usize) -> Vec<u8> {
    let mut v = b"%PDF-1.4\n".to_vec();
    while v.len() + 7 < n {
        v.push(b'a' + (v.len() % 26) as u8);
    }
    v.extend_from_slice(b"\n%%EOF\n");
    v.truncate(n);
    v
}

fn connect() -> TcpStream {
    let s = TcpStream::connect(("127.0.0.1", port() as u16)).unwrap();
    s.set_nonblocking(true).unwrap();
    s
}

/// Writes all of b, polling the server while the socket is full. False when the server closed it first.
fn send(s: &mut TcpStream, b: &[u8]) -> bool {
    let mut at = 0;
    let until = Instant::now() + Duration::from_secs(60);
    while at < b.len() {
        match s.write(&b[at..]) {
            Ok(n) => at += n,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                httpd_poll();
                std::thread::sleep(Duration::from_millis(1));
            }
            Err(_) => return false,
        }
        assert!(Instant::now() < until, "the server never took the request");
    }
    httpd_poll();
    true
}

struct Reply {
    head: String,
    body: Vec<u8>,
}
impl Reply {
    fn status(&self) -> &str {
        self.head.lines().next().unwrap_or("")
    }
    fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }
    /// The send's code: the first line of the reply to a send the Kindle asked about, `code NNNN`.
    fn code(&self) -> Option<String> {
        let t = self.text();
        let c = t.strip_prefix("code ")?.split('\n').next()?.to_string();
        (c.len() == 4 && c.bytes().all(|d| d.is_ascii_digit()) && t.len() > 9).then_some(c)
    }
    /// What it says last: a refusal's one line, or the answer under a send's code.
    fn said(&self) -> String {
        self.text().trim_end().rsplit('\n').next().unwrap_or("").to_string()
    }
}
fn split(all: Vec<u8>) -> Reply {
    let at = all.windows(4).position(|w| w == b"\r\n\r\n").map_or(all.len(), |i| i + 4);
    Reply { head: String::from_utf8_lossy(&all[..at]).into_owned(), body: all[at..].to_vec() }
}

/// The server's answer on s, everything up to its close, polling the server between reads.
fn reply(s: TcpStream) -> Reply {
    reply_after(s, Vec::new())
}

/// The same, after `got` (what early() read of it).
fn reply_after(mut s: TcpStream, got: Vec<u8>) -> Reply {
    let mut out = got;
    let mut buf = vec![0u8; 1 << 16];
    let until = Instant::now() + Duration::from_secs(60);
    loop {
        httpd_poll();
        match s.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => out.extend_from_slice(&buf[..n]),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => std::thread::sleep(Duration::from_millis(1)),
            Err(_) => break,
        }
        assert!(Instant::now() < until, "the server never finished its answer");
    }
    split(out)
}

/// A send's answer as "<status> <what it says last>": "200 imported", "409 another PDF is being sent".
fn outcome(s: TcpStream) -> String {
    let r = reply(s);
    format!("{} {}", r.status().split(' ').nth(1).unwrap_or(""), r.said())
}

/// What has come on s while the Kindle asks, once the reply's head and the send's code are in (at most 10 s).
fn early(s: &mut TcpStream) -> Vec<u8> {
    let mut got = Vec::new();
    let mut b = [0u8; 4096];
    let until = Instant::now() + Duration::from_secs(10);
    while Instant::now() < until && split(got.clone()).code().is_none() {
        httpd_poll();
        match s.read(&mut b) {
            Ok(0) => break,
            Ok(n) => got.extend_from_slice(&b[..n]),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => std::thread::sleep(Duration::from_millis(1)),
            Err(_) => break,
        }
    }
    got
}

/// The answer to a request refused: this end says it has sent all it will (as a browser that read an early answer
/// stops), so that the server's reading and dropping of the rest ends at once.
fn refused_reply(s: TcpStream) -> Reply {
    let _ = s.shutdown(std::net::Shutdown::Write);
    reply(s)
}

/// Nothing comes on s for ms milliseconds (no answer, no close), the server polled all the while.
fn quiet(s: &mut TcpStream, ms: u64) -> bool {
    let until = Instant::now() + Duration::from_millis(ms);
    let mut b = [0u8; 1];
    while Instant::now() < until {
        httpd_poll();
        match s.peek(&mut b) {
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => std::thread::sleep(Duration::from_millis(2)),
            _ => return false,
        }
    }
    true
}

/// Polls the server until `done` holds (at most 10 s): whether it did.
fn poll_until(done: impl Fn() -> bool) -> bool {
    let until = Instant::now() + Duration::from_secs(10);
    while Instant::now() < until {
        httpd_poll();
        if done() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    false
}

fn get(path: &str) -> Reply {
    let mut s = connect();
    check!(send(&mut s, format!("GET {path} HTTP/1.0\r\nHost: 127.0.0.1:{}\r\n\r\n", port()).as_bytes()));
    reply(s)
}

/// The token the Share page carries, read from the page as its script reads it.
fn page_token() -> String {
    let t = get("/").text();
    const AT: &str = "window.GRIMOIRE_UPLOAD={\"token\":\"";
    let at = t.find(AT).map(|i| i + AT.len());
    let tok = at.map(|i| t[i..].chars().take_while(|c| c.is_ascii_hexdigit()).collect::<String>()).unwrap_or_default();
    TOKENS.lock().unwrap().push(tok.clone());
    tok
}

/// The head of an upload as the Share page's script sends it, with `extra` header lines.
fn head(token: Option<&str>, name: Option<&str>, len: usize, extra: &str) -> String {
    let mut h = format!("POST /upload HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nOrigin: http://127.0.0.1:{}\r\nContent-Type: application/pdf\r\nContent-Length: {len}\r\n", port(), port());
    if let Some(t) = token {
        h.push_str(&format!("X-Grimoire-Token: {t}\r\n"));
    }
    if let Some(n) = name {
        h.push_str(&format!("X-Grimoire-Name: {n}\r\n"));
    }
    h.push_str(extra);
    h.push_str("\r\n");
    h
}

/// A connection that sent `head` and `body`.
fn upload(head: &str, body: &[u8]) -> TcpStream {
    let mut s = connect();
    let mut all = head.as_bytes().to_vec();
    all.extend_from_slice(body);
    check!(send(&mut s, &all));
    s
}

/// The question the Kindle asks, once the body is in.
fn question() -> Option<(u32, String, u64, String)> {
    poll_until(|| httpd_upload_question().is_some());
    let q = httpd_upload_question();
    if let Some(q) = &q {
        CODES.lock().unwrap().push(q.3.clone());
    }
    q
}

fn part_there() -> bool {
    std::fs::symlink_metadata(part_path()).is_ok()
}
fn import_names() -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(format!("{}/import", root()))
        .map(|d| d.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect())
        .unwrap_or_default();
    v.sort();
    v
}
/// Nothing of an upload is left: no temporary file, no question, nothing under way, and import/ holds `names`.
fn nothing_kept(names: &[&str]) -> bool {
    poll_until(|| httpd_upload_state() == UploadState::None);
    !part_there() && httpd_upload_question().is_none() && httpd_upload_state() == UploadState::None && import_names() == names
}

// ---- the page ----

fn the_page_offers_to_send_with_a_token_of_its_own() {
    let r = get("/");
    check!(r.status() == "HTTP/1.0 200 OK");
    let t = r.text();
    check!(t.contains("</a></p><div id=upload></div><script>window.GRIMOIRE_UPLOAD={\"token\":\""));
    let tok = page_token();
    check!(tok.len() == 32 && tok.bytes().all(|c| c.is_ascii_hexdigit()));
    check!(t.contains(&format!("\",\"max\":{UPLOAD_MAX},\"words\":{{\"send\":\"Send a PDF to the Kindle\",\"sending\":\"Sending PDF: {{percent}}\"")));
    check!(t.contains("\"confirm\":\"Confirm on the Kindle.\",\"code\":\"Code %s\",\"sent\":\"Copied to grimoire/import.\",\"no\":\"Not imported.\""));
    check!(t.contains("\"reload\":\"Open Share on the Kindle, then reload this page.\",\"wait\":\"Wait 5 seconds, then send again.\""));
    check!(t.contains("\"big\":\"PDF too big: over 64 MB.\",\"full\":\"Couldn't import PDF. Is the storage full?\"}};</script><script>"));
    check!(t.contains("x.open('POST', '/upload');") && t.contains("x.setRequestHeader('X-Grimoire-Token', cfg.token);"));
    // the page's script without its comment lines, blank lines and indents: those stay in the file for its readers
    let script = t.split("}};</script><script>").nth(1).and_then(|x| x.split("</script>").next()).unwrap_or("");
    let file = include_str!("../../web/upload.js");
    check!(script.lines().count() > 40 && !script.lines().any(|l| l.is_empty() || l.starts_with("//") || l.starts_with(' ')));
    check!(script.len() + 1000 < file.len());
    check!(t.ends_with("</script></body></html>"));
    // the same token on every load while Share stays open; the notebook pages have no box
    check!(page_token() == tok);
    check!(!get("/api/list.json").text().contains(&tok));
    // the other routes answer as before
    check!(get("/upload").status() == "HTTP/1.0 404 Not Found");
    let mut s = connect();
    check!(send(&mut s, format!("OPTIONS /upload HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nOrigin: http://evil.example\r\nAccess-Control-Request-Method: POST\r\n\r\n", port()).as_bytes()));
    let pre = reply(s);
    check!(pre.status() == "HTTP/1.0 405 Method Not Allowed" && !pre.head.contains("Access-Control"));
    let mut s = connect();
    check!(send(&mut s, format!("PUT /upload HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nContent-Length: 0\r\n\r\n", port()).as_bytes()));
    check!(reply(s).status() == "HTTP/1.0 405 Method Not Allowed");
}

// ---- a PDF sent, asked about, kept or not ----

fn a_pdf_waits_for_the_kindle_and_import_keeps_it() {
    let tok = page_token();
    let body = pdf(300_000); // more than one read, and more than one poll's
    let mut s = upload(&head(Some(&tok), Some("2027%20Planner.pdf"), body.len(), ""), &body);
    let q = question();
    check!(q.as_ref().is_some_and(|(_, n, z, _)| n == "2027 Planner.pdf" && *z == body.len() as u64));
    check!(httpd_upload_state() == UploadState::Asking);
    // before the answer: the bytes in the temporary file only, nothing in import/; the browser has the reply's head
    // and the send's code, the Kindle's, and no answer yet
    check!(std::fs::read(part_path()).ok().as_deref() == Some(&body[..]));
    check!(import_names().is_empty());
    let code = q.as_ref().map_or(String::new(), |q| q.3.clone());
    let got = early(&mut s);
    let head_now = split(got.clone());
    check!(head_now.status() == "HTTP/1.0 200 OK" && head_now.text() == format!("code {code}\n"));
    check!(head_now.head.contains("Content-Type: text/plain; charset=utf-8\r\n") && !head_now.head.contains("Content-Length"));
    check!(quiet(&mut s, 300));
    // the code is in this send's reply alone: not in the page, the list, or another browser's answer
    let other = refused_reply(upload(&head(None, Some("a.pdf"), body.len(), ""), &body));
    check!(other.status() == "HTTP/1.0 401 Unauthorized" && httpd_upload_question().is_some_and(|q| q.3 == code));
    for t in [get("/").text(), get("/api/list.json").text(), other.text()] {
        check!(!t.contains(&format!("code {code}")) && !t.contains(&format!("Code {code}")));
    }
    let serial = q.map_or(0, |q| q.0);
    check!(httpd_upload_answer(serial, true) == Some(Ok("2027 Planner.pdf".to_string())));
    check!(std::fs::read(format!("{}/import/2027 Planner.pdf", root())).ok().as_deref() == Some(&body[..]));
    let r = reply_after(s, got);
    check!(r.status() == "HTTP/1.0 200 OK" && r.text() == format!("code {code}\nimported\n"));
    check!(nothing_kept(&["2027 Planner.pdf"]));
    // an answer to a question gone is none
    check!(httpd_upload_answer(serial, true).is_none() && httpd_upload_answer(serial, false).is_none());
    // the same name again never replaces the first: " 2"
    let other = pdf(5000);
    let s = upload(&head(Some(&tok), Some("2027%20Planner.pdf"), other.len(), ""), &other);
    let q = question().map_or(0, |q| q.0);
    check!(httpd_upload_answer(q, true) == Some(Ok("2027 Planner 2.pdf".to_string())));
    check!(outcome(s) == "200 imported");
    check!(std::fs::read(format!("{}/import/2027 Planner.pdf", root())).ok().as_deref() == Some(&body[..]));
    check!(std::fs::read(format!("{}/import/2027 Planner 2.pdf", root())).ok().as_deref() == Some(&other[..]));
    check!(nothing_kept(&["2027 Planner 2.pdf", "2027 Planner.pdf"]));
    // a body that ends with the head's own read (a small PDF comes in one piece)
    let tiny = pdf(64);
    let s = upload(&head(Some(&tok), Some("tiny.pdf"), tiny.len(), ""), &tiny);
    let q = question();
    check!(q.as_ref().is_some_and(|(_, n, z, _)| n == "tiny.pdf" && *z == 64));
    let code = q.as_ref().map_or(String::new(), |q| q.3.clone());
    check!(httpd_upload_answer(q.map_or(0, |q| q.0), false).is_none());
    let r = reply(s);
    check!(r.status() == "HTTP/1.0 200 OK" && r.text() == format!("code {code}\ndeclined on the Kindle\n"));
}

fn cancel_keeps_nothing() {
    let before: Vec<String> = import_names();
    let tok = page_token();
    let body = pdf(20_000);
    let s = upload(&head(Some(&tok), Some("Cancelled.pdf"), body.len(), ""), &body);
    let q = question();
    check!(part_there());
    check!(httpd_upload_answer(q.map_or(0, |q| q.0), false).is_none());
    check!(outcome(s) == "200 declined on the Kindle");
    let names: Vec<&str> = before.iter().map(|s| s.as_str()).collect();
    check!(nothing_kept(&names));
}

// ---- refusals ----

fn every_refusal_keeps_nothing() {
    let before: Vec<String> = import_names();
    let names: Vec<&str> = before.iter().map(|s| s.as_str()).collect();
    let tok = page_token();
    let body = pdf(2000);
    let n = body.len();
    // (another last digit than the page's own: a token that ends in 0 must not be "wrong" with a 0)
    let wrong = format!("{}{}", &tok[..31], if tok.ends_with('0') { '1' } else { '0' });
    check!(wrong.len() == 32 && wrong != tok);
    let refused = |h: String, b: &[u8], status: &str, text: &str| {
        let r = refused_reply(upload(&h, b));
        let ok = r.status() == status && r.text() == text;
        if !ok {
            eprintln!("    wanted {status} {text:?}, got {} {:?}", r.status(), r.text());
        }
        ok
    };
    // who may send: the page's token, from the page's own origin, by the Kindle's address
    check!(refused(head(None, Some("a.pdf"), n, ""), &body, "HTTP/1.0 401 Unauthorized", "token required\n"));
    check!(refused(head(Some(&wrong), Some("a.pdf"), n, ""), &body, "HTTP/1.0 401 Unauthorized", "wrong token\n"));
    check!(refused(head(Some(&tok[..31]), Some("a.pdf"), n, ""), &body, "HTTP/1.0 401 Unauthorized", "wrong token\n"));
    check!(refused(head(Some(""), Some("a.pdf"), n, ""), &body, "HTTP/1.0 401 Unauthorized", "wrong token\n"));
    let other_site = head(Some(&tok), Some("a.pdf"), n, "").replace(&format!("Origin: http://127.0.0.1:{}", port()), "Origin: http://evil.example");
    check!(refused(other_site, &body, "HTTP/1.0 403 Forbidden", "another site\n"));
    let null_site = head(Some(&tok), Some("a.pdf"), n, "").replace(&format!("Origin: http://127.0.0.1:{}", port()), "Origin: null");
    check!(refused(null_site, &body, "HTTP/1.0 403 Forbidden", "another site\n"));
    check!(refused(head(Some(&tok), Some("a.pdf"), n, "Sec-Fetch-Site: cross-site\r\n"), &body, "HTTP/1.0 403 Forbidden", "another site\n"));
    let rebound = head(Some(&tok), Some("a.pdf"), n, "").replace("Host: 127.0.0.1", "Host: evil.example.com");
    check!(refused(rebound, &body, "HTTP/1.0 400 Bad Request", "use the address shown on the Kindle\n"));
    // a header the rules read, there twice or too long to read: refused, never read as absent
    let page_origin = format!("Origin: http://127.0.0.1:{}\r\n", port());
    let long_origin = head(Some(&tok), Some("a.pdf"), n, "").replace(&page_origin, &format!("Origin: http://{}.example\r\n", "a".repeat(250)));
    check!(refused(long_origin, &body, "HTTP/1.0 403 Forbidden", "another site\n"));
    check!(refused(head(Some(&tok), Some("a.pdf"), n, "Origin: http://evil.example\r\n"), &body, "HTTP/1.0 403 Forbidden", "another site\n"));
    check!(refused(head(Some(&tok), Some("a.pdf"), n, &format!("Sec-Fetch-Site: cross-site{}\r\n", " ".repeat(40))), &body, "HTTP/1.0 403 Forbidden", "another site\n"));
    check!(refused(head(Some(&tok), Some("a.pdf"), n, "Sec-Fetch-Site: same-origin\r\nSec-Fetch-Site: cross-site\r\n"), &body, "HTTP/1.0 403 Forbidden", "another site\n"));
    check!(refused(head(Some(&tok), Some("a.pdf"), n, &format!("Host: 127.0.0.1:{}\r\n", port())), &body, "HTTP/1.0 400 Bad Request", "bad request\n"));
    check!(refused(head(Some(&tok), Some("a.pdf"), n, &format!("X-Grimoire-Token: {tok}\r\n")), &body, "HTTP/1.0 401 Unauthorized", "wrong token\n"));
    // a script elsewhere with no Origin at all (curl) and the page's own Sec-Fetch-Site are fine: the token decides
    let s = upload(&head(Some(&tok), Some("a.pdf"), n, "Sec-Fetch-Site: same-origin\r\n").replace(&format!("Origin: http://127.0.0.1:{}\r\n", port()), ""), &body);
    let q = question();
    check!(q.is_some());
    httpd_upload_answer(q.map_or(0, |q| q.0), false);
    check!(outcome(s) == "200 declined on the Kindle");
    // the body: a length, never chunked, a PDF's first bytes, no more than the import takes
    check!(refused(head(Some(&tok), Some("a.pdf"), n, "").replace(&format!("Content-Length: {n}\r\n"), ""), &body, "HTTP/1.0 411 Length Required", "length required\n"));
    check!(refused(head(Some(&tok), Some("a.pdf"), n, "").replace(&format!("Content-Length: {n}"), "Content-Length: 2kb"), &body, "HTTP/1.0 411 Length Required", "length required\n"));
    check!(refused(head(Some(&tok), Some("a.pdf"), n, "Transfer-Encoding: chunked\r\n"), &body, "HTTP/1.0 411 Length Required", "length required\n"));
    check!(refused(head(Some(&tok), Some("a.pdf"), n, &format!("Transfer-Encoding: chunked{}\r\n", " ".repeat(70))), &body, "HTTP/1.0 411 Length Required", "length required\n"));
    // one length only, and a body of exactly that length (here more of it came with the head)
    check!(refused(head(Some(&tok), Some("a.pdf"), n, "Content-Length: 10\r\n"), &body, "HTTP/1.0 400 Bad Request", "bad request\n"));
    check!(refused(head(Some(&tok), Some("a.pdf"), 10, &format!("Content-Length: {n}\r\n")), &body, "HTTP/1.0 400 Bad Request", "bad request\n"));
    check!(refused(head(Some(&tok), Some("a.pdf"), n, ""), &[&body[..], b"0123456789"].concat(), "HTTP/1.0 400 Bad Request", "more than its length\n"));
    check!(refused(head(Some(&tok), Some("a.pdf"), 4, ""), b"%PDF", "HTTP/1.0 415 Unsupported Media Type", "not a PDF\n"));
    check!(refused(head(Some(&tok), Some("a.pdf"), UPLOAD_MAX as usize + 1, ""), b"", "HTTP/1.0 413 Content Too Large", "too big\n"));
    let mut gif = b"GIF89a".to_vec();
    gif.resize(n, 0);
    check!(refused(head(Some(&tok), Some("a.pdf"), n, ""), &gif, "HTTP/1.0 415 Unsupported Media Type", "not a PDF\n"));
    check!(refused(head(Some(&tok), Some("a.pdf"), n, ""), b"%PDX-1.4 and the rest", "HTTP/1.0 415 Unsupported Media Type", "not a PDF\n"));
    // its first bytes split over two reads are checked as one
    let mut s = upload(&head(Some(&tok), Some("a.pdf"), n, ""), b"%P");
    check!(quiet(&mut s, 100) && httpd_upload_state() == UploadState::Receiving && !std::fs::read(part_path()).is_ok_and(|b| !b.is_empty()));
    check!(send(&mut s, b"Q"));
    let r = refused_reply(s);
    check!(r.status() == "HTTP/1.0 415 Unsupported Media Type");
    // and taken when they are a PDF's
    let mut s = upload(&head(Some(&tok), Some("split.pdf"), n, ""), &body[..2]);
    check!(quiet(&mut s, 50) && httpd_upload_state() == UploadState::Receiving);
    check!(send(&mut s, &body[2..]));
    let q = question();
    check!(q.as_ref().is_some_and(|q| q.1 == "split.pdf") && std::fs::read(part_path()).ok().as_deref() == Some(&body[..]));
    httpd_upload_answer(q.map_or(0, |q| q.0), false);
    check!(outcome(s) == "200 declined on the Kindle");
    // a head too big for the server
    let mut flood = head(Some(&tok), Some("a.pdf"), n, "").trim_end_matches("\r\n").to_string();
    flood.push_str("\r\nX-Pad: ");
    while flood.len() < 4095 {
        flood.push('a');
    }
    let r = reply(upload(&flood, b""));
    check!(r.status() == "HTTP/1.0 400 Bad Request" && r.text() == "request too large\n");
    check!(nothing_kept(&names));
}

// The refusal reaches a browser still sending its body (what follows is read and dropped, so that the close does not
// reset the connection under the answer).
fn a_refusal_reaches_a_browser_still_sending() {
    let body = pdf(1 << 20);
    let mut s = connect();
    check!(send(&mut s, head(Some("0123456789abcdef0123456789abcdef"), Some("a.pdf"), body.len(), "").as_bytes()));
    check!(send(&mut s, &body[..512 << 10]));
    // the rest still going while the answer is read
    let _ = send(&mut s, &body[512 << 10..]);
    let r = refused_reply(s);
    check!(r.status() == "HTTP/1.0 401 Unauthorized" && r.text() == "wrong token\n");
}

// ---- names ----

fn a_name_is_shown_and_never_a_path() {
    let tok = page_token();
    let body = pdf(3000);
    for (raw, shown) in [
        ("..%2F..%2Fsettings.ini", "_.._settings.ini.pdf"),
        ("a%2Fb.pdf", "a_b.pdf"),
        ("..%5C..%5Cx.pdf", "_.._x.pdf"),
        ("..", UPLOAD_FALLBACK),
        ("x%0Ay.pdf", "x_y.pdf"),
        ("x%00y.pdf", "x_y.pdf"),
        ("x%1By.pdf", "x_y.pdf"),
        (".hidden.pdf", "hidden.pdf"),
        ("notes.txt", "notes.txt.pdf"),
        ("Caf%C3%A9%20menu.pdf", "Caf\u{e9} menu.pdf"),
        ("2027%20Planner%E3%85%A4.pdf", "2027 Planner.pdf"),
        ("2027%E2%80%A8Planner", "2027 Planner.pdf"),
        ("%E2%80%8B.pdf", UPLOAD_FALLBACK),
    ] {
        let s = upload(&head(Some(&tok), Some(raw), body.len(), ""), &body);
        let q = question();
        check!(q.as_ref().is_some_and(|q| q.1 == shown));
        httpd_upload_answer(q.map_or(0, |q| q.0), false);
        check!(outcome(s) == "200 declined on the Kindle");
    }
    // a very long name: over what a header here holds (the fallback), over 255 bytes (cut short before .pdf); none
    let cut = format!("{}.pdf", "b".repeat(248));
    for (h, shown) in [
        (head(Some(&tok), Some(&"a".repeat(1100)), body.len(), ""), UPLOAD_FALLBACK),
        (head(Some(&tok), Some(&format!("{}.pdf", "b".repeat(300))), body.len(), ""), cut.as_str()),
        (head(Some(&tok), None, body.len(), ""), UPLOAD_FALLBACK),
    ] {
        let s = upload(&h, &body);
        let q = question();
        check!(q.as_ref().is_some_and(|q| q.1 == shown));
        httpd_upload_answer(q.map_or(0, |q| q.0), false);
        check!(outcome(s) == "200 declined on the Kindle");
    }
    // kept, a name that broke the rules is its cleaned self in import/, and nothing went anywhere else
    let before = import_names();
    let s = upload(&head(Some(&tok), Some("..%2F..%2Fsettings.ini"), body.len(), ""), &body);
    let q = question();
    check!(httpd_upload_answer(q.map_or(0, |q| q.0), true) == Some(Ok("_.._settings.ini.pdf".to_string())));
    check!(outcome(s) == "200 imported");
    let mut after = before.clone();
    after.push("_.._settings.ini.pdf".to_string());
    after.sort();
    check!(import_names() == after);
    check!(std::fs::read_to_string(format!("{}/settings.ini", root())).unwrap_or_default() == "# untouched\n");
    check!(std::fs::read_dir(root()).map(|d| d.count()).unwrap_or(0) == 5); // notebooks, trash, settings.ini, .cache, import
}

// ---- ways out ----

fn two_at_once_keep_neither() {
    let before: Vec<String> = import_names();
    let names: Vec<&str> = before.iter().map(|s| s.as_str()).collect();
    let tok = page_token();
    let body = pdf(10_000);
    // the second while the Kindle asks about the first
    let first = upload(&head(Some(&tok), Some("first.pdf"), body.len(), ""), &body);
    check!(question().is_some());
    let second = upload(&head(Some(&tok), Some("second.pdf"), body.len(), ""), &body);
    let r2 = refused_reply(second);
    check!(r2.status() == "HTTP/1.0 409 Conflict" && r2.text() == "another PDF is being sent\n");
    let r1 = reply(first);
    check!(r1.status() == "HTTP/1.0 200 OK" && r1.code().is_some() && r1.said() == "another PDF was sent");
    check!(nothing_kept(&names));
    // the second while the first's body still comes
    let mut first = upload(&head(Some(&tok), Some("first.pdf"), body.len(), ""), &body[..4000]);
    check!(quiet(&mut first, 50) && httpd_upload_state() == UploadState::Receiving && part_there());
    let second = upload(&head(Some(&tok), Some("second.pdf"), body.len(), ""), &body);
    check!(refused_reply(second).status() == "HTTP/1.0 409 Conflict");
    let _ = send(&mut first, &body[4000..]);
    check!(refused_reply(first).status() == "HTTP/1.0 409 Conflict");
    check!(nothing_kept(&names));
    // a refused second (no token) leaves the first as it was
    let first = upload(&head(Some(&tok), Some("first.pdf"), body.len(), ""), &body);
    let q = question();
    check!(refused_reply(upload(&head(None, Some("second.pdf"), body.len(), ""), &body)).status() == "HTTP/1.0 401 Unauthorized");
    check!(httpd_upload_question() == q);
    httpd_upload_answer(q.map_or(0, |q| q.0), false);
    check!(outcome(first) == "200 declined on the Kindle");
    check!(nothing_kept(&names));
}

fn a_body_cut_short_or_a_browser_gone_keeps_nothing() {
    let before: Vec<String> = import_names();
    let names: Vec<&str> = before.iter().map(|s| s.as_str()).collect();
    let tok = page_token();
    let body = pdf(50_000);
    // the browser closes before the end of the body
    let mut s = upload(&head(Some(&tok), Some("cut.pdf"), body.len(), ""), &body[..20_000]);
    check!(quiet(&mut s, 50) && part_there());
    drop(s);
    check!(nothing_kept(&names));
    // the browser closes while the Kindle asks
    let s = upload(&head(Some(&tok), Some("gone.pdf"), body.len(), ""), &body);
    check!(question().is_some());
    drop(s);
    check!(nothing_kept(&names));
}

// A connection sends exactly the length it said: a byte past it once the Kindle asks ends the send (its question goes,
// nothing is kept, the answer's last line says why), and so do more bytes and a close right after them.
fn a_body_longer_than_its_length_keeps_nothing() {
    let before: Vec<String> = import_names();
    let names: Vec<&str> = before.iter().map(|s| s.as_str()).collect();
    let tok = page_token();
    let body = pdf(4028);
    let mut s = upload(&head(Some(&tok), Some("more.pdf"), body.len(), ""), &body);
    check!(question().is_some());
    check!(send(&mut s, b"and more"));
    check!(poll_until(|| httpd_upload_question().is_none()));
    let r = refused_reply(s);
    check!(r.status() == "HTTP/1.0 200 OK" && r.code().is_some() && r.said() == "more than its length");
    check!(nothing_kept(&names));
    // 4028 bytes said, 200 KB sent, then a close
    let mut all = head(Some(&tok), Some("lie.pdf"), body.len(), "").into_bytes();
    all.extend_from_slice(&body);
    all.extend(std::iter::repeat_n(b'x', 200_000));
    let mut s = connect();
    let _ = send(&mut s, &all);
    drop(s);
    check!(nothing_kept(&names));
}

fn leaving_share_keeps_nothing_and_the_token_goes_with_it() {
    let before: Vec<String> = import_names();
    let names: Vec<&str> = before.iter().map(|s| s.as_str()).collect();
    let tok = page_token();
    let body = pdf(8000);
    // Live view keeps the server while Share goes: the question ends, its browser is told
    check!(httpd_acquire(HTTPD_LIVE, port(), 300.0, true) == 0);
    let s = upload(&head(Some(&tok), Some("left.pdf"), body.len(), ""), &body);
    check!(question().is_some());
    httpd_release(HTTPD_SHARE);
    check!(outcome(s) == "200 share is closed");
    check!(nothing_kept(&names));
    // with Share closed nothing is taken
    let r = refused_reply(upload(&head(Some(&tok), Some("closed.pdf"), body.len(), ""), &body));
    check!(r.status() == "HTTP/1.0 403 Forbidden" && r.text() == "share is closed\n");
    // a body under way when Share goes
    check!(httpd_acquire(HTTPD_SHARE, port(), 300.0, true) == 0);
    let tok2 = page_token();
    check!(tok2.len() == 32 && tok2 != tok);
    let mut s = upload(&head(Some(&tok2), Some("half.pdf"), body.len(), ""), &body[..3000]);
    check!(quiet(&mut s, 50) && part_there());
    httpd_release(HTTPD_SHARE);
    check!(nothing_kept(&names));
    drop(s);
    // Share again: the old page's token is refused, the new page's is taken
    check!(httpd_acquire(HTTPD_SHARE, port(), 300.0, true) == 0);
    httpd_release(HTTPD_LIVE);
    let r = refused_reply(upload(&head(Some(&tok2), Some("old.pdf"), body.len(), ""), &body));
    check!(r.status() == "HTTP/1.0 401 Unauthorized");
    let tok3 = page_token();
    let s = upload(&head(Some(&tok3), Some("new.pdf"), body.len(), ""), &body);
    let q = question();
    check!(q.as_ref().is_some_and(|q| q.1 == "new.pdf"));
    // the server stopping (Grimoire leaving) ends it too
    httpd_stop();
    check!(nothing_kept(&names));
    check!(outcome(s) == "200 share is closed");
    check!(httpd_acquire(HTTPD_SHARE, port(), 300.0, true) == 0);
    // sleep (the Share screen's) ends it the same way
    let s = upload(&head(Some(&page_token()), Some("asleep.pdf"), body.len(), ""), &body);
    check!(question().is_some());
    httpd_upload_discard(UploadEnd::Asleep);
    check!(nothing_kept(&names));
    check!(outcome(s) == "200 the Kindle is asleep");
}

fn a_leftover_of_a_crash_goes_at_the_start() {
    check!(std::fs::create_dir_all(format!("{}/.cache", root())).is_ok());
    check!(std::fs::write(part_path(), b"%PDF-left by a run that ended").is_ok());
    check!(upload_sweep());
    check!(!part_there());
    check!(!upload_sweep());
    // a link at the temporary file's name is the entry removed, never its target
    let target = format!("{}/../outside.pdf", root());
    check!(std::fs::write(&target, b"%PDF-mine").is_ok());
    check!(std::os::unix::fs::symlink(&target, part_path()).is_ok());
    let tok = page_token();
    let body = pdf(4000);
    let s = upload(&head(Some(&tok), Some("over-a-link.pdf"), body.len(), ""), &body);
    let q = question();
    check!(std::fs::symlink_metadata(part_path()).is_ok_and(|m| m.file_type().is_file()));
    check!(std::fs::read(&target).ok().as_deref() == Some(&b"%PDF-mine"[..]));
    httpd_upload_answer(q.map_or(0, |q| q.0), false);
    check!(outcome(s) == "200 declined on the Kindle");
    check!(std::fs::read(&target).ok().as_deref() == Some(&b"%PDF-mine"[..]));
    let _ = std::fs::remove_file(&target);
}

// The time limits, shortened for the test (GRIMOIRE_UPLOAD_*_MS, the simulator's builds only).
fn slow_bodies_and_unanswered_questions_end() {
    let before: Vec<String> = import_names();
    let names: Vec<&str> = before.iter().map(|s| s.as_str()).collect();
    let tok = page_token();
    let body = pdf(6000);
    std::env::set_var("GRIMOIRE_UPLOAD_STALL_MS", "300");
    let mut s = upload(&head(Some(&tok), Some("stalls.pdf"), body.len(), ""), &body[..100]);
    check!(quiet(&mut s, 100) && part_there());
    check!(poll_until(|| !part_there()));
    let r = refused_reply(s);
    check!(r.status() == "HTTP/1.0 408 Request Timeout" && r.text() == "too slow\n");
    check!(nothing_kept(&names));
    // bytes that keep coming, too slowly for the whole
    std::env::set_var("GRIMOIRE_UPLOAD_BODY_MS", "500");
    let mut s = upload(&head(Some(&tok), Some("drips.pdf"), body.len(), ""), &body[..10]);
    let mut at = 10;
    let until = Instant::now() + Duration::from_secs(5);
    let mut ended = false;
    while Instant::now() < until && at < body.len() {
        if !quiet(&mut s, 100) {
            ended = true;
            break;
        }
        let _ = send(&mut s, &body[at..at + 10]);
        at += 10;
    }
    check!(ended && at < body.len());
    let r = refused_reply(s);
    check!(r.status() == "HTTP/1.0 408 Request Timeout");
    check!(nothing_kept(&names));
    std::env::remove_var("GRIMOIRE_UPLOAD_STALL_MS");
    std::env::remove_var("GRIMOIRE_UPLOAD_BODY_MS");
    // the Kindle asks for a while, and no longer
    std::env::set_var("GRIMOIRE_UPLOAD_ASK_MS", "300");
    let mut s = upload(&head(Some(&tok), Some("unanswered.pdf"), body.len(), ""), &body);
    check!(question().is_some());
    let got = early(&mut s);
    check!(quiet(&mut s, 150));
    let r = reply_after(s, got);
    check!(r.status() == "HTTP/1.0 200 OK" && r.code().is_some() && r.said() == "no answer on the Kindle");
    check!(nothing_kept(&names));
    std::env::remove_var("GRIMOIRE_UPLOAD_ASK_MS");
}

// Room for it above the storage's floor (100 MB after it), as the copy job's (GRIMOIRE_UPLOAD_FREE_BYTES, the
// simulator's builds only).
fn no_room_is_refused() {
    let before: Vec<String> = import_names();
    let names: Vec<&str> = before.iter().map(|s| s.as_str()).collect();
    let tok = page_token();
    let body = pdf(6000);
    std::env::set_var("GRIMOIRE_UPLOAD_FREE_BYTES", ((100u64 << 20) + body.len() as u64 - 1).to_string());
    let r = refused_reply(upload(&head(Some(&tok), Some("full.pdf"), body.len(), ""), &body));
    check!(r.status() == "HTTP/1.0 507 Insufficient Storage" && r.text() == "no room\n");
    check!(nothing_kept(&names));
    std::env::set_var("GRIMOIRE_UPLOAD_FREE_BYTES", ((100u64 << 20) + body.len() as u64).to_string());
    let s = upload(&head(Some(&tok), Some("fits.pdf"), body.len(), ""), &body);
    let q = question();
    check!(q.is_some());
    httpd_upload_answer(q.map_or(0, |q| q.0), false);
    check!(outcome(s) == "200 declined on the Kindle");
    std::env::remove_var("GRIMOIRE_UPLOAD_FREE_BYTES");
}

/// Polls the server for ms milliseconds.
fn idle(ms: u64) {
    let until = Instant::now() + Duration::from_millis(ms);
    while Instant::now() < until {
        httpd_poll();
        std::thread::sleep(Duration::from_millis(2));
    }
}

// After a send ends, however it ends (Cancel, the browser gone, no answer, a second send, Import), the server takes no
// new one for UPLOAD_QUIET_MS (shortened here): it is refused with 503 and nothing is kept, and once that time is over
// the next is asked about as usual. Two sends at once still end both, and a third right behind them waits.
fn after_a_send_ends_the_next_waits() {
    let before: Vec<String> = import_names();
    let names: Vec<&str> = before.iter().map(|s| s.as_str()).collect();
    let tok = page_token();
    let body = pdf(3000);
    let n = body.len();
    std::env::set_var("GRIMOIRE_UPLOAD_QUIET_MS", "600");
    idle(700); // (the case before ended with a Cancel)
    let waits = || {
        let r = refused_reply(upload(&head(Some(&tok), Some("next.pdf"), n, ""), &body));
        let ok = r.status() == "HTTP/1.0 503 Service Unavailable" && r.text() == "wait a few seconds\n";
        if !ok {
            eprintln!("    wanted 503, got {} {:?}", r.status(), r.text());
        }
        ok && httpd_upload_question().is_none() && !part_there()
    };
    let asked = || {
        let s = upload(&head(Some(&tok), Some("next.pdf"), n, ""), &body);
        (question(), s)
    };
    // Cancel: the next waits, then is asked about
    let (q, s) = asked();
    check!(httpd_upload_answer(q.map_or(0, |q| q.0), false).is_none());
    check!(outcome(s) == "200 declined on the Kindle");
    check!(waits());
    check!(waits());
    idle(700);
    let (q, s) = asked();
    check!(q.is_some());
    // the browser gone while the Kindle asks: the same
    drop(s);
    check!(poll_until(|| httpd_upload_question().is_none()));
    check!(waits());
    idle(700);
    // no answer in time: the same
    std::env::set_var("GRIMOIRE_UPLOAD_ASK_MS", "100");
    let (q, s) = asked();
    check!(q.is_some());
    check!(outcome(s) == "200 no answer on the Kindle");
    std::env::remove_var("GRIMOIRE_UPLOAD_ASK_MS");
    check!(waits());
    idle(700);
    // two sends at once end both, and a third right behind them waits: no question at all
    let (q, mine) = asked();
    check!(q.is_some());
    let mut other = connect();
    let mut third = connect();
    let mut all = head(Some(&tok), Some("next.pdf"), n, "").into_bytes();
    all.extend_from_slice(&body);
    // (both written before the server is polled again)
    check!(other.write_all(&all).is_ok() && third.write_all(&all).is_ok());
    check!(refused_reply(other).status() == "HTTP/1.0 409 Conflict");
    check!(outcome(mine) == "200 another PDF was sent");
    let r = refused_reply(third);
    check!(r.status() == "HTTP/1.0 503 Service Unavailable" && r.text() == "wait a few seconds\n");
    check!(nothing_kept(&names));
    idle(700);
    // Import: the same, so no other question comes over its toast
    let (q, s) = asked();
    let kept = httpd_upload_answer(q.map_or(0, |q| q.0), true).and_then(|r| r.ok()).unwrap_or_default();
    check!(kept == "next.pdf");
    check!(outcome(s) == "200 imported");
    check!(waits());
    let mut names = names.clone();
    names.push(&kept);
    names.sort();
    check!(nothing_kept(&names));
    idle(700);
    let (q, s) = asked();
    check!(q.is_some());
    httpd_upload_answer(q.map_or(0, |q| q.0), false);
    check!(outcome(s) == "200 declined on the Kindle");
    std::env::set_var("GRIMOIRE_UPLOAD_QUIET_MS", "0");
}

// stderr into a file for the whole run: the log says what came and how it ended, never a name or a token.
static SAVED: AtomicI32 = AtomicI32::new(-1);
fn log_capture(path: &str) {
    let path = std::ffi::CString::new(path).unwrap();
    // SAFETY: dup, open and dup2 on descriptor 2 and one this function opens, which it closes again.
    unsafe {
        SAVED.store(libc::dup(2), Ordering::Relaxed);
        let fd = libc::open(path.as_ptr(), libc::O_WRONLY | libc::O_CREAT | libc::O_TRUNC, 0o644 as libc::c_uint);
        if fd >= 0 {
            libc::dup2(fd, 2);
            libc::close(fd);
        }
    }
}
fn log_captured(path: &str) -> String {
    let saved = SAVED.swap(-1, Ordering::Relaxed);
    if saved >= 0 {
        // SAFETY: puts descriptor 2 back from the copy log_capture made, and closes the copy.
        unsafe {
            libc::dup2(saved, 2);
            libc::close(saved);
        }
    }
    let t = std::fs::read_to_string(path).unwrap_or_default();
    eprint!("{t}");
    t
}

#[test]
fn test_share_upload() {
    let root = format!("{}/upload-{}/grimoire", tl_tmp(), std::process::id());
    let _ = ROOT.set(root.clone());
    let _ = std::fs::remove_dir_all(format!("{}/upload-{}", tl_tmp(), std::process::id()));
    check!(store_init(&root) == 0);
    check!(std::fs::write(format!("{root}/settings.ini"), "# untouched\n").is_ok());
    store_set_device(300.0, 1860, 2480);
    ui_init(300.0);
    check!(font_init() == 0);
    i18n_apply("en");
    let _ = PORT.set(free_port());
    let log = format!("{}/upload-{}/captured.log", tl_tmp(), std::process::id());
    log_capture(&log);
    check!(httpd_acquire(HTTPD_SHARE, port(), 300.0, true) == 0);
    run!(the_page_offers_to_send_with_a_token_of_its_own);
    // (the cases below send again at once after a send ends: no quiet time for them)
    std::env::set_var("GRIMOIRE_UPLOAD_QUIET_MS", "0");
    run!(a_pdf_waits_for_the_kindle_and_import_keeps_it);
    run!(cancel_keeps_nothing);
    run!(every_refusal_keeps_nothing);
    run!(a_refusal_reaches_a_browser_still_sending);
    run!(a_name_is_shown_and_never_a_path);
    run!(two_at_once_keep_neither);
    run!(a_body_cut_short_or_a_browser_gone_keeps_nothing);
    run!(a_body_longer_than_its_length_keeps_nothing);
    run!(leaving_share_keeps_nothing_and_the_token_goes_with_it);
    run!(a_leftover_of_a_crash_goes_at_the_start);
    run!(slow_bodies_and_unanswered_questions_end);
    run!(no_room_is_refused);
    run!(after_a_send_ends_the_next_waits);
    httpd_stop();
    let t = log_captured(&log);
    // what the log said: each way in and out, and none of the names or tokens sent
    for line in ["httpd: an upload of 292 KB is coming in", "httpd: an upload of 292 KB is in; the Kindle asks",
        "httpd: an upload was kept: it waits in import/ for the import", "httpd: an upload was not kept (Cancel on the Kindle)",
        "httpd: an upload was refused (no token)", "httpd: an upload was refused (a wrong token)", "httpd: an upload was refused (another site)",
        "httpd: an upload was refused (not a PDF)", "httpd: an upload was refused (too big)", "httpd: an upload was not kept (a second upload came)",
        "httpd: an upload was cut short; nothing kept", "httpd: the browser that sent an upload left; nothing kept",
        "httpd: an upload was not kept (Share closed)", "httpd: an upload was refused (Share is closed)",
        "httpd: an upload was refused (too soon after the last one ended)", "httpd: an upload was refused (more bytes than its length)",
        "httpd: an upload was not kept (more bytes than its length came)", "httpd: an upload was refused (a Content-Length twice or too long)",
        "httpd: an upload was refused (an Origin or Sec-Fetch-Site header twice or too long)"] {
        check!(t.contains(line));
    }
    for name in ["Planner", "Cancelled", "settings.ini", "first.pdf", "Caf", "Untitled", "over-a-link", "hidden"] {
        check!(!t.contains(name));
    }
    for tok in TOKENS.lock().unwrap().iter().filter(|t| !t.is_empty()) {
        check!(!t.contains(tok.as_str()));
    }
    // no send's code in what httpd logged (its lines past their time of day), nor in any file Grimoire wrote
    let codes = CODES.lock().unwrap().clone();
    check!(codes.len() > 20);
    let whole = |hay: &[u8], code: &str| hay.windows(6).any(|w| !w[0].is_ascii_digit() && &w[1..5] == code.as_bytes() && !w[5].is_ascii_digit());
    let httpd: Vec<&str> = t.lines().filter_map(|l| l.split_once(" httpd: ").map(|x| x.1)).collect();
    check!(!httpd.is_empty());
    let mut files = Vec::new();
    let mut dirs = vec![root.clone()];
    while let Some(d) = dirs.pop() {
        for e in std::fs::read_dir(&d).into_iter().flatten().flatten() {
            let p = e.path();
            if p.is_dir() { dirs.push(p.to_string_lossy().into_owned()); } else if let Ok(b) = std::fs::read(&p) { files.push(b); }
        }
    }
    for code in &codes {
        check!(!httpd.iter().any(|l| whole(format!(" {l} ").as_bytes(), code)));
        check!(!files.iter().any(|b| whole(&[b" ", &b[..], b" "].concat(), code)));
    }
    let _ = std::fs::remove_dir_all(format!("{}/upload-{}", tl_tmp(), std::process::id()));
    done();
}
