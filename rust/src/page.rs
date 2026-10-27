//! page.h: strokes and pages. (Also page.c: hit testing, page rendering and compaction.)

use crate::prelude::*;
use std::rc::Rc;

#[derive(Clone, Debug, Default)]
pub struct Stroke {
    pub tool: ToolKind,
    /// snapshot at draw time
    pub params: ToolParams,
    /// fixed from stroke_new on: an erase stroke's texture is its top byte
    pub seed: u32,
    /// the record's spare bytes, carried as read: colour, layer, flags
    /// (docs/FORMAT.md); this version reads only ext[2]'s low 4 bits, the
    /// tip variant (brush.h)
    pub ext: [u8; 3],
    /// the points (the C's pts, n, cap)
    pub pts: Vec<PenSample>,
    /// covers every rendered pixel (points + the render tool's max radius + 2)
    pub bbox: Rect,
}

/// Erase strokes (tool TOOL_ERASER) restore the paper under them instead of depositing ink: white
/// ink with a drawing tool's texture and settings, or the precise eraser's round hard tip
/// (docs/FORMAT.md, Erase strokes). The seed's top byte holds the texture, the tool + 1 (0 = the
/// round tip), and the brush hashes only its low 24 bits.
#[inline]
pub fn stroke_is_erase(s: &Stroke) -> bool {
    s.tool == TOOL_ERASER
}
/// The tool whose look and reach a stroke draws with: its own, or an erase stroke's texture
/// (TOOL_ERASER, the round tip, also for a texture this version does not have). It comes from the
/// seed, so every stroke knows it before its first point: the reader, scale_page and the editor.
pub fn stroke_render_tool(s: &Stroke) -> ToolKind {
    if s.tool != TOOL_ERASER {
        return s.tool;
    }
    let t = (s.seed >> 24) as i32; // the texture: a drawing tool + 1, or 0 for the round tip
    if t >= 1 && t <= TOOL_COUNT as i32 && t - 1 != TOOL_ERASER.0 {
        ToolKind(t - 1)
    } else {
        TOOL_ERASER
    }
}
/// The seed for a new erase stroke: the texture (a drawing tool, or TOOL_ERASER for the round tip;
/// anything else is the round tip too) in the top byte, the low 24 bits of hash below it.
pub fn erase_seed(texture: ToolKind, hash: u32) -> u32 {
    let t = texture.0;
    let top = if t >= 0 && t < TOOL_COUNT as i32 && t != TOOL_ERASER.0 { t as u32 + 1 } else { 0 };
    top << 24 | (hash & 0xFFFFFF)
}
/// The tip variant s was drawn with (brush.h): ext[2]'s low 4 bits.
#[inline]
pub fn stroke_tip(s: &Stroke) -> u8 {
    s.ext[2] & 0x0F
}
/// A stroke being drawn now: records the tip this version draws its tool with (the render tool's,
/// so white ink in the highlighter's texture gets the highlighter's).
pub fn stroke_mark_tip(s: &mut Stroke) {
    s.ext[2] = (s.ext[2] & 0xF0) | brush_tip_current(stroke_render_tool(s));
}
/// Starts b to draw s, live or replayed: one start for both, so they give the same pixels. An
/// erase stroke's brush restores toward the paper given (as in brush_set_erase, NULL = white),
/// which must cover the canvas's clip while it draws; an ink stroke's ignores it.
pub fn stroke_brush_begin(b: &mut BrushState, s: &Stroke, dpi: f32, paper: Option<std::rc::Rc<[u8]>>, x0: i32, y0: i32, stride: i32) {
    let erase = stroke_is_erase(s);
    let t = stroke_render_tool(s);
    brush_begin(b, t, &s.params, if erase { s.seed & 0xFFFFFF } else { s.seed }, dpi);
    // The tip it was drawn with; one this version does not know is drawn with the current one.
    if brush_tip_known(t, stroke_tip(s)) {
        b.tip = stroke_tip(s);
    }
    if erase {
        brush_set_erase(b, paper, x0, y0, stride);
    }
}

/// a notebook id with its terminator (STORE_ID_LEN)
pub const PAGE_NB_LEN: usize = 24;

/// A stroke's identity while it is boxed: its address, which the C's undo history finds strokes by
/// ("Strokes are located by pointer on undo/redo", history.c). Compared, never dereferenced.
pub type StrokeId = usize;
#[inline]
pub fn stroke_id(s: &Stroke) -> StrokeId {
    s as *const Stroke as usize
}

