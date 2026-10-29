use grimoire::prelude::*;

#[allow(dead_code)]
#[path = "../src/bin/tgstudy.rs"]
mod tgstudy;

#[test]
fn trace_parser_handles_header_frames_pen_outcomes_and_bad_lines() {
    match touchgest_trace_parse_line("# grimoire touch-trace 1, rotation 3, 1860x2480, dpi 300")
        .unwrap()
    {
        TgTraceLine::Header {
            rotation,
            width,
            height,
            dpi,
        } => assert_eq!((rotation, width, height, dpi), (3, 1860, 2480, 300)),
        other => panic!("unexpected header parse: {other:?}"),
    }
    match touchgest_trace_parse_line("F 42 2:7:120:240:18:1").unwrap() {
        TgTraceLine::Frame(f) => {
            assert_eq!(f.k_ms, 42);
            assert_eq!(f.n, 1);
            assert_eq!(
                (
                    f.c[0].slot,
                    f.c[0].id,
                    f.c[0].x,
                    f.c[0].y,
                    f.c[0].major,
                    f.c[0].tool
                ),
                (2, 7, 120.0, 240.0, 18, 1)
            );
        }
        other => panic!("unexpected frame parse: {other:?}"),
    }
    assert!(matches!(
        touchgest_trace_parse_line("P 50 down"),
        Ok(TgTraceLine::Pen {
            k_ms: 50,
            what: TgTracePen::Down
        })
    ));
    match touchgest_trace_parse_line("E 54 shape fired undo 41 78 160").unwrap() {
        TgTraceLine::Outcome { k_ms, text } => {
            assert_eq!((k_ms, text), (54, "shape fired undo 41 78 160"))
        }
        other => panic!("unexpected outcome parse: {other:?}"),
    }
    assert!(matches!(
        touchgest_trace_parse_line("# note"),
        Ok(TgTraceLine::Ignore)
    ));
    assert!(touchgest_trace_parse_line("F 1 broken-contact").is_err());
}

#[test]
fn splitter_uses_four_second_contact_frame_gaps() {
    let events = tgstudy::parse_trace("F 100\nF 4099\nF 8099\n").unwrap();
    let split = tgstudy::split_blocks(events);
    assert_eq!(split.frame_counts, [2, 1]);
    assert_eq!(split.block_for_event, [0, 0, 1]);
}

#[test]
fn synthetic_trace_replays_expected_swipe_and_tap_under_both_tunes() {
    let events =
        tgstudy::parse_trace(include_str!("../../tests/fixtures/touch/synthetic.txt")).unwrap();
    let split = tgstudy::split_blocks(events);
    assert_eq!(split.frame_counts.len(), 2);
    let names = vec!["A up".to_owned(), "E tap".to_owned()];
    for tune in [TG_TUNE_V1, TG_TUNE] {
        let result = tgstudy::replay(&split, &names, &tune);
        assert_eq!(result.len(), 2);
        assert!(
            result[0].sessions.iter().any(|s| s.outcome == "fired 2.up"),
            "{:?}",
            result[0].sessions
        );
        assert!(
            result[1]
                .sessions
                .iter()
                .any(|s| s.outcome == "fired 2.tap"),
            "{:?}",
            result[1].sessions
        );
        assert_eq!(result[0].trace_outcomes, ["E 2940 fired 2.up"]);
        assert_eq!(result[1].trace_outcomes, ["E 8380 fired 2.tap"]);
    }
}
