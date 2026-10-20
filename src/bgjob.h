// bgjob.h: work in a background process. Rendering a notebook to PDF takes seconds on the
// Kindle; on the UI thread it froze typing and drawing (and a long one tripped the launcher's
// watchdog, which killed Grimoire). One child process at a time, niced, works through a small
// queue; each child loads what it needs itself. Besides the PDF mirror: compacting a notebook's
// pages (erased ink taken out of the files), installing a template into a notebook, and importing a
// folder of page images.
#pragma once
#include <stdbool.h>

typedef enum {
    BG_MIRROR_PDF = 0,  // id: the notebook; writes grimoire/PDFs/<folder>/<name>.pdf
    BG_COMPACT_PAGES,   // id: the notebook; drops strokes that erasing made invisible (src/bgwork_compact.c)
    BG_TEMPLATE,        // id: the notebook, path: the source image; normalizes it into the notebook (src/bgwork_template.c)
    BG_IMPORT_FOLDER,   // path: a folder of page images; makes a notebook of them (src/bgwork_template.c)
} BgKind;

// Queues a job (a queued duplicate, same kind, id and path, is merged). notify: report the result
// through bg_poll (for something the user asked for and waits on).
void bg_queue(BgKind kind, const char *id, const char *path, float dpi, bool notify);
// Queues the mirror PDF of notebook `id` (bg_queue with BG_MIRROR_PDF).
void bg_mirror_pdf(const char *id, float dpi, const char *mirror_dir, bool notify);
// Before a notebook opens: its queued compaction is dropped and a running one stopped (its writes
// are safe to abandon), so the editor and a background rewrite never meet. Returns when it is gone.
void bg_cancel_notebook(const char *id);
// Reaps a finished child and starts the next job. Returns 1 / -1 when a notify job just
// succeeded / failed, else 0. Call it often (app_tick). The kind of the job that just finished
// (notify or not) is in *done_kind when done_kind is not NULL and something finished (else -1).
int bg_poll_kind(int *done_kind);
int bg_poll(void);
bool bg_busy(void);
// At exit: hands everything still queued to one detached child, which finishes on its own.
void bg_detach_rest(void);

// The work itself, one function per kind other than the PDF mirror (run in the child; 0 = done).
int bg_compact_pages(const char *id, float dpi);
int bg_template(const char *id, const char *src, float dpi);
int bg_import_folder(const char *path, float dpi);
