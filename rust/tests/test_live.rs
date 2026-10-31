// test_live.rs: Live view's tile marks and runs, its frames, and a viewer child end to end over a
// socket pair: the hello, the tiles (PNGs read back against the screen), only what changed after,
// and the pen's place after them.
// (the C tests index by position, and so do these ports of them)
#![allow(clippy::needless_range_loop, clippy::too_many_arguments)]
mod testlib;
use grimoire::ffi_stb::{stbi_image_free, stbi_load_from_memory};
use grimoire::prelude::*;
use testlib::*;

fn marks_cover_exactly_the_tiles_a_rect_touches() {
    let mut w = [0u32; LIVE_WORDS];
    live_mark(&mut w, 1860, 2480, Rect { x: 0, y: 0, w: 1, h: 1 });
    check!(w[0] == 1u32);
    w = [0u32; LIVE_WORDS];
    live_mark(&mut w, 1860, 2480, Rect { x: 63, y: 63, w: 2, h: 2 }); // the corner of four tiles
    let across = live_tiles_across(1860);
    check!(across == 30);
    check!(w[0] == (3u32 | 3u32 << 30)); // tiles 0 and 1, and 30 and 31 (the next row's first two)
    w = [0u32; LIVE_WORDS];
    live_mark(&mut w, 1860, 2480, Rect { x: -50, y: -50, w: 5000, h: 5000 }); // cut to the screen: every tile, no more
    let tiles = 30 * 39;
    let mut set = 0;
    for i in 0..LIVE_WORDS * 32 {
        set += (w[i / 32] >> (i % 32)) & 1;
    }
    check!(set == tiles);
    w = [0u32; LIVE_WORDS];
    live_mark(&mut w, 1860, 2480, Rect { x: 1859, y: 2479, w: 1, h: 1 }); // the last pixel: the last tile
    check!((w[(tiles as usize - 1) / 32] >> ((tiles as usize - 1) % 32) & 1u32) == 1u32);
    w = [0u32; LIVE_WORDS];
    live_mark(&mut w, 2648, 1986, Rect { x: 0, y: 0, w: 2648, h: 1986 }); // the bigger Scribe, sideways: still fits
    set = 0;
    for i in 0..LIVE_WORDS * 32 {
        set += (w[i / 32] >> (i % 32)) & 1;
    }
    check!(set as i32 == live_tiles_across(2648) * 32);
}

fn runs_join_neighbours_in_a_row_and_stay_on_screen() {
    let mut w = [0u32; LIVE_WORDS];
    live_mark(&mut w, 1860, 2480, Rect { x: 100, y: 100, w: 200, h: 10 }); // tiles 1..4 of row 1
    live_mark(&mut w, 1860, 2480, Rect { x: 1800, y: 2470, w: 60, h: 10 }); // the last tile
    let mut r = [RECT_EMPTY; 8];
    let n = live_runs(&w, 1860, 2480, &mut r);
    check!(n == 2);
    check!(r[0].x == 64 && r[0].y == 64 && r[0].w == 256 && r[0].h == 64);
    check!(r[1].x == 1792 && r[1].y == 2432 && r[1].w == 68 && r[1].h == 48); // cut at the edges
    check!(live_runs(&w, 1860, 2480, &mut r[..1]) == 1); // at most cap
}

fn frame_headers_are_little_endian() {
    let mut h = [0u8; 5];
    live_frame_header(&mut h, 0x010203, LIVE_TILE_FRAME as u8);
    check!(h[0] == 0x04 && h[1] == 0x02 && h[2] == 0x01 && h[3] == 0 && h[4] == LIVE_TILE_FRAME as u8);
}

// Reads exactly the bytes asked for (the C's read loops: a read that gives nothing ends it).
fn read_n(fd: i32, buf: &mut [u8]) -> bool {
    let mut got = 0;
    while got < buf.len() {
        // SAFETY: read(2) into the unfilled tail of buf, at most its length.
        let r = unsafe { libc::read(fd, buf[got..].as_mut_ptr() as *mut libc::c_void, buf.len() - got) };
        if r <= 0 {
            return false;
        }
        got += r as usize;
    }
    true
}

// Reads one frame from fd: its type and its payload. None at the end.
fn read_frame(fd: i32) -> Option<(u8, Vec<u8>)> {
    let mut h = [0u8; 5];
    if !read_n(fd, &mut h) {
        return None;
    }
    let len = h[0] as u32 | (h[1] as u32) << 8 | (h[2] as u32) << 16 | (h[3] as u32) << 24;
    if len < 1 {
        return None;
    }
    let mut body = vec![0u8; (len - 1) as usize];
    if !read_n(fd, &mut body) {
        return None;
    }
    Some((h[4], body))
}

fn u16(p: &[u8]) -> u32 {
    p[0] as u32 | (p[1] as u32) << 8
}

// Skips the HTTP head: everything up to the blank line.
fn skip_head(fd: i32) -> bool {
    let mut last = [0u8; 4];
    loop {
        let mut c = 0u8;
        // SAFETY: read(2) of one byte into the local c.
        if unsafe { libc::read(fd, &mut c as *mut u8 as *mut libc::c_void, 1) } != 1 {
            return false;
        }
        last.copy_within(1.., 0);
        last[3] = c;
        if &last == b"\r\n\r\n" {
            return true;
        }
    }
}

// A gray PNG decoded by stb_image: its pixels and size.
fn decode_png(data: &[u8]) -> Option<(Vec<u8>, i32, i32)> {
    let (mut pw, mut ph, mut ch) = (0, 0, 0);
    // SAFETY: stb reads data.len() bytes of data; the pixels it returns (pw * ph of them, one
    // channel asked for) are copied out and freed with its own free.
    unsafe {
        let px = stbi_load_from_memory(data.as_ptr(), data.len() as i32, &mut pw, &mut ph, &mut ch, 1);
        if px.is_null() {
            return None;
        }
        let v = std::slice::from_raw_parts(px, (pw * ph) as usize).to_vec();
        stbi_image_free(px as *mut libc::c_void);
        Some((v, pw, ph))
    }
}

// Reads frames up to the next batch end, drawing the tiles into screen (w x h); returns how many
// tiles came, or -1 when the stream ended first.
fn read_batch(fd: i32, screen: &mut [u8], w: i32, h: i32, hellos: &mut i32) -> i32 {
    let mut tiles = 0;
    loop {
        let Some((type_, body)) = read_frame(fd) else { return -1 };
        let len = body.len();
        if type_ == LIVE_HELLO as u8 {
            check!(len == 12 && u16(&body) == LIVE_VERSION && u16(&body[2..]) == w as u32 && u16(&body[4..]) == h as u32 && u16(&body[6..]) == LIVE_TILE as u32);
            *hellos += 1;
        } else if type_ == LIVE_TILE_FRAME as u8 {
            let x = u16(&body) as i32;
            let y = u16(&body[2..]) as i32;
            let tw = u16(&body[4..]) as i32;
            let th = u16(&body[6..]) as i32;
            let px = decode_png(&body[8..]);
            let fits = matches!(&px, Some((_, pw, ph)) if *pw == tw && *ph == th) && x + tw <= w && y + th <= h;
            check!(fits);
            if let (Some((px, _, _)), true) = (&px, fits) {
                for r in 0..th as usize {
                    let at = (y as usize + r) * w as usize + x as usize;
                    screen[at..at + tw as usize].copy_from_slice(&px[r * tw as usize..][..tw as usize]);
                }
            }
            tiles += 1;
        } else if type_ == LIVE_BATCH as u8 {
            return tiles;
        }
    }
}

fn socket_pair() -> [i32; 2] {
    let mut sv = [0i32; 2];
    // SAFETY: socketpair fills the two descriptors of the local array.
    check!(unsafe { libc::socketpair(libc::AF_UNIX, libc::SOCK_STREAM, 0, sv.as_mut_ptr()) } == 0);
    sv
}

fn close(fd: i32) {
    // SAFETY: closes a descriptor this test owns.
    unsafe { libc::close(fd) };
}

fn a_reconnect_reclaims_an_ended_viewer_without_a_poll_tick() {
    check!(live_start(64, 64) == 0);
    let first = socket_pair();
    check!(live_take_viewer(first[0]));
    close(first[0]);
    check!(skip_head(first[1]));
    let second = socket_pair();
    check!(live_take_viewer(second[0]));
    close(second[0]);
    check!(skip_head(second[1]));
    check!(live_viewers() == 2);
    close(first[1]);
    sleep_ms(100); // let the closed socket end its worker; deliberately no live_poll
    let reconnect = socket_pair();
    check!(live_take_viewer(reconnect[0]));
    close(reconnect[0]);
    // A failed take owns no socket: do not block reading it in the regression's failing case.
    check!(!quiet(reconnect[1], 1000));
    close(reconnect[1]);
    close(second[1]);
    live_stop();
}

fn a_viewer_sends_the_screen_then_what_changed() {
    let (w, h) = (300, 200);
    let mut c = Canvas::default();
    check!(canvas_init(&mut c, w, h) == 0);
    for y in 0..h as usize {
        for x in 0..w as usize {
            c.px[y * c.stride as usize + x] = ((x * 7 + y * 3) & 0xFF) as u8;
        }
    }
    check!(live_start(w, h) == 0 && live_on());
    live_push(&c, Rect { x: 0, y: 0, w, h });
    let sv = socket_pair();
    check!(live_take_viewer(sv[0]));
    close(sv[0]);
    check!(live_viewers() == 1);
    check!(skip_head(sv[1]));
    // The state first (awake, the colors as drawn, no warmth known), then the hello and the screen.
    let f = read_frame(sv[1]);
    check!(matches!(&f, Some((t, b)) if *t == LIVE_STATE as u8 && b.len() == 3 && b[0] == LIVE_AWAKE as u8 && b[1] == 0 && b[2] == LIVE_NO_WARMTH as u8));
    let mut seen = vec![0u8; (w * h) as usize];
    let mut hellos = 0;
    let first = read_batch(sv[1], &mut seen, w, h, &mut hellos);
    check!(hellos == 1 && first > 0);
    let mut same = true;
    for y in 0..h as usize {
        same &= seen[y * w as usize..][..w as usize] == c.px[y * c.stride as usize..][..w as usize];
    }
    check!(same);
    // A stroke's worth: one tile changes, one tile comes.
    canvas_fill(&mut c, Rect { x: 70, y: 70, w: 20, h: 20 }, 0);
    live_push(&c, Rect { x: 70, y: 70, w: 20, h: 20 });
    check!(read_batch(sv[1], &mut seen, w, h, &mut hellos) == 1);
    check!(seen[(80 * w + 80) as usize] == 0 && hellos == 1);
    // A push that changes nothing sends nothing; the next real change comes alone.
    live_push(&c, Rect { x: 200, y: 100, w: 50, h: 50 });
    canvas_fill(&mut c, Rect { x: 5, y: 150, w: 10, h: 10 }, 255);
    live_push(&c, Rect { x: 5, y: 150, w: 10, h: 10 });
    check!(read_batch(sv[1], &mut seen, w, h, &mut hellos) == 1);
    check!(seen[(155 * w + 8) as usize] == 255);
    // Turned: a new hello, the whole screen again.
    live_resize(h, w);
    let mut t = Canvas::default();
    check!(canvas_init(&mut t, h, w) == 0);
    canvas_fill(&mut t, Rect { x: 0, y: 0, w: h, h: w }, 40);
    live_push(&t, Rect { x: 0, y: 0, w: h, h: w });
    let mut turned = vec![0u8; (w * h) as usize];
    while hellos < 2 && read_batch(sv[1], &mut turned, h, w, &mut hellos) >= 0 {}
    check!(hellos == 2);
    // Bye: the viewer says so and ends; nothing is left running.
    live_stop();
    let mut bye = false;
    while let Some((type_, body)) = read_frame(sv[1]) {
        if type_ == LIVE_STATE as u8 && body.len() == 3 && body[0] == LIVE_BYE as u8 {
            bye = true;
        }
    }
    check!(bye && !live_on() && live_viewers() == 0);
    close(sv[1]);
    canvas_free(&mut t);
    canvas_free(&mut c);
}

