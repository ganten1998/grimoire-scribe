//! live.h: Live view. Protocol version 2. All integers and IEEE-754 f32 values are little-endian.
//!
//! GET /live/stream carries authoritative PNG tiles, at most about 30 batches/s, and the legacy
//! newest-only cursor. GET /live/pen carries samples on an independent socket and process, at
//! ordinary priority: no PNG encoding, tile socket backpressure or 33 ms sleep is on its path.
//! Both need the same PAIR_LIVE cookie. Each endpoint admits two connections; live_viewers reports
//! the larger count, so a browser with both sockets counts once for sleep/idle policy. Both children
//! die with the app. GET /live/time, also paired, returns {"ms": CLOCK_MONOTONIC milliseconds mod 2^32}.
//!
//! The main process writes a 512-entry ring in mmap'd shared memory. Each entry has an atomic
//! publication stamp and 22 atomic payload words: sample (20 bytes), style (64), epoch (4).
//! The stamp is zero while writing, then the sample sequence; readers check it before and after
//! copying the atomic words. Overrun never blocks the writer. The input hook does no allocation,
//! logging or network IO; one nonblocking wake per pen reader per input batch releases the samples.
//! Kernel event times are used when available, read times otherwise. Sequence zero is reserved;
//! comparisons use signed 32-bit differences, so wrap is safe within half the sequence space.
//!
//! Each frame: u32 length (type byte + payload), u8 type, then payload:
//!  1 hello (12 bytes): u16 version=2, width, height, tile=64; u32 epoch. A turn, page/screen change,
//!    sleep or pairing transition changes epoch. Clear provisional ink, never join across epochs.
//!  2 tile: u16 x,y,width,height, then a grayscale PNG. Only /live/stream.
//!  3 batch (16): u32 batch sequence, send time ms, presented sample sequence, presented sample ms.
//!    The last two fields describe input already dispatched and ALL its dirty pixels copied by
//!    app_flush, never the ring head (work_pump can read ahead). A child snapshots this watermark
//!    before taking tile marks. Zero means no acknowledgement. A failed/incomplete tile batch
//!    acknowledges nothing. A no-pixel-change acknowledgement still gets a batch. Retire preview
//!    segments only when the complete batch is decoded AND painted, by sequence, not time.
//!  4 ping (0): after 15 s idle.
//!  5 state (3): u8 awake=0/asleep=1/pairing=2/bye=3, inverted (0/1), warmth (0..254; 255 unknown).
//!    First on both streams. Non-awake clears ink/cursor and sends no screen or sample content.
//!    Tile stream also sends inversion/warmth changes. Pairing codes and native PIN input stay out.
//!  6 cursor (5): u16 x,y; u8 away=0/hover=1/touch=2. Legacy cursor on /live/stream, after tiles,
//!    newest only, at most once per 33 ms. The v2 browser ignores it while /live/pen is connected.
//!  7 samples (8 + 20*n): u32 epoch, reset (0/1), then n samples. Reset forgets previous stroke
//!    continuity; n may be zero. Current sender emits one sample per frame, immediately on wake.
//!    Each sample: +0 u32 sequence, +4 u32 time ms, +8 u16 x, +10 u16 y (screen pixels),
//!    +12 u16 normalized pressure 0..65535, +14 i8 tilt x, +15 i8 tilt y (screen-frame degrees),
//!    +16 u8 away/hover/touch, +17 u8 flags, +18 u16 reserved=0.
//!    Flags: 1 eraser end, 2 side button, 4 down, 8 input gap, 16 preview permitted. Missing sequence
//!    numbers and gaps break continuity. Only an explicit down can begin ink. A reconnect sends
//!    only the newest place, without down; an overrun resets continuity and sends the retained tail.
//!  8 style (64): applies to following samples, sent before the first and whenever it changes.
//!    +0 u8 enabled, +1 u8 tool (existing 0..9 IDs), +2 u8 white, +3 u8 whole-stroke eraser;
//!    +4,+8 f32 min/max screen width; +12,+16,+20 f32 pressure floor/ceiling/gamma;
//!    +24,+28 f32 min/max opacity; +32 four i32 page clip x,y,w,h; +48 four i32 panel exclusion.
//!    Width/opacity use clamp((pressure-floor)/(ceiling-floor),0,1)^gamma. Preview is round ink;
//!    nibs, grain, smoothing, tilt response and exact paper restoration come from the real tiles.
//!    Whole-stroke erasers show only the cursor: samples cannot identify the strokes they remove.
//!
//! Every sample stores its style, so a queued sample cannot inherit a later tool. UI contacts latch
//! preview off through their gesture; while queued UI work or page rendering runs, capture is
//! conservative. The browser clips ink away from the panel, bounds history to 2048 segments, and
//! uses no prediction. Its meter estimates sample age at the rAF drawing callback, not physical
//! scanout, from the smallest of eight timing RTTs (uncertainty at least RTT/2); refreshes every 30 s.
//! Tile age is batch send to drawing callback. Neither is a phone-video measurement of visible ink.

use crate::prelude::*;
use std::cell::RefCell;
use std::sync::atomic::{fence, AtomicU32, Ordering::SeqCst};


pub const LIVE_TILE: i32 = 64;
/// tile marks: 2048 tiles, more than either Scribe's screen has
pub const LIVE_WORDS: usize = 64;
pub const LIVE_VIEWERS: usize = 2;
/// no viewer for this long: Live view turns itself off
pub const LIVE_IDLE_OFF_MS: u32 = 10 * 60 * 1000;
/// the state frame's warmth when there is none to follow
pub const LIVE_NO_WARMTH: i32 = 255;
/// legacy cursor only; fast samples never wait for it
pub const LIVE_PEN_MS: u32 = 33;

pub const LIVE_AWAKE: i32 = 0;
pub const LIVE_ASLEEP: i32 = 1;
pub const LIVE_PAIRING: i32 = 2;
pub const LIVE_BYE: i32 = 3;
pub const LIVE_HELLO: i32 = 1;
pub const LIVE_TILE_FRAME: i32 = 2;
pub const LIVE_BATCH: i32 = 3;
pub const LIVE_PING: i32 = 4;
pub const LIVE_STATE: i32 = 5;
pub const LIVE_PEN_FRAME: i32 = 6;
pub const LIVE_SAMPLES: i32 = 7;
pub const LIVE_STYLE: i32 = 8;
pub const LIVE_PEN_AWAY: i32 = 0;
pub const LIVE_PEN_HOVER: i32 = 1;
pub const LIVE_PEN_TOUCH: i32 = 2;

