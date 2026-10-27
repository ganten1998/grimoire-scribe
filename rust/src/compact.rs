//! compact.h: compaction, which takes the ink erased entirely out of a notebook's page files
//! (src/bgwork_compact.c; docs/FORMAT.md, Compaction). An erase stroke (white ink, the precise
//! eraser) takes ink away from how a page looks, not from its file: the ink stays in the strokes
//! before it, where a zip, a copy made with grimoire-pull or an older version still finds it. The
//! notebook's erased.txt lists the pages the editor wrote an erase stroke to; when the notebook
//! closes, a background job (BG_COMPACT_PAGES, bgjob.h) leaves out of those pages' files what
//! changes nothing on the page: ink erased entirely, and erase strokes left with nothing to do. A
//! stroke erased only in part stays whole in the file, with the erase strokes over it.

// The functions are bgwork_compact.c's, documented at their definitions in bgwork_compact.rs.
pub use crate::bgwork_compact::{compact_listed, compact_note_page, compact_queue_listed, COMPACT_TEST_BEFORE_REPLACE};
