// test_httpd: the share server's routes, served and read back inside one process. (The C tree has
// no unit test for httpd.c or share.c: tests/share_check.sh and tests/live_check.sh check them
// through the headless runner, and keep doing so. These are the same checks a level lower, against
// the library alone, where the host build can run them without the runner.)
//
// One process, one thread: the client end of each exchange is a non-blocking socket read between
// two httpd_poll calls, so no second thread is alive when a render child is forked.
#![allow(clippy::needless_range_loop)]
mod testlib;
use grimoire::prelude::*;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::OnceLock;
use std::time::{Duration, Instant};
use testlib::*;

static ROOT: OnceLock<String> = OnceLock::new();
static PORT: OnceLock<i32> = OnceLock::new();

fn port() -> i32 {
    *PORT.get().unwrap()
}

fn free_port() -> i32 {
    let l = TcpListener::bind(("127.0.0.1", 0)).unwrap();
    l.local_addr().unwrap().port() as i32
}

/// The server's answer to raw (everything up to the close), polling the server between reads.
fn talk(raw: &[u8]) -> Vec<u8> {
    let mut s = TcpStream::connect(("127.0.0.1", port() as u16)).unwrap();
    s.set_nonblocking(true).unwrap();
    s.write_all(raw).unwrap();
    let mut out = Vec::new();
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
    out
}

struct Reply {
    head: String,
    body: Vec<u8>,
}
impl Reply {
    fn status(&self) -> &str {
        self.head.lines().next().unwrap_or("")
    }
    fn header(&self, name: &str) -> Option<&str> {
        self.head.lines().skip(1).find_map(|l| {
            let (k, v) = l.split_once(": ")?;
            k.eq_ignore_ascii_case(name).then_some(v)
        })
    }
    fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }
}

fn request(method: &str, path: &str, extra: &str) -> Reply {
    let raw = format!("{method} {path} HTTP/1.0\r\nHost: 127.0.0.1:{}\r\n{extra}\r\n", port());
    split(talk(raw.as_bytes()))
}
fn get(path: &str) -> Reply {
    request("GET", path, "")
}
fn split(all: Vec<u8>) -> Reply {
    let at = all.windows(4).position(|w| w == b"\r\n\r\n").map_or(all.len(), |i| i + 4);
    Reply { head: String::from_utf8_lossy(&all[..at]).into_owned(), body: all[at..].to_vec() }
}
fn has(hay: &str, needle: &str) -> bool {
    hay.contains(needle)
}
/// (width, height) of a PNG's IHDR
fn png_size(b: &[u8]) -> (u32, u32) {
    if b.len() < 24 || &b[1..4] != b"PNG" {
        return (0, 0);
    }
    (u32::from_be_bytes(b[16..20].try_into().unwrap()), u32::from_be_bytes(b[20..24].try_into().unwrap()))
}

fn squiggle(t: ToolKind, y: f32) -> Box<Stroke> {
    let mut p = ToolParams::default();
    tool_params_default(t, &mut p);
    let mut s = stroke_new(t, &p, 99);
    for i in 0..50 {
        let f = i as f32;
        let q = PenSample { x: fmla(f, 20.0, 300.0), y: fmla(30.0, (f * 0.4).sin(), y), pressure: 0.2 + f / 70.0, tilt_x: 10.0, tilt_y: -5.0, t_ms: i as u32 * 4 };
        stroke_append(&mut s, &q, 300.0);
    }
    s
}

fn ink(nb: &NotebookInfo, index: i32, y: f32) {
    let mut p = None;
    check!(store_load_page(nb, index, 1860, 2480, &mut p) == 0);
    let mut p = p.unwrap();
    page_add(&mut p, squiggle(TOOL_PENCIL, y));
    check!(store_save_page(nb, index, &p) == 0);
}

/// The notebooks the routes are asked about: (plain, three pages in a folder with markup in both
/// names, one with no thumbnail).
struct Books {
    plain: NotebookInfo,
    three: NotebookInfo,
    bare: NotebookInfo,
}
static BOOKS: OnceLock<(String, String, String)> = OnceLock::new();
fn books() -> Books {
    let (a, b, c) = BOOKS.get().unwrap();
    let load = |id: &str| {
        let mut nb = NotebookInfo::default();
        assert_eq!(store_load_info(id, &mut nb), 0);
        nb
    };
    Books { plain: load(a), three: load(b), bare: load(c) }
}

