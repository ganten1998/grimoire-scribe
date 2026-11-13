//! Grimoire, a handwriting notebook for the Kindle Scribe: the Rust port of the C tree in ../src,
//! one module per C file, under the same names (rust/PORTING.md has the rules and the map). The
//! design record is the C's comments, carried over: docs/DESIGN.md and docs/FORMAT.md are the
//! architecture and the byte-exact formats.
//!
//! The C has one namespace, its headers included where needed; here `prelude` re-exports every
//! header's module, and each module starts with `use crate::prelude::*;`, so a call reads as the C
//! call it translates (`canvas_fill(c, r, 0)`, `qlog!("...")`). The platform modules
//! (display_fbink, input_evdev, light_kindle; sim_sdl; platform_none) are not in the prelude: their
//! functions reach the rest through display.rs, input.rs and light.rs, as display.h, input.h and
//! light.h are implemented by one platform file per build in the C.

// Three of clippy's lints would change what the port keeps: functions take the C's arguments,
// however many (the signatures are the contract between the modules, PORTING.md); loops over an
// index stay loops over the index the C writes; and a cast to a libc type's width stays where that
// width differs by target (time_t, off_t and c_long are 32 bits on the Kindle, 64 on the host,
// where clippy sees the cast do nothing).
#![allow(clippy::too_many_arguments, clippy::needless_range_loop, clippy::unnecessary_cast)]

#[cfg(all(feature = "kindle", feature = "sim"))]
compile_error!("the kindle and sim features are two builds of the app: enable one");

// ---- infrastructure (no C file of its own) ----
pub mod actions; // action ids, registry metadata and binding groups
pub mod assets; // build/gen/*.c: fonts, eraser sample, web pages, catalogs
pub mod cfmt; // printf, for log lines, translations and every byte-exact text
pub mod ffi_fbink; // third_party/FBInk via csrc/fbink_shim.c (the Kindle)
pub mod ffi_qr; // third_party/qrcodegen
pub mod ffi_stb; // third_party/stb_*.h

// ---- wave 1: foundations ----
pub mod bluetooth;
pub mod btapi;
pub mod calib_fit;
pub mod canvas;
pub mod fbcheck;
pub mod fbpack;
pub mod fingershape;
pub mod fingershape_synth;
pub mod font;
pub mod fontcheck; // your own fonts: the safe face, the check's worker and the writer's side of it (2.1.0-beta.2)
pub mod userfonts; // your own fonts: grimoire/fonts, its cache, the FontCheck job, the load and the crash guard
pub mod gesturemap; // which action each gesture does: the built-ins and gestures.ini
pub mod hidkeys;
pub mod i18n;
pub mod json_min;
pub mod koreader;
pub mod md5; // MD5 and KOReader's partial MD5, a book's key (2.1)
pub mod latency;
pub mod linkoutline;
pub mod pair;
pub mod pdfimport; // PDFs imported without KOReader: scan, slices, commit (2.1.0-beta.2)
pub mod pdfwork; // the PDF worker and its record protocol (2.1)
pub mod picwork; // pictures decoded in the sandboxed worker, and the writer's side of it (2.1.0-beta.2)
pub mod pensync;
pub mod penbutton;
pub mod power;
pub mod qr;
pub mod sandbox; // the safe loader: one worker for every untrusted parser (2.1)
pub mod shortcuts;
pub mod shortcut_profiles; // per-device shortcuts: root/shortcuts/<device>.ini (Rust only, 2.0.0-beta.2)
pub mod touchgest; // the touch recognizer: contact frames in, touch, pinch and gesture events out
pub mod update;
pub mod wifi;
pub mod util; // common.h (util.c)
pub use util as common;

// ---- wave 2: the model ----
pub mod annotate;
pub mod backlinks; // which pages of other notebooks link here: root/backlinks.ini, a cache (Rust only, 2.0.0-beta.2)
pub mod libindex; // every notebook's pages, names and links for the Graph of all notebooks: root/graph-index.ini (Rust only, 2.0.0-beta.2)
pub mod bgimage;
pub mod bgjob;
pub mod bgwork_compact; // compact.h + bg_compact_pages
pub mod bgwork_template; // templates.h + bg_template, bg_import_folder
pub mod brush;
pub mod compact; // compact.h's surface (bodies in bgwork_compact.rs)
pub mod export;
pub mod childcaps; // oom_score_adj and RLIMIT_AS for every background child (2.1, lane L9)
pub mod pdfcache; // the notebook PDF's page streams (2.1, lane L9)
pub mod history;
pub mod lasso; // lasso select, copy, cut, paste, move (the pure part)
pub mod lasso_live; // the lasso's live selection preview (the pure part)
pub mod clipboard; // root/clipboard.qpg
pub mod shape;
pub mod graph;
pub mod musicbuild;
pub mod nav;
pub mod doclinks; // doclinks.ini: the links a document brought (2.1, lane L4)
pub mod pdflinks; // document links from any source: validation, seeds, fills, the water level (2.1, lane L4)
pub mod notebookpicker;
pub mod page;
pub mod paper;
pub mod store;
pub mod templates; // templates.h's surface (bodies in bgwork_template.rs)
pub mod tplsheet;

