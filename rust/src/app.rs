//! app.h: the App, screen switching, and the dirty-rect / refresh accumulator.

use crate::prelude::*;

#[repr(i32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ScreenId {
    #[default]
    Library = 0,  // SCR_LIBRARY
    Editor = 1,  // SCR_EDITOR
    Penset = 2,  // SCR_PENSET
    Calib = 3,  // SCR_CALIB
    Share = 4,  // SCR_SHARE
}
impl ScreenId {
    /// SCR_COUNT
    pub const COUNT: usize = 5;
}

/// Screen modules keep their own state; each allocates it in its *_init (library.rs, editor.rs,
/// pensettings.rs, calibrate.rs, share.rs define these).
pub use crate::calibrate::CalibState;
pub use crate::editor::EditorState;
pub use crate::library::LibraryState;
pub use crate::pensettings::PenSetState;
pub use crate::share::ShareState;

/// The one App of a process (mainloop.rs owns it, boxed). There is no global pointer to it: what the
/// C reaches through one (app_probe's g_probe_app, live_hover's g_live_app) is passed or mirrored
/// (PORTING.md, The App).
#[derive(Default)]
pub struct App {
    pub shape_log: bool,
    pub w: i32,
    pub h: i32,
    pub dpi: f32,
    /// mirrors the panel
    pub screen: Canvas,
    pub settings: Settings,
    /// the default shortcuts: shortcuts.ini and controller.ini, which every keyboard and controller uses
    /// but one with shortcuts of its own
    pub shortcuts: Shortcuts,
    /// the keyboards' and controllers' own shortcuts (shortcut_profiles.rs): a key or button uses its
    /// device's while that is on (app_key_map)
    pub shortcut_profiles: Vec<ShortcutProfile>,
    /// Live view holds have independent identity and a lease; physical releases cannot end them.
    pub live_holds: Vec<crate::live_keys::LiveKey>,
    /// what each touch gesture does: the built-ins and gestures.ini (app_gestures_load)
    pub gestures: GestureMap,
    pub pb_config: PbConfig,
    /// A bound pen drag defers non-Fast presents and toasts until the nib lifts.
    pub pen_action_down: bool,
    pub pen_toast_pending: Option<(FixedStr<256>, i32)>,
    pub last_shortcut_code: i32,
    pub last_shortcut_mods: i32,
    pub last_shortcut_gamepad: bool,
    /// Erase while held's key or button, held now (0 none), and whether it is a controller's: its
    /// release ends it
    pub hold_code: i32,
    pub hold_pad: bool,
    /// keyboards and controllers that came or went (IN_HID_*)
    pub hid_changes: u32,
    /// the Bluetooth helper's state changes and answers (bluetooth.h)
    pub bt_changes: u32,
    /// when to start the helper (after the first frame), 0 none
    pub bt_start_at: u32,
    /// their last toasts: came, went (0 never)
    pub hid_toast_at: [u32; 2],
    pub scr: ScreenId,
    pub prev_scr: ScreenId,
    pub running: bool,
    /// Live view's watermark: the live_seq of the newest captured event a screen has finished
    /// handling, and that event's in_sample_ms. app_flush hands them to live_presented once every
    /// pixel the event dirtied is in the stream, so a batch never retires ink it does not show yet.
    pub live_processed: u32,
    pub live_processed_ms: u32,
    /// Dirty accumulator, flushed once per loop iteration.
    pub dirty: Rect,
    pub dirty_mode: RefreshMode,
    /// The redraws the screens have asked for (app_invalidate), counted: a page turned, ink drawn or taken
    /// back, a sheet or menu opened or closed, the view moved, the panel changed. A refresh of what the panel
    /// shows already (the editor's settle after writing) leaves it as it was, and the app's own toasts are not
    /// redraws. With the settings, what tells whether an action did anything: a finger shape names its action
    /// only when one of the two changed (editor.rs fs_fire, 2.0.0-beta.2).
    pub changes: u32,
    /// promote to FULL every 30
    pub ui_refreshes_since_full: i32,
    /// what fast refreshes left on the panel that only a flash wipes (display.rs, issue #11): the
    /// editor adds to it and cleans it; every Full, and a Clean over it, wipes it here at the flush
    pub du_wear: DuWear,
    /// applied device rotation (quarter turns clockwise)
    pub rotation: i32,
    /// reported by the sensor, applied when it is safe; -1 none
    pub pending_rotation: i32,
    pub native_pin: bool,
    pub input_yielded: bool,
    pub koreader_wallpaper: Canvas,
    /// the Kindle is showing the sleep screen
    pub asleep: bool,
    /// for the sleep timeout
    pub last_input: u32,
    pub sleep_requested: bool,
    /// 0 none: a follow-up redraw after waking
    pub wake_redraw_at: u32,
    /// when the power service was last told the user is active
    pub last_activity_poke: u32,
    /// a press waiting for the power service to announce sleep
    pub power_pressed_at: u32,
    /// asleep + power pressed: wake by then if outOfScreenSaver never came
    pub wake_fallback_at: u32,
    /// last preventScreenSaver sent (-1 none)
    pub prevent_applied: i32,
    /// leave/enter called only to re-lay out the same screen (rotation,
    /// a redraw after sleep): screens keep what they run (the share server)
    pub relayout: bool,
    /// Toast (drawn by app over any screen, auto-hidden after toast_until).
    /// room for two lines, in any language
    pub toast: FixedStr<256>,
    pub toast_until: u32,
    pub toast_rect: Rect,
    pub toast_visible: bool,
    /// The refreshes that carried the toast (app_flush overlays it), counted: what tells a toast put up
    /// before long work that holds the main loop reached the panel before the work (editor.rs: "Adding N
    /// pages...").
    pub toast_presents: u32,
    /// pixels the toast covers, restored when it hides
    pub toast_under: Canvas,
    /// pixels Live view's chip covers, put back after each present
    pub chip_under: Canvas,
    /// where the chip was last put on the panel (empty for none): it moves with the editor's panel, and goes
    /// with a menu in front of the page
    pub chip_at: Rect,
    /// toasts stay above this y (the keyboard, a sheet's buttons); 0 none.
    /// The screen sets it as it draws; app_switch clears it.
    pub toast_floor: i32,
    /// The staying notice (app_notice): what each key asks to say, and what the panel shows of it.
    pub notice: Notice,
    pub lib: Option<Box<LibraryState>>,
    pub ed: Option<Box<EditorState>>,
    pub ps: Option<Box<PenSetState>>,
    pub cal: Option<Box<CalibState>>,
    pub share: Option<Box<ShareState>>,
    /// The power button press being followed (its release decides which fallback it needs).
    /// 0 none
    pub power_down_at: u32,
    /// it went down while Grimoire showed its sleep screen
    pub power_down_asleep: bool,
    /// the power service announced a sleep or a wake since then
    pub power_answered: bool,
    /// wake fallbacks in a row that found the Kindle still locked
    pub locked_wakes: i32,
    /// when leaving stayed once because the page could not be saved (0 never)
    pub leave_unsaved_at: u32,
    /// a whole book KOReader sent (annotate.h): while its pages are being
    /// added, pen and touch wait and a toast says how far it got
    pub book: BookStatus,
    /// when that toast last changed
    pub book_told: u32,
    /// the main loop's last batch ended because no event was left queued (not at its 64 events or
    /// 8 ms): the editor's prefetch and the Links bar's hold timer wait for this
    pub input_drained: bool,
    /// What a hardware key does to the open on-screen keyboard (Esc: its Cancel; the arrows, Home, End and Ctrl or
    /// Cmd with A, C, X and V: keyboard_key_input), taken by app_keyboard_pointer at the release app_type_text
    /// sends (set over its touch-down too, which the keyboard then takes as a contact on no key; None the rest of
    /// the time).
    pub kb_inject: Option<KbInput>,
}


// Screen module interface (each implemented in its own module, as in its own .c):
//   X_init(a) allocates state; X_free(a);
//   X_enter(a) on switch-in (draw everything into a.screen and invalidate);
//   X_leave(a) on switch-out; X_event(a, ev); X_tick(a, now).
// (The C's DECLARE_SCREEN declares these; the definitions are the screens'.)
pub use crate::calibrate::{calibrate_enter, calibrate_event, calibrate_free, calibrate_init, calibrate_leave, calibrate_tick,
};
pub use crate::editor::{editor_enter, editor_event, editor_free, editor_init, editor_leave, editor_tick,
};
pub use crate::library::{library_enter, library_event, library_free, library_init, library_leave, library_tick,
};
pub use crate::pensettings::{
    pensettings_enter, pensettings_event, pensettings_free, pensettings_init, pensettings_leave, pensettings_tick,
};
pub use crate::share::{share_enter, share_event, share_free, share_init, share_leave, share_tick};

// Cross-screen entry points: the screens' (documented at their definitions), and app.c's own.
pub use crate::calibrate::calibrate_open;
pub use crate::editor::{
    editor_busy, editor_control_at, editor_fit_changed, editor_hold_eraser, editor_modal_key, editor_live_chip, editor_live_hole, editor_live_panel, editor_live_style, editor_live_under_panel, editor_open_notebook,
    editor_open_notebook_at, editor_page_canvas, editor_page_in_front, editor_panel_point, editor_picture_made, editor_pin_sleep_page, editor_probe, editor_save_now,
    editor_shortcut, editor_shortcut_at, editor_suspend,
};
pub use crate::library::{library_bt, library_learn_key, library_probe};
pub use crate::pensettings::{pensettings_open, pensettings_probe, pensettings_shortcut};
pub use crate::share::{share_draw_asleep, share_live_card_up, share_open, share_open_live, share_sleep,
};

// app.c: screen switching, the dirty-rect accumulator, toasts.

use std::cell::{Cell, RefCell};
use std::sync::atomic::{AtomicBool, Ordering};

// The module's state (app.c's file-level statics), the main thread's alone (PORTING.md, Global state).
#[derive(Default)]
struct State {
    // The app's own repaints (toast show/hide) are kept apart from the screens' so a toast
    // never mistakes its own pixels for fresh screen content underneath it.
    self_dirty: Rect,
    // the sleep screen app_sleep last chose, for app_probe's "asleep.mode"
    sleep_shown: i32,
    // sleep_mode's note about the Kindle's own screensaver, said once
    told: bool,
    // as last told (a toast when the first comes)
    live_viewers: i32,
    /// the panel's rect as Live view's layer last has it (live_push_screen)
    live_panel: Rect,
    // sleep_minute_ms's minute, read from the environment once (the simulator's)
    #[cfg(feature = "sim")]
    minute_ms: i32,
}

thread_local! {
    static S: RefCell<State> = RefCell::new(State::default());
}

fn with<R>(f: impl FnOnce(&mut State) -> R) -> R {
    S.with(|s| f(&mut s.borrow_mut()))
}

// For live_hover: Live view is on, from this app (the C's g_live_app != NULL), and that and the app
// being awake together, which is all the hook reads (PORTING.md, The App: there is no pointer to the
// App for it). live_hover_sync keeps the second equal to the C's `g_live_app && !g_live_app->asleep`
// wherever either half changes.
static G_LIVE_APP: AtomicBool = AtomicBool::new(false);
static G_LIVE_AWAKE: AtomicBool = AtomicBool::new(false);

// For live_capture, which input_sample_hook calls from inside input reading (work_pump included, in
// the middle of app work), where the C reads its g_live_app: what it reads of the App, kept here
// (PORTING.md, The App). on, asleep and input_yielded follow the App wherever they change
// (live_hover_sync); editor and styles are what the C's live_capture would find: a->scr ==
// SCR_EDITOR && a->book.state != BOOK_RUNNING, and editor_live_style for an event of each eraser
// and side button state. They are made again after every event and every tick (live_cap_sync) and
// as Live view starts: a sample input_poll reads takes the App as the loop left it, one a pump reads
// in the middle of an event the App as that event began (PORTING.md, Decided differences).
#[derive(Clone, Copy, Default)]
struct LiveCap {
    on: bool,
    asleep: bool,
    input_yielded: bool,
    editor: bool,
    /// [eraser][Button + 2 * Button2]
    styles: [[LiveStyle; 4]; 2],
}

thread_local! {
    static LIVE_CAP: Cell<LiveCap> = Cell::new(LiveCap::default());
    // app.c's g_live_ui_pending: the sequence of a captured UI pen-down not yet handled, 0 none
    static LIVE_UI_PENDING: Cell<u32> = const { Cell::new(0) };
    // and g_live_capture_blocked
    static LIVE_CAPTURE_BLOCKED: Cell<bool> = const { Cell::new(false) };
}

fn live_hover_sync(a: &App) {
    G_LIVE_AWAKE.store(G_LIVE_APP.load(Ordering::Relaxed) && !a.asleep, Ordering::Relaxed,
    );
    LIVE_CAP.with(|c| {
        let mut cap = c.get();
        cap.on = G_LIVE_APP.load(Ordering::Relaxed);
        cap.asleep = a.asleep;
        cap.input_yielded = a.input_yielded;
        c.set(cap);
        live_cap_publish(&cap, a.live_processed);
    });
}

// What live_capture reads, for the pen viewers that read the raw digitizer themselves (live.rs,
// LiveCapture), with the newest pen event handled: published wherever LIVE_CAP changes.
fn live_cap_publish(cap: &LiveCap, handled: u32) {
    live_set_capture(&LiveCapture { ok: cap.on && !cap.asleep && !cap.input_yielded, editor: cap.editor, handled, styles: cap.styles ,
    });
}

// LIVE_CAPTURE_BLOCKED, told to the pen viewers too (live_set_blocked).
fn live_block(on: bool) {
    LIVE_CAPTURE_BLOCKED.with(|b| b.set(on));
    live_set_blocked(on);
}

fn live_cap_sync(a: &App) {
    let mut cap = LiveCap { on: G_LIVE_APP.load(Ordering::Relaxed), asleep: a.asleep, input_yielded: a.input_yielded, ..Default::default() };
    if cap.on && a.scr == ScreenId::Editor && a.book.state != BookState::Running {
        cap.editor = true;
        for (e, row) in cap.styles.iter_mut().enumerate() {
            for (side, style) in row.iter_mut().enumerate() {
                let ev = InEvent { eraser: e == 1, side_button: side & 1 != 0, side_button2: side & 2 != 0, ..Default::default() };
                editor_live_style(a, &ev, style);
            }
        }
    }
    LIVE_CAP.with(|c| c.set(cap));
    live_cap_publish(&cap, a.live_processed);
}

struct ScreenOps {
    init: fn(&mut App),
    free_: fn(&mut App),
    enter: fn(&mut App),
    leave: fn(&mut App),
    event: fn(&mut App, &InEvent),
    tick: fn(&mut App, u32),
}

static K_SCREENS: [ScreenOps; ScreenId::COUNT] = [
    // SCR_LIBRARY
    ScreenOps {
        init: library_init,
        free_: library_free,
        enter: library_enter,
        leave: library_leave,
        event: library_event,
        tick: library_tick,
    },
    // SCR_EDITOR
    ScreenOps { init: editor_init, free_: editor_free, enter: editor_enter, leave: editor_leave, event: editor_event, tick: editor_tick ,
    },
    // SCR_PENSET
    ScreenOps {
        init: pensettings_init,
        free_: pensettings_free,
        enter: pensettings_enter,
        leave: pensettings_leave,
        event: pensettings_event,
        tick: pensettings_tick,
    },
    // SCR_CALIB
    ScreenOps {
        init: calibrate_init,
        free_: calibrate_free,
        enter: calibrate_enter,
        leave: calibrate_leave,
        event: calibrate_event,
        tick: calibrate_tick,
    },
    // SCR_SHARE
    ScreenOps { init: share_init, free_: share_free, enter: share_enter, leave: share_leave, event: share_event, tick: share_tick ,
    },
];

fn ops(s: ScreenId) -> &'static ScreenOps {
    &K_SCREENS[s as usize]
}

/// getenv: the variable's value, None when it is not set (or not UTF-8: every one this module reads
/// is the launcher's ASCII).
fn getenv(name: &str) -> Option<String> {
    std::env::var(name).ok()
}

/// atoi, as musl's (the Kindle's) computes it: white space and a sign first, then the digits, in
/// int, wrapping.
fn atoi(s: &str) -> i32 {
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() && (b[i] == b' ' || (9..=13).contains(&b[i])) {
        i += 1;
    }
    let mut neg = false;
    if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
        neg = b[i] == b'-';
        i += 1;
    }
    let mut n: i32 = 0;
    while i < b.len() && b[i].is_ascii_digit() {
        n = n.wrapping_mul(10).wrapping_sub((b[i] - b'0') as i32);
        i += 1;
    }
    if neg {
        n
    } else {
        n.wrapping_neg()
    }
}

/// (uint32_t)strtoul(s, NULL, 10): white space and a sign first, then the digits; a value past the
/// range is ULONG_MAX (cast: 0xFFFFFFFF), and a minus negates it as an unsigned long.
fn strtoul_u32(s: &str) -> u32 {
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() && (b[i] == b' ' || (9..=13).contains(&b[i])) {
        i += 1;
    }
    let mut neg = false;
    if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
        neg = b[i] == b'-';
        i += 1;
    }
    let mut v: u64 = 0;
    let mut over = false;
    while i < b.len() && b[i].is_ascii_digit() {
        match v.checked_mul(10).and_then(|x| x.checked_add((b[i] - b'0') as u64)) {
            Some(x) => v = x,
            None => over = true,
        }
        i += 1;
    }
    if over {
        return u32::MAX;
    }
    (if neg { v.wrapping_neg() } else { v }) as u32
}

// The app that started this session and gets the screen back after it: "koreader" when
// KOReader's Grimoire plugin did (kindle/app/grimoire.sh --hosted koreader sets GRIMOIRE_HOST),
// "" for a start from the Kindle's Library.
fn app_host() -> &'static str {
    if getenv("GRIMOIRE_HOST").as_deref() == Some("koreader") {
        "koreader"
    } else {
        ""
    }
}

// The sleep screen to use. Hosted, KOReader keeps the Kindle UI paused for its whole session and
// the launcher never wakes it, so the Kindle's own screensaver (mode 2) cannot show: the page
// does instead.
fn sleep_mode(a: &App) -> i32 {
    if a.native_pin {
        return 2;
    }
    if a.settings.sleep_mode != 2 || app_host().is_empty() {
        return a.settings.sleep_mode;
    }
    if !with(|s| std::mem::replace(&mut s.told, true)) {
        qlog!("sleep: the Kindle's own screensaver needs the Kindle UI, which KOReader keeps paused: the page instead");
    }
    0
}

/// Where documents mirror PDFs go (/mnt/us/documents on Kindle, <root>/documents elsewhere).
/// Where the PDF mirror lives: grimoire/PDFs (outside documents/, so the Kindle never indexes
/// it: each indexed PDF made the home screen redraw its library, on top of Grimoire).
pub fn app_mirror_dir() -> String {
    let mut buf = String::new();
    csnprintf!(&mut buf, 600, "%s/PDFs", &store_root());
    buf
}