fn nothing_is_sent_while_the_pairing_card_is_up() {
    let (w, h) = (128, 128);
    let mut c = Canvas::default();
    check!(canvas_init(&mut c, w, h) == 0);
    canvas_fill(&mut c, Rect { x: 0, y: 0, w, h }, 200);
    check!(live_start(w, h) == 0);
    live_push(&c, Rect { x: 0, y: 0, w, h });
    let sv = socket_pair();
    check!(live_take_viewer(sv[0]));
    close(sv[0]);
    check!(skip_head(sv[1]));
    let mut seen = vec![0u8; (w * h) as usize];
    let mut hellos = 0;
    check!(read_batch(sv[1], &mut seen, w, h, &mut hellos) > 0);
    live_set_state(LIVE_PAIRING);
    let f = read_frame(sv[1]);
    check!(matches!(&f, Some((t, b)) if *t == LIVE_STATE as u8 && b.len() == 3 && b[0] == LIVE_PAIRING as u8));
    canvas_fill(&mut c, Rect { x: 0, y: 0, w: 64, h: 64 }, 0); // the code on the card
    live_push(&c, Rect { x: 0, y: 0, w: 64, h: 64 });
    canvas_fill(&mut c, Rect { x: 0, y: 0, w: 64, h: 64 }, 200); // the card gone, the screen as it was
    live_push(&c, Rect { x: 0, y: 0, w: 64, h: 64 });
    live_set_state(LIVE_AWAKE);
    let f = read_frame(sv[1]);
    check!(matches!(&f, Some((t, b)) if *t == LIVE_STATE as u8 && b.len() == 3 && b[0] == LIVE_AWAKE as u8));
    // Every tile looked at again, none of the card's sent: the next batch holds the one real change.
    canvas_fill(&mut c, Rect { x: 100, y: 100, w: 10, h: 10 }, 0);
    live_push(&c, Rect { x: 100, y: 100, w: 10, h: 10 });
    check!(read_batch(sv[1], &mut seen, w, h, &mut hellos) == 1);
    check!(seen[0] == 200 && seen[105 * w as usize + 105] == 0 && hellos == 1);
    live_stop();
    close(sv[1]);
    canvas_free(&mut c);
}

// The next frame that is not a tile or a batch end.
fn next_state(fd: i32, out: &mut [u8; 3]) -> bool {
    while let Some((type_, body)) = read_frame(fd) {
        let state = type_ == LIVE_STATE as u8 && body.len() == 3;
        if state {
            out.copy_from_slice(&body);
        }
        if state {
            return true;
        }
    }
    false
}

fn the_browser_is_told_when_the_colors_invert() {
    let (w, h) = (128, 128);
    let mut c = Canvas::default();
    check!(canvas_init(&mut c, w, h) == 0);
    canvas_fill(&mut c, Rect { x: 0, y: 0, w, h }, 230);
    check!(live_start(w, h) == 0);
    live_push(&c, Rect { x: 0, y: 0, w, h });
    live_set_invert(true); // inverted before the browser came: its first frame says so
    let sv = socket_pair();
    check!(live_take_viewer(sv[0]));
    close(sv[0]);
    check!(skip_head(sv[1]));
    let mut st = [0u8; 3];
    check!(next_state(sv[1], &mut st) && st[0] == LIVE_AWAKE as u8 && st[1] == 1);
    let mut seen = vec![0u8; (w * h) as usize];
    let mut hellos = 0;
    check!(read_batch(sv[1], &mut seen, w, h, &mut hellos) > 0 && seen[0] == 230); // (the frame as drawn)
    live_set_invert(false);
    check!(next_state(sv[1], &mut st) && st[0] == LIVE_AWAKE as u8 && st[1] == 0);
    live_set_invert(false); // no change, nothing told: the next frames are the next change's
    canvas_fill(&mut c, Rect { x: 10, y: 10, w: 10, h: 10 }, 0);
    live_push(&c, Rect { x: 10, y: 10, w: 10, h: 10 });
    let f = read_frame(sv[1]);
    check!(matches!(&f, Some((t, _)) if *t == LIVE_TILE_FRAME as u8));
    live_set_invert(true);
    live_stop(); // bye, with the colors as they are (told apart first, or only with it)
    let mut bye = false;
    while next_state(sv[1], &mut st) {
        if st[0] == LIVE_BYE as u8 {
            bye = st[1] == 1;
        }
    }
    check!(bye);
    close(sv[1]);
    canvas_free(&mut c);
}

fn the_browser_is_told_the_front_light_warmth() {
    let (w, h) = (64, 64);
    let mut c = Canvas::default();
    check!(canvas_init(&mut c, w, h) == 0);
    canvas_fill(&mut c, Rect { x: 0, y: 0, w, h }, 230);
    check!(live_start(w, h) == 0);
    live_push(&c, Rect { x: 0, y: 0, w, h });
    live_set_warmth(10); // set before the browser came: its first frame says so
    let sv = socket_pair();
    check!(live_take_viewer(sv[0]));
    close(sv[0]);
    check!(skip_head(sv[1]));
    let mut st = [0u8; 3];
    check!(next_state(sv[1], &mut st) && st[0] == LIVE_AWAKE as u8 && st[1] == 0 && st[2] == 10);
    let mut seen = vec![0u8; (w * h) as usize];
    let mut hellos = 0;
    check!(read_batch(sv[1], &mut seen, w, h, &mut hellos) > 0);
    live_set_warmth(10); // no change: nothing told
    live_set_warmth(24);
    check!(next_state(sv[1], &mut st) && st[0] == LIVE_AWAKE as u8 && st[2] == 24);
    live_set_warmth(300); // past what a byte holds: the most the frame can say short of none
    check!(next_state(sv[1], &mut st) && st[2] == (LIVE_NO_WARMTH - 1) as u8);
    live_set_warmth(-1); // no warm light (or not known)
    check!(next_state(sv[1], &mut st) && st[2] == LIVE_NO_WARMTH as u8);
    live_set_warmth(3);
    live_stop(); // bye, with the warmth as it is
    let mut bye = false;
    while next_state(sv[1], &mut st) {
        if st[0] == LIVE_BYE as u8 {
            bye = st[2] == 3;
        }
    }
    check!(bye);
    close(sv[1]);
    canvas_free(&mut c);
}

fn the_pen_word_packs_cuts_and_reads_back() {
    check!(live_pen_word(LIVE_PEN_HOVER, 10.4, 20.6, 300, 200) == (1u32 << 30 | 21u32 << 15 | 10u32));
    check!(live_pen_word(LIVE_PEN_AWAY, 10.0, 20.0, 300, 200) == 0);
    check!(live_pen_word(7, 10.0, 20.0, 300, 200) == 0); // (not a state: away)
    // Past the edges (a pen calibrated a little past the glass), and a NaN: cut to the screen.
    check!(live_pen_word(LIVE_PEN_TOUCH, -40.0, 9999.0, 300, 200) == (2u32 << 30 | 199u32 << 15));
    check!(live_pen_word(LIVE_PEN_TOUCH, f32::NAN, 5.0, 300, 200) == (2u32 << 30 | 5u32 << 15));
    let mut p = [0u8; 5];
    live_pen_payload(&mut p, live_pen_word(LIVE_PEN_TOUCH, 1859.0, 2479.0, 1860, 2480)); // the far corner
    check!(p[0] == 0x43 && p[1] == 0x07 && p[2] == 0xAF && p[3] == 0x09 && p[4] == LIVE_PEN_TOUCH as u8);
    live_pen_payload(&mut p, 0);
    check!(p[0] == 0 && p[1] == 0 && p[2] == 0 && p[3] == 0 && p[4] == LIVE_PEN_AWAY as u8);
}

// Nothing to read on fd for ms.
fn quiet(fd: i32, ms: i32) -> bool {
    let mut p = libc::pollfd { fd, events: libc::POLLIN, revents: 0 };
    // SAFETY: poll reads and writes the one pollfd of the local p.
    unsafe { libc::poll(&mut p, 1, ms) == 0 }
}

// The next pen frame (x, y, state into out), the frames before it skipped: the tiles and batch ends
// among them counted where asked.
fn next_pen(fd: i32, out: &mut [u32; 3], mut tiles: Option<&mut i32>, mut batches: Option<&mut i32>) -> bool {
    while let Some((type_, body)) = read_frame(fd) {
        let pen = type_ == LIVE_PEN_FRAME as u8 && body.len() == 5;
        if pen {
            out[0] = u16(&body);
            out[1] = u16(&body[2..]);
            out[2] = body[4] as u32;
        }
        if let Some(t) = tiles.as_deref_mut() {
            if type_ == LIVE_TILE_FRAME as u8 {
                *t += 1;
            }
        }
        if let Some(b) = batches.as_deref_mut() {
            if type_ == LIVE_BATCH as u8 {
                *b += 1;
            }
        }
        if pen {
            return true;
        }
    }
    false
}

