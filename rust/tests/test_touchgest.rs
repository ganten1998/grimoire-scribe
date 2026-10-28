// test_touchgest: the touch recognizer (touchgest.rs), fed contact frames as input_evdev.rs and the
// runner build them: every contact down at one SYN_REPORT, in slot order, at the frame's kernel time.
// First what the app had before it existed (one finger's stream, the pinch, the palm rules), then the
// fixes the extraction carries (cancels instead of lifts, landing order, kernel time).
mod testlib;
use grimoire::prelude::*;
use testlib::*;

struct Rig {
    g: TouchGest,
    pen: TgPen,
    k: u32,
    c: Vec<TgContact>,
}

impl Rig {
    fn new() -> Rig {
        let mut g = TouchGest::default();
        touchgest_init(&mut g, 300.0);
        touchgest_set_view(&mut g, TgView { zoom_ok: true, gestures_on: true, ..Default::default() });
        Rig { g, pen: TgPen::default(), k: 100_000, c: Vec::new() }
    }
    /// A contact in slot id (its tracking id 100 + id), tool and touch_major given.
    fn put(&mut self, id: i32, x: f32, y: f32, tool: u8, major: i32) {
        let c = TgContact { id: 100 + id, slot: id, x, y, nx: x / 1860.0, ny: y / 2480.0, major, tool, pressure: 0 };
        match self.c.iter_mut().find(|o| o.slot == id) {
            Some(o) => *o = c,
            None => self.c.push(c),
        }
    }
    fn down(&mut self, id: i32, x: f32, y: f32) {
        self.put(id, x, y, 0, 0);
    }
    fn mv(&mut self, id: i32, x: f32, y: f32) {
        let (tool, major) = self.c.iter().find(|o| o.slot == id).map(|o| (o.tool, o.major)).unwrap();
        self.put(id, x, y, tool, major);
    }
    fn up(&mut self, id: i32) {
        self.c.retain(|o| o.slot != id);
    }
    /// The frame at the current time, then the time moves on by dt: its events.
    fn frame(&mut self, dt: u32) -> Vec<InEvent> {
        let mut f = TgFrame { k_ms: self.k, ..Default::default() };
        let mut c = self.c.clone();
        c.sort_by_key(|o| o.slot);
        for o in c {
            f.c[f.n] = o;
            f.n += 1;
        }
        let mut out = TgOut::default();
        touchgest_frame(&mut self.g, &f, &self.pen, &mut out);
        self.k = self.k.wrapping_add(dt);
        out.ev[..out.n].to_vec()
    }
    fn cancel(&mut self) -> Vec<InEvent> {
        let mut out = TgOut::default();
        touchgest_cancel(&mut self.g, self.k, &mut out);
        out.ev[..out.n].to_vec()
    }
    fn probe(&self, key: &str) -> String {
        let mut s = String::new();
        check!(touchgest_probe(&self.g, key, &mut s));
        s
    }
}

fn types(ev: &[InEvent]) -> Vec<InType> {
    ev.iter().map(|e| e.type_).collect()
}
fn is_cancel(e: &InEvent) -> bool {
    e.type_ == InType::TouchUp && e.x == -10000.0 && e.y == -10000.0 && e.value & TOUCH_F_CANCEL != 0
}
fn same_place(e: &InEvent, x: f32, y: f32) -> bool {
    e.x.to_bits() == x.to_bits()
        && e.y.to_bits() == y.to_bits()
        && e.nx.to_bits() == (x / 1860.0).to_bits()
        && e.ny.to_bits() == (y / 2480.0).to_bits()
}

// ---- what the app had ----

