#include "shortcuts.h"
#include "hidkeys.h"
#include "page.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <strings.h>
#include <unistd.h>
#include <sys/stat.h>
#include <ctype.h>

// The D-pad's starter lines: in a new shortcuts.ini, and in memory for one that binds none of its
// directions (shortcuts_load_all).
#define DPAD_LINES "pad.Up=previous-tool\npad.Down=next-tool\npad.Left=previous-page\npad.Right=next-page\n"
static const char defaults[] =
    "# Grimoire shortcuts. Enable in Settings > Controller. Reload there after editing.\n"
    "# key.NAME or key.Ctrl+NAME; pad.NUMBER is a Linux evdev gamepad button code.\n"
    "# 1 to 9: the tool picker's pens in its order, left to right and down.\n"
    "key.1=tool:pencil\nkey.2=tool:mech\nkey.3=tool:charcoal\nkey.4=tool:ballpoint\n"
    "key.5=tool:fineliner\nkey.6=tool:marker\nkey.7=tool:fountain\nkey.8=tool:calligraphy\n"
    "key.9=tool:highlighter\nkey.e=tool:eraser\nkey.w=white\nkey.Ctrl+z=undo\nkey.Ctrl+y=redo\n"
    "key.Left=previous-page\nkey.Right=next-page\nkey.PageUp=previous-tool\nkey.PageDown=next-tool\n"
    "# Generic evdev gamepad defaults; controllers may report different codes.\n"
    "pad.304=undo\npad.305=tool:eraser\npad.307=redo\npad.308=white\n"
    "pad.310=previous-tool\npad.311=next-tool\npad.314=previous-page\npad.315=next-page\n"
    "# Digital D-pad buttons and standard HAT0 directions share these bindings.\n"
    DPAD_LINES;

int shortcut_keycode(const char *name) {
    // Linux input-event-codes.h, not SDL's unrelated USB scan code numbering.
    static const struct { const char *name; int code; } keys[] = {
        {"1",2},{"2",3},{"3",4},{"4",5},{"5",6},{"6",7},{"7",8},{"8",9},{"9",10},{"0",11},
        {"q",16},{"w",17},{"e",18},{"r",19},{"t",20},{"y",21},{"u",22},{"i",23},{"o",24},{"p",25},
        {"a",30},{"s",31},{"d",32},{"f",33},{"g",34},{"h",35},{"j",36},{"k",37},{"l",38},
        {"z",44},{"x",45},{"c",46},{"v",47},{"b",48},{"n",49},{"m",50},
        {"Left",105},{"Right",106},{"Up",103},{"Down",108},{"PageUp",104},{"PageDown",109},
        {"Space",57},{"Enter",28},{"Tab",15},{"Escape",1},
        {"VolumeUp",115},{"VolumeDown",114},{"PlayPause",164},{"NextSong",163},{"PreviousSong",165},
        {"Home",102},{"End",107},{"Back",158},
    };
    for (size_t i = 0; i < sizeof keys / sizeof *keys; i++) if (!strcasecmp(name, keys[i].name)) return keys[i].code;
    if (!strncasecmp(name, "code", 4)) {
        char *end; long n = strtol(name + 4, &end, 10);
        if (end != name + 4 && !*end && n > 0 && n <= 767) return (int)n;
    }
    return -1;
}

