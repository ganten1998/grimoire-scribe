// export.h: turning notebooks into files people can use elsewhere. Pure (no UI);
// shared by the app, the share server, and the host-side tools/grimoire-export CLI.
// Compression uses stbi_zlib_compress from third_party/stb_image_write.h.
#pragma once
#include "store.h"

typedef struct {
    uint8_t *data;
    size_t len, cap;
    bool err;  // sticky: an append ran out of memory, the contents are incomplete
} Buf;
void buf_free(Buf *b);
int buf_append(Buf *b, const void *p, size_t n);
int buf_printf(Buf *b, const char *fmt, ...) __attribute__((format(printf, 2, 3)));

// One page rendered at full size (paper + ink) as an 8-bit grayscale PNG.
int export_page_png(const NotebookInfo *nb, int index, float dpi, Buf *out);
// What export_page_png_ex drew.
typedef struct {
    int w, h;        // the page's full size, px
    uint32_t paper;  // its PaperKind
    Rect ink;        // ink-only renders: box of the pixels darker than 250 (empty if none)
} PageRender;
// The same with only some layers (PAGE_LAYER_* from page.h; a left-out layer is white), shrunk
// `shrink` times by averaging whole blocks (1 = full size). info may be NULL.
int export_page_png_ex(const NotebookInfo *nb, int index, float dpi, int layers, int shrink, PageRender *info,
                       Buf *out);
// Whole notebook as a PDF: one page per notebook page, each a Flate-compressed DeviceGray
// image at the page's physical size (w/dpi inches). Title = notebook name.
int export_notebook_pdf(const NotebookInfo *nb, float dpi, Buf *out);
// A zip (stored or deflated entries, CRC32, no zip64) streaming builder.
typedef struct {
    Buf *out;
    Buf central;
    int count;
} ZipWriter;
void zip_begin(ZipWriter *z, Buf *out);
int zip_add(ZipWriter *z, const char *name, const void *data, size_t len, bool deflate);
int zip_end(ZipWriter *z);
// Notebook zip: "<name>/<name>.pdf", "<name>/page-001.png"..., "<name>/raw/" (notebook.ini + .qpg).
int export_notebook_zip(const NotebookInfo *nb, float dpi, ZipWriter *z, const char *prefix);
// Every notebook in folder ("" = all notebooks, grouped "<folder>/<name>/..."). To bound memory
// on the device, folder/all zips carry each notebook's PDF + raw files, not per-page PNGs.
int export_folder_zip(const char *folder, float dpi, Buf *out);
// Writes the mirror <mirror_dir>/<folder>/<name>.pdf atomically. 0 on success.
int export_mirror_pdf(const NotebookInfo *nb, float dpi, const char *mirror_dir);
// Where export_mirror_pdf puts (or put) this notebook's PDF.
void export_mirror_path(const NotebookInfo *nb, const char *mirror_dir, char *buf, size_t cap);
// File-name-safe version of a notebook/folder name ("/" ":" etc. replaced, never empty).
void export_safe_name(const char *in, char *out, size_t cap);