// A calibration maps to the pixels of the panel it was made on. One made on a panel of another size
// (the grimoire folder copied over from another Scribe) is scaled to this one, w x h upright, before
// it is used, and recorded as this panel's (pen.panel, touch.panel) from the next settings save on.
fn calib_follow_panel(c: &mut InputCalib, pw: &mut i32, ph: &mut i32, w: i32, h: i32, what: &str) {
    if !c.valid {
        return;
    }
    let recorded = *pw > 0 && *ph > 0;
    let (rw, rh) = if recorded { (*pw, *ph) } else { (1860, 2480) }; // the panel on record (or taken as it)
    let rec = if recorded { "" } else { " (not recorded: taken as an 1860x2480 Scribe's)" };
    let (mut fw, mut fh) = (*pw, *ph); // the panel it was made on, its own scale says (calib_to_panel)
    match calib_to_panel(c, &mut fw, &mut fh, w, h) {
        CalibMove::Scaled => {
            if fw == rw && fh == rh {
                qlog!("calibration: the %s calibration was made on a %dx%d panel%s; scaled to this %dx%d one", what, fw, fh, rec, w, h);
            } else {
                qlog!(
                    "calibration: the %s calibration was made on a %dx%d panel%s, but its scale fits a %dx%d one: scaled from that to this %dx%d one",
                    what, rw, rh, rec, fw, fh, w, h
                );
            }
        }
        CalibMove::MadeHere => {
            qlog!(
                "calibration: the %s calibration was made on a %dx%d panel%s, but its scale fits this %dx%d one: made here, kept as it is",
                what, rw, rh, rec, w, h
            );
        }
        _ => {}
    }
    *pw = w;
    *ph = h;
}

// A page KOReader's Grimoire plugin sent to annotate (annotate.h), at startup, before anything opens
// a notebook: the editor opens on its page. kindle/app/grimoire.sh asks for it (GRIMOIRE_ANNOTATE=1)
// only when KOReader hosts Grimoire; a request this start was not asked to open is deleted
// unopened. True when the page could not be opened, with why in msg, for app_init's startup toast.
fn annotate_at_start(a: &mut App, msg: &mut String, cap: usize) -> bool {
    let asked = getenv("GRIMOIRE_ANNOTATE");
    let mut r = AnnotateResult::default();
    annotate_take(asked.as_deref() == Some("1") && !app_host().is_empty(), &mut r,
    );
    if r.outcome == AnnotateOutcome::Opened || r.outcome == AnnotateOutcome::Made {
        let nb = r.nb;
        editor_open_notebook_at(a, nb.as_str(), r.pid, r.outcome == AnnotateOutcome::Made);
        return false;
    }
    if r.outcome != AnnotateOutcome::Failed {
        return false;
    }
    match r.why {
        AnnotateWhy::Picture => {
            // TRANSLATORS: notices when Grimoire opens from KOReader's "Annotate this page in Grimoire"
            // and that page cannot be opened, in a toast: one line of about 1700 px at 44 px in bold, or
            // two. This one: the picture of the page KOReader sent is damaged or missing.
            csnprintf!(msg, cap, "%s", tr("Couldn't open page: its image is unreadable."));
        }
        AnnotateWhy::Storage => {
            // TRANSLATORS: (as above) the page could not be added to its notebook: a file could not be
            // written.
            csnprintf!(msg, cap, "%s", tr("Couldn't add page from KOReader. Is the storage full?"));
        }
        _ => {
            // TRANSLATORS: (as above) any other reason; %s is the log file that says why.
            csnprintf!(msg, cap, tr("Couldn't open page from KOReader (see %s)."), "grimoire.log");
        }
    }
    true
}

// Why a whole book KOReader sent could not be opened, for the toast.
fn book_failed(a: &mut App) {
    let mut msg = String::new();
    if a.book.too_many {
        // TRANSLATORS: notices when Grimoire opens from KOReader's "Open this book in Grimoire", in a
        // toast: one line of about 1700 px at 44 px in bold, or two. This one: the book has more pages
        // than a notebook holds; %d is that number (2000).
        csnprintf!(&mut msg, 256, tr("Book too long: notebooks hold %d pages."), STORE_MAX_PAGES as i32);
    } else if a.book.why == AnnotateWhy::Picture {
        // TRANSLATORS: (as above) a page picture KOReader made is damaged or missing.
        csnprintf!(&mut msg, 256, "%s", tr("Couldn't open book: a page image is unreadable."));
    } else if a.book.why == AnnotateWhy::Storage {
        // TRANSLATORS: (as above) the pages could not be added: a file could not be written.
        csnprintf!(&mut msg, 256, "%s", tr("Couldn't add book pages. Is the storage full?"));
    } else {
        // TRANSLATORS: (as above) any other reason; %s is the log file that says why.
        csnprintf!(&mut msg, 256, tr("Couldn't open book from KOReader (see %s)."), "grimoire.log");
    }
    app_toast(a, &msg, 8000);
}

// A whole book KOReader sent (annotate.h), its pages added a slice at a time: how far it got, in a
// toast, then the editor opens on the reader's page.
fn book_tick(a: &mut App) {
    book_fits_tick();
    if a.book.state != BookState::Running {
        return;
    }
    book_step(150, &mut a.book);
    if a.book.state == BookState::Done {
        let nb = a.book.nb;
        // (links only: the notebook changed too, so that its mirror PDF, which draws the tabs, is made again)
        let added = a.book.made || a.book.added > 0 || a.book.links;
        app_notice_clear(a, NoticeKey::Book); // (the notice saying how far it got goes)
        editor_open_notebook_at(a, nb.as_str(), a.book.pid, added);
        if a.book.added != 0 {
            let mut msg = String::new();
            if a.book.links_out {
                // TRANSLATORS: in a toast, once the pages of a book KOReader sent are in its notebook, but some of
                // the book's links (a planner's tabs) are not: there were too many, or they could not be saved.
                csnprintf!(&mut msg, 256, ng_("Added %d book page. Some links were left out.", "Added %d book pages. Some links were left out.", a.book.added as i64), a.book.added);
            } else {
                // TRANSLATORS: in a toast, once the pages of a book KOReader sent are in its notebook.
                csnprintf!(&mut msg, 256, ng_("Added %d book page.", "Added %d book pages.", a.book.added as i64), a.book.added);
            }
            app_toast(a, &msg, 5000);
        } else if a.book.links_out {
            // TRANSLATORS: in a toast, once a book KOReader sent opened in its notebook with no new pages: some of
            // the book's links (a planner's tabs) are not in it, as there were too many, or they could not be saved.
            app_toast(a, tr("Some links were left out."), 5000);
        }
        if a.book.links && a.book.added == 0 && a.settings.mirror_pdf {
            // (the mirror PDF draws the tabs: a fill that added no page makes it again here; pages added do at close)
            let (dpi, dir) = (a.dpi, app_mirror_dir());
            bg_mirror_pdf(nb.as_str(), dpi, &dir, false);
        }
    } else if a.book.state == BookState::Failed {
        app_notice_clear(a, NoticeKey::Book); // (the notice saying how far it got goes)
        book_failed(a);
    } else {
        book_progress(a, now_ms());
    }
}

// How far the book got, in the staying notice (Book: on every screen), its text made at most every 600 ms.
// The notice redraws only when its text changes, at most once every 2 s, outside the count whose 30th Ui
// is a Full: a long book no longer flashes the panel about every 18 s (2.1, DESIGN.md "One notice").
fn book_progress(a: &mut App, now: u32) {
    if now.wrapping_sub(a.book_told) < 600 {
        return;
    }
    a.book_told = now;
    let mut msg = String::new();
    // TRANSLATORS: in a toast while Grimoire puts the pages of a book KOReader sent into its
    // notebook (a few seconds for a big book): pages done, of how many. One line of about 1700 px.
    csnprintf!(&mut msg, 256, tr("Adding book pages: %d of %d"), a.book.added, a.book.total);
    app_notice(a, NoticeKey::Book, &msg);
}

pub fn app_init(a: &mut App, w: i32, h: i32, dpi: f32) -> i32 {
    *a = App::default();
    a.w = w;
    a.h = h;
    a.dpi = dpi;
    a.running = true;
    let pin = getenv("GRIMOIRE_PASSCODE");
    a.native_pin = pin.as_deref() == Some("enabled");
    if canvas_init(&mut a.screen, w, h) != 0 {
        return -1;
    }
    store_set_device(dpi, w, h);
    // The lasso's clipboard lasts one session: an earlier one's goes before anything can paste it.
    clipboard_clear_at_start();
    settings_load(&mut a.settings);
    let mut shortcuts_path = String::new();
    csnprintf!(&mut shortcuts_path, 600, "%s/shortcuts.ini", &store_root());
    // A starter map nobody edited becomes this build's (grimoire.log says so; no toast); edited files stay.
    if shortcuts_upgrade_starter(&shortcuts_path) {
        qlog!("shortcuts: %s was an earlier starter map; it is this build's now", &shortcuts_path);
    }
    app_shortcuts_load(a);
    app_gestures_load(a);
    // The owner's touch trace (docs/KINDLE.md): an empty <root>/touch-trace asks for
    // <root>/touch-trace.txt, written while Grimoire has the panel (only the Kindle writes one).
    a.shape_log = std::path::Path::new(&cformat!("%s/shape-log", &store_root())).is_file();
    let switch = cformat!("%s/touch-trace", &store_root());
    if std::fs::metadata(&switch).is_ok() {
        input_touch_trace(Some(&cformat!("%s/touch-trace.txt", &store_root())));
    }
    input_shortcuts_enable(app_shortcuts_on(a));
    // The Bluetooth helper (bluetooth.h): the launcher says where its script is; in the simulator a
    // test may stand a helper in. It starts a moment after the first frame, if it is wanted.
    #[cfg(feature = "sim")]
    let standin = getenv("GRIMOIRE_BT_API");
    #[cfg(not(feature = "sim"))]
    let standin: Option<String> = None;
    bt_init(&store_root(), getenv("GRIMOIRE_BT_SCRIPT").as_deref(), standin.as_deref(),
    );
    wifi_init(getenv("GRIMOIRE_WIFI_SCRIPT").as_deref());
    // Settings > Updates (update.h): the launcher says where update.sh and the request file are.
    let mut request = String::new();
    let asked = getenv("GRIMOIRE_UPDATE_REQUEST");
    csnprintf!(&mut request, 700, "%s/.update-request", &store_root());
    let asked = asked.filter(|s| !s.is_empty());
    update_init(getenv("GRIMOIRE_UPDATE_SCRIPT").as_deref(), Some(asked.as_deref().unwrap_or(&request)),
    );
    let mut failed = String::new();
    csnprintf!(&mut failed, 700, "%s/.update-failed", &store_root());
    if std::fs::metadata(&failed).is_ok() {
        let _ = std::fs::remove_file(&failed);
        qlog!("update: the last update did not install (update.sh's lines above in this log say why)");
        // TRANSLATORS: a toast at the start after an update KPM could not install (Settings > Updates).
        app_toast(a, tr("Last update failed. See grimoire.log."), 6000,
        );
    }
    a.bt_start_at = if a.settings.controller && a.settings.shortcuts_enabled { now_ms().wrapping_add(1500) } else { 0 };
    // Shortcuts on means a scan for keyboards and controllers every 5 s, controller the Bluetooth
    // helper with it: both worth knowing when a log is read for the pen's sake.
    qlog!(
        "settings: sleep_mode %d, sleep_minutes %d, side_button %d, shortcuts %d, controller %d",
        a.settings.sleep_mode,
        a.settings.sleep_minutes,
        a.settings.side_button,
        a.settings.shortcuts_enabled,
        a.settings.controller
    );
    let st = &mut a.settings;
    calib_follow_panel(&mut st.pen_calib, &mut st.pen_panel_w, &mut st.pen_panel_h, w, h, "pen",
    );
    calib_follow_panel(&mut st.touch_calib, &mut st.touch_panel_w, &mut st.touch_panel_h, w, h, "touch",
    );
    if a.settings.pen_calib.valid || a.settings.touch_calib.valid {
        input_set_calib(
            if a.settings.pen_calib.valid { Some(&a.settings.pen_calib) } else { None },
            if a.settings.touch_calib.valid { Some(&a.settings.touch_calib) } else { None },
        );
    }
    ui_init(dpi);
    if font_init() != 0 {
        return -1;
    }
    ui_set_theme(if a.settings.theme == UiTheme::Classic as i32 { UiTheme::Classic } else { UiTheme::Gothic });
    font_set_family(a.settings.font.max(0) as usize);
    // A font of the owner's own (font=file:...), if it passed its check and the crash guard lets it in:
    // before the first screen is drawn, so that screen is the one the guard waits for.
    crate::userfonts::userfonts_apply(a);
    display_set_invert(a.settings.invert);
    for op in K_SCREENS.iter() {
        (op.init)(a);
    }
    a.scr = ScreenId::Library;
    a.prev_scr = ScreenId::Library;
    (ops(ScreenId::Library).enter)(a);
    app_invalidate_all(a, RefreshMode::Full);
    a.pending_rotation = -1;
    a.last_input = now_ms();
    a.prevent_applied = -1;
    app_apply_sleep_policy(a);
    // The owner's pictures (templates.h): the templates and import folders, and the folders of page
    // pictures copied into import/ since the last run, made into notebooks in the background.
    templates_init();
    crate::userfonts::userfonts_init();
    import_scan(dpi);
    let caps = input_caps();
    if a.settings.rotation_lock != 0 {
        app_set_rotation(a, a.settings.rotation_lock - 1);
    } else if caps.has_orientation && a.settings.follow_rotation && caps.orientation != 0 {
        app_set_rotation(a, caps.orientation);
    }
    // A page KOReader sent to annotate: the editor opens on it now, under the toast below. When it
    // could not be opened, why is that toast.
    let mut note = String::new();
    let annotate_failed = annotate_at_start(a, &mut note, 256);
    // A whole book KOReader sent: its pages are added over the next moments (book_tick), and the
    // editor then opens on the reader's page.
    let book = getenv("GRIMOIRE_BOOK");
    book_start(book.as_deref() == Some("1") && !app_host().is_empty(), &mut a.book,
    );
    // Beta.1 could leave the passcode off after a restart: the launcher says so (GRIMOIRE_PASSCODE), and
    // the log notes it, with no toast (the owner, 2026-11-04). Current standalone sessions leave it
    // enabled and yield to the native keypad on sleep.
    let passcode = getenv("GRIMOIRE_PASSCODE");
    let mut msg = String::new();
    if passcode.as_deref() == Some("left-off") {
        qlog!("device passcode: left off by an earlier session");
    }
    if annotate_failed {
        app_toast(a, &note, 8000);
    } else if a.book.state == BookState::Failed {
        book_failed(a);
    } else if !caps.has_pen {
        // TRANSLATORS: %s is the app's name, Grimoire.
        csnprintf!(&mut msg, 256, tr("No pen found. %s is made for Kindle Scribe; writing needs its pen."), APP_NAME);
        app_toast(a, &msg, 10000);
    }
    0
}

/// saves everything
pub fn app_shutdown(a: &mut App) {
    if a.scr == ScreenId::Editor {
        editor_save_now(a);
    }
    (ops(a.scr).leave)(a);
    settings_save(&a.settings);
    // The viewers stop before anything else is freed, and before bg_detach_rest: a detached export
    // must not hold a viewer's socket, and the headless runner waits for every child at exit.
    live_view_stop(a, false);
    for op in K_SCREENS.iter() {
        (op.free_)(a);
    }
    // Queued PDF exports finish on their own after Grimoire exits. After editor_free: closing the
    // open notebook (leaving straight from the editor) queues its mirror PDF.
    bg_detach_rest();
    canvas_free(&mut a.koreader_wallpaper);
    canvas_free(&mut a.toast_under);
    canvas_free(&mut a.chip_under);
    canvas_free(&mut a.screen);
    font_shutdown();
}

/// Unified pointer phase for UI screens: pen and finger both act as a pointer.
/// Returns -1 if the event is not a pointer event, else 0 down / 1 move / 2 up.
pub fn app_pointer_phase(ev: &InEvent) -> i32 {
    match ev.type_ {
        InType::PenDown | InType::TouchDown => 0,
        InType::PenMove | InType::TouchMove => 1,
        InType::PenUp | InType::TouchUp => 2,
        _ => -1,
    }
}

// Someone is here: "Sleep after" starts counting again, and the power service is told so, at most
// once a minute (Grimoire grabs the pen and touch, so the power service does not see them itself).
fn note_input(a: &mut App) {
    a.last_input = now_ms(); // (event times can run slightly ahead of now_ms)
    a.sleep_requested = false;
    if a.last_input.wrapping_sub(a.last_activity_poke) > 60000 {
        a.last_activity_poke = a.last_input;
        sys_activity();
    }
}

// A keyboard or controller came or went (shortcuts on): the Shortcuts sheet's devices and the Pair sheet's
// status line follow, and a short toast says so, at most once a direction in 30 s, so a link that keeps
// dropping does not flash the screen. None over the sleep screen or a stroke being drawn.
fn hid_changed(a: &mut App, ev: &InEvent) {
    let attach = ev.type_ == InType::HidAttach;
    a.hid_changes = a.hid_changes.wrapping_add(1);
    if a.asleep || editor_busy(a) || !hidkeys_due(&mut a.hid_toast_at[if attach { 0 } else { 1 }], now_ms(), 30000,
        ) {
        return;
    }
    // TRANSLATORS: toasts (one line of about 1700 px at 44 px in bold) when a controller, keyboard
    // or page turner comes or goes. "In use by another app": another program (a button mapper)
    // takes its presses, so Grimoire gets none.
    let msg = if !attach { tr("Controller disconnected") } else if ev.value != 0 { tr("Controller in use by another app") } else { tr("Controller connected") };
    app_toast(a, msg, 1500);
}

pub fn app_event(a: &mut App, ev: &InEvent) {
    if a.pen_action_down && matches!(ev.type_, InType::PenUp | InType::PenLeave) {
        app_pen_feedback_end(a);
    }
    app_event_screens(a, ev);
    live_block(false); // (the work before the screens' ends at any return)
    live_cap_sync(a);
}

/// The Graph is in front (the Library's, or a notebook's): its keys are its own.
pub fn app_graph_up(a: &App) -> bool {
    match a.scr {
        ScreenId::Library => crate::library::library_graph_up(a),
        ScreenId::Editor => crate::editor::editor_graph_up(a),
        _ => false,
    }
}

/// Whether keys and a controller's buttons are shortcuts now: Settings > Shortcuts' Enable shortcuts.
pub fn app_shortcuts_on(a: &App) -> bool {
    a.settings.shortcuts_enabled
}

/// The shortcuts a key or button of ev's device uses: the device's own (its profile, while on), else the
/// default (shortcuts.ini and controller.ini). gestures.ini, the pen's button and finger shapes are not per
/// device.
pub fn app_key_map<'a>(a: &'a App, ev: &InEvent) -> &'a Shortcuts {
    shortcut_profile_for(&a.shortcut_profiles, ev.hid_name.as_str(), ev.hid_vendor, ev.hid_product).unwrap_or(&a.shortcuts)
}

/// The default shortcuts (shortcuts.ini's keys, and its buttons while there is no controller.ini) and every
/// device's own again: at the start, Reload shortcuts.ini, Enable shortcuts.
pub fn app_shortcuts_load(a: &mut App) {
    let sp = cformat!("%s/shortcuts.ini", &store_root());
    let cp = cformat!("%s/controller.ini", &store_root());
    shortcuts_load_all(&mut a.shortcuts, &sp, &cp);
    a.shortcut_profiles = shortcut_profiles_load(&shortcut_profiles_dir());
}

/// The open on-screen keyboard's area, None when no keyboard is open.
pub fn app_keyboard_area(a: &App) -> Option<Rect> {
    match a.scr {
        ScreenId::Library => crate::library::library_keyboard_area(a),
        ScreenId::Editor => crate::editor::editor_keyboard_area(a),
        _ => None,
    }
}