// One finger: TouchDown at once, TouchMove only when the place changes, TouchUp where it last was,
// every value exactly the reader's (map()'s f32 and the normalized place), timed by the frame.
fn one_finger_is_the_stream_it_was() {
    let mut r = Rig::new();
    r.down(0, 500.25, 700.5);
    let k0 = r.k;
    let ev = r.frame(13);
    check!(types(&ev) == [InType::TouchDown]);
    check!(same_place(&ev[0], 500.25, 700.5) && ev[0].value == 0 && ev[0].k_ms == k0 && ev[0].t_ms == k0);
    check!(r.probe("touch.class") == "one" && r.probe("touch.contacts") == "1");
    r.mv(0, 510.75, 690.125);
    let ev = r.frame(13);
    check!(types(&ev) == [InType::TouchMove] && same_place(&ev[0], 510.75, 690.125));
    check!(r.frame(13).is_empty()); // the same place: nothing
    r.up(0);
    let ev = r.frame(13);
    check!(types(&ev) == [InType::TouchUp] && same_place(&ev[0], 510.75, 690.125) && ev[0].value == 0);
    check!(r.probe("touch.class") == "none" && r.probe("touch.contacts") == "0" && !touchgest_busy(&r.g));
}

// A contact the panel calls a palm (tool 2) cancels the finger and suppresses every contact until all
// have lifted; so does the finger itself turning into one.
fn a_palm_cancels_and_suppresses() {
    let mut r = Rig::new();
    r.down(0, 900.0, 1200.0);
    check!(types(&r.frame(13)) == [InType::TouchDown]);
    r.put(1, 300.0, 300.0, 2, 0);
    let ev = r.frame(13);
    check!(ev.len() == 1 && is_cancel(&ev[0]));
    check!(r.probe("touch.suppressed") == "1" && r.probe("touch.class") == "suppressed");
    r.down(2, 1500.0, 2000.0);
    check!(r.frame(13).is_empty());
    r.up(2);
    r.up(0);
    check!(r.frame(13).is_empty() && r.probe("touch.suppressed") == "1");
    r.up(1);
    check!(r.frame(13).is_empty() && r.probe("touch.suppressed") == "0");
    // The finger itself flagged a palm.
    r.k += 1000;
    r.down(0, 900.0, 1200.0);
    check!(types(&r.frame(13)) == [InType::TouchDown]);
    r.put(0, 905.0, 1200.0, 2, 0);
    let ev = r.frame(13);
    check!(ev.len() == 1 && is_cancel(&ev[0]));
    r.up(0);
    check!(r.frame(13).is_empty() && r.probe("touch.class") == "none");
}

// A second finger soon after the first: the first ends with the cancel, then PinchBegin (value 1)
// with the fingers in the order they landed.
fn a_second_finger_at_120_ms_pinches() {
    let mut r = Rig::new();
    r.down(0, 800.0, 1200.0);
    check!(types(&r.frame(120)) == [InType::TouchDown]);
    r.down(1, 1100.0, 1250.0);
    let ev = r.frame(13);
    check!(types(&ev) == [InType::TouchUp, InType::PinchBegin]);
    check!(is_cancel(&ev[0]) && ev[1].value == 1);
    check!(ev[1].x == 800.0 && ev[1].y == 1200.0 && ev[1].x2 == 1100.0 && ev[1].y2 == 1250.0);
    check!(r.probe("touch.class") == "landing");
    r.up(0);
    check!(types(&r.frame(13)) == [InType::PinchEnd]);
    r.up(1);
    check!(r.frame(13).is_empty() && r.probe("touch.class") == "none");
}

// Later than 400 ms it is a palm or a second hand: the first finger's cancel, and nothing until all
// have lifted.
fn a_second_finger_at_450_ms_is_a_palm() {
    let mut r = Rig::new();
    r.down(0, 800.0, 1200.0);
    check!(types(&r.frame(450)) == [InType::TouchDown]);
    r.down(1, 1100.0, 1250.0);
    let ev = r.frame(13);
    check!(ev.len() == 1 && is_cancel(&ev[0]) && r.probe("touch.suppressed") == "1");
    r.mv(1, 1300.0, 1250.0);
    check!(r.frame(13).is_empty());
    r.up(0);
    check!(r.frame(13).is_empty());
    r.up(1);
    check!(r.frame(13).is_empty());
    r.down(0, 800.0, 1200.0);
    check!(types(&r.frame(13)) == [InType::TouchDown]);
}

