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


// ---- links, named destinations and an outline (lane L6) ----

/// What a link annotation does.
#[derive(Clone, Debug)]
pub enum Act {
    /// `/Dest [page /XYZ ...]`: an explicit destination naming the page's object (page from 1)
    Page(usize),
    /// `/Dest [n /Fit]`: the page by a 0-based number (out of spec, seen in the wild)
    PageNum(usize),
    /// `/Dest /name` and `/Dest (name)`
    NameDest(String),
    StrDest(String),
    /// `/A << /S /GoTo /D (name) >>` and `/D [page /Fit]`
    GoToName(String),
    GoToPage(usize),
    /// `/A << /S /Named /N /NextPage >>` and the other three
    Named(&'static str),
    Uri(&'static str),
    Launch(&'static str),
    /// a destination whose page object is not in the file
    Missing,
    /// no action at all
    Nothing,
}

#[derive(Clone, Debug)]
pub struct Lnk {
    /// x0 y0 x1 y1 in user space
    pub rect: [f64; 4],
    pub act: Act,
    /// QuadPoints, eight numbers each
    pub quads: Vec<[f64; 8]>,
    /// /F, 0 for none
    pub flags: i32,
}

impl Lnk {
    pub fn new(rect: [f64; 4], act: Act) -> Lnk {
        Lnk { rect, act, quads: Vec::new(), flags: 0 }
    }
}

/// An outline item: its title as the raw bytes of a PDF string, the page it goes to, its children.
#[derive(Clone, Debug)]
pub struct OutItem {
    pub title: Vec<u8>,
    pub page: usize,
    pub kids: Vec<OutItem>,
}

impl OutItem {
    pub fn new(title: &str, page: usize) -> OutItem {
        OutItem { title: title.as_bytes().to_vec(), page, kids: Vec::new() }
    }
}

/// UTF-16BE with a BOM, as the pdfTeX manual's titles arrive.
pub fn utf16be(s: &str) -> Vec<u8> {
    let mut v = vec![0xFE, 0xFF];
    for u in s.encode_utf16() {
        v.extend_from_slice(&u.to_be_bytes());
    }
    v
}

#[derive(Default, Clone, Debug)]
pub struct Extra {
    /// the catalog's name tree (/Names /Dests): a root with two leaves of /Kids
    pub tree: Vec<(String, usize)>,
    /// more entries in the tree, `bulk0000001` and so on, each to page 2
    pub tree_bulk: usize,
    /// the first leaf lists the root among its kids
    pub tree_loop: bool,
    /// the PDF 1.1 /Dests dictionary
    pub dests: Vec<(String, usize)>,
    pub outline: Vec<OutItem>,
    /// the last top-level item's /Next is the first
    pub outline_loop: bool,
}

fn outline_count(items: &[OutItem]) -> usize {
    items.iter().map(|i| 1 + outline_count(&i.kids)).sum()
}

/// The pages of `pages`, each with its links, plus the catalog's extras. Objects: 1 catalog, 2 pages, 3 font,
/// page i (from 0) 4 + 2i and its content 5 + 2i, then every annotation in page order, the name tree (root and
/// two leaves, when there is one), then the outline (root, then the items depth first).
pub fn make_linked(pages: &[PageSpec], links: &[Vec<Lnk>], extra: &Extra) -> Vec<u8> {
    let n = pages.len();
    let pobj = |p: usize| 4 + 2 * (p - 1); // page from 1
    let a0 = 4 + 2 * n;
    let total_annots: usize = (0..n).map(|i| links.get(i).map_or(0, |l| l.len())).sum();
    let t0 = a0 + total_annots;
    let has_tree = !extra.tree.is_empty() || extra.tree_bulk > 0;
    let o0 = t0 + if has_tree { 3 } else { 0 };
    let has_outline = !extra.outline.is_empty();
    let mut w = Writer::new();
    let mut cat = String::from("<< /Type /Catalog /Pages 2 0 R");
    if has_tree {
        cat.push_str(&format!(" /Names << /Dests {t0} 0 R >>"));
    }
    if !extra.dests.is_empty() {
        cat.push_str(" /Dests <<");
        for (k, p) in &extra.dests {
            cat.push_str(&format!(" /{k} [{} 0 R /Fit]", pobj(*p)));
        }
        cat.push_str(" >>");
    }
    if has_outline {
        cat.push_str(&format!(" /Outlines {o0} 0 R"));
    }
    cat.push_str(" >>");
    w.obj(cat.as_bytes());
    let kids: Vec<String> = (0..n).map(|i| format!("{} 0 R", 4 + 2 * i)).collect();
    w.obj(format!("<< /Type /Pages /Count {n} /Kids [{}] >>", kids.join(" ")).as_bytes());
    w.obj(b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>");
    let mut next_annot = a0;
    for (i, p) in pages.iter().enumerate() {
        let crop = p.crop.map_or(String::new(), |c| format!(" /CropBox [{} {} {} {}]", c[0], c[1], c[2], c[3]));
        let rot = if p.rotate != 0 { format!(" /Rotate {}", p.rotate) } else { String::new() };
        let mine = links.get(i).map_or(0, |l| l.len());
        let annots = if mine > 0 {
            let refs: Vec<String> = (next_annot..next_annot + mine).map(|k| format!("{k} 0 R")).collect();
            format!(" /Annots [{}]", refs.join(" "))
        } else {
            String::new()
        };
        next_annot += mine;
        w.obj(
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {} {}]{crop}{rot}{annots} /Resources << /Font << /F1 3 0 R >> >> /Contents {} 0 R >>",
                p.w,
                p.h,
                5 + 2 * i
            )
            .as_bytes(),
        );
        w.stream(p.content.as_bytes());
    }
    for page_links in links.iter().take(n) {
        for l in page_links {
            let act = match &l.act {
                Act::Page(p) => format!("/Dest [{} 0 R /XYZ 10 10 0]", pobj(*p)),
                Act::PageNum(p) => format!("/Dest [{p} /Fit]"),
                Act::NameDest(s) => format!("/Dest /{s}"),
                Act::StrDest(s) => format!("/Dest ({s})"),
                Act::GoToName(s) => format!("/A << /S /GoTo /D ({s}) >>"),
                Act::GoToPage(p) => format!("/A << /S /GoTo /D [{} 0 R /Fit] >>", pobj(*p)),
                Act::Named(nm) => format!("/A << /S /Named /N /{nm} >>"),
                Act::Uri(u) => format!("/A << /S /URI /URI ({u}) >>"),
                Act::Launch(f) => format!("/A << /S /Launch /F ({f}) >>"),
                Act::Missing => "/Dest [999 0 R /Fit]".to_string(),
                Act::Nothing => String::new(),
            };
            let quads = if l.quads.is_empty() {
                String::new()
            } else {
                let v: Vec<String> = l.quads.iter().flat_map(|q| q.iter().map(|x| x.to_string())).collect();
                format!(" /QuadPoints [{}]", v.join(" "))
            };
            let f = if l.flags != 0 { format!(" /F {}", l.flags) } else { String::new() };
            w.obj(format!("<< /Type /Annot /Subtype /Link /Rect [{} {} {} {}] /Border [0 0 0] {act}{quads}{f} >>", l.rect[0], l.rect[1], l.rect[2], l.rect[3]).as_bytes());
        }
    }
    if has_tree {
        let mut entries: Vec<(String, usize)> = extra.tree.clone();
        for k in 0..extra.tree_bulk {
            entries.push((format!("bulk{k:07}"), 2));
        }
        entries.sort();
        let half = entries.len() / 2;
        let leaf = |es: &[(String, usize)]| -> String {
            let mut s = String::from("<< /Names [");
            for (k, p) in es {
                s.push_str(&format!("({k}) [{} 0 R /XYZ 0 0 0] ", pobj(*p)));
            }
            s.push(']');
            s
        };
        w.obj(format!("<< /Kids [{} 0 R {} 0 R] >>", t0 + 1, t0 + 2).as_bytes());
        let looped = if extra.tree_loop { format!(" /Kids [{t0} 0 R]") } else { String::new() };
        w.obj(format!("{}{looped} >>", leaf(&entries[..half])).as_bytes());
        w.obj(format!("{} >>", leaf(&entries[half..])).as_bytes());
    }
    if has_outline {
        let total = outline_count(&extra.outline);
        w.obj(format!("<< /Type /Outlines /First {} 0 R /Last {} 0 R /Count {total} >>", o0 + 1, o0 + 1 + {
            // the last top-level item's number: the items before it, with their kids
            let top = &extra.outline;
            outline_count(&top[..top.len() - 1])
        }).as_bytes());
        // depth first: the number of an item is o0 + 1 + (the items before it in that order)
        fn emit(w: &mut Writer, items: &[OutItem], parent: usize, first_no: usize, pobj: &dyn Fn(usize) -> usize, looped: bool) {
            let mut no = first_no;
            let nos: Vec<usize> = items
                .iter()
                .map(|i| {
                    let me = no;
                    no += 1 + outline_count(&i.kids);
                    me
                })
                .collect();
            for (k, it) in items.iter().enumerate() {
                let hex: String = it.title.iter().map(|b| format!("{b:02X}")).collect();
                let mut d = format!("<< /Title <{hex}> /Parent {parent} 0 R");
                if k > 0 {
                    d.push_str(&format!(" /Prev {} 0 R", nos[k - 1]));
                }
                if k + 1 < items.len() {
                    d.push_str(&format!(" /Next {} 0 R", nos[k + 1]));
                } else if looped {
                    d.push_str(&format!(" /Next {} 0 R", nos[0]));
                }
                if !it.kids.is_empty() {
                    let kid0 = nos[k] + 1;
                    let kid_last = nos[k] + 1 + outline_count(&it.kids[..it.kids.len() - 1]);
                    d.push_str(&format!(" /First {kid0} 0 R /Last {kid_last} 0 R /Count {}", outline_count(&it.kids)));
                }
                d.push_str(&format!(" /Dest [{} 0 R /Fit] >>", pobj(it.page)));
                w.obj(d.as_bytes());
                emit(w, &it.kids, nos[k], nos[k] + 1, pobj, false);
            }
        }
        emit(&mut w, &extra.outline, o0, o0 + 1, &pobj, extra.outline_loop);
    }
    w.finish(1, "")
}

/// A page of the given size with a content stream and no links.
pub fn plain(w: f64, h: f64, content: &str) -> PageSpec {
    PageSpec::new(w, h, content)
}

/// The year planner of the tests: 380 empty pages, 15,000 links. Page 1 (the year) holds 1,000 day cells, each of
/// the 12 month pages 150, every other page the 33 tabs of its edge. Over a notebook's water level (14,400), so the
/// densest page (the year) is the one that loses links.
pub fn year_planner() -> Vec<u8> {
    let n = 380usize;
    let pages: Vec<PageSpec> = (0..n).map(|_| PageSpec::new(465.0, 620.0, "")).collect();
    let target = |p: usize, k: usize| ((p - 1 + 1 + (k % 379)) % 380) + 1; // never p itself
    let mut links: Vec<Vec<Lnk>> = Vec::new();
    for p in 1..=n {
        let mut v = Vec::new();
        if p == 1 {
            for k in 0..1000usize {
                let (c, r) = (k % 25, k / 25);
                let x = 10.0 + c as f64 * 17.0;
                let y = 10.0 + r as f64 * 14.0;
                v.push(Lnk::new([x, y, x + 15.0, y + 12.0], Act::Page(target(p, k))));
            }
        } else if p <= 13 {
            for k in 0..150usize {
                let (c, r) = (k % 15, k / 15);
                let x = 10.0 + c as f64 * 27.0;
                let y = 10.0 + r as f64 * 30.0;
                v.push(Lnk::new([x, y, x + 25.0, y + 28.0], Act::Page(target(p, k))));
            }
        } else {
            for k in 0..33usize {
                let y = 10.0 + k as f64 * 17.0;
                v.push(Lnk::new([430.0, y, 460.0, y + 15.0], Act::Page(target(p, k))));
            }
        }
        links.push(v);
    }
    make_linked(&pages, &links, &Extra::default())
}

/// Four pages of 465 x 620 points (a quarter of the Scribe's page: one point is 4 px), each with a tab on its
/// right edge that goes to the next page but one, and (page 1) a second link to page 4 by a name in the name
/// tree. The scenario taps a tab.
pub fn tabs() -> Vec<u8> {
    let pages: Vec<PageSpec> = (0..4).map(|k| PageSpec::new(465.0, 620.0, &planner_content_small(k + 1))).collect();
    let mut links: Vec<Vec<Lnk>> = Vec::new();
    for p in 1..=4usize {
        let mut v = vec![Lnk::new([380.0, 500.0, 460.0, 540.0], Act::Page(if p <= 2 { p + 2 } else { p - 2 }))];
        if p == 1 {
            v.push(Lnk::new([20.0, 20.0, 100.0, 60.0], Act::NameDest("last".into())));
        }
        links.push(v);
    }
    let extra = Extra { tree: vec![("last".into(), 4)], outline: vec![OutItem::new("Cover", 1), OutItem::new("Notes", 3)], ..Default::default() };
    make_linked(&pages, &links, &extra)
}

fn planner_content_small(k: usize) -> String {
    let mut c = String::new();
    c.push_str("0 G 2 w 20 20 425 580 re S\n");
    c.push_str(&gray_rect(0.7, 380.0, 500.0, 80.0, 40.0)); // the tab
    c.push_str(&format!("BT /F1 18 Tf 40 580 Td (Tabs {k}) Tj ET\n"));
    c
}

/// The files the scenarios copy: (path under tests/fixtures/pdf, bytes). Each folder is one scenario's `# copy:`.
pub fn scenario_files() -> Vec<(&'static str, Vec<u8>)> {
    vec![
        ("planner/2027 Planner.pdf", planner(7)),
        ("colour/Colour.pdf", colour()),
        ("locked/Locked.pdf", encrypted_planner(2, "secret", "owner")),
        ("twice/First.pdf", planner(3)),
        ("twice/Second.pdf", planner(3)),
        ("folder/Work/Report.pdf", planner(2)),
        ("mixed/Etudes Op. 10 notes.pdf", planner(2)),
        ("junk/Broken.pdf", junk()),
        ("pick/Planner.pdf", planner(2)),
        ("tabs/Tabs Planner.pdf", tabs()),
    ]
}
