// test_history: per-page undo and redo (history.rs). Ink steps behave as they always did; a Link step
// (phase 2) moves between the stacks like any other, leaves the page untouched and hands the editor
// its records.
mod testlib;
use grimoire::prelude::*;
use testlib::*;

// A stroke of one point at (x, y).
fn dot(x: f32, y: f32) -> Box<Stroke> {
    let mut prm = ToolParams::default();
    tool_params_default(TOOL_BALLPOINT, &mut prm);
    let mut s = stroke_new(TOOL_BALLPOINT, &prm, 1);
    stroke_append(&mut s, &PenSample { x, y, pressure: 0.5, tilt_x: 0.0, tilt_y: 0.0, t_ms: 0 }, 300.0);
    s
}
fn ids(p: &Page) -> Vec<StrokeId> {
    p.strokes.iter().map(|s| stroke_id(s)).collect()
}
fn rec(seed: u32, target: &str) -> LinkRec {
    let mut t = FixedStr::<NAV_TARGET_LEN>::default();
    t.set(target);
    LinkRec { seed, target: t, area: Rect { x: 10, y: 20, w: 30, h: 40 }, has_area: true, ..Default::default() }
}
fn made(pid: u32, seed: u32, target: &str) -> LinkEdit {
    LinkEdit { pid, records: vec![(None, Some(rec(seed, target)))] }
}
fn fresh() -> History {
    let mut h = History::default();
    history_init(&mut h);
    h
}

// Add, Erase, Clear and Paper as before phase 2: an action owns its strokes exactly when they are not
// on the page, and the dirty rect is their boxes (or the page for Clear and Paper).
fn ink_steps_unchanged() {
    let mut p = page_new(1000, 1000, PAPER_BLANK);
    let mut h = fresh();
    check!(h.limit == 500);
    let (a, b, c) = (dot(100.0, 100.0), dot(300.0, 300.0), dot(500.0, 500.0));
    let (ba, bc) = (a.bbox, c.bbox);
    let before = vec![stroke_id(&a), stroke_id(&b), stroke_id(&c)];
    for (i, s) in [a, b, c].into_iter().enumerate() {
        history_push_add(&mut h, i as i32, &s);
        page_add(&mut p, s);
    }
    check!(history_undo_kind(&h) == Some(ActKind::Add) && history_redo_kind(&h).is_none());
    let mut d = RECT_EMPTY;
    check!(history_undo(&mut h, &mut p, &mut d) && d == bc && ids(&p) == before[..2].to_vec());
    check!(h.redo[0].owns && h.redo[0].owned.len() == 1 && h.redo[0].link.is_none());
    check!(history_moved_link(&h, true).is_none());
    check!(history_redo(&mut h, &mut p, &mut d) && d == bc && ids(&p) == before);
    let rm_c = page_remove(&mut p, 2);
    let rm_a = page_remove(&mut p, 0);
    history_push_erase(&mut h, &[0, 2], vec![rm_a, rm_c]);
    check!(history_undo_kind(&h) == Some(ActKind::Erase));
    check!(history_undo(&mut h, &mut p, &mut d) && d == rect_union(ba, bc) && ids(&p) == before);
    check!(history_redo(&mut h, &mut p, &mut d) && ids(&p) == vec![before[1]]);
    history_push_clear(&mut h, &mut p);
    check!(p.strokes.is_empty() && history_undo_kind(&h) == Some(ActKind::Clear));
    check!(history_undo(&mut h, &mut p, &mut d) && d == Rect { x: 0, y: 0, w: 1000, h: 1000 });
    check!(ids(&p) == vec![before[1]]);
    history_push_paper(&mut h, PAPER_BLANK, PAPER_GRID);
    p.paper = PAPER_GRID;
    check!(history_undo_kind(&h) == Some(ActKind::Paper));
    check!(history_undo(&mut h, &mut p, &mut d) && p.paper == PAPER_BLANK && d.w == 1000);
    check!(history_redo(&mut h, &mut p, &mut d) && p.paper == PAPER_GRID);
    history_free(&mut h);
    page_free(p);
}

// A Link step moves to redo and back with an empty dirty rect, the page untouched, and its records
// readable on the stack it moved to.
fn link_step_moves() {
    let mut p = page_new(1000, 1000, PAPER_BLANK);
    let s = dot(200.0, 200.0);
    let sid = stroke_id(&s);
    page_add(&mut p, s);
    let mut h = fresh();
    history_push_link(&mut h, made(7, 0xbeef, "p3"));
    check!(history_undo_kind(&h) == Some(ActKind::Link));
    check!(history_moved_link(&h, false).map(|e| e.pid) == Some(7));
    let mut d = Rect { x: 1, y: 1, w: 1, h: 1 };
    check!(history_undo(&mut h, &mut p, &mut d));
    check!(d == RECT_EMPTY && ids(&p) == vec![sid] && p.paper == PAPER_BLANK);
    check!(history_undo_kind(&h).is_none() && history_redo_kind(&h) == Some(ActKind::Link));
    let e = history_moved_link(&h, true);
    check!(e.is_some_and(|e| e.pid == 7 && e.records.len() == 1));
    check!(e.is_some_and(|e| e.records[0].0.is_none() && e.records[0].1 == Some(rec(0xbeef, "p3"))));
    check!(e.is_some_and(|e| e.records[0].1.is_some_and(|r| r.target == "p3")));
    d = Rect { x: 1, y: 1, w: 1, h: 1 };
    check!(history_redo(&mut h, &mut p, &mut d) && d == RECT_EMPTY && ids(&p) == vec![sid]);
    check!(history_moved_link(&h, false).is_some_and(|e| e.pid == 7));
    check!(history_moved_link(&h, true).is_none());
    history_free(&mut h);
    page_free(p);
}

