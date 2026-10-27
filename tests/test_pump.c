// test_pump.c: the work pump (common.h). Long work on the main thread calls work_pump() as it goes,
// so the Kindle reads its pens meanwhile: the raw digitizer's kernel buffer holds about 100 ms of
// hover, and on 2026-10-27 a 1.2 s page render, a 1.3 s Paper sheet and the Display sheet's slider
// steps of 90-100 ms lost its pressure and tilt 26 times. These check that a save calls it between
// its waits, and that only the process that set the hook runs it.
#include <errno.h>
#include <fcntl.h>
#include <sys/wait.h>
#include <unistd.h>

#include "canvas.h"
#include "testlib.h"

static int g_calls;
static void count(void) { g_calls++; }

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
    RUN(a_save_pumps_between_its_waits);
    RUN(only_the_process_that_set_it_pumps);
    return tl_done();
}
