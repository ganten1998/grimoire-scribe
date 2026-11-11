//! store.h: everything on disk. See docs/DESIGN.md for the layout and docs/FORMAT.md for the
//! files themselves. (store.c: notebooks, pages, settings, folders and thumbnails on disk.)

use crate::prelude::*;
use std::cell::RefCell;
use std::cmp::Ordering;
use std::io::Read;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{DirBuilderExt, MetadataExt};

pub const STORE_MAX_PAGES: usize = 2000;
pub const STORE_ID_LEN: usize = 24;
pub const STORE_NAME_LEN: usize = 96;
pub const STORE_EXTRA_LEN: usize = 2048;
pub const STORE_SOURCE_LEN: usize = 64;

#[derive(Clone, Debug, Default)]
pub struct NotebookInfo {
    /// directory name, e.g. "20261010-143205-a3f1"
    pub id: FixedStr<STORE_ID_LEN>,
    pub name: FixedStr<STORE_NAME_LEN>,
    /// "" = top level; nested folders are slash-separated logical names (not disk paths).
    pub folder: FixedStr<STORE_NAME_LEN>,
    /// default paper for new pages
    pub paper: PaperKind,
    pub created: i64,
    pub modified: i64,
    /// index reopened next time
    pub last_page: i32,
    /// page order; files are p<id>.qpg. At most STORE_MAX_PAGES; its length is the C's npages
    /// (NotebookInfo::npages).
    pub page_ids: Vec<u32>,
    pub next_page_id: u32,
    /// its pages' size, px: the screen it was made on, kept wherever it is opened (notebook.ini
    /// page_w, page_h; see store_load_info)
    pub page_w: i32,
    pub page_h: i32,
    /// the book it annotates (notebook.ini source=): KOReader's key of
    /// the book's file (annotate.h); "" for none
    pub source: FixedStr<STORE_SOURCE_LEN>,
    /// notebook.ini lines this build does not know ("key=value\n"
    /// each), written back as they were
    pub extra: FixedStr<STORE_EXTRA_LEN>,
    /// left out of the Graph of all notebooks (notebook.ini graph=0, 2.0.0-beta.2; the Library's notebook
    /// menu): its pages are no nodes there, links to it no edges. Older versions carry the line as it is.
    pub graph_omit: bool,
}

impl NotebookInfo {
    /// The C's npages.
    pub fn npages(&self) -> i32 {
        self.page_ids.len() as i32
    }
}

pub const SETTINGS_VERSION: i32 = 4;
/// pens pinned to the editor's panel, at most
pub const PANEL_PINS_MAX: usize = 3;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Settings {
    /// settings_version from the file (defaults migrate on upgrade)
    pub version: i32,
    pub tools: [ToolParams; TOOL_COUNT],
    /// what the pen draws with (may be the eraser)
    pub current_tool: ToolKind,
    /// the drawing tool the toolbar's tool button shows
    pub draw_tool: ToolKind,
    /// the last custom music layout built (offered on the Custom tile)
    pub music_layout: PaperKind,
    /// floating panel top-left
    pub panel_x: i32,
    pub panel_y: i32,
    pub panel_collapsed: bool,
    /// holding the pen button: 0 nothing, 1 eraser, 2 highlighter,
    /// 3 white ink with the pen's own drawing tool
    pub side_button: i32,
    /// write grimoire/PDFs/<folder>/<name>.pdf on close
    pub mirror_pdf: bool,
    /// turn the UI with the Kindle (otherwise stay as is)
    pub follow_rotation: bool,
    /// white on black
    pub invert: bool,
    /// optional external keyboard/gamepad mappings
    pub shortcuts_enabled: bool,
    /// start the Bluetooth helper with Grimoire (bluetooth.h)
    pub controller: bool,
    /// Settings > Updates' channel: 0 by the version (a beta: preview),
    /// 1 releases only (stable), 2 betas too (preview); update.h
    pub updates: i32,
    /// two-finger zoom in the editor
    pub pinch_zoom: bool,
    /// sleep screen: 0 the page I'm on, 1 a pinned page, 2 the Kindle's own
    pub sleep_mode: i32,
    /// sleep after this long without input (0 = never)
    pub sleep_minutes: i32,
    /// UiTheme: 0 gothic, 1 classic (settings.ini: theme=gothic|classic)
    pub theme: i32,
    /// Classic's font, an index of font.rs's FONT_FAMILIES (settings.ini font=, the family's id:
    /// terminess, atkinson, noto-sans or literata; 2.1). Gothic keeps its own faces whatever it is.
    pub font: i32,
    /// Classic's font when it is one of the owner's own (settings.ini font=file:<path>, 2.1.0-beta.2): its
    /// path under grimoire/fonts, "" for a family (`font`, which is then what draws when the file is missing
    /// or has not passed its check: Terminess). userfonts.rs loads it.
    pub font_file: FixedStr<208>,
    /// The Bold file that goes with it (settings.ini font_bold=file:<path>), "" to let Grimoire find one: the
    /// font's own weight axis, else a Bold file of its family beside it, else the regular overprinted.
    pub font_bold: FixedStr<208>,
    pub pen_calib: InputCalib,
    pub touch_calib: InputCalib,
    /// The panel each calibration was made on (settings.ini pen.panel, touch.panel: WxH). A
    /// calibration maps to panel pixels, so on a panel of another size it is scaled (app_init).
    /// 0 x 0: not recorded (a calibration from before 1.1, or one a version without the key saved).
    pub pen_panel_w: i32,
    pub pen_panel_h: i32,
    pub touch_panel_w: i32,
    pub touch_panel_h: i32,
    /// share server, default 8765
    pub http_port: i32,
    /// the "passcode paused while Grimoire is open" notice was shown once
    pub passcode_told: bool,
    /// settings.ini language=: "auto" (the Kindle's), or a code of i18n.h's list
    pub language: FixedStr<8>,
    /// the eraser (its button, the eraser end, side_button 1): 0 takes
    /// whole strokes away, 1 the precise eraser (erase strokes)
    pub eraser_mode: i32,
    /// the drawing tools draw white ink (erase strokes, own texture)
    pub white: bool,
    /// The pens pinned to the editor's panel under its tool button, in pin order: drawing tools
    /// only, each once (settings.ini panel_tools=, their names comma-separated).
    pub pins: [ToolKind; PANEL_PINS_MAX],
    pub npins: i32,
    /// The pen in use when the eraser was last picked on the panel, a tool id or -1 for none: where
    /// a tap on the eraser in use goes back to while that pen is still on the panel.
    pub before_eraser: i32,
    /// The clock and battery corner is on (settings.ini status_on; status.rs)
    pub status_on: bool,
    /// and how large it is: 0 small, 1 medium, 2 large (settings.ini status_size)
    pub status_size: i32,
    /// and in which corner of the page it sits: 0 top left (the default), 1 top right, 2 bottom right,
    /// 3 bottom left (settings.ini status_corner; the tap zone follows it)
    pub status_corner: i32,
    /// The clock shows the time on the 12-hour clock (settings.ini clock_12h): "2:32 PM" instead of
    /// "14:32". The Display sheet's "Display time as 12 hour" row.
    pub clock_12h: bool,
    /// Rotation to hold while locked (settings.ini rotation_lock): 0 follows the Kindle, 1..=4
    /// hold the current rotation plus one. The app resolves the current orientation at lock time.
    pub rotation_lock: i32,
    /// Two-finger gestures (the swipes and taps of gestures.ini; settings.ini gestures): off, a pinch
    /// and a pan still work.
    pub gestures: bool,
    /// Sideways, the editor shows the whole page (settings.ini landscape_fit); off, the page's width
    /// fills the screen as upright.
    pub landscape_fit: bool,
    /// Two fingers left on the last page add a page (settings.ini gesture_new_page, on by default); off, a
    /// gesture only says it is the last page (a key's next page still adds one).
    pub gesture_new_page: bool,
    /// Legacy status-display value, normalized to the clock corner (1) on load.
    pub status_mode: i32,
    pub shapes: bool,
    pub shape_hold_ms: i32,
    /// Four-sided shapes: 0 uneven, 1 rectangles, 2 squares.
    pub shape_quads: i32,
    /// Round shapes: 0 as drawn, 1 prefer circles, 2 circles only.
    pub shape_round: i32,
    pub shape_links: bool,
    /// The smallest closed shape a held stroke snaps to, screen mm across, 3 to 40 (10 by default;
    /// rc.9 to rc.11 saved 0 for their 10 mm). Since 2.0.0-rc.12 the recognizer's own floor.
    pub shape_min_mm: i32,
    /// What's new (about.rs): the version whose notes were last shown or due (settings.ini
    /// whatsnew_seen), and the box Don't show this again of the 2.0.0 betas (whatsnew_hide): read and
    /// written back as it was, so an older Grimoire that reads the file keeps its meaning, but since
    /// 2.0.0 never set and never read for whether the notes open (whatsnew_due). 2.0.0.
    pub whatsnew_seen: FixedStr<32>,
    pub whatsnew_hide: bool,
    /// Smart shapes round a snapped polygon's corners by this percentage of the largest radius that
    /// fits (shape.rs fillet; 0: sharp). 2.0.0-rc.12.
    pub shape_corner: i32,
    /// Memory only, never saved: this start follows an update (a file a Grimoire saved, its notes seen
    /// another version's), so the Library opens What's new once.
    pub whatsnew_due: bool,
    /// One-time hints already shown, a bitmask (settings.ini hints; bit numbers: docs/FORMAT.md).
    pub hints: i32,
    /// The eye: dotted boxes over the page where a finger tap follows a link (settings.ini link_areas).
    pub link_areas: bool,
    /// Memory only, never saved: settings.ini was an existing version 3 file without a gestures key,
    /// so this run is the owner's first with the gestures, and the one-time toasts of what changed show.
    pub upgraded: bool,
}

// ---- state ----

/// A known settings.ini key whose value this build could not use as written (see keep_value).
struct Kept {
    key: String,
    /// the value used instead, as settings_save writes it (the longest: panel_tools)
    used: String,
    /// the value in the file
    raw: Vec<u8>,
}

/// store.c's file-level statics. Touched by the main thread only (PORTING.md, Global state).
struct State {
    g_root: String,
    g_dpi: f32,
    g_w: i32,
    g_h: i32,
    /// notebook.ini lines this build does not know, written back after the keys it knows. Whole
    /// lines only: one that no longer fits is left out (counted here, logged by store_load_info).
    g_info_dropped: i32,
    g_corrupt_pages: i32,
    /// Set by store_load_info when it had to work the page size out from the page files.
    g_size_derived: bool,
    g_restored_pages: i32,
    /// qlog_once's seen[]: the hashes of the lines logged so far (at most 256)
    seen: Vec<u64>,
    /// What settings.ini holds that this build does not understand goes back into the file as it
    /// was: lines with keys it does not know (written after the ones it knows), and values of known
    /// keys it cannot use as written (side_button=4, a theme it lacks, a slider past its range). Such a
    /// value is used as the nearest one this build has, and written back unchanged until the setting is
    /// saved with another value. Kept here, not in Settings (which is copied by value), for the file
    /// settings_load read last.
    g_set_unknown: Vec<u8>,
    /// unknown lines past the bound, logged once per load
    g_set_dropped: i32,
    g_kept: Vec<Kept>,
    /// how many notebook.ini files store_load_info has read (store_info_reads)
    g_info_reads: u32,
}

impl State {
    const fn new() -> State {
        State {
            g_root: String::new(),
            g_dpi: 300.0,
            g_w: 1860,
            g_h: 2480,
            g_info_dropped: 0,
            g_corrupt_pages: 0,
            g_size_derived: false,
            g_restored_pages: 0,
            seen: Vec::new(),
            g_set_unknown: Vec::new(),
            g_set_dropped: 0,
            g_kept: Vec::new(),
            g_info_reads: 0,
        }
    }
}

thread_local! {
    static S: RefCell<State> = const { RefCell::new(State::new()) };
}

// Each borrow is one read or write of the statics: nothing here calls another module, a log or a
// callback while holding it.
fn with<R>(f: impl FnOnce(&mut State) -> R) -> R {
    S.with(|s| f(&mut s.borrow_mut()))
}
fn g_root() -> String {
    with(|s| s.g_root.clone())
}
fn g_dpi() -> f32 {
    with(|s| s.g_dpi)
}
fn g_wh() -> (i32, i32) {
    with(|s| (s.g_w, s.g_h))
}

/// Device geometry: the page size of the notebooks made here, and the dpi stroke bboxes are sized at.
pub fn store_set_device(dpi: f32, w: i32, h: i32) {
    with(|s| {
        if dpi > 50.0 {
            s.g_dpi = dpi;
        }
        // Pages are portrait whatever way the screen was turned at launch.
        if w > 0 && h > 0 {
            s.g_w = if w < h { w } else { h };
            s.g_h = if w < h { h } else { w };
        }
    });
}
pub fn store_dpi() -> f32 {
    g_dpi()
}
/// The page size new notebooks get (the device's portrait screen), regardless of rotation.
pub fn store_page_size(w: &mut i32, h: &mut i32) {
    let (gw, gh) = g_wh();
    *w = gw;
    *h = gh;
}

// ---- libc's number parsing, over the bytes of a file ----
//
// settings.ini, notebook.ini and the other files are bytes, not always UTF-8, and the C reads their
// numbers with atoi, strtoul and strtof: these read them the same way (white space, a sign, the
// digits up to the first other character; strtoul and strtol saturate, as the target's `long` does,
// which is 32 bits on the Kindle).

fn c_isspace(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | 0x0B | 0x0C | b'\r')
}

/// The bytes up to the first NUL: what the C sees of a buffer it reads as a string.
fn c_text(b: &[u8]) -> &[u8] {
    match b.iter().position(|&c| c == 0) {
        Some(n) => &b[..n],
        None => b,
    }
}

/// strtok_r(txt, "\n"): the non-empty lines.
fn c_lines(txt: &[u8]) -> impl Iterator<Item = &[u8]> {
    c_text(txt).split(|&b| b == b'\n').filter(|l| !l.is_empty())
}

/// White space and a sign, then the digits of s: (negative, where the digits start, where they end);
/// the end is 0 when there are none (strtol's endptr is then s itself).
fn c_digits(s: &[u8]) -> (bool, usize, usize) {
    let mut i = 0;
    while i < s.len() && c_isspace(s[i]) {
        i += 1;
    }
    let mut neg = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        neg = s[i] == b'-';
        i += 1;
    }
    let ds = i;
    while i < s.len() && s[i].is_ascii_digit() {
        i += 1;
    }
    if i == ds {
        (neg, ds, 0)
    } else {
        (neg, ds, i)
    }
}

/// strtoul(s, &end, 10) with an `unsigned long` of the target: the value, and the bytes it read
/// (0: no digits).
fn c_strtoul(s: &[u8]) -> (u64, usize) {
    let (neg, ds, end) = c_digits(s);
    if end == 0 {
        return (0, 0);
    }
    let max = libc::c_ulong::MAX as u64;
    let mut v: u64 = 0;
    let mut over = false;
    for &d in &s[ds..end] {
        match v.checked_mul(10).and_then(|x| x.checked_add((d - b'0') as u64)) {
            Some(x) if x <= max => v = x,
            _ => {
                over = true;
                break;
            }
        }
    }
    let v = if over {
        max
    } else if neg {
        v.wrapping_neg() & max
    } else {
        v
    };
    (v, end)
}

/// strtol(s, &end, 10) with a `long` of the target.
fn c_strtol(s: &[u8]) -> (i64, usize) {
    let (neg, ds, end) = c_digits(s);
    if end == 0 {
        return (0, 0);
    }
    let lmax = libc::c_long::MAX as i64;
    let lmin = libc::c_long::MIN as i64;
    let mut mag: u64 = 0;
    for &d in &s[ds..end] {
        mag = mag.checked_mul(10).and_then(|x| x.checked_add((d - b'0') as u64)).unwrap_or(u64::MAX);
    }
    let v = if neg {
        if mag > lmax as u64 + 1 {
            lmin
        } else {
            (-(mag as i128)) as i64
        }
    } else if mag > lmax as u64 {
        lmax
    } else {
        mag as i64
    };
    (v, end)
}

/// atoi, as musl's (the Kindle's) computes it: in int, wrapping.
fn c_atoi(s: &[u8]) -> i32 {
    let (neg, ds, end) = c_digits(s);
    let mut n: i32 = 0;
    if end != 0 {
        for &d in &s[ds..end] {
            n = n.wrapping_mul(10).wrapping_sub((d - b'0') as i32);
        }
    }
    if neg {
        n
    } else {
        n.wrapping_neg()
    }
}

/// atoll, the same in long long.
fn c_atoll(s: &[u8]) -> i64 {
    let (neg, ds, end) = c_digits(s);
    let mut n: i64 = 0;
    if end != 0 {
        for &d in &s[ds..end] {
            n = n.wrapping_mul(10).wrapping_sub((d - b'0') as i64);
        }
    }
    if neg {
        n
    } else {
        n.wrapping_neg()
    }
}

/// strtof(s, NULL): the longest prefix that is a number, correctly rounded. (A hex float of more
/// than 53 bits is rounded twice: nobody writes one.)
fn c_strtof(s: &[u8]) -> f32 {
    let mut i = 0;
    while i < s.len() && c_isspace(s[i]) {
        i += 1;
    }
    let mut neg = false;
    if i < s.len() && (s[i] == b'+' || s[i] == b'-') {
        neg = s[i] == b'-';
        i += 1;
    }
    let r = &s[i..];
    let starts = |w: &[u8]| r.len() >= w.len() && r[..w.len()].eq_ignore_ascii_case(w);
    let hexd = |b: u8| (b as char).is_ascii_hexdigit();
    let v: f32 = if starts(b"infinity") || starts(b"inf") {
        f32::INFINITY
    } else if starts(b"nan") {
        f32::NAN
    } else if starts(b"0x") && r.len() > 2 && (hexd(r[2]) || (r[2] == b'.' && r.len() > 3 && hexd(r[3]))) {
        let mut mant: u64 = 0;
        let mut exp2: i32 = 0;
        let mut j = 2;
        let mut dot = false;
        while j < r.len() && (hexd(r[j]) || (r[j] == b'.' && !dot)) {
            if r[j] == b'.' {
                dot = true;
            } else if mant < (1u64 << 60) {
                mant = mant * 16 + (r[j] as char).to_digit(16).unwrap_or(0) as u64;
                if dot {
                    exp2 -= 4;
                }
            } else if !dot {
                exp2 += 4;
            }
            j += 1;
        }
        if j < r.len() && (r[j] | 0x20) == b'p' {
            let mut k = j + 1;
            let mut eneg = false;
            if k < r.len() && (r[k] == b'+' || r[k] == b'-') {
                eneg = r[k] == b'-';
                k += 1;
            }
            let ds = k;
            let mut e: i32 = 0;
            while k < r.len() && r[k].is_ascii_digit() {
                e = e.saturating_mul(10).saturating_add((r[k] - b'0') as i32);
                k += 1;
            }
            if k > ds {
                exp2 = exp2.saturating_add(if eneg { -e } else { e });
            }
        }
        ((mant as f64) * 2f64.powi(exp2)) as f32
    } else {
        let mut j = 0;
        while j < r.len() && r[j].is_ascii_digit() {
            j += 1;
        }
        let int_digits = j;
        let mut frac_digits = 0;
        if j < r.len() && r[j] == b'.' {
            let mut k = j + 1;
            while k < r.len() && r[k].is_ascii_digit() {
                k += 1;
            }
            frac_digits = k - j - 1;
            if int_digits > 0 || frac_digits > 0 {
                j = k;
            }
        }
        if int_digits + frac_digits == 0 {
            0.0
        } else {
            if j < r.len() && (r[j] | 0x20) == b'e' {
                let mut k = j + 1;
                if k < r.len() && (r[k] == b'+' || r[k] == b'-') {
                    k += 1;
                }
                let ds = k;
                while k < r.len() && r[k].is_ascii_digit() {
                    k += 1;
                }
                if k > ds {
                    j = k;
                }
            }
            // (ASCII digits, '.', 'e' and a sign: valid UTF-8)
            std::str::from_utf8(&r[..j]).ok().and_then(|t| t.parse::<f32>().ok()).unwrap_or(0.0)
        }
    };
    if neg {
        -v
    } else {
        v
    }
}

// ---- sscanf's patterns for file names ----

/// sscanf(name, "p%lu.qp%7s", &id, tail) == 2 with tail "g": a page file's name, and its number.
/// (sscanf skips white space before %lu and %s, and %lu takes a sign: "p +7.qp g" is page 7 too.)
fn scan_page_name(name: &[u8]) -> Option<u64> {
    let s = name.strip_prefix(b"p")?;
    let (id, used) = c_strtoul(s);
    if used == 0 {
        return None;
    }
    let s = s[used..].strip_prefix(b".qp")?;
    let s = &s[s.iter().take_while(|&&b| c_isspace(b)).count()..];
    let tail = &s[..s.iter().take(7).take_while(|&&b| !c_isspace(b)).count()];
    (tail == b"g").then_some(id)
}

/// sscanf(name, "p%lu.qpg.damaged-%lld%n", &id, &t, &end) == 2 with the whole name read: a page set
/// aside by a version that cannot open it, its number and time.
fn scan_aside_name(name: &[u8]) -> Option<(u64, i64)> {
    let s = name.strip_prefix(b"p")?;
    let (id, used) = c_strtoul(s);
    if used == 0 {
        return None;
    }
    let s = s[used..].strip_prefix(b".qpg.damaged-")?;
    let (t, used) = c_strtoll(s);
    if used == 0 || used != s.len() {
        return None;
    }
    Some((id, t))
}

/// strtoll(s, &end, 10).
fn c_strtoll(s: &[u8]) -> (i64, usize) {
    let (neg, ds, end) = c_digits(s);
    if end == 0 {
        return (0, 0);
    }
    let mut mag: u64 = 0;
    for &d in &s[ds..end] {
        mag = mag.checked_mul(10).and_then(|x| x.checked_add((d - b'0') as u64)).unwrap_or(u64::MAX);
    }
    let v = if neg {
        if mag > i64::MAX as u64 + 1 {
            i64::MIN
        } else {
            (-(mag as i128)) as i64
        }
    } else if mag > i64::MAX as u64 {
        i64::MAX
    } else {
        mag as i64
    };
    (v, end)
}

/// sscanf(s, "P5 %d %d %d%n"): the three numbers and where the third ends, or None when the text
/// is not that (fewer than three conversions). %d takes white space and a sign first.
fn scan_p5_header(d: &[u8]) -> Option<(i32, i32, i32, usize)> {
    let d = c_text(d);
    if !d.starts_with(b"P5") {
        return None;
    }
    let mut pos = 2;
    let mut v = [0i32; 3];
    for x in v.iter_mut() {
        let (n, used) = c_strtoll(&d[pos..]);
        if used == 0 {
            return None;
        }
        // (stored in an int: its low 32 bits)
        *x = n as i32;
        pos += used;
    }
    Some((v[0], v[1], v[2], pos))
}

// ---- text ----

/// One argument of sb_printf: a value for the format's own conversions, or the raw bytes a `%s`
/// takes (names and values in the files are bytes, not always UTF-8: Rust's `%s` takes a &str).
enum SbArg<'a> {
    V(Arg<'a>),
    B(&'a [u8]),
}
fn val<'a>(x: impl Into<Arg<'a>>) -> SbArg<'a> {
    SbArg::V(x.into())
}
fn raw(x: &[u8]) -> SbArg<'_> {
    SbArg::B(x)
}

/// printf into b, for a format whose `%s` conversions take bytes (raw) and the others values (a).
/// A text that never fails to fit: the C's Sb grows (and a failed allocation sticks; here there is
/// none to fail).
fn sb_printf(b: &mut Vec<u8>, fmt: &str, args: &[SbArg]) {
    let mut rest = fmt;
    let mut at = 0;
    loop {
        let (head, tail) = match rest.find("%s") {
            Some(i) => (&rest[..i], Some(&rest[i + 2..])),
            None => (rest, None),
        };
        // the conversions of head: each '%' that is not "%%"
        let hb = head.as_bytes();
        let mut n = 0;
        let mut i = 0;
        while i < hb.len() {
            if hb[i] == b'%' {
                if i + 1 < hb.len() && hb[i + 1] == b'%' {
                    i += 1;
                } else {
                    n += 1;
                }
            }
            i += 1;
        }
        let vals: Vec<Arg> = args[at..at + n]
            .iter()
            .map(|x| match x {
                SbArg::V(v) => *v,
                SbArg::B(_) => Arg::I32(0),
            })
            .collect();
        at += n;
        b.extend_from_slice(crate::cfmt::cformat(head, &vals).as_bytes());
        match tail {
            Some(t) => {
                match &args[at] {
                    SbArg::B(bytes) => b.extend_from_slice(bytes),
                    SbArg::V(Arg::Str(s)) => b.extend_from_slice(s.as_bytes()),
                    SbArg::V(_) => {}
                }
                at += 1;
                rest = t;
            }
            None => break,
        }
    }
}

/// A note about something that stays as it is (a copy kept aside, lines that do not fit) is logged
/// the first time it comes up in a run, not on every listing or opening of the notebook.
fn qlog_once_str(msg: &str) {
    // (the C formats into a 512-byte line)
    let msg = cut_at_bytes(msg, 511);
    let mut h: u64 = 14695981039346656037; // FNV-1a of the whole line
    for &c in msg.as_bytes() {
        h = (h ^ c as u64).wrapping_mul(1099511628211);
    }
    let fresh = with(|s| {
        if s.seen.contains(&h) {
            return false;
        }
        if s.seen.len() < 256 {
            s.seen.push(h);
        }
        true
    });
    if fresh {
        qlog!("%s", msg);
    }
}
macro_rules! qlog_once {
    ($fmt:expr $(, $x:expr)* $(,)?) => {
        qlog_once_str(&cformat!($fmt $(, $x)*))
    };
}

// ---- tiny ini ----

/// The lines of an ini file as (key, value) to cb, which keeps what it knows and sends the rest to
/// its own list of unknown lines (the C's IniCb and IniUnknown): so keys a newer version wrote can
/// be written back (docs/FORMAT.md). Comment lines and lines with no '=' are skipped; a value loses
/// one trailing CR.
fn ini_parse_file(path: &str, cb: &mut dyn FnMut(&[u8], &[u8])) -> i32 {
    let Some(txt) = read_file(path) else { return -1 ;
    };
    for line in c_lines(&txt) {
        let Some(eq) = line.iter().position(|&b| b == b'=') else { continue ;
        };
        if line[0] == b'#' {
            continue;
        }
        let mut v = &line[eq + 1..];
        if let [rest @ .., b'\r'] = v {
            v = rest;
        }
        cb(&line[..eq], v);
    }
    0
}

/// Names are single-line in the ini. One longer than the field is cut short at a character's
/// start, never inside a UTF-8 character: a name cut in the middle of one would be written back
/// so, and its mirror PDF's path and the share pages would carry a broken character.
fn clean_line<const N: usize>(dst: &mut FixedStr<N>, src: &[u8]) {
    let src = c_text(src);
    let mut n = src.len().min(N.saturating_sub(1));
    let mut buf = src[..n].to_vec();
    if src.get(n).is_some_and(|&b| b & 0xC0 == 0x80) {
        // cut inside a character: drop its first bytes
        while n > 0 && buf[n - 1] & 0xC0 == 0x80 {
            n -= 1;
        }
        if n > 0 && buf[n - 1] & 0xC0 == 0xC0 {
            n -= 1;
        }
        buf.truncate(n);
    }
    for c in buf.iter_mut() {
        if *c == b'\n' || *c == b'\r' {
            *c = b' ';
        }
    }
    dst.set_bytes(&buf);
}

// ---- notebooks ----

/// A page size a notebook can have: a screen's (bigger ones are damage; export.c refuses them too).
fn page_size_ok(w: i64, h: i64) -> bool {
    (100..=6000).contains(&w) && (100..=6000).contains(&h)
}
/// A size as written in an ini value ("1986"), 0 for one that is no size at all; and where the
/// number ends.
fn size_value(v: &[u8]) -> (i32, usize) {
    let (x, used) = c_strtol(v);
    (if x > 0 && x <= 100000 { x as i32 } else { 0 }, used)
}

/// The size of a notebook's pages: nb->page_w x nb->page_h, which store_create and store_load_info
/// always set (the device's size for a NotebookInfo that has none).
pub fn store_notebook_page_size(nb: &NotebookInfo, w: &mut i32, h: &mut i32) {
    let own = page_size_ok(nb.page_w as i64, nb.page_h as i64);
    let (gw, gh) = g_wh();
    *w = if own { nb.page_w } else { gw };
    *h = if own { nb.page_h } else { gh };
}
pub fn store_root() -> String {
    g_root()
}

/// creates root/, notebooks/, trash/
pub fn store_init(root: &str) -> i32 {
    with(|s| {
        strlcpy_q(&mut s.g_root, root, 512);
    });
    if mkdir_p(root) != 0 {
        qlog!("store: cannot create %s", root);
        return -1;
    }
    let mut p = String::new();
    csnprintf!(&mut p, 600, "%s/notebooks", root);
    mkdir_p(&p);
    csnprintf!(&mut p, 600, "%s/trash", root);
    mkdir_p(&p);
    0
}

