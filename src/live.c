// live.c: Live view (live.h). The main loop owns the shared frame and the viewer table; each viewer
// is a forked child that reads the frame, never writes it, and owns one browser's connection.
#include "live.h"

#include <errno.h>
#include <fcntl.h>
#include <netinet/in.h>
#include <netinet/tcp.h>
#include <poll.h>
#include <signal.h>
#include <stdatomic.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/mman.h>
#include <sys/socket.h>
#include <sys/time.h>
#include <sys/wait.h>
#include <unistd.h>
#ifdef __linux__
#include <sys/prctl.h>
#endif

#include "stb_image_write.h"
// (stb's own PNG writer to memory, which its header does not declare; export.c and httpd.c use it too)
extern unsigned char *stbi_write_png_to_mem(const unsigned char *, int, int, int, int, int *);

#ifndef MSG_NOSIGNAL
#define MSG_NOSIGNAL 0
#endif

_Static_assert(ATOMIC_INT_LOCK_FREE == 2, "the tile marks must work across fork");

// The frame both sides map. w and h change only on a turn, written before epoch moves on; px holds
// w * h bytes (the same in either orientation), rows w apart. Each viewer slot has marks of its
// own: a viewer takes its marks as it sends, which must leave the other's.
typedef struct {
    atomic_uint epoch, state, seq, invert;
    atomic_uint w, h;
    atomic_uint dirty[LIVE_VIEWERS][LIVE_WORDS];
    uint8_t px[];
} Shared;

static Shared *g_sh;
static size_t g_sh_size;
static struct {
    pid_t pid;
    int wake;  // the write end of its wake pipe (non-blocking)
} g_v[LIVE_VIEWERS];
static uint32_t g_last_viewer;  // when the last viewer left (or Live view started)

// ---- pure parts ----

int live_tiles_across(int w) { return (w + LIVE_TILE - 1) / LIVE_TILE; }

void live_mark(uint32_t *words, int w, int h, Rect r) {
    r = rect_intersect(r, (Rect){0, 0, w, h});
    if (rect_is_empty(r)) return;
    int across = live_tiles_across(w);
    for (int ty = r.y / LIVE_TILE; ty <= (r.y + r.h - 1) / LIVE_TILE; ty++)
        for (int tx = r.x / LIVE_TILE; tx <= (r.x + r.w - 1) / LIVE_TILE; tx++) {
            int i = ty * across + tx;
            if (i < LIVE_WORDS * 32) words[i / 32] |= 1u << (i % 32);
        }
}

static bool marked(const uint32_t *words, int i) { return i < LIVE_WORDS * 32 && (words[i / 32] >> (i % 32)) & 1u; }

int live_runs(const uint32_t *words, int w, int h, Rect *out, int cap) {
    int across = live_tiles_across(w), down = (h + LIVE_TILE - 1) / LIVE_TILE, n = 0;
    for (int ty = 0; ty < down; ty++)
        for (int tx = 0; tx < across;) {
            if (!marked(words, ty * across + tx)) {
                tx++;
                continue;
            }
            int end = tx;
            while (end < across && marked(words, ty * across + end)) end++;
            if (n < cap)
                out[n++] = rect_intersect((Rect){tx * LIVE_TILE, ty * LIVE_TILE, (end - tx) * LIVE_TILE, LIVE_TILE},
                                          (Rect){0, 0, w, h});
            tx = end;
        }
    return n;
}

void live_frame_header(uint8_t out[5], uint32_t len, uint8_t type) {
    uint32_t total = len + 1;
    out[0] = (uint8_t)total;
    out[1] = (uint8_t)(total >> 8);
    out[2] = (uint8_t)(total >> 16);
    out[3] = (uint8_t)(total >> 24);
    out[4] = type;
}

// ---- the main loop ----

bool live_on(void) { return g_sh != NULL; }

