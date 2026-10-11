// history.h: per-page undo/redo. Strokes removed from the page are owned by the history.
#pragma once
#include "page.h"

typedef enum { ACT_ADD, ACT_ERASE, ACT_PAPER, ACT_CLEAR } ActKind;

typedef struct {
    ActKind kind;
    int count;
    int *indices;       // page indices, ascending (ADD: 1 entry; ERASE/CLEAR: removed ones)
    Stroke **strokes;   // owned when the strokes are NOT on the page
    bool owns;          // whether strokes[] are currently owned by this action
    PaperKind paper_old, paper_new;
} Action;

typedef struct {
    Action *undo; int nundo, capundo;
    Action *redo; int nredo, capredo;
    int limit;          // max undo depth (oldest dropped), default 500
} History;

void history_init(History *h);
void history_free(History *h);     // frees owned strokes
void history_clear(History *h);

// Record an action that has ALREADY been applied to the page.
void history_push_add(History *h, int index, Stroke *s);                  // s stays owned by page
void history_push_erase(History *h, int n, const int *indices, Stroke **removed); // takes ownership of removed
void history_push_paper(History *h, PaperKind old_k, PaperKind new_k);
void history_push_clear(History *h, Page *p);   // call BEFORE clearing: it takes all strokes out

bool history_can_undo(const History *h);
bool history_can_redo(const History *h);
// Apply undo/redo to the page. *dirty receives the page area that must be re-rendered
// (the full page for paper changes). Returns false if nothing to do.
bool history_undo(History *h, Page *p, Rect *dirty);
bool history_redo(History *h, Page *p, Rect *dirty);