#[derive(Clone, Debug, Default)]
pub struct Page {
    /// may be a kind this build does not know (paper_is_known): drawn blank, kept
    pub paper: PaperKind,
    /// The notebook the page is in (its id): where an image paper's picture is (bgimage.c). The store
    /// sets it on every page it makes or reads for a notebook; "" (a page of none) draws one white.
    pub nb: FixedStr<PAGE_NB_LEN>,
    /// page px
    pub w: i32,
    pub h: i32,
    /// owned, draw order (the C's strokes, n, cap). Boxed: a stroke keeps its address while it moves
    /// between the page and the undo history, which knows it by that (stroke_id).
    pub strokes: Vec<Box<Stroke>>,
    /// chunks after the last stroke, written back as they were (the C's tail, tail_len)
    pub tail: Vec<u8>,
    /// from a newer Grimoire (or not all readable): shown, never changed or written
    pub readonly: bool,
}

pub fn stroke_new(t: ToolKind, p: &ToolParams, seed: u32) -> Box<Stroke> {
    Box::new(Stroke { tool: t, params: *p, seed, ..Default::default() })
}
pub fn stroke_free(s: Box<Stroke>) {
    drop(s)
}
/// grows bbox (by the render tool)
pub fn stroke_append(s: &mut Stroke, pt: &PenSample, dpi: f32) {
    if s.pts.len() == s.pts.capacity() {
        // (the C doubles from 64, and drops the point when the memory is not there)
        let nc = if s.pts.capacity() > 0 { s.pts.capacity() * 2 } else { 64 };
        if s.pts.try_reserve_exact(nc - s.pts.len()).is_err() {
            return;
        }
    }
    s.pts.push(*pt);
    // An erase stroke reaches as far as its texture's tool does (white charcoal's specks).
    let pad = brush_max_radius(stroke_render_tool(s), &s.params, dpi) + 2.0;
    s.bbox = rect_union(s.bbox, rect_from_points(pt.x, pt.y, pt.x, pt.y, pad));
}

// (The float expressions below are the C's, contracted as clang contracts them: PORTING.md, Floating
// point. `a * b + c * d` fuses its left product, `a * b - c * d` too.)
fn seg_point_dist2(px: f32, py: f32, ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
    let dx = bx - ax;
    let dy = by - ay;
    let l2 = fmla(dx, dx, dy * dy);
    let t = if l2 > 0.0 { clampf(fmla(px - ax, dx, (py - ay) * dy) / l2, 0.0, 1.0) } else { 0.0 };
    let ex = px - fmla(dx, t, ax);
    let ey = py - fmla(dy, t, ay);
    fmla(ex, ex, ey * ey)
}

fn segs_intersect(ax: f32, ay: f32, bx: f32, by: f32, cx: f32, cy: f32, dx: f32, dy: f32) -> bool {
    let d1 = fmla(dx - cx, ay - cy, -((dy - cy) * (ax - cx)));
    let d2 = fmla(dx - cx, by - cy, -((dy - cy) * (bx - cx)));
    let d3 = fmla(bx - ax, cy - ay, -((by - ay) * (cx - ax)));
    let d4 = fmla(bx - ax, dy - ay, -((by - ay) * (dx - ax)));
    ((d1 > 0.0) != (d2 > 0.0)) && ((d3 > 0.0) != (d4 > 0.0))
}

fn seg_seg_dist2(ax: f32, ay: f32, bx: f32, by: f32, cx: f32, cy: f32, dx: f32, dy: f32) -> f32 {
    if segs_intersect(ax, ay, bx, by, cx, cy, dx, dy) {
        return 0.0;
    }
    let mut m = seg_point_dist2(ax, ay, cx, cy, dx, dy);
    m = m.min(seg_point_dist2(bx, by, cx, cy, dx, dy));
    m = m.min(seg_point_dist2(cx, cy, ax, ay, bx, by));
    m = m.min(seg_point_dist2(dx, dy, ax, ay, bx, by));
    m
}

