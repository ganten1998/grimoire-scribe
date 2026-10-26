// nav.h: a notebook's section names (nav.ini) and links (links.ini), kept apart from page ink. Page
// ids, not page positions, keep them attached when pages move or notes are added.
#pragma once
#include "page.h"
#include "store.h"

#define NAV_NAME_LEN 96
#define NAV_FILE_MAX (256 * 1024)
#define NAV_UNKNOWN_MAX (16 * 1024)
#define NAV_LINKS_MAX 4000       // in a notebook
#define NAV_LINKS_PER_PAGE 64
#define NAV_TARGET_LEN 16
#define NAV_TO_INDEX (-2)        // nav_link_page: the link goes to the Index
#define NAV_LINK_MIN_MM 7.f      // a link's box is at least this a side, to tap or click

typedef struct { uint32_t pid; char text[NAV_NAME_LEN]; } NavName;
// A link (link.<pid>=<seed>,<target> in links.ini): the ink strokes on page pid with this seed are
// its box, drawn as ordinary ink, so erasing or undoing them takes the link away, and older versions
// show the box. Targets: "index" (the Index), "p<page id>" (a page); any other (a newer version's)
// is carried and does nothing. A link whose box is gone stays, doing nothing, until its page goes.
typedef struct { uint32_t pid, seed; char target[NAV_TARGET_LEN]; } NavLink;
typedef struct {
    NavName *names;  // sorted by page id
    int count, capacity;
    char *unknown;
    size_t unknown_len;
    bool writable;  // false after an unreadable or oversized nav.ini: never overwrite it
    NavLink *links;  // sorted by page id, then seed
    int nlinks, links_cap;
    char *links_unknown;  // links.ini's lines this version does not read, carried as nav.ini's are
    size_t links_unknown_len;
    bool links_writable;  // the same for links.ini
    bool links_file;      // links.ini is there (else it is written only once there is a link)
} Nav;

void nav_init(Nav *nav);
void nav_free(Nav *nav);
// nav must be initialized. Missing is an empty writable index; other read failures hide the
// names and refuse edits. links.ini the same, on its own: either can fail with the other read.
// Unknown lines of both are kept. -1 when either failed.
int nav_load(const char *id, Nav *nav);
const char *nav_name(const Nav *nav, uint32_t pid);  // empty when unnamed
// A single line, trimmed, at most 95 bytes, cut between UTF-8 characters. Empty removes a name.
int nav_set_name(Nav *nav, uint32_t pid, const char *name);
// Atomic. Names of deleted pages are pruned only after a successful save; unknown lines stay.
int nav_save(const char *id, Nav *nav, const NotebookInfo *nb);
// Named pages in notebook order; returns the full count, writes at most cap page indices.
int nav_index(const Nav *nav, const NotebookInfo *nb, int *indices, int cap);
int nav_section_of(const Nav *nav, const NotebookInfo *nb, int index);  // page index, or -1

// ---- links ----

// Adds the link of the strokes with `seed` on page pid, or changes its target. target: "index" or
// "p<page id>". -1 when it cannot (not writable, a bad target, the limits above).
int nav_link_set(Nav *nav, uint32_t pid, uint32_t seed, const char *target);
void nav_link_remove(Nav *nav, uint32_t pid, uint32_t seed);
// Page pid's links: how many, and the first in nav->links (*first).
int nav_links_of(const Nav *nav, uint32_t pid, int *first);
// Where a link goes in nb: a page index, NAV_TO_INDEX, or -1 (a page gone, a target this version
// does not know).
int nav_link_page(const NavLink *l, const NotebookInfo *nb);
// The boxes of page p's links (p is page pid): for each link whose strokes are there, the box of
// their points (the ink strokes only), made at least min_px a side around its centre. Writes at most
// cap links and their boxes; returns how many it wrote.
int nav_link_areas(const Nav *nav, const Page *p, uint32_t pid, float min_px, NavLink *links, Rect *areas, int cap);
// The link whose box holds (x, y) on page p, the one drawn last when boxes overlap. False for none.
bool nav_link_at(const Nav *nav, const Page *p, uint32_t pid, float x, float y, float min_px, NavLink *out);
// The seed for a new ink stroke on page p (page pid): want, or the next after it that no ink stroke on
// p and no link of pid has, so a link's box never takes in a stroke drawn after it.
uint32_t nav_new_seed(const Nav *nav, const Page *p, uint32_t pid, uint32_t want);
// Writes links.ini, leaving out the links of pages no longer in nb and those going to such pages.
// -1 when not links_writable or the write failed (then nothing is left out).
int nav_save_links(const char *id, Nav *nav, const NotebookInfo *nb);