/// Ids are generated by us; reject anything that could escape the notebooks dir.
fn id_ok(id: &[u8]) -> bool {
    if id.is_empty() || id.len() >= STORE_ID_LEN {
        return false;
    }
    id.iter().all(|&c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
}

/// root/notebooks/<id> (the C's dir[600]), None for an id that is no notebook's.
fn notebook_dir(id: &[u8]) -> Option<String> {
    if !id_ok(id) {
        return None;
    }
    let mut dir = String::new();
    csnprintf!(&mut dir, 600, "%s/notebooks/%s", &g_root(), std::str::from_utf8(id).unwrap_or(""));
    Some(dir)
}

pub fn store_notebook_dir(id: &str, buf: &mut String, cap: usize) -> i32 {
    if !id_ok(id.as_bytes()) {
        return -1;
    }
    csnprintf!(buf, cap, "%s/notebooks/%s", &g_root(), id);
    0
}

fn info_cb(nb: &mut NotebookInfo, k: &[u8], v: &[u8]) -> bool {
    match k {
        b"name" => clean_line(&mut nb.name, v),
        b"folder" => clean_line(&mut nb.folder, v),
        // A paper this build does not know stays as it is: new pages get it, a newer version draws it.
        b"paper" => nb.paper = PaperKind(c_strtoul(v).0 as u32),
        b"created" => nb.created = c_atoll(v),
        b"modified" => nb.modified = c_atoll(v),
        b"last_page" => nb.last_page = c_atoi(v),
        b"next_page_id" => nb.next_page_id = c_strtoul(v).0 as u32,
        // The page size (1.1 on). A value no screen has is not kept: store_load_info finds the size.
        b"page_w" => nb.page_w = size_value(v).0,
        b"page_h" => nb.page_h = size_value(v).0,
        // The book it annotates (1.1 on, annotate.h); a value of another form is a newer version's.
        b"graph" if v == b"0" || v == b"1" => nb.graph_omit = v == b"0",
        b"source" if source_ok(v) => {
            nb.source.set_bytes(v);
        }
        b"pages" => {
            nb.page_ids.clear();
            let mut p = v;
            while !p.is_empty() && nb.page_ids.len() < STORE_MAX_PAGES {
                let (id, used) = c_strtoul(p);
                if used == 0 {
                    break;
                }
                if id > 0 {
                    nb.page_ids.push(id as u32);
                }
                p = if p.get(used) == Some(&b',') { &p[used + 1..] } else { &p[used..] };
            }
        }
        _ => return false,
    }
    true
}

fn info_unknown(nb: &mut NotebookInfo, k: &[u8], v: &[u8]) {
    let have = nb.extra.len();
    let need = k.len() + v.len() + 2;
    if have + need >= STORE_EXTRA_LEN {
        with(|s| s.g_info_dropped += 1);
        return;
    }
    let mut b = nb.extra.as_bytes().to_vec();
    b.extend_from_slice(k);
    b.push(b'=');
    b.extend_from_slice(v);
    b.push(b'\n');
    nb.extra.set_bytes(&b);
}

fn rebuild_pages(dir: &str, nb: &mut NotebookInfo) {
    let Ok(rd) = std::fs::read_dir(dir) else { return ;
    };
    for e in rd {
        if nb.page_ids.len() >= STORE_MAX_PAGES {
            break;
        }
        let Ok(e) = e else { break };
        if let Some(id) = scan_page_name(e.file_name().as_bytes()) {
            if id > 0 && id < 0x7fffffff {
                nb.page_ids.push(id as u32);
            }
        }
    }
    nb.page_ids.sort_unstable();
}

/// Pages found damaged since the last call (each was renamed to p<id>.qpg.damaged-<time>).
pub fn store_take_corrupt_count() -> i32 {
    with(|s| std::mem::take(&mut s.g_corrupt_pages))
}

/// The size most of a notebook's page files have (each from its first 16 bytes); a tie goes to the
/// size that comes first in page order. Pages without one (missing, damaged) do not count. Leaves
/// nb->page_w x page_h as it is when no page has a size. Stops reading once one size has more than
/// half the pages, so a notebook of one size reads half its page files.
fn notebook_size_from_pages(dir: &str, nb: &mut NotebookInfo) {
    // sizes counted, in the order first seen (a fifth is a page of damage)
    const SIZES: usize = 4;
    let mut sw = [0i32; SIZES];
    let mut sh = [0i32; SIZES];
    let mut count = [0i32; SIZES];
    let mut nsizes = 0;
    for i in 0..nb.page_ids.len() {
        let path = cformat!("%s/p%u.qpg", dir, nb.page_ids[i]);
        let Some((w, h)) = page_file_size(&path) else { continue ;
        };
        let mut k = 0;
        while k < nsizes && (sw[k] != w || sh[k] != h) {
            k += 1;
        }
        if k == nsizes {
            if nsizes == SIZES {
                continue;
            }
            sw[k] = w;
            sh[k] = h;
            count[k] = 0;
            nsizes += 1;
        }
        count[k] += 1;
        if count[k] * 2 > nb.npages() {
            break; // more than half: no other size can have more
        }
    }
    let mut best: Option<usize> = None;
    for k in 0..nsizes {
        if best.is_none_or(|b| count[k] > count[b]) {
            best = Some(k); // a tie: the one seen first
        }
    }
    let Some(best) = best else { return };
    nb.page_w = sw[best];
    nb.page_h = sh[best];
}

/// How many notebook.ini files this thread has read (store_load_info): what a test counts to show a
/// screen reads each notebook once.
pub fn store_info_reads() -> u32 {
    with(|s| s.g_info_reads)
}

/// A notebook.ini with no page size (versions before 1.1 write none, and 1.0.0-beta drops it on a
/// save) takes the size of its first page file that has one, else the device's.
pub fn store_load_info(id_in: &str, out: &mut NotebookInfo) -> i32 {
    let mut id = FixedStr::<STORE_ID_LEN>::new();
    id.set(id_in); // id_in may be out->id itself: copy it before the reset
    let Some(dir) = notebook_dir(id.as_bytes()) else { return -1 ;
    };
    *out = NotebookInfo::default();
    out.id.set_bytes(id.as_bytes());
    let path = cformat!("%s/notebook.ini", &dir);
    with(|s| {
        s.g_info_dropped = 0;
        s.g_info_reads = s.g_info_reads.wrapping_add(1);
    });
    if ini_parse_file(&path, &mut |k, v| {
        if !info_cb(out, k, v) {
            info_unknown(out, k, v);
        }
    }) != 0
    {
        return -1;
    }
    let dropped = with(|s| s.g_info_dropped);
    if dropped != 0 {
        qlog_once!("store: %s: %d unknown notebook.ini line(s) do not fit in the %d bytes kept, left out", &id, dropped, STORE_EXTRA_LEN);
    }
    if out.page_ids.is_empty() {
        // A damaged or empty notebook.ini: rebuild the page list from the page files, so the
        // next save cannot drop every page but the first.
        rebuild_pages(&dir, out);
        if !out.page_ids.is_empty() {
            qlog!("store: %s had no page list, found %d page file(s)", &id, out.npages());
        }
    }
    if out.page_ids.is_empty() {
        // never leave a notebook without a page
        out.page_ids.push(1);
    }
    if !page_size_ok(out.page_w as i64, out.page_h as i64) {
        // No page size recorded (a notebook from before 1.1, or one 1.0.0-beta saved: it drops the
        // keys): the size most of its page files have, else the device's. The next save records
        // it. Most, not the first's: a version before 1.1 on a Scribe of the other size writes
        // each page it edits at that size, and 1.0.0-beta re-saves page 1 after a page delete.
        let (gw, gh) = g_wh();
        out.page_w = gw;
        out.page_h = gh;
        notebook_size_from_pages(&dir, out);
        with(|s| s.g_size_derived = true);
    }
    let maxid = out.page_ids.iter().copied().max().unwrap_or(0);
    if out.next_page_id <= maxid {
        out.next_page_id = maxid.wrapping_add(1);
    }
    out.last_page = clampi(out.last_page, 0, out.npages() - 1);
    if out.name.is_empty() {
        // TRANSLATORS: the name of a notebook that has none (it is saved with it, as a name given).
        out.name.set(tr("Untitled"));
    }
    0
}

fn write_info(nb: &NotebookInfo) -> i32 {
    let Some(dir) = notebook_dir(nb.id.as_bytes()) else { return -1 ;
    };
    let rc = write_info_at(&dir, nb);
    if rc == 0 {
        libindex_note_info(nb); // (the Graph of all notebooks' index: its pages, name and folder now)
    }
    rc
}

/// A notebook built in a folder of its own before it becomes one (an import, in a dot-folder that
/// store_list skips, renamed into notebooks/ once whole): its notebook.ini (modified as it is), a
/// page file, and bg/pages.ini for pages pids[i] on papers[i] (image papers only; bgdir is its bg).
pub fn store_write_info_in(dir: &str, nb: &NotebookInfo) -> i32 {
    write_info_at(dir, nb)
}

fn write_info_at(dir: &str, nb: &NotebookInfo) -> i32 {
    let path = cformat!("%s/notebook.ini", dir);
    let mut name = FixedStr::<STORE_NAME_LEN>::new();
    let mut folder = FixedStr::<STORE_NAME_LEN>::new();
    clean_line(&mut name, nb.name.as_bytes());
    clean_line(&mut folder, nb.folder.as_bytes());
    let mut b: Vec<u8> = Vec::new();
    sb_printf(
        &mut b,
        "# Grimoire notebook\nname=%s\nfolder=%s\npaper=%u\ncreated=%lld\nmodified=%lld\n\
         last_page=%d\nnext_page_id=%u\npages=",
        &[
            raw(name.as_bytes()),
            raw(folder.as_bytes()),
            val(nb.paper.0),
            val(nb.created),
            val(nb.modified),
            val(nb.last_page),
            val(nb.next_page_id),
        ],
    );
    for i in 0..nb.page_ids.len().min(STORE_MAX_PAGES) {
        sb_printf(
            &mut b,
            "%s%u",
            &[val(if i > 0 { "," } else { "" }), val(nb.page_ids[i])],
        );
    }
    b.push(b'\n');
    let (mut pw, mut ph) = (0, 0);
    store_notebook_page_size(nb, &mut pw, &mut ph);
    sb_printf(&mut b, "page_w=%d\npage_h=%d\n", &[val(pw), val(ph)]);
    if source_ok(nb.source.as_bytes()) {
        sb_printf(&mut b, "source=%s\n", &[raw(nb.source.as_bytes())]);
    }
    if nb.graph_omit {
        b.extend_from_slice(b"graph=0\n"); // (2.0.0-beta.2; no line: in the Graph)
    }
    b.extend_from_slice(nb.extra.as_bytes()); // keys a newer version wrote, as they were
    write_file_atomic(&path, &b)
}

/// also bumps modified
pub fn store_save_info(nb: &mut NotebookInfo) -> i32 {
    nb.modified = wall_time();
    write_info(nb)
}

/// store_source_ok over bytes (a source= value, or a NotebookInfo's own).
fn source_ok(key: &[u8]) -> bool {
    let n = key
        .iter()
        .take(STORE_SOURCE_LEN)
        .position(|&c| c == 0)
        .unwrap_or(key.len().min(STORE_SOURCE_LEN));
    if n == 0 || n >= STORE_SOURCE_LEN {
        return false;
    }
    key[..n]
        .iter()
        .all(|&c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
}
/// A book key as notebook.ini's source= holds one: 1 to 63 letters, digits, '-' or '_'. A source=
/// line with any other value is one this version does not know: kept as it is (NotebookInfo.extra).
pub fn store_source_ok(key: &str) -> bool {
    source_ok(key.as_bytes())
}

fn page_path(nb: &NotebookInfo, pid: u32) -> String {
    cformat!("%s/notebooks/%s/p%u.qpg", &g_root(), &nb.id, pid)
}

fn write_page_at(path: &str, p: &Page) -> i32 {
    let mut data = Vec::new();
    if page_serialize(p, &mut data) != 0 {
        return -1;
    }
    write_file_atomic(path, &data)
}

fn write_page_file(nb: &NotebookInfo, pid: u32, p: &Page) -> i32 {
    write_page_at(&page_path(nb, pid), p)
}

pub fn store_write_page_in(dir: &str, pid: u32, p: &Page) -> i32 {
    let path = cformat!("%s/p%u.qpg", dir, pid);
    write_page_at(&path, p)
}

// ---- bg/pages.ini: the image paper of every page that has one ----
//
// A page's paper is in its own file, but 1.0.0-beta saves a page whose paper it does not know as
// version 1 Blank, and the page's picture would be lost to it. This list, which no older version
// touches, brings it back (docs/FORMAT.md, Image papers): every save that makes a page's paper an
// image paper, or makes it stop being one, updates it.

#[derive(Clone, Copy)]
struct SideEntry {
    pid: u32,
    kind: u32,
}
#[derive(Default)]
struct Side {
    e: Vec<SideEntry>,
}

fn side_path(bgdir: &str) -> String {
    cformat!("%s/pages.ini", bgdir)
}

fn side_get(s: &Side, pid: u32) -> Option<u32> {
    s.e.iter().find(|e| e.pid == pid).map(|e| e.kind)
}

/// Lists pid with kind, or (kind 0) takes it off the list. -1 out of memory.
fn side_set(s: &mut Side, pid: u32, kind: u32) -> i32 {
    for i in 0..s.e.len() {
        if s.e[i].pid != pid {
            continue;
        }
        if kind != 0 {
            s.e[i].kind = kind;
        } else {
            s.e.swap_remove(i);
        }
        return 0;
    }
    if kind == 0 {
        return 0;
    }
    if s.e.try_reserve(1).is_err() {
        return -1;
    }
    s.e.push(SideEntry { pid, kind });
    0
}

/// Reads <bgdir>/pages.ini: lines p<page id>=<paper value>, for image papers only. 0 (no file: an
/// empty list), or -1 when it is there but does not read, or there is no memory for it: then nothing
/// may be saved over it, or every other page's entry would go.
fn side_load(bgdir: &str, s: &mut Side) -> i32 {
    *s = Side::default();
    let path = side_path(bgdir);
    let Some(txt) = read_file(&path) else {
        return if access_ok(&path) { -1 } else { 0 };
    };
    let mut rc = 0;
    for line in c_lines(&txt) {
        if rc != 0 {
            break;
        }
        if line[0] != b'p' {
            continue;
        }
        let (pid, used) = c_strtoul(&line[1..]);
        if used == 0 || line.get(1 + used) != Some(&b'=') || pid == 0 || pid >= 0x7fffffff {
            continue;
        }
        let kind = c_strtoul(&line[1 + used + 1..]).0;
        if paper_is_image(PaperKind(kind as u32)) {
            rc = side_set(s, pid as u32, kind as u32);
        }
    }
    if rc != 0 {
        *s = Side::default();
    }
    rc
}

fn side_save(bgdir: &str, s: &mut Side) -> i32 {
    s.e.sort_by_key(|e| e.pid);
    let mut b: Vec<u8> = Vec::new();
    b.extend_from_slice(
        cformat!("# Grimoire: the image paper of each page that has one (docs/FORMAT.md)\n")
            .as_bytes(),
    );
    for e in &s.e {
        b.extend_from_slice(cformat!("p%u=%u\n", e.pid, e.kind).as_bytes());
    }
    let path = side_path(bgdir);
    if mkdir_p(bgdir) != 0 {
        -1
    } else {
        write_file_atomic(&path, &b)
    }
}

/// Brings bg/pages.ini in step with page pid's paper: an image paper listed, any other not.
fn side_note(nb: &NotebookInfo, pid: u32, paper: PaperKind) {
    let mut bgdir = String::new();
    if bgimage_dir(nb.id.as_str(), &mut bgdir, 700) != 0 {
        return;
    }
    let path = side_path(&bgdir);
    let image = paper_is_image(paper);
    if !image && !access_ok(&path) {
        return; // nothing listed: nothing to take off
    }
    let mut s = Side::default();
    if side_load(&bgdir, &mut s) != 0 {
        qlog_once!(
            "store: %s: bg/pages.ini does not read; left as it is",
            &nb.id
        );
        return;
    }
    let have = side_get(&s, pid);
    let listed = have.is_some();
    let wrong = if image {
        !(listed && have == Some(paper.0))
    } else {
        listed
    };
    if wrong
        && (side_set(&mut s, pid, if image { paper.0 } else { 0 }) != 0
            || side_save(&bgdir, &mut s) != 0)
    {
        qlog!(
            "store: %s: could not update bg/pages.ini for p%u.qpg",
            &nb.id,
            pid
        );
    }
}

/// A page an older version saved as blank paper (version 1 Blank) that bg/pages.ini lists with an
/// image paper gets its picture back: in this version a page only ever goes to Blank by a save that
/// takes it off the list. Whether it did.
fn side_restore(nb: &NotebookInfo, pid: u32, p: &mut Page) -> bool {
    if p.paper != PAPER_BLANK || p.readonly {
        return false;
    }
    let mut bgdir = String::new();
    if bgimage_dir(nb.id.as_str(), &mut bgdir, 700) != 0 {
        return false;
    }
    let path = side_path(&bgdir);
    if !access_ok(&path) {
        return false;
    }
    let mut s = Side::default();
    let kind = if side_load(&bgdir, &mut s) == 0 {
        side_get(&s, pid)
    } else {
        None
    };
    if let Some(kind) = kind {
        p.paper = PaperKind(kind);
        qlog_once!(
            "store: %s: p%u.qpg was saved with blank paper by an older version; its picture comes back from \
             bg/pages.ini",
            &nb.id,
            pid
        );
    }
    kind.is_some()
}

/// How many pages store_load_page gave their image paper back since the last call: the thumbnail
/// and PDF an older version made of them show blank paper, to be drawn again.
pub fn store_take_restored_count() -> i32 {
    with(|s| std::mem::take(&mut s.g_restored_pages))
}

pub fn store_write_pages_list_in(bgdir: &str, pids: &[u32], papers: &[PaperKind]) -> i32 {
    let mut s = Side::default();
    let mut rc = 0;
    for (&pid, &paper) in pids.iter().zip(papers) {
        if rc != 0 {
            break;
        }
        if paper_is_image(paper) {
            rc = side_set(&mut s, pid, paper.0);
        }
    }
    if rc == 0 {
        rc = side_save(bgdir, &mut s);
    }
    rc
}

/// bg/pages.ini (bgdir is the notebook's bg) with the lines of pages pids[i] on papers[i] added or
/// replaced, the other lines kept, in one write: pages added at once (annotate.h, a whole book).
pub fn store_note_papers_in(bgdir: &str, pids: &[u32], papers: &[PaperKind]) -> i32 {
    let mut s = Side::default();
    if side_load(bgdir, &mut s) != 0 {
        return -1;
    }
    let mut rc = 0;
    for (&pid, &paper) in pids.iter().zip(papers) {
        if rc != 0 {
            break;
        }
        rc = side_set(&mut s, pid, if paper_is_image(paper) { paper.0 } else { 0 });
    }
    if rc == 0 {
        rc = side_save(bgdir, &mut s);
    }
    rc
}

/// The paper of a page added after (or left in place of) a page on `after`: the same paper, but a
/// document page's picture (an imported page image, paper_is_document) belongs to that page alone:
/// then the notebook's own paper (Blank if that is a document page's too). A template stays.
pub fn store_new_page_paper(nb: &NotebookInfo, after: PaperKind) -> PaperKind {
    if !paper_is_document(after) {
        return after;
    }
    if paper_is_document(nb.paper) {
        PAPER_BLANK
    } else {
        nb.paper
    }
}

/// A notebook store_create just made (its one empty page) goes onto `paper`, which may be an image
/// paper of its own pictures: store_create takes only a paper the pickers offer, and a template's
/// picture can go into a notebook's bg/ only once the notebook exists (templates.h). Its own paper
/// (notebook.ini, what new pages get) and its first page's (listed in bg/pages.ini) are written.
pub fn store_start_on(nb: &mut NotebookInfo, paper: PaperKind) -> i32 {
    let (mut w, mut h) = (0, 0);
    store_notebook_page_size(nb, &mut w, &mut h);
    let mut p: Option<Box<Page>> = None;
    if store_load_page(nb, 0, w, h, &mut p) != 0 {
        return -1;
    }
    let Some(mut p) = p else { return -1 };
    p.paper = paper;
    let rc = store_save_page(nb, 0, &p); // (listed in bg/pages.ini, as every image paper's page)
    drop(p);
    if rc != 0 {
        return -1;
    }
    nb.paper = paper;
    store_save_info(nb)
}

#[cfg(test)]
thread_local! {
    /// tests: the next random suffixes, in order (a collision made on purpose)
    static FORCED_HEX: std::cell::RefCell<std::collections::VecDeque<String>> = const { std::cell::RefCell::new(std::collections::VecDeque::new()) };
}

/// Tests: the next ids' random suffixes (each 4 hex digits), in order; an empty list clears them.
#[cfg(test)]
pub(crate) fn store_force_random_hex(next: &[&str]) {
    FORCED_HEX.with(|f| {
        let mut f = f.borrow_mut();
        if next.is_empty() {
            f.clear();
        }
        f.extend(next.iter().map(|h| h.to_string()));
    });
}

fn random_hex(n: usize) -> String {
    #[cfg(test)]
    if let Some(h) = FORCED_HEX.with(|f| f.borrow_mut().pop_front()) {
        return h;
    }
    let mut b = [0u8; 8];
    if !crate::util::random_fill(&mut b) {
        // (no system randomness: the clock, the writer and a count, so two ids made in the same
        // millisecond on the same thread still differ)
        static COUNT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let k = COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let x: u32 = now_ms() ^ (std::process::id()).wrapping_mul(2654435761u32) ^ k.wrapping_mul(0x9e37_79b9).rotate_left(16);
        b[..4].copy_from_slice(&x.to_ne_bytes());
        b[4..].copy_from_slice(&k.to_ne_bytes());
    }
    const HX: &[u8; 16] = b"0123456789abcdef";
    (0..n)
        .map(|i| HX[((b[i / 2] >> (if i & 1 != 0 { 0 } else { 4 })) & 15) as usize] as char)
        .collect()
}

/// A fresh notebook id ("20261010-143205-a3f1": the local time and 4 random hex digits).
pub fn store_make_id(id: &mut String /* STORE_ID_LEN - 1 bytes */) {
    let tm = local_tm(wall_time());
    let rnd = random_hex(4);
    csnprintf!(
        id,
        STORE_ID_LEN,
        "%04d%02d%02d-%02d%02d%02d-%s",
        tm.tm_year + 1900,
        tm.tm_mon + 1,
        tm.tm_mday,
        tm.tm_hour,
        tm.tm_min,
        tm.tm_sec,
        &rnd
    );
}

/// With 1 empty page, at the device's page size (recorded as the notebook's).
pub fn store_create(name: &str, folder: &str, paper: PaperKind, out: &mut NotebookInfo) -> i32 {
    let nb = out;
    *nb = NotebookInfo::default();
    let t = wall_time();
    let mut id = String::new();
    store_make_id(&mut id);
    nb.id.set(&id);
    clean_line(
        &mut nb.name,
        if !name.is_empty() {
            name.as_bytes()
        } else {
            tr("Untitled").as_bytes()
        },
    );
    clean_line(&mut nb.folder, folder.as_bytes());
    nb.paper = paper_sanitize(paper.0, PAPER_MEDIUM);
    nb.created = t;
    nb.modified = t;
    nb.page_ids = vec![1];
    nb.next_page_id = 2;
    // the device's page size, the notebook's wherever it is opened later
    let (gw, gh) = g_wh();
    nb.page_w = gw;
    nb.page_h = gh;
    let parent = cformat!("%s/notebooks", &g_root());
    if mkdir_p(&parent) != 0 {
        return -1;
    }
    // Two notebooks made in the same second must never share a directory: plain mkdir, and a
    // fresh random suffix if the name is taken.
    let mut attempt = 0;
    loop {
        let dir = notebook_dir(nb.id.as_bytes()).unwrap_or_default();
        let err = match std::fs::DirBuilder::new().mode(0o755).create(&dir) {
            Ok(()) => break,
            Err(e) => e.raw_os_error().unwrap_or(0),
        };
        if err != libc::EEXIST || attempt >= 8 {
            qlog!("store: cannot create %s: %s", &dir, &strerror(err));
            return -1;
        }
        let rnd2 = random_hex(4);
        let mut b = nb.id.as_bytes().to_vec();
        let at = b.len().saturating_sub(4);
        b.truncate(at);
        b.extend_from_slice(rnd2.as_bytes());
        nb.id.set_bytes(&b);
        attempt += 1;
    }
    let mut p = page_new(nb.page_w, nb.page_h, nb.paper);
    p.nb.set_bytes(nb.id.as_bytes());
    let rc = write_page_file(nb, 1, &p);
    drop(p);
    if rc != 0 {
        return -1;
    }
    write_info(nb)
}

pub fn store_rename(id: &str, name: &str) -> i32 {
    let mut nb = NotebookInfo::default();
    let mut rc = store_load_info(id, &mut nb);
    if rc == 0 {
        clean_line(&mut nb.name, name.as_bytes());
        rc = store_save_info(&mut nb);
    }
    rc
}

/// Notebook id left out of the Graph of all notebooks, or shown there again (notebook.ini graph=0): its
/// notebook.ini written with nothing else changed (not even modified: the Library's order stays).
pub fn store_set_graph_omit(id: &str, omit: bool) -> i32 {
    let mut nb = NotebookInfo::default();
    let rc = store_load_info(id, &mut nb);
    if rc != 0 {
        return rc;
    }
    if nb.graph_omit == omit {
        return 0;
    }
    nb.graph_omit = omit;
    write_info(&nb)
}

pub fn store_set_folder(id: &str, folder: &str) -> i32 {
    // Preserve the historic UTF-8 truncation of flat names. Nested destinations must never
    // silently become a different path; the Library validates all new joins before calling us.
    if folder.contains('/') && !store_folder_valid(folder) {return -1;}
    let mut clean=FixedStr::<STORE_NAME_LEN>::new();clean_line(&mut clean,folder.as_bytes());
    if !clean.is_empty() && !store_folder_valid(clean.as_str()){return -1;}
    let mut nb = NotebookInfo::default();
    let mut rc = store_load_info(id, &mut nb);
    if rc == 0 {
        clean_line(&mut nb.folder, folder.as_bytes());
        rc = store_save_info(&mut nb);
    }
    if rc == 0 {
        graph_omit_prune(); // (the folder it left may be gone)
    }
    rc
}

fn move_to_trash(src: &str, base: &str) -> i32 {
    let mut dst = String::new();
    csnprintf!(
        &mut dst,
        800,
        "%s/trash/%s-%lld",
        &g_root(),
        base,
        wall_time()
    );
    if let Err(e) = std::fs::rename(src, &dst) {
        qlog!(
            "trash %s: %s",
            src,
            &strerror(e.raw_os_error().unwrap_or(0))
        );
        return -1;
    }
    0
}

/// moves to trash/
pub fn store_delete(id: &str) -> i32 {
    let Some(dir) = notebook_dir(id.as_bytes()) else {
        return -1;
    };
    let rc = move_to_trash(&dir, id);
    if rc == 0 {
        libindex_note_gone(id);
        graph_omit_prune(); // (its folder may be gone with it)
    }
    rc
}

fn cmp_modified(x: &NotebookInfo, y: &NotebookInfo) -> Ordering {
    if x.modified != y.modified {
        return if x.modified < y.modified {
            Ordering::Greater
        } else {
            Ordering::Less
        };
    }
    x.id.as_bytes().cmp(y.id.as_bytes())
}

/// Library. store_list fills out; sorted by modified, newest first.
pub fn store_list(out: &mut Vec<NotebookInfo>) -> i32 {
    out.clear();
    let dir = cformat!("%s/notebooks", &g_root());
    let Ok(rd) = std::fs::read_dir(&dir) else {
        return -1;
    };
    let mut arr: Vec<NotebookInfo> = Vec::new();
    for e in rd {
        let Ok(e) = e else { break };
        let d_name = e.file_name();
        let d_name = d_name.as_bytes();
        if d_name.first() == Some(&b'.') || !id_ok(d_name) {
            continue;
        }
        let name = std::str::from_utf8(d_name).unwrap_or_default();
        with(|s| s.g_size_derived = false);
        let mut nb = NotebookInfo::default();
        if store_load_info(name, &mut nb) == 0 {
            // A notebook from before 1.1: its size was worked out from its page files. Record it
            // now (as written, its modified time unchanged) so later listings read one file, not
            // half its pages.
            if with(|s| s.g_size_derived) && write_info(&nb) != 0 {
                qlog_once!("store: could not record the page size of %s", name);
            }
            arr.push(nb);
        } else {
            qlog!("store: skipping unreadable notebook %s", name);
        }
    }
    arr.sort_by(cmp_modified);
    *out = arr;
    0
}

/// Footer totals, reading one notebook at a time without retaining every page list or writing
/// legacy size metadata. Unreadable notebooks are omitted just as they are in the Library.
pub fn store_page_totals()->Option<(usize,usize)> {
    let dir=cformat!("%s/notebooks",&g_root());
    let mut pages=0usize;let mut books=0usize;
    for entry in std::fs::read_dir(dir).ok()? {
        let entry=entry.ok()?;let name=entry.file_name();let name=name.as_bytes();
        if name.first()==Some(&b'.') || !id_ok(name) {continue;}
        let mut nb=NotebookInfo::default();
        if store_load_info(std::str::from_utf8(name).ok()?,&mut nb)==0 {pages+=nb.page_ids.len();books+=1;}
    }
    Some((pages,books))
}

// ---- folders ----

type FolderName = FixedStr<STORE_NAME_LEN>;

fn load_folders_ini() -> Vec<FolderName> {
    let path = cformat!("%s/folders.ini", &g_root());
    let mut out = Vec::new();
    let Some(txt) = read_file(&path) else {
        return out;
    };
    let mut first = true;
    for line in c_lines(&txt) {
        // Only the header line is a comment: a folder may well be called "#1".
        let header = first && line.starts_with(b"# ") && line.windows(8).any(|w| w == b" folders");
        first = false;
        if line.is_empty() || header {
            continue;
        }
        let mut f = FolderName::new();
        clean_line(&mut f, line);
        out.push(f);
    }
    out
}

fn save_folders_ini(f: &[FolderName]) -> i32 {
    let path = cformat!("%s/folders.ini", &g_root());
    let mut buf: Vec<u8> = Vec::new();
    buf.extend_from_slice(b"# Grimoire folders\n");
    for name in f {
        buf.extend_from_slice(name.as_bytes());
        buf.push(b'\n');
    }
    write_file_atomic(&path, &buf)
}

// ---- folders left out of the Graph (root/graph-omit.ini, 2.0.0-beta.2) ----
//
// # Grimoire: folders left out of the Graph of all notebooks
// folder=School/2025
//
// One line a folder (its logical path, as notebook.ini's folder= writes it); everything inside it, nested
// folders and notebooks moved in later included, is left out. Kept apart from folders.ini, whose every line
// older versions read as a folder's name. No other version reads it; one that renames or removes a folder
// leaves a stale line, which names no folder and so leaves nothing out.

const GRAPH_OMIT_HEADER: &[u8] = b"# Grimoire: folders left out of the Graph of all notebooks\n";
/// graph-omit.ini is read up to this size; a bigger one is not this version's and leaves nothing out.
pub const GRAPH_OMIT_FILE_MAX: usize = 64 * 1024;

fn graph_omit_path() -> String {
    cformat!("%s/graph-omit.ini", &g_root())
}

/// The folders left out of the Graph of all notebooks (graph-omit.ini), as written; none when there is no
/// such file or it does not read.
pub fn store_graph_omit_folders() -> Vec<String> {
    let mut out = Vec::new();
    let path = graph_omit_path();
    if std::fs::metadata(&path).map_or(true, |m| !m.is_file() || m.len() as usize > GRAPH_OMIT_FILE_MAX) {
        return out;
    }
    let Some(txt) = read_file(&path) else { return out };
    for line in txt.split(|&c| c == b'\n') {
        let line = line.strip_suffix(b"\r").unwrap_or(line);
        let Some(v) = line.strip_prefix(b"folder=") else { continue };
        let Ok(f) = std::str::from_utf8(v) else { continue };
        if store_folder_valid(f) && !out.iter().any(|o: &String| o == f) {
            out.push(f.to_string());
        }
    }
    out
}

fn save_graph_omit(folders: &[String]) -> i32 {
    let path = graph_omit_path();
    if folders.is_empty() {
        return match std::fs::remove_file(&path) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => -1,
            _ => 0,
        };
    }
    let mut b = GRAPH_OMIT_HEADER.to_vec();
    for f in folders {
        b.extend_from_slice(b"folder=");
        b.extend_from_slice(f.as_bytes());
        b.push(b'\n');
    }
    write_file_atomic(&path, &b)
}

/// Folder `path` (and everything in it) left out of the Graph of all notebooks, or shown there again.
/// Showing a folder that is left out only because a folder around it is changes nothing.
pub fn store_set_folder_graph_omit(path: &str, omit: bool) -> i32 {
    if !store_folder_valid(path) {
        return -1;
    }
    let mut f = store_graph_omit_folders();
    let had = f.iter().any(|x| x == path);
    if had == omit {
        return 0;
    }
    if omit {
        f.push(path.to_string());
        f.sort();
    } else {
        f.retain(|x| x != path);
    }
    let rc = save_graph_omit(&f);
    if rc == 0 {
        libindex_note_omit();
    }
    rc
}

/// Whether folder `path` is left out of the Graph of all notebooks, by its own line or a folder around it.
pub fn store_folder_graph_omitted(folders: &[String], path: &str) -> bool {
    !path.is_empty() && folders.iter().any(|f| store_folder_contains(f, path))
}

// A folder renamed or moved (old to new, a subtree): its lines and those of the folders inside it follow.
fn graph_omit_renamed(old: &str, new: &str) {
    let f = store_graph_omit_folders();
    if !f.iter().any(|x| store_folder_contains(old, x)) {
        return;
    }
    let mut moved: Vec<String> = f.into_iter().map(|x| if store_folder_contains(old, &x) { format!("{}{}", new, &x[old.len()..]) } else { x }).filter(|x| store_folder_valid(x)).collect();
    moved.sort();
    moved.dedup();
    if save_graph_omit(&moved) != 0 {
        qlog!("store: graph-omit.ini not updated for a folder renamed");
    }
    libindex_note_omit();
}

// A folder that lived only through its notebooks' folder= paths (no folders.ini line: older builds, imports)
// goes when its last notebook moves out or goes to the trash, with no removal: its graph-omit.ini lines go
// with it, so a folder made later with that path starts in the Graph. (Lists the folders, which reads every
// notebook.ini, only while some folder is left out.)
fn graph_omit_prune() {
    let f = store_graph_omit_folders();
    if f.is_empty() {
        return;
    }
    let mut all = Vec::new();
    if store_list_folders(&mut all) != 0 {
        return;
    }
    let kept: Vec<String> = f.iter().filter(|x| all.iter().any(|a| a == *x)).cloned().collect();
    if kept.len() == f.len() {
        return;
    }
    if save_graph_omit(&kept) != 0 {
        qlog!("store: graph-omit.ini not updated for a folder gone");
    }
    libindex_note_omit();
}

/// Logical folder ancestry, with a component boundary (School does not contain Schoolwork).
pub fn store_folder_contains(parent: &str, path: &str) -> bool {
    parent.is_empty() || path == parent || path.strip_prefix(parent).is_some_and(|rest| rest.starts_with('/'))
}
pub fn store_folder_parent(path: &str) -> &str { path.rsplit_once('/').map_or("",|(parent,_)|parent) }
pub fn store_folder_leaf(path: &str) -> &str { path.rsplit('/').next().unwrap_or("") }
pub fn store_folder_valid(path: &str) -> bool {
    !path.is_empty() && path.len() < STORE_NAME_LEN && !path.contains(['\n','\r','\0'])
        && path.split('/').all(|part| !part.is_empty() && part != "." && part != "..")
}
/// Join a user-entered folder name to its parent without silently truncating a destination.
pub fn store_folder_join(parent: &str, name: &str, out: &mut FixedStr<STORE_NAME_LEN>) -> bool {
    if name.contains('/') || name.is_empty() {return false;}
    let path=if parent.is_empty(){name.to_string()}else{format!("{parent}/{name}")};
    if !store_folder_valid(&path){return false;}out.set(&path);true
}

/// Folders exist as notebook.folder values plus root/folders.ini (so empty folders persist).
pub fn store_add_folder(name: &str) -> i32 {
    if name.contains('/') && !store_folder_valid(name){return -1;}
    let mut clean=FixedStr::<STORE_NAME_LEN>::new();clean_line(&mut clean,name.as_bytes());
    if !store_folder_valid(clean.as_str()){return -1;}
    let name=clean.as_str();
    let mut f = load_folders_ini();
    if f.iter().any(|x| x.as_bytes() == name.as_bytes()) {
        return 0;
    }
    let mut nf = FolderName::new();
    clean_line(&mut nf, name.as_bytes());
    f.push(nf);
    save_folders_ini(&f)
}

/// Rename a logical subtree; the page files and notebook identities never move.
/// Validate every destination and keep exact metadata for rollback before the first write.
pub fn store_rename_folder(old: &str, new: &str) -> i32 {
    if !store_folder_valid(old) || !store_folder_valid(new) {
        return -1;
    }
    if old == new {
        return 0;
    }
    if store_folder_contains(old, new) {
        return -1;
    }
    let mut all = Vec::new();
    if store_list_folders(&mut all) != 0 {
        return -1;
    }
    if !all.iter().any(|f| f == old) || all.iter().any(|f| store_folder_contains(new, f)) {
        return -1;
    }
    let mut folders = load_folders_ini();
    for f in &mut folders {
        if store_folder_contains(old, f.as_str()) {
            let path = format!("{}{}", new, &f.as_str()[old.len()..]);
            if !store_folder_valid(&path) {
                return -1;
            }
            f.set(&path);
        }
    }
    let mut books = Vec::new();
    if store_list(&mut books) != 0 {
        return -1;
    }
    let mut changes = Vec::new();
    if changes.try_reserve_exact(books.len()).is_err() {
        return -1;
    }
    for mut nb in books {
        if !store_folder_contains(old, nb.folder.as_str()) {
            continue;
        }
        let dest = format!("{}{}", new, &nb.folder.as_str()[old.len()..]);
        if !store_folder_valid(&dest) {
            return -1;
        }
        let Some(dir) = notebook_dir(nb.id.as_bytes()) else {
            return -1;
        };
        let path = format!("{dir}/notebook.ini");
        let Some(bytes) = read_file(&path) else {
            return -1;
        };
        let before = nb.clone();
        nb.folder.set(&dest);
        changes.push(Rewrite { now: nb, before, path, bytes });
    }
    if commit_rewrite(&changes, &folders) != 0 {
        return -1;
    }
    graph_omit_renamed(old, new);
    0
}

// One notebook's rewrite: what it becomes, what it was, and its notebook.ini's exact bytes for a rollback.
struct Rewrite {
    now: NotebookInfo,
    before: NotebookInfo,
    path: String,
    bytes: Vec<u8>,
}

// Every changed notebook.ini, then folders.ini; a write that fails puts every file written back as it was
// (its exact bytes), the Graph's index too, and returns -1: the tree is as before the call.
fn commit_rewrite(changes: &[Rewrite], folders: &[FolderName]) -> i32 {
    let undo = |n: usize| {
        for c in &changes[..n] {
            write_file_atomic(&c.path, &c.bytes);
            libindex_note_info(&c.before);
        }
    };
    for i in 0..changes.len() {
        if write_info(&changes[i].now) != 0 {
            undo(i);
            return -1;
        }
    }
    if save_folders_ini(folders) != 0 {
        undo(changes.len());
        return -1;
    }
    0
}

/// Whether a folder of exactly this path is there (a folders.ini line, or a notebook's folder= path or an
/// ancestor of one).
pub fn store_folder_exists(path: &str) -> bool {
    let mut all = Vec::new();
    store_list_folders(&mut all) == 0 && all.iter().any(|f| f == path)
}

/// The name Keep both gives a folder that meets one of its own name: path's leaf and " 2" (then " 3" ...),
/// the first that no folder has, in any case of its letters (two would share a mirror directory on a storage
/// that ignores case). None when the path cannot be made or every number is taken.
pub fn store_folder_unique(path: &str) -> Option<String> {
    let mut all = Vec::new();
    if !store_folder_valid(path) || store_list_folders(&mut all) != 0 {
        return None;
    }
    let low: Vec<String> = all.iter().map(|f| f.to_lowercase()).collect();
    (2..1000).map(|k| format!("{path} {k}")).find(|c| store_folder_valid(c) && !low.contains(&c.to_lowercase()))
}

// What a notebook's mirror PDF path comes from: its folder's and its name's safe names, compared without
// case (export_mirror_path), so two notebooks that share one share a file.
fn mirror_key(folder: &str, name: &str) -> (String, String) {
    let (mut f, mut n) = (String::new(), String::new());
    if !folder.is_empty() {
        crate::export::export_safe_name(folder, &mut f, STORE_NAME_LEN);
    }
    crate::export::export_safe_name(name, &mut n, STORE_NAME_LEN);
    (f.to_lowercase(), n.to_lowercase())
}

/// Merge folder `old` (with all it holds) into the folder `new` that is already there: a logical rewrite of
/// the folder prefix in every notebook.ini and folders.ini line in one pass, with store_rename_folder's
/// rollback (a failed write puts every file back as it was). Nothing moves on the disk and no notebook is
/// ever replaced: one that would share a mirror PDF path with another there is named "Name 2" (then " 3"
/// ...), the clashing ones in name order. Graph-omit lines follow as for a move. -1 (and nothing changed)
/// when either folder is not there, old is new, or new is inside old.
pub fn store_merge_folder(old: &str, new: &str) -> i32 {
    if !store_folder_valid(old) || !store_folder_valid(new) || old == new || store_folder_contains(old, new) {
        return -1;
    }
    let mut all = Vec::new();
    if store_list_folders(&mut all) != 0 || !all.iter().any(|f| f == old) || !all.iter().any(|f| f == new) {
        return -1;
    }
    let mut folders: Vec<FolderName> = Vec::new();
    for mut f in load_folders_ini() {
        if store_folder_contains(old, f.as_str()) {
            let path = format!("{}{}", new, &f.as_str()[old.len()..]);
            if !store_folder_valid(&path) {
                return -1;
            }
            f.set(&path);
        }
        if !folders.iter().any(|x| x.as_bytes() == f.as_bytes()) {
            folders.push(f);
        }
    }
    let mut books = Vec::new();
    if store_list(&mut books) != 0 {
        return -1;
    }
    // The paths the notebooks that stay hold; the ones that come take theirs as they are free.
    let mut taken: Vec<(String, String)> =
        books.iter().filter(|nb| !store_folder_contains(old, nb.folder.as_str())).map(|nb| mirror_key(nb.folder.as_str(), nb.name.as_str())).collect();
    let mut coming: Vec<NotebookInfo> = books.into_iter().filter(|nb| store_folder_contains(old, nb.folder.as_str())).collect();
    coming.sort_by(|a, b| name_cmp(a.name.as_str(), b.name.as_str()).then_with(|| a.id.as_str().cmp(b.id.as_str())));
    let mut changes = Vec::new();
    if changes.try_reserve_exact(coming.len()).is_err() {
        return -1;
    }
    for mut nb in coming {
        let dest = format!("{}{}", new, &nb.folder.as_str()[old.len()..]);
        if !store_folder_valid(&dest) {
            return -1;
        }
        let before = nb.clone();
        nb.folder.set(&dest);
        let base = nb.name.as_str().to_string();
        let mut name = base.clone();
        let mut k = 2;
        while taken.contains(&mirror_key(&dest, &name)) {
            if k >= 1000 {
                return -1;
            }
            let suffix = format!(" {k}");
            let mut cut = base.len().min(STORE_NAME_LEN - 1 - suffix.len());
            while !base.is_char_boundary(cut) {
                cut -= 1;
            }
            name = format!("{}{suffix}", &base[..cut]);
            k += 1;
        }
        taken.push(mirror_key(&dest, &name));
        nb.name.set(&name);
        let Some(dir) = notebook_dir(nb.id.as_bytes()) else {
            return -1;
        };
        let path = format!("{dir}/notebook.ini");
        let Some(bytes) = read_file(&path) else {
            return -1;
        };
        changes.push(Rewrite { now: nb, before, path, bytes });
    }
    if commit_rewrite(&changes, &folders) != 0 {
        return -1;
    }
    graph_omit_renamed(old, new);
    0
}

/// only from folders.ini; notebooks keep theirs
pub fn store_remove_folder(name: &str) -> i32 {
    let mut f = load_folders_ini();
    f.retain(|x| x.as_bytes() != name.as_bytes());
    let rc = save_folders_ini(&f);
    if rc == 0 && store_graph_omit_folders().iter().any(|x| x == name) {
        // (an empty folder gone: its Graph line too, so a new folder of that name starts in the Graph)
        store_set_folder_graph_omit(name, false);
    }
    rc
}

// Folder names in the order a reader expects, accents or not: each letter compared as its base
// letter (lowercase, accents dropped; ß as ss, æ as ae, œ as oe, ĳ as ij, þ as th), so Álgebra
// and Études sort among the A's and E's instead of after Z. Other characters compare as their
// bytes. Equal that way, the names' own bytes decide, so the order never depends on the input's.
// (code point from, to, base letters)
const K_FOLD: [(u16, u16, &str); 48] = [
    (0xC0, 0xC5, "a"),
    (0xC6, 0xC6, "ae"),
    (0xC7, 0xC7, "c"),
    (0xC8, 0xCB, "e"),
    (0xCC, 0xCF, "i"),
    (0xD0, 0xD0, "d"),
    (0xD1, 0xD1, "n"),
    (0xD2, 0xD6, "o"),
    (0xD8, 0xD8, "o"),
    (0xD9, 0xDC, "u"),
    (0xDD, 0xDD, "y"),
    (0xDE, 0xDE, "th"),
    (0xDF, 0xDF, "ss"),
    (0xE0, 0xE5, "a"),
    (0xE6, 0xE6, "ae"),
    (0xE7, 0xE7, "c"),
    (0xE8, 0xEB, "e"),
    (0xEC, 0xEF, "i"),
    (0xF0, 0xF0, "d"),
    (0xF1, 0xF1, "n"),
    (0xF2, 0xF6, "o"),
    (0xF8, 0xF8, "o"),
    (0xF9, 0xFC, "u"),
    (0xFD, 0xFD, "y"),
    (0xFE, 0xFE, "th"),
    (0xFF, 0xFF, "y"),
    (0x100, 0x105, "a"),
    (0x106, 0x10D, "c"),
    (0x10E, 0x111, "d"),
    (0x112, 0x11B, "e"),
    (0x11C, 0x123, "g"),
    (0x124, 0x127, "h"),
    (0x128, 0x131, "i"),
    (0x132, 0x133, "ij"),
    (0x134, 0x135, "j"),
    (0x136, 0x138, "k"),
    (0x139, 0x142, "l"),
    (0x143, 0x14B, "n"),
    (0x14C, 0x151, "o"),
    (0x152, 0x153, "oe"),
    (0x154, 0x159, "r"),
    (0x15A, 0x161, "s"),
    (0x162, 0x167, "t"),
    (0x168, 0x173, "u"),
    (0x174, 0x175, "w"),
    (0x176, 0x178, "y"),
    (0x179, 0x17E, "z"),
    (0x17F, 0x17F, "s"),
];

/// The next character of s folded for sorting into out (up to 4 bytes, then a 0); returns its
/// length in s.
pub(crate) fn fold_next(s: &[u8], out: &mut [u8; 5]) -> usize {
    let u = |i: usize| s.get(i).copied().unwrap_or(0);
    if u(0) < 0x80 {
        out[0] = if u(0).is_ascii_uppercase() {
            u(0) + 32
        } else {
            u(0)
        };
        out[1] = 0;
        return 1;
    }
    let mut len = if (u(0) & 0xE0) == 0xC0 {
        2
    } else if (u(0) & 0xF0) == 0xE0 {
        3
    } else if (u(0) & 0xF8) == 0xF0 {
        4
    } else {
        1
    };
    let mut i = 1;
    while i < len {
        if (u(i) & 0xC0) != 0x80 {
            len = 1; // broken: the lead byte alone, as it is
        }
        i += 1;
    }
    if len == 2 {
        let cp = ((u(0) & 0x1F) as u32) << 6 | (u(1) & 0x3F) as u32;
        if (0x300..=0x36F).contains(&cp) {
            // a combining accent (a decomposed é): dropped
            out[0] = 0;
            return 2;
        }
        for &(lo, hi, base) in K_FOLD.iter() {
            if cp >= lo as u32 && cp <= hi as u32 {
                out[..base.len()].copy_from_slice(base.as_bytes());
                out[base.len()] = 0;
                return 2;
            }
        }
    }
    out[..len].copy_from_slice(&s[..len]);
    out[len] = 0;
    len
}

fn cmp_name(a: &[u8], b: &[u8]) -> Ordering {
    let (mut x, mut y) = (a, b);
    let mut fx = [0u8; 5];
    let mut fy = [0u8; 5];
    // what is left of each name's current folded character
    let (mut px, mut py) = (0usize, 0usize);
    loop {
        while fx[px] == 0 && !x.is_empty() {
            let n = fold_next(x, &mut fx);
            x = &x[n..];
            px = 0;
        }
        while fy[py] == 0 && !y.is_empty() {
            let n = fold_next(y, &mut fy);
            y = &y[n..];
            py = 0;
        }
        if fx[px] == 0 || fy[py] == 0 {
            if fx[px] != fy[py] {
                // one ran out first: it goes first
                return if fx[px] != 0 {
                    Ordering::Greater
                } else {
                    Ordering::Less
                };
            }
            break;
        }
        if fx[px] != fy[py] {
            return if fx[px] < fy[py] {
                Ordering::Less
            } else {
                Ordering::Greater
            };
        }
        px += 1;
        py += 1;
    }
    a.cmp(b)
}

/// The order of names in lists people read (DESIGN.md, Name order): cmp_name's letters (each compared as its
/// base letter, lowercase, accents dropped) with natural_cmp's numbers (a run of digits compared by its value,
/// so "Álgebra 2" goes before "Álgebra 10"). Equal that way ("a01" and "a1", "Arbol" and "Árbol"), the names'
/// own bytes decide, so it is a total order that never depends on the input's. For the file browser, the
/// order of a PDF import and the font list; the Library's folders keep cmp_name, and the pictures of an
/// import folder keep natural_cmp (bgwork_template.rs), whose order the Kindle's caches already hold.
pub fn name_cmp(a: &str, b: &str) -> Ordering {
    let (mut x, mut y) = (a.as_bytes(), b.as_bytes());
    let mut fx = [0u8; 5];
    let mut fy = [0u8; 5];
    // what is left of each name's current folded character
    let (mut px, mut py) = (0usize, 0usize);
    let digit = |s: &[u8]| s.first().is_some_and(|c| c.is_ascii_digit());
    // A run of digits: its leading zeros, then its other digits.
    let run = |s: &[u8]| {
        let z = s.iter().take_while(|&&c| c == b'0').count();
        (z, s[z..].iter().take_while(|c| c.is_ascii_digit()).count())
    };
    loop {
        // the next letters, up to a digit (a combining accent folds to nothing and is passed)
        while fx[px] == 0 && !x.is_empty() && !digit(x) {
            let n = fold_next(x, &mut fx);
            x = &x[n..];
            px = 0;
        }
        while fy[py] == 0 && !y.is_empty() && !digit(y) {
            let n = fold_next(y, &mut fy);
            y = &y[n..];
            py = 0;
        }
        if fx[px] == 0 && fy[py] == 0 && digit(x) && digit(y) {
            let ((zx, nx), (zy, ny)) = (run(x), run(y));
            // more digits (past the zeros): a bigger number; as many: the digits decide
            let c = nx.cmp(&ny).then_with(|| x[zx..zx + nx].cmp(&y[zy..zy + ny]));
            if c != Ordering::Equal {
                return c;
            }
            x = &x[zx + nx..];
            y = &y[zy + ny..];
            continue;
        }
        // a digit on one side only: compared as the character it is
        if fx[px] == 0 && !x.is_empty() {
            let n = fold_next(x, &mut fx);
            x = &x[n..];
            px = 0;
        }
        if fy[py] == 0 && !y.is_empty() {
            let n = fold_next(y, &mut fy);
            y = &y[n..];
            py = 0;
        }
        if fx[px] == 0 || fy[py] == 0 {
            if fx[px] != fy[py] {
                // one ran out first: it goes first
                return if fx[px] != 0 { Ordering::Greater } else { Ordering::Less };
            }
            break;
        }
        if fx[px] != fy[py] {
            return fx[px].cmp(&fy[py]);
        }
        px += 1;
        py += 1;
    }
    a.as_bytes().cmp(b.as_bytes())
}

/// distinct non-empty folders, sorted
pub fn store_list_folders(out: &mut Vec<String>) -> i32 {
    let mut nbs = Vec::new();
    store_list(&mut nbs);
    store_list_folders_from(&nbs, out)
}

/// The same from the notebooks a caller has just listed (store_list): the Library's reload reads each
/// notebook.ini once, not once for its cards and again for its folders (browser.md 9).
pub fn store_list_folders_from(nbs: &[NotebookInfo], out: &mut Vec<String>) -> i32 {
    let f = load_folders_ini();
    let mut all: Vec<FolderName> = Vec::new();
    for i in 0..f.len() + nbs.len() {
        let name = if i < f.len() {
            &f[i]
        } else {
            &nbs[i - f.len()].folder
        };
        if name.is_empty() {
            continue;
        }
        // Include implicit ancestors of notebooks and explicit empty subfolders, so every path
        // is reachable even when an older build wrote only its leaf path.
        let mut path = name.as_str();
        loop {
            if !all.iter().any(|x| x.as_bytes() == path.as_bytes()) {
                if all.try_reserve(1).is_err() {return -1;}
                all.push(FolderName::from_str(path));
            }
            let parent=store_folder_parent(path);
            if parent.is_empty(){break;}path=parent;
        }
    }
    all.sort_by(|x, y| cmp_name(x.as_bytes(), y.as_bytes()));
    *out = all.iter().map(|f| f.as_str().to_string()).collect();
    0
}

// ---- pages ----

/// A new page's id: the next one, but never one whose page file is there already (one the page list
/// does not name, as after a page list was rebuilt): the next free id instead. 0 when none is found.
fn take_page_id(nb: &NotebookInfo) -> u32 {
    let mut pid = nb.next_page_id;
    let mut tries = 0;
    loop {
        if pid == 0 || pid == u32::MAX {
            return 0;
        }
        let path = page_path(nb, pid);
        if let Err(e) = std::fs::symlink_metadata(&path) {
            return if e.raw_os_error() == Some(libc::ENOENT) {
                pid
            } else {
                0
            };
        }
        if tries == 64 {
            return 0;
        }
        qlog_once!(
            "store: %s: p%u.qpg is not in the page list; left as it is",
            &nb.id,
            pid
        );
        tries += 1;
        pid += 1;
    }
}

/// Page pid, its file written, goes into the page list at index at. If the list cannot be saved,
/// roll back the insertion and remove only the new file, so a later save cannot add it by accident.
fn list_page(nb: &mut NotebookInfo, at: i32, pid: u32) -> i32 {
    let next = nb.next_page_id;
    let modified = nb.modified;
    nb.page_ids.insert(at as usize, pid);
    nb.next_page_id = pid + 1;
    if store_save_info(nb) == 0 {
        return 0;
    }
    nb.page_ids.remove(at as usize);
    nb.next_page_id = next;
    nb.modified = modified;
    let path = page_path(nb, pid);
    let _ = std::fs::remove_file(&path);
    -1
}

/// Pages. Index is the position in nb->page_ids.
/// Writes an empty page at the notebook's page size (never over a page file already there: the
/// next free id) + info. Its paper is store_new_page_paper(nb, paper): callers pass the paper of
/// the page it follows (or replaces).
pub fn store_add_page(nb: &mut NotebookInfo, at_index: i32, paper: PaperKind) -> i32 {
    if nb.page_ids.len() >= STORE_MAX_PAGES {
        return -1;
    }
    let at = clampi(at_index, 0, nb.npages());
    let pid = take_page_id(nb);
    if pid == 0 {
        return -1;
    }
    let (mut w, mut h) = (0, 0);
    store_notebook_page_size(nb, &mut w, &mut h);
    let paper = store_new_page_paper(nb, paper); // never another page's document picture
    let mut p = page_new(w, h, paper);
    p.nb.set_bytes(nb.id.as_bytes());
    let rc = write_page_file(nb, pid, &p);
    drop(p);
    if rc != 0 {
        return -1;
    }
    if list_page(nb, at, pid) != 0 {
        return -1;
    }
    side_note(nb, pid, paper);
    0
}

/// Blank pages a batch added at once (the Pages view's held +, 2.0.0-beta.2), to take them back: the
/// notebook's order before and after, the ids added (in order), and the bytes each was written with
/// (one blank page, the same for all), and their paper (bg/pages.ini lists an image paper's pages). Kept by the
/// Pages view, never written to disk; it goes with the notebook's closing. Its Redo (store_redo_add_pages)
/// writes the same pages again from it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PageAddUndo {
    pub notebook: String,
    pub before: Vec<u32>,
    pub after: Vec<u32>,
    pub added: Vec<u32>,
    pub blank: Vec<u8>,
    pub paper: PaperKind,
}

