//! Replay a Kindle touch-trace through the production touchgest recognizer.
//!
//!   tgstudy TRACE [--tune v1|new] [--set FIELD=VALUE ...]
//!                 [--sweep FIELD=A:B:STEP] [--blocks BLOCKS.txt]

use grimoire::input::InType;
use grimoire::touchgest::*;
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;

#[derive(Clone, Debug)]
pub enum TraceEvent {
    Header {
        rotation: i32,
        width: i32,
        height: i32,
        dpi: i32,
    },
    Frame(TgFrame),
    Pen {
        k_ms: u32,
        what: TgTracePen,
    },
    Outcome {
        k_ms: u32,
        text: String,
    },
}

#[derive(Clone, Debug, Default)]
pub struct SplitTrace {
    pub events: Vec<TraceEvent>,
    pub block_for_event: Vec<usize>,
    pub frame_counts: Vec<usize>,
}

#[derive(Clone, Debug)]
pub struct SessionResult {
    pub k_ms: u32,
    pub outcome: String,
    pub gesture: Option<i32>,
    pub measure: String,
}

#[derive(Clone, Debug, Default)]
pub struct BlockResult {
    pub frames: usize,
    pub sessions: Vec<SessionResult>,
    pub trace_outcomes: Vec<String>,
}

#[derive(Clone, Debug)]
struct Sweep {
    field: String,
    start: f64,
    end: f64,
    step: f64,
}

#[derive(Clone, Debug)]
struct Options {
    trace: String,
    tune: TgTune,
    tune_name: &'static str,
    sweep: Option<Sweep>,
    blocks: Option<String>,
}

const DEFAULT_BLOCKS: [&str; 12] = [
    "A up",
    "B down",
    "C left",
    "D right",
    "E tap",
    "F pinch",
    "G pan",
    "H after pen",
    "I light swipes",
    "J palm",
    "K reading",
    "L one finger",
];

pub fn parse_trace(text: &str) -> Result<Vec<TraceEvent>, String> {
    let mut events = Vec::new();
    for (line_no, raw) in text.lines().enumerate() {
        let line = raw.trim_end_matches('\r');
        match touchgest_trace_parse_line(line) {
            Ok(TgTraceLine::Ignore) => {}
            Ok(TgTraceLine::Header {
                rotation,
                width,
                height,
                dpi,
            }) => events.push(TraceEvent::Header {
                rotation,
                width,
                height,
                dpi,
            }),
            Ok(TgTraceLine::Frame(frame)) => events.push(TraceEvent::Frame(frame)),
            Ok(TgTraceLine::Pen { k_ms, what }) => events.push(TraceEvent::Pen { k_ms, what }),
            Ok(TgTraceLine::Outcome { k_ms, text }) => events.push(TraceEvent::Outcome {
                k_ms,
                text: text.to_owned(),
            }),
            Err(e) => return Err(format!("line {}: {e:?}", line_no + 1)),
        }
    }
    Ok(events)
}

/// Split at gaps of 4 seconds or more between contact-frame records. Headers and P/E lines remain
/// attached to the current block; only F timestamps decide the block boundary.
pub fn split_blocks(events: Vec<TraceEvent>) -> SplitTrace {
    let mut split = SplitTrace {
        events,
        ..Default::default()
    };
    let mut block = 0usize;
    let mut previous_frame = None;
    for event in &split.events {
        if let TraceEvent::Frame(frame) = event {
            if let Some(previous) = previous_frame {
                let gap = frame.k_ms.wrapping_sub(previous);
                if gap >= 4000 && (gap as i32) > 0 {
                    block += 1;
                }
            }
            previous_frame = Some(frame.k_ms);
            while split.frame_counts.len() <= block {
                split.frame_counts.push(0);
            }
            split.frame_counts[block] += 1;
        }
        split.block_for_event.push(block);
    }
    split
}

