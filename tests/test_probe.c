// test_probe.c: the scripted runner's probes (app_probe in app.c) name things as the app's files do.
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

#include "app.h"
#include "testlib.h"

static char root[600];

// editor.tool and settings.TOOL.PARAM name tools with app_probe_tool, a copy of store.c's names:
// settings_save writes "<tool>.<param>=" for every tool and every setting, and the probes must
// name each of those tools (a tool added to one list and not the other fails here).
TEST(tool_names_are_settings_ini_names) {
    Settings s;
    settings_default(&s);
    CHECK(settings_save(&s) == 0);
    char path[700];
    snprintf(path, sizeof path, "%s/settings.ini", store_root());
    FILE *f = fopen(path, "r");
    CHECK(f != NULL);
    if (!f) return;
    int n;
    tool_param_table(&n);
    int seen[TOOL_COUNT] = {0}, unnamed = 0;
    char line[256];
    while (fgets(line, sizeof line, f)) {
        char *dot = strchr(line, '.'), *eq = strchr(line, '=');
        if (!dot || !eq || dot > eq) continue;
        *dot = 0;
        if (!strcmp(line, "pen") || !strcmp(line, "touch")) continue;  // the calibrations
        int t = 0;
        while (t < TOOL_COUNT && !(app_probe_tool((ToolKind)t) && !strcmp(line, app_probe_tool((ToolKind)t)))) t++;
        if (t < TOOL_COUNT) seen[t]++;
        else unnamed++;
    }
    fclose(f);
    for (int t = 0; t < TOOL_COUNT; t++) CHECK(seen[t] == n);
    CHECK(unnamed == 0);
    CHECK(app_probe_tool(TOOL_COUNT) == NULL);
    CHECK(app_probe_tool((ToolKind)-1) == NULL);
}

int main(void) {
    snprintf(root, sizeof root, "%s/probe-%d", tl_tmp(), (int)getpid());
    char cmd[700];
    snprintf(cmd, sizeof cmd, "rm -rf '%s'", root);
    system(cmd);
    CHECK(store_init(root) == 0);
    RUN(tool_names_are_settings_ini_names);
    int rc = tl_done();
    system(cmd);
    return rc;
}