/// True if the segment (x0,y0)-(x1,y1) swept with radius r touches the stroke.
pub fn stroke_hit_segment(s: &Stroke, x0: f32, y0: f32, x1: f32, y1: f32, r: f32, dpi: f32) -> bool {
    let n = s.pts.len();
    if n == 0 {
        return false;
    }
    let sweep = rect_from_points(x0, y0, x1, y1, r + 1.0);
    if !rect_overlaps(sweep, s.bbox) {
        return false;
    }
    for i in 0..n {
        let a = &s.pts[i];
        let b = &s.pts[if i + 1 < n { i + 1 } else { i }];
        let mut ra = 0.0f32;
        let mut op = 0.0f32;
        brush_dab(s.tool, &s.params, a, dpi, &mut ra, &mut op);
        let lim = r + ra;
        if seg_seg_dist2(x0, y0, x1, y1, a.x, a.y, b.x, b.y) <= lim * lim {
            return true;
        }
    }
    false
}

/// True if (x,y) lies within r px of the rendered stroke (polyline distance <= r + half width).
pub fn stroke_hit(s: &Stroke, x: f32, y: f32, r: f32, dpi: f32) -> bool {
    stroke_hit_segment(s, x, y, x, y, r, dpi)
}

/// any paper value is kept as it is
pub fn page_new(w: i32, h: i32, paper: PaperKind) -> Box<Page> {
    // an unknown kind stays as it is: a newer Grimoire draws it (docs/FORMAT.md)
    Box::new(Page { w, h, paper, ..Default::default() })
}
/// frees strokes and tail too
pub fn page_free(p: Box<Page>) {
    drop(p)
}
/// takes ownership, appends
pub fn page_add(p: &mut Page, s: Box<Stroke>) {
    let n = p.strokes.len() as i32;
    page_insert(p, n, s)
}

fn page_grow(p: &mut Page) -> bool {
    if p.strokes.len() < p.strokes.capacity() {
        return true;
    }
    let nc = if p.strokes.capacity() > 0 { p.strokes.capacity() * 2 } else { 32 };
    p.strokes.try_reserve_exact(nc - p.strokes.len()).is_ok()
}
/// takes ownership
pub fn page_insert(p: &mut Page, index: i32, s: Box<Stroke>) {
    if !page_grow(p) {
        qlog!("page: out of memory, stroke dropped");
        stroke_free(s);
        return;
    }
    let index = clampi(index, 0, p.strokes.len() as i32);
    p.strokes.insert(index as usize, s);
}
/// returns ownership
pub fn page_remove(p: &mut Page, index: i32) -> Box<Stroke> {
    // (the C returns NULL for an index out of range; every caller passes one that is in it, and the
    // skeleton's signature has no None: this panics where the C would hand back NULL)
    assert!(index >= 0 && (index as usize) < p.strokes.len(), "page_remove: index {index} out of range");
    p.strokes.remove(index as usize)
}
pub fn page_clear_strokes(p: &mut Page) {
    p.strokes.clear();
}

/// Renders paper + every stroke intersecting region, clipped to region. Strokes are
/// replayed from their first sample with brush_add so the result equals the live render.
/// A stroke with a tool this build does not know is never drawn. Erase strokes restore the paper
/// drawn here (a copy of it where they reach into the region).
pub fn page_render(c: &mut Canvas, p: &Page, region: Rect, dpi: f32) {
    page_render_layers(c, p, region, dpi, PAGE_LAYER_ALL)
}
/// The same with either layer left out (the canvas keeps what it had there, white if fresh):
/// the share studio recolors paper and ink separately. Without the paper layer, erase strokes
/// restore toward white, which is right on a fresh white canvas only: never draw a page in two
/// passes (the paper layer, then the ink layer over it), as that wipes the paper's lines under
/// every erase stroke. Draw both layers in one call.
pub const PAGE_LAYER_PAPER: i32 = 1;
pub const PAGE_LAYER_INK: i32 = 2;
pub const PAGE_LAYER_ALL: i32 = 3;
pub fn page_render_layers(c: &mut Canvas, p: &Page, region: Rect, dpi: f32, layers: i32) {
    render_layers(c, p, region, dpi, layers, None, -1, None)
}
/// The page's paper alone over region, as page_render draws it: an image paper from its notebook's
/// picture (bgimage.c), any other kind by paper_render (which draws an image paper blank). Wherever
/// a page's paper is needed apart from its ink (what an erase stroke restores toward, drawn live),
/// take it from here.
pub fn page_paper_render(c: &mut Canvas, p: &Page, region: Rect, dpi: f32) {
    if paper_is_image(p.paper) {
        bgimage_render(c, p.nb.as_str(), p.paper, region);
    } else {
        paper_render(c, p.paper, region, dpi);
    }
}

