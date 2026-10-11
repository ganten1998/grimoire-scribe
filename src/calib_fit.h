// calib_fit.h: the pure math behind pen/touch calibration (InputCalib mapping + fitting).
#pragma once
#include "input.h"

typedef struct {
    float nx, ny;   // device-normalized 0..1
    float sx, sy;   // screen px the user was aiming at
} CalPoint;

// Applies an InputCalib (see input.h for the formula).
void calib_apply(const InputCalib *c, float nx, float ny, float *sx, float *sy);
// Decides axis swap from the data, then least-squares fits scale + offset per axis.
// Needs >= 3 points with spread on both axes. Returns false if the data is degenerate.
bool calib_fit(const CalPoint *p, int n, InputCalib *out);