fn block_names(path: Option<&str>, count: usize) -> Result<Vec<String>, String> {
    let mut names = if let Some(path) = path {
        fs::read_to_string(path)
            .map_err(|e| format!("cannot read block names {path}: {e}"))?
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .map(str::to_owned)
            .collect::<Vec<_>>()
    } else {
        DEFAULT_BLOCKS.iter().map(|s| (*s).to_owned()).collect()
    };
    while names.len() < count {
        names.push(format!("Block {}", names.len() + 1));
    }
    names.truncate(count);
    Ok(names)
}

fn set_field(t: &mut TgTune, field: &str, value: &str) -> Result<(), String> {
    macro_rules! u32_field {
        ($name:ident) => {{
            t.$name = value
                .parse()
                .map_err(|_| format!("bad value for {}: {}", field, value))?;
        }};
    }
    macro_rules! i32_field {
        ($name:ident) => {{
            t.$name = value
                .parse()
                .map_err(|_| format!("bad value for {}: {}", field, value))?;
        }};
    }
    macro_rules! f32_field {
        ($name:ident) => {{
            let v: f32 = value
                .parse()
                .map_err(|_| format!("bad value for {}: {}", field, value))?;
            if !v.is_finite() {
                return Err(format!("non-finite value for {field}"));
            }
            t.$name = v;
        }};
    }
    macro_rules! bool_field {
        ($name:ident) => {{
            t.$name = match value {
                "1" | "true" | "on" => true,
                "0" | "false" | "off" => false,
                _ => return Err(format!("bad boolean for {}: {}", field, value)),
            };
        }};
    }
    match field {
        "land_ms" => u32_field!(land_ms),
        "palm_ms" => u32_field!(palm_ms),
        "pen_window_ms" => u32_field!(pen_window_ms),
        "pen_gone_ms" => u32_field!(pen_gone_ms),
        "far_mm" => f32_field!(far_mm),
        "rest_ms" => u32_field!(rest_ms),
        "rest_mm" => f32_field!(rest_mm),
        "commit_spread" => f32_field!(commit_spread),
        "commit_px" => f32_field!(commit_px),
        "zoom_ratio" => f32_field!(zoom_ratio),
        "zoom_opposite_mm" => f32_field!(zoom_opposite_mm),
        "zoom_upgrade" => f32_field!(zoom_upgrade),
        "zoom_after_pen" => bool_field!(zoom_after_pen),
        "grace_ms" => u32_field!(grace_ms),
        "regrab_mm" => f32_field!(regrab_mm),
        "tan_axis" => f32_field!(tan_axis),
        "swipe_mm" => f32_field!(swipe_mm),
        "finger_mm" => f32_field!(finger_mm),
        "cos_min" => f32_field!(cos_min),
        "finger_ratio" => f32_field!(finger_ratio),
        "spread_max" => f32_field!(spread_max),
        "spread_opposite" => bool_field!(spread_opposite),
        "swipe_ms" => u32_field!(swipe_ms),
        "follow_ms" => u32_field!(follow_ms),
        "tap_lift_ms" => u32_field!(tap_lift_ms),
        "tap_ms" => u32_field!(tap_ms),
        "tap_mm" => f32_field!(tap_mm),
        "apart_min_mm" => f32_field!(apart_min_mm),
        "apart_max_mm" => f32_field!(apart_max_mm),
        "palm_major" => i32_field!(palm_major),
        "hover_far_mm" => f32_field!(hover_far_mm),
        "hover_age_ms" => u32_field!(hover_age_ms),
        _ => return Err(format!("unknown TgTune field: {field}")),
    }
    Ok(())
}

fn set_sweep_value(t: &mut TgTune, field: &str, value: f64) -> Result<(), String> {
    if !value.is_finite() {
        return Err("sweep value is not finite".to_owned());
    }
    set_field(t, field, &value.to_string())
}

