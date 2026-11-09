// pdffix: a small PDF writer for the tests (2.1.0-beta.2, DESIGN.md 9 "L5"; areas/pdf.md 7). Every PDF a test
// or a scenario reads is made here, never hand-edited: `make pdf-fixtures` writes the few the scenarios copy
// into tests/fixtures/pdf/, and test_pdf_fixtures.rs checks the committed bytes are what this makes now.
// No compression and no fonts but Helvetica (hayro draws it with its own substitute), so every file is
// plain text a person can read.
#![allow(dead_code)]

use grimoire::md5::Md5;

/// What one page is: its size in points, /Rotate, an optional crop box, and its content stream.
#[derive(Clone, Debug)]
pub struct PageSpec {
    pub w: f64,
    pub h: f64,
    pub rotate: i32,
    pub crop: Option<[f64; 4]>,
    pub content: String,
}

impl PageSpec {
    pub fn new(w: f64, h: f64, content: &str) -> PageSpec {
        PageSpec { w, h, rotate: 0, crop: None, content: content.to_string() }
    }
}

/// A filled rectangle in user space: `r g b` then the box.
pub fn rect(rgb: (f64, f64, f64), x: f64, y: f64, w: f64, h: f64) -> String {
    format!("{} {} {} rg {} {} {} {} re f\n", rgb.0, rgb.1, rgb.2, x, y, w, h)
}

pub fn gray_rect(g: f64, x: f64, y: f64, w: f64, h: f64) -> String {
    rect((g, g, g), x, y, w, h)
}

/// A plain object list written out with its cross-reference table and trailer.
struct Writer {
    out: Vec<u8>,
    offs: Vec<usize>,
}

impl Writer {
    fn new() -> Writer {
        Writer { out: b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n".to_vec(), offs: Vec::new() }
    }
    fn obj(&mut self, body: &[u8]) -> usize {
        self.offs.push(self.out.len());
        let n = self.offs.len();
        self.out.extend_from_slice(format!("{n} 0 obj\n").as_bytes());
        self.out.extend_from_slice(body);
        self.out.extend_from_slice(b"\nendobj\n");
        n
    }
    fn stream(&mut self, data: &[u8]) -> usize {
        let mut b = format!("<< /Length {} >>\nstream\n", data.len()).into_bytes();
        b.extend_from_slice(data);
        b.extend_from_slice(b"\nendstream");
        self.obj(&b)
    }
    fn finish(mut self, root: usize, extra_trailer: &str) -> Vec<u8> {
        let xref = self.out.len();
        self.out.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", self.offs.len() + 1).as_bytes());
        for o in &self.offs {
            self.out.extend_from_slice(format!("{o:010} 00000 n \n").as_bytes());
        }
        self.out.extend_from_slice(
            format!("trailer\n<< /Size {} /Root {root} 0 R {extra_trailer}>>\nstartxref\n{xref}\n%%EOF\n", self.offs.len() + 1).as_bytes(),
        );
        self.out
    }
}

/// A document of these pages: objects 1 (catalog), 2 (page tree), a font, then each page and its content.
pub fn make(pages: &[PageSpec]) -> Vec<u8> {
    let mut w = Writer::new();
    let n = pages.len();
    // object numbers: 1 catalog, 2 pages, 3 font, then page i = 4 + 2i, content = 5 + 2i
    w.obj(b"<< /Type /Catalog /Pages 2 0 R >>");
    let kids: Vec<String> = (0..n).map(|i| format!("{} 0 R", 4 + 2 * i)).collect();
    w.obj(format!("<< /Type /Pages /Count {n} /Kids [{}] >>", kids.join(" ")).as_bytes());
    w.obj(b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>");
    for (i, p) in pages.iter().enumerate() {
        let crop = p.crop.map_or(String::new(), |c| format!(" /CropBox [{} {} {} {}]", c[0], c[1], c[2], c[3]));
        let rot = if p.rotate != 0 { format!(" /Rotate {}", p.rotate) } else { String::new() };
        w.obj(
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {} {}]{crop}{rot} /Resources << /Font << /F1 3 0 R >> >> /Contents {} 0 R >>",
                p.w,
                p.h,
                5 + 2 * i
            )
            .as_bytes(),
        );
        w.stream(p.content.as_bytes());
    }
    w.finish(1, "")
}