static const char *const k_tools[] = {"pencil","mech","ballpoint","eraser","highlighter","fountain","fineliner","marker","charcoal","calligraphy"};
static const struct { const char *name; int action; } k_actions[] = {
    {"undo",SHORTCUT_UNDO},{"redo",SHORTCUT_REDO},{"previous-page",SHORTCUT_PREV_PAGE},
    {"next-page",SHORTCUT_NEXT_PAGE},{"previous-tool",SHORTCUT_PREV_TOOL},{"next-tool",SHORTCUT_NEXT_TOOL},{"white",SHORTCUT_WHITE},
    {"index",SHORTCUT_INDEX},{"hold-eraser",SHORTCUT_HOLD_ERASER},{"eraser-mode",SHORTCUT_ERASER_MODE},
    {"clear-page",SHORTCUT_CLEAR_PAGE},{"first-page",SHORTCUT_FIRST_PAGE},{"last-page",SHORTCUT_LAST_PAGE},
    {"new-page",SHORTCUT_NEW_PAGE},{"fit-page",SHORTCUT_FIT_PAGE},{"pages",SHORTCUT_PAGES},
    {"tool-picker",SHORTCUT_TOOL_PICKER},{"paper",SHORTCUT_PAPER},{"display",SHORTCUT_DISPLAY},
    {"pen-feel",SHORTCUT_PEN_FEEL},{"fold-panel",SHORTCUT_FOLD_PANEL},{"library",SHORTCUT_LIBRARY},
    {"close-menu",SHORTCUT_CLOSE_MENU},
};
// Gamepad buttons by name: the Linux gamepad convention's, the D-pad (HAT0 decodes to these), and
// the analog triggers (hidkeys_trigger_event reports them as these).
static const struct { const char *name; int code; } k_buttons[] = {
    {"Up",544},{"Down",545},{"Left",546},{"Right",547},
    {"South",304},{"East",305},{"North",307},{"West",308},
    {"L1",310},{"R1",311},{"L2",312},{"R2",313},{"Select",314},{"Start",315},
    {"LT",HIDKEYS_LT},{"RT",HIDKEYS_RT},
};

const char *shortcut_action_name(int action) {
    static char tool[24];
    if (action >= SHORTCUT_TOOL_BASE && action < SHORTCUT_TOOL_BASE + TOOL_COUNT) {
        snprintf(tool, sizeof tool, "tool:%s", k_tools[action - SHORTCUT_TOOL_BASE]);
        return tool;
    }
    for (size_t i = 0; i < sizeof k_actions / sizeof *k_actions; i++)
        if (k_actions[i].action == action) return k_actions[i].name;
    return NULL;
}

void shortcut_pad_name(int code, char *out, size_t cap) {
    for (size_t i = 0; i < sizeof k_buttons / sizeof *k_buttons; i++)
        if (k_buttons[i].code == code) {
            snprintf(out, cap, "%s", k_buttons[i].name);
            return;
        }
    snprintf(out, cap, "%d", code);
}

void shortcut_pad_label(int code, bool switch_layout, char *out, size_t cap) {
    // The Pro Controller's simple report, in its order from BTN_A (hidkeys_switch).
    static const char *const k_switch[] = {"B", "A", "Y", "X", "L", "R", "ZL", "ZR", "-", "+",
                                           "LS", "RS", "Home", "Capture"};
    if (switch_layout && code >= 304 && code < 304 + (int)(sizeof k_switch / sizeof *k_switch))
        snprintf(out, cap, "%s", k_switch[code - 304]);
    else shortcut_pad_name(code, out, cap);
}