fn parse_sweep(value: &str) -> Result<Sweep, String> {
    let (field, values) = value
        .split_once('=')
        .ok_or_else(|| "--sweep needs FIELD=A:B:STEP".to_owned())?;
    let mut parts = values.split(':');
    let start = parts
        .next()
        .unwrap_or("")
        .parse::<f64>()
        .map_err(|_| "bad sweep start".to_owned())?;
    let end = parts
        .next()
        .unwrap_or("")
        .parse::<f64>()
        .map_err(|_| "bad sweep end".to_owned())?;
    let step = parts
        .next()
        .unwrap_or("")
        .parse::<f64>()
        .map_err(|_| "bad sweep step".to_owned())?;
    if parts.next().is_some()
        || !start.is_finite()
        || !end.is_finite()
        || !step.is_finite()
        || step == 0.0
        || (end - start) * step < 0.0
    {
        return Err("invalid sweep range".to_owned());
    }
    Ok(Sweep {
        field: field.to_owned(),
        start,
        end,
        step,
    })
}

fn parse_args(args: &[String]) -> Result<Option<Options>, String> {
    if args.iter().any(|a| a == "--help" || a == "-h") {
        return Ok(None);
    }
    let mut trace = None;
    let mut tune_name = "new";
    let mut tune = TG_TUNE;
    let mut sweep = None;
    let mut blocks = None;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--tune" => {
                i += 1;
                match args.get(i).map(String::as_str) {
                    Some("v1") => {
                        tune = TG_TUNE_V1;
                        tune_name = "v1";
                    }
                    Some("new") => {
                        tune = TG_TUNE;
                        tune_name = "new";
                    }
                    Some(v) => return Err(format!("unknown tune: {v}")),
                    None => return Err("--tune needs v1 or new".to_owned()),
                }
            }
            "--set" => {
                i += 1;
                let assignment = args
                    .get(i)
                    .ok_or_else(|| "--set needs FIELD=VALUE".to_owned())?;
                let (field, value) = assignment
                    .split_once('=')
                    .ok_or_else(|| "--set needs FIELD=VALUE".to_owned())?;
                set_field(&mut tune, field, value)?;
            }
            "--sweep" => {
                i += 1;
                if sweep.is_some() {
                    return Err("only one --sweep is supported".to_owned());
                }
                sweep = Some(parse_sweep(
                    args.get(i)
                        .ok_or_else(|| "--sweep needs FIELD=A:B:STEP".to_owned())?,
                )?);
            }
            "--blocks" => {
                i += 1;
                blocks = Some(
                    args.get(i)
                        .ok_or_else(|| "--blocks needs a file".to_owned())?
                        .clone(),
                );
            }
            option if option.starts_with('-') => return Err(format!("unknown option: {option}")),
            path if trace.is_none() => trace = Some(path.to_owned()),
            path => return Err(format!("unexpected argument: {path}")),
        }
        i += 1;
    }
    Ok(Some(Options {
        trace: trace.ok_or_else(|| "missing TRACE".to_owned())?,
        tune,
        tune_name,
        sweep,
        blocks,
    }))
}

fn outcome_name(outcome: TgOutcome) -> (String, Option<i32>) {
    match outcome {
        TgOutcome::None => ("none".to_owned(), None),
        TgOutcome::Fired(id) => (format!("fired {}", touchgest_gesture_name(id)), Some(id)),
        TgOutcome::Rejected(reason) => {
            (format!("rejected {}", touchgest_reject_name(reason)), None)
        }
    }
}

fn collect_output(
    g: &TouchGest,
    out: &TgOut,
    k_ms: u32,
    block: &mut BlockResult,
    pinch_class: &mut Option<i32>,
) {
    let mut ended = false;
    for event in &out.ev[..out.n] {
        if event.type_ == InType::PinchMove {
            *pinch_class = Some(event.value);
        } else if event.type_ == InType::PinchEnd {
            ended = true;
        }
    }
    if out.outcome != TgOutcome::None {
        let (name, gesture) = outcome_name(out.outcome);
        let mut measure = String::new();
        let _ = touchgest_log_measure(g, true, &mut measure);
        block.sessions.push(SessionResult {
            k_ms,
            outcome: name,
            gesture,
            measure,
        });
        *pinch_class = None;
    } else if ended {
        let name = match *pinch_class {
            Some(1) => Some("zoom"),
            Some(2) => Some("pan"),
            _ => None,
        };
        if let Some(name) = name {
            let mut measure = String::new();
            let _ = touchgest_log_measure(g, true, &mut measure);
            block.sessions.push(SessionResult {
                k_ms,
                outcome: name.to_owned(),
                gesture: None,
                measure,
            });
        }
        *pinch_class = None;
    }
}

