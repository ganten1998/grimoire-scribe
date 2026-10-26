// test_update.c: Settings > Updates' pure parts: versions as KPM orders them (as tools/kpm-package.py
// encodes them), the newest Grimoire in `kpm search grimoire`'s output, and the request file.
#include "update.h"
#include "common.h"
#include "testlib.h"

#include <unistd.h>

TEST(versions_order_as_kpm_orders_them) {
    uint32_t k[3];
    CHECK(update_version_key("2.0.0-beta.5", k) && k[0] == 2 && k[1] == 0 && k[2] == 1005);
    CHECK(update_version_key("2.0.0-rc.1", k) && k[2] == 5001);
    CHECK(update_version_key("2.0.0", k) && k[2] == 9999);
    CHECK(update_version_key("2.0.1", k) && k[2] == 19999);
    CHECK(update_version_key("1.2.0", k) && k[0] == 1 && k[1] == 2 && k[2] == 9999);
    CHECK(!update_version_key("2.0", k) && !update_version_key("2.0.0-beta.0", k) && !update_version_key("2.0.0-beta.01", k));
    CHECK(!update_version_key("2.0.0-alpha.1", k) && !update_version_key("2.0.0-beta.1000", k) && !update_version_key("v2.0.0", k));
    CHECK(!update_version_key("2.0.0 ", k) && !update_version_key("", k) && !update_version_key(NULL, k));
    CHECK(update_version_cmp("2.0.0-beta.4", "2.0.0-beta.5") < 0);
    CHECK(update_version_cmp("2.0.0-beta.9", "2.0.0-rc.1") < 0);
    CHECK(update_version_cmp("2.0.0-rc.3", "2.0.0") < 0);
    CHECK(update_version_cmp("2.0.0", "2.0.1-beta.1") < 0);  // a hotfix's beta is newer than the release
    CHECK(update_version_cmp("2.0.1", "2.1.0-beta.1") < 0);
    CHECK(update_version_cmp("1.2.0", "2.0.0-beta.1") < 0);
    CHECK(update_version_cmp("2.0.0-beta.5", "2.0.0-beta.5") == 0);
    CHECK(update_version_cmp("junk", "1.0.0") < 0 && update_version_cmp("1.0.0", "junk") > 0);
    CHECK(update_is_prerelease("2.0.0-beta.5") && update_is_prerelease("2.0.0-rc.2") && !update_is_prerelease("2.0.0"));
}

TEST(the_newest_grimoire_in_kpms_search) {
    char v[UPDATE_VERSION_LEN];
    // As KPM 0.2 prints it: an INFO line, then a line per package found, colour codes about.
    const char *out = "\x1b[32mFound 3 package(s) for grimoire:\x1b[0m\n"
                      "  - grimoire (Grimoire): Handwriting notebooks for the jailbroken Kindle Scribe. 2.0.0-beta.4\r\n"
                      "\x1b[32m  - grimoire (Grimoire): Handwriting notebooks for the jailbroken Kindle Scribe. 2.0.0-beta.6\x1b[0m\n"
                      "  - grimoire (Grimoire): Handwriting notebooks for the jailbroken Kindle Scribe. 1.2.0\n"
                      "  - grimoire-tools (Grimoire tools): Not ours 9.9.9\n";
    CHECK(update_latest(out, v) && !strcmp(v, "2.0.0-beta.6"));
    CHECK(!update_latest("Could not find any packages for 'grimoire'\n", v) && !v[0]);
    CHECK(!update_latest("  - grimoire (Grimoire): a description with no version\n", v));
    CHECK(!update_latest("", v) && !update_latest(NULL, v));
}

TEST(the_request_is_written_for_the_launcher) {
    char path[800];
    snprintf(path, sizeof path, "%s/grimoire-update-%d", tl_tmp(), (int)getpid());
    char script[800];
    snprintf(script, sizeof script, "%s/no-such-update.sh", tl_tmp());
    update_init(script, path);
    CHECK(!update_available() && !update_check(true));  // no script: no updates here
    CHECK(update_request(true) == 0);
    size_t len;
    char *text = read_file(path, &len);
    CHECK(text && !strcmp(text, "preview\n"));
    free(text);
    CHECK(update_request(false) == 0);
    text = read_file(path, &len);
    CHECK(text && !strcmp(text, "stable\n"));
    free(text);
    unlink(path);
}

int main(void) {
    RUN(versions_order_as_kpm_orders_them);
    RUN(the_newest_grimoire_in_kpms_search);
    RUN(the_request_is_written_for_the_launcher);
    return tl_done();
}
