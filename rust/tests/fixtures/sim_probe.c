// sim_probe.c: runs tests/fixtures/sim_probe.qs through the C headless runner's script reading
// (src/sim_sdl.c, included whole so its tally of checks can be read) and prints, to stderr in
// order with the runner's own messages, every event it makes (the floats as their bits), what a
// small stand-in for the app does with them, and the end of the run (display_close: the refresh
// counts and the checks' verdict, exit status 3). rust/src/sim_sdl.rs's sim_probe_child does the
// same with the Rust runner; its test compares the two outputs byte for byte. Nothing here is
// timed: every wait is "wait 0", and the events' times are printed as their offsets from the
// batch's first event, rounded down to 5 ms (the offsets the script asks for are multiples of 5).
//
// Built and run from the top of the tree (the clang of the Makefile's host build: -ffp-contract=on):
//   clang -std=gnu11 -D_GNU_SOURCE -DGRIMOIRE_SIM -Isrc -Ithird_party -O1 -o /tmp/sim_probe \
//     rust/tests/fixtures/sim_probe.c src/util.c src/calib_fit.c src/hidkeys.c src/power.c -lm
//   (mkdir -p /tmp/p && cd /tmp/p && GRIMOIRE_SCRIPT=$OLDPWD/rust/tests/fixtures/sim_probe.qs \
//     GRIMOIRE_SHOTS=shots-sim-probe GRIMOIRE_SIM_SIZE=1200x1600 /tmp/sim_probe) \
//     2> rust/tests/fixtures/sim_probe_c.txt
#include "../../../src/sim_sdl.c"

// Stand-ins for what the runner reaches beyond the files linked: no App (app_probe's g_probe_app is
// NULL until app_init), no background job, no keyboard drawn, and a shot's PNG not written (the
// Rust twin writes it, and its test reads the PNG's header).
int app_probe(const char *key, char *out, size_t cap) {
    (void)key;
    (void)out;
    (void)cap;
    return -1;
}
bool bg_busy(void) { return false; }
unsigned keyboard_draws_take(void) { return 0; }
int stbi_write_png(char const *filename, int w, int h, int comp, const void *data, int stride_in_bytes) {
    (void)filename;
    (void)w;
    (void)h;
    (void)comp;
    (void)data;
    (void)stride_in_bytes;
    return 1;
}
// (util.c's dates reach the catalog; the probe shows none)
const char *i18n_getc(const char *ctx, const char *msgid) {
    (void)ctx;
    return msgid;
}

static uint32_t bits(float f) {
    uint32_t u;
    memcpy(&u, &f, sizeof u);
    return u;
}

static void print_ev(const InEvent *e, uint32_t t0) {
    fprintf(stderr,
            "ev %d x %08x y %08x nx %08x ny %08x p %08x tx %08x ty %08x x2 %08x y2 %08x v %d m %u g %d e %d s %d "
            "dt %u hid \"%s\"\n",
            (int)e->type, bits(e->x), bits(e->y), bits(e->nx), bits(e->ny), bits(e->pressure), bits(e->tilt_x),
            bits(e->tilt_y), bits(e->x2), bits(e->y2), e->value, e->key_mods, e->gamepad, e->eraser, e->side_button,
            (e->t_ms - t0) / 5 * 5, e->hid_name);
}

static Canvas g_canvas;
static void canvas_make(int w, int h) {
    free(g_canvas.px);
    g_canvas.px = malloc((size_t)w * h);
    g_canvas.w = w;
    g_canvas.h = h;
    g_canvas.stride = w;
    for (int y = 0; y < h; y++)
        for (int x = 0; x < w; x++) g_canvas.px[(size_t)y * w + x] = (uint8_t)((x * 7 + y * 13) & 255);
}

