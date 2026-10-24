// Optional keyboard/gamepad mappings. Linux evdev key numbers are stable across Kindle models.
#pragma once
#include "common.h"
#define SHORTCUT_MAX 48
enum { KEYMOD_CTRL = 1, KEYMOD_SHIFT = 2, KEYMOD_ALT = 4, KEYMOD_META = 8 };
enum {
    SHORTCUT_NONE = 0, SHORTCUT_TOOL_BASE = 1, // plus ToolKind (0..9)
    SHORTCUT_UNDO = 20, SHORTCUT_REDO, SHORTCUT_PREV_PAGE, SHORTCUT_NEXT_PAGE,
    SHORTCUT_PREV_TOOL, SHORTCUT_NEXT_TOOL, SHORTCUT_WHITE,
};
typedef struct { int code, mods, action; bool gamepad; } ShortcutBinding;
typedef struct { ShortcutBinding bindings[SHORTCUT_MAX]; int count, rejected; } Shortcuts;
int shortcut_keycode(const char *name);
int shortcut_action(const char *name);
// Missing file: defaults. Existing empty file: no bindings. Never rewrites user mappings.
void shortcuts_load(Shortcuts *map, const char *path);
int shortcuts_lookup(const Shortcuts *map, bool gamepad, int code, int mods);
int shortcuts_write_defaults(const char *path); // creates only if absent
