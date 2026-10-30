//! live.h: Live view, a computer's browser watching what Grimoire shows (docs/DESIGN.md, Live view).
//!
//! While it is on, every rect the main loop presents is copied from App.screen into a shared frame
//! (one memcpy of a rect it presents anyway; toasts and the live chip are never in it) and its
//! 64 px tiles are marked. A browser that has paired (pair.h) gets a viewer child of its own, forked
//! by the share server: it sends the tiles that changed as gray PNGs over one long HTTP response
//! (GET /live/stream), at most about 30 batches a second, and never holds up the pen: a slow
//! browser only makes the marks pile up, and the next batch carries what is newest. Two viewers at
//! most. Nothing is written to disk.
//!
//! The pen has a stream of its own (GET /live/pen, fast ink): the browser draws a quick stroke from
//! its samples the moment they come, and the tiles that carry the real one take its place as they
//! follow. Neither a PNG being made, a full tile socket nor the 33 ms between batches is ever on its
//! way: its viewer child (two at most too, a browser with both streams counting once in
//! live_viewers) makes no PNGs, keeps a small send buffer, runs at the ordinary priority where the
//! tile ones are niced, and sends what is new as soon as it wakes.
//!
//! Where the Kindle has a raw digitizer (WacomDigitizer, which Grimoire reads without a grab), the
//! pen viewer opens that node itself (EVIOCSCLOCKID: CLOCK_MONOTONIC kernel times) and sends each
//! of its frames as it comes, whatever the main loop is doing: a textured brush whose ink frames
//! take tens of milliseconds used to hold the samples back until the loop read the pens again, and
//! they reached the browser in bursts. The main loop publishes in the shared frame what the viewer
//! cannot read itself: how the raw frames map onto the screen (input_raw_pen, live_set_raw_map), the
//! ink style of each eraser and side button state with whether quick ink is allowed at all and the
//! newest pen event it has handled (live_set_capture), whether it is in the middle of UI work
//! (live_set_blocked), and the ink epoch. Without a raw digitizer, or while its place is not
//! trusted, the samples come the old way: the input hook (input_sample_hook) hands each pen event
//! to live_sample as it is read, work_pump's reads too: one record in a ring of LIVE_RING in the
//! shared frame (the sample, the ink style it draws with, its ink epoch), which is all the pen path
//! pays, with no allocation and no log line, and one byte down each pen viewer's wake pipe per read
//! of the pens (live_wake_samples). Each record has a stamp, zero while it is written and its
//! sequence after, read before and after its words are copied: a viewer a whole ring behind sees no
//! half of one, and the writer never waits for it.
//!
//! A sample is known by the kernel's time of its frame in microseconds (u32, wrapping: its
//! identity), the same for the raw node's frame the viewer reads and the main loop's own event of
//! it. The scaled pen's frames, which the main loop follows, are relayed from the raw ones about
//! 0.1 ms after them (pensync.h), well inside a frame's 2 to 5 ms: an event of the scaled pen
//! stamped t covers every raw frame stamped t or before, and no later one. That is what lets the
//! tiles' watermark (below) retire the quick ink the viewer sent from the raw node exactly.
//!
//! The editor's panel and the clock and battery corner come as layers of their own (frame 9): the
//! frame holds the page under the panel (app_flush composes that rect again without it), and the
//! corner, an overlay never in the frame, is copied as it is painted. The browser shows either or
//! neither over the page. The Kindle's own screen is unchanged.
//!
//! Two epochs. The screen's moves on at a start and a turn, the only times the tiles start again
//! from white. The ink epoch moves on whenever ink drawn before can no longer join what comes (a
//! turn, a page or a screen changed, a pinch, sleep, the pairing card: live_reset), and nothing the
//! browser drew in the one before stays; a page turn costs only the tiles that differ. The batches
//! say how far the real ink goes: app_flush hands live_presented the newest pen event a screen has
//! finished, once every pixel it dirtied is in the frame (the ring's head can be ahead of it: a pump
//! reads ahead during long work), with the ink epoch it was presented in, and a viewer reads that
//! before it takes its marks, so every pixel it vouches for is in that batch or an earlier one.
//! GET /live/time, paired too, answers {"ms": now_ms()} for the page's latency meter.
//!
//! The streams' frames are little-endian (floats IEEE 754): u32 length (of the type byte and the
//! payload), u8 type, payload:
//!   1 hello      u16 version (3), w, h, tile; u32 epoch: the screen's size, after a start or a turn.
//!                On /live/pen the epoch is the ink epoch, and a hello comes after each change of it.
//!   2 tile       u16 x, y, w, h, then the PNG of that rect (/live/stream only)
//!   3 batch end  u32 seq, u32 the Kindle's ms; u32 the newest pen sample (its identity) whose ink the
//!                tiles so far hold, u32 its ms and u32 the ink epoch it was presented in: draw what came
//!                since the last one, and drop the quick ink of every sample up to that identity once it
//!                shows. The sample is 0 for none, and all three are 0 in a batch cut short (the state or
//!                the screen's epoch moved on, or a PNG could not be made). A new watermark comes in a
//!                batch of its own when no pixel changed.
//!   4 ping       nothing: sent after 15 s without a batch, or (/live/pen) without a sample
//!   5 state      u8 LIVE_AWAKE, LIVE_ASLEEP, LIVE_PAIRING or LIVE_BYE; u8 1 when Grimoire shows its
//!                colors inverted (white on black: the frame never is), else 0; u8 the front light's
//!                warmth, 0..LIGHT_MAX, or LIVE_NO_WARMTH (no warm light, or not known). First thing
//!                on every stream, then whenever any of them changes (/live/pen: the state only).
//!                Any but LIVE_AWAKE takes the pen and the quick ink away, and nothing of the screen
//!                or the pen is sent while it lasts: the pairing code and the Kindle's own passcode
//!                are never streamed.
//!   6 pen        u16 x, y in the screen's pixels (the tiles'), u8 LIVE_PEN_AWAY, LIVE_PEN_HOVER or
//!                LIVE_PEN_TOUCH: where the Kindle's pen is, for the browser to draw over the picture
//!                (/live/stream only; the page follows it only while it has no /live/pen). After the
//!                tiles of the same batch, at most every LIVE_PEN_MS and only when it changed: the
//!                newest place, so a browser that falls behind skips the ones between. Only while the
//!                state is LIVE_AWAKE; a hello or any other state takes the pen away (the browser
//!                forgets it), and the next pen frame says where it is again.
//!   7 samples    u32 the ink epoch; u32 1 when the stroke before cannot be joined (after a hello, or
//!                an overrun of the ring), else 0; then n samples (n = 0 right after a hello, else at
//!                most LIVE_PEN_BATCH), oldest first, LIVE_SAMPLE_BYTES each: u32 identity (the
//!                kernel's time of the frame in microseconds, wrapping; never 0), u32 ms (the same
//!                time in the Kindle's ms, for the latency meter), u16 x, y (the screen's pixels), u16
//!                pressure (0..65535), i8 tilt x, y (degrees, the screen's frame), u8 LIVE_PEN_AWAY,
//!                LIVE_PEN_HOVER or LIVE_PEN_TOUCH, u8 flags (LIVE_ERASER, LIVE_SIDE, LIVE_SIDE2, LIVE_DOWN,
//!                LIVE_GAP, LIVE_DRAW), u16 0. LIVE_GAP breaks the stroke, and only LIVE_DOWN starts
//!                one. A stream starts at the newest sample, without LIVE_DOWN, so it never draws a
//!                stroke it saw only the end of.
//!   8 style      LIVE_STYLE_BYTES (live_style_payload): what the samples after it draw, before the
//!                first and whenever it changes. u8 enabled (0: no quick ink, as on a menu), u8 the
//!                tool (0..9), u8 white (white ink or the precise eraser), u8 whole (the stroke eraser,
//!                whose samples cannot say which strokes it takes: only its pen shows); f32 width min
//!                and max (screen pixels, at the zoom), f32 pressure floor, ceiling and gamma, f32
//!                opacity min and max (width and opacity follow clamp((p - floor) / (ceiling - floor),
//!                0, 1) ^ gamma); i32 x, y, w, h of the page on the screen (the ink's clip), then of
//!                the panel (no ink over it). The quick ink is round: nibs, grain, smoothing, tilt and
//!                the paper white ink brings back come with the tiles.
//!   9 layer      u8 LIVE_LAYER_PANEL (the editor's panel) or LIVE_LAYER_STATUS (the clock and battery
//!                corner), u8 0, u16 x, y, w, h, then the PNG of that rect (/live/stream only); w = 0
//!                and no PNG when the layer is not on the screen. In the batch it changed in, and
//!                again after every hello. The tiles under it hold what is under it.

use crate::prelude::*;
use std::cell::RefCell;
use std::sync::atomic::{fence, AtomicU32, Ordering::SeqCst};


pub const LIVE_TILE: i32 = 64;
/// tile marks: 2048 tiles, more than either Scribe's screen has
pub const LIVE_WORDS: usize = 64;
/// of each kind: tile viewers (/live/stream), and pen viewers (/live/pen)
pub const LIVE_VIEWERS: usize = 2;
/// no viewer for this long: Live view turns itself off
pub const LIVE_IDLE_OFF_MS: u32 = 10 * 60 * 1000;
/// the state frame's warmth when there is none to follow
pub const LIVE_NO_WARMTH: i32 = 255;
/// pen frames (the tile stream's cursor) at most this often; the samples never wait for it
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
pub const LIVE_LAYER: i32 = 9;
/// the streams' version, in each hello
pub const LIVE_VERSION: u32 = 3;
/// the layers (frame 9): the editor's panel, and the clock and battery corner
pub const LIVE_LAYERS: usize = 2;
pub const LIVE_LAYER_PANEL: usize = 0;
pub const LIVE_LAYER_STATUS: usize = 1;
pub const LIVE_PEN_AWAY: i32 = 0;
pub const LIVE_PEN_HOVER: i32 = 1;
pub const LIVE_PEN_TOUCH: i32 = 2;