// Fast ink uses its own connection, so neither PNG encoding nor a full tile socket can hold it.
pub const LIVE_RING: usize = 512;
pub const LIVE_SAMPLE_BYTES: usize = 20;
pub const LIVE_STYLE_BYTES: usize = 64;
// flags: eraser end, side button, stroke start, discontinuity, allowed to preview this sample.
pub const LIVE_ERASER: u8 = 1;
pub const LIVE_SIDE: u8 = 2;
pub const LIVE_DOWN: u8 = 4;
pub const LIVE_GAP: u8 = 8;
pub const LIVE_DRAW: u8 = 16;
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LiveStyle {
    pub enabled: u8,
    pub tool: u8,
    pub white: u8,
    pub whole: u8,
    pub width_min: f32,
    pub width_max: f32,
    pub pressure_floor: f32,
    pub pressure_ceiling: f32,
    pub pressure_gamma: f32,
    pub opacity_min: f32,
    pub opacity_max: f32,
    pub clip: Rect,
    pub exclude: Rect,
}

// live.c: Live view. The main loop owns the shared frame and the viewer table; each viewer
// is a forked child that reads the frame, never writes it, and owns one browser's connection.

// (the C's _Static_assert: the tile marks must work across fork)
const _: () = assert!(cfg!(target_has_atomic = "32"), "the tile marks must work across fork");

// The frame both sides map. w and h change only on a turn, written before epoch moves on; px holds
// w * h bytes (the same in either orientation), rows w apart, right after this header (the C's
// flexible array member). Each viewer slot has marks of its own: a viewer takes its marks as it
// sends, which must leave the other's. pen is live_pen_word's.
#[repr(C)]
struct Shared {
    epoch: AtomicU32,
    state: AtomicU32,
    seq: AtomicU32,
    invert: AtomicU32,
    warmth: AtomicU32,
    pen: AtomicU32,
    w: AtomicU32,
    h: AtomicU32,
    sample_head: AtomicU32,
    ack_version: AtomicU32,
    ack_sample: AtomicU32,
    ack_ms: AtomicU32,
    samples: [SampleSlot; LIVE_RING],
    dirty: [[AtomicU32; LIVE_WORDS]; LIVE_VIEWERS],
}
#[repr(C)]
struct SampleSlot {
    stamp: AtomicU32,
    words: [AtomicU32; 22],
}
/// sizeof(Shared) in the C: where px begins
const SHARED_SIZE: usize = std::mem::size_of::<Shared>();

/// The mapping (g_sh and g_sh_size): a copy of it is what the viewer child is handed.
#[derive(Clone, Copy)]
struct Frame {
    base: *mut Shared,
    size: usize,
}

impl Frame {
    fn sh(&self) -> &Shared {
        // SAFETY: base is the start of the MAP_SHARED | MAP_ANON mapping live_start made, page
        // aligned, zero filled and at least size_of::<Shared>() long; Shared holds only atomics, so
        // a shared reference is valid however the other processes write it. The mapping stays until
        // live_stop unmaps it, which is after the last use of any copy of the Frame in this process.
        unsafe { &*self.base }
    }
    /// The first pixel byte. The pixels are read and written through this pointer in short
    /// copies, never as a slice: another process (the main loop, or a viewer) is at them too.
    fn px(&self) -> *mut u8 {
        // SAFETY: the mapping is SHARED_SIZE + w * h long, so SHARED_SIZE is inside it (or its end).
        unsafe { (self.base as *mut u8).add(SHARED_SIZE) }
    }
    fn px_len(&self) -> usize {
        self.size - SHARED_SIZE
    }
}

#[derive(Clone, Copy)]
struct Viewer {
    pid: libc::pid_t,
    /// the write end of its wake pipe (non-blocking)
    wake: i32,
}

struct Live {
    sh: Option<Frame>,
    v: [Viewer; LIVE_VIEWERS * 2],
    sample_wake: bool,
    /// when the last viewer left (or Live view started)
    last_viewer: u32,
}

thread_local! {
    static S: RefCell<Live> = const { RefCell::new(Live { sh: None, v: [Viewer { pid: 0, wake: 0 }; LIVE_VIEWERS * 2], sample_wake: false, last_viewer: 0 }) };
}
fn with<R>(f: impl FnOnce(&mut Live) -> R) -> R {
    S.with(|s| f(&mut s.borrow_mut()))
}
fn frame() -> Option<Frame> {
    with(|s| s.sh)
}

// ---- pure parts ----

pub fn live_tiles_across(w: i32) -> i32 {
    (w + LIVE_TILE - 1) / LIVE_TILE
}

pub fn live_mark(words: &mut [u32; LIVE_WORDS], w: i32, h: i32, r: Rect) {
    let r = rect_intersect(r, Rect { x: 0, y: 0, w, h });
    if rect_is_empty(r) {
        return;
    }
    let across = live_tiles_across(w);
    for ty in r.y / LIVE_TILE..=(r.y + r.h - 1) / LIVE_TILE {
        for tx in r.x / LIVE_TILE..=(r.x + r.w - 1) / LIVE_TILE {
            let i = ty * across + tx;
            if i < (LIVE_WORDS * 32) as i32 {
                words[(i / 32) as usize] |= 1u32 << (i % 32);
            }
        }
    }
}

fn marked(words: &[u32; LIVE_WORDS], i: i32) -> bool {
    i < (LIVE_WORDS * 32) as i32 && (words[(i / 32) as usize] >> (i % 32)) & 1 != 0
}

pub fn live_runs(words: &[u32; LIVE_WORDS], w: i32, h: i32, out: &mut [Rect]) -> i32 {
    let across = live_tiles_across(w);
    let down = (h + LIVE_TILE - 1) / LIVE_TILE;
    let mut n = 0usize;
    for ty in 0..down {
        let mut tx = 0;
        while tx < across {
            if !marked(words, ty * across + tx) {
                tx += 1;
                continue;
            }
            let mut end = tx;
            while end < across && marked(words, ty * across + end) {
                end += 1;
            }
            if n < out.len() {
                out[n] = rect_intersect(Rect { x: tx * LIVE_TILE, y: ty * LIVE_TILE, w: (end - tx) * LIVE_TILE, h: LIVE_TILE }, Rect { x: 0, y: 0, w, h });
                n += 1;
            }
            tx = end;
        }
    }
    n as i32
}

pub fn live_frame_header(out: &mut [u8; 5], len: u32, type_: u8) {
    let total = len.wrapping_add(1);
    out[0] = total as u8;
    out[1] = (total >> 8) as u8;
    out[2] = (total >> 16) as u8;
    out[3] = (total >> 24) as u8;
    out[4] = type_;
}

// A coordinate as a whole pixel of a screen n wide: cut as a float first (NaN to 0), so a pen
// calibrated past the edge never overflows the int.
#[allow(clippy::neg_cmp_op_on_partial_ord)] // (!(v > 0): a NaN gives 0 too)
fn pen_px(v: f32, n: i32) -> u32 {
    if !(v > 0.0) {
        0
    } else if v >= (n - 1) as f32 {
        (n - 1) as u32
    } else {
        (v + 0.5) as u32
    }
}

