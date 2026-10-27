//! bgjob.h: work in a background process. Rendering a notebook to PDF takes seconds on the
//! Kindle; on the UI thread it froze typing and drawing (and a long one tripped the launcher's
//! watchdog, which killed Grimoire). One child process at a time, niced, works through a small
//! queue; each child loads what it needs itself. Besides the PDF mirror: compacting a notebook's
//! pages (erased ink taken out of the files), installing a template into a notebook, and importing a
//! folder of page images.

use crate::prelude::*;

#[repr(i32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BgKind {
    /// id: the notebook; writes grimoire/PDFs/<folder>/<name>.pdf
    #[default]
    MirrorPdf = 0,  // BG_MIRROR_PDF
    /// id: the notebook; drops strokes that erasing made invisible (src/bgwork_compact.c)
    CompactPages = 1,  // BG_COMPACT_PAGES
    /// id: the notebook, path: the source image; normalizes it into the notebook (src/bgwork_template.c)
    Template = 2,  // BG_TEMPLATE
    /// path: a folder of page images; makes a notebook of them (src/bgwork_template.c)
    ImportFolder = 3,  // BG_IMPORT_FOLDER
}

/// Queues a job (a queued duplicate, same kind, id and path, is merged). notify: report the result
/// through bg_poll (for something the user asked for and waits on). Jobs run one at a time, what the
/// user waits on first (a notify job, a template's picture), then mirror PDFs, imports, compaction;
/// a full queue lets its oldest compaction go first (its pages stay listed).
pub fn bg_queue(kind: BgKind, id: Option<&str>, path: Option<&str>, dpi: f32, notify: bool) {
    todo!()
}
/// Queues the mirror PDF of notebook `id` (bg_queue with BG_MIRROR_PDF).
pub fn bg_mirror_pdf(id: &str, dpi: f32, mirror_dir: &str, notify: bool) {
    todo!()
}
/// Before a notebook opens or goes to the trash: its queued compaction is dropped and a running one
/// stopped (its writes are safe to abandon), so the editor and a background rewrite never meet.
/// Returns when it is gone.
pub fn bg_cancel_notebook(id: &str) {
    todo!()
}
/// Reaps a finished child and starts the next job. Returns 1 / -1 when a notify job just
/// succeeded / failed, else 0. Call it often (app_tick). The kind of the job that just finished
/// (notify or not) is in *done_kind when done_kind is not NULL and something finished (else -1).
pub fn bg_poll_kind(done_kind: Option<&mut i32>) -> i32 {
    todo!()
}
pub fn bg_poll() -> i32 {
    todo!()
}
pub fn bg_busy() -> bool {
    todo!()
}
/// At exit: hands everything still queued to one detached child, which finishes on its own.
pub fn bg_detach_rest() {
    todo!()
}

/// The work itself, one function per kind other than the PDF mirror (run in the child; 0 = done).
/// (bg_compact_pages is in bgwork_compact.rs, bg_template and bg_import_folder in
/// bgwork_template.rs, as in the C.)
pub use crate::bgwork_compact::bg_compact_pages;
pub use crate::bgwork_template::{bg_import_folder, bg_template};