fn make_books() {
    let mut plain = NotebookInfo::default();
    check!(store_create("Plain notebook", "", PAPER_NARROW, &mut plain) == 0);
    ink(&plain, 0, 500.0);
    let mut thumb = Canvas::default();
    check!(canvas_init(&mut thumb, 232, 310) == 0);
    for y in 0..thumb.h {
        for x in 0..thumb.w {
            thumb.px[(y * thumb.stride + x) as usize] = ((x * 3 + y * 5) & 0xFF) as u8;
        }
    }
    check!(store_save_thumb(plain.id.as_str(), &thumb) == 0);
    let mut three = NotebookInfo::default();
    check!(store_create("<b>\"quoted\" & 'single'</b> </script>", "Work & <Play>", PAPER_GRID, &mut three) == 0);
    check!(store_add_page(&mut three, 1, PAPER_GRID) == 0);
    check!(store_add_page(&mut three, 2, PAPER_DOTS) == 0);
    ink(&three, 1, 700.0);
    let mut bare = NotebookInfo::default();
    check!(store_create("Caf\u{e9} \u{fc}ber \u{d7} x", "Work & <Play>", PAPER_BLANK, &mut bare) == 0);
    check!(store_add_folder("Empty folder") == 0);
    let _ = BOOKS.set((plain.id.to_string(), three.id.to_string(), bare.id.to_string()));
}

fn serving(owner: u32) {
    check!(httpd_acquire(owner, port(), 300.0, true) == 0);
}

// ---- the routes ----

fn the_index_lists_every_notebook_in_its_folder() {
    let b = books();
    let r = get("/");
    check!(r.status() == "HTTP/1.0 200 OK");
    check!(r.header("Content-Type") == Some("text/html; charset=utf-8"));
    check!(r.header("Cache-Control") == Some("no-store"));
    check!(r.header("Content-Length").and_then(|v| v.parse::<usize>().ok()) == Some(r.body.len()));
    let t = r.text();
    check!(has(&t, "<html lang=en>"));
    check!(has(&t, "<h1 class=brand><svg class=bat"));
    check!(has(&t, "rel=icon type=image/svg+xml") && has(&t, "rel=apple-touch-icon"));
    check!(has(&t, "</h1><p class=meta>3 notebooks on your Kindle</p>"));
    check!(has(&t, "<a class=big href='/all.zip'>Download everything (zip)</a>"));
    check!(has(&t, &format!("<a href='/nb/{}'><img loading=lazy src='/nb/{}/thumb.png' alt=''></a>", b.plain.id, b.plain.id)));
    check!(has(&t, "<div class=name>Plain notebook</div>"));
    // names are text, never markup
    check!(has(&t, "&lt;b&gt;&quot;quoted&quot; &amp; &#39;single&#39;&lt;/b&gt; &lt;/script&gt;"));
    check!(!has(&t, "<b>\"quoted\""));
    check!(has(&t, "<h2>Work &amp; &lt;Play&gt; <a class=meta href='/folder/Work%20%26%20%3CPlay%3E.zip'>(zip)</a></h2>"));
    check!(has(&t, "<h2>Empty folder <a class=meta href='/folder/Empty%20folder.zip'>(zip)</a></h2>"));
    check!(has(&t, "<p class=meta>empty</p>"));
    check!(has(&t, "3 pages"));
    check!(has(&t, "1 page &middot;"));
    check!(has(&t, &format!("<a href='/nb/{}.pdf'>PDF</a><a href='/nb/{}.zip'>ZIP</a>", b.three.id, b.three.id)));
    check!(has(&t, &format!("<p class=meta style='margin-top:40px'>Grimoire {}</p>", GRIMOIRE_VERSION)));
    check!(has(&t, "window.GRIMOIRE_DOWNLOAD_WORDS=[\"Preparing PDF...\",\"Preparing PDF: {done} / {total} pages\""));
    check!(t.ends_with("</script></body></html>"));
    // the Gothic style's fonts (the next test asks for the Classic ones)
    check!(has(&t, "/font/jacquard24.ttf?v=") && has(&t, "/font/jersey10.ttf?v=") && has(&t, "<body class=gothic>"));
    check!(get("/index.html").body == r.body);
}

fn the_listing_is_json_for_scripts() {
    let b = books();
    let r = get("/api/list.json");
    check!(r.status() == "HTTP/1.0 200 OK" && r.header("Content-Type") == Some("application/json"));
    let t = r.text();
    check!(t.starts_with(&format!("{{\"device\":\"Grimoire\",\"version\":\"{}\",\"notebooks\":[{{\"id\":\"", GRIMOIRE_VERSION)));
    check!(has(&t, &format!("\"id\":\"{}\",\"name\":\"Plain notebook\",\"folder\":\"\",\"pages\":1,\"paper\":\"Narrow rule\",\"modified\":", b.plain.id)));
    // json_str keeps a name from closing a script: < > & are escaped, and so are quotes
    check!(has(&t, "\"name\":\"\\u003cb\\u003e\\\"quoted\\\" \\u0026 'single'\\u003c/b\\u003e \\u003c/script\\u003e\",\"folder\":\"Work \\u0026 \\u003cPlay\\u003e\",\"pages\":3,\"paper\":\"Graph paper\""));
    check!(has(&t, &format!("\"pdf\":\"/nb/{}.pdf\",\"zip\":\"/nb/{}.zip\",\"pngs\":[\"/nb/{}/1.png\",\"/nb/{}/2.png\",\"/nb/{}/3.png\"]", b.three.id, b.three.id, b.three.id, b.three.id, b.three.id)));
    check!(t.ends_with("],\"folders\":[\"Empty folder\",\"Work \\u0026 \\u003cPlay\\u003e\"]}\n"));
}

