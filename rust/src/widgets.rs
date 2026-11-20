//! widgets.c: the editor's widget windows and their layout.
//!
//! Every widget comes from an installed package (widgetpkg.rs). A widget is a small window on the
//! editor canvas: it can be raised, dragged by its title bar, minimized, resized from its
//! bottom-right grip, and closed. Its place and size are kept in widgets.ini at the store root, so
//! they come back on the next launch.
//!
//! The module is geometry and state only. editor.rs draws the frames and routes input.

use crate::prelude::*;

/// The most widgets that can be open at once.
pub const WIDGETS_MAX: usize = 8;
/// The square a minimized widget takes, at 300 dpi.
const WIDGET_MINIMIZED: f32 = 112.0;
/// How far from the canvas edge a new widget starts.
const WIDGET_MARGIN: f32 = 28.0;
/// How far each new widget is offset from the one before it.
const WIDGET_CASCADE: f32 = 22.0;
/// The smallest frame the editor will make, whatever a widget asks for, at 300 dpi.
const WIDGET_FLOOR_W: f32 = 120.0;
const WIDGET_FLOOR_H: f32 = 80.0;
/// A frame never takes more than this share of the canvas either way.
const WIDGET_MAX_SHARE: i32 = 70;

/// What a package widget draws in its body: a native implementation Grimoire carries, or package
/// metadata where this build has no WASM host yet.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WidgetKind {
    /// an installed package: its body is the package's own runtime, which this build has no host for yet
    #[default]
    Package,
    /// a package with `spec.runtime: native`, `spec.entry: calculator`
    Calculator,
}

/// One entry in the widget catalog. Sizes are at 300 dpi, as the rest of the UI's constants.
#[derive(Clone, Copy, Debug)]
pub struct WidgetDef {
    pub id: &'static str,
    pub name: &'static str,
    pub icon: IconKind,
    pub kind: WidgetKind,
    pub default_w: f32,
    pub default_h: f32,
    pub min_w: f32,
    pub min_h: f32,
    pub max_w: f32,
    pub max_h: f32,
    pub resizable: bool,
    /// the installed package's version, author and description
    pub version: &'static str,
    pub author: &'static str,
    pub description: &'static str,
}

/// The catalog starts empty: every widget is an installed package (widgetpkg.rs).
pub const WIDGET_CATALOG: [WidgetDef; 0] = [];

/// A native package widget's keypad, top to bottom and left to right.
pub const CALCULATOR_KEYS: [&str; 20] = [
    "AC", "+/-", "%", "/",
    "7", "8", "9", "x",
    "4", "5", "6", "-",
    "1", "2", "3", "+",
    "0", ".", "<", "=",
];
const CALCULATOR_ROWS: i32 = 5;
const CALCULATOR_COLS: i32 = 4;

/// The icon a manifest's `spec.icon` names, as one of Grimoire's own (ui.rs): a package chooses
/// among the UI's icons, never a picture of its own, so every row and title bar draws the same
/// glyph the rest of the app does.
pub fn widget_icon(name: &str) -> IconKind {
    let name = name.trim().to_ascii_lowercase();
    let name = name.strip_prefix("md-").unwrap_or(&name);
    match name {
        "calculator" => IconKind::Calculator,
        "package" => IconKind::Package,
        "clock" => IconKind::Clock,
        "timer" | "hourglass" => IconKind::Hourglass,
        "music" => IconKind::Music,
        "notebook" => IconKind::Notebook,
        "pdf" => IconKind::Pdf,
        "image" | "photo" => IconKind::Image,
        "files" | "folder" => IconKind::Files,
        "search" => IconKind::Search,
        "download" => IconKind::Download,
        "language" => IconKind::Language,
        "gear" | "settings" => IconKind::Gear,
        "update" => IconKind::Update,
        "paper" | "page" => IconKind::Paper,
        "library" => IconKind::Library,
        "link" => IconKind::Link,
        "live" => IconKind::Live,
        "gamepad" => IconKind::Gamepad,
        "eye" => IconKind::Eye,
        _ => IconKind::Grid,
    }
}

