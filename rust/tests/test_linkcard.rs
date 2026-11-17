// The long-press card (src/linkcard.rs, Rust only): where it goes above the Links bar (beside a held link it
// is the popout's place, tests/test_popout.rs), its rows for every kind of link, its height, and the heavy
// border's four strips.
#![allow(clippy::field_reassign_with_default)]
mod testlib;
use grimoire::prelude::*;
use testlib::*;

const UP: Rect = Rect { x: 0, y: 0, w: 1860, h: 2480 };
const SIDE: Rect = Rect { x: 0, y: 0, w: 2480, h: 1860 };

// From the bar: above it, 16 px over it, in the held chip's column, clamped to the margins; the bar's three
// heights (456, 596, 736) and the tallest card always fit below the corner zone.
fn the_card_goes_above_the_bar() {
    let (w, h) = (780, 666);
    for (screen, chip_xs) in [(UP, [40, 942, 1500]), (SIDE, [40, 848, 1656])] {
        for bh in [456, 596, 736] {
            let bar = Rect { x: 20, y: screen.h - 20 - bh, w: screen.w - 40, h: bh };
            for cx in chip_xs {
                let chip = Rect { x: cx, y: bar.y + 200, w: 700, h: 124 };
                let r = linkcard_place_above(w, h, chip, bar, screen);
                check!(r.y == bar.y - 16 - h && r.y > 165 + 8);
                check!(r.x >= 20 && r.x + r.w <= screen.w - 20);
                check!(r.x == cx.min(screen.w - 20 - w));
            }
        }
    }
}

fn nb5() -> NotebookInfo {
    let mut nb = NotebookInfo::default();
    nb.page_ids = vec![10, 20, 30, 40, 50];
    nb
}

fn nav_with(links: &[(u32, &str)]) -> Nav {
    let mut nav = Nav::default();
    nav_init(&mut nav);
    check!(nav_set_name(&mut nav, 30, "Árbol") == 0);
    for &(seed, t) in links {
        check!(nav_link_set(&mut nav, 10, seed, t) == 0);
    }
    nav
}

fn rows(c: &LinkCard) -> String {
    let mut out = String::new();
    check!(linkcard_probe(c, "rows", &mut out, 128));
    out
}

// The rows of every case (PHASE2 8.3).
fn the_rows_follow_the_link() {
    let nb = nb5();
    let nav = nav_with(&[(1, "p30"), (2, "p20"), (3, "index"), (4, "p99"), (5, "p30"), (6, "index")]);
    let mut c = LinkCard::default();
    // a link to a named page
    linkcard_set(&mut c, 10, &[1], &nb, &nav, false, false);
    check!(c.open && c.page == 2 && c.target == "p30" && c.title == "Árbol");
    check!(rows(&c) == "rename retarget redraw delete");
    check!(c.labels[0] == "Rename the page" && c.labels[3] == "Delete link");
    check!(linkcard_size(&c) == (780, 666));
    // to an unnamed page: Name page 2...
    linkcard_set(&mut c, 10, &[2], &nb, &nav, false, false);
    check!(c.title == "Page 2" && rows(&c) == "rename retarget redraw delete" && c.labels[0] == "Name page 2...");
    // to the Index: no Rename
    linkcard_set(&mut c, 10, &[3], &nb, &nav, false, false);
    check!(c.title == "The Index" && rows(&c) == "retarget redraw delete" && linkcard_size(&c) == (780, 536));
    // a page gone: Rename disabled
    linkcard_set(&mut c, 10, &[4], &nb, &nav, false, false);
    check!(c.page < 0 && c.title == "Page not here" && rows(&c) == "-rename retarget redraw delete");
    // nav.ini cannot be saved: Rename disabled
    let mut ro = nav_with(&[(1, "p30")]);
    ro.writable = false;
    linkcard_set(&mut c, 10, &[1], &nb, &ro, false, false);
    check!(rows(&c) == "-rename retarget redraw delete");
    // a bar row of two links to page 3: no Redraw, Change where they go..., Delete 2 links, and the line
    linkcard_set(&mut c, 10, &[1, 5], &nb, &nav, false, true);
    check!(c.from_bar && rows(&c) == "rename retarget delete");
    check!(c.labels[1] == "Change where they go..." && c.labels[2] == "Delete 2 links" && linkcard_size(&c) == (780, 608));
    let mut out = String::new();
    check!(linkcard_probe(&c, "links", &mut out, 8) && out == "2");
    // two to the Index
    linkcard_set(&mut c, 10, &[3, 6], &nb, &nav, false, true);
    check!(rows(&c) == "retarget delete" && linkcard_size(&c) == (780, 478));
    // a read-only page: no rows
    linkcard_set(&mut c, 10, &[1], &nb, &nav, true, false);
    check!(rows(&c) == "readonly" && c.nrows == 0 && linkcard_size(&c) == (780, 218));
    // closed: nothing
    let c = LinkCard::default();
    check!(rows(&c).is_empty());
    check!(linkcard_probe(&c, "rect", &mut out, 32) && out.is_empty());
    check!(linkcard_probe(&c, "border", &mut out, 32) && out == "0");
    check!(!linkcard_probe(&c, "nonsense", &mut out, 32));
}

// The card's height for 2, 3 and 4 rows, the ×N line, and read-only.
fn the_card_is_as_tall_as_its_rows() {
    check!(linkcard_height(4, false, false) == 666);
    check!(linkcard_height(3, false, false) == 536);
    check!(linkcard_height(2, false, false) == 406);
    check!(linkcard_height(3, true, false) == 608);
    check!(linkcard_height(2, true, false) == 478);
    check!(linkcard_height(0, false, true) == 218);
}

