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
        touchgest_set_tune(&mut g, &TG_TUNE_V1);
        touchgest_set_view(&mut g, TgView { zoom_ok: true, gestures_on: true, ink: true, ..Default::default() });
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
    /// The frame at the current time with contact id's release reported at (x, y) in it.
    fn frame_ended(&mut self, dt: u32, id: i32, x: f32, y: f32) -> Vec<InEvent> {
        let mut f = TgFrame { k_ms: self.k, ..Default::default() };
        let mut c = self.c.clone();
        c.sort_by_key(|o| o.slot);
        for o in c {
            f.c[f.n] = o;
            f.n += 1;
        }
        f.ended[0] = TgContact { id: 100 + id, slot: id, x, y, nx: x / 1860.0, ny: y / 2480.0, ..Default::default() };
        f.ended_n = 1;
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

fn tuned(t: &TgTune) -> Rig {
    let mut r = Rig::new();
    touchgest_set_tune(&mut r.g, t);
    r
}

fn record_mode(r: &mut Rig) {
    touchgest_set_view(&mut r.g, TgView { zoom_ok: true, gestures_on: true, record: true, ..Default::default() });
}

/// Two fingers: a from a0 to a1 and b from b0 to b1, b landing gap ms after a, then steps frames
/// 20 ms apart, then a lifts and 5 ms later b: every event.
fn drag2(r: &mut Rig, a0: (f32, f32), a1: (f32, f32), b0: (f32, f32), b1: (f32, f32), gap: u32, steps: i32) -> Vec<InEvent> {
    let mut all = Vec::new();
    r.down(0, a0.0, a0.1);
    all.extend(r.frame(gap));
    r.down(1, b0.0, b0.1);
    all.extend(r.frame(20));
    for s in 1..=steps {
        let f = s as f32 / steps as f32;
        r.mv(0, a0.0 + (a1.0 - a0.0) * f, a0.1 + (a1.1 - a0.1) * f);
        r.mv(1, b0.0 + (b1.0 - b0.0) * f, b0.1 + (b1.1 - b0.1) * f);
        all.extend(r.frame(20));
    }
    r.up(0);
    all.extend(r.frame(5));
    r.up(1);
    all.extend(r.frame(20));
    all
}
/// A two-finger swipe centred on (x, y), the fingers sep px apart across the motion (dx, dy).
fn swipe2(r: &mut Rig, x: f32, y: f32, dx: f32, dy: f32, sep: f32, gap: u32) -> Vec<InEvent> {
    let l = (dx * dx + dy * dy).sqrt();
    let (px, py) = if l > 0.0 { (-dy / l * sep * 0.5, dx / l * sep * 0.5) } else { (sep * 0.5, 0.0) };
    drag2(r, (x - px, y - py), (x - px + dx, y - py + dy), (x + px, y + py), (x + px + dx, y + py + dy), gap, 10)
}
fn fired(ev: &[InEvent]) -> i32 {
    ev.iter().find(|e| e.type_ == InType::Gesture).map_or(0, |e| e.value)
}
fn mm(v: f32) -> f32 {
    v * 300.0 / 25.4
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
/// Past 2^31 ms of uptime (24.9 days) a pen never seen is still no pen near: a swipe fires, and so does
/// one recorded (k - 0 read as a signed number used to come out negative, a pen just gone).
fn the_clock_past_2_31_ms_is_no_pen() {
    for k in [100_000u32, 0x8000_0000u32 + 5_000] {
        let mut r = Rig::new();
        r.k = k;
        let ev = swipe2(&mut r, 930.0, 1240.0, 0.0, mm(40.0), mm(25.0), 10);
        check!(fired(&ev) != 0);
        let mut r = Rig::new();
        r.k = k;
        record_mode(&mut r);
        let ev = swipe2(&mut r, 930.0, 1240.0, 0.0, mm(40.0), mm(25.0), 10);
        check!(fired(&ev) != 0);
    }
}
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
    // (Lifted together without moving, it is a two-finger tap: see two_finger_taps.)
    r.mv(0, 700.0, 1200.0);
    r.mv(1, 1200.0, 1250.0);
    check!(types(&r.frame(13)) == [InType::PinchMove]);
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
    r.pen.last = (900.0, 1100.0); // (where it last touched: near the finger)
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

// The writing hand rests only on the notebook's ink: off it (the Library, a sheet, a keyboard) and on
// the page's controls (the panel) a finger is a finger whatever the pen does.
fn the_pen_window_is_the_inks_alone() {
    let mut r = tuned(&TG_TUNE);
    touchgest_set_view(&mut r.g, TgView { gestures_on: true, ..Default::default() });
    r.pen.prox = true;
    r.down(0, 900.0, 1200.0);
    check!(types(&r.frame(13)) == [InType::TouchDown]);
    r.mv(0, 920.0, 1200.0);
    check!(types(&r.frame(13)) == [InType::TouchMove]); // (a pen near never cancels it there)
    r.up(0);
    check!(types(&r.frame(13)) == [InType::TouchUp]);
    r.pen.prox = false;
    r.pen.left_ms = r.k;
    r.k += 400;
    r.down(0, 900.0, 1200.0);
    check!(types(&r.frame(13)) == [InType::TouchDown]);
    r.up(0);
    r.frame(13);
    // On the page: the panel's rect takes the finger, the ink beside it holds it.
    let panel = Rect::new(1700, 200, 140, 1500);
    touchgest_set_view(&mut r.g, TgView { gestures_on: true, ink: true, controls: [panel, RECT_EMPTY], ..Default::default() });
    r.pen.prox = true;
    r.pen.touching = true;
    r.down(0, 1760.0, 900.0);
    check!(types(&r.frame(13)) == [InType::TouchDown]);
    r.up(0);
    check!(types(&r.frame(13)) == [InType::TouchUp]);
    r.down(0, 1500.0, 900.0);
    check!(r.frame(13).is_empty() && r.probe("touch.suppressed") == "1");
    r.up(0);
    r.frame(13);
}

// On the ink, one finger far from the pen counts by the session rules: a fresh hover sample 60 mm
// away, or the pen gone 300 ms and 40 mm from where it last touched. A far hover leaves a live press.
fn one_finger_far_from_the_pen() {
    let mut r = tuned(&TG_TUNE);
    r.pen.prox = true;
    r.pen.hover = Some((300.0, 300.0, r.k));
    r.down(0, 900.0, 1800.0); // 1617 px (137 mm) from the hover
    check!(types(&r.frame(13)) == [InType::TouchDown]);
    r.pen.hover = Some((310.0, 300.0, r.k));
    r.mv(0, 905.0, 1800.0);
    check!(types(&r.frame(13)) == [InType::TouchMove]);
    r.k += 200; // the sample goes stale: the pen's place is not known, the press stays
    r.mv(0, 910.0, 1800.0);
    check!(types(&r.frame(13)) == [InType::TouchMove]);
    r.pen.hover = Some((880.0, 1750.0, r.k)); // the pen comes over it
    r.mv(0, 912.0, 1800.0);
    let ev = r.frame(13);
    check!(ev.len() == 1 && is_cancel(&ev[0]));
    r.up(0);
    r.frame(13);
    // A hover near the landing holds it; so does a stale far one.
    r.pen.hover = Some((900.0, 1700.0, r.k));
    r.down(0, 900.0, 1800.0);
    check!(r.frame(13).is_empty());
    r.up(0);
    r.frame(13);
    r.pen.hover = Some((300.0, 300.0, r.k.wrapping_sub(200)));
    r.down(0, 900.0, 1800.0);
    check!(r.frame(13).is_empty());
    r.up(0);
    r.frame(13);
    // Gone: under 300 ms always held; then held near its last point, a finger far from it counts.
    r.pen.prox = false;
    r.pen.hover = None;
    r.pen.last = (300.0, 300.0);
    r.pen.left_ms = r.k;
    r.k += 200;
    r.down(0, 900.0, 1800.0);
    check!(r.frame(13).is_empty());
    r.up(0);
    r.frame(13);
    r.k += 200;
    r.down(0, 350.0, 350.0);
    check!(r.frame(13).is_empty());
    r.up(0);
    r.frame(13);
    r.down(0, 900.0, 1800.0);
    check!(types(&r.frame(13)) == [InType::TouchDown]);
    r.up(0);
    check!(types(&r.frame(13)) == [InType::TouchUp]);
}

// A finger's last movement reported with its release (a flick) ends its stream where it lifted: a
// TouchMove there, then the TouchUp at the same place.
fn one_finger_lifts_where_it_ended() {
    let mut r = Rig::new();
    r.down(0, 900.0, 1200.0);
    check!(types(&r.frame(13)) == [InType::TouchDown]);
    r.up(0);
    let ev = r.frame_ended(13, 0, 980.0, 1210.0);
    check!(types(&ev) == [InType::TouchMove, InType::TouchUp]);
    check!(same_place(&ev[0], 980.0, 1210.0) && same_place(&ev[1], 980.0, 1210.0));
    // Released where it last was: the TouchUp alone.
    r.down(0, 900.0, 1200.0);
    r.frame(13);
    r.up(0);
    check!(types(&r.frame_ended(13, 0, 900.0, 1200.0)) == [InType::TouchUp]);
}

// The keyboard: a second finger landing while the first is down (fast typing rolls over) is the next
// key, never a two-finger session: the first lifts where it is, the second goes down.
fn keyboard_roll_over_types_both_keys() {
    let mut r = Rig::new();
    let keys = Rect::new(0, 1640, 1860, 840);
    touchgest_set_view(&mut r.g, TgView { gestures_on: true, keyboard_open: true, keys, ..Default::default() });
    r.down(0, 221.0, 2045.0);
    check!(types(&r.frame(60)) == [InType::TouchDown]);
    r.down(1, 398.0, 2045.0);
    let ev = r.frame(40);
    check!(types(&ev) == [InType::TouchUp, InType::TouchDown]);
    check!(same_place(&ev[0], 221.0, 2045.0) && same_place(&ev[1], 398.0, 2045.0));
    r.up(0);
    check!(r.frame(40).is_empty()); // (the first finger's lift went out already)
    r.mv(1, 400.0, 2045.0);
    check!(types(&r.frame(13)) == [InType::TouchMove]);
    r.up(1);
    check!(types(&r.frame(13)) == [InType::TouchUp]);
    // The second lifting first: its lift, and the first lifting later gives nothing.
    r.down(0, 221.0, 2045.0);
    r.frame(60);
    r.down(1, 398.0, 2045.0);
    r.frame(40);
    r.up(1);
    check!(types(&r.frame(40)) == [InType::TouchUp]);
    r.up(0);
    check!(r.frame(13).is_empty() && r.probe("touch.contacts") == "0");
    // Off the keys (a row of the sheet over them, say) the same pair is a two-finger session, as it
    // always was: the first finger's press is cancelled, never tapped.
    r.down(0, 221.0, 1200.0);
    r.frame(60);
    r.down(1, 398.0, 2045.0);
    let ev = r.frame(40);
    check!(ev.len() == 2 && is_cancel(&ev[0]) && ev[1].type_ == InType::PinchBegin);
    r.up(0);
    r.up(1);
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
        // (without zoom, a spread is a drag going nowhere: a swipe that will be too short)
        check!(all.iter().all(|e| e.type_ != InType::PinchMove || e.value == if zoom_ok { 1 } else { 3 }));
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
    // Travel alone commits too (a pan where the page scrolls).
    let mut r = Rig::new();
    touchgest_set_view(&mut r.g, TgView { zoom_ok: true, scrolls_v: true, ..Default::default() });
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

// EVIOCGMTSLOTS writes only the panel's own slots: on a 10-slot panel the ids of slots 10..16 stay as
// the buffer went in. They must read as no contact (-1): a 0 is a valid tracking id, six fingers that
// never lift, and touch is dead until a restart.
fn a_panel_with_fewer_slots_leaves_no_phantoms() {
    const PANEL_SLOTS: usize = 10;
    // The kernel's answer: slots 3 and 7 are down (ids 203 and 207) at (100 + i, 200 + i).
    let mut kernel = |buf: &mut [i32; TG_MAX + 1]| {
        let code = buf[0];
        for i in 0..PANEL_SLOTS.min(buf.len() - 1) {
            buf[1 + i] = match code {
                0x39 => {
                    if i == 3 || i == 7 {
                        200 + i as i32
                    } else {
                        -1
                    }
                }
                0x35 => 100 + i as i32,
                _ => 200 + i as i32,
            };
        }
        true
    };
    let mut col = [7i32; TG_MAX];
    check!(mt_column(&mut kernel, 0x39, -1, &mut col));
    check!(col[3] == 203 && col[7] == 207);
    check!(col.iter().filter(|&&id| id >= 0).count() == 2);
    check!(col[PANEL_SLOTS..].iter().all(|&id| id == -1));
    // Any other column reads 0 past the panel's slots, and the panel's own are its answer.
    check!(mt_column(&mut kernel, 0x35, 0, &mut col));
    check!(col[9] == 109 && col[PANEL_SLOTS..].iter().all(|&v| v == 0));
    // A request the kernel refuses leaves the column as it was.
    let mut col = [5i32; TG_MAX];
    check!(!mt_column(&mut |_| false, 0x39, -1, &mut col) && col == [5i32; TG_MAX]);
    // A panel with more slots than the buffer holds fills it; the code goes in the first int.
    let mut seen = 0;
    check!(mt_column(
        &mut |buf| {
            seen = buf[0];
            for v in buf[1..].iter_mut() {
                *v = 1;
            }
            true
        },
        0x39,
        -1,
        &mut col
    ));
    check!(seen == 0x39 && col == [1i32; TG_MAX]);
}

// The frame after a cancel holds what the panel says is down (a resync after a drop, the fingers
// still down when the Kindle hands the panel back): they wait to lift, even never seen to land.
fn the_frame_after_a_cancel() {
    let mut r = Rig::new();
    check!(r.cancel().is_empty());
    r.down(0, 900.0, 1200.0);
    check!(r.frame(13).is_empty() && r.probe("touch.suppressed") == "1");
    r.down(1, 1200.0, 1200.0);
    check!(r.frame(13).is_empty());
    r.up(0);
    r.up(1);
    check!(r.frame(13).is_empty() && r.probe("touch.class") == "none");
    r.down(0, 900.0, 1200.0);
    check!(types(&r.frame(13)) == [InType::TouchDown]);
    r.up(0);
    r.frame(13);
    // A cancel followed by a frame with nothing down holds nothing after it.
    r.cancel();
    check!(r.frame(13).is_empty());
    r.down(0, 900.0, 1200.0);
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

// ---- two fingers: swipes, taps, pans ----

// A swipe fires at its first lift: PinchEnd then Gesture in the one call, the gesture's start and
// end midpoints with it; then every contact waits to lift.
fn a_swipe_fires_at_the_first_lift() {
    let mut r = Rig::new();
    r.down(0, 782.5, 900.0);
    r.frame(30);
    r.down(1, 1077.5, 900.0);
    check!(types(&r.frame(20)) == [InType::TouchUp, InType::PinchBegin]);
    let mut moves = Vec::new();
    for s in 1..=10 {
        r.mv(0, 782.5, 900.0 + 40.0 * s as f32);
        r.mv(1, 1077.5, 900.0 + 40.0 * s as f32);
        moves.extend(r.frame(20));
    }
    check!(moves.iter().all(|e| e.type_ == InType::PinchMove && e.value == 3) && moves.len() == 9);
    check!(r.probe("touch.class") == "swipe");
    r.up(0);
    let ev = r.frame(5);
    check!(types(&ev) == [InType::PinchEnd, InType::Gesture]);
    check!(ev[0].value == 0 && ev[1].value == G_2_DOWN && ev[1].x == 930.0 && ev[1].y == 900.0 && ev[1].y2 == 1300.0);
    check!(ev[1].k_ms == ev[0].k_ms);
    check!(r.probe("touch.gesture") == "2.down" && r.probe("touch.reject").is_empty() && r.probe("touch.suppressed") == "1");
    r.mv(1, 1077.5, 1400.0);
    check!(r.frame(5).is_empty());
    r.up(1);
    check!(r.frame(5).is_empty() && r.probe("touch.class") == "none");
    // Each way.
    for (dx, dy, id) in [(0.0, -400.0, G_2_UP), (-400.0, 0.0, G_2_LEFT), (400.0, 0.0, G_2_RIGHT)] {
        let mut r = Rig::new();
        check!(fired(&swipe2(&mut r, 930.0, 1200.0, dx, dy, 295.0, 30)) == id);
    }
}

// Each threshold on both sides.
fn the_thresholds() {
    // Travel 19 / 21 mm.
    let mut r = Rig::new();
    check!(fired(&swipe2(&mut r, 930.0, 900.0, 0.0, mm(19.0), 295.0, 30)) == 0 && r.probe("touch.reject") == "short");
    let mut r = Rig::new();
    check!(fired(&swipe2(&mut r, 930.0, 900.0, 0.0, mm(21.0), 295.0, 30)) == G_2_DOWN);
    // 29 / 31 degrees off its axis (nothing scrolls: past 30 it is refused at the commit).
    let deg = |d: f32| d.to_radians();
    let mut r = Rig::new();
    check!(fired(&swipe2(&mut r, 930.0, 900.0, 400.0 * deg(29.0).sin(), 400.0 * deg(29.0).cos(), 295.0, 30)) == G_2_DOWN);
    let mut r = Rig::new();
    let ev = swipe2(&mut r, 930.0, 900.0, 400.0 * deg(31.0).sin(), 400.0 * deg(31.0).cos(), 295.0, 30);
    check!(fired(&ev) == 0 && r.probe("touch.reject") == "straight");
    check!(ev.iter().filter(|e| e.type_ == InType::PinchEnd).count() == 1 && !ev.iter().any(|e| e.type_ == InType::PinchMove));
    // Landing 140 / 160 ms: later, a zoom or pan only.
    let mut r = Rig::new();
    check!(fired(&swipe2(&mut r, 930.0, 900.0, 0.0, 400.0, 295.0, 140)) == G_2_DOWN);
    let mut r = Rig::new();
    check!(fired(&swipe2(&mut r, 930.0, 900.0, 0.0, 400.0, 295.0, 160)) == 0 && r.probe("touch.reject") == "late");
    // The spread changed by 0.55 / 0.65 of the midpoint's travel at the commit: a drag, then a zoom.
    for (half, class) in [(27.5f32, "swipe"), (32.5, "zoom")] {
        let mut r = Rig::new();
        r.down(0, 800.0, 1200.0);
        r.down(1, 1100.0, 1200.0);
        r.frame(20);
        r.mv(0, 800.0 - half, 1300.0);
        r.mv(1, 1100.0 + half, 1300.0);
        check!(types(&r.frame(20)) == [InType::PinchMove] && r.probe("touch.class") == class);
    }
    // The pen left 500 ms ago, its last point 35 / 45 mm from the nearest finger.
    for (gap, want) in [(35.0f32, 0), (45.0, G_2_DOWN)] {
        let mut r = Rig::new();
        r.pen.left_ms = r.k.wrapping_sub(500);
        r.pen.last = (782.5 - mm(gap), 1100.0);
        let ev = swipe2(&mut r, 930.0, 900.0, 0.0, 400.0, 295.0, 30);
        check!(fired(&ev) == want);
        if want == 0 {
            check!(r.probe("touch.reject") == "pen" && ev.iter().any(|e| e.type_ == InType::PinchEnd && e.value == 1));
        }
    }
    // The farther finger 1.9 / 2.1 times the nearer.
    for (ratio, want) in [(1.9f32, G_2_DOWN), (2.1, 0)] {
        let mut r = Rig::new();
        touchgest_set_view(&mut r.g, TgView { gestures_on: true, ..Default::default() });
        let ev = drag2(&mut r, (730.0, 900.0), (730.0, 900.0 + 200.0 * ratio), (1130.0, 900.0), (1130.0, 1100.0), 30, 10);
        check!(fired(&ev) == want);
        if want == 0 {
            check!(r.probe("touch.reject") == "direction");
        }
    }
    // A tap's last lift 290 / 310 ms after its first landing.
    for (last, want) in [(290u32, G_2_TAP), (310, 0)] {
        let mut r = Rig::new();
        r.down(0, 850.0, 1200.0);
        r.frame(10);
        r.down(1, 1150.0, 1200.0);
        r.frame(last - 20);
        r.up(0);
        check!(r.frame(10).is_empty() && r.probe("touch.class") == "tap");
        r.up(1);
        let ev = r.frame(10);
        check!(fired(&ev) == want);
        if want == 0 {
            check!(types(&ev) == [InType::PinchEnd] && r.probe("touch.reject") == "slow");
        } else {
            check!(types(&ev) == [InType::PinchEnd, InType::Gesture] && r.probe("touch.gesture") == "2.tap");
        }
    }
    // A tap's finger moving 4 / 6 mm (across the pair, so the spread hardly changes).
    for (moved, want) in [(4.0f32, G_2_TAP), (6.0, 0)] {
        let mut r = Rig::new();
        r.down(0, 800.0, 1200.0);
        r.down(1, 1100.0, 1200.0);
        r.frame(40);
        r.mv(1, 1100.0, 1200.0 + mm(moved));
        check!(r.frame(40).is_empty());
        r.up(0);
        r.up(1);
        let ev = r.frame(10);
        check!(fired(&ev) == want);
        if want == 0 {
            check!(r.probe("touch.reject") == "short");
        }
    }
}

// Two fingers pan on an axis the page scrolls on, per axis: a drag along the other axis is a swipe,
// and a drag at a scroll limit still pans (it never turns a page).
fn per_axis_pan() {
    let mut r = Rig::new();
    touchgest_set_view(&mut r.g, TgView { scrolls_v: true, zoom_ok: true, gestures_on: true, ..Default::default() });
    let ev = swipe2(&mut r, 930.0, 900.0, 0.0, 400.0, 295.0, 30);
    check!(fired(&ev) == 0 && ev.iter().filter(|e| e.type_ == InType::PinchMove).all(|e| e.value == 2));
    check!(ev.iter().filter(|e| e.type_ == InType::PinchMove).count() == 9 && r.probe("touch.reject").is_empty());
    let mut r = Rig::new();
    touchgest_set_view(&mut r.g, TgView { scrolls_v: true, zoom_ok: true, gestures_on: true, ..Default::default() });
    check!(fired(&swipe2(&mut r, 1300.0, 1200.0, -400.0, 0.0, 295.0, 30)) == G_2_LEFT);
    // (touchgest never knows the limits: the page says it scrolls, and up past the top it pans too)
    let mut r = Rig::new();
    touchgest_set_view(&mut r.g, TgView { scrolls_v: true, gestures_on: true, ..Default::default() });
    let ev = swipe2(&mut r, 930.0, 300.0, 0.0, -250.0, 295.0, 30);
    check!(fired(&ev) == 0 && ev.iter().any(|e| e.type_ == InType::PinchMove && e.value == 2));
    // A diagonal pans where either axis scrolls; with nothing scrolling it is not straight.
    let mut r = Rig::new();
    touchgest_set_view(&mut r.g, TgView { scrolls_h: true, gestures_on: true, ..Default::default() });
    let ev = swipe2(&mut r, 930.0, 900.0, 300.0, 300.0, 295.0, 30);
    check!(fired(&ev) == 0 && ev.iter().any(|e| e.type_ == InType::PinchMove && e.value == 2));
    let mut r = Rig::new();
    check!(fired(&swipe2(&mut r, 930.0, 900.0, 300.0, 300.0, 295.0, 30)) == 0 && r.probe("touch.reject") == "straight");
}

// Sliding back under 20 mm cancels the swipe; a third finger refuses the session; gestures off or a
// keyboard up refuse what would fire; fingers too near or too far apart are no pair.
fn other_refusals() {
    let mut r = Rig::new();
    let mut all = Vec::new();
    r.down(0, 782.5, 900.0);
    r.down(1, 1077.5, 900.0);
    all.extend(r.frame(20));
    for y in [1000.0, 1100.0, 1200.0, 1100.0, 1000.0] {
        r.mv(0, 782.5, y);
        r.mv(1, 1077.5, y);
        all.extend(r.frame(20));
    }
    r.up(0);
    r.up(1);
    all.extend(r.frame(20));
    check!(fired(&all) == 0 && r.probe("touch.reject") == "short");
    // A third finger.
    let mut r = Rig::new();
    r.down(0, 782.5, 900.0);
    r.down(1, 1077.5, 900.0);
    r.frame(30);
    r.down(2, 930.0, 1500.0);
    check!(types(&r.frame(20)) == [InType::PinchEnd] && r.probe("touch.reject") == "three");
    // ... or a late one, once the count is fixed.
    let mut r = Rig::new();
    r.down(0, 782.5, 900.0);
    r.down(1, 1077.5, 900.0);
    r.frame(200);
    r.down(2, 930.0, 1500.0);
    check!(types(&r.frame(20)) == [InType::PinchEnd] && r.probe("touch.reject") == "late");
    // Off, and a keyboard up.
    let mut r = Rig::new();
    touchgest_set_view(&mut r.g, TgView { zoom_ok: true, ..Default::default() });
    check!(fired(&swipe2(&mut r, 930.0, 900.0, 0.0, 400.0, 295.0, 30)) == 0 && r.probe("touch.reject") == "off");
    let mut r = Rig::new();
    touchgest_set_view(&mut r.g, TgView { zoom_ok: true, gestures_on: true, keyboard_open: true, ..Default::default() });
    check!(fired(&swipe2(&mut r, 930.0, 900.0, 0.0, 400.0, 295.0, 30)) == 0 && r.probe("touch.reject") == "keyboard");
    // ... where a pinch still zooms and a drag still pans.
    let mut r = Rig::new();
    touchgest_set_view(&mut r.g, TgView { zoom_ok: true, scrolls_v: true, ..Default::default() });
    let ev = drag2(&mut r, (800.0, 1200.0), (600.0, 1200.0), (1100.0, 1200.0), (1300.0, 1200.0), 20, 8);
    check!(ev.iter().any(|e| e.type_ == InType::PinchMove && e.value == 1));
    // 7 / 9 mm apart.
    let mut r = Rig::new();
    check!(fired(&swipe2(&mut r, 930.0, 900.0, 0.0, 400.0, mm(7.0), 30)) == 0 && r.probe("touch.reject") == "apart");
    let mut r = Rig::new();
    check!(fired(&swipe2(&mut r, 930.0, 900.0, 0.0, 400.0, mm(9.0), 30)) == G_2_DOWN);
    let mut r = Rig::new();
    check!(fired(&swipe2(&mut r, 930.0, 900.0, 0.0, 400.0, mm(75.0), 30)) == 0 && r.probe("touch.reject") == "apart");
    // Too slow: over a second from the first landing to the lift.
    let mut r = Rig::new();
    r.down(0, 782.5, 900.0);
    r.down(1, 1077.5, 900.0);
    r.frame(100);
    for s in 1..=10 {
        r.mv(0, 782.5, 900.0 + 40.0 * s as f32);
        r.mv(1, 1077.5, 900.0 + 40.0 * s as f32);
        r.frame(100);
    }
    r.up(0);
    check!(fired(&r.frame(10)) == 0 && r.probe("touch.reject") == "slow");
}

// The pen always wins: a session it is near (hovering, touching, or gone under 300 ms) never fires,
// and only the pen refused it (PinchEnd value 1, for the editor's toast); a zoom under way ends.
fn the_pen_near_a_session() {
    for (prox, gone) in [(true, 5000u32), (false, 200)] {
        let mut r = Rig::new();
        r.pen.prox = prox;
        r.pen.left_ms = r.k.wrapping_sub(gone);
        r.pen.last = (100.0, 100.0);
        let ev = swipe2(&mut r, 930.0, 900.0, 0.0, 400.0, 295.0, 30);
        check!(fired(&ev) == 0 && r.probe("touch.reject") == "pen");
        check!(ev.iter().filter(|e| e.type_ == InType::PinchEnd).map(|e| e.value).collect::<Vec<_>>() == [1]);
        check!(!ev.iter().any(|e| e.type_ == InType::TouchDown)); // the first finger stays quiet
    }
    // In the pen window nothing zooms or pans: a drag the page scrolls with moves nothing.
    let mut r = Rig::new();
    touchgest_set_view(&mut r.g, TgView { scrolls_v: true, zoom_ok: true, gestures_on: true, ..Default::default() });
    r.pen.left_ms = r.k.wrapping_sub(600);
    let ev = swipe2(&mut r, 930.0, 900.0, 0.0, 400.0, 295.0, 30);
    check!(fired(&ev) == 0 && !ev.iter().any(|e| e.type_ == InType::PinchMove) && r.probe("touch.reject").is_empty());
    // A zoom under way ends when the pen comes near.
    let mut r = Rig::new();
    r.down(0, 800.0, 1200.0);
    r.down(1, 1100.0, 1200.0);
    r.frame(20);
    r.mv(1, 1200.0, 1200.0);
    check!(types(&r.frame(20)) == [InType::PinchMove]);
    r.pen.prox = true;
    r.mv(1, 1250.0, 1200.0);
    check!(types(&r.frame(20)) == [InType::PinchEnd] && r.probe("touch.suppressed") == "1");
    r.pen.prox = false;
    r.up(0);
    r.up(1);
    check!(r.frame(20).is_empty());
}

// A resting contact (a grip thumb still for 300 ms, a palm) far from the gesture's fingers is left
// out; if it moves, the gesture is refused (hand).
fn resting_contacts() {
    let mut r = Rig::new();
    r.down(5, 200.0, 2200.0);
    check!(types(&r.frame(500)) == [InType::TouchDown]);
    r.down(0, 782.5, 900.0);
    let ev = r.frame(30);
    check!(ev.len() == 1 && is_cancel(&ev[0]) && r.probe("touch.suppressed") == "1");
    r.down(1, 1077.5, 900.0);
    check!(types(&r.frame(20)) == [InType::PinchBegin]);
    let mut all = Vec::new();
    for s in 1..=10 {
        r.mv(0, 782.5, 900.0 + 40.0 * s as f32);
        r.mv(1, 1077.5, 900.0 + 40.0 * s as f32);
        all.extend(r.frame(20));
    }
    r.up(0);
    all.extend(r.frame(5));
    check!(fired(&all) == G_2_DOWN);
    r.up(1);
    r.frame(5);
    // The thumb still rests: another swipe works without it lifting.
    r.k += 300;
    check!(fired(&swipe2(&mut r, 930.0, 900.0, 0.0, 400.0, 295.0, 30)) == G_2_DOWN);
    // It moves: the hand moved.
    r.k += 300;
    r.down(0, 782.5, 900.0);
    r.frame(30);
    r.down(1, 1077.5, 900.0);
    r.frame(20);
    r.mv(5, 260.0, 2200.0);
    check!(types(&r.frame(20)) == [InType::PinchEnd] && r.probe("touch.reject") == "hand");
    r.up(0);
    r.up(1);
    r.up(5);
    r.frame(20);
    // A thumb that landed 350 ms before (a slow pair at first), then the swipe's second finger: the
    // thumb rests and the two make the pair.
    let mut r = Rig::new();
    r.down(5, 200.0, 2200.0);
    r.frame(350);
    r.down(0, 782.5, 900.0);
    check!(types(&r.frame(30)) == [InType::TouchUp, InType::PinchBegin]);
    r.down(1, 1077.5, 900.0);
    check!(r.frame(20).is_empty());
    let mut all = Vec::new();
    for s in 1..=10 {
        r.mv(0, 782.5, 900.0 + 40.0 * s as f32);
        r.mv(1, 1077.5, 900.0 + 40.0 * s as f32);
        all.extend(r.frame(20));
    }
    r.up(0);
    all.extend(r.frame(5));
    check!(fired(&all) == G_2_DOWN);
    // A palm (tool 2) resting before the pair: single fingers wait, the pair swipes.
    let mut r = Rig::new();
    r.put(6, 300.0, 2100.0, 2, 0);
    check!(r.frame(40).is_empty() && r.probe("touch.suppressed") == "1");
    r.down(2, 1500.0, 900.0);
    check!(r.frame(300).is_empty()); // one finger beside a palm: nothing
    r.up(2);
    r.frame(40);
    check!(fired(&swipe2(&mut r, 930.0, 900.0, 0.0, 400.0, 295.0, 30)) == G_2_DOWN);
    // A palm near the fingers is no resting one.
    let mut r = Rig::new();
    r.put(6, 900.0, 1000.0, 2, 0);
    r.frame(40);
    check!(fired(&swipe2(&mut r, 930.0, 900.0, 0.0, 400.0, 295.0, 30)) == 0);
}

// The windows that close with no new frame: the pen window's first finger, a tap's last lift.
fn the_tick() {
    let mut r = Rig::new();
    r.pen.left_ms = r.k.wrapping_sub(500);
    r.pen.last = (950.0, 1200.0);
    r.down(0, 900.0, 1200.0);
    r.frame(13);
    check!(r.probe("touch.class") == "suppressed");
    let mut out = TgOut::default();
    touchgest_tick(&mut r.g, r.k + 200, &mut out);
    check!(out.n == 0);
    r.k += 300;
    r.down(1, 1200.0, 1200.0);
    check!(r.frame(13).is_empty()); // too late to pair: the first finger is the pen's hand
    r.up(0);
    r.up(1);
    r.frame(13);
    // A tap whose second finger stays: refused 150 ms after the first lift, without a frame.
    let mut r = Rig::new();
    r.down(0, 850.0, 1200.0);
    r.down(1, 1150.0, 1200.0);
    r.frame(60);
    r.up(0);
    check!(r.frame(10).is_empty() && r.probe("touch.class") == "tap");
    touchgest_tick(&mut r.g, r.k + 100, &mut out);
    check!(out.n == 0);
    touchgest_tick(&mut r.g, r.k + 200, &mut out);
    check!(out.n == 1 && out.ev[0].type_ == InType::PinchEnd && out.outcome == TgOutcome::Rejected(TgReject::Slow));
    check!(r.probe("touch.reject") == "slow");
}

// Two-finger taps: both lift quickly, without moving; the midpoint in the clock corner is still a tap
// (the corner is one finger's).
fn two_finger_taps() {
    let mut r = Rig::new();
    r.down(0, 25.0, 100.0);
    r.frame(10);
    r.down(1, 175.0, 100.0);
    r.frame(80);
    r.up(0);
    check!(r.frame(10).is_empty());
    r.up(1);
    let ev = r.frame(10);
    check!(types(&ev) == [InType::PinchEnd, InType::Gesture] && ev[1].value == G_2_TAP);
    check!(ev[1].x == 100.0 && ev[1].y == 100.0 && ev[1].x2 == 100.0);
    // Lifted in the same frame.
    let mut r = Rig::new();
    r.down(0, 850.0, 1200.0);
    r.down(1, 1150.0, 1200.0);
    r.frame(100);
    r.up(0);
    r.up(1);
    check!(fired(&r.frame(10)) == G_2_TAP);
    // A finger landing while the tap waits: late.
    let mut r = Rig::new();
    r.down(0, 850.0, 1200.0);
    r.down(1, 1150.0, 1200.0);
    r.frame(60);
    r.up(0);
    r.frame(10);
    r.down(2, 600.0, 600.0);
    check!(types(&r.frame(10)) == [InType::PinchEnd] && r.probe("touch.reject") == "late");
    // Three fingers at once are three (nothing fires in this version).
    let mut r = Rig::new();
    r.down(0, 700.0, 1200.0);
    r.down(1, 900.0, 1200.0);
    r.down(2, 1100.0, 1200.0);
    check!(r.frame(60).is_empty() && r.probe("touch.reject") == "three");
}

// ---- the touch log and the trace ----

// The formatters' exact lines, and nothing at all while input is not grabbed (the PIN's).
fn the_log_and_trace_lines() {
    let mut f = TgFrame { k_ms: 123456, ..Default::default() };
    f.c[0] = TgContact { id: 41, slot: 0, x: 100.4, y: 200.6, nx: 0.05, ny: 0.08, major: 7, tool: 0, pressure: 30 };
    f.c[1] = TgContact { id: 42, slot: 3, x: 1500.5, y: 2400.0, nx: 0.8, ny: 0.97, major: 9, tool: 2, pressure: 0 };
    f.n = 2;
    let mut s = String::new();
    check!(touchgest_log_frame(&f, true, &mut s) && s == "touch: k 123456 n 2 s0:id41:t0:m7:p30 s3:id42:t2:m9:p0");
    check!(!touchgest_log_frame(&f, false, &mut s) && s.is_empty());
    let mut t = String::new();
    check!(touchgest_trace_frame(&f, true, &mut t) && t == "F 123456 0:41:100:201:7:0 3:42:1501:2400:9:2\n");
    check!(touchgest_trace_pen(123460, "prox", true, &mut t));
    check!(touchgest_trace_outcome(123470, TgOutcome::Fired(G_2_DOWN), true, &mut t));
    check!(touchgest_trace_outcome(123480, TgOutcome::Rejected(TgReject::Pen), true, &mut t));
    check!(!touchgest_trace_outcome(123490, TgOutcome::None, true, &mut t));
    check!(t == "F 123456 0:41:100:201:7:0 3:42:1501:2400:9:2\nP 123460 prox\nE 123470 fired 2.down\nE 123480 rejected pen\n");
    let mut u = String::new();
    check!(!touchgest_trace_frame(&f, false, &mut u) && !touchgest_trace_pen(1, "down", false, &mut u));
    check!(!touchgest_trace_outcome(1, TgOutcome::Fired(G_2_UP), false, &mut u) && u.is_empty());
    touchgest_trace_header(1, 2480, 1860, 300.0, &mut u);
    check!(u == "# grimoire touch-trace 1, rotation 1, 2480x1860, dpi 300\n");
    check!(touchgest_log_fingertips(12, 20, 33, true, &mut s) && s == "touch: fingertips touch_major median 12 max 20 over 33 frames");
    check!(!touchgest_log_fingertips(12, 20, 33, false, &mut s) && s.is_empty());
    // A trace frame appends to what the buffer holds and allocates nothing more when it has room.
    let mut b = String::with_capacity(4096);
    let cap = b.capacity();
    for _ in 0..20 {
        touchgest_trace_frame(&f, true, &mut b);
    }
    check!(b.capacity() == cap);
}

// A touch session's record: how many contacts, their landing and lift gaps from the first, how it
// went; one contact logs no session line.
fn the_session_line() {
    let mut r = Rig::new();
    let mut s = String::new();
    swipe2(&mut r, 930.0, 900.0, 0.0, 400.0, 295.0, 30);
    check!(touchgest_session_contacts(&r.g) == 2);
    check!(touchgest_log_session(&r.g, true, &mut s));
    check!(s == "touch: session 2 contacts, landing gaps 0 30 ms, lift gaps 0 5 ms: fired 2.down");
    check!(!touchgest_log_session(&r.g, false, &mut s) && s.is_empty());
    let mut r = Rig::new();
    r.down(0, 800.0, 1200.0);
    r.frame(450);
    r.down(1, 1100.0, 1200.0);
    r.frame(20);
    r.up(1);
    r.frame(40);
    r.up(0);
    r.frame(20);
    check!(touchgest_log_session(&r.g, true, &mut s) && s == "touch: session 2 contacts, landing gaps 0 450 ms, lift gaps 0 40 ms: palm");
    let mut r = Rig::new();
    check!(fired(&swipe2(&mut r, 930.0, 900.0, 0.0, mm(10.0), 295.0, 30)) == 0);
    touchgest_result_name(&r.g, &mut s);
    check!(s == "rejected short");
    let mut r = Rig::new();
    r.down(0, 800.0, 1200.0);
    r.frame(20);
    r.up(0);
    r.frame(20);
    check!(touchgest_session_contacts(&r.g) == 1 && !touchgest_log_session(&r.g, true, &mut s));
    // A zoom, a pan, a cancel.
    let mut r = Rig::new();
    drag2(&mut r, (800.0, 1200.0), (600.0, 1200.0), (1100.0, 1200.0), (1300.0, 1200.0), 20, 8);
    touchgest_result_name(&r.g, &mut s);
    check!(s == "zoom");
    let mut r = Rig::new();
    touchgest_set_view(&mut r.g, TgView { scrolls_v: true, gestures_on: true, ..Default::default() });
    swipe2(&mut r, 930.0, 900.0, 0.0, 400.0, 295.0, 30);
    touchgest_result_name(&r.g, &mut s);
    check!(s == "pan");
    let mut r = Rig::new();
    r.down(0, 800.0, 1200.0);
    r.frame(20);
    r.cancel();
    r.up(0);
    r.frame(20);
    touchgest_result_name(&r.g, &mut s);
    check!(s == "cancelled");
}

fn forgiving_tuning_edges() {
    for (travel, want) in [(15.0, 0), (17.0, G_2_RIGHT)] {
        let mut r = tuned(&TG_TUNE);
        touchgest_set_view(&mut r.g, TgView { gestures_on: true, ..Default::default() });
        check!(fired(&swipe2(&mut r, 900.0, 1200.0, mm(travel), 0.0, mm(20.0), 0)) == want);
    }
    for (ratio, want) in [(2.9, G_2_RIGHT), (3.1, 0)] {
        let mut r = tuned(&TG_TUNE);
        touchgest_set_view(&mut r.g, TgView { gestures_on: true, ..Default::default() });
        let ev = drag2(&mut r, (700.0, 1000.0), (700.0 + mm(12.0), 1000.0),
            (700.0, 1300.0), (700.0 + mm(12.0 * ratio), 1300.0), 0, 10);
        check!(fired(&ev) == want);
    }
    for (cos, want) in [(0.68f32, 0), (0.72, G_2_RIGHT)] {
        let mut t = TG_TUNE;
        t.spread_max = 100.0;
        let mut r = tuned(&t);
        touchgest_set_view(&mut r.g, TgView { gestures_on: true, ..Default::default() });
        let y = mm(24.0) * (1.0 - cos * cos).sqrt() / cos;
        let ev = drag2(&mut r, (700.0, 1000.0), (700.0 + mm(24.0), 1000.0 - y),
            (700.0, 1300.0), (700.0 + mm(24.0), 1300.0 + y), 0, 10);
        check!(fired(&ev) == want);
    }
    for (duration, want) in [(390, G_2_TAP), (410, 0)] {
        let mut r = tuned(&TG_TUNE);
        r.down(0, 800.0, 1200.0); r.down(1, 1100.0, 1200.0); r.frame(duration);
        r.up(0); r.up(1);
        check!(fired(&r.frame(0)) == want);
    }
    // Converging sideways fingers do not constitute opposite along-axis motion.
    for (t, want) in [(&TG_TUNE, G_2_RIGHT), (&TG_TUNE_V1, 0)] {
        let mut r = tuned(t);
        touchgest_set_view(&mut r.g, TgView { gestures_on: true, ..Default::default() });
        let ev = drag2(&mut r, (700.0, 900.0), (1300.0, 1080.0), (700.0, 1500.0), (1300.0, 1320.0), 0, 10);
        check!(fired(&ev) == want);
    }
}

fn following_lift_and_zoom_regrab() {
    for (go_on, want) in [(true, G_2_RIGHT), (false, 0)] {
        let mut r = tuned(&TG_TUNE);
        touchgest_set_view(&mut r.g, TgView { gestures_on: true, ..Default::default() });
        r.down(0, 700.0, 1000.0); r.down(1, 700.0, 1300.0); r.frame(40);
        r.mv(0, 790.0, 1000.0); r.mv(1, 790.0, 1300.0); r.frame(30);
        r.up(0); let mut ev = r.frame(40);
        if go_on { r.mv(1, 1000.0, 1300.0); }
        ev.extend(r.frame(40)); r.up(1); ev.extend(r.frame(0));
        check!(fired(&ev) == want);
    }
    for (gap, away, regrabs) in [(60, 12.0, true), (100, 12.0, false), (60, 20.0, false)] {
        let mut r = tuned(&TG_TUNE);
        r.down(0, 800.0, 1200.0); r.down(1, 1100.0, 1200.0); r.frame(20);
        r.mv(0, 700.0, 1200.0); r.mv(1, 1200.0, 1200.0); r.frame(20);
        r.up(0); let lift = r.k; check!(!types(&r.frame(gap)).contains(&InType::PinchEnd));
        let mut out = TgOut::default(); touchgest_tick(&mut r.g, r.k, &mut out);
        let mut ev = out.ev[..out.n].to_vec();
        r.down(2, 700.0 + mm(away), 1200.0); ev.extend(r.frame(0));
        check!(ev.iter().any(|e| e.type_ == InType::PinchBegin && e.value == 1) == regrabs);
        if !regrabs {
            // A mismatched regrab also resolves at the grace deadline, using the original lift time.
            touchgest_tick(&mut r.g, lift + 100, &mut out); ev.extend_from_slice(&out.ev[..out.n]);
            check!(ev.iter().any(|e| e.type_ == InType::PinchEnd && e.k_ms == lift));
        }
    }
    let mut r = tuned(&TG_TUNE);
    r.down(0, 800.0, 1200.0); r.down(1, 1100.0, 1200.0); r.frame(20);
    r.mv(0, 890.0, 1200.0); r.mv(1, 1190.0, 1200.0); r.frame(20);
    check!(r.probe("touch.class") == "swipe");
    r.mv(0, 750.0, 1200.0); r.mv(1, 1300.0, 1200.0);
    check!(r.frame(20).iter().any(|e| e.type_ == InType::PinchMove && e.value == 1));
    check!(r.probe("touch.class") == "zoom");
}

fn record_hover_three_and_measure() {
    for (fingers, slots, want) in [(2, 2, G_2_TAP), (3, 3, G_3_TAP), (3, 2, -11)] {
        let mut r = tuned(&TG_TUNE); record_mode(&mut r); touchgest_set_slots(&mut r.g, slots);
        for i in 0..fingers { r.down(i, 700.0 + i as f32 * 180.0, 1200.0); }
        let mut ev = r.frame(100); for i in 0..fingers { r.up(i); } ev.extend(r.frame(0));
        check!(fired(&ev) == want);
        check!(!ev.iter().any(|e| matches!(e.type_, InType::PinchBegin | InType::PinchMove | InType::PinchEnd)));
    }
    for (far, age, want) in [(80.0, 0, G_2_RIGHT), (30.0, 0, 0), (80.0, 51, 0)] {
        let mut r = tuned(&TG_TUNE);
        touchgest_set_view(&mut r.g, TgView { gestures_on: true, ..Default::default() });
        r.pen.prox = true;
        r.down(0, 300.0 + mm(far), 1000.0); r.down(1, 300.0 + mm(far), 1250.0);
        r.pen.hover = Some((300.0, 1100.0, r.k.wrapping_sub(age))); r.frame(20);
        let mut ev = Vec::new();
        for i in 1..=10 {
            r.pen.hover = Some((300.0, 1100.0, r.k.wrapping_sub(age)));
            r.mv(0, 300.0 + mm(far) + i as f32 * 30.0, 1000.0);
            r.mv(1, 300.0 + mm(far) + i as f32 * 30.0, 1250.0); ev.extend(r.frame(20));
        }
        r.up(0); r.up(1); r.pen.hover = Some((300.0, 1100.0, r.k.wrapping_sub(age))); ev.extend(r.frame(0));
        check!(fired(&ev) == want);
    }
    let mut r = tuned(&TG_TUNE);
    let ev = swipe2(&mut r, 900.0, 1200.0, 300.0, 0.0, 240.0, 0);
    check!(fired(&ev) == G_2_RIGHT);
    let mut line = String::new();
    check!(touchgest_log_measure(&r.g, true, &mut line) && line.starts_with("touch: measure d0 "));
    check!(!line.contains("900") && !line.contains("1200"));
    check!(!touchgest_log_measure(&r.g, false, &mut line) && line.is_empty());
    for (r, code) in [(TgReject::Pen,-1),(TgReject::Late,-2),(TgReject::Straight,-3),(TgReject::Short,-4),
        (TgReject::Direction,-5),(TgReject::Spread,-5),(TgReject::Hand,-6),(TgReject::Zoom,-7),
        (TgReject::Apart,-9),(TgReject::Slow,-10),(TgReject::Three,-11)] { check!(touchgest_reject_code(r) == code); }
    // A record card has no zoomable page, but spreading must still be reserved for zoom.
    let mut rec = tuned(&TG_TUNE);
    touchgest_set_view(&mut rec.g, TgView { record: true, gestures_on: true, ..Default::default() });
    let ev = drag2(&mut rec, (700.0, 1200.0), (500.0, 1200.0), (1000.0, 1200.0), (1200.0, 1200.0), 0, 10);
    check!(fired(&ev) == -7 && !ev.iter().any(|e| matches!(e.type_, InType::PinchBegin | InType::PinchMove | InType::PinchEnd)));
    // Runtime and recording consume the same tuned verdict, while recording has no pinch side effects.
    let mut runtime = tuned(&TG_TUNE); let mut record = tuned(&TG_TUNE); record_mode(&mut record);
    let a = swipe2(&mut runtime, 900.0, 1200.0, 300.0, 0.0, 240.0, 0);
    let b = swipe2(&mut record, 900.0, 1200.0, 300.0, 0.0, 240.0, 0);
    check!(fired(&a) == fired(&b) && fired(&b) == G_2_RIGHT);
    check!(!b.iter().any(|e| matches!(e.type_, InType::PinchBegin | InType::PinchMove | InType::PinchEnd)));
}

#[test]
fn test_touchgest() {
    run!(the_clock_past_2_31_ms_is_no_pen);
    run!(one_finger_is_the_stream_it_was);
    run!(a_palm_cancels_and_suppresses);
    run!(a_second_finger_at_120_ms_pinches);
    run!(a_second_finger_at_450_ms_is_a_palm);
    run!(the_pen_window_holds_one_finger);
    run!(the_pen_window_is_the_inks_alone);
    run!(one_finger_far_from_the_pen);
    run!(one_finger_lifts_where_it_ended);
    run!(keyboard_roll_over_types_both_keys);
    run!(value_0_is_never_a_classified_pinch);
    run!(a_pinch_commits_past_the_dead_zone);
    run!(a_third_finger);
    run!(pen_arrival_cancels_never_taps);
    run!(syn_dropped_leaves_no_phantom);
    run!(a_panel_with_fewer_slots_leaves_no_phantoms);
    run!(the_frame_after_a_cancel);
    run!(the_pair_is_by_landing_order);
    run!(busy_counts_every_contact);
    run!(kernel_time_decides);
    run!(the_flag_bits);
    run!(a_swipe_fires_at_the_first_lift);
    run!(the_thresholds);
    run!(per_axis_pan);
    run!(other_refusals);
    run!(the_pen_near_a_session);
    run!(resting_contacts);
    run!(the_tick);
    run!(two_finger_taps);
    run!(the_log_and_trace_lines);
    run!(the_session_line);
    run!(forgiving_tuning_edges);
    run!(following_lift_and_zoom_regrab);
    run!(record_hover_three_and_measure);
    done();
}