/// A native calculator's state. It is `Copy`, as a frame is: a frame's calculator comes back with
/// its place and size after a restart, and pressing keys changes no layout.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Calculator {
    buf: [u8; 24],
    len: u8,
    acc: f64,
    /// 0 none, 1 +, 2 −, 3 ×, 4 ÷
    op: u8,
    /// the buffer is the number being typed (a digit replaces it when false)
    typing: bool,
    /// the buffer is an answer: a digit starts a new sum
    evaluated: bool,
    error: bool,
}

impl Default for Calculator {
    fn default() -> Self {
        let mut c = Calculator { buf: [0; 24], len: 1, acc: 0.0, op: 0, typing: false, evaluated: false, error: false };
        c.buf[0] = b'0';
        c
    }
}

impl Calculator {
    pub fn display(&self) -> &str {
        std::str::from_utf8(&self.buf[..self.len as usize]).unwrap_or("0")
    }

    fn set_text(&mut self, s: &str) {
        self.len = s.len().min(self.buf.len()) as u8;
        self.buf[..self.len as usize].copy_from_slice(&s.as_bytes()[..self.len as usize]);
    }

    fn text(&self) -> String {
        self.display().to_string()
    }

    fn current(&self) -> f64 {
        self.text().parse::<f64>().unwrap_or(0.0)
    }

    fn number(v: f64) -> String {
        if !v.is_finite() {
            return "Error".into();
        }
        if v.abs() >= 1_000_000_000_000.0 {
            return format!("{v:.6e}");
        }
        if v.fract() == 0.0 {
            return format!("{}", v as i64);
        }
        let mut s = format!("{v:.8}");
        while s.ends_with('0') {
            s.pop();
        }
        if s.ends_with('.') {
            s.pop();
        }
        s
    }

    fn show(&mut self, v: f64) {
        let s = Self::number(v);
        self.error = s == "Error";
        self.set_text(&s);
    }

    fn reset(&mut self) {
        *self = Calculator::default();
    }

    fn digit(&mut self, d: char) {
        if self.error || self.evaluated {
            self.reset();
        }
        if !self.typing {
            self.len = 0;
            self.typing = true;
        }
        if self.len == 1 && self.buf[0] == b'0' {
            self.len = 0;
        }
        if (self.len as usize) < 18 {
            self.buf[self.len as usize] = d as u8;
            self.len += 1;
        }
        if self.len == 0 {
            self.set_text("0");
        }
    }

    fn dot(&mut self) {
        if self.error || self.evaluated {
            self.reset();
        }
        if !self.typing {
            self.set_text("0");
            self.typing = true;
        }
        if !self.text().contains('.') && (self.len as usize) < 18 {
            self.buf[self.len as usize] = b'.';
            self.len += 1;
        }
    }

    fn backspace(&mut self) {
        if self.error || self.evaluated || !self.typing {
            return;
        }
        self.len = self.len.saturating_sub(1);
        if self.len == 0 || (self.len == 1 && self.buf[0] == b'-') {
            self.set_text("0");
            self.typing = false;
        }
    }

    fn sign(&mut self) {
        if self.error {
            return;
        }
        let current = self.text();
        if current == "0" {
            return;
        }
        if current.starts_with('-') {
            self.set_text(&current[1..]);
        } else {
            self.set_text(&format!("-{current}"));
        }
        self.typing = true;
        self.evaluated = false;
    }

    fn percent(&mut self) {
        if self.error {
            return;
        }
        let v = self.current() / 100.0;
        self.show(v);
        self.typing = true;
        self.evaluated = false;
    }

    fn apply(acc: f64, rhs: f64, op: u8) -> f64 {
        match op {
            1 => acc + rhs,
            2 => acc - rhs,
            3 => acc * rhs,
            4 if rhs != 0.0 => acc / rhs,
            4 => f64::NAN,
            _ => rhs,
        }
    }

    fn operator(&mut self, op: u8) {
        if self.error {
            self.reset();
            return;
        }
        if self.op != 0 && self.typing {
            self.acc = Self::apply(self.acc, self.current(), self.op);
        } else if self.op == 0 {
            self.acc = self.current();
        }
        self.op = op;
        self.typing = false;
        self.evaluated = false;
        let acc = self.acc;
        self.show(acc);
    }

