// sim_sdl.c: desktop simulator implementing display.h + input.h.
//
// Window mode (SDL3, built with GRIMOIRE_SIM_SDL): mouse = pen, right button = finger, keys
// tweak pressure/tilt. Headless mode (GRIMOIRE_SCRIPT=<file>, and the only mode of the
// grimoire-headless build, which does not link SDL at all): events come from a script and the
// screen can be dumped to PNG ("shot NAME") for automated checks. See docs/SIM.md.
#ifdef GRIMOIRE_SIM_SDL
#define SDL_MAIN_HANDLED
#include <SDL3/SDL.h>
#endif
#include <ctype.h>
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "app.h"  // app_probe, for the script's expect checks
#include "calib_fit.h"
#include "display.h"
#include "input.h"
#include "stb_image_write.h"

static int g_w = 1860, g_h = 2480;
#ifdef GRIMOIRE_SIM_SDL
static float g_scale = 0.36f;
#endif
static bool g_headless;

// ---- shared event queue ----

#define QN 8192
static InEvent g_q[QN];
static int g_qh, g_qt;
static void q_push(InEvent e) {
    int n = (g_qt + 1) % QN;
    if (n == g_qh) return;  // full: drop
    g_q[g_qt] = e;
    g_qt = n;
}
static bool q_pop(InEvent *e) {
    if (g_qh == g_qt) return false;
    *e = g_q[g_qh];
    g_qh = (g_qh + 1) % QN;
    return true;
}

static InputCalib g_pen, g_touch;
static InputCaps g_caps = {true, true, true, true, true, false, 0, "SDL mouse (sim)", "SDL right button (sim)"};
static int g_watch_fd = -1;

void input_default_calib(InputCalib *pen, InputCalib *touch) {
    InputCalib c = {false, false, (float)g_w, 0.f, (float)g_h, 0.f};
    if (pen) *pen = c;
    if (touch) *touch = c;
}

void input_set_calib(const InputCalib *pen, const InputCalib *touch) {
    if (pen) g_pen = *pen;
    if (touch) g_touch = *touch;
}

const InputCaps *input_caps(void) { return &g_caps; }
void input_watch_fd(int fd) { g_watch_fd = fd; }
// The simulated panel is the logical screen itself: mouse and script coordinates are already
// in rotated screen space, so these are identities. An uncalibrated mapping follows the turned
// screen's size (display_set_orientation ran first), or after a turn every coordinate would land
// scaled by the old size (x by 0.75, y by 1.33).
void input_set_orientation(int quarter_turns) {
    (void)quarter_turns;
    if (!g_pen.valid) input_default_calib(&g_pen, NULL);
    if (!g_touch.valid) input_default_calib(NULL, &g_touch);
}
void input_logical_to_panel(float lx, float ly, float *px, float *py) {
    *px = lx;
    *py = ly;
}

// The sim reports "device" coordinates as screen/size so calibration math is exercised.
static InEvent make_ev(InType t, float sx, float sy, bool pen) {
    InEvent e = {0};
    e.type = t;
    e.nx = sx / (float)g_w;
    e.ny = sy / (float)g_h;
    const InputCalib *c = pen ? &g_pen : &g_touch;
    calib_apply(c, e.nx, e.ny, &e.x, &e.y);
    e.t_ms = now_ms();
    return e;
}

// ---- display ----

static const Canvas *g_last_screen;
static bool g_sim_invert;  // display_set_invert
static uint8_t *g_eink;  // what the simulated panel shows
static int g_stats[3];
static long long g_area[3];
#ifdef GRIMOIRE_SIM_SDL
static SDL_Window *g_win;
static SDL_Renderer *g_ren;
static SDL_Texture *g_tex;
#endif

static void parse_size(void) {
    const char *s = getenv("GRIMOIRE_SIM_SIZE");
    int w, h;
    if (s && sscanf(s, "%dx%d", &w, &h) == 2 && w > 100 && h > 100) {
        g_w = w;
        g_h = h;
    }
}