// While the pen is near, and for a second after it leaves, a finger is the writing hand: it gives
// nothing, and one already down ends with the cancel, never with a lift in its place.
fn the_pen_window_holds_one_finger() {
    let mut r = Rig::new();
    r.pen.prox = true;
    r.down(0, 900.0, 1200.0);
    check!(r.frame(13).is_empty() && r.probe("touch.suppressed") == "1");
    r.pen.prox = false;
    r.pen.left_ms = r.k;
    r.k += 2000;
    r.mv(0, 950.0, 1200.0);
    check!(r.frame(13).is_empty()); // suppressed until it lifts, as before
    r.up(0);
    check!(r.frame(13).is_empty());
    // Within the second after the leave.
    r.pen.left_ms = r.k;
    r.k += 999;
    r.down(0, 900.0, 1200.0);
    check!(r.frame(13).is_empty());
    r.up(0);
    check!(r.frame(13).is_empty());
    r.k += 1000;
    r.down(0, 900.0, 1200.0);
    check!(types(&r.frame(13)) == [InType::TouchDown]);
    // The pen comes near while the finger is down (seen at a touch frame): the cancel.
    r.pen.prox = true;
    r.mv(0, 905.0, 1200.0);
    let ev = r.frame(13);
    check!(ev.len() == 1 && is_cancel(&ev[0]));
    r.up(0);
    check!(r.frame(13).is_empty());
    // A frame older than the leave (read after it) still counts the pen near.
    r.pen.prox = false;
    r.pen.left_ms = r.k + 50;
    r.down(0, 900.0, 1200.0);
    check!(r.frame(13).is_empty());
    r.up(0);
    r.frame(13);
}

// PinchBegin and PinchMove always say they are classified: 0 is the runner's legacy pinch alone.
fn value_0_is_never_a_classified_pinch() {
    for zoom_ok in [true, false] {
        let mut r = Rig::new();
        touchgest_set_view(&mut r.g, TgView { zoom_ok, gestures_on: true, ..Default::default() });
        r.down(0, 800.0, 1200.0);
        r.down(1, 1100.0, 1200.0);
        let mut all = r.frame(20);
        for i in 1..=8 {
            r.mv(0, 800.0 - 40.0 * i as f32, 1200.0);
            r.mv(1, 1100.0 + 40.0 * i as f32, 1200.0);
            all.extend(r.frame(20));
        }
        r.up(0);
        r.up(1);
        all.extend(r.frame(20));
        let begins = all.iter().filter(|e| e.type_ == InType::PinchBegin && e.value == 1).count();
        let moves = all.iter().filter(|e| e.type_ == InType::PinchMove).count();
        check!(begins == 1 && moves >= 6);
        check!(all.iter().all(|e| e.type_ != InType::PinchMove || e.value == if zoom_ok { 1 } else { 2 }));
        check!(all.iter().filter(|e| e.type_ == InType::PinchEnd).count() == 1);
    }
}

// Nothing moves until the fingers clearly spread or travel (the editor's dead zone: a tenth of the
// spread, or 70 px of midpoint at 300 dpi); then each frame brings a PinchMove.
fn a_pinch_commits_past_the_dead_zone() {
    let mut r = Rig::new();
    r.down(0, 800.0, 1200.0);
    r.down(1, 1100.0, 1200.0);
    check!(types(&r.frame(20)) == [InType::PinchBegin]);
    r.mv(1, 1128.0, 1200.0); // 9% wider, 14 px of midpoint
    check!(r.frame(20).is_empty() && r.probe("touch.class") == "landing");
    r.mv(1, 1131.0, 1200.0); // 10.3%
    let ev = r.frame(20);
    check!(types(&ev) == [InType::PinchMove] && ev[0].value == 1 && ev[0].x2 == 1131.0);
    check!(r.probe("touch.class") == "zoom");
    check!(types(&r.frame(20)) == [InType::PinchMove]); // one a frame, as before
    // Travel alone commits too.
    let mut r = Rig::new();
    touchgest_set_view(&mut r.g, TgView { zoom_ok: true, ..Default::default() });
    r.down(0, 800.0, 1200.0);
    r.down(1, 1100.0, 1200.0);
    r.frame(20);
    r.mv(0, 800.0, 1269.0);
    r.mv(1, 1100.0, 1269.0);
    check!(r.frame(20).is_empty());
    r.mv(0, 800.0, 1271.0);
    r.mv(1, 1100.0, 1271.0);
    let ev = r.frame(20);
    check!(types(&ev) == [InType::PinchMove] && ev[0].value == 2 && r.probe("touch.class") == "pan");
}

