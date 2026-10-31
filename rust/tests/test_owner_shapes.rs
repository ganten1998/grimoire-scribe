//! The owner's "Small shapes" page (2026-10-31, rc.11): three circles, four squares and three triangles
//! of growing size, then four parallelograms, drawn on the Kindle with Smallest shape at its old "Any".
//! Only the largest of each snapped: the recognizer's fixed 10 mm floor refused the rest, below what
//! the slider could set. Since 2.0.0-rc.12 Smallest shape is that floor, 3 to 40 mm.
use grimoire::prelude::*;

fn owner_page() -> Box<Page> {
    let data = include_bytes!("fixtures/owner_small_shapes.qpg");
    let mut p = None;
    assert_eq!(page_deserialize(data, &mut p), 0);
    p.unwrap()
}

fn kinds(p: &Page, floor_mm: f32) -> Vec<ShapeKind> {
    p.strokes
        .iter()
        .map(|s| {
            // The owner's settings: Four sides uneven allowed, Round shapes as drawn, at zoom 1.
            let mut prm = shape_params(300.0, 1.0, p.paper, 1860);
            prm.quad_mode = 0;
            prm.circle_mode = 0;
            prm.min_mm = floor_mm;
            shape_recognize(&s.pts, &prm).kind
        })
        .collect()
}

#[test]
fn the_default_floor_keeps_the_old_verdicts() {
    use ShapeKind::*;
    let p = owner_page();
    assert_eq!(p.strokes.len(), 14);
    // Strokes 0-2 circles, 3-6 squares, 7-9 triangles, 10-13 parallelograms (all four snap since rc.12).
    assert_eq!(
        kinds(&p, SHAPE_CLOSED_ACROSS_MM),
        vec![None, None, Circle, None, None, None, Square, None, None, Triangle, Quad, Quad, Quad, Quad]
    );
}

#[test]
fn a_low_floor_snaps_the_small_ones_as_what_they_are() {
    use ShapeKind::*;
    let k = kinds(&owner_page(), 4.0);
    for (i, kind) in k.iter().enumerate() {
        let ok = match i {
            0..=2 => matches!(kind, Circle | Ellipse),
            3..=6 => matches!(kind, Square | Rectangle),
            7..=9 => *kind == Triangle,
            _ => *kind == Quad,
        };
        assert!(ok, "stroke {i}: {kind:?} (all: {k:?})");
    }
}

#[test]
fn the_floor_is_clamped() {
    assert_eq!(shape_floor(0.0), SHAPE_MIN_LO_MM);
    assert_eq!(shape_floor(100.0), SHAPE_MIN_HI_MM);
    assert_eq!(shape_floor(f32::NAN), SHAPE_CLOSED_ACROSS_MM);
    assert_eq!(shape_floor(10.0), 10.0);
}
