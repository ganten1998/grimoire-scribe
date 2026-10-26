// latency.h: the pure parts of the ink-metrics latency report (unit-tested on the host), and the
// hook between the two Kindle platform files that feed it.
//
// With /mnt/us/grimoire/ink-metrics present, every live ink frame is timed from the pen to the
// panel, and the spread is logged every 10 s (display_fbink.c):
//   raw     the digitizer's own sample (WacomDigitizer), stamped by the kernel
//   pen     the same sample relayed by the Kindle's pen service (stylus-custom), which Grimoire reads
//   submit  Grimoire's refresh ioctl returned
//   start   the driver took the update to the panel (MXCFB_WAIT_FOR_UPDATE_SUBMISSION)
//   done    its waveform ended (MXCFB_WAIT_FOR_UPDATE_COMPLETE)
// All in CLOCK_MONOTONIC: the input devices are switched to it with EVIOCSCLOCKID.
#pragma once
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

// Durations in bins of unit_us; the last bin holds everything past the range.
#define LAT_BINS 256
typedef struct {
    uint32_t unit_us;
    uint32_t n, max_us;
    uint64_t sum_us;
    uint32_t bins[LAT_BINS];
} LatHist;

void lat_hist_init(LatHist *h, uint32_t unit_us);
void lat_hist_add(LatHist *h, int64_t us);  // a negative duration counts as 0
// The duration under which pct percent of the samples fall (the upper edge of their bin; the
// largest sample for the last bin), in us. 0 with no samples.
uint32_t lat_hist_pct(const LatHist *h, int pct);
// "p50 1.2 p90 2.0 max 7.9 ms (n 1250)", or "none".
void lat_hist_format(const LatHist *h, char *out, size_t cap);

// The pen service relays each raw digitizer sample as a scaled one. To time that hop, the recent
// raw samples are kept, and a relayed sample is matched to the raw one at the same place: the
// relayed axes are the raw ones scaled, possibly reversed (an orientation is learned from the
// samples, the one whose nearest raw samples have been closest so far).
#define LAT_RING 32
typedef struct {
    uint64_t t_us;
    int x, y;
} LatRaw;
typedef struct {
    LatRaw ring[LAT_RING];
    int next, count;
    int rmin[2], rmax[2];  // the raw digitizer's x and y ranges
    int smin[2], smax[2];  // the relayed pen's
    uint64_t err[4];       // per orientation (bit 0: x reversed, bit 1: y reversed): summed best errors, capped
} LatMatch;

void lat_match_init(LatMatch *m, const int rmin[2], const int rmax[2], const int smin[2], const int smax[2]);
void lat_match_raw(LatMatch *m, uint64_t t_us, int x, int y);
// The kernel time of the raw sample a relayed one at (x, y) came from: of the recent raw samples, the
// nearest (the newest of equals) under the best orientation so far, if within tol px. 0 when none.
uint64_t lat_match_pen(LatMatch *m, int x, int y, int tol);

// Kindle only (input_evdev.c, read by display_fbink.c for each live ink frame): the kernel times of
// the newest and the oldest pen sample emitted since the last call, and of the raw sample the newest
// came from (0 when it matched none). False when no pen sample came since the last call, or the pen's
// events are not in CLOCK_MONOTONIC.
bool input_ink_times(uint64_t *newest, uint64_t *oldest, uint64_t *raw);