    fn equals(&mut self) {
        if self.error || self.op == 0 {
            return;
        }
        let rhs = self.current();
        let v = Self::apply(self.acc, rhs, self.op);
        self.acc = v;
        self.op = 0;
        self.typing = false;
        self.evaluated = true;
        self.show(v);
    }

    /// One key of `CALCULATOR_KEYS`. A key outside the keypad does nothing.
    pub fn press(&mut self, key: usize) {
        match key {
            0 => self.reset(),
            1 => self.sign(),
            2 => self.percent(),
            3 => self.operator(4),
            4 => self.digit('7'),
            5 => self.digit('8'),
            6 => self.digit('9'),
            7 => self.operator(3),
            8 => self.digit('4'),
            9 => self.digit('5'),
            10 => self.digit('6'),
            11 => self.operator(2),
            12 => self.digit('1'),
            13 => self.digit('2'),
            14 => self.digit('3'),
            15 => self.operator(1),
            16 => self.digit('0'),
            17 => self.dot(),
            18 => self.backspace(),
            19 => self.equals(),
            _ => {}
        }
    }
}

impl WidgetFrame {
    /// The display and keypad's places inside a frame's body, as the editor draws them.
    pub fn calculator_display(&self, body: Rect) -> Rect {
        Rect { x: body.x, y: body.y, w: body.w, h: ui_px(78.0).min(body.h / 4) }
    }

    pub fn calculator_key(&self, body: Rect, key: usize) -> Rect {
        let display = self.calculator_display(body);
        let gap = ui_px(4.0);
        let top = display.y + display.h + ui_px(10.0);
        // Keep the bottom-right resize grip off the last key.
        let grid_h = (body.y + body.h - ui_px(54.0) - top).max(0);
        let kw = (body.w - gap * (CALCULATOR_COLS - 1)).max(0) / CALCULATOR_COLS;
        let kh = (grid_h - gap * (CALCULATOR_ROWS - 1)).max(0) / CALCULATOR_ROWS;
        if kw <= 0 || kh <= 0 {
            return RECT_EMPTY;
        }
        let col = key as i32 % CALCULATOR_COLS;
        let row = key as i32 / CALCULATOR_COLS;
        Rect { x: body.x + col * (kw + gap), y: top + row * (kh + gap), w: kw, h: kh }
    }

    /// Which calculator key is under a point in the frame's body.
    pub fn calculator_key_at(&self, body: Rect, x: i32, y: i32) -> Option<usize> {
        (0..CALCULATOR_KEYS.len()).find(|&k| rect_contains(self.calculator_key(body, k), x, y))
    }
}

/// The installed packages in name order. The package entries are cached by widgetpkg.rs and
/// refreshed after an install, update or remove.
pub fn widget_catalog() -> Vec<&'static WidgetDef> {
    let mut v: Vec<&'static WidgetDef> = WIDGET_CATALOG.iter().collect();
    v.extend(crate::widgetpkg::widget_package_defs());
    v
}

pub fn widget_def(id: &str) -> Option<&'static WidgetDef> {
    WIDGET_CATALOG.iter().find(|d| d.id == id).or_else(|| crate::widgetpkg::widget_package_defs().into_iter().find(|d| d.id == id))
}

/// A widget window's place on the canvas. x, y, w and h are in screen pixels; w and h are the
/// restored size while minimized too, so a restore comes back the size it was.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WidgetLayout {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub minimized: bool,
    pub z: i32,
}

/// An open widget window.
#[derive(Clone, Copy, Debug)]
pub struct WidgetFrame {
    pub def: &'static WidgetDef,
    pub layout: WidgetLayout,
    /// a native calculator package's state (unused by other widgets)
    pub calc: Calculator,
}

/// Where a contact came down in a frame. The editor routes input by these.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WidgetPart {
    #[default]
    None,
    Title,
    Close,
    Minimize,
    Resize,
    Body,
}

/// The state of one contact on a widget: which frame and part, where it came down, and the frame's
/// place then.
#[derive(Clone, Copy, Debug)]
pub struct WidgetDrag {
    pub index: usize,
    pub part: WidgetPart,
    pub down_x: i32,
    pub down_y: i32,
    pub start: Rect,
    pub moved: bool,
}

