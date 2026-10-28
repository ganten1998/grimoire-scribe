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
    pub w: i32,
    pub h: i32,
    pub dpi: f32,
    /// mirrors the panel
    pub screen: Canvas,
    pub settings: Settings,
    pub shortcuts: Shortcuts,
    /// what each touch gesture does: the built-ins and gestures.ini (app_gestures_load)
    pub gestures: GestureMap,
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
    /// promote to FULL every 30
    pub ui_refreshes_since_full: i32,
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
    /// pixels the toast covers, restored when it hides
    pub toast_under: Canvas,
    /// pixels Live view's chip covers, put back after each present
    pub chip_under: Canvas,
    /// toasts stay above this y (the keyboard, a sheet's buttons); 0 none.
    /// The screen sets it as it draws; app_switch clears it.
    pub toast_floor: i32,
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
}


// Screen module interface (each implemented in its own module, as in its own .c):
//   X_init(a) allocates state; X_free(a);
//   X_enter(a) on switch-in (draw everything into a.screen and invalidate);
//   X_leave(a) on switch-out; X_event(a, ev); X_tick(a, now).
// (The C's DECLARE_SCREEN declares these; the definitions are the screens'.)
pub use crate::calibrate::{calibrate_enter, calibrate_event, calibrate_free, calibrate_init, calibrate_leave, calibrate_tick};
pub use crate::editor::{editor_enter, editor_event, editor_free, editor_init, editor_leave, editor_tick};
pub use crate::library::{library_enter, library_event, library_free, library_init, library_leave, library_tick};
pub use crate::pensettings::{
    pensettings_enter, pensettings_event, pensettings_free, pensettings_init, pensettings_leave, pensettings_tick,
};
pub use crate::share::{share_enter, share_event, share_free, share_init, share_leave, share_tick};

// Cross-screen entry points: the screens' (documented at their definitions), and app.c's own.
pub use crate::calibrate::calibrate_open;
pub use crate::editor::{
    editor_busy, editor_fit_changed, editor_fit_toast, editor_hold_eraser, editor_live_panel, editor_live_style, editor_live_under_panel, editor_open_notebook,
    editor_open_notebook_at, editor_page_canvas, editor_panel_point, editor_picture_made, editor_pin_sleep_page, editor_probe, editor_save_now,
    editor_shortcut, editor_shortcut_at, editor_suspend,
};
pub use crate::calibrate::calibrate_control_at;
pub use crate::editor::editor_control_at;
pub use crate::library::{library_bt, library_control_at, library_learn_key, library_probe};
pub use crate::pensettings::{pensettings_control_at, pensettings_open, pensettings_probe, pensettings_shortcut};
pub use crate::share::{share_control_at, share_draw_asleep, share_live_card_up, share_open, share_open_live, share_sleep};

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
    /// [eraser][side_button]
    styles: [[LiveStyle; 2]; 2],
}

thread_local! {
    static LIVE_CAP: Cell<LiveCap> = Cell::new(LiveCap::default());
    // app.c's g_live_ui_pending: the sequence of a captured UI pen-down not yet handled, 0 none
    static LIVE_UI_PENDING: Cell<u32> = const { Cell::new(0) };
    // and g_live_capture_blocked
    static LIVE_CAPTURE_BLOCKED: Cell<bool> = const { Cell::new(false) };
}

