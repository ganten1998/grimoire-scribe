// bgimage.h: image papers, pictures as page backgrounds (docs/FORMAT.md, Image papers). Each one
// lives in its notebook's folder as bg/<id>.png (the id in 6 hex digits), made once from a source
// picture fitted to the notebook's page size, and never changed after: a page drawn today and in
// any later version looks the same. Drawing one copies it from a small cache of decoded pictures,
// so a region drawn alone gets exactly the pixels it gets in a whole page.
//
// Making a picture (decoding a big JPEG, fitting it, encoding the PNG) takes seconds on the
// Kindle: that runs in a forked child (bgjob.h, src/bgwork_template.c), never on the UI thread.
// Drawing one decodes a page-size PNG once (it stays cached), which the UI thread can afford.
#pragma once
#include "canvas.h"
#include "paper.h"

// ---- drawing (any process: the editor, a share server child, the PDF mirror's child) ----

// Draws image paper k of notebook nb (its id) into region of c (the canvas is the whole page),
// within c's clip: white where the picture does not reach. A picture of another size than the
// canvas is fitted inside it (aspect kept, centred) by the filter bgimage_normalize uses, pixel by
// pixel from page coordinates. A missing or unreadable file draws white, is remembered (it is not
// read again until bgimage_forget or bgimage_retry_missing), and raises bgimage_take_missing's flag.
void bgimage_render(Canvas *c, const char *nb, PaperKind k, Rect region);
// Whether a page drew white since the last call because its picture was missing or unreadable:
// for the editor to say so (renderers in forked children have nobody to tell).
bool bgimage_take_missing(void);
// Forgets the decoded pictures and remembered failures of notebook nb (NULL: of all of them).
void bgimage_forget(const char *nb);
// Forgets only the remembered failures (a job may have written the file since).
void bgimage_retry_missing(void);

// ---- files ----

// root/notebooks/<nb>/bg (-1 for an id that is no notebook's).
int bgimage_dir(const char *nb, char *buf, size_t cap);
// <bgdir>/<id, 6 hex digits>.png
void bgimage_file(const char *bgdir, uint32_t id, char *buf, size_t cap);

// ---- making pictures (slow: forked children) ----

// A picture: w x h pixels of ch channels (1 gray, 3 RGB), rows of w * ch bytes, top row first.
typedef struct {
    uint8_t *px;
    int w, h, ch;
} BgPicture;
void bgpicture_free(BgPicture *p);

// The biggest picture accepted: about 40 megapixels (a 12 MP phone photo is fine; decoding one
// takes up to 4 bytes a pixel, more for a progressive JPEG).
#define BG_MAX_PIXELS 40000000L

// Whether data is a PNG, JPEG or PNM (PGM, PPM) picture Grimoire takes (no bigger than
// BG_MAX_PIXELS, at most 8192 px a side), from its header alone: 0 with its size, or -1 and why.
int bgimage_check(const uint8_t *data, size_t len, int *w, int *h, const char **why);
// Decodes one: transparency over white (8 bits a channel; 16-bit PNGs are cut to 8), and gray
// (ch 1) when no pixel has colour, else RGB. 0, or -1 and why.
int bgimage_decode(const uint8_t *data, size_t len, BgPicture *out, const char **why);
// The picture as it is kept for a pw x ph page: fitted inside the page with its aspect kept,
// centred on white, by an area filter (each page pixel averages the part of the picture it covers,
// in 12-bit fixed point, so every build gets the same bytes). Its channels stay as they are.
int bgimage_normalize(const BgPicture *src, int pw, int ph, BgPicture *out);
// What a gray screen shows of it: its luma (Rec. 709: 0.2126 R + 0.7152 G + 0.0722 B), w x h.
int bgimage_luma(const BgPicture *p, Canvas *out);
// Writes it as a PNG (gray or RGB), atomically. 0 on success.
int bgimage_write_png(const char *path, const BgPicture *p);

// Content keys (64 bits): a kept picture's (its size, channels and pixels), and a template's
// before it is made (the source file's bytes and the page size it is made for). docs/FORMAT.md
// says how ids come from them.
uint64_t bgimage_key(const BgPicture *p);
uint64_t bgimage_source_key(const uint8_t *data, size_t len, int pw, int ph);
// The same key for a file, read a piece at a time (never all of it in memory): 0 with *key and
// *len, or -1 (unreadable, or changed while it was read).
int bgimage_source_key_file(const char *path, int pw, int ph, uint64_t *key, size_t *len);

// ---- bg/index.ini: which id holds which key in a notebook's bg folder ----

typedef struct {
    uint32_t id;
    uint64_t key;
} BgIndexEntry;
typedef struct {
    BgIndexEntry *e;
    int n, cap;
} BgIndex;
// Reads <bgdir>/index.ini (none: empty). 0, or -1 out of memory.
int bgindex_load(const char *bgdir, BgIndex *ix);
int bgindex_save(const char *bgdir, const BgIndex *ix);   // atomically, sorted by id
void bgindex_free(BgIndex *ix);
bool bgindex_find(const BgIndex *ix, uint64_t key, uint32_t *id);
// The id for key: the one the index gives it, else a new one, added to ix (not yet saved) and
// *added set: the key's low 24 bits, stepping to the next id past one that holds another key or
// that has a file in bgdir with no entry (whatever it holds). 0 is never given. 0 on success.
int bgindex_claim(BgIndex *ix, const char *bgdir, uint64_t key, uint32_t *id, bool *added);