/// samples the shared frame keeps: a pen viewer that falls further behind starts again from the
/// newest of them (frame 7's reset)
pub const LIVE_RING: usize = 512;
pub const LIVE_SAMPLE_BYTES: usize = 20;
pub const LIVE_STYLE_BYTES: usize = 64;
/// samples in one frame at most: a backlog goes as a few frames of 8 + 64 * 20 bytes, each one
/// TCP segment on Wi-Fi, not one per sample
pub const LIVE_PEN_BATCH: usize = 64;
// A sample's flags: the eraser end, the side button held, the stroke's first sample (pen down), the
// first after samples the kernel dropped (InEvent.gap), and quick ink allowed for it (its style's
// enabled, and not a pen-down on UI).
pub const LIVE_ERASER: u8 = 1;
pub const LIVE_SIDE: u8 = 2;
pub const LIVE_DOWN: u8 = 4;
pub const LIVE_GAP: u8 = 8;
pub const LIVE_DRAW: u8 = 16;
/// Second pen button, independent of the first side button.
pub const LIVE_SIDE2: u8 = 32;
/// What a pen sample draws, as the browser imitates it (frame 8); editor_live_style makes it, all
/// zero (enabled 0) where a sample draws nothing.
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
// is a forked child that reads the frame, never writes it (but for its own wake flag), and owns
// one browser's connection.

// (the C's _Static_assert: the tile marks must work across fork)
const _: () = assert!(cfg!(target_has_atomic = "32"), "the tile marks must work across fork");

// The frame both sides map. w and h change only on a turn, written before epoch moves on; px holds
// w * h bytes (the same in either orientation), rows w apart, right after this header (the C's
// flexible array member), then a buffer of as many bytes for each layer. Each tile viewer slot has
// marks of its own: a viewer takes its marks as it sends, which must leave the other's. pen is
// live_pen_word's. ack_* is live_presented's watermark, written between two steps of ack_version
// (odd while it changes) so a viewer reads it whole. woken: a slot's wake byte is on its way (wake):
// the child clears it once it has drained its pipe and before it looks at the frame again, so a wake
// while it is set needs no byte of its own. capture, raw_map and each layer are written the same
// way (Seq), and blocked is live_set_blocked's. ring_only: GRIMOIRE_LIVE_PEN_SOURCE=ring (tests and
// measurements: the pen viewers take the samples from the ring even where a raw digitizer is).
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
    ink_epoch: AtomicU32,
    sample_head: AtomicU32,
    ack_version: AtomicU32,
    ack_sample: AtomicU32,
    ack_ms: AtomicU32,
    ack_ink: AtomicU32,
    woken: [AtomicU32; LIVE_VIEWERS * 2],
    blocked: AtomicU32,
    ring_only: AtomicU32,
    capture: Seq<CAPTURE_WORDS>,
    raw_map: Seq<MAP_WORDS>,
    raw_epoch: AtomicU32,
    layers: [Seq<5>; LIVE_LAYERS],
    samples: [SampleSlot; LIVE_RING],
    dirty: [[AtomicU32; LIVE_WORDS]; LIVE_VIEWERS],
}
// A ring record: stamp, then the 88 bytes as 22 words (word i is bytes 4i..4i+3, little-endian):
// the sample as frame 7 sends it (0..19), its style as frame 8 does (20..83), its ink epoch (84).
#[repr(C)]
struct SampleSlot {
    stamp: AtomicU32,
    words: [AtomicU32; 22],
}
/// Words the main loop writes whole and a viewer reads whole: version is odd while they change, so
/// a reader that sees it odd, or changed by the end of its copy, has no snapshot and tries again
/// later. The writer never waits for a reader.
#[repr(C)]
struct Seq<const N: usize> {
    version: AtomicU32,
    words: [AtomicU32; N],
}
impl<const N: usize> Seq<N> {
    /// Bytes as words (little-endian; a short last word is zero filled).
    fn write(&self, bytes: &[u8]) {
        self.version.fetch_add(1, SeqCst);
        for (i, word) in self.words.iter().enumerate() {
            let mut b = [0u8; 4];
            for (k, v) in b.iter_mut().enumerate() {
                *v = bytes.get(i * 4 + k).copied().unwrap_or(0);
            }
            word.store(u32::from_le_bytes(b), SeqCst);
        }
        self.version.fetch_add(1, SeqCst);
    }
    /// The words into out (as many bytes as it holds); the version they were read at, None when
    /// they were changing.
    fn read(&self, out: &mut [u8]) -> Option<u32> {
        let v = self.version.load(SeqCst);
        if v & 1 != 0 {
            return None;
        }
        for (i, word) in self.words.iter().enumerate() {
            let b = word.load(SeqCst).to_le_bytes();
            for (k, v) in b.iter().enumerate() {
                if let Some(o) = out.get_mut(i * 4 + k) {
                    *o = *v;
                }
            }
        }
        if self.version.load(SeqCst) == v {
            Some(v)
        } else {
            None
        }
    }
}
/// live_set_capture's words: u8 ok, u8 editor, u16 0, u32 handled, then the eight styles
/// ([eraser][Button + 2 * Button2], LIVE_STYLE_BYTES each).
const CAPTURE_BYTES: usize = 8 + 8 * LIVE_STYLE_BYTES;
const CAPTURE_WORDS: usize = CAPTURE_BYTES / 4;
/// live_set_raw_map's words (raw_map_bytes)
const MAP_BYTES: usize = 228;
const MAP_WORDS: usize = MAP_BYTES / 4;
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
    /// w * h: the pixels, and each layer's buffer
    fn px_len(&self) -> usize {
        (self.size - SHARED_SIZE) / (1 + LIVE_LAYERS)
    }
    /// The first byte of layer i's buffer (px_len bytes), read and written in short copies as px.
    fn layer_px(&self, i: usize) -> *mut u8 {
        // SAFETY: the mapping holds px_len bytes of pixels and then px_len for each layer.
        unsafe { self.px().add(self.px_len() * (1 + i)) }
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
    /// the last identity handed out (ident)
    last_ident: u32,
    /// what live_set_capture and live_set_raw_map wrote last
    capture_sent: [u8; CAPTURE_BYTES],
    map_sent: [u8; MAP_BYTES],
    /// slots 0..LIVE_VIEWERS: the tile viewers; then the pen viewers
    v: [Viewer; LIVE_VIEWERS * 2],
    /// a sample was stored since the pen viewers were last woken (live_wake_samples)
    sample_wake: bool,
    /// when the last viewer left (or Live view started)
    last_viewer: u32,
}

thread_local! {
    static S: RefCell<Live> = const { RefCell::new(Live { sh: None, last_ident: 0, capture_sent: [0; CAPTURE_BYTES], map_sent: [0; MAP_BYTES], v: [Viewer { pid: 0, wake: 0 }; LIVE_VIEWERS * 2], sample_wake: false, last_viewer: 0 }) };
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

/// The parts of run r (one row of tiles, as live_runs cuts it) that changed: snap holds the run's
/// pixels (r.h rows of r.w), sent the w-wide screen as the browser has it. Each part is a stretch of
/// whole tiles of the run whose pixels differ, so a wide redraw that changed one tile sends that
/// tile alone. At most out.len() parts: the last then reaches over the rest that changed.
pub fn live_changed_parts(sent: &[u8], w: i32, snap: &[u8], r: Rect, out: &mut [Rect]) -> usize {
    let (rw, rx) = (r.w.max(0) as usize, r.x as usize);
    let mut n = 0usize;
    let mut open = false;
    let mut cx = 0usize;
    while cx < rw && !out.is_empty() {
        let cw = (LIVE_TILE as usize).min(rw - cx);
        let changed = (0..r.h.max(0) as usize).any(|y| {
            let at = (r.y as usize + y) * w as usize + rx + cx;
            sent[at..at + cw] != snap[y * rw + cx..][..cw]
        });
        if changed {
            if open || n == out.len() {
                out[n - 1].w = r.x + (cx + cw) as i32 - out[n - 1].x;
            } else {
                out[n] = Rect { x: r.x + cx as i32, y: r.y, w: cw as i32, h: r.h };
                n += 1;
            }
            open = true;
        } else {
            open = false;
        }
        cx += cw;
    }
    n
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

// A byte down the wake pipe of each viewer in slots that has none on its way yet (Shared.woken):
// two calls in one flush (live_push, then live_presented) cost one write.
fn wake(slots: std::ops::Range<usize>) {
    let Some(f) = frame() else { return };
    let v = with(|s| s.v);
    for i in slots {
        if v[i].pid > 0 && f.sh().woken[i].swap(1, SeqCst) == 0 {
            // SAFETY: write(2) of one byte from a static to the wake pipe's write end, which this
            // process owns for as long as the viewer's pid is set.
            let r = unsafe { libc::write(v[i].wake, b"x".as_ptr() as *const libc::c_void, 1) };
            if r < 0 { /* full: it is awake already */ }
        }
    }
}
// The tile viewers: the screen, its watermark, the pen's place, the colors and the light.
fn wake_tiles() {
    wake(0..LIVE_VIEWERS);
}
// The pen viewers: samples and the ink epoch.
fn wake_pens() {
    wake(LIVE_VIEWERS..LIVE_VIEWERS * 2);
}
// Both: the state, and a turn.
fn wake_all() {
    wake(0..LIVE_VIEWERS * 2);
}

fn mark_all(f: &Frame) {
    let sh = f.sh();
    for v in 0..LIVE_VIEWERS {
        for i in 0..LIVE_WORDS {
            sh.dirty[v][i].store(0xFFFFFFFF, SeqCst);
        }
    }
}

// live_presented's watermark, changed whole (Shared.ack_version).
fn set_ack(sh: &Shared, sample: u32, ms: u32, ink: u32) {
    sh.ack_version.fetch_add(1, SeqCst);
    sh.ack_sample.store(sample, SeqCst);
    sh.ack_ms.store(ms, SeqCst);
    sh.ack_ink.store(ink, SeqCst);
    sh.ack_version.fetch_add(1, SeqCst);
}

// The ink epoch moves on, never to 0: a batch's 0 says it vouches for nothing.
fn next_ink_epoch(sh: &Shared) {
    let mut e = sh.ink_epoch.load(SeqCst).wrapping_add(1);
    if e == 0 {
        e = 1;
    }
    sh.ink_epoch.store(e, SeqCst);
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
    let size = SHARED_SIZE + (w as usize) * (h as usize) * (1 + LIVE_LAYERS);
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
    sh.ink_epoch.store(1, SeqCst); // (nothing presented in it yet: ack_ink stays 0 until app_flush)
    let ring_only = std::env::var_os("GRIMOIRE_LIVE_PEN_SOURCE").is_some_and(|v| v.as_encoded_bytes() == b"ring");
    sh.ring_only.store(ring_only as u32, SeqCst);
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
    // SAFETY: waitpid on a child this process forked (the pid is set only by take_viewer), no status wanted.
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
        s.capture_sent = [0; CAPTURE_BYTES];
        s.map_sent = [0; MAP_BYTES];
    });
}