impl PageAddUndo {
    /// Whether it still undoes the newest change to nb's pages: the same notebook, its pages still the
    /// list the batch left (a page added, deleted, copied or moved since changed it).
    pub fn applies(&self, nb: &NotebookInfo) -> bool {
        self.notebook.as_str() == nb.id.as_str() && self.after == nb.page_ids
    }
}

/// How many more pages notebook nb can take (STORE_MAX_PAGES in all).
pub fn store_room_for_pages(nb: &NotebookInfo) -> i32 {
    STORE_MAX_PAGES.saturating_sub(nb.page_ids.len()) as i32
}

/// `n` blank pages at once at index `at_index` (clamped as store_add_page clamps it), each as
/// store_add_page writes one (the notebook's page size, paper store_new_page_paper(nb, paper), an id
/// whose file is not there already): the page files first, then bg/pages.ini once when the paper is an
/// image paper, then notebook.ini once, which lists them all. A failure takes back every file it wrote
/// and leaves nb as it was: no page is added (never some of them). -1 for n < 1, more pages than
/// store_room_for_pages, or a file that cannot be written; 0 with `undo` the record that takes them
/// back (store_undo_add_pages).
pub fn store_add_pages(nb: &mut NotebookInfo, at_index: i32, n: i32, paper: PaperKind, undo: &mut Option<PageAddUndo>) -> i32 {
    if n < 1 || n > store_room_for_pages(nb) {
        return -1;
    }
    let at = clampi(at_index, 0, nb.npages()) as usize;
    let (mut w, mut h) = (0, 0);
    store_notebook_page_size(nb, &mut w, &mut h);
    let paper = store_new_page_paper(nb, paper); // never another page's document picture
    let mut p = page_new(w, h, paper);
    p.nb.set_bytes(nb.id.as_bytes());
    let mut blank = Vec::new();
    if page_serialize(&p, &mut blank) != 0 {
        return -1;
    }
    drop(p);
    let mut added: Vec<u32> = Vec::new();
    // (a failure takes back every page file this batch wrote, and says nothing was added)
    macro_rules! bail {
        () => {{
            for &pid in &added {
                let _ = std::fs::remove_file(page_path(nb, pid));
            }
            return -1;
        }};
    }
    if added.try_reserve(n as usize).is_err() {
        return -1;
    }
    let mut next = nb.next_page_id;
    for _ in 0..n {
        // (each page is an fsync'd write: a long batch keeps the pens read and the launcher's heartbeat
        // going, as Copy to notebook... does)
        work_pump();
        // The next free id from `next` on (take_page_id's rule, without touching nb until the end).
        let saved = nb.next_page_id;
        nb.next_page_id = next;
        let pid = take_page_id(nb);
        nb.next_page_id = saved;
        if pid == 0 || write_file_atomic(&page_path(nb, pid), &blank) != 0 {
            bail!();
        }
        added.push(pid);
        next = pid + 1;
    }
    let mut bgdir = String::new();
    if paper_is_image(paper)
        && (bgimage_dir(nb.id.as_str(), &mut bgdir, 700) != 0 || store_note_papers_in(&bgdir, &added, &vec![paper; added.len()]) != 0)
    {
        bail!();
    }
    let was = nb.clone();
    nb.page_ids.splice(at..at, added.iter().copied());
    nb.next_page_id = next;
    if store_save_info(nb) != 0 {
        *nb = was;
        // Unlisted again (those pages are not there), in one write of bg/pages.ini as they were listed.
        if paper_is_image(paper) && store_note_papers_in(&bgdir, &added, &vec![PAPER_BLANK; added.len()]) != 0 {
            qlog!("store: %s: bg/pages.ini still lists the %d pages a failed batch took back", &nb.id, added.len() as i32);
        }
        bail!();
    }
    *undo = Some(PageAddUndo { notebook: nb.id.as_str().to_string(), before: was.page_ids, after: nb.page_ids.clone(), added, blank, paper });
    0
}

