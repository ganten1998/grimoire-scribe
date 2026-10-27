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
    BgKind kind;
    char id[STORE_ID_LEN];
    char dir[600];  // BG_MIRROR_PDF: the mirror folder; the others: their path argument
    float dpi;
    bool notify;
} Job;

static const char *const k_kind_names[] = {"export", "compaction", "template", "import"};

static Job g_q[BG_QUEUE];
static int g_n;
static pid_t g_pid = -1;
static bool g_running_notify;
static Job g_running;  // the job the child is running (valid while g_pid > 0)

static int run_mirror(const Job *j) {
    NotebookInfo *nb = malloc(sizeof *nb);
    if (!nb) return -1;
    int li = store_load_info(j->id, nb);
    int rc = li == 0 ? export_mirror_pdf(nb, j->dpi, j->dir) : -1;
    if (rc) qlog("bg: export of %s failed (load %d, export %d)", j->id, li, rc);
    free(nb);
    return rc;
}

static int run_job(const Job *j) {
    switch (j->kind) {
    case BG_MIRROR_PDF: return run_mirror(j);
    case BG_COMPACT_PAGES: return bg_compact_pages(j->id, j->dpi);
    case BG_TEMPLATE: return bg_template(j->id, j->dir, j->dpi);
    case BG_IMPORT_FOLDER: return bg_import_folder(j->dir, j->dpi);
    }
    return -1;
}

// In a forked child: let go of everything inherited from Grimoire (input devices, possibly
// grabbed; the framebuffer; the power-service pipe), and take a name of its own so nobody
// mistakes it for Grimoire. stdout/stderr stay: they are the log.
static void child_setup(void) {
    work_pump_set(NULL);  // the pens are Grimoire's to read (common.h), never a child's
#ifdef __linux__
    prctl(PR_SET_NAME, "grimoire-bg", 0, 0, 0);
#endif
    long max = sysconf(_SC_OPEN_MAX);
    if (max < 0 || max > 4096) max = 4096;
    for (int fd = 3; fd < max; fd++) close(fd);
    if (nice(10) < 0) { /* best effort */ }
}

// Which job goes first: something the user waits on (an export they asked for, a template's
// picture), then the mirror PDFs, then imports, and compaction last. A compaction can take minutes
// on the Kindle, and its pages stay listed until it is done, so nothing is lost by its waiting.
static int rank(const Job *j) {
    if (j->notify || j->kind == BG_TEMPLATE) return 0;
    return j->kind == BG_MIRROR_PDF ? 1 : j->kind == BG_IMPORT_FOLDER ? 2 : 3;
}

static void take(int i, Job *out) {
    *out = g_q[i];
    memmove(g_q + i, g_q + i + 1, sizeof g_q[0] * (size_t)(g_n - i - 1));
    g_n--;
}

// Grimoire's own SIGTERM, SIGINT and SIGHUP handlers only set a flag. A child keeps them until it
// sets its own, and a stop sent in between (bg_cancel_notebook, just after the fork) would be lost:
// Grimoire would wait in waitpid for the whole job. So they are held across the fork, and the child
// puts them back to their defaults before it lets a held one in.
static void stops_set(sigset_t *s) {
    sigemptyset(s);
    sigaddset(s, SIGTERM);
    sigaddset(s, SIGINT);
    sigaddset(s, SIGHUP);
}

static void child_stops_by_default(const sigset_t *was) {
    struct sigaction sa;
    memset(&sa, 0, sizeof sa);
    sa.sa_handler = SIG_DFL;
    sigemptyset(&sa.sa_mask);
    sigaction(SIGTERM, &sa, NULL);
    sigaction(SIGINT, &sa, NULL);
    sigaction(SIGHUP, &sa, NULL);
    sigprocmask(SIG_SETMASK, was, NULL);
}

