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
#include <sys/wait.h>

#include "app.h"  // app_probe, for the script's expect checks
#include "bgjob.h"  // bg_busy, for waitbg
#include "calib_fit.h"
#include "display.h"
#include "hidkeys.h"
#include "input.h"
#include "power.h"
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
static int g_rot;  // quarter turns clockwise the screen is turned from the panel (input_set_orientation)

// The simulated panel is the screen as it is upright: its sides swap while the Kindle is sideways.
static void panel_size(float *pw, float *ph) {
    *pw = (float)((g_rot & 1) ? g_h : g_w);
    *ph = (float)((g_rot & 1) ? g_w : g_h);
}

void input_default_calib(InputCalib *pen, InputCalib *touch) {
    float pw, ph;
    panel_size(&pw, &ph);
    InputCalib c = {false, false, pw, 0.f, ph, 0.f};
    if (pen) *pen = c;
    if (touch) *touch = c;
}

void input_set_calib(const InputCalib *pen, const InputCalib *touch) {
    if (pen) g_pen = *pen;
    if (touch) g_touch = *touch;
}

const InputCaps *input_caps(void) { return &g_caps; }
static bool g_shortcuts_enabled;
void input_shortcuts_enable(bool enabled) { g_shortcuts_enabled = enabled; }
// The script's keyboards and controllers ("hid attach", "hid detach"): connected whether or not
// shortcuts are on, but watched, listed and announced only while they are, as on the Kindle.
static InputHid g_hids[4];
static int g_nhids;
int input_hids(InputHid *out, int cap) {
    int n = 0;
    for (int i = 0; g_shortcuts_enabled && i < g_nhids && n < cap; i++) out[n++] = g_hids[i];
    return n;
}
void input_watch_fd(int fd) { g_watch_fd = fd; }
// Panel <-> screen for a Kindle turned g_rot quarter turns clockwise, as in input_evdev.c.
// display_set_orientation has already swapped g_w and g_h when this is set.
void input_set_orientation(int quarter_turns) { g_rot = quarter_turns & 3; }
static void panel_to_logical(float px, float py, float *lx, float *ly) {
    float pw, ph;
    panel_size(&pw, &ph);
    float W = pw - 1.f, H = ph - 1.f;
    switch (g_rot & 3) {
    case 1: *lx = H - py; *ly = px; break;
    case 2: *lx = W - px; *ly = H - py; break;
    case 3: *lx = py; *ly = W - px; break;
    default: *lx = px; *ly = py; break;
    }
}
void input_logical_to_panel(float lx, float ly, float *px, float *py) {
    float pw, ph;
    panel_size(&pw, &ph);
    float W = pw - 1.f, H = ph - 1.f;
    switch (g_rot & 3) {
    case 1: *px = ly; *py = H - lx; break;
    case 2: *px = W - lx; *py = H - ly; break;
    case 3: *px = W - ly; *py = lx; break;
    default: *px = lx; *py = ly; break;
    }
}

// Mouse and script coordinates are screen pixels. Like the Kindle's digitizer, the sim reports
// "device" coordinates as a place on the upright panel (0..1), which the calibration maps to
// panel pixels, turned to the screen after: a calibration made upright stays right sideways, and
// the calibration math is exercised.
static InEvent make_ev(InType t, float sx, float sy, bool pen) {
    InEvent e = {0};
    e.type = t;
    float px, py, pw, ph;
    input_logical_to_panel(sx, sy, &px, &py);
    panel_size(&pw, &ph);
    e.nx = px / pw;
    e.ny = py / ph;
    calib_apply(pen ? &g_pen : &g_touch, e.nx, e.ny, &px, &py);
    panel_to_logical(px, py, &e.x, &e.y);
    e.t_ms = now_ms();
    return e;
}

// ---- display ----

