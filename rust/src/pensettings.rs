//! pensettings.c: per-tool pressure / tilt sensitivity. Tabs per tool, one slider per
//! tweakable parameter, a live pressure curve, a raw-vs-mapped readout, and a test pad that
//! draws with the tool being tuned.

use crate::prelude::*;
use std::f32::consts::PI;
use std::rc::Rc;

const H_BACK: i32 = 1;
const H_TAB: i32 = 2;
const H_RESET: i32 = 3;
const H_CLEAR: i32 = 4;
const H_SLIDER: i32 = 5;
const H_PAD: i32 = 6;
const H_ERASER_MODE: i32 = 7;

/// The screen's state (pensettings.c's, which app.h keeps opaque): pensettings_init makes it, in a.ps.
/// Its fields are the translator's, from the C struct, by the rules of PORTING.md.
#[derive(Default)]
pub struct PenSetState {
    pub tab: ToolKind,
    pub ret: ScreenId,
    pub hits: HitList,
    pub press: Press,
    pub pressed: Hit,
    pub has_pressed: bool,
    // slider drag
    /// param table index being dragged, -1 none
    pub active: i32,
    /// (unused in the C too)
    pub sl: Slider,
    pub slider_drag: SliderDrag,
    pub slider_dirty: bool,
    pub last_paint: u32,
    // test pad
    pub pad: Rect,
    pub graph: Rect,
    pub readout: Rect,
    pub drawing: bool,
    pub pen_owner: bool,
    pub bs: BrushState,
    pub sm: Smoother,
    // The precise eraser (eraser_mode 1) and white ink (white) on the pad: erase strokes, as in the
    // editor, restoring the pad as it was drawn bare (its lines and its Clear button), copied here
    // when it is. White on that shows nothing: the pad then has a black swatch to try it on.
    pub paper: Canvas,
    /// empty when there is none
    pub swatch: Rect,
    /// owned copy of the handwritten practice ink
    pub sample: Option<Box<Page>>,
    pub sample_box: Rect,
    pub wipe_x: f32,
    pub wipe_y: f32,
    /// the stroke on the pad is the precise eraser's
    pub erasing: bool,
    /// the stroke on the pad is white ink
    pub whiting: bool,
    /// whole-stroke eraser: hits practice ink on its tab, clears the other pads
    pub wiping: bool,
    /// last pen pressure seen on the pad (-1 none)
    pub live_raw: f32,
    pub live_tx: f32,
    pub live_ty: f32,
    pub last_readout: u32,
    pub slider_rows: [Rect; 16],
    pub slider_param: [i32; 16],
    pub nsliders: i32,
}

/// The screen's state (the C's `S`, a->ps).
fn st(a: &mut App) -> &mut PenSetState {
    a.ps.as_deref_mut().expect("Pen feel's state: pensettings_init has not run")
}

fn visible_param(pi: &ParamInfo, t: ToolKind) -> bool {
    t != TOOL_ERASER || pi.eraser_uses
}

fn value_text(pi: &ParamInfo, v: f32, buf: &mut String, cap: usize) {
    if pi.unit == "deg" {
        // TRANSLATORS: an angle on Pen feel's sliders and in its readout ("45 deg"); "%d°" is fine.
        csnprintf!(buf, cap, tr("%d deg"), v.round_ties_even() as i32);
    } else if pi.unit == "mm" {
        csnprintf!(buf, cap, "%.2f mm", v);
    } else if pi.unit == "x" {
        // TRANSLATORS: the value of Pen feel's Tilt widens slider: how much wider the line gets with the
        // pen on its side, a percentage ("+35%"; in Spanish "+35 %", the usual spacing), up
        // to +400%. Right of its slider's name, at 44 px in bold: the two share 740 px, the value takes
        // what it needs (about 150 px for "+400 %" in Gothic, 120 in Classic) and the name the rest.
        csnprintf!(buf, cap, tr("+%.0f%%"), v * 100.0);
    } else {
        csnprintf!(buf, cap, "%.2f", v);
    }
}

fn draw_graph(a: &mut App) {
    let App { screen: c, settings, ps, .. } = &mut *a;
    let s = ps.as_deref().expect("Pen feel's state");
    let g = s.graph;
    let p = &settings.tools[s.tab.0 as usize];
    // Clear enough margin for the live dot (it can sit on the graph's edge).
    canvas_fill(c, rect_inflate(g, ui_px(26.0)), 255);
    canvas_rect_outline(c, g, ui_px(3.0), 0);
    // Quarter grid.
    for i in 1..4 {
        canvas_hline(c, g.x, g.x + g.w, g.y + g.h * i / 4, 1, 200);
        canvas_vline(c, g.x + g.w * i / 4, g.y, g.y + g.h, 1, 200);
    }
    // Deadzone + "full at" markers.
    let fx = g.x + (p.pressure_floor * g.w as f32) as i32;
    let cx = g.x + (p.pressure_ceiling * g.w as f32) as i32;
    let mut y = g.y;
    while y < g.y + g.h {
        canvas_vline(c, fx, y, y + ui_px(12.0), ui_px(3.0), 120);
        canvas_vline(c, cx, y, y + ui_px(12.0), ui_px(3.0), 120);
        y += ui_px(24.0);
    }
    let (mut px, mut py) = (0.0f32, 0.0f32);
    for i in 0..=64 {
        let x = i as f32 / 64.0;
        let y = brush_pressure(p, x);
        // (g.x + x * g.w and g.y + g.h - y * g.h: the product is contracted with the sum, see fmla)
        let sx = fmla(x, g.w as f32, g.x as f32);
        let sy = fmla(-y, g.h as f32, (g.y + g.h) as f32);
        if i != 0 {
            canvas_line_aa(c, px, py, sx, sy, ui_px(6.0) as f32, 0);
        }
        px = sx;
        py = sy;
    }
    if s.live_raw >= 0.0 {
        let y = brush_pressure(p, s.live_raw);
        canvas_disc_aa(c, fmla(s.live_raw, g.w as f32, g.x as f32), fmla(-y, g.h as f32, (g.y + g.h) as f32), ui_px(18.0) as f32, 0, 1.0);
        canvas_disc_aa(c, fmla(s.live_raw, g.w as f32, g.x as f32), fmla(-y, g.h as f32, (g.y + g.h) as f32), ui_px(8.0) as f32, 255, 1.0);
    }
    let ss = ui_small_size();
    // TRANSLATORS: the axes of Pen feel's pressure graph: under it, how hard the pen presses; over
    // it, how much ink that puts down. Up to 570 px at 34 px (the test pad starts there).
    font_draw(c, FontFace::Regular, ss, g.x, g.y + g.h + ui_px(44.0), tr("pen pressure"), 60);
    // TRANSLATORS: over Pen feel's pressure graph (see pen pressure): how much ink.
    font_draw(c, FontFace::Regular, ss, g.x, g.y - ui_px(14.0), tr("ink"), 60);
}