// Whether stroke i is drawn: it is not `left_out`, nor marked in skip (None = none marked).
fn drawn(skip: Option<&[bool]>, left_out: i32, i: usize) -> bool {
    i as i32 != left_out && !skip.is_some_and(|s| s[i])
}

// A stroke in pieces (compaction): runs of its points, each with the brush as it stands before the
// run and the rect its dabs can draw into, measured by a dry run of the brush. A region drawn from
// the pieces that reach into it, each from its own brush, gets exactly what a replay from the first
// point gives it: the brush's course does not depend on the canvas, and the other pieces draw
// nothing there.
struct Piece {
    /// points first..end-1
    first: i32,
    end: i32,
    /// on the page (empty: they draw nothing)
    box_: Rect,
    /// before point `first`
    b: BrushState,
}

// A page's strokes in pieces, each stroke cut the first time it is drawn so.
struct Pieces<'a> {
    p: &'a Page,
    dpi: f32,
    /// per stroke: its pieces once cut, else None
    pc: Vec<Option<Vec<Piece>>>,
}

// page_render_layers, leaving out the strokes marked in skip and stroke left_out, and drawing each
// stroke from its pieces if ps is given (compaction).
fn render_layers(c: &mut Canvas, p: &Page, region: Rect, dpi: f32, layers: i32, skip: Option<&[bool]>, left_out: i32, mut ps: Option<&mut Pieces>) {
    let saved = c.clip;
    let r = rect_intersect(region, saved);
    if rect_is_empty(r) {
        return;
    }
    let n = p.strokes.len();
    // Erase strokes restore the paper under them: with the paper layer, a copy of the paper drawn
    // here, of the part of r their bboxes reach (u); without it, white.
    let mut u = RECT_EMPTY;
    if (layers & PAGE_LAYER_PAPER) != 0 && (layers & PAGE_LAYER_INK) != 0 {
        for i in 0..n {
            if drawn(skip, left_out, i) && stroke_is_erase(&p.strokes[i]) {
                u = rect_union(u, rect_intersect(p.strokes[i].bbox, r));
            }
        }
    }
    // (the room for the copy, taken now and filled after the paper is drawn)
    let mut copy: Option<Vec<u8>> = None;
    if !rect_is_empty(u) {
        let mut room = Vec::new();
        if room.try_reserve_exact(u.w as usize * u.h as usize).is_ok() {
            copy = Some(room);
        } else {
            if r.w > 1 || r.h > 1 {
                // Out of memory: the same pixels in two halves, each with a smaller copy.
                let mut a = r;
                let mut b = r;
                if r.h >= r.w {
                    a.h = r.h / 2;
                    b.y += a.h;
                    b.h -= a.h;
                } else {
                    a.w = r.w / 2;
                    b.x += a.w;
                    b.w -= a.w;
                }
                render_layers(c, p, a, dpi, layers, skip, left_out, ps.as_deref_mut());
                render_layers(c, p, b, dpi, layers, skip, left_out, ps.as_deref_mut());
                return;
            }
            static LOGGED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
            if !LOGGED.load(std::sync::atomic::Ordering::Relaxed) {
                qlog!("page: out of memory, erase strokes restore toward white");
            }
            LOGGED.store(true, std::sync::atomic::Ordering::Relaxed);
        }
    }
    if (layers & PAGE_LAYER_PAPER) != 0 {
        page_paper_render(c, p, r, dpi);
    }
    if (layers & PAGE_LAYER_INK) == 0 {
        return;
    }
    // (shared with each erase stroke's brush: the C hands them all the one buffer)
    let copy: Option<Rc<[u8]>> = copy.map(|mut room| {
        for y in 0..u.h {
            let at = ((u.y + y) * c.stride + u.x) as usize;
            room.extend_from_slice(&c.px[at..at + u.w as usize]);
        }
        Rc::from(room)
    });
    let mut b = BrushState::default();
    for i in 0..n {
        let s: &Stroke = &p.strokes[i];
        if !drawn(skip, left_out, i) || (s.tool.0 as u32) >= TOOL_COUNT as u32 || !rect_overlaps(s.bbox, r) {
            continue;
        }
        // An erase stroke stays inside its bbox, which u covers where it meets r: clipped to u it
        // draws the same pixels, and it never reads past the copy.
        canvas_set_clip(c, if copy.is_some() && stroke_is_erase(s) { u } else { r });
        let pcs: Option<&[Piece]> = match ps.as_deref_mut() {
            Some(ps) => pieces_of(ps, i),
            None => None,
        };
        // The pens are read as it goes (work_pump, common.h): a dense page took 1.2 s on the Kindle.
        // Every 128 points is 3-15 ms there, depending on the tool.
        let Some(pcs) = pcs else {
            // whole (also when there was no memory to cut it)
            stroke_brush_begin(&mut b, s, dpi, copy.clone(), u.x, u.y, u.w);
            for k in 0..s.pts.len() {
                brush_add(&mut b, c, &s.pts[k]);
                if (k & 127) == 127 {
                    work_pump();
                }
            }
            work_pump();
            continue;
        };
        let mut next = -1; // the point b stands before
        for pc in pcs {
            if !rect_overlaps(pc.box_, r) {
                continue;
            }
            if pc.first != next {
                b = pc.b.clone();
                if stroke_is_erase(s) {
                    brush_set_erase(&mut b, copy.clone(), u.x, u.y, u.w);
                }
            }
            for m in pc.first..pc.end {
                brush_add(&mut b, c, &s.pts[m as usize]);
                if ((m - pc.first) & 127) == 127 {
                    work_pump();
                }
            }
            next = pc.end;
        }
        work_pump();
    }
    c.clip = saved;
}