/// A hardware key on the open on-screen keyboard (Esc: its Cancel): a contact on its text line, where
/// no key is, reaches the keyboard's owner as a tap on it would, and at the lift app_keyboard_pointer
/// applies input instead of a key (keyboard_input). So every keyboard (a name, a page number, Find, New
/// page...) takes it with its own Done and Cancel, and draws and refreshes exactly as for a key. False
/// when no keyboard is open, or its owner did not take it.
pub fn app_type_text(a: &mut App, input: KbInput) -> bool {
    let Some(area) = app_keyboard_area(a) else { return false };
    let field = keyboard_field_rect(area);
    let (x, y) = ((field.x + field.w / 2) as f32, (field.y + field.h / 2) as f32);
    let t = now_ms();
    let mut ev = InEvent { type_: InType::TouchDown, x, y, t_ms: t, k_ms: t, ..Default::default() };
    // (the input rides on the touch-down too, so a contact on the text, which puts the caret there, does not: the
    // keyboard takes the touch-down as a contact on no key, app_keyboard_pointer_split)
    a.kb_inject = Some(input);
    (ops(a.scr).event)(a, &ev);
    ev.type_ = InType::TouchUp;
    a.kb_inject = Some(input);
    (ops(a.scr).event)(a, &ev);
    if a.kb_inject.take().is_some() {
        qlog!("keys: a key found no keyboard to take it");
        return false;
    }
    true
}

/// A hardware keyboard's key on the open on-screen keyboard, when it is one that edits text (keyboard_key_input):
/// applied through app_type_text, so it draws and refreshes as the keyboard's own keys do. False when it is not
/// such a key, or no keyboard took it.
fn app_text_key(a: &mut App, ev: &InEvent) -> bool {
    match keyboard_key_input(ev.value, ev.key_mods) {
        Some(input) => app_keyboard_area(a).is_some() && app_type_text(a, input),
        None => false,
    }
}

/// Shared keyboard/controller/Live view dispatch. Callers own enablement, wake gating and
/// press/release identity; the editor owns modal/stroke guards. Status actions work on every screen.
fn app_shortcut_action(a: &mut App, action: i32, src: ActionSrc) -> bool {
    if action == SHORTCUT_STATUS {
        status_toggle(a);
        return true;
    }
    if action == SHORTCUT_STATUS_MODE {
        status_cycle_mode(a);
        return true;
    }
    if action == SHORTCUT_STATUS_CORNER {
        status_cycle_corner(a);
        return true;
    }
    if a.scr == ScreenId::Penset {
        pensettings_shortcut(a, action);
        return false;
    }
    if a.scr == ScreenId::Share {
        // (only Esc acts there: back to where Share was opened from)
        return action == SHORTCUT_BACK && crate::share::share_back(a);
    }
    if a.scr == ScreenId::Library {
        // (the Graph as a place of its own: index-map opens it from the Library, go-back closes it)
        if crate::library::library_shortcut(a, action) { return true; }
        return app_global_action(a, action, src);
    }
    a.scr == ScreenId::Editor && editor_shortcut_at(a, action, a.w as f32 * 0.5, a.h as f32 * 0.5, src)
}

// A handful of authenticated keys per tick. The server never calls the editor from inside a
// request, and stale/revoked sessions cannot replay work after a slow frame or a Live view restart.
fn live_keys_tick(a: &mut App) {
    let mut had_holds = !a.live_holds.is_empty();
    live_block(true);
    for _ in 0..4 {
        let Some(key) = httpd_live_key_pop() else { break ;
        };
        if !key.down {
            a.live_holds.retain(|h| h.code != key.code || h.cookie != key.cookie);
        } else if !a.asleep && live_on() {
            let action = httpd_live_key_action(&key, app_place(a));
            note_input(a);
            if app_shortcut_action(a, action, ActionSrc::Key) && action == SHORTCUT_HOLD_ERASER {
                had_holds = true;
                a.live_holds.push(key);
            }
        }
    }
    a.live_holds.retain(|key| !a.asleep && httpd_live_key_held(key));
    if had_holds || !a.live_holds.is_empty() {
        editor_hold_eraser(a, a.hold_code != 0 || !a.live_holds.is_empty());
    }
    live_block(false);
}

fn app_event_screens(a: &mut App, ev: &InEvent) {
    // Keys, the power button, sleep and wake do work that pumps the pens too (a shortcut turns the
    // page): no quick ink from what it reads before that work is done, as for a screen's own UI
    // work below.
    live_block(true);
    if ev.type_ == InType::Quit {
        a.running = false;
        return;
    }
    if ev.type_ == InType::Key {
        // A pad that types (the Pages view's count pad) takes a keyboard's keys first, shortcuts on or off.
        if !a.asleep && !ev.gamepad && editor_modal_key(a, ev) {
            note_input(a);
            return;
        }
        // A Library dialog with keys of its own (Exit Grimoire?, New notebook's Enter) takes them first.
        if !a.asleep && !ev.gamepad && crate::library::library_modal_key(a, ev.value) {
            note_input(a);
            return;
        }
        // A keyboard that types a name, a page or Find takes the keys that edit text (the arrows, Home and End, and
        // Ctrl or Cmd with A, C, X and V), shortcuts on or off: none of them reaches what is under it.
        if !a.asleep && !ev.gamepad && app_text_key(a, ev) {
            note_input(a);
            return;
        }
        if !a.asleep && app_shortcuts_on(a) {
            // A key or a controller button counts as pen and touch do: paging with them alone
            // must not let "Sleep after" run out.
            note_input(a);
            a.last_shortcut_code = ev.value;
            a.last_shortcut_mods = ev.key_mods as i32;
            a.last_shortcut_gamepad = ev.gamepad;
            if (a.scr == ScreenId::Library || crate::library::library_settings_active(a)) && library_learn_key(a, ev) {
                return;
            }
            // (one key may do one thing in each place: the Library, a page, the Pages view, the Graph, Pen feel;
            // and each device may have shortcuts of its own, app_key_map)
            let map = app_key_map(a, ev);
            let mut action = shortcuts_lookup_at(map, ActionSrc::from(ev.gamepad), ev.value, ev.key_mods as i32, app_place(a));
            if app_graph_up(a) {
                // The Graph's keys: an older edited file with none of its actions has the arrows and V
                // there too (shortcuts_lookup_graph); Enter opens the selected page.
                action = shortcuts_lookup_graph(map, ActionSrc::from(ev.gamepad), ev.value, ev.key_mods as i32);
                if action == SHORTCUT_NONE && !ev.gamepad && matches!(ev.value, 28 | 96) && ev.key_mods == 0 {
                    action = GRAPH_KEY_ENTER;
                }
            }
            if app_shortcut_action(a, action, if ev.gamepad { ActionSrc::Pad } else { ActionSrc::Key },
            )
                && action == SHORTCUT_HOLD_ERASER
            {
                a.hold_code = ev.value;
                a.hold_pad = ev.gamepad;
            }
        }
        return;
    }
    if ev.type_ == InType::KeyUp {
        // Erase while held ends at its button's release, on any screen, asleep or not (no other
        // action waits for a release). A release never counts as input.
        if a.hold_code != 0 && ev.value == a.hold_code && ev.gamepad == a.hold_pad {
            a.hold_code = 0;
            editor_hold_eraser(a, !a.live_holds.is_empty());
        }
        return;
    }
    if ev.type_ == InType::HidAttach || ev.type_ == InType::HidDetach {
        if ev.type_ == InType::HidDetach && a.hold_code != 0 {
            // (whichever went, its release cannot come)
            a.hold_code = 0;
            editor_hold_eraser(a, !a.live_holds.is_empty());
        }
        hid_changed(a, ev);
        return;
    }
    if ev.type_ == InType::Orientation {
        if a.settings.follow_rotation && a.settings.rotation_lock == 0 {
            a.pending_rotation = ev.value & 3;
        }
        return;
    }
    if ev.type_ == InType::Power {
        // The power service handles the button itself (it announces the sleep, which draws the
        // sleep screen); just make sure nothing is lost. What the press needs from Grimoire is
        // decided at its release, or at the 4 s hold, which leaves Grimoire.
        qlog!("power button: saving");
        if a.scr == ScreenId::Editor {
            editor_save_now(a);
        }
        // It replaces the press before it: a fallback that one left waiting must not go off while
        // this key is down (a sleep request in the middle of a hold: press, see nothing, then hold).
        // Its own release arms whatever it needs.
        a.power_pressed_at = 0;
        a.wake_fallback_at = 0;
        a.power_down_at = now_ms() | 1;
        a.power_down_asleep = a.asleep;
        a.power_answered = false;
        return;
    }
    if ev.type_ == InType::PowerUp {
        if ev.value >= 0 {
            qlog!("power button: up after %d ms", ev.value);
        } else {
            qlog!("power button: up (its release was missed, so how long it was held is not known)");
        }
        let followed = a.power_down_at != 0;
        a.power_down_at = 0;
        // A press the power service already answered needs nothing more: a wake announced while
        // the key was still down must not turn the release into a sleep request (that made the
        // Kindle fall asleep again right after waking).
        match power_up_action(followed, a.power_answered, a.power_down_asleep, a.asleep, ev.value,
        ) {
            PowerUpAction::SleepWait => a.power_pressed_at = now_ms() | 1, // expect goingToScreenSaver soon
            PowerUpAction::WakeWait => a.wake_fallback_at = now_ms().wrapping_add(3000) | 1, // and outOfScreenSaver
            PowerUpAction::Leave => {
                qlog!(
                    "power button held %d ms, but Grimoire was busy at the %d s mark: saving and leaving Grimoire",
                    ev.value,
                    INPUT_POWER_HOLD_MS / 1000
                );
                app_leave(a);
            }
            _ => {}
        }
        return;
    }
    if ev.type_ == InType::PowerHeld {
        // The way out that always works: a locked Kindle, a panel that shows nothing, any screen.
        qlog!("power button held %d s: saving and leaving Grimoire", INPUT_POWER_HOLD_MS / 1000);
        app_leave(a);
        return;
    }
    if ev.type_ == InType::Sleep {
        a.power_pressed_at = 0;
        a.power_answered = true;
        app_sleep(a);
        return;
    }
    if ev.type_ == InType::Wake {
        a.power_pressed_at = 0;
        a.power_answered = true;
        if !a.asleep {
            // Out of a screensaver Grimoire never saw start: still take the screen back.
            qlog!("wake: out of the screensaver while Grimoire thought it was awake; repainting");
            a.asleep = true;
            live_hover_sync(a);
            live_set_state(LIVE_ASLEEP);
            if a.scr == ScreenId::Editor && !editor_suspend(a) {
                qlog!("wake: could not save the interrupted page; keeping it in memory");
            }
        }
        app_wake(a);
        return;
    }
    // While the Kindle shows its sleep screen, pen and touch are not Grimoire's: drawing over the
    // screensaver because the panel reported a touch is how a wrong screen ended up on it. The side
    // button and pinches start nothing either (three clicks cleared the page); only their ends get
    // through, so one that began before the sleep still finishes.
    if a.asleep && power_asleep_drops(ev) {
        return;
    }
    // Whether the pen is near, for the clock and battery corner to hold its refreshes back.
    if !a.asleep {
        status_pen(ev);
    }
    live_block(false);
    let mut captured = *ev;
    let t = ev.type_ as i32;
    if captured.live_seq == 0 && t >= InType::PenDown as i32 && t <= InType::PenLeave as i32 {
        live_capture(&mut captured);
    }
    let ev = &captured;
    // While a book's pages are being added (a few seconds), nothing may open its notebook.
    if a.book.state == BookState::Running
        && (app_pointer_phase(ev) >= 0 || ev.type_ == InType::PinchBegin || ev.type_ == InType::Key || ev.type_ == InType::Gesture)
    {
        return;
    }
    if !a.asleep
        && (app_pointer_phase(ev) >= 0
            || ev.type_ == InType::PinchBegin
            || ev.type_ == InType::PenButton
            || ev.type_ == InType::Gesture)
    {
        note_input(a);
    }
    // A finger tap in the clock and battery's corner of the notebook page, on no control, turns it on or
    // off; the screen sees the touch as it always does.
    if !a.asleep {
        status_touch(a, ev);
    }
    // A pump inside UI work must not preview the old page or tool before that work finishes.
    let mut before = LiveStyle::default();
    if live_on() && a.scr == ScreenId::Editor {
        editor_live_style(a, ev, &mut before);
    }
    live_block(!(ev.type_ == InType::PenMove
        || (ev.type_ == InType::PenDown && before.enabled != 0 && !rect_contains(before.exclude, ev.x as i32, ev.y as i32))),
    );
    (ops(a.scr).event)(a, ev);
    live_block(false);
    if ev.live_seq != 0 {
        a.live_processed = ev.live_seq;
        a.live_processed_ms = in_sample_ms(ev);
        LIVE_UI_PENDING.with(|p| {
            if p.get() != 0 && (ev.live_seq.wrapping_sub(p.get()) as i32) >= 0 {
                p.set(0);
            }
        });
    }
    live_wake_samples();
}

/// The gestures again: the built-ins and `<root>/gestures.ini` over them (gesturemap.rs). Gestures
/// need no Enable shortcuts: settings.gestures turns them off.
pub fn app_gestures_load(a: &mut App) {
    let path = cformat!("%s/gestures.ini", &store_root());
    gesturemap_pen_button2(&mut a.gestures, input_caps().pen_button2);
    let n = gesturemap_load(&mut a.gestures, &path);
    app_gestures_sync(a);
    if n > 0 {
        qlog!("gestures: %d line(s) of gestures.ini not used", n);
    } else if n < 0 {
        qlog!("gestures: gestures.ini could not be read; the built-in gestures apply");
    }
}

/// Refresh the pen timing policy after a map copy is saved and published.
pub fn app_gestures_sync(a: &mut App) {
    a.pb_config = gesturemap_pen_config(&a.gestures);
}

/// A bound drag ended. Its feedback is safe to show once the nib is up.
pub fn app_pen_feedback_end(a: &mut App) {
    a.pen_action_down = false;
    if let Some((msg, ms)) = a.pen_toast_pending.take() {
        app_toast(a, msg.as_str(), ms);
    }
}

// The toast is never stored in a.screen: it is overlaid just for the refresh and the pixels
// underneath are put straight back, so no screen redraw can ever capture it as "background".
fn toast_hide(a: &mut App) {
    if !a.toast_visible {
        return;
    }
    a.toast_visible = false;
    let r = a.toast_rect;
    with(|s| s.self_dirty = rect_union(s.self_dirty, r));
}

/// Takes the toast down now (a hint whose sheet closed before it timed out).
pub fn app_toast_dismiss(a: &mut App) {
    toast_hide(a);
}

// Where the toast goes: its usual place near the bottom, or, when the screen keeps the bottom
// for its controls (the keyboard, a sheet's buttons), just above them, so their labels stay
// readable while it shows.
fn toast_place(a: &App) -> Rect {
    toast_place_for(a, a.toast.as_str())
}

// The same for any text (the staying notice's goes in the toast's place).
fn toast_place_for(a: &App, msg: &str) -> Rect {
    let mut r = ui_toast_rect(Rect { x: 0, y: 0, w: a.w, h: a.h ,
        }, msg,
    );
    let gap = ui_px(20.0);
    if a.toast_floor > 0 && r.y + r.h + gap > a.toast_floor {
        r.y = a.toast_floor - r.h - gap;
    }
    if r.y < gap {
        r.y = gap;
    }
    r
}

fn toast_draw(a: &mut App) {
    a.toast_rect = toast_place(a);
    a.toast_visible = true;
    // A toast takes the staying notice's place for its time: where the notice was is repainted from the
    // screen in the toast's own refresh, and it comes back once the toast goes (notice_ride).
    let r = rect_union(a.toast_rect, std::mem::take(&mut a.notice.rect));
    a.notice.shown = None;
    with(|s| s.self_dirty = rect_union(s.self_dirty, r));
}

// A toast already up when the screen's floor changed (a sheet opened under it) moves: where it
// was is repainted from the screen, and it is shown in its new place.
fn toast_follow(a: &mut App) {
    if !a.toast_visible {
        return;
    }
    let r = toast_place(a);
    if r.x == a.toast_rect.x && r.y == a.toast_rect.y && r.w == a.toast_rect.w && r.h == a.toast_rect.h {
        return;
    }
    let old = a.toast_rect;
    with(|s| s.self_dirty = rect_union(s.self_dirty, rect_union(old, r)));
    a.toast_rect = r;
}

pub fn app_toast(a: &mut App, msg: &str, ms: i32) {
    if a.pen_action_down {
        let mut text = FixedStr::default();
        text.set_cut(msg);
        a.pen_toast_pending = Some((text, ms));
        return;
    }
    toast_hide(a);
    a.toast.set_cut(msg); // (too long: cut between two characters)
    a.toast_until = now_ms().wrapping_add(ms as u32);
    toast_draw(a);
}

// ---- the staying notice (2.1, DESIGN.md "One notice that stays until ready") ----

/// The staying notice's keys, in the order one shows when several ask: the screen in front's own (FontCheck,
/// Loading, Previews: only on the screen that asked), then Book (every screen), then Import (the Library and,
/// with 2.1's browser, its screens; not the Import place, whose rows say the same).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoticeKey {
    /// new fonts being checked (2.1.0-beta.2), on the Font sheet above its buttons
    FontCheck,
    /// a folder being read in the file browser (2.1.0-beta.2)
    Loading,
    /// previews being made for the file browser (2.2)
    Previews,
    /// KOReader's book being added
    Book,
    /// a PDF import (2.1.0-beta.2)
    Import,
}

impl NoticeKey {
    const ALL: [NoticeKey; 5] = [NoticeKey::FontCheck, NoticeKey::Loading, NoticeKey::Previews, NoticeKey::Book, NoticeKey::Import];

    /// its name for the runner's notice.key
    pub fn name(self) -> &'static str {
        match self {
            NoticeKey::FontCheck => "fontcheck",
            NoticeKey::Loading => "loading",
            NoticeKey::Previews => "previews",
            NoticeKey::Book => "book",
            NoticeKey::Import => "import",
        }
    }
}

/// The staying notice's redraws for a new text, at most one this often.
pub const NOTICE_EVERY_MS: u32 = 2000;

/// What each key asks the staying notice to say (empty: nothing), with the screen that asked, and what the
/// panel shows of it. Drawn like a toast: overlaid for its presents, never stored in App.screen.
#[derive(Default)]
pub struct Notice {
    asked: [(FixedStr<256>, ScreenId); 5],
    /// the key and text on the panel, and where (empty: nothing of it is on the panel)
    shown: Option<NoticeKey>,
    shown_text: FixedStr<256>,
    rect: Rect,
    /// when it was last drawn
    drawn_ms: u32,
    /// its own presents, counted (a Ui each, none of them in ui_refreshes_since_full)
    pub presents: u32,
    under: Canvas,
}

/// The staying notice for `key` says `text` until app_notice_clear(key) (or another text), on the screens its
/// key belongs to. It has no strings of its own: the texts are its users'. It is drawn at the next flush, in
/// the toast's place: a new text redraws it at most once every 2 s, by one Ui of its rect outside the count
/// whose 30th Ui is a Full, so a long job never flashes the panel. A toast goes over it for the toast's time.
pub fn app_notice(a: &mut App, key: NoticeKey, text: &str) {
    let asked = &mut a.notice.asked[key as usize];
    asked.0.set_cut(text);
    asked.1 = a.scr;
}

