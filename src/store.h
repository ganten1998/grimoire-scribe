// store.h: everything on disk. See docs/DESIGN.md for the layout and docs/FORMAT.md for the
// files themselves.
#pragma once
#include "page.h"
#include "input.h"

#define STORE_MAX_PAGES 2000
#define STORE_ID_LEN 24
#define STORE_NAME_LEN 96
#define STORE_EXTRA_LEN 2048
#define STORE_SOURCE_LEN 64

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
    int page_w, page_h;           // its pages' size, px: the screen it was made on, kept wherever it
                                  // is opened (notebook.ini page_w, page_h; see store_load_info)
    char source[STORE_SOURCE_LEN]; // the book it annotates (notebook.ini source=): KOReader's key of
                                   // the book's file (annotate.h); "" for none
    char extra[STORE_EXTRA_LEN];  // notebook.ini lines this build does not know ("key=value\n"
                                  // each), written back as they were
} NotebookInfo;

#define SETTINGS_VERSION 2
#define PANEL_PINS_MAX 4          // pens pinned to the editor's panel, at most

typedef struct {
    int version;                  // settings_version from the file (defaults migrate on upgrade)
    ToolParams tools[TOOL_COUNT];
    ToolKind current_tool;        // what the pen draws with (may be the eraser)
    ToolKind draw_tool;           // the drawing tool the toolbar's tool button shows
    PaperKind music_layout;       // the last custom music layout built (offered on the Custom tile)
    int panel_x, panel_y;         // floating panel top-left
    bool panel_collapsed;
    int side_button;              // holding the pen button: 0 nothing, 1 eraser, 2 highlighter,
                                  // 3 white ink with the pen's own drawing tool
    bool mirror_pdf;              // write grimoire/PDFs/<folder>/<name>.pdf on close
    bool follow_rotation;         // turn the UI with the Kindle (otherwise stay as is)
    bool invert;                  // white on black
    bool pinch_zoom;              // two-finger zoom in the editor
    int sleep_mode;               // sleep screen: 0 the page I'm on, 1 a pinned page, 2 the Kindle's own
    int sleep_minutes;            // sleep after this long without input (0 = never)
    int theme;                    // UiTheme: 0 gothic, 1 classic (settings.ini: theme=gothic|classic)
    InputCalib pen_calib, touch_calib;
    // The panel each calibration was made on (settings.ini pen.panel, touch.panel: WxH). A
    // calibration maps to panel pixels, so on a panel of another size it is scaled (app_init).
    // 0 x 0: not recorded (a calibration from before 1.1, or one a version without the key saved).
    int pen_panel_w, pen_panel_h, touch_panel_w, touch_panel_h;
    int http_port;                // share server, default 8765
    bool passcode_told;           // the "passcode paused while Grimoire is open" notice was shown once
    char language[8];             // settings.ini language=: "auto" (the Kindle's), or a code of i18n.h's list
    int eraser_mode;              // the eraser (its button, the eraser end, side_button 1): 0 takes
                                  // whole strokes away, 1 the precise eraser (erase strokes)
    bool white;                   // the drawing tools draw white ink (erase strokes, own texture)
    // The pens pinned to the editor's panel under its tool button, in pin order: drawing tools
    // only, each once (settings.ini panel_tools=, their names comma-separated).
    ToolKind pins[PANEL_PINS_MAX];
    int npins;
    // The pen in use when the eraser was last picked on the panel, a tool id or -1 for none: where
    // a tap on the eraser in use goes back to while that pen is still on the panel.
    int before_eraser;
} Settings;

int store_init(const char *root);   // creates root/, notebooks/, trash/
const char *store_root(void);
// Device geometry: the page size of the notebooks made here, and the dpi stroke bboxes are sized at.
void store_set_device(float dpi, int w, int h);
float store_dpi(void);
// The page size new notebooks get (the device's portrait screen), regardless of rotation.
void store_page_size(int *w, int *h);
// The size of a notebook's pages: nb->page_w x nb->page_h, which store_create and store_load_info
// always set (the device's size for a NotebookInfo that has none).
void store_notebook_page_size(const NotebookInfo *nb, int *w, int *h);

