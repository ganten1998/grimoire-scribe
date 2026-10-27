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
    todo!()
}
/// Puts a pen tilt (degrees per axis, from the digitizer) into the logical screen's frame: the
/// calibration's axis swap and mirrors, then `rot` quarter turns clockwise, exactly as positions
/// are mapped, so a lean toward the screen's bottom right reads (+, +) however the Kindle is held.
/// `flip` turns the lean around for a digitizer whose tilt axes are opposite to its position axes.
pub fn calib_tilt(c: &InputCalib, rot: i32, flip: bool, tilt_x: &mut f32, tilt_y: &mut f32) {
    todo!()
}
/// Decides axis swap from the data, then least-squares fits scale + offset per axis.
/// Needs >= 3 points with spread on both axes. Returns false if the data is degenerate.
pub fn calib_fit(p: &[CalPoint], out: &mut InputCalib) -> bool {
    todo!()
}

/// A calibration maps to the pixels of the panel it was made on. Scaled to a w x h panel from a
/// from_w x from_h one: sx = ax*u + bx is in panel pixels, so ax and bx go by w / from_w, ay and by
/// by h / from_h (with the axes swapped too: ax always maps to the panel's x).
pub fn calib_scale(c: &mut InputCalib, from_w: i32, from_h: i32, w: i32, h: i32) {
    todo!()
}
/// Whether a mapping spans a w x h panel: |ax| and |ay| within 2.5% of w and h. A calibration only
/// corrects the default mapping, which spans the panel exactly, and the Scribes' two panel sizes
/// are 6.8% apart (1860x2480 and 1986x2648).
pub fn calib_spans(c: &InputCalib, w: i32, h: i32) -> bool {
    todo!()
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
    todo!()
}
