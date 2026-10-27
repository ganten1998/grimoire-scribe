//! calib_fit.h: the pure math behind pen/touch calibration (InputCalib mapping + fitting).

use crate::prelude::*;

#[derive(Clone, Copy, Debug, Default)]
pub struct CalPoint {
    /// device-normalized 0..1
    pub nx: f32,
    pub ny: f32,
    /// screen px the user was aiming at
    pub sx: f32,
    pub sy: f32,
}

/// Applies an InputCalib (see input.h for the formula).
pub fn calib_apply(c: &InputCalib, nx: f32, ny: f32, sx: &mut f32, sy: &mut f32) {
    let u = if c.swap_xy { ny } else { nx };
    let v = if c.swap_xy { nx } else { ny };
    // ax*u + bx in one expression: clang fuses it (PORTING.md, Floating point).
    *sx = fmla(c.ax, u, c.bx);
    *sy = fmla(c.ay, v, c.by);
}
/// Puts a pen tilt (degrees per axis, from the digitizer) into the logical screen's frame: the
/// calibration's axis swap and mirrors, then `rot` quarter turns clockwise, exactly as positions
/// are mapped, so a lean toward the screen's bottom right reads (+, +) however the Kindle is held.
/// `flip` turns the lean around for a digitizer whose tilt axes are opposite to its position axes.
pub fn calib_tilt(c: &InputCalib, rot: i32, flip: bool, tilt_x: &mut f32, tilt_y: &mut f32) {
    let mut x = *tilt_x;
    let mut y = *tilt_y;
    if c.swap_xy {
        std::mem::swap(&mut x, &mut y);
    }
    if c.ax < 0.0 {
        x = -x;
    }
    if c.ay < 0.0 {
        y = -y;
    }
    if flip {
        x = -x;
        y = -y;
    }
    // A direction turns like a position delta (see panel_to_logical in input_evdev.c).
    match rot & 3 {
        1 => {
            *tilt_x = -y;
            *tilt_y = x;
        }
        2 => {
            *tilt_x = -x;
            *tilt_y = -y;
        }
        3 => {
            *tilt_x = y;
            *tilt_y = -x;
        }
        _ => {
            *tilt_x = x;
            *tilt_y = y;
        }
    }
}

fn corr(a: &[f32], b: &[f32]) -> f32 {
    let n = a.len();
    let mut ma = 0f64;
    let mut mb = 0f64;
    for i in 0..n {
        ma += a[i] as f64;
        mb += b[i] as f64;
    }
    ma /= n as f64;
    mb /= n as f64;
    let mut sab = 0f64;
    let mut saa = 0f64;
    let mut sbb = 0f64;
    // `s += x * y` is one fused multiply-add in the C (clang's -ffp-contract=on), in double.
    for i in 0..n {
        let da = a[i] as f64 - ma;
        let db = b[i] as f64 - mb;
        sab = fmla(da, db, sab);
        saa = fmla(da, da, saa);
        sbb = fmla(db, db, sbb);
    }
    if saa <= 1e-12 || sbb <= 1e-12 {
        return 0.0;
    }
    (sab / (saa * sbb).sqrt()) as f32
}

/// y = a*x + b least squares.
fn fit_line(x: &[f32], y: &[f32], a: &mut f32, b: &mut f32) -> bool {
    let n = x.len();
    let mut mx = 0f64;
    let mut my = 0f64;
    for i in 0..n {
        mx += x[i] as f64;
        my += y[i] as f64;
    }
    mx /= n as f64;
    my /= n as f64;
    let mut sxy = 0f64;
    let mut sxx = 0f64;
    for i in 0..n {
        let dx = x[i] as f64 - mx;
        sxy = fmla(dx, y[i] as f64 - my, sxy);
        sxx = fmla(dx, dx, sxx);
    }
    if sxx < 1e-6 {
        return false;
    }
    *a = (sxy / sxx) as f32;
    // my - a*mx: fused too (a is a float promoted to double).
    *b = fmla(-(*a as f64), mx, my) as f32;
    true
}
/// Decides axis swap from the data, then least-squares fits scale + offset per axis.
/// Needs >= 3 points with spread on both axes. Returns false if the data is degenerate.
pub fn calib_fit(p: &[CalPoint], out: &mut InputCalib) -> bool {
    let n = p.len();
    if !(3..=32).contains(&n) {
        return false;
    }
    let mut nx = [0f32; 32];
    let mut ny = [0f32; 32];
    let mut sx = [0f32; 32];
    let mut sy = [0f32; 32];
    for i in 0..n {
        nx[i] = p[i].nx;
        ny[i] = p[i].ny;
        sx[i] = p[i].sx;
        sy[i] = p[i].sy;
    }
    let (nx, ny, sx, sy) = (&nx[..n], &ny[..n], &sx[..n], &sy[..n]);
    let swap = corr(ny, sx).abs() > corr(nx, sx).abs();
    let mut c = InputCalib { valid: true, swap_xy: swap, ..Default::default() };
    if !fit_line(if swap { ny } else { nx }, sx, &mut c.ax, &mut c.bx) {
        return false;
    }
    if !fit_line(if swap { nx } else { ny }, sy, &mut c.ay, &mut c.by) {
        return false;
    }
    if !c.ax.is_finite() || !c.ay.is_finite() || c.ax.abs() < 1.0 || c.ay.abs() < 1.0 {
        return false;
    }
    *out = c;
    true
}