/// Renders the whole page then box-downsamples into thumb (thumb size sets the scale).
pub fn page_render_thumb(thumb: &mut Canvas, p: &Page, dpi: f32) {
    let mut full = Canvas::default();
    if canvas_init(&mut full, p.w, p.h) != 0 {
        return;
    }
    page_render(&mut full, p, Rect { x: 0, y: 0, w: p.w, h: p.h }, dpi);
    canvas_downsample(thumb, &full);
    canvas_free(&mut full);
}

// ---- compaction ----

// Stroke i's pieces, cut now if they are not yet (a piece ends at the first point that takes its
// rect past 16 mm either way), or None when out of memory: then the stroke is drawn whole.
fn pieces_of<'a>(ps: &'a mut Pieces, i: usize) -> Option<&'a [Piece]> {
    if ps.pc[i].is_some() {
        return ps.pc[i].as_deref();
    }
    let p = ps.p;
    let s: &Stroke = &p.strokes[i];
    let span = mm_to_px(16.0, ps.dpi);
    // no pixels: dry
    let mut none = Canvas { px: Vec::new(), w: p.w, h: p.h, stride: p.w, clip: Rect { x: 0, y: 0, w: p.w, h: p.h } };
    let mut b = BrushState::default();
    stroke_brush_begin(&mut b, s, ps.dpi, None, 0, 0, 0);
    b.dry = true;
    let mut cut: Vec<Piece> = Vec::new();
    let mut k = 0usize;
    while k < s.pts.len() {
        if cut.try_reserve(1).is_err() {
            return None;
        }
        let first = k as i32;
        let mut box_ = RECT_EMPTY;
        let mut before = b.clone();
        before.dry = false;
        loop {
            box_ = rect_union(box_, brush_add(&mut b, &mut none, &s.pts[k]));
            k += 1;
            if !(k < s.pts.len() && (box_.w as f32) < span && (box_.h as f32) < span) {
                break;
            }
        }
        cut.push(Piece { first, end: k as i32, box_, b: before });
    }
    cut.shrink_to_fit();
    ps.pc[i] = Some(cut);
    ps.pc[i].as_deref()
}

// The rects of stroke i's pieces (pieces_of's, copied: the checks below draw while they go through
// them, and drawing borrows the pieces too).
fn piece_boxes(ps: &mut Pieces, i: usize) -> Option<Vec<Rect>> {
    pieces_of(ps, i).map(|pcs| pcs.iter().map(|pc| pc.box_).collect())
}

// What compaction works with.
struct Compact<'a> {
    p: &'a Page,
    dpi: f32,
    /// the strokes marked so far
    drop: &'a mut [bool],
    ps: Pieces<'a>,
    // The reference: the page as it is, each layer drawn where a check of that layer first needs
    // it (`ready` per REF_BLOCK square). Marked strokes change nothing, so drawing it without them
    // gives the same pixels and it never goes stale.
    /// the page, the ink layer
    layer: [Canvas; 2],
    ready: [Vec<u8>; 2],
    blocks_x: i32,
    // Where the paper is white, a pixel of the ink layer is that pixel of the page, whatever is
    // drawn: the same strokes over the same white, restoring toward the same white. So the ink
    // layer is checked only in the rows where the paper has marks (`marked`, per row and REF_BLOCK
    // of columns), and not at all on paper white all over (then there is no ink canvas either).
    ink_too: bool,
    marked: Vec<u8>,
    /// the reference under a region, while a check draws over it
    keep: Vec<u8>,
}