fn live_hover_sync(a: &App) {
    G_LIVE_AWAKE.store(G_LIVE_APP.load(Ordering::Relaxed) && !a.asleep, Ordering::Relaxed);
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
    live_set_capture(&LiveCapture { ok: cap.on && !cap.asleep && !cap.input_yielded, editor: cap.editor, handled, styles: cap.styles });
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
                let ev = InEvent { eraser: e == 1, side_button: side == 1, ..Default::default() };
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
    ScreenOps { init: editor_init, free_: editor_free, enter: editor_enter, leave: editor_leave, event: editor_event, tick: editor_tick },
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
    ScreenOps { init: share_init, free_: share_free, enter: share_enter, leave: share_leave, event: share_event, tick: share_tick },
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
    annotate_take(asked.as_deref() == Some("1") && !app_host().is_empty(), &mut r);
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
            csnprintf!(msg, cap, "%s", tr("Could not open the page KOReader sent: its picture does not read."));
        }
        AnnotateWhy::Storage => {
            // TRANSLATORS: (as above) the page could not be added to its notebook: a file could not be
            // written.
            csnprintf!(msg, cap, "%s", tr("Could not add the page KOReader sent. Is the storage full?"));
        }
        _ => {
            // TRANSLATORS: (as above) any other reason; %s is the log file that says why.
            csnprintf!(msg, cap, tr("Could not open the page KOReader sent (see %s)."), "grimoire.log");
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
        csnprintf!(&mut msg, 256, tr("The book has more pages than a notebook holds (%d)."), STORE_MAX_PAGES as i32);
    } else if a.book.why == AnnotateWhy::Picture {
        // TRANSLATORS: (as above) a page picture KOReader made is damaged or missing.
        csnprintf!(&mut msg, 256, "%s", tr("Could not open the book KOReader sent: a page's picture does not read."));
    } else if a.book.why == AnnotateWhy::Storage {
        // TRANSLATORS: (as above) the pages could not be added: a file could not be written.
        csnprintf!(&mut msg, 256, "%s", tr("Could not add the book's pages. Is the storage full?"));
    } else {
        // TRANSLATORS: (as above) any other reason; %s is the log file that says why.
        csnprintf!(&mut msg, 256, tr("Could not open the book KOReader sent (see %s)."), "grimoire.log");
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
        let added = a.book.made || a.book.added > 0;
        editor_open_notebook_at(a, nb.as_str(), a.book.pid, added);
        a.toast_until = now_ms(); // (the one saying how far it got, if it showed, goes)
        if a.book.added != 0 {
            let mut msg = String::new();
            // TRANSLATORS: in a toast, once the pages of a book KOReader sent are in its notebook.
            csnprintf!(&mut msg, 256, ng_("Added %d page of the book.", "Added %d pages of the book.", a.book.added as i64), a.book.added);
            app_toast(a, &msg, 5000);
        }
    } else if a.book.state == BookState::Failed {
        app_redraw_mode(a, RefreshMode::Ui); // (the toast saying how far it got goes)
        book_failed(a);
    } else if now_ms().wrapping_sub(a.book_told) >= 600 {
        a.book_told = now_ms();
        let mut msg = String::new();
        // TRANSLATORS: in a toast while Grimoire puts the pages of a book KOReader sent into its
        // notebook (a few seconds for a big book): pages done, of how many. One line of about 1700 px.
        csnprintf!(&mut msg, 256, tr("Adding the book's pages: %d of %d"), a.book.added, a.book.total);
        app_toast(a, &msg, 60000);
    }
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
    settings_load(&mut a.settings);
    let mut shortcuts_path = String::new();
    let mut controller_path = String::new();
    csnprintf!(&mut shortcuts_path, 600, "%s/shortcuts.ini", &store_root());
    csnprintf!(&mut controller_path, 1024, "%s/controller.ini", &store_root());
    shortcuts_load_all(&mut a.shortcuts, &shortcuts_path, &controller_path);
    app_gestures_load(a);
    // The owner's touch trace (docs/KINDLE.md): an empty <root>/touch-trace asks for
    // <root>/touch-trace.txt, written while Grimoire has the panel (only the Kindle writes one).
    let switch = cformat!("%s/touch-trace", &store_root());
    if std::fs::metadata(&switch).is_ok() {
        input_touch_trace(Some(&cformat!("%s/touch-trace.txt", &store_root())));
    }
    input_shortcuts_enable(a.settings.shortcuts_enabled);
    // The Bluetooth helper (bluetooth.h): the launcher says where its script is; in the simulator a
    // test may stand a helper in. It starts a moment after the first frame, if it is wanted.
    #[cfg(feature = "sim")]
    let standin = getenv("GRIMOIRE_BT_API");
    #[cfg(not(feature = "sim"))]
    let standin: Option<String> = None;
    bt_init(&store_root(), getenv("GRIMOIRE_BT_SCRIPT").as_deref(), standin.as_deref());
    // Settings > Updates (update.h): the launcher says where update.sh and the request file are.
    let mut request = String::new();
    let asked = getenv("GRIMOIRE_UPDATE_REQUEST");
    csnprintf!(&mut request, 700, "%s/.update-request", &store_root());
    let asked = asked.filter(|s| !s.is_empty());
    update_init(getenv("GRIMOIRE_UPDATE_SCRIPT").as_deref(), Some(asked.as_deref().unwrap_or(&request)));
    let mut failed = String::new();
    csnprintf!(&mut failed, 700, "%s/.update-failed", &store_root());
    if std::fs::metadata(&failed).is_ok() {
        let _ = std::fs::remove_file(&failed);
        qlog!("update: the last update did not install (update.sh's lines above in this log say why)");
        // TRANSLATORS: a toast at the start after an update KPM could not install (Settings > Updates).
        app_toast(a, tr("The last update did not install. grimoire.log says why."), 6000);
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
    calib_follow_panel(&mut st.pen_calib, &mut st.pen_panel_w, &mut st.pen_panel_h, w, h, "pen");
    calib_follow_panel(&mut st.touch_calib, &mut st.touch_panel_w, &mut st.touch_panel_h, w, h, "touch");
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
    import_scan(dpi);
    let caps = input_caps();
    if caps.has_orientation && a.settings.follow_rotation && caps.orientation != 0 {
        app_set_rotation(a, caps.orientation);
    }
    // A page KOReader sent to annotate: the editor opens on it now, under the toast below. When it
    // could not be opened, why is that toast, unless the passcode was left off.
    let mut note = String::new();
    let annotate_failed = annotate_at_start(a, &mut note, 256);
    // A whole book KOReader sent: its pages are added over the next moments (book_tick), and the
    // editor then opens on the reader's page.
    let book = getenv("GRIMOIRE_BOOK");
    book_start(book.as_deref() == Some("1") && !app_host().is_empty(), &mut a.book);
    // Compatibility notice for beta.1, which could leave the passcode off after a restart.
    // Current standalone sessions leave it enabled and yield to the native keypad on sleep.
    let passcode = getenv("GRIMOIRE_PASSCODE");
    let mut msg = String::new();
    if passcode.as_deref() == Some("left-off") {
        qlog!("device passcode: left off by an earlier session; telling the user");
        // TRANSLATORS: notices at startup, in a toast: one line of about 1700 px at 44 px in bold,
        // or two. "Kindle Settings" is the Kindle's own Settings, as its menus name them.
        app_toast(a, tr("Your Kindle passcode was left off. Turn it back on in Kindle Settings."), 10000);
    } else if annotate_failed {
        app_toast(a, &note, 8000);
    } else if a.book.state == BookState::Failed {
        book_failed(a);
    } else if !caps.has_pen {
        // TRANSLATORS: %s is the app's name, Grimoire.
        csnprintf!(&mut msg, 256, tr("No pen found. %s is made for the Kindle Scribe; writing needs its pen."), APP_NAME);
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

// A keyboard or controller came or went (shortcuts on): the Controller and keys sheet's status line
// follows, and a short toast says so, at most once a direction in 30 s, so a link that keeps
// dropping does not flash the screen. None over the sleep screen or a stroke being drawn.
fn hid_changed(a: &mut App, ev: &InEvent) {
    let attach = ev.type_ == InType::HidAttach;
    a.hid_changes = a.hid_changes.wrapping_add(1);
    if a.asleep || editor_busy(a) || !hidkeys_due(&mut a.hid_toast_at[if attach { 0 } else { 1 }], now_ms(), 30000) {
        return;
    }
    // TRANSLATORS: toasts (one line of about 1700 px at 44 px in bold) when a controller, keyboard
    // or page turner comes or goes. "Held by another app": another program (a button mapper)
    // takes its presses, so Grimoire gets none.
    let msg = if !attach { tr("Controller disconnected") } else if ev.value != 0 { tr("Controller held by another app") } else { tr("Controller connected") };
    app_toast(a, msg, 1500);
}

pub fn app_event(a: &mut App, ev: &InEvent) {
    app_event_screens(a, ev);
    live_block(false); // (the work before the screens' ends at any return)
    live_cap_sync(a);
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
        if !a.asleep && a.settings.shortcuts_enabled {
            // A key or a controller button counts as pen and touch do: paging with them alone
            // must not let "Sleep after" run out.
            note_input(a);
            a.last_shortcut_code = ev.value;
            a.last_shortcut_mods = ev.key_mods as i32;
            a.last_shortcut_gamepad = ev.gamepad;
            if a.scr == ScreenId::Library && library_learn_key(a, ev.value, ev.gamepad) {
                return;
            }
            let action = shortcuts_lookup(&a.shortcuts, ev.gamepad, ev.value, ev.key_mods as i32);
            if action == SHORTCUT_STATUS {
                status_toggle(a); // (the clock and battery corner is on every screen)
                return;
            }
            if a.scr == ScreenId::Penset {
                pensettings_shortcut(a, action);
            } else if a.scr == ScreenId::Editor
                && editor_shortcut_at(a, action, a.w as f32 * 0.5, a.h as f32 * 0.5, if ev.gamepad { ActionSrc::Pad } else { ActionSrc::Key })
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
            editor_hold_eraser(a, false);
        }
        return;
    }
    if ev.type_ == InType::HidAttach || ev.type_ == InType::HidDetach {
        if ev.type_ == InType::HidDetach && a.hold_code != 0 {
            // (whichever went, its release cannot come)
            a.hold_code = 0;
            editor_hold_eraser(a, false);
        }
        hid_changed(a, ev);
        return;
    }
    if ev.type_ == InType::Orientation {
        if a.settings.follow_rotation {
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
        match power_up_action(followed, a.power_answered, a.power_down_asleep, a.asleep, ev.value) {
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
    // A finger tap in the top left corner on no control turns the clock and battery on or off; the
    // screen sees the touch as it always does.
    if !a.asleep {
        status_touch(a, ev);
    }
    // A pump inside UI work must not preview the old page or tool before that work finishes.
    let mut before = LiveStyle::default();
    if live_on() && a.scr == ScreenId::Editor {
        editor_live_style(a, ev, &mut before);
    }
    live_block(!(ev.type_ == InType::PenMove
        || (ev.type_ == InType::PenDown && before.enabled != 0 && !rect_contains(before.exclude, ev.x as i32, ev.y as i32))));
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
    let n = gesturemap_load(&mut a.gestures, &path);
    if n > 0 {
        qlog!("gestures: %d line(s) of gestures.ini not used", n);
    } else if n < 0 {
        qlog!("gestures: gestures.ini could not be read; the built-in gestures apply");
    }
}

/// Whether the current screen has a control at (x, y): a button, a sheet or menu that is up, a
/// link on the page, the editor's panel. The clock and battery corner leaves a tap there to it.
pub fn app_control_at(a: &App, x: f32, y: f32) -> bool {
    match a.scr {
        ScreenId::Library => library_control_at(a, x, y),
        ScreenId::Editor => editor_control_at(a, x, y),
        ScreenId::Penset => pensettings_control_at(a, x, y),
        ScreenId::Calib => calibrate_control_at(a, x, y),
        ScreenId::Share => share_control_at(a, x, y),
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

// Where the toast goes: its usual place near the bottom, or, when the screen keeps the bottom
// for its controls (the keyboard, a sheet's buttons), just above them, so their labels stay
// readable while it shows.
fn toast_place(a: &App) -> Rect {
    let mut r = ui_toast_rect(Rect { x: 0, y: 0, w: a.w, h: a.h }, a.toast.as_str());
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
    let r = a.toast_rect;
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
    toast_hide(a);
    a.toast.set_cut(msg); // (too long: cut between two characters)
    a.toast_until = now_ms().wrapping_add(ms as u32);
    toast_draw(a);
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
        style = cap.styles[ev.eraser as usize][ev.side_button as usize];
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

// Where the chip that says Live view is on sits: the top right corner, inside the screens' 50 px
// margin (clear of the Library's buttons), never in the stream.
fn live_chip(a: &App) -> Rect {
    let s = ui_px(42.0);
    let m = ui_px(4.0);
    Rect { x: a.w - s - m, y: m, w: s, h: s }
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
    let panel = if a.scr == ScreenId::Editor { editor_live_panel(a) } else { RECT_EMPTY };
    let under = rect_intersect(r, panel);
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
        if ok && httpd_acquire(HTTPD_LIVE, a.settings.http_port, a.dpi, ui_theme() == UiTheme::Gothic) != 0 {
            live_stop();
            ok = false;
        }
        if !ok {
            // TRANSLATORS: a toast: Live view could not start (no memory for its copy of the screen,
            // or the server's port is taken).
            app_toast(a, tr("Could not start Live view"), 3000);
            return;
        }
        with(|s| {
            s.live_viewers = 0;
            s.live_panel = RECT_EMPTY; // (the new frame has no layer yet)
        });
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
        display_skip_overlay(live_chip(a));
        app_invalidate(a, live_chip(a), RefreshMode::Ui);
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
    app_watch_server(a);
    display_skip_overlay(RECT_EMPTY);
    app_invalidate(a, live_chip(a), RefreshMode::Ui); // (the chip goes: what is under it comes back)
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
        app_toast(a, tr("Live view turned off: no one watched for 10 minutes."), 4000);
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
    live_hover_sync(a);
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
    if mode == 2 {
        a.input_yielded = true;
        live_hover_sync(a);
        if !input_set_grab(false) || !sys_kindle_ui(true) {
            qlog!("sleep: could not hand input and screen to the Kindle; leaving for native unlock");
            a.running = false;
        }
        return;
    }
    let full = Rect { x: 0, y: 0, w: a.w, h: a.h };
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
        if !sys_kindle_ui(false) || !input_set_grab(true) {
            qlog!("wake: could not take the screen and input back; leaving");
            a.running = false;
            return;
        }
        a.input_yielded = false;
    }
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
            app_toast(a, tr("Could not save this page. Hold power 4 s again to leave anyway."), 10000);
            return;
        }
        let mut msg = String::new();
        // TRANSLATORS: the last notice before Grimoire closes, with the page saved, or without it;
        // %s is the app's name, Grimoire.
        csnprintf!(&mut msg, 256, if saved { tr("Saved. Leaving %s.") } else { tr("Leaving %s without this page.") }, APP_NAME);
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
    app_toast(a, if hosted { tr("Updating. Open Grimoire from KOReader again in a moment.") } else { tr("Updating. Grimoire starts again by itself.") }, 3000);
    a.running = false;
}

/// Turn the UI to a device rotation now (relayouts every screen).
pub fn app_set_rotation(a: &mut App, quarter_turns: i32) {
    let r = quarter_turns & 3;
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
    if live_on() {
        display_skip_overlay(live_chip(a));
    }
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

// A background job that finished: an import says how it went (the Library shows its new notebook),
// a template's picture may now be there to draw. Returns bg_poll's result for the PDF mirror's toast.
fn poll_jobs(a: &mut App) -> i32 {
    let mut done = -1;
    let r = bg_poll_kind(Some(&mut done));
    if done == BgKind::Template as i32 {
        bgimage_retry_missing();
        editor_picture_made(a); // (the open page may be on that picture: a page KOReader sent)
    }
    if done != BgKind::ImportFolder as i32 {
        return r;
    }
    let mut pages = 0;
    let got = import_finished(r > 0, &mut pages);
    if got > 0 && a.scr == ScreenId::Library && !a.asleep {
        app_redraw_mode(a, RefreshMode::Ui); // (waking redraws it too)
    }
    if got > 0 {
        let mut msg = String::new();
        // TRANSLATORS: a folder of page pictures (grimoire/import/<name>) became a new notebook.
        csnprintf!(&mut msg, 256, ng_("Imported a notebook of %d page.", "Imported a notebook of %d pages.", pages as i64), pages);
        app_toast(a, &msg, 8000);
    } else if got < 0 {
        let mut msg = String::new();
        // TRANSLATORS: a folder of page pictures could not become a notebook; %s is the log file
        // that says why. The folder stays where it was.
        csnprintf!(&mut msg, 256, tr("Could not import a folder of pages (see %s)."), "grimoire.log");
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
        if a.scr == ScreenId::Library {
            library_bt(a, bt, &bt_reply);
        }
    }
    book_tick(a);
    let bg = poll_jobs(a);
    if bg != 0 {
        let mut msg = String::new();
        // TRANSLATORS: a PDF saved in the background, and where (the Kindle's folder of them).
        if bg > 0 { csnprintf!(&mut msg, 256, tr("PDF saved to %s"), "grimoire/PDFs"); }
        app_toast(a, if bg > 0 { msg.as_str() } else { tr("Could not save the PDF") }, 2500);
    }
    if store_take_corrupt_count() > 0 {
        let mut msg = String::new();
        // TRANSLATORS: a page that could not be read; %s is the log file that says which.
        csnprintf!(&mut msg, 256, tr("A damaged page was set aside; a blank one took its place (see %s)"), "grimoire.log");
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
    let r = rect_intersect(r, Rect { x: 0, y: 0, w: a.w, h: a.h });
    if rect_is_empty(r) {
        return;
    }
    // Keep the strongest mode requested since the last flush.
    if rect_is_empty(a.dirty) || m as i32 > a.dirty_mode as i32 {
        a.dirty_mode = m;
    }
    a.dirty = rect_union(a.dirty, r);
}

pub fn app_invalidate_all(a: &mut App, m: RefreshMode) {
    app_invalidate(a, Rect { x: 0, y: 0, w: a.w, h: a.h }, m);
}

/// presents dirty rect (if any)
pub fn app_flush(a: &mut App) {
    app_flush_in(a);
    work_pump_mark(); // (a present's waits are the panel's, not work that should pump)
}

fn app_flush_in(a: &mut App) {
    if a.asleep {
        return; // the sleep screen stays up until wake
    }
    if rect_is_empty(a.dirty) && rect_is_empty(with(|s| s.self_dirty)) {
        live_presented(a.live_processed, a.live_processed_ms);
        status_flush(a); // (the clock and battery corner alone: its own small refresh, if one is due)
        return;
    }
    toast_follow(a); // (a screen that moves its floor redraws, so there is something to flush)
    let mut m = a.dirty_mode;
    let self_dirty = with(|s| s.self_dirty);
    if !rect_is_empty(self_dirty) {
        if rect_is_empty(a.dirty) || (m as i32) < RefreshMode::Ui as i32 {
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
    let chip = if live_on() { live_chip(a) } else { RECT_EMPTY };
    let mut chipped = !rect_is_empty(chip) && rect_overlaps(a.dirty, chip);
    if chipped {
        a.dirty = rect_union(a.dirty, chip);
        canvas_free(&mut a.chip_under);
        chipped = canvas_init(&mut a.chip_under, chip.w, chip.h) == 0;
        if chipped {
            canvas_blit(&mut a.chip_under, &a.screen, chip, 0, 0);
            ui_frame(&mut a.screen, chip, ui_px(8.0), 0, true, 0);
            ui_icon(&mut a.screen, IconKind::Live, rect_inflate(chip, -ui_px(2.0)), 255);
        }
    }
    // The corner's clock over what it covers, put back after the present like the others (the
    // toast goes over it).
    let mut dirty = a.dirty;
    let corner = status_cover(a, &mut dirty);
    a.dirty = dirty;
    let mut overlay = a.toast_visible && rect_overlaps(a.dirty, a.toast_rect);
    if overlay {
        a.dirty = rect_union(a.dirty, a.toast_rect);
        canvas_free(&mut a.toast_under);
        overlay = canvas_init(&mut a.toast_under, a.toast_rect.w, a.toast_rect.h) == 0;
        if overlay {
            canvas_blit(&mut a.toast_under, &a.screen, a.toast_rect, 0, 0);
            ui_toast_at(&mut a.screen, a.toast_rect, a.toast.as_str());
        }
    }
    display_present(&a.screen, a.dirty, m);
    if overlay {
        canvas_blit(&mut a.screen, &a.toast_under, Rect { x: 0, y: 0, w: a.toast_rect.w, h: a.toast_rect.h }, a.toast_rect.x, a.toast_rect.y);
    }
    if let Some(c) = corner {
        status_uncover(a, c);
    }
    if chipped {
        canvas_blit(&mut a.screen, &a.chip_under, Rect { x: 0, y: 0, w: chip.w, h: chip.h }, chip.x, chip.y);
    }
    status_presented(a, a.dirty);
    a.dirty = RECT_EMPTY;
    a.dirty_mode = RefreshMode::Fast;
    status_flush(a); // (a refresh of the corner this one did not hold)
}

/// calls leave/enter and redraws fully (REFRESH_FULL)
pub fn app_switch(a: &mut App, s: ScreenId) {
    // (the C returns for a ScreenId out of range; a Rust ScreenId cannot be)
    live_reset();
    a.live_processed = 0;
    a.live_processed_ms = 0;
    (ops(a.scr).leave)(a);
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

/// typing paused this long: the keyboard's grayscale refresh
const KB_SETTLE_MS: u32 = 900;

/// A pen or finger event on an on-screen keyboard (keyboard.h), drawn and refreshed the one way every
/// keyboard is, so a key costs the same over any screen: only when the key changed something, only
/// the keyboard (never what is under it), and on the fast waveform, the text line first and on its
/// own, then the key. Returns keyboard_pointer's result; on KB_DONE and KB_CANCEL nothing is drawn
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
pub fn app_keyboard_pointer_split(a: &mut App, kb: &mut Keyboard, phase: i32, ev: &InEvent) -> (KbResult, Rect) {
    let mut d = RECT_EMPTY;
    let r = keyboard_pointer(kb, phase, ev.x, ev.y, ev.t_ms, &mut d);
    if r == KbResult::Done || r == KbResult::Cancel {
        return (r, RECT_EMPTY);
    }
    // A sample that changed nothing draws nothing: the pen resting on a key sends one at every
    // digitizer frame, and each whole keyboard drawn again is main-thread work the next key waits for.
    if rect_is_empty(d) && rect_is_empty(kb.field_dirty) {
        return (r, RECT_EMPTY);
    }
    // Typing stays on the fast waveform: the text first (what the eye is on), then the key, each a
    // refresh of its own. One over both would span the keyboard from the text line down to the key:
    // on the Kindle several times the pixels to copy, and the next key's refreshes would overlap it
    // and wait in the driver's queue for it to end (docs/INK-PERFORMANCE.md), every key behind the
    // one before. One clean GL16 of the keyboard follows once typing pauses (app_keyboard_tick).
    keyboard_draw(kb, &mut a.screen, Rect { x: 0, y: 0, w: a.w, h: a.h });
    if !rect_is_empty(kb.field_dirty) {
        app_invalidate(a, kb.field_dirty, RefreshMode::Fast);
        app_flush(a);
    }
    kb.settle_at = now_ms().wrapping_add(KB_SETTLE_MS);
    if kb.settle_at == 0 {
        kb.settle_at = 1;
    }
    (r, d)
}

/// From the screen's tick while its keyboard is up: once typing pauses, one grayscale refresh of the
/// keyboard cleans up what the fast ones left.
pub fn app_keyboard_tick(a: &mut App, kb: &mut Keyboard, now: u32) {
    if kb.settle_at == 0 || (now.wrapping_sub(kb.settle_at) as i32) < 0 {
        return;
    }
    kb.settle_at = 0;
    if kb.open && kb.pressed_key < 0 {
        app_invalidate(a, kb.area, RefreshMode::Ui); // (not under a press)
    }
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
/// and its rect, "" if none), "asleep" (0|1), "asleep.mode", "host", "rotation" (0..3), "size"
/// (WxH), "language", "bg.busy", "settings.<settings.ini key>" for a few and for every tool's pen
/// feel ("settings.ballpoint.size_max_mm"), "light.intensity", "light.warmth", and "editor.*",
/// "library.*", "penset.*" (below), "annotate.*" (app.c) and "templates.*" (the open Templates
/// sheet: New notebook's on the Library, else the editor's); docs/SIM.md has them all. 0 and the
/// value in out, -1 for an unknown key. (The C finds the App through g_probe_app; the runner passes
/// it here: sim_sdl.rs's probe point is handed the App by mainloop.rs.)
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
    if let Some(k) = key.strip_prefix("penset.") {
        return pensettings_probe(a, k, out, cap);
    }
    // The Templates sheet (tplsheet.h): New notebook's on the Library, else the editor's.
    if key.starts_with("templates.") {
        return if a.scr == ScreenId::Library { library_probe(a, key, out, cap) } else { editor_probe(a, key, out, cap) };
    }
    // Settings, under their settings.ini names.
    let st = &a.settings;
    let nums: [(&str, i32); 25] = [
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
        ("settings.status_size", st.status_size),
        ("settings.gestures", st.gestures as i32),
        ("settings.landscape_fit", st.landscape_fit as i32),
        ("settings.hints", st.hints),
        ("settings.link_areas", st.link_areas as i32),
        ("shortcuts.code", a.last_shortcut_code),
        ("native_pin", a.native_pin as i32),
        ("input.yielded", a.input_yielded as i32),
        ("sleep.wallpaper", !a.koreader_wallpaper.px.is_empty() as i32),
        ("bg.busy", bg_busy() as i32), // a background job (an export, a compaction) runs or waits
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
            let Some(name) = app_probe_tool(ToolKind(t as i32)) else { continue };
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
    if key == "screen" {
        csnprintf!(out, cap, "%s", K_NAMES[a.scr as usize]);
    } else if key == "toast" {
        csnprintf!(out, cap, "%s", if a.toast_visible { a.toast.as_str() } else { "" });
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
    } else if key == "live.on" {
        csnprintf!(out, cap, "%d", if live_on() { 1 } else { 0 });
    } else if key == "live.viewers" {
        csnprintf!(out, cap, "%d", live_viewers());
    } else if key == "live.pairing" {
        csnprintf!(out, cap, "%d", if share_live_card_up(a) { 1 } else { 0 });
    } else if key == "live.chip" {
        let r = live_chip(a);
        if live_on() {
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
        let n = shortcuts_pads_for(&a.shortcuts, shortcut_action(action), &mut codes);
        out.clear();
        let mut i = 0;
        while i < n && i < 4 {
            let mut name = String::new();
            shortcut_pad_name(codes[i as usize], &mut name, 16);
            append_cut(out, cap, &cformat!("%s%s", if i != 0 { "," } else { "" }, &name));
            i += 1;
        }
    } else if let Some(action) = key.strip_prefix("gestures.") {
        // The touch gestures that do an action, as gestures.ini names them: "gestures.pages" ->
        // "touch.2.down" ("" none).
        let act = shortcut_action(action);
        out.clear();
        for fingers in 2..=3u8 {
            for kind in [TouchKind::Down, TouchKind::Up, TouchKind::Left, TouchKind::Right, TouchKind::Tap] {
                let k = GestureKey::Touch { fingers, kind };
                if act != SHORTCUT_NONE && gesturemap_action(&a.gestures, k) == act {
                    let mut name = String::new();
                    gesturemap_key_name(k, &mut name);
                    append_cut(out, cap, &cformat!("%s%s", if out.is_empty() { "" } else { "," }, &name));
                }
            }
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
        ["pencil", "mech", "ballpoint", "eraser", "highlighter", "fountain", "fineliner", "marker", "charcoal", "calligraphy"];
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
        m.part[2] = MusicPart(12);
        app_probe_music(Some(&m), &mut out, 64);
        assert_eq!(out, "5: treble 0 12");
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
        app_invalidate(&mut a, Rect { x: 10, y: 10, w: 5, h: 5 }, RefreshMode::Ui);
        app_invalidate(&mut a, Rect { x: 50, y: 60, w: 5, h: 5 }, RefreshMode::Fast);
        assert_eq!(a.dirty_mode, RefreshMode::Ui);
        assert_eq!(a.dirty, Rect { x: 10, y: 10, w: 45, h: 55 });
        // Off the screen: nothing.
        app_invalidate(&mut a, Rect { x: 300, y: 0, w: 5, h: 5 }, RefreshMode::Full);
        assert_eq!(a.dirty_mode, RefreshMode::Ui);
        app_invalidate_all(&mut a, RefreshMode::Full);
        assert_eq!((a.dirty, a.dirty_mode), (Rect { x: 0, y: 0, w: 100, h: 200 }, RefreshMode::Full));
        // With nothing dirty, the first mode asked for is the mode, however weak.
        a.dirty = RECT_EMPTY;
        app_invalidate(&mut a, Rect { x: 0, y: 0, w: 1, h: 1 }, RefreshMode::Fast);
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
}
