// The Find sheet (src/findsheet.rs, Rust only): the rows what is typed offers, and where its card goes. The
// editor draws and refreshes it (tests/scenarios/z_find_*.qs, j_pages_keyboard.qs).
#![allow(clippy::field_reassign_with_default)]
mod testlib;
use grimoire::prelude::*;
use testlib::*;

fn app(w: i32, h: i32) -> App {
    ui_init(300.0);
    App { w, h, dpi: 300.0, ..Default::default() }
}

// A notebook of 12 pages whose names are set as given: (page id, name).
fn notebook(names: &[(u32, &str)]) -> (NotebookInfo, Nav) {
    let mut nb = NotebookInfo::default();
    nb.page_ids = (1..=12).collect();
    let mut nav = Nav::default();
    nav_init(&mut nav);
    for (pid, name) in names {
        check!(nav_set_name(&mut nav, *pid, name) == 0);
    }
    (nb, nav)
}

fn typed(f: &mut FindSheet, a: &App, nb: &NotebookInfo, nav: &Nav, text: &str) -> Vec<String> {
    // (typing keeps the keyboard where it was drawn)
    let area = f.kb.area;
    keyboard_open(&mut f.kb, "Page number or name", text.as_bytes());
    f.kb.area = area;
    findsheet_update(f, a, nb, nav);
    (0..f.n).map(|i| f.rows[i].label.as_str().to_string()).collect()
}

fn the_rows_follow_what_is_typed() {
    let a = app(1860, 2480);
    let (nb, nav) = notebook(&[(3, "\u{c1}rbol"), (5, "Arena"), (7, "La reuni\u{f3}n"), (9, "Plan de arbitraje"), (11, "Zebra")]);
    let mut f = FindSheet::default();
    findsheet_open(&mut f, &a);
    check!(f.open && f.n == 0 && f.rect == RECT_EMPTY && f.kb.title == "Page number or name");
    // nothing typed: nothing offered
    check!(typed(&mut f, &a, &nb, &nav, "").is_empty() && f.rect == RECT_EMPTY && f.hits.h.is_empty());
    // a number the notebook has: one row, a button; one it has not: a line that is no button
    let rows = typed(&mut f, &a, &nb, &nav, "12");
    check!(rows == ["Go to page 12"] && f.rows[0].page == 11 && !f.rows[0].named && f.hits.h.len() == 1);
    let rows = typed(&mut f, &a, &nb, &nav, "13");
    check!(rows == ["There is no page 13 (12 pages)."] && f.rows[0].page == -1 && f.hits.h.is_empty());
    let rows = typed(&mut f, &a, &nb, &nav, "0");
    check!(rows == ["There is no page 0 (12 pages)."]);
    let rows = typed(&mut f, &a, &nb, &nav, "99999999999999999999");
    check!(rows.len() == 1 && f.rows[0].page == -1);
    // names: the beginning first, then a word, then anywhere; page order among equals; accents and capitals ignored
    let rows = typed(&mut f, &a, &nb, &nav, "ar");
    check!(rows == ["\u{c1}rbol", "Arena", "Plan de arbitraje"]); // two begin so (pages 3, 5), a word begins so (9)
    check!(f.rows[0].page == 2 && f.rows[1].page == 4 && f.rows[2].page == 8 && f.rows.iter().take(3).all(|r| r.named));
    check!(typed(&mut f, &a, &nb, &nav, "ARB") == ["\u{c1}rbol", "Plan de arbitraje"]);
    check!(typed(&mut f, &a, &nb, &nav, "reu") == ["La reuni\u{f3}n"]);
    check!(typed(&mut f, &a, &nb, &nav, "bra") == ["Zebra"]);
    check!(typed(&mut f, &a, &nb, &nav, "  rbo ") == ["\u{c1}rbol"]);
    // none: one line that is no button
    let rows = typed(&mut f, &a, &nb, &nav, "qq");
    check!(rows == ["No page names match."] && f.rows[0].page == -1 && f.hits.h.is_empty());
    // a notebook with no names offers none either
    let (nb2, nav2) = notebook(&[]);
    check!(typed(&mut f, &a, &nb2, &nav2, "a") == ["No page names match."]);
    // one page: its number is singular
    let mut one = nb2.clone();
    one.page_ids = vec![1];
    check!(typed(&mut f, &a, &one, &nav2, "2") == ["There is no page 2 (1 page)."]);
}

