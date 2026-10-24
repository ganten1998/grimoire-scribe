// KOReader discovery and optional integration files. No KOReader settings are executed.
#pragma once
#include <stdbool.h>
#include <stddef.h>
bool koreader_find(char *out, size_t cap);
// 0 plugin, 1 screenshot gestures, 2 square corners, 3 Terminess menus.
const char *koreader_item_name(int item);
bool koreader_item_installed(int item);
// Installs only our files. A different file at the same path is kept (returns -1).
int koreader_item_set(int item, bool enabled);
