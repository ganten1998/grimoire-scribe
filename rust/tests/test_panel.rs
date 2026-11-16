// The editor's floating panel. With 0 to 3 pens pinned it fits the
// 2022/2024 Scribes' screen and the Scribe 3's and Colorsoft's, upright and sideways, keeping its
// 12 px margins (93 px buttons with three pins on older Scribes sideways, otherwise full size).
// The collapsed square never changes. Next and previous
// tool go through its pens alone, and the tool picker's next and previous through the picker's nine.
// Index loops and float comparisons are written as the C test writes them.
#![allow(clippy::needless_range_loop, clippy::float_cmp)]
mod testlib;

use grimoire::prelude::*;
use testlib::*;

fn fits_both_scribes_both_ways() {
    check!(PANEL_BUTTONS == 14);
    check!(PANEL_PINS_MAX == 3);
    for (w,h) in [(1860,2480),(2480,1860),(1986,2648),(2648,1986)] {
        for pins in 0..=PANEL_PINS_MAX as i32 {
            let f = panel_fit(h,pins);
            check!(panel_height(f,pins)+2*ui_px(PANEL_MARGIN as f32)<=h);
            check!(panel_w()+2*ui_px(PANEL_MARGIN as f32)<=w && f.scale >= 0.93 && f.space >= 0.5);
            check!(panel_btn_h(f)>=ui_px(93.0));
            // Two or three pins on the older Scribes in landscape need smaller buttons.
            check!(panel_btn_h(f)==ui_px(if h==1860 && pins==3 {93.0}else if h==1860 && pins==2 {99.0}else{100.0}));
            if h>=2480 {check!(f.scale==1.0 && f.space==1.0);}
        }
    }
    check!(panel_height(PanelFit{scale:1.0,space:1.0},0)==1734);
    check!(panel_box()==ui_px(120.0));
}