/// The notice for `key` is done: its rect is repainted from the screen by one Ui at the next flush (also
/// outside the count), or the next key's notice takes its place.
pub fn app_notice_clear(a: &mut App, key: NoticeKey) {
    a.notice.asked[key as usize].0.clear();
}

/// The key whose notice shows now: none while a toast is up.
fn notice_pick(a: &App) -> Option<NoticeKey> {
    if a.toast_visible {
        return None;
    }
    NoticeKey::ALL.into_iter().find(|&k| {
        let (text, asker) = &a.notice.asked[k as usize];
        !text.is_empty()
            && match k {
                NoticeKey::FontCheck | NoticeKey::Loading | NoticeKey::Previews => a.scr == *asker,
                NoticeKey::Book => true,
                NoticeKey::Import => a.scr == ScreenId::Library && !crate::library::library_import_place_up(a),
            }
    })
}

#[cfg(test)]
thread_local! {
    static NOTICE_CLOCK: Cell<Option<u32>> = const { Cell::new(None) };
}

fn notice_now() -> u32 {
    #[cfg(test)]
    if let Some(t) = NOTICE_CLOCK.with(|c| c.get()) {
        return t;
    }
    now_ms()
}

/// Draws the notice for `key` at `r` into the screen, keeping what it covers to put back (notice_put_back).
fn notice_draw(a: &mut App, key: NoticeKey, r: Rect) {
    let text = a.notice.asked[key as usize].0;
    canvas_free(&mut a.notice.under);
    if canvas_init(&mut a.notice.under, r.w, r.h) != 0 {
        return;
    }
    canvas_blit(&mut a.notice.under, &a.screen, r, 0, 0);
    ui_toast_at(&mut a.screen, r, text.as_str());
    a.notice.shown = Some(key);
    a.notice.shown_text = text;
    a.notice.rect = r;
    a.notice.drawn_ms = notice_now();
}

fn notice_put_back(a: &mut App, r: Rect) {
    if a.notice.under.w == r.w && a.notice.under.h == r.h && !a.notice.under.px.is_empty() {
        canvas_blit(&mut a.screen, &a.notice.under, Rect { x: 0, y: 0, w: r.w, h: r.h }, r.x, r.y);
    }
    canvas_free(&mut a.notice.under);
}

/// The notice rides a present that reaches its place (or the place it leaves): drawn into it, or its old
/// place repainted. Returns where it was drawn, for notice_put_back after the present.
fn notice_ride(a: &mut App) -> Option<Rect> {
    let old = a.notice.rect;
    let want = notice_pick(a).map(|k| (k, toast_place_for(a, a.notice.asked[k as usize].0.as_str())));
    let reached = want.is_some_and(|(_, r)| rect_overlaps(a.dirty, r)) || rect_overlaps(a.dirty, old);
    if !reached {
        return None;
    }
    a.dirty = rect_union(a.dirty, old);
    a.notice.rect = RECT_EMPTY;
    a.notice.shown = None;
    let (k, r) = want?;
    a.dirty = rect_union(a.dirty, r);
    notice_draw(a, k, r);
    Some(r)
}

/// The notice's own present, after the flush (as the clock corner's is): when a key asks for it and it is not
/// on the panel, when its place moved, when its text changed and 2 s have gone since it was drawn, or when
/// nothing asks for it any more and it is on the panel. One Ui of its rect, never counted toward the Full;
/// held while the pen is down, as toasts are.
fn notice_flush(a: &mut App) {
    if a.asleep || a.pen_action_down {
        return;
    }
    let old = a.notice.rect;
    let want = notice_pick(a);
    let (mut area, drawn) = match want {
        None if rect_is_empty(old) => return,
        None => (old, None),
        Some(k) => {
            let text = a.notice.asked[k as usize].0;
            let r = toast_place_for(a, text.as_str());
            if r == old && a.notice.shown == Some(k) {
                let same = text.as_str() == a.notice.shown_text.as_str();
                if same || notice_now().wrapping_sub(a.notice.drawn_ms) < NOTICE_EVERY_MS {
                    return;
                }
            }
            (rect_union(old, r), Some((k, r)))
        }
    };
    area = rect_intersect(area, Rect { x: 0, y: 0, w: a.w, h: a.h });
    a.notice.rect = RECT_EMPTY;
    a.notice.shown = None;
    let corner = status_cover(a, &mut area);
    if let Some((k, r)) = drawn {
        notice_draw(a, k, r);
    }
    display_present(&a.screen, area, RefreshMode::Ui);
    if let Some((_, r)) = drawn {
        notice_put_back(a, r);
    }
    if let Some(c) = corner {
        status_uncover(a, c);
    }
    status_presented(a, area);
    a.notice.presents = a.notice.presents.wrapping_add(1);
    notice_live(a);
}

/// The staying notice in Live view's toast layer as the panel has it (it is in the toast's place, and never
/// with the toast), or nothing once it is gone.
fn notice_live(a: &App) {
    if !live_on() || a.toast_visible {
        return;
    }
    if rect_is_empty(a.notice.rect) {
        live_layer(LIVE_LAYER_TOAST, &a.screen, RECT_EMPTY);
        return;
    }
    let text = a.notice.shown_text;
    live_layer_drawn(LIVE_LAYER_TOAST, a.notice.rect, |c, all| ui_toast_at(c, all, text.as_str()));
}

/// What the app overlays on the panel that App.screen does not hold (the toast, the staying notice): the
/// screen guard leaves it out of its comparison.
pub fn app_overlay_rect(a: &App) -> Rect {
    let toast = if a.toast_visible { a.toast_rect } else { RECT_EMPTY };
    rect_union(toast, a.notice.rect)
}

/// Same, with the refresh to use (waking uses a no-flash refresh: faster to get back to work).
pub fn app_redraw_mode(a: &mut App, m: RefreshMode) {
    a.toast_visible = false;
    a.dirty = RECT_EMPTY;
    a.relayout = true;
    (ops(a.scr).leave)(a);
    (ops(a.scr).enter)(a);
    a.relayout = false;
    app_invalidate_all(a, m);
}

/// Redraw the current screen from scratch (after sleep, or anything that drew over us).
pub fn app_redraw(a: &mut App) {
    app_redraw_mode(a, RefreshMode::Full);
}

// ---- Live view ----

fn live_capture(ev: &mut InEvent) {
    let cap = LIVE_CAP.with(|c| c.get());
    if !cap.on || cap.asleep || cap.input_yielded {
        return;
    }
    let mut style = LiveStyle::default();
    if cap.editor && !LIVE_CAPTURE_BLOCKED.with(|b| b.get()) && LIVE_UI_PENDING.with(|p| p.get()) == 0 {
        style = cap.styles[ev.eraser as usize][(ev.side_button as usize) | ((ev.side_button2 as usize) << 1)];
    }
    let ui = ev.type_ == InType::PenDown && (style.enabled == 0 || rect_contains(style.exclude, ev.x as i32, ev.y as i32));
    if ui {
        style = LiveStyle::default(); // (a pen-down on UI draws nothing: its LIVE_DRAW says so too)
    }
    ev.live_seq = live_sample(ev, &style);
    if ui {
        LIVE_UI_PENDING.with(|p| p.set(ev.live_seq));
    }
    live_pen(
        if ev.type_ == InType::PenLeave {
            LIVE_PEN_AWAY
        } else if ev.type_ == InType::PenDown || ev.type_ == InType::PenMove {
            LIVE_PEN_TOUCH
        } else {
            LIVE_PEN_HOVER
        },
        ev.x,
        ev.y,
    );
}

// The pen hovering, from input_hover_hook while Live view is on (on the Kindle the pen's own events
// never say where it hovers). Never asleep, as for app_event's pen events.
fn live_hover(x: f32, y: f32) {
    if G_LIVE_AWAKE.load(Ordering::Relaxed) {
        live_pen(LIVE_PEN_HOVER, x, y);
    }
}

pub fn app_watch_server(a: &mut App) {
    input_watch_fd(if httpd_running() && !a.asleep { httpd_fd() } else { -1 });
}

// Where the chip that says Live view is on sits. On a notebook page it is beside the editor's panel, and moves with
// it: left of it (right where the panel is at the screen's left edge), level with its header; with a menu, sheet or
// bar in front, which draws over the panel, there is none. On the other screens (no panel) it is in the top right
// corner, inside the screens' 50 px margin (clear of the Library's buttons). Never in the stream.
fn live_chip(a: &App) -> Rect {
    if status_reserved(a) > 0 { return RECT_EMPTY; }
    let s = ui_px(42.0);
    let m = ui_px(4.0);
    if a.scr == ScreenId::Editor {
        return editor_live_chip(a, s);
    }
    Rect { x: a.w - s - m, y: m, w: s, h: s ,
    }
}

// The chip is where live_chip says now: when it moved (the panel was dragged or folded, a menu opened or closed,
// Live view started or stopped) the pixels it left are shown again from the screen, and the screen guard skips
// the new place.
fn live_chip_sync(a: &mut App) {
    let now = if live_on() { live_chip(a) } else { RECT_EMPTY };
    if now == a.chip_at {
        return;
    }
    let old = a.chip_at;
    a.chip_at = now;
    display_skip_overlay(now);
    app_invalidate(a, old, RefreshMode::Ui); // (what is under it comes back)
    app_invalidate(a, now, RefreshMode::Ui);
}

// The raw digitizer's map for the pen viewers (input_raw_pen), as it stands: at each tick, as Live
// view starts and as the screen turns (what changes it: a turn, the pen given to the Kindle, the
// place learned or lost).
fn live_raw_sync() {
    let mut m = LiveRawMap::default();
    let ok = input_raw_pen(&mut m);
    live_set_raw_map(if ok { Some(&m) } else { None });
}

// A rect of a.screen presented, into Live view's frame (live_push): in the editor, the page under its
// panel instead of the panel (composed again without it, editor_live_under_panel), and the panel as
// its own layer (live_layer), for the browser to show or not.
fn live_push_screen(a: &mut App, r: Rect) {
    if !live_on() {
        return;
    }
    // The layer is the panel the browser draws over the frame, as the screen holds it: with a menu over it, the menu
    // there too, as the Kindle shows it (issue #16); the frame is pushed without the panel wherever the screen holds
    // one, the page and any menu over it in its place.
    let (panel, hole) = if a.scr == ScreenId::Editor { (editor_live_panel(a), editor_live_hole(a)) } else { (RECT_EMPTY, RECT_EMPTY) };
    let under = rect_intersect(r, hole);
    let moved = with(|s| s.live_panel) != panel;
    if !rect_is_empty(under) || moved {
        live_layer(LIVE_LAYER_PANEL, &a.screen, panel);
        with(|s| s.live_panel = panel);
    }
    if rect_is_empty(under) {
        live_push(&a.screen, r);
    } else {
        editor_live_under_panel(a, under, r);
    }
}

/// Live view (live.h): the Share screen's button turns it on and shows the pairing card (again,
/// while it is on); live_view_stop turns it off (say: with a toast). app_watch_server keeps
/// input_poll watching the share server while it runs and the Kindle is awake.
pub fn live_view_open(a: &mut App) {
    if !live_on() {
        let mut ok = live_start(a.w, a.h) == 0;
        if ok && httpd_acquire(HTTPD_LIVE, a.settings.http_port, a.dpi, ui_theme() == UiTheme::Gothic,
            ) != 0 {
            live_stop();
            ok = false;
        }
        if !ok {
            // TRANSLATORS: a toast: Live view could not start (no memory for its copy of the screen,
            // or the server's port is taken).
            app_toast(a, tr("Couldn't start Live view"), 3000);
            return;
        }
        with(|s| {
            s.live_viewers = 0;
            s.live_panel = RECT_EMPTY; // (the new frame has no layer yet)
        });
        httpd_live_keys_load(&format!("{}/live-keys.ini", store_root()));
        live_set_invert(a.settings.invert);
        let (mut intensity, mut warmth) = (0, 0);
        light_get(&mut intensity, &mut warmth); // (once: then light_warmth has it, as the Display sheet sets it)
        live_set_warmth(warmth);
        G_LIVE_APP.store(true, Ordering::Relaxed);
        live_hover_sync(a);
        a.live_processed = 0;
        a.live_processed_ms = 0;
        LIVE_UI_PENDING.with(|p| p.set(0));
        live_cap_sync(a);
        live_raw_sync();
        status_live(a); // (the clock and battery corner's layer, if it is up)
        input_sample_hook(Some(live_capture), Some(live_wake_samples));
        input_hover_hook(Some(live_hover));
        live_chip_sync(a);
        qlog!("live: on");
    }
    share_open_live(a);
}

pub fn live_view_stop(a: &mut App, say: bool) {
    if !live_on() {
        return;
    }
    input_hover_hook(None);
    input_sample_hook(None, None);
    G_LIVE_APP.store(false, Ordering::Relaxed);
    live_hover_sync(a);
    live_stop();
    pair_forget(PAIR_LIVE); // the browsers that paired must pair again next time
    httpd_release(HTTPD_LIVE);
    if !a.live_holds.is_empty() {
        a.live_holds.clear();
        editor_hold_eraser(a, a.hold_code != 0);
    }
    app_watch_server(a);
    live_chip_sync(a); // (the chip goes: what is under it comes back)
    with(|s| s.live_viewers = 0);
    qlog!("live: off");
    // TRANSLATORS: a toast as Live view stops (the Stop button on its card).
    if say { app_toast(a, tr("Live view ended."), 2500); }
}

fn live_tick(a: &mut App, now: u32) {
    if !live_on() {
        return;
    }
    live_set_invert(a.settings.invert); // (however it changed: the browsers follow it)
    live_set_warmth(light_warmth());
    live_raw_sync();
    if a.scr != ScreenId::Share {
        httpd_poll(); // (Share's own tick polls it there)
    }
    let mut idle = false;
    let n = live_poll(now, &mut idle);
    if idle && !share_live_card_up(a) {
        // (the card keeps its code fresh: it waits there)
        live_view_stop(a, false);
        // TRANSLATORS: a toast: Live view turned itself off after 10 minutes with nobody watching.
        app_toast(a, tr("Live view off: no viewers for 10 minutes."), 4000,
        );
        return;
    }
    // TRANSLATORS: a toast as the first computer starts watching the screen (Live view).
    if n > 0 && with(|s| s.live_viewers) == 0 { app_toast(a, tr("A computer is watching this screen."), 3000); }
    with(|s| s.live_viewers = n);
}

// The sleep screen: the page you were on (or the pinned page), so the Kindle "sleeps on" your
// notebook and wakes straight back into it. Mode 2 hands over to the Kindle's own screensaver.
fn app_sleep(a: &mut App) {
    if a.asleep {
        // Already on the sleep screen, so a press that was waiting for a wake has its answer: the
        // Kindle went (back) to sleep. The fallback must not wake Grimoire over it.
        a.wake_fallback_at = 0;
        return;
    }
    a.asleep = true;
    keyboard_holds_end(); // (a lift while asleep is dropped: a held delete must not go on deleting at the wake)
    live_hover_sync(a);
    status_save_now(a); // (a setting the corner changed, saved after its refresh: the refresh is not coming)
    status_reset(); // (the sleep screen is drawn without the corner)
    a.wake_redraw_at = 0;
    a.wake_fallback_at = 0;
    live_set_state(LIVE_ASLEEP); // (the sleep screen is never streamed: it is never pushed)
    app_watch_server(a);
    if a.scr == ScreenId::Editor && !editor_suspend(a) {
        qlog!("sleep: could not save the open page; keeping it in memory");
    }
    if a.scr == ScreenId::Share {
        share_sleep(a);
    }
    qlog!("sleep: mode %d", a.settings.sleep_mode);
    let mode = sleep_mode(a);
    with(|s| s.sleep_shown = mode);
    display_sleep();
    if mode == 2 {
        a.input_yielded = true;
        live_hover_sync(a);
        if !input_set_grab(false) || !sys_kindle_ui(true) {
            qlog!("sleep: could not hand input and screen to the Kindle; leaving for native unlock");
            a.running = false;
        }
        return;
    }
    let full = Rect { x: 0, y: 0, w: a.w, h: a.h ,
    };
    let mut pinned = Canvas::default();
    let mut wallpaper = false;
    if mode == 3 {
        if a.koreader_wallpaper.px.is_empty() {
            store_load_koreader_sleep(&mut a.koreader_wallpaper);
        }
        if !a.koreader_wallpaper.px.is_empty() {
            wallpaper = true;
        } else {
            qlog!("sleep: no KOReader image wallpaper available; showing the page");
            with(|s| s.sleep_shown = 0);
        }
    }
    let use_pinned = mode == 1 && store_load_sleep_image(&mut pinned) == 0;
    // The screen is taken out of the App while the picture is drawn into it, as the picture may be
    // the editor's page, which editor_page_canvas lends from the App (PORTING.md, The App).
    let mut c = std::mem::take(&mut a.screen);
    let drawn = {
        let src: Option<&Canvas> = if use_pinned {
            Some(&pinned)
        } else if wallpaper {
            Some(&a.koreader_wallpaper)
        } else {
            editor_page_canvas(a)
        };
        match src {
            Some(src) => {
                canvas_reset_clip(&mut c);
                canvas_fill(&mut c, full, 255);
                canvas_blit_fit(&mut c, src, full);
                true
            }
            None => false,
        }
    };
    a.screen = c;
    if !drawn && a.scr == ScreenId::Share {
        canvas_reset_clip(&mut a.screen);
        share_draw_asleep(a); // not the code: nothing answers it while asleep
    } // (else, with no page open, the screen as it is, e.g. the library, stays up)
    display_present(&a.screen, full, RefreshMode::Full);
    display_wait();
    canvas_free(&mut pinned);
}

fn app_wake(a: &mut App) {
    if !a.asleep {
        return;
    }
    if a.input_yielded {
        if !sys_kindle_ui(false) {
            qlog!("wake: could not take the screen and input back; leaving");
            a.running = false;
            return;
        }
        a.input_yielded = false;
    }
    // Wallpaper/page sleep also crosses a hardware suspend: releases may be lost and queued
    // contacts may belong to the screensaver. Drain and resync every wake, suppressing contacts
    // already down until they lift, just as after native unlock.
    if !input_set_grab(true) {
        qlog!("wake: could not reset input ownership; leaving");
        a.running = false;
        return;
    }
    display_wake();
    a.asleep = false;
    live_hover_sync(a);
    status_reset();
    a.wake_fallback_at = 0;
    a.power_pressed_at = 0; // a press taken for a sleep press was this wake's: no sleep request after it
    a.locked_wakes = 0; // it woke, so it was not locked
    a.last_input = now_ms();
    a.sleep_requested = false;
    qlog!("wake");
    // (On the pairing card its own enter says so again before the code is drawn.)
    if !share_live_card_up(a) {
        live_set_state(LIVE_AWAKE);
    }
    app_watch_server(a);
    let mode = sleep_mode(a);

    // A sleep request turned preventScreenSaver off: "Sleep after: never" wants it back on.
    app_apply_sleep_policy(a);
    // Back to work at once (no-flash refresh), then once more a moment later in case the Kindle
    // UI (its screensaver closing, the status bar) drew something on the way out.
    app_redraw_mode(a, RefreshMode::Ui);
    a.wake_redraw_at = now_ms().wrapping_add(if mode == 2 { 2500 } else { 1500 });
}

