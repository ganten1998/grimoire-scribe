// btapi.c: see btapi.h.
#include "btapi.h"

#include <string.h>

#include "json_min.h"

#define TOKENS 512

bool btapi_address_ok(const char *a) {
    if (!a || strlen(a) != 17) return false;
    for (int i = 0; i < 17; i++) {
        char c = a[i];
        bool hex = (c >= '0' && c <= '9') || (c >= 'a' && c <= 'f') || (c >= 'A' && c <= 'F');
        if (i % 3 == 2 ? c != ':' : !hex) return false;
    }
    return true;
}

// A string field, made printable ASCII (a run of anything else one '?'), trimmed; "" for none.
static void text_of(const char *json, const JsonTok *t, int ntok, int obj, const char *key, char *out, size_t cap) {
    char raw[256];
    out[0] = 0;
    int v = json_get(json, t, ntok, obj, key);
    if (v < 0 || !json_string(json, &t[v], raw, sizeof raw)) return;
    size_t o = 0;
    bool other = false;
    for (const unsigned char *p = (const unsigned char *)raw; *p && o + 1 < cap; p++) {
        if (*p >= 0x20 && *p < 0x7F) {
            out[o++] = (char)*p;
            other = false;
        } else if (!other) {
            out[o++] = '?';
            other = true;
        }
    }
    while (o && out[o - 1] == ' ') o--;
    out[o] = 0;
    size_t lead = strspn(out, " ");
    if (lead) memmove(out, out + lead, o - lead + 1);
}

static bool flag(const char *json, const JsonTok *t, int ntok, int obj, const char *key) {
    bool v = false;
    int at = json_get(json, t, ntok, obj, key);
    return at >= 0 && json_bool(&t[at], &v) && v;
}

// The devices of array key: those with a sound address, at most BT_DEVICES.
static void devices(const char *json, const JsonTok *t, int ntok, int obj, const char *key, BtReply *out) {
    int arr = json_get(json, t, ntok, obj, key);
    for (int i = 0; arr >= 0 && out->ndev < BT_DEVICES; i++) {
        int d = json_item(t, ntok, arr, i);
        if (d < 0) break;
        if (t[d].type != JS_OBJECT) continue;
        BtDevice dv = {0};
        text_of(json, t, ntok, d, "address", dv.address, sizeof dv.address);
        if (!btapi_address_ok(dv.address)) continue;
        text_of(json, t, ntok, d, "name", dv.name, sizeof dv.name);
        char proto[16];
        text_of(json, t, ntok, d, "protocol", proto, sizeof proto);
        dv.classic = !strcmp(proto, "classic");
        out->dev[out->ndev++] = dv;
    }
}

int btapi_parse(const char *json, size_t len, BtReply *out) {
    static JsonTok t[TOKENS];  // (the UI thread's alone)
    memset(out, 0, sizeof *out);
    int n = json_parse(json, len, t, TOKENS);
    if (n <= 0 || t[0].type != JS_OBJECT) return -1;
    out->ok = flag(json, t, n, 0, "ok");
    text_of(json, t, n, 0, "error", out->error, sizeof out->error);
    text_of(json, t, n, 0, "version", out->version, sizeof out->version);
    out->running = flag(json, t, n, 0, "daemon_running");
    out->scanning = flag(json, t, n, 0, "scanning");
    out->pairing = flag(json, t, n, 0, "pairing");
    devices(json, t, n, 0, "devices", out);
    // Live sessions mark the paired devices they are (and add one not listed, as connected).
    int conns = json_get(json, t, n, 0, "connections");
    for (int i = 0; conns >= 0; i++) {
        int c = json_item(t, n, conns, i);
        if (c < 0) break;
        char addr[BT_ADDR_LEN];
        text_of(json, t, n, c, "address", addr, sizeof addr);
        if (!btapi_address_ok(addr) || !flag(json, t, n, c, "hid_ready")) continue;
        int k = 0;
        while (k < out->ndev && strcasecmp(out->dev[k].address, addr)) k++;
        if (k == out->ndev) {
            if (k >= BT_DEVICES) continue;
            BtDevice dv = {0};
            memcpy(dv.address, addr, sizeof addr);
            text_of(json, t, n, c, "name", dv.name, sizeof dv.name);
            out->dev[out->ndev++] = dv;
        }
        out->dev[k].connected = true;
    }
    return 0;
}
