// nav.h: notebook section names, kept apart from page ink in nav.ini. Page ids, not page
// positions, keep names attached when pages move or notes are added.
#pragma once
#include "store.h"

#define NAV_NAME_LEN 96
#define NAV_FILE_MAX (256 * 1024)
#define NAV_UNKNOWN_MAX (16 * 1024)

typedef struct { uint32_t pid; char text[NAV_NAME_LEN]; } NavName;
typedef struct {
    NavName *names;  // sorted by page id
    int count, capacity;
    char *unknown;
    size_t unknown_len;
    bool writable;  // false after an unreadable or oversized file: never overwrite it
} Nav;

void nav_init(Nav *nav);
void nav_free(Nav *nav);
// nav must be initialized. Missing is an empty writable index; other read failures hide the
// names and refuse edits. Unknown lines are kept, including future link records.
int nav_load(const char *id, Nav *nav);
const char *nav_name(const Nav *nav, uint32_t pid);  // empty when unnamed
// A single line, trimmed, at most 95 bytes, cut between UTF-8 characters. Empty removes a name.
int nav_set_name(Nav *nav, uint32_t pid, const char *name);
// Atomic. Names of deleted pages are pruned only after a successful save; unknown lines stay.
int nav_save(const char *id, Nav *nav, const NotebookInfo *nb);
// Named pages in notebook order; returns the full count, writes at most cap page indices.
int nav_index(const Nav *nav, const NotebookInfo *nb, int *indices, int cap);
int nav_section_of(const Nav *nav, const NotebookInfo *nb, int index);  // page index, or -1
