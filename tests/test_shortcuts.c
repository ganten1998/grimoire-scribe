#include "shortcuts.h"
#include "store.h"
#include "testlib.h"
#include <unistd.h>

TEST(defaults_and_exact_modifiers) {
    Shortcuts m;
    shortcuts_load(&m, "/nonexistent/grimoire-shortcuts.ini");
    CHECK(m.count == 29 && m.rejected == 0);
    CHECK(shortcuts_lookup(&m, false, 2, 0) == SHORTCUT_TOOL_BASE + TOOL_PENCIL);
    // 1 to 9 in the tool picker's order: 3 the charcoal, 4 the ballpoint, 7 the fountain pen.
    for (int i = 0; i < TOOL_PENS; i++) CHECK(shortcuts_lookup(&m, false, 2 + i, 0) == SHORTCUT_TOOL_BASE + (int)tool_order[i]);
    CHECK(shortcuts_lookup(&m, false, 4, 0) == SHORTCUT_TOOL_BASE + TOOL_CHARCOAL);
    CHECK(shortcuts_lookup(&m, false, 8, 0) == SHORTCUT_TOOL_BASE + TOOL_FOUNTAIN);
    CHECK(shortcuts_lookup(&m, false, 44, KEYMOD_CTRL) == SHORTCUT_UNDO);
    CHECK(shortcuts_lookup(&m, false, 44, KEYMOD_CTRL | KEYMOD_SHIFT) == SHORTCUT_NONE);
    CHECK(shortcuts_lookup(&m, true, 304, 0) == SHORTCUT_UNDO);
    CHECK(shortcuts_lookup(&m, false, 304, 0) == SHORTCUT_NONE);
    CHECK(shortcut_keycode("code767") == 767 && shortcut_keycode("code768") == -1);
    CHECK(shortcut_keycode("KEY_A") == -1);
    CHECK(shortcuts_lookup(&m, true, 546, 0) == SHORTCUT_PREV_PAGE);
    CHECK(shortcuts_lookup(&m, true, 545, 0) == SHORTCUT_NEXT_TOOL);
    CHECK(shortcut_keycode("VolumeDown") == 114);
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
    shortcuts_load(&m, path); CHECK(m.count == 29);
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
TEST(controller_ini_learns_one_button_per_action) {
    char sp[800], cp[800];
    snprintf(sp, sizeof sp, "%s/shortcuts-c.ini", tl_tmp());
    snprintf(cp, sizeof cp, "%s/controller.ini", tl_tmp());
    unlink(sp); unlink(cp);
    Shortcuts m;
    shortcuts_load_all(&m, sp, cp);  // neither file: the defaults
    CHECK(m.count == 29 && shortcuts_lookup(&m, true, 304, 0) == SHORTCUT_UNDO);
    int codes[4];
    CHECK(shortcuts_pads_for(&m, SHORTCUT_PREV_PAGE, codes, 4) == 2);  // Select and the D-pad's Left
    // Learn: East is Undo now; South loses it (it was Undo's), and the eraser loses East.
    CHECK(shortcuts_learn(&m, cp, 305, SHORTCUT_UNDO) == 0);
    CHECK(shortcuts_lookup(&m, true, 305, 0) == SHORTCUT_UNDO && shortcuts_lookup(&m, true, 304, 0) == SHORTCUT_NONE);
    CHECK(shortcuts_pads_for(&m, SHORTCUT_TOOL_BASE + TOOL_ERASER, NULL, 0) == 0);
    CHECK(shortcuts_learn(&m, cp, 316, SHORTCUT_PREV_PAGE) == 0);  // a button with no name: its number
    CHECK(shortcuts_pads_for(&m, SHORTCUT_PREV_PAGE, codes, 4) == 1 && codes[0] == 316);
    CHECK(shortcuts_lookup(&m, false, 44, KEYMOD_CTRL) == SHORTCUT_UNDO);  // keys untouched
    size_t len; char *text = read_file(cp, &len);
    CHECK(text && strstr(text, "pad.East=undo\n") && strstr(text, "pad.316=previous-page\n") && !strstr(text, "key."));
    free(text);
    // Read back: the same gamepad buttons, from controller.ini alone; shortcuts.ini's pad. lines set aside.
    const char *custom = "key.e=tool:marker\npad.South=redo\n";
    CHECK(write_file_atomic(sp, custom, strlen(custom)) == 0);
    Shortcuts back;
    shortcuts_load_all(&back, sp, cp);
    CHECK(shortcuts_lookup(&back, true, 304, 0) == SHORTCUT_NONE && shortcuts_lookup(&back, true, 305, 0) == SHORTCUT_UNDO);
    CHECK(shortcuts_lookup(&back, false, 18, 0) == SHORTCUT_TOOL_BASE + TOOL_MARKER);
    // A key line in controller.ini is not its to hold.
    const char *odd = "pad.North=redo\nkey.z=undo\n";
    CHECK(write_file_atomic(cp, odd, strlen(odd)) == 0);
    shortcuts_load_all(&back, sp, cp);
    CHECK(back.rejected == 1 && shortcuts_lookup(&back, false, 44, 0) == SHORTCUT_NONE && shortcuts_lookup(&back, true, 307, 0) == SHORTCUT_REDO);
    // A file that cannot be written leaves the map as it was.
    char bad[900]; snprintf(bad, sizeof bad, "%s/no-such-dir/controller.ini", tl_tmp());
    Shortcuts before = back;
    CHECK(shortcuts_learn(&back, bad, 308, SHORTCUT_UNDO) != 0 && back.count == before.count &&
          shortcuts_lookup(&back, true, 308, 0) == shortcuts_lookup(&before, true, 308, 0));
    char name[16];
    shortcut_pad_name(544, name, sizeof name); CHECK(!strcmp(name, "Up"));
    CHECK(!strcmp(shortcut_action_name(SHORTCUT_TOOL_BASE + TOOL_ERASER), "tool:eraser") && !shortcut_action_name(999));
    unlink(sp); unlink(cp);
}
// Every action has a name the files keep, which reads back as it; one no version knows yet is
// refused and counted while the other lines load (as older versions treat the names added later).
TEST(every_action_by_name) {
    int named = 0;
    for (int a = 1; a <= SHORTCUT_LAST; a++) {
        const char *name = shortcut_action_name(a);
        if (!name) continue;
        named++;
        CHECK(shortcut_action(name) == a);
    }
    CHECK(named == 10 + 8 + 15);  // the tools, the first actions, and 2.0.0-beta.5's
    char path[800]; snprintf(path, sizeof path, "%s/shortcuts-new.ini", tl_tmp());
    const char *text = "pad.L2=hold-eraser\npad.R2=eraser-mode\npad.600=clear-page\npad.601=first-page\n"
                       "pad.602=last-page\npad.603=new-page\npad.604=fit-page\npad.605=pages\n"
                       "pad.606=tool-picker\npad.607=paper\npad.608=display\npad.609=pen-feel\n"
                       "pad.610=fold-panel\npad.611=library\npad.612=close-menu\nkey.z=undo\n"
                       "pad.North=menu:frobnicate\nkey.x=hold-pen\npad.613=tool:crayon\n";
    CHECK(write_file_atomic(path, text, strlen(text)) == 0);
    Shortcuts m; shortcuts_load(&m, path);
    CHECK(m.count == 16 && m.rejected == 3);
    CHECK(shortcuts_lookup(&m, true, 312, 0) == SHORTCUT_HOLD_ERASER && shortcuts_lookup(&m, true, 612, 0) == SHORTCUT_CLOSE_MENU);
    CHECK(shortcuts_lookup(&m, false, 44, 0) == SHORTCUT_UNDO && shortcuts_lookup(&m, true, 307, 0) == SHORTCUT_NONE);
    unlink(path);
    // Learned into controller.ini by name: every action a button, which read back, with room to spare.
    char sp[800], cp[800];
    snprintf(sp, sizeof sp, "%s/shortcuts-all.ini", tl_tmp());
    snprintf(cp, sizeof cp, "%s/controller-all.ini", tl_tmp());
    unlink(sp); unlink(cp);
    shortcuts_load_all(&m, sp, cp);
    int code = 600;
    for (int a = 1; a <= SHORTCUT_LAST; a++)
        if (shortcut_action_name(a)) CHECK(shortcuts_learn(&m, cp, code++, a) == 0);
    CHECK(shortcuts_learn(&m, cp, 312, SHORTCUT_HOLD_ERASER) == 0 && shortcuts_learn(&m, cp, 316, SHORTCUT_CLOSE_MENU) == 0);
    size_t len; char *saved = read_file(cp, &len);
    CHECK(saved && strstr(saved, "pad.L2=hold-eraser\n") && strstr(saved, "pad.316=close-menu\n") && strstr(saved, "=tool:calligraphy\n"));
    free(saved);
    Shortcuts back; shortcuts_load_all(&back, sp, cp);
    CHECK(back.rejected == 0 && back.count == 17 + named);  // the default keys, and a button each
    for (int a = 1; a <= SHORTCUT_LAST; a++)
        if (shortcut_action_name(a)) CHECK(shortcuts_pads_for(&back, a, NULL, 0) == 1);
    CHECK(shortcuts_lookup(&back, true, 312, 0) == SHORTCUT_HOLD_ERASER);
    unlink(sp); unlink(cp);
}
int main(void) {
    RUN(defaults_and_exact_modifiers);
    RUN(custom_mappings_and_limits);
    RUN(settings_are_opt_in);
    RUN(controller_ini_learns_one_button_per_action);
    RUN(every_action_by_name);
    return tl_done();
}