fn apply_pen(pen: &mut TgPen, k_ms: u32, what: TgTracePen) {
    pen.hover = None;
    pen.last = (-1_000_000.0, -1_000_000.0); // trace files deliberately omit the pen's coordinates
    match what {
        TgTracePen::Prox => {
            pen.prox = true;
            pen.touching = false;
        }
        TgTracePen::Down => {
            pen.prox = true;
            pen.touching = true;
        }
        TgTracePen::Up => {
            pen.prox = true;
            pen.touching = false;
        }
        TgTracePen::Leave => {
            pen.prox = false;
            pen.touching = false;
            pen.left_ms = k_ms;
        }
    }
}

fn advance_ticks(
    g: &mut TouchGest,
    pen: &TgPen,
    from: &mut u32,
    to: u32,
    block: &mut BlockResult,
    class: &mut Option<i32>,
) {
    let mut gap = to.wrapping_sub(*from);
    if gap as i32 <= 0 {
        return;
    }
    while gap >= 10 {
        *from = from.wrapping_add(10);
        let mut out = TgOut::default();
        touchgest_tick(g, *from, &mut out);
        collect_output(g, &out, *from, block, class);
        gap -= 10;
    }
    *from = to;
    let _ = pen;
}

pub fn replay(split: &SplitTrace, names: &[String], tune: &TgTune) -> Vec<BlockResult> {
    let mut blocks = vec![BlockResult::default(); split.frame_counts.len()];
    if blocks.is_empty() {
        return blocks;
    }
    let dpi = split
        .events
        .iter()
        .find_map(|e| match e {
            TraceEvent::Header { dpi, .. } if *dpi > 50 => Some(*dpi as f32),
            _ => None,
        })
        .unwrap_or(300.0);
    let mut g = TouchGest::default();
    touchgest_init(&mut g, dpi);
    touchgest_set_tune(&mut g, tune);
    let mut pen = TgPen {
        last: (-1_000_000.0, -1_000_000.0),
        left_ms: 0u32.wrapping_sub(10000),
        ..Default::default()
    };
    let mut clock = None;
    let mut class = None;
    let mut active_block = 0usize;
    let first_label = names.first().map(String::as_str).unwrap_or("");
    let first_pan = first_label.to_ascii_lowercase().contains("pan");
    touchgest_set_view(
        &mut g,
        TgView {
            scrolls_v: first_pan,
            scrolls_h: first_pan,
            zoom_ok: true,
            gestures_on: true,
            ..Default::default()
        },
    );
    for (index, event) in split.events.iter().enumerate() {
        let block_no = split.block_for_event[index].min(blocks.len() - 1);
        if active_block != block_no {
            active_block = block_no;
            let label = names.get(block_no).map(String::as_str).unwrap_or("");
            let pan = block_no == 6 || label.to_ascii_lowercase().contains("pan");
            touchgest_set_view(
                &mut g,
                TgView {
                    scrolls_v: pan,
                    scrolls_h: pan,
                    zoom_ok: true,
                    gestures_on: true,
                    ..Default::default()
                },
            );
        }
        let k_ms = match event {
            TraceEvent::Frame(frame) => Some(frame.k_ms),
            TraceEvent::Pen { k_ms, .. } | TraceEvent::Outcome { k_ms, .. } => Some(*k_ms),
            TraceEvent::Header { .. } => None,
        };
        if let (Some(now), Some(target)) = (clock.as_mut(), k_ms) {
            advance_ticks(&mut g, &pen, now, target, &mut blocks[block_no], &mut class);
        } else if let Some(target) = k_ms {
            clock = Some(target);
        }
        match event {
            TraceEvent::Header { .. } => {}
            TraceEvent::Frame(frame) => {
                blocks[block_no].frames += 1;
                let mut out = TgOut::default();
                touchgest_frame(&mut g, frame, &pen, &mut out);
                collect_output(&g, &out, frame.k_ms, &mut blocks[block_no], &mut class);
                clock = Some(frame.k_ms);
            }
            TraceEvent::Pen { k_ms, what } => apply_pen(&mut pen, *k_ms, *what),
            TraceEvent::Outcome { k_ms, text } => blocks[block_no]
                .trace_outcomes
                .push(format!("E {k_ms} {text}")),
        }
    }
    blocks
}

