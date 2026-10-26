// Optional keyboard/gamepad mappings. Linux evdev key numbers are stable across Kindle models.
#pragma once
#include "common.h"
#define SHORTCUT_MAX 64
enum { KEYMOD_CTRL = 1, KEYMOD_SHIFT = 2, KEYMOD_ALT = 4, KEYMOD_META = 8 };
enum {
    SHORTCUT_NONE = 0, SHORTCUT_TOOL_BASE = 1, // plus ToolKind (0..9)
    SHORTCUT_UNDO = 20, SHORTCUT_REDO, SHORTCUT_PREV_PAGE, SHORTCUT_NEXT_PAGE,
    SHORTCUT_PREV_TOOL, SHORTCUT_NEXT_TOOL, SHORTCUT_WHITE,
    SHORTCUT_INDEX,
};
typedef struct { int code, mods, action; bool gamepad; } ShortcutBinding;
typedef struct { ShortcutBinding bindings[SHORTCUT_MAX]; int count, rejected; } Shortcuts;
int shortcut_keycode(const char *name);
int shortcut_action(const char *name);
// Missing file: defaults. Existing empty file: no bindings. Never rewrites user mappings.
void shortcuts_load(Shortcuts *map, const char *path);
int shortcuts_lookup(const Shortcuts *map, bool gamepad, int code, int mods);
int shortcuts_write_defaults(const char *path); // creates only if absent
// controller.ini (Settings > Controller > Buttons): the gamepad buttons learned there. While it is
// there they come from it alone (shortcuts.ini's pad.* lines are set aside); key.* lines are
// always shortcuts.ini's. Grimoire writes it whole, atomically; a line it cannot read is rejected
// and counted, as in shortcuts.ini.
void shortcuts_load_all(Shortcuts *map, const char *shortcuts_path, const char *controller_path);
// Gamepad button code does action from now on, as its only button: the button's old action and the
// action's other buttons lose theirs. Writes controller.ini with every gamepad binding; -1 when it
// could not (map unchanged).
int shortcuts_learn(Shortcuts *map, const char *controller_path, int code, int action);
// The gamepad buttons for action: how many there are, the first cap of them in codes.
int shortcuts_pads_for(const Shortcuts *map, int action, int *codes, int cap);
// A gamepad button as the files name it: "South", "Up", or its number ("316").
void shortcut_pad_name(int code, char *out, size_t cap);
// An action's name in the files ("undo", "tool:eraser"), or NULL.
const char *shortcut_action_name(int action);