static const Canvas *g_last_screen;
static bool g_sim_invert;  // display_set_invert
static uint8_t *g_eink;  // what the simulated panel shows
static int g_stats[3];
static int g_ink_frames;
static long long g_area[3];
// Live ink as display_fbink.c keeps it, for the script's display.* checks: whether a colour panel
// would wait after the last fast refresh, and whether the app asks for the pen mode.
static LiveInk g_ink;
static int g_last_fast = -1;  // the last fast refresh: 1 live ink, 0 one to wait after, -1 none yet
static bool g_pen_mode;       // display_pen_mode
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
    if (mode == REFRESH_FAST) g_last_fast = live_ink_frame(&g_ink, mode, now_ms()) ? 1 : 0;
    g_stats[mode]++;
    if (mode == REFRESH_FAST && g_ink.on) g_ink_frames++;
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
void display_live_ink(bool on) { live_ink_mark(&g_ink, on, now_ms()); }
void display_pen_mode(bool on) {
    display_live_ink(on);
    g_pen_mode = on;
}
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
// The simulated power service's state, what sys_powerd_state reports: set by the script's
// `powerd <state>` ("unknown" reads as unreadable). It never changes on its own.
static char g_powerd_state[32] = "active";
int sys_powerd_state(char *out, size_t cap) {
    if (!g_powerd_state[0] || !strcmp(g_powerd_state, "unknown")) return -1;
    strlcpy_q(out, g_powerd_state, cap);
    return 0;
}
// Simulated sleep: the "power service" answers a sleep request straight away, like powerd takes a
// power button: an awake Kindle goes to sleep, one already in its screensaver (by `powerd`) wakes.
static void q_push(InEvent e);
void sys_request_sleep(void) {
    bool dozing = powerd_dozing(powerd_parse_state(g_powerd_state));
    if (dozing) fprintf(stderr, "sim: powerButton 1 while the power service is in %s: it wakes the Kindle\n", g_powerd_state);
    InEvent e = {0};
    e.type = dozing ? IN_WAKE : IN_SLEEP;
    e.value = dozing ? 1 : 2;
    e.t_ms = now_ms();
    q_push(e);
}
bool sys_kindle_ui(bool run) { (void)run; return true; }
static bool g_sim_grab = true;
bool input_set_grab(bool grab) { g_sim_grab = grab; return true; }
void sys_prevent_sleep(bool prevent) { (void)prevent; }
// How often the app told the power service someone is here: the runner's "power.pokes".
static int g_pokes;
void sys_activity(void) { g_pokes++; }
bool display_broken(void) { return false; }
void display_skip_overlay(Rect r) { (void)r; }

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

// A headless run ends once the PDF exports it started are written (bgjob.c leaves them to child
// processes that outlive Grimoire), so what reads its data next (a scenario's "# root:" copy,
// share_check.sh, the exporter) never meets a file half-written. A minute at most.
static void wait_for_exports(void) {
    bool said = false;
    for (int ms = 0; ms < 60000;) {
        pid_t r = waitpid(-1, NULL, WNOHANG);
        if (r < 0) return;     // no child left
        if (r > 0) continue;   // one ended: any more?
        if (!said) fprintf(stderr, "headless: waiting for the PDF exports to finish\n");
        said = true;
        sleep_ms(20);
        ms += 20;
    }
    fprintf(stderr, "headless: an export still runs after a minute; not waiting for it\n");
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
    if (g_headless) wait_for_exports();
    expect_finish();  // exits with status 3 if a check failed
}

// ---- script mode ----

static FILE *g_script;
static uint32_t g_wait_until;
static bool g_script_hold; // external test driver owns lifetime after the hold command
static bool g_wait_bg;  // waitbg: the wait ends early once no background job is queued or running
static char g_pending_shot[128];
static bool g_script_done;
static float g_px, g_py;  // last pen position
static bool g_side, g_eraser;

