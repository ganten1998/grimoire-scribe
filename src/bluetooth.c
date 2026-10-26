// bluetooth.c: see bluetooth.h.
#include "bluetooth.h"

#include <arpa/inet.h>
#include <errno.h>
#include <fcntl.h>
#include <netinet/in.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <sys/stat.h>
#include <sys/time.h>
#include <sys/wait.h>
#include <unistd.h>

#include "common.h"

#define REPLY_MAX (64 * 1024)

static char g_script[512], g_root[512];
static char g_host[64] = "127.0.0.1";
static int g_port = 8321;
static bool g_api;            // there is an API to ask (the launcher's script, or a test's stand-in)
static bool g_standin;        // the API is a stand-in that runs already (tests): nothing to start or stop
static BtState g_state;
// The script run and the request, each a child writing to a pipe.
static struct {
    pid_t pid;
    int fd;
    char buf[REPLY_MAX + 1];
    size_t len;
} g_job[2];
enum { JOB_SCRIPT, JOB_REQUEST };

void bt_init(const char *root, const char *script, const char *standin) {
    snprintf(g_root, sizeof g_root, "%s", root ? root : "");
    snprintf(g_script, sizeof g_script, "%s", script ? script : "");
    snprintf(g_host, sizeof g_host, "127.0.0.1");
    g_port = 8321;
    g_api = g_script[0] != 0;
    g_standin = false;
    g_state = BT_STOPPED;
    const char *colon = standin ? strrchr(standin, ':') : NULL;
    if (colon && colon > standin && (size_t)(colon - standin) < sizeof g_host) {
        memcpy(g_host, standin, (size_t)(colon - standin));
        g_host[colon - standin] = 0;
        g_port = atoi(colon + 1);
        g_api = g_standin = true;
        g_script[0] = 0;
    }
    for (int i = 0; i < 2; i++) {
        if (g_job[i].pid <= 0) g_job[i].fd = -1;
    }
}

static bool executable(const char *path) {
    struct stat st;
    return stat(path, &st) == 0 && S_ISREG(st.st_mode) && (st.st_mode & 0111);
}

bool bt_present(void) {
    if (g_standin) return true;
    if (!g_script[0]) return false;
    char path[700];
    snprintf(path, sizeof path, "%s/bluetooth/kindle-hid-passthrough", g_root);
    return executable(path) || executable("/mnt/us/kindle_hid_passthrough/kindle-hid-passthrough");
}

BtState bt_state(void) { return g_state; }
bool bt_busy(void) { return g_job[JOB_REQUEST].pid > 0; }

// A child for job j: its stdout (and the parent's end) a pipe. Returns the child's pid in the
// child (0) and the parent, -1 when it could not fork.
static pid_t spawn(int j) {
    int p[2];
    if (pipe(p) != 0) return -1;
    pid_t pid = fork();
    if (pid < 0) {
        close(p[0]);
        close(p[1]);
        return -1;
    }
    if (pid == 0) {
        dup2(p[1], 1);
        for (int fd = 3; fd < 1024; fd++) close(fd);
        struct sigaction sa;
        memset(&sa, 0, sizeof sa);
        sa.sa_handler = SIG_DFL;
        sigaction(SIGTERM, &sa, NULL);
        sigaction(SIGINT, &sa, NULL);
        sigaction(SIGHUP, &sa, NULL);
        sigaction(SIGPIPE, &sa, NULL);
        return 0;
    }
    close(p[1]);
    fcntl(p[0], F_SETFL, fcntl(p[0], F_GETFL, 0) | O_NONBLOCK);
    g_job[j].pid = pid;
    g_job[j].fd = p[0];
    g_job[j].len = 0;
    return pid;
}

static void run_script(const char *verb, BtState during) {
    if (!g_script[0] || g_job[JOB_SCRIPT].pid > 0) return;
    pid_t pid = spawn(JOB_SCRIPT);
    if (pid == 0) {
        char root[600];
        snprintf(root, sizeof root, "GRIMOIRE_ROOT=%s", g_root);
        putenv(root);
        execl("/bin/sh", "sh", g_script, verb, (char *)NULL);
        _exit(127);
    }
    if (pid > 0) g_state = during;
    else qlog("bluetooth: could not run the helper's script (%s)", strerror(errno));
}

void bt_start(void) {
    if (g_standin) {  // it runs already: nothing to start
        g_state = BT_EXTERNAL;
        return;
    }
    if (g_state == BT_OWNED || g_state == BT_EXTERNAL || g_state == BT_STARTING) return;
    qlog("bluetooth: starting the helper");
    run_script("start", BT_STARTING);
}

