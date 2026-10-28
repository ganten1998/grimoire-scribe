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
    NavLink { pid: 10, seed, target: FixedStr::from_str(target) }
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

#[test]
fn test_linkbar() {
    ui_init(300.0);
    run!(links_group_by_target_and_count);
    run!(entries_are_numbered_in_reading_order);
    run!(the_index_and_missing_targets_are_entries);
    done();
}
