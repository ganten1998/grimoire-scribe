// test_pump.c: the work pump (common.h). Long work on the main thread calls work_pump() as it goes,
// so the Kindle reads its pens meanwhile: the raw digitizer's kernel buffer holds about 100 ms of
// hover, and on 2026-10-27 a 1.2 s page render, a 1.3 s Paper sheet and the Display sheet's slider
// steps of 90-100 ms lost its pressure and tilt 26 times. These check that the long loops call it,
// that what they draw does not change for it, and that only the process that set the hook runs it.
#include <errno.h>
#include <fcntl.h>
#include <sys/wait.h>
#include <unistd.h>

#include "canvas.h"
#include "page.h"
#include "paper.h"
#include "testlib.h"

static const float DPI = 300.f;
enum { W = 1860, H = 2480 };

static int g_calls;
static void count(void) { g_calls++; }

// A dense page: 56 strokes of 600 points each, the owner's p28 shape made heavier.
static Page *dense_page(void) {
    Page *pg = page_new(W, H, PAPER_MEDIUM);
    ToolParams tp;
    tool_params_default(TOOL_BALLPOINT, &tp);
    for (int i = 0; i < 56; i++) {
        Stroke *s = stroke_new(TOOL_BALLPOINT, &tp, 0x1234u + (uint32_t)i);
        for (int k = 0; k < 600; k++) {
            float f = (float)k / 599.f;
            PenSample q = {150.f + 1500.f * f, 120.f + 40.f * (float)i + 15.f * sinf(f * 40.f), 0.3f + 0.5f * f, 10.f, -5.f,
                           (uint32_t)k * 3u};
            stroke_append(s, &q, DPI);
        }
        page_add(pg, s);
    }
    return pg;
}

static uint32_t hash(const Canvas *c) {
    uint32_t h = 2166136261u;
    for (int y = 0; y < c->h; y++)
        for (int x = 0; x < c->w; x++) h = (h ^ c->px[(size_t)y * (size_t)c->stride + (size_t)x]) * 16777619u;
    return h;
}

// Each stroke pumps after it and every 128 points inside it, and the paper after it is drawn: on the
// Kindle every 128 points is 3-15 ms, depending on the tool. The pixels are the same with no hook.
TEST(a_page_render_pumps_and_draws_the_same) {
    Page *pg = dense_page();
    Canvas a, b;
    CHECK(canvas_init(&a, W, H) == 0 && canvas_init(&b, W, H) == 0);
    work_pump_set(NULL);
    page_render(&a, pg, (Rect){0, 0, W, H}, DPI);
    work_pump_set(count);
    g_calls = 0;
    page_render(&b, pg, (Rect){0, 0, W, H}, DPI);
    CHECK(g_calls >= 56 * (1 + 600 / 128) + 1);
    CHECK(hash(&a) == hash(&b));
    CHECK(!memcmp(a.px, b.px, (size_t)a.stride * (size_t)H));
    // A part of the page (a dirty rect): only the strokes in it are drawn, and they still pump.
    g_calls = 0;
    page_render(&b, pg, (Rect){0, 0, W, 300}, DPI);
    CHECK(g_calls >= 4 * (1 + 600 / 128));
    CHECK(!memcmp(a.px, b.px, (size_t)a.stride * (size_t)H));
    // The thumbnail of a page: its render, then the downsample, every 64 rows of the page.
    Canvas thumb;
    CHECK(canvas_init(&thumb, W / 6, H / 6) == 0);
    g_calls = 0;
    page_render_thumb(&thumb, pg, DPI);
    CHECK(g_calls >= 56 * (1 + 600 / 128) + H / 64);
    work_pump_set(NULL);
    canvas_free(&thumb);
    canvas_free(&a);
    canvas_free(&b);
    page_free(pg);
}

