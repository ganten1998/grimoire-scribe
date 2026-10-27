// light_kindle.c: front light through lipc (com.lab126.powerd), as the stock UI and KOReader do.
#include <ctype.h>
#include <errno.h>
#include <poll.h>
#include <signal.h>
#include <spawn.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/wait.h>
#include <unistd.h>

#include "common.h"
#include "light.h"

static int get_prop(const char *prop) {
    char cmd[128];
    snprintf(cmd, sizeof cmd, "lipc-get-prop -i com.lab126.powerd %s 2>/dev/null", prop);
    FILE *p = popen(cmd, "r");
    if (!p) return -1;
    int v = -1;
    if (fscanf(p, "%d", &v) != 1) v = -1;
    pclose(p);
    return v;
}

extern char **environ;

// The shells run_bg started and has not reaped yet. Each is reaped by its pid as it ends (never
// waitpid(-1): the background worker and Live view reap their own children).
#define BG_SHELLS 8
static pid_t g_shells[BG_SHELLS];

static void reap_shells(void) {
    for (int i = 0; i < BG_SHELLS; i++)
        if (g_shells[i] > 0 && waitpid(g_shells[i], NULL, WNOHANG) != 0) g_shells[i] = 0;  // ended (or not ours now)
}

// Runs a shell command without waiting for it. Writing to the power service takes tens of ms; doing
// that on the input path made the first touches laggy. posix_spawn starts it without copying
// Grimoire (musl's shares the memory until the exec): the double fork used before copied the page
// tables of a 60-100 MB process twice, on every light slider step (each 200 ms) and once a minute
// in the middle of a stroke (sys_activity). A shell that ended is reaped at the next call, so at
// most a few wait as zombies. When it cannot spawn, or eight are still running, the double fork.
static void run_bg(const char *cmd) {
    reap_shells();
    for (int i = 0; i < BG_SHELLS; i++) {
        if (g_shells[i] > 0) continue;
        char *argv[] = {"sh", "-c", (char *)cmd, NULL};
        pid_t pid;
        if (posix_spawn(&pid, "/bin/sh", NULL, NULL, argv, environ) == 0) {
            g_shells[i] = pid;
            return;
        }
        break;
    }
    pid_t pid = fork();
    if (pid < 0) {
        qlog("sys: cannot fork for: %s", cmd);
        return;
    }
    if (pid == 0) {
        if (fork() == 0) {
            execl("/bin/sh", "sh", "-c", cmd, (char *)NULL);
            _exit(127);
        }
        _exit(0);
    }
    waitpid(pid, NULL, 0);  // the intermediate child exits at once
}

static void set_prop(const char *prop, int v) {
    char cmd[128];
    snprintf(cmd, sizeof cmd, "lipc-set-prop -i com.lab126.powerd %s %d >/dev/null 2>&1", prop, clampi(v, 0, LIGHT_MAX));
    run_bg(cmd);
}

bool light_available(void) { return access("/usr/bin/lipc-set-prop", X_OK) == 0 || access("/bin/lipc-set-prop", X_OK) == 0; }

// The levels as last read or set. Each read is two synchronous lipc-get-prop runs (95-191 ms on the
// Scribe 2, at every Display sheet open and Live view start until 2.0.0-beta.5), and while they run
// nothing reads the pens (no work pump can run inside a popen). Nothing else changes the light while
// Grimoire runs (the Kindle UI and KOReader are paused, and the power service gives the same level
// back after its screensaver), so it is read once, before the pens are opened (main.c).
static int g_intensity = -1, g_warmth = -1;

void light_refresh(void) {
    g_intensity = get_prop("flIntensity");
    g_warmth = get_prop("currentAmberLevel");
}

void light_get(int *intensity, int *warmth) {
    if (g_intensity < 0) light_refresh();  // never read, or the power service did not answer then
    *intensity = g_intensity;
    *warmth = g_warmth;
}

void light_set_intensity(int level) {
    set_prop("flIntensity", level);
    g_intensity = clampi(level, 0, LIGHT_MAX);
}
void light_set_warmth(int level) {
    set_prop("currentAmberLevel", level);
    g_warmth = clampi(level, 0, LIGHT_MAX);
}
int light_warmth(void) { return g_warmth; }

