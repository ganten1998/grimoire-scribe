// Exercise the real native UI handover with command stand-ins: order, failure and timeout.
#include "testlib.h"
#include "../src/light_kindle.c"
#include <sys/stat.h>

TEST(native_ui_boundary) {
    char dir[800], path[1000], log[1000], search[2000];
    snprintf(dir, sizeof dir, "%s/lock-ui-%ld", tl_tmp(), (long)getpid());
    CHECK(mkdir_p(dir) == 0);
    snprintf(log, sizeof log, "%s/calls", dir);
    setenv("LOCK_TEST_LOG", log, 1);
    const char *before = getenv("PATH");
    char *saved = strdup(before ? before : "/usr/bin:/bin");
    snprintf(search, sizeof search, "%s:%s", dir, saved);
    setenv("PATH", search, 1);
    snprintf(path, sizeof path, "%s/killall", dir);
    const char *killshim = "#!/bin/sh\nprintf 'killall %s\\n' \"$*\" >>\"$LOCK_TEST_LOG\"\n";
    CHECK(write_file_atomic(path, killshim, strlen(killshim)) == 0 && chmod(path, 0700) == 0);
    snprintf(path, sizeof path, "%s/lipc-set-prop", dir);
    const char *lipcshim = "#!/bin/sh\nprintf 'lipc %s\\n' \"$*\" >>\"$LOCK_TEST_LOG\"\n"
                           "[ \"${LOCK_TEST_FAIL:-}\" = yes ] && exit 1\nexit 0\n";
    CHECK(write_file_atomic(path, lipcshim, strlen(lipcshim)) == 0 && chmod(path, 0700) == 0);
    setenv("GRIMOIRE_AWESOME_STOPPED", "yes", 1);
    setenv("GRIMOIRE_PILLOW_DISABLED", "yes", 1);
    CHECK(sys_kindle_ui(true));
    CHECK(sys_kindle_ui(false));
    size_t len; char *data = read_file(log, &len);
    const char *want = "killall -CONT awesome\nlipc com.lab126.pillow disableEnablePillow enable\n"
                       "lipc com.lab126.pillow disableEnablePillow disable\nkillall -STOP awesome\n";
    CHECK(data && len == strlen(want) && !memcmp(data, want, len));
    free(data);
    setenv("LOCK_TEST_FAIL", "yes", 1);
    CHECK(!sys_kindle_ui(false));
    unsetenv("LOCK_TEST_FAIL");
    const char *slow = "#!/bin/sh\nsleep 6\n";
    CHECK(write_file_atomic(path, slow, strlen(slow)) == 0 && chmod(path, 0700) == 0);
    uint32_t started = now_ms();
    CHECK(!sys_kindle_ui(false));
    CHECK(now_ms() - started < 5000); // a stuck lipc must not trap the app indefinitely
    unsetenv("GRIMOIRE_AWESOME_STOPPED"); unsetenv("GRIMOIRE_PILLOW_DISABLED");
    setenv("PATH", saved, 1); free(saved);
}
int main(void) { RUN(native_ui_boundary); return tl_done(); }
