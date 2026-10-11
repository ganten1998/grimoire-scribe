// dispmenu.c: the Display sheet.
#include "dispmenu.h"

#include <stdio.h>

#include "light.h"

enum { D_BRIGHT = 1, D_WARM, D_INVERT, D_DONE };

static Rect card_rect(App *a) {
    int w = a->w - 2 * ui_px(70);
    if (w > ui_px(1500)) w = ui_px(1500);
    int h = ui_px(120) + ui_px(72) + 3 * ui_px(150) + ui_px(190);
    return (Rect){(a->w - w) / 2, a->h / 4, w, h};
}

void dispmenu_open(DispMenu *m, App *a) {
    *m = (DispMenu){.open = true};
    light_get(&m->intensity, &m->warmth);
    m->rect = card_rect(a);
}

static void slider_for(DispMenu *m, App *a, int which, Rect row, Slider *s) {
    *s = (Slider){.min = 0, .max = LIGHT_MAX, .step = 1, .unit = ""};
    s->label = which == D_BRIGHT ? "Brightness" : "Warmth";
    s->value = (float)(which == D_BRIGHT ? m->intensity : m->warmth);
    if (s->value < 0) s->value = 0;
    ui_slider_layout(s, row);
}

Rect dispmenu_draw(DispMenu *m, App *a) {
    Canvas *c = &a->screen;
    hits_clear(&m->hits);
    m->rect = card_rect(a);
    Rect in = ui_sheet(c, m->rect, "Display");
    int rh = ui_px(150);
    bool have = light_available() && m->intensity >= 0;
    for (int i = 0; i < 2; i++) {
        Rect row = {in.x, in.y + i * rh, in.w, rh};
        if (!have) {
            canvas_fill(c, row, 255);
            if (i == 0)
                font_draw_left(c, FONT_REGULAR, ui_text_size(), row, "Front light controls are not available here.", 60);
            continue;
        }
        Slider s;
        slider_for(m, a, i == 0 ? D_BRIGHT : D_WARM, row, &s);
        char vt[16];
        snprintf(vt, sizeof vt, "%d", (int)s.value);
        ui_slider_draw(c, &s, vt);
        hits_add(&m->hits, row, i == 0 ? D_BRIGHT : D_WARM, 0);
    }
    Rect inv = {in.x, in.y + 2 * rh + ui_px(20), in.w, ui_px(110)};
    ui_checkbox(c, inv, "Invert colors in Quire (white on black)", a->settings.invert);
    hits_add(&m->hits, inv, D_INVERT, 0);
    Rect done = {in.x + in.w - ui_px(360), in.y + in.h - ui_px(140), ui_px(360), ui_px(140)};
    ui_button(c, done, ICON_CHECK, "Done", BTN_PRIMARY);
    hits_add(&m->hits, done, D_DONE, 0);
    return m->rect;
}

static void apply(DispMenu *m, int which) {
    if (which == D_BRIGHT) light_set_intensity(m->intensity);
    else light_set_warmth(m->warmth);
}

DmResult dispmenu_pointer(DispMenu *m, App *a, int ph, float x, float y, uint32_t t, Rect *dirty) {
    *dirty = RECT_EMPTY;
    if (!m->open) return DM_NONE;
    if (ph == 0) {
        press_begin(&m->press, x, y, t);
        if (!rect_contains(m->rect, (int)x, (int)y)) return DM_NONE;
        const Hit *h = hits_find(&m->hits, x, y);
        m->active = h && (h->id == D_BRIGHT || h->id == D_WARM) ? h->id : 0;
        if (!m->active) return DM_NONE;
    }
    if (m->active) {
        Slider s;
        int rh = ui_px(150);
        Rect in = rect_inflate((Rect){m->rect.x, m->rect.y + ui_px(120), m->rect.w, m->rect.h - ui_px(120)}, -ui_px(36));
        slider_for(m, a, m->active, (Rect){in.x, in.y + (m->active - 1) * rh, in.w, rh}, &s);
        ui_slider_set_from_x(&s, x);
        int *v = m->active == D_BRIGHT ? &m->intensity : &m->warmth;
        bool changed = (int)s.value != *v;
        *v = (int)s.value;
        // The light reacts live, but do not flood the power service while dragging.
        if (ph == 2 || (changed && t - m->last_apply > 200)) {
            apply(m, m->active);
            m->last_apply = t;
        }
        if (changed || ph == 2) {
            dispmenu_draw(m, a);
            *dirty = m->rect;
        }
        if (ph == 2) m->active = 0;
        return changed ? DM_CHANGED : DM_NONE;
    }
    if (ph != 2) return DM_NONE;
    press_move(&m->press, x, y);
    if (!rect_contains(m->rect, (int)m->press.x0, (int)m->press.y0)) {
        // A tap outside the sheet closes it.
        m->open = false;
        *dirty = m->rect;
        return DM_CLOSED;
    }
    const Hit *h = hits_find(&m->hits, x, y);
    if (!h || !press_is_tap(&m->press, h->r)) return DM_NONE;
    if (h->id == D_DONE) {
        m->open = false;
        *dirty = m->rect;
        return DM_CLOSED;
    }
    if (h->id == D_INVERT) {
        a->settings.invert = !a->settings.invert;
        settings_save(&a->settings);
        display_set_invert(a->settings.invert);
        dispmenu_draw(m, a);
        return DM_INVERTED;
    }
    return DM_NONE;
}