void bt_stop(void) {
    if (g_state == BT_EXTERNAL || g_standin) {
        g_state = BT_STOPPED;  // (someone else's: left running)
        return;
    }
    if (g_state == BT_STOPPED || g_state == BT_STOPPING) return;
    qlog("bluetooth: stopping the helper");
    run_script("stop-owned", BT_STOPPING);
}

// The child of a request: a GET over loopback, 3 s at most, its body to stdout.
static void request_child(const char *path) {
    int fd = socket(AF_INET, SOCK_STREAM, 0);
    if (fd < 0) _exit(1);
    struct timeval tv = {3, 0};
    setsockopt(fd, SOL_SOCKET, SO_RCVTIMEO, &tv, sizeof tv);
    setsockopt(fd, SOL_SOCKET, SO_SNDTIMEO, &tv, sizeof tv);
    struct sockaddr_in sa = {0};
    sa.sin_family = AF_INET;
    sa.sin_port = htons((uint16_t)g_port);
    if (inet_pton(AF_INET, g_host, &sa.sin_addr) != 1 || connect(fd, (struct sockaddr *)&sa, sizeof sa) != 0) _exit(1);
    char req[700];
    int n = snprintf(req, sizeof req, "GET %s HTTP/1.0\r\nHost: %s\r\nConnection: close\r\n\r\n", path, g_host);
    if (n < 0 || (size_t)n >= sizeof req || send(fd, req, (size_t)n, 0) != n) _exit(1);
    static char buf[REPLY_MAX + 4096];
    size_t len = 0;
    for (;;) {
        ssize_t r = recv(fd, buf + len, sizeof buf - len, 0);
        if (r < 0 && errno == EINTR) continue;
        if (r <= 0) break;
        len += (size_t)r;
        if (len == sizeof buf) _exit(1);  // more than any answer it gives
    }
    char *body = memmem(buf, len, "\r\n\r\n", 4);
    if (!body || strncmp(buf, "HTTP/1.", 7) || !strstr(buf, " 200 ")) _exit(1);
    body += 4;
    size_t blen = len - (size_t)(body - buf);
    if (blen > REPLY_MAX || write(1, body, blen) != (ssize_t)blen) _exit(1);
    _exit(0);
}

bool bt_request(const char *path) {
    if (!g_api || g_job[JOB_REQUEST].pid > 0 || !path || path[0] != '/' || strlen(path) > 400) return false;
    for (const char *p = path; *p; p++)
        if (*p <= ' ' || *p > '~') return false;  // (one line, nothing to escape)
    pid_t pid = spawn(JOB_REQUEST);
    if (pid == 0) request_child(path);
    return pid > 0;
}

// Reads what job j has written; true once it has ended (its status in *status).
static bool finished(int j, int *status) {
    if (g_job[j].pid <= 0) return false;
    for (;;) {
        size_t room = REPLY_MAX - g_job[j].len;
        if (!room) break;
        ssize_t r = read(g_job[j].fd, g_job[j].buf + g_job[j].len, room);
        if (r > 0) g_job[j].len += (size_t)r;
        else break;
    }
    if (waitpid(g_job[j].pid, status, WNOHANG) != g_job[j].pid) return false;
    for (ssize_t r; (r = read(g_job[j].fd, g_job[j].buf + g_job[j].len, REPLY_MAX - g_job[j].len)) > 0;)
        g_job[j].len += (size_t)r;
    g_job[j].buf[g_job[j].len] = 0;
    close(g_job[j].fd);
    g_job[j].fd = -1;
    g_job[j].pid = 0;
    return true;
}

BtEvent bt_tick(BtReply *reply) {
    int status;
    if (finished(JOB_SCRIPT, &status)) {
        // The script's last word: owned, external, stopped, missing or failed.
        const char *out = g_job[JOB_SCRIPT].buf;
        BtState was = g_state;
        g_state = strstr(out, "owned") ? BT_OWNED : strstr(out, "external") ? BT_EXTERNAL
                : strstr(out, "stopped") ? BT_STOPPED : BT_FAILED;
        if (was == BT_STOPPING && g_state == BT_FAILED) g_state = BT_STOPPED;
        qlog("bluetooth: the helper is %s", g_state == BT_OWNED ? "running (Grimoire's)" : g_state == BT_EXTERNAL
                                             ? "running (not Grimoire's)" : g_state == BT_STOPPED ? "stopped" : "not running");
        return BT_EV_STATE;
    }
    if (finished(JOB_REQUEST, &status)) {
        bool ok = WIFEXITED(status) && WEXITSTATUS(status) == 0;
        if (ok && btapi_parse(g_job[JOB_REQUEST].buf, g_job[JOB_REQUEST].len, reply) == 0) return BT_EV_REPLY;
        return BT_EV_FAILED;
    }
    return BT_EV_NONE;
}
