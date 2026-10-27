// input.h: pen / touch / button events in screen pixels. Implemented by input_evdev.c
// (Kindle) and sim_sdl.c (desktop).
#pragma once
#include "common.h"

// Maps a device axis pair, normalized to 0..1 over the device's absinfo range, to screen
// px:  if swap_xy, (u,v) = (ny, nx) else (nx, ny);  sx = ax*u + bx;  sy = ay*v + by.
// Inverted axes are negative a. valid=false means "use the built-in default guess".
typedef struct {
    bool valid;
    bool swap_xy;
    float ax, bx, ay, by;
} InputCalib;

typedef enum {
    IN_NONE = 0,
    IN_PEN_DOWN, IN_PEN_MOVE, IN_PEN_UP,   // contact (pressure > 0 / BTN_TOUCH)
    IN_PEN_HOVER,                          // in proximity, not touching
    IN_PEN_LEAVE,                          // left proximity
    IN_TOUCH_DOWN, IN_TOUCH_MOVE, IN_TOUCH_UP,  // first finger only
    IN_POWER,                              // power button pressed
    IN_QUIT,                               // signal / window closed
    IN_PEN_BUTTON,                         // pen side button pressed/released (side_button = state)
    IN_ORIENTATION,                        // device turned: value = quarter turns clockwise (0..3)
    IN_PINCH_BEGIN, IN_PINCH_MOVE, IN_PINCH_END,  // two fingers: (x,y) and (x2,y2)
    IN_SLEEP,                              // the Kindle is going to sleep (goingToScreenSaver)
    IN_WAKE,                               // and waking up (outOfScreenSaver)
    IN_POWER_UP,                           // power button released: value = ms it was held (-1 unknown)
    IN_KEY,                               // external keyboard/gamepad press: value=evdev key code
    IN_POWER_HELD,                         // power button held INPUT_POWER_HOLD_MS: once per press
    IN_HID_ATTACH,                         // an optional keyboard or controller came: hid_name,
                                           // gamepad, value 1 when another app holds it (input_hids)
    IN_HID_DETACH,                         // and went: hid_name, gamepad
    IN_KEY_UP,                             // an IN_KEY's key or button let go: value, gamepad
} InType;

// Holding the power button this long saves and leaves Grimoire (below the Kindle's own ~7 s menu).
#define INPUT_POWER_HOLD_MS 4000

typedef struct {
    InType type;
    float x, y;            // screen px after calibration
    float nx, ny;          // device-normalized 0..1 before calibration (for calibrate.c)
    float pressure;        // 0..1 normalized raw
    float tilt_x, tilt_y;  // degrees from vertical, 0 if unsupported
    unsigned key_mods;
    bool gamepad;
    bool eraser;           // eraser end in use (BTN_TOOL_RUBBER)
    bool side_button;      // BTN_STYLUS held
    int value;             // IN_ORIENTATION: quarter turns clockwise; IN_POWER_UP/HELD: ms held
    float x2, y2;          // pinch: the second finger (screen px)
    uint32_t t_ms;
    char hid_name[41];     // IN_HID_*: the device's name, printable ASCII (hidkeys_name)
} InEvent;

typedef struct {
    bool has_pen, has_pressure, has_tilt, has_touch, has_power, has_orientation;
    int orientation;       // current device orientation at open (quarter turns clockwise)
    char pen_name[64], touch_name[64];
} InputCaps;

int input_open(int screen_w, int screen_h, const InputCalib *pen, const InputCalib *touch);
// NULL leaves that mapping unchanged; pass input_default_calib's values to reset one.
void input_set_calib(const InputCalib *pen, const InputCalib *touch);
// The default (uncalibrated) mapping guess, so calibrate.c can show/compare it.
void input_default_calib(InputCalib *pen, InputCalib *touch);
const InputCaps *input_caps(void);
// Returns 1 and fills ev if an event is ready within timeout_ms (0 = non-blocking,
// -1 = forever), else 0. Never blocks once events are queued.
int input_poll(InEvent *ev, int timeout_ms);
// Extra fd to wake input_poll (e.g. the HTTP share server socket); -1 to clear.
// When it becomes readable, input_poll returns 0 early.
void input_watch_fd(int fd);
void input_shortcuts_enable(bool enabled);
// The optional keyboards and controllers watched now, none while shortcuts are off: their names
// (as hid_name), whether each is a controller, and whether another app holds it. Returns how many.
typedef struct { char name[41]; bool gamepad, held; } InputHid;
int input_hids(InputHid *out, int cap);
// Logical screen rotation (quarter turns clockwise of the device): events are reported in the
// rotated screen's coordinates from now on. Calibration stays in panel coordinates.
void input_set_orientation(int quarter_turns);
// Panel (unrotated) coordinates of a logical screen point, for calibration targets.
void input_logical_to_panel(float lx, float ly, float *px, float *py);
// Release pen and touch to the native PIN screen, then grab them after native unlock.
bool input_set_grab(bool grab);
void input_close(void);