int display_open(int *w, int *h, float *dpi) {
    parse_size();
    g_headless = getenv("GRIMOIRE_SCRIPT") != NULL;
    g_eink = malloc((size_t)g_w * g_h);
    if (!g_eink) return -1;
    memset(g_eink, 255, (size_t)g_w * g_h);
    *w = g_w;
    *h = g_h;
    *dpi = 300.f;
#ifndef GRIMOIRE_SIM_SDL
    if (!g_headless) {
        fprintf(stderr, "grimoire-headless needs GRIMOIRE_SCRIPT=<file> (use grimoire-sim for a window)\n");
        return -1;
    }
#endif
    if (g_headless) return 0;
#ifdef GRIMOIRE_SIM_SDL
    if (!SDL_Init(SDL_INIT_VIDEO)) {
        fprintf(stderr, "SDL_Init: %s\n", SDL_GetError());
        return -1;
    }
    const SDL_DisplayMode *dm = SDL_GetCurrentDisplayMode(SDL_GetPrimaryDisplay());
    if (dm) g_scale = fminf(0.5f, (float)(dm->h - 80) / (float)g_h);
    g_win = SDL_CreateWindow("Grimoire (sim)", (int)(g_w * g_scale), (int)(g_h * g_scale), 0);
    g_ren = g_win ? SDL_CreateRenderer(g_win, NULL) : NULL;
    g_tex = g_ren ? SDL_CreateTexture(g_ren, SDL_PIXELFORMAT_RGB24, SDL_TEXTUREACCESS_STREAMING, g_w, g_h) : NULL;
    if (g_tex) SDL_SetTextureScaleMode(g_tex, SDL_SCALEMODE_LINEAR);
    return (g_win && g_ren && g_tex) ? 0 : -1;
#else
    return -1;
#endif
}

static void sdl_show(void) {
#ifdef GRIMOIRE_SIM_SDL
    if (g_headless || !g_tex) return;
    void *pix;
    int pitch;
    if (!SDL_LockTexture(g_tex, NULL, &pix, &pitch)) return;
    for (int y = 0; y < g_h; y++) {
        uint8_t *row = (uint8_t *)pix + y * pitch;
        const uint8_t *src = g_eink + (size_t)y * g_w;
        for (int x = 0; x < g_w; x++) {
            // A touch of warm paper tint so it reads like e-ink, not a white LCD.
            uint8_t v = src[x];
            row[3 * x] = (uint8_t)(v * 236 / 255 + 6);
            row[3 * x + 1] = (uint8_t)(v * 232 / 255 + 8);
            row[3 * x + 2] = (uint8_t)(v * 222 / 255 + 10);
        }
    }
    SDL_UnlockTexture(g_tex);
    SDL_RenderClear(g_ren);
    SDL_RenderTexture(g_ren, g_tex, NULL, NULL);
    SDL_RenderPresent(g_ren);
#endif
}

// Same 8x8 ordered dither as display_fbink.c, so simulator shots look like the device.
static uint8_t sim_dither(uint8_t v, int x, int y) {
    static const uint8_t b[64] = {0,  32, 8,  40, 2,  34, 10, 42, 48, 16, 56, 24, 50, 18, 58, 26,
                                  12, 44, 4,  36, 14, 46, 6,  38, 60, 28, 52, 20, 62, 30, 54, 22,
                                  3,  35, 11, 43, 1,  33, 9,  41, 51, 19, 59, 27, 49, 17, 57, 25,
                                  15, 47, 7,  39, 13, 45, 5,  37, 63, 31, 55, 23, 61, 29, 53, 21};
    if (v == 0 || v == 255) return v;
    return v > b[((y & 7) << 3) | (x & 7)] * 4 + 2 ? 255 : 0;
}

void display_present(const Canvas *screen, Rect r, RefreshMode mode) {
    g_last_screen = screen;
    r = rect_intersect(r, (Rect){0, 0, g_w, g_h});
    if (rect_is_empty(r) || mode < 0 || mode > 2) return;
    g_stats[mode]++;
    g_area[mode] += (long long)r.w * r.h;
    for (int y = r.y; y < r.y + r.h; y++) {
        const uint8_t *s = screen->px + (size_t)y * screen->stride;
        uint8_t *d = g_eink + (size_t)y * g_w;
        for (int x = r.x; x < r.x + r.w; x++) {
            d[x] = mode == REFRESH_FAST ? sim_dither(s[x], x, y) : s[x];  // DU is 1-bit
            if (g_sim_invert) d[x] = (uint8_t)(255 - d[x]);
        }
    }
    sdl_show();
}