impl WidgetFrame {
    pub fn new(def: &'static WidgetDef) -> Self {
        WidgetFrame {
            def,
            layout: WidgetLayout {
                x: 0,
                y: 0,
                w: ui_px(def.default_w),
                h: ui_px(def.default_h),
                minimized: false,
                z: 0,
            },
            calc: Calculator::default(),
        }
    }

    /// The frame as drawn and hit: a minimized widget is a small square at its corner.
    pub fn rect(&self) -> Rect {
        if self.layout.minimized {
            let s = ui_px(WIDGET_MINIMIZED).min(self.layout.w).min(self.layout.h).max(ui_px(64.0));
            Rect { x: self.layout.x, y: self.layout.y, w: s, h: s }
        } else {
            Rect { x: self.layout.x, y: self.layout.y, w: self.layout.w, h: self.layout.h }
        }
    }

    /// The sizes this frame may take on `area`: the widget's own bounds, never past the screen's
    /// floor and share.
    pub fn limits(&self, area: Rect) -> (i32, i32, i32, i32) {
        let floor_w = ui_px(WIDGET_FLOOR_W);
        let floor_h = ui_px(WIDGET_FLOOR_H);
        let share_w = (area.w * WIDGET_MAX_SHARE / 100).max(floor_w);
        let share_h = (area.h * WIDGET_MAX_SHARE / 100).max(floor_h);
        let min_w = ui_px(self.def.min_w).clamp(floor_w, share_w);
        let min_h = ui_px(self.def.min_h).clamp(floor_h, share_h);
        let max_w = ui_px(self.def.max_w).clamp(min_w, share_w);
        let max_h = ui_px(self.def.max_h).clamp(min_h, share_h);
        (min_w, min_h, max_w, max_h)
    }

    /// Keeps the frame's size in its bounds and its whole rectangle on `area`.
    pub fn clamp(&mut self, area: Rect) -> bool {
        let before = self.layout;
        let (min_w, min_h, max_w, max_h) = self.limits(area);
        self.layout.w = self.layout.w.clamp(min_w, max_w);
        self.layout.h = self.layout.h.clamp(min_h, max_h);
        let r = self.rect();
        self.layout.x = clampi(self.layout.x, area.x, area.x + area.w - r.w);
        self.layout.y = clampi(self.layout.y, area.y, area.y + area.h - r.h);
        self.layout != before
    }

    pub fn move_to(&mut self, x: i32, y: i32, area: Rect) -> bool {
        let r = self.rect();
        let nx = clampi(x, area.x, area.x + area.w - r.w);
        let ny = clampi(y, area.y, area.y + area.h - r.h);
        let changed = nx != self.layout.x || ny != self.layout.y;
        self.layout.x = nx;
        self.layout.y = ny;
        changed
    }

    pub fn resize_to(&mut self, w: i32, h: i32, area: Rect) -> bool {
        let before = self.layout;
        let (min_w, min_h, max_w, max_h) = self.limits(area);
        self.layout.w = w.clamp(min_w, max_w);
        self.layout.h = h.clamp(min_h, max_h);
        let r = self.rect();
        self.layout.x = clampi(self.layout.x, area.x, area.x + area.w - r.w);
        self.layout.y = clampi(self.layout.y, area.y, area.y + area.h - r.h);
        self.layout != before
    }

    pub fn title_rect(&self) -> Rect {
        let r = self.rect();
        let h = (ui_px(64.0)).min(r.h);
        Rect { x: r.x, y: r.y, w: r.w, h }
    }

    pub fn close_rect(&self) -> Rect {
        let r = self.rect();
        let side = ui_px(64.0).min(r.h).min(r.w / 3);
        Rect { x: r.x + r.w - side, y: r.y, w: side, h: side }
    }

    pub fn minimize_rect(&self) -> Rect {
        let r = self.close_rect();
        Rect { x: r.x - r.w, y: r.y, w: r.w, h: r.h }
    }

    pub fn resize_rect(&self) -> Rect {
        let r = self.rect();
        let side = ui_px(72.0).min(r.w / 3).min(r.h / 3);
        Rect { x: r.x + r.w - side, y: r.y + r.h - side, w: side, h: side }
    }
}