int live_viewers(void) {
    int n = 0;
    for (int i = 0; i < LIVE_VIEWERS; i++) n += g_v[i].pid > 0;
    return n;
}

static void wake_all(void) {
    for (int i = 0; i < LIVE_VIEWERS; i++)
        if (g_v[i].pid > 0 && write(g_v[i].wake, "x", 1) < 0) { /* full: it is awake already */ }
}

static void mark_all(void) {
    for (int v = 0; v < LIVE_VIEWERS; v++)
        for (int i = 0; i < LIVE_WORDS; i++) atomic_store(&g_sh->dirty[v][i], 0xFFFFFFFFu);
}

int live_start(int w, int h) {
    if (g_sh) return 0;
    if (w <= 0 || h <= 0 || live_tiles_across(w) * ((h + LIVE_TILE - 1) / LIVE_TILE) > LIVE_WORDS * 32 ||
        live_tiles_across(h) * ((w + LIVE_TILE - 1) / LIVE_TILE) > LIVE_WORDS * 32)
        return -1;
    size_t size = sizeof(Shared) + (size_t)w * (size_t)h;
    void *p = mmap(NULL, size, PROT_READ | PROT_WRITE, MAP_SHARED | MAP_ANON, -1, 0);
    if (p == MAP_FAILED) return -1;
    g_sh = p;
    g_sh_size = size;
    memset(g_sh->px, 255, (size_t)w * (size_t)h);
    atomic_store(&g_sh->w, (unsigned)w);
    atomic_store(&g_sh->h, (unsigned)h);
    atomic_store(&g_sh->state, LIVE_AWAKE);
    atomic_store(&g_sh->epoch, 1);
    mark_all();
    g_last_viewer = now_ms();
    return 0;
}

static void reap(int i, bool wait_for_it) {
    if (g_v[i].pid <= 0) return;
    if (waitpid(g_v[i].pid, NULL, wait_for_it ? 0 : WNOHANG) <= 0 && !wait_for_it) return;
    close(g_v[i].wake);
    g_v[i].pid = 0;
    g_v[i].wake = -1;
    if (!live_viewers()) g_last_viewer = now_ms();
}

void live_stop(void) {
    if (!g_sh) return;
    atomic_store(&g_sh->state, LIVE_BYE);
    wake_all();
    // Each viewer says bye and ends; one that does not within a second is stopped.
    uint32_t t0 = now_ms();
    while (live_viewers() && now_ms() - t0 < 1000) {
        for (int i = 0; i < LIVE_VIEWERS; i++) reap(i, false);
        if (live_viewers()) sleep_ms(10);
    }
    for (int i = 0; i < LIVE_VIEWERS; i++)
        if (g_v[i].pid > 0) {
            kill(g_v[i].pid, SIGTERM);
            reap(i, true);
        }
    munmap(g_sh, g_sh_size);
    g_sh = NULL;
}

void live_push(const Canvas *screen, Rect r) {
    if (!g_sh) return;
    int w = (int)atomic_load(&g_sh->w), h = (int)atomic_load(&g_sh->h);
    if (screen->w != w || screen->h != h) return;  // (a turn not told yet: live_resize comes)
    r = rect_intersect(r, (Rect){0, 0, w, h});
    if (rect_is_empty(r)) return;
    for (int y = r.y; y < r.y + r.h; y++)
        memcpy(&g_sh->px[(size_t)y * (size_t)w + (size_t)r.x], &screen->px[(size_t)y * (size_t)screen->stride + (size_t)r.x],
               (size_t)r.w);
    // Marked after the copy: a viewer that read a tile while it was being written sees it marked
    // again, and sends it again whole.
    uint32_t words[LIVE_WORDS] = {0};
    live_mark(words, w, h, r);
    for (int v = 0; v < LIVE_VIEWERS; v++)
        for (int i = 0; i < LIVE_WORDS; i++)
            if (words[i]) atomic_fetch_or(&g_sh->dirty[v][i], words[i]);
    wake_all();
}

