//! Allocation-free pen-button timing. Dispatch and drawing belong to the editor.
use crate::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PbIn { pub t_ms: u32, pub button: PenBtn, pub down: bool }
impl Default for PbIn {
    fn default() -> Self { Self { t_ms: 0, button: PenBtn::Button, down: false } }
}
#[derive(Clone, Copy, Debug, Default)]
pub struct PbPen { pub prox: bool, pub touching: bool }
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum PbOut {
    #[default] None,
    Fire(PenBtn, PenKind), DragBegin(PenBtn), DragEnd(PenBtn), HoldBegin(PenBtn), HoldEnd(PenBtn),
}
#[derive(Clone, Copy, Debug)]
pub struct PbConfig { pub gap_ms: u32, pub press_ms: u32, pub hold_ms: u32, pub waits: [bool; 3] }
impl Default for PbConfig {
    fn default() -> Self { Self { gap_ms: 350, press_ms: 450, hold_ms: 1200, waits: [true; 3] } }
}
#[derive(Clone, Copy, Debug, Default)]
struct ButtonState {
    down: bool, down_at: u32, used: bool, hold: bool, drag: bool,
    clicks: u8, released_at: u32,
}
#[derive(Clone, Debug, Default)]
pub struct PenButtons { buttons: [ButtonState; 3], pen: PbPen }
fn button_index(b: PenBtn) -> usize { match b { PenBtn::Button => 0, PenBtn::Button2 => 1, PenBtn::Eraser => 2 } }
fn button_at(i: usize) -> PenBtn { [PenBtn::Button, PenBtn::Button2, PenBtn::Eraser][i] }
fn click_kind(n: u8) -> PenKind { match n { 2 => PenKind::Double, 3 => PenKind::Triple, _ => PenKind::Tap } }

pub fn penbutton_event(p: &mut PenButtons, c: &PbConfig, e: PbIn, pen: PbPen) -> PbOut {
    p.pen = pen;
    let i = button_index(e.button);
    let s = &mut p.buttons[i];
    if !pen.prox { *s = ButtonState::default(); return PbOut::None; }
    if e.down {
        if s.down { return PbOut::None; }
        // A press after the deadline starts a new count; normally tick already delivered the old one.
        let overdue = s.clicks > 0 && e.t_ms.wrapping_sub(s.released_at) > c.gap_ms;
        let old = if overdue { Some(click_kind(s.clicks)) } else { None };
        if overdue || s.clicks == 3 { s.clicks = 0; }
        s.down = true; s.down_at = e.t_ms; s.used = pen.touching; s.hold = false;
        s.drag = pen.touching;
        if s.drag { s.clicks = 0; return PbOut::DragBegin(e.button); }
        return old.map_or(PbOut::None, |k| PbOut::Fire(e.button, k));
    }
    if !s.down { return PbOut::None; }
    s.down = false;
    if s.drag { s.drag = false; s.clicks = 0; return PbOut::DragEnd(e.button); }
    if s.hold { s.hold = false; s.clicks = 0; return PbOut::HoldEnd(e.button); }
    if s.used || pen.touching || e.t_ms.wrapping_sub(s.down_at) > c.press_ms {
        s.clicks = 0; return PbOut::None;
    }
    s.clicks += 1; s.released_at = e.t_ms;
    if !c.waits[i] {
        s.clicks = 0; return PbOut::Fire(e.button, PenKind::Tap);
    }
    PbOut::None
}

pub fn penbutton_pen(p: &mut PenButtons, _c: &PbConfig, pen: PbPen, _t_ms: u32) -> PbOut {
    p.pen = pen;
    // Each call yields one end; another call with the same input drains another held button.
    if !pen.prox {
        for i in 0..3 {
            let s = &mut p.buttons[i];
            let out = if s.drag { PbOut::DragEnd(button_at(i)) }
                else if s.hold { PbOut::HoldEnd(button_at(i)) } else { PbOut::None };
            *s = ButtonState::default();
            if out != PbOut::None { return out; }
        }
        return PbOut::None;
    }
    for i in 0..3 {
        let s = &mut p.buttons[i];
        if pen.touching && s.down {
            s.used = true; s.clicks = 0;
            if s.hold { continue; }
            if !s.drag { s.drag = true; return PbOut::DragBegin(button_at(i)); }
        } else if !pen.touching && s.drag {
            s.drag = false; return PbOut::DragEnd(button_at(i));
        }
    }
    PbOut::None
}

