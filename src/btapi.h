// btapi.h: the Bluetooth helper's answers (kindle-hid-passthrough's local API, bluetooth.h), read
// into fixed records: /status, /scan-status and /pair-status. Pure, for tests/test_btapi.c. What
// comes from the helper (names come over the radio) is kept only as it is safe to show and to put
// back in a request: names printable ASCII, addresses six hex pairs.
#pragma once
#include <stdbool.h>
#include <stddef.h>

#define BT_NAME_LEN 41
#define BT_ADDR_LEN 18  // "AA:BB:CC:DD:EE:FF" and its end
#define BT_DEVICES 8

typedef struct {
    char address[BT_ADDR_LEN];
    char name[BT_NAME_LEN];
    bool classic;     // protocol "classic" (else BLE)
    bool connected;   // in /status: a live session with its input node made (hid_ready)
} BtDevice;

typedef struct {
    bool ok;               // the helper said ok
    char error[80];        // its error, when not ok (printable ASCII)
    bool running, scanning, pairing;
    char version[24];
    BtDevice dev[BT_DEVICES];  // /status: the paired ones (connected marked); /scan-status: found
    int ndev;
} BtReply;

// A reply's JSON (len bytes): 0, or -1 when it is not one the helper sends (then *out is empty).
int btapi_parse(const char *json, size_t len, BtReply *out);
// True for "AA:BB:CC:DD:EE:FF" (hex digits, either case).
bool btapi_address_ok(const char *addr);