// The Paper sheet's previews: a whole page of each paper, then shrunk keeping its lines.
TEST(a_paper_preview_pumps) {
    Canvas c, small;
    CHECK(canvas_init(&c, W, H) == 0 && canvas_init(&small, 300, 400) == 0);
    work_pump_set(count);
    g_calls = 0;
    paper_render(&c, PAPER_GRAND, (Rect){0, 0, W, H}, DPI);
    CHECK(g_calls == 1);
    g_calls = 0;
    canvas_downsample_keep_lines(&small, &c, 0.5f);
    CHECK(g_calls >= H / 64 - 1 && g_calls <= H / 64 + 1);
    g_calls = 0;
    canvas_downsample(&small, &c);
    CHECK(g_calls >= H / 64 - 1 && g_calls <= H / 64 + 1);
    // Nothing to draw: no pump either.
    g_calls = 0;
    paper_render(&c, PAPER_GRAND, RECT_EMPTY, DPI);
    CHECK(g_calls == 0);
    work_pump_set(NULL);
    canvas_free(&c);
    canvas_free(&small);
}

// A save pumps after its write, its fsync, the rename and the folder's fsync: each wait on the
// storage is then one fsync at most (15-25 ms on the Kindle; a page's whole save took 65-104 ms).
TEST(a_save_pumps_between_its_waits) {
    char path[700];
    snprintf(path, sizeof path, "%s/pump-%d.txt", tl_tmp(), (int)getpid());
    work_pump_set(count);
    g_calls = 0;
    CHECK(write_file_atomic(path, "grimoire", 8) == 0);
    CHECK(g_calls == 4);
    unlink(path);
    // One that cannot even start (no such folder) has nothing to pump between.
    g_calls = 0;
    CHECK(write_file_atomic("/nonexistent-grimoire-dir/x.txt", "x", 1) != 0);
    CHECK(g_calls == 0);
    work_pump_set(NULL);
    g_calls = 0;
    work_pump();
    CHECK(g_calls == 0);
}

// Only the process that set the hook runs it. A forked child (a background export, a share server
// render) inherits the hook with the rest of the memory, and its own renders and saves call
// work_pump; reading Grimoire's input devices from there would take Grimoire's pen events (the
// file description is shared), or read a file of the child's that came to reuse their number.
static int g_pipe[2];
static void tell(void) {
    char c = 1;
    if (write(g_pipe[1], &c, 1) != 1) { /* the check below sees it */ }
}

TEST(only_the_process_that_set_it_pumps) {
    CHECK(pipe(g_pipe) == 0);
    fcntl(g_pipe[0], F_SETFL, O_NONBLOCK);
    work_pump_set(tell);
    pid_t pid = fork();
    if (pid == 0) {
        char path[700];
        snprintf(path, sizeof path, "%s/pump-child-%d.txt", tl_tmp(), (int)getpid());
        work_pump();
        write_file_atomic(path, "child", 5);
        unlink(path);
        Canvas c, small;
        if (canvas_init(&c, 640, 640) == 0 && canvas_init(&small, 64, 64) == 0) canvas_downsample(&small, &c);
        _exit(0);
    }
    CHECK(pid > 0);
    int status = -1;
    waitpid(pid, &status, 0);
    CHECK(WIFEXITED(status) && WEXITSTATUS(status) == 0);
    char c;
    errno = 0;
    CHECK(read(g_pipe[0], &c, 1) < 0 && errno == EAGAIN);  // the child never ran it
    work_pump();
    CHECK(read(g_pipe[0], &c, 1) == 1);  // this process does
    work_pump_set(NULL);
    work_pump();
    CHECK(read(g_pipe[0], &c, 1) < 0);
    close(g_pipe[0]);
    close(g_pipe[1]);
}

int main(void) {
    RUN(a_page_render_pumps_and_draws_the_same);
    RUN(a_paper_preview_pumps);
    RUN(a_save_pumps_between_its_waits);
    RUN(only_the_process_that_set_it_pumps);
    return tl_done();
}