void display_wait(void) {}

void display_set_orientation(int turns, int *w, int *h) {
    // Portrait size is whichever of the configured dimensions is the narrow one.
    int pw = g_w < g_h ? g_w : g_h, ph = g_w < g_h ? g_h : g_w;
    g_w = (turns & 1) ? ph : pw;
    g_h = (turns & 1) ? pw : ph;
    free(g_eink);
    g_eink = malloc((size_t)g_w * g_h);
    if (g_eink) memset(g_eink, 255, (size_t)g_w * g_h);
    *w = g_w;
    *h = g_h;
}
void display_pen_mode(bool on) { (void)on; }
void display_set_invert(bool on) { g_sim_invert = on; }

// Simulated front light (the simulator has no light to drive).
#include "light.h"
static int g_light_i = 12, g_light_w = 8;
bool light_available(void) { return true; }
void light_get(int *i, int *w) {
    *i = g_light_i;
    *w = g_light_w;
}
void light_set_intensity(int v) { g_light_i = clampi(v, 0, LIGHT_MAX); }
void light_set_warmth(int v) { g_light_w = clampi(v, 0, LIGHT_MAX); }
// Simulated sleep: the "power service" answers a sleep request straight away.
static void q_push(InEvent e);
void sys_request_sleep(void) {
    InEvent e = {0};
    e.type = IN_SLEEP;
    e.value = 2;
    e.t_ms = now_ms();
    q_push(e);
}
void sys_kindle_ui(bool run) { (void)run; }
void sys_prevent_sleep(bool prevent) { (void)prevent; }
void sys_activity(void) {}
bool display_overwritten(const Canvas *screen) { (void)screen; return false; }
bool display_find_overwrite(const Canvas *screen, Rect skip, Rect *where) {
    (void)screen;
    (void)skip;
    *where = RECT_EMPTY;
    return false;
}
bool display_band_overwritten(const Canvas *screen, Rect band, Rect skip) {
    (void)screen;
    (void)band;
    (void)skip;
    return false;
}

static void expect_finish(void);
void display_close(void) {
    if (g_headless || getenv("GRIMOIRE_SIM_STATS"))
        fprintf(stderr, "refreshes: fast %d (%.1f MP), ui %d (%.1f MP), full %d (%.1f MP)\n", g_stats[0],
                g_area[0] / 1e6, g_stats[1], g_area[1] / 1e6, g_stats[2], g_area[2] / 1e6);
#ifdef GRIMOIRE_SIM_SDL
    if (g_tex) SDL_DestroyTexture(g_tex);
    if (g_ren) SDL_DestroyRenderer(g_ren);
    if (g_win) SDL_DestroyWindow(g_win);
    if (!g_headless) SDL_Quit();
#endif
    free(g_eink);
    expect_finish();  // exits with status 3 if a check failed
}

// ---- script mode ----

static FILE *g_script;
static uint32_t g_wait_until;
static char g_pending_shot[128];
static bool g_script_done;
static float g_px, g_py;  // last pen position
static bool g_side, g_eraser;

static void shot(const char *name) {
    if (!g_last_screen) return;
    const char *dir = getenv("GRIMOIRE_SHOTS");
    char path[700];
    snprintf(path, sizeof path, "%s/%s.png", dir && *dir ? dir : "shots", name);
    mkdir_p(dir && *dir ? dir : "shots");
    // The panel state (with DU thresholding where it applies) is what a user would see.
    stbi_write_png(path, g_w, g_h, 1, g_eink, g_w);
    fprintf(stderr, "shot: %s\n", path);
}

