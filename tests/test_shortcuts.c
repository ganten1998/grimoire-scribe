#include "shortcuts.h"
#include "store.h"
#include "testlib.h"
#include <unistd.h>

TEST(defaults_and_exact_modifiers) {
    Shortcuts m;
    shortcuts_load(&m, "/nonexistent/grimoire-shortcuts.ini");
    CHECK(m.count == 25 && m.rejected == 0);
    CHECK(shortcuts_lookup(&m, false, 2, 0) == SHORTCUT_TOOL_BASE + TOOL_PENCIL);
    CHECK(shortcuts_lookup(&m, false, 44, KEYMOD_CTRL) == SHORTCUT_UNDO);
    CHECK(shortcuts_lookup(&m, false, 44, KEYMOD_CTRL | KEYMOD_SHIFT) == SHORTCUT_NONE);
    CHECK(shortcuts_lookup(&m, true, 304, 0) == SHORTCUT_UNDO);
    CHECK(shortcuts_lookup(&m, false, 304, 0) == SHORTCUT_NONE);
    CHECK(shortcut_keycode("code767") == 767 && shortcut_keycode("code768") == -1);
    CHECK(shortcut_keycode("KEY_A") == -1);
}
TEST(custom_mappings_and_limits) {
    char path[800]; snprintf(path, sizeof path, "%s/shortcuts.ini", tl_tmp());
    const char *text = "# Custom\n key.Ctrl+SHIFT+z = redo\r\nkey.e=tool:pencil\nkey.e=tool:marker\n"
                       "pad.305=tool:eraser\npad.2=undo\nkey.Hyper+x=undo\nkey.1=erase-all\ngarbage\n";
    CHECK(write_file_atomic(path, text, strlen(text)) == 0);
    Shortcuts m; shortcuts_load(&m, path);
    CHECK(m.count == 3 && m.rejected == 4);
    CHECK(shortcuts_lookup(&m, false, 44, KEYMOD_CTRL | KEYMOD_SHIFT) == SHORTCUT_REDO);
    CHECK(shortcuts_lookup(&m, false, 18, 0) == SHORTCUT_TOOL_BASE + TOOL_MARKER);
    CHECK(shortcuts_write_defaults(path) == 0);
    shortcuts_load(&m, path);
    CHECK(m.count == 3); // enabling or reloading never overwrites user choices
    CHECK(write_file_atomic(path, "", 0) == 0);
    shortcuts_load(&m, path); CHECK(m.count == 0);
    FILE *f = fopen(path, "wb"); CHECK(f != NULL);
    if (f) { for (int i = 0; i < 65537; i++) fputc('x', f); fclose(f); }
    shortcuts_load(&m, path); CHECK(m.count == 0 && m.rejected == 1);
    unlink(path);
    CHECK(shortcuts_write_defaults(path) == 0);
    shortcuts_load(&m, path); CHECK(m.count == 25);
    unlink(path);
}
TEST(settings_are_opt_in) {
    char path[800]; snprintf(path, sizeof path, "%s/shortcuts-store", tl_tmp());
    CHECK(store_init(path) == 0);
    Settings s; settings_default(&s);
    CHECK(!s.shortcuts_enabled);
    s.shortcuts_enabled = true; CHECK(settings_save(&s) == 0);
    Settings loaded; CHECK(settings_load(&loaded) == 0 && loaded.shortcuts_enabled);
    loaded.shortcuts_enabled = false; CHECK(settings_save(&loaded) == 0);
    CHECK(settings_load(&s) == 0 && !s.shortcuts_enabled);
}
int main(void) {
    RUN(defaults_and_exact_modifiers);
    RUN(custom_mappings_and_limits);
    RUN(settings_are_opt_in);
    return tl_done();
}