const REF_BLOCK: i32 = 16;

// Makes sure the reference of layer l (0 the page, 1 the ink layer) is drawn over r, a rect on the
// page: the rect around the blocks r meets that are not yet (drawing a block again changes nothing).
fn ref_ready(x: &mut Compact, l: usize, r: Rect) {
    let bx0 = r.x / REF_BLOCK;
    let bx1 = (r.x + r.w - 1) / REF_BLOCK;
    let by0 = r.y / REF_BLOCK;
    let by1 = (r.y + r.h - 1) / REF_BLOCK;
    let mut mx0 = bx1 + 1;
    let mut mx1 = bx0 - 1;
    let mut my0 = by1 + 1;
    let mut my1 = by0 - 1;
    for by in by0..=by1 {
        for bx in bx0..=bx1 {
            if x.ready[l][(by * x.blocks_x + bx) as usize] == 0 {
                mx0 = if bx < mx0 { bx } else { mx0 };
                mx1 = if bx > mx1 { bx } else { mx1 };
                my0 = if by < my0 { by } else { my0 };
                my1 = if by > my1 { by } else { my1 };
            }
        }
    }
    if mx1 < mx0 {
        return;
    }
    let t = Rect { x: mx0 * REF_BLOCK, y: my0 * REF_BLOCK, w: (mx1 - mx0 + 1) * REF_BLOCK, h: (my1 - my0 + 1) * REF_BLOCK };
    let t = rect_intersect(t, Rect { x: 0, y: 0, w: x.p.w, h: x.p.h });
    if l != 0 {
        canvas_fill(&mut x.layer[l], t, 255); // the ink layer starts white
    }
    render_layers(&mut x.layer[l], x.p, t, x.dpi, if l != 0 { PAGE_LAYER_INK } else { PAGE_LAYER_ALL }, Some(&*x.drop), -1, Some(&mut x.ps));
    for by in my0..=my1 {
        let at = (by * x.blocks_x + mx0) as usize;
        x.ready[l][at..at + (mx1 - mx0 + 1) as usize].fill(1);
    }
}

// Whether leaving out stroke k too (besides the marked ones) changes r, a rect on the page, in layer
// l: 1 if it does, 0 if not, -1 out of memory. Draws r without them over the reference, compares
// the two and puts the reference back.
fn differs_in(x: &mut Compact, l: usize, r: Rect, k: i32) -> i32 {
    let need = r.w as usize * r.h as usize;
    if need > x.keep.len() {
        if x.keep.try_reserve_exact(need - x.keep.len()).is_err() {
            return -1;
        }
        x.keep.resize(need, 0);
    }
    ref_ready(x, l, r);
    {
        let c = &x.layer[l];
        for y in 0..r.h {
            let at = ((r.y + y) * c.stride + r.x) as usize;
            let to = y as usize * r.w as usize;
            x.keep[to..to + r.w as usize].copy_from_slice(&c.px[at..at + r.w as usize]);
        }
    }
    if l != 0 {
        canvas_fill(&mut x.layer[l], r, 255);
    }
    render_layers(&mut x.layer[l], x.p, r, x.dpi, if l != 0 { PAGE_LAYER_INK } else { PAGE_LAYER_ALL }, Some(&*x.drop), k, Some(&mut x.ps));
    let mut same = true;
    let c = &mut x.layer[l];
    for y in 0..r.h {
        let at = ((r.y + y) * c.stride + r.x) as usize;
        let row = &mut c.px[at..at + r.w as usize];
        let to = y as usize * r.w as usize;
        let was = &x.keep[to..to + r.w as usize];
        same = same && row == was;
        row.copy_from_slice(was);
    }
    (!same) as i32
}

// Whether row y of the paper has marks in the column blocks r spans.
fn row_marked(x: &Compact, y: i32, r: Rect) -> bool {
    let m = &x.marked[y as usize * x.blocks_x as usize..];
    for bx in r.x / REF_BLOCK..=(r.x + r.w - 1) / REF_BLOCK {
        if m[bx as usize] != 0 {
            return true;
        }
    }
    false
}

