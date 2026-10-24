#include "testlib.h"
#include "koreader.h"
#include "store.h"
#include "bgimage.h"
#include <unistd.h>
#include <sys/stat.h>

TEST(discovery_and_owned_files) {
    char root[800], us[800], ko[1000], p[1200], got[1200], src[1200];
    snprintf(root, sizeof root, "%s/ko-manager-%ld", tl_tmp(), (long)getpid());
    snprintf(us, sizeof us, "%s/us", root);
    setenv("GRIMOIRE_US", us, 1);
    unsetenv("KOREADER_DIR");
    CHECK(store_init(root) == 0);
    CHECK(!koreader_find(got, sizeof got));
    snprintf(ko, sizeof ko, "%s/extensions/koreader", us);
    CHECK(mkdir_p(ko) == 0);
    snprintf(p, sizeof p, "%s/reader.lua", ko);
    CHECK(write_file_atomic(p, "KOReader", 8) == 0);
    CHECK(koreader_find(got, sizeof got) && !strcmp(got, ko));
    snprintf(src, sizeof src, "%s/app/koreader/patches", root);
    CHECK(mkdir_p(src) == 0);
    snprintf(src, sizeof src, "%s/app/koreader/patches/%s", root, koreader_item_name(1));
    CHECK(write_file_atomic(src, "our patch", 9) == 0);
    CHECK(!koreader_item_installed(1));
    CHECK(koreader_item_set(1, true) == 0 && koreader_item_installed(1));
    CHECK(koreader_item_set(1, true) == 0);  // idempotent
    CHECK(koreader_item_set(1, false) == 0 && !koreader_item_installed(1));
    CHECK(koreader_item_set(1, false) == 0);
    snprintf(p, sizeof p, "%s/patches/%s", ko, koreader_item_name(1));
    CHECK(write_file_atomic(p, "someone else's patch", 20) == 0);
    CHECK(koreader_item_set(1, true) != 0);
    CHECK(koreader_item_set(1, false) != 0);
    size_t len; char *data = read_file(p, &len);
    CHECK(data && len == 20 && !memcmp(data, "someone else's patch", 20));
    free(data);
    CHECK(koreader_item_set(-1, true) != 0 && koreader_item_set(4, true) != 0);
    snprintf(src, sizeof src, "%s/app/koreader/patches/%s", root, koreader_item_name(3));
    CHECK(write_file_atomic(src, "new font patch", 14) == 0);
    snprintf(p, sizeof p, "%s/app/koreader/legacy", root); CHECK(mkdir_p(p) == 0);
    snprintf(p, sizeof p, "%s/app/koreader/legacy/%s", root, koreader_item_name(3));
    CHECK(write_file_atomic(p, "old font patch", 14) == 0);
    snprintf(p, sizeof p, "%s/patches/%s", ko, koreader_item_name(3));
    CHECK(write_file_atomic(p, "old font patch", 14) == 0);
    CHECK(koreader_item_set(3, true) == 0); // known previous version can upgrade explicitly
    data = read_file(p, &len); CHECK(data && len == 14 && !memcmp(data, "new font patch", 14)); free(data);
    CHECK(koreader_item_set(3, false) == 0);
    snprintf(src, sizeof src, "%s/app/koreader/grimoire.koplugin", root);
    CHECK(mkdir_p(src) == 0);
    snprintf(src, sizeof src, "%s/app/koreader/grimoire.koplugin/main.lua", root);
    CHECK(write_file_atomic(src, "new plugin", 10) == 0);
    snprintf(src, sizeof src, "%s/app/koreader/grimoire.koplugin/_meta.lua", root);
    CHECK(write_file_atomic(src, "metadata", 8) == 0);
    CHECK(koreader_item_set(0, true) == 0 && koreader_item_installed(0));
    CHECK(koreader_item_set(0, false) == 0 && !koreader_item_installed(0));
    snprintf(p, sizeof p, "%s/.koplugin-installed", root);
    data = read_file(p, &len);
    CHECK(data && len == 8 && !memcmp(data, "removed\n", 8)); free(data);
    unsetenv("GRIMOIRE_US");
}

TEST(wallpaper_and_setting) {
    char root[800], path[1000];
    snprintf(root, sizeof root, "%s/ko-wallpaper-%ld", tl_tmp(), (long)getpid());
    CHECK(store_init(root) == 0);
    Settings s, t;
    settings_default(&s); s.sleep_mode = 3;
    CHECK(settings_save(&s) == 0 && settings_load(&t) == 0 && t.sleep_mode == 3);
    Canvas in = {0}, out = {0};
    CHECK(canvas_init(&in, 20, 30) == 0);
    canvas_fill(&in, (Rect){0, 0, 20, 30}, 83);
    snprintf(path, sizeof path, "%s/koreader-sleep.png", root);
    BgPicture picture = {.px = in.px, .w = 20, .h = 30, .ch = 1};
    CHECK(bgimage_write_png(path, &picture) == 0);
    CHECK(store_load_koreader_sleep(&out) == 0 && out.w == 20 && out.h == 30 && out.px[0] == 83);
    canvas_free(&out); canvas_free(&in);
    CHECK(write_file_atomic(path, "not an image", 12) == 0);
    CHECK(store_load_koreader_sleep(&out) != 0 && !out.px);
}
int main(void) {
    RUN(discovery_and_owned_files);
    RUN(wallpaper_and_setting);
    return tl_done();
}