// A third finger ends a session that has not committed (three), and is left out of a zoom.
fn a_third_finger() {
    let mut r = Rig::new();
    r.down(0, 800.0, 1200.0);
    r.down(1, 1100.0, 1200.0);
    r.frame(20);
    r.down(2, 950.0, 1600.0);
    let ev = r.frame(20);
    check!(types(&ev) == [InType::PinchEnd] && r.probe("touch.reject") == "three" && r.probe("touch.suppressed") == "1");
    let mut r = Rig::new();
    r.down(0, 800.0, 1200.0);
    r.down(1, 1100.0, 1200.0);
    r.frame(20);
    r.mv(1, 1200.0, 1200.0);
    check!(types(&r.frame(20)) == [InType::PinchMove]);
    r.down(2, 950.0, 1600.0);
    check!(types(&r.frame(20)) == [InType::PinchMove] && r.probe("touch.class") == "zoom");
    r.up(2);
    check!(types(&r.frame(20)) == [InType::PinchMove]);
    r.up(0);
    check!(types(&r.frame(20)) == [InType::PinchEnd]);
}

// ---- the fixes ----

// The pen arriving (input_evdev's pen_frame calls touchgest_cancel) ends the finger with the cancel:
// its lift afterwards is no tap.
fn pen_arrival_cancels_never_taps() {
    let mut r = Rig::new();
    r.down(0, 600.0, 1100.0);
    check!(types(&r.frame(13)) == [InType::TouchDown]);
    let ev = r.cancel();
    check!(ev.len() == 1 && is_cancel(&ev[0]));
    r.up(0);
    check!(r.frame(13).is_empty() && r.probe("touch.class") == "none");
    // A session ends with PinchEnd.
    r.down(0, 800.0, 1200.0);
    r.down(1, 1100.0, 1200.0);
    r.frame(13);
    check!(types(&r.cancel()) == [InType::PinchEnd]);
    check!(r.cancel().is_empty()); // once
    r.up(0);
    r.up(1);
    check!(r.frame(13).is_empty());
}

// After SYN_DROPPED the reader resyncs the slots and cancels: a contact whose lift was lost is gone
// at the next frame, and nothing waits for it.
fn syn_dropped_leaves_no_phantom() {
    let mut r = Rig::new();
    r.down(0, 800.0, 1200.0);
    r.down(1, 1100.0, 1200.0);
    r.frame(13);
    check!(types(&r.cancel()) == [InType::PinchEnd]);
    r.up(1); // its lift was in what the kernel dropped
    check!(r.frame(13).is_empty() && r.probe("touch.contacts") == "1");
    r.up(0);
    check!(r.frame(13).is_empty() && r.probe("touch.contacts") == "0" && !touchgest_busy(&r.g));
    r.down(3, 500.0, 500.0);
    check!(types(&r.frame(13)) == [InType::TouchDown]);
}

// The pair is the order the fingers landed in, not their slots'.
fn the_pair_is_by_landing_order() {
    let mut r = Rig::new();
    r.down(5, 1300.0, 900.0);
    r.frame(60);
    r.down(1, 600.0, 1000.0);
    let ev = r.frame(20);
    check!(types(&ev) == [InType::TouchUp, InType::PinchBegin]);
    check!(ev[1].x == 1300.0 && ev[1].y == 900.0 && ev[1].x2 == 600.0 && ev[1].y2 == 1000.0);
    r.mv(1, 500.0, 1000.0);
    let ev = r.frame(20);
    check!(types(&ev) == [InType::PinchMove] && ev[0].x == 1300.0 && ev[0].x2 == 500.0);
}

