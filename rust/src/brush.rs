//! brush.h: drawing tools, their tweakable sensitivity, and the deterministic dab renderer.

use crate::prelude::*;
use std::f32::consts::PI;
use std::sync::atomic::Ordering;
use std::sync::OnceLock;

/// Stored on disk: never reorder. A newtype, not an enum: a stroke keeps a tool this build does not
/// know (it is never drawn), and settings.ini can name any number.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ToolKind(pub i32);
pub const TOOL_PENCIL: ToolKind = ToolKind(0);
pub const TOOL_MECH_PENCIL: ToolKind = ToolKind(1);
pub const TOOL_BALLPOINT: ToolKind = ToolKind(2);
pub const TOOL_ERASER: ToolKind = ToolKind(3);
pub const TOOL_HIGHLIGHTER: ToolKind = ToolKind(4);
/// italic nib: thick/thin by stroke direction
pub const TOOL_FOUNTAIN: ToolKind = ToolKind(5);
/// constant thin crisp line
pub const TOOL_FINELINER: ToolKind = ToolKind(6);
/// broad felt tip
pub const TOOL_MARKER: ToolKind = ToolKind(7);
/// soft stick: coarse grain, crumbly edges, broad on its side
pub const TOOL_CHARCOAL: ToolKind = ToolKind(8);
/// broad-edge nib held at 90 degrees to the pen's lean
pub const TOOL_CALLIGRAPHY: ToolKind = ToolKind(9);
pub const TOOL_COUNT: usize = 10;

/// "Pencil", "Mechanical pencil", "Ballpoint", "Eraser", "Highlighter"...
/// The English names go to grimoire.log too; tool_label is what the screens show.
pub fn tool_name(t: ToolKind) -> &'static str {
    match t {
        // TRANSLATORS: the drawing tools: in the editor's tool picker (268 px at 34 px in Gothic, 284 in
        // Classic, where the Mechanical pencil and the Calligraphy pen go by shorter names) and after
        // "Pen feel:".
        TOOL_PENCIL => nc_("tool", "Pencil"),
        TOOL_MECH_PENCIL => nc_("tool", "Mechanical pencil"),
        TOOL_BALLPOINT => nc_("tool", "Ballpoint"),
        TOOL_ERASER => nc_("tool", "Eraser"),
        TOOL_HIGHLIGHTER => nc_("tool", "Highlighter"),
        TOOL_FOUNTAIN => nc_("tool", "Fountain pen"),
        TOOL_FINELINER => nc_("tool", "Fineliner"),
        TOOL_MARKER => nc_("tool", "Marker"),
        TOOL_CHARCOAL => nc_("tool", "Charcoal"),
        TOOL_CALLIGRAPHY => nc_("tool", "Calligraphy pen"),
        _ => nc_("tool", "Tool"),
    }
}
/// the same in the language in use, for the screens
pub fn tool_label(t: ToolKind) -> &'static str {
    c_("tool", tool_name(t))
}

/// The tool picker's pens in its order, three to a row (dry media, then pens, then the nibs and the
/// highlighter), then the eraser: the order of next and previous in the picker (the first
/// TOOL_PENS; next and previous tool too, with fewer than two pens on the panel), Pen feel's tabs and
/// the controller's Buttons sheet.
pub const TOOL_PENS: usize = TOOL_COUNT - 1;
pub static TOOL_ORDER: [ToolKind; TOOL_COUNT] = [
    TOOL_PENCIL,
    TOOL_MECH_PENCIL,
    TOOL_CHARCOAL,
    TOOL_BALLPOINT,
    TOOL_FINELINER,
    TOOL_MARKER,
    TOOL_FOUNTAIN,
    TOOL_CALLIGRAPHY,
    TOOL_HIGHLIGHTER,
    TOOL_ERASER,
];

/// Every field is user-tweakable per tool (pensettings.c draws one slider per entry of
/// tool_param_table). Stored in settings.ini and snapshotted into each stroke.
/// Thirteen f32 in this order, which page files and settings.ini also keep (docs/FORMAT.md): get and
/// get_mut reach them by that index (the C's offsetof).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ToolParams {
    /// width at zero (post-curve) pressure
    pub size_min_mm: f32,
    /// width at full pressure
    pub size_max_mm: f32,
    /// darkness 0..1 at zero pressure
    pub opacity_min: f32,
    /// darkness 0..1 at full pressure
    pub opacity_max: f32,
    /// raw pressure 0..1 below which the pen reads as 0 (deadzone)
    pub pressure_floor: f32,
    /// raw pressure 0..1 at/above which it reads as full
    pub pressure_ceiling: f32,
    /// curve: out = in^gamma (<1 = soft/sensitive, >1 = firm)
    pub pressure_gamma: f32,
    /// tilt from vertical below which tilt counts as 0
    pub tilt_min_deg: f32,
    /// tilt at/above which tilt counts as 1
    pub tilt_max_deg: f32,
    /// width *= 1 + tilt_size * tilt  (shading with the side)
    pub tilt_size: f32,
    /// opacity *= 1 - tilt_opacity * tilt
    pub tilt_opacity: f32,
    /// 0..1 paper-tooth texture
    pub grain: f32,
    /// 0..1 input smoothing (applied before storing points)
    pub smoothing: f32,
}

