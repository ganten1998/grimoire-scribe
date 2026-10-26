// latency.c: see latency.h.
#include "latency.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

void lat_hist_init(LatHist *h, uint32_t unit_us) {
    memset(h, 0, sizeof *h);
    h->unit_us = unit_us ? unit_us : 1;
}

void lat_hist_add(LatHist *h, int64_t us) {
    if (us < 0) us = 0;
    if (us > UINT32_MAX) us = UINT32_MAX;
    uint64_t bin = (uint64_t)us / h->unit_us;
    h->bins[bin < LAT_BINS ? bin : LAT_BINS - 1]++;
    h->n++;
    h->sum_us += (uint64_t)us;
    if ((uint32_t)us > h->max_us) h->max_us = (uint32_t)us;
}

uint32_t lat_hist_pct(const LatHist *h, int pct) {
    if (!h->n) return 0;
    uint64_t want = ((uint64_t)h->n * (uint64_t)pct + 99) / 100;  // the rank of the sample, from 1
    if (want < 1) want = 1;
    uint64_t seen = 0;
    for (int i = 0; i < LAT_BINS; i++) {
        seen += h->bins[i];
        if (seen < want) continue;
        if (i == LAT_BINS - 1) return h->max_us;
        uint32_t edge = (uint32_t)(i + 1) * h->unit_us;
        return edge < h->max_us ? edge : h->max_us;
    }
    return h->max_us;
}

void lat_hist_format(const LatHist *h, char *out, size_t cap) {
    if (!h->n) {
        snprintf(out, cap, "none");
        return;
    }
    snprintf(out, cap, "p50 %.1f p90 %.1f max %.1f ms (n %u)", lat_hist_pct(h, 50) / 1000.0,
             lat_hist_pct(h, 90) / 1000.0, h->max_us / 1000.0, h->n);
}

void lat_match_init(LatMatch *m, const int rmin[2], const int rmax[2], const int smin[2], const int smax[2]) {
    memset(m, 0, sizeof *m);
    for (int a = 0; a < 2; a++) {
        m->rmin[a] = rmin[a];
        m->rmax[a] = rmax[a];
        m->smin[a] = smin[a];
        m->smax[a] = smax[a];
    }
}

void lat_match_raw(LatMatch *m, uint64_t t_us, int x, int y) {
    m->ring[m->next] = (LatRaw){t_us, x, y};
    m->next = (m->next + 1) % LAT_RING;
    if (m->count < LAT_RING) m->count++;
}

// A raw coordinate on axis a, in the relayed pen's units, reversed or not.
static float scaled(const LatMatch *m, int a, int v, bool reversed) {
    int span = m->rmax[a] - m->rmin[a];
    if (span <= 0) return 0.f;
    float f = (float)(v - m->rmin[a]) / (float)span;
    if (reversed) f = 1.f - f;
    return (float)m->smin[a] + f * (float)(m->smax[a] - m->smin[a]);
}

uint64_t lat_match_pen(LatMatch *m, int x, int y, int tol) {
    if (!m->count) return 0;
    float best[4];
    int at[4];
    for (int o = 0; o < 4; o++) {
        best[o] = 1e9f;
        at[o] = -1;
        // Newest first, so the newest of equally near samples wins (a pen held still).
        for (int k = 1; k <= m->count; k++) {
            int i = (m->next - k + LAT_RING) % LAT_RING;
            float dx = scaled(m, 0, m->ring[i].x, o & 1) - (float)x;
            float dy = scaled(m, 1, m->ring[i].y, (o & 2) != 0) - (float)y;
            float e = (dx < 0 ? -dx : dx) + (dy < 0 ? -dy : dy);
            if (e < best[o]) {
                best[o] = e;
                at[o] = i;
            }
        }
        m->err[o] += (uint64_t)(best[o] < 64.f ? best[o] : 64.f);
    }
    int pick = 0;
    for (int o = 1; o < 4; o++)
        if (m->err[o] < m->err[pick]) pick = o;
    if (at[pick] < 0 || best[pick] > (float)tol) return 0;
    return m->ring[at[pick]].t_us;
}
