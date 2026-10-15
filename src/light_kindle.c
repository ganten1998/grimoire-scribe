// light_kindle.c: front light through lipc (com.lab126.powerd), as the stock UI and KOReader do.
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

// Runs a shell command without waiting for it (double fork: no zombie, no stall). Writing to the
// power service takes tens of ms; doing that on the input path made the first touches laggy.
static void run_bg(const char *cmd) {
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

void light_get(int *intensity, int *warmth) {
    *intensity = get_prop("flIntensity");
    *warmth = get_prop("currentAmberLevel");
}

void light_set_intensity(int level) { set_prop("flIntensity", level); }
void light_set_warmth(int level) { set_prop("currentAmberLevel", level); }

void sys_request_sleep(void) {
    qlog("sleep: asking the power service to sleep");
    run_bg("lipc-set-prop -i com.lab126.powerd powerButton 1 >/dev/null 2>&1");
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

void sys_kindle_ui(bool run) {
    const char *stopped = getenv("QUIRE_AWESOME_STOPPED");
    if (!stopped || strcmp(stopped, "yes")) return;  // the launcher did not pause it
    run_bg(run ? "killall -CONT awesome 2>/dev/null" : "killall -STOP awesome 2>/dev/null");
}
