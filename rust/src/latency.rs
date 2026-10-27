//! latency.h: the pure parts of the ink-metrics latency report (unit-tested on the host), and the
//! hook between the two Kindle platform files that feed it.
//!
//! With /mnt/us/grimoire/ink-metrics present, every live ink frame is timed from the pen to the
//! refresh ioctl, and the spread is logged every 10 s (display_fbink.c):
//!   raw     the digitizer's own sample (WacomDigitizer), stamped by the kernel
//!   pen     the same sample relayed by the Kindle's pen service (stylus-custom), which Grimoire reads
//!   submit  Grimoire's refresh ioctl returned
//! Nothing past the ioctl: a submission wait on the driver's markers can panic the kernel, and a
//! second complete wait on one sits out 5 s (display_fbink.c, at the waits). All in
//! CLOCK_MONOTONIC: the input devices are switched to it with EVIOCSCLOCKID.

use crate::prelude::*;

/// Durations in bins of unit_us; the last bin holds everything past the range.
pub const LAT_BINS: usize = 256;
#[derive(Clone, Debug)]
pub struct LatHist {
    pub unit_us: u32,
    pub n: u32,
    pub max_us: u32,
    pub sum_us: u64,
    pub bins: [u32; LAT_BINS],
}

impl Default for LatHist {
    fn default() -> Self {
        LatHist { unit_us: 0, n: 0, max_us: 0, sum_us: 0, bins: [0; LAT_BINS] }
    }
}

pub fn lat_hist_init(h: &mut LatHist, unit_us: u32) {
    *h = LatHist::default();
    h.unit_us = if unit_us != 0 { unit_us } else { 1 };
}
/// a negative duration counts as 0
pub fn lat_hist_add(h: &mut LatHist, us: i64) {
    let mut us = us;
    if us < 0 {
        us = 0;
    }
    if us > u32::MAX as i64 {
        us = u32::MAX as i64;
    }
    let bin = (us as u64) / (h.unit_us as u64);
    h.bins[if bin < LAT_BINS as u64 { bin as usize } else { LAT_BINS - 1 }] += 1;
    h.n += 1;
    h.sum_us += us as u64;
    if us as u32 > h.max_us {
        h.max_us = us as u32;
    }
}
/// The duration under which pct percent of the samples fall (the upper edge of their bin; the
/// largest sample for the last bin), in us. 0 with no samples.
pub fn lat_hist_pct(h: &LatHist, pct: i32) -> u32 {
    if h.n == 0 {
        return 0;
    }
    // the rank of the sample, from 1
    let mut want = ((h.n as u64).wrapping_mul(pct as u64).wrapping_add(99)) / 100;
    if want < 1 {
        want = 1;
    }
    let mut seen: u64 = 0;
    for i in 0..LAT_BINS {
        seen += h.bins[i] as u64;
        if seen < want {
            continue;
        }
        if i == LAT_BINS - 1 {
            return h.max_us;
        }
        let edge = ((i + 1) as u32).wrapping_mul(h.unit_us);
        return if edge < h.max_us { edge } else { h.max_us };
    }
    h.max_us
}
/// "p50 1.2 p90 2.0 max 7.9 ms (n 1250)", or "none".
pub fn lat_hist_format(h: &LatHist, out: &mut String, cap: usize) {
    if h.n == 0 {
        csnprintf!(out, cap, "none");
        return;
    }
    csnprintf!(
        out,
        cap,
        "p50 %.1f p90 %.1f max %.1f ms (n %u)",
        lat_hist_pct(h, 50) as f64 / 1000.0,
        lat_hist_pct(h, 90) as f64 / 1000.0,
        h.max_us as f64 / 1000.0,
        h.n
    );
}

