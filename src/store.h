// store.h: everything on disk. See docs/DESIGN.md for the layout and docs/FORMAT.md for the
// files themselves.
#pragma once
#include "page.h"
#include "input.h"

#define STORE_MAX_PAGES 2000
#define STORE_ID_LEN 24
#define STORE_NAME_LEN 96
#define STORE_EXTRA_LEN 2048

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
    char extra[STORE_EXTRA_LEN];  // notebook.ini lines this build does not know ("key=value\n"
                                  // each), written back as they were
} NotebookInfo;

#define SETTINGS_VERSION 2

typedef struct {
    int version;                  // settings_version from the file (defaults migrate on upgrade)
    ToolParams tools[TOOL_COUNT];
    ToolKind current_tool;        // what the pen draws with (may be the eraser)
    ToolKind draw_tool;           // the drawing tool the toolbar's tool button shows
    PaperKind music_layout;       // the last custom music layout built (offered on the Custom tile)
    int panel_x, panel_y;         // floating panel top-left
    bool panel_collapsed;
    int side_button;              // holding the pen button: 0 nothing, 1 eraser, 2 highlighter
    bool mirror_pdf;              // write grimoire/PDFs/<folder>/<name>.pdf on close
    bool follow_rotation;         // turn the UI with the Kindle (otherwise stay as is)
    bool invert;                  // white on black
    bool pinch_zoom;              // two-finger zoom in the editor
    int sleep_mode;               // sleep screen: 0 the page I'm on, 1 a pinned page, 2 the Kindle's own
    int sleep_minutes;            // sleep after this long without input (0 = never)
    int theme;                    // UiTheme: 0 gothic, 1 classic (settings.ini: theme=gothic|classic)
    InputCalib pen_calib, touch_calib;
    int http_port;                // share server, default 8765
    bool passcode_told;           // the "passcode paused while Grimoire is open" notice was shown once
} Settings;

int store_init(const char *root);   // creates root/, notebooks/, trash/
const char *store_root(void);
// Device geometry used to scale pages drawn at another size and to size stroke bboxes.
void store_set_device(float dpi, int w, int h);
float store_dpi(void);
// The page size new pages get (the device's portrait screen), regardless of rotation.
void store_page_size(int *w, int *h);

void settings_default(Settings *s);
// Missing file / keys -> defaults; values clamped to ParamInfo ranges. What the file holds that
// this build does not understand (a newer version's keys, or values beyond its ranges) is kept
// and written back by settings_save (docs/FORMAT.md).
int settings_load(Settings *s);
int settings_save(const Settings *s);

// Library. store_list allocates *out (free()); sorted by modified, newest first.
int store_list(NotebookInfo **out, int *n);
int store_list_folders(char (**out)[STORE_NAME_LEN], int *n);   // distinct non-empty folders, sorted
// Folders exist as notebook.folder values plus root/folders.ini (so empty folders persist).
int store_add_folder(const char *name);
int store_remove_folder(const char *name);   // only from folders.ini; notebooks keep theirs
int store_create(const char *name, const char *folder, PaperKind paper, NotebookInfo *out); // with 1 empty page
int store_load_info(const char *id, NotebookInfo *out);
// Pages found damaged since the last call (each was renamed to p<id>.qpg.damaged-<time>).
int store_take_corrupt_count(void);
// When a notebook is opened: pages an older version set aside (p<id>.qpg.damaged-<time>, as
// 1.0.0-beta does with any page from a newer version) that open now go back to p<id>.qpg, the
// newest copy if there are several. Only pages in the page list come back; if p<id>.qpg exists
// again, both are kept. Returns how many came back; each is logged.
int store_recover_pages(const NotebookInfo *nb);
int store_save_info(NotebookInfo *nb);      // also bumps modified
int store_rename(const char *id, const char *name);
int store_set_folder(const char *id, const char *folder);
int store_delete(const char *id);           // moves to trash/
int store_notebook_dir(const char *id, char *buf, size_t cap);

// Pages. Index is the position in nb->page_ids.
// Writes an empty page (never over a page file already there: the next free id) + info. Its paper
// is store_new_page_paper(nb, paper): callers pass the paper of the page it follows (or replaces).
int store_add_page(NotebookInfo *nb, int at_index, PaperKind paper);
// The paper of a page added after (or left in place of) a page on `after`: the same paper, but a
// document page's picture (1.1's imported page image, paper_is_document) belongs to that page
// alone: then the notebook's own paper (Blank if that is a document page's too).
PaperKind store_new_page_paper(const NotebookInfo *nb, PaperKind after);
int store_delete_page(NotebookInfo *nb, int index);  // keeps >= 1 page; file moved to trash/
// Missing page file -> empty page of nb->paper. Points are scaled if the stored size differs.
// A page from a newer version comes back read-only (Page.readonly) and is never set aside.
int store_load_page(const NotebookInfo *nb, int index, int w, int h, Page **out);
// A read-only page is never written: its file stays as it is (and this returns 0).
int store_save_page(const NotebookInfo *nb, int index, const Page *p);

// The pinned sleep screen (a full page, P5 PGM at root/sleep.pgm).
int store_save_sleep_image(const Canvas *c);
int store_load_sleep_image(Canvas *out);

// Thumbnails (P5 PGM, ~ 1/8 page size).
int store_save_thumb(const char *id, const Canvas *thumb);
int store_load_thumb(const char *id, Canvas *out);   // canvas_init'd by the callee

// Page file format "QPG1" (little-endian), byte by byte in docs/FORMAT.md:
//   char magic[4]="QPG1"; u16 version; u16 paper; u32 w, h;   (these 16 bytes in every version)
//   (version 2, a paper 1.0.0-beta does not know: u32 paper follows h; the u16 then says
//    PAPER_MUSIC for custom music, PAPER_BLANK otherwise)  u32 nstrokes;
//   per stroke: u8 tool; u8 ext[3]; u32 seed; f32 params[13] (ToolParams field order);
//               u32 npts; per point: f32 x, y; u16 pressure*65535; i16 tilt_x*100, tilt_y*100; u32 t_ms
//   then chunks to the end of the file: u32 tag, u32 length, length bytes.
// Readers reject bad magic, clamp counts against the file size, and never trust lengths. A page
// from a newer version (version 3 to 255, a tool or a critical chunk this build does not know,
// bytes after the strokes that are not chunks) reads as a read-only page instead of failing:
// page_deserialize returns 0 with Page.readonly set, and page_serialize refuses such a page.
int page_serialize(const Page *p, uint8_t **out, size_t *len);
int page_deserialize(const uint8_t *buf, size_t len, Page **out);