pub fn live_pen_word(state: i32, x: f32, y: f32, w: i32, h: i32) -> u32 {
    if (state != LIVE_PEN_HOVER && state != LIVE_PEN_TOUCH) || w <= 0 || h <= 0 || w > 0x7FFF || h > 0x7FFF {
        return 0;
    }
    (state as u32) << 30 | pen_px(y, h) << 15 | pen_px(x, w)
}

pub fn live_pen_payload(out: &mut [u8; 5], word: u32) {
    let x = word & 0x7FFF;
    let y = word >> 15 & 0x7FFF;
    out[0] = x as u8;
    out[1] = (x >> 8) as u8;
    out[2] = y as u8;
    out[3] = (y >> 8) as u8;
    out[4] = (word >> 30) as u8;
}

// ---- the main loop ----

pub fn live_on() -> bool {
    frame().is_some()
}

pub fn live_viewers() -> i32 {
    with(|s| {
        let mut tiles = 0;
        let mut pens = 0;
        for i in 0..LIVE_VIEWERS {
            tiles += (s.v[i].pid > 0) as i32;
            pens += (s.v[i + LIVE_VIEWERS].pid > 0) as i32;
        }
        if tiles > pens {
            tiles
        } else {
            pens
        }
    })
}

fn wake_all() {
    let v = with(|s| s.v);
    for viewer in v.iter() {
        if viewer.pid > 0 {
            // SAFETY: write(2) of one byte from a local to the wake pipe's write end, which this
            // process owns for as long as the viewer's pid is set.
            let r = unsafe { libc::write(viewer.wake, b"x".as_ptr() as *const libc::c_void, 1) };
            if r < 0 { /* full: it is awake already */ }
        }
    }
}

fn mark_all(f: &Frame) {
    let sh = f.sh();
    for v in 0..LIVE_VIEWERS {
        for i in 0..LIVE_WORDS {
            sh.dirty[v][i].store(0xFFFFFFFF, SeqCst);
        }
    }
}

/// Turns it on for a w x h screen: the shared frame, all of it marked. -1 when there is no memory
/// (or the screen has more tiles than the marks hold).
pub fn live_start(w: i32, h: i32) -> i32 {
    if live_on() {
        return 0;
    }
    if w <= 0
        || h <= 0
        || live_tiles_across(w) * ((h + LIVE_TILE - 1) / LIVE_TILE) > (LIVE_WORDS * 32) as i32
        || live_tiles_across(h) * ((w + LIVE_TILE - 1) / LIVE_TILE) > (LIVE_WORDS * 32) as i32
    {
        return -1;
    }
    let size = SHARED_SIZE + (w as usize) * (h as usize);
    // SAFETY: an anonymous shared mapping: no file, no address asked for; the result is checked.
    let p = unsafe { libc::mmap(std::ptr::null_mut(), size, libc::PROT_READ | libc::PROT_WRITE, libc::MAP_SHARED | libc::MAP_ANON, -1, 0) };
    if p == libc::MAP_FAILED {
        return -1;
    }
    let f = Frame { base: p as *mut Shared, size };
    // SAFETY: px() is the start of the w * h pixel bytes of the mapping, which nobody else has yet.
    unsafe { std::ptr::write_bytes(f.px(), 255, (w as usize) * (h as usize)) };
    let sh = f.sh();
    sh.w.store(w as u32, SeqCst);
    sh.h.store(h as u32, SeqCst);
    sh.state.store(LIVE_AWAKE as u32, SeqCst);
    sh.warmth.store(LIVE_NO_WARMTH as u32, SeqCst);
    sh.epoch.store(1, SeqCst);
    mark_all(&f);
    with(|s| {
        s.sh = Some(f);
        s.last_viewer = now_ms();
    });
    0
}

fn reap(i: usize, wait_for_it: bool) {
    let v = with(|s| s.v[i]);
    if v.pid <= 0 {
        return;
    }
    // SAFETY: waitpid on a child this process forked (the pid is set only by live_take_viewer), no status wanted.
    let r = unsafe { libc::waitpid(v.pid, std::ptr::null_mut(), if wait_for_it { 0 } else { libc::WNOHANG }) };
    if r <= 0 && !wait_for_it {
        return;
    }
    // SAFETY: closes the wake pipe's write end, which this process owns.
    unsafe { libc::close(v.wake) };
    with(|s| {
        s.v[i].pid = 0;
        s.v[i].wake = -1;
    });
    if live_viewers() == 0 {
        with(|s| s.last_viewer = now_ms());
    }
}

/// Turns it off: each viewer is told (LIVE_BYE), then stopped and reaped; the frame goes.
pub fn live_stop() {
    let Some(f) = frame() else { return };
    f.sh().state.store(LIVE_BYE as u32, SeqCst);
    wake_all();
    // Each viewer says bye and ends; one that does not within a second is stopped.
    let t0 = now_ms();
    while live_viewers() > 0 && now_ms().wrapping_sub(t0) < 1000 {
        for i in 0..LIVE_VIEWERS * 2 {
            reap(i, false);
        }
        if live_viewers() > 0 {
            sleep_ms(10);
        }
    }
    for i in 0..LIVE_VIEWERS * 2 {
        let pid = with(|s| s.v[i].pid);
        if pid > 0 {
            // SAFETY: SIGTERM to a child this process forked and has not reaped yet.
            unsafe { libc::kill(pid, libc::SIGTERM) };
            reap(i, true);
        }
    }
    // SAFETY: the mapping live_start made, whole; nothing uses it after this (the Frame is
    // dropped from the state first, and the viewers are gone).
    unsafe { libc::munmap(f.base as *mut libc::c_void, f.size) };
    with(|s| {
        s.sh = None;
        s.sample_wake = false;
    });
}

/// A rect of the screen presented: copied into the frame, its tiles marked, the viewers woken.
pub fn live_push(screen: &Canvas, r: Rect) {
    let Some(f) = frame() else { return };
    let sh = f.sh();
    let w = sh.w.load(SeqCst) as i32;
    let h = sh.h.load(SeqCst) as i32;
    if screen.w != w || screen.h != h {
        return; // (a turn not told yet: live_resize comes)
    }
    let r = rect_intersect(r, Rect { x: 0, y: 0, w, h });
    if rect_is_empty(r) {
        return;
    }
    for y in r.y..r.y + r.h {
        let from = &screen.px[(y as usize) * (screen.stride as usize) + r.x as usize..][..r.w as usize];
        // SAFETY: the row (y, r.x..r.x + r.w) is inside the w * h pixel bytes of the mapping (r is
        // cut to the screen), and `from` is a slice of the caller's canvas, a different allocation.
        unsafe { std::ptr::copy_nonoverlapping(from.as_ptr(), f.px().add((y as usize) * (w as usize) + r.x as usize), r.w as usize) };
    }
    // Marked after the copy: a viewer that read a tile while it was being written sees it marked
    // again, and sends it again whole.
    let mut words = [0u32; LIVE_WORDS];
    live_mark(&mut words, w, h, r);
    for marks in sh.dirty.iter() {
        for (mark, word) in marks.iter().zip(words.iter()) {
            if *word != 0 {
                mark.fetch_or(*word, SeqCst);
            }
        }
    }
    wake_all();
}