static void start_next(void) {
    if (g_pid > 0 || g_n == 0) return;
    int next = 0;
    for (int i = 1; i < g_n; i++)
        if (rank(&g_q[i]) < rank(&g_q[next])) next = i;
    Job j;
    take(next, &j);
    sigset_t stops, was;
    stops_set(&stops);
    sigprocmask(SIG_BLOCK, &stops, &was);
    pid_t pid = fork();
    if (pid != 0) sigprocmask(SIG_SETMASK, &was, NULL);
    if (pid < 0) {
        if (j.kind != BG_MIRROR_PDF) {  // only the PDF is worth a stall of the UI
            qlog("bg: cannot fork; %s of %s skipped", k_kind_names[j.kind], j.id[0] ? j.id : "a folder");
            return;
        }
        qlog("bg: cannot fork; exporting %s in the foreground", j.id);
        run_job(&j);
        return;
    }
    if (pid == 0) {
        // The child only renders and writes files: no display, no input, no atexit handlers.
        child_stops_by_default(&was);
        child_setup();
#ifdef GRIMOIRE_SIM
        // Tests: an export as slow as a big notebook's on the Kindle, so the next one queues.
        const char *slow = getenv("GRIMOIRE_BG_DELAY_MS");
        if (slow) sleep_ms(atoi(slow));
#endif
        _exit(run_job(&j) == 0 ? 0 : 1);
    }
    g_pid = pid;
    g_running = j;
    g_running_notify = j.notify;
    if (j.kind == BG_MIRROR_PDF) qlog("bg: exporting %s (pid %d)", j.id, (int)pid);
    else qlog("bg: %s of %s (pid %d)", k_kind_names[j.kind], j.id[0] ? j.id : "a folder", (int)pid);
}

void bg_queue(BgKind kind, const char *id, const char *path, float dpi, bool notify) {
    if (!id) id = "";
    if (!path) path = "";
    for (int i = 0; i < g_n; i++)
        if (g_q[i].kind == kind && !strcmp(g_q[i].id, id) && !strcmp(g_q[i].dir, path)) {
            g_q[i].notify |= notify;
            start_next();
            return;
        }
    if (g_n == BG_QUEUE) {
        // The oldest of the last to go first: a compaction (its pages stay listed for the next one)
        // before a mirror, before anything the user waits on.
        int drop = 0;
        for (int i = 1; i < g_n; i++)
            if (rank(&g_q[i]) > rank(&g_q[drop])) drop = i;
        Job gone;
        take(drop, &gone);
        qlog("bg: queue full, dropping the oldest %s (%s)", k_kind_names[gone.kind], gone.id[0] ? gone.id : "a folder");
    }
    Job *j = &g_q[g_n++];
    memset(j, 0, sizeof *j);
    j->kind = kind;
    strlcpy_q(j->id, id, sizeof j->id);
    strlcpy_q(j->dir, path, sizeof j->dir);
    j->dpi = dpi;
    j->notify = notify;
    start_next();
}

void bg_mirror_pdf(const char *id, float dpi, const char *mirror_dir, bool notify) {
    bg_queue(BG_MIRROR_PDF, id, mirror_dir, dpi, notify);
}

void bg_cancel_notebook(const char *id) {
    for (int i = 0; i < g_n;) {
        if (g_q[i].kind == BG_COMPACT_PAGES && !strcmp(g_q[i].id, id)) {
            memmove(g_q + i, g_q + i + 1, sizeof g_q[0] * (size_t)(g_n - i - 1));
            g_n--;
            qlog("bg: compaction of %s dropped (the notebook opens or goes)", id);
        } else
            i++;
    }
    if (g_pid > 0 && g_running.kind == BG_COMPACT_PAGES && !strcmp(g_running.id, id)) {
        kill(g_pid, SIGTERM);
        int st;
        waitpid(g_pid, &st, 0);
        qlog("bg: compaction of %s stopped (the notebook opens or goes)", id);
        g_pid = -1;
    }
}

int bg_poll_kind(int *done_kind) {
    int result = 0;
    if (done_kind) *done_kind = -1;
    if (g_pid > 0) {
        int st = 0;
        pid_t r = waitpid(g_pid, &st, WNOHANG);
        if (r == g_pid || (r < 0)) {
            bool ok = r == g_pid && WIFEXITED(st) && WEXITSTATUS(st) == 0;
            qlog("bg: %s pid %d %s", k_kind_names[g_running.kind], (int)g_pid, ok ? "done" : "FAILED");
            if (g_running_notify) result = ok ? 1 : -1;
            if (done_kind) *done_kind = (int)g_running.kind;
            g_pid = -1;
        }
    }
    start_next();
    return result;
}

int bg_poll(void) { return bg_poll_kind(NULL); }

bool bg_busy(void) { return g_pid > 0 || g_n > 0; }

void bg_detach_rest(void) {
    // Compaction waits for the next run: in a detached child it could still be rewriting pages
    // when the notebook is opened again, out of bg_cancel_notebook's reach.
    for (int i = 0; i < g_n;) {
        if (g_q[i].kind == BG_COMPACT_PAGES) {
            qlog("bg: compaction of %s left for the next run", g_q[i].id);
            memmove(g_q + i, g_q + i + 1, sizeof g_q[0] * (size_t)(g_n - i - 1));
            g_n--;
        } else
            i++;
    }
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
    qlog("bg: %d export(s) left to a detached child (pid %d)", g_n, (int)pid);  // (exports and the other jobs)
    g_n = 0;
}
