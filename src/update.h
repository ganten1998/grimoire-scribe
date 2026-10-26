// update.h: Settings > Updates (docs/KPM.md, Updates). Grimoire's updates come through KPM, the
// Kindle's package manager, from Grimoire's own feed: kindle/app/update.sh adds the chosen channel's
// feed to KPM (and takes the other channel's away: KPM installs the newest Grimoire any feed it knows
// lists), refreshes KPM's index and lists what it offers. KPM does the downloading (Grimoire has no
// HTTPS of its own). An update itself cannot run inside Grimoire (KPM's hooks refuse while it runs):
// Grimoire writes the request and leaves, and the launcher hands it to update.sh, which installs the
// newest Grimoire with KPM once the launcher is gone, and starts it again.
#pragma once
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#define UPDATE_FEED_PREVIEW "https://raw.githubusercontent.com/ganten1998/grimoire-scribe/kpm/preview/manifest.json"
#define UPDATE_FEED_STABLE "https://raw.githubusercontent.com/ganten1998/grimoire-scribe/kpm/stable/manifest.json"
#define UPDATE_VERSION_LEN 24

// ---- pure parts (tests/test_update.c) ----

// A Grimoire version as KPM orders it (tools/kpm-package.py): "2.0.0-beta.5" is 2.0.1005,
// "2.0.0-rc.1" 2.0.5001, "2.0.0" 2.0.9999, "2.0.1" 2.0.19999. False for anything else.
bool update_version_key(const char *version, uint32_t key[3]);
// <0, 0, >0 as version a is older than, the same as, or newer than b; unreadable ones are oldest.
int update_version_cmp(const char *a, const char *b);
// A prerelease (beta or rc): its updates come from the preview channel unless told otherwise.
bool update_is_prerelease(const char *version);
// The newest Grimoire in `kpm search grimoire`'s output (each package a line "- grimoire (Grimoire):
// <description>", the description ending in its version; colour codes and other packages skipped).
// False when it lists none.
bool update_latest(const char *search_output, char version[UPDATE_VERSION_LEN]);

// ---- on the Kindle ----

// script: kindle/app/update.sh (the launcher says where), or NULL: no updates here. request: the file
// "Update now" writes for the launcher (it says where too; without a launcher, the simulator's, the
// caller passes one in the data folder).
void update_init(const char *script, const char *request);
const char *update_request_path(void);
bool update_available(void);
// Checks the channel (preview or stable) in a forked child; false while one is under way.
bool update_check(bool preview);
bool update_checking(void);
typedef enum { UPDATE_EV_NONE = 0, UPDATE_EV_DONE, UPDATE_EV_FAILED } UpdateEvent;
// Reaps the check: UPDATE_EV_DONE with the newest version it offers in *latest ("" when it offers
// none), UPDATE_EV_FAILED when the check did not work (no network, no KPM: *why says which).
UpdateEvent update_tick(char latest[UPDATE_VERSION_LEN], char *why, size_t why_cap);
// Writes the request ("preview" or "stable") for the launcher. 0, or -1 when it could not.
int update_request(bool preview);