/// The screen turned (w x h now): a new hello, every tile again, the pen away until it is told again.
pub fn live_resize(w: i32, h: i32) {
    let Some(f) = frame() else { return };
    if (w as usize).wrapping_mul(h as usize).wrapping_add(SHARED_SIZE) != f.size {
        return;
    }
    let sh = f.sh();
    sh.w.store(w as u32, SeqCst);
    sh.h.store(h as u32, SeqCst);
    sh.pen.store(0, SeqCst); // (its place was on the screen before the turn)
    sh.ack_sample.store(0, SeqCst);
    sh.ack_ms.store(0, SeqCst);
    // SAFETY: px() is the start of the w * h pixel bytes (the size was checked against the mapping's).
    unsafe { std::ptr::write_bytes(f.px(), 255, (w as usize) * (h as usize)) };
    sh.epoch.fetch_add(1, SeqCst);
    mark_all(&f);
    wake_all();
}

pub fn live_reset() {
    let Some(f) = frame() else { return };
    let sh = f.sh();
    sh.ack_sample.store(0, SeqCst);
    sh.ack_ms.store(0, SeqCst);
    sh.epoch.fetch_add(1, SeqCst);
    mark_all(&f);
    wake_all();
}

/// LIVE_AWAKE, LIVE_ASLEEP or LIVE_PAIRING (the pairing card is up: nothing is sent while it is).
/// Any but LIVE_AWAKE takes the pen away.
pub fn live_set_state(state: i32) {
    let Some(f) = frame() else { return };
    let sh = f.sh();
    // The state first, then (for the pairing card) its pixels: a viewer that reads them sees the
    // state after its own fence, and sends nothing.
    let previous = sh.state.swap(state as u32, SeqCst);
    if previous != state as u32 {
        live_reset();
    }
    if state != LIVE_AWAKE {
        sh.pen.store(0, SeqCst); // (the browsers take it away with the state)
    }
    fence(SeqCst);
    if state == LIVE_AWAKE {
        mark_all(&f); // whatever showed meanwhile comes again
    }
    wake_all();
}

/// The pen: LIVE_PEN_AWAY, LIVE_PEN_HOVER or LIVE_PEN_TOUCH at (x, y), the screen's pixels as the
/// events have them (after the turn). One atomic store; a write to the viewers' wake pipes only when
/// it comes into range. The caller never tells it while the Kindle sleeps (live_set_state clears it).
pub fn live_pen(state: i32, x: f32, y: f32) {
    let Some(f) = frame() else { return };
    let sh = f.sh();
    let word = live_pen_word(state, x, y, sh.w.load(SeqCst) as i32, sh.h.load(SeqCst) as i32);
    // A sample is this store and nothing more. Only the pen coming into range wakes the viewers:
    // while it is in range they look at it again every LIVE_PEN_MS themselves (viewer_main).
    if sh.pen.swap(word, SeqCst) == 0 && word != 0 {
        wake_all();
    }
}

/// Grimoire's colors inverted on the Kindle or not (settings.invert, applied as it presents: the
/// frame holds them as drawn): the viewers tell their browsers, which follow it unless told not to.
pub fn live_set_invert(on: bool) {
    let Some(f) = frame() else { return };
    if f.sh().invert.swap(on as u32, SeqCst) == on as u32 {
        return;
    }
    wake_all();
}

/// The Kindle's front light warmth (0..LIGHT_MAX; -1: no warm light, or not known): the browsers
/// tint the picture's light with it, unless told otherwise.
pub fn live_set_warmth(level: i32) {
    let v: u32 = if level < 0 { LIVE_NO_WARMTH as u32 } else { clampi(level, 0, LIVE_NO_WARMTH - 1) as u32 };
    let Some(f) = frame() else { return };
    if f.sh().warmth.swap(v, SeqCst) == v {
        return;
    }
    wake_all();
}

/// Reaps viewers that ended. Returns how many viewers there are now. *idle_off is set when Live
/// view has had no viewer for LIVE_IDLE_OFF_MS (the caller turns it off and says so).
pub fn live_poll(now: u32, idle_off: &mut bool) -> i32 {
    *idle_off = false;
    if !live_on() {
        return 0;
    }
    for i in 0..LIVE_VIEWERS * 2 {
        reap(i, false);
    }
    let n = live_viewers();
    if n == 0 && (now.wrapping_sub(with(|s| s.last_viewer)) as i32) >= LIVE_IDLE_OFF_MS as i32 {
        *idle_off = true;
    }
    n
}

// ---- a viewer ----

// (MSG_NOSIGNAL where the platform has it; the viewer ignores SIGPIPE anyway)
#[cfg(any(target_os = "linux", target_os = "android"))]
const MSG_NOSIGNAL: libc::c_int = libc::MSG_NOSIGNAL;
#[cfg(not(any(target_os = "linux", target_os = "android")))]
const MSG_NOSIGNAL: libc::c_int = 0;

fn send_all(fd: i32, mut data: &[u8]) -> bool {
    while !data.is_empty() {
        // SAFETY: send(2) reads data.len() bytes from data, a live slice.
        let w = unsafe { libc::send(fd, data.as_ptr() as *const libc::c_void, data.len(), MSG_NOSIGNAL) };
        if w < 0 && errno() == libc::EINTR {
            continue;
        }
        if w <= 0 {
            return false;
        }
        data = &data[w as usize..];
    }
    true
}

fn send_frame(fd: i32, type_: u8, a: &[u8], b: &[u8]) -> bool {
    let mut head = [0u8; 5];
    live_frame_header(&mut head, (a.len() + b.len()) as u32, type_);
    if a.len() + b.len() <= 128 {
        let mut packet = [0u8; 133];
        packet[..5].copy_from_slice(&head);
        packet[5..5 + a.len()].copy_from_slice(a);
        packet[5 + a.len()..5 + a.len() + b.len()].copy_from_slice(b);
        return send_all(fd, &packet[..5 + a.len() + b.len()]);
    }
    send_all(fd, &head) && (a.is_empty() || send_all(fd, a)) && (b.is_empty() || send_all(fd, b))
}

fn put16(p: &mut [u8], v: u32) {
    p[0] = v as u8;
    p[1] = (v >> 8) as u8;
}
fn put32(p: &mut [u8], v: u32) {
    put16(p, v & 0xFFFF);
    put16(&mut p[2..], v >> 16);
}