/// A rect of the screen presented: copied into the frame, its tiles marked, the tile viewers woken.
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
    wake_tiles();
}

/// The screen turned (w x h now): a new hello on both streams, every tile again, the pen away until
/// it is told again, and a new ink epoch (the ink before is on the screen before the turn).
pub fn live_resize(w: i32, h: i32) {
    let Some(f) = frame() else { return };
    if (w as usize).wrapping_mul(h as usize).wrapping_mul(1 + LIVE_LAYERS).wrapping_add(SHARED_SIZE) != f.size {
        return;
    }
    let sh = f.sh();
    sh.w.store(w as u32, SeqCst);
    sh.h.store(h as u32, SeqCst);
    sh.pen.store(0, SeqCst); // (its place was on the screen before the turn)
    set_ack(sh, 0, 0, sh.ack_ink.load(SeqCst)); // (the turned screen's first flush says it again)
    // SAFETY: px() is the start of the w * h pixel bytes (the size was checked against the mapping's).
    unsafe { std::ptr::write_bytes(f.px(), 255, (w as usize) * (h as usize)) };
    for layer in sh.layers.iter() {
        layer.write(&[0u8; 20]); // (on the screen before the turn: the turned one's flush tells them again)
    }
    next_ink_epoch(sh);
    sh.epoch.fetch_add(1, SeqCst);
    mark_all(&f);
    wake_all();
}

/// The ink drawn so far can no longer be joined by what comes: a page or a screen changed, a pinch
/// began, the state changed (live_set_state). The ink epoch moves on, which the pen viewers say with
/// a hello and the browsers clear their quick ink for, and the watermark goes until the next
/// present. The screen's epoch stays: the tiles that differ come as they always do.
pub fn live_reset() {
    let Some(f) = frame() else { return };
    let sh = f.sh();
    set_ack(sh, 0, 0, sh.ack_ink.load(SeqCst));
    next_ink_epoch(sh);
    wake_pens();
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
        sh.raw_epoch.fetch_add(1, SeqCst); // retire raw queues even if a worker misses a brief sleep
        live_reset(); // (no ink joins across a sleep or the pairing card)
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
/// events have them (after the turn). One atomic store; a write to the tile viewers' wake pipes only
/// when it comes into range. The caller never tells it while the Kindle sleeps (live_set_state
/// clears it). (The tile stream's cursor: the pen viewers send the samples themselves.)
pub fn live_pen(state: i32, x: f32, y: f32) {
    let Some(f) = frame() else { return };
    let sh = f.sh();
    let word = live_pen_word(state, x, y, sh.w.load(SeqCst) as i32, sh.h.load(SeqCst) as i32);
    // A sample is this store and nothing more. Only the pen coming into range wakes the viewers:
    // while it is in range they look at it again every LIVE_PEN_MS themselves (viewer_main).
    if sh.pen.swap(word, SeqCst) == 0 && word != 0 {
        wake_tiles();
    }
}

/// Grimoire's colors inverted on the Kindle or not (settings.invert, applied as it presents: the
/// frame holds them as drawn): the viewers tell their browsers, which follow it unless told not to.
pub fn live_set_invert(on: bool) {
    let Some(f) = frame() else { return };
    if f.sh().invert.swap(on as u32, SeqCst) == on as u32 {
        return;
    }
    wake_tiles(); // (the pen viewers send the state alone)
}

/// The Kindle's front light warmth (0..LIGHT_MAX; -1: no warm light, or not known): the browsers
/// tint the picture's light with it, unless told otherwise.
pub fn live_set_warmth(level: i32) {
    let v: u32 = if level < 0 { LIVE_NO_WARMTH as u32 } else { clampi(level, 0, LIVE_NO_WARMTH - 1) as u32 };
    let Some(f) = frame() else { return };
    if f.sh().warmth.swap(v, SeqCst) == v {
        return;
    }
    wake_tiles();
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

/// A style as frame 8 carries it (and the ring keeps it with each sample): out holds at least
/// LIVE_STYLE_BYTES.
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

/// CLOCK_MONOTONIC in microseconds (the clock of the pens' kernel times, EVIOCSCLOCKID).
fn mono_us() -> u64 {
    let mut ts = libc::timespec { tv_sec: 0, tv_nsec: 0 };
    // SAFETY: clock_gettime writes one timespec into our local.
    unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut ts) };
    (ts.tv_sec as u64) * 1_000_000 + (ts.tv_nsec as u64) / 1000
}

/// A pen event's identity (frame 7): the kernel's time of its frame in microseconds (InEvent.k_us),
/// or, where nobody says (the simulator's scripted events), when it is captured, kept after the last
/// one handed out so that no two events share one. Never 0.
fn ident(ev: &InEvent) -> u32 {
    let last = with(|s| s.last_ident);
    let mut id = ev.k_us;
    if id == 0 {
        id = mono_us() as u32;
        if (id.wrapping_sub(last) as i32) <= 0 {
            id = last.wrapping_add(1);
        }
    }
    if id == 0 {
        id = 1; // zero means an event that was not captured
    }
    with(|s| s.last_ident = id);
    id
}

/// A pen event as it is read (input_sample_hook, through app.c's live_capture), with what it draws:
/// stored in the ring for the pen viewers. Its identity, or 0 when nothing was stored (Live view is
/// off, or not awake). No allocation, no log line, no system call: the wake is
/// live_wake_samples's, once per read of the pens.
pub fn live_sample(ev: &InEvent, style: &LiveStyle) -> u32 {
    let Some(f) = frame() else { return 0 };
    let sh = f.sh();
    if sh.state.load(SeqCst) != LIVE_AWAKE as u32 {
        return 0;
    }
    let mut seq = sh.sample_head.load(SeqCst).wrapping_add(1);
    if seq == 0 {
        seq = 1; // (the ring's own count: a stamp of zero is a record being written)
    }
    let id = ident(ev);
    let mut record = [0u8; 88];
    put32(&mut record, id);
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
        | (if ev.side_button2 { LIVE_SIDE2 } else { 0 })
        | (if ev.type_ == InType::PenDown { LIVE_DOWN } else { 0 })
        | (if ev.gap { LIVE_GAP } else { 0 })
        | (if style.enabled != 0 { LIVE_DRAW } else { 0 });
    live_style_payload(&mut record[20..20 + LIVE_STYLE_BYTES], style);
    put32(&mut record[84..], sh.ink_epoch.load(SeqCst));
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
    id
}

/// What quick ink may draw, as app.rs's LiveCap has it, for the pen viewers that read the raw
/// digitizer themselves: ok (Live view on, the app awake and holding its input), whether the editor
/// is up (quick ink is the editor's only), the newest pen event the main loop has handled (its
/// identity: App.live_processed), and the style of each eraser ([0] off, [1] on) and the two independent
/// button bits, as editor_live_style makes them.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LiveCapture {
    pub ok: bool,
    pub editor: bool,
    pub handled: u32,
    pub styles: [[LiveStyle; 4]; 2],
}

fn capture_bytes(c: &LiveCapture) -> [u8; CAPTURE_BYTES] {
    let mut b = [0u8; CAPTURE_BYTES];
    b[0] = c.ok as u8;
    b[1] = c.editor as u8;
    put32(&mut b[4..], c.handled);
    for (e, row) in c.styles.iter().enumerate() {
        for (side, style) in row.iter().enumerate() {
            live_style_payload(&mut b[8 + (e * 4 + side) * LIVE_STYLE_BYTES..], style);
        }
    }
    b
}

/// The main loop's capture state (after every event and tick, and wherever ok changes): written
/// only when it changed, so a viewer's snapshot stays good meanwhile.
pub fn live_set_capture(c: &LiveCapture) {
    let Some(f) = frame() else { return };
    let b = capture_bytes(c);
    if with(|s| s.capture_sent == b) {
        return;
    }
    f.sh().capture.write(&b);
    with(|s| s.capture_sent = b);
}

/// The main loop is in UI work that a pen sample must not preview through (app.rs's
/// LIVE_CAPTURE_BLOCKED): set before a screen handles an event that is no stroke's, cleared after.
pub fn live_set_blocked(on: bool) {
    let Some(f) = frame() else { return };
    f.sh().blocked.store(on as u32, SeqCst);
}

/// Where the raw digitizer is and how its frames map onto the screen (input_raw_pen fills it): its
/// node (st_rdev, 0 for any; the path to open), screen x = m[0] rx plus m[1] ry plus m[2] and y =
/// m[3] rx plus m[4] ry plus m[5] for a raw place cut to rmin..rmax (the result cut to the screen),
/// pressure pmin..pmax as 0..1, each tilt axis's range and resolution (tilt_deg), and the tilt into
/// the screen's frame: x' = tm[0] x plus tm[1] y, y' = tm[2] x plus tm[3] y.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LiveRawMap {
    pub dev: u64,
    pub path: FixedStr<128>,
    pub m: [f32; 6],
    pub rmin: [i32; 2],
    pub rmax: [i32; 2],
    pub pmin: i32,
    pub pmax: i32,
    pub tilt_min: [i32; 2],
    pub tilt_max: [i32; 2],
    pub tilt_res: [i32; 2],
    pub tm: [f32; 4],
}

fn raw_map_bytes(m: Option<&LiveRawMap>) -> [u8; MAP_BYTES] {
    let mut b = [0u8; MAP_BYTES];
    let Some(m) = m else { return b };
    b[0] = 1;
    put32(&mut b[4..], m.dev as u32);
    put32(&mut b[8..], (m.dev >> 32) as u32);
    let path = m.path.as_bytes();
    let n = path.len().min(127);
    b[12..12 + n].copy_from_slice(&path[..n]);
    for (i, v) in m.m.iter().enumerate() {
        put32(&mut b[140 + i * 4..], v.to_bits());
    }
    let ints = [m.rmin[0], m.rmin[1], m.rmax[0], m.rmax[1], m.pmin, m.pmax, m.tilt_min[0], m.tilt_max[0], m.tilt_res[0], m.tilt_min[1], m.tilt_max[1], m.tilt_res[1]];
    for (i, v) in ints.iter().enumerate() {
        put32(&mut b[164 + i * 4..], *v as u32);
    }
    for (i, v) in m.tm.iter().enumerate() {
        put32(&mut b[212 + i * 4..], v.to_bits());
    }
    b
}