fn a_viewer_sends_the_pen_after_the_tiles_and_only_its_newest_place() {
    let (w, h) = (300, 200);
    let mut c = Canvas::default();
    check!(canvas_init(&mut c, w, h) == 0);
    canvas_fill(&mut c, Rect { x: 0, y: 0, w, h }, 230);
    check!(live_start(w, h) == 0);
    live_push(&c, Rect { x: 0, y: 0, w, h });
    let sv = socket_pair();
    check!(live_take_viewer(sv[0]));
    close(sv[0]);
    check!(skip_head(sv[1]));
    let mut st = [0u8; 3];
    check!(next_state(sv[1], &mut st) && st[0] == LIVE_AWAKE as u8);
    let mut seen = vec![0u8; (w * h) as usize];
    let mut hellos = 0;
    check!(read_batch(sv[1], &mut seen, w, h, &mut hellos) > 0);
    check!(quiet(sv[1], 300)); // no pen yet: nothing (the browser starts with none)
    // Coming into range wakes the viewer, idle in its one-second wait, at once.
    let mut t0 = now_ms();
    live_pen(LIVE_PEN_HOVER, 10.0, 20.0);
    let mut pen = [0u32; 3];
    check!(next_pen(sv[1], &mut pen, None, None) && pen[0] == 10 && pen[1] == 20 && pen[2] == LIVE_PEN_HOVER as u32);
    check!(now_ms().wrapping_sub(t0) < 600);
    // A stroke's tile and the pen's place after it: the tile's batch first, then the pen.
    canvas_fill(&mut c, Rect { x: 70, y: 70, w: 20, h: 20 }, 0);
    live_push(&c, Rect { x: 70, y: 70, w: 20, h: 20 });
    live_pen(LIVE_PEN_TOUCH, 80.0, 80.0);
    let (mut tiles, mut batches) = (0, 0);
    check!(next_pen(sv[1], &mut pen, Some(&mut tiles), Some(&mut batches)) && pen[0] == 80 && pen[1] == 80 && pen[2] == LIVE_PEN_TOUCH as u32);
    check!(tiles == 1 && batches == 1);
    // Fifty places at once: the newest comes, with at most one between.
    for i in 0..50 {
        live_pen(LIVE_PEN_HOVER, 100.0 + i as f32, 50.0);
    }
    let mut frames = 0;
    loop {
        let got = next_pen(sv[1], &mut pen, None, None);
        let in_range = got && {
            frames += 1;
            frames <= 2
        };
        check!(in_range);
        if !(pen[0] != 149 && frames <= 2) {
            break;
        }
    }
    check!(pen[0] == 149 && pen[1] == 50 && pen[2] == LIVE_PEN_HOVER as u32);
    // Moving for 300 ms, a place every 5 ms: about 30 a second, never more, the last one last. The
    // bound is from the time it took, not the 300 ms asked for: under a loaded make -j the sleeps
    // run long, and every frame was sent between t0 and the moment the last one was read.
    t0 = now_ms();
    for i in 0..60 {
        live_pen(LIVE_PEN_HOVER, 10.0 + i as f32, 150.0);
        sleep_ms(5);
    }
    frames = 0;
    let mut last_read = t0;
    while !quiet(sv[1], 150) && next_pen(sv[1], &mut pen, None, None) {
        frames += 1;
        last_read = now_ms();
    }
    check!(frames >= 2 && frames <= (last_read.wrapping_sub(t0) / LIVE_PEN_MS) as i32 + 2);
    check!(frames < 60); // (60 places: fewer frames than that, whatever the load)
    check!(pen[0] == 69 && pen[1] == 150);
    // A pen that stays where it is is not told again; gone, it is told once.
    live_pen(LIVE_PEN_HOVER, 69.0, 150.0);
    check!(quiet(sv[1], 200));
    live_pen(LIVE_PEN_AWAY, 0.0, 0.0);
    check!(next_pen(sv[1], &mut pen, None, None) && pen[2] == LIVE_PEN_AWAY as u32);
    check!(quiet(sv[1], 300));
    live_stop();
    close(sv[1]);
    canvas_free(&mut c);
}

fn the_pen_is_never_sent_asleep_and_a_turn_takes_it_away() {
    let (w, h) = (128, 96);
    let mut c = Canvas::default();
    check!(canvas_init(&mut c, w, h) == 0);
    canvas_fill(&mut c, Rect { x: 0, y: 0, w, h }, 230);
    check!(live_start(w, h) == 0);
    live_push(&c, Rect { x: 0, y: 0, w, h });
    live_pen(LIVE_PEN_HOVER, 5.0, 6.0); // in range as the browser comes: told after its first batch
    let sv = socket_pair();
    check!(live_take_viewer(sv[0]));
    close(sv[0]);
    check!(skip_head(sv[1]));
    let mut pen = [0u32; 3];
    let (mut tiles, mut batches) = (0, 0);
    check!(next_pen(sv[1], &mut pen, Some(&mut tiles), Some(&mut batches)) && pen[0] == 5 && pen[1] == 6 && pen[2] == LIVE_PEN_HOVER as u32);
    check!(tiles > 0 && batches == 1);
    // Asleep: the state, and the pen is gone with it (the browser takes it away); awake again, it
    // is not told until it moves.
    live_set_state(LIVE_ASLEEP);
    let mut st = [0u8; 3];
    check!(next_state(sv[1], &mut st) && st[0] == LIVE_ASLEEP as u8);
    check!(quiet(sv[1], 200));
    live_set_state(LIVE_AWAKE);
    check!(next_state(sv[1], &mut st) && st[0] == LIVE_AWAKE as u8);
    check!(quiet(sv[1], 200));
    live_pen(LIVE_PEN_HOVER, 7.0, 8.0);
    check!(next_pen(sv[1], &mut pen, None, None) && pen[0] == 7 && pen[1] == 8);
    // The pairing card takes it away the same way.
    live_set_state(LIVE_PAIRING);
    check!(next_state(sv[1], &mut st) && st[0] == LIVE_PAIRING as u8);
    live_set_state(LIVE_AWAKE);
    check!(next_state(sv[1], &mut st) && st[0] == LIVE_AWAKE as u8);
    check!(quiet(sv[1], 200));
    // Turned: the hello, and no pen until it is told on the turned screen (96 wide now).
    live_pen(LIVE_PEN_HOVER, 100.0, 10.0);
    check!(next_pen(sv[1], &mut pen, None, None) && pen[0] == 100);
    live_resize(h, w);
    let mut t = Canvas::default();
    check!(canvas_init(&mut t, h, w) == 0);
    canvas_fill(&mut t, Rect { x: 0, y: 0, w: h, h: w }, 40);
    live_push(&t, Rect { x: 0, y: 0, w: h, h: w });
    // Every frame until the turned screen is all sent: one batch, or two when the viewer took its
    // snapshot while live_push was still copying (seen with a dozen of these tests at once). One
    // hello among them and no pen.
    let (mut hellos, mut pens) = (0, 0);
    while !quiet(sv[1], 200) {
        let Some((type_, _)) = read_frame(sv[1]) else { break };
        hellos += (type_ == LIVE_HELLO as u8) as i32;
        pens += (type_ == LIVE_PEN_FRAME as u8) as i32;
    }
    check!(hellos == 1 && pens == 0);
    live_pen(LIVE_PEN_HOVER, 100.0, 120.0);
    check!(next_pen(sv[1], &mut pen, None, None) && pen[0] == 95 && pen[1] == 120); // (x cut to the new width)
    live_stop();
    close(sv[1]);
    canvas_free(&mut t);
    canvas_free(&mut c);
}

fn no_viewer_for_ten_minutes_turns_it_off() {
    check!(live_start(64, 64) == 0);
    let mut idle = true;
    let t = now_ms();
    check!(live_poll(t, &mut idle) == 0 && !idle);
    check!(live_poll(t.wrapping_add(LIVE_IDLE_OFF_MS - 1000), &mut idle) == 0 && !idle);
    check!(live_poll(t.wrapping_add(LIVE_IDLE_OFF_MS + 1000), &mut idle) == 0 && idle);
    live_stop();
    check!(live_poll(t.wrapping_add(LIVE_IDLE_OFF_MS + 1000), &mut idle) == 0 && !idle); // off: nothing to turn off
}

fn u32_(p: &[u8]) -> u32 {
    u16(p) | u16(&p[2..]) << 16
}

fn next_kind(fd: i32, kind: i32) -> Option<Vec<u8>> {
    while let Some((type_, body)) = read_frame(fd) {
        if type_ == kind as u8 {
            return Some(body);
        }
    }
    None
}

// The samples of the next samples frames, in order: (frame body, offset of the sample); the frames'
// own heads (u32 ink epoch, u32 reset) are checked by the callers that care.
fn next_samples(fd: i32, n: usize, frames: &mut Vec<Vec<u8>>) -> Vec<(u32, [u8; LIVE_SAMPLE_BYTES])> {
    let mut out = Vec::new();
    while out.len() < n {
        let Some(body) = next_kind(fd, LIVE_SAMPLES) else { break };
        check!(body.len() >= 8 && (body.len() - 8) % LIVE_SAMPLE_BYTES == 0 && body.len() <= 8 + LIVE_SAMPLE_BYTES * LIVE_PEN_BATCH);
        for at in (8..body.len()).step_by(LIVE_SAMPLE_BYTES) {
            let mut one = [0u8; LIVE_SAMPLE_BYTES];
            one.copy_from_slice(&body[at..at + LIVE_SAMPLE_BYTES]);
            out.push((u32_(&body), one));
        }
        frames.push(body);
    }
    out
}

fn fast_samples_arrive_in_order_without_a_present() {
    check!(live_start(300, 200) == 0);
    let sv = socket_pair();
    check!(live_take_pen_viewer(sv[0]));
    close(sv[0]);
    check!(skip_head(sv[1]));
    let hello = next_kind(sv[1], LIVE_HELLO);
    check!(matches!(&hello, Some(b) if b.len() == 12 && u16(b) == LIVE_VERSION && u16(&b[2..]) == 300 && u16(&b[4..]) == 200 && u32_(&b[8..]) == 1));
    let b = next_kind(sv[1], LIVE_SAMPLES);
    check!(matches!(&b, Some(b) if b.len() == 8 && u32_(b) == 1 && u32_(&b[4..]) == 1));
    let style = LiveStyle {
        enabled: 1,
        tool: 2,
        width_min: 1.0,
        width_max: 8.0,
        pressure_ceiling: 1.0,
        pressure_gamma: 1.0,
        opacity_min: 0.5,
        opacity_max: 1.0,
        clip: Rect { x: 0, y: 0, w: 300, h: 200 },
        ..Default::default()
    };
    let mut first = 0;
    for i in 0..60 {
        let e = InEvent {
            type_: if i != 0 { InType::PenMove } else { InType::PenDown },
            x: i as f32,
            y: 50.0,
            pressure: 0.5,
            tilt_x: -30.0,
            tilt_y: 40.0,
            eraser: true,
            side_button: true,
            t_ms: 500,
            k_ms: 200 + i as u32,
            k_us: 200_000 + 250 * i as u32, // (four frames a millisecond: each its own identity)
            ..Default::default()
        };
        let seq = live_sample(&e, &style);
        if i == 0 {
            first = seq;
        }
    }
    live_wake_samples();
    let b = next_kind(sv[1], LIVE_STYLE);
    check!(matches!(&b, Some(b) if b.len() == LIVE_STYLE_BYTES));
    let mut expected = [0u8; LIVE_STYLE_BYTES];
    live_style_payload(&mut expected, &style);
    check!(matches!(&b, Some(b) if b[..] == expected[..]));
    // All of them, in order; the ones that piled up go several to a frame.
    let mut frames = Vec::new();
    let got = next_samples(sv[1], 60, &mut frames);
    check!(got.len() == 60 && frames.len() < 60);
    for (i, (epoch, s)) in got.iter().enumerate() {
        let i = i as u32;
        check!(*epoch == 1);
        check!(first == 200_000 && u32_(s) == first + 250 * i && u32_(&s[4..]) == 200 + i);
        check!(u16(&s[8..]) == i && u16(&s[10..]) == 50 && u16(&s[12..]) == 32768);
        check!(s[14] as i8 == -30 && s[15] == 40 && s[16] == LIVE_PEN_TOUCH as u8);
        check!(s[17] == (LIVE_ERASER | LIVE_SIDE | LIVE_DRAW | if i != 0 { 0 } else { LIVE_DOWN }));
    }
    check!(quiet(sv[1], 50)); // no tile was needed, or sent, to release the pen
    live_set_state(LIVE_PAIRING);
    let mut st = [0u8; 3];
    check!(next_state(sv[1], &mut st) && st[0] == LIVE_PAIRING as u8);
    let secret = InEvent { type_: InType::PenDown, x: 100.0, y: 100.0, t_ms: 999, ..Default::default() };
    check!(live_sample(&secret, &style) == 0);
    live_wake_samples();
    check!(quiet(sv[1], 50));
    live_set_state(LIVE_AWAKE);
    // Awake again: a new ink epoch (each state change moved it on), told by a hello.
    let hello = next_kind(sv[1], LIVE_HELLO);
    check!(matches!(&hello, Some(b) if u32_(&b[8..]) == 3));
    let b = next_kind(sv[1], LIVE_SAMPLES);
    check!(matches!(&b, Some(b) if b.len() == 8 && u32_(b) == 3));
    check!(quiet(sv[1], 50)); // a resume cannot replay a previous epoch's stroke
    live_stop();
    close(sv[1]);
}

