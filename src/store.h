// store.h: everything on disk. See docs/DESIGN.md for the layout.
#pragma once
#include "page.h"
#include "input.h"

#define STORE_MAX_PAGES 2000
#define STORE_ID_LEN 24
#define STORE_NAME_LEN 96

typedef struct {
    char id[STORE_ID_LEN];        // directory name, e.g. "20261010-143205-a3f1"
    char name[STORE_NAME_LEN];
    char folder[STORE_NAME_LEN];  // "" = top level; one level of folders only
    PaperKind paper;              // default paper for new pages
    int64_t created, modified;
    int last_page;                // index reopened next time
    int npages;
    uint32_t page_ids[STORE_MAX_PAGES]; // page order; files are p<id>.qpg
    uint32_t next_page_id;
} NotebookInfo;

#define SETTINGS_VERSION 2

typedef struct {
    int version;                  // settings_version from the file (defaults migrate on upgrade)
    ToolParams tools[TOOL_COUNT];
    ToolKind current_tool;
    int panel_x, panel_y;         // floating panel top-left
    bool panel_collapsed;
    int side_button;              // holding the pen button: 0 nothing, 1 eraser, 2 highlighter
    bool mirror_pdf;              // write documents/Quire/<folder>/<name>.pdf on close
    bool follow_rotation;         // turn the UI with the Kindle (otherwise stay as is)
    bool invert;                  // white on black
    bool pinch_zoom;              // two-finger zoom in the editor
    InputCalib pen_calib, touch_calib;
    int http_port;                // share server, default 8765
} Settings;

int store_init(const char *root);   // creates root/, notebooks/, trash/
const char *store_root(void);
// Device geometry used to scale pages drawn at another size and to size stroke bboxes.
void store_set_device(float dpi, int w, int h);
float store_dpi(void);
// The page size new pages get (the device's portrait screen), regardless of rotation.
void store_page_size(int *w, int *h);

void settings_default(Settings *s);
int settings_load(Settings *s);     // missing file / keys -> defaults; values clamped to ParamInfo ranges
int settings_save(const Settings *s);

// Library. store_list allocates *out (free()); sorted by modified, newest first.
int store_list(NotebookInfo **out, int *n);
int store_list_folders(char (**out)[STORE_NAME_LEN], int *n);   // distinct non-empty folders, sorted
// Folders exist as notebook.folder values plus root/folders.ini (so empty folders persist).
int store_add_folder(const char *name);
int store_remove_folder(const char *name);   // only from folders.ini; notebooks keep theirs
int store_create(const char *name, const char *folder, PaperKind paper, NotebookInfo *out); // with 1 empty page
int store_load_info(const char *id, NotebookInfo *out);
int store_save_info(NotebookInfo *nb);      // also bumps modified
int store_rename(const char *id, const char *name);
int store_set_folder(const char *id, const char *folder);
int store_delete(const char *id);           // moves to trash/
int store_notebook_dir(const char *id, char *buf, size_t cap);

// Pages. Index is the position in nb->page_ids.
int store_add_page(NotebookInfo *nb, int at_index, PaperKind paper); // writes empty page + info
int store_delete_page(NotebookInfo *nb, int index);  // keeps >= 1 page; file moved to trash/
// Missing page file -> empty page of nb->paper. Points are scaled if the stored size differs.
int store_load_page(const NotebookInfo *nb, int index, int w, int h, Page **out);
int store_save_page(const NotebookInfo *nb, int index, const Page *p);

// Thumbnails (P5 PGM, ~ 1/8 page size).
int store_save_thumb(const char *id, const Canvas *thumb);
int store_load_thumb(const char *id, Canvas *out);   // canvas_init'd by the callee

// Page file format "QPG1" (little-endian):
//   char magic[4]="QPG1"; u16 version=1; u16 paper; u32 w, h; u32 nstrokes;
//   per stroke: u8 tool; u8 pad[3]; u32 seed; f32 params[13] (ToolParams field order);
//               u32 npts; per point: f32 x, y; u16 pressure*65535; i16 tilt_x*100, tilt_y*100; u32 t_ms
// Readers reject bad magic, clamp counts against the file size, and never trust lengths.
int page_serialize(const Page *p, uint8_t **out, size_t *len);
int page_deserialize(const uint8_t *buf, size_t len, Page **out);
