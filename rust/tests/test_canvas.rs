use grimoire::prelude::*;

fn canvas(w: i32, h: i32, gray: u8) -> Canvas {
    let mut c = Canvas::default();
    assert_eq!(canvas_init(&mut c, w, h), 0);
    canvas_fill(&mut c, Rect { x: 0, y: 0, w, h }, gray);
    c
}

fn at(c: &Canvas, x: i32, y: i32) -> u8 {
    c.px[(y * c.stride + x) as usize]
}

#[test]
fn dotted_polyline_spaces_dots_along_corners_and_keeps_phase_when_split() {
    let path = [(10.0, 10.0), (18.0, 10.0), (18.0, 18.0)];
    let mut whole = canvas(32, 32, 100);
    let phase = canvas_dot_poly(&mut whole, &path, 0, 4, 2, 0);
    assert_eq!(phase, 0);
    for p in [(10, 10), (14, 10), (18, 10), (18, 14), (18, 18)] {
        assert_eq!(at(&whole, p.0, p.1), 0, "dot at {p:?}");
    }
    assert_eq!(at(&whole, 18, 12), 255); // the corner's 2 px gap has no dot
    assert_eq!(at(&whole, 12, 10), 255);
    assert!(whole.px.iter().all(|&p| p == 0 || p == 100 || p == 255));

    let mut pieces = canvas(32, 32, 100);
    let p1 = canvas_dot_poly(&mut pieces, &path[..=1], 0, 4, 2, 0);
    let p2 = canvas_dot_poly(&mut pieces, &path[1..], 0, 4, 2, p1);
    assert_eq!(p2, phase);
    assert_eq!(pieces.px, whole.px);
}

#[test]
fn dotted_polyline_honors_clip_and_keeps_only_binary_overlay_values() {
    let mut c = canvas(32, 32, 100);
    canvas_set_clip(&mut c, Rect { x: 15, y: 8, w: 8, h: 8 });
    canvas_dot_poly(&mut c, &[(10.0, 10.0), (22.0, 10.0)], 2, 6, 2, 0);
    for y in 0..32 {
        for x in 0..32 {
            if !(15..23).contains(&x) || !(8..16).contains(&y) {
                assert_eq!(at(&c, x, y), 100, "outside clip at {x},{y}");
            }
        }
    }
    assert!(c.px.iter().all(|&p| p == 0 || p == 100 || p == 255));
    assert!(c.px.contains(&0));
    assert!(c.px.contains(&255));
}
