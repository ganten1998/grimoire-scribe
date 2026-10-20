// compact.h: compaction, which takes the ink erased entirely out of a notebook's page files
// (src/bgwork_compact.c; docs/FORMAT.md, Compaction). An erase stroke (white ink, the precise
// eraser) takes ink away from how a page looks, not from its file: the ink stays in the strokes
// before it, where a zip, a copy made with grimoire-pull or an older version still finds it. The
// notebook's erased.txt lists the pages the editor wrote an erase stroke to; when the notebook
// closes, a background job (BG_COMPACT_PAGES, bgjob.h) leaves out of those pages' files what
// changes nothing on the page: ink erased entirely, and erase strokes left with nothing to do. A
// stroke erased only in part stays whole in the file, with the erase strokes over it.
#pragma once
#include <stdbool.h>
#include <stdint.h>

// Lists page `page_id` of notebook `id` in its erased.txt (if it is not there yet): the editor
// calls it when it saves a page with an erase stroke it kept this session. 0, or -1 if the list
// could not be written, or is there but could not be read whole (then it is left as it is, and
// the editor tries again at its next save).
int compact_note_page(const char *id, uint32_t page_id);
// Whether notebook `id` may have pages listed: true for a list that is there but does not read now
// (the background pass tries it again).
bool compact_listed(const char *id);
// At startup: queues the compaction of every notebook with pages listed (an earlier run left while
// the notebook was open, or a compaction was stopped before it was done). Returns how many.
int compact_queue_listed(float dpi);

// Tests only (NULL otherwise): called with the page file's path once the pass has written a page's
// new file, just before it checks that the page file still holds what it read and renames the new
// file over it, with the stops held (SIGTERM waits). A test writes the page there as another writer
// might, or asks the pass to stop.
extern void (*compact_test_before_replace)(const char *page_path);