fn a_ring_overrun_is_explicit_and_keeps_the_newest_samples() {
    check!(live_start(300, 200) == 0);
    let sv = socket_pair();
    check!(live_take_pen_viewer(sv[0]));
    close(sv[0]);
    check!(skip_head(sv[1]));
    check!(next_kind(sv[1], LIVE_SAMPLES).is_some());
    // No wake until all writes: the reader is in poll, so it sees a complete overrun.
    let style = LiveStyle::default();
    let mut last = 0;
    for i in 0..LIVE_RING + 10 {
        let e = InEvent { type_: InType::PenMove, x: (i % 300) as f32, t_ms: i as u32, k_us: 1 + i as u32, ..Default::default() };
        last = live_sample(&e, &style);
    }
    live_wake_samples();
    let mut frames = Vec::new();
    let got = next_samples(sv[1], LIVE_RING, &mut frames);
    check!(!frames.is_empty() && u32_(&frames[0][4..]) == 1); // (the reset, on the first frame only)
    check!(frames.iter().skip(1).all(|f| u32_(&f[4..]) == 0));
    check!(got.len() == LIVE_RING && u32_(&got[0].1) == last - LIVE_RING as u32 + 1);
    check!(got.windows(2).all(|w| u32_(&w[1].1) == u32_(&w[0].1) + 1) && u32_(&got[LIVE_RING - 1].1) == last);
    // A backlog of 512 goes as frames of LIVE_PEN_BATCH, not as 512 sends.
    check!(frames.len() <= LIVE_RING / LIVE_PEN_BATCH + 1);
    live_stop();
    close(sv[1]);
}

fn batch_acknowledges_only_presented_input_even_with_newer_samples() {
    check!(live_start(64, 64) == 0);
    let mut c = Canvas::default();
    check!(canvas_init(&mut c, 64, 64) == 0);
    canvas_fill(&mut c, Rect { x: 0, y: 0, w: 64, h: 64 }, 255);
    let style = LiveStyle { enabled: 1, ..Default::default() };
    let mut e = InEvent { type_: InType::PenDown, x: 5.0, y: 6.0, t_ms: 123, k_us: 7000, ..Default::default() };
    let rendered = live_sample(&e, &style);
    canvas_fill(&mut c, Rect { x: 5, y: 6, w: 1, h: 1 }, 0);
    live_push(&c, Rect { x: 0, y: 0, w: 64, h: 64 });
    live_presented(rendered, 123);
    e.x = 7.0;
    e.t_ms = 124;
    e.k_us = 7001;
    check!(live_sample(&e, &style) == rendered + 1); // pumped ahead, not dispatched to the editor
    let sv = socket_pair();
    check!(live_take_viewer(sv[0]));
    close(sv[0]);
    check!(skip_head(sv[1]));
    let b = next_kind(sv[1], LIVE_BATCH);
    check!(matches!(&b, Some(b) if b.len() == 20 && u32_(&b[8..]) == rendered && u32_(&b[12..]) == 123 && u32_(&b[16..]) == 1));
    // A no-op event still retires its preview, even when no tile changed.
    live_presented(rendered + 1, 124);
    let b = next_kind(sv[1], LIVE_BATCH);
    check!(matches!(&b, Some(b) if b.len() == 20 && u32_(&b[8..]) == rendered + 1 && u32_(&b[16..]) == 1));
    // A page turned (live_reset): the watermark goes, and the next present (none of the new page's
    // samples processed yet) says the new ink epoch in a batch of its own, vouching for no sample.
    live_reset();
    live_presented(0, 0);
    let b = next_kind(sv[1], LIVE_BATCH);
    check!(matches!(&b, Some(b) if b.len() == 20 && u32_(&b[8..]) == 0 && u32_(&b[16..]) == 2));
    live_presented(0, 0); // (nothing new: no batch)
    check!(quiet(sv[1], 100));
    live_stop();
    close(sv[1]);
    canvas_free(&mut c);
}

// A page turned, a pinch, a screen changed (live_reset): the pen's ink epoch moves on, never the
// screen's. The tiles that did not change are not sent again, and the browser keeps its picture.
fn a_reset_moves_the_ink_epoch_and_sends_no_tiles() {
    let (w, h) = (300, 200);
    let mut c = Canvas::default();
    check!(canvas_init(&mut c, w, h) == 0);
    for y in 0..h as usize {
        for x in 0..w as usize {
            c.px[y * c.stride as usize + x] = ((x * 5 + y * 11) & 0xFF) as u8;
        }
    }
    check!(live_start(w, h) == 0);
    live_push(&c, Rect { x: 0, y: 0, w, h });
    let sv = socket_pair();
    check!(live_take_viewer(sv[0]));
    close(sv[0]);
    check!(skip_head(sv[1]));
    let pv = socket_pair();
    check!(live_take_pen_viewer(pv[0]));
    close(pv[0]);
    check!(skip_head(pv[1]));
    let b = next_kind(pv[1], LIVE_HELLO);
    check!(matches!(&b, Some(b) if u32_(&b[8..]) == 1));
    let mut seen = vec![0u8; (w * h) as usize];
    let mut hellos = 0;
    check!(read_batch(sv[1], &mut seen, w, h, &mut hellos) > 0 && hellos == 1);
    check!(quiet(pv[1], 50) || next_kind(pv[1], LIVE_SAMPLES).is_some_and(|b| b.len() == 8));
    // The pen hovering before the page turns: the newest record when the new epoch's hello goes.
    let style = LiveStyle { enabled: 1, clip: Rect { x: 0, y: 0, w, h }, ..Default::default() };
    let hover = InEvent { type_: InType::PenHover, x: 5.0, y: 5.0, ..Default::default() };
    live_sample(&hover, &style);
    live_wake_samples();
    let mut frames = Vec::new();
    check!(next_samples(pv[1], 1, &mut frames).len() == 1);
    live_reset();
    check!(quiet(sv[1], 200)); // no hello, no tile: nothing changed on the screen
    let b = next_kind(pv[1], LIVE_HELLO);
    check!(matches!(&b, Some(b) if b.len() == 12 && u32_(&b[8..]) == 2));
    let b = next_kind(pv[1], LIVE_SAMPLES);
    check!(matches!(&b, Some(b) if b.len() == 8 && u32_(b) == 2 && u32_(&b[4..]) == 1));
    // The next stroke, in the new epoch: its pen-down keeps LIVE_DOWN (the newest record at the
    // hello was the old epoch's hover, which is never sent).
    for i in 0..3 {
        let e = InEvent { type_: if i != 0 { InType::PenMove } else { InType::PenDown }, x: 10.0 + i as f32, y: 10.0, pressure: 0.5, ..Default::default() };
        live_sample(&e, &style);
    }
    live_wake_samples();
    let got = next_samples(pv[1], 3, &mut frames);
    check!(got.len() == 3 && got.iter().all(|(epoch, _)| *epoch == 2));
    check!(got.first().is_some_and(|(_, s)| s[17] & LIVE_DOWN != 0) && got.iter().skip(1).all(|(_, s)| s[17] & LIVE_DOWN == 0));
    // A change after it: one tile, in a batch, with the same hello as before.
    canvas_fill(&mut c, Rect { x: 70, y: 70, w: 20, h: 20 }, 0);
    live_push(&c, Rect { x: 70, y: 70, w: 20, h: 20 });
    check!(read_batch(sv[1], &mut seen, w, h, &mut hellos) == 1 && hellos == 1);
    check!(seen[(80 * w + 80) as usize] == 0);
    live_stop();
    close(sv[1]);
    close(pv[1]);
    canvas_free(&mut c);
}

// A browser that connects in the middle of a stroke gets the pen's newest place without LIVE_DOWN,
// even when that sample is the stroke's pen-down: it never starts ink it saw only part of.
fn a_pen_viewer_joining_mid_stroke_gets_no_pen_down() {
    check!(live_start(300, 200) == 0);
    let style = LiveStyle { enabled: 1, clip: Rect { x: 0, y: 0, w: 300, h: 200 }, ..Default::default() };
    let hover = InEvent { type_: InType::PenHover, x: 40.0, y: 40.0, ..Default::default() };
    live_sample(&hover, &style);
    let down = InEvent { type_: InType::PenDown, x: 50.0, y: 50.0, pressure: 0.5, k_us: 90_000, ..Default::default() };
    let seq = live_sample(&down, &style);
    let sv = socket_pair();
    check!(live_take_pen_viewer(sv[0]));
    close(sv[0]);
    check!(skip_head(sv[1]));
    let mut frames = Vec::new();
    let got = next_samples(sv[1], 1, &mut frames);
    check!(got.len() == 1 && u32_(&got[0].1) == seq && got[0].1[16] == LIVE_PEN_TOUCH as u8);
    check!(got[0].1[17] & LIVE_DOWN == 0 && got[0].1[17] & LIVE_DRAW != 0);
    let mv = InEvent { type_: InType::PenMove, x: 60.0, y: 50.0, pressure: 0.5, k_us: 90_001, ..Default::default() };
    live_sample(&mv, &style);
    live_wake_samples();
    let got = next_samples(sv[1], 1, &mut frames);
    check!(got.len() == 1 && u32_(&got[0].1) == seq + 1);
    live_stop();
    close(sv[1]);
}

