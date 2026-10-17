// mkicon.c: draws the Library cover (assets/icon.png) with the app's own gothic widgets.
// `make icon` rebuilds it; the PNG is committed, so packaging never needs this tool.
#include <stdio.h>

#include "canvas.h"
#include "font.h"
#include "ui.h"

extern int stbi_write_png(const char *filename, int w, int h, int comp, const void *data, int stride);

int main(int argc, char **argv) {
    const char *out = argc > 1 ? argv[1] : "assets/icon.png";
    const int W = 300, H = 400;
    Canvas c;
    if (canvas_init(&c, W, H) != 0 || font_init() != 0) return 1;
    ui_init(100.f);  // a third of the screen's scale: ornaments sized for a 300 px cover
    ui_set_theme(THEME_GOTHIC);
    Rect card = {6, 6, W - 12, H - 12};
    canvas_chamfer_rect(&c, card, 26, 0, false, 8);
    canvas_chamfer_rect(&c, rect_inflate(card, -14), 18, 0, false, 2);
    ui_frieze(&c, 22, W - 22, 22, 18, 0);
    ui_emblem(&c, W / 2.f, 160.f, 66.f, 0, 255);
    font_draw_centered(&c, FONT_TITLE, 86.f, (Rect){14, 262, W - 28, 80}, "Grimoire", 0);
    canvas_lozenge(&c, W / 2.f, 352.f, 6.f, 0);
    canvas_hline(&c, 60, W / 2 - 14, 351, 2, 0);
    canvas_hline(&c, W / 2 + 14, W - 60, 351, 2, 0);
    if (!stbi_write_png(out, W, H, 1, c.px, c.stride)) return 1;
    printf("wrote %s\n", out);
    return 0;
}