// ---- expect: checks in a script ----
//
// "expect KEY is VALUE" (exact), "expect KEY has TEXT" (substring), "expect KEY not VALUE". The
// value is the rest of the line after one space; nothing there means the empty string. KEY is
// anything app_probe() answers (docs/SIM.md lists them), "running": 1 while the app runs, 0
// once it has quit, "watching": 1 while input_poll also watches a file descriptor (the share
// server's), else 0, or a display.* key (display_probe). A check sees the app once everything
// queued before it was handled, ticked and flushed: at once after a line that ends a batch (a
// tap, wait or shot; that is one main-loop pass before a shot in its place would be taken), else
// at the next poll, like a shot. A failed check is printed and the run goes on; at the end the
// run exits with status 3 (display_close).
// A line the runner does not know fails the same way, or a misspelt check would pass unseen.

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
    for (int i = 0; i < (int)(sizeof k_expect_ops / sizeof k_expect_ops[0]); i++)
        if (ol == strlen(k_expect_ops[i]) && !strncmp(o, k_expect_ops[i], ol)) op = i;
    if (kl == 0 || kl >= sizeof g_expect.key || op < 0 || vl >= sizeof g_expect.value) {
        g_expect.bad = true;
        return;
    }
    memcpy(g_expect.key, k, kl);
    memcpy(g_expect.value, v, vl);
    g_expect.op = (ExpectOp)op;
}

// The display.* keys, which the simulated panel answers: "display.ink" 1 while a stroke is drawn
// (display_live_ink), "display.pen_mode" 1 while the app asks for the pen mode (display_pen_mode),
// "display.last_fast" "ink" when the last fast refresh was live ink, "wait" when a colour panel
// would have waited for the driver's read after it (display_fbink.c), empty before any.
static bool display_probe(const char *key, char *out, size_t cap) {
    if (!strcmp(key, "display.ink")) snprintf(out, cap, "%d", g_ink.on ? 1 : 0);
    else if (!strcmp(key, "display.pen_mode")) snprintf(out, cap, "%d", g_pen_mode ? 1 : 0);
    else if (!strcmp(key, "display.ink_frames")) snprintf(out, cap, "%d", g_ink_frames);
    else if (!strcmp(key, "display.last_fast"))
        snprintf(out, cap, "%s", g_last_fast < 0 ? "" : g_last_fast ? "ink" : "wait");
    else return false;
    return true;
}

// "power.pokes": how many times the app told the simulated power service someone is here
// (sys_activity), which it does at most once a minute.
static bool power_probe(const char *key, char *out, size_t cap) {
    if (strcmp(key, "power.pokes")) return false;
    snprintf(out, cap, "%d", g_pokes);
    return true;
}

// Checks the expect just read. gone: the app quit before the script got to it, so only "running"
// has an answer (0); any other key fails as not run.
static void expect_check_as(bool gone) {
    g_expect.pending = false;
    g_expect_total++;
    char got[512] = "";
    bool running = !strcmp(g_expect.key, "running"), watching = !strcmp(g_expect.key, "watching");
    if (g_expect.bad) {
        fprintf(stderr, "expect failed (line %d): cannot read it (expect KEY is|has|not VALUE)\n", g_expect.line);
    } else if (gone && !running) {
        fprintf(stderr, "expect failed (line %d): not run (the app quit before the script got here)\n", g_expect.line);
    } else if (!running && !watching && !display_probe(g_expect.key, got, sizeof got) &&
               !power_probe(g_expect.key, got, sizeof got) && app_probe(g_expect.key, got, sizeof got) != 0) {
        fprintf(stderr, "expect failed (line %d): unknown key \"%s\"\n", g_expect.line, g_expect.key);
    } else {
        if (running) snprintf(got, sizeof got, "%d", gone ? 0 : 1);
        if (watching) snprintf(got, sizeof got, "%d", g_watch_fd >= 0 ? 1 : 0);
        bool ok = g_expect.op == EXPECT_IS    ? !strcmp(got, g_expect.value)
                  : g_expect.op == EXPECT_HAS ? strstr(got, g_expect.value) != NULL
                                              : strcmp(got, g_expect.value) != 0;
        if (ok) return;
        fprintf(stderr, "expect failed (line %d): %s is \"%s\", wanted %s \"%s\"\n", g_expect.line, g_expect.key, got,
                k_expect_ops[g_expect.op], g_expect.value);
    }
    g_expect_failed++;
}

static void expect_check(void) { expect_check_as(false); }