/// A calibration maps to the pixels of the panel it was made on. Scaled to a w x h panel from a
/// from_w x from_h one: sx = ax*u + bx is in panel pixels, so ax and bx go by w / from_w, ay and by
/// by h / from_h (with the axes swapped too: ax always maps to the panel's x).
pub fn calib_scale(c: &mut InputCalib, from_w: i32, from_h: i32, w: i32, h: i32) {
    if from_w <= 0 || from_h <= 0 {
        return;
    }
    // Multiplied first: the default mapping (ax = from_w) scales to exactly w.
    c.ax = c.ax * w as f32 / from_w as f32;
    c.bx = c.bx * w as f32 / from_w as f32;
    c.ay = c.ay * h as f32 / from_h as f32;
    c.by = c.by * h as f32 / from_h as f32;
}
/// Whether a mapping spans a w x h panel: |ax| and |ay| within 2.5% of w and h. A calibration only
/// corrects the default mapping, which spans the panel exactly, and the Scribes' two panel sizes
/// are 6.8% apart (1860x2480 and 1986x2648).
pub fn calib_spans(c: &InputCalib, w: i32, h: i32) -> bool {
    w > 0 && h > 0 && (c.ax.abs() - w as f32).abs() <= 0.025 * w as f32 && (c.ay.abs() - h as f32).abs() <= 0.025 * h as f32
}
/// What calib_to_panel did with a calibration.
#[repr(i32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CalibMove {
    /// made on a panel of this size (or not valid): as it is
    #[default]
    Same = 0,  // CALIB_SAME
    /// made on a panel of another size: scaled to this one
    Scaled = 1,  // CALIB_SCALED
    /// recorded as another size's, but its own scale says this one's: as it is
    MadeHere = 2,  // CALIB_MADE_HERE
}
/// A calibration recorded as made on a *from_w x *from_h panel (0 x 0: not recorded, taken as made
/// on an 1860x2480 Scribe), used on a w x h one, scaled to it if it was made on another. Its own
/// scale has the last word, as a record can be wrong: a version that does not record the panel
/// (1.0.0-beta.2) carries the line through a calibration of its own. So it was made on the first of
/// these panels it spans: the recorded one, this one, a Scribe's of the other size (1860x2480 or
/// 1986x2648); spanning none of them, the recorded one. *from_w x *from_h is set to that panel.
pub fn calib_to_panel(c: &mut InputCalib, from_w: &mut i32, from_h: &mut i32, w: i32, h: i32) -> CalibMove {
    if *from_w <= 0 || *from_h <= 0 {
        *from_w = 1860;
        *from_h = 2480;
    }
    if !c.valid {
        return CalibMove::Same;
    }
    // Made on the first of these it spans: the recorded panel, this one, then the Scribes' (the
    // 2022 and 2024 Scribes; the Scribe 3 and the Colorsoft); spanning none, the recorded one.
    let panels = [[*from_w, *from_h], [w, h], [1860, 2480], [1986, 2648]];
    let mut mw = *from_w;
    let mut mh = *from_h;
    for p in &panels {
        if calib_spans(c, p[0], p[1]) {
            mw = p[0];
            mh = p[1];
            break;
        }
    }
    let recorded_here = *from_w == w && *from_h == h;
    *from_w = mw;
    *from_h = mh;
    if mw == w && mh == h {
        return if recorded_here { CalibMove::Same } else { CalibMove::MadeHere };
    }
    calib_scale(c, mw, mh, w, h);
    CalibMove::Scaled
}