fn a_notebook_page_shows_one_page_and_clamps_the_entry() {
    let b = books();
    let id = b.three.id.as_str();
    let t = get(&format!("/nb/{id}")).text();
    check!(has(&t, &format!("<img style='max-width:100%;height:auto' src='/nb/{id}/1.png?f=2' alt='Page 1'>")));
    check!(has(&t, "id=page-number name=p type=number inputmode=numeric min=1 max=3 value=1"));
    check!(has(&t, "<span>/ 3</span>") && has(&t, "rel=next>&rarr;</a>") && !has(&t, "rel=prev"));
    check!(t.matches("<img ").count() == 1);
    check!(has(&t, &format!("<a class=big href='/nb/{id}/studio'>Effects (transparent, colors, gradients, filters)</a>")));
    let last = get(&format!("/nb/{id}?p=999999999")).text();
    check!(has(&last, "max=3 value=3") && has(&last, &format!("/nb/{id}/3.png?f=2")) && has(&last, "rel=prev") && !has(&last, "rel=next"));
    check!(has(&get(&format!("/nb/{id}?p=-9")).text(), "value=1"));
    check!(has(&get(&format!("/nb/{id}?p=%32")).text(), "max=3 value=2"));
    check!(has(&get(&format!("/nb/{id}?p=x")).text(), "value=1"));
    check!(has(&get(&format!("/nb/{id}/")).text(), "value=1"));
    // the name is text here too
    check!(has(&t, "<title>&lt;b&gt;&quot;quoted&quot;"));
}

fn the_studio_page_carries_its_notebook_as_json() {
    let b = books();
    let id = b.three.id.as_str();
    let r = get(&format!("/nb/{id}/studio"));
    check!(r.status() == "HTTP/1.0 200 OK");
    let t = r.text();
    check!(has(&t, &format!("<script>window.GRIMOIRE_NB={{\"id\":\"{id}\",\"name\":\"\\u003cb\\u003e\\\"quoted\\\"")));
    check!(has(&t, ",\"pages\":3,\"dpi\":300,\"version\":\""));
    check!(has(&t, "};window.GRIMOIRE_L10N={};</script><style>"));
    check!(has(&t, "<div id=studio><p class=meta>Loading the studio...</p></div>"));
    check!(has(&t, "function imCommand"));
    check!(get("/nb/zzzz/studio").status() == "HTTP/1.0 404 Not Found");
}

fn page_images_come_in_layers_and_sizes() {
    let b = books();
    let id = b.three.id.as_str();
    let r = get(&format!("/nb/{id}/2.png"));
    check!(r.status() == "HTTP/1.0 200 OK" && r.header("Content-Type") == Some("image/png"));
    check!(png_size(&r.body) == (1860, 2480));
    check!(r.header("X-Page-Size") == Some("1860x2480") && r.header("X-Paper") == Some("11") && r.header("X-Ink-Box").is_none());
    let ink_only = get(&format!("/nb/{id}/2.png?layer=ink"));
    check!(png_size(&ink_only.body) == (1860, 2480));
    check!(ink_only.header("X-Page-Size") == Some("1860x2480"));
    check!(ink_only.header("X-Paper") == Some("11"));
    let boxed = ink_only.header("X-Ink-Box").unwrap_or("");
    check!(boxed.split(',').count() == 4 && boxed.split(',').all(|n| n.parse::<i32>().is_ok()));
    // a page without strokes has no ink box
    check!(get(&format!("/nb/{id}/1.png?layer=ink")).header("X-Ink-Box") == Some("none"));
    let paper = get(&format!("/nb/{id}/2.png?layer=paper&f=2"));
    check!(png_size(&paper.body) == (930, 1240));
    check!(paper.header("X-Page-Size") == Some("1860x2480") && paper.header("X-Ink-Box").is_none());
    check!(png_size(&get(&format!("/nb/{id}/2.png?f=8")).body) == (232, 310));
    check!(png_size(&get(&format!("/nb/{id}/2.png?layer=all")).body) == (1860, 2480));
    check!(r.body != ink_only.body && r.body != paper.body);
    for bad in ["f=9", "f=0", "f=x", "layer=nope", "layer=INK"] {
        let e = get(&format!("/nb/{id}/2.png?{bad}"));
        check!(e.status() == "HTTP/1.0 400 Bad Request");
        check!(e.text() == "layer is ink, paper or all; f is 1 to 8\n");
    }
    for gone in ["/0.png", "/4.png", "/-1.png", "/1.pngx", "/1.png/", "/x.png", "/1", "/.png"] {
        check!(get(&format!("/nb/{id}{gone}")).status() == "HTTP/1.0 404 Not Found");
    }
    // sscanf's %d: a sign and leading white space are part of the number
    check!(get(&format!("/nb/{id}/+1.png")).status() == "HTTP/1.0 200 OK");
    check!(get(&format!("/nb/{id}/%201.png")).status() == "HTTP/1.0 200 OK");
}

