// annotate.h: "Annotate this page in Grimoire", from KOReader (docs/FORMAT.md, Annotated pages).
// KOReader's Grimoire plugin (kindle/koreader/grimoire.koplugin) draws the page its reader is on
// into a picture, writes it with a request into grimoire/import/.annotate/ (a dot-folder the folder
// import skips), and hands over to Grimoire with --annotate, which kindle/app/grimoire.sh passes on
// (GRIMOIRE_ANNOTATE=1) only when KOReader hosts Grimoire. At startup Grimoire finds the notebook
// for that book (notebook.ini source=), or makes one named after the book's title, and the page for
// that book page (the notebook's source.ini), or adds one: a document page whose paper is the
// picture, among the book's other pages in book order. The editor opens on it, and the request goes.
//
// grimoire.log goes into public bug reports: the book's title, its file and its key are never
// logged here, only notebook ids, page numbers and counts.
#pragma once
#include "store.h"

#define ANNOTATE_DIR "import/.annotate"   // the requests' folder, in the data root
#define ANNOTATE_REQUEST "request.ini"    // the request, in it
#define ANNOTATE_STALE_S 300              // a request older (or newer) than this is not opened
#define ANNOTATE_TITLE_LEN 512            // the most of a title a request is read with

// A request (request.ini), as annotate_parse reads it.
typedef struct {
    int version;                     // 1
    char book[STORE_SOURCE_LEN];     // the book's key: KOReader's partial MD5 of its file
    char title[ANNOTATE_TITLE_LEN];  // the book's title, for a new notebook's name (never logged)
    int page, pages;                 // the page the reader is on, from 1, and how many there are
    char kind[16];                   // "paged" (PDF, DjVu, comics) or "reflowable" (EPUB, FB2, TXT)
    char picture[64];                // the picture's file name, in the requests' folder
    int64_t time;                    // when it was written (unix seconds); 0 when not given
} AnnotateRequest;

// Reads a request's text. 0, or -1 and why (for the log: never the title).
int annotate_parse(const char *text, AnnotateRequest *r, const char **why);
// Whether a request is too old (or too new: the clock was set back) to open at `now`: its time, or
// file_time (the file's own) when it gives none, more than ANNOTATE_STALE_S away.
bool annotate_too_old(const AnnotateRequest *r, int64_t now, int64_t file_time);

// ---- a notebook's source.ini: the book it annotates, and which page holds which book page ----

typedef struct {
    int page, pages;   // the book page, of how many the book had then (a reflowable book's change
                       // with its layout: page 12 of 300 and page 12 of 340 are two pages)
    uint32_t pid;      // the notebook's page that holds it
} AnnotatePage;

typedef struct {
    char book[STORE_SOURCE_LEN];
    char kind[16];
    AnnotatePage *e;
    int n, cap;
    char extra[1024];  // lines this version does not know, written back as they were
} AnnotateMap;

// Reads source.ini's text into m (NULL: an empty map). -1 out of memory.
int annotate_map_parse(const char *text, AnnotateMap *m);
// The text to write: entries for pages no longer in nb (deleted) are left out. Caller frees.
char *annotate_map_text(const AnnotateMap *m, const NotebookInfo *nb, size_t *len);
void annotate_map_free(AnnotateMap *m);
// The page that holds book page `page` of `pages` (an id in nb's page list), or 0.
uint32_t annotate_map_find(const AnnotateMap *m, const NotebookInfo *nb, int page, int pages);
int annotate_map_add(AnnotateMap *m, int page, int pages, uint32_t pid);
// Where a new page for book page `page` of `pages` goes in nb's page list: just before the first
// page that holds a later book page (by its place in the book: page 12 of 300 before page 50 of
// 400), so the pages written after an annotated page (notes on it) stay with it; at the end when no
// page holds a later one.
int annotate_insert_at(const NotebookInfo *nb, const AnnotateMap *m, int page, int pages);

// A new notebook's name: the title on one line (control characters as spaces, broken UTF-8 left
// out, spaces trimmed), cut to the name limit between two characters, never inside one; empty
// stays empty.
void annotate_name(const char *title, char *out, size_t cap);

// Whether data is a PNG whose chunks are all there, in order, and whose critical ones (IHDR, PLTE,
// IDAT, IEND) check out against their CRCs, with nothing after IEND: a file cut short or damaged is
// caught before anything is made of it. 0 with its size, or -1 and why.
int annotate_png_check(const uint8_t *data, size_t len, int *w, int *h, const char **why);

// ---- at startup ----

typedef enum {
    ANNOTATE_NONE,    // no request
    ANNOTATE_STALE,   // a request this start was not asked to open, or too old: deleted unopened
    ANNOTATE_OPENED,  // the page was there already
    ANNOTATE_MADE,    // a page was added, in a notebook made for the book or in its own
    ANNOTATE_FAILED,  // nothing was made (the notebook stays as it was); msg says why
} AnnotateOutcome;

// Why nothing was made (FAILED), for the toast the caller shows.
typedef enum {
    ANNOTATE_WHY_LOG,      // see grimoire.log (the request does not read, a notebook's files do not)
    ANNOTATE_WHY_PICTURE,  // the picture is missing, cut short, damaged, or no picture Grimoire takes
    ANNOTATE_WHY_STORAGE,  // a file could not be written: the storage is probably full
} AnnotateWhy;

typedef struct {
    AnnotateOutcome outcome;
    char nb[STORE_ID_LEN];  // OPENED, MADE: the notebook to open, and its page that holds the
    uint32_t pid;           // book page (its last_page says so too)
    AnnotateWhy why;        // FAILED
    int page, pages;        // the book page asked for (OPENED, MADE, FAILED)
} AnnotateResult;

// The request in grimoire/import/.annotate/, if any, at startup, before anything opens a notebook.
// asked: KOReader's plugin started Grimoire to open it (GRIMOIRE_ANNOTATE=1, hosted). A request
// this start was not asked for (Grimoire started from the Library, or by Open Grimoire, after a
// hand-over that never reached it), or one too old, is deleted unopened, with one log line. Either
// way the folder is emptied after. The book's notebook has its compaction stopped first, as when a
// notebook opens. A page picture of the size of its notebook's pages (KOReader draws them at the
// Scribe's) becomes the page's picture as it is; one of another size (a notebook from a Scribe of the
// other size, a screenshot taken sideways) is fitted to the page in the background (BG_TEMPLATE),
// and the page draws white until then. Opens nothing itself: the caller opens the page given.
void annotate_take(bool asked, AnnotateResult *out);

// The headless runner's annotate.* keys (app_probe forwards the ones it does not answer itself):
// "result" (none, stale, opened, made, failed: what this start's request came to), "leftover"
// (files left in the requests' folder), "book_page" ("12/300": the book page held by page index
// open_page of the notebook this start's request opened; "" for a page that holds none, or no such
// notebook). 0, or -1 for a key it lacks.
int annotate_probe(const char *key, int open_page, char *out, size_t cap);
