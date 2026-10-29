//! The Index map. This view owns its raster and interaction; the editor owns navigation and names.
use crate::prelude::*;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum MapResult {
    #[default]
    None,
    Open(u32),
    Close,
    Rename(u32),
    Pages,
    List,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum MapLayout {
    Seed,
    Running,
    #[default]
    Done,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MapHitKind {
    Control(u8),
    Node(usize),
    Tray(usize),
}
#[derive(Clone, Copy, Debug)]
struct MapHit {
    rect: Rect,
    kind: MapHitKind,
    centre: (f32, f32),
    radius: f32,
}
struct MapDrag {
    x: f32,
    y: f32,
    ox: f32,
    oy: f32,
    zoom: f32,
    distance: f32,
    moved: bool,
    last_present: u32,
    saved: Canvas,
}
#[derive(Default)]
pub struct MapView {
    pub open: bool,
    pub needs_redraw: bool,
    canvas: Canvas,
    g: GraphModel,
    ini: GraphIni,
    id: FixedStr<STORE_ID_LEN>,
    pages: Vec<u32>,
    zoom: f32,
    ox: f32,
    oy: f32,
    mode: GraphMode,
    around: Vec<GraphRing>,
    centre: i32,
    selected: i32,
    layout: MapLayout,
    hits: Vec<MapHit>,
    labels: Vec<Rect>,
    drag: Option<MapDrag>,
    last_input_ms: u32,
    job_done: bool,
    tray_expanded: bool,
    w: i32,
    h: i32,
}
const BACK: u8 = 1;
const LIST: u8 = 2;
const ALL: u8 = 3;
const AROUND: u8 = 4;
const FIT: u8 = 5;
const MINUS: u8 = 6;
const PLUS: u8 = 7;
const UNLINKED: u8 = 8;
const TIDY: u8 = 9;
const OPEN: u8 = 10;
const AROUND_IT: u8 = 11;
const RENAME: u8 = 12;
const EXPAND_TRAY: u8 = 13;
const GRAPH: u8 = 14;
fn u(px: f32) -> i32 {
    ui_px(px)
}
fn full(v: &MapView) -> Rect {
    Rect {
        x: 0,
        y: 0,
        w: v.w,
        h: v.h,
    }
}
fn plot(v: &MapView) -> Rect {
    let bottom = if v.selected >= 0 { u(320.0) } else { u(170.0) };
    let tray = if v.ini.show_unlinked && !v.g.tray.is_empty() {
        if v.w > v.h {
            u(520.0)
        } else {
            u(240.0)
        }
    } else {
        0
    };
    Rect {
        x: u(16.0),
        y: u(190.0),
        w: v.w - u(32.0) - if v.w > v.h { tray } else { 0 },
        h: v.h - u(190.0) - bottom - if v.w <= v.h { tray } else { 0 },
    }
}
fn save(v: &mut MapView) {
    v.ini.mode = v.mode;
    graph_ini_store(&mut v.ini, &v.g);
    if v.ini.writable {
        graph_ini_write(v.id.as_str(), &mut v.ini);
    }
}
fn reserve<T>(v: &mut Vec<T>, n: usize) -> bool {
    v.try_reserve_exact(n.saturating_sub(v.len())).is_ok()
}
fn location(v: &MapView, i: usize) -> Option<(f32, f32)> {
    let n = &v.g.nodes[i];
    if v.mode == GraphMode::Around {
        return v.around.iter().find(|r| r.node == i as i32).map(|r| (r.x, r.y));
    }
    if n.tier == GraphTier::Tray || !n.placed {
        return None;
    }
    if n.tier == GraphTier::Satellite && (!v.ini.show_unlinked || (n.sat_rank >= GRAPH_SAT_DRAWN && v.zoom < 2.0)) {
        return None;
    }
    Some((n.x, n.y))
}
fn screen_point(v: &MapView, p: (f32, f32)) -> (f32, f32) {
    ((p.0 - v.ox) * v.zoom, (p.1 - v.oy) * v.zoom)
}
fn page_number(v: &MapView, i: usize) -> usize {
    v.pages.iter().position(|&p| p == v.g.nodes[i].pid).map_or(0, |p| p + 1)
}
fn label(v: &MapView, nav: &Nav, i: usize) -> String {
    let name = nav_name(nav, v.g.nodes[i].pid);
    if name.is_empty() {
        cformat!(tr("Page %d"), page_number(v, i) as i32)
    } else {
        name.to_string()
    }
}
fn fit_points(v: &mut MapView, local: bool) {
    let r = plot(v);
    let mut bounds = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    let mut count = 0;
    for i in 0..v.g.nodes.len() {
        if local && v.g.open >= 0 && i != v.g.open as usize {
            let linked = graph_edge_between(&v.g, v.g.open, i as i32) >= 0
                || graph_neighbours(&v.g, v.g.open)
                    .iter()
                    .any(|&n| graph_edge_between(&v.g, n, i as i32) >= 0);
            if !linked {
                continue;
            }
        }
        if let Some((x, y)) = location(v, i) {
            let pad = v.g.nodes[i].r + 12.0;
            bounds.0 = bounds.0.min(x - pad);
            bounds.1 = bounds.1.min(y - pad);
            bounds.2 = bounds.2.max(x + pad);
            bounds.3 = bounds.3.max(y + pad);
            count += 1;
        }
    }
    if count == 0 {
        v.zoom = 1.0;
        v.ox = -(r.x + r.w / 2) as f32;
        v.oy = -(r.y + r.h / 2) as f32;
        return;
    }
    let z = ((r.w - u(80.0)) as f32 / (bounds.2 - bounds.0).max(1.0))
        .min((r.h - u(80.0)) as f32 / (bounds.3 - bounds.1).max(1.0))
        .clamp(0.25, 6.0);
    v.zoom = z;
    v.ox = (bounds.0 + bounds.2) * 0.5 - (r.x + r.w / 2) as f32 / z;
    v.oy = (bounds.1 + bounds.3) * 0.5 - (r.y + r.h / 2) as f32 / z;
}
fn default_fit(v: &mut MapView) {
    fit_points(v, false);
    if v.mode == GraphMode::All && v.g.open >= 0 && v.g.force.iter().any(|&i| v.g.nodes[i as usize].r * v.zoom < 10.0) {
        fit_points(v, true);
    }
}
fn around(v: &mut MapView, centre: i32) -> bool {
    if graph_around(&v.g, centre, 2, &mut v.around) < 0 {
        return false;
    }
    v.mode = GraphMode::Around;
    v.centre = centre;
    true
}
fn layout(v: &mut MapView, a: &mut App, force: bool) -> bool {
    let plan = if force {
        GraphPlan::Full
    } else {
        graph_apply_ini(&mut v.g, &v.ini)
    };
    let ok = if plan == GraphPlan::Full {
        if graph_layout_in_process(&v.g) {
            graph_layout(&mut v.g, &mut work_pump)
        } else {
            v.layout = MapLayout::Seed;
            if !graph_seed(&mut v.g, &mut work_pump) {
                return false;
            }
            let mut path = String::new();
            if graph_ini_path(v.id.as_str(), &mut path) != 0 {
                return false;
            }
            v.layout = MapLayout::Running;
            // Save the seed and view choices before the child reads the cache.
            save(v);
            bg_queue(BgKind::GraphLayout, Some(v.id.as_str()), Some(&path), a.dpi, false);
            true
        }
    } else {
        graph_place_new(&mut v.g, &mut work_pump) >= 0
    };
    if ok && v.layout != MapLayout::Running {
        v.layout = MapLayout::Done;
    }
    ok
}
pub fn mapview_open(
    v: &mut MapView,
    a: &mut App,
    nb: &NotebookInfo,
    nav: &Nav,
    open_pid: u32,
    around_open: bool,
) -> bool {
    *v = MapView::default();
    #[cfg(feature = "sim")]
    if std::env::var_os("GRIMOIRE_SIM_MAP_NOMEM").is_some_and(|s| s == "1") {
        app_toast(a, tr("Not enough memory for the map."), 3000);
        return false;
    }
    v.w = a.w;
    v.h = a.h;
    v.id = nb.id;
    v.selected = -1;
    v.centre = -1;
    let n = nb.page_ids.len();
    if canvas_init(&mut v.canvas, a.w, a.h) != 0
        || !reserve(&mut v.pages, n)
        || !reserve(&mut v.hits, n + 32)
        || !reserve(&mut v.labels, n)
        || graph_build(&mut v.g, nb, nav, open_pid) != 0
    {
        app_toast(a, tr("Not enough memory for the map."), 3000);
        return false;
    }
    v.pages.extend_from_slice(&nb.page_ids);
    graph_ini_read(nb.id.as_str(), &mut v.ini);
    v.mode = v.ini.mode;
    if !layout(v, a, false) {
        app_toast(a, tr("Not enough memory for the map."), 3000);
        return false;
    }
    if (around_open || v.mode == GraphMode::Around) && !around(v, v.g.open) {
        return false;
    }
    save(v);
    v.open = true;
    default_fit(v);
    v.last_input_ms = now_ms();
    mapview_draw(v, a, nav, full(v));
    app_invalidate(a, full(v), RefreshMode::Full);
    true
}
pub fn mapview_close(v: &mut MapView) {
    save(v);
    *v = MapView::default();
}
fn button(v: &mut MapView, c: &mut Canvas, r: Rect, id: u8, text: &str, icon: IconKind, active: bool) {
    ui_button(
        c,
        r,
        icon,
        text,
        if active {
            ButtonStyle::Plain
        } else {
            ButtonStyle::Disabled
        },
    );
    if active {
        v.hits.push(MapHit {
            rect: r,
            kind: MapHitKind::Control(id),
            centre: (0.0, 0.0),
            radius: 0.0,
        });
    }
}
fn ring(c: &mut Canvas, x: f32, y: f32, r: f32, t: f32) {
    canvas_disc_aa(c, x, y, r, 0, 1.0);
    if r > t {
        canvas_disc_aa(c, x, y, r - t, 255, 1.0);
    }
}
fn visible_rect(v: &MapView, i: usize) -> Rect {
    location(v, i).map_or(RECT_EMPTY, |p| {
        let (x, y) = screen_point(v, p);
        let radius = v.g.nodes[i].r * v.zoom + 12.0;
        rect_from_points(x, y, x, y, radius)
    })
}
fn render(v: &mut MapView, c: &mut Canvas, nav: &Nav) {
    canvas_reset_clip(c);
    canvas_fill(c, full(v), 255);
    v.hits.clear();
    v.labels.clear();
    let pr = plot(v);
    canvas_set_clip(c, pr);
    for k in 0..v.g.edges.len() {
        if k & 63 == 0 {
            work_pump();
        }
        let e = v.g.edges[k];
        let (Some(pa), Some(pb)) = (location(v, e.a as usize), location(v, e.b as usize)) else {
            continue;
        };
        let (a, b) = (screen_point(v, pa), screen_point(v, pb));
        let selected = v.selected == e.a || v.selected == e.b;
        let width = if selected {
            5.0
        } else {
            (2.5 + (e.mult - 1) as f32).min(5.0)
        };
        let incoming = selected
            && if v.selected == e.a {
                e.dir & GRAPH_DIR_AB == 0
            } else {
                e.dir & GRAPH_DIR_BA == 0
            };
        if incoming {
            canvas_dash_poly(c, &[a, b], u(5.0), u(20.0), u(14.0), 0, 0);
        } else {
            canvas_line_aa(c, a.0, a.1, b.0, b.1, width, 0);
        }
    }
    // All knockouts precede all dots, so a neighbouring dot cannot erase another dot's ring.
    for i in 0..v.g.nodes.len() {
        if i & 63 == 0 {
            work_pump();
        }
        if let Some(p) = location(v, i) {
            let (x, y) = screen_point(v, p);
            canvas_disc_aa(c, x, y, v.g.nodes[i].r * v.zoom + 4.0, 255, 1.0);
        }
    }
    for i in 0..v.g.nodes.len() {
        if i & 63 == 0 {
            work_pump();
        }
        let Some(p) = location(v, i) else {
            continue;
        };
        let (x, y) = screen_point(v, p);
        let n = v.g.nodes[i];
        let r = n.r * v.zoom;
        if !rect_overlaps(visible_rect(v, i), pr) {
            continue;
        }
        // Outer rings first: their white interiors must not erase the dot's fill or inner ring.
        if i as i32 == v.selected {
            ring(c, x, y, r + 12.0, 8.0);
        }
        if n.open {
            ring(c, x, y, r + 6.0, 2.0);
        }
        let level = match n.degree {
            0 => 0,
            1 => 1,
            2 | 3 => 2,
            _ => 4,
        };
        canvas_disc_pattern(c, x, y, r, level);
        if level < 4 {
            ring(c, x, y, r, 3.0);
            canvas_disc_pattern(c, x, y, (r - 3.0).max(0.0), level);
        }
        if n.index_links > 0 {
            canvas_lozenge(c, x + r, y - r, u(8.0) as f32, 0);
        }
        v.hits.push(MapHit {
            rect: rect_from_points(x, y, x, y, r.max(u(59.0) as f32)),
            kind: MapHitKind::Node(i),
            centre: (x, y),
            radius: r.max(u(59.0) as f32),
        });
    }
    // Priority order is selected, open, neighbours, then named pages by degree.
    let mut candidates = Vec::new();
    if reserve(&mut candidates, v.g.nodes.len()) {
        for i in 0..v.g.nodes.len() {
            if location(v, i).is_some()
                && (v.g.nodes[i].named
                    || v.zoom >= 2.0
                    || v.mode == GraphMode::Around
                    || i as i32 == v.selected
                    || v.g.nodes[i].open)
            {
                candidates.push(i);
            }
        }
        candidates.sort_unstable_by_key(|&i| {
            (
                if i as i32 == v.selected {
                    0
                } else if v.g.nodes[i].open {
                    1
                } else if v.selected >= 0 && graph_edge_between(&v.g, v.selected, i as i32) >= 0 {
                    2
                } else {
                    3
                },
                std::cmp::Reverse(v.g.nodes[i].degree),
                v.g.nodes[i].pid,
            )
        });
        for i in candidates {
            let (x, y) = screen_point(v, location(v, i).unwrap());
            let r = v.g.nodes[i].r * v.zoom + u(16.0) as f32;
            let name = label(v, nav, i);
            let w = font_measure(FontFace::Bold, u(34.0) as f32, &name).min(u(320.0)) + u(16.0);
            let h = u(48.0);
            let spots = [
                Rect {
                    x: (x + r) as i32,
                    y: y as i32 - h / 2,
                    w,
                    h,
                },
                Rect {
                    x: (x - r) as i32 - w,
                    y: y as i32 - h / 2,
                    w,
                    h,
                },
                Rect {
                    x: x as i32 - w / 2,
                    y: (y - r) as i32 - h,
                    w,
                    h,
                },
                Rect {
                    x: x as i32 - w / 2,
                    y: (y + r) as i32,
                    w,
                    h,
                },
            ];
            if let Some(r) = spots.into_iter().find(|r| {
                rect_intersect(*r, pr) == *r
                    && !v.labels.iter().any(|&old| rect_overlaps(old, *r))
                    && !(0..v.g.nodes.len()).any(|other| {
                        other != i
                            && location(v, other).is_some_and(|p| {
                                let (cx, cy) = screen_point(v, p);
                                let radius = v.g.nodes[other].r * v.zoom + 4.0;
                                let nearest_x = cx.clamp(r.x as f32, (r.x + r.w) as f32);
                                let nearest_y = cy.clamp(r.y as f32, (r.y + r.h) as f32);
                                (cx - nearest_x).hypot(cy - nearest_y) < radius
                            })
                    })
            }) {
                v.labels.push(r);
                canvas_fill(c, r, if i as i32 == v.selected { 0 } else { 255 });
                font_draw_left_name(
                    c,
                    FontFace::Bold,
                    u(34.0) as f32,
                    r,
                    &name,
                    if i as i32 == v.selected { 255 } else { 0 },
                );
            }
        }
    }
    if v.ini.show_unlinked && v.zoom < 2.0 && v.mode == GraphMode::All {
        for i in 0..v.g.nodes.len() {
            let more = graph_sat_more(&v.g, i as i32);
            if more > 0 {
                if let Some(p) = location(v, i) {
                    let (x, y) = screen_point(v, p);
                    ui_text(
                        c,
                        FontFace::Bold,
                        u(34.0) as f32,
                        u(28.0) as f32,
                        Rect {
                            x: x as i32 + u(30.0),
                            y: y as i32 + u(30.0),
                            w: u(160.0),
                            h: u(48.0),
                        },
                        &cformat!("+%d", more),
                        1,
                        false,
                        0,
                    );
                }
            }
        }
    }
    canvas_reset_clip(c);
    if v.ini.show_unlinked && !v.g.tray.is_empty() {
        let right = v.w > v.h;
        let tray = if right {
            Rect {
                x: pr.x + pr.w + u(16.0),
                y: pr.y,
                w: u(480.0),
                h: pr.h,
            }
        } else {
            Rect {
                x: u(30.0),
                y: pr.y + pr.h,
                w: v.w - u(60.0),
                h: u(230.0),
            }
        };
        let title = if v.g.tray.len() > 200 && !v.tray_expanded {
            cformat!(ng_("Not linked yet: %d page. Show", "Not linked yet: %d pages. Show", v.g.tray.len() as i64), v.g.tray.len() as i32)
        } else {
            cformat!(tr("Not linked yet (%d)"), v.g.tray.len() as i32)
        };
        let title_r = Rect { h: u(50.0), ..tray };
        ui_text(
            c,
            FontFace::Bold,
            u(34.0) as f32,
            u(28.0) as f32,
            title_r,
            &title,
            1,
            false,
            0,
        );
        if v.g.tray.len() > 200 && !v.tray_expanded {
            v.hits.push(MapHit {
                rect: title_r,
                kind: MapHitKind::Control(EXPAND_TRAY),
                centre: (0.0, 0.0),
                radius: 0.0,
            });
        } else {
            canvas_set_clip(c, tray);
            for k in 0..v.g.tray.len() {
                let i = v.g.tray[k] as usize;
                let r = Rect {
                    x: tray.x,
                    y: tray.y + u(50.0) + k as i32 * u(40.0) - (v.oy.max(0.0) * v.zoom) as i32,
                    w: tray.w,
                    h: u(40.0),
                };
                if rect_overlaps(r, tray) {
                    ui_text(
                        c,
                        FontFace::Regular,
                        u(32.0) as f32,
                        u(26.0) as f32,
                        r,
                        &label(v, nav, i),
                        1,
                        false,
                        0,
                    );
                    v.hits.push(MapHit {
                        rect: rect_intersect(r, tray),
                        kind: MapHitKind::Tray(i),
                        centre: (0.0, 0.0),
                        radius: 0.0,
                    });
                }
            }
            canvas_reset_clip(c);
        }
    }
    let bh = u(140.0);
    let gap = u(12.0);
    let y = u(20.0);
    button(
        v,
        c,
        Rect {
            x: u(20.0),
            y,
            w: u(150.0),
            h: u(150.0),
        },
        BACK,
        "",
        IconKind::Back,
        true,
    );
    ui_text(
        c,
        FontFace::Bold,
        u(46.0) as f32,
        u(34.0) as f32,
        Rect {
            x: u(190.0),
            y,
            w: u(220.0),
            h: bh,
        },
        tr("Index"),
        1,
        false,
        0,
    );
    let graph = Rect {
        x: u(420.0),
        y,
        w: u(200.0),
        h: bh,
    };
    ui_button(c, graph, IconKind::None, tr("Graph"), ButtonStyle::Selected);
    v.hits.push(MapHit {
        rect: graph,
        kind: MapHitKind::Control(GRAPH),
        centre: (0.0, 0.0),
        radius: 0.0,
    });
    button(
        v,
        c,
        Rect {
            x: u(632.0),
            y,
            w: u(200.0),
            h: bh,
        },
        LIST,
        tr("List"),
        IconKind::None,
        true,
    );
    button(
        v,
        c,
        Rect {
            x: u(848.0),
            y,
            w: u(300.0),
            h: bh,
        },
        ALL,
        tr("All"),
        IconKind::None,
        true,
    );
    button(
        v,
        c,
        Rect {
            x: u(1160.0),
            y,
            w: u(300.0),
            h: bh,
        },
        AROUND,
        tr("Around"),
        IconKind::None,
        v.g.open >= 0 || v.selected >= 0,
    );
    button(
        v,
        c,
        Rect {
            x: v.w - u(300.0),
            y,
            w: u(280.0),
            h: bh,
        },
        FIT,
        tr("Fit"),
        IconKind::None,
        true,
    );
    let bottom = v.h - if v.selected >= 0 { u(320.0) } else { u(170.0) };
    if v.selected >= 0 {
        let i = v.selected as usize;
        let n = v.g.nodes[i];
        ui_text(
            c,
            FontFace::Bold,
            u(64.0) as f32,
            u(34.0) as f32,
            Rect {
                x: u(30.0),
                y: bottom,
                w: v.w - u(60.0),
                h: u(90.0),
            },
            &label(v, nav, i),
            1,
            false,
            0,
        );
        let details = cformat!(
            tr("Page %d. Links to %d, linked from %d."),
            page_number(v, i) as i32,
            n.links_to,
            n.linked_from
        );
        ui_text(
            c,
            FontFace::Regular,
            u(34.0) as f32,
            u(28.0) as f32,
            Rect {
                x: u(30.0),
                y: bottom + u(92.0),
                w: v.w - u(60.0),
                h: u(60.0),
            },
            &details,
            1,
            false,
            0,
        );
        let w = (v.w - u(80.0)) / 3;
        for (k, id, text) in [
            (0, OPEN, tr("Open")),
            (1, AROUND_IT, tr("Around it")),
            (2, RENAME, tr("Rename")),
        ] {
            button(
                v,
                c,
                Rect {
                    x: u(20.0) + k * (w + gap),
                    y: bottom + u(165.0),
                    w,
                    h: bh,
                },
                id,
                text,
                IconKind::None,
                id != RENAME || nav.writable,
            );
        }
    } else {
        let bw = u(220.0);
        button(v, c, Rect { x: u(20.0), y: bottom + u(15.0), w: bw, h: bh }, MINUS, "", IconKind::Minus, true);
        button(v, c, Rect { x: u(252.0), y: bottom + u(15.0), w: bw, h: bh }, PLUS, "", IconKind::Plus, true);
        button(
            v,
            c,
            Rect { x: u(500.0), y: bottom + u(15.0), w: u(640.0), h: bh },
            UNLINKED,
            tr(if v.ini.show_unlinked {
                n_("Unlinked: Show")
            } else {
                n_("Unlinked: Hide")
            }),
            IconKind::None,
            true,
        );
        button(
            v,
            c,
            Rect {
                x: v.w - u(470.0),
                y: bottom + u(15.0),
                w: u(450.0),
                h: bh,
            },
            TIDY,
            tr("Tidy up"),
            IconKind::None,
            v.layout != MapLayout::Running,
        );
    }
}
pub fn mapview_draw(v: &mut MapView, a: &mut App, nav: &Nav, r: Rect) {
    if !v.open {
        return;
    }
    v.w = a.w;
    v.h = a.h;
    if v.canvas.w != a.w || v.canvas.h != a.h {
        v.drag = None;
        if canvas_init(&mut v.canvas, a.w, a.h) != 0 {
            return;
        }
        default_fit(v);
    }
    let mut canvas = std::mem::take(&mut v.canvas);
    render(v, &mut canvas, nav);
    v.canvas = canvas;
    canvas_blit(&mut a.screen, &v.canvas, r, r.x, r.y);
    v.needs_redraw = false;
}
fn hit(v: &MapView, x: f32, y: f32) -> Option<MapHitKind> {
    if let Some(h) = v
        .hits
        .iter()
        .rev()
        .find(|h| !matches!(h.kind, MapHitKind::Node(_)) && rect_contains(h.rect, x as i32, y as i32))
    {
        return Some(h.kind);
    }
    v.hits
        .iter()
        .filter_map(|h| {
            if matches!(h.kind, MapHitKind::Node(_)) {
                let d = (h.centre.0 - x).hypot(h.centre.1 - y);
                (d <= h.radius).then_some((d, h.kind))
            } else {
                None
            }
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|p| p.1)
}
fn snapshot(v: &MapView) -> Canvas {
    let mut c = Canvas::default();
    if canvas_init(&mut c, v.w, v.h) == 0 {
        canvas_blit(&mut c, &v.canvas, full(v), 0, 0);
    }
    c
}
fn zoom_at(v: &mut MapView, factor: f32, x: f32, y: f32) {
    let z = (v.zoom * factor).clamp(0.25, 6.0);
    v.ox += x / v.zoom - x / z;
    v.oy += y / v.zoom - y / z;
    v.zoom = z;
}
fn paint(v: &mut MapView, a: &mut App, nav: &Nav, mode: RefreshMode) {
    mapview_draw(v, a, nav, full(v));
    app_invalidate(a, full(v), mode);
}
pub fn mapview_pointer(v: &mut MapView, a: &mut App, nav: &Nav, phase: i32, e: &InEvent) -> MapResult {
    v.last_input_ms = e.t_ms;
    if phase == 0 {
        v.drag = Some(MapDrag {
            x: e.x,
            y: e.y,
            ox: v.ox,
            oy: v.oy,
            zoom: v.zoom,
            distance: 0.0,
            moved: false,
            last_present: e.t_ms,
            saved: snapshot(v),
        });
        return MapResult::None;
    }
    if phase == 1 {
        let pr = plot(v);
        let Some(d) = v.drag.as_mut() else {
            return MapResult::None;
        };
        if (e.x - d.x).hypot(e.y - d.y) > u(12.0) as f32 {
            d.moved = true;
        }
        if d.moved {
            v.ox = d.ox - (e.x - d.x) / v.zoom;
            v.oy = d.oy - (e.y - d.y) / v.zoom;
            if e.t_ms.wrapping_sub(d.last_present) >= 180 && !d.saved.px.is_empty() {
                d.last_present = e.t_ms;
                let (dx, dy) = ((e.x - d.x) as i32, (e.y - d.y) as i32);
                canvas_set_clip(&mut a.screen, pr);
                canvas_fill(&mut a.screen, pr, 255);
                canvas_blit(&mut a.screen, &d.saved, pr, pr.x + dx, pr.y + dy);
                canvas_reset_clip(&mut a.screen);
                app_invalidate(a, pr, RefreshMode::Fast);
            }
        }
        return MapResult::None;
    }
    let Some(d) = v.drag.take() else {
        return MapResult::None;
    };
    if touch_cancelled(e) {
        v.ox = d.ox;
        v.oy = d.oy;
        paint(v, a, nav, RefreshMode::Ui);
        return MapResult::None;
    }
    if d.moved {
        paint(v, a, nav, RefreshMode::Ui);
        return MapResult::None;
    }
    let selected = v.selected;
    let mut label_dirty = v.labels.iter().fold(RECT_EMPTY, |r, &l| rect_union(r, l));
    let old_bounds = if selected >= 0 {
        visible_rect(v, selected as usize)
    } else {
        RECT_EMPTY
    };
    match hit(v, e.x, e.y) {
        Some(MapHitKind::Node(i) | MapHitKind::Tray(i)) => {
            if selected == i as i32 {
                return MapResult::Open(v.g.nodes[i].pid);
            }
            v.selected = i as i32;
        }
        Some(MapHitKind::Control(id)) => match id {
            BACK => return MapResult::Close,
            LIST => return MapResult::List,
            GRAPH => return MapResult::None,
            ALL => {
                v.mode = GraphMode::All;
                save(v);
                default_fit(v);
            }
            AROUND | AROUND_IT => {
                let centre = if v.selected >= 0 { v.selected } else { v.g.open };
                if around(v, centre) {
                    save(v);
                    default_fit(v);
                }
            }
            FIT => default_fit(v),
            MINUS | PLUS => {
                let r = plot(v);
                zoom_at(
                    v,
                    if id == PLUS { 1.5 } else { 1.0 / 1.5 },
                    (r.x + r.w / 2) as f32,
                    (r.y + r.h / 2) as f32,
                );
            }
            UNLINKED => {
                v.ini.show_unlinked = !v.ini.show_unlinked;
                save(v);
                default_fit(v);
            }
            TIDY => {
                if layout(v, a, true) {
                    save(v);
                    if v.mode == GraphMode::Around {
                        around(v, v.centre);
                    }
                    default_fit(v);
                }
            }
            OPEN if v.selected >= 0 => return MapResult::Open(v.g.nodes[v.selected as usize].pid),
            RENAME if v.selected >= 0 => return MapResult::Rename(v.g.nodes[v.selected as usize].pid),
            EXPAND_TRAY => v.tray_expanded = true,
            _ => {}
        },
        None => v.selected = -1,
    }
    if selected != v.selected {
        let new_bounds = if v.selected >= 0 {
            visible_rect(v, v.selected as usize)
        } else {
            RECT_EMPTY
        };
        for edge in &v.g.edges {
            if edge.a == selected || edge.b == selected || edge.a == v.selected || edge.b == v.selected {
                if let (Some(a), Some(b)) = (location(v, edge.a as usize), location(v, edge.b as usize)) {
                    let (a, b) = (screen_point(v, a), screen_point(v, b));
                    label_dirty = rect_union(label_dirty, rect_from_points(a.0, a.1, b.0, b.1, 8.0));
                }
            }
        }
        mapview_draw(v, a, nav, full(v));
        label_dirty = v.labels.iter().fold(label_dirty, |r, &l| rect_union(r, l));
        let card = Rect {
            x: 0,
            y: v.h - u(320.0),
            w: v.w,
            h: u(320.0),
        };
        app_invalidate(
            a,
            rect_intersect(
                rect_union(rect_union(rect_union(old_bounds, new_bounds), card), label_dirty),
                full(v),
            ),
            RefreshMode::Ui,
        );
    } else {
        paint(v, a, nav, RefreshMode::Ui);
    }
    MapResult::None
}
pub fn mapview_pinch(v: &mut MapView, a: &mut App, e: &InEvent) {
    v.last_input_ms = e.t_ms;
    let (x, y) = ((e.x + e.x2) * 0.5, (e.y + e.y2) * 0.5);
    let distance = (e.x2 - e.x).hypot(e.y2 - e.y).max(1.0);
    if e.type_ == InType::PinchBegin {
        v.drag = Some(MapDrag {
            x,
            y,
            ox: v.ox,
            oy: v.oy,
            zoom: v.zoom,
            distance,
            moved: true,
            last_present: e.t_ms,
            saved: snapshot(v),
        });
        return;
    }
    if e.type_ == InType::PinchEnd {
        v.drag = None;
        v.needs_redraw = true;
        return;
    }
    let pr = plot(v);
    let Some(d) = v.drag.as_mut() else {
        return;
    };
    let z = (d.zoom * distance / d.distance).clamp(0.25, 6.0);
    v.ox = d.ox + d.x / d.zoom - x / z;
    v.oy = d.oy + d.y / d.zoom - y / z;
    v.zoom = z;
    if e.t_ms.wrapping_sub(d.last_present) < 180 || d.saved.px.is_empty() {
        return;
    }
    d.last_present = e.t_ms;
    canvas_set_clip(&mut a.screen, pr);
    for yy in pr.y..pr.y + pr.h {
        for xx in pr.x..pr.x + pr.w {
            let sx = (((xx as f32 / z + v.ox - d.ox) * d.zoom).round()) as i32;
            let sy = (((yy as f32 / z + v.oy - d.oy) * d.zoom).round()) as i32;
            a.screen.px[(yy * a.screen.stride + xx) as usize] = if rect_contains(pr, sx, sy) {
                d.saved.px[(sy * d.saved.stride + sx) as usize]
            } else {
                255
            };
        }
    }
    canvas_reset_clip(&mut a.screen);
    app_invalidate(a, pr, RefreshMode::Fast);
}
pub fn mapview_background_done(v: &mut MapView, ok: bool) {
    if v.open && v.layout == MapLayout::Running {
        if ok {
            v.job_done = true;
        } else {
            v.layout = MapLayout::Seed;
            v.job_done = false;
        }
    }
}
pub fn mapview_tick(v: &mut MapView, _a: &mut App, now: u32) -> MapResult {
    if v.open && v.job_done && v.drag.is_none() && now.wrapping_sub(v.last_input_ms) >= 500 {
        let mut ini = GraphIni::default();
        if graph_ini_read(v.id.as_str(), &mut ini) == 0 {
            v.ini = ini;
            graph_apply_ini(&mut v.g, &v.ini);
            graph_place_new(&mut v.g, &mut work_pump);
            if v.mode == GraphMode::Around {
                around(v, v.centre);
            }
            v.layout = MapLayout::Done;
            v.needs_redraw = true;
        } else {
            v.layout = MapLayout::Seed;
        }
        v.job_done = false;
    }
    MapResult::None
}
pub fn mapview_rebuild(v: &mut MapView, a: &mut App, nb: &NotebookInfo, nav: &Nav, open_pid: u32) -> bool {
    save(v);
    let mut g = GraphModel::default();
    if graph_build(&mut g, nb, nav, open_pid) != 0 {
        return false;
    }
    let old = std::mem::replace(&mut v.g, g);
    if !layout(v, a, false) {
        v.g = old;
        return false;
    }
    // A rename changes membership (an unnamed satellite can become a named force node),
    // but every dot the user has already seen stays where it was.
    for node in &mut v.g.nodes {
        let at = graph_node_of(&old, node.pid);
        if at >= 0 && old.nodes[at as usize].placed {
            let before = old.nodes[at as usize];
            node.x = before.x;
            node.y = before.y;
            node.placed = true;
        }
    }
    if v.mode == GraphMode::Around {
        around(v, v.centre);
    }
    save(v);
    v.needs_redraw = true;
    true
}
pub fn mapview_probe(v: &MapView, key: &str, out: &mut String) -> bool {
    out.clear();
    match key {
        "" => out.push_str(if v.open { "graph" } else { "0" }),
        "nodes" => out.push_str(&v.g.nodes.len().to_string()),
        "edges" => out.push_str(&v.g.edges.len().to_string()),
        "force" => out.push_str(&v.g.force.len().to_string()),
        "satellites" => out.push_str(
            &v.g.nodes
                .iter()
                .filter(|n| n.tier == GraphTier::Satellite)
                .count()
                .to_string(),
        ),
        "tray" => out.push_str(&v.g.tray.len().to_string()),
        "orphans" => out.push_str(
            &v.g.nodes
                .iter()
                .filter(|n| n.tier == GraphTier::Force && n.degree == 0)
                .count()
                .to_string(),
        ),
        "selected" => {
            if v.selected >= 0 {
                out.push_str(&page_number(v, v.selected as usize).to_string());
            }
        }
        "mode" => out.push_str(if v.mode == GraphMode::All { "all" } else { "around" }),
        "layout" => out.push_str(match v.layout {
            MapLayout::Seed => "seed",
            MapLayout::Running => "running",
            MapLayout::Done => "done",
        }),
        "labels" => out.push_str(&v.labels.len().to_string()),
        "zoom" => out.push_str(&format!("{:.2}", v.zoom)),
        _ => {
            if let Some(pid) = key.strip_prefix("node.").and_then(|s| s.parse::<u32>().ok()) {
                let i = graph_node_of(&v.g, pid);
                if i >= 0 {
                    let n = v.g.nodes[i as usize];
                    let p = location(v, i as usize).map(|p| screen_point(v, p));
                    let (x, y) = p.unwrap_or((0.0, 0.0));
                    out.push_str(&format!(
                        "{} {:.0} {:.0} {:.0} {}",
                        match n.tier {
                            GraphTier::Force => "force",
                            GraphTier::Satellite => "satellite",
                            GraphTier::Tray => "tray",
                        },
                        n.r * v.zoom,
                        x,
                        y,
                        p.is_some() as i32
                    ));
                }
            } else if let Some(xy) = key.strip_prefix("ui_at.") {
                let Some((x, y)) = xy
                    .split_once('.')
                    .and_then(|(x, y)| Some((x.parse::<f32>().ok()?, y.parse::<f32>().ok()?)))
                else {
                    return false;
                };
                match hit(v, x, y) {
                    Some(MapHitKind::Node(i)) => out.push_str(&format!("node.{}", v.g.nodes[i].pid)),
                    Some(MapHitKind::Tray(i)) => out.push_str(&format!("tray.{}", v.g.nodes[i].pid)),
                    Some(MapHitKind::Control(id)) => out.push_str(match id {
                        BACK => "back",
                        GRAPH => "graph",
                        LIST => "list",
                        ALL => "all",
                        AROUND => "around",
                        FIT => "fit",
                        MINUS => "minus",
                        PLUS => "plus",
                        UNLINKED => "unlinked",
                        TIDY => "tidy",
                        OPEN => "card.open",
                        AROUND_IT => "card.around",
                        RENAME => "card.rename",
                        _ => "tray.show",
                    }),
                    None => {}
                }
            } else {
                return false;
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    fn view(w: i32, h: i32, n: u32, linked: bool) -> (MapView, Nav) {
        ui_init(300.0);
        assert_eq!(font_init(), 0);
        let mut nb = NotebookInfo::default();
        nb.page_ids = (1..=n).collect();
        let mut nav = Nav::default();
        nav_init(&mut nav);
        if linked {
            nav_set_name(&mut nav, 1, "Start");
            nav_set_name(&mut nav, 2, "Destination");
            nav_link_set(&mut nav, 1, 5, "p2");
        }
        let mut v = MapView {
            open: true,
            w,
            h,
            selected: -1,
            centre: -1,
            ..Default::default()
        };
        v.pages = nb.page_ids.clone();
        assert_eq!(graph_build(&mut v.g, &nb, &nav, if linked { 1 } else { 0 }), 0);
        assert!(graph_layout(&mut v.g, &mut || {}));
        assert!(reserve(&mut v.hits, n as usize + 32));
        assert!(reserve(&mut v.labels, n as usize));
        default_fit(&mut v);
        (v, nav)
    }
    #[test]
    fn fitted_nodes_and_labels_stay_in_the_viewport() {
        for (w, h) in [(1860, 2480), (2480, 1860)] {
            let (mut v, nav) = view(w, h, 4, true);
            v.selected = 1;
            default_fit(&mut v);
            let mut c = Canvas::default();
            canvas_init(&mut c, w, h);
            render(&mut v, &mut c, &nav);
            assert!((0.25..=6.0).contains(&v.zoom));
            assert!(v.labels.len() >= 2);
            for (i, &a) in v.labels.iter().enumerate() {
                assert_eq!(rect_intersect(a, plot(&v)), a);
                assert!(!v.labels[i + 1..].iter().any(|&b| rect_overlaps(a, b)));
            }
            for &i in &v.g.force {
                assert!(rect_overlaps(visible_rect(&v, i as usize), plot(&v)));
            }
        }
    }
    #[test]
    fn an_open_selected_dot_keeps_both_its_pattern_and_rings() {
        let (mut v, nav) = view(1860, 2480, 2, true);
        v.selected = 0;
        let mut c = Canvas::default();
        canvas_init(&mut c, v.w, v.h);
        render(&mut v, &mut c, &nav);
        let n = v.g.nodes[0];
        let (x, y) = screen_point(&v, (n.x, n.y));
        let r = n.r * v.zoom;
        let pixel = |xx: f32| c.px[y.round() as usize * c.stride as usize + xx.round() as usize];
        // Hollowing an outer ring used to erase the inner one and all of the dither.
        assert_eq!(pixel(x + r - 1.0), 0);
        assert_eq!(pixel(x + r + 5.0), 0);
        let inside = Rect { x: x.round() as i32 - 10, y: y.round() as i32 - 10, w: 20, h: 20 };
        let mut black = 0;
        for yy in inside.y..inside.y + inside.h {
            for xx in inside.x..inside.x + inside.w {
                black += (c.px[(yy * c.stride + xx) as usize] == 0) as i32;
            }
        }
        assert!(black > 0 && black < 400);
    }
    #[test]
    fn the_nearest_dot_wins_and_controls_take_their_own_taps() {
        let mut v = MapView::default();
        for (i, x) in [(0, 100.0), (1, 160.0)] {
            v.hits.push(MapHit {
                rect: Rect {
                    x: 0,
                    y: 0,
                    w: 220,
                    h: 220,
                },
                kind: MapHitKind::Node(i),
                centre: (x, 100.0),
                radius: 59.0,
            });
        }
        assert_eq!(hit(&v, 141.0, 100.0), Some(MapHitKind::Node(1)));
        assert_eq!(hit(&v, 110.0, 100.0), Some(MapHitKind::Node(0)));
        assert_eq!(hit(&v, 240.0, 100.0), None);
        v.hits.push(MapHit {
            rect: Rect {
                x: 100,
                y: 90,
                w: 50,
                h: 30,
            },
            kind: MapHitKind::Control(BACK),
            centre: (0.0, 0.0),
            radius: 0.0,
        });
        assert_eq!(hit(&v, 110.0, 100.0), Some(MapHitKind::Control(BACK)));
    }
    #[test]
    fn trays_collapse_over_two_hundred_and_expand_in_both_orientations() {
        for (w, h) in [(1860, 2480), (2480, 1860)] {
            for n in [10, 250] {
                let (mut v, nav) = view(w, h, n, false);
                let mut c = Canvas::default();
                canvas_init(&mut c, w, h);
                render(&mut v, &mut c, &nav);
                assert_eq!(v.g.tray.len(), n as usize);
                assert_eq!(
                    v.hits.iter().any(|h| h.kind == MapHitKind::Control(EXPAND_TRAY)),
                    n > 200
                );
                v.tray_expanded = true;
                render(&mut v, &mut c, &nav);
                assert!(v.hits.iter().any(|h| matches!(h.kind, MapHitKind::Tray(_))));
                v.ini.show_unlinked = false;
                render(&mut v, &mut c, &nav);
                assert!(!v.hits.iter().any(|h| matches!(h.kind, MapHitKind::Tray(_))));
            }
        }
    }
}

/// Runner taps resolve the node on this host instead of hard-coding layout coordinates.
pub fn mapview_node_point(v: &MapView, pid: u32) -> Option<(f32, f32)> {
    let i = graph_node_of(&v.g, pid);
    if !v.open || i < 0 {
        return None;
    }
    let i = i as usize;
    v.hits.iter().find_map(|h| match h.kind {
        MapHitKind::Node(n) if n == i => Some(h.centre),
        MapHitKind::Tray(n) if n == i => Some(((h.rect.x + h.rect.w / 2) as f32, (h.rect.y + h.rect.h / 2) as f32)),
        _ => None,
    })
}