fn draw_readout(a: &mut App) {
    let dpi = a.dpi;
    let App { screen: c, settings, ps, .. } = &mut *a;
    let s = ps.as_deref().expect("Pen feel's state");
    canvas_fill(c, s.readout, 255);
    let p = &settings.tools[s.tab.0 as usize];
    let mut line = String::new();
    if s.live_raw < 0.0 {
        // TRANSLATORS: Pen feel's readout line, until the pen draws on the test pad (1760 px at
        // 34 px; the readings below take that room too).
        csnprintf!(&mut line, 200, "%s", tr("Draw on the pad to see live pressure and tilt."));
    } else {
        let smp = PenSample { x: 0.0, y: 0.0, pressure: s.live_raw, tilt_x: s.live_tx, tilt_y: s.live_ty, t_ms: 0 };
        let (mut r, mut op) = (0.0f32, 0.0f32);
        brush_dab(s.tab, p, &smp, dpi, &mut r, &mut op);
        let tilt_deg = (((s.live_tx * PI / 180.0).tan().powf(2.0) + (s.live_ty * PI / 180.0).tan().powf(2.0)).sqrt()).atan() * 180.0 / PI;
        let caps = input_caps();
        let mut tilt = String::new();
        if !caps.has_tilt {
            // TRANSLATORS: the tilt in Pen feel's readout (below), from a pen that does not report it.
            csnprintf!(&mut tilt, 96, "%s", tr("tilt: not reported"));
        } else {
            // TRANSLATORS: the tilt in Pen feel's readout (below): the pen's lean, and what it counts
            // for (0 to 1).
            csnprintf!(&mut tilt, 96, tr("tilt %d deg -> %.2f"), tilt_deg.round_ties_even() as i32, brush_tilt(p, s.live_tx, s.live_ty));
        }
        if s.tab == TOOL_CALLIGRAPHY {
            // TRANSLATORS: Pen feel's readout while the calligraphy pen draws on the test pad, two
            // spaces apart: the raw pressure and what the curve makes of it, the tilt (above), the
            // nib's width and angle (1760 px at 34 px).
            csnprintf!(&mut line, 200, tr("pressure %.2f -> %.2f  %s  nib %.2f mm at %d deg"), s.live_raw, brush_pressure(p, s.live_raw), &tilt, r * 2.0 * 25.4 / dpi, brush_nib_angle(p, s.live_tx, s.live_ty).round_ties_even() as i32);
        } else {
            // TRANSLATORS: Pen feel's readout while a tool draws on the test pad, two spaces apart:
            // the raw pressure and what the curve makes of it, the tilt (above), the line's width
            // and its ink (0 to 1) (1760 px at 34 px).
            csnprintf!(&mut line, 200, tr("pressure %.2f -> %.2f  %s  width %.2f mm  ink %.2f"), s.live_raw, brush_pressure(p, s.live_raw), &tilt, r * 2.0 * 25.4 / dpi, op);
        }
    }
    font_draw_left(c, FontFace::Regular, ui_small_size(), s.readout, &line, 0);
}

fn draw_slider_row(a: &mut App, i: usize) {
    let App { screen, settings, ps, .. } = &mut *a;
    let s = ps.as_deref().expect("Pen feel's state");
    let p = &tool_param_table()[s.slider_param[i] as usize];
    let mut sl = Slider { min: p.min, max: p.max, step: p.step, label: tr(p.label), unit: p.unit, ..Default::default() };
    // The calligraphy nib turns with the pen's lean once the pen tilts this far.
    // TRANSLATORS: the calligraphy pen's names for "Tilt starts at" and "Tilt full at" (570 px at
    // 44 px in Gothic, 600 in Classic, beside an angle).
    if s.tab == TOOL_CALLIGRAPHY && p.key == "tilt_min_deg" { sl.label = tr("Nib follows lean from"); }
    if s.tab == TOOL_CALLIGRAPHY && p.key == "tilt_max_deg" { sl.label = tr("Nib fully follows at"); }
    sl.value = settings.tools[s.tab.0 as usize].get(p.field);
    ui_slider_layout(&mut sl, s.slider_rows[i]);
    let mut vt = String::new();
    value_text(p, sl.value, &mut vt, 32);
    ui_slider_draw(screen, &sl, Some(&vt));
}

fn clear_rect(pad: Rect) -> Rect {
    Rect { x: pad.x + pad.w - ui_px(220.0), y: pad.y + ui_px(14.0), w: ui_px(206.0), h: ui_px(100.0) }
}

