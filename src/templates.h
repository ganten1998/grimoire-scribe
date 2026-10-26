// templates.h: pictures the owner copies onto the Kindle over USB (docs/FORMAT.md, Image papers).
//   grimoire/templates/*.png|jpg|jpeg|pgm   templates: a page's paper, in any notebook
//   grimoire/import/<Name>/                 a folder of page pictures (a PDF exported as PNGs, a
//                                           scan): made into a notebook named <Name> at startup
// The slow work (decoding, fitting a picture to the page, writing it) runs in a forked child
// through bgjob.h (BG_TEMPLATE, BG_IMPORT_FOLDER; src/bgwork_template.c), never on the UI thread.
#pragma once
#include "bgimage.h"
#include "store.h"

// At startup: grimoire/templates/, grimoire/photos/ and grimoire/import/, each with a README.txt,
// when missing.
void templates_init(void);

// ---- templates ----

#define TEMPLATE_PATH_MAX 600  // a template's path, as a background job takes it (bgjob.c)

typedef struct {
    char path[TEMPLATE_PATH_MAX];  // the file
    char name[STORE_NAME_LEN];     // its name without the extension, as a sheet shows it
    bool ready;                    // its preview is made: template_thumb reads it
    bool bad;                      // not a picture Grimoire can use (a sheet leaves it out)
    Canvas thumb;                  // its preview (1/6 of the page), once template_thumb read it
} Template;

// The templates in grimoire/templates/, in the natural order of their names (page-2 before
// page-10), and which have their previews made (kept in grimoire/templates/.cache/); none is read
// here: a sheet reads those it shows with template_thumb, a page of tiles at a time. Previews still
// missing are made in the background (one BG_TEMPLATE job with no notebook): scan again when
// bg_poll_kind reports a BG_TEMPLATE done. A file whose path is longer than TEMPLATE_PATH_MAX is
// left out. Free *out with template_list_free. 0, or -1 without the folder.
int template_scan(Template **out, int *n, float dpi);
// Template t's preview (read once, then kept in t), or NULL while it is not made (or t is bad).
const Canvas *template_thumb(Template *t);
void template_list_free(Template *t, int n);
// What template_scan found of each of the n templates brought up to date (a preview made since,
// a file found bad), from the cache's files alone, queueing nothing: a sheet fills in its tiles
// with it while the previews job runs. How many of them changed.
int template_refresh(Template *t, int n);
// The paper template t already is in notebook nb, by the notebook's bg/index.ini (ix, from
// bgindex_load): 0 with *kind, or -1 when it was never put there, or its preview is not made yet
// (its key comes from the hash the preview keeps). Reads nothing big, installs nothing.
int template_paper(const Template *t, const NotebookInfo *nb, const BgIndex *ix, PaperKind *kind);

// Puts the template at path into notebook nb, to be a page's paper: *kind is the paper a page takes
// for it, known at once. Its picture is made for the notebook's page size in the background
// (BG_TEMPLATE), and a page on it draws white until then (bgimage.h: the editor redraws the page
// once that job is done, after bgimage_retry_missing). The id in *kind comes from a hash of the
// file's bytes and the page size, so installing the same file again gives the same paper, and makes
// its picture again if it went missing. 0, or -1 (*why, for the log: not a picture Grimoire can
// use, a file over 16 MB, not readable, or one over 4 MB whose preview is not made yet). The hash
// is the one the previews job keeps beside each preview, so nothing big is read on the UI thread:
// only a template of at most 4 MB with no preview yet is read here (whole, once).
int template_install(const NotebookInfo *nb, const char *path, PaperKind *kind, const char **why);
// New notebook on a template: notebook nb, which store_create just made, goes onto the template at
// path. The template is installed into it (above: the first page draws white until its picture is
// made), then the notebook's own paper and its first page go onto it (store_start_on). 0, or -1
// and *why (for the log) when that could not be done: then the notebook stays on the paper it was
// made on.
int template_start_notebook(NotebookInfo *nb, const char *path, const char **why);

// ---- photos ----
//
// grimoire/photos/*.png|jpg|jpeg|pgm: photos the owner copies onto the Kindle over USB, each to be a
// page's paper: the page's own picture (a document page, Image papers: a new page after it gets the
// notebook's paper), fitted to the page and turned upright by its EXIF orientation, previews too.
// The same scan, previews and sheet as templates.

// As template_scan, for grimoire/photos/.
int photos_scan(Template **out, int *n, float dpi);
// Whether path names a file in grimoire/photos/ (a background job tells a photo by it).
bool photo_path(const char *path);
// As template_install, for a photo: *kind a document page's paper on its picture, made upright.
int photo_install(const NotebookInfo *nb, const char *path, PaperKind *kind, const char **why);
// As template_paper, for a photo.
int photo_paper(const Template *t, const NotebookInfo *nb, const BgIndex *ix, PaperKind *kind);

// ---- imports ----

// At startup: every folder in grimoire/import/ (but done/ and dot-folders) holding page pictures
// is queued to become a notebook (BG_IMPORT_FOLDER), at most 8 a start. What an import that was cut
// off left in notebooks/ (.import-<id>/) is put in place when it was whole, else removed. A folder
// tried 3 times, or refused (a page that is no picture Grimoire takes), waits until its pictures
// change (import/.attempts, docs/FORMAT.md). Returns how many were queued.
int import_scan(float dpi);
// When bg_poll_kind reports a BG_IMPORT_FOLDER job done (ok: it succeeded): 1 with the new
// notebook's page count in *pages once an import made its notebook, -1 when one failed, 0 when
// nothing came of it (another run had imported that folder).
int import_finished(bool ok, int *pages);

// Natural order of names: runs of digits compare by their value, other characters without regard
// to case; equal so, the bytes decide.
int natural_cmp(const char *a, const char *b);
