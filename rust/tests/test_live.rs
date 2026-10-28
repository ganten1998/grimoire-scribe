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
            check!(len == 12 && u16(&body) == 2 && u16(&body[2..]) == w as u32 && u16(&body[4..]) == h as u32 && u16(&body[6..]) == LIVE_TILE as u32);
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
    check!(read_batch(sv[1], &mut seen, w, h, &mut hellos) > 0);
    if seen[105 * w as usize + 105] != 0 {
        check!(read_batch(sv[1], &mut seen, w, h, &mut hellos) == 1);
    }
    check!(seen[0] == 200 && seen[105 * w as usize + 105] == 0);
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
    // A fresh epoch on wake: the picture is sent again, without a pen.
    while !quiet(sv[1], 100) {
        let f = read_frame(sv[1]);
        check!(matches!(&f, Some((t, _)) if *t != LIVE_PEN_FRAME as u8));
    }
    live_pen(LIVE_PEN_HOVER, 7.0, 8.0);
    check!(next_pen(sv[1], &mut pen, None, None) && pen[0] == 7 && pen[1] == 8);
    // The pairing card takes it away the same way.
    live_set_state(LIVE_PAIRING);
    check!(next_state(sv[1], &mut st) && st[0] == LIVE_PAIRING as u8);
    live_set_state(LIVE_AWAKE);
    check!(next_state(sv[1], &mut st) && st[0] == LIVE_AWAKE as u8);
    // A fresh epoch on wake: the picture is sent again, without a pen.
    while !quiet(sv[1], 100) {
        let f = read_frame(sv[1]);
        check!(matches!(&f, Some((t, _)) if *t != LIVE_PEN_FRAME as u8));
    }
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

fn fast_samples_arrive_in_order_without_a_present() {
    check!(live_start(300, 200) == 0);
    let sv = socket_pair();
    check!(live_take_pen_viewer(sv[0]));
    close(sv[0]);
    check!(skip_head(sv[1]));
    let b = next_kind(sv[1], LIVE_SAMPLES);
    check!(matches!(&b, Some(b) if b.len() == 8 && u32_(&b[4..]) == 1));
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
    for i in 0..60u32 {
        let Some(body) = next_kind(sv[1], LIVE_SAMPLES) else {
            check!(false);
            break;
        };
        check!(body.len() == 28);
        if body.len() != 28 {
            break;
        }
        check!(u32_(&body[8..]) == first + i && u32_(&body[12..]) == 200 + i);
        check!(u16(&body[16..]) == i && u16(&body[18..]) == 50 && u16(&body[20..]) == 32768);
        check!(body[22] as i8 == -30 && body[23] == 40 && body[24] == LIVE_PEN_TOUCH as u8);
        check!(body[25] == (LIVE_ERASER | LIVE_SIDE | LIVE_DRAW | if i != 0 { 0 } else { LIVE_DOWN }));
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
    let b = next_kind(sv[1], LIVE_SAMPLES);
    check!(matches!(&b, Some(b) if b.len() == 8));
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
        let e = InEvent { type_: InType::PenMove, x: (i % 300) as f32, t_ms: i as u32, ..Default::default() };
        last = live_sample(&e, &style);
    }
    live_wake_samples();
    let b = next_kind(sv[1], LIVE_SAMPLES);
    check!(matches!(&b, Some(b) if b.len() == 28 && u32_(&b[4..]) == 1));
    let mut seq = b.as_deref().map(|b| u32_(&b[8..])).unwrap_or(0);
    check!(seq == last - LIVE_RING as u32 + 1);
    while seq != last {
        let Some(body) = next_kind(sv[1], LIVE_SAMPLES) else {
            check!(false);
            break;
        };
        check!(body.len() == 28 && u32_(&body[8..]) == seq + 1);
        seq = u32_(&body[8..]);
    }
    live_stop();
    close(sv[1]);
}

fn batch_acknowledges_only_presented_input_even_with_newer_samples() {
    check!(live_start(64, 64) == 0);
    let mut c = Canvas::default();
    check!(canvas_init(&mut c, 64, 64) == 0);
    canvas_fill(&mut c, Rect { x: 0, y: 0, w: 64, h: 64 }, 255);
    let style = LiveStyle { enabled: 1, ..Default::default() };
    let mut e = InEvent { type_: InType::PenDown, x: 5.0, y: 6.0, t_ms: 123, ..Default::default() };
    let rendered = live_sample(&e, &style);
    canvas_fill(&mut c, Rect { x: 5, y: 6, w: 1, h: 1 }, 0);
    live_push(&c, Rect { x: 0, y: 0, w: 64, h: 64 });
    live_presented(rendered, 123);
    e.x = 7.0;
    e.t_ms = 124;
    check!(live_sample(&e, &style) == rendered + 1); // pumped ahead, not dispatched to the editor
    let sv = socket_pair();
    check!(live_take_viewer(sv[0]));
    close(sv[0]);
    check!(skip_head(sv[1]));
    let b = next_kind(sv[1], LIVE_BATCH);
    check!(matches!(&b, Some(b) if b.len() == 16 && u32_(&b[8..]) == rendered && u32_(&b[12..]) == 123));
    // A no-op event still retires its preview, even when no tile changed.
    live_presented(rendered + 1, 124);
    let b = next_kind(sv[1], LIVE_BATCH);
    check!(matches!(&b, Some(b) if b.len() == 16 && u32_(&b[8..]) == rendered + 1));
    live_stop();
    close(sv[1]);
    canvas_free(&mut c);
}

#[test]
fn test_live() {
    run!(marks_cover_exactly_the_tiles_a_rect_touches);
    run!(runs_join_neighbours_in_a_row_and_stay_on_screen);
    run!(frame_headers_are_little_endian);
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
    done();
}