// Saves and leaves Grimoire: the way out of any stuck state (a 4 s hold of the power button, a
// Kindle that stays locked). The launcher's exit trap gives the Kindle its UI back. A page that
// cannot be written (a full store) is never lost unannounced: awake, the first try says so and
// stays, and another within a minute leaves without the page. Asleep (no toast can be seen, and
// the Kindle needs its pen and touch back) it leaves at once, and the log says what was lost.
fn app_leave(a: &mut App) {
    a.power_down_at = 0; // the press is spent: its release, if it is read at all, asks for nothing
    let saved = a.scr != ScreenId::Editor || editor_save_now(a);
    let now = now_ms();
    if !saved {
        qlog!("leaving: the open page could not be saved");
    }
    if !a.asleep {
        let warned = a.leave_unsaved_at != 0 && now.wrapping_sub(a.leave_unsaved_at) < 60000;
        if !saved && !warned {
            a.leave_unsaved_at = now | 1;
            qlog!("leaving: staying this once (holding the power button 4 s again within a minute leaves without the page)");
            // TRANSLATORS: holding the power button 4 s leaves Grimoire, but this page could not be
            // saved (a full storage), so it stays once.
            app_toast(a, tr("Couldn't save page. Hold power 4 s again to leave anyway."), 10000,
            );
            return;
        }
        let mut msg = String::new();
        // TRANSLATORS: the last notice before Grimoire closes, with the page saved, or without it;
        // %s is the app's name, Grimoire.
        csnprintf!(&mut msg, 256, if saved { tr("Saved. Leaving %s.") } else { tr("Leaving %s without saving page.") }, APP_NAME);
        app_toast(a, &msg, 3000); // shown by the last flush
    }
    a.running = false;
}

/// Settings > Updates' Update now: the request is written; Grimoire leaves (the launcher hands the
/// update to update.sh).
pub fn app_leave_for_update(a: &mut App) {
    let hosted = app_host() == "koreader";
    // TRANSLATORS: the last notice before Grimoire closes for an update (Settings > Updates): KPM
    // installs it, then Grimoire starts again; or, opened from KOReader, KOReader comes back and
    // opens the new one next time.
    app_toast(a, if hosted { tr("Updating. Reopen Grimoire from KOReader in a moment.") } else { tr("Updating. Grimoire restarts by itself.") }, 3000,
    );
    a.running = false;
}

/// Turn the UI to a device rotation now (relayouts every screen).
pub fn app_set_rotation(a: &mut App, quarter_turns: i32) {
    let r = quarter_turns & 3;
    if a.settings.rotation_lock != 0 && r != a.settings.rotation_lock - 1 {
        return;
    }
    // (every platform fills both; the App's own size stands in until it does)
    let (mut w, mut h) = (a.w, a.h);
    display_set_orientation(r, &mut w, &mut h);
    input_set_orientation(r);
    a.rotation = r;
    if w != a.w || h != a.h {
        let mut nc = Canvas::default();
        if canvas_init(&mut nc, w, h) != 0 {
            return;
        }
        canvas_free(&mut a.screen);
        a.screen = nc;
        a.w = w;
        a.h = h;
    }
    a.dirty = RECT_EMPTY;
    live_raw_sync(); // (the turned map before the new epoch: no frame of it placed the old way)
    live_resize(w, h);
    with(|s| s.live_panel = RECT_EMPTY); // (live_resize took the layers away)
    a.chip_at = RECT_EMPTY; // (the whole screen is drawn again: the next flush puts the chip where it goes now)
    // Re-lay out the current screen in the new shape.
    a.relayout = true;
    (ops(a.scr).leave)(a);
    (ops(a.scr).enter)(a);
    a.relayout = false;
    if a.toast_visible {
        a.toast_rect = toast_place(a); // the new shape (and the floor its enter set)
    }
    app_invalidate_all(a, RefreshMode::Full);
}

/// Apply the sleep policy to the power service (call after changing settings.sleep_minutes).
pub fn app_apply_sleep_policy(a: &mut App) {
    let want = (a.settings.sleep_minutes == 0) as i32;
    if want == a.prevent_applied {
        return;
    }
    a.prevent_applied = want;
    sys_prevent_sleep(want != 0);
}

// Asks the power service to sleep. The request turns preventScreenSaver off first (the button is
// ignored while it is on); app_wake puts a "never" policy back.
fn request_sleep(a: &mut App) {
    sys_request_sleep();
    a.prevent_applied = 0;
}

// The power service's state, for a fallback to decide on and log ("unknown" if unreadable). It
// blocks a moment (lipc), which only these rare fallbacks can afford.
fn powerd_state_now(name: &mut String, cap: usize) -> PowerdState {
    if sys_powerd_state(name, cap) != 0 {
        strlcpy_q(name, "unknown", cap);
        return PowerdState::Unknown;
    }
    powerd_parse_state(Some(name.as_str()))
}

// A new notebook an import made: its Library folder goes into folders.ini (the UI's file: never written by a job
// child beside it), and whether the Library shows its card now (D20: then no toast says "Imported").
fn import_made(a: &App, id: &str) -> bool {
    let mut nb = NotebookInfo::default();
    if store_load_info(id, &mut nb) != 0 {
        return false;
    }
    if !nb.folder.is_empty() {
        store_add_folder(nb.folder.as_str());
    }
    crate::library::library_shows_folder(a, nb.folder.as_str())
}

// A PDF import ended (pdfimport.rs): what to tell the user. With the Library in front and the new card on it
// (D20) no toast says "Imported"; elsewhere one does. One that left pages blank says so whatever is in front,
// as it is news the card does not carry.
fn pdf_import_done(a: &mut App, id: &str, ok: bool) {
    use crate::pdfimport::{FailWhy, Finished};
    let fin = crate::pdfimport::finished(id, ok);
    let mut msg = String::new();
    match fin {
        Finished::Nothing => {}
        Finished::Imported { pages, blank, trimmed } => {
            let shown = import_made(a, id);
            if a.scr == ScreenId::Library && !a.asleep {
                app_redraw_mode(a, RefreshMode::Ui); // (waking redraws it too)
            }
            if blank > 0 {
                // TRANSLATORS: a PDF became a new notebook, but some of its pages could not be read and are
                // blank; %d is its page count. In a toast: one line of about 1700 px at 44 px in bold, or two.
                csnprintf!(&mut msg, 256, ng_("Imported notebook (%d page). Some pages couldn't be read.", "Imported notebook (%d pages). Some pages couldn't be read.", pages as i64), pages as i32);
            } else if trimmed > 0 {
                // TRANSLATORS: a PDF became a new notebook, but its document links (a planner's tabs) were more than
                // a notebook holds and some were left out; %d is its page count. In a toast: one or two lines.
                csnprintf!(&mut msg, 256, ng_("Imported notebook (%d page). Some links were left out.", "Imported notebook (%d pages). Some links were left out.", pages as i64), pages as i32);
            } else if !shown {
                // TRANSLATORS: a folder of page pictures (grimoire/import/<name>) or a PDF became a new notebook.
                csnprintf!(&mut msg, 256, ng_("Imported notebook (%d page).", "Imported notebook (%d pages).", pages as i64), pages as i32);
            }
        }
        Finished::Already => {
            // TRANSLATORS: a toast: the PDF dropped in grimoire/import (or picked) is one Grimoire made a notebook
            // of before. A file of grimoire/import moves to import/done; a file picked elsewhere stays where it is.
            csnprintf!(&mut msg, 256, "%s", tr("PDF already imported."));
        }
        Finished::Failed(why) => match why {
            FailWhy::Password => {
                // TRANSLATORS: a toast: the PDF is protected by a password, which Grimoire cannot ask for.
                csnprintf!(&mut msg, 256, "%s", tr("PDF needs a password."));
            }
            FailWhy::TooLong(n) => {
                // TRANSLATORS: a toast: the PDF has more pages than a notebook holds; %d is that number (2000).
                csnprintf!(&mut msg, 256, tr("PDF too long: notebooks hold %d pages."), n as i32);
            }
            FailWhy::TooBig(mb) => {
                // TRANSLATORS: a toast: the PDF's file is bigger than Grimoire reads; %d is the size in MB (512).
                csnprintf!(&mut msg, 256, tr("PDF too big: over %d MB."), mb as i32);
            }
            FailWhy::Storage => {
                // TRANSLATORS: a toast: the PDF could not be imported because the Kindle's storage is full.
                csnprintf!(&mut msg, 256, "%s", tr("Couldn't import PDF. Is the storage full?"));
            }
            FailWhy::Generic => {
                // TRANSLATORS: a toast: the PDF could not become a notebook; %s is the log file that says why.
                // The PDF stays where it was.
                csnprintf!(&mut msg, 256, tr("Couldn't import PDF (see %s)."), "grimoire.log");
            }
        },
    }
    if !msg.is_empty() {
        app_toast(a, &msg, 8000);
    }
}

// The Library's rescan of import/ (a PDF copied in while Grimoire runs): when the Library comes in front, at most
// every 10 s, and again after 5 s while a file waits to hold still. No timer runs while the editor is in front.
pub fn app_import_rescan(a: &mut App, entering: bool) {
    let now = now_ms();
    if a.asleep || a.scr != ScreenId::Library || !crate::pdfimport::scan_due(now, entering) {
        return;
    }
    crate::pdfimport::scan_noted(now);
    import_scan(a.dpi);
}

// The staying notice of an import (the Library's: "Importing: page 37 of 412"), from the shared progress record
// the writer fills, or from the coarse progress in import/.marks while a previous run's detached child imports.
// Also: imports the job queue no longer knows are let go of (M11), and the end of a previous run's detached
// import is seen within 10 s, its notebook shown and the files waiting taken (M8).
fn pdf_import_tick(a: &mut App, now: u32) {
    if (crate::pdfimport::pending_count() > 0 || crate::bgwork_template::pictures_pending() > 0) && !bg_busy() {
        crate::pdfimport::reconcile(a.dpi);
        crate::pdfimport::lock_release_if_idle();
    }
    if crate::pdfimport::detached_poll(now) && a.scr == ScreenId::Library && !a.asleep {
        app_redraw_mode(a, RefreshMode::Ui);
    }
    match crate::pdfimport::progress_text(now) {
        Some((page, pages)) => {
            let mut msg = String::new();
            // TRANSLATORS: the staying notice at the bottom of the Library while a PDF becomes a notebook
            // (and a row of the Import place): the page it is on, of how many. One line of about 1700 px.
            csnprintf!(&mut msg, 256, tr("Importing: page %d of %d"), page as i32, pages as i32);
            app_notice(a, NoticeKey::Import, &msg);
            crate::pdfimport::notice_set(true);
        }
        None => {
            if crate::pdfimport::notice_on() && crate::pdfimport::pending_count() == 0 && !crate::pdfimport::detached_running() {
                app_notice_clear(a, NoticeKey::Import);
                crate::pdfimport::notice_set(false);
            }
        }
    }
    if a.scr == ScreenId::Library {
        app_import_rescan(a, false);
    }
}

// A background job that finished: an import says how it went (the Library shows its new notebook),
// a template's picture may now be there to draw. Returns bg_poll's result for the PDF mirror's toast.
fn poll_jobs(a: &mut App) -> i32 {
    let mut done = -1;
    let mut done_id=FixedStr::new();
    let r = bg_poll_job(Some(&mut done),Some(&mut done_id));
    if done == BgKind::PagePreviews as i32 {
        crate::editor::editor_previews_done(a,done_id.as_str());
        return 0;
    }
    if done == BgKind::GraphLayout as i32 {
        crate::editor::editor_map_done(a,done_id.as_str(), r > 0);
        return 0;
    }
    if done == BgKind::GraphLayoutAll as i32 {
        crate::editor::editor_map_all_done(a, r > 0);
        return 0;
    }
    if done == BgKind::Template as i32 {
        store_invalidate_page_previews(done_id.as_str());
        bgimage_retry_missing();
        editor_picture_made(a); // (the open page may be on that picture: a page KOReader sent)
    }
    if done == BgKind::PdfImport as i32 {
        pdf_import_done(a, done_id.as_str(), r > 0);
        return 0;
    }
    if done == BgKind::FontCheck as i32 {
        // The Font sheet's rows fill in; the font settings.ini asks for, if it waited for this, draws from now
        // on (only while the Library is in front: never in the middle of a page).
        let changed = crate::userfonts::userfonts_job_done(a, bg_last_exit());
        crate::library::library_fonts_checked(a, changed);
        return 0;
    }
    if done != BgKind::ImportFolder as i32 {
        return r;
    }
    let mut pages = 0;
    let got = crate::bgwork_template::import_finished_id(done_id.as_str(), r > 0, &mut pages);
    crate::pdfimport::lock_release_if_idle(); // (the lock is the UI's while any import is queued or running)
    let shown = got > 0 && import_made(a, done_id.as_str());
    if got > 0 && a.scr == ScreenId::Library && !a.asleep {
        app_redraw_mode(a, RefreshMode::Ui); // (waking redraws it too)
    }
    if got > 0 && !shown {
        let mut msg = String::new();
        // TRANSLATORS: a folder of page pictures (grimoire/import/<name>) became a new notebook.
        csnprintf!(&mut msg, 256, ng_("Imported notebook (%d page).", "Imported notebook (%d pages).", pages as i64), pages);
        app_toast(a, &msg, 8000);
    } else if got < 0 {
        let mut msg = String::new();
        // TRANSLATORS: a folder of page pictures could not become a notebook; %s is the log file
        // that says why. The folder stays where it was.
        csnprintf!(&mut msg, 256, tr("Couldn't import page folder (see %s)."), "grimoire.log");
        app_toast(a, &msg, 8000);
    }
    0
}

// A minute of "Sleep after". In the simulator a scenario can make it shorter
// (GRIMOIRE_SLEEP_MINUTE_MS), to see the timer run out without waiting minutes for it.
fn sleep_minute_ms() -> i32 {
    #[cfg(feature = "sim")]
    {
        let ms = with(|s| s.minute_ms);
        if ms != 0 {
            return ms;
        }
        let s = getenv("GRIMOIRE_SLEEP_MINUTE_MS");
        let ms = match s.as_deref() {
            Some(s) if atoi(s) > 0 => atoi(s),
            _ => 60000,
        };
        with(|s| s.minute_ms = ms);
        ms
    }
    #[cfg(not(feature = "sim"))]
    {
        60000
    }
}

/// timers: autosave, toast, long press, httpd
pub fn app_tick(a: &mut App, now: u32) {
    app_tick_screens(a, now);
    live_keys_tick(a);
    live_cap_sync(a);
}

fn app_tick_screens(a: &mut App, now: u32) {
    // The Bluetooth helper's script runs and requests are reaped even asleep.
    if a.bt_start_at != 0 && now.wrapping_sub(a.bt_start_at) as i32 >= 0 {
        a.bt_start_at = 0;
        if bt_present() {
            bt_start();
        }
    }
    let mut bt_reply = BtReply::default();
    let bt = bt_tick(&mut bt_reply);
    if bt != BtEvent::None {
        a.bt_changes = a.bt_changes.wrapping_add(1);
        if a.scr == ScreenId::Library || crate::library::library_settings_active(a) {
            library_bt(a, bt, &bt_reply);
        }
    }
    wifi_tick();
    book_tick(a);
    pdf_import_tick(a, now);
    let bg = poll_jobs(a);
    if bg != 0 {
        let mut msg = String::new();
        // TRANSLATORS: a PDF saved in the background, and where (the Kindle's folder of them).
        if bg > 0 { csnprintf!(&mut msg, 256, tr("PDF saved to %s"), "grimoire/PDFs"); }
        app_toast(a, if bg > 0 { msg.as_str() } else { tr("Couldn't save PDF") }, 2500,
        );
    }
    if store_take_corrupt_count() > 0 {
        let mut msg = String::new();
        // TRANSLATORS: a page that could not be read; %s is the log file that says which.
        csnprintf!(&mut msg, 256, tr("Damaged page replaced with a blank one (see %s)"), "grimoire.log");
        app_toast(a, &msg, 6000);
    }
    // A short power press the power service did not follow with a sleep announcement. If it is in
    // its screensaver already, Grimoire missed the announcement: show the sleep screen and send
    // nothing (a powerButton now would wake the Kindle again). Otherwise ask it to sleep.
    if a.power_pressed_at != 0 && !a.asleep && now.wrapping_sub(a.power_pressed_at) as i32 > 2500 {
        a.power_pressed_at = 0;
        let mut st = String::new();
        if powerd_dozing(powerd_state_now(&mut st, 48)) {
            qlog!("sleep: no announcement after the power button, but the power service is in %s; going to sleep without asking", &st);
            app_sleep(a);
        } else {
            qlog!("sleep: no announcement after the power button (power service: %s); requesting sleep", &st);
            request_sleep(a);
        }
    }
    // A press on the sleep screen with no outOfScreenSaver after it. A power service still in its
    // screensaver means the Kindle is still locked (its passcode) or asleep: never draw over it.
    // The second time in a row Grimoire leaves instead, so the Kindle gets its pen and touch back
    // even where the 4 s hold cannot be seen (a power key that reports press and release at once).
    if a.asleep && a.wake_fallback_at != 0 && now.wrapping_sub(a.wake_fallback_at) as i32 >= 0 {
        a.wake_fallback_at = 0;
        let mut st = String::new();
        if a.native_pin {
            qlog!("wake: waiting for the Kindle's native PIN unlock (outOfScreenSaver)");
            return;
        }
        match power_wake_fallback(powerd_state_now(&mut st, 48), &mut a.locked_wakes) {
            WakeFallback::Stay => {
                qlog!(
                    "wake: no outOfScreenSaver within 3 s of the power button, and the power service is in %s: the Kindle is still locked or asleep, so Grimoire stays on its sleep screen (another press that brings no wake, or holding the power button 4 s, leaves Grimoire)",
                    &st
                );
            }
            WakeFallback::Leave => {
                qlog!(
                    "wake: again no outOfScreenSaver within 3 s of the power button, and the power service is in %s: the Kindle stays locked, so Grimoire saves and leaves (the Kindle can then ask for its passcode)",
                    &st
                );
                app_leave(a);
            }
            _ => {
                qlog!("wake: power pressed but no outOfScreenSaver within 3 s (power service: %s); waking anyway", &st);
                app_wake(a);
            }
        }
    }
    if a.wake_redraw_at != 0 && now.wrapping_sub(a.wake_redraw_at) as i32 >= 0 && !(a.scr == ScreenId::Editor && editor_busy(a)) {
        a.wake_redraw_at = 0;
        app_invalidate_all(a, RefreshMode::Ui); // the screen is already right: just push it again
    }
    // Sleep after a while without input, the same way the power button does.
    if !a.asleep
        && !a.sleep_requested
        && a.settings.sleep_minutes > 0
        && now.wrapping_sub(a.last_input) as i32 >= a.settings.sleep_minutes.wrapping_mul(sleep_minute_ms())
        && !(a.scr == ScreenId::Editor && editor_busy(a))
        && a.scr != ScreenId::Share
        && live_viewers() == 0
    {
        a.sleep_requested = true;
        request_sleep(a);
    }
    if a.asleep {
        return;
    }
    if a.settings.rotation_lock != 0 {
        a.pending_rotation = -1;
    }
    if a.pending_rotation >= 0 && a.pending_rotation != a.rotation && !(a.scr == ScreenId::Editor && editor_busy(a)) {
        let r = a.pending_rotation;
        a.pending_rotation = -1;
        app_set_rotation(a, r);
    } else if a.pending_rotation == a.rotation {
        a.pending_rotation = -1;
    }
    (ops(a.scr).tick)(a, now);
    live_tick(a, now);
    status_tick(a, now);
    if a.toast_visible && now.wrapping_sub(a.toast_until) as i32 >= 0 {
        toast_hide(a);
    }
}

