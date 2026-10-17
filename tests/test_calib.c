// test_calib.c: calibration fit recovers swapped / mirrored / scaled mappings exactly.
#include "calib_fit.h"
#include "testlib.h"

static void roundtrip(InputCalib truth) {
    float tx[4] = {0.12f, 0.88f, 0.88f, 0.12f}, ty[4] = {0.12f, 0.12f, 0.88f, 0.88f};
    CalPoint pts[4];
    for (int i = 0; i < 4; i++) {
        // Invert the true mapping to get what the device reports for each target.
        float sx = tx[i] * 1860.f, sy = ty[i] * 2480.f;
        float u = (sx - truth.bx) / truth.ax, v = (sy - truth.by) / truth.ay;
        pts[i] = (CalPoint){truth.swap_xy ? v : u, truth.swap_xy ? u : v, sx, sy};
    }
    InputCalib got;
    CHECK(calib_fit(pts, 4, &got));
    CHECK(got.swap_xy == truth.swap_xy);
    CHECK_NEAR(got.ax, truth.ax, 0.05);
    CHECK_NEAR(got.bx, truth.bx, 0.05);
    CHECK_NEAR(got.ay, truth.ay, 0.05);
    CHECK_NEAR(got.by, truth.by, 0.05);
    float sx, sy;
    calib_apply(&got, pts[2].nx, pts[2].ny, &sx, &sy);
    CHECK_NEAR(sx, 0.88f * 1860.f, 0.1);
    CHECK_NEAR(sy, 0.88f * 2480.f, 0.1);
}

TEST(fits) {
    roundtrip((InputCalib){true, false, 1860.f, 0.f, 2480.f, 0.f});      // identity
    roundtrip((InputCalib){true, true, 1860.f, 0.f, 2480.f, 0.f});       // swapped
    roundtrip((InputCalib){true, true, -1860.f, 1860.f, 2480.f, 0.f});   // swapped + x mirrored
    roundtrip((InputCalib){true, false, 1850.f, 12.f, -2470.f, 2475.f}); // y mirrored + offset
}

TEST(degenerate) {
    CalPoint same[3] = {{0.5f, 0.5f, 10, 10}, {0.5f, 0.5f, 900, 10}, {0.5f, 0.5f, 10, 900}};
    InputCalib c;
    CHECK(!calib_fit(same, 3, &c));
    CHECK(!calib_fit(same, 2, &c));
}

// Tilt follows the same swap / mirror / rotation as positions. The owner's case: a right hand
// leaning toward the bottom right reads (-40, -46) raw with the Kindle at rotation 2.
TEST(tilt_frame) {
    InputCalib id = {false, false, 1860.f, 0.f, 2480.f, 0.f};
    float x = -40.f, y = -46.f;
    calib_tilt(&id, 2, false, &x, &y);
    CHECK_NEAR(x, 40.f, 1e-6);
    CHECK_NEAR(y, 46.f, 1e-6);
    x = 10.f, y = 20.f;
    calib_tilt(&id, 0, false, &x, &y);
    CHECK(x == 10.f && y == 20.f);
    // A quarter turn maps a panel direction (dx, dy) to (-dy, dx), as panel_to_logical does.
    x = 10.f, y = 20.f;
    calib_tilt(&id, 1, false, &x, &y);
    CHECK(x == -20.f && y == 10.f);
    x = 10.f, y = 20.f;
    calib_tilt(&id, 3, false, &x, &y);
    CHECK(x == 20.f && y == -10.f);
    // Swap and mirrors come from the calibration; flip reverses.
    InputCalib sw = {true, true, -1860.f, 1860.f, 2480.f, 0.f};
    x = 10.f, y = 20.f;
    calib_tilt(&sw, 0, false, &x, &y);
    CHECK(x == -20.f && y == 10.f);
    x = 10.f, y = 20.f;
    calib_tilt(&id, 0, true, &x, &y);
    CHECK(x == -10.f && y == -20.f);
}

int main(void) {
    RUN(fits);
    RUN(degenerate);
    RUN(tilt_frame);
    return tl_done();
}