impl ToolParams {
    pub const COUNT: usize = 13;
    /// Field i in declaration order (0 size_min_mm .. 12 smoothing).
    pub fn get(&self, i: usize) -> f32 {
        let p = self;
        [
            p.size_min_mm,
            p.size_max_mm,
            p.opacity_min,
            p.opacity_max,
            p.pressure_floor,
            p.pressure_ceiling,
            p.pressure_gamma,
            p.tilt_min_deg,
            p.tilt_max_deg,
            p.tilt_size,
            p.tilt_opacity,
            p.grain,
            p.smoothing,
        ][i]
    }
    pub fn get_mut(&mut self, i: usize) -> &mut f32 {
        match i {
            0 => &mut self.size_min_mm,
            1 => &mut self.size_max_mm,
            2 => &mut self.opacity_min,
            3 => &mut self.opacity_max,
            4 => &mut self.pressure_floor,
            5 => &mut self.pressure_ceiling,
            6 => &mut self.pressure_gamma,
            7 => &mut self.tilt_min_deg,
            8 => &mut self.tilt_max_deg,
            9 => &mut self.tilt_size,
            10 => &mut self.tilt_opacity,
            11 => &mut self.grain,
            12 => &mut self.smoothing,
            _ => panic!("ToolParams has 13 fields, not {}", i + 1),
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct ParamInfo {
    /// settings.ini key, e.g. "pressure_gamma"
    pub key: &'static str,
    /// UI label, e.g. "Pressure curve"
    pub label: &'static str,
    /// "mm", "deg", "" ...
    pub unit: &'static str,
    /// the field, as ToolParams::get takes it (the C's offsetof(ToolParams, field))
    pub field: usize,
    pub min: f32,
    pub max: f32,
    pub step: f32,
    /// shown on the eraser tab (size only)
    pub eraser_uses: bool,
}

const fn param(field: usize, key: &'static str, label: &'static str, unit: &'static str, min: f32, max: f32, step: f32, eraser_uses: bool) -> ParamInfo {
    ParamInfo { key, label, unit, field, min, max, step, eraser_uses }
}
// The labels are looked up where Pen feel draws them (pensettings.c); keys and units stay English.
// The field is the ToolParams index named in each row's comment (the C's offsetof).
static K_PARAMS: [ParamInfo; ToolParams::COUNT] = [
    // TRANSLATORS: Pen feel's sliders, each label left of its value, at 44 px: in Gothic about 530 px
    // beside a width (up to "12.00 mm"), 570 beside an angle, 630 beside a plain number ("1.25"); in
    // Classic 560, 600 and 640.
    param(0, "size_min_mm", n_("Min width"), "mm", 0.10, 12.0, 0.01, true), // size_min_mm
    param(1, "size_max_mm", n_("Max width"), "mm", 0.10, 12.0, 0.01, true), // size_max_mm
    param(2, "opacity_min", n_("Light darkness"), "", 0.0, 1.0, 0.01, false), // opacity_min
    param(3, "opacity_max", n_("Full darkness"), "", 0.0, 1.0, 0.01, false), // opacity_max
    param(4, "pressure_floor", n_("Pressure deadzone"), "", 0.0, 0.5, 0.01, false), // pressure_floor
    param(5, "pressure_ceiling", n_("Pressure full at"), "", 0.05, 1.0, 0.01, false), // pressure_ceiling
    param(6, "pressure_gamma", n_("Pressure curve"), "", 0.25, 4.0, 0.05, false), // pressure_gamma
    param(7, "tilt_min_deg", n_("Tilt starts at"), "deg", 0.0, 85.0, 1.0, false), // tilt_min_deg
    param(8, "tilt_max_deg", n_("Tilt full at"), "deg", 5.0, 90.0, 1.0, false), // tilt_max_deg
    param(9, "tilt_size", n_("Tilt widens"), "x", 0.0, 4.0, 0.05, false), // tilt_size
    param(10, "tilt_opacity", n_("Tilt lightens"), "", 0.0, 1.0, 0.01, false), // tilt_opacity
    param(11, "grain", n_("Grain"), "", 0.0, 1.0, 0.01, false), // grain
    param(12, "smoothing", n_("Smoothing"), "", 0.0, 1.0, 0.01, false), // smoothing
];
// (the C's _Static_assert: the param table covers every ToolParams field)
const _: () = assert!(ToolParams::COUNT * std::mem::size_of::<f32>() == std::mem::size_of::<ToolParams>());

pub fn tool_param_table() -> &'static [ParamInfo] {
    &K_PARAMS
}

/// ToolParams from its thirteen fields in declaration order, the way the C's initializers list them.
const fn tp(v: [f32; ToolParams::COUNT]) -> ToolParams {
    ToolParams {
        size_min_mm: v[0],
        size_max_mm: v[1],
        opacity_min: v[2],
        opacity_max: v[3],
        pressure_floor: v[4],
        pressure_ceiling: v[5],
        pressure_gamma: v[6],
        tilt_min_deg: v[7],
        tilt_max_deg: v[8],
        tilt_size: v[9],
        tilt_opacity: v[10],
        grain: v[11],
        smoothing: v[12],
    }
}

pub fn tool_params_default(t: ToolKind, out: &mut ToolParams) {
    *out = match t {
        TOOL_MECH_PENCIL => tp([0.3000, 0.3000, 0.5000, 0.9500, 0.0000, 1.0000, 1.5000, 0.0000, 90.0000, 0.0000, 0.0000, 0.5000, 0.0000]),
        TOOL_BALLPOINT => tp([0.3000, 0.4800, 0.8500, 1.0000, 0.0000, 1.0000, 1.5000, 0.0000, 90.0000, 0.0000, 0.0000, 0.0000, 0.5000]),
        TOOL_ERASER => tp([2.5000, 7.0000, 1.0000, 1.0000, 0.0000, 0.6000, 1.0000, 30.0000, 75.0000, 0.0000, 0.0000, 0.0000, 0.0000]),
        // Owner's 2026-10-30 settings: 0.25..1.00 mm; the angled nib makes it calligraphic.
        TOOL_FOUNTAIN => tp([0.2500, 1.0000, 1.0000, 1.0000, 0.0000, 1.0000, 1.0000, 0.0000, 90.0000, 0.0000, 0.0000, 0.0000, 0.0000]),
        TOOL_FINELINER => tp([0.2800, 0.2800, 1.0000, 1.0000, 0.0000, 1.0000, 1.0000, 0.0000, 90.0000, 0.0000, 0.0000, 0.0000, 1.0000]),
        TOOL_MARKER => tp([1.0000, 1.0000, 0.9100, 1.0000, 0.0100, 1.0000, 1.0000, 0.0000, 75.0000, 1.5000, 0.0000, 0.1500, 0.5000]),
        // 0.30..0.55 mm, pressure-led with broad shading as the pen leans.
        TOOL_CHARCOAL => tp([0.3000, 0.5500, 0.7500, 1.0000, 0.0000, 1.0000, 0.5000, 25.0000, 90.0000, 4.0000, 0.3500, 0.6500, 0.0000]),
        // A broad, flat nib with a constant 1.75 mm width. The tilt
        // range here says when the nib starts to follow the pen's lean (see brush_nib_angle).
        TOOL_CALLIGRAPHY => tp([1.7500, 1.7500, 0.7500, 1.0000, 0.0000, 1.0000, 1.0000, 0.0000, 90.0000, 0.0000, 0.0000, 0.0000, 0.5000]),
        // Broad and flat; darken-only blending keeps the writing underneath fully black.
        TOOL_HIGHLIGHTER => tp([5.0000, 5.0000, 0.2500, 0.7500, 0.0000, 1.0000, 1.0000, 0.0000, 90.0000, 0.0000, 0.0000, 0.0000, 0.0000]),
        // TOOL_PENCIL and any other: 0.12..0.32 mm: a sharp pencil; tilt still shades wider.
        _ => tp([0.1200, 0.3200, 0.8500, 0.9500, 0.0000, 1.0000, 1.0000, 25.0000, 55.0000, 4.0000, 0.7500, 0.7500, 0.0000]),
    };
}
#[inline]
pub fn tool_param_ptr<'a>(p: &'a mut ToolParams, pi: &ParamInfo) -> &'a mut f32 {
    p.get_mut(pi.field)
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PenSample {
    /// page px
    pub x: f32,
    pub y: f32,
    /// raw normalized 0..1 (device ABS_PRESSURE / max)
    pub pressure: f32,
    /// degrees, 0 = vertical; 0,0 if the digitizer lacks tilt. In the page's frame: positive = the
    /// pen's top leans toward +x (right) / +y (down)
    pub tilt_x: f32,
    pub tilt_y: f32,
    pub t_ms: u32,
}

/// -> 0..1
pub fn brush_pressure(p: &ToolParams, raw: f32) -> f32 {
    let lo = sane(p.pressure_floor, 0.0, 0.5, 0.0);
    let mut hi = sane(p.pressure_ceiling, 0.05, 1.0, 1.0);
    if hi <= lo + 0.01 {
        hi = lo + 0.01;
    }
    let x = clampf((sane(raw, 0.0, 1.0, 0.0) - lo) / (hi - lo), 0.0, 1.0);
    x.powf(sane(p.pressure_gamma, 0.25, 4.0, 1.0))
}
/// -> 0..1
pub fn brush_tilt(p: &ToolParams, tx: f32, ty: f32) -> f32 {
    let tx = sane(tx, -89.0, 89.0, 0.0) * PI / 180.0;
    let ty = sane(ty, -89.0, 89.0, 0.0) * PI / 180.0;
    let a = tx.tan();
    let b = ty.tan();
    let deg = fmla(a, a, b * b).sqrt().atan() * 180.0 / PI;
    let lo = sane(p.tilt_min_deg, 0.0, 85.0, 30.0);
    let mut hi = sane(p.tilt_max_deg, 5.0, 90.0, 70.0);
    if hi <= lo + 1.0 {
        hi = lo + 1.0;
    }
    let x = clampf((deg - lo) / (hi - lo), 0.0, 1.0);
    (x * x) * fmla(-2.0f32, x, 3.0) // smoothstep
}
/// Dab radius (px) and opacity (0..1) for a sample.
pub fn brush_dab(t: ToolKind, p: &ToolParams, s: &PenSample, dpi: f32, radius_px: &mut f32, opacity: &mut f32) {
    let floor_ = sane(p.pressure_floor, 0.0, 0.5, 0.0);
    let pr = brush_pressure(p, s.pressure);
    let ts = sane(p.tilt_size, 0.0, 4.0, 0.0);
    let to = sane(p.tilt_opacity, 0.0, 1.0, 0.0);
    let tl = if ts > 0.0 || to > 0.0 { brush_tilt(p, s.tilt_x, s.tilt_y) } else { 0.0 };
    let mut w = lerpf(sane(p.size_min_mm, 0.1, 12.0, 0.3), sane(p.size_max_mm, 0.1, 12.0, 0.5), pr);
    w *= fmla(ts, tl, 1.0);
    let r = mm_to_px(w, dpi) * 0.5;
    *radius_px = if r < 0.5 { 0.5 } else { r };
    let mut o = lerpf(sane(p.opacity_min, 0.0, 1.0, 0.5), sane(p.opacity_max, 0.0, 1.0, 1.0), pr);
    o *= fmla(-to, tl, 1.0);
    // Below the deadzone the pen is touching but does not mark.
    if s.pressure < floor_ {
        o = 0.0;
    }
    *opacity = clampf(o, 0.0, 1.0);
    let _ = t;
}
/// Largest radius any sample of this tool with these params can produce (for bbox padding).
pub fn brush_max_radius(t: ToolKind, p: &ToolParams, dpi: f32) -> f32 {
    let mut w = sane(p.size_min_mm, 0.1, 12.0, 0.3).max(sane(p.size_max_mm, 0.1, 12.0, 0.5));
    w *= 1.0 + sane(p.tilt_size, 0.0, 4.0, 0.0);
    // + 30%: graphite edges follow the paper tooth slightly past the nominal radius. Charcoal
    // reaches further: a rougher edge, and loose specks beyond it.
    let grain = sane(p.grain, 0.0, 1.0, 0.0);
    let k = if t == TOOL_CHARCOAL { 1.0 + (0.5f32 * 0.95 * grain).max(0.40) } else { fmla(0.3f32, grain, 1.0) };
    let r = fmla(mm_to_px(w, dpi) * 0.5, k, if t == TOOL_CHARCOAL { 1.0 } else { 0.0 });
    if r < 0.5 {
        0.5
    } else {
        r
    }
}

// Default lean for a pen held nearly upright: toward the bottom right, the way a right hand
// holds a calligraphy pen. As twice-the-edge-angle: the edge is at 90 degrees to the lean.
// Returns (cos, sin) of that angle (the C's c2 and s2 out-parameters).
fn nib_target(p: &ToolParams, tx: f32, ty: f32) -> (f32, f32) {
    // Lean direction on the page: the pen's top, projected (screen frame, y down).
    let lx = (sane(tx, -89.0, 89.0, 0.0) * PI / 180.0).tan();
    let ly = (sane(ty, -89.0, 89.0, 0.0) * PI / 180.0).tan();
    let w = brush_tilt(p, tx, ty); // 0 = too upright to trust the lean
    let def2 = -0.5 * PI; // lean 45 deg down-right -> edge at -45 deg -> 2x = -90
    let mut a2 = def2;
    if w > 0.0 && (lx != 0.0 || ly != 0.0) {
        // Edge angle = lean + 90 deg, so twice it = 2 * lean + 180 deg.
        let lean2 = fmla(2.0f32, ly.atan2(lx), PI);
        let mut d = lean2 - def2;
        while d > PI {
            d -= 2.0 * PI;
        }
        while d <= -PI {
            d += 2.0 * PI;
        }
        a2 = fmla(w, d, def2); // along the shorter way round, so the blend never jumps
    }
    (a2.cos(), a2.sin())
}
/// Calligraphy: the nib's pen angle for a pen tilted (tilt_x, tilt_y), in degrees above the
/// horizontal (45 = the edge rises to the right at 45 degrees). The broad edge sits at 90
/// degrees to the pen's lean; a pen held nearly upright gets the default 45 (leaning bottom right).
pub fn brush_nib_angle(p: &ToolParams, tilt_x: f32, tilt_y: f32) -> f32 {
    let (c2, s2) = nib_target(p, tilt_x, tilt_y);
    // Screen angle of the edge (y down) -> pen angle above the horizontal, in 0..180.
    let mut deg = -0.5 * s2.atan2(c2) * 180.0 / PI;
    while deg < 0.0 {
        deg += 180.0;
    }
    while deg >= 180.0 {
        deg -= 180.0;
    }
    deg
}

/// Incremental renderer. Live drawing and re-rendering both go through brush_add so the
/// results are pixel-identical. Ink color is fixed per tool (pencil graphite ~40,
/// mech ~30, ballpoint ~10).
#[derive(Clone, Debug, Default)]
pub struct BrushState {
    pub tool: ToolKind,
    pub params: ToolParams,
    pub seed: u32,
    pub dpi: f32,
    pub have_prev: bool,
    pub prev: PenSample,
    /// distance carried to the next dab
    pub carry: f32,
    pub dab_index: u32,
    /// calligraphy: smoothed nib orientation as (cos, sin) of twice the edge angle, so an edge
    /// and its reverse are the same orientation
    pub nib_init: bool,
    pub nib_c: f32,
    pub nib_s: f32,
    /// Erase mode (brush_set_erase): the dabs restore the paper instead of depositing ink.
    pub erase: bool,
    /// erase mode: a dab has lightened a pixel of the canvas drawn on so far.
    /// That canvas only: over ink the page's darker lines hide, an erase
    /// stroke can leave the page as it was and still change the ink layer,
    /// so whether a stroke may be left out is page_droppable's to say
    pub restored: bool,
    /// the paper under the canvas, one byte per pixel; None = white. Shared with whoever made it
    /// (the C's pointer into its caller's buffer): PORTING.md, Ownership.
    pub paper: Option<std::rc::Rc<Vec<u8>>>,
    /// the canvas pixel that paper[0] lies under
    pub paper_x0: i32,
    pub paper_y0: i32,
    pub paper_stride: i32,
    /// Dry (set by hand after begin): brush_add draws nothing and touches no pixel (the canvas may
    /// have none), but returns the rect each dab would draw into, clipped as ever, and the brush
    /// moves on exactly as when it draws. page_droppable measures strokes so.
    pub dry: bool,
    /// the tip variant (below): brush_begin sets the tool's current one,
    /// stroke_brush_begin the one its stroke was drawn with
    pub tip: u8,
}

/// Tip variants (a stroke's ext[2], low 4 bits; docs/FORMAT.md): 0 is a tool's first tip, the one
/// every version drew it with before variants existed. A tool's tip changes only by a new variant,
/// recorded on the strokes drawn with it, so a page already written keeps its look.
pub const TIP_FIRST: u8 = 0;
/// the highlighter's fixed vertical edge (1.2.0; its first tip is round)
pub const TIP_HIGHLIGHTER_VERTICAL: u8 = 1;
/// The tip this version gives new strokes of tool t.
pub fn brush_tip_current(t: ToolKind) -> u8 {
    if t == TOOL_HIGHLIGHTER {
        TIP_HIGHLIGHTER_VERTICAL
    } else {
        TIP_FIRST
    }
}
/// Whether this version knows tip variant v of tool t (it draws an unknown one with the current tip).
pub fn brush_tip_known(t: ToolKind, v: u8) -> bool {
    v == TIP_FIRST || (t == TOOL_HIGHLIGHTER && v == TIP_HIGHLIGHTER_VERTICAL)
}

// Per-tool look that is not user-tweakable.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum NibKind {
    #[default]
    None, // NIB_NONE
    Italic,    // NIB_ITALIC
    BroadEdge, // NIB_BROAD_EDGE
    Vertical,  // NIB_VERTICAL
}
#[derive(Clone, Copy, Debug, Default)]
struct ToolLook {
    ink: u8,
    /// fraction of the radius that fades out
    rim: f32,
    /// extra darkness toward the center (ballpoint)
    core: f32,
    /// per-dab darkness variation, scaled by grain
    jitter: f32,
    /// paper-tooth deposition with ragged edges (pencils, charcoal)
    graphite: bool,
    /// italic: an ellipse at a fixed 40 degrees (fountain pen); broad edge: a
    /// square-cut flat nib turned by the pen's lean (calligraphy)
    nib: NibKind,
    // Graphite only (pencils use the defaults):
    /// tooth sampled at twice the scale: clumpy charcoal grit
    coarse: bool,
    /// how far the edge follows the tooth (x radius x grain)
    rough: f32,
    /// soft edge width (x radius)
    edge: f32,
    /// how much full pressure fills the tooth's valleys
    fill: f32,
    /// chance of a loose speck just outside the edge
    dust: f32,
    /// half-width of the tooth threshold's soft step (wider = grayer, less speckle)
    band: f32,
}