/// The editor's open widgets. Draw order is z order; the topmost wins a hit.
#[derive(Clone, Debug)]
pub struct WidgetHost {
    pub frames: Vec<WidgetFrame>,
    next_z: i32,
    /// Whether widgets.ini has been read (the editor reads it when it first opens).
    pub loaded: bool,
}

impl Default for WidgetHost {
    fn default() -> Self {
        WidgetHost::new()
    }
}

impl WidgetHost {
    pub fn new() -> Self {
        WidgetHost { frames: Vec::new(), next_z: 0, loaded: false }
    }

    pub fn is_open(&self, id: &str) -> bool {
        self.frames.iter().any(|f| f.def.id == id)
    }

    pub fn index_of(&self, id: &str) -> Option<usize> {
        self.frames.iter().position(|f| f.def.id == id)
    }

    /// Raises a frame to the top, returning whether its z changed.
    pub fn raise(&mut self, index: usize) -> bool {
        let Some(f) = self.frames.get_mut(index) else { return false };
        let z = self.next_z.max(0);
        if f.layout.z == z {
            return false;
        }
        f.layout.z = z;
        self.next_z = z + 1;
        true
    }

    /// Opens a catalog widget, or raises it when it is already open.
    pub fn spawn(&mut self, def: &'static WidgetDef, area: Rect, panel: Rect) -> bool {
        if let Some(i) = self.index_of(def.id) {
            self.raise(i);
            return false;
        }
        if self.frames.len() >= WIDGETS_MAX {
            return false;
        }
        let mut f = WidgetFrame::new(def);
        let (_, _, max_w, max_h) = f.limits(area);
        f.layout.w = f.layout.w.clamp(ui_px(WIDGET_FLOOR_W), max_w);
        f.layout.h = f.layout.h.clamp(ui_px(WIDGET_FLOOR_H), max_h);
        // Top right of the canvas, left of the panel when the panel is there: the first thing a
        // new widget shows should not be hidden under the toolbar.
        let mut x = area.x + area.w - f.layout.w - ui_px(WIDGET_MARGIN);
        if panel.x > area.x + area.w / 2 {
            x = panel.x - f.layout.w - ui_px(WIDGET_MARGIN);
        }
        let step = ui_px(WIDGET_CASCADE) * self.frames.len() as i32;
        let y = area.y + ui_px(WIDGET_MARGIN) + step;
        f.layout.x = clampi(x - step, area.x, (area.x + area.w - f.layout.w).max(area.x));
        f.layout.y = clampi(y, area.y, (area.y + area.h - f.layout.h).max(area.y));
        f.layout.z = self.next_z;
        self.next_z += 1;
        self.frames.push(f);
        true
    }

    /// Opens a frame from widgets.ini, with its saved place and size, clamped to this screen.
    pub fn restore(&mut self, def: &'static WidgetDef, layout: WidgetLayout, area: Rect) {
        if self.is_open(def.id) || self.frames.len() >= WIDGETS_MAX {
            return;
        }
        let mut f = WidgetFrame { def, layout, calc: Calculator::default() };
        f.clamp(area);
        self.next_z = self.next_z.max(f.layout.z + 1);
        self.frames.push(f);
    }

    pub fn close(&mut self, index: usize) -> bool {
        if index < self.frames.len() {
            self.frames.remove(index);
            true
        } else {
            false
        }
    }

    /// The topmost frame under a point, and which part of it.
    pub fn hit(&self, x: i32, y: i32) -> Option<(usize, WidgetPart)> {
        let mut order: Vec<usize> = (0..self.frames.len()).collect();
        order.sort_by_key(|&i| self.frames[i].layout.z);
        for &i in order.iter().rev() {
            let f = &self.frames[i];
            let r = f.rect();
            if !rect_contains(r, x, y) {
                continue;
            }
            if f.layout.minimized {
                return Some((i, WidgetPart::Title));
            }
            if rect_contains(f.close_rect(), x, y) {
                return Some((i, WidgetPart::Close));
            }
            if rect_contains(f.minimize_rect(), x, y) {
                return Some((i, WidgetPart::Minimize));
            }
            if f.def.resizable && rect_contains(f.resize_rect(), x, y) {
                return Some((i, WidgetPart::Resize));
            }
            if rect_contains(f.title_rect(), x, y) {
                return Some((i, WidgetPart::Title));
            }
            return Some((i, WidgetPart::Body));
        }
        None
    }

