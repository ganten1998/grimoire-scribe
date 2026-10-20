// compact.h: compaction, which takes erased ink out of a notebook's page files
// (src/bgwork_compact.c; docs/FORMAT.md, Compaction). An erase stroke (white ink, the precise
// eraser) takes ink away from how a page looks, not from its file: the ink stays in the strokes
// before it, where a zip, a copy made with grimoire-pull or an older version still finds it. The
// notebook's erased.txt lists the pages the editor wrote an erase stroke to; when the notebook
// closes, a background job (BG_COMPACT_PAGES, bgjob.h) leaves out of those pages' files what
// changes nothing on the page.
#pragma once
#include <stdbool.h>
#include <stdint.h>

// Lists page `page_id` of notebook `id` in its erased.txt (if it is not there yet): the editor
// calls it when it keeps a page's first erase stroke of a session. 0, or -1 if the list could not
// be written (then the page is compacted only once it is listed again).
int compact_note_page(const char *id, uint32_t page_id);
// Whether notebook `id` has pages listed.
bool compact_listed(const char *id);
// At startup: queues the compaction of every notebook with pages listed (an earlier run left while
// the notebook was open, or a compaction was stopped before it was done). Returns how many.
int compact_queue_listed(float dpi);