// The same for both layers: the page over r, then the ink layer over the runs of rows of r where
// the paper has marks.
fn differs(x: &mut Compact, r: Rect, k: i32) -> i32 {
    let mut d = differs_in(x, 0, r, k);
    let mut y = r.y;
    while x.ink_too && d == 0 && y < r.y + r.h {
        if row_marked(x, y, r) {
            let mut end = y + 1;
            while end < r.y + r.h && row_marked(x, end, r) {
                end += 1;
            }
            d = differs_in(x, 1, Rect { x: r.x, y, w: r.w, h: end - y }, k);
            y = end;
        }
        y += 1;
    }
    d
}

// Whether an erase stroke after stroke i reaches into r.
fn erased_after(p: &Page, i: usize, r: Rect) -> bool {
    for j in i + 1..p.strokes.len() {
        if stroke_is_erase(&p.strokes[j]) && rect_overlaps(p.strokes[j].bbox, r) {
            return true;
        }
    }
    false
}

// Whether leaving out ink stroke i changes the page (1, 0, or -1 out of memory). It can change only
// what its pieces draw over, so each piece's rect is checked. Pieces no later erase stroke reaches
// go first: they show unless later ink covers them, so a stroke that stays is told at once.
fn ink_matters(x: &mut Compact, i: usize) -> i32 {
    let Some(boxes) = piece_boxes(&mut x.ps, i) else { return -1 };
    for pass in 0..2 {
        for r in boxes.iter().copied() {
            if rect_is_empty(r) || erased_after(x.p, i, r) != (pass == 1) {
                continue;
            }
            let d = differs(x, r, i as i32);
            if d != 0 {
                return d;
            }
        }
    }
    0
}

// The same for erase stroke j. It only lightens, and a pixel no ink before it darkened is at its
// paper already, so it can change only where its pieces meet the pieces of earlier ink that is
// not marked (inside that ink's bbox): those rects are checked, and an erase stroke over nothing
// else is marked without drawing anything.
fn erase_matters(x: &mut Compact, j: usize) -> i32 {
    let p = x.p;
    let Some(mine) = piece_boxes(&mut x.ps, j) else { return -1 };
    for box_ in mine {
        let mut r = RECT_EMPTY;
        let mut i = 0;
        while i < j && !rect_is_empty(box_) {
            let s: &Stroke = &p.strokes[i];
            let ink = !stroke_is_erase(s) && (s.tool.0 as u32) < TOOL_COUNT as u32;
            if x.drop[i] || !ink || !rect_overlaps(s.bbox, box_) {
                i += 1;
                continue;
            }
            let Some(theirs) = piece_boxes(&mut x.ps, i) else { return -1 };
            for m in theirs {
                r = rect_union(r, rect_intersect(box_, m));
            }
            i += 1;
        }
        if rect_is_empty(r) {
            continue;
        }
        let d = differs(x, r, j as i32);
        if d != 0 {
            return d;
        }
    }
    0
}

// A zeroed buffer of n bytes, or None when the memory is not there (the C's calloc).
fn zeroed(n: usize) -> Option<Vec<u8>> {
    let mut v = Vec::new();
    v.try_reserve_exact(n).ok()?;
    v.resize(n, 0);
    Some(v)
}

// Gets x ready: room for the pieces, the reference canvases (nothing drawn in them yet) and
// whether the paper is white all over. 0, or -1 out of memory.
fn compact_start(x: &mut Compact) -> i32 {
    let p = x.p;
    let n = p.strokes.len();
    x.ps = Pieces { p, dpi: x.dpi, pc: Vec::new() };
    if x.ps.pc.try_reserve_exact(n).is_err() {
        return -1;
    }
    x.ps.pc.resize_with(n, || None);
    if canvas_init(&mut x.layer[0], p.w, p.h) != 0 {
        return -1;
    }
    x.blocks_x = (p.w + REF_BLOCK - 1) / REF_BLOCK;
    let blocks = x.blocks_x as usize * ((p.h + REF_BLOCK - 1) / REF_BLOCK) as usize;
    let (Some(r0), Some(r1), Some(marked)) = (zeroed(blocks), zeroed(blocks), zeroed(x.blocks_x as usize * p.h as usize)) else {
        return -1;
    };
    x.ready = [r0, r1];
    x.marked = marked;
    // Where the paper has marks (the reference is drawn over it later).
    page_paper_render(&mut x.layer[0], p, Rect { x: 0, y: 0, w: p.w, h: p.h }, x.dpi);
    for y in 0..p.h {
        let at = (y * x.layer[0].stride) as usize;
        let row = &x.layer[0].px[at..at + p.w as usize];
        let m = &mut x.marked[y as usize * x.blocks_x as usize..];
        for k in 0..p.w as usize {
            if row[k] != 255 {
                m[k / REF_BLOCK as usize] = 1;
                x.ink_too = true;
            }
        }
    }
    if x.ink_too && canvas_init(&mut x.layer[1], p.w, p.h) != 0 {
        -1
    } else {
        0
    }
}