pub fn live_style_payload(out: &mut [u8], s: &LiveStyle) {
    out[0] = s.enabled;
    out[1] = s.tool;
    out[2] = s.white;
    out[3] = s.whole;
    let values = [s.width_min, s.width_max, s.pressure_floor, s.pressure_ceiling, s.pressure_gamma, s.opacity_min, s.opacity_max];
    for (i, v) in values.iter().enumerate() {
        put32(&mut out[4 + i * 4..], v.to_bits());
    }
    let rects = [s.clip.x, s.clip.y, s.clip.w, s.clip.h, s.exclude.x, s.exclude.y, s.exclude.w, s.exclude.h];
    for (i, v) in rects.iter().enumerate() {
        put32(&mut out[32 + i * 4..], *v as u32);
    }
}

pub fn live_sample(ev: &InEvent, style: &LiveStyle) -> u32 {
    let Some(f) = frame() else { return 0 };
    let sh = f.sh();
    if sh.state.load(SeqCst) != LIVE_AWAKE as u32 {
        return 0;
    }
    let mut seq = sh.sample_head.load(SeqCst).wrapping_add(1);
    if seq == 0 {
        seq = 1; // zero means an event that was not captured
    }
    let mut record = [0u8; 88];
    put32(&mut record, seq);
    put32(&mut record[4..], in_sample_ms(ev));
    put16(&mut record[8..], pen_px(ev.x, sh.w.load(SeqCst) as i32));
    put16(&mut record[10..], pen_px(ev.y, sh.h.load(SeqCst) as i32));
    put16(&mut record[12..], pen_px(ev.pressure * 65535.0, 65536));
    record[14] = clampf(ev.tilt_x, -90.0, 90.0) as i8 as u8;
    record[15] = clampf(ev.tilt_y, -90.0, 90.0) as i8 as u8;
    record[16] = if ev.type_ == InType::PenLeave {
        LIVE_PEN_AWAY
    } else if ev.type_ == InType::PenDown || ev.type_ == InType::PenMove {
        LIVE_PEN_TOUCH
    } else {
        LIVE_PEN_HOVER
    } as u8;
    record[17] = (if ev.eraser { LIVE_ERASER } else { 0 })
        | (if ev.side_button { LIVE_SIDE } else { 0 })
        | (if ev.type_ == InType::PenDown { LIVE_DOWN } else { 0 })
        | (if ev.gap { LIVE_GAP } else { 0 })
        | (if style.enabled != 0 { LIVE_DRAW } else { 0 });
    live_style_payload(&mut record[20..20 + LIVE_STYLE_BYTES], style);
    put32(&mut record[84..], sh.epoch.load(SeqCst));
    let slot = &sh.samples[seq as usize % LIVE_RING];
    // Every payload word is atomic as well: an overrun may write while a child reads. The two
    // stamp reads reject that snapshot, without a C data race or a lock held by a slow viewer.
    slot.stamp.store(0, SeqCst);
    for (i, word) in slot.words.iter().enumerate() {
        word.store(get32(&record[i * 4..]), SeqCst);
    }
    slot.stamp.store(seq, SeqCst);
    sh.sample_head.store(seq, SeqCst);
    with(|s| s.sample_wake = true);
    seq
}

pub fn live_wake_samples() {
    let v = with(|s| {
        if s.sh.is_none() || !s.sample_wake {
            return None;
        }
        s.sample_wake = false;
        Some(s.v)
    });
    let Some(v) = v else { return };
    for viewer in &v[LIVE_VIEWERS..] {
        if viewer.pid > 0 {
            // SAFETY: write(2) of one byte from a static to the wake pipe's write end, which this
            // process owns for as long as the viewer's pid is set.
            let r = unsafe { libc::write(viewer.wake, b"p".as_ptr() as *const libc::c_void, 1) };
            if r < 0 { /* already awake */ }
        }
    }
}

pub fn live_presented(sample: u32, ms: u32) {
    let Some(f) = frame() else { return };
    let sh = f.sh();
    if sample == 0 || sh.ack_sample.load(SeqCst) == sample {
        return;
    }
    sh.ack_version.fetch_add(1, SeqCst);
    sh.ack_sample.store(sample, SeqCst);
    sh.ack_ms.store(ms, SeqCst);
    sh.ack_version.fetch_add(1, SeqCst);
    wake_all();
}

fn read_ack(sh: &Shared, sample: &mut u32, ms: &mut u32) {
    let a = sh.ack_version.load(SeqCst);
    *sample = sh.ack_sample.load(SeqCst);
    *ms = sh.ack_ms.load(SeqCst);
    if (a & 1) != 0 || a != sh.ack_version.load(SeqCst) {
        *sample = 0;
        *ms = 0;
    }
}

fn sample_read(sh: &Shared, seq: u32, out: &mut [u8; 88]) -> bool {
    let slot = &sh.samples[seq as usize % LIVE_RING];
    if slot.stamp.load(SeqCst) != seq {
        return false;
    }
    for (i, word) in slot.words.iter().enumerate() {
        put32(&mut out[i * 4..], word.load(SeqCst));
    }
    slot.stamp.load(SeqCst) == seq
}

fn get32(p: &[u8]) -> u32 {
    p[0] as u32 | (p[1] as u32) << 8 | (p[2] as u32) << 16 | (p[3] as u32) << 24
}

fn send_hello(fd: i32, w: i32, h: i32, epoch: u32) -> bool {
    let mut p = [0u8; 12];
    put16(&mut p, 2);
    put16(&mut p[2..], w as u32);
    put16(&mut p[4..], h as u32);
    put16(&mut p[6..], LIVE_TILE as u32);
    put32(&mut p[8..], epoch);
    send_frame(fd, LIVE_HELLO as u8, &p, &[])
}

fn send_state(fd: i32, state: u32, invert: u32, warmth: u32) -> bool {
    let s = [state as u8, if invert != 0 { 1 } else { 0 }, warmth as u8];
    send_frame(fd, LIVE_STATE as u8, &s, &[])
}

fn send_pen(fd: i32, word: u32) -> bool {
    let mut p = [0u8; 5];
    live_pen_payload(&mut p, word);
    send_frame(fd, LIVE_PEN_FRAME as u8, &p, &[])
}

/// A PNG that stb_image_write made (malloc'ed by it), freed when dropped.
struct Png {
    ptr: *mut u8,
    len: usize,
}

impl Png {
    /// The w x h gray tile as a PNG, None when stb has no memory for it.
    fn of_tile(tile: &[u8], w: i32, h: i32) -> Option<Png> {
        assert!(tile.len() >= (w as usize) * (h as usize));
        let mut len: libc::c_int = 0;
        // SAFETY: stb reads h rows of w bytes from tile (checked above) and returns a malloc'ed
        // buffer of len bytes, or null.
        let ptr = unsafe { crate::ffi_stb::stbi_write_png_to_mem(tile.as_ptr(), w, w, h, 1, &mut len) };
        if ptr.is_null() {
            None
        } else {
            Some(Png { ptr, len: len as usize })
        }
    }
    fn as_slice(&self) -> &[u8] {
        // SAFETY: ptr points at len bytes stb wrote, alive until drop.
        unsafe { std::slice::from_raw_parts(self.ptr, self.len) }
    }
}

