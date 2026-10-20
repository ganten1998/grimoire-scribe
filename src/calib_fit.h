// calib_fit.h: the pure math behind pen/touch calibration (InputCalib mapping + fitting).
#pragma once
#include "input.h"

typedef struct {
    float nx, ny;   // device-normalized 0..1
    float sx, sy;   // screen px the user was aiming at
} CalPoint;

// Applies an InputCalib (see input.h for the formula).
void calib_apply(const InputCalib *c, float nx, float ny, float *sx, float *sy);
// Puts a pen tilt (degrees per axis, from the digitizer) into the logical screen's frame: the
// calibration's axis swap and mirrors, then `rot` quarter turns clockwise, exactly as positions
// are mapped, so a lean toward the screen's bottom right reads (+, +) however the Kindle is held.
// `flip` turns the lean around for a digitizer whose tilt axes are opposite to its position axes.
void calib_tilt(const InputCalib *c, int rot, bool flip, float *tilt_x, float *tilt_y);
// Decides axis swap from the data, then least-squares fits scale + offset per axis.
// Needs >= 3 points with spread on both axes. Returns false if the data is degenerate.
bool calib_fit(const CalPoint *p, int n, InputCalib *out);

// A calibration maps to the pixels of the panel it was made on. Scaled to a w x h panel from a
// from_w x from_h one: sx = ax*u + bx is in panel pixels, so ax and bx go by w / from_w, ay and by
// by h / from_h (with the axes swapped too: ax always maps to the panel's x).
void calib_scale(InputCalib *c, int from_w, int from_h, int w, int h);
// Whether a mapping spans a w x h panel: |ax| and |ay| within 2.5% of w and h. A calibration only
// corrects the default mapping, which spans the panel exactly, and the Scribes' two panel sizes
// are 6.8% apart (1860x2480 and 1986x2648).
bool calib_spans(const InputCalib *c, int w, int h);
// What calib_to_panel did with a calibration.
typedef enum {
    CALIB_SAME,       // made on a panel of this size (or not valid): as it is
    CALIB_SCALED,     // made on a panel of another size: scaled to this one
    CALIB_MADE_HERE,  // recorded as another size's, but its own scale says this one's: as it is
} CalibMove;
// A calibration recorded as made on a from_w x from_h panel (0 x 0: not recorded, taken as made
// on an 1860x2480 Scribe), used on a w x h one. One whose own scale spans this panel and not that
// one was made here all the same: by a version that does not record the panel, or after the
// record was written (1.0.0-beta.2 carries the line through a calibration of its own).
CalibMove calib_to_panel(InputCalib *c, int from_w, int from_h, int w, int h);