void settings_default(Settings *s);
// Missing file / keys -> defaults; values clamped to ParamInfo ranges. What the file holds that
// this build does not understand (a newer version's keys, or values beyond its ranges) is kept
// and written back by settings_save (docs/FORMAT.md).
int settings_load(Settings *s);
int settings_save(const Settings *s);
// settings.ini's language= as the file has it, last line counting ("" without the file or the
// key), read on its own: main.c sets the language before the app, and its settings, start.
void settings_language(char *out, size_t cap);
// The pinned pens: whether t is one; t pinned after the others, or unpinned (the others keep their
// order), false for the eraser, a tool past the list, or a pin past PANEL_PINS_MAX; and their
// settings.ini names in pin order, comma-separated, as panel_tools= has them ("" for none).
bool settings_pinned(const Settings *s, ToolKind t);
bool settings_toggle_pin(Settings *s, ToolKind t);
void settings_pins_text(const Settings *s, char *out, size_t cap);

// Library. store_list allocates *out (free()); sorted by modified, newest first.
int store_list(NotebookInfo **out, int *n);
int store_list_folders(char (**out)[STORE_NAME_LEN], int *n);   // distinct non-empty folders, sorted
// Folders exist as notebook.folder values plus root/folders.ini (so empty folders persist).
int store_add_folder(const char *name);
int store_remove_folder(const char *name);   // only from folders.ini; notebooks keep theirs
// With 1 empty page, at the device's page size (recorded as the notebook's).
int store_create(const char *name, const char *folder, PaperKind paper, NotebookInfo *out);
// A notebook.ini with no page size (versions before 1.1 write none, and 1.0.0-beta drops it on a
// save) takes the size of its first page file that has one, else the device's.
int store_load_info(const char *id, NotebookInfo *out);
// Pages found damaged since the last call (each was renamed to p<id>.qpg.damaged-<time>).
int store_take_corrupt_count(void);
// When a notebook is opened: pages an older version set aside (p<id>.qpg.damaged-<time>, as
// 1.0.0-beta does with any page from a newer version) that open now go back to p<id>.qpg, the
// newest copy if there are several. Only pages in the page list come back; if p<id>.qpg exists
// again, both are kept. Returns how many came back; each is logged.
int store_recover_pages(const NotebookInfo *nb);
int store_save_info(NotebookInfo *nb);      // also bumps modified
// A book key as notebook.ini's source= holds one: 1 to 63 letters, digits, '-' or '_'. A source=
// line with any other value is one this version does not know: kept as it is (NotebookInfo.extra).
bool store_source_ok(const char *key);
// A fresh notebook id ("20261010-143205-a3f1": the local time and 4 random hex digits).
void store_make_id(char id[STORE_ID_LEN]);
int store_rename(const char *id, const char *name);
int store_set_folder(const char *id, const char *folder);
int store_delete(const char *id);           // moves to trash/
int store_notebook_dir(const char *id, char *buf, size_t cap);