/// Redoes a batch store_undo_add_pages took back (2.0.0-beta.2, the owner's ruling of 2026-11-03): the same
/// pages again, with the same ids, at the same places (`u.after`), each file written with the bytes it had
/// (the blank page the batch wrote: the undo took them back only while they were still that), an image
/// paper listed again in bg/pages.ini, then notebook.ini once. last_page follows its page. -1 with nb as it
/// was (and no file left behind) when `u` is not for nb as it is now (another notebook, or its pages are
/// not the list from before the batch: a change since), one of its ids is listed or has a file that is
/// not that blank page, or a file cannot be written.
pub fn store_redo_add_pages(nb: &mut NotebookInfo, u: &PageAddUndo) -> i32 {
    if u.notebook.as_str() != nb.id.as_str() || nb.page_ids != u.before || u.added.is_empty() {
        return -1;
    }
    if u.added.len() > store_room_for_pages(nb) as usize || u.added.iter().any(|p| nb.page_ids.contains(p)) {
        return -1;
    }
    let rest: Vec<u32> = u.after.iter().copied().filter(|p| !u.added.contains(p)).collect();
    if rest != u.before || u.after.len() != u.before.len() + u.added.len() {
        return -1; // (not the batch's lists: never from a batch)
    }
    // A file left where one of them was (its removal failed) is used only when it is still that blank page.
    let mut written: Vec<u32> = Vec::new();
    macro_rules! bail {
        () => {{
            for &pid in &written {
                let _ = std::fs::remove_file(page_path(nb, pid));
            }
            return -1;
        }};
    }
    for &pid in &u.added {
        work_pump(); // (each page is an fsync'd write, as the batch's were)
        let path = page_path(nb, pid);
        match std::fs::read(&path) {
            Ok(d) if d == u.blank => continue,
            Ok(_) => bail!(),
            Err(e) if e.raw_os_error() != Some(libc::ENOENT) => bail!(),
            Err(_) => {}
        }
        if write_file_atomic(&path, &u.blank) != 0 {
            bail!();
        }
        written.push(pid);
    }
    let mut bgdir = String::new();
    if paper_is_image(u.paper)
        && (bgimage_dir(nb.id.as_str(), &mut bgdir, 700) != 0 || store_note_papers_in(&bgdir, &u.added, &vec![u.paper; u.added.len()]) != 0)
    {
        bail!();
    }
    let was = nb.clone();
    let open = nb.page_ids.get(nb.last_page.max(0) as usize).copied();
    nb.page_ids = u.after.clone();
    if let Some(i) = open.and_then(|pid| nb.page_ids.iter().position(|&p| p == pid)) {
        nb.last_page = i as i32;
    }
    let past = u.added.iter().copied().max().unwrap_or(0).saturating_add(1);
    nb.next_page_id = nb.next_page_id.max(past);
    if store_save_info(nb) != 0 {
        *nb = was;
        if paper_is_image(u.paper) && store_note_papers_in(&bgdir, &u.added, &vec![PAPER_BLANK; u.added.len()]) != 0 {
            qlog!("store: %s: bg/pages.ini still lists the %d pages a failed redo took back", &nb.id, u.added.len() as i32);
        }
        bail!();
    }
    0
}

/// The paper of page `index` as a page added after it takes it (store_new_page_paper does the rest): its
/// file's paper read from the file's start, without its strokes (a dense page is seconds of reading on a
/// Kindle), an image paper an older version saved as Blank given back from bg/pages.ini as an open gives it
/// back; the notebook's paper for a page whose file is missing, does not read or is from a newer version
/// (shown blank, its paper unknown), as an open shows it.
pub fn store_page_paper(nb: &NotebookInfo, index: i32) -> PaperKind {
    let Some(&pid) = nb.page_ids.get(index.max(0) as usize).filter(|_| index >= 0) else { return nb.paper };
    let mut b = [0u8; 20];
    let Ok(mut f) = std::fs::File::open(page_path(nb, pid)) else { return nb.paper };
    let mut got = 0;
    while got < b.len() {
        match f.read(&mut b[got..]) {
            Ok(0) | Err(_) => break,
            Ok(n) => got += n,
        }
    }
    if got < 16 || &b[..4] != b"QPG1" {
        return nb.paper;
    }
    let ver = u16::from_le_bytes([b[4], b[5]]);
    let paper = match ver {
        1 => u16::from_le_bytes([b[6], b[7]]) as u32,
        2 if got >= 20 => u32::from_le_bytes([b[16], b[17], b[18], b[19]]),
        _ => return nb.paper,
    };
    let mut p = page_new(1, 1, PaperKind(paper));
    side_restore(nb, pid, &mut p);
    p.paper
}

/// Whether the pages a batch added are still as it wrote them: each file byte for byte the blank page
/// it was (or not there at all, which holds nothing either). One page drawn on, even with the ink erased
/// again, is not.
pub fn store_added_pages_untouched(nb: &NotebookInfo, u: &PageAddUndo) -> bool {
    u.added.iter().all(|&pid| match std::fs::read(page_path(nb, pid)) {
        Ok(d) => d == u.blank,
        Err(e) => e.raw_os_error() == Some(libc::ENOENT),
    })
}

/// Takes back the pages a batch added (store_add_pages): notebook.ini written with the list as it was
/// before the batch, last_page following its page (the page before the batch when it named one of
/// them), then their files removed (a file that stays is never listed again: take_page_id passes it).
/// -1 with nb as it was when `u` no longer applies (PageAddUndo::applies), a page it added is no longer
/// blank (store_added_pages_untouched), or notebook.ini could not be written.
pub fn store_undo_add_pages(nb: &mut NotebookInfo, u: &PageAddUndo) -> i32 {
    if !u.applies(nb) || !store_added_pages_untouched(nb, u) {
        return -1;
    }
    let rest: Vec<u32> = nb.page_ids.iter().copied().filter(|p| !u.added.contains(p)).collect();
    if rest != u.before {
        return -1; // (not the batch's list: never from a batch)
    }
    let was = std::mem::replace(&mut nb.page_ids, rest);
    let (last, modified) = (nb.last_page, nb.modified);
    let open = was.get(nb.last_page.max(0) as usize).copied().unwrap_or(0);
    nb.last_page = match nb.page_ids.iter().position(|&p| p == open) {
        Some(i) => i as i32,
        None => {
            // one of the pages taken back: the page before them
            let first = was.iter().position(|p| u.added.contains(p)).unwrap_or(0) as i32;
            clampi(first - 1, 0, nb.npages() - 1)
        }
    };
    if store_save_info(nb) != 0 {
        nb.page_ids = was;
        nb.last_page = last;
        nb.modified = modified;
        return -1;
    }
    for &pid in &u.added {
        let _ = std::fs::remove_file(page_path(nb, pid));
        side_note(nb, pid, PAPER_BLANK); // (an image paper's line goes with its page)
    }
    0
}

/// A copy of page `index` as a new page at `at` (an id as store_add_page takes one): the page file's
/// bytes as they are, ink and paper (a read-only page's too), or for a page whose file is missing,
/// an empty page of the notebook's paper, as it shows. An unreadable or damaged file refuses the
/// copy. A copy of a page on an image
/// paper shares its picture (listed in bg/pages.ini under the copy's id too). -1 at STORE_MAX_PAGES,
/// or when a file cannot be written.
pub fn store_duplicate_page(nb: &mut NotebookInfo, index: i32, at: i32) -> i32 {
    if nb.page_ids.len() >= STORE_MAX_PAGES || index < 0 || index >= nb.npages() {
        return -1;
    }
    let at = clampi(at, 0, nb.npages());
    let path = page_path(nb, nb.page_ids[index as usize]);
    let st = std::fs::metadata(&path);
    let missing = st.is_err();
    if let Err(e) = &st {
        if e.raw_os_error() != Some(libc::ENOENT) {
            return -1;
        }
    }
    if let Ok(m) = &st {
        if !m.is_file() {
            return -1;
        }
    }
    let mut data: Option<Vec<u8>> = if missing { None } else { read_file(&path) };
    let mut p: Option<Box<Page>> = None;
    // Only an absent page means blank. A read error or damaged file must not look like a
    // successful blank copy; a newer page that reads as read-only is copied byte for byte.
    if !missing {
        let size_matches = match (&data, &st) {
            (Some(d), Ok(m)) => m.size() == d.len() as u64,
            _ => false,
        };
        if data.is_none()
            || !size_matches
            || page_deserialize(data.as_deref().unwrap_or(&[]), &mut p) != 0
        {
            return -1;
        }
    }
    if data.is_none() {
        let (mut w, mut h) = (0, 0);
        store_notebook_page_size(nb, &mut w, &mut h);
        p = Some(page_new(w, h, store_new_page_paper(nb, nb.paper)));
    } else if let Some(pg) = p.as_deref_mut() {
        if side_restore(nb, nb.page_ids[index as usize], pg) {
            data = None; // its picture, which an older version's save left out: the copy written with it
        }
    }
    let Some(p) = p else { return -1 };
    let pid = take_page_id(nb);
    let mut rc = -1;
    if pid != 0 {
        let copy = page_path(nb, pid);
        rc = match &data {
            Some(d) => write_file_atomic(&copy, d),
            None => write_page_at(&copy, &p),
        };
    }
    let paper = p.paper;
    drop(p);
    if rc != 0 {
        return -1;
    }
    if list_page(nb, at, pid) != 0 {
        return -1;
    }
    side_note(nb, pid, paper); // a copy of a picture's page has the same picture
    0
}

/// keeps >= 1 page; file moved to trash/
pub fn store_delete_page(nb: &mut NotebookInfo, index: i32) -> i32 {
    if nb.page_ids.len() <= 1 || index < 0 || index >= nb.npages() {
        return -1;
    }
    let pid = nb.page_ids[index as usize];
    let path = page_path(nb, pid);
    let mut base = String::new();
    csnprintf!(&mut base, 80, "%s-p%u", &nb.id, pid);
    if access_ok(&path) {
        move_to_trash(&path, &base);
    }
    side_note(nb, pid, PAPER_BLANK); // a deleted page has no paper to list
    nb.page_ids.remove(index as usize);
    nb.last_page = clampi(nb.last_page, 0, nb.npages() - 1);
    store_save_info(nb)
}

/// Where `store_move_pages` puts the pages moved: `to` names a place in the order before the move
/// (before page `to`, or `npages` for the end). The list without the moved pages, and the index in
/// it they go to: before the first page from `to` on that is not moved, else at the end.
fn move_order(ids: &[u32], moved: &[bool], to: usize) -> (Vec<u32>, usize) {
    let rest: Vec<u32> = ids.iter().zip(moved).filter(|(_, &m)| !m).map(|(&p, _)| p).collect();
    let at = (0..ids.len()).filter(|&i| !moved[i]).position(|i| i >= to).unwrap_or(rest.len());
    (rest, at)
}

/// The order `store_move_pages` makes of `ids`: pages `indices` (positions in ids, any order, each once)
/// together, in the order they had, before page `to` (`to` == ids.len(): at the end). None for bad
/// input. The Pages view asks it whether a drop changes anything before it asks for the move.
pub fn store_move_order(ids: &[u32], indices: &[i32], to: i32) -> Option<Vec<u32>> {
    let n = ids.len();
    if indices.is_empty() || to < 0 || to as usize > n {
        return None;
    }
    let mut moved = vec![false; n];
    for &i in indices {
        if i < 0 || i as usize >= n || moved[i as usize] {
            return None;
        }
        moved[i as usize] = true;
    }
    let (mut order, at) = move_order(ids, &moved, to as usize);
    let block: Vec<u32> = (0..n).filter(|&i| moved[i]).map(|i| ids[i]).collect();
    order.splice(at..at, block);
    Some(order)
}

// nb's pages put in `order` (the same ids in another order) and notebook.ini written: the one path a
// move and its undo share. last_page follows the page it named. 0, or -1 with nb as it was.
fn reorder_pages(nb: &mut NotebookInfo, order: Vec<u32>) -> i32 {
    let was = std::mem::replace(&mut nb.page_ids, order);
    let (last, modified) = (nb.last_page, nb.modified);
    let open = was.get(nb.last_page.max(0) as usize).copied();
    if let Some(i) = open.and_then(|pid| nb.page_ids.iter().position(|&p| p == pid)) {
        nb.last_page = i as i32;
    }
    if store_save_info(nb) != 0 {
        nb.page_ids = was;
        nb.last_page = last;
        nb.modified = modified;
        return -1;
    }
    0
}

/// Moves pages `indices` (positions in nb.page_ids, any order, each once) so that they stand together,
/// in the order they had, before page `to` as the pages are now (`to` == npages: at the end). A `to`
/// that is one of the pages moved means the first page after it that is not moved. Only the page list
/// changes: links, names, book pages, the map and bg/pages.ini name pages by id, never by place, so no
/// other file is written (docs/FORMAT.md, notebook.ini). last_page follows the page it named. 0, or -1
/// (bad indices, or notebook.ini could not be written: then nb is as it was). No change (the pages
/// already stand there) writes nothing and returns 0.
pub fn store_move_pages(nb: &mut NotebookInfo, indices: &[i32], to: i32) -> i32 {
    let mut undo = None;
    store_move_pages_undo(nb, indices, to, &mut undo)
}

/// The last page move of a notebook, to undo it (2.0.0-beta.2): its order before and after. Kept by the
/// Pages view, never written to disk; it goes with the notebook's closing.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PageMoveUndo {
    pub notebook: String,
    pub before: Vec<u32>,
    pub after: Vec<u32>,
}

impl PageMoveUndo {
    /// Whether it still undoes the newest change to nb's pages: the same notebook, its pages still in
    /// the order the move left (a page added, deleted or copied since, or another move, changed it).
    pub fn applies(&self, nb: &NotebookInfo) -> bool {
        self.notebook.as_str() == nb.id.as_str() && self.after == nb.page_ids
    }
}

/// store_move_pages, and when the order changed, `undo` the record that takes it back
/// (store_undo_move_pages). No change, or a failure, leaves `undo` as it was.
pub fn store_move_pages_undo(nb: &mut NotebookInfo, indices: &[i32], to: i32, undo: &mut Option<PageMoveUndo>) -> i32 {
    let Some(order) = store_move_order(&nb.page_ids, indices, to) else { return -1 };
    if order == nb.page_ids {
        return 0;
    }
    let before = nb.page_ids.clone();
    if reorder_pages(nb, order) != 0 {
        return -1;
    }
    *undo = Some(PageMoveUndo { notebook: nb.id.as_str().to_string(), before, after: nb.page_ids.clone() });
    0
}

/// Undoes a page move: nb's pages back in the order `u` had before it, through the same path as the
/// move (notebook.ini; nothing else names pages by place), last_page following its page. -1 with nb as
/// it was when `u` no longer applies (PageMoveUndo::applies: another notebook, or its pages changed
/// since) or notebook.ini could not be written.
pub fn store_undo_move_pages(nb: &mut NotebookInfo, u: &PageMoveUndo) -> i32 {
    if !u.applies(nb) {
        return -1;
    }
    let mut a = u.before.clone();
    let mut b = nb.page_ids.clone();
    a.sort_unstable();
    b.sort_unstable();
    if a != b {
        return -1; // (not the same pages: never from a move)
    }
    reorder_pages(nb, u.before.clone())
}

/// Redoes a page move store_undo_move_pages took back (2.0.0-beta.2, the owner's ruling of 2026-11-03): nb's
/// pages in the exact order the move left (`u.after`), through the same path, last_page following its page.
/// -1 with nb as it was when nb is not as the undo left it (another notebook, or its pages changed since) or
/// notebook.ini could not be written.
pub fn store_redo_move_pages(nb: &mut NotebookInfo, u: &PageMoveUndo) -> i32 {
    if u.notebook.as_str() != nb.id.as_str() || nb.page_ids != u.before {
        return -1;
    }
    let mut a = u.after.clone();
    let mut b = nb.page_ids.clone();
    a.sort_unstable();
    b.sort_unstable();
    if a != b {
        return -1; // (not the same pages: never from a move)
    }
    reorder_pages(nb, u.after.clone())
}

/// A page Copy to notebook... made: the source page's id, its copy's in the other notebook, and
/// whether the copy holds erase strokes (to list for compaction there).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PageCopy {
    pub from: u32,
    pub to: u32,
    pub erased: bool,
}

// A picture copied into another notebook's bg/: the source's key when its index.ini names one, else a
// key of the picture's bytes of a kind of its own (or, for a picture that is not there, of the
// notebook and id it was), so copying the same picture twice gives one file there.
fn copied_picture_key(src_ix: &BgIndex, src_id: &str, pic: u32, data: Option<&[u8]>) -> u64 {
    if let Some(e) = src_ix.e.iter().find(|e| e.id == pic) {
        return e.key;
    }
    let mut h: u64 = 0xc0f1_ed00_9a9e_5eed;
    match data {
        Some(d) => h ^= bgimage_content_hash(d).rotate_left(17) ^ d.len() as u64,
        None => {
            for &b in src_id.as_bytes() {
                h = (h ^ b as u64).wrapping_mul(0x100000001b3);
            }
            h ^= pic as u64;
        }
    }
    h.wrapping_mul(0x9e37_79b9_7f4a_7c15)
}

/// Copy to notebook...: pages `pids` of notebook `src` (in that order, each once) copied into notebook
/// `dst` (another notebook, its info as read) before its page `at` (dst.npages: at the end), each a new
/// page there with an id of its own. A page file is copied byte for byte, a read-only one too, as
/// store_duplicate_page copies (a missing file: an empty page of the source's paper; one that does not
/// read refuses the copy). A page of another size than dst's keeps its own size in its file and is
/// scaled when dst opens it (scale_page, docs/FORMAT.md Page size). A page on an image paper (a
/// template, a photo, an imported or book page) takes its picture along into dst's bg/: under the
/// same id when dst has it free (or holds the same picture there), else under a new one, and then the
/// copy's paper value (bytes 16 to 19) is the only change to its bytes. A read-only page whose picture
/// would need a new id is refused. Then dst's bg/index.ini and bg/pages.ini, and last its notebook.ini,
/// which lists the copies. 0 with `out` (one PageCopy per page, in order) and dst updated; -1 with dst
/// as it was and no page file left behind.
pub fn store_copy_pages_to(src: &NotebookInfo, pids: &[u32], dst: &mut NotebookInfo, at: i32, out: &mut Vec<PageCopy>) -> i32 {
    out.clear();
    if pids.is_empty() || src.id == dst.id || at < 0 || at > dst.npages() || dst.page_ids.len() + pids.len() > STORE_MAX_PAGES {
        return -1;
    }
    if pids.iter().enumerate().any(|(i, p)| !src.page_ids.contains(p) || pids[..i].contains(p)) {
        return -1;
    }
    let (mut src_bg, mut dst_bg) = (String::new(), String::new());
    if bgimage_dir(src.id.as_str(), &mut src_bg, 700) != 0 || bgimage_dir(dst.id.as_str(), &mut dst_bg, 700) != 0 {
        return -1;
    }
    let (mut dw, mut dh) = (0, 0);
    store_notebook_page_size(dst, &mut dw, &mut dh);
    let mut src_ix = BgIndex::default();
    let mut dst_ix = BgIndex::default();
    let mut dst_ix_read: Option<bool> = None; // read once, when a picture first needs it
    let mut ix_changed = false;
    let mut written: Vec<String> = Vec::new(); // page files and pictures this copy made, for a failure
    let mut papers: Vec<PaperKind> = Vec::new();
    let mut next = dst.next_page_id;
    // (a failure takes back every file this copy wrote, and says nothing was copied)
    macro_rules! bail {
        () => {{
            out.clear();
            for f in &written {
                let _ = std::fs::remove_file(f);
            }
            return -1;
        }};
    }
    let src_ix_ok = bgindex_load(&src_bg, &mut src_ix) == 0;
    if !src_ix_ok {
        src_ix = BgIndex::default();
    }
    for &pid in pids {
        // (each page is a read, a parse and an fsync'd write: a long copy keeps the pens read and the
        // launcher's heartbeat going, which waits 15 s)
        work_pump();
        let path = page_path(src, pid);
        let st = std::fs::metadata(&path);
        let missing = matches!(&st, Err(e) if e.raw_os_error() == Some(libc::ENOENT));
        if st.as_ref().is_err() && !missing || st.as_ref().is_ok_and(|m| !m.is_file()) {
            bail!();
        }
        let mut data: Option<Vec<u8>> = if missing { None } else { read_file(&path) };
        let mut p: Option<Box<Page>> = None;
        if !missing {
            let whole = matches!((&data, &st), (Some(d), Ok(m)) if m.size() == d.len() as u64);
            if !whole || page_deserialize(data.as_deref().unwrap_or(&[]), &mut p) != 0 {
                bail!();
            }
        }
        if data.is_none() {
            p = Some(page_new(dw, dh, store_new_page_paper(src, src.paper)));
        } else if let Some(pg) = p.as_deref_mut() {
            if side_restore(src, pid, pg) {
                data = None; // its picture, which an older version's save left out: the copy written with it
            }
        }
        let Some(mut pg) = p else { bail!() };
        let erased = pg.strokes.iter().any(|s| stroke_is_erase(s));
        let mut paper = pg.paper;
        if paper_is_image(paper) {
            if dst_ix_read.is_none() {
                dst_ix_read = Some(bgindex_load(&dst_bg, &mut dst_ix) == 0);
            }
            if dst_ix_read != Some(true) {
                qlog!("store: %s: bg/index.ini does not read; no picture page copied there", &dst.id);
                bail!();
            }
            let pic = paper_image_id(paper);
            let mut from = String::new();
            bgimage_file(&src_bg, pic, &mut from, 700);
            let bytes = read_file(&from);
            let key = copied_picture_key(&src_ix, src.id.as_str(), pic, bytes.as_deref());
            let (mut id, mut added) = (0u32, false);
            if bgindex_claim(&mut dst_ix, &dst_bg, key, &mut id, &mut added) != 0 {
                bail!();
            }
            ix_changed |= added;
            let mut to = String::new();
            bgimage_file(&dst_bg, id, &mut to, 700);
            if let Some(b) = &bytes {
                if !access_ok(&to) {
                    if mkdir_p(&dst_bg) != 0 || write_file_atomic(&to, b) != 0 {
                        bail!();
                    }
                    written.push(to);
                }
            }
            let role = if paper_is_document(paper) { PAPER_ROLE_DOCUMENT } else { PAPER_ROLE_TEMPLATE };
            let want = paper_image(role, id);
            if want != paper {
                if pg.readonly {
                    qlog!("store: p%u.qpg of %s is read-only and its picture needs another id in %s; not copied", pid, &src.id, &dst.id);
                    bail!();
                }
                // Version 2 keeps the whole paper value at bytes 16 to 19: only those change.
                match data.as_mut() {
                    Some(d) if d.len() >= 20 && d[4..6] == [2, 0] && d[16..20] == paper.0.to_le_bytes() => {
                        d[16..20].copy_from_slice(&want.0.to_le_bytes());
                    }
                    _ => data = None,
                }
                pg.paper = want;
                paper = want;
            }
        }
        // The copy's id: dst's next, never one whose file is there already (as take_page_id).
        let mut tries = 0;
        let to = loop {
            if next == 0 || next == u32::MAX || tries > 64 {
                break 0;
            }
            let path = cformat!("%s/notebooks/%s/p%u.qpg", &g_root(), &dst.id, next);
            match std::fs::symlink_metadata(&path) {
                Err(e) if e.raw_os_error() == Some(libc::ENOENT) => break next,
                Err(_) => break 0,
                Ok(_) => {
                    tries += 1;
                    next += 1;
                }
            }
        };
        if to == 0 {
            bail!();
        }
        let copy = cformat!("%s/notebooks/%s/p%u.qpg", &g_root(), &dst.id, to);
        let rc = match &data {
            Some(d) => write_file_atomic(&copy, d),
            None => write_page_at(&copy, &pg),
        };
        if rc != 0 {
            bail!();
        }
        written.push(copy);
        next = to + 1;
        papers.push(paper);
        out.push(PageCopy { from: pid, to, erased });
    }
    let new_ids: Vec<u32> = out.iter().map(|c| c.to).collect();
    if ix_changed && bgindex_save(&dst_bg, &dst_ix) != 0 {
        bail!();
    }
    if papers.iter().any(|&k| paper_is_image(k)) && store_note_papers_in(&dst_bg, &new_ids, &papers) != 0 {
        bail!();
    }
    let was = dst.clone();
    for (k, &pid) in new_ids.iter().enumerate() {
        dst.page_ids.insert(at as usize + k, pid);
    }
    dst.next_page_id = next;
    if store_save_info(dst) != 0 {
        *dst = was;
        bail!();
    }
    0
}

/// A page file of another size than its notebook's: an older version opened it on a screen of
/// another size (versions before 1.1 scale every page to the screen) and wrote it so after an edit.
/// Its points go back to the notebook's size. Rules and stroke widths are in mm, the same on every
/// screen, so the older version's scaling moved the ink off its lines: this puts it back.
fn scale_page(p: &mut Page, w: i32, h: i32) {
    if p.w == w && p.h == h {
        return;
    }
    let sx = w as f32 / p.w as f32;
    let sy = h as f32 / p.h as f32;
    let dpi = g_dpi();
    for s in p.strokes.iter_mut() {
        // A floor marker stays where it is, off every page (docs/FORMAT.md, Scoped erase strokes).
        let marker = is_floor_marker(s);
        let pts = std::mem::take(&mut s.pts);
        s.bbox = RECT_EMPTY;
        for mut q in pts {
            if !marker {
                q.x *= sx;
                q.y *= sy;
            }
            stroke_append(s, &q, dpi);
        }
    }
    p.w = w;
    p.h = h;
}

/// The page at w x h, the notebook's page size (store_notebook_page_size): a page file of another
/// size, one an older version scaled to its own screen, has its points scaled back. Missing page
/// file -> empty page of nb->paper. A damaged file is set aside (p<id>.qpg.damaged-<time>). A page
/// from a newer version comes back read-only (Page.readonly) and is never set aside. A page that
/// bg/pages.ini lists with an image paper but that reads as Blank (an older version saved it so)
/// gets its image paper back, and is written so again (each such page counted, below). Every page
/// comes with Page.nb set to the notebook's id.
pub fn store_load_page(
    nb: &NotebookInfo,
    index: i32,
    w: i32,
    h: i32,
    out: &mut Option<Box<Page>>,
) -> i32 {
    load_page_with(nb, index, w, h, out, false)
}

/// store_load_page for a read the pens must not wait for: the same page, set aside and written back as an
/// open would, but work_pump() runs every 256 strokes as it is read. The editor's peek reads a page this way
/// while the Links bar is up (a dense page is 1.2 s of reading and drawing on the Kindle).
pub fn store_load_page_pumped(
    nb: &NotebookInfo,
    index: i32,
    w: i32,
    h: i32,
    out: &mut Option<Box<Page>>,
) -> i32 {
    load_page_with(nb, index, w, h, out, true)
}

