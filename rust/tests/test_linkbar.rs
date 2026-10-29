// The Links bar (src/linkbar.rs, Rust only): how a page's links group into entries and in what order they are
// numbered (the eye's tags and the bar's rows are one list), and what the bar's layout makes of them.
#![allow(clippy::field_reassign_with_default)]
mod testlib;
use grimoire::prelude::*;
use testlib::*;

fn nb5() -> NotebookInfo {
    let mut nb = NotebookInfo::default();
    nb.page_ids = vec![10, 20, 30, 40, 50];
    nb
}

fn link(seed: u32, target: &str) -> NavLink {
    NavLink { pid: 10, seed, target: FixedStr::from_str(target), ..Default::default() }
}

fn box_at(x: i32, y: i32) -> Rect {
    Rect { x, y, w: 200, h: 100 }
}

fn group(nb: &NotebookInfo, links: &[NavLink], areas: &[Rect]) -> (usize, Vec<LbEntry>, [u8; LB_MAX]) {
    let mut entries = [LbEntry::default(); LB_MAX];
    let mut numbers = [0u8; LB_MAX];
    let n = linkbar_group(nb, links, areas, 300.0, &mut entries, &mut numbers);
    (n, entries[..n].to_vec(), numbers)
}

// Two links to page 3 are one entry with count 2: one row, "x2".
fn links_group_by_target_and_count() {
    let nb = nb5();
    let links = [link(1, "p30"), link(2, "p20"), link(3, "p30")];
    let areas = [box_at(300, 1000), box_at(300, 1400), box_at(300, 1800)];
    let (n, e, num) = group(&nb, &links, &areas);
    check!(n == 2);
    check!(e[0].target == "p30" && e[0].count == 2 && e[0].page == 2 && e[0].first_box == areas[0]);
    check!(e[1].target == "p20" && e[1].count == 1 && e[1].page == 1 && e[1].first_box == areas[1]);
    // every box wears its entry's number
    check!(num[0] == 1 && num[1] == 2 && num[2] == 1 && num[3] == 0);
}

// Reading order: down the page; boxes whose tops are within 10 mm (118 px at 300 dpi) are one line, read
// left to right. The entries take their numbers from where their first box is read.
fn entries_are_numbered_in_reading_order() {
    let nb = nb5();
    // a lower box to page 20 listed first, then two on one line (tops 100 px apart) to 30 and 40, the right one
    // listed before the left one
    let links = [link(1, "p20"), link(2, "p40"), link(3, "p30")];
    let areas = [box_at(300, 1800), box_at(900, 1000), box_at(300, 1100)];
    let (n, e, num) = group(&nb, &links, &areas);
    check!(n == 3);
    check!(e[0].target == "p30" && e[1].target == "p40" && e[2].target == "p20");
    check!(num[0] == 3 && num[1] == 2 && num[2] == 1);
    // 118 px apart is one line, 119 is the next
    let links = [link(1, "p20"), link(2, "p30")];
    let (_, e, _) = group(&nb, &links, &[box_at(900, 1000), box_at(300, 1118)]);
    check!(e[0].target == "p30" && e[1].target == "p20");
    let (_, e, _) = group(&nb, &links, &[box_at(900, 1000), box_at(300, 1119)]);
    check!(e[0].target == "p20" && e[1].target == "p30");
    // an entry's first box is the first of its boxes in reading order, wherever it was listed
    let links = [link(1, "p20"), link(2, "p20")];
    let (n, e, num) = group(&nb, &links, &[box_at(300, 2000), box_at(300, 1000)]);
    check!(n == 1 && e[0].count == 2 && e[0].first_box == box_at(300, 1000) && num[0] == 1 && num[1] == 1);
}

// A link to the Index, one to a page that is gone, and one to a target a newer version made.
fn the_index_and_missing_targets_are_entries() {
    let nb = nb5();
    let links = [link(1, "index"), link(2, "p99"), link(3, "q7"), link(4, "p99")];
    let areas = [box_at(300, 1000), box_at(300, 1300), box_at(300, 1600), box_at(300, 1900)];
    let (n, e, _) = group(&nb, &links, &areas);
    check!(n == 3);
    check!(e[0].target == "index" && e[0].page == NAV_TO_INDEX && e[0].count == 1);
    check!(e[1].target == "p99" && e[1].page == -1 && e[1].count == 2);
    check!(e[2].target == "q7" && e[2].page == -1 && e[2].count == 1);
    // nothing: no entries
    let (n, _, num) = group(&nb, &[], &[]);
    check!(n == 0 && num.iter().all(|&v| v == 0));
}