static void pen_ev(InType t, float x, float y, float p, float tx, float ty, uint32_t dt) {
    InEvent e = make_ev(t, x, y, true);
    e.pressure = p;
    e.tilt_x = tx;
    e.tilt_y = ty;
    e.eraser = g_eraser;
    e.side_button = g_side;
    e.t_ms += dt;
    q_push(e);
    g_px = x;
    g_py = y;
}

static void touch_ev(InType t, float x, float y) { q_push(make_ev(t, x, y, false)); }

// ---- expect: checks in a script ----
//
// "expect KEY is VALUE" (exact), "expect KEY has TEXT" (substring), "expect KEY not VALUE". The
// value is the rest of the line after one space; nothing there means the empty string. KEY is
// anything app_probe() answers (docs/SIM.md lists them). A check sees the app once everything
// queued before it was handled and the screen flushed: right away after a line that ends a batch
// (a tap, wait or shot), else at the next poll, like a shot. A failed check is printed and the
// run goes on; at the end the run exits with status 3 (display_close).

typedef enum { EXPECT_IS, EXPECT_HAS, EXPECT_NOT } ExpectOp;
static const char *const k_expect_ops[] = {"is", "has", "not"};

static struct {
    bool pending;  // read, not checked yet
    bool bad;      // the line could not be read: it fails when its turn comes
    int line;
    ExpectOp op;
    char key[64];
    char value[512];
} g_expect;
static int g_line_no;  // the script line being read (1-based)
static int g_expect_total, g_expect_failed;

// Reads "expect KEY OP VALUE" (s starts at "expect") into g_expect.
static void expect_read(const char *s) {
    memset(&g_expect, 0, sizeof g_expect);
    g_expect.pending = true;
    g_expect.line = g_line_no;
    const char *k = s + 6;
    while (*k == ' ' || *k == '\t') k++;
    size_t kl = strcspn(k, " \t\r\n");
    const char *o = k + kl;
    while (*o == ' ' || *o == '\t') o++;
    size_t ol = strcspn(o, " \t\r\n");
    const char *v = o + ol;
    if (*v == ' ' || *v == '\t') v++;  // the one space before the value
    size_t vl = strcspn(v, "\r\n");
    int op = -1;
    for (int i = 0; i < 3; i++)
        if (ol == strlen(k_expect_ops[i]) && !strncmp(o, k_expect_ops[i], ol)) op = i;
    if (kl == 0 || kl >= sizeof g_expect.key || op < 0 || vl >= sizeof g_expect.value) {
        g_expect.bad = true;
        return;
    }
    memcpy(g_expect.key, k, kl);
    memcpy(g_expect.value, v, vl);
    g_expect.op = (ExpectOp)op;
}

static void expect_check(void) {
    g_expect.pending = false;
    g_expect_total++;
    char got[512] = "";
    if (g_expect.bad) {
        fprintf(stderr, "expect failed (line %d): cannot read it (expect KEY is|has|not VALUE)\n", g_expect.line);
    } else if (app_probe(g_expect.key, got, sizeof got) != 0) {
        fprintf(stderr, "expect failed (line %d): unknown key \"%s\"\n", g_expect.line, g_expect.key);
    } else {
        bool ok = g_expect.op == EXPECT_IS    ? !strcmp(got, g_expect.value)
                  : g_expect.op == EXPECT_HAS ? strstr(got, g_expect.value) != NULL
                                              : strcmp(got, g_expect.value) != 0;
        if (ok) return;
        fprintf(stderr, "expect failed (line %d): %s is \"%s\", wanted %s \"%s\"\n", g_expect.line, g_expect.key, got,
                k_expect_ops[g_expect.op], g_expect.value);
    }
    g_expect_failed++;
}

static void expect_not_run(int line) {
    fprintf(stderr, "expect failed (line %d): not run (the app quit before the script got here)\n", line);
    g_expect_total++;
    g_expect_failed++;
}