// (compact_free: nothing to do by hand, the buffers drop with x)

/// Compaction, for privacy: erased ink stays in the page until it is dropped (zips and
/// grimoire-pull copy the page files), and every erase stroke replays on each render. Marks
/// drop[i] (p->n entries) for each stroke that can be left out without changing a pixel of the
/// page or of its ink layer: ink that later erase strokes restore entirely, and erase strokes that
/// change nothing. Each is checked with the ones marked before it already left out, so leave them
/// out together. Returns how many, or -1 when out of memory (none marked). A page without erase
/// strokes has none. p is not changed (the editor's undo history holds pointers to its strokes):
/// leave the marked strokes out of a copy. dpi is the one its strokes were built at.
///
/// Cost: each candidate (ink a later erase stroke reaches, and every erase stroke) is checked by
/// drawing, without it, the rects it draws into, and comparing them with the page as it is (drawn
/// there once for all the checks). Only the pieces of strokes (runs of about 16 mm) that reach a
/// rect are replayed, and the ink layer is drawn only where the paper has marks. On the host, in
/// full renders of the same page: a page of writing with three words scrubbed out took 0.2, half a
/// page scrubbed six times over 2, a sketch of 360 long lines reworked under 20 half-page scrubs 4.
/// On the device that can be tens of seconds, and the launcher's watchdog kills Grimoire after
/// 15 s without a heartbeat: never call it on the UI thread. It only reads p, so a forked child (as
/// bgjob.c runs exports) can work on its own copy of the page. Memory: two page-size canvases (one
/// on paper white all over) and the pieces of the strokes it draws.
pub fn page_droppable(p: &Page, dpi: f32, drop: &mut [bool]) -> i32 {
    let ns = p.strokes.len();
    let mut any = false;
    for (i, d) in drop.iter_mut().enumerate().take(ns) {
        *d = false;
        any = any || stroke_is_erase(&p.strokes[i]);
    }
    if !any || p.w <= 0 || p.h <= 0 {
        return 0;
    }
    // A stroke whose leaving out changes neither the page nor its ink layer is marked. Marked
    // strokes change nothing, so the page drawn without them is the page itself, and each check
    // sees the ones marked before it. Ink goes first: once it is out, the erase strokes over it may
    // have nothing left to change.
    let mut x = Compact {
        p,
        dpi,
        drop: &mut drop[..ns],
        ps: Pieces { p, dpi, pc: Vec::new() },
        layer: [Canvas::default(), Canvas::default()],
        ready: [Vec::new(), Vec::new()],
        blocks_x: 0,
        ink_too: false,
        marked: Vec::new(),
        keep: Vec::new(),
    };
    let page = Rect { x: 0, y: 0, w: p.w, h: p.h };
    let mut n = if compact_start(&mut x) == 0 { 0 } else { -1 };
    let mut pass = 0;
    while pass < 2 && n >= 0 {
        let mut i = 0;
        while i < ns && n >= 0 {
            let s: &Stroke = &p.strokes[i];
            let skip = (s.tool.0 as u32) >= TOOL_COUNT as u32 || stroke_is_erase(s) != (pass == 1) // (ink or erase, by the pass)
                || (pass == 0 && !erased_after(p, i, s.bbox)) // only erasing makes ink go
                || rect_is_empty(rect_intersect(s.bbox, page));
            if !skip {
                let d = if pass != 0 { erase_matters(&mut x, i) } else { ink_matters(&mut x, i) };
                if d < 0 {
                    n = -1;
                } else if d == 0 {
                    x.drop[i] = true;
                    n += 1;
                }
            }
            i += 1;
        }
        pass += 1;
    }
    if n < 0 {
        x.drop.fill(false);
    }
    n
}