int shortcut_action(const char *name) {
    const char *const *tools = k_tools;
    if (!strncmp(name, "tool:", 5)) {
        for (int i = 0; i < TOOL_COUNT; i++) if (!strcmp(name + 5, tools[i])) return SHORTCUT_TOOL_BASE + i;
        return SHORTCUT_NONE;
    }
    for (size_t i = 0; i < sizeof k_actions / sizeof *k_actions; i++) if (!strcmp(name, k_actions[i].name)) return k_actions[i].action;
    return SHORTCUT_NONE;
}
static bool binding(void *ctx, const char *key, const char *value) {
    Shortcuts *m = ctx;
    ShortcutBinding b = {.code = -1, .action = shortcut_action(value)};
    if (!strncmp(key, "pad.", 4)) {
        char *end; long n = strtol(key + 4, &end, 10);
        if (end != key + 4 && !*end && n >= 256 && n <= 767) b.code = (int)n;
        for (size_t i = 0; i < sizeof k_buttons / sizeof *k_buttons; i++)
            if (!strcasecmp(key + 4, k_buttons[i].name)) b.code = k_buttons[i].code;
        b.gamepad = true;
    } else if (!strncmp(key, "key.", 4)) {
        const char *p = key + 4, *plus;
        while ((plus = strchr(p, '+'))) {
            size_t n = (size_t)(plus - p);
            if (n == 4 && !strncasecmp(p, "Ctrl", n)) b.mods |= KEYMOD_CTRL;
            else if (n == 5 && !strncasecmp(p, "Shift", n)) b.mods |= KEYMOD_SHIFT;
            else if (n == 3 && !strncasecmp(p, "Alt", n)) b.mods |= KEYMOD_ALT;
            else if (n == 4 && !strncasecmp(p, "Meta", n)) b.mods |= KEYMOD_META;
            else { m->rejected++; return false; }
            p = plus + 1;
        }
        b.code = shortcut_keycode(p);
    }
    if (b.code < 0 || !b.action) { m->rejected++; return false; }
    for (int i = 0; i < m->count; i++) {
        ShortcutBinding *old = &m->bindings[i];
        if (old->code == b.code && old->mods == b.mods && old->gamepad == b.gamepad) { *old = b; return true; }
    }
    if (m->count == SHORTCUT_MAX) { m->rejected++; return false; }
    m->bindings[m->count++] = b;
    return true;
}
static char *trim(char *s) {
    while (isspace((unsigned char)*s)) s++;
    size_t n = strlen(s);
    while (n && isspace((unsigned char)s[n - 1])) s[--n] = 0;
    return s;
}
static void parse(Shortcuts *map, char *text) {
    char *save = NULL;
    for (char *line = strtok_r(text, "\n", &save); line; line = strtok_r(NULL, "\n", &save)) {
        line = trim(line);
        if (!*line || *line == '#' || *line == ';') continue;
        char *eq = strchr(line, '=');
        if (!eq) { map->rejected++; continue; }
        *eq = 0; binding(map, trim(line), trim(eq + 1));
    }
}
void shortcuts_load(Shortcuts *map, const char *path) {
    memset(map, 0, sizeof *map);
    char *text;
    if (access(path, F_OK) == 0) {
        struct stat st;
        if (stat(path, &st) || !S_ISREG(st.st_mode) || st.st_size > 65536) { map->rejected++; return; }
        // Bound the read too, including a file changed since stat.
        FILE *f = fopen(path, "rb");
        text = malloc(65538);
        if (!f || !text) { if (f) fclose(f); free(text); map->rejected++; return; }
        size_t n = fread(text, 1, 65537, f);
        bool failed = ferror(f) || n > 65536;
        fclose(f);
        if (failed) { free(text); map->rejected++; return; }
        text[n] = 0;
    } else text = strdup(defaults);
    if (!text) return;
    parse(map, text);
    free(text);
}
int shortcuts_lookup(const Shortcuts *map, bool gamepad, int code, int mods) {
    for (int i = 0; i < map->count; i++) {
        const ShortcutBinding *b = &map->bindings[i];
        if (b->gamepad == gamepad && b->code == code && b->mods == mods) return b->action;
    }
    return SHORTCUT_NONE;
}
int shortcuts_write_defaults(const char *path) {
    if (access(path, F_OK) == 0) return 0;
    return write_file_atomic(path, defaults, sizeof defaults - 1);
}

// No version before 2.0.0-beta.5 read a D-pad or a hat, so a shortcuts.ini written then binds none of
// its directions: such a file gets the D-pad's starter lines too, in memory (as many as there is room
// for, none counted as refused). An empty file keeps no mappings.
static void add_dpad(Shortcuts *map) {
    for (int i = 0; i < map->count; i++)
        if (map->bindings[i].gamepad && map->bindings[i].code >= 544 && map->bindings[i].code <= 547) return;
    Shortcuts dpad = {0};
    char text[] = DPAD_LINES;
    parse(&dpad, text);
    for (int i = 0; i < dpad.count && map->count < SHORTCUT_MAX; i++) map->bindings[map->count++] = dpad.bindings[i];
}