// The app quit before the script ended (Library > Exit, say): the checks it never got to fail
// too, or a tap that lands on Exit by mistake would let the rest of a script pass unseen.
static void expect_unrun(void) {
    if (g_expect.pending) {
        g_expect.pending = false;
        expect_not_run(g_expect.line);
    }
    char line[512];
    while (g_script && fgets(line, sizeof line, g_script)) {
        g_line_no++;
        const char *s = line;
        while (isspace((unsigned char)*s)) s++;
        if (!strncmp(s, "expect", 6) && (!s[6] || isspace((unsigned char)s[6]))) expect_not_run(g_line_no);
    }
}

static void expect_finish(void) {
    if (!g_expect_total) return;
    if (!g_expect_failed) {
        fprintf(stderr, "expect: all %d checks passed\n", g_expect_total);
        return;
    }
    fprintf(stderr, "expect: %d of %d checks failed\n", g_expect_failed, g_expect_total);
    exit(3);
}

// Reads script lines until a batch boundary (wait/shot/end). Returns false at EOF.
static void script_step(void) {
    char line[512];
    while (fgets(line, sizeof line, g_script)) {
        g_line_no++;
        char *s = line;
        while (isspace((unsigned char)*s)) s++;
        if (!*s || *s == '#') continue;
        char a[16] = {0}, b[64] = {0};
        float v[10] = {0};
        int n = sscanf(s, "%15s %63s", a, b);
        if (n < 1) continue;
        if (!strcmp(a, "pen") || !strcmp(a, "eraser")) {
            g_eraser = !strcmp(a, "eraser");
            int k = sscanf(s, "%*s %*s %f %f %f %f %f %f %f %f %f %f", &v[0], &v[1], &v[2], &v[3], &v[4], &v[5], &v[6],
                           &v[7], &v[8], &v[9]);
            if (!strcmp(b, "down")) pen_ev(IN_PEN_DOWN, v[0], v[1], k > 2 ? v[2] : 0.6f, v[3], v[4], 0);
            else if (!strcmp(b, "move")) pen_ev(IN_PEN_MOVE, v[0], v[1], k > 2 ? v[2] : 0.6f, v[3], v[4], 0);
            else if (!strcmp(b, "up")) pen_ev(IN_PEN_UP, v[0], v[1], 0.f, 0, 0, 0);
            else if (!strcmp(b, "line")) {
                // x0 y0 x1 y1 steps p0 p1 [tx ty]
                int steps = k > 4 ? (int)v[4] : 20;
                if (steps < 1) steps = 1;
                float p0 = k > 5 ? v[5] : 0.6f, p1 = k > 6 ? v[6] : p0;
                for (int i = 0; i <= steps; i++) {
                    float f = (float)i / steps;
                    pen_ev(i ? IN_PEN_MOVE : IN_PEN_DOWN, lerpf(v[0], v[2], f), lerpf(v[1], v[3], f), lerpf(p0, p1, f),
                           v[7], v[8], (uint32_t)i * 5);
                }
                pen_ev(IN_PEN_UP, v[2], v[3], 0, 0, 0, (uint32_t)(steps + 1) * 5);
            } else if (!strcmp(b, "curve")) {
                // cx cy r steps p [turns] [tx ty]: a loop like cursive handwriting
                int steps = k > 3 ? (int)v[3] : 60;
                float p = k > 4 ? v[4] : 0.6f, turns = k > 5 && v[5] > 0 ? v[5] : 1.f;
                for (int i = 0; i <= steps; i++) {
                    float f = (float)i / steps, ang = f * turns * 6.2831853f;
                    float x = v[0] + v[2] * (cosf(ang) - 1.f) + f * v[2] * 3.f * turns, y = v[1] + v[2] * sinf(ang);
                    pen_ev(i ? IN_PEN_MOVE : IN_PEN_DOWN, x, y, p * (0.7f + 0.3f * sinf(f * 3.14159f)), v[6], v[7],
                           (uint32_t)i * 5);
                }
                pen_ev(IN_PEN_UP, g_px, g_py, 0, 0, 0, (uint32_t)(steps + 1) * 5);
            }
            g_eraser = false;
        } else if (!strcmp(a, "side")) {
            g_side = !strcmp(b, "on");
            InEvent e = make_ev(IN_PEN_BUTTON, g_px, g_py, true);
            e.side_button = g_side;
            q_push(e);
        } else if (!strcmp(a, "touch")) {
            sscanf(s, "%*s %*s %f %f", &v[0], &v[1]);
            touch_ev(!strcmp(b, "down") ? IN_TOUCH_DOWN : !strcmp(b, "move") ? IN_TOUCH_MOVE : IN_TOUCH_UP, v[0], v[1]);
        } else if (!strcmp(a, "tap") || !strcmp(a, "pentap")) {
            sscanf(s, "%*s %f %f", &v[0], &v[1]);
            if (!strcmp(a, "tap")) {
                touch_ev(IN_TOUCH_DOWN, v[0], v[1]);
                touch_ev(IN_TOUCH_UP, v[0], v[1]);
            } else {
                pen_ev(IN_PEN_DOWN, v[0], v[1], 0.5f, 0, 0, 0);
                pen_ev(IN_PEN_UP, v[0], v[1], 0, 0, 0, 20);
            }
            return;  // one tap per batch, like a human
        } else if (!strcmp(a, "drag")) {
            // drag pen|touch x0 y0 x1 y1 steps
            sscanf(s, "%*s %*s %f %f %f %f %f", &v[0], &v[1], &v[2], &v[3], &v[4]);
            bool pen = !strcmp(b, "pen");
            int steps = v[4] > 0 ? (int)v[4] : 10;
            for (int i = 0; i <= steps; i++) {
                float f = (float)i / steps;
                float x = lerpf(v[0], v[2], f), y = lerpf(v[1], v[3], f);
                if (pen) pen_ev(i ? IN_PEN_MOVE : IN_PEN_DOWN, x, y, 0.5f, 0, 0, (uint32_t)i * 100);
                else {
                    InEvent e = make_ev(i ? IN_TOUCH_MOVE : IN_TOUCH_DOWN, x, y, false);
                    e.t_ms += (uint32_t)i * 100;
                    q_push(e);
                }
            }
            if (pen) pen_ev(IN_PEN_UP, v[2], v[3], 0, 0, 0, (uint32_t)(steps + 1) * 100);
            else {
                InEvent e = make_ev(IN_TOUCH_UP, v[2], v[3], false);
                e.t_ms += (uint32_t)(steps + 1) * 100;
                q_push(e);
            }
            return;
        } else if (!strcmp(a, "wait")) {
            g_wait_until = now_ms() + (uint32_t)atoi(b);
            return;
        } else if (!strcmp(a, "shot")) {
            strlcpy_q(g_pending_shot, b, sizeof g_pending_shot);
            return;
        } else if (!strcmp(a, "expect")) {
            expect_read(s);
            // Nothing queued since the last flush: check now and read on (no extra refresh or
            // timer tick slips in). Else once this batch is handled and flushed, like a shot.
            if (g_qh == g_qt) {
                expect_check();
                continue;
            }
            return;
        } else if (!strcmp(a, "rotate")) {
            InEvent e = make_ev(IN_ORIENTATION, 0, 0, true);
            e.value = atoi(b) & 3;
            q_push(e);
            return;
        } else if (!strcmp(a, "pinch")) {
            // pinch CX CY D0 D1 STEPS [DX DY]: two fingers around (CX,CY), spread D0 -> D1,
            // the midpoint drifting by (DX,DY): zoom and pan in one gesture.
            int k = sscanf(s, "%*s %f %f %f %f %f %f %f", &v[0], &v[1], &v[2], &v[3], &v[4], &v[5], &v[6]);
            int steps = k > 4 && v[4] > 0 ? (int)v[4] : 8;
            for (int i = 0; i <= steps; i++) {
                float f = (float)i / steps, d = lerpf(v[2], v[3], f) * 0.5f;
                float cx = v[0] + (k > 5 ? v[5] : 0) * f, cy = v[1] + (k > 6 ? v[6] : 0) * f;
                InEvent e = make_ev(i ? IN_PINCH_MOVE : IN_PINCH_BEGIN, cx - d, cy, false);
                e.x2 = cx + d;
                e.y2 = cy;
                e.t_ms += (uint32_t)i * 160;
                q_push(e);
            }
            InEvent e = make_ev(IN_PINCH_END, 0, 0, false);
            e.t_ms += (uint32_t)(steps + 1) * 160;
            q_push(e);
            return;
        } else if (!strcmp(a, "sleep") || !strcmp(a, "wake")) {
            InEvent e = make_ev(!strcmp(a, "sleep") ? IN_SLEEP : IN_WAKE, 0, 0, true);
            e.value = !strcmp(a, "sleep") ? 2 : 1;
            q_push(e);
            return;
        } else if (!strcmp(a, "power")) {
            q_push(make_ev(IN_POWER, 0, 0, true));
        } else if (!strcmp(a, "quit")) {
            q_push(make_ev(IN_QUIT, 0, 0, true));
            g_script_done = true;
            return;
        } else {
            fprintf(stderr, "script: unknown command: %s", s);
        }
    }
    if (!g_script_done) {
        g_script_done = true;
        q_push(make_ev(IN_QUIT, 0, 0, true));
    }
}