// A batch cut short (the state moved on while it was being sent) vouches for nothing. The screen
// is noise, 2.25 MB of PNGs, far more than the socket holds: the viewer is still sending when the
// first tile is read, and the pairing card comes then.
fn a_batch_cut_short_acknowledges_nothing() {
    let (w, h) = (1536, 1536);
    let mut c = Canvas::default();
    check!(canvas_init(&mut c, w, h) == 0);
    let mut seed: u32 = 12345;
    for v in c.px.iter_mut() {
        seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
        *v = (seed >> 16) as u8;
    }
    check!(live_start(w, h) == 0);
    live_push(&c, Rect { x: 0, y: 0, w, h });
    let style = LiveStyle { enabled: 1, ..Default::default() };
    let e = InEvent { type_: InType::PenDown, x: 5.0, y: 6.0, t_ms: 77, ..Default::default() };
    let s = live_sample(&e, &style);
    live_presented(s, 77);
    let sv = socket_pair();
    check!(live_take_viewer(sv[0]));
    close(sv[0]);
    check!(skip_head(sv[1]));
    let (mut tiles, mut batch, mut paired) = (0, None, false);
    while let Some((type_, body)) = read_frame(sv[1]) {
        if type_ == LIVE_TILE_FRAME as u8 {
            tiles += 1;
            if !paired {
                live_set_state(LIVE_PAIRING);
                paired = true;
            }
        } else if type_ == LIVE_BATCH as u8 {
            batch = Some(body);
            break;
        }
    }
    let all = live_tiles_across(w) * live_tiles_across(h) / live_tiles_across(w); // (one run a row)
    check!(paired && tiles >= 1 && tiles < all);
    check!(matches!(&batch, Some(b) if b.len() == 20 && u32_(&b[8..]) == 0 && u32_(&b[12..]) == 0 && u32_(&b[16..]) == 0));
    let mut st = [0u8; 3];
    check!(next_state(sv[1], &mut st) && st[0] == LIVE_PAIRING as u8);
    live_stop();
    close(sv[1]);
    canvas_free(&mut c);
}

// ---- the raw digitizer, read by the pen viewers themselves ----

fn raw_ev(sec: u64, usec: u64, type_: u16, code: u16, value: i32) -> LiveInputEvent {
    LiveInputEvent { sec: sec as libc::c_ulong, usec: usec as libc::c_ulong, type_, code, value }
}
const SYN: u16 = 0;
const KEY: u16 = 1;
const ABS: u16 = 3;

// A recorded shape of the Scribe's raw digitizer (WacomDigitizer: x, y, pressure 0..4095, tilt in
// hundredths of a degree, BTN_TOOL_PEN for proximity, BTN_TOUCH for contact), as frames.
fn raw_frames(evs: &[LiveInputEvent]) -> (LiveRawPen, Vec<LiveRawFrame>) {
    let mut st = LiveRawPen::default();
    let mut out = Vec::new();
    for e in evs {
        let mut f = LiveRawFrame::default();
        if live_raw_event(&mut st, 0, e, &mut f) {
            out.push(f);
        }
    }
    (st, out)
}

fn the_raw_digitizer_decodes_into_frames() {
    let t = 1_000u64; // (seconds: the identity is the time's microseconds, wrapping)
    let evs = [
        // the pen comes near: no frame before it is placed
        raw_ev(t, 0, KEY, 0x140, 1),
        raw_ev(t, 0, SYN, 0, 0),
        raw_ev(t, 4000, ABS, 0, 1000),
        raw_ev(t, 4000, ABS, 1, 2000),
        raw_ev(t, 4000, SYN, 0, 0),
        // touches (pressure, and BTN_TOUCH)
        raw_ev(t, 8000, KEY, 0x14a, 1),
        raw_ev(t, 8000, ABS, 0x18, 900),
        raw_ev(t, 8000, ABS, 0x1a, -4000),
        raw_ev(t, 8000, SYN, 0, 0),
        raw_ev(t, 8250, ABS, 0, 1010),
        raw_ev(t, 8250, SYN, 0, 0),
        // a drop: the half frame is not sent, the one after it is a gap
        raw_ev(t, 9000, ABS, 0, 1500),
        raw_ev(t, 9000, SYN, 3, 0),
        raw_ev(t, 9500, ABS, 0, 1600),
        raw_ev(t, 9500, SYN, 0, 0),
        raw_ev(t, 10000, ABS, 0, 1700),
        raw_ev(t, 10000, SYN, 0, 0),
        // lifts, the side button held, then goes away
        raw_ev(t, 12000, KEY, 0x14a, 0),
        raw_ev(t, 12000, ABS, 0x18, 0),
        raw_ev(t, 12000, KEY, 0x14b, 1),
        raw_ev(t, 12000, SYN, 0, 0),
        raw_ev(t, 16000, KEY, 0x140, 0),
        raw_ev(t, 16000, SYN, 0, 0),
        raw_ev(t, 20000, SYN, 0, 0),
        // the eraser end comes
        raw_ev(t, 30000, KEY, 0x141, 1),
        raw_ev(t, 30000, ABS, 0, 5),
        raw_ev(t, 30000, SYN, 0, 0),
    ];
    let (st, f) = raw_frames(&evs);
    check!(f.len() == 7);
    let us = |usec: u64| (t * 1_000_000 + usec) as u32;
    check!(f[0].state == LIVE_PEN_HOVER && f[0].us == us(4000) && f[0].ms == (t * 1000 + 4) as u32 && f[0].x == 1000 && f[0].y == 2000 && !f[0].down);
    check!(f[1].state == LIVE_PEN_TOUCH && f[1].down && f[1].us == us(8000) && f[1].p == 900 && f[1].tx == -4000);
    check!(f[2].state == LIVE_PEN_TOUCH && !f[2].down && f[2].us == us(8250) && f[2].x == 1010 && f[2].ms == (t * 1000 + 8) as u32);
    check!(f[3].us == us(10000) && f[3].gap && f[3].x == 1700 && !f[3].down); // (9500 closed the dropped frame)
    check!(f[4].state == LIVE_PEN_HOVER && f[4].side && !f[4].gap);
    check!(f[5].state == LIVE_PEN_AWAY && f[5].us == us(16000)); // (once: the empty frame after says nothing)
    check!(f[6].state == LIVE_PEN_HOVER && f[6].eraser && f[6].x == 5);
    check!(st.was_near && !st.was_touch);
}

fn raw_map(path: &str) -> LiveRawMap {
    let mut m = LiveRawMap {
        m: [0.5, 0.0, 10.0, 0.0, -0.25, 300.0],
        rmin: [0, 0],
        rmax: [1000, 1000],
        pmin: 0,
        pmax: 4095,
        tilt_min: [-9000, -9000],
        tilt_max: [9000, 9000],
        tm: [0.0, -1.0, 1.0, 0.0],
        ..Default::default()
    };
    m.path.set(path);
    m
}

fn a_raw_frame_is_placed_and_weighed_by_the_map() {
    let m = raw_map("");
    let f = LiveRawFrame { us: 77, ms: 5, state: LIVE_PEN_TOUCH, down: true, side: true, x: 200, y: 400, p: 4095, tx: 4500, ty: -9000, ..Default::default() };
    let mut s = [0u8; LIVE_SAMPLE_BYTES];
    live_raw_sample(&m, 300, 400, &f, &mut s);
    check!(u32_(&s) == 77 && u32_(&s[4..]) == 5);
    check!(u16(&s[8..]) == 110 && u16(&s[10..]) == 200 && u16(&s[12..]) == 65535);
    // tilt (45, -90) degrees turned a quarter: (90, 45)
    check!(s[14] as i8 == 90 && s[15] as i8 == 45 && s[16] == LIVE_PEN_TOUCH as u8 && s[17] == (LIVE_SIDE | LIVE_DOWN));
    // cut to the raw ranges, then to the screen; hovering weighs nothing
    let far = LiveRawFrame { state: LIVE_PEN_HOVER, x: 5000, y: -50, p: 3000, ..f };
    live_raw_sample(&m, 300, 400, &far, &mut s);
    check!(u16(&s[8..]) == 299 && u16(&s[10..]) == 300 && u16(&s[12..]) == 0 && s[17] == LIVE_SIDE | LIVE_DOWN);
}

// A raw frame's sample at (x, y) with its flags, as live_raw_sample makes it.
fn gate_sample(f: &LiveRawFrame, x: u32, y: u32) -> [u8; LIVE_SAMPLE_BYTES] {
    let mut s = [0u8; LIVE_SAMPLE_BYTES];
    s[..4].copy_from_slice(&f.us.to_le_bytes());
    s[8..10].copy_from_slice(&(x as u16).to_le_bytes());
    s[10..12].copy_from_slice(&(y as u16).to_le_bytes());
    s[16] = f.state as u8;
    s[17] = (if f.eraser { LIVE_ERASER } else { 0 }) | (if f.side { LIVE_SIDE } else { 0 }) | (if f.side2 { LIVE_SIDE2 } else { 0 }) | (if f.down { LIVE_DOWN } else { 0 }) | (if f.gap { LIVE_GAP } else { 0 });
    s
}

// One frame through the gate: (identity, flags, the style's tool and enabled) of each sample sent.
fn gate(g: &mut LiveRawGate, cap: &LiveCapture, blocked: bool, f: &LiveRawFrame, x: u32, y: u32) -> Vec<(u32, u8, u8, u8)> {
    let mut out = Vec::new();
    live_raw_gate(g, cap, blocked, f, &gate_sample(f, x, y), &mut |s, st| out.push((u32_(s), s[17], st.tool, st.enabled)));
    out
}

