//! templates.h: pictures the owner copies onto the Kindle over USB (docs/FORMAT.md, Image papers).
//!   grimoire/templates/*.png|jpg|jpeg|pgm   templates: a page's paper, in any notebook
//!   grimoire/import/<Name>/                 a folder of page pictures (a PDF exported as PNGs, a
//!                                           scan): made into a notebook named <Name> at startup
//! The slow work (decoding, fitting a picture to the page, writing it) runs in a forked child
//! through bgjob.h (BG_TEMPLATE, BG_IMPORT_FOLDER; src/bgwork_template.c), never on the UI thread.

use crate::prelude::*;


// ---- templates ----

/// a template's path, as a background job takes it (bgjob.c)
pub const TEMPLATE_PATH_MAX: usize = 600;

#[derive(Clone, Debug, Default)]
pub struct Template {
    /// the file
    pub path: FixedStr<TEMPLATE_PATH_MAX>,
    /// its name without the extension, as a sheet shows it
    pub name: FixedStr<STORE_NAME_LEN>,
    /// its preview is made: template_thumb reads it
    pub ready: bool,
    /// not a picture Grimoire can use (a sheet leaves it out)
    pub bad: bool,
    /// its preview (1/6 of the page), once template_thumb read it
    pub thumb: Canvas,
}



// The functions are bgwork_template.c's, documented at their definitions in bgwork_template.rs.
pub use crate::bgwork_template::{
    templates_init,
    template_scan,
    template_thumb,
    template_list_free,
    template_refresh,
    template_paper,
    template_install,
    template_start_notebook,
    photos_scan,
    photo_path,
    photo_install,
    photo_paper,
    import_scan,
    import_finished,
    natural_cmp,
};