// White ink on the pad: while the drawing tools draw white (settings.white), the pen draws white
// on a drawing tool's tab, with its settings and texture, as in the editor.
fn pad_white(a: &mut App) -> bool {
    let tab = st(a).tab;
    a.settings.white && tab != TOOL_ERASER
}

fn pad_targets_free(a: &mut App) {
    let s = st(a);
    s.sample = None;
    s.sample_box = RECT_EMPTY;
}

// The pad's bare paper as the brush reads it: the C hands the brush the canvas's pixels, the
// brushes here keep a shared copy (PORTING.md, Ownership). None without a copy.
fn paper_px(paper: &Canvas) -> Option<Rc<Vec<u8>>> {
    if paper.px.is_empty() {
        None
    } else {
        Some(Rc::new(paper.px.clone()))
    }
}

fn pad_targets_draw(a: &mut App) {
    let dpi = a.dpi;
    let App { screen, ps, .. } = &mut *a;
    let s = ps.as_deref().expect("Pen feel's state");
    let Some(sample) = s.sample.as_deref() else {
        return;
    };
    let saved = screen.clip;
    canvas_set_clip(screen, s.pad);
    let paper = paper_px(&s.paper);
    for stroke in sample.strokes.iter() {
        let mut b = BrushState::default();
        stroke_brush_begin(&mut b, stroke, dpi, paper.clone(), s.pad.x, s.pad.y, s.paper.stride);
        for pt in stroke.pts.iter() {
            brush_add(&mut b, screen, pt);
        }
    }
    screen.clip = saved;
}

fn pad_targets_seed(a: &mut App) {
    let dpi = a.dpi;
    let mut p: Option<Box<Page>> = None;
    if page_deserialize(GRIMOIRE_ERASER_SAMPLE, &mut p) != 0 || p.as_ref().is_none_or(|p| p.readonly) {
        return;
    }
    let mut p = p.unwrap();
    // The source was written at 300 dpi. Fit its ink, without its page's empty margins or paper.
    let mut ink = RECT_EMPTY;
    for s in p.strokes.iter() {
        let r = brush_max_radius(stroke_render_tool(s), &s.params, 300.0) + 2.0;
        for pt in s.pts.iter() {
            ink = rect_union(ink, rect_from_points(pt.x, pt.y, pt.x, pt.y, r));
        }
    }
    if rect_is_empty(ink) {
        return;
    }
    let pad = st(a).pad;
    let area = Rect { x: pad.x + ui_px(30.0), y: pad.y + ui_px(130.0), w: pad.w - ui_px(60.0), h: pad.h - ui_px(260.0) };
    let scale = (area.w as f32 / ink.w as f32).min(area.h as f32 / ink.h as f32);
    // (area.w - ink.w * scale: the product is contracted with the subtraction, see fmla)
    let x = area.x as f32 + fmla(-(ink.w as f32), scale, area.w as f32) / 2.0;
    let y = area.y as f32 + fmla(-(ink.h as f32), scale, area.h as f32) / 2.0;
    st(a).sample_box = Rect {
        x: x.round_ties_even() as i32,
        y: y.round_ties_even() as i32,
        w: (ink.w as f32 * scale).round_ties_even() as i32,
        h: (ink.h as f32 * scale).round_ties_even() as i32,
    };
    for s in p.strokes.iter_mut() {
        let n = s.pts.len();
        s.params.size_min_mm *= scale * 300.0 / dpi;
        s.params.size_max_mm *= scale * 300.0 / dpi;
        let old = std::mem::replace(&mut s.pts, Vec::with_capacity(n)); // (the C starts over in the same array)
        s.bbox = RECT_EMPTY;
        for mut q in old {
            q.x = fmla(q.x - ink.x as f32, scale, x);
            q.y = fmla(q.y - ink.y as f32, scale, y);
            stroke_append(s, &q, dpi);
        }
    }
    st(a).sample = Some(p);
    pad_targets_draw(a);
}

// With the precise eraser or white ink, the pad as just drawn bare is its paper: they restore it,
// lines and Clear button, as they restore a page's paper in the editor.
fn pad_paper_take(a: &mut App) {
    let white = pad_white(a);
    let eraser_mode = a.settings.eraser_mode;
    let App { screen, ps, .. } = &mut *a;
    let s = ps.as_deref_mut().expect("Pen feel's state");
    if eraser_mode != 1 && !white && s.tab != TOOL_ERASER {
        canvas_free(&mut s.paper);
        return;
    }
    if !s.paper.px.is_empty() && (s.paper.w != s.pad.w || s.paper.h != s.pad.h) {
        canvas_free(&mut s.paper);
    }
    if s.paper.px.is_empty() && canvas_init(&mut s.paper, s.pad.w, s.pad.h) != 0 {
        return;
    }
    canvas_blit(&mut s.paper, screen, s.pad, 0, 0);
}

fn draw_pad_bare(a: &mut App) {
    let dpi = a.dpi;
    let App { screen: c, ps, .. } = &mut *a;
    let s = ps.as_deref().expect("Pen feel's state");
    canvas_fill(c, s.pad, 255);
    // A few ruled lines so stroke widths read like they will on paper.
    let pitch = mm_to_px(7.14, dpi);
    let mut y = s.pad.y as f32 + pitch;
    while y < (s.pad.y + s.pad.h - 4) as f32 {
        canvas_hline(c, s.pad.x, s.pad.x + s.pad.w, y as i32, ui_px(2.0), 185);
        y += pitch;
    }
    ui_frame(c, rect_inflate(s.pad, ui_px(4.0)), ui_px(14.0), 0, false, ui_px(4.0));
    // TRANSLATORS: the button that wipes Pen feel's test pad (174 px at 44 px, or 34 px).
    // TRANSLATORS: on the eraser tab, restore the handwritten practice ink (174 px at 34 px).
    ui_button(c, clear_rect(s.pad), IconKind::None, if s.tab == TOOL_ERASER { tr("Reset pad") } else { tr("Clear") }, ButtonStyle::Plain);
}

