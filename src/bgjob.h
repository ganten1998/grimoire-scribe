// bgjob.h: PDF mirror exports in a background process. Rendering a notebook to PDF takes
// seconds on the Kindle; on the UI thread it froze typing and drawing (and a long one tripped
// the launcher's watchdog, which killed Grimoire). One child process at a time, niced, works
// through a small queue of notebook ids; each child loads the notebook's latest info itself.
#pragma once
#include <stdbool.h>

// Queues the mirror PDF of notebook `id` (a queued duplicate is merged). notify: report
// the result through bg_poll (for an export the user asked for).
void bg_mirror_pdf(const char *id, float dpi, const char *mirror_dir, bool notify);
// Reaps a finished child and starts the next job. Returns 1 / -1 when a notify job just
// succeeded / failed, else 0. Call it often (app_tick).
int bg_poll(void);
bool bg_busy(void);
// At exit: hands everything still queued to one detached child, which finishes on its own.
void bg_detach_rest(void);
