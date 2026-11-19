//! widgets.c: the editor's widget windows and their layout.
//!
//! Phase 1 has built-in demo widgets only. A widget is a small window on the editor canvas: it can
//! be raised, dragged by its title bar, minimized, resized from its bottom-right grip, and closed.
//! Its place and size are kept in widgets.ini at the store root, so they come back on the next
//! launch. The catalog is the seam a package-installed widget replaces later; nothing here knows
//! about packages, WASM or the network.
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

/// What a built-in demo widget draws in its body. Package widgets will carry their own body; this
/// enum is what Phase 1 can draw without a renderer.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WidgetKind {
    #[default]
    StickyNotes,
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
}

/// The built-in demo widgets. A package-installed widget takes an entry's place here later; until
/// then these two show the frame, popup and persistence working end to end.
pub const WIDGET_CATALOG: [WidgetDef; 2] = [
    WidgetDef {
        id: "demo.sticky-notes",
        name: "Sticky Notes",
        icon: IconKind::Notebook,
        kind: WidgetKind::StickyNotes,
        default_w: 260.0,
        default_h: 220.0,
        min_w: 180.0,
        min_h: 140.0,
        max_w: 520.0,
        max_h: 520.0,
        resizable: true,
    },
    WidgetDef {
        id: "demo.calculator",
        name: "Calculator",
        icon: IconKind::Grid,
        kind: WidgetKind::Calculator,
        default_w: 300.0,
        default_h: 380.0,
        min_w: 220.0,
        min_h: 280.0,
        max_w: 460.0,
        max_h: 560.0,
        resizable: true,
    },
];

pub fn widget_catalog() -> &'static [WidgetDef] {
    &WIDGET_CATALOG
}

pub fn widget_def(id: &str) -> Option<&'static WidgetDef> {
    WIDGET_CATALOG.iter().find(|d| d.id == id)
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
        let mut f = WidgetFrame { def, layout };
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

    #[test]
    fn spawn_stays_on_the_canvas_and_to_the_left_of_the_panel() {
        ui_init(300.0);
        let mut h = WidgetHost::new();
        let notes = widget_def("demo.sticky-notes").unwrap();
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
        let mut f = WidgetFrame::new(widget_def("demo.calculator").unwrap());
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
        let mut f = WidgetFrame::new(widget_def("demo.sticky-notes").unwrap());
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
            id: "demo.sticky-notes".to_string(),
            layout: WidgetLayout { x: 10, y: 20, w: 260, h: 220, minimized: true, z: 3 },
        }];
        assert_eq!(widgets_save(&entries), 0);
        assert_eq!(widgets_load(), entries);
        let mut host = WidgetHost::new();
        host.load(area());
        assert_eq!(host.frames.len(), 1);
        assert_eq!(host.frames[0].def.id, "demo.sticky-notes");
        assert!(host.frames[0].layout.minimized);
        // A bad line is skipped, not a reason to lose the good ones.
        std::fs::write(
            root.join("widgets.ini"),
            "# c\ndemo.sticky-notes=1,2,3,4,0,5,0\nbad=1,2\nother=1,2,3,4,0,6,0\n",
        )
        .unwrap();
        let got = widgets_load();
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].id, "demo.sticky-notes");
        assert_eq!(got[1].layout.z, 6);
        let _ = std::fs::remove_dir_all(&root);
    }
}
