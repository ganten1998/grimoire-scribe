#include "koreader.h"
#include "common.h"
#include "store.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

extern unsigned char TerminessNerdFontMono_Regular_ttf[], TerminessNerdFontMono_Bold_ttf[];
extern unsigned int TerminessNerdFontMono_Regular_ttf_len, TerminessNerdFontMono_Bold_ttf_len;
static const char *const files[] = {
    "grimoire.koplugin", "2-no-diagonal-screenshot.lua", "2-mosaic-square-corners.lua", "2-terminess-ui-font.lua"
};
static bool installed_at(const char *dir) {
    char p[1024];
    snprintf(p, sizeof p, "%s/reader.lua", dir);
    return access(p, R_OK) == 0;
}
bool koreader_find(char *out, size_t cap) {
    const char *host = getenv("KOREADER_DIR");
    if (host && *host && installed_at(host)) {
        strlcpy_q(out, host, cap);
        return true;
    }
    const char *us = getenv("GRIMOIRE_US");
    if (!us || !*us) us = "/mnt/us";
    static const char *const places[] = {"koreader", "extensions/koreader", "documents/koreader"};
    for (size_t i = 0; i < sizeof places / sizeof *places; i++) {
        snprintf(out, cap, "%s/%s", us, places[i]);
        if (installed_at(out)) return true;
    }
    *out = 0;
    return false;
}
const char *koreader_item_name(int item) { return item >= 0 && item < 4 ? files[item] : ""; }
static bool paths(int item, char *src, char *dst) {
    char ko[800];
    if (item < 0 || item > 3 || !koreader_find(ko, sizeof ko)) return false;
    snprintf(src, 1024, "%s/app/koreader/%s/%s", store_root(), item ? "patches" : "", files[item]);
    snprintf(dst, 1024, "%s/%s/%s", ko, item ? "patches" : "plugins", files[item]);
    return true;
}
bool koreader_item_installed(int item) {
    char src[1024], dst[1024];
    if (!paths(item, src, dst)) return false;
    if (!item) {
        size_t n = strlen(dst);
        snprintf(dst + n, sizeof dst - n, "/main.lua");
    }
    return access(dst, R_OK) == 0;
}
static bool same(const char *a, const char *b) {
    size_t an, bn;
    void *ap = read_file(a, &an), *bp = read_file(b, &bn);
    bool yes = ap && bp && an == bn && !memcmp(ap, bp, an);
    free(ap); free(bp);
    return yes;
}
static int copy_owned(const char *src, const char *dst, bool enabled) {
    if (access(dst, F_OK) == 0 && !same(src, dst)) return -1;
    if (!enabled) return unlink(dst) == 0 || access(dst, F_OK) != 0 ? 0 : -1;
    size_t len;
    void *data = read_file(src, &len);
    if (!data) return -1;
    int rc = write_file_atomic(dst, data, len);
    free(data);
    return rc;
}
int koreader_item_set(int item, bool enabled) {
    char src[1024], dst[1024], ko[800];
    if (!paths(item, src, dst) || !koreader_find(ko, sizeof ko)) return -1;
    char dir[1024];
    snprintf(dir, sizeof dir, "%s/%s", ko, item ? "patches" : "plugins/grimoire.koplugin");
    if (enabled && mkdir_p(dir) != 0) return -1;
    if (!item) {
        // This directory belongs to Grimoire. Update the plugin, including older Grimoire versions.
        static const char *const parts[] = {"main.lua", "_meta.lua"};
        for (int i = 0; i < 2; i++) {
            char from[1200], to[1200];
            snprintf(from, sizeof from, "%s/%s", src, parts[i]);
            snprintf(to, sizeof to, "%s/%s", dst, parts[i]);
            if (enabled) {
                size_t len; void *data = read_file(from, &len);
                if (!data) return -1;
                int rc = write_file_atomic(to, data, len);
                free(data);
                if (rc != 0) return -1;
            } else if (unlink(to) != 0 && access(to, F_OK) == 0) return -1;
        }
        if (!enabled) rmdir(dst);
        char mark[1024];
        snprintf(mark, sizeof mark, "%s/.koplugin-installed", store_root());
        // An explicit removal stays removed at later Library launches.
        if (write_file_atomic(mark, enabled ? "managed\n" : "removed\n", 8) != 0) return -1;
        return 0;
    }
    if (enabled && item == 3) {
        snprintf(dir, sizeof dir, "%s/fonts/Grimoire", ko);
        if (mkdir_p(dir) != 0) return -1;
        char font[1200];
        snprintf(font, sizeof font, "%s/TerminessNerdFontMono-Regular.ttf", dir);
        if (write_file_atomic(font, TerminessNerdFontMono_Regular_ttf, TerminessNerdFontMono_Regular_ttf_len) != 0) return -1;
        snprintf(font, sizeof font, "%s/TerminessNerdFontMono-Bold.ttf", dir);
        if (write_file_atomic(font, TerminessNerdFontMono_Bold_ttf, TerminessNerdFontMono_Bold_ttf_len) != 0) return -1;
    }
    return copy_owned(src, dst, enabled);
}