void live_resize(int w, int h) {
    if (!g_sh || (size_t)w * (size_t)h + sizeof(Shared) != g_sh_size) return;
    atomic_store(&g_sh->w, (unsigned)w);
    atomic_store(&g_sh->h, (unsigned)h);
    memset(g_sh->px, 255, (size_t)w * (size_t)h);
    atomic_fetch_add(&g_sh->epoch, 1);
    mark_all();
    wake_all();
}

void live_set_state(int state) {
    if (!g_sh) return;
    // The state first, then (for the pairing card) its pixels: a viewer that reads them sees the
    // state after its own fence, and sends nothing.
    atomic_store(&g_sh->state, (unsigned)state);
    atomic_thread_fence(memory_order_seq_cst);
    if (state == LIVE_AWAKE) mark_all();  // whatever showed meanwhile comes again
    wake_all();
}

void live_set_invert(bool on) {
    if (!g_sh || atomic_exchange(&g_sh->invert, on ? 1u : 0u) == (on ? 1u : 0u)) return;
    wake_all();
}

int live_poll(uint32_t now, bool *idle_off) {
    if (idle_off) *idle_off = false;
    if (!g_sh) return 0;
    for (int i = 0; i < LIVE_VIEWERS; i++) reap(i, false);
    int n = live_viewers();
    if (!n && idle_off && (int32_t)(now - g_last_viewer) >= LIVE_IDLE_OFF_MS) *idle_off = true;
    return n;
}

// ---- a viewer ----

static bool send_all(int fd, const void *data, size_t len) {
    const uint8_t *p = data;
    while (len) {
        ssize_t w = send(fd, p, len, MSG_NOSIGNAL);
        if (w < 0 && errno == EINTR) continue;
        if (w <= 0) return false;
        p += w;
        len -= (size_t)w;
    }
    return true;
}

static bool send_frame(int fd, uint8_t type, const void *a, size_t alen, const void *b, size_t blen) {
    uint8_t head[5];
    live_frame_header(head, (uint32_t)(alen + blen), type);
    return send_all(fd, head, 5) && (!alen || send_all(fd, a, alen)) && (!blen || send_all(fd, b, blen));
}

static void put16(uint8_t *p, unsigned v) { p[0] = (uint8_t)v; p[1] = (uint8_t)(v >> 8); }
static void put32(uint8_t *p, uint32_t v) { put16(p, v & 0xFFFF); put16(p + 2, v >> 16); }

static bool send_hello(int fd, int w, int h, uint32_t epoch) {
    uint8_t p[12];
    put16(p, 1);
    put16(p + 2, (unsigned)w);
    put16(p + 4, (unsigned)h);
    put16(p + 6, LIVE_TILE);
    put32(p + 8, epoch);
    return send_frame(fd, LIVE_HELLO, p, sizeof p, NULL, 0);
}

static bool send_state(int fd, unsigned state, unsigned invert) {
    uint8_t s[2] = {(uint8_t)state, (uint8_t)(invert ? 1 : 0)};
    return send_frame(fd, LIVE_STATE, s, sizeof s, NULL, 0);
}

