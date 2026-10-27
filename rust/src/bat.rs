//! bat.h: Grimoire's pixel bat, shared by the logo (tools/mklogo.c), the Library cover
//! (tools/mkicon.c) and the browser pages (httpd.c: their icon and title). Drawn on the same
//! design-pixel grid as the Jacquard 24 title it sits with: k x k blocks.

use crate::prelude::*;

/// Wings spread: wrist claws, pointed tips, three scalloped fingers a side.
pub static K_BAT: [&str; 14] = [
    ".........X.........X...X.........X.........",
    "......XXXXXX.......X...X.......XXXXXX......",
    "...XXXXXXXXXXX.....XXXXX.....XXXXXXXXXXX...",
    "XXXXXXXXXXXXXXX....XXXXX....XXXXXXXXXXXXXXX",
    ".XXXXXXXXXXXXXXXX.XXXXXXX.XXXXXXXXXXXXXXXX.",
    "..XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX..",
    "...XXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX...",
    "...XX...XXXXXXXXXXXXXXXXXXXXXXXXXXX...XX...",
    "...X....XXX..XXXXXXXXXXXXXXXXX..XXX....X...",
    "...X.....X....XXXXXXXXXXXXXXX....X.....X...",
    ".........X....XX..XXXXXXX..XX....X.........",
    "...................XXXXX...................",
    "...................XXXXX...................",
    "....................X.X....................",
];
pub const BAT_W: i32 = 43;
pub const BAT_H: i32 = 14;

pub fn draw_bat(c: &mut Canvas, x: i32, y: i32, k: i32) {
    for r in 0..BAT_H {
        for col in 0..BAT_W {
            if K_BAT[r as usize].as_bytes()[col as usize] == b'X' {
                canvas_fill(c, Rect { x: x + col * k, y: y + r * k, w: k, h: k }, 0);
            }
        }
    }
}