fn intended(block: usize, result: &SessionResult) -> (bool, bool) {
    let Some(id) = result.gesture else {
        return match block {
            5 => (result.outcome == "zoom", false),
            6 => (result.outcome == "pan", false),
            7 => (result.outcome == "zoom" || result.outcome == "pan", false),
            _ => (false, false),
        };
    };
    let expected = [G_2_UP, G_2_DOWN, G_2_LEFT, G_2_RIGHT, G_2_TAP];
    if block < 5 {
        (id == expected[block], id != expected[block])
    } else if block == 7 || block == 8 {
        ((G_2_UP..=G_2_RIGHT).contains(&id), false)
    } else {
        (false, false)
    }
}

fn summarize(_name: &str, index: usize, result: &BlockResult) -> (usize, usize, usize, String) {
    let mut outcomes = BTreeMap::<String, usize>::new();
    let mut intended_n = 0;
    let mut wrong_n = 0;
    let mut negative_n = 0;
    for session in &result.sessions {
        *outcomes.entry(session.outcome.clone()).or_default() += 1;
        let (right, wrong) = intended(index, session);
        intended_n += right as usize;
        wrong_n += wrong as usize;
        if matches!(index, 9 | 10)
            && (session.gesture.is_some() || session.outcome == "zoom" || session.outcome == "pan")
        {
            negative_n += 1;
        }
    }
    let counts = outcomes
        .into_iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join(", ");
    (
        intended_n,
        negative_n,
        wrong_n,
        if counts.is_empty() {
            "none".to_owned()
        } else {
            counts
        },
    )
}

fn print_block_report(names: &[String], result: &[BlockResult]) -> (usize, usize, usize) {
    let (mut intended_total, mut negative_total, mut wrong_total) = (0, 0, 0);
    println!("sessions:");
    for (i, block) in result.iter().enumerate() {
        let name = names.get(i).map(String::as_str).unwrap_or("Block");
        for session in &block.sessions {
            println!(
                "  {} @ {}: {}{}",
                name,
                session.k_ms,
                session.outcome,
                if session.measure.is_empty() {
                    String::new()
                } else {
                    format!("; {}", session.measure)
                }
            );
        }
    }
    println!("blocks:");
    for (i, block) in result.iter().enumerate() {
        let name = names.get(i).map(String::as_str).unwrap_or("Block");
        let (intended_n, negative_n, wrong_n, counts) = summarize(name, i, block);
        intended_total += intended_n;
        negative_total += negative_n;
        wrong_total += wrong_n;
        let swipe = if i <= 3 || i == 8 {
            format!("; intended fires {intended_n}")
        } else {
            String::new()
        };
        let wrong = if i <= 3 || i == 8 {
            format!("; wrong direction {wrong_n}")
        } else {
            String::new()
        };
        println!(
            "  {}: {} frames, {} sessions ({}){}{}{}",
            name,
            block.frames,
            block.sessions.len(),
            counts,
            swipe,
            wrong,
            if i == 9 || i == 10 {
                format!("; negative fires {negative_n}")
            } else {
                String::new()
            }
        );
        for session in &block.sessions {
            if matches!(i, 9 | 10)
                && (session.gesture.is_some()
                    || session.outcome == "zoom"
                    || session.outcome == "pan")
            {
                println!(
                    "    unexpected fire @ {}: {}",
                    session.k_ms, session.outcome
                );
            }
        }
        for trace in &block.trace_outcomes {
            println!("    trace reference: {trace}");
        }
    }
    (intended_total, negative_total, wrong_total)
}