    pub fn draw_order(&self) -> Vec<usize> {
        let mut order: Vec<usize> = (0..self.frames.len()).collect();
        order.sort_by_key(|&i| self.frames[i].layout.z);
        order
    }

    /// Keeps every frame on this screen after a turn or a resize.
    pub fn reclamp(&mut self, area: Rect) -> bool {
        let mut changed = false;
        for f in &mut self.frames {
            changed |= f.clamp(area);
        }
        changed
    }

    pub fn load(&mut self, area: Rect) {
        self.frames.clear();
        self.next_z = 0;
        for e in widgets_load() {
            if let Some(def) = widget_def(&e.id) {
                self.restore(def, e.layout, area);
            }
        }
        self.loaded = true;
    }

    pub fn save(&self) -> i32 {
        let entries: Vec<WidgetEntry> = self
            .frames
            .iter()
            .map(|f| WidgetEntry { id: f.def.id.to_string(), layout: f.layout })
            .collect();
        widgets_save(&entries)
    }
}

/// One line of widgets.ini.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WidgetEntry {
    pub id: String,
    pub layout: WidgetLayout,
}

fn widgets_path() -> String {
    format!("{}/widgets.ini", store_root())
}

/// Reads widgets.ini. A missing or unreadable file is an empty layout. Lines that are not a known
/// id with seven comma-separated fields are skipped; the file is small and human-editable.
pub fn widgets_load() -> Vec<WidgetEntry> {
    let root = store_root();
    if root.is_empty() {
        return Vec::new();
    }
    let Some(data) = read_file_max(&widgets_path(), 64 * 1024) else {
        return Vec::new();
    };
    let text = String::from_utf8_lossy(&data);
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        let Some((id, rest)) = line.split_once('=') else { continue };
        let id = id.trim();
        if id.is_empty() || id.len() > 64 || !id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'-' || b == b'_') {
            continue;
        }
        let v: Vec<&str> = rest.split(',').map(str::trim).collect();
        if v.len() != 7 {
            continue;
        }
        let Ok(x) = v[0].parse::<i32>() else { continue };
        let Ok(y) = v[1].parse::<i32>() else { continue };
        let Ok(w) = v[2].parse::<i32>() else { continue };
        let Ok(h) = v[3].parse::<i32>() else { continue };
        let Ok(minimized) = v[4].parse::<i32>() else { continue };
        let Ok(z) = v[5].parse::<i32>() else { continue };
        // Seven fields are required, but the last is reserved for a future flag; accept it only as
        // a number so a malformed line cannot make the rest look valid.
        if v[6].parse::<i32>().is_err() {
            continue;
        }
        if w <= 0 || h <= 0 {
            continue;
        }
        out.push(WidgetEntry {
            id: id.to_string(),
            layout: WidgetLayout { x, y, w, h, minimized: minimized != 0, z },
        });
    }
    out
}