// Capture the bare paper before seeding the eraser targets or white-ink swatch as ink.
// For white ink, a black swatch lies
// under the Clear button's row (white on the bare pad shows nothing: on the swatch it shows the
// tool's look, its grain and pressure, and the pad's lines it brings back).
fn draw_pad_frame(a: &mut App) {
    pad_targets_free(a);
    draw_pad_bare(a);
    pad_paper_take(a);
    st(a).swatch = RECT_EMPTY;
    let white = pad_white(a);
    let App { screen, ps, .. } = &mut *a;
    let s = ps.as_deref_mut().expect("Pen feel's state");
    if s.tab == TOOL_ERASER {
        pad_targets_seed(a);
    } else if white && !s.paper.px.is_empty() {
        s.swatch = Rect { x: s.pad.x + ui_px(30.0), y: s.pad.y + ui_px(130.0), w: s.pad.w - ui_px(60.0), h: s.pad.h - ui_px(160.0) };
        canvas_fill(screen, s.swatch, 0);
    }
}

fn draw_all(a: &mut App) {
    let (aw, ah) = (a.w, a.h);
    hits_clear(&mut st(a).hits);
    canvas_fill(&mut a.screen, Rect { x: 0, y: 0, w: aw, h: ah }, 255);
    let m = ui_px(50.0);
    let th = ui_px(190.0);
    let bh = ui_px(150.0);
    let by = (th - bh) / 2;
    let back = Rect { x: m, y: by, w: bh, h: bh };
    let reset = Rect { x: aw - m - ui_px(360.0), y: by, w: ui_px(360.0), h: bh };
    let tw = (aw - 2 * m - (TOOL_COUNT as i32 - 1) * ui_px(16.0)) / TOOL_COUNT as i32;
    let ty = th + ui_px(24.0);
    let tabh = ui_px(150.0);
    let top = ty + tabh + ui_px(70.0);
    let gs = ui_px(500.0);
    {
        let App { screen: c, ps, .. } = &mut *a;
        let s = ps.as_deref_mut().expect("Pen feel's state");
        ui_button(c, back, IconKind::Back, "", ButtonStyle::Plain);
        hits_add(&mut s.hits, back, H_BACK, 0);
        // TRANSLATORS: puts the tool's pen feel back as it came (328 px at 44 px, or 34 px).
        ui_button(c, reset, IconKind::None, tr("Reset tool"), ButtonStyle::Plain);
        hits_add(&mut s.hits, reset, H_RESET, 0);
        let mut title = String::new();
        // TRANSLATORS: the title of the screen where each tool's pressure and tilt are tuned, with the
        // tool's name (1200 px at 70 px).
        csnprintf!(&mut title, 128, tr("Pen feel: %s"), tool_label(s.tab));
        font_draw_left(
            c,
            FontFace::Bold,
            ui_title_size() * 1.1,
            Rect { x: back.x + bh + ui_px(30.0), y: 0, w: reset.x - back.x - bh - ui_px(50.0), h: th },
            &title,
            0,
        );
        ui_rule(c, 0, aw, th, ui_px(4.0), 0);
        // Tabs.
        for (i, &t) in TOOL_ORDER.iter().enumerate() {
            let r = Rect { x: m + i as i32 * (tw + ui_px(16.0)), y: ty, w: tw, h: tabh };
            // With this many tools the tabs are icons only; the title names the selected one.
            ui_button(c, r, ui_tool_icon(t.0), "", if s.tab == t { ButtonStyle::Selected } else { ButtonStyle::Plain });
            hits_add(&mut s.hits, r, H_TAB, t.0);
        }
        // Graph + pad.
        s.graph = Rect { x: m + ui_px(10.0), y: top, w: gs, h: gs };
        s.pad = Rect { x: s.graph.x + gs + ui_px(70.0), y: top, w: aw - m - (s.graph.x + gs + ui_px(70.0)), h: gs };
        s.readout = Rect { x: m, y: top + gs + ui_px(70.0), w: aw - 2 * m, h: ui_px(70.0) };
    }
    draw_graph(a);
    draw_pad_frame(a);
    {
        let s = st(a);
        hits_add(&mut s.hits, s.pad, H_PAD, 0);
        hits_add(&mut s.hits, clear_rect(s.pad), H_CLEAR, 0);
    }
    // TRANSLATORS: over Pen feel's test pad, which the pen draws on with the tool (1180 px at 34 px).
    let mut pad_label = tr("test pad");
    // TRANSLATORS: the same while the pens draw white ink, which shows only over ink: the pad then
    // has a black part to try it on (1180 px at 34 px).
    if pad_white(a) { pad_label = tr("test pad (white ink: try it on the black)"); }
    let pad = st(a).pad;
    font_draw(&mut a.screen, FontFace::Regular, ui_small_size(), pad.x, pad.y - ui_px(14.0), pad_label, 60);
    draw_readout(a);
    // Sliders.
    let pi = tool_param_table();
    let tab = st(a).tab;
    let mut y = st(a).readout.y + st(a).readout.h + ui_px(30.0);
    canvas_hline(&mut a.screen, m, aw - m, y, ui_px(3.0), 0);
    y += ui_px(16.0);
    let avail = ah - y - ui_px(20.0);
    let mut count = 0;
    for p in pi.iter() {
        count += visible_param(p, tab) as i32;
    }
    let mut rh = if count != 0 { avail / count } else { avail };
    if rh > ui_px(118.0) {
        rh = ui_px(118.0);
    }
    st(a).nsliders = 0;
    let mut i = 0;
    while i < pi.len() && st(a).nsliders < 16 {
        if !visible_param(&pi[i], tab) {
            i += 1;
            continue;
        }
        let ns = st(a).nsliders;
        {
            let s = st(a);
            s.slider_rows[ns as usize] = Rect { x: m, y, w: aw - 2 * m, h: rh };
            s.slider_param[ns as usize] = i as i32;
        }
        draw_slider_row(a, ns as usize);
        {
            let s = st(a);
            hits_add(&mut s.hits, s.slider_rows[ns as usize], H_SLIDER, ns);
            s.nsliders += 1;
        }
        y += rh;
        i += 1;
    }
    if tab != TOOL_ERASER {
        return;
    }
    // What the eraser does (settings.ini eraser_mode; its button in the editor, the pen's eraser
    // end, the pen's button set to erase): two choices right of the row's name, where a slider row
    // has its name and its track.
    let eraser_mode = a.settings.eraser_mode;
    let App { screen: c, ps, .. } = &mut *a;
    let s = ps.as_deref_mut().expect("Pen feel's state");
    let row = Rect { x: m, y: y + ui_px(24.0), w: aw - 2 * m, h: ui_px(120.0) };
    let name_w = row.w * 42 / 100;
    let g = ui_px(20.0);
    let cw = (row.w - name_w - g) / 2;
    // TRANSLATORS: Pen feel's eraser tab: the name of the row that picks how the eraser works, left
    // of its two choices, as a slider's name is left of it (720 px at 44 px, or two lines at 34 px).
    let name = tr("Eraser:");
    ui_text(c, FontFace::Regular, ui_text_size(), ui_small_size(), Rect { x: row.x, y: row.y, w: name_w - ui_px(16.0), h: row.h }, name, 2, false, 0);
    const MODES: [&str; 2] = [
        // TRANSLATORS: the first choice of Pen feel's Eraser row: the eraser takes every stroke it
        // touches away whole (as before 1.1). 468 px at 44 px, or two lines at 34 px.
        nc_("eraser", "Whole strokes"),
        // TRANSLATORS: the second choice of Pen feel's Eraser row: the eraser takes away only what
        // it touches, as white ink does, keeping the paper's lines (468 px at 44 px).
        nc_("eraser", "Precise"),
    ];
    for i in 0..2 {
        let r = Rect { x: row.x + name_w + i * (cw + g), y: row.y, w: cw, h: row.h };
        ui_button(c, r, IconKind::None, c_("eraser", MODES[i as usize]), if eraser_mode == i { ButtonStyle::Selected } else { ButtonStyle::Plain });
        hits_add(&mut s.hits, r, H_ERASER_MODE, i);
    }
    y = row.y + row.h;
    let mut note = tr("The eraser removes whole strokes; harder pressure = bigger reach.");
    // TRANSLATORS: under Pen feel's Eraser row while the precise eraser is picked (1760 px at 44 px).
    if eraser_mode == 1 { note = tr("The eraser takes away only what it touches; harder pressure = bigger reach."); }
    font_draw_left(c, FontFace::Regular, ui_text_size(), Rect { x: m, y: y + ui_px(30.0), w: aw - 2 * m, h: ui_px(80.0) }, note, 60);
}