// Add, Link, Add: three undos come off in reverse order, three redos in order.
fn interleaved_with_ink() {
    let mut p = page_new(1000, 1000, PAPER_BLANK);
    let mut h = fresh();
    let a = dot(100.0, 100.0);
    let (ia, ba) = (stroke_id(&a), a.bbox);
    history_push_add(&mut h, 0, &a);
    page_add(&mut p, a);
    history_push_link(&mut h, made(1, 5, "index"));
    let b = dot(600.0, 600.0);
    let (ib, bb) = (stroke_id(&b), b.bbox);
    history_push_add(&mut h, 1, &b);
    page_add(&mut p, b);
    let mut d = RECT_EMPTY;
    check!(history_undo_kind(&h) == Some(ActKind::Add));
    check!(history_undo(&mut h, &mut p, &mut d) && d == bb && ids(&p) == vec![ia]);
    check!(history_undo_kind(&h) == Some(ActKind::Link));
    check!(history_undo(&mut h, &mut p, &mut d) && d == RECT_EMPTY && ids(&p) == vec![ia]);
    check!(history_moved_link(&h, true).is_some_and(|e| e.records[0].1.is_some_and(|r| r.seed == 5)));
    check!(history_undo(&mut h, &mut p, &mut d) && d == ba && p.strokes.is_empty());
    check!(!history_undo(&mut h, &mut p, &mut d) && history_undo_kind(&h).is_none());
    check!(history_redo_kind(&h) == Some(ActKind::Add));
    check!(history_redo(&mut h, &mut p, &mut d) && d == ba && ids(&p) == vec![ia]);
    check!(history_redo_kind(&h) == Some(ActKind::Link));
    check!(history_redo(&mut h, &mut p, &mut d) && d == RECT_EMPTY && ids(&p) == vec![ia]);
    check!(history_redo(&mut h, &mut p, &mut d) && d == bb && ids(&p) == vec![ia, ib]);
    check!(!history_redo(&mut h, &mut p, &mut d));
    history_free(&mut h);
    page_free(p);
}

// A group of N records (a xN row of the bar) is one step.
fn group_is_one_step() {
    let mut p = page_new(1000, 1000, PAPER_BLANK);
    let mut h = fresh();
    let e = LinkEdit {
        pid: 2,
        records: vec![(Some(rec(1, "p3")), None), (Some(rec(2, "p3")), None), (Some(rec(3, "p3")), None)],
    };
    history_push_link(&mut h, e);
    check!(h.undo.len() == 1);
    let mut d = RECT_EMPTY;
    check!(history_undo(&mut h, &mut p, &mut d) && h.undo.is_empty() && h.redo.len() == 1);
    check!(history_moved_link(&h, true).is_some_and(|e| e.records.len() == 3 && e.records[2].0.is_some_and(|r| r.seed == 3)));
    history_free(&mut h);
    page_free(p);
}

// At limit 2 a third push drops the oldest step, a Link step's records with it.
fn eviction() {
    let mut h = fresh();
    h.limit = 2;
    history_push_link(&mut h, made(1, 1, "p2"));
    history_push_link(&mut h, made(1, 2, "p2"));
    history_push_link(&mut h, made(1, 3, "p2"));
    check!(h.undo.len() == 2);
    let seeds: Vec<u32> =
        h.undo.iter().map(|a| a.link.as_ref().and_then(|e| e.records[0].1).map_or(0, |r| r.seed)).collect();
    check!(seeds == vec![2, 3]);
    history_free(&mut h);
}

// The drop-last helpers remove only the top step; no records push nothing; a push clears redo.
fn drops_and_pushes() {
    let mut p = page_new(1000, 1000, PAPER_BLANK);
    let mut h = fresh();
    history_push_link(&mut h, LinkEdit { pid: 1, records: Vec::new() });
    check!(h.undo.is_empty() && history_undo_kind(&h).is_none());
    history_push_paper(&mut h, PAPER_BLANK, PAPER_GRID);
    history_push_link(&mut h, made(1, 1, "p2"));
    history_push_link(&mut h, made(1, 2, "p2"));
    let mut d = RECT_EMPTY;
    check!(history_undo(&mut h, &mut p, &mut d) && history_undo(&mut h, &mut p, &mut d));
    check!(h.undo.len() == 1 && h.redo.len() == 2);
    // the top of redo is the step undone last (seed 1)
    history_drop_last_redo(&mut h);
    check!(h.redo.len() == 1 && h.undo.len() == 1);
    check!(h.redo[0].link.as_ref().is_some_and(|e| e.records[0].1.is_some_and(|r| r.seed == 2)));
    check!(history_redo(&mut h, &mut p, &mut d) && h.undo.len() == 2);
    history_drop_last_undo(&mut h);
    check!(h.undo.len() == 1 && history_undo_kind(&h) == Some(ActKind::Paper));
    history_drop_last_redo(&mut h); // (an empty stack: nothing)
    history_drop_last_undo(&mut h);
    history_drop_last_undo(&mut h);
    check!(h.undo.is_empty() && h.redo.is_empty());
    history_push_link(&mut h, made(1, 4, "p2"));
    check!(history_undo(&mut h, &mut p, &mut d) && history_can_redo(&h));
    history_push_link(&mut h, made(1, 5, "p2"));
    check!(!history_can_redo(&h) && h.undo.len() == 1);
    history_free(&mut h);
    page_free(p);
}

#[test]
fn test_history() {
    run!(ink_steps_unchanged);
    run!(link_step_moves);
    run!(interleaved_with_ink);
    run!(group_is_one_step);
    run!(eviction);
    run!(drops_and_pushes);
    done();
}