fn raw_gate_follows_the_capture_state() {
    let style = LiveStyle { enabled: 1, tool: 3, exclude: Rect { x: 250, y: 0, w: 50, h: 400 }, ..Default::default() };
    let eraser = LiveStyle { enabled: 1, tool: 9, ..style };
    let mut cap = LiveCapture { ok: true, editor: true, handled: 0, styles: [[style; 4], [eraser; 4]] };
    let mut g = LiveRawGate::default();
    let down = LiveRawFrame { us: 100, state: LIVE_PEN_TOUCH, down: true, ..Default::default() };
    let mv = LiveRawFrame { us: 104, state: LIVE_PEN_TOUCH, ..Default::default() };
    let up = LiveRawFrame { us: 108, state: LIVE_PEN_HOVER, ..Default::default() };
    check!(gate(&mut g, &cap, false, &down, 10, 10) == vec![(100, LIVE_DOWN | LIVE_DRAW, 3, 1)] && g.ui == 0);
    let er = LiveRawFrame { eraser: true, ..mv };
    check!(gate(&mut g, &cap, false, &er, 10, 10) == vec![(104, LIVE_ERASER | LIVE_DRAW, 9, 1)]);
    // A stroke drawing goes on through the main loop's UI work (each event it handles is that for a
    // moment), and a gap starts it again.
    check!(gate(&mut g, &cap, true, &mv, 10, 10) == vec![(104, LIVE_DRAW, 3, 1)]);
    let gap = LiveRawFrame { us: 105, gap: true, ..mv };
    check!(gate(&mut g, &cap, false, &gap, 10, 10) == vec![(105, LIVE_GAP | LIVE_DOWN | LIVE_DRAW, 3, 1)]);
    // No editor, or a style that draws nothing: the stroke ends, and stays ended.
    cap.editor = false;
    check!(gate(&mut g, &cap, false, &mv, 10, 10) == vec![(104, 0, 0, 0)] && !g.drawing);
    cap.editor = true;
    check!(gate(&mut g, &cap, false, &mv, 10, 10) == vec![(104, 0, 3, 1)]);
    check!(gate(&mut g, &cap, false, &up, 10, 10) == vec![(108, 0, 3, 1)]);
    // A pen-down on the panel is UI: nothing draws until the main loop has handled it.
    let tap = LiveRawFrame { us: 200, ..down };
    check!(gate(&mut g, &cap, false, &tap, 260, 10) == vec![(200, LIVE_DOWN, 0, 0)] && g.ui == 200);
    check!(gate(&mut g, &cap, false, &up, 260, 10) == vec![(108, 0, 0, 0)]);
    // The next stroke comes before it is handled: held, not judged (it used to be taken for UI too,
    // and all its quick ink was lost: the page starts a stroke only at a pen-down) ...
    let next = LiveRawFrame { us: 300, ..down };
    let next2 = LiveRawFrame { us: 304, ..mv };
    check!(gate(&mut g, &cap, false, &next, 10, 10).is_empty() && g.ui == 200 && g.pending == 300);
    cap.handled = 150; // (an event before the tap's)
    check!(gate(&mut g, &cap, false, &next2, 12, 10).is_empty() && g.n == 2);
    // ... and drawn from its pen-down once the tap is handled.
    cap.handled = 250;
    let next3 = LiveRawFrame { us: 308, ..mv };
    check!(gate(&mut g, &cap, false, &next3, 14, 10) == vec![(300, LIVE_DOWN | LIVE_DRAW, 3, 1), (304, LIVE_DRAW, 3, 1), (308, LIVE_DRAW, 3, 1)]);
    check!(g.ui == 0 && g.drawing && g.n == 0);
    // A pen-down while the main loop is in UI work (a pen-up's commit) is held until it is out of it.
    check!(gate(&mut g, &cap, false, &LiveRawFrame { us: 310, ..up }, 14, 10).len() == 1);
    let d4 = LiveRawFrame { us: 400, ..down };
    check!(gate(&mut g, &cap, true, &d4, 20, 20).is_empty());
    check!(gate(&mut g, &cap, true, &LiveRawFrame { us: 404, ..mv }, 21, 20).is_empty());
    check!(gate(&mut g, &cap, false, &LiveRawFrame { us: 408, ..mv }, 22, 20).iter().map(|s| (s.0, s.1)).collect::<Vec<_>>()
        == vec![(400, LIVE_DOWN | LIVE_DRAW), (404, LIVE_DRAW), (408, LIVE_DRAW)]);
    // Judged after that work, where it came down: on the panel (the work moved it there), it is UI.
    check!(gate(&mut g, &cap, false, &LiveRawFrame { us: 410, ..up }, 22, 20).len() == 1);
    let d5 = LiveRawFrame { us: 500, ..down };
    check!(gate(&mut g, &cap, true, &d5, 270, 20).is_empty());
    check!(gate(&mut g, &cap, false, &LiveRawFrame { us: 504, ..mv }, 30, 20) == vec![(500, LIVE_DOWN, 0, 0), (504, 0, 0, 0)] && g.ui == 500);
    // Lifted before it could be judged: its samples go on undrawn.
    cap.handled = 500;
    check!(gate(&mut g, &cap, false, &LiveRawFrame { us: 510, ..up }, 30, 20).len() == 1 && g.ui == 0);
    check!(gate(&mut g, &cap, true, &LiveRawFrame { us: 600, ..down }, 30, 20).is_empty());
    check!(gate(&mut g, &cap, true, &LiveRawFrame { us: 610, ..up }, 30, 20).iter().map(|s| s.1).collect::<Vec<_>>() == vec![LIVE_DOWN, 0]);
    // The hold runs out: the oldest go on undrawn, and the stroke's first drawn sample says LIVE_DOWN.
    check!(gate(&mut g, &cap, true, &LiveRawFrame { us: 1000, ..down }, 30, 20).is_empty());
    for i in 1..LIVE_RAW_HOLD as u32 {
        check!(gate(&mut g, &cap, true, &LiveRawFrame { us: 1000 + i, ..mv }, 30, 20).is_empty());
    }
    let spill = gate(&mut g, &cap, true, &LiveRawFrame { us: 1000 + LIVE_RAW_HOLD as u32, ..mv }, 30, 20);
    check!(spill.len() == LIVE_RAW_HOLD && spill[0].1 == LIVE_DOWN && spill.iter().all(|s| s.1 & LIVE_DRAW == 0));
    let go = gate(&mut g, &cap, false, &LiveRawFrame { us: 2000, ..mv }, 30, 20);
    check!(go.len() == 2 && go[0] == (1000 + LIVE_RAW_HOLD as u32, LIVE_DOWN | LIVE_DRAW, 3, 1) && go[1] == (2000, LIVE_DRAW, 3, 1));
    // Forgotten (a new epoch): nothing held, no stroke under way.
    g.forget();
    check!(gate(&mut g, &cap, false, &mv, 10, 10) == vec![(104, 0, 3, 1)]);
}

fn ev_bytes(e: &LiveInputEvent) -> Vec<u8> {
    // SAFETY: LiveInputEvent is repr(C) plain integers with no padding read as anything but bytes;
    // the slice covers exactly the one value, alive for the copy.
    unsafe { std::slice::from_raw_parts(e as *const LiveInputEvent as *const u8, std::mem::size_of::<LiveInputEvent>()) }.to_vec()
}

fn write_events(fd: i32, evs: &[LiveInputEvent]) {
    let mut b = Vec::new();
    for e in evs {
        b.extend(ev_bytes(e));
    }
    // SAFETY: write(2) of the local buffer to the FIFO.
    let n = unsafe { libc::write(fd, b.as_ptr() as *const libc::c_void, b.len()) };
    check!(n == b.len() as isize);
}

fn mono_now() -> (u64, u64) {
    let mut ts = libc::timespec { tv_sec: 0, tv_nsec: 0 };
    // SAFETY: clock_gettime writes one timespec into the local.
    unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut ts) };
    (ts.tv_sec as u64, ts.tv_nsec as u64 / 1000)
}

// A pen viewer reads the raw node itself (here a FIFO standing in for it): its samples come while the
// main loop does nothing at all (no live_sample, no wake: as stalled as it can be), placed by the
// published map, drawn with the published style, known by the frame's microseconds; and the tiles'
// watermark names the same identity once the main loop's own event of that frame is presented.
fn a_pen_viewer_reads_the_raw_node_itself() {
    let dir = tl_tmp();
    let _ = std::fs::create_dir_all(&dir);
    // SAFETY: getpid takes no arguments.
    let path = cformat!("%s/rawpen-%d", &dir, unsafe { libc::getpid() });
    let c = std::ffi::CString::new(path.clone()).unwrap();
    // SAFETY: unlink and mkfifo read the NUL-terminated path.
    unsafe { libc::unlink(c.as_ptr()) };
    check!(unsafe { libc::mkfifo(c.as_ptr(), 0o600) } == 0);
    // SAFETY: open reads the path; O_RDWR keeps the FIFO open for the viewer's reads.
    let w = unsafe { libc::open(c.as_ptr(), libc::O_RDWR | libc::O_NONBLOCK) };
    check!(w >= 0);
    if w < 0 {
        return; // (no FIFO here: nothing below could be read)
    }
    check!(live_start(300, 400) == 0);
    let style = LiveStyle { enabled: 1, tool: 2, clip: Rect { x: 0, y: 0, w: 300, h: 400 }, exclude: Rect { x: 280, y: 0, w: 20, h: 400 }, ..Default::default() };
    live_set_capture(&LiveCapture { ok: true, editor: true, handled: 0, styles: [[style; 4]; 2] });
    live_set_raw_map(Some(&raw_map(&path)));
    let sv = socket_pair();
    check!(live_take_pen_viewer(sv[0]));
    close(sv[0]);
    check!(skip_head(sv[1]));
    check!(next_kind(sv[1], LIVE_HELLO).is_some());
    check!(next_kind(sv[1], LIVE_SAMPLES).is_some());
    sleep_ms(100); // (the viewer has opened the node)
    let (sec, usec) = mono_now();
    let mut evs = vec![raw_ev(sec, usec, KEY, 0x140, 1), raw_ev(sec, usec, ABS, 0, 100), raw_ev(sec, usec, ABS, 1, 800), raw_ev(sec, usec, SYN, 0, 0)];
    evs.extend([raw_ev(sec, usec + 4000, KEY, 0x14a, 1), raw_ev(sec, usec + 4000, ABS, 0x18, 2048), raw_ev(sec, usec + 4000, SYN, 0, 0)]);
    for i in 1..=10u64 {
        evs.extend([raw_ev(sec, usec + 4000 + i * 3000, ABS, 0, 100 + 10 * i as i32), raw_ev(sec, usec + 4000 + i * 3000, SYN, 0, 0)]);
    }
    write_events(w, &evs);
    let mut frames = Vec::new();
    let got = next_samples(sv[1], 12, &mut frames);
    check!(got.len() == 12);
    let id = |du: u64| (sec * 1_000_000 + usec + du) as u32;
    check!(got.len() == 12 && u32_(&got[0].1) == id(0) && got[0].1[16] == LIVE_PEN_HOVER as u8 && u16(&got[0].1[8..]) == 60 && u16(&got[0].1[10..]) == 100);
    check!(got.len() == 12 && u32_(&got[1].1) == id(4000) && got[1].1[17] == LIVE_DOWN | LIVE_DRAW && u16(&got[1].1[12..]).abs_diff(32776) <= 1);
    check!(got.len() == 12 && got[11].1[17] == LIVE_DRAW && u32_(&got[11].1) == id(34000) && u16(&got[11].1[8..]) == 110);
    // The main loop's own event of the stroke's last frame, presented: the batch names that identity.
    let mut c2 = Canvas::default();
    check!(canvas_init(&mut c2, 300, 400) == 0);
    canvas_fill(&mut c2, Rect { x: 0, y: 0, w: 300, h: 400 }, 255);
    live_push(&c2, Rect { x: 0, y: 0, w: 300, h: 400 });
    let e = InEvent { type_: InType::PenMove, x: 110.0, y: 100.0, k_us: id(34000), ..Default::default() };
    let s = live_sample(&e, &style);
    check!(s == id(34000));
    live_presented(s, 9);
    let tv = socket_pair();
    check!(live_take_viewer(tv[0]));
    close(tv[0]);
    check!(skip_head(tv[1]));
    let b = next_kind(tv[1], LIVE_BATCH);
    check!(matches!(&b, Some(b) if u32_(&b[8..]) == id(34000)));
    // A pen-down on the panel (the style's exclusion) draws nothing, and nothing after it does until
    // the main loop says it handled that tap.
    let (sec, usec) = mono_now();
    write_events(w, &[raw_ev(sec, usec, KEY, 0x14a, 0), raw_ev(sec, usec, ABS, 0x18, 0), raw_ev(sec, usec, SYN, 0, 0),
        raw_ev(sec, usec + 4000, ABS, 0, 590), raw_ev(sec, usec + 4000, KEY, 0x14a, 1), raw_ev(sec, usec + 4000, ABS, 0x18, 900), raw_ev(sec, usec + 4000, SYN, 0, 0)]);
    let got = next_samples(sv[1], 2, &mut frames);
    check!(got.len() == 2 && got[1].1[17] & LIVE_DOWN != 0 && got[1].1[17] & LIVE_DRAW == 0 && u16(&got[1].1[8..]) == 305 - 6);
    // Sleep retires the raw reader. A replaced device at the same path must be opened again
    // on wake, even when the calibration/map did not change. No held pen crosses that boundary.
    live_set_state(LIVE_ASLEEP);
    let mut st = [0u8; 3];
    check!(next_state(sv[1], &mut st) && st[0] == LIVE_ASLEEP as u8);
    let (sec, usec) = mono_now();
    write_events(w, &[raw_ev(sec, usec, ABS, 0, 300), raw_ev(sec, usec, SYN, 0, 0)]);
    check!(quiet(sv[1], 100));
    // SAFETY: remove/recreate the test's NUL-terminated FIFO path; old open fds stay valid.
    unsafe { libc::unlink(c.as_ptr()) };
    // SAFETY: mkfifo reads the same path and creates only this test's node.
    check!(unsafe { libc::mkfifo(c.as_ptr(), 0o600) } == 0);
    // SAFETY: open reads the path; the returned descriptor is checked and closed below.
    let replacement = unsafe { libc::open(c.as_ptr(), libc::O_RDWR | libc::O_NONBLOCK) };
    check!(replacement >= 0);
    live_set_state(LIVE_AWAKE);
    check!(next_kind(sv[1], LIVE_HELLO).is_some());
    check!(next_kind(sv[1], LIVE_SAMPLES).is_some());
    let (sec, usec) = mono_now();
    write_events(replacement, &[raw_ev(sec, usec, KEY, 0x140, 1), raw_ev(sec, usec, ABS, 0, 200),
        raw_ev(sec, usec, ABS, 1, 800), raw_ev(sec, usec, SYN, 0, 0)]);
    let got = if quiet(sv[1], 2000) { Vec::new() } else { next_samples(sv[1], 1, &mut frames) };
    check!(got.len() == 1 && u32_(&got[0].1) == (sec * 1_000_000 + usec) as u32
        && got[0].1[16] == LIVE_PEN_HOVER as u8 && got[0].1[17] & (LIVE_DOWN | LIVE_DRAW) == 0);
    // Unlike the simulator's O_RDWR FIFO, a real read-only input source can hang up. Retire
    // it and use the main reader's ring; never spin on HUP/EOF and starve the app.
    #[cfg(not(feature = "sim"))]
    {
        close(replacement);
        sleep_ms(100);
        let e = InEvent { type_: InType::PenHover, x: 90.0, y: 110.0, k_us: 7654321, ..Default::default() };
        live_sample(&e, &style);
        live_wake_samples();
        let got = if quiet(sv[1], 2000) { Vec::new() } else { next_samples(sv[1], 1, &mut frames) };
        check!(got.len() == 1 && u32_(&got[0].1) == 7654321);
    }
    live_stop();
    close(sv[1]);
    close(tv[1]);
    close(w);
    #[cfg(feature = "sim")]
    close(replacement);
    // SAFETY: unlink reads the path.
    unsafe { libc::unlink(c.as_ptr()) };
    canvas_free(&mut c2);
}