fn thumbnails_are_the_stored_card_or_the_first_page() {
    let b = books();
    let r = get(&format!("/nb/{}/thumb.png", b.plain.id));
    check!(r.status() == "HTTP/1.0 200 OK" && r.header("Content-Type") == Some("image/png"));
    check!(png_size(&r.body) == (232, 310));
    // no card yet: the first page at full size
    check!(png_size(&get(&format!("/nb/{}/thumb.png", b.bare.id)).body) == (1860, 2480));
}

fn fonts_icons_and_the_cache() {
    let r = get("/font/jersey10.ttf?v=1");
    check!(r.status() == "HTTP/1.0 200 OK" && r.header("Content-Type") == Some("font/ttf"));
    check!(r.header("Cache-Control") == Some("public, max-age=31536000, immutable"));
    check!(r.body == grimoire::assets::FONT_JERSEY10);
    check!(get("/font/jacquard24.ttf").body == grimoire::assets::FONT_JACQUARD24);
    check!(get("/font/terminess.ttf").body == grimoire::assets::FONT_TERMINESS_REGULAR);
    check!(get("/font/terminess-bold.ttf").body == grimoire::assets::FONT_TERMINESS_BOLD);
    let nope = get("/font/nope.ttf");
    check!(nope.status() == "HTTP/1.0 404 Not Found" && nope.text() == "not found\n" && nope.header("Cache-Control") == Some("no-store"));
    let icon = get("/icon.svg");
    check!(icon.header("Content-Type") == Some("image/svg+xml") && icon.header("Cache-Control") == Some("public, max-age=31536000, immutable"));
    check!(icon.text().starts_with("<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 -14.5 43 43' shape-rendering='crispEdges'>"));
    check!(icon.text().ends_with("'/></svg>"));
    let touch = get("/apple-touch-icon.png");
    check!(png_size(&touch.body) == (180, 180) && touch.header("Cache-Control") == Some("public, max-age=31536000, immutable"));
    check!(get("/apple-touch-icon-precomposed.png").body == touch.body);
}

fn the_classic_style_has_its_own_fonts() {
    let r = get("/");
    check!(r.status() == "HTTP/1.0 200 OK");
    let t = r.text();
    check!(has(&t, "/font/terminess.ttf?v=") && has(&t, "/font/terminess-bold.ttf?v=") && has(&t, "<body class=classic>"));
    check!(!has(&t, "jacquard24"));
}

fn heads_answer_without_a_body() {
    let b = books();
    let full = get("/api/list.json");
    let head = request("HEAD", "/api/list.json", "");
    check!(head.status() == "HTTP/1.0 200 OK" && head.body.is_empty());
    check!(head.header("Content-Length").and_then(|v| v.parse::<usize>().ok()) == Some(full.body.len()));
    let pdf = request("HEAD", &format!("/nb/{}.pdf", b.plain.id), "");
    check!(pdf.status() == "HTTP/1.0 200 OK" && pdf.body.is_empty());
    check!(pdf.header("Content-Type") == Some("application/pdf"));
    check!(pdf.header("Content-Length").and_then(|v| v.parse::<usize>().ok()).unwrap_or(0) > 1000);
}

fn downloads_come_from_render_children() {
    let b = books();
    let pdf = get(&format!("/nb/{}.pdf", b.three.id));
    check!(pdf.status() == "HTTP/1.0 200 OK" && pdf.header("Content-Type") == Some("application/pdf"));
    check!(pdf.body.starts_with(b"%PDF-"));
    check!(pdf.header("Content-Length").and_then(|v| v.parse::<usize>().ok()) == Some(pdf.body.len()));
    // the name is made safe for a file (export_safe_name), cut to ASCII for the plain filename and given
    // whole, percent-encoded, in filename*
    let mut safe = String::new();
    export_safe_name(b.three.name.as_str(), &mut safe, STORE_NAME_LEN);
    check!(safe == "_b__quoted_ & 'single'__b_ __script_");
    check!(pdf.header("Content-Disposition") == Some(&format!("attachment; filename=\"{safe}.pdf\"; filename*=UTF-8''_b__quoted_%20%26%20%27single%27__b_%20__script_.pdf")[..]));
    let zip = get(&format!("/nb/{}.zip", b.plain.id));
    check!(zip.status() == "HTTP/1.0 200 OK" && zip.header("Content-Type") == Some("application/zip"));
    check!(zip.body.starts_with(b"PK\x03\x04"));
    check!(zip.header("Content-Disposition").is_some_and(|d| d.starts_with("attachment; filename=\"Plain notebook.zip\"")));
    let all = get("/all.zip");
    check!(all.body.starts_with(b"PK") && all.header("Content-Disposition").is_some_and(|d| d.contains("filename=\"Grimoire.zip\"")));
    let folder = get("/folder/Work%20%26%20%3CPlay%3E.zip");
    check!(folder.status() == "HTTP/1.0 200 OK" && folder.body.starts_with(b"PK"));
    check!(folder.header("Content-Disposition").is_some_and(|d| d.starts_with("attachment; filename=\"Work & _Play_.zip\"")));
    // a folder nobody has is an empty zip, one longer than any folder or without .zip is not found
    check!(get("/folder/Nobody.zip").status() == "HTTP/1.0 200 OK");
    check!(get(&format!("/folder/{}.zip", "x".repeat(96))).status() == "HTTP/1.0 404 Not Found");
    check!(get(&format!("/folder/{}.zip", "x".repeat(95))).status() == "HTTP/1.0 200 OK");
    check!(get("/folder/.zip").status() == "HTTP/1.0 404 Not Found");
    check!(get("/folder/abc").status() == "HTTP/1.0 404 Not Found");
}

