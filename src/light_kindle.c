// light_kindle.c: front light through lipc (com.lab126.powerd), as the stock UI and KOReader do.
#include <stdio.h>
#include <stdlib.h>
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

static void set_prop(const char *prop, int v) {
    char cmd[128];
    snprintf(cmd, sizeof cmd, "lipc-set-prop -i com.lab126.powerd %s %d >/dev/null 2>&1", prop, clampi(v, 0, LIGHT_MAX));
    if (system(cmd) != 0) qlog("light: %s -> %d failed", prop, v);
}

bool light_available(void) { return access("/usr/bin/lipc-set-prop", X_OK) == 0 || access("/bin/lipc-set-prop", X_OK) == 0; }

void light_get(int *intensity, int *warmth) {
    *intensity = get_prop("flIntensity");
    *warmth = get_prop("currentAmberLevel");
}

void light_set_intensity(int level) { set_prop("flIntensity", level); }
void light_set_warmth(int level) { set_prop("currentAmberLevel", level); }
