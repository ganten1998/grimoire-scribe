// history.c: per-page undo/redo.
//
// Ownership: an action owns its strokes exactly when they are NOT on the page.
//   ADD    on page after push/redo (not owned); undo removes it (owned).
//   ERASE  removed on push/redo (owned); undo re-inserts them (not owned).
//   CLEAR  same as ERASE for every stroke.
// Strokes are located by pointer on undo/redo; the stored index is the insert position.
#include "history.h"

#include <stdlib.h>
#include <string.h>

void history_init(History *h) {
    memset(h, 0, sizeof *h);
    h->limit = 500;
}

static void action_free(Action *a) {
    if (a->owns)
        for (int i = 0; i < a->count; i++) stroke_free(a->strokes[i]);
    free(a->indices);
    free(a->strokes);
    memset(a, 0, sizeof *a);
}

static void clear_redo(History *h) {
    for (int i = 0; i < h->nredo; i++) action_free(&h->redo[i]);
    h->nredo = 0;
}

void history_clear(History *h) {
    for (int i = 0; i < h->nundo; i++) action_free(&h->undo[i]);
    h->nundo = 0;
    clear_redo(h);
}

void history_free(History *h) {
    history_clear(h);
    free(h->undo);
    free(h->redo);
    memset(h, 0, sizeof *h);
}

static bool push_to(Action **arr, int *n, int *cap, Action a) {
    if (*n == *cap) {
        int nc = *cap ? *cap * 2 : 32;
        Action *na = realloc(*arr, (size_t)nc * sizeof *na);
        if (!na) return false;
        *arr = na;
        *cap = nc;
    }
    (*arr)[(*n)++] = a;
    return true;
}

static void push_undo(History *h, Action a) {
    clear_redo(h);
    if (h->limit > 0 && h->nundo >= h->limit) {
        action_free(&h->undo[0]);
        memmove(&h->undo[0], &h->undo[1], (size_t)(h->nundo - 1) * sizeof *h->undo);
        h->nundo--;
    }
    if (!push_to(&h->undo, &h->nundo, &h->capundo, a)) action_free(&a);
}

static bool alloc_lists(Action *a, int n) {
    a->count = n;
    a->indices = calloc((size_t)(n ? n : 1), sizeof *a->indices);
    a->strokes = calloc((size_t)(n ? n : 1), sizeof *a->strokes);
    if (!a->indices || !a->strokes) {
        free(a->indices);
        free(a->strokes);
        return false;
    }
    return true;
}

void history_push_add(History *h, int index, Stroke *s) {
    Action a = {.kind = ACT_ADD};
    if (!alloc_lists(&a, 1)) return;
    a.indices[0] = index;
    a.strokes[0] = s;
    push_undo(h, a);
}

void history_push_erase(History *h, int n, const int *indices, Stroke **removed) {
    if (n <= 0) return;
    Action a = {.kind = ACT_ERASE, .owns = true};
    if (!alloc_lists(&a, n)) {
        for (int i = 0; i < n; i++) stroke_free(removed[i]);
        return;
    }
    memcpy(a.indices, indices, (size_t)n * sizeof *indices);
    memcpy(a.strokes, removed, (size_t)n * sizeof *removed);
    push_undo(h, a);
}

void history_push_paper(History *h, PaperKind old_k, PaperKind new_k) {
    if (old_k == new_k) return;
    push_undo(h, (Action){.kind = ACT_PAPER, .paper_old = old_k, .paper_new = new_k});
}

void history_push_clear(History *h, Page *p) {
    if (p->n == 0) return;
    Action a = {.kind = ACT_CLEAR, .owns = true};
    if (!alloc_lists(&a, p->n)) return;
    for (int i = 0; i < p->n; i++) {
        a.indices[i] = i;
        a.strokes[i] = p->strokes[i];
    }
    p->n = 0;  // strokes now belong to the action
    push_undo(h, a);
}

bool history_can_undo(const History *h) { return h->nundo > 0; }
bool history_can_redo(const History *h) { return h->nredo > 0; }

static int find_stroke(const Page *p, const Stroke *s, int hint) {
    if (hint >= 0 && hint < p->n && p->strokes[hint] == s) return hint;
    for (int i = p->n - 1; i >= 0; i--)
        if (p->strokes[i] == s) return i;
    return -1;
}

static Rect bbox_of(const Action *a) {
    Rect r = RECT_EMPTY;
    for (int i = 0; i < a->count; i++)
        if (a->strokes[i]) r = rect_union(r, a->strokes[i]->bbox);
    return r;
}

// Takes the action's strokes off the page (becomes owner).
static void take_off(Action *a, Page *p) {
    for (int i = a->count - 1; i >= 0; i--) {
        int at = find_stroke(p, a->strokes[i], a->indices[i]);
        if (at >= 0) page_remove(p, at);
    }
    a->owns = true;
}

// Puts the action's strokes back at their indices (page becomes owner).
static void put_back(Action *a, Page *p) {
    for (int i = 0; i < a->count; i++) {
        int before = p->n;
        page_insert(p, a->indices[i], a->strokes[i]);
        if (p->n == before) a->strokes[i] = NULL;  // insert failed and freed it
    }
    a->owns = false;
}

static bool apply(Action *a, Page *p, Rect *dirty, bool undo) {
    Rect full = {0, 0, p->w, p->h};
    switch (a->kind) {
    case ACT_ADD:
        *dirty = bbox_of(a);
        if (undo) take_off(a, p);
        else put_back(a, p);
        break;
    case ACT_ERASE:
        *dirty = bbox_of(a);
        if (undo) put_back(a, p);
        else take_off(a, p);
        break;
    case ACT_CLEAR:
        *dirty = full;
        if (undo) put_back(a, p);
        else take_off(a, p);
        break;
    case ACT_PAPER:
        *dirty = full;
        p->paper = undo ? a->paper_old : a->paper_new;
        break;
    }
    return true;
}

bool history_undo(History *h, Page *p, Rect *dirty) {
    Rect d = RECT_EMPTY;
    if (!h->nundo) return false;
    Action a = h->undo[--h->nundo];
    apply(&a, p, &d, true);
    if (!push_to(&h->redo, &h->nredo, &h->capredo, a)) action_free(&a);
    if (dirty) *dirty = d;
    return true;
}

bool history_redo(History *h, Page *p, Rect *dirty) {
    Rect d = RECT_EMPTY;
    if (!h->nredo) return false;
    Action a = h->redo[--h->nredo];
    apply(&a, p, &d, false);
    if (!push_to(&h->undo, &h->nundo, &h->capundo, a)) action_free(&a);
    if (dirty) *dirty = d;
    return true;
}