fn there_are_at_most_five_rows_upright_and_three_sideways() {
    let mut names: Vec<(u32, String)> = Vec::new();
    for pid in 1..=12u32 {
        names.push((pid, format!("Tema {pid}")));
    }
    let refs: Vec<(u32, &str)> = names.iter().map(|(p, n)| (*p, n.as_str())).collect();
    let (nb, nav) = notebook(&refs);
    let a = app(1860, 2480);
    let mut f = FindSheet::default();
    findsheet_open(&mut f, &a);
    let rows = typed(&mut f, &a, &nb, &nav, "tema");
    check!(rows.len() == 5 && rows[0] == "Tema 1" && rows[4] == "Tema 5");
    let b = app(2480, 1860);
    findsheet_open(&mut f, &b);
    let rows = typed(&mut f, &b, &nb, &nav, "tema");
    check!(rows.len() == 3 && rows[2] == "Tema 3");
}

// The card sits 12 px above the keyboard, a row 124 px with 16 between and 16 of padding, never above 8 px from the
// screen's top; only the rows that are buttons have hits, at the rows' places.
fn the_card_is_above_the_keyboard() {
    let (nb, nav) = notebook(&[(1, "Aa"), (2, "Ab"), (3, "Ac"), (4, "Ad"), (5, "Ae")]);
    for (w, h) in [(1860, 2480), (2480, 1860)] {
        let a = app(w, h);
        let mut f = FindSheet::default();
        findsheet_open(&mut f, &a);
        let kb = keyboard_modal_rect(Rect { x: 0, y: 0, w, h });
        check!(f.kb.area == kb);
        let rows = typed(&mut f, &a, &nb, &nav, "a");
        let n = rows.len() as i32;
        check!(n == if w > h { 3 } else { 5 });
        check!(f.rect.y + f.rect.h == kb.y - 12 && f.rect.x == kb.x && f.rect.w == kb.w);
        check!(f.rect.h == 32 + n * 124 + (n - 1) * 16);
        check!(f.rect.y >= 8);
        check!(f.hits.h.len() as i32 == n);
        // the last row sits on the bottom padding, the first on the top's
        check!(f.hits.h[0].r.y == f.rect.y + 16 && f.hits.h[(n - 1) as usize].r.y + 124 == f.rect.y + f.rect.h - 16);
        check!(f.hits.h.iter().all(|h| rect_contains_rect(f.rect, h.r)));
        // a row's probe label is the name alone; past the last is empty
        let mut out = String::new();
        check!(findsheet_probe(&f, "1", &mut out, 64) && out == "Aa");
        check!(findsheet_probe(&f, "rows", &mut out, 64) && out == n.to_string());
        check!(findsheet_probe(&f, "9", &mut out, 64) && out.is_empty());
        check!(!findsheet_probe(&f, "x", &mut out, 64));
    }
}

fn rect_contains_rect(o: Rect, i: Rect) -> bool {
    i.x >= o.x && i.y >= o.y && i.x + i.w <= o.x + o.w && i.y + i.h <= o.y + o.h
}

#[test]
fn test_findsheet() {
    ui_init(300.0);
    check!(font_init() == 0);
    run!(the_rows_follow_what_is_typed);
    run!(there_are_at_most_five_rows_upright_and_three_sideways);
    run!(the_card_is_above_the_keyboard);
    done();
}