fn look_of(t: ToolKind, tip: u8) -> ToolLook {
    let z = ToolLook::default();
    let mut lk = match t {
        TOOL_MECH_PENCIL => ToolLook { ink: 32, rim: 0.20, core: 0.0, jitter: 0.06, graphite: true, ..z },
        TOOL_BALLPOINT => ToolLook { ink: 12, rim: 0.10, core: 0.35, jitter: 0.0, graphite: false, ..z },
        // Round (its first tip, through 1.1) or a fixed vertical edge (since 1.2.0).
        TOOL_HIGHLIGHTER => ToolLook {
            ink: 110,
            rim: 0.06,
            core: 0.0,
            jitter: 0.0,
            graphite: false,
            nib: if tip == TIP_HIGHLIGHTER_VERTICAL { NibKind::Vertical } else { NibKind::None },
            ..z
        },
        TOOL_FOUNTAIN => ToolLook { ink: 8, rim: 0.10, core: 0.0, jitter: 0.0, graphite: false, nib: NibKind::Italic, ..z },
        TOOL_CALLIGRAPHY => ToolLook { ink: 6, rim: 0.08, core: 0.0, jitter: 0.0, graphite: false, nib: NibKind::BroadEdge, ..z },
        TOOL_FINELINER => ToolLook { ink: 5, rim: 0.08, core: 0.0, jitter: 0.0, graphite: false, ..z },
        TOOL_MARKER => ToolLook { ink: 22, rim: 0.14, core: 0.0, jitter: 0.0, graphite: false, ..z },
        TOOL_CHARCOAL => ToolLook {
            ink: 14,
            rim: 0.0,
            core: 0.0,
            jitter: 0.12,
            graphite: true,
            nib: NibKind::None,
            coarse: true,
            rough: 0.95,
            edge: 0.32,
            fill: 0.50,
            dust: 0.05,
            band: 0.24,
        },
        // The precise eraser's round hard tip, for erase strokes only (it never deposits ink).
        TOOL_ERASER => ToolLook { ink: 255, rim: 0.08, core: 0.0, jitter: 0.0, graphite: false, ..z },
        _ => ToolLook { ink: 38, rim: 0.35, core: 0.0, jitter: 0.10, graphite: true, ..z },
    };
    if lk.graphite && !lk.coarse {
        // the pencils
        lk.rough = 0.6;
        lk.edge = 0.22;
        lk.fill = 0.70;
        lk.band = 0.16;
    }
    lk
}