/// switch to SCR_PENSET (returns to prev_scr)
pub fn pensettings_open(a: &mut App, tab: ToolKind) {
    let scr = a.scr;
    let s = st(a);
    s.tab = tab;
    s.ret = scr;
    app_switch(a, ScreenId::Penset);
}

pub fn pensettings_init(a: &mut App) {
    let mut s = Box::<PenSetState>::default();
    s.active = -1;
    s.live_raw = -1.0;
    a.ps = Some(s);
}
pub fn pensettings_free(a: &mut App) {
    if a.ps.is_some() {
        pad_targets_free(a);
        canvas_free(&mut st(a).paper);
    }
    a.ps = None;
}
pub fn pensettings_enter(a: &mut App) {
    {
        let s = st(a);
        s.active = -1;
        s.slider_dirty = false;
        s.drawing = false;
        s.erasing = false;
        s.whiting = false;
        s.wiping = false;
        s.has_pressed = false;
        s.live_raw = -1.0;
    }
    draw_all(a);
}
pub fn pensettings_leave(a: &mut App) {
    if st(a).drawing {
        display_live_ink(false); // left in the middle of a stroke on the pad
    }
    {
        let s = st(a);
        s.drawing = false;
        s.erasing = false;
        s.whiting = false;
        s.wiping = false;
        canvas_free(&mut s.paper);
    }
    pad_targets_free(a);
    st(a).swatch = RECT_EMPTY;
    settings_save(&a.settings);
}

fn repaint_sliders(a: &mut App, m: RefreshMode) {
    let n = st(a).nsliders;
    for i in 0..n {
        draw_slider_row(a, i as usize);
    }
    draw_graph(a);
    draw_readout(a);
    let s = st(a);
    let dirty = rect_union(rect_union(s.slider_rows[0], s.slider_rows[(s.nsliders - 1) as usize]), rect_inflate(s.graph, ui_px(70.0)));
    let r = rect_union(dirty, s.readout);
    app_invalidate(a, r, m);
    st(a).slider_dirty = false;
}

