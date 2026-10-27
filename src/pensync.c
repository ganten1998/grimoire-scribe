// pensync.c: the pen after a SYN_DROPPED, and its samples' kernel times (pensync.h).
#include "pensync.h"

bool pensync_contact(const PenState *s, bool touch_btn, int p_min) {
    if (touch_btn) return s->touching;
    return (s->tool_pen || s->tool_rubber) && s->p > p_min;
}

// The PenState field an event sets (a PS_F_* bit), 0 for one the pen's reader does not follow.
static unsigned field_of(const PenEv *e) {
    if (e->type == PS_EV_KEY) {
        switch (e->code) {
        case PS_BTN_TOOL_PEN: return PS_F_TOOL_PEN;
        case PS_BTN_TOOL_RUBBER: return PS_F_TOOL_RUBBER;
        case PS_BTN_TOUCH: return PS_F_TOUCH;
        case PS_BTN_STYLUS: return PS_F_SIDE;
        default: return 0;
        }
    }
    if (e->type == PS_EV_ABS) {
        switch (e->code) {
        case PS_ABS_X: return PS_F_X;
        case PS_ABS_Y: return PS_F_Y;
        case PS_ABS_PRESSURE: return PS_F_P;
        case PS_ABS_TILT_X: return PS_F_TX;
        case PS_ABS_TILT_Y: return PS_F_TY;
        default: return 0;
        }
    }
    return 0;
}

static void set_field(PenState *s, unsigned f, int v) {
    switch (f) {
    case PS_F_TOOL_PEN: s->tool_pen = v != 0; break;
    case PS_F_TOOL_RUBBER: s->tool_rubber = v != 0; break;
    case PS_F_TOUCH: s->touching = v != 0; break;
    case PS_F_SIDE: s->side = v != 0; break;
    case PS_F_X: s->x = v; break;
    case PS_F_Y: s->y = v; break;
    case PS_F_P: s->p = v; break;
    case PS_F_TX: s->tx = v; break;
    case PS_F_TY: s->ty = v; break;
    default: break;
    }
}

static int get_field(const PenState *s, unsigned f) {
    switch (f) {
    case PS_F_TOOL_PEN: return s->tool_pen;
    case PS_F_TOOL_RUBBER: return s->tool_rubber;
    case PS_F_TOUCH: return s->touching;
    case PS_F_SIDE: return s->side;
    case PS_F_X: return s->x;
    case PS_F_Y: return s->y;
    case PS_F_P: return s->p;
    case PS_F_TX: return s->tx;
    case PS_F_TY: return s->ty;
    default: return 0;
    }
}

static bool is_syn(const PenEv *e, int code) { return e->type == PS_EV_SYN && e->code == code; }

static void push(PenFrame *out, int *n, int cap, PenFrame f) {
    if (*n < cap) out[(*n)++] = f;
}

// The lift that goes first when the pen touched at the last good point and no longer does: the
// stroke ends where it was last seen, not wherever the tail (or the device now) finds the pen.
static void lift_first(const PenTail *t, const PenState *first, uint64_t t_us, PenFrame *out, int *n, int cap,
                       PenTailInfo *in) {
    if (!pensync_contact(&t->good, t->touch_btn, t->p_min) || pensync_contact(first, t->touch_btn, t->p_min)) return;
    PenState up = t->good;
    up.touching = false;
    if (!t->touch_btn) up.p = t->p_min;
    push(out, n, cap, (PenFrame){.s = up, .t_us = t_us});
    in->lifted = true;
}

int pensync_tail(const PenTail *t, PenFrame *out, int cap, PenTailInfo *info) {
    PenTailInfo in = {0};
    int n = 0;
    // A second drop in what was read throws away everything before it too: start after the last.
    int start = 0;
    for (int i = 0; i < t->n; i++)
        if (is_syn(&t->ev[i], PS_SYN_DROPPED)) start = i + 1;
    // The partial frame, up to its SYN_REPORT: left out (the rest of it was thrown away).
    int k = start;
    while (k < t->n && !is_syn(&t->ev[k], PS_SYN_REPORT)) k++;
    in.skipped = k - start;
    int tail = k < t->n ? k + 1 : t->n, end = tail;
    for (int i = tail; i < t->n; i++)
        if (is_syn(&t->ev[i], PS_SYN_REPORT)) end = i + 1;
    in.used = k < t->n ? end : t->n;

    // The state before the tail's first frame. What the tail carries is known from it; what it
    // never carries did not change in it, and the device's present state says what that is.
    PenState s = t->good;
    unsigned carried = 0;
    for (int i = tail; i < end; i++) {
        unsigned f = field_of(&t->ev[i]);
        if (!f || (carried & f)) continue;
        carried |= f;
        set_field(&s, f, (f & PS_F_KEYS) ? !t->ev[i].value : t->ev[i].value);
    }
    for (unsigned f = 1; f & PS_F_ALL; f <<= 1)
        if (!(carried & f) && (t->now_ok & f)) set_field(&s, f, get_field(&t->now, f));

    bool moved = false;
    for (int i = tail; i < end; i++) {
        const PenEv *e = &t->ev[i];
        unsigned f = field_of(e);
        if (f) {
            set_field(&s, f, e->value);
            if (f & PS_F_KEYS) in.keys = true;
            else moved = true;
        }
        if (!is_syn(e, PS_SYN_REPORT)) continue;
        if (!in.frames) {
            lift_first(t, &s, e->t_us, out, &n, cap, &in);
            push(out, &n, cap, (PenFrame){.s = s, .t_us = e->t_us, .moved = true, .gap = true});
            in.first_us = e->t_us;
        } else {
            push(out, &n, cap, (PenFrame){.s = s, .t_us = e->t_us, .moved = moved});
        }
        in.last_us = e->t_us;
        in.frames++;
        moved = false;
    }

    if (!in.frames) {
        // Nothing complete after the partial frame: the pen is as the device says now, at the time
        // of the partial frame's SYN_REPORT (or of the last event read).
        uint64_t at = k < t->n ? t->ev[k].t_us : t->n ? t->ev[t->n - 1].t_us : 0;
        lift_first(t, &s, at, out, &n, cap, &in);
        push(out, &n, cap, (PenFrame){.s = s, .t_us = at, .moved = true, .gap = true});
    } else if (t->now_ok & PS_F_KEYS) {
        // A button the tail carried that the device says has changed since: that event came after
        // the tail was read, and EVIOCGKEY took it out of the queue. It happens here.
        PenState after = s;
        for (unsigned f = PS_F_TOOL_PEN; f & PS_F_KEYS; f <<= 1)
            if (t->now_ok & f) set_field(&after, f, get_field(&t->now, f));
        if (after.tool_pen != s.tool_pen || after.tool_rubber != s.tool_rubber || after.touching != s.touching ||
            after.side != s.side) {
            s = after;
            push(out, &n, cap, (PenFrame){.s = s, .t_us = in.last_us});
        }
    }
    in.x = s.x;
    in.y = s.y;
    if (info) *info = in;
    return n;
}

uint32_t pensync_ms(uint64_t t_us, bool mono, uint32_t now) {
    if (!mono || !t_us) return now;
    uint32_t k = (uint32_t)(t_us / 1000u);  // as now_ms() truncates: sec * 1000 + usec / 1000
    uint32_t age = now - k;
    if ((int32_t)age < 0 || age > PENSYNC_K_MAX_AGE_MS) return now;
    return k;
}
