// bat.h: Grimoire's pixel bat, shared by the logo (tools/mklogo.c), the Library cover
// (tools/mkicon.c) and the browser pages (httpd.c: their icon and title). Drawn on the same
// design-pixel grid as the Jacquard 24 title it sits with: k x k blocks.
#pragma once
#include "canvas.h"

// Wings spread: wrist claws, pointed tips, three scalloped fingers a side.
static const char *k_bat[] = {
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
};
enum { BAT_W = 43, BAT_H = 14 };

static inline void draw_bat(Canvas *c, int x, int y, int k) {
    for (int r = 0; r < BAT_H; r++)
        for (int col = 0; col < BAT_W; col++)
            if (k_bat[r][col] == 'X') canvas_fill(c, (Rect){x + col * k, y + r * k, k, k}, 0);
}
