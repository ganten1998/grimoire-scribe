// bgjob.c: see bgjob.h.
#include "bgjob.h"

#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/types.h>
#include <sys/wait.h>
#include <unistd.h>
#ifdef __linux__
#include <sys/prctl.h>
#endif

#include "export.h"
#include "store.h"

#define BG_QUEUE 16

typedef struct {
    char id[STORE_ID_LEN];
    char dir[600];
    float dpi;
    bool notify;
} Job;

static Job g_q[BG_QUEUE];
static int g_n;
static pid_t g_pid = -1;
static bool g_running_notify;

static int run_job(const Job *j) {
    NotebookInfo *nb = malloc(sizeof *nb);
    if (!nb) return -1;
    int li = store_load_info(j->id, nb);
    int rc = li == 0 ? export_mirror_pdf(nb, j->dpi, j->dir) : -1;
    if (rc) qlog("bg: export of %s failed (load %d, export %d)", j->id, li, rc);
    free(nb);
    return rc;
}

// In a forked child: let go of everything inherited from Grimoire (input devices, possibly
// grabbed; the framebuffer; the power-service pipe), and take a name of its own so nobody
// mistakes it for Grimoire. stdout/stderr stay: they are the log.
static void child_setup(void) {
#ifdef __linux__
    prctl(PR_SET_NAME, "grimoire-bg", 0, 0, 0);
#endif
    long max = sysconf(_SC_OPEN_MAX);
    if (max < 0 || max > 4096) max = 4096;
    for (int fd = 3; fd < max; fd++) close(fd);
    if (nice(10) < 0) { /* best effort */ }
}

static void start_next(void) {
    if (g_pid > 0 || g_n == 0) return;
    Job j = g_q[0];
    memmove(g_q, g_q + 1, sizeof g_q[0] * (size_t)(g_n - 1));
    g_n--;
    pid_t pid = fork();
    if (pid < 0) {
        qlog("bg: cannot fork; exporting %s in the foreground", j.id);
        run_job(&j);
        return;
    }
    if (pid == 0) {
        // The child only renders and writes files: no display, no input, no atexit handlers.
        child_setup();
#ifdef GRIMOIRE_SIM
        // Tests: an export as slow as a big notebook's on the Kindle, so the next one queues.
        const char *slow = getenv("GRIMOIRE_BG_DELAY_MS");
        if (slow) sleep_ms(atoi(slow));
#endif
        _exit(run_job(&j) == 0 ? 0 : 1);
    }
    g_pid = pid;
    g_running_notify = j.notify;
    qlog("bg: exporting %s (pid %d)", j.id, (int)pid);
}

void bg_mirror_pdf(const char *id, float dpi, const char *mirror_dir, bool notify) {
    for (int i = 0; i < g_n; i++)
        if (!strcmp(g_q[i].id, id)) {
            g_q[i].notify |= notify;
            start_next();
            return;
        }
    if (g_n == BG_QUEUE) {
        qlog("bg: queue full, dropping the oldest job (%s)", g_q[0].id);
        memmove(g_q, g_q + 1, sizeof g_q[0] * (BG_QUEUE - 1));
        g_n--;
    }
    Job *j = &g_q[g_n++];
    memset(j, 0, sizeof *j);
    strlcpy_q(j->id, id, sizeof j->id);
    strlcpy_q(j->dir, mirror_dir, sizeof j->dir);
    j->dpi = dpi;
    j->notify = notify;
    start_next();
}

int bg_poll(void) {
    int result = 0;
    if (g_pid > 0) {
        int st = 0;
        pid_t r = waitpid(g_pid, &st, WNOHANG);
        if (r == g_pid || (r < 0)) {
            bool ok = r == g_pid && WIFEXITED(st) && WEXITSTATUS(st) == 0;
            qlog("bg: export pid %d %s", (int)g_pid, ok ? "done" : "FAILED");
            if (g_running_notify) result = ok ? 1 : -1;
            g_pid = -1;
        }
    }
    start_next();
    return result;
}

bool bg_busy(void) { return g_pid > 0 || g_n > 0; }

void bg_detach_rest(void) {
    if (g_n == 0) return;
    pid_t pid = fork();
    if (pid == 0) {
        setsid();
        child_setup();
        // Wait for the running export first (it writes the same folder).
        while (g_pid > 0 && kill(g_pid, 0) == 0) usleep(200000);
        for (int i = 0; i < g_n; i++) run_job(&g_q[i]);
        _exit(0);
    }
    qlog("bg: %d export(s) left to a detached child (pid %d)", g_n, (int)pid);
    g_n = 0;
}