pub fn app_invalidate(a: &mut App, r: Rect, m: RefreshMode) {
    let r = rect_intersect(r, Rect { x: 0, y: 0, w: a.w, h: a.h ,
        },
    );
    if rect_is_empty(r) {
        return;
    }
    // Keep the strongest mode requested since the last flush.
    if rect_is_empty(a.dirty) || m.strength() > a.dirty_mode.strength() {
        a.dirty_mode = m;
    }
    a.dirty = rect_union(a.dirty, r);
    a.changes = a.changes.wrapping_add(1);
}

pub fn app_invalidate_all(a: &mut App, m: RefreshMode) {
    app_invalidate(a, Rect { x: 0, y: 0, w: a.w, h: a.h ,
        }, m,
    );
}

/// presents dirty rect (if any)
pub fn app_flush(a: &mut App) {
    app_flush_in(a);
    work_pump_mark(); // (a present's waits are the panel's, not work that should pump)
}

fn app_flush_in(a: &mut App) {
    if a.pen_action_down && (a.dirty_mode != RefreshMode::Fast || !rect_is_empty(with(|s| s.self_dirty))) {
        return;
    }
    if a.asleep {
        return; // the sleep screen stays up until wake
    }
    live_chip_sync(a);
    if rect_is_empty(a.dirty) && rect_is_empty(with(|s| s.self_dirty)) {
        live_presented(a.live_processed, a.live_processed_ms);
        status_flush(a); // (the clock and battery corner alone: its own small refresh, if one is due)
        notice_flush(a); // (the staying notice alone, the same way)
        return;
    }
    toast_follow(a); // (a screen that moves its floor redraws, so there is something to flush)
    let mut m = a.dirty_mode;
    let self_dirty = with(|s| s.self_dirty);
    if !rect_is_empty(self_dirty) {
        if rect_is_empty(a.dirty) || m.strength() < RefreshMode::Ui.strength() {
            m = RefreshMode::Ui;
        }
        a.dirty = rect_union(a.dirty, self_dirty);
        with(|s| s.self_dirty = RECT_EMPTY);
    }
    if m == RefreshMode::Ui {
        a.ui_refreshes_since_full += 1;
        if a.ui_refreshes_since_full >= 30 {
            m = RefreshMode::Full;
        }
    }
    if m == RefreshMode::Full {
        a.ui_refreshes_since_full = 0;
    }
    live_push_screen(a, a.dirty); // (before the toast, the chip and the corner: none is streamed)
    live_presented(a.live_processed, a.live_processed_ms);
    // The chip over what it covers, put back after the present, as the toast is.
    let chip = a.chip_at;
    let mut chipped = !rect_is_empty(chip) && rect_overlaps(a.dirty, chip);
    if chipped {
        a.dirty = rect_union(a.dirty, chip);
        canvas_free(&mut a.chip_under);
        chipped = canvas_init(&mut a.chip_under, chip.w, chip.h) == 0;
        if chipped {
            canvas_blit(&mut a.chip_under, &a.screen, chip, 0, 0);
            ui_frame(&mut a.screen, chip, ui_px(8.0), 0, true, 0);
            ui_icon(&mut a.screen, IconKind::Live, rect_inflate(chip, -ui_px(2.0)), 255,
            );
        }
    }
    // The corner's clock over what it covers, put back after the present like the others (the
    // toast goes over it).
    let mut dirty = a.dirty;
    let corner = status_cover(a, &mut dirty);
    a.dirty = dirty;
    let noticed = notice_ride(a); // (the staying notice, never with the toast)
    let reached = a.toast_visible && rect_overlaps(a.dirty, a.toast_rect);
    let mut overlay = reached;
    if overlay {
        a.dirty = rect_union(a.dirty, a.toast_rect);
        canvas_free(&mut a.toast_under);
        overlay = canvas_init(&mut a.toast_under, a.toast_rect.w, a.toast_rect.h) == 0;
        if overlay {
            canvas_blit(&mut a.toast_under, &a.screen, a.toast_rect, 0, 0);
            ui_toast_at(&mut a.screen, a.toast_rect, a.toast.as_str());
            a.toast_presents = a.toast_presents.wrapping_add(1);
        }
    }
    live_toast_sync(a, overlay, reached, noticed.is_some());
    display_present(&a.screen, a.dirty, m);
    du_wear_presented(&mut a.du_wear, a.dirty, m);
    if overlay {
        canvas_blit(&mut a.screen, &a.toast_under, Rect { x: 0, y: 0, w: a.toast_rect.w, h: a.toast_rect.h ,
            }, a.toast_rect.x, a.toast_rect.y,
        );
    }
    if let Some(r) = noticed {
        notice_put_back(a, r);
    }
    if let Some(c) = corner {
        status_uncover(a, c);
    }
    if chipped {
        canvas_blit(&mut a.screen, &a.chip_under, Rect { x: 0, y: 0, w: chip.w, h: chip.h ,
            }, chip.x, chip.y,
        );
    }
    status_presented(a, a.dirty);
    a.dirty = RECT_EMPTY;
    a.dirty_mode = RefreshMode::Fast;
    status_flush(a); // (a refresh of the corner this one did not hold)
    notice_flush(a); // (and of the staying notice)
    crate::userfonts::userfonts_screen_drawn(); // (a custom font's first screen is on the panel: its guard mark goes)
}

// Live view's toast layer (live.rs, LIVE_LAYER_TOAST) as the present app_flush is about to send leaves
// the panel (2.0.0-beta.2, the owner: "Enable toasts on the live view display. they don't show up
// now."): the toast this present draws (drawn), drawn again for the layer by live_layer_drawn with
// alpha, so only the toast's own pixels are in it and none of what is under its round or pointed ends
// (the live chip, the panel and the corner, which the browser shows or not by its own switches); none
// once a present goes out without it (hidden: its place is in this present; or reached and not drawn,
// for want of memory); else as it was, since a present away from the toast leaves it on the panel.
fn live_toast_sync(a: &App, drawn: bool, reached: bool, noticed: bool) {
    if !live_on() {
        return;
    }
    if !drawn {
        if noticed {
            notice_live(a);
        } else if reached || (!a.toast_visible && rect_is_empty(a.notice.rect)) {
            live_layer(LIVE_LAYER_TOAST, &a.screen, RECT_EMPTY);
        }
        return;
    }
    let msg = a.toast.as_str();
    live_layer_drawn(LIVE_LAYER_TOAST, a.toast_rect, |c, all| ui_toast_at(c, all, msg));
}

/// calls leave/enter and redraws fully (REFRESH_FULL)
pub fn app_switch(a: &mut App, s: ScreenId) {
    // (the C returns for a ScreenId out of range; a Rust ScreenId cannot be)
    live_reset();
    a.live_processed = 0;
    a.live_processed_ms = 0;
    (ops(a.scr).leave)(a);
    keyboard_holds_end(); // (the lift goes to the new screen: a held delete on the old one lets go now)
    a.prev_scr = a.scr;
    a.scr = s;
    a.toast_floor = 0; // the new screen sets its own
    (ops(s).enter)(a);
    app_invalidate_all(a, RefreshMode::Full); // a visible toast is overlaid again at the flush
}

/// Pressed-state feedback for a tapped control: invert + FAST refresh now.
pub fn app_press_feedback(a: &mut App, r: Rect) {
    canvas_invert(&mut a.screen, r);
    live_push_screen(a, r);
    display_present(&a.screen, r, RefreshMode::Fast);
}

/// The pressed look app_press_feedback gave `pressed`, taken off at the lift, before what the tap does
/// draws: the rect to refresh, empty when nothing was pressed or the surface was drawn again since
/// (its draw forgets the press).
pub fn app_press_release(a: &mut App, pressed: &mut Option<Rect>) -> Rect {
    match pressed.take() {
        Some(r) => {
            canvas_invert(&mut a.screen, r);
            r
        }
        None => RECT_EMPTY,
    }
}

/// typing paused this long: the keyboard's grayscale refresh
const KB_SETTLE_MS: u32 = 900;

/// A pen or finger event on an on-screen keyboard (keyboard.h), drawn and refreshed the one way every
/// keyboard is, so a key costs the same over any screen: only when the key changed something, only
/// the keyboard (never what is under it), and on the fast waveform, the text line first and on its
/// own, then the key. Returns keyboard_pointer's result (or, for a move of a contact holding delete,
/// keyboard_repeat's: Changed when it deleted again); on KB_DONE and KB_CANCEL nothing is drawn
/// (the caller closes it and draws what it covered). (kb is not inside a: a screen takes its
/// keyboard out of its state for the call, PORTING.md, The App.)
pub fn app_keyboard_pointer(a: &mut App, kb: &mut Keyboard, phase: i32, ev: &InEvent) -> KbResult {
    let (r, d) = app_keyboard_pointer_split(a, kb, phase, ev);
    app_invalidate(a, d, RefreshMode::Fast);
    r
}

/// app_keyboard_pointer up to and including the text line's own refresh and the pause's timer, but the key's
/// rect is returned (empty when there is nothing to refresh) instead of invalidated: for a screen that draws
/// and refreshes something of its own between the text and the key (Find's rows), which would otherwise
/// present with the key's rect already in the accumulator, one union from its rows down to the key, the
/// overlapping refresh this host exists to avoid. The caller invalidates the key's rect, on the fast waveform,
/// after its own flush; app_keyboard_pointer is this and that, byte for byte what it was.
pub fn app_keyboard_pointer_split(a: &mut App, kb: &mut Keyboard, phase: i32, ev: &InEvent,
) -> (KbResult, Rect) {
    let mut d = RECT_EMPTY;
    // (a hardware key's input, app_type_text: at the lift, instead of a key; its touch-down is a contact on no key,
    // which ends a hold as any new contact does, and never one on the text, which would move the caret)
    if phase == 0 && a.kb_inject.is_some() {
        kb.repeat_at = 0;
        kb.pressed_key = -1;
        kb.field_drag = false;
        return (KbResult::None, RECT_EMPTY);
    }
    let mut r = match a.kb_inject.take() {
        Some(input) if phase == 2 => keyboard_input(kb, &input, &mut d),
        _ => keyboard_pointer(kb, phase, ev.x, ev.y, ev.t_ms, &mut d),
    };
    if r == KbResult::Done || r == KbResult::Cancel {
        return (r, RECT_EMPTY);
    }
    // Delete held down: a sample of the contact on it (the pen resting there sends one at every digitizer
    // frame, a finger now and then) is a moment it may delete again, as the screen's tick is.
    if phase == 1 && r == KbResult::None && kb.repeat_at != 0 {
        r = keyboard_repeat(kb, ev.t_ms, display_fast_busy());
    }
    // A sample that changed nothing draws nothing: the pen resting on a key sends one at every
    // digitizer frame, and each whole keyboard drawn again is main-thread work the next key waits for.
    if rect_is_empty(d) && rect_is_empty(kb.field_dirty) {
        return (r, RECT_EMPTY);
    }
    kb_typed(a, kb);
    (r, d)
}

// What a key changed, drawn and refreshed (app_keyboard_pointer_split, and a held delete's repeats from the tick).
// Typing stays on the fast waveform: the text first (what the eye is on), then the key, each a refresh of its
// own. One over both would span the keyboard from the text line down to the key: on the Kindle several times the
// pixels to copy, and the next key's refreshes would overlap it and wait in the driver's queue for it to end
// (docs/INK-PERFORMANCE.md), every key behind the one before. One clean GL16 of the keyboard follows once typing
// pauses (app_keyboard_tick).
fn kb_typed(a: &mut App, kb: &mut Keyboard) {
    keyboard_draw(kb, &mut a.screen, Rect { x: 0, y: 0, w: a.w, h: a.h ,
        },
    );
    if !rect_is_empty(kb.field_dirty) {
        app_invalidate(a, kb.field_dirty, RefreshMode::Fast);
        app_flush(a);
    }
    // (the count pad's Done, whose label shows the count: on its own too, never one span with the text)
    if !rect_is_empty(kb.also_dirty) {
        app_invalidate(a, kb.also_dirty, RefreshMode::Fast);
        app_flush(a);
    }
    kb.settle_at = now_ms().wrapping_add(KB_SETTLE_MS);
    if kb.settle_at == 0 {
        kb.settle_at = 1;
    }
}

/// A key of a hardware keyboard on an on-screen keyboard (keyboard_press: the key with that code, as if
/// tapped), drawn and refreshed as a tap's is, the text line and then the count pad's Done each on its
/// own (no on-screen key was pressed, so none is refreshed). On KB_DONE and KB_CANCEL nothing is drawn.
/// It ends a held delete, as a second finger's key does: the digit it types is never the next repeat's.
pub fn app_keyboard_press(a: &mut App, kb: &mut Keyboard, code: u32) -> KbResult {
    kb.repeat_at = 0;
    let mut d = RECT_EMPTY;
    let r = keyboard_press(kb, code, &mut d);
    if r == KbResult::Done || r == KbResult::Cancel || r == KbResult::None {
        return r;
    }
    // (Changed, or Moved: the caret's line drawn and refreshed as the text's)
    keyboard_draw(kb, &mut a.screen, Rect { x: 0, y: 0, w: a.w, h: a.h });
    for dirty in [kb.field_dirty, kb.also_dirty, d] {
        if !rect_is_empty(dirty) {
            app_invalidate(a, dirty, RefreshMode::Fast);
            app_flush(a);
        }
    }
    kb.settle_at = now_ms().wrapping_add(KB_SETTLE_MS).max(1);
    r
}

/// From the screen's tick while its keyboard is up: delete held down deletes again when it is due
/// (keyboard_repeat), drawn and refreshed as a key's typing is, the text line and then the count pad's Done each
/// on its own (the key, pressed all along, is not refreshed), and then KbResult::Changed, for a screen that
/// follows the text (Find's rows); else, once typing pauses, one grayscale refresh of the keyboard cleans up what
/// the fast ones left, and None. (The main loop goes round at least every 30 ms, its input poll's timeout,
/// so a repeat 65 ms apart needs no wake of its own; with no hold this is the one test of repeat_at.)
pub fn app_keyboard_tick(a: &mut App, kb: &mut Keyboard, now: u32) -> KbResult {
    if kb.repeat_at != 0 {
        // (Changed for delete's, Moved for ◀'s and ▶'s: the caret moved, the text as it was)
        let r = keyboard_repeat(kb, now, display_fast_busy());
        if r == KbResult::Changed || r == KbResult::Moved {
            kb_typed(a, kb);
            return r;
        }
    }
    if kb.settle_at == 0 || (now.wrapping_sub(kb.settle_at) as i32) < 0 {
        return KbResult::None;
    }
    kb.settle_at = 0;
    if kb.open && kb.pressed_key < 0 {
        app_invalidate(a, kb.area, RefreshMode::Ui); // (not under a press)
    }
    KbResult::None
}

// The runner's annotate.* keys: "role", the open page's paper ("document" for a page KOReader sent
// or an imported one, "template", "" for any other); "luma", the open page as the editor holds it
// now, its mean (255: white, as a picture still being made draws); the rest are annotate_probe's,
// for the page open in the editor.
fn annotate_app_probe(a: &mut App, key: &str, out: &mut String, cap: usize) -> i32 {
    let mut v = String::new();
    if key == "role" {
        let k = PaperKind(if editor_probe(a, "paper", &mut v, 32) == 0 { strtoul_u32(&v) } else { 0 });
        csnprintf!(out, cap, "%s", if paper_is_document(k) { "document" } else if paper_is_image(k) { "template" } else { "" });
        return 0;
    }
    if key == "luma" {
        let c = editor_page_canvas(a);
        let mut sum: u64 = 0;
        let np: u64 = c.map_or(0, |c| c.w as u64 * c.h as u64);
        if let Some(c) = c {
            for y in 0..c.h {
                for x in 0..c.w {
                    sum += c.px[y as usize * c.stride as usize + x as usize] as u64;
                }
            }
        }
        match (sum + np / 2).checked_div(np) {
            Some(mean) => csnprintf!(out, cap, "%d", mean as i32),
            None => csnprintf!(out, cap, "%s", ""),
        };
        return 0;
    }
    let page = if editor_probe(a, "page", &mut v, 32) == 0 { atoi(&v) } else { 0 };
    annotate_probe(key, page - 1, out, cap)
}

/// out += piece, as snprintf(out + strlen(out), cap - strlen(out), "%s", piece) appends it: cut so
/// out keeps at most cap - 1 bytes.
fn append_cut(out: &mut String, cap: usize, piece: &str) {
    if cap == 0 || out.len() >= cap - 1 {
        return;
    }
    let room = cap - 1 - out.len();
    out.push_str(cut_at_bytes(piece, room));
}