// A web link's card (2.2): the host as its title, the address as stored, one row (Done), no page; its height for one
// line of address and for six (the most it shows).
fn a_web_links_card_shows_its_address_and_done() {
    let mut c = LinkCard::default();
    linkcard_set_web(&mut c, 10, 0x1234, "https://Example.org:8080/a?b#c");
    check!(c.open && c.page == -1 && c.seeds == vec![0x1234] && c.pid == 10);
    check!(c.title == "example.org:8080");
    check!(rows(&c) == "close");
    let mut out = String::new();
    check!(linkcard_probe(&c, "web", &mut out, 256) && out == "https://Example.org:8080/a?b#c");
    check!(linkcard_web_height(1) == 1018);
    check!(linkcard_web_height(6) == 1018 + 5 * 46);
    // the card of a link that is not a web link shows no address
    let nb = nb5();
    let nav = nav_with(&[(1, "p30")]);
    linkcard_set(&mut c, 10, &[1], &nb, &nav, false, false);
    check!(c.web.is_empty());
    check!(linkcard_probe(&c, "web", &mut out, 256) && out.is_empty());
}

// A web link's card shows the whole address and the real end of its host in every look the l10n check runs (Gothic,
// and Classic in Terminess, Atkinson, Noto Sans and Literata): the address is drawn in Terminess, where 200 bytes of
// any printable character take 5 lines, under the 6 the card draws; the title, in Terminess bold, shows a host too
// long for it as "..." and its end. (In a look's own face, 200 bytes of a wide character took up to 9 lines.)
fn a_web_links_card_shows_the_whole_address_in_every_look() {
    check!(font_init() == 0);
    let mut looks: Vec<(&str, bool, usize)> = vec![("gothic", true, 0), ("classic", false, 0)];
    for id in ["atkinson", "noto-sans", "literata"] {
        looks.push((id, false, font_family_find(id.as_bytes()).expect(id)));
    }
    let host = format!("www.example.org.{}.evil.example", "W".repeat(WEBLINK_ADDRESS_MAX - 8 - 16 - 13 - 1));
    let spoof = format!("https://{host}/");
    check!(spoof.len() == WEBLINK_ADDRESS_MAX && weblinks_address_ok(spoof.as_bytes()));
    for (name, gothic, family) in looks {
        font_set_gothic(gothic);
        font_set_family(family);
        let mut worst = (0, ' ');
        for b in 0x21u8..=0x7e {
            let a = format!("https://{}", (b as char).to_string().repeat(WEBLINK_ADDRESS_MAX - 8));
            let n = linkcard_web_lines(&a);
            if n > worst.0 {
                worst = (n, b as char);
            }
        }
        if worst.0 > 5 || worst.0 > LINKCARD_WEB_LINES_MAX {
            println!("{name}: 200 bytes of {:?} take {} lines", worst.1, worst.0);
            check!(false);
        }
        // the title: the host whole when it fits, else its end
        check!(linkcard_web_title("example.org") == "example.org");
        let t = linkcard_web_title(&host);
        check!(t.starts_with("...") && t.ends_with(".evil.example") && host.ends_with(&t[3..]));
        check!(font_measure(FontFace::FixedBold, ui_text_size(), &t) <= 780 - 2 * 24 - 16);
        let mut c = LinkCard::default();
        linkcard_set_web(&mut c, 1, 1, &spoof);
        let mut out = String::new();
        check!(linkcard_probe(&c, "web_lines", &mut out, 64) && out == format!("{0} {0}", linkcard_web_lines(&spoof)));
    }
    font_set_gothic(false);
    font_set_family(0);
}

// The four strips do not overlap and together are exactly what the border paints: on a mid-gray canvas every
// pixel in them turns black or white and none outside them changes.
fn the_strips_are_what_the_border_paints() {
    for r in [Rect { x: 100, y: 120, w: 600, h: 300 }, Rect { x: 50, y: 60, w: 40, h: 36 }, Rect { x: 0, y: 0, w: 200, h: 80 }] {
        let st = linkcard_strips(r);
        for i in 0..4 {
            for j in i + 1..4 {
                check!(!rect_overlaps(st[i], st[j]));
            }
        }
        let mut c = Canvas::default();
        check!(canvas_init(&mut c, 900, 600) == 0);
        canvas_fill(&mut c, Rect { x: 0, y: 0, w: 900, h: 600 }, 128);
        linkcard_border(&mut c, r);
        let mut black = 0;
        for y in 0..600 {
            for x in 0..900 {
                let p = c.px[(y * c.stride + x) as usize];
                let on = st.iter().any(|&s| rect_contains(s, x, y));
                if on {
                    check!(p == 0 || p == 255);
                    black += (p == 0) as i32;
                } else {
                    check!(p == 128);
                }
            }
        }
        check!(black > 0);
        check!(linkcard_border_reach(r) == rect_inflate(r, 3));
    }
}

#[test]
fn test_linkcard() {
    ui_init(300.0);
    run!(the_card_goes_above_the_bar);
    run!(the_rows_follow_the_link);
    run!(the_card_is_as_tall_as_its_rows);
    run!(a_web_links_card_shows_its_address_and_done);
    run!(a_web_links_card_shows_the_whole_address_in_every_look);
    run!(the_strips_are_what_the_border_paints);
    done();
}