/// raw_map_bytes read back; None for none.
fn raw_map_read(b: &[u8; MAP_BYTES]) -> Option<LiveRawMap> {
    if b[0] == 0 {
        return None;
    }
    let mut m = LiveRawMap { dev: get32(&b[4..]) as u64 | (get32(&b[8..]) as u64) << 32, ..Default::default() };
    let end = b[12..140].iter().position(|&c| c == 0).unwrap_or(128);
    m.path.set_bytes(&b[12..12 + end]);
    for i in 0..6 {
        m.m[i] = f32::from_bits(get32(&b[140 + i * 4..]));
    }
    let int = |i: usize| get32(&b[164 + i * 4..]) as i32;
    m.rmin = [int(0), int(1)];
    m.rmax = [int(2), int(3)];
    m.pmin = int(4);
    m.pmax = int(5);
    m.tilt_min = [int(6), int(9)];
    m.tilt_max = [int(7), int(10)];
    m.tilt_res = [int(8), int(11)];
    for i in 0..4 {
        m.tm[i] = f32::from_bits(get32(&b[212 + i * 4..]));
    }
    Some(m)
}

/// The raw digitizer's map as the main loop knows it now (app_tick, and as Live view starts), or
/// None while there is none (no raw digitizer, its place not trusted, the pen not Grimoire's): the
/// pen viewers then take the samples from the ring. Written only when it changed.
pub fn live_set_raw_map(m: Option<&LiveRawMap>) {
    let Some(f) = frame() else { return };
    let b = raw_map_bytes(m);
    if with(|s| s.map_sent == b) {
        return;
    }
    f.sh().raw_map.write(&b);
    with(|s| s.map_sent = b);
    wake_pens(); // (a pen viewer opens the node, or lets it go)
}

/// Layer i (LIVE_LAYER_PANEL, LIVE_LAYER_STATUS) is r of screen now, or not on the screen (r empty):
/// its pixels copied into the layer's buffer, the tile viewers woken. An empty layer told again
/// changes nothing.
pub fn live_layer(i: usize, screen: &Canvas, r: Rect) {
    let Some(f) = frame() else { return };
    let sh = f.sh();
    let w = sh.w.load(SeqCst) as i32;
    let h = sh.h.load(SeqCst) as i32;
    if screen.w != w || screen.h != h || i >= LIVE_LAYERS {
        return; // (a turn not told yet: live_resize comes, and the flush after it tells the layers)
    }
    let r = rect_intersect(r, Rect { x: 0, y: 0, w, h });
    let layer = &sh.layers[i];
    let mut was = [0u8; 20];
    let empty = rect_is_empty(r);
    if empty && layer.read(&mut was).is_some() && get32(&was[8..]) == 0 {
        return;
    }
    let mut words = [0u8; 20];
    if !empty {
        put32(&mut words, r.x as u32);
        put32(&mut words[4..], r.y as u32);
        put32(&mut words[8..], r.w as u32);
        put32(&mut words[12..], r.h as u32);
        // The same rect with the same pixels (every flush that reaches the panel tells it): nothing.
        if layer.read(&mut was).is_some() && was[..16] == words[..16] {
            let same = (0..r.h).all(|y| {
                let from = &screen.px[((r.y + y) as usize) * (screen.stride as usize) + r.x as usize..][..r.w as usize];
                // SAFETY: row y of the packed copy is inside the layer's buffer (r is cut to the
                // screen); only this process writes it, so it holds what was copied last.
                let kept = unsafe { std::slice::from_raw_parts(f.layer_px(i).add((y as usize) * (r.w as usize)) as *const u8, r.w as usize) };
                from == kept
            });
            if same {
                return;
            }
        }
    }
    layer.version.fetch_add(1, SeqCst);
    for y in 0..if empty { 0 } else { r.h } {
        let from = &screen.px[((r.y + y) as usize) * (screen.stride as usize) + r.x as usize..][..r.w as usize];
        // SAFETY: row y of the packed r.w x r.h copy is inside the layer's px_len (= w * h) bytes,
        // since r is cut to the screen; `from` is the caller's canvas, a different allocation.
        unsafe { std::ptr::copy_nonoverlapping(from.as_ptr(), f.layer_px(i).add((y as usize) * (r.w as usize)), r.w as usize) };
    }
    for (k, word) in layer.words.iter().enumerate() {
        word.store(get32(&words[k * 4..]), SeqCst);
    }
    layer.version.fetch_add(1, SeqCst);
    wake_tiles();
}

// ---- the raw digitizer, as a pen viewer reads it ----

// linux/input.h's numbers the raw pen uses (input_evdev.rs has them all).
const EV_SYN: u16 = 0x00;
const EV_KEY: u16 = 0x01;
const EV_ABS: u16 = 0x03;
const SYN_REPORT: u16 = 0;
const SYN_DROPPED: u16 = 3;
const BTN_TOOL_PEN: u16 = 0x140;
const BTN_TOOL_RUBBER: u16 = 0x141;
const BTN_TOUCH: u16 = 0x14a;
const BTN_STYLUS: u16 = 0x14b;
const BTN_STYLUS2: u16 = 0x14c;
const ABS_X: u16 = 0x00;
const ABS_Y: u16 = 0x01;
const ABS_PRESSURE: u16 = 0x18;
const ABS_TILT_X: u16 = 0x1a;
const ABS_TILT_Y: u16 = 0x1b;

/// struct input_event as the kernel writes it (input_evdev.rs's InputEvent): two unsigned longs of
/// time, type, code and value. The simulator's stand-in node takes the host's layout of the same.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct LiveInputEvent {
    pub sec: libc::c_ulong,
    pub usec: libc::c_ulong,
    pub type_: u16,
    pub code: u16,
    pub value: i32,
}

/// The raw digitizer's newest values, as a pen viewer reads its events (live_raw_event).
#[derive(Clone, Copy, Debug, Default)]
pub struct LiveRawPen {
    pub x: i32,
    pub y: i32,
    pub p: i32,
    pub tx: i32,
    pub ty: i32,
    pub tool_pen: bool,
    pub tool_rubber: bool,
    pub touch: bool,
    pub side: bool,
    pub side2: bool,
    /// x and y read since the pen came near
    pub placed: bool,
    /// the last frame sent had contact, was near
    pub was_touch: bool,
    pub was_near: bool,
    /// SYN_DROPPED: what comes up to the next SYN_REPORT is half a frame; the frame after it is a gap
    pub dropping: bool,
    pub gap: bool,
}

/// One raw frame (its SYN_REPORT), as a pen viewer sends it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LiveRawFrame {
    /// its identity (microseconds) and its ms (in now_ms()'s clock)
    pub us: u32,
    pub ms: u32,
    /// LIVE_PEN_AWAY, LIVE_PEN_HOVER or LIVE_PEN_TOUCH
    pub state: i32,
    pub down: bool,
    pub gap: bool,
    pub eraser: bool,
    pub side: bool,
    pub side2: bool,
    pub x: i32,
    pub y: i32,
    pub p: i32,
    pub tx: i32,
    pub ty: i32,
}

/// One event of the raw digitizer into st: true at the end of a frame to send (out). The pen is near
/// while either tool's button is down, touches while BTN_TOUCH is or its pressure is over pmin (the
/// raw digitizer's contact, as raw_hover reads it in input_evdev.rs), and is sent once placed; its
/// going away is one frame of LIVE_PEN_AWAY. After SYN_DROPPED the half frame is not sent, and the
/// next one is a gap (the caller asks the node what it holds meanwhile, where it can).
pub fn live_raw_event(st: &mut LiveRawPen, pmin: i32, ev: &LiveInputEvent, out: &mut LiveRawFrame) -> bool {
    match (ev.type_, ev.code) {
        (EV_SYN, SYN_DROPPED) => {
            st.dropping = true;
            false
        }
        (EV_SYN, SYN_REPORT) => {
            if st.dropping {
                st.dropping = false;
                st.gap = true;
                return false;
            }
            let near = st.tool_pen || st.tool_rubber;
            let us = (ev.sec as u64).wrapping_mul(1_000_000).wrapping_add(ev.usec as u64);
            let mut f = LiveRawFrame {
                us: if us as u32 == 0 { 1 } else { us as u32 },
                ms: (us / 1000) as u32, // (as now_ms() truncates: sec * 1000 + usec / 1000)
                eraser: st.tool_rubber,
                side: st.side,
                side2: st.side2,
                x: st.x,
                y: st.y,
                p: st.p,
                tx: st.tx,
                ty: st.ty,
                ..Default::default()
            };
            if !near {
                st.placed = false;
                let was = st.was_near;
                st.was_near = false;
                st.was_touch = false;
                if !was {
                    return false;
                }
                f.state = LIVE_PEN_AWAY;
                *out = f;
                return true;
            }
            if !st.placed {
                return false;
            }
            let touching = st.touch || st.p > pmin;
            f.state = if touching { LIVE_PEN_TOUCH } else { LIVE_PEN_HOVER };
            f.down = touching && !st.was_touch;
            f.gap = std::mem::take(&mut st.gap);
            st.was_touch = touching;
            st.was_near = true;
            *out = f;
            true
        }
        _ if st.dropping => false,
        (EV_KEY, BTN_TOOL_PEN) => {
            st.tool_pen = ev.value != 0;
            false
        }
        (EV_KEY, BTN_TOOL_RUBBER) => {
            st.tool_rubber = ev.value != 0;
            false
        }
        (EV_KEY, BTN_TOUCH) => {
            st.touch = ev.value != 0;
            false
        }
        (EV_KEY, BTN_STYLUS) => {
            st.side = ev.value != 0;
            false
        }
        (EV_KEY, BTN_STYLUS2) => {
            st.side2 = ev.value != 0;
            false
        }
        (EV_ABS, ABS_X) => {
            st.x = ev.value;
            st.placed = true;
            false
        }
        (EV_ABS, ABS_Y) => {
            st.y = ev.value;
            st.placed = true;
            false
        }
        (EV_ABS, ABS_PRESSURE) => {
            st.p = ev.value;
            false
        }
        (EV_ABS, ABS_TILT_X) => {
            st.tx = ev.value;
            false
        }
        (EV_ABS, ABS_TILT_Y) => {
            st.ty = ev.value;
            false
        }
        _ => false,
    }
}

// Tilt in degrees, as input_evdev.rs's tilt_deg reads an axis: units per radian where the axis says
// its resolution, small ranges as degrees, larger ones scaled to +-90.
fn raw_tilt_deg(v: i32, min: i32, max: i32, res: i32) -> f32 {
    if max == min {
        return 0.0;
    }
    if res > 0 {
        return clampf(v as f32 / res as f32 * 57.29578, -90.0, 90.0);
    }
    let m = if min.wrapping_abs() > max.wrapping_abs() { min.wrapping_abs() } else { max.wrapping_abs() };
    if m <= 127 {
        return clampf(v as f32, -90.0, 90.0);
    }
    clampf(v as f32 * 90.0 / m as f32, -90.0, 90.0)
}