// ---- input ----

static float g_pressure = 0.6f;
#ifdef GRIMOIRE_SIM_SDL
static float g_tx, g_ty;
static bool g_mouse_down, g_finger_down;
#endif

int input_open(int w, int h, const InputCalib *pen, const InputCalib *touch) {
    input_default_calib(&g_pen, &g_touch);
    if (pen) g_pen = *pen;
    if (touch) g_touch = *touch;
    const char *sp = getenv("GRIMOIRE_SIM_PRESSURE");
    if (sp) g_pressure = clampf((float)atof(sp), 0.f, 1.f);
    const char *script = getenv("GRIMOIRE_SCRIPT");
    if (script) {
        g_script = fopen(script, "r");
        if (!g_script) {
            fprintf(stderr, "cannot open script %s\n", script);
            return -1;
        }
    }
    return 0;
}

void input_close(void) {
    expect_unrun();
    if (g_script) fclose(g_script);
    g_script = NULL;
}

#ifdef GRIMOIRE_SIM_SDL
static void update_title(void) {
    if (!g_win) return;
    char t[200];
    snprintf(t, sizeof t, "Grimoire (sim)  pressure %.1f  tilt %.0f,%.0f  [1-9,0 pressure, arrows tilt, hold E eraser, S side, P power]",
             g_pressure, g_tx, g_ty);
    SDL_SetWindowTitle(g_win, t);
}

