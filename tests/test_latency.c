// test_latency.c: the ink-metrics latency report's histogram, and matching the pen service's relayed
// samples to the raw digitizer's (the hop's timing).
#include "latency.h"
#include "testlib.h"

TEST(percentiles_are_the_upper_edges_of_their_bins) {
    LatHist h;
    lat_hist_init(&h, 100);
    CHECK(lat_hist_pct(&h, 50) == 0);
    for (int i = 1; i <= 100; i++) lat_hist_add(&h, i * 100 - 40);  // 60, 160, ..., 9960 us
    CHECK(h.n == 100);
    CHECK(lat_hist_pct(&h, 50) == 5000);
    CHECK(lat_hist_pct(&h, 90) == 9000);
    CHECK(lat_hist_pct(&h, 100) == 9960);  // capped at the largest sample, not its bin's edge
    CHECK(lat_hist_pct(&h, 1) == 100);
    CHECK(h.max_us == 9960);
    char s[96];
    lat_hist_format(&h, s, sizeof s);
    CHECK(!strcmp(s, "p50 5.0 p90 9.0 max 10.0 ms (n 100)"));
}

TEST(out_of_range_and_negative_durations) {
    LatHist h;
    lat_hist_init(&h, 1000);
    lat_hist_add(&h, -5);  // clocks a hair apart: counts as no delay
    lat_hist_add(&h, 10 * 1000 * 1000);  // past the 256 ms range: the last bin, still the max
    CHECK(h.bins[0] == 1);
    CHECK(h.bins[LAT_BINS - 1] == 1);
    CHECK(lat_hist_pct(&h, 50) == 1000);
    CHECK(lat_hist_pct(&h, 90) == 10 * 1000 * 1000);
    char s[96];
    LatHist empty;
    lat_hist_init(&empty, 1000);
    lat_hist_format(&empty, s, sizeof s);
    CHECK(!strcmp(s, "none"));
}

// The Scribe 2's two pens: the raw digitizer 0..15624 x 0..20832, the relayed one 0..1860 x 0..2480.
static const int RMIN[2] = {0, 0}, RMAX[2] = {15624, 20832}, SMIN[2] = {0, 0}, SMAX[2] = {1860, 2480};

static void relay(int rx, int ry, int o, int *sx, int *sy) {
    float fx = rx / 15624.f, fy = ry / 20832.f;
    if (o & 1) fx = 1.f - fx;
    if (o & 2) fy = 1.f - fy;
    *sx = (int)(fx * 1860.f + .5f);
    *sy = (int)(fy * 2480.f + .5f);
}

TEST(a_relayed_stroke_matches_its_raw_samples_in_any_orientation) {
    for (int o = 0; o < 4; o++) {
        LatMatch m;
        lat_match_init(&m, RMIN, RMAX, SMIN, SMAX);
        int matched = 0, right = 0;
        // Raw samples every 4 ms along a diagonal; each relayed 3 samples (12 ms) later.
        for (int k = 0; k < 400; k++) {
            int rx = 2000 + k * 23, ry = 3000 + k * 31;
            lat_match_raw(&m, 1000000u + (uint64_t)k * 4000u, rx, ry);
            if (k < 3) continue;
            int sx, sy;
            relay(2000 + (k - 3) * 23, 3000 + (k - 3) * 31, o, &sx, &sy);
            uint64_t t = lat_match_pen(&m, sx, sy, 3);
            if (t) matched++;
            if (t == 1000000u + (uint64_t)(k - 3) * 4000u) right++;
        }
        // Until the orientation is learned a few early ones may miss; then every one is exact.
        CHECK(matched >= 390);
        CHECK(right >= 390);
    }
}

TEST(a_sample_from_nowhere_matches_nothing) {
    LatMatch m;
    lat_match_init(&m, RMIN, RMAX, SMIN, SMAX);
    CHECK(lat_match_pen(&m, 100, 100, 3) == 0);  // no raw samples yet
    for (int k = 0; k < 10; k++) lat_match_raw(&m, (uint64_t)k * 4000u, 8000, 9000);
    CHECK(lat_match_pen(&m, 952, 1072, 3) == 9 * 4000u);  // a pen held still: the newest of the equals
    CHECK(lat_match_pen(&m, 100, 100, 3) == 0);           // far from every raw sample
}

TEST(the_ring_forgets_old_samples) {
    LatMatch m;
    lat_match_init(&m, RMIN, RMAX, SMIN, SMAX);
    lat_match_raw(&m, 5, 1000, 1000);
    for (int k = 0; k < LAT_RING; k++) lat_match_raw(&m, 100u + (uint64_t)k, 9000, 9000);
    CHECK(m.count == LAT_RING);
    CHECK(lat_match_pen(&m, 119, 119, 3) == 0);  // the sample at (1000, 1000) was pushed out
}

int main(void) {
    RUN(percentiles_are_the_upper_edges_of_their_bins);
    RUN(out_of_range_and_negative_durations);
    RUN(a_relayed_stroke_matches_its_raw_samples_in_any_orientation);
    RUN(a_sample_from_nowhere_matches_nothing);
    RUN(the_ring_forgets_old_samples);
    return tl_done();
}
