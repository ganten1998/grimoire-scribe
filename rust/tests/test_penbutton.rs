use grimoire::prelude::*;

fn hover() -> PbPen { PbPen { prox: true, touching: false } }
fn touch() -> PbPen { PbPen { prox: true, touching: true } }
fn event(p: &mut PenButtons, c: &PbConfig, t_ms: u32, button: PenBtn, down: bool, pen: PbPen) -> PbOut {
    penbutton_event(p, c, PbIn { t_ms, button, down }, pen)
}
fn press(p: &mut PenButtons, c: &PbConfig, t: u32) -> PbOut {
    event(p, c, t, PenBtn::Button, true, hover())
}
fn release(p: &mut PenButtons, c: &PbConfig, t: u32) -> PbOut {
    event(p, c, t, PenBtn::Button, false, hover())
}
fn probe(p: &PenButtons, key: &str) -> String {
    let mut s = String::new(); assert!(penbutton_probe(p, key, &mut s)); s
}

#[test]
fn gap_boundaries_use_release_to_next_press() {
    let c = PbConfig::default();
    for gap in [349, 350, 351] {
        let mut p = PenButtons::default();
        assert_eq!(press(&mut p, &c, 100), PbOut::None);
        assert_eq!(release(&mut p, &c, 180), PbOut::None);
        let old = if gap > 350 { PbOut::Fire(PenBtn::Button, PenKind::Tap) } else { PbOut::None };
        assert_eq!(press(&mut p, &c, 180 + gap), old);
        assert_eq!(release(&mut p, &c, 260 + gap), PbOut::None);
        assert_eq!(penbutton_tick(&mut p, &c, 609 + gap), PbOut::None);
        let kind = if gap > 350 { PenKind::Tap } else { PenKind::Double };
        assert_eq!(penbutton_tick(&mut p, &c, 610 + gap), PbOut::Fire(PenBtn::Button, kind));
        assert_eq!(penbutton_tick(&mut p, &c, 1000 + gap), PbOut::None);
    }
}

#[test]
fn press_at_450_is_a_click_but_451_is_not() {
    for waits in [false, true] {
        let c = PbConfig { waits: [waits; 3], ..Default::default() };
        for duration in [450, 451] {
            let mut p = PenButtons::default();
            assert_eq!(press(&mut p, &c, 100), PbOut::None);
            let click = duration == 450;
            assert_eq!(release(&mut p, &c, 100 + duration),
                if click && !waits { PbOut::Fire(PenBtn::Button, PenKind::Tap) } else { PbOut::None });
            assert_eq!(penbutton_tick(&mut p, &c, 100 + duration + 350),
                if click && waits { PbOut::Fire(PenBtn::Button, PenKind::Tap) } else { PbOut::None });
        }
    }
}

#[test]
fn hold_begins_at_1200_once_and_ends_on_release() {
    let c = PbConfig::default(); let mut p = PenButtons::default();
    assert_eq!(press(&mut p, &c, 100), PbOut::None);
    assert_eq!(penbutton_tick(&mut p, &c, 1299), PbOut::None);
    assert_eq!(penbutton_tick(&mut p, &c, 1300), PbOut::HoldBegin(PenBtn::Button));
    assert_eq!(penbutton_tick(&mut p, &c, 1301), PbOut::None);
    assert_eq!(penbutton_tick(&mut p, &c, 3000), PbOut::None);
    assert_eq!(release(&mut p, &c, 3001), PbOut::HoldEnd(PenBtn::Button));
    assert_eq!(release(&mut p, &c, 3002), PbOut::None);
    assert_eq!(penbutton_tick(&mut p, &c, 4000), PbOut::None);
    // Timing is drained by the caller: a queued release cannot later become a hold.
    assert_eq!(press(&mut p, &c, 5000), PbOut::None);
    assert_eq!(release(&mut p, &c, 6300), PbOut::None);
    assert_eq!(penbutton_tick(&mut p, &c, 6400), PbOut::None);
}

#[test]
fn touching_prevents_holds_and_clicks_even_after_lift() {
    let c = PbConfig::default(); let mut p = PenButtons::default();
    assert_eq!(event(&mut p, &c, 100, PenBtn::Button, true, touch()), PbOut::DragBegin(PenBtn::Button));
    assert_eq!(penbutton_tick(&mut p, &c, 1299), PbOut::None);
    assert_eq!(penbutton_tick(&mut p, &c, 1300), PbOut::None);
    assert_eq!(penbutton_pen(&mut p, &c, hover(), 1400), PbOut::DragEnd(PenBtn::Button));
    assert_eq!(penbutton_tick(&mut p, &c, 3000), PbOut::None);
    assert_eq!(release(&mut p, &c, 3001), PbOut::None);
    assert_eq!(penbutton_tick(&mut p, &c, 4000), PbOut::None);
    assert_eq!(probe(&p, "pen.used"), "1");
}