/// A raw frame as frame 7's sample (out: LIVE_SAMPLE_BYTES), placed on the w x h screen by m. Its
/// flags but LIVE_DRAW, which the caller adds.
pub fn live_raw_sample(m: &LiveRawMap, w: i32, h: i32, f: &LiveRawFrame, out: &mut [u8]) {
    let rx = (f.x.clamp(m.rmin[0].min(m.rmax[0]), m.rmax[0].max(m.rmin[0]))) as f32;
    let ry = (f.y.clamp(m.rmin[1].min(m.rmax[1]), m.rmax[1].max(m.rmin[1]))) as f32;
    let sx = m.m[0] * rx + m.m[1] * ry + m.m[2];
    let sy = m.m[3] * rx + m.m[4] * ry + m.m[5];
    let span = m.pmax.wrapping_sub(m.pmin);
    let p = if f.state != LIVE_PEN_TOUCH || span <= 0 { 0.0 } else { clampf(f.p.wrapping_sub(m.pmin) as f32 / span as f32, 0.0, 1.0) };
    let tx = raw_tilt_deg(f.tx, m.tilt_min[0], m.tilt_max[0], m.tilt_res[0]);
    let ty = raw_tilt_deg(f.ty, m.tilt_min[1], m.tilt_max[1], m.tilt_res[1]);
    put32(out, f.us);
    put32(&mut out[4..], f.ms);
    put16(&mut out[8..], pen_px(sx, w));
    put16(&mut out[10..], pen_px(sy, h));
    put16(&mut out[12..], pen_px(p * 65535.0, 65536));
    out[14] = clampf(m.tm[0] * tx + m.tm[1] * ty, -90.0, 90.0) as i8 as u8;
    out[15] = clampf(m.tm[2] * tx + m.tm[3] * ty, -90.0, 90.0) as i8 as u8;
    out[16] = f.state as u8;
    out[17] = (if f.eraser { LIVE_ERASER } else { 0 }) | (if f.side { LIVE_SIDE } else { 0 }) | (if f.side2 { LIVE_SIDE2 } else { 0 }) | (if f.down { LIVE_DOWN } else { 0 }) | (if f.gap { LIVE_GAP } else { 0 });
    out[18] = 0;
    out[19] = 0;
}

/// a after b, or the same (identities wrap)
fn at_or_after(a: u32, b: u32) -> bool {
    (a.wrapping_sub(b) as i32) >= 0
}

/// How many samples of a stroke not decided yet a pen viewer holds back (live_raw_gate): about half
/// a second of the raw digitizer's frames. Past that they go on undrawn, and the stroke's quick ink
/// starts where it is decided.
pub const LIVE_RAW_HOLD: usize = 128;

/// What a pen viewer that reads the raw digitizer keeps between its frames to decide their ink
/// (live_raw_gate). It runs ahead of the main loop, often by a whole stroke or more: a pen-down
/// that comes while the main loop is in UI work (blocked), or before it has handled a UI pen-down
/// shown earlier, cannot be judged yet, and its samples wait here until it can.
#[derive(Clone, Copy)]
pub struct LiveRawGate {
    /// a UI pen-down the main loop has not handled yet (its identity; 0 none)
    pub ui: u32,
    /// the stroke under way draws (its first drawn sample said LIVE_DOWN)
    pub drawing: bool,
    /// the pen-down not decided yet (its identity; 0 none), its flags and where it came down
    pub pending: u32,
    pub down_flags: u8,
    pub at: (i32, i32),
    /// the samples held back since it, and whether some went on undrawn already (the hold ran out)
    pub held: [[u8; LIVE_SAMPLE_BYTES]; LIVE_RAW_HOLD],
    pub n: usize,
    pub spilled: bool,
}

impl Default for LiveRawGate {
    fn default() -> Self {
        LiveRawGate { ui: 0, drawing: false, pending: 0, down_flags: 0, at: (0, 0), held: [[0; LIVE_SAMPLE_BYTES]; LIVE_RAW_HOLD], n: 0, spilled: false }
    }
}

impl LiveRawGate {
    /// The stroke forgotten, what it held dropped: nothing was sent meanwhile (asleep, a new ink
    /// epoch, the source changed), and the browser starts no stroke without a pen-down. A UI pen-down
    /// still waiting stays: the main loop has it to handle either way.
    pub fn forget(&mut self) {
        self.drawing = false;
        self.pending = 0;
        self.n = 0;
        self.spilled = false;
    }
}

// The style a sample's eraser and side button state draws with (its flags), or none.
fn gate_style(cap: &LiveCapture, free: bool, flags: u8) -> LiveStyle {
    if free {
        cap.styles[(flags & LIVE_ERASER != 0) as usize][(flags & LIVE_SIDE != 0) as usize | (((flags & LIVE_SIDE2 != 0) as usize) << 1)]
    } else {
        LiveStyle::default()
    }
}

// The held samples on their way: drawn with each one's style (the first saying LIVE_DOWN when the
// stroke's own pen-down went on undrawn), or undrawn.
fn gate_release(g: &mut LiveRawGate, cap: &LiveCapture, drawn: bool, send: &mut dyn FnMut(&[u8], &LiveStyle)) {
    let free = cap.ok && cap.editor && g.ui == 0;
    for i in 0..g.n {
        let mut s = g.held[i];
        let st = gate_style(cap, free, s[17]);
        if drawn && st.enabled != 0 {
            s[17] |= LIVE_DRAW;
            if i == 0 && g.spilled {
                s[17] |= LIVE_DOWN;
            }
        }
        send(&s, &st);
    }
    g.n = 0;
}

/// One raw frame's sample (live_raw_sample's bytes) through the gate, as app.rs's live_capture
/// decides an event's ink: send(sample, style) for each sample to send now, oldest first. Nothing
/// draws unless quick ink is allowed (cap: the app awake and holding its input, the editor up) and
/// no UI pen-down is still unhandled (g.ui). A pen-down where its style draws nothing or on the
/// panel is UI: nothing draws until the main loop has handled it (g.ui set to it). One that comes
/// while the main loop is in UI work (blocked), or behind an unhandled UI pen-down, is held with the
/// samples after it until neither holds, then judged by the capture state the main loop published
/// after that work: drawn from its pen-down, or UI. A stroke once drawing goes on drawing through
/// the main loop's UI work (each event it handles is that for a moment; the browser ends a stroke
/// at its first undrawn sample and starts one only at a pen-down), and a gap in it starts it again.
pub fn live_raw_gate(g: &mut LiveRawGate, cap: &LiveCapture, blocked: bool, f: &LiveRawFrame, sample: &[u8], send: &mut dyn FnMut(&[u8], &LiveStyle)) {
    if g.ui != 0 && at_or_after(cap.handled, g.ui) {
        g.ui = 0;
    }
    let mut s = [0u8; LIVE_SAMPLE_BYTES];
    s.copy_from_slice(&sample[..LIVE_SAMPLE_BYTES]);
    s[17] &= !LIVE_DRAW;
    if f.state != LIVE_PEN_TOUCH {
        gate_release(g, cap, false, send); // (lifted before it could be judged: undrawn)
        g.drawing = false;
        g.pending = 0;
        g.spilled = false;
        send(&s, &gate_style(cap, cap.ok && cap.editor && g.ui == 0, s[17]));
        return;
    }
    if f.down {
        gate_release(g, cap, false, send);
        g.drawing = false;
        g.pending = f.us;
        g.down_flags = s[17];
        g.at = ((s[8] as u32 | (s[9] as u32) << 8) as i32, (s[10] as u32 | (s[11] as u32) << 8) as i32);
        g.spilled = false;
    }
    if g.pending != 0 {
        if g.n == LIVE_RAW_HOLD {
            gate_release(g, cap, false, send);
            g.spilled = true;
        }
        g.held[g.n] = s;
        g.n += 1;
        if blocked || g.ui != 0 {
            return; // (judged at a later frame)
        }
        let st = gate_style(cap, cap.ok && cap.editor, g.down_flags);
        if st.enabled == 0 || rect_contains(st.exclude, g.at.0, g.at.1) {
            g.ui = g.pending;
            g.pending = 0;
            gate_release(g, cap, false, send);
        } else {
            g.pending = 0;
            g.drawing = true;
            gate_release(g, cap, true, send);
        }
        return;
    }
    let st = gate_style(cap, cap.ok && cap.editor && g.ui == 0, s[17]);
    if g.drawing && st.enabled != 0 {
        s[17] |= LIVE_DRAW | if f.gap { LIVE_DOWN } else { 0 };
    } else {
        g.drawing = false;
    }
    send(&s, &st);
}

/// After a read of the pens (input_sample_hook's batch callback, and app_event): the pen viewers
/// woken, once, when a sample came since.
pub fn live_wake_samples() {
    let due = with(|s| {
        let due = s.sh.is_some() && s.sample_wake;
        s.sample_wake = false;
        due
    });
    if due {
        wake_pens();
    }
}

