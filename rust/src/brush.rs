//! brush.h: drawing tools, their tweakable sensitivity, and the deterministic dab renderer.

use crate::prelude::*;

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
pub fn tool_name(t: ToolKind) -> &'static str {
    todo!()
}
/// the same in the language in use, for the screens
pub fn tool_label(t: ToolKind) -> &'static str {
    todo!()
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

pub fn tool_param_table() -> &'static [ParamInfo] {
    todo!()
}
pub fn tool_params_default(t: ToolKind, out: &mut ToolParams) {
    todo!()
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
    todo!()
}
/// -> 0..1
pub fn brush_tilt(p: &ToolParams, tilt_x: f32, tilt_y: f32) -> f32 {
    todo!()
}
/// Dab radius (px) and opacity (0..1) for a sample.
pub fn brush_dab(t: ToolKind, p: &ToolParams, s: &PenSample, dpi: f32, radius_px: &mut f32, opacity: &mut f32) {
    todo!()
}
/// Largest radius any sample of this tool with these params can produce (for bbox padding).
pub fn brush_max_radius(t: ToolKind, p: &ToolParams, dpi: f32) -> f32 {
    todo!()
}
/// Calligraphy: the nib's pen angle for a pen tilted (tilt_x, tilt_y), in degrees above the
/// horizontal (45 = the edge rises to the right at 45 degrees). The broad edge sits at 90
/// degrees to the pen's lean; a pen held nearly upright gets the default 45 (leaning bottom right).
pub fn brush_nib_angle(p: &ToolParams, tilt_x: f32, tilt_y: f32) -> f32 {
    todo!()
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
    pub paper: Option<std::rc::Rc<[u8]>>,
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
    todo!()
}
/// Whether this version knows tip variant v of tool t (it draws an unknown one with the current tip).
pub fn brush_tip_known(t: ToolKind, v: u8) -> bool {
    todo!()
}

pub fn brush_begin(b: &mut BrushState, t: ToolKind, p: &ToolParams, seed: u32, dpi: f32) {
    todo!()
}
/// Puts b (just begun) in erase mode, for an erase stroke: white ink with the tool's texture, or
/// the precise eraser's round hard tip (TOOL_ERASER). Its dabs lighten each pixel toward the paper
/// under it, never past it, and a pixel the dab covers fully lands exactly on the paper. The paper
/// under canvas pixel (x, y) is paper[(y - y0) * stride + (x - x0)], and it must cover every pixel
/// of the canvas's clip while b draws (the dabs stay inside the clip). NULL restores toward white:
/// right for a canvas of ink alone, wrong over paper with lines (it would wipe them).
pub fn brush_set_erase(b: &mut BrushState, paper: Option<std::rc::Rc<[u8]>>, x0: i32, y0: i32, stride: i32) {
    todo!()
}
/// The erase fast path (on unless cleared): a round dab with no grain at full opacity restores the
/// core of each row it covers in one pass. Its pixels are the per-pixel path's, byte for byte;
/// tests clear it to prove that, and measurements to time both.
pub static BRUSH_ERASE_SPANS: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);
/// Renders dabs from the previous sample to s (the first call stamps one dab at s).
/// Returns the dirty rect touched (may be empty). A TOOL_ERASER brush draws only in erase mode.
pub fn brush_add(b: &mut BrushState, c: &mut Canvas, s: &PenSample) -> Rect {
    todo!()
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
    todo!()
}
pub fn smoother_apply(s: &mut Smoother, amount: f32, io: &mut PenSample) {
    todo!()
}