fn load_page_with(
    nb: &NotebookInfo,
    index: i32,
    w: i32,
    h: i32,
    out: &mut Option<Box<Page>>,
    pump: bool,
) -> i32 {
    *out = None;
    if index < 0 || index >= nb.npages() {
        return -1;
    }
    let pid = nb.page_ids[index as usize];
    let path = page_path(nb, pid);
    let mut p: Option<Box<Page>> = None;
    if let Some(data) = read_file(&path) {
        if page_deserialize_with(&data, &mut p, pump) != 0 {
            // Keep the damaged file instead of letting the next autosave replace it with the
            // blank page shown in its place. (A page from a newer version is not damage: it
            // opens read-only.)
            let mut aside = String::new();
            csnprintf!(&mut aside, 760, "%s.damaged-%lld", &path, wall_time());
            let rc = std::fs::rename(&path, &aside);
            qlog!(
                "store: corrupt page %s, %s; starting blank",
                &path,
                if rc.is_ok() {
                    "set aside"
                } else {
                    "could not set aside"
                }
            );
            with(|s| s.g_corrupt_pages += 1);
        } else if let Some(pg) = p.as_deref_mut() {
            if pg.readonly {
                qlog!(
                    "store: %s opens read-only and stays as it is on disk",
                    &path
                );
            } else if side_restore(nb, pid, pg) {
                // Written back as this version writes it, so the picture stays without the list's help,
                // and counted (store_take_restored_count): what the older version drew of the page, the
                // thumbnail and the PDF, is drawn again.
                if write_page_file(nb, pid, pg) != 0 {
                    qlog!("store: %s: could not write its paper back", &path);
                }
                with(|s| s.g_restored_pages += 1);
            }
        }
    }
    let mut p = match p {
        Some(p) => p,
        None => page_new(w, h, nb.paper),
    };
    p.nb.set_bytes(nb.id.as_bytes());
    if p.w != w || p.h != h {
        qlog_once!("store: %s is %dx%d: scaled to %dx%d", &path, p.w, p.h, w, h);
    }
    scale_page(&mut p, w, h);
    *out = Some(p);
    0
}

/// The same for exports, always at the notebook's page size, and nothing on disk changes: a page
/// that is missing or does not read is a blank page of nb->paper, and its file stays as it is.
pub fn store_read_page(nb: &NotebookInfo, index: i32, out: &mut Option<Box<Page>>) -> i32 {
    *out = None;
    if index < 0 || index >= nb.npages() {
        return -1;
    }
    let (mut w, mut h) = (0, 0);
    store_notebook_page_size(nb, &mut w, &mut h);
    let pid = nb.page_ids[index as usize];
    let path = page_path(nb, pid);
    let mut p: Option<Box<Page>> = None;
    if let Some(data) = read_file(&path) {
        // Damage leaves p NULL: a blank page below, the file as it is.
        if page_deserialize(&data, &mut p) == 0 {
            if let Some(pg) = p.as_deref_mut() {
                side_restore(nb, pid, pg);
            }
        }
    }
    let mut p = match p {
        Some(p) => p,
        None => page_new(w, h, nb.paper),
    };
    p.nb.set_bytes(nb.id.as_bytes());
    scale_page(&mut p, w, h);
    *out = Some(p);
    0
}

/// A read-only page is never written: its file stays as it is (and this returns 0). bg/pages.ini
/// follows the page's paper (an image paper listed, any other not).
pub fn store_save_page(nb: &NotebookInfo, index: i32, p: &Page) -> i32 {
    if index < 0 || index >= nb.npages() {
        return -1;
    }
    // A read-only page stays as it is on disk: writing it would lose what this build cannot read.
    if p.readonly {
        return 0;
    }
    let pid = nb.page_ids[index as usize];
    side_note(nb, pid, p.paper);
    write_page_file(nb, pid, p)
}

// ---- pages 1.0.0-beta set aside ----

struct Aside {
    pid: u32,
    t: i64,
    name: String,
}

fn cmp_aside(x: &Aside, y: &Aside) -> Ordering {
    if x.pid != y.pid {
        return x.pid.cmp(&y.pid);
    }
    y.t.cmp(&x.t) // newest first
}

/// Whether a file reads as a page now, normally or read-only (quietly: the page is not opened).
fn page_file_opens(path: &str) -> bool {
    let Some(data) = read_file(path) else {
        return false;
    };
    let mut p: Option<Box<Page>> = None;
    page_parse(&data, &mut p, true, false) == 0
}

/// Whether a page file starts as version 1 Blank (its first 8 bytes, the rest unread): how 1.0.0-beta
/// writes a page whose image paper it does not know.
fn page_file_v1_blank(path: &str) -> bool {
    let mut b = [0u8; 8];
    let Ok(mut f) = std::fs::File::open(path) else {
        return false;
    };
    let got = matches!(f.read(&mut b), Ok(8));
    got && &b[..4] == b"QPG1" && b[4] == 1 && b[5] == 0 && b[6] == PAPER_BLANK.0 as u8 && b[7] == 0
}

/// When a notebook is opened in the editor: every page bg/pages.ini lists that reads as version 1
/// Blank (an older version saved it so) gets its image paper back now and is written so, as
/// store_load_page does for one page as it opens: then the notebook's thumbnail and PDF, which the
/// older version drew on blank paper, are drawn again when it closes, also for pages not visited.
/// Each counts in store_take_restored_count. Returns how many.
pub fn store_restore_listed_pages(nb: &NotebookInfo) -> i32 {
    let mut bgdir = String::new();
    if bgimage_dir(nb.id.as_str(), &mut bgdir, 700) != 0 {
        return 0;
    }
    let path = side_path(&bgdir);
    if !access_ok(&path) {
        return 0;
    }
    let mut s = Side::default();
    if side_load(&bgdir, &mut s) != 0 {
        return 0; // (left as it is: store_load_page says so)
    }
    let mut restored = 0;
    for e in &s.e {
        let listed = nb.page_ids.contains(&e.pid);
        let pp = page_path(nb, e.pid);
        // Only a page of the notebook that reads as Blank by its first bytes is read whole: a big
        // notebook of imported pages lists every page, and opens without reading each one.
        if !listed || !page_file_v1_blank(&pp) {
            continue;
        }
        let mut p: Option<Box<Page>> = None;
        if let Some(data) = read_file(&pp) {
            if page_parse(&data, &mut p, true, false) == 0 {
                if let Some(pg) = p.as_deref_mut() {
                    if !pg.readonly && pg.paper == PAPER_BLANK {
                        pg.paper = PaperKind(e.kind);
                        if write_page_file(nb, e.pid, pg) == 0 {
                            restored += 1;
                        } else {
                            qlog!(
                                "store: %s: could not write p%u.qpg's paper back",
                                &nb.id,
                                e.pid
                            );
                        }
                    }
                }
            }
        }
    }
    if restored != 0 {
        qlog!(
            "store: %s: %d page(s) an older version saved with blank paper get their pictures back from bg/pages.ini",
            &nb.id,
            restored
        );
    }
    with(|st| st.g_restored_pages += restored);
    restored
}

/// When a notebook is opened: pages an older version set aside (p<id>.qpg.damaged-<time>, as
/// 1.0.0-beta does with any page from a newer version) that open now go back to p<id>.qpg, the
/// newest copy if there are several. Only pages in the page list come back; if p<id>.qpg exists
/// again, both are kept. Returns how many came back; each is logged.
pub fn store_recover_pages(nb: &NotebookInfo) -> i32 {
    let Some(dir) = notebook_dir(nb.id.as_bytes()) else {
        return 0;
    };
    let Ok(rd) = std::fs::read_dir(&dir) else {
        return 0;
    };
    let mut asides: Vec<Aside> = Vec::new();
    for e in rd {
        let Ok(e) = e else { break };
        let d_name = e.file_name();
        let d_name = d_name.as_bytes();
        let Some((id, t)) = scan_aside_name(d_name) else {
            continue;
        };
        if id == 0 || id >= 0x7fffffff || d_name.len() >= 64 {
            continue;
        }
        asides.push(Aside {
            pid: id as u32,
            t,
            name: String::from_utf8_lossy(d_name).into_owned(),
        });
    }
    asides.sort_by(cmp_aside);
    let n = asides.len();
    let mut restored = 0;
    let mut i = 0;
    while i < n {
        let mut j = i;
        while j < n && asides[j].pid == asides[i].pid {
            j += 1;
        }
        let pid = asides[i].pid;
        let listed = nb.page_ids.contains(&pid);
        if !listed {
            // No longer one of the notebook's pages (deleted since, or missing from a page list
            // that was rebuilt): put back, it would be no page of the notebook, and a new page
            // could take its number and write over it. It stays aside.
            qlog_once!(
                "store: %s: p%u.qpg is not in the page list; its set-aside copies stay aside (%s)",
                &nb.id,
                pid,
                &asides[i].name
            );
            i = j;
            continue;
        }
        let path = cformat!("%s/p%u.qpg", &dir, pid);
        let exists = access_ok(&path);
        // The newest copy that opens now, normally or read-only; real damage stays aside.
        for k in i..j {
            let aside = cformat!("%s/%s", &dir, &asides[k].name);
            if !page_file_opens(&aside) {
                continue;
            }
            if exists {
                // An older version wrote a fresh page there since (it was edited): keep both.
                qlog_once!(
                    "store: %s: %s opens now, but p%u.qpg exists too; both kept",
                    &nb.id,
                    &asides[k].name,
                    pid
                );
            } else {
                match std::fs::rename(&aside, &path) {
                    Ok(()) => {
                        qlog!(
                            "store: %s: recovered p%u.qpg from %s",
                            &nb.id,
                            pid,
                            &asides[k].name
                        );
                        restored += 1;
                    }
                    Err(e) => {
                        qlog_once!(
                            "store: %s: could not recover %s: %s",
                            &nb.id,
                            &asides[k].name,
                            &strerror(e.raw_os_error().unwrap_or(0))
                        );
                    }
                }
            }
            break;
        }
        i = j;
    }
    restored
}

// ---- page format ----

fn w_u8(w: &mut Vec<u8>, v: u8) {
    w.push(v);
}
fn w_u16(w: &mut Vec<u8>, v: u16) {
    w.extend_from_slice(&v.to_le_bytes());
}
fn w_u32(w: &mut Vec<u8>, v: u32) {
    w.extend_from_slice(&v.to_le_bytes());
}
fn w_f32(w: &mut Vec<u8>, f: f32) {
    w_u32(w, f.to_bits());
}

/// The fixed paper kinds 1.0.0-beta knows (its PAPER_COUNT). Pages on one of them are version 1;
/// any other kind is version 2 (docs/FORMAT.md). Never changes, even when kinds are added.
const PAGE_V1_PAPERS: u32 = 17;
/// Versions a newer Grimoire may write (3 and up, staying below 256): such a page opens read-only.
/// Anything else (0, or a number no version uses) is damage.
const PAGE_NEWER_MAX: u32 = 255;

/// A pen sample as the page file keeps it and reads it back: pressure in steps of 1/65535, tilt in
/// hundredths of a degree (the very conversions below and in page_deserialize). The editor draws every
/// sample so, so the ink on the screen is exactly what opening the page again draws: before 2.0.0-rc.10
/// a reopened page could differ by a gray level at a few pixels, which a page's kept drawing would
/// then have carried (store_save_page_render).
pub fn sample_as_stored(q: &mut PenSample) {
    q.pressure = ((clampf(q.pressure, 0.0, 1.0) * 65535.0).round_ties_even() as i64) as u16 as f32 / 65535.0;
    q.tilt_x = ((((clampf(q.tilt_x, -90.0, 90.0) * 100.0).round_ties_even() as i64) as i16) as u16 as i16) as f32 / 100.0;
    q.tilt_y = ((((clampf(q.tilt_y, -90.0, 90.0) * 100.0).round_ties_even() as i64) as i16) as u16 as i16) as f32 / 100.0;
}

/// Page file format "QPG1" (little-endian), byte by byte in docs/FORMAT.md:
///   char magic[4]="QPG1"; u16 version; u16 paper; u32 w, h;   (these 16 bytes in every version)
///   (version 2, a paper 1.0.0-beta does not know: u32 paper follows h; the u16 then says
///    PAPER_MUSIC for custom music, PAPER_BLANK otherwise)  u32 nstrokes;
///   per stroke: u8 tool; u8 ext[3]; u32 seed; f32 params[13] (ToolParams field order);
///               u32 npts; per point: f32 x, y; u16 pressure*65535; i16 tilt_x*100, tilt_y*100; u32 t_ms
///   then chunks to the end of the file: u32 tag, u32 length, length bytes.
/// Readers reject bad magic, clamp counts against the file size, and never trust lengths. A page
/// from a newer version (version 3 to 255, a tool or a critical chunk this build does not know,
/// bytes after the strokes that are not chunks) reads as a read-only page instead of failing:
/// page_deserialize returns 0 with Page.readonly set, and page_serialize refuses such a page.
pub fn page_serialize(p: &Page, out: &mut Vec<u8>) -> i32 {
    // A page this build cannot fully read would lose what it cannot read: it is never written.
    if p.readonly {
        return -1;
    }
    let mut w: Vec<u8> = Vec::new();
    // Version 2 only when the paper needs all 32 bits (custom music, or a paper from a newer
    // version): other pages stay version 1, as 1.0.0-beta writes them. In version 2 the u16 says
    // PAPER_MUSIC for custom music, as it always did, and PAPER_BLANK for any other kind.
    let v2 = p.paper.0 >= PAGE_V1_PAPERS;
    let p16 = if !v2 {
        p.paper
    } else if paper_is_custom(p.paper) {
        PAPER_MUSIC
    } else {
        PAPER_BLANK
    };
    w.extend_from_slice(b"QPG1");
    w_u16(&mut w, if v2 { 2 } else { 1 });
    w_u16(&mut w, p16.0 as u16);
    w_u32(&mut w, p.w as u32);
    w_u32(&mut w, p.h as u32);
    if v2 {
        w_u32(&mut w, p.paper.0);
    }
    w_u32(&mut w, p.strokes.len() as u32);
    for s in p.strokes.iter() {
        w_u8(&mut w, s.tool.0 as u8);
        w.extend_from_slice(&s.ext); // as read (0 for strokes drawn here)
        w_u32(&mut w, s.seed);
        for k in 0..ToolParams::COUNT {
            w_f32(&mut w, s.params.get(k));
        }
        w_u32(&mut w, s.pts.len() as u32);
        for q in s.pts.iter() {
            w_f32(&mut w, q.x);
            w_f32(&mut w, q.y);
            w_u16(
                &mut w,
                ((clampf(q.pressure, 0.0, 1.0) * 65535.0).round_ties_even() as i64) as u16,
            );
            w_u16(
                &mut w,
                (((clampf(q.tilt_x, -90.0, 90.0) * 100.0).round_ties_even() as i64) as i16) as u16,
            );
            w_u16(
                &mut w,
                (((clampf(q.tilt_y, -90.0, 90.0) * 100.0).round_ties_even() as i64) as i16) as u16,
            );
            w_u32(&mut w, q.t_ms);
        }
    }
    // The chunks that followed the strokes, as they were read (this version writes none itself).
    w.extend_from_slice(&p.tail);
    *out = w;
    0
}

struct R<'a> {
    d: &'a [u8],
    n: usize,
    pos: usize,
    err: bool,
}

fn r_take<'a>(r: &mut R<'a>, n: usize) -> Option<&'a [u8]> {
    if r.err || r.n - r.pos < n {
        r.err = true;
        return None;
    }
    let p = &r.d[r.pos..r.pos + n];
    r.pos += n;
    Some(p)
}
fn r_u8(r: &mut R) -> u8 {
    r_take(r, 1).map_or(0, |p| p[0])
}
fn r_u16(r: &mut R) -> u16 {
    r_take(r, 2).map_or(0, |p| u16::from_le_bytes([p[0], p[1]]))
}
fn r_u32(r: &mut R) -> u32 {
    r_take(r, 4).map_or(0, |p| u32::from_le_bytes([p[0], p[1], p[2], p[3]]))
}
fn r_f32(r: &mut R) -> f32 {
    let f = f32::from_bits(r_u32(r));
    if f.is_finite() {
        f
    } else {
        0.0
    }
}

const STROKE_HEADER_BYTES: usize = 4 + 4 + 13 * 4 + 4;
const POINT_BYTES: usize = 4 + 4 + 2 + 2 + 2 + 4;

fn sane_size(w: u32, h: u32) -> bool {
    w > 0 && h > 0 && w <= 20000 && h <= 20000
}

/// A page from a newer version: only the 16 bytes every version starts with are read (magic,
/// version, u16 paper, w, h). It is shown blank, read-only.
fn newer_page(ver: u32, w: u32, h: u32, out: &mut Option<Box<Page>>, quiet: bool) -> i32 {
    let sane = sane_size(w, h);
    let (gw, gh) = g_wh();
    let mut p = page_new(
        if sane { w as i32 } else { gw },
        if sane { h as i32 } else { gh },
        PAPER_BLANK,
    );
    p.readonly = true;
    if !quiet {
        qlog!(
            "store: read-only page: version %u is from a newer Grimoire, shown blank",
            ver
        );
    }
    *out = Some(p);
    0
}

fn tag_char(c: u8, first: bool) -> bool {
    c.is_ascii_alphabetic() || (!first && c.is_ascii_digit())
}

/// The chunks after the last stroke, to the end of the file: u32 tag (four ASCII letters or digits,
/// the first a letter), u32 length, the bytes. This version knows no chunk. One whose tag starts
/// with a lowercase letter is safe to copy: carried, written back as it was. One starting with an
/// uppercase letter is critical, and like bytes that are not chunks, makes the page read-only.
/// The whole tail is kept either way. Returns why the page is read-only, or None.
fn read_tail(r: &mut R, p: &mut Page) -> Option<String> {
    if r.pos >= r.n {
        return None;
    }
    let tl = r.n - r.pos;
    let mut tail: Vec<u8> = Vec::new();
    if tail.try_reserve_exact(tl).is_err() {
        return Some("no memory for its chunks".to_string());
    }
    tail.extend_from_slice(&r.d[r.pos..]);
    p.tail = tail;
    while r.pos < r.n {
        let c = &r.d[r.pos..];
        if r.n - r.pos < 8 {
            return Some("cut-off bytes after the strokes".to_string());
        }
        let ok = tag_char(c[0], true)
            && tag_char(c[1], false)
            && tag_char(c[2], false)
            && tag_char(c[3], false);
        if !ok {
            return Some("bytes after the strokes that are not chunks".to_string());
        }
        let clen = u32::from_le_bytes([c[4], c[5], c[6], c[7]]);
        if clen as usize > r.n - r.pos - 8 {
            return Some("a chunk longer than the file".to_string());
        }
        if c[0].is_ascii_uppercase() {
            // (the tag is four ASCII letters or digits: tag_char)
            return Some(cformat!(
                "a chunk this version does not know (%.4s)",
                std::str::from_utf8(&c[..4]).unwrap_or("")
            ));
        }
        r.pos += 8 + clen as usize;
    }
    None
}

/// The reader behind page_deserialize. `quiet` leaves the reason a page opens read-only out of the
/// log (for a check of a file, when no page is being opened). `pump` calls work_pump() every PUMP_STROKES
/// strokes as it goes (the editor's pumped reader: a page of thousands of strokes is read while the pens
/// are, on the Kindle).
fn page_parse(buf: &[u8], out: &mut Option<Box<Page>>, quiet: bool, pump: bool) -> i32 {
    *out = None;
    let len = buf.len();
    let mut r = R {
        d: buf,
        n: len,
        pos: 0,
        err: false,
    };
    match r_take(&mut r, 4) {
        Some(m) if m == b"QPG1" => {}
        _ => return -1,
    }
    let ver = r_u16(&mut r) as u32;
    let mut paper = r_u16(&mut r) as u32;
    let w = r_u32(&mut r);
    let h = r_u32(&mut r);
    if r.err {
        return -1;
    }
    if ver > 2 && ver <= PAGE_NEWER_MAX {
        return newer_page(ver, w, h, out, quiet);
    }
    if ver != 1 && ver != 2 {
        return -1;
    }
    if ver == 2 {
        paper = r_u32(&mut r);
    }
    let ns = r_u32(&mut r);
    if r.err || !sane_size(w, h) {
        return -1;
    }
    if ns as usize > (len - r.pos) / STROKE_HEADER_BYTES {
        return -1;
    }
    // A paper this build does not know stays as it is: drawn blank, written back.
    let mut p = page_new(w as i32, h as i32, PaperKind(paper));
    let dpi = g_dpi();
    let mut unknown_tools: u32 = 0;
    for si in 0..ns {
        if pump && si % PUMP_STROKES == PUMP_STROKES - 1 {
            work_pump();
        }
        if r.err {
            break;
        }
        let tool = r_u8(&mut r);
        let ext = r_take(&mut r, 3);
        let seed = r_u32(&mut r);
        let mut prm = ToolParams::default();
        for k in 0..ToolParams::COUNT {
            *prm.get_mut(k) = r_f32(&mut r);
        }
        let np = r_u32(&mut r) as usize;
        if r.err || np > (len - r.pos) / POINT_BYTES {
            r.err = true;
            break;
        }
        if tool as usize >= TOOL_COUNT {
            // A newer version's tool: the record is laid out the same, so the page reads on.
            // The stroke is left out (never drawn) and the page becomes read-only, so no save
            // here can drop it.
            r_take(&mut r, np * POINT_BYTES);
            unknown_tools += 1;
            continue;
        }
        // Clamp params into their UI ranges so a hostile file cannot make huge dabs.
        for pi in tool_param_table() {
            let v = tool_param_ptr(&mut prm, pi);
            *v = clampf(*v, pi.min, pi.max);
        }
        let mut s = stroke_new(ToolKind(tool as i32), &prm, seed);
        if let Some(e) = ext {
            s.ext.copy_from_slice(e);
        }
        for _ in 0..np {
            // (the fields in the file's order: a struct expression reads them in the order written)
            let q = PenSample {
                x: clampf(r_f32(&mut r), -10000.0, 30000.0),
                y: clampf(r_f32(&mut r), -10000.0, 30000.0),
                pressure: r_u16(&mut r) as f32 / 65535.0,
                tilt_x: (r_u16(&mut r) as i16) as f32 / 100.0,
                tilt_y: (r_u16(&mut r) as i16) as f32 / 100.0,
                t_ms: r_u32(&mut r),
            };
            stroke_append(&mut s, &q, dpi);
        }
        page_add(&mut p, s);
    }
    if r.err {
        return -1;
    }
    let mut ro = read_tail(&mut r, &mut p);
    if unknown_tools != 0 {
        ro = Some(cformat!(
            "%u stroke%s with a tool this version does not have",
            unknown_tools,
            if unknown_tools == 1 { "" } else { "s" }
        ));
    }
    if let Some(ro) = ro {
        p.readonly = true;
        if !quiet {
            qlog!("store: read-only page: %s", &ro);
        }
    }
    *out = Some(p);
    0
}

pub fn page_deserialize(buf: &[u8], out: &mut Option<Box<Page>>) -> i32 {
    page_deserialize_with(buf, out, false)
}

/// page_deserialize with no "read-only page" line in the log: for a file in the page format that is no
/// notebook's page and carries a critical chunk of its own on purpose, so that line would be false there
/// (the lasso's clipboard, whose `CLIP` chunk keeps older versions from taking it for a page: clipboard.rs).
/// A notebook's pages go through page_deserialize, which still logs it.
pub fn page_deserialize_quiet(buf: &[u8], out: &mut Option<Box<Page>>) -> i32 {
    page_parse(buf, out, true, false)
}

/// How many strokes the pumped reader reads between two calls of work_pump: 256 strokes of a dense page
/// are a few milliseconds on the Kindle.
pub const PUMP_STROKES: u32 = 256;

/// page_deserialize, calling work_pump() every PUMP_STROKES strokes while it reads when `pump` (the editor's
/// peek reads a page of its own while the pens are read).
fn page_deserialize_with(buf: &[u8], out: &mut Option<Box<Page>>, pump: bool) -> i32 {
    page_parse(buf, out, false, pump)
}

/// The size a page file says it has, from the 16 bytes every version starts with (magic, version,
/// u16 paper, w, h), the rest unread. None for a file that has none (missing, cut short, damage).
fn page_file_size(path: &str) -> Option<(i32, i32)> {
    let mut b = [0u8; 16];
    let mut f = std::fs::File::open(path).ok()?;
    let got = matches!(f.read(&mut b), Ok(16));
    if !got || &b[..4] != b"QPG1" {
        return None;
    }
    let ver = b[4] as u32 | (b[5] as u32) << 8;
    let pw = u32::from_le_bytes([b[8], b[9], b[10], b[11]]);
    let ph = u32::from_le_bytes([b[12], b[13], b[14], b[15]]);
    if ver == 0 || ver > PAGE_NEWER_MAX || !page_size_ok(pw as i64, ph as i64) {
        return None;
    }
    Some((pw as i32, ph as i32))
}

/// access(path, F_OK) == 0
fn access_ok(path: &str) -> bool {
    std::fs::metadata(path).is_ok()
}

// ---- thumbnails ----

fn save_pgm(path: &str, t: &Canvas) -> i32 {
    pgm_bytes(t).map_or(-1, |buf| write_file_atomic(path, &buf))
}

fn pgm_bytes(t: &Canvas) -> Option<Vec<u8>> {
    let hdr = cformat!("P5\n%d %d\n255\n", t.w, t.h);
    let (w, h, stride) = (
        t.w.max(0) as usize,
        t.h.max(0) as usize,
        t.stride.max(0) as usize,
    );
    let mut buf: Vec<u8> = Vec::new();
    if buf.try_reserve_exact(hdr.len() + w * h).is_err() {
        return None;
    }
    buf.extend_from_slice(hdr.as_bytes());
    for y in 0..h {
        buf.extend_from_slice(&t.px[y * stride..y * stride + w]);
    }
    Some(buf)
}

// The page previews are a disposable cache: written to a per-process name and renamed into place,
// never fsynced (a torn file fails its stamp or its size check and is made again). Each fsync on the
// Kindle waits 15-25 ms.
fn write_cache_file(path: &str, data: &[u8]) -> i32 {
    #[cfg(feature = "sim")]
    if crate::util::SIM_STORAGE_FULL.load(std::sync::atomic::Ordering::Relaxed) {
        return -1;
    }
    if std::fs::write(path, data).is_err() {
        let _ = std::fs::remove_file(path);
        return -1;
    }
    0
}

fn load_pgm(path: &str, out: &mut Canvas) -> i32 {
    *out = Canvas::default();
    let Some(d) = read_file(path) else { return -1 };
    let len = d.len();
    let Some((w, h, mx, off)) = scan_p5_header(&d) else {
        return -1;
    };
    if w <= 0
        || h <= 0
        || w > 8000
        || h > 8000
        || mx != 255
        || off + 1 + (w as usize) * (h as usize) > len
    {
        return -1;
    }
    if canvas_init(out, w, h) != 0 {
        return -1;
    }
    let n = (w as usize) * (h as usize);
    out.px[..n].copy_from_slice(&d[off + 1..off + 1 + n]);
    0
}

/// P5 PGM, atomically
pub fn store_write_pgm(path: &str, c: &Canvas) -> i32 {
    save_pgm(path, c)
}
/// canvas_init'd by the callee
pub fn store_read_pgm(path: &str, out: &mut Canvas) -> i32 {
    load_pgm(path, out)
}

/// Thumbnails (P5 PGM, ~ 1/8 page size).
pub fn store_save_thumb(id: &str, thumb: &Canvas) -> i32 {
    let Some(dir) = notebook_dir(id.as_bytes()) else {
        return -1;
    };
    let path = cformat!("%s/thumb.pgm", &dir);
    save_pgm(&path, thumb)
}

/// Which page 1 the card was drawn from (store_page_source_identity of the page, with its id), beside the card
/// (thumb.src) and bound to the card file itself: a card written by anything else (an older version, another
/// path) matches no source, so a close redraws it rather than trusting that it shows page 1 as it is.
pub fn store_note_thumb_source(id: &str, source: &str) {
    let Some(dir) = notebook_dir(id.as_bytes()) else { return };
    let stamp = store_thumb_stamp(id);
    if stamp == 0 {
        return;
    }
    let path = format!("{dir}/thumb.src");
    let mine = format!("{path}.{}", std::process::id());
    if write_cache_file(&mine, format!("{stamp}|{source}").as_bytes()) == 0 && std::fs::rename(&mine, &path).is_ok() {
        return;
    }
    let _ = std::fs::remove_file(mine);
}

/// What store_note_thumb_source noted, while the card is still the file it was noted for.
pub fn store_thumb_source(id: &str) -> Option<String> {
    let dir = notebook_dir(id.as_bytes())?;
    let text = preview_tag(&format!("{dir}/thumb.src"))?;
    let (stamp, source) = text.split_once('|')?;
    (stamp == store_thumb_stamp(id).to_string()).then(|| source.to_string())
}

/// canvas_init'd by the callee
pub fn store_load_thumb(id: &str, out: &mut Canvas) -> i32 {
    *out = Canvas::default();
    let Some(dir) = notebook_dir(id.as_bytes()) else {
        return -1;
    };
    let path = cformat!("%s/thumb.pgm", &dir);
    load_pgm(&path, out)
}

