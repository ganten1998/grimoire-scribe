// mklogo.c: the Grimoire logo: "Grimoire" in the gothic title face (Jacquard 24, through the
// app's own pixel renderer, so each design pixel is a k x k block exactly as on the Kindle) and
// a pixel bat on the same grid. `make logo` writes docs/logo.png (black) and docs/logo-dark.png
// (white), both on transparent; the PNGs are committed, so nothing else needs this tool.
//   usage: mklogo <layout: above | crest | right> <k> <out.png> [white]
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "bat.h"
#include "canvas.h"
#include "font.h"

extern int stbi_write_png(const char *filename, int w, int h, int comp, const void *data, int stride);

int main(int argc, char **argv) {
    if (argc < 4) {
        fprintf(stderr, "usage: mklogo <above|crest|right> <k> <out.png> [white]\n");
        return 2;
    }
    const char *layout = argv[1], *out = argv[3];
    int k = atoi(argv[2]);
    bool white = argc > 4 && !strcmp(argv[4], "white");
    if (k < 1 || k > 16 || font_init() != 0) return 1;
    font_set_gothic(true);
    // The title face is Jacquard at 1.1x the asked size; 43 design pixels make one line.
    float px = 43.f * (float)k / 1.1f;
    const char *word = "Grimoire";
    int tw = font_measure(FONT_TITLE, px, word), asc = font_ascent(FONT_TITLE, px), desc = font_descent(FONT_TITLE, px);
    int m = 6 * k, bw = BAT_W * k, bh = BAT_H * k;
    int W, H, tx, base, bx, by;
    bool crest = !strcmp(layout, "crest");
    if (crest || !strcmp(layout, "above")) {  // the bat over the middle of the word (crest: and a rule)
        W = (tw > bw ? tw : bw) + 2 * m;
        H = m + bh + 4 * k + asc + desc + (crest ? 8 * k : 0) + m;
        tx = (W - tw) / 2;
        bx = (W - bw) / 2;
        by = m;
        base = by + bh + 4 * k + asc;
    } else {  // flying off the end of the word, above the "re"
        int rx = font_measure(FONT_TITLE, px, "Grimoi");
        tx = m;
        bx = tx + rx + bw / 4;
        by = m;
        base = m + bh / 2 + asc;
        W = (bx + bw > tx + tw ? bx + bw : tx + tw) + m;
        H = base + desc + m;
    }
    Canvas c;
    if (canvas_init(&c, W, H) != 0) return 1;
    font_draw(&c, FONT_TITLE, px, tx, base, word, 0);
    draw_bat(&c, bx, by, k);
    if (crest) {  // the Library cover's rule: two hairlines and a lozenge
        int y = base + desc + 2 * k, cx = W / 2;
        canvas_hline(&c, tx + 4 * k, cx - 6 * k, y, k, 0);
        canvas_hline(&c, cx + 6 * k, tx + tw - 4 * k, y, k, 0);
        for (int r = 0; r <= 3; r++) {  // a pixel diamond, 7 design pixels across
            Rect row = {cx - (3 - r) * k - k / 2, y - r * k, (2 * (3 - r) + 1) * k, k};
            canvas_fill(&c, row, 0);
            row.y = y + r * k;
            canvas_fill(&c, row, 0);
        }
    }
    // One color on transparent: alpha is the ink's darkness.
    uint8_t *ga = malloc((size_t)W * H * 2);
    if (!ga) return 1;
    for (int y = 0; y < H; y++)
        for (int x = 0; x < W; x++) {
            ga[(y * W + x) * 2] = white ? 255 : 0;
            ga[(y * W + x) * 2 + 1] = (uint8_t)(255 - c.px[y * c.stride + x]);
        }
    int ok = stbi_write_png(out, W, H, 2, ga, W * 2);
    free(ga);
    canvas_free(&c);
    if (!ok) return 1;
    printf("wrote %s (%dx%d)\n", out, W, H);
    return 0;
}