// Nib shapes: long axis along the edge, the short one this fraction of it.
const ITALIC_RATIO: f32 = 0.22;
const BROAD_RATIO: f32 = 0.13;

fn sane(v: f32, lo: f32, hi: f32, dflt: f32) -> f32 {
    if v.is_nan() {
        return dflt; // NaN
    }
    clampf(v, lo, hi)
}

// ---- paper tooth: a fixed, tiled value-noise texture over absolute page pixels ----

const TOOTH_N: usize = 512;
/// g_tooth and g_tooth_ready: built once (brush_begin), then only read.
static TOOTH: OnceLock<Box<[u8]>> = OnceLock::new();

fn hash2(x: u32, y: u32) -> u32 {
    let mut h = x.wrapping_mul(374761393).wrapping_add(y.wrapping_mul(668265263));
    h = (h ^ (h >> 13)).wrapping_mul(1274126177);
    h ^ (h >> 16)
}

// Bilinear value noise in [0,1] with lattice cells of cw x ch pixels (wrapping on TOOTH_N).
fn vnoise(x: i32, y: i32, cw: i32, ch: i32, salt: u32) -> f32 {
    let gx = x / cw;
    let gy = y / ch;
    let fx = (x % cw) as f32 / cw as f32;
    let fy = (y % ch) as f32 / ch as f32;
    let nx = TOOTH_N as i32 / cw;
    let ny = TOOTH_N as i32 / ch;
    let lattice = |ax: i32, ay: i32| (hash2(ax as u32, (ay as u32).wrapping_add(salt)) & 0xFFFF) as f32 / 65535.0;
    let v00 = lattice(gx % nx, gy % ny);
    let v10 = lattice((gx + 1) % nx, gy % ny);
    let v01 = lattice(gx % nx, (gy + 1) % ny);
    let v11 = lattice((gx + 1) % nx, (gy + 1) % ny);
    lerpf(lerpf(v00, v10, fx), lerpf(v01, v11, fx), fy)
}

