//! dispmenu.h: the Display sheet (front light brightness + warmth, invert Grimoire's colors).
//! A modal hosted by the editor and the library; it draws itself and handles pointer input.

use crate::prelude::*;


#[repr(i32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum DmResult {
    #[default]
    None = 0,  // DM_NONE
    Changed = 1,  // DM_CHANGED
    Closed = 2,  // DM_CLOSED
    Inverted = 3,  // DM_INVERTED
}

#[derive(Clone, Debug, Default)]
pub struct DispMenu {
    pub open: bool,
    pub rect: Rect,
    pub hits: HitList,
    /// 0..LIGHT_MAX, -1 unknown
    pub intensity: i32,
    pub warmth: i32,
    /// slider being dragged (its D_* id), 0 none
    pub active: i32,
    /// that slider's row
    pub active_row: Rect,
    /// its value changed since its row was last drawn
    pub row_stale: bool,
    /// when its row was last drawn
    pub last_draw: u32,
    pub last_apply: u32,
    pub press: Press,
}

pub fn dispmenu_open(m: &mut DispMenu, a: &mut App) {
    todo!()
}
/// Draws into a->screen; returns the rect covered.
pub fn dispmenu_draw(m: &mut DispMenu, a: &mut App) -> Rect {
    todo!()
}
/// Pointer phase 0 down / 1 move / 2 up. DM_CLOSED: the host should recompose m->rect.
/// DM_INVERTED: colors flipped, the host should repaint everything (full refresh).
pub fn dispmenu_pointer(m: &mut DispMenu, a: &mut App, phase: i32, x: f32, y: f32, t: u32, dirty: &mut Rect) -> DmResult {
    todo!()
}