// Busy while any contact is down, whichever: the one left of a pair, a palm, a pen-tool contact.
fn busy_counts_every_contact() {
    let mut r = Rig::new();
    check!(!touchgest_busy(&r.g));
    r.down(0, 800.0, 1200.0);
    r.down(1, 1100.0, 1200.0);
    r.frame(13);
    r.up(0);
    r.frame(13);
    check!(touchgest_busy(&r.g) && r.probe("touch.contacts") == "1");
    r.up(1);
    r.frame(13);
    check!(!touchgest_busy(&r.g));
    r.put(4, 300.0, 300.0, 2, 0);
    r.frame(13);
    check!(touchgest_busy(&r.g));
    r.up(4);
    r.frame(13);
    // A contact the panel calls a pen is never a finger, and still counts.
    r.put(7, 300.0, 300.0, 1, 0);
    check!(r.frame(13).is_empty() && touchgest_busy(&r.g) && r.probe("touch.class") == "none");
    r.down(0, 800.0, 1200.0);
    check!(types(&r.frame(13)) == [InType::TouchDown]);
}

// The frame's kernel time decides, not when it was read: a second finger 13 ms after the first by the
// kernel pinches, however late the reader got to it.
fn kernel_time_decides() {
    let mut r = Rig::new();
    r.down(0, 800.0, 1200.0);
    check!(types(&r.frame(13)) == [InType::TouchDown]); // (read 1.2 s later: the clock is not asked)
    r.down(1, 1100.0, 1200.0);
    check!(types(&r.frame(13)) == [InType::TouchUp, InType::PinchBegin]);
    // And the times wrap.
    let mut r = Rig::new();
    r.k = u32::MAX - 5;
    r.pen.left_ms = r.k.wrapping_sub(5000);
    r.down(0, 800.0, 1200.0);
    r.frame(100);
    r.down(1, 1100.0, 1200.0);
    check!(types(&r.frame(13)) == [InType::TouchUp, InType::PinchBegin]);
}

// TOUCH_F_BROAD from the frame its TOUCH_MAJOR passes T_palm on (a test T_palm here: by default it
// never does); TOUCH_F_CANCEL only on cancels.
fn the_flag_bits() {
    let mut r = Rig::new();
    touchgest_set_palm_major(&mut r.g, 50);
    r.put(0, 800.0, 1200.0, 0, 50);
    let ev = r.frame(13);
    check!(types(&ev) == [InType::TouchDown] && ev[0].value == 0);
    r.put(0, 810.0, 1200.0, 0, 51);
    let ev = r.frame(13);
    check!(types(&ev) == [InType::TouchMove] && ev[0].value == TOUCH_F_BROAD);
    r.put(0, 820.0, 1200.0, 0, 20);
    check!(r.frame(13)[0].value == TOUCH_F_BROAD); // it stays
    r.up(0);
    let ev = r.frame(13);
    check!(types(&ev) == [InType::TouchUp] && ev[0].value == TOUCH_F_BROAD && ev[0].value & TOUCH_F_CANCEL == 0);
    let mut r = Rig::new();
    r.put(0, 800.0, 1200.0, 0, i32::MAX);
    check!(r.frame(13)[0].value == 0); // the default T_palm: never
}

#[test]
fn test_touchgest() {
    run!(one_finger_is_the_stream_it_was);
    run!(a_palm_cancels_and_suppresses);
    run!(a_second_finger_at_120_ms_pinches);
    run!(a_second_finger_at_450_ms_is_a_palm);
    run!(the_pen_window_holds_one_finger);
    run!(value_0_is_never_a_classified_pinch);
    run!(a_pinch_commits_past_the_dead_zone);
    run!(a_third_finger);
    run!(pen_arrival_cancels_never_taps);
    run!(syn_dropped_leaves_no_phantom);
    run!(the_pair_is_by_landing_order);
    run!(busy_counts_every_contact);
    run!(kernel_time_decides);
    run!(the_flag_bits);
    done();
}