#[test]
fn drag_ends_on_first_lift_or_button_release_and_never_clicks() {
    // The model has no binding dependency: an unbound drag yields only these edges,
    // leaving side-button ink to the caller.
    let c = PbConfig::default();
    for release_first in [false, true] {
        let mut p = PenButtons::default();
        assert_eq!(press(&mut p, &c, 0), PbOut::None);
        assert_eq!(penbutton_pen(&mut p, &c, touch(), 10), PbOut::DragBegin(PenBtn::Button));
        assert_eq!(penbutton_pen(&mut p, &c, touch(), 11), PbOut::None);
        if release_first {
            assert_eq!(event(&mut p, &c, 20, PenBtn::Button, false, touch()), PbOut::DragEnd(PenBtn::Button));
            assert_eq!(penbutton_pen(&mut p, &c, hover(), 30), PbOut::None);
        } else {
            assert_eq!(penbutton_pen(&mut p, &c, hover(), 20), PbOut::DragEnd(PenBtn::Button));
            assert_eq!(release(&mut p, &c, 30), PbOut::None);
        }
        assert_eq!(penbutton_tick(&mut p, &c, 1200), PbOut::None);
        assert_eq!(probe(&p, "pen.clicks"), "0");
    }
}

#[test]
fn no_wait_taps_fire_on_release_and_buttons_have_independent_timing() {
    let c = PbConfig { waits: [false, true, false], ..Default::default() };
    let mut p = PenButtons::default();
    for t in [0, 100, 200, 300] {
        assert_eq!(press(&mut p, &c, t), PbOut::None);
        assert_eq!(release(&mut p, &c, t + 20), PbOut::Fire(PenBtn::Button, PenKind::Tap));
    }
    for t in [400, 500] {
        assert_eq!(event(&mut p, &c, t, PenBtn::Button2, true, hover()), PbOut::None);
        assert_eq!(event(&mut p, &c, t + 20, PenBtn::Button2, false, hover()), PbOut::None);
    }
    assert_eq!(probe(&p, "pen.Button2.clicks"), "2");
    assert_eq!(probe(&p, "pen.Button.clicks"), "0");
    assert_eq!(penbutton_tick(&mut p, &c, 870), PbOut::Fire(PenBtn::Button2, PenKind::Double));
}

#[test]
fn a_fourth_press_restarts_at_one() {
    let c = PbConfig::default(); let mut p = PenButtons::default();
    for t in [0, 100, 200] {
        assert_eq!(press(&mut p, &c, t), PbOut::None);
        assert_eq!(release(&mut p, &c, t + 20), PbOut::None);
    }
    assert_eq!(probe(&p, "pen.clicks"), "3");
    assert_eq!(press(&mut p, &c, 300), PbOut::None);
    assert_eq!(probe(&p, "pen.clicks"), "0");
    assert_eq!(release(&mut p, &c, 320), PbOut::None);
    assert_eq!(penbutton_tick(&mut p, &c, 669), PbOut::None);
    assert_eq!(penbutton_tick(&mut p, &c, 670), PbOut::Fire(PenBtn::Button, PenKind::Tap));
}

#[test]
fn pen_leave_cancels_counts_and_ends_active_gestures() {
    let c = PbConfig::default(); let mut p = PenButtons::default();
    press(&mut p, &c, 0); release(&mut p, &c, 20);
    assert_eq!(penbutton_pen(&mut p, &c, PbPen::default(), 30), PbOut::None);
    assert_eq!(probe(&p, "pen.clicks"), "0");
    assert_eq!(penbutton_tick(&mut p, &c, 1000), PbOut::None);
    press(&mut p, &c, 2000);
    assert_eq!(penbutton_tick(&mut p, &c, 3200), PbOut::HoldBegin(PenBtn::Button));
    assert_eq!(penbutton_pen(&mut p, &c, PbPen::default(), 3300), PbOut::HoldEnd(PenBtn::Button));
    assert_eq!(release(&mut p, &c, 3400), PbOut::None);
    press(&mut p, &c, 4000);
    assert_eq!(penbutton_pen(&mut p, &c, touch(), 4010), PbOut::DragBegin(PenBtn::Button));
    assert_eq!(event(&mut p, &c, 4020, PenBtn::Button, false, PbPen::default()), PbOut::DragEnd(PenBtn::Button));
    assert_eq!(penbutton_pen(&mut p, &c, PbPen::default(), 4020), PbOut::None);
    assert_eq!(penbutton_tick(&mut p, &c, 6000), PbOut::None);
}

