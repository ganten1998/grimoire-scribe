// test_panel.c: the editor's floating panel (src/panel.h). With 0 to 4 pens pinned it fits the
// 2022/2024 Scribes' screen and the Scribe 3's and Colorsoft's, upright and sideways, keeping its
// 12 px margins and its buttons at full size; the collapsed square never changes. Next and previous
// tool go through its pens alone, and the tool picker's next and previous through the picker's nine.
// Index loops and float comparisons are written as the C test writes them.
#![allow(clippy::needless_range_loop, clippy::float_cmp)]
mod testlib;

use grimoire::prelude::*;
use testlib::*;

fn fits_both_scribes_both_ways() {
    let screens: [[i32; 2]; 2] = [[1860, 2480], [1986, 2648]];
    for s in 0..2 {
        for turned in 0..2 {
            let w = if turned != 0 { screens[s][1] } else { screens[s][0] };
            let h = if turned != 0 { screens[s][0] } else { screens[s][1] };
            for pins in 0..=PANEL_PINS_MAX as i32 {
                let f = panel_fit(h, pins);
                let ph = panel_height(f, pins);
                // Inside the screen with 12 px free above and below, as wide as ever.
                check!(ph + 2 * ui_px(PANEL_MARGIN as f32) <= h && panel_w() <= w);
                // The buttons stay as big as without pins (and sideways as upright): only the
                // room between them gives.
                check!(panel_btn_h(f) == ui_px(100.0) && panel_head_h(f) == ui_px(80.0) && panel_label_h(f) == ui_px(60.0));
                check!(f.scale == 1.0 && f.space >= 0.5);
                if pins == 0 {
                    check!(f.space == 1.0 && ph == ui_px(1410.0)); // as before pins existed
                }
            }
        }
    }
    // The only one that needs the room between the buttons: four pins on the 2022/2024 Scribe
    // sideways (1842 px at full size, 1836 px of room).
    let f = panel_fit(1860, 4);
    check!(panel_height(PanelFit { scale: 1.0, space: 1.0 }, 4) == ui_px(1842.0) && f.space < 1.0 && f.scale == 1.0);
    check!(panel_height(f, 4) <= 1860 - 24 && panel_gap(f) >= ui_px(4.0) && panel_div_h(f) >= ui_px(10.0));
    check!(panel_fit(1860, 3).space == 1.0);
    check!(panel_box() == ui_px(120.0)); // collapsed: the same square, pins or not
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

#[test]
fn test_panel() {
    ui_init(300.0);
    run!(fits_both_scribes_both_ways);
    run!(shrinks_to_fit_a_short_screen);
    run!(next_tool_walks_the_panels_pens);
    run!(picker_step_walks_the_nine);
    done();
}
