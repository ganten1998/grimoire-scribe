// test_latency.rs: the ink-metrics latency report's histogram, and matching the pen service's relayed
// samples to the raw digitizer's (the hop's timing).
// (the C tests index by position, and so do these ports of them)
#![allow(clippy::needless_range_loop, clippy::too_many_arguments)]
mod testlib;
use grimoire::prelude::*;
use testlib::*;

fn percentiles_are_the_upper_edges_of_their_bins() {
    let mut h = LatHist::default();
    lat_hist_init(&mut h, 100);
    check!(lat_hist_pct(&h, 50) == 0);
    for i in 1..=100 {
        lat_hist_add(&mut h, i * 100 - 40); // 60, 160, ..., 9960 us
    }
    check!(h.n == 100);
    check!(lat_hist_pct(&h, 50) == 5000);
    check!(lat_hist_pct(&h, 90) == 9000);
    check!(lat_hist_pct(&h, 100) == 9960); // capped at the largest sample, not its bin's edge
    check!(lat_hist_pct(&h, 1) == 100);
    check!(h.max_us == 9960);
    let mut s = String::new();
    lat_hist_format(&h, &mut s, 96);
    check!(s == "p50 5.0 p90 9.0 max 10.0 ms (n 100)");
}

fn out_of_range_and_negative_durations() {
    let mut h = LatHist::default();
    lat_hist_init(&mut h, 1000);
    lat_hist_add(&mut h, -5); // clocks a hair apart: counts as no delay
    lat_hist_add(&mut h, 10 * 1000 * 1000); // past the 256 ms range: the last bin, still the max
    check!(h.bins[0] == 1);
    check!(h.bins[LAT_BINS - 1] == 1);
    check!(lat_hist_pct(&h, 50) == 1000);
    check!(lat_hist_pct(&h, 90) == 10 * 1000 * 1000);
    let mut s = String::new();
    let mut empty = LatHist::default();
    lat_hist_init(&mut empty, 1000);
    lat_hist_format(&empty, &mut s, 96);
    check!(s == "none");
}

// The Scribe 2's two pens: the raw digitizer 0..15624 x 0..20832, the relayed one 0..1860 x 0..2480.
const RMIN: [i32; 2] = [0, 0];
const RMAX: [i32; 2] = [15624, 20832];
const SMIN: [i32; 2] = [0, 0];
const SMAX: [i32; 2] = [1860, 2480];

fn relay(rx: i32, ry: i32, o: i32) -> (i32, i32) {
    let mut fx = rx as f32 / 15624.0;
    let mut fy = ry as f32 / 20832.0;
    if o & 1 != 0 {
        fx = 1.0 - fx;
    }
    if o & 2 != 0 {
        fy = 1.0 - fy;
    }
    // (`fx * 1860.f + .5f` is a fused multiply-add in the C test, built with clang's default
    // -ffp-contract=on)
    (fx.mul_add(1860.0, 0.5) as i32, fy.mul_add(2480.0, 0.5) as i32)
}

fn a_relayed_stroke_matches_its_raw_samples_in_any_orientation() {
    for o in 0..4 {
        let mut m = LatMatch::default();
        lat_match_init(&mut m, &RMIN, &RMAX, &SMIN, &SMAX);
        let mut matched = 0;
        let mut right = 0;
        // Raw samples every 4 ms along a diagonal; each relayed 3 samples (12 ms) later.
        for k in 0..400i32 {
            let rx = 2000 + k * 23;
            let ry = 3000 + k * 31;
            lat_match_raw(&mut m, 1000000u64 + k as u64 * 4000u64, rx, ry);
            if k < 3 {
                continue;
            }
            let (sx, sy) = relay(2000 + (k - 3) * 23, 3000 + (k - 3) * 31, o);
            let t = lat_match_pen(&mut m, sx, sy, 3);
            if t != 0 {
                matched += 1;
            }
            if t == 1000000u64 + (k - 3) as u64 * 4000u64 {
                right += 1;
            }
        }
        // Until the orientation is learned a few early ones may miss; then every one is exact.
        check!(matched >= 390);
        check!(right >= 390);
    }
}

fn a_sample_from_nowhere_matches_nothing() {
    let mut m = LatMatch::default();
    lat_match_init(&mut m, &RMIN, &RMAX, &SMIN, &SMAX);
    check!(lat_match_pen(&mut m, 100, 100, 3) == 0); // no raw samples yet
    for k in 0..10u64 {
        lat_match_raw(&mut m, k * 4000, 8000, 9000);
    }
    check!(lat_match_pen(&mut m, 952, 1072, 3) == 9 * 4000u64); // a pen held still: the newest of the equals
    check!(lat_match_pen(&mut m, 100, 100, 3) == 0); // far from every raw sample
}

fn the_ring_forgets_old_samples() {
    let mut m = LatMatch::default();
    lat_match_init(&mut m, &RMIN, &RMAX, &SMIN, &SMAX);
    lat_match_raw(&mut m, 5, 1000, 1000);
    for k in 0..LAT_RING as u64 {
        lat_match_raw(&mut m, 100 + k, 9000, 9000);
    }
    check!(m.count == LAT_RING as i32);
    check!(lat_match_pen(&mut m, 119, 119, 3) == 0); // the sample at (1000, 1000) was pushed out
}

#[test]
fn test_latency() {
    run!(percentiles_are_the_upper_edges_of_their_bins);
    run!(out_of_range_and_negative_durations);
    run!(a_relayed_stroke_matches_its_raw_samples_in_any_orientation);
    run!(a_sample_from_nowhere_matches_nothing);
    run!(the_ring_forgets_old_samples);
    done();
}