// What the app would do, as far as the runner sees it: live ink over a stroke and a fast refresh at
// its end, the turn, the sleep request of a power press, and shortcuts on after F1.
static int g_w, g_h;
static void react(const InEvent *e) {
    if (e->type == IN_PEN_DOWN) display_live_ink(true);
    if (e->type == IN_PEN_UP) {
        Rect r = {(int)e->x - 20, (int)e->y - 20, 40, 40};
        display_present(&g_canvas, r, REFRESH_FAST);
        display_live_ink(false);
    }
    if (e->type == IN_TOUCH_UP) display_present(&g_canvas, (Rect){0, 0, 100, 50}, REFRESH_UI);
    if (e->type == IN_ORIENTATION) {
        display_set_orientation(e->value, &g_w, &g_h);
        input_set_orientation(e->value);
        canvas_make(g_w, g_h);
        InputCalib p, t;
        input_default_calib(&p, &t);
        float px, py;
        input_logical_to_panel(10.f, 20.f, &px, &py);
        fprintf(stderr, "turned %d: %dx%d, default calib %08x %08x, (10, 20) on the panel %08x %08x\n", e->value, g_w,
                g_h, bits(p.ax), bits(p.ay), bits(px), bits(py));
    }
    if (e->type == IN_POWER) {
        char st[32] = "";
        int r = sys_powerd_state(st, sizeof st);
        fprintf(stderr, "powerd %d \"%s\"\n", r, r == 0 ? st : "-");
        sys_request_sleep();
    }
    if (e->type == IN_KEY && e->value == 59) input_shortcuts_enable(true);
    if (e->type == IN_HID_ATTACH || e->type == IN_HID_DETACH) {
        InputHid h[4];
        int n = input_hids(h, 4);
        fprintf(stderr, "hids %d", n);
        for (int i = 0; i < n; i++) fprintf(stderr, " [%s %d %d %d]", h[i].name, h[i].gamepad, h[i].held, h[i].switch_layout);
        fprintf(stderr, "\n");
    }
}

int main(void) {
    float dpi;
    int r = display_open(&g_w, &g_h, &dpi);
    fprintf(stderr, "open %d: %dx%d %08x\n", r, g_w, g_h, bits(dpi));
    r = input_open(g_w, g_h, NULL, NULL);
    fprintf(stderr, "input %d: %s, %s\n", r, input_caps()->pen_name, input_caps()->touch_name);
    // A calibration with the axes swapped and one mirrored, for the pen; the finger keeps the default.
    InputCalib pen = {true, true, -1210.5f, 1205.25f, 1590.75f, 3.5f};
    input_set_calib(&pen, NULL);
    light_set_intensity(30);
    light_set_warmth(-2);
    int li, lw;
    light_get(&li, &lw);
    fprintf(stderr, "light %d %d %d %d\n", li, lw, light_warmth(), light_available());
    canvas_make(g_w, g_h);
    display_live_ink(true);
    display_present(&g_canvas, (Rect){10, 20, 30, 40}, REFRESH_FAST);
    display_live_ink(false);
    display_present(&g_canvas, (Rect){-5, 0, 50, 10}, REFRESH_UI);
    display_present(&g_canvas, (Rect){0, 0, g_w, g_h}, REFRESH_FULL);
    display_present(&g_canvas, (Rect){g_w, 0, 10, 10}, REFRESH_FULL);
    sys_activity();
    sys_activity();
    // (g_wait_until starts at 0, a wait that is over only while now_ms() is within 2^31 ms of it:
    // the probe starts from now, as the Rust twin does)
    g_wait_until = now_ms();
    bool quit = false;
    for (int pass = 0; pass < 1000 && !quit; pass++) {
        InEvent ev;
        if (!input_poll(&ev, 30)) {
            fprintf(stderr, "(none)\n");
            continue;
        }
        uint32_t t0 = ev.t_ms;
        do {
            print_ev(&ev, t0);
            react(&ev);
            if (ev.type == IN_QUIT) quit = true;
        } while (input_poll(&ev, 0));
        fprintf(stderr, "-- batch\n");
    }
    input_close();
    fprintf(stderr, "tally %d %d, line %d, storage %d\n", g_expect_total, g_expect_failed, g_line_no, g_sim_storage_full);
    display_close();
    fprintf(stderr, "(not reached: a check failed)\n");
    return 0;
}