/// The headless runner's `expect` checks read the app's state through this (tests only; cheap).
/// Keys: "screen" (library|editor|penset|calib|share), "toast" and "toast.rect" (the visible toast
/// and its rect, "" if none), "notice" and "notice.key" (the staying notice's text and key on the panel), "asleep" (0|1), "asleep.mode", "host", "rotation" (0..3), "size"
/// (WxH), "language", "bg.busy", "bg.children", "settings.<settings.ini key>" for a few and for
/// every tool's pen feel ("settings.ballpoint.size_max_mm"), "light.intensity", "light.warmth",
/// and "editor.*", "library.*", "penset.*" (below), "annotate.*" (app.c) and "templates.*" (the
/// open Templates sheet: New notebook's on the Library, else the editor's); docs/SIM.md has them
/// all. 0 and the value in out, -1 for an unknown key. (The C finds the App through g_probe_app; the
/// runner passes it here: sim_sdl.rs's probe point is handed the App by mainloop.rs.)
pub fn app_probe(a: &mut App, key: &str, out: &mut String, cap: usize) -> i32 {
    const K_NAMES: [&str; ScreenId::COUNT] = ["library", "editor", "penset", "calib", "share"];
    if cap == 0 {
        return -1;
    }
    if let Some(k) = key.strip_prefix("annotate.") {
        return annotate_app_probe(a, k, out, cap);
    }
    if let Some(k) = key.strip_prefix("status.") {
        return status_probe(a, k, out, cap);
    }
    if key.starts_with("fshape.") || key.starts_with("pen.") { return editor_probe(a, key, out, cap); }
    if let Some(k) = key.strip_prefix("editor.") {
        return editor_probe(a, k, out, cap);
    }
    // The open notebook's last_page + 1: the page notebook.ini reopens it on (0 with none open).
    if key == "notebook.last_page" {
        return editor_probe(a, "notebook_last_page", out, cap);
    }
    if let Some(k) = key.strip_prefix("library.") {
        return library_probe(a, k, out, cap);
    }
    // The storage the picker browses (the runner's, docs/SIM.md): storage.exists.PATH, storage.size.PATH (-1
    // without it) and storage.names.DIR (sorted, joined by |), PATH relative to the storage root, no link followed.
    if let Some(rest) = key.strip_prefix("storage.") {
        let Some(root) = crate::pdfimport::storage_root() else { return -1 };
        let (what, p) = rest.split_once('.').unwrap_or((rest, ""));
        let full = format!("{}/{}", root.trim_end_matches('/'), p);
        match what {
            "exists" => {
                csnprintf!(out, cap, "%d", std::fs::symlink_metadata(&full).is_ok() as i32);
            }
            "size" => {
                csnprintf!(out, cap, "%d", std::fs::symlink_metadata(&full).map_or(-1, |m| m.len() as i64) as i32);
            }
            "names" => {
                let mut v: Vec<String> = std::fs::read_dir(&full).map(|d| d.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect()).unwrap_or_default();
                v.sort();
                csnprintf!(out, cap, "%s", &v.join("|"));
            }
            _ => return -1,
        }
        return 0;
    }
    // The file picker (the Library's mode, browser.rs): files.*
    if key.starts_with("files.") {
        return library_probe(a, key, out, cap);
    }
    if let Some(k) = key.strip_prefix("penset.") {
        return pensettings_probe(a, k, out, cap);
    }
    // The Templates sheet (tplsheet.h): New notebook's on the Library, else the editor's.
    if key.starts_with("templates.") {
        return if a.scr == ScreenId::Library { library_probe(a, key, out, cap) } else { editor_probe(a, key, out, cap) };
    }
    // Settings, under their settings.ini names.
    let st = &a.settings;
    let nums: [(&str, i32); 38] = [
        ("settings.invert", st.invert as i32),
        ("settings.mirror_pdf", st.mirror_pdf as i32),
        ("settings.follow_rotation", st.follow_rotation as i32),
        ("settings.pinch_zoom", st.pinch_zoom as i32),
        ("settings.side_button", st.side_button),
        ("settings.sleep_mode", st.sleep_mode),
        ("settings.sleep_minutes", st.sleep_minutes),
        ("settings.pen.valid", st.pen_calib.valid as i32),
        ("settings.touch.valid", st.touch_calib.valid as i32),
        ("settings.eraser_mode", st.eraser_mode),
        ("settings.white", st.white as i32),
        ("settings.shortcuts_enabled", st.shortcuts_enabled as i32),
        ("settings.controller", st.controller as i32),
        ("settings.updates", st.updates),
        ("settings.status_on", st.status_on as i32),
        ("settings.status_mode", st.status_mode),
        ("settings.shapes", st.shapes as i32),
        ("settings.shape_hold_ms", st.shape_hold_ms),
        ("settings.shape_quads", st.shape_quads),
        ("settings.shape_round", st.shape_round),
        ("settings.shape_links", st.shape_links as i32),
        ("settings.shape_min_mm", st.shape_min_mm),
        ("settings.whatsnew_hide", st.whatsnew_hide as i32),
        ("settings.shape_corner", st.shape_corner),
        ("settings.status_size", st.status_size),
        ("settings.status_corner", st.status_corner),
        ("settings.rotation_lock", st.rotation_lock),
        ("settings.gestures", st.gestures as i32),
        ("settings.landscape_fit", st.landscape_fit as i32),
        ("settings.gesture_new_page", st.gesture_new_page as i32),
        ("settings.hints", st.hints),
        ("settings.link_areas", st.link_areas as i32),
        ("shortcuts.code", a.last_shortcut_code),
        ("native_pin", a.native_pin as i32),
        ("input.yielded", a.input_yielded as i32),
        ("sleep.wallpaper", !a.koreader_wallpaper.px.is_empty() as i32,
        ),
        ("bg.busy", bg_busy() as i32), // a background job (an export, a compaction) runs or waits
        ("bg.children", bg_children()), // background processes running now (two at most: a second for page previews)
    ];
    for (k, v) in nums {
        if key == k {
            csnprintf!(out, cap, "%d", v);
            return 0;
        }
    }
    if key == "light.intensity" || key == "light.warmth" {
        let (mut li, mut lw) = (0, 0);
        light_get(&mut li, &mut lw);
        csnprintf!(out, cap, "%d", if key == "light.intensity" { li } else { lw });
        return 0;
    }
    // A tool's pen feel, as settings.ini writes it: "settings.ballpoint.size_max_mm".
    let tool = key.strip_prefix("settings.");
    let dot = tool.and_then(|t| t.find('.'));
    if let (Some(tool), Some(dot)) = (tool, dot) {
        for t in 0..TOOL_COUNT {
            let Some(name) = app_probe_tool(ToolKind(t as i32)) else { continue ;
            };
            if name.len() != dot || !tool.starts_with(name) {
                continue;
            }
            for pi in tool_param_table() {
                if &tool[dot + 1..] == pi.key {
                    csnprintf!(out, cap, "%.4f", a.settings.tools[t].get(pi.field));
                    return 0;
                }
            }
        }
    }
    let st = &a.settings;
    // What's new: the version whose notes were last shown or due, as settings.ini has it.
    if key == "settings.whatsnew_seen" {
        csnprintf!(out, cap, "%s", st.whatsnew_seen.as_str());
        return 0;
    }
    if key == "screen" {
        csnprintf!(out, cap, "%s", K_NAMES[a.scr as usize]);
    } else if key == "toast" {
        csnprintf!(out, cap, "%s", if a.toast_visible { a.toast.as_str() } else { "" });
    } else if key == "notice" || key == "notice.key" {
        let shown = a.notice.shown.filter(|_| !rect_is_empty(a.notice.rect));
        let v = match shown {
            None => "",
            Some(k) if key == "notice.key" => k.name(),
            Some(_) => a.notice.shown_text.as_str(),
        };
        csnprintf!(out, cap, "%s", v);
    } else if key == "toast.rect" {
        let r = a.toast_rect;
        if a.toast_visible {
            csnprintf!(out, cap, "%d %d %d %d", r.x, r.y, r.w, r.h);
        } else {
            csnprintf!(out, cap, "%s", "");
        }
    } else if key == "asleep" {
        csnprintf!(out, cap, "%d", if a.asleep { 1 } else { 0 });
    } else if key == "asleep.mode" {
        if a.asleep {
            csnprintf!(out, cap, "%d", with(|s| s.sleep_shown));
        } else {
            csnprintf!(out, cap, "%s", "");
        }
    } else if key == "host" {
        csnprintf!(out, cap, "%s", app_host());
    } else if key=="wifi.open" {
        csnprintf!(out,cap,"%d",crate::share::share_wifi_open(a) as i32);
    } else if key=="wifi.busy" {
        csnprintf!(out,cap,"%d",wifi_busy() as i32);
    } else if key=="wifi.failed" {
        csnprintf!(out,cap,"%d",wifi_failed() as i32);
    } else if key=="wifi.state" {
        csnprintf!(out,cap,"%s",match wifi_status(){None=>"unknown",Some(st) if !st.wireless||!st.enabled=>"off",Some(st) if st.connected==Some(true)=>"connected",Some(_)=>"on"});
    } else if key == "live.on" {
        csnprintf!(out, cap, "%d", if live_on() { 1 } else { 0 });
    } else if key == "live.viewers" {
        csnprintf!(out, cap, "%d", live_viewers());
    } else if key == "live.pairing" {
        csnprintf!(out, cap, "%d", if share_live_card_up(a) { 1 } else { 0 });
    } else if key == "live.panel" || key == "live.toast" {
        let (r, _) = live_layer_now(if key == "live.panel" { LIVE_LAYER_PANEL } else { LIVE_LAYER_TOAST });
        if rect_is_empty(r) {
            csnprintf!(out, cap, "%s", "");
        } else {
            csnprintf!(out, cap, "%d %d %d %d", r.x, r.y, r.w, r.h);
        }
    } else if key == "live.chip" {
        let r = a.chip_at;
        if live_on() && !rect_is_empty(r) {
            csnprintf!(out, cap, "%d %d %d %d", r.x, r.y, r.w, r.h);
        } else {
            csnprintf!(out, cap, "%s", "");
        }
    } else if key == "update.requested" {
        // What "Update now" asked the launcher for ("preview" or "stable"), empty for nothing.
        let text = read_file(&update_request_path());
        let bytes = text.as_deref().unwrap_or(&[]);
        let bytes = &bytes[..bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len())]; // (%s ends at a NUL)
        let s = match std::str::from_utf8(bytes) {
            Ok(s) => s,
            Err(e) => std::str::from_utf8(&bytes[..e.valid_up_to()]).unwrap_or(""),
        };
        csnprintf!(out, cap, "%s", s);
        while out.ends_with('\n') {
            out.pop();
        }
    } else if let Some(action) = key.strip_prefix("pads.") {
        // The controller buttons for an action, as the files name them: "pads.undo" -> "East" ("" none).
        let mut codes = [0i32; 4];
        let n = shortcuts_pads_for(&a.shortcuts, ActionSrc::Pad,
            shortcut_action(action), &mut codes,
        );
        out.clear();
        let mut i = 0;
        while i < n && i < 4 {
            let mut name = String::new();
            shortcut_pad_name(codes[i as usize], &mut name, 16);
            append_cut(out, cap, &cformat!("%s%s", if i != 0 { "," } else { "" }, &name),
            );
            i += 1;
        }
    } else if key == "shortcuts.profiles" {
        // The device profiles in root/shortcuts as their files are now, by slug: "slug:on slug:off" (off: the
        // device uses the default, its file kept), "" for none.
        let list: Vec<String> = shortcut_profiles_load(&shortcut_profiles_dir())
            .iter()
            .map(|p| format!("{}:{}", p.slug, if p.on { "on" } else { "off" }))
            .collect();
        csnprintf!(out, cap, "%s", &list.join(" "));
    } else if key == "gestures.shape_count" {
        csnprintf!(out, cap, "%d", gesturemap_shapes(&a.gestures).len() as i32);
    } else if key == "gestures.refused" {
        csnprintf!(out, cap, "%d", a.gestures.refused);
    } else if let Some(action) = key.strip_prefix("gestures.shape.") {
        out.clear();
        if let Some(g) = gesturemap_shape_of(&a.gestures, shortcut_action(action)) {
            csnprintf!(out, cap, "%d", g.n as i32);
        }
    } else if let Some(action) = key.strip_prefix("gestures.") {
        let act = shortcut_action(action);
        out.clear();
        if let Some(k) =gesturemap_key_for(&a.gestures, act , ActionSrc::Touch){
            gesturemap_key_name(k, out);
                }
            else if gesturemap_shape_of(&a.gestures, act).is_some()
            || gesturemap_shape_newer(&a.gestures, act)
        {
            out.push_str("shape");
        } else if let Some(k) = gesturemap_key_for(&a.gestures, act, ActionSrc::Pen) {
            gesturemap_key_name(k, out);
        }
    } else if key == "rotation" {
        csnprintf!(out, cap, "%d", a.rotation);
    } else if key == "size" {
        csnprintf!(out, cap, "%dx%d", a.w, a.h);
    } else if key == "settings.pen.panel" || key == "settings.touch.panel" {
        let pen = key == "settings.pen.panel";
        let pw = if pen { st.pen_panel_w } else { st.touch_panel_w };
        let ph = if pen { st.pen_panel_h } else { st.touch_panel_h };
        if pw > 0 {
            csnprintf!(out, cap, "%dx%d", pw, ph);
        } else {
            csnprintf!(out, cap, "%s", "");
        }
    } else if key == "settings.theme" {
        csnprintf!(out, cap, "%s", if st.theme == UiTheme::Classic as i32 { "classic" } else { "gothic" });
    } else if key == "settings.font" {
        // (terminess for a font this version does not have: settings.kept.font has the file's value)
        if st.font_file.is_empty() {
            csnprintf!(out, cap, "%s", FONT_FAMILIES[clampi(st.font, 0, FONT_FAMILIES.len() as i32 - 1) as usize].id);
        } else {
            csnprintf!(out, cap, "file:%s", st.font_file.as_str());
        }
    } else if key == "settings.font_bold" {
        csnprintf!(out, cap, "%s", st.font_bold.as_str());
    } else if key == "font" {
        // Classic's font in use: a family's id, or file:<path> while a font of the owner's own draws
        if font_own() && crate::userfonts::userfonts_in_use(st.font_file.as_str()) {
            csnprintf!(out, cap, "file:%s", st.font_file.as_str());
        } else {
            csnprintf!(out, cap, "%s", FONT_FAMILIES[font_family()].id);
        }
    } else if key == "font.guard" {
        csnprintf!(out, cap, "%s", &crate::userfonts::userfonts_guard_probe()); // the crash guard's marks
    } else if key == "font.own.bytes" {
        csnprintf!(out, cap, "%d", font_own_bytes() as i32); // the font files the UI holds
    } else if key == "font.own.drawn" {
        csnprintf!(out, cap, "%d", (font_own_drawn() > 0) as i32); // a custom font drew a glyph
    } else if let Some(k) = key.strip_prefix("settings.kept.") {
        csnprintf!(out, cap, "%s", &String::from_utf8_lossy(&settings_kept(k).unwrap_or_default()).into_owned());
    } else if key == "settings.panel_tools" {
        settings_pins_text(st, out, cap); // the pins, as saved
    } else if key == "settings.language" {
        csnprintf!(out, cap, "%s", &st.language); // "auto" for one it lacks
    } else if key == "language" {
        csnprintf!(out, cap, "%s", i18n_current()); // the language in use
    } else {
        return -1;
    }
    0
}