/// app_flush: every pixel of the screen up to the pen event `sample` (its ms) is in the frame now.
/// The tile viewers vouch for it, with the ink epoch it was presented in, in their next batch, one
/// of its own when no pixel changed. Never the ring's head: a pump reads ahead during long work.
pub fn live_presented(sample: u32, ms: u32) {
    let Some(f) = frame() else { return };
    let sh = f.sh();
    let ink = sh.ink_epoch.load(SeqCst);
    if sh.ack_sample.load(SeqCst) == sample && sh.ack_ink.load(SeqCst) == ink {
        return;
    }
    set_ack(sh, sample, ms, ink);
    wake_tiles();
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

// A small frame goes as one send (one TCP segment with TCP_NODELAY), its head and payload together.
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
fn get32(p: &[u8]) -> u32 {
    p[0] as u32 | (p[1] as u32) << 8 | (p[2] as u32) << 16 | (p[3] as u32) << 24
}

// live_presented's watermark, whole: (0, 0, 0) while it is being changed.
fn read_ack(sh: &Shared, sample: &mut u32, ms: &mut u32, ink: &mut u32) {
    let a = sh.ack_version.load(SeqCst);
    *sample = sh.ack_sample.load(SeqCst);
    *ms = sh.ack_ms.load(SeqCst);
    *ink = sh.ack_ink.load(SeqCst);
    if (a & 1) != 0 || a != sh.ack_version.load(SeqCst) {
        *sample = 0;
        *ms = 0;
        *ink = 0;
    }
}

// The ring's record of sample seq into out; false when it is not there whole (overwritten since, or
// being written now).
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

fn send_hello(fd: i32, w: i32, h: i32, epoch: u32) -> bool {
    let mut p = [0u8; 12];
    put16(&mut p, LIVE_VERSION);
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
        Png::of_part(tile, w, w, h)
    }
    /// The same for the w x h rect at the start of rows stride bytes apart (a part of a run).
    fn of_part(tile: &[u8], stride: i32, w: i32, h: i32) -> Option<Png> {
        assert!(w <= stride && (h <= 0 || tile.len() >= (stride as usize) * (h as usize - 1) + w as usize));
        let mut len: libc::c_int = 0;
        // SAFETY: stb reads h rows of w bytes, stride apart, from tile (checked above) and returns a
        // malloc'ed buffer of len bytes, or null.
        let ptr = unsafe { crate::ffi_stb::stbi_write_png_to_mem(tile.as_ptr(), stride, w, h, 1, &mut len) };
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

// After a poll: the wake pipe drained when it had something, then the slot's wake flag cleared, in
// that order and before the frame is read again (Shared.woken): a wake that came meanwhile either
// left its byte in the pipe or is seen in the frame.
fn drain_wake(sh: &Shared, slot: usize, wake: i32, readable: bool) {
    if readable {
        let mut drain = [0u8; 256];
        // SAFETY: read into the local array, at most its length, from the wake pipe.
        while unsafe { libc::read(wake, drain.as_mut_ptr() as *mut libc::c_void, drain.len()) } > 0 {}
    }
    sh.woken[slot].store(0, SeqCst);
}

// Frame 7 from out, which holds 5 bytes for the frame's head and 8 for its own first, then n samples
// (from byte 13): one send.
fn send_samples(fd: i32, out: &mut [u8], epoch: u32, reset: bool, n: usize) -> bool {
    let len = 8 + n * LIVE_SAMPLE_BYTES;
    let mut head = [0u8; 5];
    live_frame_header(&mut head, len as u32, LIVE_SAMPLES as u8);
    out[..5].copy_from_slice(&head);
    put32(&mut out[5..], epoch);
    put32(&mut out[9..], reset as u32);
    send_all(fd, &out[..5 + len])
}

// ---- a pen viewer's own reader of the raw digitizer ----

/// EVIOCSCLOCKID (input_evdev.rs's: linux/ioctl.h's encoding of 'E' 0xa0 with an int)
#[cfg(feature = "kindle")]
const EVIOCSCLOCKID: u32 = 0x4004_45a0;

/// The raw digitizer's node as a pen viewer opened it for itself (raw_open), with the map it was
/// opened for and what its events have said so far.
struct RawSource {
    fd: i32,
    map: LiveRawMap,
    pen: LiveRawPen,
}

/// Opens the node m names, never grabbed (the main loop's reader of it, and the pen service's, see
/// every event as before: each reader of an evdev node has a queue of its own), non-blocking, its
/// times in CLOCK_MONOTONIC as the main loop's are; -1 when it cannot, or it is not that node (dev).
/// The simulator's stand-in is a FIFO, opened for writing too so that it never reads as ended while
/// no writer has it open.
fn raw_open(m: &LiveRawMap) -> i32 {
    let Ok(path) = std::ffi::CString::new(m.path.as_bytes()) else { return -1 };
    if m.path.is_empty() {
        return -1;
    }
    #[cfg(feature = "sim")]
    let flags = libc::O_RDWR | libc::O_NONBLOCK | libc::O_CLOEXEC;
    #[cfg(not(feature = "sim"))]
    let flags = libc::O_RDONLY | libc::O_NONBLOCK | libc::O_CLOEXEC;
    // SAFETY: open reads the NUL-terminated path; the result is checked.
    let fd = unsafe { libc::open(path.as_ptr(), flags) };
    if fd < 0 {
        return -1;
    }
    if m.dev != 0 {
        // SAFETY: fstat writes one stat into the zeroed local.
        let mut st: libc::stat = unsafe { std::mem::zeroed() };
        if unsafe { libc::fstat(fd, &mut st) } != 0 || st.st_rdev as u64 != m.dev {
            // SAFETY: closes the descriptor opened above.
            unsafe { libc::close(fd) };
            return -1;
        }
    }
    #[cfg(feature = "kindle")]
    {
        let clk = libc::CLOCK_MONOTONIC as libc::c_int;
        // SAFETY: EVIOCSCLOCKID reads one int, our local.
        if unsafe { libc::ioctl(fd, EVIOCSCLOCKID as libc::Ioctl, &clk as *const libc::c_int) } != 0 {
            // SAFETY: closes the descriptor opened above.
            unsafe { libc::close(fd) };
            return -1; // (its times would not be the main loop's: no identities in common)
        }
    }
    fd
}

/// After a drop (SYN_DROPPED), what the node holds now (EVIOCGABS, EVIOCGKEY), as input_evdev.rs's
/// wacom_resync asks it. Only on the Kindle; the simulator's FIFO drops nothing.
#[cfg(feature = "kindle")]
fn raw_resync(r: &mut RawSource) {
    #[repr(C)]
    #[derive(Default)]
    struct Absinfo {
        value: i32,
        minimum: i32,
        maximum: i32,
        fuzz: i32,
        flat: i32,
        resolution: i32,
    }
    let abs = |code: u16| -> Option<i32> {
        let mut ai = Absinfo::default();
        let req = (2u32 << 30) | ((std::mem::size_of::<Absinfo>() as u32) << 16) | (0x45 << 8) | (0x40 + code as u32);
        // SAFETY: EVIOCGABS writes one input_absinfo into our local.
        (unsafe { libc::ioctl(r.fd, req as libc::Ioctl, &mut ai as *mut Absinfo) } >= 0).then_some(ai.value)
    };
    if let Some(v) = abs(ABS_X) {
        r.pen.x = v;
        r.pen.placed = true;
    }
    if let Some(v) = abs(ABS_Y) {
        r.pen.y = v;
    }
    if let Some(v) = abs(ABS_PRESSURE) {
        r.pen.p = v;
    }
    if let Some(v) = abs(ABS_TILT_X) {
        r.pen.tx = v;
    }
    if let Some(v) = abs(ABS_TILT_Y) {
        r.pen.ty = v;
    }
    let mut keys = [0u8; 96];
    let req = (2u32 << 30) | ((keys.len() as u32) << 16) | (0x45 << 8) | 0x18;
    // SAFETY: EVIOCGKEY writes at most keys.len() bytes into the local array.
    if unsafe { libc::ioctl(r.fd, req as libc::Ioctl, keys.as_mut_ptr()) } >= 0 {
        let bit = |b: u16| keys[(b / 8) as usize] & (1 << (b % 8)) != 0;
        r.pen.tool_pen = bit(BTN_TOOL_PEN);
        r.pen.tool_rubber = bit(BTN_TOOL_RUBBER);
        r.pen.touch = bit(BTN_TOUCH);
        r.pen.side = bit(BTN_STYLUS);
        r.pen.side2 = bit(BTN_STYLUS2);
    }
}
#[cfg(not(feature = "kindle"))]
fn raw_resync(_r: &mut RawSource) {}

fn raw_close(r: Option<RawSource>) {
    if let Some(r) = r {
        // SAFETY: closes the node's descriptor, which only this viewer has.
        unsafe { libc::close(r.fd) };
    }
}

/// What a pen viewer keeps between its sends (pen_main).
struct PenOut {
    out: [u8; 13 + LIVE_SAMPLE_BYTES * LIVE_PEN_BATCH],
    n: usize,
    frame_reset: bool,
    style: [u8; LIVE_STYLE_BYTES],
    have_style: bool,
    /// a send failed: the browser is gone
    dead: bool,
    /// when something was last sent (for the ping)
    last: u32,
}

impl PenOut {
    fn flush(&mut self, fd: i32, epoch: u32) {
        if self.n > 0 && !self.dead {
            let n = self.n;
            if !send_samples(fd, &mut self.out, epoch, self.frame_reset, n) {
                self.dead = true;
            }
            self.last = now_ms();
        }
        self.n = 0;
    }
    /// One sample (LIVE_SAMPLE_BYTES) drawn with style (its frame-8 bytes): the style first when it
    /// changed, after the samples before it, which draw with the one before.
    fn push(&mut self, fd: i32, epoch: u32, sample: &[u8], style: &[u8], reset: &mut bool) {
        if !self.have_style || self.style[..] != style[..LIVE_STYLE_BYTES] {
            self.flush(fd, epoch);
            self.style.copy_from_slice(&style[..LIVE_STYLE_BYTES]);
            if !self.dead && !send_frame(fd, LIVE_STYLE as u8, &self.style, &[]) {
                self.dead = true;
            }
            self.have_style = true;
        }
        if self.n == 0 {
            self.frame_reset = *reset;
        }
        let at = 13 + self.n * LIVE_SAMPLE_BYTES;
        self.out[at..at + LIVE_SAMPLE_BYTES].copy_from_slice(&sample[..LIVE_SAMPLE_BYTES]);
        *reset = false;
        self.n += 1;
        if self.n == LIVE_PEN_BATCH {
            self.flush(fd, epoch);
        }
    }
}

// Everything the raw node has queued, read and sent: each frame placed by the map, its ink decided by
// the gate (live_raw_gate) from the main loop's capture state and the ink epoch it is in. Nothing is
// sent but while sending is on (the stream awake, in this epoch, quick ink's capture allowed): the
// node is read empty either way, so what comes later is never stale. cap is the capture state as last
// read whole: the main loop rewrites it at every pen event, and a read that meets a write keeps the
// one before (an undrawn sample in a stroke would end its quick ink in the browser).
fn raw_pump(sh: &Shared, r: &mut RawSource, o: &mut PenOut, fd: i32, epoch: u32, sending: bool, gate: &mut LiveRawGate, cap: &mut LiveCapture, reset: &mut bool) -> bool {
    let mut evs = [LiveInputEvent::default(); 64];
    let mut cap_b = [0u8; CAPTURE_BYTES];
    let mut have_cap = false;
    loop {
        // SAFETY: read into the local array of input_event records, at most its size in bytes.
        let got = unsafe { libc::read(r.fd, evs.as_mut_ptr() as *mut libc::c_void, std::mem::size_of_val(&evs)) };
        if got < 0 {
            if errno() == libc::EINTR {
                continue;
            }
            if errno() == libc::EAGAIN || errno() == libc::EWOULDBLOCK {
                break;
            }
            return false; // a detached digitizer: stop polling this descriptor
        }
        if got == 0 {
            return false;
        }
        let n = got as usize / std::mem::size_of::<LiveInputEvent>();
        for ev in evs[..n].iter() {
            let mut fr = LiveRawFrame::default();
            let dropping = r.pen.dropping;
            if !live_raw_event(&mut r.pen, r.map.pmin, ev, &mut fr) {
                if dropping && !r.pen.dropping {
                    raw_resync(r); // (the half frame is over: what the node holds now)
                }
                continue;
            }
            if !sending || o.dead || sh.state.load(SeqCst) != LIVE_AWAKE as u32 || sh.ink_epoch.load(SeqCst) != epoch {
                gate.forget(); // (told at the top of pen_main with a hello; this frame is the epoch's before)
                continue;
            }
            if !have_cap {
                // (the writer never waits: a few tries, else the one before stays)
                for _ in 0..4 {
                    if sh.capture.read(&mut cap_b).is_some() {
                        *cap = capture_read(&cap_b);
                        break;
                    }
                }
                have_cap = true;
            }
            if !cap.ok {
                gate.forget();
                continue; // (asleep, the input the Kindle's, Live view going: nothing of the pen is sent)
            }
            let mut sample = [0u8; LIVE_SAMPLE_BYTES];
            let w = sh.w.load(SeqCst) as i32;
            let h = sh.h.load(SeqCst) as i32;
            live_raw_sample(&r.map, w, h, &fr, &mut sample);
            let mut style_b = [0u8; LIVE_STYLE_BYTES];
            live_raw_gate(gate, cap, sh.blocked.load(SeqCst) != 0, &fr, &sample, &mut |s, st| {
                live_style_payload(&mut style_b, st);
                o.push(fd, epoch, s, &style_b, reset);
            });
        }
    }
    o.flush(fd, epoch);
    true
}

/// capture_bytes read back.
fn capture_read(b: &[u8; CAPTURE_BYTES]) -> LiveCapture {
    let mut c = LiveCapture { ok: b[0] != 0, editor: b[1] != 0, handled: get32(&b[4..]), ..Default::default() };
    for e in 0..2 {
        for side in 0..4 {
            c.styles[e][side] = live_style_read(&b[8 + (e * 4 + side) * LIVE_STYLE_BYTES..]);
        }
    }
    c
}

/// live_style_payload read back.
pub fn live_style_read(b: &[u8]) -> LiveStyle {
    let f = |i: usize| f32::from_bits(get32(&b[4 + i * 4..]));
    let int = |i: usize| get32(&b[32 + i * 4..]) as i32;
    LiveStyle {
        enabled: b[0],
        tool: b[1],
        white: b[2],
        whole: b[3],
        width_min: f(0),
        width_max: f(1),
        pressure_floor: f(2),
        pressure_ceiling: f(3),
        pressure_gamma: f(4),
        opacity_min: f(5),
        opacity_max: f(6),
        clip: Rect { x: int(0), y: int(1), w: int(2), h: int(3) },
        exclude: Rect { x: int(4), y: int(5), w: int(6), h: int(7) },
    }
}

// A pen viewer (GET /live/pen): the state as it changes, a hello for each ink epoch, and the samples,
// every one that came since the last wake in as few frames as they fit, each with the style before it
// when that changed: from the raw digitizer's node while the main loop publishes a map for it (raw:
// read the moment the node has a frame, whatever the main loop is doing), else from the ring. It
// makes no PNG and never waits for anything but its wake pipe and the node.
fn pen_main(f: Frame, slot: usize, fd: i32, wake: i32, parent: libc::pid_t) -> ! {
    let sh = f.sh();
    let mut epoch: u32 = 0;
    let mut cursor: u32 = 0;
    let mut told: u32 = !0;
    let mut reset = true;
    let mut snapshot = true;
    let mut o = PenOut { out: [0u8; 13 + LIVE_SAMPLE_BYTES * LIVE_PEN_BATCH], n: 0, frame_reset: false, style: [0u8; LIVE_STYLE_BYTES], have_style: false, dead: false, last: now_ms() };
    let mut raw: Option<RawSource> = None;
    let mut map_seen: u32 = !0;
    let mut raw_epoch: u32 = !0;
    // what the raw node's frames draw (live_raw_gate), and the capture state as last read whole
    let mut gate = LiveRawGate::default();
    let mut cap = LiveCapture::default();
    loop {
        let state = sh.state.load(SeqCst);
        let next_raw_epoch = sh.raw_epoch.load(SeqCst);
        // The device can suspend or be replaced with the very same path and calibration. Never
        // keep its reader (or a held contact) across sleep; wake opens a fresh event queue.
        if state != LIVE_AWAKE as u32 || raw_epoch != next_raw_epoch {
            raw_close(raw.take());
            raw_epoch = next_raw_epoch;
            map_seen = !0;
            gate.forget();
            o.n = 0;
            o.have_style = false;
        }
        // The raw node: only while awake, once the main loop publishes a map for it.
        let mut mb = [0u8; MAP_BYTES];
        let map_version = if state == LIVE_AWAKE as u32 { sh.raw_map.read(&mut mb) } else { None };
        if let Some(v) = map_version {
            if v != map_seen {
                map_seen = v;
                let m = if sh.ring_only.load(SeqCst) != 0 { None } else { raw_map_read(&mb) };
                match (m, raw.as_mut()) {
                    (Some(m), Some(r)) if r.map.path == m.path && r.map.dev == m.dev => r.map = m,
                    (m, _) => {
                        let had = raw.is_some();
                        raw_close(raw.take());
                        if let Some(m) = m {
                            let rfd = raw_open(&m);
                            if rfd >= 0 {
                                raw = Some(RawSource { fd: rfd, map: m, pen: LiveRawPen::default() });
                            }
                        }
                        if had != raw.is_some() {
                            reset = true; // (from one source to the other: no stroke joins across)
                            cursor = sh.sample_head.load(SeqCst);
                            o.have_style = false;
                            gate.forget();
                        }
                    }
                }
            }
        }
        let e = sh.ink_epoch.load(SeqCst);
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
            // newest place only; never replay a stroke when reconnecting
            cursor = sh.sample_head.load(SeqCst).saturating_sub(1);
            o.have_style = false;
            snapshot = cursor != sh.sample_head.load(SeqCst);
            if !send_samples(fd, &mut o.out, epoch, true, 0) {
                break;
            }
            o.last = now_ms();
            reset = false;
            gate.forget(); // (nothing held from the epoch before)
        }
        if let Some(r) = raw.as_mut() {
            cursor = sh.sample_head.load(SeqCst); // (the ring's samples are the same frames, later)
            let alive = raw_pump(sh, r, &mut o, fd, epoch, state == LIVE_AWAKE as u32, &mut gate, &mut cap, &mut reset);
            if o.dead {
                child_exit();
            }
            if !alive {
                raw_close(raw.take());
                reset = true;
                gate.forget();
                o.n = 0;
                o.have_style = false;
            }
        } else if state == LIVE_AWAKE as u32 {
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
                if !sample_read(sh, next, &mut record) {
                    reset = true;
                    break;
                }
                cursor = next;
                if get32(&record[84..]) != epoch {
                    // (an ink epoch before: never sent. The snapshot is the first record after the
                    // hello, whichever it is: the next stroke's pen-down keeps its LIVE_DOWN.)
                    snapshot = false;
                    continue;
                }
                if sh.state.load(SeqCst) != LIVE_AWAKE as u32 || sh.ink_epoch.load(SeqCst) != epoch {
                    break;
                }
                if reset || snapshot {
                    record[17] &= !LIVE_DOWN;
                }
                snapshot = false;
                let (sample, style) = record.split_at(20);
                o.push(fd, epoch, sample, style, &mut reset);
                if o.dead {
                    child_exit();
                }
            }
            o.flush(fd, epoch);
            if o.dead {
                child_exit();
            }
        }
        if now_ms().wrapping_sub(o.last) >= 15000 {
            if !send_frame(fd, LIVE_PING as u8, &[], &[]) {
                break;
            }
            o.last = now_ms();
        }
        let rfd = raw.as_ref().map_or(-1, |r| r.fd);
        let mut pf = [
            libc::pollfd { fd: wake, events: libc::POLLIN, revents: 0 },
            libc::pollfd { fd, events: libc::POLLIN, revents: 0 },
            libc::pollfd { fd: rfd, events: libc::POLLIN, revents: 0 },
        ];
        // SAFETY: poll reads and writes the pollfds of the local array (a negative fd is skipped).
        let got = unsafe { libc::poll(pf.as_mut_ptr(), if rfd >= 0 { 3 } else { 2 }, 1000) };
        if got < 0 && errno() != libc::EINTR {
            break;
        }
        // The browser sends nothing on this stream: anything from it, or its hanging up, ends it.
        if pf[1].revents & (libc::POLLHUP | libc::POLLERR | libc::POLLIN) != 0 {
            break;
        }
        if rfd >= 0 && pf[2].revents & (libc::POLLHUP | libc::POLLERR | libc::POLLNVAL) != 0 {
            raw_close(raw.take()); // (the node went: the ring until a new map names it)
            reset = true;
            gate.forget();
            o.n = 0;
            o.have_style = false;
            cursor = sh.sample_head.load(SeqCst);
        }
        drain_wake(sh, slot, wake, pf[0].revents & libc::POLLIN != 0);
        // SAFETY: getppid takes no arguments.
        if unsafe { libc::getppid() } != parent {
            break;
        }
    }
    raw_close(raw.take());
    // SAFETY: shutdown of our own socket's write side, then _exit, which ends the child at once and
    // runs none of the parent's exit handlers.
    unsafe {
        libc::shutdown(fd, libc::SHUT_WR);
        libc::_exit(0)
    }
}