fn progress_is_answered_while_and_after_a_child_renders() {
    let b = books();
    let token = "0123456789abcdef0123456789abcdef";
    check!(get(&format!("/api/download/{token}")).status() == "HTTP/1.0 404 Not Found");
    let pdf = get(&format!("/nb/{}.pdf?progress={token}", b.three.id));
    check!(pdf.status() == "HTTP/1.0 200 OK" && pdf.body.starts_with(b"%PDF-"));
    let st = get(&format!("/api/download/{token}"));
    check!(st.status() == "HTTP/1.0 200 OK" && st.header("Content-Type") == Some("application/json"));
    check!(st.text() == format!("{{\"state\":4,\"pages\":3,\"total\":3,\"sent\":{0},\"bytes\":{0}}}", pdf.body.len()));
    // an identifier is used once, and must be 32 hex digits
    check!(get(&format!("/nb/{}.pdf?progress={token}", b.three.id)).status() == "HTTP/1.0 400 Bad Request");
    check!(get(&format!("/nb/{}.pdf?progress=invalid", b.three.id)).text() == "invalid or busy download identifier\n");
    check!(get("/api/download/invalid").text() == "unknown download\n");
    // a failed export leaves a failed record, not a stuck bar
    let missing = "fedcba9876543210fedcba9876543210";
    check!(get(&format!("/nb/missing.pdf?progress={missing}")).status() == "HTTP/1.0 404 Not Found");
    check!(get(&format!("/api/download/{missing}")).text() == "{\"state\":5,\"pages\":0,\"total\":0,\"sent\":17,\"bytes\":17}");
    // only a GET keeps a record: a HEAD with a progress identifier is answered and forgotten
    let head = "00112233445566778899aabbccddeeff";
    let h = request("HEAD", &format!("/nb/{}.pdf?progress={head}", b.three.id), "");
    check!(h.body.is_empty() && h.status() == "HTTP/1.0 200 OK");
    check!(get(&format!("/api/download/{head}")).status() == "HTTP/1.0 404 Not Found");
}

fn refusals_and_the_words_they_use() {
    let b = books();
    let raw = |s: &str| split(talk(s.as_bytes()));
    check!(raw("BAD\r\n\r\n").status() == "HTTP/1.0 400 Bad Request");
    check!(raw("BAD\r\n\r\n").text() == "bad request\n");
    check!(raw("\r\n\r\n").text() == "bad request\n");
    let post = request("POST", "/", "");
    check!(post.status() == "HTTP/1.0 405 Method Not Allowed" && post.text() == "GET only\n");
    let evil = split(talk(b"GET / HTTP/1.0\r\nHost: evil.example.com\r\n\r\n"));
    check!(evil.status() == "HTTP/1.0 400 Bad Request" && evil.text() == "use the address shown on the Kindle\n");
    check!(split(talk(b"GET /api/list.json HTTP/1.0\r\nHost: localhost:8765\r\n\r\n")).status() == "HTTP/1.0 200 OK");
    check!(split(talk(b"GET /api/list.json HTTP/1.0\r\nHost: [::1]:8080\r\n\r\n")).status() == "HTTP/1.0 200 OK");
    check!(split(talk(b"GET /api/list.json HTTP/1.1\r\n\r\n")).status() == "HTTP/1.0 200 OK");
    check!(split(talk(b"GET /api/list.json HTTP/1.0\nHost: 10.0.0.1\n\n")).status() == "HTTP/1.0 200 OK");
    check!(get("/nb/../x").text() == "bad path\n");
    check!(get("/nb/%2e%2e/x").text() == "bad path\n");
    check!(get("/nope").text() == "not found\n");
    check!(get("/nb/").text() == "not found\n");
    check!(get(&format!("/nb/{}", "a".repeat(24))).text() == "not found\n");
    let gone = get("/nb/zzzz");
    check!(gone.status() == "HTTP/1.0 404 Not Found" && gone.text() == "no such notebook\n");
    check!(get(&format!("/nb/{}/nope", b.plain.id)).text() == "not found\n");
    // the whole request in 4095 bytes without its end
    let big = format!("GET / HTTP/1.0\r\nX-Pad: {}", "a".repeat(4095 - 23));
    check!(big.len() == 4095);
    let r = split(talk(big.as_bytes()));
    check!(r.status() == "HTTP/1.0 400 Bad Request" && r.text() == "request too large\n");
    // a %xx that is not hex ends the path as a C string does
    check!(get("/nb/%zz").text() == "not found\n");
}