/// The pen service relays each raw digitizer sample as a scaled one. To time that hop, the recent
/// raw samples are kept, and a relayed sample is matched to the raw one at the same place: the
/// relayed axes are the raw ones scaled, possibly reversed (an orientation is learned from the
/// samples, the one whose nearest raw samples have been closest so far).
pub const LAT_RING: usize = 32;
#[derive(Clone, Copy, Debug, Default)]
pub struct LatRaw {
    pub t_us: u64,
    pub x: i32,
    pub y: i32,
}
#[derive(Clone, Debug, Default)]
pub struct LatMatch {
    pub ring: [LatRaw; LAT_RING],
    pub next: i32,
    pub count: i32,
    /// the raw digitizer's x and y ranges
    pub rmin: [i32; 2],
    pub rmax: [i32; 2],
    /// the relayed pen's
    pub smin: [i32; 2],
    pub smax: [i32; 2],
    /// per orientation (bit 0: x reversed, bit 1: y reversed): summed best errors, capped
    pub err: [u64; 4],
}

pub fn lat_match_init(m: &mut LatMatch, rmin: &[i32; 2], rmax: &[i32; 2], smin: &[i32; 2], smax: &[i32; 2]) {
    *m = LatMatch::default();
    m.rmin = *rmin;
    m.rmax = *rmax;
    m.smin = *smin;
    m.smax = *smax;
}
pub fn lat_match_raw(m: &mut LatMatch, t_us: u64, x: i32, y: i32) {
    m.ring[m.next as usize] = LatRaw { t_us, x, y };
    m.next = (m.next + 1) % LAT_RING as i32;
    if m.count < LAT_RING as i32 {
        m.count += 1;
    }
}

// A raw coordinate on axis a, in the relayed pen's units, reversed or not.
// (`(float)smin + f * (float)(smax - smin)` is one fused multiply-add in the C build: PORTING.md,
// Floating point.)
fn scaled(m: &LatMatch, a: usize, v: i32, reversed: bool) -> f32 {
    let span = m.rmax[a] - m.rmin[a];
    if span <= 0 {
        return 0.0;
    }
    let mut f = (v - m.rmin[a]) as f32 / span as f32;
    if reversed {
        f = 1.0 - f;
    }
    fmla(f, (m.smax[a] - m.smin[a]) as f32, m.smin[a] as f32)
}

/// The kernel time of the raw sample a relayed one at (x, y) came from: of the recent raw samples, the
/// nearest (the newest of equals) under the best orientation so far, if within tol px. 0 when none.
pub fn lat_match_pen(m: &mut LatMatch, x: i32, y: i32, tol: i32) -> u64 {
    if m.count == 0 {
        return 0;
    }
    let mut best = [0f32; 4];
    let mut at = [0i32; 4];
    for o in 0..4usize {
        best[o] = 1e9;
        at[o] = -1;
        // Newest first, so the newest of equally near samples wins (a pen held still).
        for k in 1..=m.count {
            let i = (m.next - k + LAT_RING as i32) % LAT_RING as i32;
            let dx = scaled(m, 0, m.ring[i as usize].x, o & 1 != 0) - x as f32;
            let dy = scaled(m, 1, m.ring[i as usize].y, (o & 2) != 0) - y as f32;
            let e = (if dx < 0.0 { -dx } else { dx }) + (if dy < 0.0 { -dy } else { dy });
            if e < best[o] {
                best[o] = e;
                at[o] = i;
            }
        }
        m.err[o] += (if best[o] < 64.0 { best[o] } else { 64.0 }) as u64;
    }
    let mut pick = 0usize;
    for o in 1..4usize {
        if m.err[o] < m.err[pick] {
            pick = o;
        }
    }
    if at[pick] < 0 || best[pick] > tol as f32 {
        return 0;
    }
    m.ring[at[pick] as usize].t_us
}

/// Kindle only (input_evdev.c, read by display_fbink.c for each live ink frame): the kernel times of
/// the newest and the oldest pen sample emitted since the last call, and of the raw sample the newest
/// came from (0 when it matched none). False when no pen sample came since the last call, or the pen's
/// events are not in CLOCK_MONOTONIC.
#[cfg(feature = "kindle")]
pub use crate::input_evdev::input_ink_times;