impl Drop for Png {
    fn drop(&mut self) {
        // SAFETY: stb allocated ptr with malloc, and nothing else frees it.
        unsafe { libc::free(self.ptr as *mut libc::c_void) };
    }
}

fn set_int_opt(fd: i32, level: libc::c_int, name: libc::c_int, v: libc::c_int) {
    // SAFETY: setsockopt reads size_of::<c_int>() bytes from the local v. (Best effort, as the C: the result is not read.)
    unsafe { libc::setsockopt(fd, level, name, &v as *const libc::c_int as *const libc::c_void, std::mem::size_of::<libc::c_int>() as libc::socklen_t) };
}

// _exit(0): ends the child at once, running none of the parent's exit handlers.
fn child_exit() -> ! {
    // SAFETY: _exit takes no pointers and never returns.
    unsafe { libc::_exit(0) }
}

fn pen_main(f: Frame, fd: i32, wake: i32, parent: libc::pid_t) -> ! {
    let sh = f.sh();
    let mut epoch: u32 = 0;
    let mut cursor: u32 = 0;
    let mut last = now_ms();
    let mut told: u32 = !0;
    let mut style = [0u8; LIVE_STYLE_BYTES];
    let mut have_style = false;
    let mut reset = true;
    let mut snapshot = true;
    loop {
        let state = sh.state.load(SeqCst);
        let e = sh.epoch.load(SeqCst);
        if state != told {
            if !send_state(fd, state, sh.invert.load(SeqCst), sh.warmth.load(SeqCst)) {
                break;
            }
            told = state;
            reset = true;
            cursor = sh.sample_head.load(SeqCst);
            if state == LIVE_BYE as u32 {
                break;
            }
        }
        if state == LIVE_AWAKE as u32 && (e != epoch || reset) {
            epoch = e;
            if !send_hello(fd, sh.w.load(SeqCst) as i32, sh.h.load(SeqCst) as i32, epoch) {
                break;
            }
            cursor = sh.sample_head.load(SeqCst);
            if cursor != 0 {
                cursor -= 1; // newest place only; never replay a stroke when reconnecting
            }
            have_style = false;
            snapshot = cursor != sh.sample_head.load(SeqCst);
            let mut packet = [0u8; 8];
            put32(&mut packet, epoch);
            put32(&mut packet[4..], 1);
            if !send_frame(fd, LIVE_SAMPLES as u8, &packet, &[]) {
                break;
            }
            reset = false;
        }
        if state == LIVE_AWAKE as u32 {
            let head = sh.sample_head.load(SeqCst);
            if head.wrapping_sub(cursor) > LIVE_RING as u32 {
                cursor = head.wrapping_sub(LIVE_RING as u32);
                reset = true;
            }
            while cursor != head {
                let mut next = cursor.wrapping_add(1);
                if next == 0 {
                    next = 1;
                }
                let mut record = [0u8; 88];
                let mut packet = [0u8; 28];
                if !sample_read(sh, next, &mut record) {
                    reset = true;
                    break;
                }
                cursor = next;
                if get32(&record[84..]) != epoch {
                    continue;
                }
                if sh.state.load(SeqCst) != LIVE_AWAKE as u32 || sh.epoch.load(SeqCst) != epoch {
                    break;
                }
                if !have_style || style[..] != record[20..20 + LIVE_STYLE_BYTES] {
                    style.copy_from_slice(&record[20..20 + LIVE_STYLE_BYTES]);
                    if !send_frame(fd, LIVE_STYLE as u8, &style, &[]) {
                        child_exit();
                    }
                    have_style = true;
                }
                put32(&mut packet, epoch);
                put32(&mut packet[4..], if reset { 1 } else { 0 });
                packet[8..8 + LIVE_SAMPLE_BYTES].copy_from_slice(&record[..LIVE_SAMPLE_BYTES]);
                if reset || snapshot {
                    packet[25] &= !LIVE_DOWN;
                }
                snapshot = false;
                if !send_frame(fd, LIVE_SAMPLES as u8, &packet, &[]) {
                    child_exit();
                }
                reset = false;
                last = now_ms();
            }
        }
        if now_ms().wrapping_sub(last) >= 15000 {
            if !send_frame(fd, LIVE_PING as u8, &[], &[]) {
                break;
            }
            last = now_ms();
        }
        let mut pf = [libc::pollfd { fd: wake, events: libc::POLLIN, revents: 0 }, libc::pollfd { fd, events: libc::POLLIN, revents: 0 }];
        // SAFETY: poll reads and writes the two pollfds of the local array.
        let got = unsafe { libc::poll(pf.as_mut_ptr(), 2, 1000) };
        if got < 0 && errno() != libc::EINTR {
            break;
        }
        if pf[1].revents & (libc::POLLHUP | libc::POLLERR | libc::POLLIN) != 0 {
            break;
        }
        if pf[0].revents & libc::POLLIN != 0 {
            let mut drain = [0u8; 256];
            // SAFETY: read into the local array, at most its length, from the wake pipe.
            while unsafe { libc::read(wake, drain.as_mut_ptr() as *mut libc::c_void, drain.len()) } > 0 {}
        }
        // SAFETY: getppid takes no arguments.
        if unsafe { libc::getppid() } != parent {
            break;
        }
    }
    // SAFETY: shutdown of our own socket's write side, then _exit, which ends the child at once and
    // runs none of the parent's exit handlers.
    unsafe {
        libc::shutdown(fd, libc::SHUT_WR);
        libc::_exit(0)
    }
}