/// A planner of `n` pages, 420 x 595 points (A5), each with a black frame, a block of gray that tells the page
/// apart (darker with the page number), a thin grid, and its number in Helvetica.
pub fn planner(n: usize) -> Vec<u8> {
    let pages: Vec<PageSpec> = (0..n).map(|k| PageSpec::new(420.0, 595.0, &planner_content(k + 1))).collect();
    make(&pages)
}

/// The gray the block of planner page `k` (from 1) is drawn in, 0 to 255.
pub fn planner_gray(k: usize) -> u8 {
    (230 - ((k * 23) % 200)) as u8
}

pub fn planner_content(k: usize) -> String {
    let g = planner_gray(k) as f64 / 255.0;
    let mut c = String::new();
    c.push_str(&gray_rect(g, 60.0, 400.0, 300.0, 120.0)); // the block
    c.push_str("0 G 2 w 20 20 380 555 re S\n"); // the frame
    c.push_str("0.6 G 0.5 w\n");
    for i in 1..8 {
        c.push_str(&format!("20 {} m 400 {} l S\n", 20 + i * 50, 20 + i * 50));
    }
    c.push_str(&format!("BT /F1 18 Tf 40 540 Td (Page {k}) Tj ET\n"));
    c
}

/// Three pages: gray, then one with a red box and a pale yellow band, then gray again.
pub fn colour() -> Vec<u8> {
    let gray = |k| PageSpec::new(420.0, 595.0, &planner_content(k));
    let mut c = planner_content(2);
    c.push_str(&rect((1.0, 0.0, 0.0), 60.0, 100.0, 120.0, 120.0));
    c.push_str(&rect((1.0, 0.98, 0.92), 20.0, 300.0, 380.0, 40.0));
    make(&[gray(1), PageSpec::new(420.0, 595.0, &c), gray(3)])
}

/// A page with a filled box at user-space (x, y, w, h), page size w0 x h0, a rotation and a crop box.
pub fn boxed(w0: f64, h0: f64, rotate: i32, crop: Option<[f64; 4]>, bx: [f64; 4]) -> Vec<u8> {
    let mut p = PageSpec::new(w0, h0, &gray_rect(0.0, bx[0], bx[1], bx[2], bx[3]));
    p.rotate = rotate;
    p.crop = crop;
    make(&[p])
}

/// n pages of nothing much (a page tree that is long, for the cap).
pub fn many(n: usize) -> Vec<u8> {
    let mut w = Writer::new();
    w.obj(b"<< /Type /Catalog /Pages 2 0 R >>");
    let kids: Vec<String> = (0..n).map(|i| format!("{} 0 R", 3 + i)).collect();
    w.obj(format!("<< /Type /Pages /Count {n} /Kids [{}] >>", kids.join(" ")).as_bytes());
    for _ in 0..n {
        w.obj(b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 100 100] >>");
    }
    w.finish(1, "")
}

// ---- encryption: RC4, 40 bit, revision 2 (what a "password" PDF of the plainest kind is) ----

fn rc4(key: &[u8], data: &[u8]) -> Vec<u8> {
    let mut s: Vec<u8> = (0..=255).collect();
    let mut j = 0usize;
    for i in 0..256 {
        j = (j + s[i] as usize + key[i % key.len()] as usize) & 255;
        s.swap(i, j);
    }
    let (mut i, mut j) = (0usize, 0usize);
    data.iter()
        .map(|&b| {
            i = (i + 1) & 255;
            j = (j + s[i] as usize) & 255;
            s.swap(i, j);
            b ^ s[(s[i] as usize + s[j] as usize) & 255]
        })
        .collect()
}

const PAD: [u8; 32] = [
    0x28, 0xBF, 0x4E, 0x5E, 0x4E, 0x75, 0x8A, 0x41, 0x64, 0x00, 0x4E, 0x56, 0xFF, 0xFA, 0x01, 0x08, 0x2E, 0x2E, 0x00, 0xB6, 0xD0, 0x68, 0x3E, 0x80,
    0x2F, 0x0C, 0xA9, 0xFE, 0x64, 0x53, 0x69, 0x7A,
];

fn md5(parts: &[&[u8]]) -> [u8; 16] {
    let mut m = Md5::new();
    for p in parts {
        m.update(p);
    }
    m.finish()
}

