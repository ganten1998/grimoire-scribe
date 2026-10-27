//! inkmode.h: what a stroke of the pen does in the editor, as pure logic (unit-tested on the host by
//! tests/test_inkwire.c): ink, white ink, the precise eraser or the stroke eraser, from the settings
//! (eraser_mode, white, side_button), the pen's eraser end and its side button; and the precise
//! eraser's settings at a zoom. White ink and the precise eraser are erase strokes (src/page.h,
//! docs/FORMAT.md): they restore the paper under them.

use crate::prelude::*;

#[repr(i32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PenAct {
    /// ink with `tool`
    #[default]
    Ink = 0,  // PEN_INK
    /// white ink: an erase stroke with `tool`'s texture and settings
    White = 1,  // PEN_WHITE
    /// the precise eraser: an erase stroke, the round tip with the eraser's settings
    Precise = 2,  // PEN_PRECISE
    /// the stroke eraser: every stroke it touches goes, but erase strokes (taking one
    Whole = 3,  // PEN_WHOLE
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PenStroke {
    pub act: PenAct,
    /// PEN_INK, PEN_WHITE: the drawing tool; the erasers: TOOL_ERASER
    pub tool: ToolKind,
}

/// The drawing tool "the pen" means: the one it draws with, or while that is the eraser the drawing
/// tool the toolbar shows (the pencil if that is the eraser too, or out of range).
#[inline]
pub fn pen_drawing_tool(s: &Settings) -> ToolKind {
    if (s.current_tool.0 as u32) < TOOL_COUNT as u32 && s.current_tool != TOOL_ERASER {
        return s.current_tool;
    }
    if (s.draw_tool.0 as u32) < TOOL_COUNT as u32 && s.draw_tool != TOOL_ERASER {
        return s.draw_tool;
    }
    TOOL_PENCIL
}

/// What a stroke does that the pen starts with its eraser end on the page (`eraser_end`) or with its
/// side button held (`side`). The eraser end always erases. The side button erases (side_button 1),
/// highlights with ink, whatever `white` says (2), draws white with the pen's drawing tool (3) or
/// does nothing (0). Otherwise the pen's tool: the eraser is the precise one with eraser_mode 1 and
/// the stroke eraser with 0, and a drawing tool draws white while `white` is set.
pub fn pen_stroke(s: &Settings, eraser_end: bool, side: bool) -> PenStroke {
    let eraser = PenStroke { act: if s.eraser_mode == 1 { PenAct::Precise } else { PenAct::Whole }, tool: TOOL_ERASER };
    if eraser_end {
        return eraser;
    }
    if side {
        if s.side_button == 1 {
            return eraser;
        }
        if s.side_button == 2 {
            return PenStroke { act: PenAct::Ink, tool: TOOL_HIGHLIGHTER };
        }
        if s.side_button == 3 {
            return PenStroke { act: PenAct::White, tool: pen_drawing_tool(s) };
        }
    }
    if s.current_tool == TOOL_ERASER {
        return eraser;
    }
    PenStroke { act: if s.white { PenAct::White } else { PenAct::Ink }, tool: pen_drawing_tool(s) }
}

/// The range a stroke's widths keep in a page file: every reader clamps them to it
/// (docs/FORMAT.md; tests/test_inkwire.c holds these to the slider table).
pub const ERASER_MM_MIN: f32 = 0.10;
pub const ERASER_MM_MAX: f32 = 12.0;

/// The precise eraser's settings for a stroke drawn at `zoom` (screen pixels per page pixel): its
/// widths divided by the zoom, so it covers as much of the screen at any zoom as the stroke eraser
/// does, within the range a page file keeps (so the stroke reads back exactly as it was drawn).
pub fn eraser_at_zoom(e: &ToolParams, zoom: f32) -> ToolParams {
    let mut p = *e;
    let zoom = if !(zoom > 0.0) { 1.0 } else { zoom }; // NaN, 0 or less: as at 100%
    let lo = p.size_min_mm / zoom;
    let hi = p.size_max_mm / zoom;
    p.size_min_mm = if !lo.is_nan() { clampf(lo, ERASER_MM_MIN, ERASER_MM_MAX) } else { ERASER_MM_MIN };
    p.size_max_mm = if !hi.is_nan() { clampf(hi, ERASER_MM_MIN, ERASER_MM_MAX) } else { ERASER_MM_MIN };
    p
}