// Pages. Index is the position in nb->page_ids.
// Writes an empty page at the notebook's page size (never over a page file already there: the
// next free id) + info. Its paper is store_new_page_paper(nb, paper): callers pass the paper of
// the page it follows (or replaces).
int store_add_page(NotebookInfo *nb, int at_index, PaperKind paper);
// The paper of a page added after (or left in place of) a page on `after`: the same paper, but a
// document page's picture (an imported page image, paper_is_document) belongs to that page alone:
// then the notebook's own paper (Blank if that is a document page's too). A template stays.
PaperKind store_new_page_paper(const NotebookInfo *nb, PaperKind after);
int store_delete_page(NotebookInfo *nb, int index);  // keeps >= 1 page; file moved to trash/
// The page at w x h, the notebook's page size (store_notebook_page_size): a page file of another
// size, one an older version scaled to its own screen, has its points scaled back. Missing page
// file -> empty page of nb->paper. A damaged file is set aside (p<id>.qpg.damaged-<time>). A page
// from a newer version comes back read-only (Page.readonly) and is never set aside. A page that
// bg/pages.ini lists with an image paper but that reads as Blank (an older version saved it so)
// gets its image paper back, and is written so again (each such page counted, below). Every page
// comes with Page.nb set to the notebook's id.
int store_load_page(const NotebookInfo *nb, int index, int w, int h, Page **out);
// How many pages store_load_page gave their image paper back since the last call: the thumbnail
// and PDF an older version made of them show blank paper, to be drawn again.
int store_take_restored_count(void);
// When a notebook is opened in the editor: every page bg/pages.ini lists that reads as version 1
// Blank (an older version saved it so) gets its image paper back now and is written so, as
// store_load_page does for one page as it opens: then the notebook's thumbnail and PDF, which the
// older version drew on blank paper, are drawn again when it closes, also for pages not visited.
// Each counts in store_take_restored_count. Returns how many.
int store_restore_listed_pages(const NotebookInfo *nb);
// A notebook store_create just made (its one empty page) goes onto `paper`, which may be an image
// paper of its own pictures: store_create takes only a paper the pickers offer, and a template's
// picture can go into a notebook's bg/ only once the notebook exists (templates.h). Its own paper
// (notebook.ini, what new pages get) and its first page's (listed in bg/pages.ini) are written.
int store_start_on(NotebookInfo *nb, PaperKind paper);
// The same for exports, always at the notebook's page size, and nothing on disk changes: a page
// that is missing or does not read is a blank page of nb->paper, and its file stays as it is.
int store_read_page(const NotebookInfo *nb, int index, Page **out);
// A read-only page is never written: its file stays as it is (and this returns 0). bg/pages.ini
// follows the page's paper (an image paper listed, any other not).
int store_save_page(const NotebookInfo *nb, int index, const Page *p);

// The pinned sleep screen (a full page, P5 PGM at root/sleep.pgm).
int store_save_sleep_image(const Canvas *c);
int store_load_sleep_image(Canvas *out);

// Thumbnails (P5 PGM, ~ 1/8 page size).
int store_save_thumb(const char *id, const Canvas *thumb);
int store_load_thumb(const char *id, Canvas *out);   // canvas_init'd by the callee
// thumb.pgm as it is on disk now, in a number that changes whenever the file is written again (a
// write is a new file, renamed into place: its inode, size and times): 0 when there is none, else
// odd. A card read from it knows by this whether it was written since.
uint64_t store_thumb_stamp(const char *id);
// From a background job, which draws a notebook's card while the editor may have the notebook open
// and write the card itself: thumb.pgm's bytes as they are now (NULL with *len 0 when there is
// none), and the card written only if thumb.pgm still holds those bytes (was, was_len), checked
// once the new card is written to a file of the job's own (thumb.pgm.card.tmp, so that the two
// never write into one file), just before it replaces the old one. 0 written; 1 the card changed
// meanwhile (it stays as it is); -1 not written.
uint8_t *store_read_thumb_file(const char *id, size_t *len);
int store_replace_thumb_if_same(const char *id, const Canvas *thumb, const uint8_t *was, size_t was_len);
// Tests: called by store_replace_thumb_if_same just before it looks at thumb.pgm again (the editor
// writing a card meanwhile).
extern void (*store_test_before_card_replace)(const char *id);
int store_write_pgm(const char *path, const Canvas *c);   // P5 PGM, atomically
int store_read_pgm(const char *path, Canvas *out);        // canvas_init'd by the callee

// A notebook built in a folder of its own before it becomes one (an import, in a dot-folder that
// store_list skips, renamed into notebooks/ once whole): its notebook.ini (modified as it is), a
// page file, and bg/pages.ini for pages pids[i] on papers[i] (image papers only; bgdir is its bg).
int store_write_info_in(const char *dir, const NotebookInfo *nb);
int store_write_page_in(const char *dir, uint32_t pid, const Page *p);
int store_write_pages_list_in(const char *bgdir, const uint32_t *pids, const PaperKind *papers, int n);

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
