// templates.h: pictures the owner copies onto the Kindle over USB (docs/FORMAT.md, Image papers).
//   grimoire/templates/*.png|jpg|jpeg|pgm   templates: a page's paper, in any notebook
//   grimoire/import/<Name>/                 a folder of page pictures (a PDF exported as PNGs, a
//                                           scan): made into a notebook named <Name> at startup
// The slow work (decoding, fitting a picture to the page, writing it) runs in a forked child
// through bgjob.h (BG_TEMPLATE, BG_IMPORT_FOLDER; src/bgwork_template.c), never on the UI thread.
#pragma once
#include "store.h"

// At startup: grimoire/templates/ and grimoire/import/, each with a README.txt, when missing.
void templates_init(void);

// ---- templates ----

typedef struct {
    char path[600];              // the file
    char name[STORE_NAME_LEN];   // its name without the extension, as a sheet shows it
    Canvas thumb;                // its preview (1/6 of the page): px NULL while it is being made
    bool bad;                    // not a picture Grimoire can use (a sheet leaves it out)
} Template;

// The templates in grimoire/templates/, in the natural order of their names (page-2 before
// page-10), with the previews made so far (kept in grimoire/templates/.cache/). Previews still
// missing are made in the background (one BG_TEMPLATE job with no notebook): scan again when
// bg_poll_kind reports a BG_TEMPLATE done. Free *out with template_list_free. 0, or -1 without
// the folder.
int template_scan(Template **out, int *n, float dpi);
void template_list_free(Template *t, int n);

// Puts the template at path into notebook nb, to be a page's paper: *kind is the paper a page takes
// for it, known at once. Its picture is made for the notebook's page size in the background
// (BG_TEMPLATE), and a page on it draws white until then (bgimage.h: the editor redraws the page
// once that job is done, after bgimage_retry_missing). The id in *kind comes from the file's bytes
// and the page size, so installing the same file again gives the same paper, and makes its picture
// again if it went missing. 0, or -1 (*why, for the log: not a picture Grimoire can use, or not
// readable). It reads the whole file to know its id: fast for a template, a moment for a big photo.
int template_install(const NotebookInfo *nb, const char *path, PaperKind *kind, const char **why);

// ---- imports ----

// At startup: every folder in grimoire/import/ (but done/ and dot-folders) holding page pictures
// is queued to become a notebook (BG_IMPORT_FOLDER), at most 8 a start. Leftovers of an import
// that was cut off (notebooks/.import-<id>/) are removed first. Returns how many were queued.
int import_scan(float dpi);
// When bg_poll_kind reports a BG_IMPORT_FOLDER job done (ok: it succeeded): 1 with the new
// notebook's page count in *pages once an import made its notebook, -1 when one failed, 0 when
// nothing came of it (another run had imported that folder).
int import_finished(bool ok, int *pages);

// Natural order of names: runs of digits compare by their value, other characters without regard
// to case; equal so, the bytes decide.
int natural_cmp(const char *a, const char *b);