#[test]
fn multiple_held_buttons_drain_on_touch_lift_tick_and_leave() {
    let c = PbConfig::default(); let buttons = [PenBtn::Button, PenBtn::Button2];
    for leave in [false, true] {
        let mut p = PenButtons::default();
        for button in buttons { assert_eq!(event(&mut p, &c, 0, button, true, hover()), PbOut::None); }
        for button in buttons { assert_eq!(penbutton_pen(&mut p, &c, touch(), 10), PbOut::DragBegin(button)); }
        assert_eq!(penbutton_pen(&mut p, &c, touch(), 10), PbOut::None);
        let pen = if leave { PbPen::default() } else { hover() };
        for button in buttons { assert_eq!(penbutton_pen(&mut p, &c, pen, 20), PbOut::DragEnd(button)); }
        assert_eq!(penbutton_pen(&mut p, &c, pen, 20), PbOut::None);
        for button in buttons { assert_eq!(event(&mut p, &c, 30, button, false, pen), PbOut::None); }
        assert_eq!(penbutton_tick(&mut p, &c, 2000), PbOut::None);
    }
    let mut p = PenButtons::default();
    for button in buttons { event(&mut p, &c, 0, button, true, hover()); }
    for button in buttons { assert_eq!(penbutton_tick(&mut p, &c, 1200), PbOut::HoldBegin(button)); }
    assert_eq!(penbutton_tick(&mut p, &c, 1200), PbOut::None);
    for button in buttons { assert_eq!(penbutton_pen(&mut p, &c, PbPen::default(), 1210), PbOut::HoldEnd(button)); }
    assert_eq!(penbutton_pen(&mut p, &c, PbPen::default(), 1210), PbOut::None);
    for button in buttons { event(&mut p, &c, 2000, button, true, hover()); event(&mut p, &c, 2020, button, false, hover()); }
    for button in buttons { assert_eq!(penbutton_tick(&mut p, &c, 2370), PbOut::Fire(button, PenKind::Tap)); }
    assert_eq!(penbutton_tick(&mut p, &c, 2370), PbOut::None);
}

#[test]
fn timestamps_wrap_for_click_gaps_presses_and_holds() {
    let c = PbConfig::default(); let start = u32::MAX - 100;
    let mut p = PenButtons::default();
    press(&mut p, &c, start); release(&mut p, &c, start.wrapping_add(20));
    assert_eq!(press(&mut p, &c, start.wrapping_add(369)), PbOut::None);
    release(&mut p, &c, start.wrapping_add(389));
    assert_eq!(penbutton_tick(&mut p, &c, start.wrapping_add(738)), PbOut::None);
    assert_eq!(penbutton_tick(&mut p, &c, start.wrapping_add(739)), PbOut::Fire(PenBtn::Button, PenKind::Double));
    let mut p = PenButtons::default(); press(&mut p, &c, start);
    assert_eq!(penbutton_tick(&mut p, &c, start.wrapping_add(1199)), PbOut::None);
    assert_eq!(penbutton_tick(&mut p, &c, start.wrapping_add(1200)), PbOut::HoldBegin(PenBtn::Button));
    assert_eq!(release(&mut p, &c, start.wrapping_add(1201)), PbOut::HoldEnd(PenBtn::Button));
    let c = PbConfig { waits: [false; 3], ..Default::default() };
    let mut p = PenButtons::default(); press(&mut p, &c, start);
    assert_eq!(release(&mut p, &c, start.wrapping_add(450)), PbOut::Fire(PenBtn::Button, PenKind::Tap));
    let mut p = PenButtons::default(); press(&mut p, &c, start);
    assert_eq!(release(&mut p, &c, start.wrapping_add(451)), PbOut::None);
}

#[test]
fn duplicate_edges_do_not_restart_a_press_or_add_a_click() {
    let c = PbConfig::default(); let mut p = PenButtons::default();
    assert_eq!(release(&mut p, &c, 0), PbOut::None);
    press(&mut p, &c, 10);
    assert_eq!(press(&mut p, &c, 1000), PbOut::None);
    assert_eq!(penbutton_tick(&mut p, &c, 1210), PbOut::HoldBegin(PenBtn::Button));
    assert_eq!(release(&mut p, &c, 1220), PbOut::HoldEnd(PenBtn::Button));
    assert_eq!(release(&mut p, &c, 1230), PbOut::None);
    let mut out = String::from("untouched");
    assert!(!penbutton_probe(&p, "pen.unknown", &mut out));
    assert_eq!(out, "untouched");
}

#[test]
fn a_wall_tick_before_a_queued_edge_does_not_expire_it() {
    for start in [1000u32, u32::MAX - 100] {
        let c = PbConfig::default();
        let mut p = PenButtons::default();
        press(&mut p, &c, start);
        assert_eq!(penbutton_tick(&mut p, &c, start.wrapping_sub(1)), PbOut::None);
        release(&mut p, &c, start.wrapping_add(80));
        assert_eq!(penbutton_tick(&mut p, &c, start.wrapping_add(40)), PbOut::None);
        assert_eq!(penbutton_tick(&mut p, &c, start.wrapping_add(429)), PbOut::None);
        assert_eq!(penbutton_tick(&mut p, &c, start.wrapping_add(430)),
            PbOut::Fire(PenBtn::Button, PenKind::Tap));
    }
}