fn set_slider(a: &mut App, row: i32, ev: &InEvent, first: bool, m: RefreshMode) {
    {
        let App { settings, ps, .. } = &mut *a;
        let s = ps.as_deref_mut().expect("Pen feel's state");
        let p = &tool_param_table()[s.slider_param[row as usize] as usize];
        let mut sl = Slider { min: p.min, max: p.max, step: p.step, ..Default::default() };
        sl.value = settings.tools[s.tab.0 as usize].get(p.field);
        ui_slider_layout(&mut sl, s.slider_rows[row as usize]);
        let changed;
        if first {
            changed = ui_slider_set_from_x(&mut sl, ev.x);
            ui_slider_drag_begin(&sl, &mut s.slider_drag, ev.x, ev.t_ms);
        } else {
            changed = ui_slider_drag_to(&mut sl, &mut s.slider_drag, ev.x, ev.y, ev.t_ms);
        }
        s.slider_dirty |= changed;
        if !changed && !s.slider_dirty && m == RefreshMode::Fast {
            return;
        }
        let tp = &mut settings.tools[s.tab.0 as usize];
        *tp.get_mut(p.field) = sl.value;
        // Keep min <= max pairs sensible. (The C compares the slider's value pointer with the pair's
        // fields: p.field is that field's index in ToolParams.)
        if tp.size_min_mm > tp.size_max_mm {
            if p.field == 0 {
                tp.size_max_mm = tp.size_min_mm;
            } else {
                tp.size_min_mm = tp.size_max_mm;
            }
        }
        if tp.pressure_floor > tp.pressure_ceiling - 0.05 {
            if p.field == 4 {
                tp.pressure_ceiling = (1.0f32).min(tp.pressure_floor + 0.05);
            } else {
                tp.pressure_floor = (0.0f32).max(tp.pressure_ceiling - 0.05);
            }
        }
        if tp.tilt_min_deg > tp.tilt_max_deg - 1.0 {
            if p.field == 7 {
                tp.tilt_max_deg = (90.0f32).min(tp.tilt_min_deg + 1.0);
            } else {
                tp.tilt_min_deg = (0.0f32).max(tp.tilt_max_deg - 1.0);
            }
        }
        // Process every motion for fractional accuracy, but keep the existing display throttle.
        if !first && m == RefreshMode::Fast && ev.t_ms.wrapping_sub(s.last_paint) < 80 {
            return;
        }
        s.last_paint = ev.t_ms;
    }
    // Paired sliders may have moved too: repaint every row (cheap) + graph + readout.
    repaint_sliders(a, m);
}

fn pad_erase_targets(a: &mut App, smp: &PenSample, first: bool) {
    let dpi = a.dpi;
    let (x0, y0);
    {
        let s = st(a);
        x0 = if first { smp.x } else { s.wipe_x };
        y0 = if first { smp.y } else { s.wipe_y };
    }
    let (mut r, mut opacity) = (0.0f32, 0.0f32);
    brush_dab(TOOL_ERASER, &a.settings.tools[TOOL_ERASER.0 as usize], smp, dpi, &mut r, &mut opacity);
    let mut changed = false;
    let s = st(a);
    if let Some(sample) = s.sample.as_deref_mut() {
        let mut i = sample.strokes.len() as i32 - 1;
        while i >= 0 {
            if stroke_hit_segment(&sample.strokes[i as usize], x0, y0, smp.x, smp.y, r, dpi) {
                stroke_free(page_remove(sample, i));
                changed = true;
            }
            i -= 1;
        }
    }
    s.wipe_x = smp.x;
    s.wipe_y = smp.y;
    if changed {
        draw_pad_bare(a);
        pad_targets_draw(a);
        let pad = st(a).pad;
        app_invalidate(a, pad, RefreshMode::Fast);
    }
}

fn pad_sample(a: &mut App, ev: &InEvent, first: bool) {
    {
        let s = st(a);
        s.live_raw = ev.pressure;
        s.live_tx = ev.tilt_x;
        s.live_ty = ev.tilt_y;
    }
    let mut smp = PenSample { x: ev.x, y: ev.y, pressure: ev.pressure, tilt_x: ev.tilt_x, tilt_y: ev.tilt_y, t_ms: ev.t_ms };
    if st(a).wiping {
        if st(a).tab == TOOL_ERASER {
            pad_erase_targets(a, &smp, first);
        }
    } else {
        let dpi = a.dpi;
        let d;
        let pad;
        {
            let App { screen, settings, ps, .. } = &mut *a;
            let s = ps.as_deref_mut().expect("Pen feel's state");
            if first && (s.erasing || s.whiting) {
                // The precise eraser's round tip with its own settings (the pad is at 100%), or white ink
                // with the tool's texture and settings, restoring the pad's paper as the editor's erase
                // strokes restore a page's (their brush hashes the seed's low 24 bits).
                let t = if s.erasing { TOOL_ERASER } else { s.tab };
                brush_begin(&mut s.bs, t, &settings.tools[t.0 as usize], ev.t_ms & 0xFFFFFF, dpi);
                brush_set_erase(&mut s.bs, paper_px(&s.paper), s.pad.x, s.pad.y, s.paper.stride);
                smoother_reset(&mut s.sm);
            } else if first {
                let p = &settings.tools[s.tab.0 as usize];
                brush_begin(&mut s.bs, s.tab, p, ev.t_ms, dpi);
                smoother_reset(&mut s.sm);
            }
            smoother_apply(&mut s.sm, s.bs.params.smoothing, &mut smp);
            let saved = screen.clip;
            canvas_set_clip(screen, s.pad);
            d = brush_add(&mut s.bs, screen, &smp);
            screen.clip = saved;
            pad = s.pad;
        }
        app_invalidate(a, rect_intersect(d, pad), RefreshMode::Fast);
    }
    if ev.t_ms.wrapping_sub(st(a).last_readout) > 150 {
        st(a).last_readout = ev.t_ms;
        draw_readout(a);
        draw_graph(a);
        let s = st(a);
        let r = rect_union(s.readout, rect_inflate(s.graph, ui_px(70.0)));
        app_invalidate(a, r, RefreshMode::Fast);
    }
}