fn the_server_counts_what_it_answers() {
    let before = httpd_requests_served();
    get("/api/list.json");
    check!(httpd_requests_served() == before + 1);
    let b = books();
    get(&format!("/nb/{}.pdf", b.plain.id)); // a child's: counted when it starts
    check!(httpd_requests_served() == before + 2);
}

fn a_second_owner_shares_the_server_and_the_last_one_stops_it() {
    check!(httpd_owners() == HTTPD_SHARE && httpd_running() && httpd_fd() >= 0);
    check!(httpd_acquire(HTTPD_LIVE, port(), 300.0, true) == 0 && httpd_owners() == HTTPD_SHARE | HTTPD_LIVE);
    httpd_release(HTTPD_SHARE);
    check!(httpd_owners() == HTTPD_LIVE && httpd_running());
    // only Live view keeps it: a download is refused, the fonts and /live answer
    let r = get("/api/list.json");
    check!(r.status() == "HTTP/1.0 403 Forbidden" && r.text() == "Open Share on the Kindle to download.\n");
    check!(get("/nb/x.pdf").status() == "HTTP/1.0 403 Forbidden");
    check!(get("/font/jersey10.ttf").status() == "HTTP/1.0 200 OK");
    let live = get("/live");
    check!(live.status() == "HTTP/1.0 200 OK");
    let t = live.text();
    check!(has(&t, "<h1 class=brand><svg class=bat") && has(&t, "Type the code shown on the Kindle:"));
    check!(has(&t, "<form class=code action='/live/pair' method=get>") && !has(&t, "class=err"));
    check!(get("/live/").body == live.body);
    check!(has(&get("/live?e=wrong").text(), "<p class=err role=alert>That code is not the one on the Kindle.</p>"));
    check!(has(&get("/live?e=locked").text(), "Too many tries. Wait 30 seconds, then type the new code."));
    check!(!has(&get("/live?e=other").text(), "class=err"));
    check!(get("/live/nope").text() == "not found\n");
    httpd_release(HTTPD_LIVE);
    check!(httpd_owners() == 0 && !httpd_running() && httpd_fd() == -1);
    // Share alone: /live is a page that says Live view is off
    serving(HTTPD_SHARE);
    check!(has(&get("/live").text(), "Live view is off. On the Kindle, open Share and tap Live view."));
    check!(get("/live/pair?code=123456").status() == "HTTP/1.0 404 Not Found");
    check!(get("/live/stream").status() == "HTTP/1.0 404 Not Found");
}

fn a_browser_pairs_with_the_code_and_gets_a_session() {
    serving(HTTPD_LIVE);
    check!(pair_begin(PAIR_LIVE, (now_ms() / 1000) as i64) == 0);
    let code = pair_code();
    let wrong = if code == "000000" { "000001" } else { "000000" };
    let r = get(&format!("/live/pair?code={wrong}"));
    check!(r.status() == "HTTP/1.0 302 Found" && r.header("Location") == Some("/live?e=wrong") && r.header("Set-Cookie").is_none());
    check!(httpd_live_pairings() == 0);
    // not paired: no stream
    check!(get("/live/stream").status() == "HTTP/1.0 401 Unauthorized");
    check!(get("/live/stream").text() == "pair first\n");
    // the code as it is shown, with its space
    let shown = format!("{}%20{}", &code[..3], &code[3..]);
    let ok = get(&format!("/live/pair?code={shown}"));
    check!(ok.status() == "HTTP/1.0 302 Found" && ok.header("Location") == Some("/live"));
    let cookie = ok.header("Set-Cookie").unwrap_or("").to_string();
    check!(cookie.starts_with("grimoire_session=") && cookie.ends_with("; HttpOnly; SameSite=Strict; Path=/"));
    check!(httpd_live_pairings() == 1);
    // the invitation is used up
    check!(get(&format!("/live/pair?code={code}")).header("Location") == Some("/live?e=wrong"));
    let session = cookie.split(';').next().unwrap_or("").to_string();
    let viewer = split(talk(format!("GET /live HTTP/1.0\r\nHost: 127.0.0.1\r\nCookie: {session}\r\n\r\n").as_bytes()));
    check!(viewer.status() == "HTTP/1.0 200 OK");
    let t = viewer.text();
    check!(has(&t, "<div id=live><p class=meta>Connecting...</p></div>") && has(&t, "window.GRIMOIRE_LIVE_WORDS={\"connecting\":\"Connecting...\",\"reconnecting\":\"Reconnecting...\""));
    check!(has(&t, "\"old\":\"This browser cannot show Live view.\"};</script><template id=live-bat><svg class=bat"));
    // paired, but Live view is not running (no live_start here)
    let stream = split(talk(format!("GET /live/stream HTTP/1.0\r\nHost: 127.0.0.1\r\nCookie: {session}\r\n\r\n").as_bytes()));
    check!(stream.status() == "HTTP/1.0 503 Service Unavailable" && stream.text() == "live view is off\n");
    let head = split(talk(format!("HEAD /live/stream HTTP/1.0\r\nHost: 127.0.0.1\r\nCookie: {session}\r\n\r\n").as_bytes()));
    check!(head.status() == "HTTP/1.0 400 Bad Request");
    // the QR code's token pairs too, once
    check!(pair_begin(PAIR_LIVE, (now_ms() / 1000) as i64) == 0);
    let token = pair_qr_token();
    check!(token.len() == 32);
    check!(get(&format!("/live/pair?t={token}")).header("Location") == Some("/live"));
    check!(httpd_live_pairings() == 2);
    // five wrong codes lock the pairing for a while
    check!(pair_begin(PAIR_LIVE, (now_ms() / 1000) as i64) == 0);
    for _ in 0..4 {
        check!(get("/live/pair?code=abc").header("Location") == Some("/live?e=wrong"));
    }
    check!(get("/live/pair?code=abc").header("Location") == Some("/live?e=locked"));
    check!(get("/live/pair").header("Location") == Some("/live?e=locked"));
    pair_reset();
    httpd_release(HTTPD_LIVE);
}