// Paper tooth: crisp per-pixel speckle, small grains and long horizontal fibers, contrast-
// stretched. A soft, blurry tooth is what made pencil look "bubbly"; graphite catches on fine,
// sharp-edged peaks of the paper.
fn tooth_init() -> Box<[u8]> {
    let mut tooth = vec![0u8; TOOTH_N * TOOTH_N].into_boxed_slice();
    for y in 0..TOOTH_N as i32 {
        for x in 0..TOOTH_N as i32 {
            let white = (hash2(x as u32, (y as u32).wrapping_add(0x7F4A)) & 0xFFFF) as f32 / 65535.0;
            let grains = vnoise(x, y, 3, 2, 0x9E37);
            let fibers = vnoise(x, y, 16, 2, 0x51ED);
            // (0.42f * white + 0.33f * grains + 0.25f * fibers: two fused multiply-adds)
            let mut n = fmla(0.25f32, fibers, fmla(0.42f32, white, 0.33 * grains));
            n = clampf((n - 0.18) / 0.64, 0.0, 1.0);
            n = (n * n) * fmla(-2.0f32, n, 3.0);
            tooth[y as usize * TOOTH_N + x as usize] = (n * 255.0).round_ties_even() as u8; // lrintf
        }
    }
    tooth
}

#[inline]
fn tooth_at(tooth: &[u8], x: i32, y: i32) -> f32 {
    tooth[((y as u32 as usize) & (TOOTH_N - 1)) * TOOTH_N + ((x as u32 as usize) & (TOOTH_N - 1))] as f32 / 255.0
}

// ---- renderer ----