// A stroke through the points given (x, y pairs), with this seed: a link's ink.
fn ink(seed: u32, xy: &[f32]) -> Box<Stroke> {
    let mut prm = ToolParams::default();
    prm.size_min_mm = 0.5;
    prm.size_max_mm = 0.5;
    prm.opacity_min = 1.0;
    prm.opacity_max = 1.0;
    let mut s = stroke_new(TOOL_FINELINER, &prm, seed);
    for i in 0..xy.len() / 2 {
        let pt = PenSample { x: xy[2 * i], y: xy[2 * i + 1], pressure: 0.5, tilt_x: 0.0, tilt_y: 0.0, t_ms: i as u32 };
        stroke_append(&mut s, &pt, 300.0);
    }
    s
}

// The bar's entries from a real page and a real Nav: grouped by target, in reading order, a link whose ink is
// gone left out, a link to the Index and to a page that is gone listed; read-only and unreadable states.
fn the_bar_is_built_from_the_page() {
    let nb = nb5();
    let mut nav = Nav::default();
    nav_init(&mut nav);
    let mut p = page_new(1860, 2480, PAPER_BLANK);
    // three boxes to page 30 (twice, tops 1000 and 1800), page 20 (1400), and a box with no link ink to 40
    page_add(&mut p, ink(0x11, &[300.0, 1000.0, 900.0, 1200.0]));
    page_add(&mut p, ink(0x22, &[300.0, 1400.0, 900.0, 1500.0]));
    page_add(&mut p, ink(0x33, &[300.0, 1800.0, 900.0, 1900.0]));
    page_add(&mut p, ink(0x44, &[1000.0, 600.0, 1200.0, 700.0]));
    check!(nav_link_set(&mut nav, 10, 0x11, "p30") == 0);
    check!(nav_link_set(&mut nav, 10, 0x22, "p20") == 0);
    check!(nav_link_set(&mut nav, 10, 0x33, "p30") == 0);
    check!(nav_link_set(&mut nav, 10, 0x55, "p40") == 0); // (no ink of seed 0x55: dormant)
    check!(nav_link_set(&mut nav, 10, 0x44, "index") == 0);
    let mut lb = LinkBar::default();
    linkbar_build(&mut lb, &nb, &nav, &p, 0, 300.0);
    // reading order: the Index (top 600), page 30 (1000, and again at 1800), page 20 (1400)
    check!(lb.open && lb.n == 3 && lb.links == 4 && lb.links_ok && !lb.readonly && lb.selected == -1);
    check!(lb.entries[0].target == "index" && lb.entries[0].page == NAV_TO_INDEX && lb.entries[0].count == 1);
    check!(lb.entries[1].target == "p30" && lb.entries[1].page == 2 && lb.entries[1].count == 2);
    check!(lb.entries[2].target == "p20" && lb.entries[2].page == 1 && lb.entries[2].count == 1);
    let mut out = String::new();
    check!(linkbar_probe(&lb, "entries", &mut out, 64) && out == "3");
    check!(linkbar_probe(&lb, "1", &mut out, 64) && out == "index");
    check!(linkbar_probe(&lb, "2", &mut out, 64) && out == "p3 x2");
    check!(linkbar_probe(&lb, "3", &mut out, 64) && out == "p2");
    check!(linkbar_probe(&lb, "4", &mut out, 64) && out.is_empty());
    check!(!linkbar_probe(&lb, "nonsense", &mut out, 64));
    // a link to a page that has gone: listed, missing
    let mut gone = nb.clone();
    gone.page_ids = vec![10, 20, 40, 50];
    linkbar_build(&mut lb, &gone, &nav, &p, 0, 300.0);
    check!(lb.n == 3 && lb.entries[1].page == -1);
    check!(linkbar_probe(&lb, "2", &mut out, 64) && out == "missing");
    // a read-only page lists its links, and says so
    p.readonly = true;
    linkbar_build(&mut lb, &nb, &nav, &p, 0, 300.0);
    check!(lb.readonly && lb.links_ok && lb.n == 3);
    // links.ini unreadable: no entries, and the bar knows
    nav.links_writable = false;
    linkbar_build(&mut lb, &nb, &nav, &p, 0, 300.0);
    check!(!lb.links_ok && lb.n == 0 && lb.open);
    // no page: closed again by the editor; an origin past the notebook lists nothing
    nav.links_writable = true;
    linkbar_build(&mut lb, &nb, &nav, &p, 9, 300.0);
    check!(lb.n == 0);
}

