// test_calib.c: calibration fit recovers swapped / mirrored / scaled mappings exactly.
mod testlib;
use grimoire::prelude::*;
use testlib::*;

fn roundtrip(truth: InputCalib) {
    let tx = [0.12f32, 0.88, 0.88, 0.12];
    let ty = [0.12f32, 0.12, 0.88, 0.88];
    let mut pts = [CalPoint::default(); 4];
    for i in 0..4 {
        // Invert the true mapping to get what the device reports for each target.
        let sx = tx[i] * 1860.0;
        let sy = ty[i] * 2480.0;
        let u = (sx - truth.bx) / truth.ax;
        let v = (sy - truth.by) / truth.ay;
        pts[i] = CalPoint { nx: if truth.swap_xy { v } else { u }, ny: if truth.swap_xy { u } else { v }, sx, sy };
    }
    let mut got = InputCalib::default();
    check!(calib_fit(&pts, &mut got));
    check!(got.swap_xy == truth.swap_xy);
    check_near!(got.ax, truth.ax, 0.05);
    check_near!(got.bx, truth.bx, 0.05);
    check_near!(got.ay, truth.ay, 0.05);
    check_near!(got.by, truth.by, 0.05);
    let (mut sx, mut sy) = (0f32, 0f32);
    calib_apply(&got, pts[2].nx, pts[2].ny, &mut sx, &mut sy);
    check_near!(sx, 0.88f32 * 1860.0, 0.1);
    check_near!(sy, 0.88f32 * 2480.0, 0.1);
}

fn fits() {
    roundtrip(InputCalib { valid: true, swap_xy: false, ax: 1860.0, bx: 0.0, ay: 2480.0, by: 0.0 }); // identity
    roundtrip(InputCalib { valid: true, swap_xy: true, ax: 1860.0, bx: 0.0, ay: 2480.0, by: 0.0 }); // swapped
    roundtrip(InputCalib { valid: true, swap_xy: true, ax: -1860.0, bx: 1860.0, ay: 2480.0, by: 0.0 }); // swapped + x mirrored
    roundtrip(InputCalib { valid: true, swap_xy: false, ax: 1850.0, bx: 12.0, ay: -2470.0, by: 2475.0 }); // y mirrored + offset
}

fn degenerate() {
    let same = [
        CalPoint { nx: 0.5, ny: 0.5, sx: 10.0, sy: 10.0 },
        CalPoint { nx: 0.5, ny: 0.5, sx: 900.0, sy: 10.0 },
        CalPoint { nx: 0.5, ny: 0.5, sx: 10.0, sy: 900.0 },
    ];
    let mut c = InputCalib::default();
    check!(!calib_fit(&same, &mut c));
    check!(!calib_fit(&same[..2], &mut c));
}

// Tilt follows the same swap / mirror / rotation as positions. The owner's case: a right hand
// leaning toward the bottom right reads (-40, -46) raw with the Kindle at rotation 2.
fn tilt_frame() {
    let id = InputCalib { valid: false, swap_xy: false, ax: 1860.0, bx: 0.0, ay: 2480.0, by: 0.0 };
    let (mut x, mut y) = (-40f32, -46f32);
    calib_tilt(&id, 2, false, &mut x, &mut y);
    check_near!(x, 40.0, 1e-6);
    check_near!(y, 46.0, 1e-6);
    x = 10.0;
    y = 20.0;
    calib_tilt(&id, 0, false, &mut x, &mut y);
    check!(x == 10.0 && y == 20.0);
    // A quarter turn maps a panel direction (dx, dy) to (-dy, dx), as panel_to_logical does.
    x = 10.0;
    y = 20.0;
    calib_tilt(&id, 1, false, &mut x, &mut y);
    check!(x == -20.0 && y == 10.0);
    x = 10.0;
    y = 20.0;
    calib_tilt(&id, 3, false, &mut x, &mut y);
    check!(x == 20.0 && y == -10.0);
    // Swap and mirrors come from the calibration; flip reverses.
    let sw = InputCalib { valid: true, swap_xy: true, ax: -1860.0, bx: 1860.0, ay: 2480.0, by: 0.0 };
    x = 10.0;
    y = 20.0;
    calib_tilt(&sw, 0, false, &mut x, &mut y);
    check!(x == -20.0 && y == 10.0);
    x = 10.0;
    y = 20.0;
    calib_tilt(&id, 0, true, &mut x, &mut y);
    check!(x == -10.0 && y == -20.0);
}

#[test]
fn test_calib() {
    run!(fits);
    run!(degenerate);
    run!(tilt_frame);
    done();
}