fn the_listening_socket_is_the_servers() {
    // a port in use is refused, and the owner stays unset
    httpd_stop();
    let l = TcpListener::bind(("0.0.0.0", free_port() as u16)).unwrap();
    let busy = l.local_addr().unwrap().port() as i32;
    check!(httpd_acquire(HTTPD_SHARE, busy, 300.0, true) == -1);
    check!(!httpd_running() && httpd_owners() == 0 && httpd_fd() == -1);
    drop(l);
    // stopping closes the connections it had
    check!(httpd_acquire(HTTPD_SHARE, port(), 300.0, true) == 0);
    let mut idle = TcpStream::connect(("127.0.0.1", port() as u16)).unwrap();
    for _ in 0..5 {
        httpd_poll();
        std::thread::sleep(Duration::from_millis(2));
    }
    httpd_stop();
    idle.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    let mut b = [0u8; 8];
    check!(matches!(idle.read(&mut b), Ok(0) | Err(_)));
    check!(!httpd_running() && httpd_owners() == 0);
    serving(HTTPD_SHARE);
}

fn addresses_come_wi_fi_first() {
    // the machine's own: no loopback, ranked, within the room given
    let mut some = [NetAddr::default(); 8];
    let n = net_ipv4_list(&mut some);
    check!((0..=8).contains(&n));
    for i in 0..n as usize {
        check!(!some[i].ip.is_empty() && !some[i].ip.as_str().starts_with("127.") && some[i].ip.as_str().split('.').count() == 4);
        check!(!some[i].ifname.is_empty());
    }
    let rank = |s: &str| if s.starts_with("wlan") { 0 } else if s.starts_with("en") || s.starts_with("eth") { 1 } else if s.starts_with("usb") { 3 } else { 2 };
    for i in 1..n as usize {
        check!(rank(some[i - 1].ifname.as_str()) <= rank(some[i].ifname.as_str()));
    }
    let mut none: [NetAddr; 0] = [];
    check!(net_ipv4_list(&mut none) == 0);
    let mut strings = Vec::new();
    let m = net_ipv4_addrs(&mut strings, 8);
    check!(m as usize == strings.len() && m <= 8);
    check!(strings.iter().all(|s| !s.is_empty() && s.len() <= 15));
    check!(net_ipv4_addrs(&mut strings, 0) == 0 && strings.is_empty());
}