/// A custom music layout for the probes: "5: treble tab6 bass" (systems per page, then the parts
/// top to bottom); "" for None.
pub fn app_probe_music(m: Option<&MusicLayout>, out: &mut String, cap: usize) {
    // The parts by short names (src/paper.h order); a part missing here shows as its number.
    const PARTS: [Option<&str>; PART_COUNT as usize] = {
        let mut p: [Option<&str>; PART_COUNT as usize] = [None; PART_COUNT as usize];
        p[PART_TREBLE.0 as usize] = Some("treble");
        p[PART_BASS.0 as usize] = Some("bass");
        p[PART_ALTO.0 as usize] = Some("alto");
        p[PART_TENOR.0 as usize] = Some("tenor");
        p[PART_PERC.0 as usize] = Some("drums");
        p[PART_STAFF.0 as usize] = Some("plain");
        p[PART_TAB4.0 as usize] = Some("tab4");
        p[PART_TAB5.0 as usize] = Some("tab5");
        p[PART_TAB6.0 as usize] = Some("tab6");
        p[PART_TAB7.0 as usize] = Some("tab7");
        p[PART_TAB8.0 as usize] = Some("tab8");
        p[PART_LYRICS.0 as usize] = Some("lyrics");
        p
    };
    if cap == 0 {
        return;
    }
    out.clear();
    let Some(m) = m else { return };
    let mut n = csnprintf!(out, cap, "%d:", m.systems) as usize;
    let mut i = 0;
    while i < m.n && (i as usize) < MUSIC_MAX_PARTS && n < cap {
        let p = m.part[i as usize].0;
        let name = if p > 0 && p < PART_COUNT { PARTS[p as usize] } else { None };
        let piece = match name {
            Some(name) => cformat!(" %s", name),
            None => cformat!(" %d", p),
        };
        append_cut(out, cap, &piece);
        n += piece.len();
        i += 1;
    }
}
/// settings.ini's name for a tool ("pencil", "mech", ...), None past the list: how the probes name
/// tools. tests/test_probe.c holds it to what settings_save writes.
pub fn app_probe_tool(t: ToolKind) -> Option<&'static str> {
    const K: [&str; TOOL_COUNT] =
        ["pencil", "mech", "ballpoint", "eraser", "highlighter", "fountain", "fineliner", "marker", "charcoal", "calligraphy",
    ];
    if t.0 >= 0 && (t.0 as usize) < TOOL_COUNT {
        Some(K[t.0 as usize])
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_music_writes_as_snprintf_would() {
        let mut out = String::from("old");
        app_probe_music(None, &mut out, 64);
        assert_eq!(out, "");
        let mut m = MusicLayout { systems: 5, n: 3, ..Default::default() };
        m.part[0] = PART_TREBLE;
        m.part[1] = PART_TAB6;
        m.part[2] = PART_BASS;
        app_probe_music(Some(&m), &mut out, 64);
        assert_eq!(out, "5: treble tab6 bass");
        // A part with no short name shows as its number.
        m.part[1] = PART_NONE;
        m.part[2] = MusicPart(13);
        app_probe_music(Some(&m), &mut out, 64);
        assert_eq!(out, "5: treble 0 13");
        // Cut as the C's buffer cuts it: "5:" then one byte of " treble", then nothing more.
        app_probe_music(Some(&m), &mut out, 4);
        assert_eq!(out, "5: ");
    }

    #[test]
    fn numbers_read_as_the_c_library_reads_them() {
        assert_eq!(strtoul_u32("1342177281"), 1342177281);
        assert_eq!(strtoul_u32("  12abc"), 12);
        assert_eq!(strtoul_u32("-1"), u32::MAX);
        assert_eq!(strtoul_u32("99999999999999999999"), u32::MAX);
        assert_eq!(strtoul_u32(""), 0);
        assert_eq!(atoi(" -12x"), -12);
        assert_eq!(atoi("+7"), 7);
        assert_eq!(atoi("x"), 0);
    }

    #[test]
    fn the_dirty_rect_keeps_the_strongest_mode() {
        let mut a = App { w: 100, h: 200, ..Default::default() };
        app_invalidate(&mut a, Rect { x: 10, y: 10, w: 5, h: 5 ,
            }, RefreshMode::Ui,
        );
        app_invalidate(&mut a, Rect { x: 50, y: 60, w: 5, h: 5 ,
            }, RefreshMode::Fast,
        );
        assert_eq!(a.dirty_mode, RefreshMode::Ui);
        assert_eq!(a.dirty, Rect { x: 10, y: 10, w: 45, h: 55 });
        // Off the screen: nothing.
        app_invalidate(&mut a, Rect { x: 300, y: 0, w: 5, h: 5 ,
            }, RefreshMode::Full,
        );
        assert_eq!(a.dirty_mode, RefreshMode::Ui);
        app_invalidate_all(&mut a, RefreshMode::Full);
        assert_eq!((a.dirty, a.dirty_mode), (Rect { x: 0, y: 0, w: 100, h: 200 }, RefreshMode::Full));
        // With nothing dirty, the first mode asked for is the mode, however weak.
        a.dirty = RECT_EMPTY;
        app_invalidate(&mut a, Rect { x: 0, y: 0, w: 1, h: 1 ,
            }, RefreshMode::Fast,
        );
        assert_eq!(a.dirty_mode, RefreshMode::Fast);
    }

    #[test]
    fn appending_cuts_at_the_buffer() {
        let mut out = String::from("East");
        append_cut(&mut out, 8, ",South");
        assert_eq!(out, "East,So");
        append_cut(&mut out, 8, ",West");
        assert_eq!(out, "East,So");
    }

    /// A hardware keyboard's key on a count pad (app_keyboard_press) ends the on-screen delete held there, as a
    /// second finger's key does: the digit it typed stays, the next repeat never comes.
    #[test]
    fn a_hardware_key_ends_a_held_delete() {
        assert_eq!(font_init(), 0);
        ui_init(300.0);
        i18n_set("en");
        let mut a = App { w: 1860, h: 2480, dpi: 300.0, ..Default::default() };
        assert_eq!(canvas_init(&mut a.screen, a.w, a.h), 0);
        #[cfg(feature = "sim")]
        {
            let (mut w, mut h) = (a.w, a.h);
            crate::sim_sdl::display_set_orientation(0, &mut w, &mut h);
        }
        let mut kb = Keyboard::default();
        keyboard_open_count(&mut kb, "Pages to add", 999, |n| format!("Add {n}"));
        kb.area = keyboard_area(&kb, Rect { x: 0, y: 0, w: a.w, h: a.h });
        for c in ['3', '4', '5'] {
            assert_eq!(app_keyboard_press(&mut a, &mut kb, c as u32), KbResult::Changed);
        }
        let mut r = RECT_EMPTY;
        assert!(keyboard_key_rect(&kb, KB_KEY_BKSP, &mut r));
        let (x, y) = ((r.x + r.w / 2) as f32, (r.y + r.h / 2) as f32);
        app_keyboard_pointer(&mut a, &mut kb, 0, &InEvent { x, y, t_ms: 1000, ..Default::default() });
        assert_eq!(kb.text.as_str(), "34");
        assert_ne!(kb.repeat_at, 0, "delete held");
        assert_eq!(app_keyboard_press(&mut a, &mut kb, '9' as u32), KbResult::Changed);
        assert_eq!((kb.text.as_str(), kb.repeat_at), ("349", 0));
        app_keyboard_tick(&mut a, &mut kb, 1000 + KB_HOLD_MS + 4 * KB_REPEAT_MS);
        app_keyboard_pointer(&mut a, &mut kb, 1, &InEvent { x, y, t_ms: 1000 + KB_HOLD_MS + 5 * KB_REPEAT_MS, ..Default::default() });
        assert_eq!(kb.text.as_str(), "349", "the finger still on del deletes nothing more");
    }

    // ---- the staying notice ----

    fn notice_app() -> App {
        assert_eq!(font_init(), 0);
        ui_init(300.0);
        i18n_set("en");
        let mut a = App { w: 1860, h: 2480, dpi: 300.0, ..Default::default() };
        assert_eq!(canvas_init(&mut a.screen, a.w, a.h), 0);
        #[cfg(feature = "sim")]
        {
            let (mut w, mut h) = (a.w, a.h);
            crate::sim_sdl::display_set_orientation(0, &mut w, &mut h);
        }
        canvas_fill(&mut a.screen, Rect { x: 0, y: 0, w: a.w, h: a.h }, 255);
        canvas_fill(&mut a.screen, Rect { x: 100, y: 2000, w: 1600, h: 300 }, 90); // something under it
        a
    }

    /// The app's clock at `t`, then a flush, as the main loop's each turn.
    fn flush_at(a: &mut App, t: u32) {
        NOTICE_CLOCK.with(|c| c.set(Some(t)));
        app_flush(a);
    }

    fn probe(a: &mut App, key: &str) -> String {
        let mut out = String::new();
        assert_eq!(app_probe(a, key, &mut out, 256), 0, "{key}");
        out
    }

    /// Shown at the next flush; drawn again only for a new text, at most once every 2 s; none of it counted
    /// toward the Full, and App.screen holds none of it.
    #[test]
    fn the_notice_redraws_for_a_new_text_at_most_every_2_s() {
        let mut a = notice_app();
        let before = a.screen.px.clone();
        app_notice(&mut a, NoticeKey::Book, "Adding book pages: 1 of 400");
        flush_at(&mut a, 10_000);
        assert_eq!(probe(&mut a, "notice"), "Adding book pages: 1 of 400");
        assert_eq!(probe(&mut a, "notice.key"), "book");
        assert_eq!(a.notice.presents, 1);
        assert_eq!(a.notice.rect, toast_place_for(&a, "Adding book pages: 1 of 400"));
        // the same text: nothing, however long
        flush_at(&mut a, 60_000);
        assert_eq!(a.notice.presents, 1);
        // a new text waits for 2 s from the last drawing
        app_notice(&mut a, NoticeKey::Book, "Adding book pages: 2 of 400");
        flush_at(&mut a, 60_500);
        assert_eq!((a.notice.presents, probe(&mut a, "notice").as_str()), (2, "Adding book pages: 2 of 400"));
        app_notice(&mut a, NoticeKey::Book, "Adding book pages: 3 of 400");
        for t in [60_600, 61_000, 62_499] {
            flush_at(&mut a, t);
        }
        assert_eq!((a.notice.presents, probe(&mut a, "notice").as_str()), (2, "Adding book pages: 2 of 400"));
        flush_at(&mut a, 62_500);
        assert_eq!((a.notice.presents, probe(&mut a, "notice").as_str()), (3, "Adding book pages: 3 of 400"));
        assert_eq!(a.ui_refreshes_since_full, 0, "none of it counted");
        // cleared: its rect from the screen at once, and the screen's bytes as they were
        app_notice_clear(&mut a, NoticeKey::Book);
        flush_at(&mut a, 62_600);
        assert_eq!((a.notice.presents, probe(&mut a, "notice").as_str()), (4, ""));
        assert!(rect_is_empty(a.notice.rect) && rect_is_empty(app_overlay_rect(&a)));
        assert!(a.screen.px == before, "App.screen unchanged");
        assert_eq!(a.ui_refreshes_since_full, 0);
        NOTICE_CLOCK.with(|c| c.set(None));
    }

    /// A toast takes the notice's place for its time, its own refresh repainting where the notice was; once it
    /// goes the notice comes back in the refresh that takes the toast down.
    #[test]
    fn a_toast_goes_over_the_notice_and_the_notice_comes_back() {
        let mut a = notice_app();
        let before = a.screen.px.clone();
        app_notice(&mut a, NoticeKey::Book, "Adding book pages: 5 of 40");
        flush_at(&mut a, 1_000);
        let at = a.notice.rect;
        assert!(!rect_is_empty(at));
        app_toast(&mut a, "Saved", 2000);
        assert!(rect_contains_rect(with(|s| s.self_dirty), at), "the toast's refresh repaints the notice's place");
        flush_at(&mut a, 1_100);
        assert_eq!((probe(&mut a, "notice").as_str(), probe(&mut a, "toast").as_str()), ("", "Saved"));
        // a new text while the toast shows: nothing drawn
        app_notice(&mut a, NoticeKey::Book, "Adding book pages: 9 of 40");
        let presents = a.notice.presents;
        flush_at(&mut a, 4_000);
        assert_eq!(a.notice.presents, presents);
        // the toast goes: the notice rides the refresh that takes it down, with its new text
        toast_hide(&mut a);
        flush_at(&mut a, 4_100);
        assert_eq!(probe(&mut a, "notice"), "Adding book pages: 9 of 40");
        assert_eq!((a.notice.presents, a.notice.rect), (presents, toast_place_for(&a, "Adding book pages: 9 of 40")));
        assert!(a.screen.px == before, "App.screen holds neither");
        NOTICE_CLOCK.with(|c| c.set(None));
    }

    /// Each key on its screens only (Book everywhere, Import on the Library, the asking screen's own there),
    /// one at a time: the screen in front's own first, then Book, then Import.
    #[test]
    fn a_notice_shows_where_its_key_belongs_and_the_front_screens_wins() {
        let mut a = notice_app();
        a.scr = ScreenId::Editor;
        app_notice(&mut a, NoticeKey::Import, "Importing: 3 of 90 pages");
        flush_at(&mut a, 1_000);
        assert_eq!((probe(&mut a, "notice").as_str(), a.notice.presents), ("", 0), "no Import over the editor");
        app_notice(&mut a, NoticeKey::Book, "Adding book pages: 1 of 3");
        flush_at(&mut a, 1_100);
        assert_eq!(probe(&mut a, "notice.key"), "book");
        // the Library in front: Book still before Import
        a.scr = ScreenId::Library;
        flush_at(&mut a, 4_000);
        assert_eq!(probe(&mut a, "notice.key"), "book");
        app_notice_clear(&mut a, NoticeKey::Book);
        flush_at(&mut a, 4_100);
        assert_eq!(probe(&mut a, "notice.key"), "import", "the next key at once");
        // a key of the screen in front (the Font sheet's check) goes first, and only there
        app_notice(&mut a, NoticeKey::FontCheck, "Checking 3 fonts...");
        flush_at(&mut a, 4_200);
        assert_eq!(probe(&mut a, "notice"), "Checking 3 fonts...");
        a.scr = ScreenId::Editor;
        assert_eq!(notice_pick(&a), None);
        a.scr = ScreenId::Library;
        app_notice_clear(&mut a, NoticeKey::FontCheck);
        app_notice_clear(&mut a, NoticeKey::Import);
        flush_at(&mut a, 4_300);
        assert_eq!(probe(&mut a, "notice"), "");
        assert_eq!(a.ui_refreshes_since_full, 0);
        NOTICE_CLOCK.with(|c| c.set(None));
    }

    /// A 400-page book (its progress made by book_progress every 600 ms, 1.5 s a page) and a 10-minute import
    /// with the Library in front, the main loop flushing every 30 ms: no Full, and at most one redraw every 2 s
    /// (the toast it replaced counted every 600 ms: a Full about every 18 s).
    #[test]
    fn a_long_book_and_a_long_import_make_no_full_refresh() {
        let mut a = notice_app();
        a.scr = ScreenId::Editor;
        a.book.total = 400;
        let mut t = 100_000u32;
        let end = t + 400 * 1500;
        while t < end {
            a.book.added = ((t - 100_000) / 1500) as i32;
            book_progress(&mut a, t);
            flush_at(&mut a, t);
            assert_eq!(a.ui_refreshes_since_full, 0);
            t += 30;
        }
        let last = probe(&mut a, "notice"); // (the newest text, or the one 2 s before it)
        assert!(["Adding book pages: 398 of 400", "Adding book pages: 399 of 400"].contains(&last.as_str()), "{last}");
        let book = a.notice.presents;
        assert!(book > 200 && book <= 400 * 1500 / NOTICE_EVERY_MS + 1, "{book}");
        app_notice_clear(&mut a, NoticeKey::Book);
        flush_at(&mut a, t);
        a.scr = ScreenId::Library;
        let (from, end) = (t, t + 10 * 60 * 1000);
        while t < end {
            let done = (t - from) / 700;
            app_notice(&mut a, NoticeKey::Import, &format!("Importing: {done} of 857 pages"));
            flush_at(&mut a, t);
            assert_eq!(a.ui_refreshes_since_full, 0);
            t += 30;
        }
        let import = a.notice.presents - book - 1;
        assert!(import > 250 && import <= 600_000 / NOTICE_EVERY_MS + 1, "{import}");
        NOTICE_CLOCK.with(|c| c.set(None));
    }
}
/// The existing screen lifecycle applied after a language or theme change.
pub fn app_relayout(a: &mut App) {
    app_redraw_mode(a, RefreshMode::Full);
}

/// Save and use the power-button sleep transition, including the current sleep-screen policy.
pub fn app_request_sleep(a: &mut App) {
    if a.asleep {
        return;
    }
    a.sleep_requested = true;
    app_sleep(a);
    request_sleep(a);
}

/// Enable shortcuts with the same defaults, readers and Bluetooth lifecycle as Settings.
pub fn app_toggle_shortcuts(a: &mut App) {
    let path = cformat!("%s/shortcuts.ini", &store_root());
    if !a.settings.shortcuts_enabled && shortcuts_write_defaults(&path) != 0 {
        app_toast(a, tr("Couldn't save shortcuts.ini"), 5000);
        return;
    }
    a.settings.shortcuts_enabled = !a.settings.shortcuts_enabled;
    settings_save(&a.settings);
    input_shortcuts_enable(a.settings.shortcuts_enabled);
    if !a.settings.shortcuts_enabled {
        bt_stop();
    } else if a.settings.controller && bt_present() {
        bt_start();
    }
    app_shortcuts_load(a);
}

pub fn app_toggle_controller(a: &mut App) {
    a.settings.controller = !a.settings.controller;
    settings_save(&a.settings);
    if a.settings.controller {
        bt_start();
    } else {
        bt_stop();
    }
}

/// Settings actions shared by the editor and Library. Callers apply their modal/stroke guards.
pub fn app_global_action(a: &mut App, action: i32, src: ActionSrc) -> bool {
    if !(SHORTCUT_INVERT..=SHORTCUT_UPDATE_CHANNEL).contains(&action) || action_sources(action) & src.mask() == 0 { return false; }
    let mut msg = String::new();
    let mut relayout = false;
    match action {
        SHORTCUT_INVERT => {
            a.settings.invert = !a.settings.invert;
            display_set_invert(a.settings.invert);
            live_set_invert(a.settings.invert);
            app_invalidate_all(a, RefreshMode::Full);
        }
        SHORTCUT_LIGHT_DOWN | SHORTCUT_LIGHT_UP | SHORTCUT_WARMTH_DOWN | SHORTCUT_WARMTH_UP => {
            let warm = action == SHORTCUT_WARMTH_DOWN || action == SHORTCUT_WARMTH_UP;
            let (light, warmth) = light_cached();
            let level = if warm { warmth } else { light };
            if level < 0 {
                // TRANSLATORS: light action before its cached level is known; toast, 1700 px at 44 px.
                app_toast(a, tr("Light not ready."), 2000);
                return true;
            }
            let up = action == SHORTCUT_LIGHT_UP || action == SHORTCUT_WARMTH_UP;
            let value = (level + if up { 1 } else { -1 }).clamp(0, LIGHT_MAX);
            if warm { light_set_warmth(value); } else { light_set_intensity(value); }
            // TRANSLATORS: cached light step and maximum; toast, 1700 px at 44 px bold.
            csnprintf!(&mut msg, 256, if warm { tr("Warmth %d of %d") } else { tr("Light %d of %d") }, value, LIGHT_MAX);
        }
        SHORTCUT_ROTATION_LOCK => {
            a.settings.rotation_lock = if a.settings.rotation_lock == 0 { 1 + a.rotation } else { 0 };
            if a.settings.rotation_lock == 0 && a.settings.follow_rotation {
                let caps = input_caps();
                if caps.has_orientation { a.pending_rotation = caps.orientation; }
            } else { a.pending_rotation = -1; }
            // TRANSLATORS: rotation action feedback, toast, 1700 px at 44 px bold.
            msg.push_str(if a.settings.rotation_lock != 0 { tr("Rotation locked.") } else { tr("Rotation follows Kindle.") });
        }
        SHORTCUT_PINCH_ZOOM => {
            a.settings.pinch_zoom = !a.settings.pinch_zoom;
            // TRANSLATORS: pinch zoom toggle feedback, toast, 1700 px at 44 px bold.
            msg.push_str(if a.settings.pinch_zoom { tr("Pinch zoom on.") } else { tr("Pinch zoom off.") });
        }
        SHORTCUT_THEME => {
            a.settings.theme = if a.settings.theme == UiTheme::Classic as i32 { UiTheme::Gothic as i32 } else { UiTheme::Classic as i32 };
            ui_set_theme(if a.settings.theme == UiTheme::Classic as i32 { UiTheme::Classic } else { UiTheme::Gothic });
            crate::userfonts::userfonts_apply(a); // (Classic's own font, if it has one; Gothic holds none)
            httpd_restyle(ui_theme() == UiTheme::Gothic); // (the share pages, if Live view is on)
            relayout = true;
        }
        SHORTCUT_SIDE_BUTTON => {
            a.settings.side_button = (a.settings.side_button + 1) % 4;
            // TRANSLATORS: pen button setting after cycling; toast, 1700 px at 44 px, up to two lines.
            const ROWS: [&str; 4] = [n_("Pen button while writing: nothing"), n_("Pen button while writing: erases"), n_("Pen button while writing: highlights"), n_("Pen button while writing: draws white")];
            msg.push_str(tr(ROWS[a.settings.side_button as usize]));
        }
        SHORTCUT_MIRROR_PDF => {
            a.settings.mirror_pdf = !a.settings.mirror_pdf;
            // TRANSLATORS: PDF mirror setting feedback; toast, 1700 px at 44 px bold.
            msg.push_str(if a.settings.mirror_pdf { tr("PDF copies on.") } else { tr("PDF copies off.") });
        }
        SHORTCUT_SLEEP_SCREEN => {
            if a.native_pin { return false; }
            // Keep the branch's existing KOReader wallpaper choice as well as the brief's three.
            a.settings.sleep_mode = (a.settings.sleep_mode + 1) % 4;
            // TRANSLATORS: Display sheet sleep screen choices, toast, 1700 px at 44 px, two lines.
            const ROWS: [&str; 4] = [n_("Sleep screen: the page I'm on (tap to change)"), n_("Sleep screen: a pinned page (tap to change)"), n_("Sleep screen: the Kindle's own screensaver (tap to change)"), n_("Sleep screen: KOReader wallpaper (tap to change)")];
            msg.push_str(tr(ROWS[a.settings.sleep_mode as usize]));
        }
        SHORTCUT_SLEEP => { app_request_sleep(a); return true; }
        SHORTCUT_GESTURES => {
            a.settings.gestures = !a.settings.gestures;
            // TRANSLATORS: gestures action feedback; off names the recovery menu. Toast, two 1700 px lines at 44 px.
            msg.push_str(if a.settings.gestures { tr("Gestures on.") } else { tr("Gestures off. Turn on in Settings > Gestures.") });
        }
        SHORTCUT_SHORTCUTS => {
            app_toggle_shortcuts(a);
            // TRANSLATORS: shortcut enablement feedback; toast, 1700 px at 44 px bold.
            msg.push_str(if a.settings.shortcuts_enabled { tr("Shortcuts on.") } else { tr("Shortcuts off.") });
        }
        SHORTCUT_BLUETOOTH_CONTROLLER => {
            app_toggle_controller(a);
            // TRANSLATORS: Bluetooth controller setting feedback; toast, 1700 px at 44 px bold.
            msg.push_str(if a.settings.controller { tr("Bluetooth controller on.") } else { tr("Bluetooth controller off.") });
        }
        SHORTCUT_LANGUAGE => {
            let current = i18n_find(i18n_current());
            let next = (current + 1).rem_euclid(i18n_language_count());
            let code = i18n_language_code(next).unwrap_or("en");
            a.settings.language.set(code);
            i18n_set(code);
            msg.push_str(i18n_language_name(next).unwrap_or("English"));
            relayout = true;
        }
        SHORTCUT_SLEEP_AFTER => {
            const STEPS: [i32; 8] = [0, 1, 2, 5, 10, 15, 30, 60];
            let at = STEPS.iter().enumerate().min_by_key(|(_, v)| (**v - a.settings.sleep_minutes).abs()).map_or(0, |(i, _)| i);
            a.settings.sleep_minutes = STEPS[(at + 1) % STEPS.len()];
            app_apply_sleep_policy(a);
            let value = if a.settings.sleep_minutes == 0 { tr("Never").to_string() } else { cformat!("%d min", a.settings.sleep_minutes) };
            // TRANSLATORS: sleep timeout after cycling, %s is its value; toast, 1700 px at 44 px bold.
            msg = cformat!(tr("Sleep after: %s"), &value);
        }
        SHORTCUT_UPDATE_CHANNEL => {
            a.settings.updates = (a.settings.updates + 1) % 3;
            // TRANSLATORS: update channels, used in a toast with Update channel: %s; 1700 px at 44 px.
            const CHANNELS: [&str; 3] = [n_("Automatic"), n_("Preview"), n_("Stable")];
            // TRANSLATORS: selected update channel, %s is its name; toast, 1700 px at 44 px bold.
            msg = cformat!(tr("Update channel: %s"), tr(CHANNELS[a.settings.updates as usize]));
        }
        _ => return false,
    }
    settings_save(&a.settings);
    if relayout { app_relayout(a); }
    if !msg.is_empty() { app_toast(a, &msg, 2000); }
    true
}