void sys_request_sleep(void) {
    qlog("sleep: asking the power service to sleep (preventScreenSaver 0, then powerButton 1)");
    // One shell, in this order: the power service ignores the button while preventScreenSaver is
    // 1, and two shells of their own could reach it either way round.
    run_bg("lipc-set-prop -i com.lab126.powerd preventScreenSaver 0 >/dev/null 2>&1;"
           " lipc-set-prop -i com.lab126.powerd powerButton 1 >/dev/null 2>&1");
}

// Runs a shell command and keeps the first line it prints (trimmed), waiting at most timeout_ms:
// a wedged lipc must not hold up Grimoire. 0 with a non-empty line, else -1.
static int run_line(const char *cmd, char *out, size_t cap, int timeout_ms) {
    int p[2];
    if (cap == 0 || pipe(p) != 0) return -1;
    pid_t pid = fork();
    if (pid < 0) {
        close(p[0]);
        close(p[1]);
        return -1;
    }
    if (pid == 0) {
        setpgid(0, 0);  // its own group, so a kill also reaches what the shell started
        dup2(p[1], 1);
        close(p[0]);
        close(p[1]);
        execl("/bin/sh", "sh", "-c", cmd, (char *)NULL);
        _exit(127);
    }
    setpgid(pid, pid);  // here too, so the group exists even if the child has not run yet
    close(p[1]);
    char buf[128];
    size_t len = 0;
    uint32_t until = now_ms() + (uint32_t)timeout_ms;
    for (;;) {
        int left = (int)(until - now_ms());
        if (left <= 0) break;
        struct pollfd pf = {p[0], POLLIN, 0};
        int r = poll(&pf, 1, left);
        if (r < 0 && errno == EINTR) continue;
        if (r <= 0) break;
        ssize_t n = read(p[0], buf + len, sizeof buf - 1 - len);
        if (n <= 0) break;  // end of output: the command is done
        len += (size_t)n;
        if (memchr(buf, '\n', len) || len >= sizeof buf - 1) break;
    }
    close(p[0]);
    if (waitpid(pid, NULL, WNOHANG) == 0) {  // still running (too slow, or more to say): stop it
        kill(-pid, SIGKILL);
        kill(pid, SIGKILL);
        waitpid(pid, NULL, 0);
    }
    buf[len] = 0;
    char *s = buf;
    while (*s && isspace((unsigned char)*s)) s++;
    size_t n = strcspn(s, "\r\n");
    while (n && isspace((unsigned char)s[n - 1])) n--;
    if (n == 0) return -1;
    if (n >= cap) n = cap - 1;
    memcpy(out, s, n);
    out[n] = 0;
    return 0;
}

int sys_powerd_state(char *out, size_t cap) {
    return run_line("lipc-get-prop -s com.lab126.powerd state 2>/dev/null", out, cap, 1500);
}

void sys_prevent_sleep(bool prevent) {
    char cmd[96];
    snprintf(cmd, sizeof cmd, "lipc-set-prop -i com.lab126.powerd preventScreenSaver %d >/dev/null 2>&1", prevent ? 1 : 0);
    run_bg(cmd);
    qlog("sleep: preventScreenSaver %d", prevent);
}

void sys_activity(void) {
    // Same as KOReader: resets the power service's idle (screensaver) timer.
    run_bg("lipc-set-prop -i com.lab126.powerd touchScreenSaverTimeout 1 >/dev/null 2>&1");
}

bool sys_kindle_ui(bool run) {
    const char *stopped = getenv("GRIMOIRE_AWESOME_STOPPED");
    const char *pillow = getenv("GRIMOIRE_PILLOW_DISABLED");
    bool wm = stopped && !strcmp(stopped, "yes"), pi = pillow && !strcmp(pillow, "yes");
    if (!wm && !pi) return true;
    char cmd[512], answer[32];
    // Synchronous at the lock boundary only: the UI must finish pausing before we draw again.
    snprintf(cmd, sizeof cmd, "%s%s echo ready",
             run ? (wm ? "killall -CONT awesome 2>/dev/null || exit 1; " : "")
                 : (pi ? "lipc-set-prop com.lab126.pillow disableEnablePillow disable >/dev/null 2>&1 || exit 1; " : ""),
             run ? (pi ? "lipc-set-prop com.lab126.pillow disableEnablePillow enable >/dev/null 2>&1 || exit 1; " : "")
                 : (wm ? "killall -STOP awesome 2>/dev/null || exit 1; " : ""));
    return run_line(cmd, answer, sizeof answer, 3000) == 0 && !strcmp(answer, "ready");
}