static void viewer_main(int slot, int fd, int wake) {
    atomic_uint *dirty = g_sh->dirty[slot];
    for (int i = 3; i < 1024; i++)
        if (i != fd && i != wake) close(i);
    struct sigaction sa;
    memset(&sa, 0, sizeof sa);
    sa.sa_handler = SIG_DFL;
    sigaction(SIGTERM, &sa, NULL);
    sigaction(SIGINT, &sa, NULL);
    sigaction(SIGHUP, &sa, NULL);
    sa.sa_handler = SIG_IGN;
    sigaction(SIGPIPE, &sa, NULL);
#ifdef __linux__
    prctl(PR_SET_NAME, "grimoire-live");
    prctl(PR_SET_PDEATHSIG, SIGTERM);
#endif
    pid_t parent = getppid();
    if (nice(5) < 0) { /* best effort */ }
    fcntl(fd, F_SETFL, fcntl(fd, F_GETFL, 0) & ~O_NONBLOCK);
    int one = 1, sndbuf = 256 * 1024;
    setsockopt(fd, IPPROTO_TCP, TCP_NODELAY, &one, sizeof one);
    setsockopt(fd, SOL_SOCKET, SO_SNDBUF, &sndbuf, sizeof sndbuf);
    struct timeval tv = {10, 0};  // a browser that takes nothing for 10 s is gone
    setsockopt(fd, SOL_SOCKET, SO_SNDTIMEO, &tv, sizeof tv);
    stbi_write_force_png_filter = 1;  // (this process's own: the fastest filter, a little bigger)
    static const char head[] = "HTTP/1.0 200 OK\r\nContent-Type: application/octet-stream\r\n"
                               "Cache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nConnection: close\r\n\r\n";
    size_t cap = g_sh_size - sizeof(Shared);
    uint8_t *sent = malloc(cap), *tile = malloc((size_t)LIVE_TILE * 4096);
    if (!sent || !tile || !send_all(fd, head, sizeof head - 1)) _exit(0);
    uint32_t epoch = 0, last_batch = 0, last_sent = now_ms();
    unsigned told = ~0u, told_invert = ~0u;  // (neither yet: the state goes first)
    int w = 0, h = 0;
    for (;;) {
        unsigned state = atomic_load(&g_sh->state), invert = atomic_load(&g_sh->invert);
        if (state == LIVE_BYE) {
            send_state(fd, LIVE_BYE, invert);
            break;
        }
        if (state != told || invert != told_invert) {
            if (!send_state(fd, state, invert)) break;
            told = state;
            told_invert = invert;
            last_sent = now_ms();
        }
        uint32_t e = atomic_load(&g_sh->epoch);
        if (state == LIVE_AWAKE && e != epoch) {
            epoch = e;
            w = (int)atomic_load(&g_sh->w);
            h = (int)atomic_load(&g_sh->h);
            memset(sent, 255, cap);  // the browser starts from white: send every tile that is not
            for (int i = 0; i < LIVE_WORDS; i++) atomic_store(&dirty[i], 0xFFFFFFFFu);
            if (!send_hello(fd, w, h, epoch)) break;
        }
        bool any = false;
        if (state == LIVE_AWAKE) {
            uint32_t words[LIVE_WORDS];
            for (int i = 0; i < LIVE_WORDS; i++) {
                words[i] = atomic_exchange(&dirty[i], 0);
                any |= words[i] != 0;
            }
            if (any) {
                Rect runs[LIVE_WORDS * 32];
                int n = live_runs(words, w, h, runs, LIVE_WORDS * 32), out = 0;
                bool ok = true;
                for (int k = 0; k < n && ok; k++) {
                    Rect r = runs[k];
                    // A snapshot of the run, then the state again: a pairing card drawn meanwhile
                    // is never sent, and its tiles come again once it is gone (live_set_state).
                    bool changed = false;
                    for (int y = 0; y < r.h; y++) {
                        uint8_t *row = &tile[(size_t)y * (size_t)r.w];
                        memcpy(row, &g_sh->px[(size_t)(r.y + y) * (size_t)w + (size_t)r.x], (size_t)r.w);
                        changed |= memcmp(&sent[(size_t)(r.y + y) * (size_t)w + (size_t)r.x], row, (size_t)r.w) != 0;
                    }
                    atomic_thread_fence(memory_order_seq_cst);
                    if (atomic_load(&g_sh->state) != LIVE_AWAKE || atomic_load(&g_sh->epoch) != epoch) {
                        ok = false;  // (live_set_state(LIVE_AWAKE) and a turn mark every tile again)
                        break;
                    }
                    if (!changed) continue;
                    int len = 0;
                    unsigned char *png = stbi_write_png_to_mem(tile, r.w, r.w, r.h, 1, &len);
                    if (!png) continue;
                    uint8_t at[8];
                    put16(at, (unsigned)r.x);
                    put16(at + 2, (unsigned)r.y);
                    put16(at + 4, (unsigned)r.w);
                    put16(at + 6, (unsigned)r.h);
                    bool sent_ok = send_frame(fd, LIVE_TILE_FRAME, at, sizeof at, png, (size_t)len);
                    free(png);
                    if (!sent_ok) _exit(0);
                    for (int y = 0; y < r.h; y++)
                        memcpy(&sent[(size_t)(r.y + y) * (size_t)w + (size_t)r.x], &tile[(size_t)y * (size_t)r.w], (size_t)r.w);
                    out++;
                }
                if (out) {
                    uint8_t p[8];
                    put32(p, atomic_fetch_add(&g_sh->seq, 1));
                    put32(p + 4, now_ms());
                    if (!send_frame(fd, LIVE_BATCH, p, sizeof p, NULL, 0)) break;
                    last_batch = last_sent = now_ms();
                }
                if (!ok) continue;  // (the state moved on: told at the top)
            }
        }
        if (now_ms() - last_sent >= 15000) {
            if (!send_frame(fd, LIVE_PING, NULL, 0, NULL, 0)) break;
            last_sent = now_ms();
        }
        // Wait for a mark, the browser hanging up, or a second (the parent gone ends it here too).
        struct pollfd pf[2] = {{wake, POLLIN, 0}, {fd, POLLIN, 0}};
        int got = poll(pf, 2, 1000);
        if (got < 0 && errno != EINTR) break;
        if (pf[1].revents & (POLLHUP | POLLERR | POLLIN)) {
            char b;
            ssize_t r = recv(fd, &b, 1, MSG_DONTWAIT);
            if (r == 0 || (r < 0 && errno != EAGAIN && errno != EWOULDBLOCK && errno != EINTR)) break;
        }
        if (pf[0].revents & POLLIN) {
            char drain[64];
            while (read(wake, drain, sizeof drain) > 0) {
            }
        }
        if (getppid() != parent) break;
        // At most about 30 batches a second: what comes meanwhile goes in the next one.
        uint32_t since = now_ms() - last_batch;
        if (since < 33) sleep_ms((int)(33 - since));
    }
    shutdown(fd, SHUT_WR);
    _exit(0);
}

