// bluetooth.h: the Bluetooth helper (kindle-hid-passthrough; docs/SHORTCUTS.md, Bluetooth) as Grimoire
// runs it. With Settings > Controller's Bluetooth on, Grimoire starts the helper after its first
// frame (kindle/app/bluetooth.sh start: only one of its own, never someone else's) and stops it when
// that is turned off; the launcher's restore stops it as Grimoire leaves, crash or not, and gives
// the Kindle its own Bluetooth jobs back. Pairing goes through the helper's local API
// (127.0.0.1:8321). Every script run and request is a forked child: bt_tick reaps them, so the UI
// never waits. Names and addresses from the helper are made safe first (btapi.h).
#pragma once
#include <stdbool.h>
#include <stdint.h>

#include "btapi.h"

typedef enum {
    BT_STOPPED = 0,  // not started by Grimoire (or stopped)
    BT_STARTING,
    BT_OWNED,        // started by Grimoire, answering
    BT_EXTERNAL,     // another one runs (KOReader's, say): Grimoire uses it, never stops it
    BT_FAILED,       // it would not start (bluetooth.sh's log says why)
    BT_STOPPING,
} BtState;

typedef enum { BT_EV_NONE = 0, BT_EV_STATE, BT_EV_REPLY, BT_EV_FAILED } BtEvent;

// root: Grimoire's data folder. script: kindle/app/bluetooth.sh (the launcher says where), or NULL.
// standin: "host:port" of a stand-in helper that runs already (tests/fake_bthelper.py), or NULL.
void bt_init(const char *root, const char *script, const char *standin);
// A helper Grimoire could start: unpacked in <root>/bluetooth/, or KHP's /mnt/us/kindle_hid_passthrough.
bool bt_present(void);
BtState bt_state(void);
void bt_start(void);
void bt_stop(void);
// A GET of path (with its query) to the helper's API: false while one is under way, or with no API.
bool bt_request(const char *path);
bool bt_busy(void);
// Reaps what finished: BT_EV_STATE (bt_state changed), BT_EV_REPLY (*reply: the request's answer)
// or BT_EV_FAILED (the request got none: the helper is gone or did not answer in 3 s).
BtEvent bt_tick(BtReply *reply);