fn padded(pw: &str) -> Vec<u8> {
    let mut v: Vec<u8> = pw.as_bytes().iter().copied().take(32).collect();
    v.extend_from_slice(&PAD[..32 - v.len()]);
    v
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02X}")).collect()
}

/// The planner of `n` pages, encrypted: opens without a password only when `user` is empty (the owner password
/// then only limits what a reader allows, which Grimoire does not enforce).
pub fn encrypted_planner(n: usize, user: &str, owner: &str) -> Vec<u8> {
    let id = [7u8; 16];
    let p: i32 = -4;
    // Algorithm 3 (revision 2): O
    let okey = md5(&[&padded(if owner.is_empty() { user } else { owner })]);
    let o = rc4(&okey[..5], &padded(user));
    // Algorithm 2: the file key; Algorithm 4: U
    let key = md5(&[&padded(user), &o, &p.to_le_bytes(), &id]);
    let key = key[..5].to_vec();
    let u = rc4(&key, &PAD);
    let mut w = Writer::new();
    let objkey = |num: usize| {
        let k = md5(&[&key, &(num as u32).to_le_bytes()[..3], &[0, 0]]);
        k[..10.min(key.len() + 5)].to_vec()
    };
    w.obj(b"<< /Type /Catalog /Pages 2 0 R >>");
    let kids: Vec<String> = (0..n).map(|i| format!("{} 0 R", 4 + 2 * i)).collect();
    w.obj(format!("<< /Type /Pages /Count {n} /Kids [{}] >>", kids.join(" ")).as_bytes());
    w.obj(b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>");
    for i in 0..n {
        w.obj(
            format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 420 595] /Resources << /Font << /F1 3 0 R >> >> /Contents {} 0 R >>", 5 + 2 * i).as_bytes(),
        );
        let num = w.offs.len() + 1;
        let enc = rc4(&objkey(num), planner_content(i + 1).as_bytes());
        w.stream(&enc);
    }
    let e = w.obj(format!("<< /Filter /Standard /V 1 /R 2 /O <{}> /U <{}> /P {p} >>", hex(&o), hex(&u)).as_bytes());
    w.finish(1, &format!("/Encrypt {e} 0 R /ID [<{}> <{}>] ", hex(&id), hex(&id)))
}

/// Bytes that are no PDF but look finished to the scan (they start and end as one does).
pub fn junk() -> Vec<u8> {
    let mut v = b"%PDF-1.4\n".to_vec();
    let mut x = 12345u32;
    for _ in 0..4000 {
        x = x.wrapping_mul(1664525).wrapping_add(1013904223);
        v.push((x >> 24) as u8);
    }
    v.extend_from_slice(b"\n%%EOF\n");
    v
}

/// The planner cut in half (its page tree and cross-reference table lost) with a finished-looking tail.
pub fn truncated(n: usize) -> Vec<u8> {
    let full = planner(n);
    let mut v = full[..full.len() / 2].to_vec();
    v.extend_from_slice(b"\n%%EOF\n");
    v
}

/// A page tree whose only kid is the tree itself.
pub fn page_tree_loop() -> Vec<u8> {
    let mut w = Writer::new();
    w.obj(b"<< /Type /Catalog /Pages 2 0 R >>");
    w.obj(b"<< /Type /Pages /Count 3 /Kids [2 0 R 2 0 R] >>");
    w.finish(1, "")
}

/// A page tree nested `depth` deep (each node holds the next).
pub fn page_tree_deep(depth: usize) -> Vec<u8> {
    let mut w = Writer::new();
    w.obj(b"<< /Type /Catalog /Pages 2 0 R >>");
    for i in 0..depth {
        w.obj(format!("<< /Type /Pages /Count 1 /Kids [{} 0 R] >>", 3 + i).as_bytes());
    }
    w.obj(b"<< /Type /Page /MediaBox [0 0 100 100] >>");
    w.finish(1, "")
}

/// The files the scenarios copy: (name, bytes).
pub fn scenario_files() -> Vec<(&'static str, Vec<u8>)> {
    vec![
        ("planner.pdf", planner(7)),
        ("colour.pdf", colour()),
        ("locked.pdf", encrypted_planner(2, "secret", "owner")),
    ]
}