void shortcuts_load_all(Shortcuts *map, const char *shortcuts_path, const char *controller_path) {
    shortcuts_load(map, shortcuts_path);
    if (map->count) add_dpad(map);
    if (!controller_path || access(controller_path, F_OK) != 0) return;
    Shortcuts pads;
    shortcuts_load(&pads, controller_path);
    int kept = 0;
    for (int i = 0; i < map->count; i++)
        if (!map->bindings[i].gamepad) map->bindings[kept++] = map->bindings[i];
    map->count = kept;
    map->rejected += pads.rejected;
    for (int i = 0; i < pads.count; i++) {
        if (!pads.bindings[i].gamepad || map->count == SHORTCUT_MAX) {
            map->rejected++;  // (a key there, or no room)
            continue;
        }
        map->bindings[map->count++] = pads.bindings[i];
    }
}

// controller.ini with every gamepad binding of map, written whole (atomically); -1 when it could not be.
static int write_pads(const Shortcuts *next, const char *controller_path) {
    char text[4096];
    size_t len = (size_t)snprintf(text, sizeof text,
                                  "# Grimoire: controller buttons, learned in Settings > Controller > Buttons.\n"
                                  "# While this file is here, shortcuts.ini's pad. lines are set aside.\n");
    for (int i = 0; i < next->count; i++) {
        const ShortcutBinding *b = &next->bindings[i];
        if (!b->gamepad) continue;
        char pad[16];
        shortcut_pad_name(b->code, pad, sizeof pad);
        int n = snprintf(text + len, sizeof text - len, "pad.%s=%s\n", pad, shortcut_action_name(b->action));
        if (n < 0 || (size_t)n >= sizeof text - len) return -1;
        len += (size_t)n;
    }
    return write_file_atomic(controller_path, text, len) != 0 ? -1 : 0;
}

int shortcuts_learn(Shortcuts *map, const char *controller_path, int code, int action) {
    const char *name = shortcut_action_name(action);
    if (!name || code < 256 || code > 767) return -1;
    Shortcuts next = *map;
    int kept = 0;
    for (int i = 0; i < next.count; i++) {
        const ShortcutBinding *b = &next.bindings[i];
        if (b->gamepad && (b->code == code || b->action == action)) continue;
        next.bindings[kept++] = *b;
    }
    next.count = kept;
    if (next.count == SHORTCUT_MAX) return -1;
    next.bindings[next.count++] = (ShortcutBinding){.code = code, .action = action, .gamepad = true};
    if (write_pads(&next, controller_path) != 0) return -1;
    *map = next;
    return 0;
}

int shortcuts_forget(Shortcuts *map, const char *controller_path, int action, int action2) {
    Shortcuts next = *map;
    int kept = 0;
    for (int i = 0; i < next.count; i++) {
        const ShortcutBinding *b = &next.bindings[i];
        if (b->gamepad && (b->action == action || (action2 && b->action == action2))) continue;
        next.bindings[kept++] = *b;
    }
    if (kept == next.count) return 0;  // (neither had a button: nothing to write)
    next.count = kept;
    if (write_pads(&next, controller_path) != 0) return -1;
    *map = next;
    return 0;
}

int shortcuts_pads_for(const Shortcuts *map, int action, int *codes, int cap) {
    int n = 0;
    for (int i = 0; i < map->count; i++) {
        const ShortcutBinding *b = &map->bindings[i];
        if (!b->gamepad || b->action != action) continue;
        if (codes && n < cap) codes[n] = b->code;
        n++;
    }
    return n;
}