// The viewer child of slot (a tile viewer below LIVE_VIEWERS, a pen viewer from there): parent is
// the app's pid, taken before the fork.
fn viewer_main(f: Frame, slot: usize, fd: i32, wake: i32, parent: libc::pid_t) -> ! {
    // First of all, to end with the app: the death signal asked for, then the app looked for, as it
    // may have ended before the signal was asked for (then nothing would ever send it).
    #[cfg(target_os = "linux")]
    // SAFETY: prctl with a NUL-terminated name and the signal number, no pointers kept.
    unsafe {
        libc::prctl(libc::PR_SET_NAME, c"grimoire-live".as_ptr() as libc::c_ulong, 0 as libc::c_ulong, 0 as libc::c_ulong, 0 as libc::c_ulong);
        libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM as libc::c_ulong, 0 as libc::c_ulong, 0 as libc::c_ulong, 0 as libc::c_ulong);
    }
    // SAFETY: getppid takes no arguments.
    if unsafe { libc::getppid() } != parent {
        child_exit();
    }
    let sh = f.sh();
    let fast = slot >= LIVE_VIEWERS;
    let dirty = &sh.dirty[slot % LIVE_VIEWERS]; // (a pen viewer's is never used)
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
    // SAFETY: nice and fcntl on our own descriptor take no pointers. (nice: best effort.)
    if !fast {
        unsafe { libc::nice(5) };
    }
    unsafe { libc::fcntl(fd, libc::F_SETFL, libc::fcntl(fd, libc::F_GETFL, 0) & !libc::O_NONBLOCK) };
    set_int_opt(fd, libc::IPPROTO_TCP, libc::TCP_NODELAY, 1);
    // Wi-Fi can disappear across suspend without a FIN/RST. A successful write only means the
    // local TCP buffer accepted it: bound unacknowledged data so stale viewers release slots.
    set_int_opt(fd, libc::SOL_SOCKET, libc::SO_KEEPALIVE, 1);
    #[cfg(target_os = "linux")]
    {
        set_int_opt(fd, libc::IPPROTO_TCP, libc::TCP_KEEPIDLE, 15);
        set_int_opt(fd, libc::IPPROTO_TCP, libc::TCP_KEEPINTVL, 5);
        set_int_opt(fd, libc::IPPROTO_TCP, libc::TCP_KEEPCNT, 3);
        set_int_opt(fd, libc::IPPROTO_TCP, libc::TCP_USER_TIMEOUT, 15000);
    }
    // A pen viewer's samples are small and want to go now: a small buffer, and a browser that takes
    // nothing for a second is gone; a tile viewer's PNGs are big: a browser that takes nothing for
    // 10 s is gone.
    set_int_opt(fd, libc::SOL_SOCKET, libc::SO_SNDBUF, if fast { 16 * 1024 } else { 256 * 1024 });
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
        pen_main(f, slot, fd, wake, parent);
    }
    let cap = f.px_len();
    let mut sent: Vec<u8> = Vec::new();
    let mut tile: Vec<u8> = Vec::new();
    if sent.try_reserve_exact(cap).is_err() || tile.try_reserve_exact(LIVE_TILE as usize * 4096).is_err() || !send_all(fd, HEAD) {
        child_exit();
    }
    sent.resize(cap, 0);
    tile.resize(LIVE_TILE as usize * 4096, 0);
    let mut layer_buf: Vec<u8> = Vec::new();
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
    // the watermark as the browser has it: its sample (0 none yet) and its ink epoch
    let mut ack_sent: u32 = 0;
    let mut ink_sent: u32 = 0;
    // each layer's version as the browser has it (odd: none yet, never a version written whole)
    let mut layer_sent: [u32; LIVE_LAYERS] = [!0; LIVE_LAYERS];
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
            ink_sent = 0;
            layer_sent = [!0; LIVE_LAYERS]; // (a hello takes them away: each comes again)
            pen = sh.pen.load(SeqCst);
        }
        if state == LIVE_AWAKE as u32 {
            let (mut ack, mut ack_ms, mut ack_ink) = (0, 0, 0);
            // Before the marks: every pixel it vouches for is in this batch or an earlier one.
            read_ack(sh, &mut ack, &mut ack_ms, &mut ack_ink);
            let news = ack_ink != ink_sent || (ack != 0 && ack != ack_sent);
            let layer_news = (0..LIVE_LAYERS).any(|i| sh.layers[i].version.load(SeqCst) != layer_sent[i]);
            let mut words = [0u32; LIVE_WORDS];
            let mut any = false;
            for i in 0..LIVE_WORDS {
                words[i] = dirty[i].swap(0, SeqCst);
                any |= words[i] != 0;
            }
            if any || news || layer_news {
                let mut runs = [RECT_EMPTY; LIVE_WORDS * 32];
                let n = live_runs(&words, w, h, &mut runs);
                let mut out = 0;
                // ok: the batch is whole, so it may vouch for the watermark; moved: the state or the
                // screen's epoch moved on under it (told at the top at once)
                let mut ok = true;
                let mut moved = false;
                'runs: for &r in runs[..n as usize].iter() {
                    // A snapshot of the run, then the state again: a pairing card drawn meanwhile
                    // is never sent, and its tiles come again once it is gone (live_set_state).
                    for y in 0..r.h as usize {
                        let at = (r.y as usize + y) * (w as usize) + r.x as usize;
                        let row = &mut tile[y * r.w as usize..][..r.w as usize];
                        // SAFETY: the run is cut to the w x h screen, so the row is inside the
                        // frame's pixel bytes (w and h are the epoch's); the copy is one memcpy
                        // of bytes the main loop may be rewriting (benign: see live_push).
                        unsafe { std::ptr::copy_nonoverlapping(f.px().add(at) as *const u8, row.as_mut_ptr(), r.w as usize) };
                    }
                    fence(SeqCst);
                    if sh.state.load(SeqCst) != LIVE_AWAKE as u32 || sh.epoch.load(SeqCst) != epoch {
                        ok = false; // (live_set_state(LIVE_AWAKE) and a turn mark every tile again)
                        moved = true;
                        break;
                    }
                    // Only the tiles of the run that changed, each stretch of them a PNG of its own.
                    let mut parts = [RECT_EMPTY; 32];
                    let np = live_changed_parts(&sent, w, &tile, r, &mut parts);
                    for &p in parts[..np].iter() {
                        let off = (p.x - r.x) as usize;
                        // No memory for the PNG: its marks back for the next try, which waits for a
                        // wake and the batches' pace like any other (a retry at once would spin while
                        // the memory stays short).
                        let Some(png) = Png::of_part(&tile[off..], r.w, p.w, p.h) else {
                            for i in 0..LIVE_WORDS {
                                dirty[i].fetch_or(words[i], SeqCst);
                            }
                            ok = false;
                            break 'runs;
                        };
                        let mut at = [0u8; 8];
                        put16(&mut at, p.x as u32);
                        put16(&mut at[2..], p.y as u32);
                        put16(&mut at[4..], p.w as u32);
                        put16(&mut at[6..], p.h as u32);
                        let sent_ok = send_frame(fd, LIVE_TILE_FRAME as u8, &at, png.as_slice());
                        drop(png);
                        if !sent_ok {
                            child_exit();
                        }
                        for y in 0..p.h as usize {
                            let at = (p.y as usize + y) * (w as usize) + p.x as usize;
                            sent[at..at + p.w as usize].copy_from_slice(&tile[y * r.w as usize + off..][..p.w as usize]);
                        }
                        out += 1;
                    }
                }
                // The layers that changed, after the tiles (the page under them first), each whole:
                // one being written as it is read comes at the next wake, which its write makes.
                for i in 0..LIVE_LAYERS {
                    if !ok || sh.layers[i].version.load(SeqCst) == layer_sent[i] {
                        continue;
                    }
                    let mut words = [0u8; 20];
                    let Some(v) = sh.layers[i].read(&mut words) else { continue };
                    let r = Rect { x: get32(&words) as i32, y: get32(&words[4..]) as i32, w: get32(&words[8..]) as i32, h: get32(&words[12..]) as i32 };
                    let n = (r.w.max(0) as usize) * (r.h.max(0) as usize);
                    if n > f.px_len() || (n > 0 && layer_buf.try_reserve(n.saturating_sub(layer_buf.len())).is_err()) {
                        continue;
                    }
                    layer_buf.resize(n.max(layer_buf.len()), 0);
                    // SAFETY: n bytes from the start of layer i's buffer, which holds px_len >= n; a
                    // copy of bytes the main loop may be rewriting, checked whole by the version below.
                    unsafe { std::ptr::copy_nonoverlapping(f.layer_px(i) as *const u8, layer_buf.as_mut_ptr(), n) };
                    fence(SeqCst);
                    if sh.layers[i].version.load(SeqCst) != v {
                        continue;
                    }
                    let mut at = [0u8; 10];
                    at[0] = i as u8;
                    put16(&mut at[2..], r.x as u32);
                    put16(&mut at[4..], r.y as u32);
                    put16(&mut at[6..], r.w as u32);
                    put16(&mut at[8..], r.h as u32);
                    let sent_ok = if n == 0 {
                        send_frame(fd, LIVE_LAYER as u8, &at, &[])
                    } else {
                        let Some(png) = Png::of_tile(&layer_buf, r.w, r.h) else { continue };
                        send_frame(fd, LIVE_LAYER as u8, &at, png.as_slice())
                    };
                    if !sent_ok {
                        child_exit();
                    }
                    layer_sent[i] = v;
                    out += 1;
                }
                if out != 0 || (ok && news) {
                    let mut p = [0u8; 20];
                    put32(&mut p, sh.seq.fetch_add(1, SeqCst));
                    put32(&mut p[4..], now_ms());
                    if ok {
                        put32(&mut p[8..], ack);
                        put32(&mut p[12..], ack_ms);
                        put32(&mut p[16..], ack_ink);
                    }
                    if !send_frame(fd, LIVE_BATCH as u8, &p, &[]) {
                        break;
                    }
                    if ok {
                        if ack != 0 {
                            ack_sent = ack;
                        }
                        ink_sent = ack_ink;
                    }
                    last_batch = now_ms();
                    last_sent = last_batch;
                }
                if moved {
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
        drain_wake(sh, slot, wake, pf[0].revents & libc::POLLIN != 0);
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

// A viewer child for the connection fd, in a free slot of its kind (the last free one).
fn take_viewer(fd: i32, fast: bool) -> bool {
    let Some(f) = frame() else { return false };
    // Reconnects are served before the next live_poll; reclaim workers that already ended.
    for i in 0..LIVE_VIEWERS * 2 {
        reap(i, false);
    }
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
    f.sh().woken[slot as usize].store(0, SeqCst); // (a new child reads the frame first thing)
    // SAFETY: getpid takes no arguments. (The child compares its parent with it: see viewer_main.)
    let parent = unsafe { libc::getpid() };
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
        viewer_main(f, slot as usize, fd, p[0], parent);
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
/// The same for GET /live/pen: a pen viewer, which sends the samples (fast ink).
pub fn live_take_pen_viewer(fd: i32) -> bool {
    take_viewer(fd, true)
}
