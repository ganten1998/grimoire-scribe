// dispmenu.c: the Display sheet: front light, invert, and sleep (screen + timeout).
#include "dispmenu.h"

#include <stdio.h>

#include "light.h"

enum { D_BRIGHT = 1, D_WARM, D_INVERT, D_SLEEP_MODE, D_PIN, D_SLEEP_AFTER, D_DONE };

// "Sleep after" choices (minutes, 0 = never); the slider moves along this list.
static const int k_sleep_steps[] = {0, 1, 2, 5, 10, 15, 30, 60};
#define N_SLEEP_STEPS ((int)(sizeof k_sleep_steps / sizeof k_sleep_steps[0]))

// The sleep screen row, one sentence per choice (settings.ini sleep_mode 0, 1, 2).
static const char *const k_mode_rows[3] = {
    // TRANSLATORS: the Display sheet's sleep screen row, which a tap moves to the next choice: what
    // the Kindle shows while it sleeps. About 1280 px at 44 px upright, more sideways.
    N_("Sleep screen: the page I'm on (tap to change)"),
    N_("Sleep screen: a pinned page (tap to change)"),
    N_("Sleep screen: the Kindle's own screensaver (tap to change)"),
};

static int step_of(int minutes) {
    int best = 0;
    for (int i = 0; i < N_SLEEP_STEPS; i++)
        if (abs(k_sleep_steps[i] - minutes) < abs(k_sleep_steps[best] - minutes)) best = i;
    return best;
}

// Rows: brightness, warmth, invert, [heading], sleep screen, [pin], sleep after.
static int row_h(App *a) {
    int rows = 6 + (editor_page_canvas(a) ? 1 : 0);
    int room = a->h - ui_px(120) - ui_px(72) - ui_px(190) - ui_px(80);
    int h = room / rows;
    return clampi(h, ui_px(100), ui_px(140));
}

static Rect card_rect(App *a) {
    int w = a->w - 2 * ui_px(70);
    if (w > ui_px(1500)) w = ui_px(1500);
    int rows = 6 + (editor_page_canvas(a) ? 1 : 0);
    int h = ui_px(120) + ui_px(72) + rows * row_h(a) + ui_px(190);
    return (Rect){(a->w - w) / 2, (a->h - h) / 3, w, h};
}

void dispmenu_open(DispMenu *m, App *a) {
    *m = (DispMenu){.open = true};
    light_get(&m->intensity, &m->warmth);
    m->rect = card_rect(a);
}

static void slider_for(DispMenu *m, App *a, int which, Rect row, Slider *s) {
    *s = (Slider){.step = 1, .unit = ""};
    if (which == D_SLEEP_AFTER) {
        s->min = 0;
        s->max = N_SLEEP_STEPS - 1;
        // TRANSLATORS: the Display sheet's sliders, each left of its value ("12", "5 min", "never"):
        // about 460 px at 44 px.
        s->label = _("Sleep after");
        s->value = (float)step_of(a->settings.sleep_minutes);
    } else {
        s->min = 0;
        s->max = LIGHT_MAX;
        s->label = which == D_BRIGHT ? _("Brightness") : _("Warmth");
        s->value = (float)(which == D_BRIGHT ? m->intensity : m->warmth);
        if (s->value < 0) s->value = 0;
    }
    ui_slider_layout(s, row);
}