// The editor's panel and the clock's corner as layers (frame 9): sent with the batch they changed in,
// whole, as PNGs of their rect; gone as w = 0; sent again after a hello.
fn a_layer_comes_with_the_tiles_and_goes() {
    check!(live_start(128, 128) == 0);
    let mut c = Canvas::default();
    check!(canvas_init(&mut c, 128, 128) == 0);
    canvas_fill(&mut c, Rect { x: 0, y: 0, w: 128, h: 128 }, 255);
    canvas_fill(&mut c, Rect { x: 100, y: 10, w: 20, h: 30 }, 40);
    live_push(&c, Rect { x: 0, y: 0, w: 128, h: 128 });
    live_layer(LIVE_LAYER_PANEL, &c, Rect { x: 100, y: 10, w: 20, h: 30 });
    let sv = socket_pair();
    check!(live_take_viewer(sv[0]));
    close(sv[0]);
    check!(skip_head(sv[1]));
    let b = next_kind(sv[1], LIVE_LAYER);
    let layer_ok = |b: &Option<Vec<u8>>, id: u8, r: (u32, u32, u32, u32), v: u8| -> bool {
        let Some(b) = b else { return false };
        if b.len() < 10 || b[0] != id || (u16(&b[2..]), u16(&b[4..]), u16(&b[6..]), u16(&b[8..])) != r {
            return false;
        }
        if r.2 == 0 {
            return b.len() == 10;
        }
        matches!(decode_png(&b[10..]), Some((px, w, h)) if w == r.2 as i32 && h == r.3 as i32 && px.iter().all(|&p| p == v))
    };
    check!(layer_ok(&b, 0, (100, 10, 20, 30), 40));
    check!(next_kind(sv[1], LIVE_BATCH).is_some());
    // the corner, then the panel gone
    canvas_fill(&mut c, Rect { x: 0, y: 0, w: 30, h: 12 }, 0);
    live_layer(LIVE_LAYER_STATUS, &c, Rect { x: 0, y: 0, w: 30, h: 12 });
    check!(layer_ok(&next_kind(sv[1], LIVE_LAYER), 1, (0, 0, 30, 12), 0));
    live_layer(LIVE_LAYER_PANEL, &c, RECT_EMPTY);
    check!(layer_ok(&next_kind(sv[1], LIVE_LAYER), 0, (0, 0, 0, 0), 0));
    live_layer(LIVE_LAYER_PANEL, &c, RECT_EMPTY); // (gone already: nothing)
    check!(next_kind(sv[1], LIVE_BATCH).is_some());
    check!(quiet(sv[1], 100));
    // A turn takes them away (the turned screen's flush tells them again).
    live_resize(128, 128);
    let mut kinds = Vec::new();
    while let Some((t, body)) = read_frame(sv[1]) {
        kinds.push((t, body.len()));
        if t == LIVE_BATCH as u8 {
            break;
        }
    }
    check!(kinds.iter().filter(|k| k.0 == LIVE_LAYER as u8 && k.1 == 10).count() == 2);
    live_stop();
    close(sv[1]);
    canvas_free(&mut c);
}

// The parts of a run that changed: whole tiles, each stretch of them one part, the screen's edge
// cut as the run is.
fn a_run_splits_into_the_tiles_that_changed() {
    let w = 300;
    let sent = vec![255u8; (w * 128) as usize];
    let r = Rect { x: 0, y: 64, w: 300, h: 64 }; // a whole row: four tiles of 64 and one of 44
    let mut snap = vec![255u8; (r.w * r.h) as usize];
    let mut out = [RECT_EMPTY; 8];
    check!(live_changed_parts(&sent, w, &snap, r, &mut out) == 0);
    snap[10 * 300 + 70] = 0; // tile 1
    snap[63 * 300 + 299] = 0; // tile 4, the narrow one, its last pixel
    let n = live_changed_parts(&sent, w, &snap, r, &mut out);
    check!(n == 2 && out[0] == Rect { x: 64, y: 64, w: 64, h: 64 } && out[1] == Rect { x: 256, y: 64, w: 44, h: 64 });
    snap[0] = 0; // tile 0 joins tile 1
    let n = live_changed_parts(&sent, w, &snap, r, &mut out);
    check!(n == 2 && out[0] == Rect { x: 0, y: 64, w: 128, h: 64 } && out[1].x == 256);
    // No room for a second part: the first reaches over the rest that changed.
    let n = live_changed_parts(&sent, w, &snap, r, &mut out[..1]);
    check!(n == 1 && out[0] == Rect { x: 0, y: 64, w: 300, h: 64 });
    // A run that starts inside the row compares the screen where it is.
    let r = Rect { x: 128, y: 0, w: 128, h: 64 };
    let mut sent = vec![255u8; (w * 128) as usize];
    let mut snap = vec![255u8; (r.w * r.h) as usize];
    sent[5 * 300 + 200] = 9;
    snap[5 * 128 + 72] = 9; // the same pixel, as sent
    check!(live_changed_parts(&sent, w, &snap, r, &mut out) == 0);
    snap[5 * 128 + 72] = 10;
    let n = live_changed_parts(&sent, w, &snap, r, &mut out);
    check!(n == 1 && out[0] == Rect { x: 192, y: 0, w: 64, h: 64 });
}

// The tile frames up to the next batch end: (x, y, w, h, pixels, the frame's bytes); None when the
// stream ended.
fn batch_tiles(fd: i32) -> Option<Vec<(i32, i32, i32, i32, Vec<u8>, usize)>> {
    let mut tiles = Vec::new();
    loop {
        let (type_, body) = read_frame(fd)?;
        if type_ == LIVE_TILE_FRAME as u8 {
            let px = decode_png(&body[8..]).map(|p| p.0).unwrap_or_default();
            tiles.push((u16(&body) as i32, u16(&body[2..]) as i32, u16(&body[4..]) as i32, u16(&body[6..]) as i32, px, 5 + body.len()));
        } else if type_ == LIVE_BATCH as u8 {
            return Some(tiles);
        }
    }
}

// A redraw of a whole row (a toolbar's) that changed one tile of it sends that tile alone.
fn a_wide_redraw_sends_only_the_tiles_that_changed() {
    let (w, h) = (300, 200);
    let mut c = Canvas::default();
    check!(canvas_init(&mut c, w, h) == 0);
    canvas_fill(&mut c, Rect { x: 0, y: 0, w, h }, 200);
    check!(live_start(w, h) == 0);
    live_push(&c, Rect { x: 0, y: 0, w, h });
    let sv = socket_pair();
    check!(live_take_viewer(sv[0]));
    close(sv[0]);
    check!(skip_head(sv[1]));
    check!(batch_tiles(sv[1]).is_some_and(|t| !t.is_empty())); // the screen
    canvas_fill(&mut c, Rect { x: 140, y: 10, w: 8, h: 8 }, 0);
    canvas_fill(&mut c, Rect { x: 270, y: 30, w: 4, h: 4 }, 0);
    live_push(&c, Rect { x: 0, y: 0, w, h: 64 });
    let t = batch_tiles(sv[1]).unwrap_or_default();
    check!(t.len() == 2);
    check!(t.first().is_some_and(|t| (t.0, t.1, t.2, t.3) == (128, 0, 64, 64) && t.4.len() == 64 * 64 && t.4[10 * 64 + 12] == 0 && t.4[0] == 200));
    check!(t.get(1).is_some_and(|t| (t.0, t.1, t.2, t.3) == (256, 0, 44, 64) && t.4.len() == 44 * 64 && t.4[30 * 44 + 14] == 0));
    // and the next one compares with what went, tile by tile
    canvas_fill(&mut c, Rect { x: 0, y: 0, w: 2, h: 2 }, 0);
    live_push(&c, Rect { x: 0, y: 0, w, h: 64 });
    let t = batch_tiles(sv[1]).unwrap_or_default();
    check!(t.len() == 1 && t.first().is_some_and(|t| (t.0, t.1, t.2, t.3) == (0, 0, 64, 64)));
    live_stop();
    close(sv[1]);
    canvas_free(&mut c);
}

