//! bgwork_compact.c: BG_COMPACT_PAGES, taking the ink erased entirely out of a notebook's page files
//! (compact.h).
//!
//! It runs in bgjob.c's background child: page_droppable can take tens of seconds on a heavily
//! erased page on the Kindle, never something for the UI thread. For each page listed in the
//! notebook's erased.txt it reads the page file, and when page_droppable finds strokes that change
//! nothing (ink that later erase strokes restore entirely, erase strokes left with nothing to
//! change) it writes the page without them, through a temporary file and a rename, and only if the
//! file still holds what it read (an older version, or a copy from a computer, may have written it
//! since). A stroke erased only in part stays whole, with the erase strokes over it. A page that is
//! done (compacted, or with nothing to leave out) goes off the list, and the list goes once it is
//! empty. A page it cannot judge stays listed: one that does not read, one of another size than its
//! notebook's, and one holding data this version reads past (a newer version's), which only the
//! version that wrote it can judge.
//!
//! Stopping it is safe at any point. bg_cancel_notebook (SIGTERM) runs before the notebook opens.
//! The pass first puts SIGTERM, SIGINT and SIGHUP back to their default action (a fork keeps
//! Grimoire's handlers, and on the Kindle input_evdev.c's only note the signal, so the pass would
//! run on and Grimoire would wait for all of it), and they wait while a file is being replaced, so
//! a stop never leaves a temporary file behind: each page is its old file or its new one. It stops
//! by itself when Grimoire goes (its parent), so a Grimoire started again can open the notebook at
//! once: the list keeps the pages for next time.
//!
//! Compaction never changes how a page looks (page_droppable checks the page with its paper and its
//! ink layer), so the notebook's PDF mirror comes out the same whether it runs before compaction or
//! after it. The two never run at once anyway (bgjob.c runs one child at a time, and keeps
//! compaction out of what it hands to the child that outlives Grimoire), and a page file is replaced
//! in one rename, so any reader (the mirror, the share server) sees the old file or the new one.

use crate::prelude::*;

/// Lists page `page_id` of notebook `id` in its erased.txt (if it is not there yet): the editor
/// calls it when it saves a page with an erase stroke it kept this session. 0, or -1 if the list
/// could not be written, or is there but could not be read whole (then it is left as it is, and
/// the editor tries again at its next save).
pub fn compact_note_page(id: &str, page_id: u32) -> i32 {
    todo!()
}
/// Whether notebook `id` may have pages listed: true for a list that is there but does not read now
/// (the background pass tries it again).
pub fn compact_listed(id: &str) -> bool {
    todo!()
}
/// At startup: queues the compaction of every notebook with pages listed (an earlier run left while
/// the notebook was open, or a compaction was stopped before it was done). Returns how many.
pub fn compact_queue_listed(dpi: f32) -> i32 {
    todo!()
}

/// Tests only (NULL otherwise): called with the page file's path once the pass has written a page's
/// new file, just before it checks that the page file still holds what it read and renames the new
/// file over it, with the stops held (SIGTERM waits). A test writes the page there as another writer
/// might, or asks the pass to stop.
pub static COMPACT_TEST_BEFORE_REPLACE: std::sync::Mutex<Option<fn(&str)>> = std::sync::Mutex::new(None);

/// bgjob.h: the work of a BG_COMPACT_PAGES job, in the child; 0 = done.
pub fn bg_compact_pages(id: &str, dpi: f32) -> i32 {
    todo!()
}