// A line script_step does not know ("expcet", "Expect"): it fails like a check, since a misspelt
// check is a check never made.
static void expect_unknown_line(void) {
    fprintf(stderr, "expect failed (line %d): unknown command\n", g_line_no);
    g_expect_total++;
    g_expect_failed++;
}

// The app quit before the script ended (Library > Exit, say): the checks it never got to fail
// too, or a tap that lands on Exit by mistake would let the rest of a script pass unseen. Where
// a script means the app to quit, "expect running is 0" says so.
static void expect_unrun(void) {
    if (g_expect.pending) expect_check_as(true);
    char line[512];
    while (g_script && fgets(line, sizeof line, g_script)) {
        g_line_no++;
        const char *s = line;
        while (isspace((unsigned char)*s)) s++;
        if (!strncmp(s, "expect", 6) && (!s[6] || isspace((unsigned char)s[6]))) {
            expect_read(s);
            expect_check_as(true);
        }
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

// The simulated power key: one event, its value how long the key was held (IN_POWER_UP and
// IN_POWER_HELD, as input_evdev.c reports them).
static uint32_t g_power_down_at;
static void power_key(InType t, int held_ms) {
    InEvent e = make_ev(t, 0, 0, true);
    e.value = held_ms;
    q_push(e);
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
        } else if (!strcmp(a, "key")) {
            int code = 0, mods = 0, pad = 0;
            sscanf(s, "%*s %d %d %d", &code, &mods, &pad);
            q_push((InEvent){.type = IN_KEY, .value = code, .key_mods = (unsigned)mods, .gamepad = pad != 0, .t_ms = now_ms()});
        } else if (!strcmp(a, "hid")) {
            // hid attach pad|key|held NAME, hid detach NAME: a controller (held: one another app
            // grabs), or a keyboard or page turner, comes or goes, as input_evdev.c reports it
            char kind[16] = "", raw[256] = "";
            bool attach = !strcmp(b, "attach");
            if (attach) sscanf(s, "%*s %*s %15s %255[^\n]", kind, raw);
            else sscanf(s, "%*s %*s %255[^\n]", raw);
            InEvent e = {.type = attach ? IN_HID_ATTACH : IN_HID_DETACH, .t_ms = now_ms()};
            hidkeys_name(e.hid_name, sizeof e.hid_name, raw);
            int i = 0;
            while (i < g_nhids && strcmp(g_hids[i].name, e.hid_name)) i++;
            if (attach && g_nhids < 4 && (!strcmp(kind, "pad") || !strcmp(kind, "key") || !strcmp(kind, "held"))) {
                InputHid h = {.gamepad = strcmp(kind, "key") != 0, .held = !strcmp(kind, "held")};
                strlcpy_q(h.name, e.hid_name, sizeof h.name);
                g_hids[g_nhids++] = h;
                e.gamepad = h.gamepad;
                e.value = h.held;
            } else if (!attach && !strcmp(b, "detach") && i < g_nhids) {
                e.gamepad = g_hids[i].gamepad;
                g_hids[i] = g_hids[--g_nhids];
            } else {
                fprintf(stderr, "script: bad hid line: %s", s);
                expect_unknown_line();
                continue;
            }
            if (g_shortcuts_enabled) q_push(e);
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
            g_wait_bg = false;
            return;
        } else if (!strcmp(a, "hold")) {
            g_script_hold = true;
            return;
        } else if (!strcmp(a, "waitbg")) {
            // Background work (an import, a template's picture, a PDF) takes as long as the machine
            // lets it: wait for it to end, not a time that holds on a quiet machine only.
            g_wait_until = now_ms() + (uint32_t)(*b ? atoi(b) : 60000);
            g_wait_bg = true;
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
            // power: a short press and its release; power down / power up: one half at a time;
            // power held: the 4 s mark of a press that is still down (what input_evdev.c sends
            // then), so "power down", "power held" leaves while the key is down
            if (!strcmp(b, "down")) {
                power_key(IN_POWER, 0);
                g_power_down_at = now_ms();
            } else if (!strcmp(b, "up")) {
                power_key(IN_POWER_UP, (int)(now_ms() - g_power_down_at));
            } else if (!strcmp(b, "held")) {
                power_key(IN_POWER_HELD, INPUT_POWER_HOLD_MS);
            } else {
                power_key(IN_POWER, 0);
                power_key(IN_POWER_UP, 150);
            }
        } else if (!strcmp(a, "powerhold")) {
            // held 4 s: the press, the hold (which leaves Grimoire), then the release
            power_key(IN_POWER, 0);
            power_key(IN_POWER_HELD, INPUT_POWER_HOLD_MS);
            power_key(IN_POWER_UP, INPUT_POWER_HOLD_MS + 200);
            return;
        } else if (!strcmp(a, "storage")) {
            // storage full: every file write fails from now on (a full userstore); storage ok
            g_sim_storage_full = !strcmp(b, "full");
            if (strcmp(b, "full") && strcmp(b, "ok")) expect_unknown_line();
        } else if (!strcmp(a, "powerd")) {
            // powerd STATE: what the power service reports from now on (active, screenSaver, ...)
            strlcpy_q(g_powerd_state, b, sizeof g_powerd_state);
        } else if (!strcmp(a, "quit")) {
            q_push(make_ev(IN_QUIT, 0, 0, true));
            g_script_done = true;
            return;
        } else {
            fprintf(stderr, "script: unknown command: %s", s);
            expect_unknown_line();
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
static bool g_power_held;  // P held long enough: IN_POWER_HELD sent
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
            else if (g_shortcuts_enabled && k != SDLK_P && k != SDLK_S) {
                int code = shortcut_keycode(SDL_GetKeyName(k));
                unsigned mods = 0;
                if (e.key.mod & SDL_KMOD_CTRL) mods |= KEYMOD_CTRL;
                if (e.key.mod & SDL_KMOD_SHIFT) mods |= KEYMOD_SHIFT;
                if (e.key.mod & SDL_KMOD_ALT) mods |= KEYMOD_ALT;
                if (e.key.mod & SDL_KMOD_GUI) mods |= KEYMOD_META;
                if (code > 0 && !e.key.repeat) q_push((InEvent){.type = IN_KEY, .value = code, .key_mods = mods, .t_ms = now_ms()});
            }
            else if (k >= SDLK_1 && k <= SDLK_9) g_pressure = (float)(k - SDLK_0) / 10.f;
            else if (k == SDLK_0) g_pressure = 1.f;
            else if (k == SDLK_LEFT) g_tx -= 5;
            else if (k == SDLK_RIGHT) g_tx += 5;
            else if (k == SDLK_UP) g_ty -= 5;
            else if (k == SDLK_DOWN) g_ty += 5;
            else if (k == SDLK_P && !e.key.repeat) {
                power_key(IN_POWER, 0);
                g_power_down_at = now_ms() | 1;
                g_power_held = false;
            } else if (k == SDLK_S && !e.key.repeat) {
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
            } else if (e.key.key == SDLK_P && g_power_down_at) {
                power_key(IN_POWER_UP, (int)(now_ms() - g_power_down_at));
                g_power_down_at = 0;
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
                ev.eraser = !g_shortcuts_enabled && keys[SDL_SCANCODE_E];
                ev.side_button = keys[SDL_SCANCODE_S];
            }
            q_push(ev);
            break;
        }
        case SDL_EVENT_WINDOW_EXPOSED: sdl_show(); break;
        default: break;
        }
    }
    // Holding P is holding the power button: 4 s leaves Grimoire.
    if (g_power_down_at && !g_power_held && (int32_t)(now_ms() - g_power_down_at) >= INPUT_POWER_HOLD_MS) {
        g_power_held = true;
        power_key(IN_POWER_HELD, INPUT_POWER_HOLD_MS);
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
        if (g_script_hold) { sleep_ms(timeout_ms); return 0; }
        if (g_wait_bg && !bg_busy()) g_wait_bg = false, g_wait_until = now_ms();  // (reaped: its toast is up)
        if ((int32_t)(now_ms() - g_wait_until) < 0) {
            uint32_t left = g_wait_until - now_ms();
            if (g_wait_bg && left > 20) left = 20;  // look again soon
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