/// Pen feel's own button and Close menu: Back
/// A controller's Pen feel button (the editor's opened it) or Close menu: Back, while nothing is held.
pub fn pensettings_shortcut(a: &mut App, action: i32) -> bool {
    if action != SHORTCUT_PEN_FEEL && action != SHORTCUT_CLOSE_MENU {
        return false;
    }
    let s = st(a);
    if s.has_pressed || s.drawing || s.erasing || s.whiting || s.wiping || s.active >= 0 {
        return false;
    }
    let to = if s.ret == ScreenId::Penset { ScreenId::Library } else { s.ret };
    app_switch(a, to);
    true
}

/// Whether the pen settings screen has a control at (x, y): a button, a tab, a slider or the pad. The
/// clock and battery corner leaves a tap there to it.
pub fn pensettings_control_at(a: &App, x: f32, y: f32) -> bool {
    a.ps.as_deref().is_none_or(|s| hits_find(&s.hits, x, y).is_some())
}

pub fn pensettings_event(a: &mut App, ev: &InEvent) {
    let pen = ev.type_ == InType::PenDown || ev.type_ == InType::PenMove || ev.type_ == InType::PenUp;
    let ph = app_pointer_phase(ev);
    if ph < 0 {
        return;
    }
    if ph == 0 {
        {
            let s = st(a);
            if s.has_pressed || s.drawing || s.active >= 0 {
                return;
            }
            press_begin(&mut s.press, ev.x, ev.y, ev.t_ms);
        }
        let Some(&h) = hits_find(&st(a).hits, ev.x, ev.y) else {
            return;
        };
        st(a).pen_owner = pen;
        if h.id == H_PAD {
            if !pen {
                return; // fingers do not draw
            }
            // The eraser (its tab, or the pen's eraser end): the precise one erases what it
            // touches; the stroke eraser hits the practice strokes on its tab.
            let white = pad_white(a);
            let eraser_mode = a.settings.eraser_mode;
            let wiping_off_tab;
            {
                let s = st(a);
                let eraser = ev.eraser || s.tab == TOOL_ERASER;
                s.erasing = eraser && eraser_mode == 1 && !s.paper.px.is_empty();
                s.whiting = !eraser && white && !s.paper.px.is_empty();
                s.wiping = eraser && !s.erasing;
                wiping_off_tab = s.wiping && s.tab != TOOL_ERASER;
            }
            if wiping_off_tab {
                draw_pad_frame(a);
                let r = rect_inflate(st(a).pad, ui_px(6.0));
                app_invalidate(a, r, RefreshMode::Ui);
            }
            st(a).drawing = true;
            display_live_ink(true); // the pad's ink is live, as in the editor
            pad_sample(a, ev, true);
            return;
        }
        if h.id == H_SLIDER {
            {
                let s = st(a);
                s.active = h.arg;
                s.last_paint = ev.t_ms;
            }
            set_slider(a, h.arg, ev, true, RefreshMode::Fast);
            return;
        }
        {
            let s = st(a);
            s.pressed = h;
            s.has_pressed = true;
        }
        app_press_feedback(a, h.r);
        return;
    }
    if pen != st(a).pen_owner {
        return;
    }
    if st(a).drawing {
        if ph == 1 {
            pad_sample(a, ev, false);
        } else {
            {
                let s = st(a);
                s.drawing = false;
                s.erasing = false;
                s.whiting = false;
                s.wiping = false;
            }
            display_live_ink(false);
            draw_readout(a);
            draw_graph(a);
            let s = st(a);
            let r = rect_union(rect_union(s.readout, rect_inflate(s.graph, ui_px(70.0))), s.pad);
            app_invalidate(a, r, RefreshMode::Ui);
        }
        return;
    }
    if st(a).active >= 0 {
        let active = st(a).active;
        if ph == 1 {
            set_slider(a, active, ev, false, RefreshMode::Fast);
        } else if ph == 2 {
            set_slider(a, active, ev, false, RefreshMode::Ui);
            st(a).active = -1;
            settings_save(&a.settings);
        }
        return;
    }
    if !st(a).has_pressed {
        return;
    }
    if ph == 1 {
        let s = st(a);
        press_move(&mut s.press, ev.x, ev.y);
        return;
    }
    let h;
    {
        let s = st(a);
        s.has_pressed = false;
        h = s.pressed;
    }
    canvas_invert(&mut a.screen, h.r);
    app_invalidate(a, h.r, RefreshMode::Ui);
    {
        let s = st(a);
        press_move(&mut s.press, ev.x, ev.y);
    }
    if !rect_contains(h.r, ev.x as i32, ev.y as i32) {
        return;
    }
    match h.id {
        H_BACK => {
            let s = st(a);
            let to = if s.ret == ScreenId::Penset { ScreenId::Library } else { s.ret };
            app_switch(a, to);
        }
        H_TAB => {
            {
                let s = st(a);
                s.tab = ToolKind(h.arg);
                s.live_raw = -1.0;
            }
            draw_all(a);
            app_invalidate_all(a, RefreshMode::Ui);
        }
        H_RESET => {
            let tab = st(a).tab;
            tool_params_default(tab, &mut a.settings.tools[tab.0 as usize]);
            if tab == TOOL_ERASER {
                a.settings.eraser_mode = 0;
            }
            settings_save(&a.settings);
            draw_all(a);
            app_invalidate_all(a, RefreshMode::Ui);
            app_toast(a, tr("Reset to defaults"), 1500);
        }
        H_CLEAR => {
            draw_pad_frame(a);
            let r = rect_inflate(st(a).pad, ui_px(6.0));
            app_invalidate(a, r, RefreshMode::Ui);
        }
        H_ERASER_MODE => {
            // Restore the test targets when changing what the eraser does to them.
            a.settings.eraser_mode = h.arg;
            settings_save(&a.settings);
            draw_all(a);
            app_invalidate_all(a, RefreshMode::Ui);
        }
        _ => {}
    }
}