// The card: 316 + 140 R tall (R rows of chips, 1 to 3 upright, 1 or 2 sideways), 60 more with the hint line,
// 20 px in from the sides and above the bottom edge; a pager from the seventh entry; six a screen either way.
fn the_card_is_as_tall_as_its_rows_need() {
    let mut lb = LinkBar::default();
    for (n, h) in [(0, 456), (1, 456), (2, 456), (3, 596), (4, 596), (5, 736), (6, 736), (7, 736), (13, 736)] {
        lb.n = n;
        lb.header_hint = false;
        linkbar_layout(&mut lb, 1860, 2480);
        check!(lb.rect == Rect { x: 20, y: 2480 - 20 - h, w: 1820, h });
        lb.header_hint = true;
        linkbar_layout(&mut lb, 1860, 2480);
        check!(lb.rect == Rect { x: 20, y: 2480 - 20 - h - 60, w: 1820, h: h + 60 });
    }
    for (n, h) in [(0, 456), (3, 456), (4, 596), (6, 596), (7, 596), (30, 596)] {
        lb.n = n;
        lb.header_hint = false;
        linkbar_layout(&mut lb, 2480, 1860);
        check!(lb.rect == Rect { x: 20, y: 1860 - 20 - h, w: 2440, h });
    }
    // the pager: screens of six, and a screen that is past the last is brought back
    lb.n = 6;
    lb.screen = 3;
    linkbar_layout(&mut lb, 1860, 2480);
    check!(lb.screen == 0);
    lb.n = 7;
    lb.screen = 3;
    linkbar_layout(&mut lb, 1860, 2480);
    check!(lb.screen == 1);
    lb.n = 13;
    lb.screen = 2;
    linkbar_layout(&mut lb, 1860, 2480);
    check!(lb.screen == 2);
    let mut out = String::new();
    lb.open = true;
    check!(linkbar_probe(&lb, "pager", &mut out, 16) && out == "3/3");
    lb.n = 5;
    linkbar_layout(&mut lb, 1860, 2480);
    check!(linkbar_probe(&lb, "pager", &mut out, 16) && out.is_empty());
    // a bigger screen: the card spans it, the height is the same
    lb.n = 2;
    linkbar_layout(&mut lb, 1986, 2648);
    check!(lb.rect == Rect { x: 20, y: 2648 - 20 - 456, w: 1946, h: 456 });
}

fn t(s: &str) -> FixedStr<NAV_TARGET_LEN> {
    FixedStr::from_str(s)
}

fn chips(lb: &LinkBar) -> Vec<String> {
    let mut v = Vec::new();
    let mut out = String::new();
    for i in 1..=lb.picks.len() {
        check!(linkbar_probe_pick(lb, &i.to_string(), &mut out, 64));
        v.push(out.clone());
    }
    v
}

