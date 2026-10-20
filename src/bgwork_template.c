// bgwork_template.c: BG_TEMPLATE (a template image normalized into a notebook) and
// BG_IMPORT_FOLDER (a folder of page images made into a notebook).
// Placeholder until templates land (they replace this file): both refuse.
#include "bgjob.h"

int bg_template(const char *id, const char *src, float dpi) {
    (void)id;
    (void)src;
    (void)dpi;
    return -1;
}

int bg_import_folder(const char *path, float dpi) {
    (void)path;
    (void)dpi;
    return -1;
}