pub fn pensettings_tick(a: &mut App, now: u32) {
    let s = st(a);
    if s.active >= 0 && s.slider_dirty && now.wrapping_sub(s.last_paint) >= 80 {
        s.last_paint = now;
        repaint_sliders(a, RefreshMode::Fast);
    }
}

// sscanf(s, "%d.%d", &x, &y) == 2: two decimal integers (each after any white space, with an
// optional sign) around a '.'.
fn scan_dot_pair(s: &str) -> Option<(i32, i32)> {
    fn int(b: &[u8], mut i: usize) -> Option<(i32, usize)> {
        while i < b.len() && matches!(b[i], b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r') {
            i += 1;
        }
        let mut neg = false;
        if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
            neg = b[i] == b'-';
            i += 1;
        }
        let start = i;
        let mut v: i64 = 0;
        while i < b.len() && b[i].is_ascii_digit() {
            v = v.saturating_mul(10).saturating_add((b[i] - b'0') as i64);
            i += 1;
        }
        if i == start {
            return None;
        }
        Some(((if neg { -v } else { v }).clamp(i32::MIN as i64, i32::MAX as i64) as i32, i))
    }
    let b = s.as_bytes();
    let (x, i) = int(b, 0)?;
    if i >= b.len() || b[i] != b'.' {
        return None;
    }
    let (y, _) = int(b, i + 1)?;
    Some((x, y))
}

/// "penset.*": the pen feel pad (app_probe's contract, without the prefix)
// Tests (app_probe's "penset." keys), while the pad keeps its copy of itself drawn bare (the precise
// eraser on, or white ink): "pad_ink" 1 while ink shows on the pad (a pixel darker than the bare
// pad there; white ink's swatch counts), "pad_lighter" 1 when a pixel is lighter than the bare pad
// (one of its lines, or its Clear button, erased away), "pad_ink_near.X.Y" 1 while ink shows
// within 12 px of the screen point X, Y. Empty without the copy. And "pad_white": 1 once white ink
// (or the precise eraser) went through the swatch (a pixel of it lighter than black), empty
// without a swatch.
pub fn pensettings_probe(a: &mut App, key: &str, out: &mut String, cap: usize) -> i32 {
    let App { screen, settings, ps, .. } = &mut *a;
    let s = ps.as_deref();
    if let Some(s) = s {
        if key == "slider_pending" {
            csnprintf!(out, cap, "%d", s.slider_dirty);
            return 0;
        }
        if let Some(name) = key.strip_prefix("param.") {
            for pi in tool_param_table() {
                if name == pi.key {
                    csnprintf!(out, cap, "%.4f", settings.tools[s.tab.0 as usize].get(pi.field));
                    return 0;
                }
            }
        }
        if key == "pad_strokes" {
            csnprintf!(out, cap, "%d", s.sample.as_ref().map_or(0, |p| p.strokes.len() as i32));
            return 0;
        }
        if key == "pad_sample_rect" {
            csnprintf!(out, cap, "%d %d %d %d", s.sample_box.x, s.sample_box.y, s.sample_box.w, s.sample_box.h);
            return 0;
        }
        if key == "pad_white" {
            let mut lit = false;
            let mut y = s.swatch.y;
            while y < s.swatch.y + s.swatch.h && !lit {
                let mut x = s.swatch.x;
                while x < s.swatch.x + s.swatch.w && !lit {
                    lit = screen.px[y as usize * screen.stride as usize + x as usize] > 0;
                    x += 1;
                }
                y += 1;
            }
            if rect_is_empty(s.swatch) {
                csnprintf!(out, cap, "%s", "");
            } else {
                csnprintf!(out, cap, "%d", if lit { 1 } else { 0 });
            }
            return 0;
        }
    }
    let (mut px, mut py) = (0, 0);
    let mut near = false;
    if let Some(rest) = key.strip_prefix("pad_ink_near.") {
        if let Some((x, y)) = scan_dot_pair(rest) {
            near = true;
            px = x;
            py = y;
        }
    }
    let lighter = key == "pad_lighter";
    let Some(s) = s else {
        return -1;
    };
    if !near && !lighter && key != "pad_ink" {
        return -1;
    }
    if s.paper.px.is_empty() || s.paper.w != s.pad.w || s.paper.h != s.pad.h {
        csnprintf!(out, cap, "%s", "");
        return 0;
    }
    let mut r = s.pad;
    if near {
        r = rect_intersect(r, Rect { x: px - 12, y: py - 12, w: 25, h: 25 });
    }
    let mut found = false;
    let mut y = r.y;
    while y < r.y + r.h && !found {
        let srow = &screen.px[y as usize * screen.stride as usize + r.x as usize..];
        let brow = &s.paper.px[(y - s.pad.y) as usize * s.paper.stride as usize + (r.x - s.pad.x) as usize..];
        let mut x = 0;
        while x < r.w && !found {
            found = if lighter { srow[x as usize] > brow[x as usize] } else { srow[x as usize] < brow[x as usize] };
            x += 1;
        }
        y += 1;
    }
    csnprintf!(out, cap, "%d", if found { 1 } else { 0 });
    0
}