fn page_preview_files(nb: &NotebookInfo, pid: u32) -> Option<(String, String, String)> {
    let dir = notebook_dir(nb.id.as_bytes())?;
    let source = format!("{dir}/p{pid}.qpg");
    let cache = format!("{dir}/.page-previews");
    Some((source, format!("{cache}/p{pid}.pgm"), format!("{cache}/p{pid}.stamp")))
}
fn preview_stamp(source: &str) -> Option<String> {
    let st = std::fs::metadata(source).ok()?;
    let generation=std::path::Path::new(source).parent()?.join(".page-previews/generation");
    let gen=std::fs::metadata(generation).map(|g|format!("{}:{}:{}",g.ino(),g.mtime(),g.mtime_nsec())).unwrap_or_default();
    Some(format!("{}:{}:{}:{}:{}:{gen}", st.ino(), st.size(), st.mtime(), st.mtime_nsec(), st.ctime_nsec()))
}
fn preview_tag(path:&str)->Option<String> {
    use std::io::Read;
    let mut text=String::new();
    std::fs::File::open(path).ok()?.take(256).read_to_string(&mut text).ok()?;
    (text.len()<256).then_some(text)
}
// Directory timestamps can have coarse resolution. Publish an opaque change token so an
// already decoded Pages view notices rewritten previews even within the same timestamp tick.
fn page_previews_changed(id:&str) {
    use std::sync::atomic::{AtomicU32,Ordering};
    static SERIAL:AtomicU32=AtomicU32::new(0);
    let Some(dir)=notebook_dir(id.as_bytes())else{return};
    let serial=SERIAL.fetch_add(1,Ordering::Relaxed);
    let time=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos();
    let token=format!("{}:{time}:{serial}",std::process::id());
    let path=format!("{dir}/.page-previews/updated");
    let mine=format!("{path}.{}",std::process::id());
    if write_cache_file(&mine,token.as_bytes())==0 {let _=std::fs::rename(&mine,&path);}
    let _=std::fs::remove_file(mine);
}
/// A valid cache's source identity before lossless ink compaction rewrites its page file.
pub fn store_page_preview_identity(nb:&NotebookInfo,pid:u32)->Option<String> {
    let (source,_,stamp)=page_preview_files(nb,pid)?;
    let identity=preview_stamp(&source)?;
    (preview_tag(&stamp).as_deref()==Some(identity.as_str())).then_some(identity)
}

/// Lossless compaction changed storage identity, not pixels. Preserve the disk preview only
/// when the exact compacted bytes are still current and its old stamp was valid beforehand.
pub fn store_retag_compacted_preview(nb:&NotebookInfo,pid:u32,before:Option<&str>,data:&[u8]) {
    let Some(before)=before else{return};
    let Some((source,_,stamp))=page_preview_files(nb,pid)else{return};
    let Some(now)=preview_stamp(&source)else{return};
    // The final component is the explicit/background-image invalidation generation. A
    // lossless QPG rewrite can keep its pixels, but must not acknowledge a newer generation.
    if before.splitn(6,':').nth(5)!=now.splitn(6,':').nth(5){return;}
    if preview_tag(&stamp).as_deref()==Some(before)
        && read_file(&source).as_deref()==Some(data)
        && preview_stamp(&source).as_deref()==Some(now.as_str()) {
        let tag=format!("{stamp}.{}",std::process::id());
        if write_cache_file(&tag,now.as_bytes())==0 && std::fs::rename(&tag,&stamp).is_err() {let _=std::fs::remove_file(tag);}
    }
    // The kept drawing of a dense page likewise: same pixels, the new file identity.
    if let Some((_,_,full))=page_render_files(nb,pid) {
        if let Some(old)=preview_tag(&full) {
            if let Some(rest)=old.strip_prefix(before) {
                if rest.starts_with('|') && read_file(&source).as_deref()==Some(data) {
                    let tag=format!("{full}.{}",std::process::id());
                    if write_cache_file(&tag,format!("{now}{rest}").as_bytes())==0 && std::fs::rename(&tag,&full).is_err() {let _=std::fs::remove_file(tag);}
                }
            }
        }
    }
}
/// Small previews only; stale or malformed caches never replace page data.
pub fn store_load_page_preview(nb: &NotebookInfo, pid: u32, out: &mut Canvas) -> bool {
    store_load_page_preview_as(nb,pid,out).is_some()
}
/// The same, with what the picture read is of: the page file identity it shows (store_page_source_identity's
/// then) and the preview file it was read from (store_page_preview_file's, taken before it is read: one written
/// again meanwhile is read again at the next check).
pub fn store_load_page_preview_as(nb: &NotebookInfo, pid: u32, out: &mut Canvas) -> Option<(String, String)> {
    let (_, image, stamp) = page_preview_files(nb, pid)?;
    let file = store_page_preview_file(nb, pid)?;
    let now = store_page_preview_fresh(nb, pid)?;
    if !std::fs::metadata(&image).is_ok_and(|m| m.len() <= 200_000)
        || load_pgm(&image,out) != 0 || out.w > 400 || out.h > 500 {
        // A current stamp over an image that does not read (a cache write cut short: they are never
        // fsynced): the stamp goes, so the preview counts as missing and is made again.
        canvas_free(out);let _=std::fs::remove_file(stamp);return None;
    }
    Some((now, file))
}
/// Page pid's preview file as it is now, whatever page file it is of: its inode, size and modification time (a
/// write is a new file, renamed into place). A preview written again over the same page file (editor.rs
/// retire_kept_before, in place of one 2.0.0 or 2.0.1 made from an old picture) is another file, so a picture
/// decoded from the one before is decoded again (bgjob.rs PreviewCache).
pub fn store_page_preview_file(nb: &NotebookInfo, pid: u32) -> Option<String> {
    let st = std::fs::metadata(page_preview_files(nb, pid)?.1).ok()?;
    Some(format!("{}:{}:{}:{}", st.ino(), st.size(), st.mtime(), st.mtime_nsec()))
}
/// Whether page pid's preview on disk is current, without decoding it: its identity when it is.
pub fn store_page_preview_fresh(nb: &NotebookInfo, pid: u32) -> Option<String> {
    let (source, _, stamp) = page_preview_files(nb, pid)?;
    let now = preview_stamp(&source)?;
    (preview_tag(&stamp).as_deref() == Some(now.as_str())).then_some(now)
}
/// Whether page pid's preview on disk is current and whole, without decoding it: its stamp is the page
/// file's, and its image is as long as its header says (a cache write cut short is shorter: it is made
/// again). What the previews job checks before it leaves a page out.
pub fn store_page_preview_ok(nb: &NotebookInfo, pid: u32) -> bool {
    let Some((_, image, _)) = page_preview_files(nb, pid) else { return false };
    if store_page_preview_fresh(nb, pid).is_none() {
        return false;
    }
    let Ok(mut f) = std::fs::File::open(&image) else { return false };
    let mut head = [0u8; 32];
    let Ok(n) = f.read(&mut head) else { return false };
    let Ok(len) = f.metadata().map(|m| m.len()) else { return false };
    match scan_p5_header(&head[..n]) {
        Some((w, h, 255, off)) if w > 0 && h > 0 && w <= 400 && h <= 500 => len >= (off + 1 + (w * h) as usize) as u64,
        _ => false,
    }
}
/// Page pid's file as it is now (inode, size, times, and the previews' invalidation generation): a
/// decoded preview made at this identity still shows the page.
pub fn store_page_source_identity(nb: &NotebookInfo, pid: u32) -> Option<String> {
    preview_stamp(&page_preview_files(nb, pid)?.0)
}
/// The small preview of a full page canvas, as the cache keeps it (at most 300 x 400).
pub fn store_page_preview_image(canvas: &Canvas, small: &mut Canvas) -> bool {
    if canvas.w <= 0 || canvas.h <= 0 {return false;}
    let scale=(300.0 / canvas.w as f32).min(400.0 / canvas.h as f32).min(1.0);
    if canvas_init(small,(canvas.w as f32*scale).max(1.0) as i32,(canvas.h as f32*scale).max(1.0) as i32)!=0{return false;}
    canvas_downsample_keep_lines(small,canvas,0.35);
    true
}
pub fn store_save_page_preview(nb: &NotebookInfo, pid: u32, canvas: &Canvas) {
    if save_page_preview(nb,pid,canvas) {page_previews_changed(nb.id.as_str());}
}

// ---- drawings kept for dense pages ----
//
// A page that takes long to draw (a dense one took 36 s on the Kindle, 2.0.0-rc.10) keeps its drawing
// beside its preview: the whole page canvas, run-length coded, under the same stamp as the preview
// (the page file's identity and the cache's invalidation generation) plus the renderer's fingerprint
// and the page's size. Opening the page then reads it back instead of drawing every stroke again;
// anything that does not match exactly is drawn as always.

fn page_render_files(nb: &NotebookInfo, pid: u32) -> Option<(String, String, String)> {
    let (source, image, _) = page_preview_files(nb, pid)?;
    let base = image.strip_suffix(".pgm")?.to_string();
    Some((source, format!("{base}.full"), format!("{base}.full.stamp")))
}

/// Time since this page's newest preview or kept drawing was written. A missing cache has no cooldown.
/// Disk time preserves the idle write limit when a page or notebook is reopened.
pub fn store_page_cache_age_ms(nb: &NotebookInfo, pid: u32) -> u32 {
    let Some((_, image, _)) = page_preview_files(nb, pid) else { return u32::MAX };
    let Some((_, full, _)) = page_render_files(nb, pid) else { return u32::MAX };
    [image, full].iter().filter_map(|p| std::fs::metadata(p).ok()?.modified().ok())
        .filter_map(|t| t.elapsed().ok()).map(|d| d.as_millis().min(u32::MAX as u128) as u32)
        .min().unwrap_or(u32::MAX)
}

/// A page whose whole drawing in the editor takes this long or longer keeps it (store_save_page_render), ms.
/// (The previews job goes by store_page_dense instead: it keeps a dense page's drawing, however quick, and no
/// other page's, however slow.)
pub const STORE_KEEP_RENDER_MS: u32 = 1000;
/// Tests: the threshold in place of STORE_KEEP_RENDER_MS (u32::MAX: none), so a unit test can count every
/// drawing as slow (and see that the previews job still keeps no light page's).
pub static STORE_TEST_KEEP_RENDER_MS: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(u32::MAX);
/// STORE_KEEP_RENDER_MS, or what a test asks for. (The simulator takes GRIMOIRE_KEEP_RENDER_MS instead,
/// so scenarios can keep pages that draw quickly.)
pub fn store_keep_render_ms() -> u32 {
    let test = STORE_TEST_KEEP_RENDER_MS.load(std::sync::atomic::Ordering::Relaxed);
    if test != u32::MAX {
        return test;
    }
    #[cfg(feature = "sim")]
    if let Some(ms) = std::env::var("GRIMOIRE_KEEP_RENDER_MS").ok().and_then(|v| v.parse().ok()) {
        return ms;
    }
    STORE_KEEP_RENDER_MS
}
/// A dense page: a page file this big or bigger, whose drawing the previews job keeps (store_page_dense).
/// It draws for seconds on a Kindle (the owner's 0.6 MB page took 5 s, a 2.4 MB one 32 s).
pub const STORE_DENSE_PAGE_BYTES: u64 = 512 * 1024;
/// A slow page: a page file this big or bigger, which says "Loading page..." as it opens with no current kept
/// drawing (store_page_slow). A page of handwriting is under 0.3 MB and draws in well under a second; one of
/// about 0.3 to 0.5 MB could take 2 to 4 s, and said nothing while the notice went by STORE_DENSE_PAGE_BYTES
/// too (the owner, 2026-11-04: the notice from 256 KB, kept drawings still from 512 KB).
pub const STORE_SLOW_PAGE_BYTES: u64 = 256 * 1024;
// Whether the page file at `source` is `bytes` or bigger (STORE_DENSE_PAGE_BYTES: store_page_dense's size,
// and the simulator's slow drawing in the previews job; STORE_SLOW_PAGE_BYTES: store_page_slow's).
fn page_file_at_least(source: &str, bytes: u64) -> bool {
    std::fs::metadata(source).is_ok_and(|m| m.len() >= bytes)
}
/// Whether page pid is dense, told without reading it: its file is STORE_DENSE_PAGE_BYTES or bigger. The
/// previews job keeps the drawing of each dense page it draws, and of no other page (store_build_page_previews;
/// the owner, 2026-11-04: "ordinary pages save nothing extra"). This was the one definition "Loading page..."
/// went by too (the owner's ruling then: "Save a drawing only for pages of 512 KB or more, the same threshold
/// that shows "Loading page...""), until the owner's later decision the same day: pages of about 0.3 to 0.5 MB
/// could sit 2 to 4 s on their first open with nothing said, so the notice now goes by store_page_slow (256 KB
/// or more) while the job's rule stays here at 512 KB. A page between the two says "Loading page..." whenever
/// it opens with no current kept drawing, and the job keeps none of it. A kept drawing does not make a page
/// dense: the editor keeps one for a page it took store_keep_render_ms() to draw (on a Kindle, most pages
/// between the two at their first open), and an earlier build of 2.0.0-beta.2's previews job kept one for any
/// page it drew in a second of processor time, many ordinary pages on a Kindle. Counted, each of those would
/// keep a drawing from the job for good.
pub fn store_page_dense(nb: &NotebookInfo, pid: u32) -> bool {
    page_render_files(nb, pid).is_some_and(|(source, _, _)| page_file_at_least(&source, STORE_DENSE_PAGE_BYTES))
}
/// Whether drawing page pid takes long enough to say so as it opens (seconds on a Kindle), told without
/// reading it: its file is STORE_SLOW_PAGE_BYTES or bigger, so every dense page is slow too. Opening one with
/// no current kept drawing says "Loading page..." (editor.rs loading_toast_due). Split from store_page_dense on
/// the owner's decision of 2026-11-04 (the notice from 256 KB, kept drawings still from 512 KB); a kept drawing
/// does not make a page slow either.
pub fn store_page_slow(nb: &NotebookInfo, pid: u32) -> bool {
    page_render_files(nb, pid).is_some_and(|(source, _, _)| page_file_at_least(&source, STORE_SLOW_PAGE_BYTES))
}
// Whether page pid has a kept drawing of its file as it is now, at the notebook's page size, by this
// renderer, layered or not (that is told only by reading the page): the previews job's guess that the page
// is quick to make, before it reads the page (store_load_page_render still checks it all). A drawing kept
// by another build (another fingerprint) or at another size is drawn again, and so is sorted by its size.
fn page_render_current(nb: &NotebookInfo, pid: u32) -> bool {
    let Some((source, _, stamp)) = page_render_files(nb, pid) else { return false };
    let (Some(now), Some(tag)) = (preview_stamp(&source), preview_tag(&stamp)) else { return false };
    let (mut w, mut h) = (0, 0);
    store_notebook_page_size(nb, &mut w, &mut h);
    tag == render_tag(&now, w, h, false) || tag == render_tag(&now, w, h, true)
}
// A layered page (page_layered: floors, 2.0.0-beta.2 on) is tagged with a "|L" suffix and only such a
// tag is trusted for it: 2.0.0-beta.1 keeps drawings under the same fingerprint, and would keep a flat
// drawing of a layered page (its scoped erasure lightening the ink under it). Unlayered pages kept
// their tags exactly then, so no kept drawing was retired.
//
// Since 2.1.0 the tag carries the kept drawings' own version after the fingerprint, "|k2", and a drawing kept by an
// earlier build (render_tag_is_before) is never read: 2.0.0 and 2.0.1 could keep the old picture of a page just
// cleared or given a new paper under the new file's stamp (editor.rs page_is_its_file), and nothing in such a tag
// tells it from a true one. Every page with a kept drawing is drawn once more after the update (on a Kindle, 11 and
// 28 s for the owner's two stress pages, saying "Loading page..."), and keeps its drawing again.
const RENDER_TAG_VERSION: u32 = 2;
fn render_tag(stamp: &str, w: i32, h: i32, layered: bool) -> String {
    format!("{stamp}|{w}x{h}|{:016x}|k{RENDER_TAG_VERSION}{}", page_render_fingerprint(), if layered { "|L" } else { "" })
}
// Whether a kept drawing's tag is one 2.0.0 or 2.0.1 wrote (store_retire_page_render): a stamp, |WxH| and a
// fingerprint, then "|L" or nothing, with no kept drawings' version. Whatever the stamp, size or renderer.
fn render_tag_is_before(tag: &str) -> bool {
    let mut parts = tag.split('|').skip(1);
    let size = parts.next().and_then(|p| p.split_once('x')).is_some_and(|(w, h)| w.parse::<u32>().is_ok() && h.parse::<u32>().is_ok());
    let fingerprint = parts.next().is_some_and(|p| p.len() == 16 && p.bytes().all(|b| b.is_ascii_hexdigit()));
    size && fingerprint && matches!((parts.next(), parts.next()), (None, None) | (Some("L"), None))
}
// Runs of one byte (most of a page is paper, much of the rest one ink level) as (count - 1) | 0x80
// then the byte; anything else as count - 1 then that many bytes. A dense page shrank 3 to 6 times.
fn rle_encode(px: &[u8], out: &mut Vec<u8>) {
    let n = px.len();
    let mut i = 0;
    while i < n {
        let mut run = 1;
        while i + run < n && run < 128 && px[i + run] == px[i] {
            run += 1;
        }
        if run >= 3 {
            out.push(0x80 | (run - 1) as u8);
            out.push(px[i]);
            i += run;
            continue;
        }
        let start = i;
        let mut lit = 0;
        while i < n && lit < 128 {
            if i + 2 < n && px[i] == px[i + 1] && px[i] == px[i + 2] {
                break;
            }
            i += 1;
            lit += 1;
        }
        out.push((lit - 1) as u8);
        out.extend_from_slice(&px[start..start + lit]);
    }
}
fn rle_decode(data: &[u8], px: &mut [u8]) -> bool {
    let (mut i, mut o) = (0, 0);
    while i < data.len() {
        let c = data[i] as usize;
        i += 1;
        if c & 0x80 != 0 {
            let run = (c & 0x7F) + 1;
            let Some(&b) = data.get(i) else { return false };
            if o + run > px.len() {
                return false;
            }
            px[o..o + run].fill(b);
            i += 1;
            o += run;
        } else {
            let lit = c + 1;
            if i + lit > data.len() || o + lit > px.len() {
                return false;
            }
            px[o..o + lit].copy_from_slice(&data[i..i + lit]);
            i += lit;
            o += lit;
        }
    }
    o == px.len()
}
/// Keeps page pid's drawing (its whole canvas, as drawn from the page file now on disk): a cache file,
/// written without waiting on the disk. False if it was not kept.
pub fn store_save_page_render(nb: &NotebookInfo, pid: u32, canvas: &Canvas, layered: bool) -> bool {
    let Some((source, _, _)) = page_render_files(nb, pid) else { return false };
    let Some(was) = preview_stamp(&source) else { return false };
    store_save_page_render_at(nb, pid, canvas, layered, &was)
}
/// The same for a drawing of the page file as it was at identity `was` (store_page_source_identity's
/// when it was read, as the previews job reads a page): kept only while the file is still that one, so a
/// page written meanwhile never keeps a drawing of what it was. A file that changes as the drawing is put
/// in place takes the drawing's stamp away with it (drawn again next time).
pub fn store_save_page_render_at(nb: &NotebookInfo, pid: u32, canvas: &Canvas, layered: bool, was: &str) -> bool {
    let Some((source, image, stamp)) = page_render_files(nb, pid) else { return false };
    if preview_stamp(&source).as_deref() != Some(was) {
        return false;
    }
    let Some(parent) = std::path::Path::new(&image).parent() else { return false };
    if canvas.w <= 0 || canvas.h <= 0 || canvas.stride != canvas.w || std::fs::create_dir_all(parent).is_err() {
        return false;
    }
    let mut bytes = Vec::new();
    if bytes.try_reserve(canvas.px.len() / 3).is_err() {
        return false;
    }
    rle_encode(&canvas.px[..(canvas.w * canvas.h) as usize], &mut bytes);
    let mine = format!("{image}.{}", std::process::id());
    let tag = format!("{stamp}.{}", std::process::id());
    let mut saved = false;
    if write_cache_file(&mine, &bytes) == 0 && preview_stamp(&source).as_deref() == Some(was)
        && std::fs::rename(&mine, &image).is_ok() {
        saved = write_cache_file(&tag, render_tag(was, canvas.w, canvas.h, layered).as_bytes()) == 0 && std::fs::rename(&tag, &stamp).is_ok();
        if saved && preview_stamp(&source).as_deref() != Some(was) {
            // (the page written while the drawing went in: it is not that page's drawing)
            let _ = std::fs::remove_file(&stamp);
            saved = false;
        }
    }
    let _ = std::fs::remove_file(mine);
    let _ = std::fs::remove_file(tag);
    saved
}
/// Page pid's kept drawing into out (a canvas of the page's size, w x h), when it is current: drawn
/// from the page file now on disk, by this renderer, at this size. False otherwise (out unchanged).
pub fn store_load_page_render(nb: &NotebookInfo, pid: u32, w: i32, h: i32, out: &mut Canvas, layered: bool) -> bool {
    let Some((source, image, stamp)) = page_render_files(nb, pid) else { return false };
    let Some(now) = preview_stamp(&source) else { return false };
    if preview_tag(&stamp).as_deref() != Some(render_tag(&now, w, h, layered).as_str()) {
        return false;
    }
    if out.w != w || out.h != h || out.stride != w {
        return false;
    }
    let Some(data) = read_file(&image) else { return false };
    let mut px = Vec::new();
    if px.try_reserve_exact((w * h) as usize).is_err() {
        return false;
    }
    px.resize((w * h) as usize, 0);
    if !rle_decode(&data, &mut px) {
        // (a write cut short: its stamp goes, so it is drawn and kept again)
        let _ = std::fs::remove_file(stamp);
        return false;
    }
    out.px[..(w * h) as usize].copy_from_slice(&px);
    true
}
// Whether page pid has a drawing kept by 2.0.0 or 2.0.1 (render_tag_is_before), current for the page file or not.
fn page_render_before(nb: &NotebookInfo, pid: u32) -> bool {
    page_render_files(nb, pid).is_some_and(|(_, _, stamp)| preview_tag(&stamp).is_some_and(|tag| render_tag_is_before(&tag)))
}
/// Takes away page pid's drawing kept by 2.0.0 or 2.0.1 (render_tag_is_before, never read since 2.1.0), current for
/// the page file or not, and the Pages preview beside it: true when there was one. Such a drawing may show the page
/// as it was before a Clear or a new paper (editor.rs page_is_its_file), and so may the preview and the Library card
/// made beside it, even once the page file changed again (an idle save inside the previews' 30 s writes the file
/// alone, and so does a notebook closed before its idle save): the editor, which has just drawn the page from its
/// strokes, makes them again (the preview, if it cannot be written, is made by the previews job from the page file).
/// The previews job keeps no drawing over such a one (store_build_page_previews), so the editor finds it there.
pub fn store_retire_page_render(nb: &NotebookInfo, pid: u32) -> bool {
    let (Some((_, image, stamp)), Some((_, small, small_stamp))) = (page_render_files(nb, pid), page_preview_files(nb, pid)) else { return false };
    if !page_render_before(nb, pid) {
        return false;
    }
    for file in [stamp, image, small_stamp, small] {
        let _ = std::fs::remove_file(file);
    }
    true
}
/// Whether page pid has a current kept drawing (it is kept fresh from then on, at every save).
pub fn store_page_render_kept(nb: &NotebookInfo, pid: u32, w: i32, h: i32, layered: bool) -> bool {
    let Some((source, _, stamp)) = page_render_files(nb, pid) else { return false };
    let Some(now) = preview_stamp(&source) else { return false };
    preview_tag(&stamp).as_deref() == Some(render_tag(&now, w, h, layered).as_str())
}
// One page's preview written, without telling an open view (the caller does: store_save_page_preview, and
// the previews job as each preview lands).
fn save_page_preview(nb: &NotebookInfo, pid: u32, canvas: &Canvas) -> bool {
    let Some((source, image, stamp)) = page_preview_files(nb,pid) else {return false};
    let Some(was) = preview_stamp(&source) else {return false};
    let Some(parent) = std::path::Path::new(&image).parent() else {return false};
    if std::fs::create_dir_all(parent).is_err() {return false;}
    let mut small=Canvas::default();
    if !store_page_preview_image(canvas,&mut small) {return false;}
    let Some(bytes)=pgm_bytes(&small) else {return false};
    let mine=format!("{image}.{}",std::process::id());
    let tag=format!("{stamp}.{}",std::process::id());
    let mut saved=false;
    if write_cache_file(&mine,&bytes)==0 && preview_stamp(&source).as_deref()==Some(was.as_str())
        && std::fs::rename(&mine,&image).is_ok() {
        saved=write_cache_file(&tag,was.as_bytes())==0 && std::fs::rename(&tag,&stamp).is_ok();
    }
    let _=std::fs::remove_file(mine);let _=std::fs::remove_file(tag);
    saved
}

/// thumb.pgm as it is on disk now, in a number that changes whenever the file is written again (a
/// write is a new file, renamed into place: its inode, size and times): 0 when there is none, else
/// odd. A card read from it knows by this whether it was written since.
pub fn store_thumb_stamp(id: &str) -> u64 {
    let Some(dir) = notebook_dir(id.as_bytes()) else {
        return 0;
    };
    let path = cformat!("%s/thumb.pgm", &dir);
    let Ok(st) = std::fs::metadata(&path) else {
        return 0;
    };
    let v = [
        st.dev(),
        st.ino(),
        st.size(),
        st.mtime() as u64,
        st.ctime() as u64,
    ];
    let mut h: u64 = 1469598103934665603; // FNV-1a over them
    for x in v {
        for b in (0..64).step_by(8) {
            h = (h ^ ((x >> b) & 0xFF)).wrapping_mul(1099511628211);
        }
    }
    h | 1
}

/// From a background job, which draws a notebook's card while the editor may have the notebook open
/// and write the card itself: thumb.pgm's bytes as they are now (NULL with *len 0 when there is
/// none), and the card written only if thumb.pgm still holds those bytes (was, was_len), checked
/// once the new card is written to a file of the job's own (thumb.pgm.card.tmp, so that the two
/// never write into one file), just before it replaces the old one. 0 written; 1 the card changed
/// meanwhile (it stays as it is); -1 not written.
pub fn store_read_thumb_file(id: &str) -> Option<Vec<u8>> {
    let dir = notebook_dir(id.as_bytes())?;
    let path = cformat!("%s/thumb.pgm", &dir);
    read_file(&path)
}

/// Tests: called by store_replace_thumb_if_same just before it looks at thumb.pgm again (the editor
/// writing a card meanwhile).
pub static STORE_TEST_BEFORE_CARD_REPLACE: std::sync::Mutex<Option<fn(&str)>> =
    std::sync::Mutex::new(None);

pub fn store_replace_thumb_if_same(id: &str, thumb: &Canvas, was: &[u8]) -> i32 {
    let Some(dir) = notebook_dir(id.as_bytes()) else {
        return -1;
    };
    let path = cformat!("%s/thumb.pgm", &dir);
    let mut mine = String::new();
    csnprintf!(&mut mine, 720, "%s.card.tmp", &path);
    if save_pgm(&mine, thumb) != 0 {
        return -1;
    }
    let hook = *STORE_TEST_BEFORE_CARD_REPLACE
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    if let Some(f) = hook {
        f(id);
    }
    // (`was` empty is both the C's NULL, no card was there, and a card of no bytes: the first is
    // what a reader finds; a card of no bytes is no card Grimoire writes.)
    let same = match read_file(&path) {
        Some(now) => now == was,
        None => was.is_empty(),
    };
    if !same || std::fs::rename(&mine, &path).is_err() {
        let _ = std::fs::remove_file(&mine);
        return if same { -1 } else { 1 };
    }
    // best effort: the rename itself durable
    if let Ok(d) = std::fs::File::open(&dir) {
        let _ = d.sync_all();
    }
    0
}

/// The pinned sleep screen (a full page, P5 PGM at root/sleep.pgm).
pub fn store_save_sleep_image(c: &Canvas) -> i32 {
    let path = cformat!("%s/sleep.pgm", &g_root());
    save_pgm(&path, c)
}

pub fn store_load_sleep_image(out: &mut Canvas) -> i32 {
    let path = cformat!("%s/sleep.pgm", &g_root());
    load_pgm(&path, out)
}

/// stbi_info_from_memory: (w, h, channels) of a picture stb_image reads.
fn stbi_info(data: &[u8]) -> Option<(i32, i32, i32)> {
    let (mut w, mut h, mut ch) = (0, 0, 0);
    // SAFETY: reads at most data.len() bytes of data and writes the three ints, all ours.
    let ok = unsafe {
        crate::ffi_stb::stbi_info_from_memory(
            data.as_ptr(),
            data.len() as libc::c_int,
            &mut w,
            &mut h,
            &mut ch,
        )
    };
    (ok != 0).then_some((w, h, ch))
}

/// stbi_load_from_memory asking for one channel: the pixels (w x h bytes) and the size.
fn stbi_load_gray(data: &[u8]) -> Option<(Vec<u8>, i32, i32)> {
    let (mut w, mut h, mut ch) = (0, 0, 0);
    // SAFETY: reads at most data.len() bytes of data and writes the three ints, all ours; on success
    // it returns w * h bytes (one channel was asked for) that we copy out and free below.
    unsafe {
        let p = crate::ffi_stb::stbi_load_from_memory(
            data.as_ptr(),
            data.len() as libc::c_int,
            &mut w,
            &mut h,
            &mut ch,
            1,
        );
        if p.is_null() {
            return None;
        }
        let v = std::slice::from_raw_parts(p, (w.max(0) as usize) * (h.max(0) as usize)).to_vec();
        crate::ffi_stb::stbi_image_free(p as *mut libc::c_void);
        Some((v, w, h))
    }
}

pub fn store_load_koreader_sleep(out: &mut Canvas) -> i32 {
    let path = cformat!("%s/koreader-sleep.png", &g_root());
    let Ok(st) = std::fs::metadata(&path) else {
        return -1;
    };
    if st.size() == 0 || st.size() > 32 * 1024 * 1024 {
        return -1;
    }
    let Some(encoded) = read_file(&path) else {
        return -1;
    };
    let Some((w, h, _)) = stbi_info(&encoded) else {
        return -1;
    };
    if w < 1 || h < 1 || w > 6000 || h > 6000 || (w as usize) * (h as usize) > 16000000 {
        return -1;
    }
    let Some((data, w, h)) = stbi_load_gray(&encoded) else {
        return -1;
    };
    drop(encoded);
    let rc = canvas_init(out, w, h);
    if rc == 0 {
        let n = (w as usize) * (h as usize);
        out.px[..n].copy_from_slice(&data[..n]);
    }
    rc
}