pub fn brush_begin(b: &mut BrushState, t: ToolKind, p: &ToolParams, seed: u32, dpi: f32) {
    TOOTH.get_or_init(tooth_init);
    *b = BrushState::default();
    b.tool = t;
    b.params = *p;
    b.seed = seed;
    b.dpi = if dpi > 50.0 && dpi < 2000.0 { dpi } else { 300.0 };
    b.tip = brush_tip_current(t);
}
/// Puts b (just begun) in erase mode, for an erase stroke: white ink with the tool's texture, or
/// the precise eraser's round hard tip (TOOL_ERASER). Its dabs lighten each pixel toward the paper
/// under it, never past it, and a pixel the dab covers fully lands exactly on the paper. The paper
/// under canvas pixel (x, y) is paper[(y - y0) * stride + (x - x0)], and it must cover every pixel
/// of the canvas's clip while b draws (the dabs stay inside the clip). NULL restores toward white:
/// right for a canvas of ink alone, wrong over paper with lines (it would wipe them).
pub fn brush_set_erase(b: &mut BrushState, paper: Option<std::rc::Rc<Vec<u8>>>, x0: i32, y0: i32, stride: i32) {
    b.erase = true;
    b.restored = false;
    b.paper = paper;
    b.paper_x0 = x0;
    b.paper_y0 = y0;
    b.paper_stride = stride;
}
/// The erase fast path (on unless cleared): a round dab with no grain at full opacity restores the
/// core of each row it covers in one pass. Its pixels are the per-pixel path's, byte for byte;
/// tests clear it to prove that, and measurements to time both.
pub static BRUSH_ERASE_SPANS: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);
/// Renders dabs from the previous sample to s (the first call stamps one dab at s).
/// Returns the dirty rect touched (may be empty). A TOOL_ERASER brush draws only in erase mode.
pub fn brush_add(b: &mut BrushState, c: &mut Canvas, s: &PenSample) -> Rect {
    if b.tool == TOOL_ERASER && !b.erase {
        b.prev = *s;
        b.have_prev = true;
        return RECT_EMPTY;
    }
    let mut dirty = RECT_EMPTY;
    if !b.have_prev {
        dirty = stamp(b, c, s);
        let mut r = 0.0f32;
        let mut op = 0.0f32;
        brush_dab(b.tool, &b.params, s, b.dpi, &mut r, &mut op);
        b.carry = spacing_for(r);
        b.prev = *s;
        b.have_prev = true;
        return dirty;
    }
    let dx = s.x - b.prev.x;
    let dy = s.y - b.prev.y;
    let len = fmla(dx, dx, dy * dy).sqrt();
    if len > 20000.0 {
        // corrupt data: do not bridge with ink
        b.prev = *s;
        return dirty;
    }
    let mut d = b.carry;
    while d <= len {
        let m = interp(&b.prev, s, if len > 0.0 { d / len } else { 1.0 });
        dirty = rect_union(dirty, stamp(b, c, &m));
        let mut r = 0.0f32;
        let mut op = 0.0f32;
        brush_dab(b.tool, &b.params, &m, b.dpi, &mut r, &mut op);
        d += spacing_for(r);
    }
    b.carry = d - len;
    b.prev = *s;
    dirty
}

fn spacing_for(r: f32) -> f32 {
    (0.15 * r).max(0.5)
}

// Where a dab's pixels go: ink deposited, or for an erase stroke the paper restored.
struct DabTarget<'a> {
    ink: u8,
    /// erase: None = white
    paper: Option<&'a [u8]>,
    x0: i32,
    y0: i32,
    stride: i32,
    /// erase: set once a pixel is lightened
    restored: bool,
}

// One pixel of a dab, the only place a dab writes one: a is the dab's own coverage, ad its share
// once the spacing is compensated (what the deposit uses). An erase dab restores toward paper, the
// paper under the pixel (stamp_as reads it).
#[inline(always)]
fn put_px<const ERASE: bool>(t: &mut DabTarget, c: &mut Canvas, x: i32, y: i32, ad: f32, a: f32, paper: u8) {
    if !ERASE {
        canvas_deposit_px(c, x, y, t.ink, ad);
        return;
    }
    let at = (y * c.stride + x) as usize;
    let was = c.px[at];
    // Full coverage lands exactly on the paper: the spacing-compensated share alone would leave
    // the pixel one level short of it.
    canvas_restore_px(c, x, y, paper, if a >= 0.999 { 1.0 } else { ad });
    if c.px[at] != was {
        t.restored = true;
    }
}

// The erase fast path's part of a row: pixels x0..x1 of row y become the lighter of themselves
// and the paper, which is what put_px makes of a pixel the dab covers fully.
fn restore_span(t: &mut DabTarget, c: &mut Canvas, y: i32, x0: i32, x1: i32) {
    let at = (y * c.stride + x0) as usize;
    let n = (x1 - x0 + 1) as usize;
    let row = &mut c.px[at..at + n];
    let mut changed: u8 = 0;
    match t.paper {
        None => {
            for v in row.iter_mut() {
                changed |= !*v;
                *v = 255;
            }
        }
        Some(paper) => {
            let pat = ((y - t.y0) as isize * t.stride as isize + (x0 - t.x0) as isize) as usize;
            let pp = &paper[pat..pat + n];
            for (v, p) in row.iter_mut().zip(pp) {
                let m = if *v > *p { *v } else { *p };
                changed |= m ^ *v;
                *v = m;
            }
        }
    }
    if changed != 0 {
        t.restored = true;
    }
}

// The fast path covers dabs up to this core radius: there the margin it leaves (below) is over
// twenty times the float rounding of the per-pixel distance. The largest dab the parameter ranges
// allow at 300 dpi has a radius of 354 px.
const SPAN_MAX_R: f32 = 512.0;