bool live_take_viewer(int fd) {
    if (!g_sh) return false;
    int slot = -1;
    for (int i = 0; i < LIVE_VIEWERS; i++)
        if (g_v[i].pid <= 0) slot = i;
    if (slot < 0) return false;
    int p[2];
    if (pipe(p) != 0) return false;
    fcntl(p[0], F_SETFL, fcntl(p[0], F_GETFL, 0) | O_NONBLOCK);
    fcntl(p[1], F_SETFL, fcntl(p[1], F_GETFL, 0) | O_NONBLOCK);
    pid_t pid = fork();
    if (pid == 0) {
        // The main loop's state stays as it is for the viewer: it reads the frame and its own
        // marks, and the other slot's pipe end closes with every other fd.
        atomic_thread_fence(memory_order_seq_cst);
        for (int i = 0; i < LIVE_WORDS; i++) atomic_store(&g_sh->dirty[slot][i], 0xFFFFFFFFu);
        viewer_main(slot, fd, p[0]);
    }
    close(p[0]);
    if (pid < 0) {
        close(p[1]);
        return false;
    }
    g_v[slot].pid = pid;
    g_v[slot].wake = p[1];
#ifdef GRIMOIRE_SIM
    qlog("live: (simulator) viewer pid %d", (int)pid);  // (tests/live_check.sh: none outlives the app)
#endif
    return true;
}