// ---- settings ----

static K_TOOL_KEYS: [&str; TOOL_COUNT] = [
    "pencil",
    "mech",
    "ballpoint",
    "eraser",
    "highlighter",
    "fountain",
    "fineliner",
    "marker",
    "charcoal",
    "calligraphy",
];

pub fn settings_default(s: &mut Settings) {
    *s = Settings::default();
    for t in 0..TOOL_COUNT {
        tool_params_default(ToolKind(t as i32), &mut s.tools[t]);
    }
    s.current_tool = TOOL_PENCIL;
    s.draw_tool = TOOL_PENCIL;
    s.music_layout = music_default();
    s.panel_x = -1;
    s.panel_y = -1;
    s.side_button = 2; // highlighter
    s.mirror_pdf = true;
    s.follow_rotation = true;
    s.pinch_zoom = true;
    s.theme = 0;
    s.font = 0; // Terminess
    s.sleep_mode = 0;
    s.sleep_minutes = 10;
    s.http_port = 8765;
    s.language.set("auto");
    s.eraser_mode = 0; // whole strokes, as before 1.1
    s.white = false;
    s.before_eraser = -1;
    s.status_on = true;
    s.status_size = 1;
    s.status_corner = 0;
    s.clock_12h = false;
    s.rotation_lock = 0;
    s.gestures = true;
    s.landscape_fit = true;
    s.gesture_new_page = true;
    s.status_mode = 1;
    s.shapes = true;
    s.shape_hold_ms = 550;
    s.shape_quads = 1;
    s.shape_round = 1;
    s.shape_links = true;
    s.shape_min_mm = 10;
    s.whatsnew_seen.clear();
    s.whatsnew_hide = false;
    s.shape_corner = 0;
    s.whatsnew_due = false;
    s.hints = 0;
    s.link_areas = false;
}

/// pen.panel / touch.panel: the panel the calibration was made on, "WxH" (1.1 on). A value that does
/// not read as a panel's size is a line this version does not understand: kept as it is, and the
/// calibration's panel is not recorded.
fn panel_kv(pw: &mut i32, ph: &mut i32, key: &str, v: &[u8]) -> bool {
    let (w, used) = size_value(v);
    let h = if v.get(used) == Some(&b'x') {
        size_value(&v[used + 1..]).0
    } else {
        0
    };
    if !page_size_ok(w as i64, h as i64) {
        return false;
    }
    *pw = w;
    *ph = h;
    let used = cformat!("%dx%d", w, h);
    keep_value(
        key,
        v,
        if v != used.as_bytes() {
            Some(&used)
        } else {
            None
        },
    );
    true
}
/// key: the whole settings.ini key ("pen.valid"), k: what follows the dot.
fn calib_kv(c: &mut InputCalib, pw: &mut i32, ph: &mut i32, key: &str, k: &[u8], v: &[u8]) -> bool {
    match k {
        b"valid" => c.valid = keep_bool(key, v),
        b"swap" => c.swap_xy = keep_bool(key, v),
        b"ax" => c.ax = c_strtof(v),
        b"bx" => c.bx = c_strtof(v),
        b"ay" => c.ay = c_strtof(v),
        b"by" => c.by = c_strtof(v),
        b"panel" => return panel_kv(pw, ph, key, v),
        _ => return false,
    }
    true
}

/// bytes of unknown lines kept (a bound against a runaway file)
const SETTINGS_UNKNOWN_MAX: usize = 65536;
const SETTINGS_KEPT_MAX: usize = 64;

fn forget_unknown_settings() {
    with(|s| {
        s.g_set_unknown = Vec::new();
        s.g_kept.clear();
    });
}

fn settings_unknown(k: &[u8], v: &[u8]) {
    let need = k.len() + v.len() + 2;
    with(|s| {
        if s.g_set_unknown.len() + need > SETTINGS_UNKNOWN_MAX {
            s.g_set_dropped += 1;
            return;
        }
        s.g_set_unknown.extend_from_slice(k);
        s.g_set_unknown.push(b'=');
        s.g_set_unknown.extend_from_slice(v);
        s.g_set_unknown.push(b'\n');
    });
}

/// The value settings.ini had for a known key that this build could not use as written, kept to be written
/// back (keep_value), None when there is none: the probes' settings.kept.KEY.
pub fn settings_kept(k: &str) -> Option<Vec<u8>> {
    with(|s| s.g_kept.iter().find(|e| e.key == k).map(|e| e.raw.clone()))
}

/// A known key read: `used` is what this build made of `raw` when it could not use it as written,
/// None when it could (which forgets an earlier line's kept value: the last line counts).
fn keep_value(k: &str, raw: &[u8], used: Option<&str>) {
    with(|s| {
        if let Some(i) = s.g_kept.iter().position(|e| e.key == k) {
            s.g_kept.swap_remove(i);
        }
        let Some(used) = used else { return };
        if s.g_kept.len() == SETTINGS_KEPT_MAX || k.len() >= 48 || raw.len() > 256 {
            return;
        }
        s.g_kept.push(Kept {
            key: k.to_string(),
            used: cut_at_bytes(used, 63).to_string(),
            raw: raw.to_vec(),
        });
    });
}

/// The pinned pens: whether t is one; t pinned after the others, or unpinned (the others keep their
/// order), false for the eraser, a tool past the list, or a pin past PANEL_PINS_MAX; and their
/// settings.ini names in pin order, comma-separated, as panel_tools= has them ("" for none).
pub fn settings_pinned(s: &Settings, t: ToolKind) -> bool {
    for i in 0..(s.npins.max(0) as usize).min(PANEL_PINS_MAX) {
        if s.pins[i] == t {
            return true;
        }
    }
    false
}

pub fn settings_toggle_pin(s: &mut Settings, t: ToolKind) -> bool {
    if t.0 < 0 || t.0 as usize >= TOOL_COUNT || t == TOOL_ERASER {
        return false;
    }
    for i in 0..s.npins.max(0) as usize {
        if s.pins[i] != t {
            continue;
        }
        s.pins.copy_within(i + 1..s.npins as usize, i);
        s.npins -= 1;
        return true;
    }
    if s.npins as usize >= PANEL_PINS_MAX {
        return false;
    }
    s.pins[s.npins as usize] = t;
    s.npins += 1;
    true
}

pub fn settings_pins_text(s: &Settings, out: &mut String, cap: usize) {
    let mut n = 0;
    let mut text = String::new();
    for i in 0..(s.npins.max(0) as usize).min(PANEL_PINS_MAX) {
        if n >= cap {
            break;
        }
        let piece = cformat!(
            "%s%s",
            if i > 0 { "," } else { "" },
            K_TOOL_KEYS[s.pins[i].0 as usize]
        );
        n += piece.len();
        text.push_str(&piece);
    }
    if cap != 0 {
        strlcpy_q(out, &text, cap);
    }
}

/// panel_tools: the pinned pens' names in pin order, comma-separated (spaces around a name are
/// allowed). A name this version does not have (a newer one's pen), the eraser (it has its own
/// button) and a name already read are left out, and the first PANEL_PINS_MAX left are the pins.
/// A value read so is kept as it was until the pins change (keep_value).
fn pins_kv(s: &mut Settings, k: &str, v: &[u8]) {
    s.npins = 0;
    let mut p = v;
    while !p.is_empty() {
        let e = p.iter().position(|&b| b == b',');
        let n = e.unwrap_or(p.len());
        let mut w = &p[..n];
        while let [b' ', rest @ ..] = w {
            w = rest;
        }
        while let [rest @ .., b' '] = w {
            w = rest;
        }
        for t in 0..TOOL_COUNT {
            if s.npins as usize >= PANEL_PINS_MAX {
                break;
            }
            if ToolKind(t as i32) != TOOL_ERASER
                && K_TOOL_KEYS[t].as_bytes() == w
                && !settings_pinned(s, ToolKind(t as i32))
            {
                s.pins[s.npins as usize] = ToolKind(t as i32);
                s.npins += 1;
            }
        }
        p = match e {
            Some(i) => &p[i + 1..],
            None => &p[n..],
        };
    }
    let mut used = String::new();
    settings_pins_text(s, &mut used, 128);
    keep_value(
        k,
        v,
        if v != used.as_bytes() {
            Some(&used)
        } else {
            None
        },
    );
}

fn keep_int(k: &str, v: &[u8], lo: i32, hi: i32) -> i32 {
    let x = c_atoi(v);
    let c = clampi(x, lo, hi);
    let used = cformat!("%d", c);
    keep_value(k, v, if x != c { Some(&used) } else { None });
    c
}

/// A 0/1 setting: any other value (a newer version's third state, invert=2) is used as on or off
/// (atoi, as 1.0.0-beta reads it) and written back as it was until the setting changes.
fn keep_bool(k: &str, v: &[u8]) -> bool {
    let on = c_atoi(v) != 0;
    keep_value(
        k,
        v,
        if v != b"0" && v != b"1" {
            Some(if on { "1" } else { "0" })
        } else {
            None
        },
    );
    on
}

fn settings_cb(s: &mut Settings, k: &[u8], v: &[u8]) -> bool {
    if let Some(dot) = k.iter().position(|&b| b == b'.') {
        let hl = dot;
        if hl >= 32 {
            return false;
        }
        let head = &k[..hl];
        let field = &k[dot + 1..];
        let key = std::str::from_utf8(k).unwrap_or("");
        if head == b"pen" {
            let (pw, ph) = (&mut s.pen_panel_w, &mut s.pen_panel_h);
            return calib_kv(&mut s.pen_calib, pw, ph, key, field, v);
        }
        if head == b"touch" {
            let (pw, ph) = (&mut s.touch_panel_w, &mut s.touch_panel_h);
            return calib_kv(&mut s.touch_calib, pw, ph, key, field, v);
        }
        for t in 0..TOOL_COUNT {
            if head != K_TOOL_KEYS[t].as_bytes() {
                continue;
            }
            for pi in tool_param_table() {
                if field != pi.key.as_bytes() {
                    continue;
                }
                let f = c_strtof(v);
                // (not a number: the value is left as it is)
                if !f.is_nan() {
                    let c = clampf(f, pi.min, pi.max);
                    *tool_param_ptr(&mut s.tools[t], pi) = c;
                    let used = cformat!("%.4f", c);
                    keep_value(key, v, if f != c { Some(&used) } else { None });
                }
                return true;
            }
            return false; // a setting this version's tools do not have
        }
        return false; // a tool (or group) this version does not have
    }
    match k {
        b"current_tool" => {
            s.current_tool = ToolKind(keep_int("current_tool", v, 0, TOOL_COUNT as i32 - 1))
        }
        b"music_layout" => {
            let x = c_strtoul(v).0 as u32;
            s.music_layout = paper_sanitize(x, music_default());
            let used = cformat!("%u", s.music_layout.0);
            keep_value(
                "music_layout",
                v,
                if s.music_layout.0 != x {
                    Some(&used)
                } else {
                    None
                },
            );
        }
        b"draw_tool" => s.draw_tool = ToolKind(keep_int("draw_tool", v, 0, TOOL_COUNT as i32 - 1)),
        b"panel_x" => s.panel_x = c_atoi(v),
        b"panel_y" => s.panel_y = c_atoi(v),
        b"panel_collapsed" => s.panel_collapsed = keep_bool("panel_collapsed", v),
        b"side_button" => s.side_button = keep_int("side_button", v, 0, 3),
        b"mirror_pdf" => s.mirror_pdf = keep_bool("mirror_pdf", v),
        b"follow_rotation" => s.follow_rotation = keep_bool("follow_rotation", v),
        b"invert" => s.invert = keep_bool("invert", v),
        b"pinch_zoom" => s.pinch_zoom = keep_bool("pinch_zoom", v),
        b"theme" => {
            s.theme = if v == b"classic" { 1 } else { 0 };
            keep_value(
                "theme",
                v,
                if s.theme == 1 || v == b"gothic" {
                    None
                } else {
                    Some("gothic")
                },
            );
        }
        b"font" => {
            s.font_file.clear();
            if let Some(rel) = v.strip_prefix(b"file:") {
                // A font of the owner's own (2.1.0-beta.2): its path under grimoire/fonts. Whether the file is
                // there and passed its check is found when it is loaded (userfonts.rs): until then, and for a
                // path no font file can have, Terminess draws, and the line stays in the file.
                s.font = 0;
                match std::str::from_utf8(rel).ok().filter(|r| crate::userfonts::userfont_rel_ok(r)) {
                    Some(r) => {
                        s.font_file.set(r);
                        keep_value("font", v, None);
                    }
                    None => {
                        qlog_once!("settings: font=file: names no font file this version can use: Terminess");
                        keep_value("font", v, Some(FONT_FAMILIES[0].id));
                    }
                }
            } else {
                // A font this version does not have (a newer one's) draws as Terminess, and stays in the file.
                let found = font_family_find(v);
                if found.is_none() {
                    qlog_once!("settings: font=%s is not a font of this version: Terminess", &String::from_utf8_lossy(v).into_owned());
                }
                s.font = found.unwrap_or(0) as i32;
                keep_value("font", v, if found.is_some() { None } else { Some(FONT_FAMILIES[0].id) });
            }
        }
        b"font_bold" => {
            // The Bold file of a custom font; anything else (empty, a value of a newer version) is not used
            // and stays in the file.
            s.font_bold.clear();
            match v.strip_prefix(b"file:").and_then(|r| std::str::from_utf8(r).ok()).filter(|r| crate::userfonts::userfont_rel_ok(r)) {
                Some(r) => {
                    s.font_bold.set(r);
                    keep_value("font_bold", v, None);
                }
                None => keep_value("font_bold", v, Some("")),
            }
        }
        b"shortcuts_enabled" => s.shortcuts_enabled = keep_bool("shortcuts_enabled", v),
        b"controller" => s.controller = keep_bool("controller", v),
        b"updates" => {
            s.updates = if v == b"stable" {
                1
            } else if v == b"preview" {
                2
            } else {
                0
            }
        }
        b"sleep_mode" => s.sleep_mode = keep_int("sleep_mode", v, 0, 3),
        b"sleep_minutes" => s.sleep_minutes = keep_int("sleep_minutes", v, 0, 240),
        b"settings_version" => s.version = c_atoi(v),
        // The port is saved as used, never kept: the launcher reads it from the file to open the firewall.
        b"http_port" => s.http_port = clampi(c_atoi(v), 1024, 65535),
        b"passcode_told" => s.passcode_told = keep_bool("passcode_told", v),
        b"language" => {
            // A language this version does not speak (a newer one's) acts as auto, and stays in the file.
            let known = std::str::from_utf8(v).is_ok_and(i18n_setting_known);
            s.language.set_bytes(if known { v } else { b"auto" });
            keep_value("language", v, if known { None } else { Some("auto") });
        }
        b"eraser_mode" => s.eraser_mode = keep_int("eraser_mode", v, 0, 1),
        b"white" => s.white = keep_bool("white", v),
        b"panel_tools" => pins_kv(s, "panel_tools", v),
        b"status_on" => s.status_on = keep_bool("status_on", v),
        b"status_size" => s.status_size = keep_int("status_size", v, 0, 2),
        b"gestures" => s.gestures = keep_bool("gestures", v),
        b"landscape_fit" => s.landscape_fit = keep_bool("landscape_fit", v),
        b"hints" => s.hints = keep_int("hints", v, 0, i32::MAX),
        b"link_areas" => s.link_areas = keep_bool("link_areas", v),
        b"status_corner" => s.status_corner = keep_int("status_corner", v, 0, 3),
        b"clock_12h" => s.clock_12h = keep_bool("clock_12h", v),
        b"shapes" => s.shapes = keep_bool("shapes", v),
        b"shape_hold_ms" => s.shape_hold_ms = keep_int("shape_hold_ms", v, 300, 1500),
        b"shape_quads" => s.shape_quads = keep_int("shape_quads", v, 0, 2),
        b"shape_round" => s.shape_round = keep_int("shape_round", v, 0, 2),
        b"shape_links" => s.shape_links = keep_bool("shape_links", v),
        // (0 was rc.9 to rc.11's "Any", which still meant the recognizer's 10 mm: it reads as that)
        b"shape_min_mm" => s.shape_min_mm = match keep_int("shape_min_mm", v, 0, 40) { 0 => 10, mm => mm.max(3) },
        b"whatsnew_seen" => clean_line(&mut s.whatsnew_seen, v),
        b"whatsnew_hide" => s.whatsnew_hide = keep_bool("whatsnew_hide", v),
        b"shape_corner" => s.shape_corner = keep_int("shape_corner", v, 0, 100),
        b"rotation_lock" => s.rotation_lock = keep_int("rotation_lock", v, 0, 4),
        b"status_mode" => s.status_mode = keep_int("status_mode", v, 0, 2),
        b"gesture_new_page" => s.gesture_new_page = keep_bool("gesture_new_page", v),
        b"before_eraser" => {
            // A pen this version does not have (a newer one's), the eraser or no number at all is none
            // here, and stays in the file.
            let (x, used) = c_strtol(v);
            s.before_eraser = if used != 0
                && used == v.len()
                && x >= 0
                && x < TOOL_COUNT as i64
                && x != TOOL_ERASER.0 as i64
            {
                x as i32
            } else {
                -1
            };
            let used = cformat!("%d", s.before_eraser);
            keep_value(
                "before_eraser",
                v,
                if v != used.as_bytes() {
                    Some(&used)
                } else {
                    None
                },
            );
        }
        _ => return false,
    }
    true
}

/// Defaults that changed between versions. A saved value still equal to the old default was never
/// tuned by the owner, so it moves to the new default; anything they changed stays as it is. Each
/// change applies once, to files from before the version that brought it.
fn settings_upgrade(s: &mut Settings) {
    // (since: the settings_version with the new default; the tool; the ToolParams field, by its
    // index (the C's offsetof): 0 size_min_mm, 1 size_max_mm, 9 tilt_size, 11 grain, 12 smoothing;
    // the old default)
    const CHANGED: [(i32, ToolKind, usize, f32); 8] = [
        (2, TOOL_PENCIL, 0, 0.30),
        (2, TOOL_PENCIL, 1, 0.85),
        (2, TOOL_PENCIL, 9, 2.6),
        (2, TOOL_PENCIL, 12, 0.25),
        (2, TOOL_PENCIL, 12, 0.12),
        (2, TOOL_MECH_PENCIL, 11, 0.35),
        (2, TOOL_MECH_PENCIL, 12, 0.20),
        // At 1.00 the ballpoint's line trailed the nib by 6 to 13 ms at writing speed, at 0.50 by
        // about 2 (docs/INK-PERFORMANCE.md, "Input smoothing").
        (3, TOOL_BALLPOINT, 12, 1.0),
    ];
    // Version 4 (quick sizes, 2.1): the pens moved onto the pen-width series. Unlike the table above this goes by
    // the PAIR: a tool's widths move to the new default only when BOTH still equal the old default pair; a pair
    // the owner changed in either field stays exactly as it is. (the tool; the old min; the old max)
    const SIZES_V4: [(ToolKind, f32, f32); 7] = [
        (TOOL_PENCIL, 0.12, 0.32),
        (TOOL_CHARCOAL, 0.30, 0.55),
        (TOOL_BALLPOINT, 0.30, 0.48),
        (TOOL_FINELINER, 0.28, 0.28),
        (TOOL_CALLIGRAPHY, 1.75, 1.75),
        (TOOL_HIGHLIGHTER, 5.0, 5.0),
        (TOOL_ERASER, 2.5, 7.0),
    ];
    if s.version >= SETTINGS_VERSION {
        return;
    }
    if s.version < 4 {
        for &(tool, lo, hi) in SIZES_V4.iter() {
            let p = &mut s.tools[tool.0 as usize];
            if (p.size_min_mm - lo).abs() < 0.005 && (p.size_max_mm - hi).abs() < 0.005 {
                let mut d = ToolParams::default();
                tool_params_default(tool, &mut d);
                qlog!(
                    "settings: %s sizes updated %.2f..%.2f -> %.2f..%.2f",
                    tool_name(tool),
                    p.size_min_mm,
                    p.size_max_mm,
                    d.size_min_mm,
                    d.size_max_mm
                );
                p.size_min_mm = d.size_min_mm;
                p.size_max_mm = d.size_max_mm;
            }
        }
    }
    for &(since, tool, off, old_v) in CHANGED.iter() {
        if s.version >= since {
            continue;
        }
        let v = s.tools[tool.0 as usize].get_mut(off);
        let mut d = ToolParams::default();
        tool_params_default(tool, &mut d);
        let nv = d.get(off);
        if (*v - old_v).abs() < 0.005 && (*v - nv).abs() >= 0.005 {
            qlog!(
                "settings: %s default updated %.2f -> %.2f",
                tool_name(tool),
                *v,
                nv
            );
            *v = nv;
        }
    }
    s.version = SETTINGS_VERSION;
}

/// Missing file / keys -> defaults; values clamped to ParamInfo ranges. What the file holds that
/// this build does not understand (a newer version's keys, or values beyond its ranges) is kept
/// and written back by settings_save (docs/FORMAT.md).
pub fn settings_load(s: &mut Settings) -> i32 {
    settings_default(s);
    s.version = 0; // files written before versioning have no settings_version key
    forget_unknown_settings();
    with(|st| st.g_set_dropped = 0);
    let path = cformat!("%s/settings.ini", &g_root());
    let mut has_gestures = false;
    let rc = if ini_parse_file(&path, &mut |k, v| {
        has_gestures |= k == b"gestures";
        if !settings_cb(s, k, v) {
            settings_unknown(k, v);
        }
    }) == 0
    {
        0
    } else {
        1
    };
    let dropped = with(|st| st.g_set_dropped);
    if dropped != 0 {
        qlog!(
            "settings: %d unknown line(s) past %d bytes not kept",
            dropped,
            SETTINGS_UNKNOWN_MAX
        );
    }
    if rc == 0 {
        // An owner upgrading from the build before the gestures: an existing file at version 3 that has
        // no gestures key (a fresh install has no file; a file this build saved has the key). The
        // one-time toasts of what changed (hints bits 0 and 8) are for this owner only.
        // (version 3 literally: the build before the gestures wrote it; later versions' files have the key)
        s.upgraded = s.version == 3 && !has_gestures;
        // What's new is for an update: a file a Grimoire saved (it has the version key) whose notes
        // seen were another version's. A fresh start, or a file written by hand, records this
        // version's as seen. Every update opens them once, whatever whatsnew_hide says: the owner's
        // rule of 2026-11-05, "no don't show this again, just a "Dismiss" and every update with
        // changes by default has at least a basic what's new". whatsnew_hide is still read and
        // written back unchanged (an older version reading the file keeps its meaning), but this
        // version never sets it and never reads it here.
        s.whatsnew_due = s.version > 0 && s.whatsnew_seen.as_str() != GRIMOIRE_VERSION;
        settings_upgrade(s);
    } else {
        s.version = SETTINGS_VERSION;
    }
    match s.status_mode {
        0 => { s.status_on = true; s.status_size = 0; s.status_corner = 0; }
        2 => s.status_on = false,
        _ => {}
    }
    s.status_mode = 1;
    if !s.whatsnew_due {
        s.whatsnew_seen.set(GRIMOIRE_VERSION);
    }
    rc
}

/// longer than any language= this build can use
const LANGUAGE_PEEK: usize = 16;

/// settings.ini's language= as the file has it, last line counting ("" without the file or the
/// key), read on its own: main.c sets the language before the app, and its settings, start.
pub fn settings_language(out: &mut String, cap: usize) {
    let path = cformat!("%s/settings.ini", &g_root());
    let mut v = FixedStr::<LANGUAGE_PEEK>::new();
    ini_parse_file(&path, &mut |k, val| {
        if k == b"language" {
            v.set_bytes(val);
        }
    });
    // (a value that is not UTF-8 reads as no language this build has, as it does in the C)
    strlcpy_q(out, &String::from_utf8_lossy(v.as_bytes()), cap);
}

/// One known key: a value kept from the file goes back as it was while the setting is unchanged.
/// Once the setting is saved with another value it was changed here, and the kept value is
/// forgotten: from then on the setting is saved as set, also when it is set back.
fn put(b: &mut Vec<u8>, k: &str, v: &str) {
    let kept = with(|st| {
        let i = st.g_kept.iter().position(|e| e.key == k)?;
        if st.g_kept[i].used == v {
            Some(st.g_kept[i].raw.clone())
        } else {
            st.g_kept.swap_remove(i);
            None
        }
    });
    b.extend_from_slice(k.as_bytes());
    b.push(b'=');
    match kept {
        Some(raw) => b.extend_from_slice(&raw),
        None => b.extend_from_slice(v.as_bytes()),
    }
    b.push(b'\n');
}

fn put_int(b: &mut Vec<u8>, k: &str, v: i32) {
    put(b, k, &cformat!("%d", v));
}

