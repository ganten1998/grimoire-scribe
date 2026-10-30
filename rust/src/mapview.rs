//! The Index map. This view owns its raster and interaction; the editor owns navigation and names.
use crate::prelude::*;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum MapResult {
    #[default]
    None,
    Open(u32),
    Target(FixedStr<NAV_TARGET_LEN>),
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
    Link(usize),
}
#[derive(Clone, Copy, Debug)]
struct MapHit {
    rect: Rect,
    kind: MapHitKind,
    centre: (f32, f32),
    radius: f32,
}
struct ForeignPlace {row:usize, notebook:String, label:String}
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
    pub list: bool,
    list_offset: usize,
    link_rows: Vec<NavLink>,
    notebook_name: String,
    foreign: Vec<ForeignPlace>,
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
const PAGES: u8 = 15;
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
    let bottom = if v.selected >= 0 { u(490.0) } else { u(340.0) };
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
        w: v.w - u(32.0) - if v.w > v.h { tray } else { 0 } - if v.foreign.is_empty(){0}else{u(470.0)},
        h: v.h - u(190.0) - bottom - if v.w <= v.h { tray } else { 0 },
    }
}
fn save(v: &mut MapView) {
    if !v.ini.writable || v.id.is_empty(){return;}
    if v.layout!=MapLayout::Done {
        // Seeds are display-only. A child may already have written its finished layout while
        // the UI waits to adopt it; only view preferences may be changed in that interval.
        graph_ini_update_preferences(v.id.as_str(),v.mode,v.ini.show_unlinked);
        return;
    }
    v.ini.mode = v.mode;
    graph_ini_store(&mut v.ini, &v.g);
    if v.ini.writable {graph_ini_write(v.id.as_str(), &mut v.ini);}
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
            if !bg_pending(BgKind::GraphLayout,v.id.as_str()) {
                bg_queue(BgKind::GraphLayout, Some(v.id.as_str()), Some(&path), a.dpi, false);
            }
            true
        }
    } else {
        graph_place_new(&mut v.g, &mut work_pump) >= 0
    };
    if ok && v.layout != MapLayout::Running {
        // A cached map can be reopened while its Tidy job is still running or queued.
        v.layout = if bg_pending(BgKind::GraphLayout,v.id.as_str()){MapLayout::Running}else{MapLayout::Done};
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
    v.link_rows=nav.links.iter().filter(|l|nb.page_ids.contains(&l.pid)).copied().collect();
    v.link_rows.sort_by_key(|l| (nav_external_target(l.target.as_str()).map_or(String::new(),|(id,_)|id.to_string()),l.pid,l.seed));
    v.notebook_name=nb.name.as_str().to_string();
    let mut seen=std::collections::HashSet::new();
    let mut books=std::collections::HashMap::<String,(NotebookInfo,Nav)>::new();
    for (row,l) in v.link_rows.iter().enumerate(){
        let Some((id,pid))=nav_external_target(l.target.as_str())else{continue};
        if !seen.insert(l.target.as_str().to_string()){continue;}
        let (other,names)=books.entry(id.to_string()).or_insert_with(||{let mut b=NotebookInfo::default();let mut n=Nav::default();store_load_info(id,&mut b);nav_load(id,&mut n);(b,n)});
        let number=other.page_ids.iter().position(|&p|p==pid).map_or(0,|n|n+1);
        let name=nav_name(names,pid);
        let title=if number==0 {tr("Page unavailable").to_string()}else if name.is_empty(){cformat!(tr("Page %d"),number as i32)}else{format!("{number} · {name}")};
        v.foreign.push(ForeignPlace{row,notebook:if other.name.is_empty(){tr("Notebook unavailable").to_string()}else{other.name.as_str().to_string()},label:title});
    }

    v.open = true;
    default_fit(v);
    v.last_input_ms = now_ms();
    mapview_draw(v, a, nav, full(v));
    app_invalidate(a, full(v), RefreshMode::Full);
    true
}
pub fn mapview_close(v: &mut MapView) {
    // Failed opens have not read the cache or established a complete model.
    if v.open {save(v);}
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
    if v.list {render_list(v,c,nav);return;}
    let pr = plot(v);
    render_foreign(v,c);
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
            let name = if v.g.nodes[i].named {label(v,nav,i)} else {cformat!(tr("p. %d"),page_number(v,i) as i32)};
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
    canvas_reset_clip(c);
    if !v.notebook_name.is_empty(){font_draw_left_name(c,FontFace::Bold,u(34.0) as f32,Rect::new(pr.x,u(195.0),pr.w,u(65.0)),&v.notebook_name,0);}
    let tabs=index_navigation(c,v.w,0);
    for (i,id) in [BACK,GRAPH,LIST,PAGES].into_iter().enumerate() {
        v.hits.push(MapHit{rect:tabs[i],kind:MapHitKind::Control(id),centre:(0.0,0.0),radius:0.0});
    }
    let bh=u(140.0);let gap=u(12.0);
    let row=v.h-u(160.0);let width=(v.w-u(80.0))/3;
    for (i,id,text) in [(0,ALL,tr("All")),(1,AROUND,tr("Around")),(2,FIT,tr("Fit"))] {
        button(v,c,Rect::new(u(20.0)+i*(width+gap),row,width,bh),id,text,IconKind::None,id!=AROUND||v.g.open>=0||v.selected>=0);
    }
    let bottom = v.h - if v.selected >= 0 { u(490.0) } else { u(340.0) };
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
    if phase == 1 && v.list {
        if let Some(d)=&mut v.drag {
            if (e.y-d.y).abs()>u(50.0) as f32 {d.moved=true;}
        }
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
        if v.list {let rows=((d.y-e.y)/u(130.0) as f32) as i32;v.list_offset=(v.list_offset as i32+rows).clamp(0,v.link_rows.len().saturating_sub(1) as i32) as usize;}
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
        Some(MapHitKind::Link(i)) => return MapResult::Target(v.link_rows[i].target),
        Some(MapHitKind::Control(id)) => match id {
            BACK => return MapResult::Close,
            LIST => {v.list=true;v.list_offset=0;},
            PAGES => return MapResult::Pages,
            GRAPH => {v.list=false;},
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
        if !v.foreign.is_empty(){paint(v,a,nav,RefreshMode::Ui);return MapResult::None;}
        mapview_draw(v, a, nav, full(v));
        label_dirty = v.labels.iter().fold(label_dirty, |r, &l| rect_union(r, l));
        let card = Rect {
            x: 0,
            y: v.h - u(490.0),
            w: v.w,
            h: u(490.0),
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
    if v.list{return;}
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
pub fn mapview_background_done(v: &mut MapView,id:&str, ok: bool) {
    if v.open && v.id.as_str()==id && v.layout == MapLayout::Running {
        if ok {
            v.job_done = true;
        } else {
            v.layout = MapLayout::Seed;
            v.job_done = false;
            v.needs_redraw = true;
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
            v.needs_redraw = true;
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
        "list" => out.push_str(if v.list{"1"}else{"0"}),
        "foreign"=>out.push_str(&v.foreign.len().to_string()),
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
                    Some(MapHitKind::Link(i)) => out.push_str(&format!("link.{}", i+1)),
                    Some(MapHitKind::Control(id)) => out.push_str(match id {
                        BACK => "back",
                        GRAPH => "graph",
                        LIST => "list",
                        PAGES => "pages",
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
    fn pan_and_pinch_preserve_the_world_anchor_and_settle_on_release() {
        let (mut v,nav)=view(1860,2480,4,true);
        let mut a=App{w:1860,h:2480,dpi:300.0,..Default::default()};canvas_init(&mut a.screen,a.w,a.h);
        mapview_draw(&mut v,&mut a,&nav,Rect::new(0,0,1860,2480));
        let (z,ox,oy)=(v.zoom,v.ox,v.oy);
        mapview_pointer(&mut v,&mut a,&nav,0,&InEvent{x:900.0,y:1000.0,t_ms:0,..Default::default()});
        mapview_pointer(&mut v,&mut a,&nav,1,&InEvent{x:1100.0,y:1100.0,t_ms:200,..Default::default()});
        assert!((v.ox-(ox-200.0/z)).abs()<0.001);assert!((v.oy-(oy-100.0/z)).abs()<0.001);
        mapview_pointer(&mut v,&mut a,&nav,2,&InEvent{x:1100.0,y:1100.0,t_ms:240,..Default::default()});
        assert!(v.drag.is_none());
        v.zoom=1.0;v.ox=0.0;v.oy=0.0;
        mapview_pinch(&mut v,&mut a,&InEvent{type_:InType::PinchBegin,x:800.0,y:1000.0,x2:1000.0,y2:1000.0,t_ms:300,..Default::default()});
        mapview_pinch(&mut v,&mut a,&InEvent{type_:InType::PinchMove,x:700.0,y:1000.0,x2:1100.0,y2:1000.0,t_ms:500,..Default::default()});
        assert_eq!(v.zoom,2.0);assert!((v.ox+900.0/v.zoom-900.0).abs()<0.001);assert!((v.oy+1000.0/v.zoom-1000.0).abs()<0.001);
        mapview_pinch(&mut v,&mut a,&InEvent{type_:InType::PinchEnd,t_ms:540,..Default::default()});
        assert!(v.drag.is_none()&&v.needs_redraw);
    }
    #[test]
    fn background_completion_belongs_only_to_its_notebook() {
        let mut v=MapView{open:true,layout:MapLayout::Running,..Default::default()};v.id.set("B");
        mapview_background_done(&mut v,"A",true);assert!(!v.job_done);assert_eq!(v.layout,MapLayout::Running);
        mapview_background_done(&mut v,"A",false);assert_eq!(v.layout,MapLayout::Running);
        mapview_background_done(&mut v,"B",true);assert!(v.job_done);
    }
    #[test]
    fn failed_open_and_pending_seed_never_replace_a_finished_cache() {
        let root=std::env::temp_dir().join(format!("grimoire-map-cache-{}",std::process::id()));
        store_init(root.to_str().unwrap());let mut nb=NotebookInfo::default();assert_eq!(store_create("Map","",PAPER_BLANK,&mut nb),0);
        let mut path=String::new();assert_eq!(graph_ini_path(nb.id.as_str(),&mut path),0);
        let original=b"# Grimoire graph\ngen=12345678\npos.1=1234,876\npin.1=1\nfuture=kept\n";
        assert_eq!(write_file_atomic(&path,original),0);
        let mut failed=MapView{id:nb.id,..Default::default()};mapview_close(&mut failed);
        assert_eq!(read_file(&path).unwrap(),original);
        let (mut v,_)=view(1860,2480,2,true);v.id=nb.id;v.layout=MapLayout::Running;
        let mut before=GraphIni::default();assert_eq!(graph_ini_read(nb.id.as_str(),&mut before),0);
        v.mode=GraphMode::Around;v.ini.show_unlinked=false;save(&mut v);
        let mut after=GraphIni::default();assert_eq!(graph_ini_read(nb.id.as_str(),&mut after),0);
        assert_eq!(after.pos,before.pos);assert_eq!(after.pins,before.pins);assert_eq!(after.unknown,before.unknown);
        assert_eq!(after.mode,GraphMode::Around);assert!(!after.show_unlinked);
        mapview_close(&mut v);graph_ini_read(nb.id.as_str(),&mut after);assert_eq!(after.pos,before.pos);
        std::fs::remove_file(&path).unwrap();
        let (mut seed,_)=view(1860,2480,2,true);seed.id=nb.id;seed.layout=MapLayout::Running;save(&mut seed);
        graph_ini_read(nb.id.as_str(),&mut after);assert!(after.pos.is_empty()&&!after.has_gen);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn cached_map_tidy_survives_close_reopen_completion_and_close() {
        let root=std::env::temp_dir().join(format!("grimoire-map-reopen-{}",std::process::id()));
        store_init(root.to_str().unwrap());let mut nb=NotebookInfo::default();assert_eq!(store_create("Map","",PAPER_BLANK,&mut nb),0);
        nb.page_ids=(1..=400).collect();nb.next_page_id=401;assert_eq!(store_save_info(&mut nb),0);
        let mut nav=Nav::default();nav_init(&mut nav);for pid in 1..=400{assert_eq!(nav_set_name(&mut nav,pid,"Named"),0);}
        assert_eq!(nav_save(nb.id.as_str(),&mut nav,&nb),0);
        let mut a=App{w:1860,h:2480,dpi:300.0,..Default::default()};canvas_init(&mut a.screen,a.w,a.h);
        let mut v=MapView{id:nb.id,open:true,w:a.w,h:a.h,..Default::default()};v.pages=nb.page_ids.clone();
        assert_eq!(graph_build(&mut v.g,&nb,&nav,1),0);assert!(graph_seed(&mut v.g,&mut ||{}));save(&mut v);
        assert!(layout(&mut v,&mut a,true));assert_eq!(v.layout,MapLayout::Running);
        mapview_close(&mut v);
        assert!(mapview_open(&mut v,&mut a,&nb,&nav,1,false));assert_eq!(v.layout,MapLayout::Running);
        let until=now_ms().wrapping_add(10000);
        while bg_busy() && (now_ms().wrapping_sub(until) as i32)<0 {
            let mut kind=-1;let mut id=FixedStr::new();let result=bg_poll_job(Some(&mut kind),Some(&mut id));
            if kind==BgKind::GraphLayout as i32 {mapview_background_done(&mut v,id.as_str(),result>0);}
            sleep_ms(2);
        }
        assert!(!bg_busy());assert!(v.job_done);
        mapview_tick(&mut v,&mut a,now_ms().wrapping_add(600));assert_eq!(v.layout,MapLayout::Done);
        let mut before=GraphIni::default();graph_ini_read(nb.id.as_str(),&mut before);assert_eq!(before.pos.len(),400);
        mapview_close(&mut v);let mut after=GraphIni::default();graph_ini_read(nb.id.as_str(),&mut after);
        assert_eq!(after.pos,before.pos);assert_eq!(after.gen,before.gen);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn failed_background_layout_reenables_tidy_visibly() {
        let mut v=MapView{open:true,layout:MapLayout::Running,..Default::default()};
        mapview_background_done(&mut v,"",false);
        assert_eq!(v.layout,MapLayout::Seed);assert!(v.needs_redraw);assert!(!v.job_done);
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

/// One navigation bar for the three Index views. Back always returns to the notebook.
pub fn index_navigation(c:&mut Canvas,w:i32,selected:usize)->[Rect;4] {
    let u=ui_px;let back=Rect::new(u(20.0),u(20.0),u(150.0),u(150.0));
    let gap=u(20.0);let first=u(200.0);let width=(w-first-u(20.0)-2*gap)/3;
    let mut tabs=[back;4];ui_button(c,back,IconKind::Back,"",ButtonStyle::Plain);
    for (i,text) in [tr("Graph"),tr("List"),tr("Pages")].into_iter().enumerate(){
        tabs[i+1]=Rect::new(first+i as i32*(width+gap),u(20.0),width,u(150.0));
        ui_button(c,tabs[i+1],IconKind::None,text,if i==selected{ButtonStyle::Selected}else{ButtonStyle::Plain});
    }
    canvas_hline(c,0,w,u(190.0),u(3.0),0);tabs
}
fn render_list(v:&mut MapView,c:&mut Canvas,nav:&Nav) {
    let tabs=index_navigation(c,v.w,1);
    for (i,id) in [BACK,GRAPH,LIST,PAGES].into_iter().enumerate(){v.hits.push(MapHit{rect:tabs[i],kind:MapHitKind::Control(id),centre:(0.0,0.0),radius:0.0});}
    let mut y=u(220.0);let mut group=String::new();
    if v.link_rows.is_empty(){font_draw_left_name(c,FontFace::Regular,ui_text_size(),Rect::new(u(50.0),y,v.w-u(100.0),u(120.0)),tr("No links in this notebook yet."),0);return;}
    for i in v.list_offset..v.link_rows.len(){
        let l=v.link_rows[i];let key=nav_external_target(l.target.as_str()).map_or(String::new(),|(id,_)|id.to_string());
        let title=if key.is_empty(){v.notebook_name.clone()}else{let mut nb=NotebookInfo::default();if store_load_info(&key,&mut nb)==0{nb.name.as_str().to_string()}else{tr("Notebook unavailable").to_string()}};
        if i==v.list_offset||key!=group {if y+u(210.0)>v.h{break;}canvas_fill(c,Rect::new(u(30.0),y,v.w-u(60.0),u(70.0)),225);font_draw_left_name(c,FontFace::Bold,ui_text_size(),Rect::new(u(50.0),y,v.w-u(100.0),u(70.0)),&title,0);y+=u(85.0);group=key;}
        if y+u(120.0)>v.h{break;}
        let from=v.pages.iter().position(|&p|p==l.pid).map_or(0,|n|n+1);
        let destination=v.foreign.iter().find(|f|f.row==i).map(|f|f.label.clone()).or_else(||nav_target_label(l.target.as_str())).unwrap_or_else(||if l.target=="index"{tr("Index").to_string()}else{let pid=l.target.as_str().strip_prefix('p').and_then(|p|p.parse().ok()).unwrap_or(0);let name=nav_name(nav,pid);if name.is_empty(){cformat!(tr("Page %d"),v.pages.iter().position(|&p|p==pid).map_or(0,|n|n+1)as i32)}else{name.to_string()}});
        let text=format!("{}  →  {}",cformat!(tr("Page %d"),from as i32),destination);
        let r=Rect::new(u(45.0),y,v.w-u(90.0),u(115.0));font_draw_left_name(c,FontFace::Regular,ui_text_size(),r,&text,0);canvas_hline(c,r.x,r.w,r.y+r.h,u(2.0),180);
        v.hits.push(MapHit{rect:r,kind:MapHitKind::Link(i),centre:(0.0,0.0),radius:0.0});y+=u(130.0);
    }
}
pub fn mapview_set_list(v:&mut MapView,a:&mut App,nav:&Nav) {v.list=true;paint(v,a,nav,RefreshMode::Full);}
pub fn mapview_control_point(v:&MapView,name:&str)->Option<(f32,f32)> {
    if let Some(n)=name.strip_prefix("link."){let i=n.parse::<usize>().ok()?.checked_sub(1)?;return v.hits.iter().find(|h|h.kind==MapHitKind::Link(i)).map(|h|((h.rect.x+h.rect.w/2)as f32,(h.rect.y+h.rect.h/2)as f32));}
    let id=match name{"back"=>BACK,"graph"=>GRAPH,"list"=>LIST,"pages"=>PAGES,"all"=>ALL,"around"=>AROUND,"fit"=>FIT,_=>return None};
    v.hits.iter().find(|h|h.kind==MapHitKind::Control(id)).map(|h|((h.rect.x+h.rect.w/2) as f32,(h.rect.y+h.rect.h/2) as f32))
}

fn render_foreign(v:&mut MapView,c:&mut Canvas) {
    if v.foreign.is_empty(){return;}
    let pr=plot(v);let x=pr.x+pr.w+u(15.0);let width=u(440.0);let mut y=u(220.0);let mut group=String::new();
    for i in 0..v.foreign.len(){
        let f=&v.foreign[i];let row=f.row;
        let key=nav_external_target(v.link_rows[row].target.as_str()).map(|(id,_)|id).unwrap_or("");
        if key!=group {
            if y+u(210.0)>pr.y+pr.h {break;}
            group=key.to_string();
            let header=Rect::new(x,y,width,u(90.0));canvas_fill(c,header,225);ui_frame(c,header,u(8.0),0,false,u(3.0));
            font_draw_left_name(c,FontFace::Bold,u(34.0)as f32,Rect::new(x+u(12.0),y,width-u(24.0),header.h),&f.notebook,0);y+=u(100.0);
        }
        if y+u(95.0)>pr.y+pr.h {
            button(v,c,Rect::new(x,y,width,u(90.0)),LIST,tr("List"),IconKind::None,true);break;
        }
        let r=Rect::new(x,y,width,u(85.0));
        ui_frame(c,r,u(8.0),0,false,u(2.0));font_draw_left_name(c,FontFace::Regular,u(32.0)as f32,Rect::new(x+u(10.0),y,width-u(20.0),r.h),&f.label,0);
        // Separate notebook boxes retain their own page identities; equal page IDs never merge.
        for l in &v.link_rows {if l.target==v.link_rows[row].target {
            if let Some(at)=v.g.nodes.iter().position(|n|n.pid==l.pid){if let Some(p)=location(v,at){let (sx,sy)=screen_point(v,p);canvas_set_clip(c,pr);canvas_line_aa(c,sx,sy,x as f32,(y+r.h/2)as f32,2.0,110);canvas_reset_clip(c);}}
        }}
        v.hits.push(MapHit{rect:r,kind:MapHitKind::Link(row),centre:(0.0,0.0),radius:0.0});y+=u(100.0);
    }
}