// ---- wave 3: the UI and the app ----
pub mod about; // the About sheet, What's new (Rust only, 2.0.0)
pub mod app;
pub mod bat; // header only
pub mod browser; // the file picker: Library > New > From a file... (2.1.0-beta.2, lane L8)
pub mod calibrate;
pub mod dispmenu;
pub mod editor;
pub mod erasersample; // header only (the bytes are in assets.rs)
pub mod findsheet; // the Find sheet: go to a page by its number or name (Rust only, the Index's phase 1)
pub mod fsview; // one storage layer: the walk, zones, listing and stats of the Kindle's storage (2.1, read-only)
pub mod httpd;
pub mod inkmode; // header only
pub mod keyboard;
pub mod library;
pub mod linkbar; // the Links bar: the page's links as numbered rows (Rust only, the Index's phase 1)
pub mod linkcard; // the long-press card: what a held link (or bar row) can do (Rust only, the Index's phase 2)
pub mod popout; // one popout: a card anchored beside what was held, its place (2.1, DESIGN.md "One popout")
pub mod quicksize; // quick sizes: a held pen button's card of four widths (2.1, lane L1)
pub mod live;
pub mod live_keys;
pub mod pagesview;
pub mod panel; // header only
pub mod pensettings;
pub mod settingsmenu; // Settings' categories and rows as data (Rust only)
pub mod share;
pub mod status; // not a port: the clock and battery corner, Rust only (2.0.0-beta.5)
pub mod ui;

// ---- wave 4: the platform and the main loop ----
pub mod display; // display.h: the types; the functions are the build's platform's
// light_kindle.c is plain POSIX (lipc through the shell): it builds everywhere, as tests/test_lockui.c
// compiles it on the host; light.rs takes it as the platform on the Kindle only.
pub mod input; // input.h: the same
pub mod light; // light.h: the same
pub mod light_kindle;
pub mod mainloop; // main.c
pub mod mapview;

#[cfg(feature = "kindle")]
pub mod display_fbink;
#[cfg(feature = "kindle")]
pub mod input_evdev;
#[cfg(feature = "sim")]
pub mod sim_sdl; // the headless runner (the SDL window simulator is not ported: PORTING.md)
// (its stubs take the platform's arguments, as the platform modules declare them, and use none)
#[cfg(not(any(feature = "kindle", feature = "sim")))]
#[allow(unused_variables, clippy::ptr_arg)]
pub mod platform_none; // host builds: tools and cargo test link no platform

/// Every header's surface, as the C's one namespace: `use crate::prelude::*;` in each module.
pub mod prelude {
    pub use crate::actions::*;
    pub use crate::annotate::*;
    pub use crate::app::*;
    pub use crate::backlinks::*;
    pub use crate::libindex::*;
    pub use crate::bat::*;
    pub use crate::bgimage::*;
    pub use crate::bgjob::*;
    pub use crate::bluetooth::*;
    pub use crate::brush::*;
    pub use crate::btapi::*;
    pub use crate::calib_fit::*;
    pub use crate::canvas::*;
    pub use crate::cfmt::Arg;
    pub use crate::compact::*;
    pub use crate::display::*;
    pub use crate::dispmenu::*;
    pub use crate::erasersample::*;
    pub use crate::export::*;
    pub use crate::pdfcache::*;
    pub use crate::childcaps::*;
    pub use crate::fbcheck::*;
    pub use crate::fbpack::*;
    pub use crate::findsheet::*;
    pub use crate::fingershape::*;
    pub use crate::font::*;
    pub use crate::gesturemap::*;
    pub use crate::hidkeys::*;
    pub use crate::history::*;
    pub use crate::shape::*;
    pub use crate::graph::*;
    pub use crate::httpd::*;
    pub use crate::i18n::*;
    pub use crate::inkmode::*;
    pub use crate::input::*;
    pub use crate::json_min::*;
    pub use crate::keyboard::*;
    pub use crate::koreader::*;
    pub use crate::latency::*;
    pub use crate::lasso::*;
    pub use crate::lasso_live::*;
    pub use crate::clipboard::*;
    pub use crate::linkoutline::*;
    pub use crate::light::*;
    pub use crate::linkbar::*;
    pub use crate::linkcard::*;
    pub use crate::popout::*;
    pub use crate::quicksize::*;
    pub use crate::live::*;
    pub use crate::musicbuild::*;
    pub use crate::nav::*;
    pub use crate::doclinks::*;
    pub use crate::pdflinks::*;
    pub use crate::notebookpicker::*;
    pub use crate::mapview::*;
    pub use crate::page::*;
    pub use crate::pagesview::*;
    pub use crate::pair::*;
    pub use crate::panel::*;
    pub use crate::paper::*;
    pub use crate::pensync::*;
    pub use crate::penbutton::*;
    pub use crate::power::*;
    pub use crate::qr::*;
    pub use crate::shortcuts::*;
    pub use crate::shortcut_profiles::*;
    pub use crate::status::*;
    pub use crate::store::*;
    pub use crate::templates::*;
    pub use crate::touchgest::*;
    pub use crate::tplsheet::*;
    pub use crate::ui::*;
    pub use crate::update::*;
    pub use crate::wifi::*;
    pub use crate::util::*;
    pub use crate::{buf_printf, cformat, csnprintf, qlog};
}