pub fn settings_save(s: &Settings) -> i32 {
    let path = cformat!("%s/settings.ini", &g_root());
    let mut b: Vec<u8> = Vec::new();
    b.extend_from_slice(b"# Grimoire settings\n");
    put_int(&mut b, "current_tool", s.current_tool.0);
    put_int(&mut b, "draw_tool", s.draw_tool.0);
    put_int(&mut b, "panel_x", s.panel_x);
    put_int(&mut b, "panel_y", s.panel_y);
    put_int(&mut b, "panel_collapsed", s.panel_collapsed as i32);
    put_int(&mut b, "side_button", s.side_button);
    put_int(&mut b, "mirror_pdf", s.mirror_pdf as i32);
    put_int(&mut b, "follow_rotation", s.follow_rotation as i32);
    put_int(&mut b, "invert", s.invert as i32);
    put_int(&mut b, "pinch_zoom", s.pinch_zoom as i32);
    put_int(&mut b, "shortcuts_enabled", s.shortcuts_enabled as i32);
    put_int(&mut b, "controller", s.controller as i32);
    if s.updates != 0 {
        put(
            &mut b,
            "updates",
            if s.updates == 1 { "stable" } else { "preview" },
        );
    }
    put_int(&mut b, "sleep_mode", s.sleep_mode);
    put_int(&mut b, "sleep_minutes", s.sleep_minutes);
    put_int(&mut b, "http_port", s.http_port);
    put_int(&mut b, "passcode_told", s.passcode_told as i32);
    put_int(&mut b, "eraser_mode", s.eraser_mode);
    put_int(&mut b, "white", s.white as i32);
    let mut t = String::new();
    settings_pins_text(s, &mut t, 64);
    put(&mut b, "panel_tools", &t);
    put_int(&mut b, "before_eraser", s.before_eraser);
    put(&mut b, "music_layout", &cformat!("%u", s.music_layout.0));
    put(
        &mut b,
        "theme",
        if s.theme == 1 { "classic" } else { "gothic" },
    );
    // A newer version's settings_version stays as it is: its own upgrades have already run.
    put_int(
        &mut b,
        "settings_version",
        if s.version > SETTINGS_VERSION {
            s.version
        } else {
            SETTINGS_VERSION
        },
    );
    let cs = [&s.pen_calib, &s.touch_calib];
    let cn = ["pen", "touch"];
    let panel = [
        [s.pen_panel_w, s.pen_panel_h],
        [s.touch_panel_w, s.touch_panel_h],
    ];
    for i in 0..2 {
        put_int(&mut b, &cformat!("%s.valid", cn[i]), cs[i].valid as i32);
        put_int(&mut b, &cformat!("%s.swap", cn[i]), cs[i].swap_xy as i32);
        b.extend_from_slice(
            cformat!(
                "%s.ax=%.6f\n%s.bx=%.6f\n%s.ay=%.6f\n%s.by=%.6f\n",
                cn[i],
                cs[i].ax,
                cn[i],
                cs[i].bx,
                cn[i],
                cs[i].ay,
                cn[i],
                cs[i].by
            )
            .as_bytes(),
        );
        if panel[i][0] > 0 {
            // the panel it was made on, once known
            put(
                &mut b,
                &cformat!("%s.panel", cn[i]),
                &cformat!("%dx%d", panel[i][0], panel[i][1]),
            );
        }
    }
    for tk in 0..TOOL_COUNT {
        for pi in tool_param_table() {
            let key = cformat!("%s.%s", K_TOOL_KEYS[tk], pi.key);
            put(&mut b, &key, &cformat!("%.4f", s.tools[tk].get(pi.field)));
        }
    }
    // (A language this build lacks goes back as it was.)
    put(
        &mut b,
        "language",
        if i18n_setting_known(s.language.as_str()) {
            s.language.as_str()
        } else {
            "auto"
        },
    );
    // The newest keys come last of the known ones: a build without them keeps their lines as lines it
    // does not know. The clock and battery corner's (status.rs), then the gestures' (Index phase 1).
    put_int(&mut b, "status_on", s.status_on as i32);
    put_int(&mut b, "status_size", s.status_size);
    put_int(&mut b, "gestures", s.gestures as i32);
    put_int(&mut b, "landscape_fit", s.landscape_fit as i32);
    put_int(&mut b, "hints", s.hints);
    put_int(&mut b, "link_areas", s.link_areas as i32);
    put_int(&mut b, "status_corner", s.status_corner);
    put_int(&mut b, "clock_12h", s.clock_12h as i32);
    put_int(&mut b, "rotation_lock", s.rotation_lock);
    put_int(&mut b, "gesture_new_page", s.gesture_new_page as i32);
    put_int(&mut b, "status_mode", s.status_mode);
    put_int(&mut b, "shapes", s.shapes as i32);
    put_int(&mut b, "shape_hold_ms", s.shape_hold_ms);
    put_int(&mut b, "shape_quads", s.shape_quads);
    put_int(&mut b, "shape_round", s.shape_round);
    put_int(&mut b, "shape_links", s.shape_links as i32);
    put_int(&mut b, "shape_min_mm", s.shape_min_mm);
    put(&mut b, "whatsnew_seen", s.whatsnew_seen.as_str());
    put_int(&mut b, "whatsnew_hide", s.whatsnew_hide as i32);
    put_int(&mut b, "shape_corner", s.shape_corner);
    // 2.1: Classic's font (a value this build does not have goes back as it was, keep_value): a family's id, or
    // file:<path> for a font of the owner's own (2.1.0-beta.2).
    if s.font_file.is_empty() {
        put(&mut b, "font", FONT_FAMILIES[clampi(s.font, 0, FONT_FAMILIES.len() as i32 - 1) as usize].id);
    } else {
        put(&mut b, "font", &format!("file:{}", s.font_file.as_str()));
    }
    // Its Bold file, a line only a file that names one has (or had: a value not used goes back as it was).
    if !s.font_bold.is_empty() {
        put(&mut b, "font_bold", &format!("file:{}", s.font_bold.as_str()));
    } else if with(|st| st.g_kept.iter().any(|e| e.key == "font_bold")) {
        put(&mut b, "font_bold", "");
    }
    // Lines this build does not know, as they were; but a pen.panel or touch.panel this build could
    // not read goes once the calibration's panel is recorded (written above in its place).
    let unknown = with(|st| st.g_set_unknown.clone());
    let mut at = 0;
    while at < unknown.len() {
        let l = &unknown[at..];
        let len = match l.iter().position(|&c| c == b'\n') {
            Some(nl) => nl + 1,
            None => l.len(),
        };
        at += len;
        let replaced = (panel[0][0] > 0 && l.starts_with(b"pen.panel="))
            || (panel[1][0] > 0 && l.starts_with(b"touch.panel="));
        if !replaced {
            b.extend_from_slice(&l[..len]);
        }
    }
    write_file_atomic(&path, &b)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Notebook ids' random part comes from the system: ids made back to back on one thread, in the same
    // millisecond, differ.
    #[test]
    fn ids_made_back_to_back_differ() {
        let ids: Vec<String> = (0..256).map(|_| random_hex(16)).collect();
        let mut sorted = ids.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(sorted.len(), ids.len(), "{ids:?}");
        assert!(ids.iter().all(|h| h.len() == 16 && h.bytes().all(|c| c.is_ascii_hexdigit())));
    }

    // Two notebooks given the same id: the second takes a fresh suffix, never "cannot create".
    #[test]
    fn a_taken_id_is_made_again() {
        let base = std::env::var("GRIMOIRE_TEST_TMP").map(std::path::PathBuf::from).unwrap_or_else(|_| std::env::temp_dir());
        let r = base.join(format!("grimoire-store-taken-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&r);
        let r = r.to_string_lossy().into_owned();
        assert_eq!(store_init(&r), 0);
        store_set_device(300.0, 600, 800);
        assert_eq!(mkdir_p(&format!("{r}/notebooks")), 0);
        let mut collided = false;
        for _ in 0..10 {
            store_force_random_hex(&["aaaa"]);
            let mut taken = String::new();
            store_make_id(&mut taken);
            let _ = std::fs::create_dir(notebook_dir(taken.as_bytes()).unwrap());
            store_force_random_hex(&["aaaa", "bbbb"]);
            let mut nb = NotebookInfo::default();
            assert_eq!(store_create("Second", "", PAPER_BLANK, &mut nb), 0, "a taken id failed the notebook");
            store_force_random_hex(&[]);
            let id = nb.id.as_str();
            if id[..id.len() - 4] == taken[..taken.len() - 4] {
                assert!(id.ends_with("bbbb"), "{id}");
                collided = true;
                break;
            }
            // (the second turned over between the two: no collision; again)
        }
        assert!(collided);
        let _ = std::fs::remove_dir_all(&r);
    }

    #[test]
    fn numbers_as_libc_reads_them() {
        assert_eq!(c_atoi(b"  -42abc"), -42);
        assert_eq!(c_atoi(b"+7"), 7);
        assert_eq!(c_atoi(b"x"), 0);
        assert_eq!(c_atoi(b"4294967297"), 1); // int arithmetic wraps
        assert_eq!(c_strtoul(b" 12,3"), (12, 3));
        assert_eq!(c_strtoul(b",1"), (0, 0));
        assert_eq!(c_strtoul(b"-1").0, libc::c_ulong::MAX as u64);
        assert_eq!(
            c_strtol(b"99999999999999999999999"),
            (libc::c_long::MAX as i64, 23)
        );
        assert_eq!(size_value(b"1986x2648"), (1986, 4));
        assert_eq!(size_value(b"100001"), (0, 6));
        assert_eq!(c_strtof(b"2.5"), 2.5);
        assert_eq!(c_strtof(b" -.5e1xyz"), -5.0);
        assert_eq!(c_strtof(b"1e"), 1.0);
        assert_eq!(c_strtof(b"0x1.8p1"), 3.0);
        assert_eq!(c_strtof(b"abc"), 0.0);
        assert!(c_strtof(b"nan").is_nan());
        assert_eq!(c_strtof(b"-inf"), f32::NEG_INFINITY);
    }

    #[test]
    fn file_names_as_sscanf_reads_them() {
        assert_eq!(scan_page_name(b"p12.qpg"), Some(12));
        assert_eq!(scan_page_name(b"p012.qpg"), Some(12));
        assert_eq!(scan_page_name(b"p12.qpg.damaged-5"), None);
        assert_eq!(scan_page_name(b"p12.qpgx"), None);
        assert_eq!(scan_page_name(b"p12.qp"), None);
        assert_eq!(scan_page_name(b"notebook.ini"), None);
        assert_eq!(
            scan_aside_name(b"p3.qpg.damaged-1792000100"),
            Some((3, 1792000100))
        );
        assert_eq!(scan_aside_name(b"p3.qpg.damaged-"), None);
        assert_eq!(scan_aside_name(b"p3.qpg.damaged-5x"), None);
        assert_eq!(scan_aside_name(b"p3.qpg"), None);
        assert_eq!(scan_p5_header(b"P5\n4 3\n255\nxyz"), Some((4, 3, 255, 10)));
        assert_eq!(scan_p5_header(b"P6\n4 3\n255\n"), None);
        assert_eq!(scan_p5_header(b"P5\n4 3\n"), None);
    }

    #[test]
    fn names_cut_at_a_character_and_stay_one_line() {
        let mut f = FixedStr::<8>::new();
        clean_line(&mut f, "ab\ncdé!".as_bytes()); // 7 bytes fit: "ab cd" + é (2 bytes) = 7
        assert_eq!(f.as_bytes(), "ab cdé".as_bytes());
        clean_line(&mut f, "abcdef\u{e9}".as_bytes()); // the limit falls inside é: it goes
        assert_eq!(f.as_bytes(), b"abcdef");
    }

    // A folder left out of the Graph that lived only through its notebooks' paths (no folders.ini line) goes
    // when its last notebook moves out or is trashed: its graph-omit.ini line goes too, so the same path made
    // again later starts in the Graph. One with a folders.ini line, or still holding a notebook, keeps it.
    #[test]
    fn an_implicit_folder_gone_takes_its_graph_line() {
        let base = std::env::var("GRIMOIRE_TEST_TMP").map(std::path::PathBuf::from).unwrap_or_else(|_| std::env::temp_dir());
        let r = base.join(format!("grimoire-store-omit-prune-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&r);
        assert_eq!(store_init(r.to_str().unwrap()), 0);
        let (mut a, mut b, mut c) = (NotebookInfo::default(), NotebookInfo::default(), NotebookInfo::default());
        assert_eq!(store_create("A", "", PAPER_BLANK, &mut a), 0);
        assert_eq!(store_create("B", "", PAPER_BLANK, &mut b), 0);
        assert_eq!(store_create("C", "", PAPER_BLANK, &mut c), 0);
        assert_eq!(store_set_folder(a.id.as_str(), "School/2025"), 0);
        assert_eq!(store_set_folder(b.id.as_str(), "Work"), 0);
        assert_eq!(store_add_folder("Kept"), 0);
        for f in ["School/2025", "Work", "Kept"] {
            assert_eq!(store_set_folder_graph_omit(f, true), 0);
        }
        // Work still holds B when C moves through it and out again.
        assert_eq!(store_set_folder(c.id.as_str(), "Work"), 0);
        assert_eq!(store_set_folder(c.id.as_str(), ""), 0);
        assert_eq!(store_graph_omit_folders(), vec!["Kept".to_string(), "School/2025".to_string(), "Work".to_string()]);
        // School/2025's only notebook moves to the top level: the folder and its line are gone.
        assert_eq!(store_set_folder(a.id.as_str(), ""), 0);
        assert_eq!(store_graph_omit_folders(), vec!["Kept".to_string(), "Work".to_string()]);
        // Work's only notebook goes to the trash: the same.
        assert_eq!(store_delete(b.id.as_str()), 0);
        assert_eq!(store_graph_omit_folders(), vec!["Kept".to_string()]);
        // Made again later: in the Graph.
        assert_eq!(store_set_folder(c.id.as_str(), "School/2025"), 0);
        assert!(!store_folder_graph_omitted(&store_graph_omit_folders(), "School/2025"));
        let _ = std::fs::remove_dir_all(&r);
    }

    // ---- Merge (2.1, L2): a folder merged into one that is there ----

    fn merge_root(tag: &str) -> std::path::PathBuf {
        let base = std::env::var("GRIMOIRE_TEST_TMP").map(std::path::PathBuf::from).unwrap_or_else(|_| std::env::temp_dir());
        let r = base.join(format!("grimoire-store-merge-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&r);
        assert_eq!(store_init(r.to_str().unwrap()), 0);
        r
    }

    fn make(name: &str, folder: &str) -> NotebookInfo {
        let mut nb = NotebookInfo::default();
        assert_eq!(store_create(name, "", PAPER_BLANK, &mut nb), 0);
        if !folder.is_empty() {
            assert_eq!(store_set_folder(nb.id.as_str(), folder), 0);
        }
        nb
    }

    // (name, folder) of every notebook, sorted
    fn tree() -> Vec<(String, String)> {
        let mut v = Vec::new();
        assert_eq!(store_list(&mut v), 0);
        let mut t: Vec<(String, String)> = v.iter().map(|n| (n.name.as_str().to_string(), n.folder.as_str().to_string())).collect();
        t.sort();
        t
    }

    // Every file the tree holds that a merge writes, as bytes: for "the tree exactly as before".
    fn snapshot(r: &std::path::Path) -> Vec<(String, Vec<u8>)> {
        let mut out = Vec::new();
        let mut stack = vec![r.to_path_buf()];
        while let Some(d) = stack.pop() {
            for e in std::fs::read_dir(&d).unwrap().flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else if p.extension().is_none_or(|x| x != "tmp") {
                    out.push((p.strip_prefix(r).unwrap().to_string_lossy().into_owned(), std::fs::read(&p).unwrap()));
                }
            }
        }
        out.sort();
        out
    }

    fn mirror_paths() -> Vec<String> {
        let mut v = Vec::new();
        assert_eq!(store_list(&mut v), 0);
        v.iter()
            .map(|n| {
                let mut p = String::new();
                crate::export::export_mirror_path(n, "/m", &mut p, 1200);
                p.to_lowercase()
            })
            .collect()
    }

    // Nested folders: every notebook.ini folder= and every folders.ini line under the old prefix takes the
    // new one in one pass; the old folder is gone; what was not under it is as it was; no name changed.
    #[test]
    fn merge_rewrites_nested_prefixes() {
        let r = merge_root("nested");
        for f in ["Keep", "School", "Old", "Old/x", "Old/x/y", "Oldish"] {
            assert_eq!(store_add_folder(f), 0);
        }
        make("a", "School");
        make("b", "Old");
        make("c", "Old/x");
        make("d", "Old/x/y");
        make("e", "Oldish");
        make("f", "");
        assert_eq!(store_merge_folder("Old", "School"), 0);
        assert_eq!(
            tree(),
            [("a", "School"), ("b", "School"), ("c", "School/x"), ("d", "School/x/y"), ("e", "Oldish"), ("f", "")]
                .map(|(n, f)| (n.to_string(), f.to_string()))
        );
        let mut all = Vec::new();
        assert_eq!(store_list_folders(&mut all), 0);
        assert_eq!(all, ["Keep", "Oldish", "School", "School/x", "School/x/y"]);
        let ini = std::fs::read_to_string(r.join("folders.ini")).unwrap();
        assert!(!ini.lines().any(|l| l == "Old" || l.starts_with("Old/")), "{ini}");
        // Two folders of the same name below: they meet too (School/x merged by a second pass)
        assert_eq!(store_add_folder("Other"), 0);
        assert_eq!(store_add_folder("Other/x"), 0);
        make("g", "Other/x");
        assert_eq!(store_merge_folder("Other", "School"), 0);
        assert!(tree().contains(&("g".to_string(), "School/x".to_string())));
        let _ = std::fs::remove_dir_all(&r);
    }

    // Names that clash get " 2" (" 3" when " 2" is taken), in name order, case ignored; every mirror PDF path
    // stays unique; a notebook that does not clash keeps its name; nothing is ever dropped.
    #[test]
    fn merge_names_that_clash_get_a_number() {
        let r = merge_root("names");
        assert_eq!(store_add_folder("A"), 0);
        assert_eq!(store_add_folder("B"), 0);
        assert_eq!(store_add_folder("B/sub"), 0);
        make("Notes", "A");
        make("Notes 2", "A");
        make("Plan", "A");
        make("Notes", "B");
        make("notes", "B");
        make("Own", "B");
        make("Notes", "B/sub"); // (another folder: it clashes with the same names once the prefix is A/sub)
        make("Notes", "A/sub");
        assert_eq!(store_add_folder("A/sub"), 0);
        assert_eq!(store_merge_folder("B", "A"), 0);
        let t = tree();
        assert_eq!(t.len(), 8, "{t:?}");
        let in_a: Vec<&str> = t.iter().filter(|(_, f)| f == "A").map(|(n, _)| n.as_str()).collect();
        assert_eq!(in_a.len(), 6);
        for n in ["Notes", "Notes 2", "Plan", "Own"] {
            assert!(in_a.contains(&n), "{n} in {in_a:?}");
        }
        // the two that came are "Notes 3" and "notes 4" (the order: Notes then notes by their bytes)
        assert!(in_a.contains(&"Notes 3") && in_a.contains(&"notes 4"), "{in_a:?}");
        let sub: Vec<&str> = t.iter().filter(|(_, f)| f == "A/sub").map(|(n, _)| n.as_str()).collect();
        assert_eq!(sub.len(), 2);
        assert!(sub.contains(&"Notes") && sub.contains(&"Notes 2"), "{sub:?}");
        let mut m = mirror_paths();
        m.sort();
        let n = m.len();
        m.dedup();
        assert_eq!(m.len(), n, "a mirror path is shared: {m:?}");
        let _ = std::fs::remove_dir_all(&r);
    }

    // Case-only differences: folders that differ only in case are two folders (the dialog asks only for an
    // exact name); merging "school" into "School" makes "Math" and "math" clash (their mirror files would be
    // one on a storage that ignores case), and Keep both's number avoids a folder of any case.
    #[test]
    fn merge_case_only_differences() {
        let r = merge_root("case");
        assert_eq!(store_add_folder("school"), 0);
        assert_eq!(store_add_folder("School"), 0);
        assert!(store_folder_exists("school") && store_folder_exists("School") && !store_folder_exists("SCHOOL"));
        make("Math", "School");
        make("math", "school");
        assert_eq!(store_merge_folder("school", "School"), 0);
        let t = tree();
        assert_eq!(t, [("Math", "School"), ("math 2", "School")].map(|(n, f)| (n.to_string(), f.to_string())));
        assert_eq!(store_add_folder("Work"), 0);
        assert_eq!(store_add_folder("work 2"), 0);
        assert_eq!(store_folder_unique("Work").as_deref(), Some("Work 3"));
        assert_eq!(store_folder_unique("Free").as_deref(), Some("Free 2"));
        // A case-only rename with no folder of the new spelling is a plain rename
        assert_eq!(store_rename_folder("Work", "WORK"), 0);
        assert!(store_folder_exists("WORK") && !store_folder_exists("Work"));
        let _ = std::fs::remove_dir_all(&r);
    }

    // A folder merged into itself or into its own child (or a folder not there) is refused, and nothing changes.
    #[test]
    fn merge_refusals_change_nothing() {
        let r = merge_root("refuse");
        for f in ["P", "P/c", "Q"] {
            assert_eq!(store_add_folder(f), 0);
        }
        make("n", "P/c");
        let before = snapshot(&r);
        assert_eq!(store_merge_folder("P", "P"), -1);
        assert_eq!(store_merge_folder("P", "P/c"), -1);
        assert_eq!(store_merge_folder("P", "Nope"), -1);
        assert_eq!(store_merge_folder("Nope", "P"), -1);
        assert_eq!(store_merge_folder("", "P"), -1);
        assert_eq!(snapshot(&r), before);
        // a parent is fine: P/c's notebooks go up into P
        assert_eq!(store_merge_folder("P/c", "P"), 0);
        assert_eq!(tree(), vec![("n".to_string(), "P".to_string())]);
        let _ = std::fs::remove_dir_all(&r);
    }

    // A write that fails at any step (each notebook.ini, then folders.ini) puts the tree back exactly as it was:
    // every file, byte for byte; and the same merge made after it works.
    #[test]
    fn merge_rollback_at_every_step() {
        let r = merge_root("rollback");
        for f in ["A", "B", "B/x"] {
            assert_eq!(store_add_folder(f), 0);
        }
        make("Notes", "A");
        make("Notes", "B");
        make("Two", "B");
        make("Deep", "B/x");
        assert_eq!(store_set_folder_graph_omit("B", true), 0);
        let before = snapshot(&r);
        let steps = 3 + 1; // the three notebooks of B (one under B/x), then folders.ini
        for k in 1..=steps as u32 {
            crate::util::FAIL_WRITE_AT.with(|c| c.set(k));
            assert_eq!(store_merge_folder("B", "A"), -1, "step {k}");
            crate::util::FAIL_WRITE_AT.with(|c| c.set(0));
            assert_eq!(snapshot(&r), before, "the tree after a failure at step {k}");
            assert_eq!(store_graph_omit_folders(), vec!["B".to_string()], "step {k}");
        }
        assert_eq!(store_merge_folder("B", "A"), 0);
        let t = tree();
        assert_eq!(t.len(), 4);
        assert!(t.contains(&("Notes 2".to_string(), "A".to_string())), "{t:?}");
        assert_eq!(store_graph_omit_folders(), vec!["A".to_string()]);
        let _ = std::fs::remove_dir_all(&r);
    }

    #[test]
    fn folder_order_folds_accents() {
        let mut v = vec!["Zoo", "Álgebra", "arte", "Études", "e\u{301}poca"];
        v.sort_by(|a, b| cmp_name(a.as_bytes(), b.as_bytes()));
        assert_eq!(v, ["Álgebra", "arte", "e\u{301}poca", "Études", "Zoo"]);
    }

    // One order for names people read: accents folded as the Library's folders fold them, numbers by their
    // value as an import folder's pictures are; equal so, the bytes decide; and it is a total order.
    #[test]
    fn names_fold_accents_and_count_numbers() {
        assert_eq!(name_cmp("Álgebra 2", "Álgebra 10"), Ordering::Less);
        assert_eq!(name_cmp("algebra 10", "Álgebra 2"), Ordering::Greater);
        assert_eq!(name_cmp("Straße 2", "Strasse 10"), Ordering::Less); // ß is ss
        assert_eq!(name_cmp("page-002", "page-3"), Ordering::Less);
        assert_eq!(name_cmp("a01", "a1"), Ordering::Less); // the same number: the bytes decide
        assert_eq!(name_cmp("a1", "a01"), Ordering::Greater);
        assert_eq!(name_cmp("x", "x1"), Ordering::Less);
        assert_eq!(name_cmp("same", "same"), Ordering::Equal);
        assert_eq!(name_cmp("", ""), Ordering::Equal);
        assert_eq!(name_cmp("", "a"), Ordering::Less);
        assert_eq!(name_cmp("B", "b"), Ordering::Less); // equal but for case: 'B' is before 'b'
        assert_eq!(name_cmp("e\u{301}2", "e10"), Ordering::Less); // a combining accent before a number
        assert_eq!(name_cmp("99999999999999999999", "100000000000000000000"), Ordering::Less); // past u64
        let mut v = vec![
            "Zoo", "Álgebra 10", "page 10", "arte", "Álgebra 2", "Page 9", "Études", "e\u{301}poca", "Arbol", "Árbol",
            "日記", "1 Intro", "10 End", "2 Middle", "02 Middle", "Straße 2", "Strasse 10",
        ];
        v.sort_by(|a, b| name_cmp(a, b));
        assert_eq!(
            v,
            [
                "1 Intro", "02 Middle", "2 Middle", "10 End", "Álgebra 2", "Álgebra 10", "Arbol", "Árbol", "arte",
                "e\u{301}poca", "Études", "Page 9", "page 10", "Straße 2", "Strasse 10", "Zoo", "日記",
            ]
        );
        // a total order: each pair one way, and the same order whatever the input's
        for a in &v {
            for b in &v {
                assert_eq!(name_cmp(a, b), name_cmp(b, a).reverse(), "{a} {b}");
                assert_eq!(name_cmp(a, b) == Ordering::Equal, a == b, "{a} {b}");
            }
        }
        let mut r = v.clone();
        r.reverse();
        r.sort_by(|a, b| name_cmp(a, b));
        assert_eq!(r, v);
    }

    #[test]
    fn printf_with_raw_bytes() {
        let mut b = Vec::new();
        sb_printf(
            &mut b,
            "name=%s\nn=%d %s|",
            &[raw(b"a\xffb"), val(5), raw(b"")],
        );
        assert_eq!(b, b"name=a\xffb\nn=5 |");
    }
}

/// Visible-page preview jobs render outside the UI process, with bounded work and stale-write checks.
/// The missing previews are made cheapest first: pages with a current kept drawing (read back, not
/// drawn), then the others by the size of their files, so a dense page never holds back the rest. Each
/// preview tells an open view as it lands (page_previews_changed), which shows that tile at once. A dense
/// page drawn here (store_page_dense: a file of 512 KB or more) keeps its drawing (store_save_page_render_at),
/// so the page also opens quickly in the editor, and its next preview is read back; no other page keeps one,
/// however slow it was to draw (2.0.0-beta.2, the owner's ruling: ordinary pages save nothing extra), not even
/// a slow one under 512 KB that says "Loading page..." as it opens (store_page_slow: 256 KB or more, the
/// owner's decision of 2026-11-04). One drawn with its picture paper white (the picture missing or
/// unreadable) keeps neither. One log line per batch says where the time went.
pub fn store_build_page_previews(id:&str, range:&str,dpi:f32)->i32 {
    let started=now_ms();
    let mut nb=NotebookInfo::default();
    if store_load_info(id,&mut nb)!=0{return -1;}
    // "first:last", or "first:last:pid" to leave page pid out: the page open in the editor, whose preview
    // is made from its canvas there (it is drawn already; drawing it again here, at every pause, kept a
    // core busy for 20 s and more on a dense page, 2.0.0-rc.10).
    let mut parts=range.split(':');
    let (Some(Ok(first)),Some(Ok(last)))=(parts.next().map(str::parse::<usize>),parts.next().map(str::parse::<usize>)) else{return -1};
    let skip=parts.next().and_then(|p|p.parse::<u32>().ok());
    if last.saturating_sub(first)>64{return -1;}
    // (pages, cached, kept drawings read back, drawn, drawings kept here)
    let (mut pages,mut cached,mut kept,mut drawn,mut keeps)=(0,0,0,0,0);
    let mut todo:Vec<(u64,usize,u32)>=Vec::new();
    for i in first..last.min(nb.page_ids.len()) {
        let pid=nb.page_ids[i];
        if Some(pid)==skip {continue;}
        pages+=1;
        // (a stamp and a length, not a decode: the view decodes what it shows itself)
        if store_page_preview_ok(&nb,pid) {cached+=1;continue;}
        let Some((source,_,_))=page_preview_files(&nb,pid) else{continue};
        let cost=if page_render_current(&nb,pid) {0} else {std::fs::metadata(&source).map_or(0,|m|m.len().max(1))};
        if todo.try_reserve(1).is_err() {break;}
        todo.push((cost,i,pid));
    }
    todo.sort_by_key(|t|t.0); // (stable: pages of one cost in their order)
    // (-1: none made)
    let (mut first_ms,mut slow)=(-1i32,(0u32,-1i32));
    for &(_,i,pid) in &todo {
        let at=now_ms();
        let Some((source,_,_))=page_preview_files(&nb,pid) else{continue};
        // (a listed page with no file reads as a blank page: no preview is made of it)
        let Some(before)=preview_stamp(&source) else{continue};
        let mut page=None;
        if store_read_page(&nb,i as i32,&mut page)!=0 {continue;}
        let Some(page)=page else{continue};
        if page.readonly {continue;}
        let mut full=Canvas::default();
        if canvas_init(&mut full,page.w,page.h)!=0 {continue;}
        let layered=page_layered(&page);
        // (whether a picture paper drew white, its picture missing or unreadable here)
        let mut missing=false;
        if store_load_page_render(&nb,pid,page.w,page.h,&mut full,layered) {
            kept+=1;
        } else {
            bgimage_take_missing(); // (what drew white before)
            page_render(&mut full,&page,Rect::new(0,0,page.w,page.h),dpi);
            missing=bgimage_take_missing();
            #[cfg(feature = "sim")]
            sim_slow_draw(&source);
            drawn+=1;
            // (none over a drawing 2.0.0 or 2.0.1 kept: the editor, drawing the page as it opens, takes that one
            // away and makes the Library card made beside it again, store_retire_page_render)
            if !missing && store_page_dense(&nb,pid) && !page_render_before(&nb,pid) && store_save_page_render_at(&nb,pid,&full,layered,&before) {keeps+=1;}
        }
        drop(page);
        // A page drawn with its picture white writes neither its kept drawing nor its preview, as the editor
        // writes neither then (its render_page_full and save_page): a kept drawing would open the page white
        // with nothing said, and both would stay so once the picture reads again. Its tile shows its number,
        // and the next view that opens asks for it again.
        if !missing && preview_stamp(&source).as_deref()==Some(before.as_str()) && save_page_preview(&nb,pid,&full) {
            page_previews_changed(id); // (an open view shows this tile now)
            if first_ms<0 {first_ms=now_ms().wrapping_sub(started) as i32;}
        }
        let ms=now_ms().wrapping_sub(at) as i32;
        if ms>slow.1 {slow=(pid,ms);}
    }
    // (and the cores the processor has: with three or more a second job process may run beside this one)
    let cores=std::thread::available_parallelism().map_or(1,|n|n.get()) as i32;
    qlog!("previews: nb=%s pages=%d cached=%d kept=%d drawn=%d keep=%d ms=%d first=%d slow=p%u:%d cores=%d",
        id,pages,cached,kept,drawn,keeps,now_ms().wrapping_sub(started) as i32,first_ms,slow.0,slow.1,cores);
    0
}
// Tests (the simulator): GRIMOIRE_SIM_DENSE_DRAW_MS=N makes the previews job's drawing of a dense page
// (store_page_dense's size, 512 KB, the job's own rule: not store_page_slow's) take N ms longer, as a Kindle
// takes seconds to draw one: a scenario then sees the other tiles come first, and the dense one's loading
// mark meanwhile.
#[cfg(feature = "sim")]
fn sim_slow_draw(source:&str) {
    let ms=std::env::var("GRIMOIRE_SIM_DENSE_DRAW_MS").ok().and_then(|v|v.parse::<i32>().ok()).unwrap_or(0);
    if ms>0 && page_file_at_least(source,STORE_DENSE_PAGE_BYTES) {sleep_ms(ms);}
}

pub fn store_page_previews_epoch(id:&str)->Option<(u64,i64,i64)> {
    let dir=notebook_dir(id.as_bytes())?;
    let stamp=std::fs::metadata(format!("{dir}/.page-previews/generation")).ok()?;
    Some((stamp.ino(),stamp.mtime(),stamp.mtime_nsec()))
}
pub fn store_page_previews_generation(id:&str)->u64 {
    let Some(dir)=notebook_dir(id.as_bytes())else{return 0};
    if let Some(token)=preview_tag(&format!("{dir}/.page-previews/updated")) {
        return bgimage_content_hash(token.as_bytes());
    }
    std::fs::metadata(format!("{dir}/.page-previews")).map(|s|(s.mtime()as u64).wrapping_mul(1_000_000_000).wrapping_add(s.mtime_nsec() as u64)).unwrap_or(0)
}
pub fn store_invalidate_page_previews(id:&str) {
    let Some(dir)=notebook_dir(id.as_bytes())else{return};
    let path=format!("{dir}/.page-previews");
    if std::fs::create_dir_all(&path).is_ok() && write_file_atomic(&format!("{path}/generation"),b"changed")==0 {
        page_previews_changed(id);
    }
}
pub fn store_cleanup_preview_work(id:&str,pid:i32) {
    let Some(dir)=notebook_dir(id.as_bytes())else{return};let path=format!("{dir}/.page-previews");
    let Ok(entries)=std::fs::read_dir(path)else{return};
    for e in entries.flatten(){let name=e.file_name();let name=name.to_string_lossy();if name.ends_with(&format!(".{pid}"))||name.ends_with(&format!(".{pid}.tmp")){let _=std::fs::remove_file(e.path());}}
}