fn viewer_main(f: Frame, slot: usize, fd: i32, wake: i32) -> ! {
    let sh = f.sh();
    let fast = slot >= LIVE_VIEWERS;
    let dirty = &sh.dirty[slot % LIVE_VIEWERS];
    for i in 3..1024 {
        if i != fd && i != wake {
            // SAFETY: closes a descriptor of this (forked) process; most of these are not open.
            unsafe { libc::close(i) };
        }
    }
    // SAFETY: sigaction with a zeroed (empty mask, no flags) struct whose handler is SIG_DFL or
    // SIG_IGN, as the C sets them; the old actions are not read.
    unsafe {
        let mut sa: libc::sigaction = std::mem::zeroed();
        sa.sa_sigaction = libc::SIG_DFL;
        libc::sigaction(libc::SIGTERM, &sa, std::ptr::null_mut());
        libc::sigaction(libc::SIGINT, &sa, std::ptr::null_mut());
        libc::sigaction(libc::SIGHUP, &sa, std::ptr::null_mut());
        sa.sa_sigaction = libc::SIG_IGN;
        libc::sigaction(libc::SIGPIPE, &sa, std::ptr::null_mut());
    }
    #[cfg(target_os = "linux")]
    // SAFETY: prctl with a NUL-terminated name and the signal number, no pointers kept.
    unsafe {
        libc::prctl(libc::PR_SET_NAME, c"grimoire-live".as_ptr() as libc::c_ulong, 0 as libc::c_ulong, 0 as libc::c_ulong, 0 as libc::c_ulong);
        libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM as libc::c_ulong, 0 as libc::c_ulong, 0 as libc::c_ulong, 0 as libc::c_ulong);
    }
    // SAFETY: getppid, nice and fcntl on our own descriptor take no pointers. (nice: best effort.)
    let parent = unsafe { libc::getppid() };
    if !fast {
        unsafe { libc::nice(5) };
    }
    unsafe { libc::fcntl(fd, libc::F_SETFL, libc::fcntl(fd, libc::F_GETFL, 0) & !libc::O_NONBLOCK) };
    set_int_opt(fd, libc::IPPROTO_TCP, libc::TCP_NODELAY, 1);
    set_int_opt(fd, libc::SOL_SOCKET, libc::SO_SNDBUF, if fast { 16 * 1024 } else { 256 * 1024 });
    // a browser that takes nothing for 10 s is gone
    let tv = libc::timeval { tv_sec: if fast { 1 } else { 10 }, tv_usec: 0 };
    // SAFETY: setsockopt reads one timeval from the local tv.
    unsafe { libc::setsockopt(fd, libc::SOL_SOCKET, libc::SO_SNDTIMEO, &tv as *const libc::timeval as *const libc::c_void, std::mem::size_of::<libc::timeval>() as libc::socklen_t) };
    // SAFETY: this process's own stb setting (the fastest filter, a little bigger); the viewer is
    // a single thread of a forked child, and nothing else reads it concurrently.
    unsafe { crate::ffi_stb::stbi_write_force_png_filter = 1 };
    static HEAD: &[u8] = b"HTTP/1.0 200 OK\r\nContent-Type: application/octet-stream\r\n\
Cache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nConnection: close\r\n\r\n";
    if fast {
        if !send_all(fd, HEAD) {
            child_exit();
        }
        pen_main(f, fd, wake, parent);
    }
    let cap = f.px_len();
    let mut sent: Vec<u8> = Vec::new();
    let mut tile: Vec<u8> = Vec::new();
    if sent.try_reserve_exact(cap).is_err() || tile.try_reserve_exact(LIVE_TILE as usize * 4096).is_err() || !send_all(fd, HEAD) {
        // SAFETY: _exit ends the child at once, running none of the parent's exit handlers.
        unsafe { libc::_exit(0) };
    }
    sent.resize(cap, 0);
    tile.resize(LIVE_TILE as usize * 4096, 0);
    let mut epoch: u32 = 0;
    let mut last_batch: u32 = 0;
    let mut last_sent = now_ms();
    let mut last_pen: u32 = 0;
    // (none yet: the state goes first)
    let mut told: u32 = !0;
    let mut told_invert: u32 = !0;
    let mut told_warmth: u32 = !0;
    // the pen as the browser has it: away, until a pen frame says otherwise
    let mut told_pen: u32 = 0;
    let mut w: i32 = 0;
    let mut h: i32 = 0;
    let mut ack_sent: u32 = 0;
    'main: loop {
        let state = sh.state.load(SeqCst);
        let invert = sh.invert.load(SeqCst);
        let warmth = sh.warmth.load(SeqCst);
        // The pen read here, before the marks, and sent after the tiles: a place it reached after a
        // push is never sent ahead of that push's tiles (live_push marks them before live_pen stores).
        let mut pen = sh.pen.load(SeqCst);
        if state == LIVE_BYE as u32 {
            send_state(fd, LIVE_BYE as u32, invert, warmth);
            break;
        }
        if state != told || invert != told_invert || warmth != told_warmth {
            if !send_state(fd, state, invert, warmth) {
                break;
            }
            told = state;
            told_invert = invert;
            told_warmth = warmth;
            last_sent = now_ms();
            if state != LIVE_AWAKE as u32 {
                told_pen = 0; // (the browser takes the pen away with it)
            }
        }
        let e = sh.epoch.load(SeqCst);
        if state == LIVE_AWAKE as u32 && e != epoch {
            epoch = e;
            w = sh.w.load(SeqCst) as i32;
            h = sh.h.load(SeqCst) as i32;
            sent.fill(255); // the browser starts from white: send every tile that is not
            for mark in dirty.iter() {
                mark.store(0xFFFFFFFF, SeqCst);
            }
            if !send_hello(fd, w, h, epoch) {
                break;
            }
            // A hello takes the pen away too. Read again: live_resize cleared it before the epoch
            // moved on, so a place read above may be one on the screen before the turn.
            told_pen = 0;
            ack_sent = 0;
            pen = sh.pen.load(SeqCst);
        }
        if state == LIVE_AWAKE as u32 {
            let (mut ack, mut ack_ms) = (0, 0);
            read_ack(sh, &mut ack, &mut ack_ms); // before the marks: every acknowledged pixel is in this batch or an earlier one
            let mut words = [0u32; LIVE_WORDS];
            let mut any = false;
            for i in 0..LIVE_WORDS {
                words[i] = dirty[i].swap(0, SeqCst);
                any |= words[i] != 0;
            }
            if any || (ack != 0 && ack != ack_sent) {
                let mut runs = [RECT_EMPTY; LIVE_WORDS * 32];
                let n = live_runs(&words, w, h, &mut runs);
                let mut out = 0;
                let mut ok = true;
                for &r in runs[..n as usize].iter() {
                    // A snapshot of the run, then the state again: a pairing card drawn meanwhile
                    // is never sent, and its tiles come again once it is gone (live_set_state).
                    let mut changed = false;
                    for y in 0..r.h as usize {
                        let at = (r.y as usize + y) * (w as usize) + r.x as usize;
                        let row = &mut tile[y * r.w as usize..][..r.w as usize];
                        // SAFETY: the run is cut to the w x h screen, so the row is inside the
                        // frame's pixel bytes (w and h are the epoch's); the copy is one memcpy
                        // of bytes the main loop may be rewriting (benign: see live_push).
                        unsafe { std::ptr::copy_nonoverlapping(f.px().add(at) as *const u8, row.as_mut_ptr(), r.w as usize) };
                        changed |= sent[at..at + r.w as usize] != *row;
                    }
                    fence(SeqCst);
                    if sh.state.load(SeqCst) != LIVE_AWAKE as u32 || sh.epoch.load(SeqCst) != epoch {
                        ok = false; // (live_set_state(LIVE_AWAKE) and a turn mark every tile again)
                        break;
                    }
                    if !changed {
                        continue;
                    }
                    let Some(png) = Png::of_tile(&tile, r.w, r.h) else {
                        for i in 0..LIVE_WORDS {
                            dirty[i].fetch_or(words[i], SeqCst);
                        }
                        ok = false;
                        break;
                    };
                    let mut at = [0u8; 8];
                    put16(&mut at, r.x as u32);
                    put16(&mut at[2..], r.y as u32);
                    put16(&mut at[4..], r.w as u32);
                    put16(&mut at[6..], r.h as u32);
                    let sent_ok = send_frame(fd, LIVE_TILE_FRAME as u8, &at, png.as_slice());
                    drop(png);
                    if !sent_ok {
                        // SAFETY: _exit ends the child at once, running none of the parent's exit handlers.
                        unsafe { libc::_exit(0) };
                    }
                    for y in 0..r.h as usize {
                        let at = (r.y as usize + y) * (w as usize) + r.x as usize;
                        sent[at..at + r.w as usize].copy_from_slice(&tile[y * r.w as usize..][..r.w as usize]);
                    }
                    out += 1;
                }
                if out != 0 || (ok && ack != 0 && ack != ack_sent) {
                    let mut p = [0u8; 16];
                    put32(&mut p, sh.seq.fetch_add(1, SeqCst));
                    put32(&mut p[4..], now_ms());
                    put32(&mut p[8..], if ok { ack } else { 0 });
                    put32(&mut p[12..], if ok { ack_ms } else { 0 });
                    if ok && ack != 0 {
                        ack_sent = ack;
                    }
                    if !send_frame(fd, LIVE_BATCH as u8, &p, &[]) {
                        break;
                    }
                    last_batch = now_ms();
                    last_sent = last_batch;
                }
                if !ok {
                    continue 'main; // (the state moved on: told at the top)
                }
            }
        }
        // The pen, after the tiles: its newest place, only when it changed, at most every LIVE_PEN_MS.
        if state != LIVE_AWAKE as u32 {
            pen = told_pen;
        }
        let mut since_pen = now_ms().wrapping_sub(last_pen);
        if pen != told_pen && since_pen >= LIVE_PEN_MS {
            if !send_pen(fd, pen) {
                break;
            }
            told_pen = pen;
            last_pen = now_ms();
            last_sent = last_pen;
            since_pen = 0;
        }
        if now_ms().wrapping_sub(last_sent) >= 15000 {
            if !send_frame(fd, LIVE_PING as u8, &[], &[]) {
                break;
            }
            last_sent = now_ms();
        }
        // Wait for a mark, the browser hanging up, or a second (the parent gone ends it here too);
        // while the pen is in range, or a place of it is still to send, only until it is due.
        let mut timeout: libc::c_int = 1000;
        if pen != told_pen {
            timeout = if since_pen < LIVE_PEN_MS { (LIVE_PEN_MS - since_pen) as libc::c_int } else { 0 };
        } else if pen != 0 {
            timeout = LIVE_PEN_MS as libc::c_int;
        }
        let mut pf = [libc::pollfd { fd: wake, events: libc::POLLIN, revents: 0 }, libc::pollfd { fd, events: libc::POLLIN, revents: 0 }];
        // SAFETY: poll reads and writes the two pollfds of the local array.
        let got = unsafe { libc::poll(pf.as_mut_ptr(), 2, timeout) };
        if got < 0 && errno() != libc::EINTR {
            break;
        }
        if pf[1].revents & (libc::POLLHUP | libc::POLLERR | libc::POLLIN) != 0 {
            let mut b = 0u8;
            // SAFETY: recv of one byte into the local b, without blocking.
            let r = unsafe { libc::recv(fd, &mut b as *mut u8 as *mut libc::c_void, 1, libc::MSG_DONTWAIT) };
            if r == 0 || (r < 0 && errno() != libc::EAGAIN && errno() != libc::EWOULDBLOCK && errno() != libc::EINTR) {
                break;
            }
        }
        if pf[0].revents & libc::POLLIN != 0 {
            let mut drain = [0u8; 64];
            // SAFETY: read into the local array, at most its length, from the wake pipe.
            while unsafe { libc::read(wake, drain.as_mut_ptr() as *mut libc::c_void, drain.len()) } > 0 {}
        }
        // SAFETY: getppid takes no arguments.
        if unsafe { libc::getppid() } != parent {
            break;
        }
        // At most about 30 batches a second: what comes meanwhile goes in the next one.
        let since = now_ms().wrapping_sub(last_batch);
        if since < 33 {
            sleep_ms((33 - since) as i32);
        }
    }
    // SAFETY: shutdown of our own socket's write side, then _exit, which ends the child at once and
    // runs none of the parent's exit handlers.
    unsafe {
        libc::shutdown(fd, libc::SHUT_WR);
        libc::_exit(0)
    }
}