// A TCP connection over loopback, as a browser's is: (the app's end, the browser's), the browser's
// receive buffer rcvbuf bytes (a slow link's window).
fn tcp_pair(rcvbuf: i32) -> [i32; 2] {
    // SAFETY: plain socket calls on descriptors made here, with local sockaddr_in and int values.
    unsafe {
        let l = libc::socket(libc::AF_INET, libc::SOCK_STREAM, 0);
        let mut sa: libc::sockaddr_in = std::mem::zeroed();
        #[cfg(target_vendor = "apple")]
        {
            sa.sin_len = std::mem::size_of::<libc::sockaddr_in>() as u8;
        }
        sa.sin_family = libc::AF_INET as libc::sa_family_t;
        sa.sin_addr.s_addr = 0x7F00_0001u32.to_be();
        let mut len = std::mem::size_of::<libc::sockaddr_in>() as libc::socklen_t;
        check!(libc::bind(l, &sa as *const libc::sockaddr_in as *const libc::sockaddr, len) == 0);
        check!(libc::listen(l, 1) == 0);
        check!(libc::getsockname(l, &mut sa as *mut libc::sockaddr_in as *mut libc::sockaddr, &mut len) == 0);
        let c = libc::socket(libc::AF_INET, libc::SOCK_STREAM, 0);
        libc::setsockopt(c, libc::SOL_SOCKET, libc::SO_RCVBUF, &rcvbuf as *const i32 as *const libc::c_void, 4);
        check!(libc::connect(c, &sa as *const libc::sockaddr_in as *const libc::sockaddr, len) == 0);
        let a = libc::accept(l, std::ptr::null_mut(), std::ptr::null_mut());
        check!(a >= 0);
        libc::close(l);
        [a, c]
    }
}

// Version v of a screen that does not compress (each its own noise), one pixel of it.
fn noise(v: u32, x: i32, y: i32) -> u8 {
    let mut k = (x as u32).wrapping_mul(0x9E37_79B1) ^ (y as u32).wrapping_mul(0x85EB_CA77) ^ v.wrapping_mul(0xC2B2_AE3D);
    k ^= k >> 15;
    k = k.wrapping_mul(0x2C1B_3C6D);
    (k >> 24) as u8
}

// A browser that stops reading (a busy link) while the screen changes again and again, and a pen
// stroke goes on: the pen's samples still come at once, and once the browser reads again it gets
// the newest screen after little more than what the tile socket held, never the versions between.
// (What the browser's own receive buffer holds is not the app's to bound: loopback's grows to
// hundreds of KiB whatever SO_RCVBUF says, so it is measured and left out.)
fn a_slow_browser_gets_the_newest_tiles_and_the_pen_never_waits() {
    let (w, h) = (512, 512);
    let mut c = Canvas::default();
    check!(canvas_init(&mut c, w, h) == 0);
    let paint = |c: &mut Canvas, v: u32| {
        for y in 0..h {
            for x in 0..w {
                c.px[(y * c.stride + x) as usize] = noise(v, x, y);
            }
        }
    };
    paint(&mut c, 0);
    check!(live_start(w, h) == 0);
    live_push(&c, Rect { x: 0, y: 0, w, h });
    let tv = tcp_pair(8 * 1024);
    check!(live_take_viewer(tv[0]));
    close(tv[0]);
    let pv = socket_pair();
    check!(live_take_pen_viewer(pv[0]));
    close(pv[0]);
    check!(skip_head(tv[1]) && skip_head(pv[1]));
    check!(next_kind(pv[1], LIVE_SAMPLES).is_some()); // (the pen stream's start)
    let mut shown = vec![0u8; (w * h) as usize];
    let mut hellos = 0;
    check!(read_batch(tv[1], &mut shown, w, h, &mut hellos) > 0 && hellos == 1);
    let style = LiveStyle { enabled: 1, width_min: 1.0, width_max: 4.0, pressure_ceiling: 1.0, pressure_gamma: 1.0, opacity_max: 1.0, clip: Rect { x: 0, y: 0, w, h }, ..Default::default() };
    // The browser reads nothing now: 20 new screens, 40 ms apart, a pen sample with each.
    const VERSIONS: u32 = 20;
    let mut pen_ms = Vec::new();
    for v in 1..=VERSIONS {
        paint(&mut c, v);
        live_push(&c, Rect { x: 0, y: 0, w, h });
        let e = InEvent { type_: if v == 1 { InType::PenDown } else { InType::PenMove }, x: v as f32, y: 9.0, pressure: 0.5, k_us: 1000 * v, k_ms: v, ..Default::default() };
        let t0 = now_ms();
        live_sample(&e, &style);
        live_wake_samples();
        let mut got = false;
        while !got && now_ms().wrapping_sub(t0) < 1000 && !quiet(pv[1], 1000) {
            let Some((type_, body)) = read_frame(pv[1]) else { break };
            got = type_ == LIVE_SAMPLES as u8 && (8..body.len()).step_by(LIVE_SAMPLE_BYTES).any(|at| u32_(&body[at..]) == 1000 * v);
        }
        check!(got);
        pen_ms.push(now_ms().wrapping_sub(t0));
        sleep_ms(40);
    }
    pen_ms.sort();
    eprintln!("    pen sample to the browser with the tile socket full: median {} ms, max {} ms", pen_ms[pen_ms.len() / 2], pen_ms[pen_ms.len() - 1]);
    check!(pen_ms[pen_ms.len() - 1] < 25);
    let mut held: libc::c_int = 0;
    // SAFETY: ioctl FIONREAD writes one int to the local held.
    check!(unsafe { libc::ioctl(tv[1], libc::FIONREAD, &mut held as *mut libc::c_int) } == 0);
    // Reading again: everything up to a quiet stream, each tile known by its version.
    let mut seen = std::collections::BTreeMap::new();
    let (mut stale, mut fresh) = (0usize, 0usize);
    let mut quiet_after = false;
    while !quiet_after {
        let Some(tiles) = batch_tiles(tv[1]) else { break };
        for (x, y, tw, th, px, bytes) in tiles {
            let v = (0..=VERSIONS).rev().find(|&v| px.len() == (tw * th) as usize && (0..th).all(|r| (0..tw).all(|q| px[(r * tw + q) as usize] == noise(v, x + q, y + r))));
            *seen.entry(v).or_insert(0) += 1;
            if v == Some(VERSIONS) { fresh += bytes } else { stale += bytes }
            for r in 0..th.min(h - y) {
                for q in 0..tw.min(w - x) {
                    if let Some(&p) = px.get((r * tw + q) as usize) {
                        shown[((y + r) * w + x + q) as usize] = p;
                    }
                }
            }
        }
        quiet_after = quiet(tv[1], 300);
    }
    eprintln!("    after the pause: {} bytes of tiles from older screens ({} of them in the browser's buffer), {} of the newest; versions {:?}", stale, held, fresh, seen);
    let newest = (0..h).all(|y| (0..w).all(|x| shown[(y * w + x) as usize] == noise(VERSIONS, x, y)));
    check!(newest);
    // What the socket held (the batch it was in the middle of, at most its buffer's worth) and the
    // newest: not the screens between.
    check!(seen.keys().all(|&v| v == Some(VERSIONS) || v.is_some_and(|v| v < VERSIONS / 2)));
    check!(stale.saturating_sub(held as usize) < 160 * 1024);
    live_stop();
    close(tv[1]);
    close(pv[1]);
    canvas_free(&mut c);
}

#[test]
fn test_live() {
    run!(marks_cover_exactly_the_tiles_a_rect_touches);
    run!(runs_join_neighbours_in_a_row_and_stay_on_screen);
    run!(frame_headers_are_little_endian);
    run!(a_reconnect_reclaims_an_ended_viewer_without_a_poll_tick);
    run!(a_viewer_sends_the_screen_then_what_changed);
    run!(nothing_is_sent_while_the_pairing_card_is_up);
    run!(the_browser_is_told_when_the_colors_invert);
    run!(the_browser_is_told_the_front_light_warmth);
    run!(the_pen_word_packs_cuts_and_reads_back);
    run!(a_viewer_sends_the_pen_after_the_tiles_and_only_its_newest_place);
    run!(the_pen_is_never_sent_asleep_and_a_turn_takes_it_away);
    run!(no_viewer_for_ten_minutes_turns_it_off);
    run!(fast_samples_arrive_in_order_without_a_present);
    run!(a_ring_overrun_is_explicit_and_keeps_the_newest_samples);
    run!(batch_acknowledges_only_presented_input_even_with_newer_samples);
    run!(a_reset_moves_the_ink_epoch_and_sends_no_tiles);
    run!(a_pen_viewer_joining_mid_stroke_gets_no_pen_down);
    run!(a_batch_cut_short_acknowledges_nothing);
    run!(the_raw_digitizer_decodes_into_frames);
    run!(a_raw_frame_is_placed_and_weighed_by_the_map);
    run!(raw_gate_follows_the_capture_state);
    run!(a_pen_viewer_reads_the_raw_node_itself);
    run!(a_layer_comes_with_the_tiles_and_goes);
    run!(a_run_splits_into_the_tiles_that_changed);
    run!(a_wide_redraw_sends_only_the_tiles_that_changed);
    run!(a_slow_browser_gets_the_newest_tiles_and_the_pen_never_waits);
    done();
}

#[test]
fn second_button_capture_styles_prevent_provisional_drag_ink() {
    let style = LiveStyle { enabled: 1, tool: 3, ..Default::default() };
    let mut cap = LiveCapture { ok: true, editor: true, styles: [[style; 4]; 2], ..Default::default() };
    // Button2 is a non-ink drag, alone and together with Button.
    cap.styles[0][2] = LiveStyle::default();
    cap.styles[0][3] = LiveStyle::default();
    for side in [false, true] {
        let mut g = LiveRawGate::default();
        let frame = LiveRawFrame { us: 100, state: LIVE_PEN_TOUCH, down: true,
            side, side2: true, ..Default::default() };
        let output = gate(&mut g, &cap, false, &frame, 10, 10);
        assert_eq!(output.len(), 1);
        assert_eq!(output[0].1 & LIVE_DRAW, 0);
        assert_eq!(output[0].1 & LIVE_SIDE2, LIVE_SIDE2);
        assert_eq!(g.ui, 100);
    }
    let mut g = LiveRawGate::default();
    let ordinary = LiveRawFrame { us: 200, state: LIVE_PEN_TOUCH, down: true, side: true, ..Default::default() };
    assert_ne!(gate(&mut g, &cap, false, &ordinary, 10, 10)[0].1 & LIVE_DRAW, 0);
    let mut raw = LiveRawPen::default();
    let mut decoded = LiveRawFrame::default();
    let event = LiveInputEvent { type_: 1, code: 0x14c, value: 1, ..Default::default() };
    assert!(!live_raw_event(&mut raw, 0, &event, &mut decoded));
    assert!(raw.side2 && !raw.side);
}