// One dab. `erase` and `dry` are constants at each call (stamp), so each gets a copy of its own: the
// ink copy is the dab as it was before erase strokes, with no erase test left in its pixel loop.
// (Every `a * b + c` of the C that clang contracts is an fmla: PORTING.md, Floating point. The sites
// were read off the C build's IR, one by one.)
#[inline(always)]
fn stamp_as<const ERASE: bool, const DRY: bool>(b: &mut BrushState, c: &mut Canvas, s: &PenSample) -> Rect {
    let mut r = 0.0f32;
    let mut op = 0.0f32;
    brush_dab(b.tool, &b.params, s, b.dpi, &mut r, &mut op);
    let idx = b.dab_index;
    b.dab_index = b.dab_index.wrapping_add(1);
    let lk = look_of(b.tool, b.tip);
    // Calligraphy: the nib follows the pen's lean, lightly smoothed along the stroke (about
    // 1.5 mm), so sensor noise does not wobble the edge. Updated for every dab, inked or not.
    let mut nca = 0.7660444f32; // italic nib at 40 degrees (fountain pen)
    let mut nsa = 0.6427876f32;
    if lk.nib == NibKind::Vertical {
        nca = 0.0;
        nsa = 1.0; // fixed vertical edge, independent of the pen's lean
    } else if lk.nib == NibKind::BroadEdge {
        let (c2, s2) = nib_target(&b.params, s.tilt_x, s.tilt_y);
        if !b.nib_init {
            b.nib_init = true;
            b.nib_c = c2;
            b.nib_s = s2;
        } else {
            let k = 1.0 - (-spacing_for(r) / mm_to_px(1.5, b.dpi)).exp();
            let nc = fmla(c2 - b.nib_c, k, b.nib_c);
            let ns = fmla(s2 - b.nib_s, k, b.nib_s);
            let l = fmla(nc, nc, ns * ns).sqrt();
            if l > 1e-3 {
                b.nib_c = nc / l;
                b.nib_s = ns / l;
            } else {
                b.nib_c = c2;
                b.nib_s = s2;
            }
        }
        let a = 0.5 * b.nib_s.atan2(b.nib_c);
        nca = a.cos();
        nsa = a.sin();
    }
    if op <= 0.0 {
        return RECT_EMPTY;
    }
    let grain = sane(b.params.grain, 0.0, 1.0, 0.0);
    let pr = if grain > 0.0 { brush_pressure(&b.params, s.pressure) } else { 0.0 };
    if lk.jitter > 0.0 {
        let j = (hash2(b.seed, idx) & 0xFFFF) as f32 / 65535.0 - 0.5;
        op = clampf(op * fmla(2.0 * j * lk.jitter, grain, 1.0), 0.0, 1.0);
    }
    // Spacing compensation: about 2r/spacing dabs cover a pixel on the stroke's spine, so
    // each dab deposits the fraction that composes to `op` overall.
    // (A nib only overlaps itself across its narrow axis: its ratio of the round tip.)
    let nib_ratio = if lk.nib == NibKind::Italic { ITALIC_RATIO } else { BROAD_RATIO };
    let n = (2.0 * r * (if lk.nib != NibKind::None { nib_ratio } else { 1.0 }) / spacing_for(r)).max(1.0);
    let inv_n = 1.0 / n;
    let g_eff = grain * fmla(-0.5f32, pr, 1.0);
    let r_in = fmla(r, 1.0 - lk.rim, -0.5).max(0.0);
    let r_out = r + 0.5;
    // Graphite: light pressure only reaches the peaks of the tooth, pressing harder fills the
    // valleys; the stroke edge follows the tooth too, so it is ragged instead of a smooth tube.
    let thr = grain * fmla(-lk.fill, pr, 0.80);
    let edge_w = (r * lk.edge).max(0.9);
    let rough = r * lk.rough * grain;
    // Charcoal dust: loose specks up to this far past the edge, fixed to the paper.
    let dust_w = if lk.dust > 0.0 { r * 0.40 } else { 0.0 };
    let reach = if lk.graphite { (rough * 0.5).max(dust_w) } else { 0.0 };
    let box_ = rect_intersect(rect_from_points(s.x, s.y, s.x, s.y, r + 1.0 + reach), c.clip);
    if DRY {
        return box_; // where it would draw, and nothing drawn (BrushState.dry)
    }
    // (the paper is shared with whoever made it: a count taken for the dab, not a copy)
    let paper_rc = b.paper.clone();
    let mut tg = DabTarget { ink: lk.ink, paper: paper_rc.as_deref().map(|v| &v[..]), x0: b.paper_x0, y0: b.paper_y0, stride: b.paper_stride, restored: false };
    let seed = b.seed;
    let tooth: &[u8] = TOOTH.get_or_init(tooth_init);
    // A smooth dab's fully covered pixels all have the same opacity. Work out their
    // spacing compensation once; edges and textured pixels keep the exact formula below.
    let core_ad = if !lk.graphite && g_eff <= 0.0 { 1.0 - (1.0 - op.min(0.999)).powf(inv_n) } else { 0.0 };
    // Erase fast path: a round dab (no nib, not graphite) with no grain at full opacity gives
    // every pixel of its core full coverage, so a row's core is restored in one pass (restore_span)
    // and the per-pixel loop skips it. The span stops a pixel short of the core's edge at both
    // ends, on rows whose core is wider than 4 px: its pixels are then at squared distance below
    // r_in^2 - 3, more than 1.4 / r_in inside the core, where the per-pixel path's float distance
    // cannot reach r_in, so it would have given each of them alpha 1 too. Byte for byte the same
    // (tests/test_erase.c compares the two).
    let spans = ERASE
        && BRUSH_ERASE_SPANS.load(Ordering::Relaxed)
        && !lk.graphite
        && lk.nib == NibKind::None
        && g_eff <= 0.0
        && op >= 0.999
        && r_in <= SPAN_MAX_R;
    for y in box_.y..box_.y + box_.h {
        // Erase: the row, and the paper under it ((x, y) is inside the clip, stamp's box being
        // clipped to it, so the paper covers it).
        let row_at = y * c.stride;
        let prow: isize = if ERASE && tg.paper.is_some() { (y - tg.y0) as isize * tg.stride as isize } else { 0 };
        let dy = y as f32 + 0.5 - s.y;
        let mut sx0 = box_.x + box_.w; // the row's core span (none: x never gets there)
        let mut sx1 = sx0;
        let q = if spans { fmla(r_in, r_in, -(dy * dy)) } else { 0.0 };
        if q > 4.0 {
            let hw = q.sqrt();
            let mut a0 = (s.x - 0.5 - hw).ceil() as i32 + 1;
            let mut a1 = (s.x - 0.5 + hw).floor() as i32 - 1;
            if a0 < box_.x {
                a0 = box_.x;
            }
            if a1 > box_.x + box_.w - 1 {
                a1 = box_.x + box_.w - 1;
            }
            if a0 <= a1 {
                restore_span(&mut tg, c, y, a0, a1);
                sx0 = a0;
                sx1 = a1;
            }
        }
        // (the C moves x itself to hop over the span: here the hop is the next x to look at)
        let mut next_x = box_.x;
        for x in box_.x..box_.x + box_.w {
            if x < next_x {
                continue;
            }
            let mut paper: u8 = 255;
            if ERASE {
                if x == sx0 {
                    next_x = sx1 + 1;
                    continue;
                }
                // A pixel already at its paper (or lighter) stays as it is (canvas_restore_px), so
                // its coverage need not be worked out: most of a scrub goes over pixels its earlier
                // dabs restored.
                if let Some(pp) = tg.paper {
                    paper = pp[(prow + (x - tg.x0) as isize) as usize];
                }
                if c.px[(row_at + x) as usize] >= paper {
                    continue;
                }
            } else if c.px[(row_at + x) as usize] <= tg.ink {
                // Deposit only darkens toward the tool's ink. Existing black PDF marks and
                // fully inked pixels cannot change, whatever the dab's shape or texture.
                continue;
            }
            let dx = x as f32 + 0.5 - s.x;
            let mut d = fmla(dx, dx, dy * dy).sqrt();
            let a: f32;
            if lk.graphite {
                let t = if lk.coarse {
                    fmla(0.78f32, tooth_at(tooth, x >> 1, y >> 1), 0.22 * tooth_at(tooth, x, y))
                } else {
                    tooth_at(tooth, x, y)
                };
                let de = fmla(0.5 - t, rough, d); // peaks pick up graphite a little further out
                if de >= r_out {
                    if dust_w <= 0.0 || de >= r_out + dust_w {
                        continue;
                    }
                    // A speck where the paper catches it: rarer further from the edge.
                    let chance = lk.dust * (1.0 - (de - r_out) / dust_w);
                    let h = (hash2((x as u32).wrapping_mul(7).wrapping_add(seed), (y as u32).wrapping_mul(13).wrapping_add(0xD057)) & 0xFFFF) as f32 / 65535.0;
                    if h >= chance {
                        continue;
                    }
                    let a = op * 0.75 * fmla(0.5f32, t, 0.5);
                    let ad = 1.0 - (1.0 - a.min(0.999)).powf(inv_n);
                    put_px::<ERASE>(&mut tg, c, x, y, ad, a, paper);
                    continue;
                }
                let prof = clampf((r_out - de) / edge_w, 0.0, 1.0);
                let lo = thr - lk.band;
                let hi = thr + lk.band;
                let mut k = clampf((t - lo) / (hi - lo), 0.0, 1.0);
                k = (k * k) * fmla(-2.0f32, k, 3.0);
                a = op * prof * lerpf(1.0, k, grain);
            } else {
                if lk.nib == NibKind::Italic {
                    // Italic nib at 40 degrees, 4.5:1: the distance is measured in the nib's frame.
                    let u = fmla(dx, nca, dy * nsa);
                    let v = fmla(-dx, nsa, dy * nca) / ITALIC_RATIO;
                    d = fmla(u, u, v * v).sqrt();
                } else if lk.nib == NibKind::BroadEdge || lk.nib == NibKind::Vertical {
                    // A flat, square-cut nib (a superellipse): crisp angled ends like a real
                    // broad-edge pen, hairlines along the edge, full width across it.
                    let u = fmla(dx, nca, dy * nsa);
                    let v = fmla(-dx, nsa, dy * nca) / BROAD_RATIO;
                    let u2 = u * u;
                    let v2 = v * v;
                    d = fmla(u2, u2, v2 * v2).sqrt().sqrt();
                }
                if d >= r_out {
                    continue;
                }
                let mut prof = if d <= r_in { 1.0 } else { (r_out - d) / (r_out - r_in) };
                if lk.core > 0.0 {
                    prof = (prof * fmla(lk.core, 1.0 - d / r_out, 1.0)).min(1.0);
                }
                let mut aa = op * prof;
                if g_eff > 0.0 {
                    let v = 1.0 - tooth_at(tooth, x, y);
                    aa *= clampf(fmla(-(g_eff * 1.4 * v), v, 1.0), 0.0, 1.0);
                }
                a = aa;
            }
            if a <= 0.002 {
                continue;
            }
            let ad = if !lk.graphite && g_eff <= 0.0 && a == op { core_ad } else { 1.0 - (1.0 - a.min(0.999)).powf(inv_n) };
            put_px::<ERASE>(&mut tg, c, x, y, ad, a, paper);
        }
    }
    if tg.restored {
        b.restored = true;
    }
    box_
}