fn take_viewer(fd: i32, fast: bool) -> bool {
    let Some(f) = frame() else { return false };
    let mut slot: i32 = -1;
    let v = with(|s| s.v);
    let first = if fast { LIVE_VIEWERS } else { 0 };
    for i in first..first + LIVE_VIEWERS {
        if v[i].pid <= 0 {
            slot = i as i32;
        }
    }
    if slot < 0 {
        return false;
    }
    let mut p = [0 as libc::c_int; 2];
    // SAFETY: pipe fills the two descriptors of the local array; fcntl calls are on those.
    if unsafe { libc::pipe(p.as_mut_ptr()) } != 0 {
        return false;
    }
    unsafe {
        libc::fcntl(p[0], libc::F_SETFL, libc::fcntl(p[0], libc::F_GETFL, 0) | libc::O_NONBLOCK);
        libc::fcntl(p[1], libc::F_SETFL, libc::fcntl(p[1], libc::F_GETFL, 0) | libc::O_NONBLOCK);
    }
    // SAFETY: fork; the child (pid 0) never returns here: it ends in viewer_main with _exit. It
    // allocates and uses stb, which the PORTING.md fork rules allow (the quiet thread holds no lock then).
    let pid = unsafe { libc::fork() };
    if pid == 0 {
        // The main loop's state stays as it is for the viewer: it reads the frame and its own
        // marks, and the other slot's pipe end closes with every other fd.
        fence(SeqCst);
        if !fast {
            for i in 0..LIVE_WORDS {
                f.sh().dirty[slot as usize][i].store(0xFFFFFFFF, SeqCst);
            }
        }
        viewer_main(f, slot as usize, fd, p[0]);
    }
    // SAFETY: closes the read end, which only the child uses.
    unsafe { libc::close(p[0]) };
    if pid < 0 {
        // SAFETY: closes the write end of the pipe made above.
        unsafe { libc::close(p[1]) };
        return false;
    }
    with(|s| {
        s.v[slot as usize].pid = pid;
        s.v[slot as usize].wake = p[1];
    });
    #[cfg(feature = "sim")]
    qlog!("live: (simulator) viewer pid %d", pid as i32); // (tests/live_check.sh: none outlives the app)
    true
}

/// Hands the connection fd (an authenticated GET /live/stream) to a new viewer child: true when it
/// took it (the caller closes its own copy), false when two are watching already or fork failed.
pub fn live_take_viewer(fd: i32) -> bool {
    take_viewer(fd, false)
}
pub fn live_take_pen_viewer(fd: i32) -> bool {
    take_viewer(fd, true)
}