#[cfg(feature = "sim")]
fn simulated_addresses_stand_in_for_the_interfaces() {
    // GRIMOIRE_SIM_ADDRS: "ifname=ip", comma or space apart; what is not an IPv4 address is skipped
    std::env::set_var("GRIMOIRE_SIM_ADDRS", "usb0=192.168.15.244,en0=10.0.0.5, wlan1=10.1.1.1 foo0=1.1.1.1,wlan0=192.168.0.20,=1.2.3.4,bad=300.1.1.1,none");
    let mut out = [NetAddr::default(); 8];
    let n = net_ipv4_list(&mut out);
    check!(n == 5);
    let got: Vec<(&str, &str)> = out[..n as usize].iter().map(|a| (a.ifname.as_str(), a.ip.as_str())).collect();
    check!(got == [("wlan1", "10.1.1.1"), ("wlan0", "192.168.0.20"), ("en0", "10.0.0.5"), ("foo0", "1.1.1.1"), ("usb0", "192.168.15.244")]);
    let mut three = [NetAddr::default(); 3];
    check!(net_ipv4_list(&mut three) == 3 && three[2].ifname == "en0");
    let mut strings = Vec::new();
    check!(net_ipv4_addrs(&mut strings, 2) == 2 && strings == ["192.168.15.244", "10.0.0.5"]);
    std::env::set_var("GRIMOIRE_SIM_ADDRS", "");
    check!(net_ipv4_list(&mut out) == 0);
    std::env::set_var("GRIMOIRE_SIM_ADDRS", "averyveryverylongname0=1.2.3.4");
    check!(net_ipv4_list(&mut out) == 1 && out[0].ifname == "averyveryverylo");
    std::env::remove_var("GRIMOIRE_SIM_ADDRS");
}

#[cfg(feature = "sim")]
fn a_render_whose_browser_hangs_up_in_the_queue_never_starts() {
    // Renders as slow as the Kindle's (GRIMOIRE_HTTPD_DELAY_MS) keep both slots busy while a third
    // and a fourth wait; the third hangs up, and only the fourth is rendered after the first two.
    std::env::set_var("GRIMOIRE_HTTPD_DELAY_MS", "400");
    let b = books();
    let path = format!("/nb/{}/1.png?f=4", b.plain.id);
    let raw = format!("GET {path} HTTP/1.0\r\nHost: 127.0.0.1\r\n\r\n");
    let before = httpd_requests_served();
    let mut conns: Vec<TcpStream> = Vec::new();
    for _ in 0..4 {
        let mut s = TcpStream::connect(("127.0.0.1", port() as u16)).unwrap();
        s.write_all(raw.as_bytes()).unwrap();
        s.set_nonblocking(true).unwrap();
        conns.push(s);
        for _ in 0..5 {
            httpd_poll();
            std::thread::sleep(Duration::from_millis(2));
        }
    }
    let quitter = conns.remove(2);
    drop(quitter);
    let until = Instant::now() + Duration::from_secs(60);
    let mut done = [false; 3];
    let mut got = [0usize; 3];
    let mut buf = vec![0u8; 1 << 16];
    while !done.iter().all(|d| *d) {
        httpd_poll();
        for i in 0..3 {
            if done[i] {
                continue;
            }
            match conns[i].read(&mut buf) {
                Ok(0) => done[i] = true,
                Ok(n) => got[i] += n,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(_) => done[i] = true,
            }
        }
        std::thread::sleep(Duration::from_millis(1));
        assert!(Instant::now() < until, "the renders never finished");
    }
    check!(got.iter().all(|n| *n > 1000));
    check!(httpd_requests_served() == before + 3);
    std::env::remove_var("GRIMOIRE_HTTPD_DELAY_MS");
}

#[test]
fn test_httpd() {
    let root = format!("{}/httpd-{}", tl_tmp(), std::process::id());
    let _ = ROOT.set(root.clone());
    let _ = std::fs::remove_dir_all(&root);
    check!(store_init(&root) == 0);
    store_set_device(300.0, 1860, 2480);
    ui_init(300.0);
    check!(font_init() == 0);
    i18n_apply("en");
    let _ = PORT.set(free_port());
    make_books();
    ui_set_theme(UiTheme::Gothic);
    serving(HTTPD_SHARE);
    run!(the_index_lists_every_notebook_in_its_folder);
    run!(the_listing_is_json_for_scripts);
    run!(a_notebook_page_shows_one_page_and_clamps_the_entry);
    run!(the_studio_page_carries_its_notebook_as_json);
    run!(page_images_come_in_layers_and_sizes);
    run!(thumbnails_are_the_stored_card_or_the_first_page);
    run!(fonts_icons_and_the_cache);
    run!(heads_answer_without_a_body);
    run!(downloads_come_from_render_children);
    run!(progress_is_answered_while_and_after_a_child_renders);
    run!(refusals_and_the_words_they_use);
    run!(the_server_counts_what_it_answers);
    // the Classic style: the server takes the pages' style when it starts
    httpd_stop();
    check!(httpd_acquire(HTTPD_SHARE, port(), 300.0, false) == 0);
    run!(the_classic_style_has_its_own_fonts);
    httpd_stop();
    serving(HTTPD_SHARE);
    run!(a_second_owner_shares_the_server_and_the_last_one_stops_it);
    run!(a_browser_pairs_with_the_code_and_gets_a_session);
    run!(the_listening_socket_is_the_servers);
    run!(addresses_come_wi_fi_first);
    #[cfg(feature = "sim")]
    run!(simulated_addresses_stand_in_for_the_interfaces);
    #[cfg(feature = "sim")]
    run!(a_render_whose_browser_hangs_up_in_the_queue_never_starts);
    httpd_stop();
    let _ = std::fs::remove_dir_all(&root);
    done();
}