static void sdl_pump(void) {
    SDL_Event e;
    const bool *keys = SDL_GetKeyboardState(NULL);
    while (SDL_PollEvent(&e)) {
        switch (e.type) {
        case SDL_EVENT_QUIT: q_push(make_ev(IN_QUIT, 0, 0, true)); break;
        case SDL_EVENT_KEY_DOWN: {
            SDL_Keycode k = e.key.key;
            if (k == SDLK_ESCAPE || (k == SDLK_Q && (e.key.mod & SDL_KMOD_GUI))) q_push(make_ev(IN_QUIT, 0, 0, true));
            else if (k >= SDLK_1 && k <= SDLK_9) g_pressure = (float)(k - SDLK_0) / 10.f;
            else if (k == SDLK_0) g_pressure = 1.f;
            else if (k == SDLK_LEFT) g_tx -= 5;
            else if (k == SDLK_RIGHT) g_tx += 5;
            else if (k == SDLK_UP) g_ty -= 5;
            else if (k == SDLK_DOWN) g_ty += 5;
            else if (k == SDLK_P) q_push(make_ev(IN_POWER, 0, 0, true));
            else if (k == SDLK_S && !e.key.repeat) {
                InEvent b = make_ev(IN_PEN_BUTTON, 0, 0, true);
                b.side_button = true;
                q_push(b);
            }
            g_tx = clampf(g_tx, -60, 60);
            g_ty = clampf(g_ty, -60, 60);
            update_title();
            break;
        }
        case SDL_EVENT_MOUSE_WHEEL: {
            // Wheel = a small pinch around the pointer (zoom in / out).
            float mx, my;
            SDL_GetMouseState(&mx, &my);
            float cx = mx / g_scale, cy = my / g_scale, d = 300.f, d1 = e.wheel.y > 0 ? d * 1.15f : d / 1.15f;
            InEvent b = make_ev(IN_PINCH_BEGIN, cx - d / 2, cy, false);
            b.x2 = cx + d / 2;
            b.y2 = cy;
            q_push(b);
            InEvent m = make_ev(IN_PINCH_MOVE, cx - d1 / 2, cy, false);
            m.x2 = cx + d1 / 2;
            m.y2 = cy;
            q_push(m);
            q_push(make_ev(IN_PINCH_END, 0, 0, false));
            break;
        }
        case SDL_EVENT_KEY_UP:
            if (e.key.key == SDLK_S) {
                InEvent b = make_ev(IN_PEN_BUTTON, 0, 0, true);
                b.side_button = false;
                q_push(b);
            }
            break;
        case SDL_EVENT_MOUSE_BUTTON_DOWN:
        case SDL_EVENT_MOUSE_BUTTON_UP:
        case SDL_EVENT_MOUSE_MOTION: {
            bool motion = e.type == SDL_EVENT_MOUSE_MOTION;
            float x = (motion ? e.motion.x : e.button.x) / g_scale, y = (motion ? e.motion.y : e.button.y) / g_scale;
            bool right = !motion && e.button.button == SDL_BUTTON_RIGHT;
            InType t;
            bool pen = true;
            if (e.type == SDL_EVENT_MOUSE_BUTTON_DOWN) {
                if (right) {
                    t = IN_TOUCH_DOWN;
                    pen = false;
                    g_finger_down = true;
                } else {
                    t = IN_PEN_DOWN;
                    g_mouse_down = true;
                }
            } else if (e.type == SDL_EVENT_MOUSE_BUTTON_UP) {
                if (right) {
                    t = IN_TOUCH_UP;
                    pen = false;
                    g_finger_down = false;
                } else {
                    t = IN_PEN_UP;
                    g_mouse_down = false;
                }
            } else if (g_finger_down) {
                t = IN_TOUCH_MOVE;
                pen = false;
            } else
                t = g_mouse_down ? IN_PEN_MOVE : IN_PEN_HOVER;
            InEvent ev = make_ev(t, x, y, pen);
            if (pen) {
                ev.pressure = (t == IN_PEN_UP || t == IN_PEN_HOVER) ? 0.f : g_pressure;
                ev.tilt_x = g_tx;
                ev.tilt_y = g_ty;
                ev.eraser = keys[SDL_SCANCODE_E];
                ev.side_button = keys[SDL_SCANCODE_S];
            }
            q_push(ev);
            break;
        }
        case SDL_EVENT_WINDOW_EXPOSED: sdl_show(); break;
        default: break;
        }
    }
}
#endif