pub fn penbutton_tick(p: &mut PenButtons, c: &PbConfig, now: u32) -> PbOut {
    if !p.pen.prox { return PbOut::None; }
    for i in 0..3 {
        let s = &mut p.buttons[i];
        if s.down && !s.used && !s.hold && !p.pen.touching && now.wrapping_sub(s.down_at) >= c.hold_ms {
            s.hold = true; s.clicks = 0; return PbOut::HoldBegin(button_at(i));
        }
        if !s.down && s.clicks > 0 && now.wrapping_sub(s.released_at) >= c.gap_ms {
            let kind = click_kind(s.clicks); s.clicks = 0; return PbOut::Fire(button_at(i), kind);
        }
    }
    PbOut::None
}

pub fn penbutton_probe(p: &PenButtons, key: &str, out: &mut String) -> bool {
    let key = key.strip_prefix("pen.").unwrap_or(key);
    let (i, field) = if let Some(k) = key.strip_prefix("Button2.") { (1, k) }
        else if let Some(k) = key.strip_prefix("Eraser.") { (2, k) }
        else { (0, key.strip_prefix("Button.").unwrap_or(key)) };
    let s = &p.buttons[i];
    let v = match field { "down" => s.down as u32, "clicks" => s.clicks as u32,
        "hold" => s.hold as u32, "drag" => s.drag as u32, "used" => s.used as u32, _ => return false };
    out.clear(); out.push_str(&v.to_string()); true
}

#[cfg(test)]
mod tests {
    use super::*;
    fn hover() -> PbPen { PbPen { prox: true, touching: false } }
    fn event(p: &mut PenButtons, c: &PbConfig, t_ms: u32, down: bool) -> PbOut {
        penbutton_event(p, c, PbIn { t_ms, down, button: PenBtn::Button }, hover())
    }
    #[test]
    fn clicks_resolve_at_the_gap_and_hold_has_one_end() {
        let c = PbConfig::default(); let mut p = PenButtons::default();
        for t in [0, 200, 400] { assert_eq!(event(&mut p, &c, t, true), PbOut::None); assert_eq!(event(&mut p, &c, t+20, false), PbOut::None); }
        assert_eq!(penbutton_tick(&mut p, &c, 769), PbOut::None);
        assert_eq!(penbutton_tick(&mut p, &c, 770), PbOut::Fire(PenBtn::Button, PenKind::Triple));
        event(&mut p, &c, 1000, true);
        assert_eq!(penbutton_tick(&mut p, &c, 2200), PbOut::HoldBegin(PenBtn::Button));
        assert_eq!(penbutton_tick(&mut p, &c, 2201), PbOut::None);
        assert_eq!(event(&mut p, &c, 2300, false), PbOut::HoldEnd(PenBtn::Button));
        assert_eq!(penbutton_tick(&mut p, &c, 3000), PbOut::None);
    }
    #[test]
    fn immediate_tap_and_clock_wrap() {
        let c = PbConfig { waits: [false; 3], ..Default::default() }; let mut p = PenButtons::default();
        event(&mut p, &c, u32::MAX-30, true);
        assert_eq!(event(&mut p, &c, 10, false), PbOut::Fire(PenBtn::Button, PenKind::Tap));
    }
    #[test]
    fn drawing_is_never_a_click_and_proximity_cancels() {
        let c = PbConfig::default(); let mut p = PenButtons::default();
        event(&mut p, &c, 0, true);
        assert_eq!(penbutton_pen(&mut p, &c, PbPen { touching: true, ..hover() }, 10), PbOut::DragBegin(PenBtn::Button));
        assert_eq!(penbutton_pen(&mut p, &c, hover(), 20), PbOut::DragEnd(PenBtn::Button));
        assert_eq!(event(&mut p, &c, 30, false), PbOut::None);
        assert_eq!(penbutton_tick(&mut p, &c, 500), PbOut::None);
        event(&mut p, &c, 600, true); event(&mut p, &c, 620, false);
        penbutton_pen(&mut p, &c, PbPen::default(), 650);
        assert_eq!(penbutton_tick(&mut p, &c, 1000), PbOut::None);
    }
}