fn stamp(b: &mut BrushState, c: &mut Canvas, s: &PenSample) -> Rect {
    if b.dry {
        return stamp_as::<false, true>(b, c, s);
    }
    if b.erase {
        stamp_as::<true, false>(b, c, s)
    } else {
        stamp_as::<false, false>(b, c, s)
    }
}

fn interp(a: &PenSample, b: &PenSample, t: f32) -> PenSample {
    PenSample {
        x: lerpf(a.x, b.x, t),
        y: lerpf(a.y, b.y, t),
        pressure: lerpf(a.pressure, b.pressure, t),
        tilt_x: lerpf(a.tilt_x, b.tilt_x, t),
        tilt_y: lerpf(a.tilt_y, b.tilt_y, t),
        // In signed 64-bit: times read from a file may run backwards.
        t_ms: ((a.t_ms as i64) + (((b.t_ms as i64 - a.t_ms as i64) as f64) * t as f64) as i64) as u32,
    }
}

/// Input smoothing (exponential, distance-aware). amount = ToolParams.smoothing.
#[derive(Clone, Copy, Debug, Default)]
pub struct Smoother {
    pub init: bool,
    pub x: f32,
    pub y: f32,
    pub pressure: f32,
}
pub fn smoother_reset(s: &mut Smoother) {
    *s = Smoother::default();
}

pub fn smoother_apply(s: &mut Smoother, amount: f32, io: &mut PenSample) {
    let amount = clampf(amount, 0.0, 1.0);
    if !s.init || amount <= 0.0 {
        s.init = true;
        s.x = io.x;
        s.y = io.y;
        s.pressure = io.pressure;
        return;
    }
    let dx = io.x - s.x;
    let dy = io.y - s.y;
    let dist = fmla(dx, dx, dy * dy).sqrt();
    // Strong smoothing for slow, careful motion; quick strokes follow closely.
    let alpha = clampf(fmla(-0.85f32, amount, 1.0) + dist / 60.0, 0.08, 1.0);
    s.x = fmla(dx, alpha, s.x);
    s.y = fmla(dy, alpha, s.y);
    s.pressure = fmla(io.pressure - s.pressure, alpha.max(0.5), s.pressure);
    io.x = s.x;
    io.y = s.y;
    io.pressure = s.pressure;
}
