#!/usr/bin/env python3
"""Writes the synthetic pen streams tests/tablet_check.sh replays through tools/scribe-tablet.c:
32-bit ARM input_events (16 bytes: tv_sec u32, tv_usec u32, type u16, code u16, value i32), as
`cat` of the Kindle's evdev node sends them. Made here, apart from the tool, so a mistake in its
reading is not repeated in its fixture.
usage: make_capture.py raw|scaled OUT"""
import struct
import sys

EV_SYN, EV_KEY, EV_ABS, EV_MSC = 0, 1, 3, 4
SYN_REPORT, SYN_DROPPED = 0, 3
ABS_X, ABS_Y, ABS_PRESSURE, ABS_TILT_X = 0x00, 0x01, 0x18, 0x1A
BTN_TOOL_PEN, BTN_TOOL_RUBBER, BTN_TOUCH, BTN_STYLUS = 0x140, 0x141, 0x14A, 0x14B
MSC_SERIAL = 0
X, Y, P, TILT = ((EV_ABS, c) for c in (ABS_X, ABS_Y, ABS_PRESSURE, ABS_TILT_X))
PEN, RUBBER, TOUCH, SIDE = ((EV_KEY, c) for c in (BTN_TOOL_PEN, BTN_TOOL_RUBBER, BTN_TOUCH, BTN_STYLUS))


def ev(out, ms, kind, value):
    us = round(ms * 1000)
    out += struct.pack("<IIHHi", us // 1000000, us % 1000000, kind[0], kind[1], value)


def frame(out, ms, *pairs):
    """One frame at ms milliseconds: (kind, value) pairs, then SYN_REPORT."""
    for kind, value in pairs:
        ev(out, ms, kind, value)
    ev(out, ms, (EV_SYN, SYN_REPORT), 0)


def raw():
    """The raw digitizer (WacomDigitizer: 0..15624 x 0..20832, pressure 0..4095)."""
    o = bytearray()
    # Connected in the middle of a stroke: no BTN_TOOL_PEN or BTN_TOUCH yet, a pressure says it touches.
    frame(o, 1000, (X, 5000), (Y, 10000), (P, 900))
    frame(o, 1003, (X, 5100))
    frame(o, 1006, (P, 0), (TOUCH, 0))
    frame(o, 1009, (X, 6000), (Y, 11000))
    frame(o, 1012, (TILT, 300))  # tilt only: nothing to post
    frame(o, 1015, (PEN, 0))
    # A stroke whose pressure dips (--pressure 1000 lets go at 750, below three quarters).
    frame(o, 1100, (PEN, 1), (X, 7000), (Y, 12000), ((EV_MSC, MSC_SERIAL), 77))
    frame(o, 1103, (TOUCH, 1), (P, 1200))
    frame(o, 1106, (X, 7010), (P, 1300))
    frame(o, 1109, (P, 800))
    frame(o, 1112, (P, 700), (X, 7020))
    frame(o, 1115, (TOUCH, 0), (P, 0))
    # Taps on one spot: the 2nd and 3rd are within 500 ms of the one before, the 4th is not.
    for t in (1200, 1260, 1900):
        frame(o, t, (TOUCH, 1), (P, 1500))
        frame(o, t + 30, (TOUCH, 0), (P, 0))
    # The side button held through a touch.
    frame(o, 2000, (SIDE, 1))
    frame(o, 2003, (TOUCH, 1), (P, 1500), (X, 8000))
    frame(o, 2006, (X, 8100))
    frame(o, 2009, (TOUCH, 0), (P, 0))
    frame(o, 2012, (SIDE, 0))
    frame(o, 2020, (PEN, 0))
    # The eraser end.
    frame(o, 2500, (RUBBER, 1), (X, 9000), (Y, 13000))
    frame(o, 2503, (TOUCH, 1), (P, 1500))
    frame(o, 2506, (TOUCH, 0), (P, 0))
    frame(o, 2509, (RUBBER, 0))
    # A drop that lost the lift's BTN_TOUCH 0: the button stays down until a pressure says.
    frame(o, 3000, (PEN, 1), (X, 10000), (Y, 14000))
    frame(o, 3003, (TOUCH, 1), (P, 2000))
    frame(o, 3006, (X, 10100))
    ev(o, 3050, (EV_SYN, SYN_DROPPED), 0)
    frame(o, 3051, (Y, 14500))
    frame(o, 3054, (X, 10200))
    frame(o, 3057, (P, 0))
    frame(o, 3060, (X, 10300))
    # A drop that lost the lift and the leaving: the pen coming back lets go.
    frame(o, 3100, (TOUCH, 1), (P, 2000))
    ev(o, 3150, (EV_SYN, SYN_DROPPED), 0)
    frame(o, 3400, (PEN, 1), (X, 11000), (Y, 15000))
    # Off the area: the pointer stays on the screen's edge.
    frame(o, 3500, (X, 20000), (Y, -500))
    # Ends touching, with part of an event: the button is let go at the end.
    frame(o, 3600, (TOUCH, 1), (P, 1500))
    o += b"\x01\x02\x03\x04\x05\x06\x07"
    return o


def scaled():
    """The scaled pen (stylus-custom: 0..1860 x 0..2480): no pressure, no place while it hovers."""
    o = bytearray()
    # Connected in the middle of a stroke: nothing says it touches until the lift.
    frame(o, 0, (X, 500), (Y, 1000))
    frame(o, 3, (X, 510))
    frame(o, 6, (TOUCH, 0))
    frame(o, 10, (PEN, 0))
    frame(o, 500, (PEN, 1), (TOUCH, 1), (X, 600), (Y, 1100))
    frame(o, 503, (X, 610))
    frame(o, 506, (TOUCH, 0))
    frame(o, 509, (PEN, 0))
    return o


if __name__ == "__main__":
    data = {"raw": raw, "scaled": scaled}[sys.argv[1]]()
    with open(sys.argv[2], "wb") as f:
        f.write(data)