fn sweep_values(sweep: &Sweep) -> Vec<f64> {
    let mut values = Vec::new();
    let mut v = sweep.start;
    for _ in 0..10001 {
        if if sweep.step > 0.0 {
            v > sweep.end + sweep.step.abs() * 1e-9
        } else {
            v < sweep.end - sweep.step.abs() * 1e-9
        } {
            break;
        }
        values.push(v);
        v += sweep.step;
    }
    values
}

fn run(args: Vec<String>) -> i32 {
    match parse_args(&args) {
        Ok(None) => {
            println!("usage: tgstudy TRACE [--tune v1|new] [--set FIELD=VALUE ...] [--sweep FIELD=A:B:STEP] [--blocks BLOCKS.txt]");
            0
        }
        Err(e) => {
            let _ = writeln!(std::io::stderr(), "tgstudy: {e}\nusage: tgstudy TRACE [--tune v1|new] [--set FIELD=VALUE ...] [--sweep FIELD=A:B:STEP] [--blocks BLOCKS.txt]");
            2
        }
        Ok(Some(opts)) => {
            let text = match fs::read_to_string(&opts.trace) {
                Ok(text) => text,
                Err(e) => {
                    let _ = writeln!(
                        std::io::stderr(),
                        "tgstudy: cannot read {}: {e}",
                        opts.trace
                    );
                    return 1;
                }
            };
            let events = match parse_trace(&text) {
                Ok(events) => events,
                Err(e) => {
                    let _ = writeln!(std::io::stderr(), "tgstudy: {}: {e}", opts.trace);
                    return 1;
                }
            };
            let split = split_blocks(events);
            let names = match block_names(opts.blocks.as_deref(), split.frame_counts.len()) {
                Ok(names) => names,
                Err(e) => {
                    let _ = writeln!(std::io::stderr(), "tgstudy: {e}");
                    return 1;
                }
            };
            let header = split.events.iter().find_map(|e| match e {
                TraceEvent::Header {
                    rotation,
                    width,
                    height,
                    dpi,
                } => Some((*rotation, *width, *height, *dpi)),
                _ => None,
            });
            println!(
                "tune: {}{}",
                opts.tune_name,
                if header.is_none() {
                    " (no header; using 300 dpi)"
                } else {
                    ""
                }
            );
            println!("trace contains no pen coordinates; pen-distance rule is treated as satisfied while the pen is gone");
            if let Some((rotation, width, height, dpi)) = header {
                println!("header: rotation {rotation}, {width}x{height}, dpi {dpi}");
            }
            println!("block sizes:");
            for (i, count) in split.frame_counts.iter().enumerate() {
                println!("  {}: {} frames", names[i], count);
            }
            if let Some(sweep) = opts.sweep {
                println!("sweep {}: value | intended fires/block | negative fires | wrong-direction fires", sweep.field);
                for value in sweep_values(&sweep) {
                    let mut tune = opts.tune;
                    if let Err(e) = set_sweep_value(&mut tune, &sweep.field, value) {
                        let _ = writeln!(std::io::stderr(), "tgstudy: {e}");
                        return 2;
                    }
                    let result = replay(&split, &names, &tune);
                    let (right, negative, wrong) = result
                        .iter()
                        .enumerate()
                        .map(|(i, b)| {
                            let (r, n, w, _) = summarize(&names[i], i, b);
                            (r, n, w)
                        })
                        .fold((0, 0, 0), |a, x| (a.0 + x.0, a.1 + x.1, a.2 + x.2));
                    println!("  {} | {} | {} | {}", value, right, negative, wrong);
                }
                return 0;
            }
            let result = replay(&split, &names, &opts.tune);
            let _ = print_block_report(&names, &result);
            0
        }
    }
}

fn main() {
    let args = std::env::args_os()
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    std::process::exit(run(args));
}