int input_poll(InEvent *ev, int timeout_ms) {
    if (q_pop(ev)) return 1;
    if (g_headless) {
        if (timeout_ms == 0) return 0;  // end of the current batch: the app flushes next
        // A shot requested by the script is taken here, after that flush, so it shows exactly
        // what the panel shows once the batch's refresh went out.
        if (g_pending_shot[0]) {
            shot(g_pending_shot);
            g_pending_shot[0] = 0;
        }
        if (g_expect.pending) expect_check();  // an expect waits for the same moment
        if ((int32_t)(now_ms() - g_wait_until) < 0) {
            uint32_t left = g_wait_until - now_ms();
            sleep_ms((int)(left < (uint32_t)timeout_ms ? left : (uint32_t)timeout_ms));
            return 0;
        }
        if (!g_script_done) script_step();
        return q_pop(ev) ? 1 : 0;
    }
#ifdef GRIMOIRE_SIM_SDL
    uint32_t until = now_ms() + (uint32_t)(timeout_ms < 0 ? 1000000 : timeout_ms);
    for (;;) {
        sdl_pump();
        if (q_pop(ev)) return 1;
        if (timeout_ms == 0 || (int32_t)(now_ms() - until) >= 0) return 0;
        SDL_Delay(4);
    }
#else
    return 0;
#endif
}