/// Writes widgets.ini atomically. The open frames are the file: a closed widget has no line.
pub fn widgets_save(entries: &[WidgetEntry]) -> i32 {
    let root = store_root();
    if root.is_empty() {
        return 0;
    }
    let _ = mkdir_p(&root);
    let mut s = String::from("# Grimoire widget layout. One line per open widget:\n");
    s.push_str("# id=x,y,w,h,minimized,z\n");
    for e in entries {
        s.push_str(&format!(
            "{}={},{},{},{},{},{},0\n",
            e.id, e.layout.x, e.layout.y, e.layout.w, e.layout.h, e.layout.minimized as i32, e.layout.z
        ));
    }
    write_file_atomic(&widgets_path(), s.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area() -> Rect {
        Rect { x: 0, y: 0, w: 1860, h: 2480 }
    }

    fn panel() -> Rect {
        Rect { x: 1700, y: 220, w: 128, h: 1700 }
    }

    static TEST_WIDGET: WidgetDef = WidgetDef {
        id: "test.widget",
        name: "Test",
        icon: IconKind::Grid,
        kind: WidgetKind::Calculator,
        default_w: 300.0,
        default_h: 420.0,
        min_w: 240.0,
        min_h: 360.0,
        max_w: 460.0,
        max_h: 620.0,
        resizable: true,
        version: "",
        author: "",
        description: "",
    };

    #[test]
    fn a_package_icon_names_a_grimoire_icon() {
        assert_eq!(widget_icon("calculator"), IconKind::Calculator);
        assert_eq!(widget_icon("package"), IconKind::Package);
        assert_eq!(widget_icon("md-timer"), IconKind::Hourglass);
        assert_eq!(widget_icon("not one"), IconKind::Grid);
    }

    #[test]
    fn spawn_stays_on_the_canvas_and_to_the_left_of_the_panel() {
        ui_init(300.0);
        let mut h = WidgetHost::new();
        let notes = &TEST_WIDGET;
        assert!(h.spawn(notes, area(), panel()));
        let r = h.frames[0].rect();
        assert!(r.x >= 0 && r.y >= 0 && r.x + r.w <= area().w && r.y + r.h <= area().h);
        assert!(r.x + r.w <= panel().x);
        // The same widget again raises, does not duplicate.
        let z = h.frames[0].layout.z;
        assert!(!h.spawn(notes, area(), panel()));
        assert_eq!(h.frames.len(), 1);
        assert!(h.frames[0].layout.z >= z);
    }

    #[test]
    fn clamp_and_resize_keep_the_floor_and_the_share() {
        ui_init(300.0);
        let mut f = WidgetFrame::new(&TEST_WIDGET);
        f.resize_to(1, 1, area());
        let (min_w, min_h, _, _) = f.limits(area());
        assert_eq!((f.layout.w, f.layout.h), (min_w, min_h));
        f.resize_to(100_000, 100_000, area());
        let (_, _, max_w, max_h) = f.limits(area());
        assert_eq!((f.layout.w, f.layout.h), (max_w, max_h));
        f.move_to(-500, 9000, area());
        assert!(f.layout.x >= area().x && f.layout.y + f.rect().h <= area().h);
    }

    #[test]
    fn minimized_rect_is_a_square_and_restore_keeps_the_size() {
        ui_init(300.0);
        let mut f = WidgetFrame::new(&TEST_WIDGET);
        let (w, h) = (f.layout.w, f.layout.h);
        f.layout.minimized = true;
        let r = f.rect();
        assert_eq!(r.w, r.h);
        f.layout.minimized = false;
        assert_eq!((f.layout.w, f.layout.h), (w, h));
    }

    #[test]
    fn layout_round_trips_through_widgets_ini() {
        ui_init(300.0);
        let root = std::env::temp_dir().join(format!("grimoire-widgets-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        assert_eq!(store_init(root.to_str().unwrap()), 0);
        let entries = vec![WidgetEntry {
            id: "test.widget".to_string(),
            layout: WidgetLayout { x: 10, y: 20, w: 260, h: 220, minimized: true, z: 3 },
        }];
        assert_eq!(widgets_save(&entries), 0);
        assert_eq!(widgets_load(), entries);
        // A bad line is skipped, not a reason to lose the good ones.
        std::fs::write(
            root.join("widgets.ini"),
            "# c\ntest.widget=1,2,3,4,0,5,0\nbad=1,2\nother=1,2,3,4,0,6,0\n",
        )
        .unwrap();
        let got = widgets_load();
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].id, "test.widget");
        assert_eq!(got[1].layout.z, 6);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn the_calculator_adds_multiplies_and_recovers_from_divide_by_zero() {
        let mut c = Calculator::default();
        assert_eq!(c.display(), "0");
        for k in [12, 15, 13, 19] {
            c.press(k);
        }
        assert_eq!(c.display(), "3");
        c.press(7);
        c.press(12);
        c.press(13);
        c.press(19);
        assert_eq!(c.display(), "36");
        c.press(0);
        for k in [12, 3, 16, 19] {
            c.press(k);
        }
        assert_eq!(c.display(), "Error");
        c.press(12);
        assert_eq!(c.display(), "1");
    }
}
