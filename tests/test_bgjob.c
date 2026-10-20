// test_bgjob.c: the background queue (src/bgjob.h): which job runs next, and which one a full
// queue lets go. The jobs here end at once (no such notebook, file or folder), which is enough:
// bg_poll_kind says the kind of each as it ends, and the result of each the user waits on.
#include <unistd.h>

#include "bgjob.h"
#include "store.h"
#include "testlib.h"

static char root[600];

typedef struct {
    int kind, result;
} Ended;

// Runs the queue dry: the jobs in the order they ended (at most cap of them).
static int drain(Ended *e, int cap) {
    int n = 0;
    for (int ms = 0; bg_busy() && ms < 30000;) {
        int k = -1, r = bg_poll_kind(&k);
        if (k >= 0) {
            if (n < cap) e[n] = (Ended){k, r};
            n++;
        } else {
            sleep_ms(2);
            ms += 2;
        }
    }
    return n;
}

// Something the user waits on first (a template's picture, an export they asked for, in the order
// they came), then the mirror PDFs, then imports, and compaction last: the first job queued starts
// at once whatever it is.
TEST(what_the_user_waits_on_goes_first) {
    CHECK(!bg_busy());
    bg_queue(BG_COMPACT_PAGES, "nb-a", NULL, 300.f, false);
    bg_queue(BG_COMPACT_PAGES, "nb-b", NULL, 300.f, false);
    bg_queue(BG_IMPORT_FOLDER, "", "/nonexistent/import/Book", 300.f, false);
    bg_mirror_pdf("nb-c", 300.f, "/nonexistent/PDFs", false);
    bg_queue(BG_TEMPLATE, "nb-d", "/nonexistent/t.png", 300.f, false);
    bg_mirror_pdf("nb-e", 300.f, "/nonexistent/PDFs", true);
    Ended e[8];
    CHECK(drain(e, 8) == 6);
    CHECK(e[0].kind == BG_COMPACT_PAGES && e[1].kind == BG_TEMPLATE);
    CHECK(e[2].kind == BG_MIRROR_PDF && e[2].result == -1);  // the export asked for (it fails here)
    CHECK(e[3].kind == BG_MIRROR_PDF && e[3].result == 0);   // the mirror, which reports nothing
    CHECK(e[4].kind == BG_IMPORT_FOLDER && e[5].kind == BG_COMPACT_PAGES);
}

// A full queue lets its oldest compaction go (the pages stay listed for the next one), not the
// oldest job: here every mirror runs and the compaction does not.
TEST(a_full_queue_lets_a_compaction_go_first) {
    CHECK(!bg_busy());
    char id[16];
    bg_mirror_pdf("running", 300.f, "/nonexistent/PDFs", false);
    for (int i = 0; i < 8; i++) {
        snprintf(id, sizeof id, "m%d", i);
        bg_mirror_pdf(id, 300.f, "/nonexistent/PDFs", false);
    }
    bg_queue(BG_COMPACT_PAGES, "nb-old", NULL, 300.f, false);
    for (int i = 8; i < 16; i++) {  // the 16th queued job fills the queue, the 17th finds it full
        snprintf(id, sizeof id, "m%d", i);
        bg_mirror_pdf(id, 300.f, "/nonexistent/PDFs", false);
    }
    Ended e[20];
    int n = drain(e, 20);
    CHECK(n == 17);
    int compactions = 0;
    for (int i = 0; i < n && i < 20; i++) compactions += e[i].kind == BG_COMPACT_PAGES;
    CHECK(compactions == 0);
}

int main(void) {
    snprintf(root, sizeof root, "%s/bgjob-%d", tl_tmp(), (int)getpid());
    char cmd[700];
    snprintf(cmd, sizeof cmd, "rm -rf '%s'", root);
    system(cmd);
    CHECK(store_init(root) == 0);
    RUN(what_the_user_waits_on_goes_first);
    RUN(a_full_queue_lets_a_compaction_go_first);
    int rc = tl_done();
    system(cmd);
    return rc;
}