Rect dispmenu_draw(DispMenu *m, App *a) {
    Canvas *c = &a->screen;
    hits_clear(&m->hits);
    m->rect = card_rect(a);
    Rect in = ui_sheet(c, m->rect, _("Display"));
    int rh = row_h(a), y = in.y;
    bool have = light_available() && m->intensity >= 0;
    for (int i = 0; i < 2; i++, y += rh) {
        Rect row = {in.x, y, in.w, rh};
        canvas_fill(c, row, 255);
        if (!have) {
            if (i == 0) {
                // TRANSLATORS: in the Display sheet where the sliders would be (1420 px at 44 px).
                font_draw_left(c, FONT_REGULAR, ui_text_size(), row, _("Front light controls are not available here."), 60);
            }
            continue;
        }
        if (i == 1 && m->warmth < 0) {  // no amber LEDs on this Kindle
            font_draw_left(c, FONT_REGULAR, ui_text_size(), row, _("This Kindle has no warmth control."), 60);
            continue;
        }
        Slider s;
        slider_for(m, a, i == 0 ? D_BRIGHT : D_WARM, row, &s);
        char vt[16];
        snprintf(vt, sizeof vt, "%d", (int)s.value);
        ui_slider_draw(c, &s, vt);
        hits_add(&m->hits, row, i == 0 ? D_BRIGHT : D_WARM, 0);
    }
    Rect inv = {in.x, y, in.w, rh};
    ui_checkbox(c, inv, _("Invert colors in Grimoire (white on black)"), a->settings.invert);
    hits_add(&m->hits, inv, D_INVERT, 0);
    y += rh;
    // Sleep section.
    canvas_hline(c, in.x, in.x + in.w, y + rh / 3, ui_px(3), 0);
    // TRANSLATORS: the heading of the Display sheet's rows about the Kindle's sleep.
    font_draw_left(c, FONT_BOLD, ui_text_size(), (Rect){in.x, y + rh / 3 + ui_px(8), in.w, rh * 2 / 3}, _("Sleep"), 0);
    y += rh;
    Rect mode = {in.x, y, in.w, rh};
    canvas_fill(c, mode, 255);
    ui_icon(c, ICON_NOTEBOOK, (Rect){mode.x, mode.y + rh / 8, rh * 3 / 4, rh * 3 / 4}, 0);
    font_draw_left(c, FONT_REGULAR, ui_text_size(), (Rect){mode.x + rh, mode.y, mode.w - rh, rh},
                   _(k_mode_rows[clampi(a->settings.sleep_mode, 0, 2)]), 0);
    hits_add(&m->hits, mode, D_SLEEP_MODE, 0);
    y += rh;
    if (editor_page_canvas(a)) {
        Rect pin = {in.x, y + ui_px(8), ui_px(560), rh - ui_px(16)};
        // TRANSLATORS: makes the open page the sleep screen (528 px at 44 px, or 34 px).
        ui_button(c, pin, ICON_NONE, _("Pin this page"), BTN_PLAIN);
        hits_add(&m->hits, pin, D_PIN, 0);
        y += rh;
    }
    Rect after = {in.x, y, in.w, rh};
    Slider s;
    slider_for(m, a, D_SLEEP_AFTER, after, &s);
    char vt[24];
    int mins = k_sleep_steps[(int)s.value];
    // TRANSLATORS: the Sleep after slider's value: minutes, or never.
    if (mins) snprintf(vt, sizeof vt, _("%d min"), mins);
    else snprintf(vt, sizeof vt, "%s", C_("sleep after", "never"));
    ui_slider_draw(c, &s, vt);
    hits_add(&m->hits, after, D_SLEEP_AFTER, 0);
    Rect done = {in.x + in.w - ui_px(360), in.y + in.h - ui_px(140), ui_px(360), ui_px(140)};
    // TRANSLATORS: closes a sheet: under its check mark here (322 px at 34 px), on Settings 342 px
    // at 44 px, on the keyboard 334 px at 44 px in bold.
    ui_button(c, done, ICON_CHECK, _("Done"), BTN_PRIMARY);
    hits_add(&m->hits, done, D_DONE, 0);
    return m->rect;
}

DmResult dispmenu_pointer(DispMenu *m, App *a, int ph, float x, float y, uint32_t t, Rect *dirty) {
    *dirty = RECT_EMPTY;
    if (!m->open) return DM_NONE;
    if (ph == 0) {
        press_begin(&m->press, x, y, t);
        if (!rect_contains(m->rect, (int)x, (int)y)) return DM_NONE;
        const Hit *h = hits_find(&m->hits, x, y);
        m->active = h && (h->id == D_BRIGHT || h->id == D_WARM || h->id == D_SLEEP_AFTER) ? h->id : 0;
        if (m->active) m->active_row = h->r;
        if (!m->active) return DM_NONE;
    }
    if (m->active) {
        Slider s;
        slider_for(m, a, m->active, m->active_row, &s);
        ui_slider_set_from_x(&s, x);
        bool changed;
        if (m->active == D_SLEEP_AFTER) {
            int mins = k_sleep_steps[clampi((int)s.value, 0, N_SLEEP_STEPS - 1)];
            changed = mins != a->settings.sleep_minutes;
            a->settings.sleep_minutes = mins;
            if (ph == 2) {
                settings_save(&a->settings);
                app_apply_sleep_policy(a);
            }
        } else {
            int *v = m->active == D_BRIGHT ? &m->intensity : &m->warmth;
            changed = (int)s.value != *v;
            *v = (int)s.value;
            // The light reacts live, but do not flood the power service while dragging.
            if (ph == 2 || (changed && t - m->last_apply > 200)) {
                if (m->active == D_BRIGHT) light_set_intensity(m->intensity);
                else light_set_warmth(m->warmth);
                m->last_apply = t;
            }
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
        m->open = false;  // a tap outside the sheet closes it
        *dirty = m->rect;
        return DM_CLOSED;
    }
    const Hit *h = hits_find(&m->hits, x, y);
    if (!h || !press_is_tap(&m->press, h->r)) return DM_NONE;
    switch (h->id) {
    case D_DONE:
        m->open = false;
        *dirty = m->rect;
        return DM_CLOSED;
    case D_INVERT:
        a->settings.invert = !a->settings.invert;
        settings_save(&a->settings);
        display_set_invert(a->settings.invert);
        dispmenu_draw(m, a);
        return DM_INVERTED;
    case D_SLEEP_MODE:
        a->settings.sleep_mode = (a->settings.sleep_mode + 1) % 3;
        settings_save(&a->settings);
        break;
    case D_PIN:
        if (editor_pin_sleep_page(a)) {
            a->settings.sleep_mode = 1;
            settings_save(&a->settings);
            app_toast(a, _("This page is now the sleep screen"), 2000);
        } else {
            app_toast(a, _("Could not save the sleep screen"), 2500);
        }
        break;
    default: return DM_NONE;
    }
    dispmenu_draw(m, a);
    *dirty = m->rect;
    return DM_CHANGED;
}