// Link to...'s chips (PHASE2 6.6): New page..., The Index, up to four recent targets (not the open page, the
// Index, a page gone or a repeat), then every named page in the Index's order, the open page included.
fn link_to_offers_its_chips_in_order() {
    let nb = nb5();
    let mut nav = Nav::default();
    nav_init(&mut nav);
    check!(nav_set_name(&mut nav, 30, "Árbol") == 0);
    check!(nav_set_name(&mut nav, 10, "One") == 0);
    check!(nav_set_name(&mut nav, 50, "Arena") == 0);
    let mut lb = LinkBar::default();
    check!(linkbar_build_pick(&mut lb, &nb, &nav, &[], 10, "", false) == 0);
    check!(lb.open && lb.mode == LbMode::Pick && !lb.no_pages && lb.selected == -1);
    check!(chips(&lb) == ["new", "index", "p1", "p3", "p5"]);
    check!(lb.picks[0].kind == PickKind::NewPage && lb.picks[1].kind == PickKind::Index && lb.picks[2].kind == PickKind::Entry);
    check!(lb.picks[3].target == "p30" && lb.picks[3].page == 2);
    // the recent targets, newest first: the open page (p10), the Index, a page gone (p99), a repeat and a fifth left out
    let recent = [t("p20"), t("p10"), t("index"), t("p99"), t("p20"), t("p40"), t("p30"), t("p50"), t("p40")];
    check!(linkbar_build_pick(&mut lb, &nb, &nav, &recent, 10, "", false) == 0);
    check!(chips(&lb) == ["new", "index", "recent p2", "recent p4", "recent p3", "recent p5", "p1", "p3", "p5"]);
    let mut out = String::new();
    check!(linkbar_probe_pick(&lb, "chips", &mut out, 16) && out == "9");
    check!(linkbar_probe_pick(&lb, "10", &mut out, 16) && out.is_empty());
    // the current target is marked wherever it is offered, and the pager opens on its first screen
    check!(linkbar_build_pick(&mut lb, &nb, &nav, &recent, 10, "p50", true) == 0);
    check!(lb.no_pages && lb.selected == 5);
    check!(chips(&lb)[5] == "recent p5 current" && chips(&lb)[8] == "p5 current" && chips(&lb)[7] == "p3");
    linkbar_layout_pick(&mut lb, 1860, 2480, false, 0);
    check!(lb.screen == 0);
    check!(linkbar_build_pick(&mut lb, &nb, &nav, &recent, 10, "p50", true) == 0);
    lb.selected = 8;
    linkbar_layout_pick(&mut lb, 1860, 2480, false, 0);
    check!(lb.screen == 1);
    check!(linkbar_probe_pick(&lb, "screen", &mut out, 16) && out == "2/2");
    // the Index is current: its chip
    check!(linkbar_build_pick(&mut lb, &nb, &nav, &[], 10, "index", true) == 0);
    check!(lb.selected == 1 && chips(&lb)[1] == "index current");
}

// Link to...'s card: 316 + 140 R tall for R rows (1 to 3 upright, 1 or 2 sideways), at the bottom 20 px above
// the edge or at the top at the line it is given; it docks at the top for an area whose centre is in the lower
// 45% of the screen.
fn link_to_is_laid_out_and_docked() {
    let nb = nb5();
    let mut lb = LinkBar::default();
    let names = ["a", "b", "c", "d", "e"];
    for (named, h_up, h_side) in [(0, 456, 456), (1, 596, 456), (2, 596, 596), (3, 736, 596), (5, 736, 596)] {
        let mut nav = Nav::default();
        nav_init(&mut nav);
        for k in 0..named {
            check!(nav_set_name(&mut nav, nb.page_ids[k], names[k]) == 0);
        }
        check!(linkbar_build_pick(&mut lb, &nb, &nav, &[], 10, "", false) == 0);
        linkbar_layout_pick(&mut lb, 1860, 2480, false, 0);
        check!(lb.rect == Rect { x: 20, y: 2480 - 20 - h_up, w: 1820, h: h_up });
        linkbar_layout_pick(&mut lb, 1860, 2480, true, 173);
        check!(lb.top && lb.rect == Rect { x: 20, y: 173, w: 1820, h: h_up });
        linkbar_layout_pick(&mut lb, 2480, 1860, false, 0);
        check!(lb.rect == Rect { x: 20, y: 1860 - 20 - h_side, w: 2440, h: h_side });
    }
    let mut out = String::new();
    check!(linkbar_probe_pick(&lb, "dock", &mut out, 16) && out == "bottom");
    check!(linkbar_probe_pick(&lb, "rect", &mut out, 32) && out == "20 1244 2440 596");
    // the dock rule: the lower 45%
    check!(!linkbar_pick_docks_top(1363, 2480) && linkbar_pick_docks_top(1364, 2480));
    check!(!linkbar_pick_docks_top(1022, 1860) && linkbar_pick_docks_top(1023, 1860));
    // closed: nothing
    let lb = LinkBar::default();
    check!(linkbar_probe_pick(&lb, "rect", &mut out, 32) && out.is_empty());
    check!(linkbar_probe_pick(&lb, "dock", &mut out, 32) && out.is_empty());
    check!(linkbar_probe_pick(&lb, "1", &mut out, 32) && out.is_empty());
    check!(!linkbar_probe_pick(&lb, "nonsense", &mut out, 32));
}

#[test]
fn test_linkbar() {
    ui_init(300.0);
    run!(links_group_by_target_and_count);
    run!(entries_are_numbered_in_reading_order);
    run!(the_index_and_missing_targets_are_entries);
    run!(the_bar_is_built_from_the_page);
    run!(the_card_is_as_tall_as_its_rows_need);
    run!(link_to_offers_its_chips_in_order);
    run!(link_to_is_laid_out_and_docked);
    done();
}