fn three_pins_roundtrip_and_a_fourth_is_refused() {
    let root = std::path::PathBuf::from(tl_tmp()).join(format!("grimoire-panel-pins-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    check!(store_init(root.to_str().unwrap()) == 0);
    let mut s = Settings::default();
    settings_default(&mut s);
    for t in [TOOL_HIGHLIGHTER, TOOL_FINELINER, TOOL_MARKER] {
        check!(settings_toggle_pin(&mut s, t));
    }
    check!(!settings_toggle_pin(&mut s, TOOL_CALLIGRAPHY));
    let mut text = String::new();
    settings_pins_text(&s, &mut text, 128);
    check!(text == "highlighter,fineliner,marker" && s.npins == 3);
    check!(settings_save(&s) == 0);
    let mut restored = Settings::default();
    check!(settings_load(&mut restored) == 0);
    settings_pins_text(&restored, &mut text, 128);
    check!(text == "highlighter,fineliner,marker" && restored.npins == 3);
    check!(!settings_pinned(&restored, TOOL_CALLIGRAPHY));
    // A replacement keeps the remaining pins' order and survives another restart.
    check!(settings_toggle_pin(&mut restored, TOOL_FINELINER));
    check!(settings_toggle_pin(&mut restored, TOOL_CALLIGRAPHY));
    check!(settings_save(&restored) == 0 && settings_load(&mut s) == 0);
    settings_pins_text(&s, &mut text, 128);
    check!(text == "highlighter,marker,calligraphy");
    std::fs::remove_dir_all(root).unwrap();
}

// A screen far shorter than any Scribe's: the room between the buttons goes down to half, then
// everything shrinks, and the panel still keeps its margins (the measures round to whole pixels).
fn shrinks_to_fit_a_short_screen() {
    let mut h = 900;
    while h <= 1900 {
        for pins in 0..=PANEL_PINS_MAX as i32 {
            let f = panel_fit(h, pins);
            check!(panel_height(f, pins) + 2 * ui_px(PANEL_MARGIN as f32) <= h);
            check!(f.scale == 1.0 || f.space == 0.5); // the buttons give only after the room
            check!(panel_btn_h(f) >= ui_px(40.0));
        }
        h += 37;
    }
}

// Pressed again and again from s's pen in use, each step taken as the editor takes it (the pen it
// gives is the one in use, and goes to the tool button unless it is pinned): the n pens it goes
// through, into out.
fn walk(mut s: Settings, step: fn(&Settings, bool) -> ToolKind, next: bool, out: &mut [ToolKind], n: usize) {
    for i in 0..n {
        let t = step(&s, next);
        s.current_tool = t;
        out[i] = t;
        if t != TOOL_ERASER && !settings_pinned(&s, t) {
            s.draw_tool = t;
        }
    }
}

// Two pens or more on the panel: next and previous tool go through them alone, top to bottom (the
// tool button's, then the pins in pin order) and round again; the eraser keeps the place of the pen
// it goes back to. Fewer than two: the tool picker's next or previous pen, which takes the tool
// button, so with none pinned its nine, as before pins existed.
fn next_tool_walks_the_panels_pens() {
    let mut s = Settings::default();
    settings_default(&mut s);
    s.draw_tool = TOOL_FOUNTAIN;
    s.pins[0] = TOOL_HIGHLIGHTER;
    s.pins[1] = TOOL_FINELINER;
    s.npins = 2;
    s.current_tool = TOOL_FOUNTAIN;
    s.before_eraser = -1;
    let mut pens = [ToolKind::default(); PANEL_PINS_MAX + 1];
    let mut got = [ToolKind::default(); 6];
    check!(panel_pens(&s, &mut pens) == 3 && pens[0] == TOOL_FOUNTAIN && pens[1] == TOOL_HIGHLIGHTER && pens[2] == TOOL_FINELINER);
    walk(s, panel_step, true, &mut got, 6);
    check!(got[0] == TOOL_HIGHLIGHTER && got[1] == TOOL_FINELINER && got[2] == TOOL_FOUNTAIN);
    check!(got[3] == TOOL_HIGHLIGHTER && got[4] == TOOL_FINELINER && got[5] == TOOL_FOUNTAIN);
    walk(s, panel_step, false, &mut got, 3);
    check!(got[0] == TOOL_FINELINER && got[1] == TOOL_HIGHLIGHTER && got[2] == TOOL_FOUNTAIN);
    // From the eraser: the pen it goes back to (a pin), or the tool button's once that is off the panel.
    s.current_tool = TOOL_ERASER;
    s.before_eraser = TOOL_HIGHLIGHTER.0;
    check!(panel_step(&s, true) == TOOL_FINELINER && panel_step(&s, false) == TOOL_FOUNTAIN);
    s.before_eraser = TOOL_MARKER.0;
    check!(panel_step(&s, true) == TOOL_HIGHLIGHTER && panel_step(&s, false) == TOOL_FINELINER);
    // The tool button's pen pinned (the button shows a grid): the pins alone, its pin in its place.
    s.draw_tool = TOOL_FINELINER;
    s.current_tool = TOOL_FINELINER;
    check!(panel_pens(&s, &mut pens) == 2 && pens[0] == TOOL_HIGHLIGHTER && pens[1] == TOOL_FINELINER);
    check!(panel_step(&s, true) == TOOL_HIGHLIGHTER && panel_step(&s, false) == TOOL_HIGHLIGHTER);
    // A pen in use that is not on the panel (settings.ini changed by hand): the first, or the last.
    s.current_tool = TOOL_CHARCOAL;
    check!(panel_step(&s, true) == TOOL_HIGHLIGHTER && panel_step(&s, false) == TOOL_FINELINER);
    // One pen on the panel, the tool button's own pinned (the button a grid): the first press takes
    // the picker's next pen (pencil, mechanical, charcoal, ballpoint, fineliner, marker, fountain,
    // calligraphy, highlighter) to the tool button, and then the panel holds two, gone between.
    s.npins = 1; // (the highlighter unpinned: the fineliner's pin alone)
    s.pins[0] = TOOL_FINELINER;
    s.current_tool = TOOL_FINELINER;
    check!(panel_pens(&s, &mut pens) == 1 && pens[0] == TOOL_FINELINER);
    walk(s, panel_step, true, &mut got, 3);
    check!(got[0] == TOOL_MARKER && got[1] == TOOL_FINELINER && got[2] == TOOL_MARKER);
    // None pinned: the picker's nine, each on the tool button, round from the highlighter to the
    // pencil and back.
    s.npins = 0;
    check!(panel_pens(&s, &mut pens) == 1 && pens[0] == TOOL_FINELINER);
    let mut nine = [ToolKind::default(); TOOL_PENS];
    walk(s, panel_step, true, &mut nine, TOOL_PENS);
    for i in 0..TOOL_PENS {
        check!(nine[i] == TOOL_ORDER[(5 + i) % TOOL_PENS]);
    }
    walk(s, panel_step, false, &mut nine, TOOL_PENS);
    for i in 0..TOOL_PENS {
        check!(nine[i] == TOOL_ORDER[(3 + TOOL_PENS - i) % TOOL_PENS]);
    }
    s.current_tool = TOOL_PENCIL;
    check!(panel_step(&s, false) == TOOL_HIGHLIGHTER && panel_step(&s, true) == TOOL_MECH_PENCIL);
    s.current_tool = TOOL_ERASER;
    s.before_eraser = TOOL_MARKER.0; // (off the panel: from the tool button's fineliner)
    check!(panel_step(&s, true) == TOOL_MARKER && panel_step(&s, false) == TOOL_BALLPOINT);
}

// The picker's next and previous: its nine in its order whatever the panel holds, round again, and
// from the eraser the pen it goes back to.
fn picker_step_walks_the_nine() {
    let mut s = Settings::default();
    settings_default(&mut s);
    s.draw_tool = TOOL_FOUNTAIN;
    s.pins[0] = TOOL_HIGHLIGHTER;
    s.pins[1] = TOOL_FINELINER;
    s.npins = 2;
    s.current_tool = TOOL_FOUNTAIN;
    s.before_eraser = -1;
    let mut nine = [ToolKind::default(); TOOL_PENS];
    walk(s, picker_step, true, &mut nine, TOOL_PENS);
    for i in 0..TOOL_PENS {
        check!(nine[i] == TOOL_ORDER[(7 + i) % TOOL_PENS]);
    }
    walk(s, picker_step, false, &mut nine, TOOL_PENS);
    for i in 0..TOOL_PENS {
        check!(nine[i] == TOOL_ORDER[(5 + TOOL_PENS - i) % TOOL_PENS]);
    }
    s.current_tool = TOOL_ERASER;
    s.before_eraser = TOOL_FINELINER.0;
    check!(picker_step(&s, true) == TOOL_MARKER && picker_step(&s, false) == TOOL_BALLPOINT);
}

/// Whether every label "p / m", p from 1 to m, fits the page chip at px (as the chip draws it: Bold, within
/// panel_chip_w), measured from p = m down; with the first page whose label does not fit (0 when all do). A count
/// whose labels all fit has every one measured.
fn every_label_fits(m: i32, px: f32) -> (bool, i32) {
    for p in (1..=m).rev() {
        if font_measure(FontFace::Bold, px, &format!("{p} / {m}")) > panel_chip_w() {
            return (false, p);
        }
    }
    (true, 0)
}

// The page chip's text (#119) keeps one form in a notebook: the page and the count on every page when every
// page's label fits the chip, else the page alone on every page, and it is never cut short. In Gothic and in
// Classic with each font of Settings > Font, at each size the panel draws the chip at (both Scribes' screens,
// upright and sideways, with 0 to 3 pens pinned: 34, 33.66 and 31.62 px), for every page count from 1 to 9999.
// Whether a count's labels all fit is found here by measuring them (every_label_fits). The chip is read on every
// page so measured (every page of a count whose labels all fit; the last pages down to the first cut short of
// one whose labels do not), on pages 1 to 9, and on every page of the counts up to 250 and of 990 to 1010, 1990
// to 2000 and 9990 to 9999. Every page alone, 1 to 9999, fits. Digits only: no language changes it.
fn the_page_chip_keeps_one_form_in_a_notebook() {
    check!(font_init() == 0);
    let styles = [("Gothic", UiTheme::Gothic, 0), ("Terminess", UiTheme::Classic, 0), ("Atkinson", UiTheme::Classic, 1), ("Noto Sans", UiTheme::Classic, 2), ("Literata", UiTheme::Classic, 3)];
    check!(styles.len() == 1 + FONT_FAMILIES.len());
    // the sizes, with a panel_fit for each
    let mut fits: Vec<PanelFit> = Vec::new();
    for h in [2480, 1860, 2648, 1986] {
        for pins in 0..=PANEL_PINS_MAX as i32 {
            let f = panel_fit(h, pins);
            if !fits.iter().any(|g| panel_chip_px(*g) == panel_chip_px(f)) {
                fits.push(f);
            }
        }
    }
    let mut sizes: Vec<f32> = fits.iter().map(|f| panel_chip_px(*f)).collect();
    sizes.sort_by(|a, b| b.total_cmp(a));
    check!(sizes == [34.0, 33.66, 31.62]);
    let every_page = |m: i32| m <= 250 || (990..=1010).contains(&m) || (1990..=2000).contains(&m) || m >= 9990;
    for (name, theme, family) in styles {
        ui_set_theme(theme);
        font_set_family(family);
        for &f in &fits {
            let px = panel_chip_px(f);
            let (mut wrong, mut whole, mut small, mut big, mut read) = (0, 0, 0, 0, 0u64);
            let alone_cut = (1..=9999).filter(|p| font_measure(FontFace::Bold, px, &p.to_string()) > panel_chip_w()).count();
            for m in 1..=9999 {
                let (all, cut) = every_label_fits(m, px);
                whole += all as i32;
                small += (all && m < 100) as i32;
                big += (all && m >= 1000) as i32;
                let first = if every_page(m) || all { 1 } else { cut };
                for p in (first..=m).chain(1..=m.min(9)) {
                    let lab = panel_page_label(p, m, f);
                    let want = if all { format!("{p} / {m}") } else { p.to_string() };
                    read += 1;
                    if lab != want {
                        if wrong < 5 {
                            eprintln!("  {name} at {px} px: page {p} of {m} reads \"{lab}\", not \"{want}\"");
                        }
                        wrong += 1;
                    }
                }
            }
            eprintln!("  {name} at {px} px: {whole} counts show the page and the count, {read} pages read, {wrong} wrong");
            check!(wrong == 0);
            check!(alone_cut == 0);
            // every count under 100 shows the page and the count, as before #119; none of 1000 or more does
            check!(small == 99 && big == 0);
            check!(panel_page_label(1, 1, f) == "1 / 1" && panel_page_label(99, 99, f) == "99 / 99");
            check!(panel_page_label(1, 1000, f) == "1" && panel_page_label(1995, 2000, f) == "1995");
            check!(panel_page_label(1, 2000, f) == "1" && panel_page_label(9999, 9999, f) == "9999");
        }
    }
    // Found again when what it was found for changes: the page count, the font (Gothic or Classic, Classic's
    // family) and the size (the screen turned, pens pinned), each in turn with the others kept. In Gothic
    // "100 / 100" is cut, while every label of 111 pages fits; in Noto Sans upright "100 / 150" is cut, while
    // sideways on the 2022/2024 Scribes' screen with 3 pens (31.62 px) every count under 1000 fits; in Literata
    // every label of 150 pages fits at full size.
    let (up, side3) = (panel_fit(2480, 0), panel_fit(1860, 3));
    ui_set_theme(UiTheme::Gothic);
    font_set_family(0);
    check!(panel_page_label(1, 111, up) == "1 / 111");
    check!(panel_page_label(1, 112, up) == "1");
    check!(panel_page_label(1, 111, up) == "1 / 111");
    check!(panel_page_label(100, 100, up) == "100" && panel_page_label(99, 99, up) == "99 / 99");
    ui_set_theme(UiTheme::Classic);
    check!(panel_page_label(1, 111, up) == "1");
    ui_set_theme(UiTheme::Gothic);
    check!(panel_page_label(1, 111, up) == "1 / 111");
    ui_set_theme(UiTheme::Classic);
    font_set_family(2);
    check!(panel_page_label(1, 150, up) == "1");
    check!(panel_page_label(1, 150, side3) == "1 / 150" && panel_page_label(100, 100, side3) == "100 / 100");
    check!(panel_page_label(1, 150, up) == "1");
    font_set_family(3);
    check!(panel_page_label(1, 150, up) == "1 / 150");
    font_set_family(2);
    check!(panel_page_label(1, 150, up) == "1");
    check!(panel_chip_w() == ui_px(112.0));
    font_set_family(0);
    ui_set_theme(UiTheme::Gothic);
}

#[test]
fn test_panel() {
    ui_init(300.0);
    run!(fits_both_scribes_both_ways);
    run!(the_page_chip_keeps_one_form_in_a_notebook);
    run!(shrinks_to_fit_a_short_screen);
    run!(three_pins_roundtrip_and_a_fourth_is_refused);
    run!(next_tool_walks_the_panels_pens);
    run!(picker_step_walks_the_nine);
    done();
}
