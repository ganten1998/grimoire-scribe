// Live updates must not turn gray paper around a stroke into a temporary black/white halo.
// (the C tests index by position, and so do these ports of them)
#![allow(clippy::needless_range_loop, clippy::too_many_arguments)]
mod testlib;
use grimoire::prelude::*;
use testlib::*;

const PW: i32 = 9;
const PH: i32 = 7;
const ROOM: usize = 512;
const GUARD: usize = 16;

fn position(x: i32, y: i32, rotation: i32, flip: bool) -> (i32, i32) {
    let (mut fx, mut fy) = match rotation {
        1 => (y, PH - 1 - x),
        2 => (PW - 1 - x, PH - 1 - y),
        3 => (PW - 1 - y, x),
        _ => (x, y),
    };
    if flip {
        fx = PW - 1 - fx;
        fy = PH - 1 - fy;
    }
    (fx, fy)
}

// (The C passes fb = memory + GUARD; here the memory and the guard offset, since the rows are
// offsets into one slice.)
fn frame(memory: &mut [u8], previous: &mut [u8], src: &[u8], bpp: i32, rotation: i32, flip: bool, invert: bool, fast: bool, known: bool) {
    let w = if rotation & 1 != 0 { PH } else { PW };
    let h = if rotation & 1 != 0 { PW } else { PH };
    let bytes = bpp / 8;
    let stride = PW * bytes + 8;
    for y in 0..h {
        let (fx, fy) = position(0, y, rotation, flip);
        let (nx, ny) = position(1, y, rotation, flip);
        let step = ((ny - fy) * stride + (nx - fx) * bytes) as isize;
        let pstep = ((ny - fy) * PW + nx - fx) as isize;
        fbpack_row(
            memory,
            GUARD + (fy * stride + fx * bytes) as usize,
            step,
            Some(&mut *previous),
            (fy * PW + fx) as usize,
            pstep,
            &src[(y * w) as usize..],
            w,
            0,
            y,
            bpp,
            invert,
            fast,
            known,
        );
    }
}

fn gray_paper_survives_live_ink_and_overlap() {
    for bpp in [8, 32] {
        for rotation in 0..4 {
            for flip in [false, true] {
                for invert in [false, true] {
                    let mut memory = [0xCDu8; ROOM];
                    let mut before = [0u8; ROOM];
                    let mut live = [0u8; ROOM];
                    let mut previous = [0u8; (PW * PH) as usize];
                    let mut src = [0u8; (PW * PH) as usize];
                    let w = if rotation & 1 != 0 { PH } else { PW };
                    let bytes = (bpp / 8) as usize;
                    let stride = (PW as usize) * bytes + 8;
                    for i in 0..(PW * PH) as usize {
                        src[i] = (40 + i % 180) as u8;
                    }
                    frame(&mut memory, &mut previous, &src, bpp, rotation, flip, invert, false, false);
                    before.copy_from_slice(&memory);
                    // One ink sample in the middle of a dirty rectangle containing gray paper.
                    let written = (2 * w + 3) as usize;
                    src[written] = 0;
                    src[written + 1] = 73;
                    frame(&mut memory, &mut previous, &src, bpp, rotation, flip, invert, true, true);
                    for i in 0..(PW * PH) as usize {
                        let (fx, fy) = position(i as i32 % w, i as i32 / w, rotation, flip);
                        let off = GUARD + fy as usize * stride + fx as usize * bytes;
                        let was = if invert { 255 - before[off] as i32 } else { before[off] as i32 };
                        let now = if invert { 255 - memory[off] as i32 } else { memory[off] as i32 };
                        if i != written && i != written + 1 {
                            check!(memory[off..off + bytes] == before[off..off + bytes]);
                            check!(memory[off] != 0 && memory[off] != 255);
                        } else if i == written {
                            check!(now == 0);
                        } else {
                            // Lighter than the paper it shows: white, or the paper until it is due.
                            check!(now == 255 || memory[off..off + bytes] == before[off..off + bytes]);
                            check!(src[i] as i32 > was);
                        }
                        check!(previous[(fy * PW + fx) as usize] == src[i]);
                    }
                    // An overlapping refresh must neither erase earlier live ink nor dither paper.
                    live.copy_from_slice(&memory);
                    frame(&mut memory, &mut previous, &src, bpp, rotation, flip, invert, true, true);
                    check!(live == memory);
                    // Settle restores the exact gray values, including the stroke's soft edge.
                    frame(&mut memory, &mut previous, &src, bpp, rotation, flip, invert, false, true);
                    for i in 0..(PW * PH) as usize {
                        let (fx, fy) = position(i as i32 % w, i as i32 / w, rotation, flip);
                        let p = GUARD + fy as usize * stride + fx as usize * bytes;
                        check!(memory[p] == if invert { 255 - src[i] } else { src[i] });
                        if bpp == 32 {
                            check!(memory[p + 1] == memory[p] && memory[p + 2] == memory[p] && memory[p + 3] == 255);
                        }
                    }
                    for i in 0..GUARD {
                        check!(memory[i] == 0xCD);
                    }
                    for y in 0..PH as usize {
                        for x in (PW as usize) * bytes..stride {
                            check!(memory[GUARD + y * stride + x] == 0xCD);
                        }
                    }
                    for i in GUARD + (PH as usize) * stride..ROOM {
                        check!(memory[i] == 0xCD);
                    }
                }
            }
        }
    }
}

// Pixel at (x, y) of an 8x8 tile as fbpack_row leaves it, Grimoire's inversion undone.
fn shown_at(fb: &[u8], stride: usize, bytes: usize, x: usize, y: usize, invert: bool) -> i32 {
    let v = fb[y * stride + x * bytes] as i32;
    if invert {
        255 - v
    } else {
        v
    }
}

// One fast frame of the whole tile at a uniform logical value. Returns pixels whose bytes changed.
fn tile_frame(fb: &mut [u8], previous: &mut [u8], changes: &mut [u8; 64], bpp: i32, invert: bool, v: i32) -> i32 {
    let bytes = (bpp / 8) as usize;
    let stride = 8 * bytes;
    let mut changed = 0;
    let mut before = [0u8; 8 * 8 * 4];
    let src = [v as u8; 8];
    before.copy_from_slice(&fb[..8 * 8 * 4]);
    for y in 0..8 {
        fbpack_row(fb, y * stride, bytes as isize, Some(&mut *previous), y * 8, 1, &src, 8, 0, y as i32, bpp, invert, true, true);
    }
    for i in 0..64 {
        if fb[i * bytes..i * bytes + bytes] != before[i * bytes..i * bytes + bytes] {
            changes[i] += 1;
            changed += 1;
        }
    }
    changed
}

// A DU transition cannot be restarted until it ends. Ink going down on gray paper must not send a
// pixel to white first and to black a waveform later: each pixel moves once, toward its value, and
// the tile keeps the darkness of the logical gray (what the settle will show).
fn live_ink_moves_each_pixel_once_toward_its_value() {
    let papers = [255, 254, 231, 200, 160, 128, 77, 20, 1, 0];
    for bpp in [8, 32] {
        for invert in [false, true] {
            for k in 0..papers.len() {
                let mut step = 1;
                while step <= 37 {
                    let paper = papers[k];
                    let bytes = (bpp / 8) as usize;
                    let stride = 8 * bytes;
                    let mut fb = [0u8; 8 * 8 * 4];
                    let mut previous = [0u8; 64];
                    let src = [paper as u8; 8];
                    let mut changes = [0u8; 64];
                    // The settle (or the page as opened) shows the paper's exact gray.
                    for y in 0..8 {
                        fbpack_row(&mut fb, y * stride, bytes as isize, Some(&mut previous), y * 8, 1, &src, 8, 0, y as i32, bpp, invert, false, true);
                    }
                    // Darker frame by frame, down to black ink.
                    let mut v = paper - step;
                    loop {
                        if v < 0 {
                            v = 0;
                        }
                        tile_frame(&mut fb, &mut previous, &mut changes, bpp, invert, v);
                        let mut sum = 0;
                        for i in 0..64usize {
                            let s = shown_at(&fb, stride, bytes, i % 8, i / 8, invert);
                            check!(s == 0 || s == paper);
                            sum += s;
                            if bpp == 32 {
                                let p = &fb[i * bytes..];
                                check!(p[1] == p[0] && p[2] == p[0] && p[3] == 255);
                            }
                        }
                        check!((sum / 64 - v).abs() <= paper * 3 / 128 + 2);
                        if v == 0 {
                            break;
                        }
                        v -= step;
                    }
                    for i in 0..64usize {
                        check!(changes[i] <= 1);
                        check!(shown_at(&fb, stride, bytes, i % 8, i / 8, invert) == 0);
                    }
                    // Erasing back to the paper: lighter frame by frame, each pixel once again.
                    changes = [0u8; 64];
                    let mut v = step;
                    loop {
                        if v > paper {
                            v = paper;
                        }
                        tile_frame(&mut fb, &mut previous, &mut changes, bpp, invert, v);
                        let mut sum = 0;
                        for i in 0..64usize {
                            let s = shown_at(&fb, stride, bytes, i % 8, i / 8, invert);
                            check!(s == 0 || s == 255);
                            sum += s;
                        }
                        check!((sum / 64 - v).abs() <= 255 * 3 / 128 + 2);
                        if v == paper {
                            break;
                        }
                        v += step;
                    }
                    for i in 0..64usize {
                        check!(changes[i] <= 1);
                    }
                    // Nothing new: an overlapping frame changes nothing.
                    changes = [0u8; 64];
                    check!(tile_frame(&mut fb, &mut previous, &mut changes, bpp, invert, paper) == 0);
                    step += 12;
                }
            }
        }
    }
}

// Where the panel shows black or white, live frames are the plain 8x8 ordered dither, as before.
fn black_and_white_pixels_keep_the_ordered_dither() {
    for invert in [false, true] {
        for shown in [0, 255] {
            for v in 0..=255i32 {
                for y in 0..8 {
                    let mut fb = [if invert { 255 - shown } else { shown } as u8; 8];
                    let mut previous = [(255 - v) as u8; 8]; // any other logical value
                    let src = [v as u8; 8];
                    fbpack_row(&mut fb, 0, 1, Some(&mut previous), 0, 1, &src, 8, 0, y, 8, invert, true, true);
                    let mut old_fb = [0x5Au8; 8];
                    // The rule without the panel's state (no cache): what every fast frame did.
                    fbpack_row(&mut old_fb, 0, 1, None, 0, 0, &src, 8, 0, y, 8, invert, true, false);
                    let expect = old_fb;
                    check!(fb == expect);
                }
            }
        }
    }
}

fn no_cache_falls_back_and_invalidating_repaints() {
    let src = [96u8; 16];
    let mut dst = [19u8; 16];
    let mut previous = [96u8; 16];
    fbpack_row(&mut dst, 0, 1, None, 0, 0, &src, 16, 3, 7, 8, false, true, false);
    for i in 0..16 {
        check!(dst[i] == 0 || dst[i] == 255);
    }
    // Changing inversion invalidates the cache; equal logical values still need new pixels.
    fbpack_row(&mut dst, 0, 1, Some(&mut previous), 0, 1, &src, 16, 3, 7, 8, true, false, false);
    for i in 0..16 {
        check!(dst[i] == 159 && previous[i] == 96);
    }
    let before = dst;
    fbpack_row(&mut dst, 0, 1, Some(&mut previous), 0, 1, &src, 0, 0, 0, 8, false, true, true);
    fbpack_row(&mut dst, 0, 1, Some(&mut previous), 0, 1, &src, 16, 0, 0, 16, false, true, true);
    check!(before == dst);
}

fn pen_refreshes_keep_coordinate_processing_at_every_edge() {
    let sizes = [[1860, 2480], [2480, 1860], [1986, 2648], [2648, 1986], [50, 200], [20, 20]];
    for p in 0..sizes.len() {
        let (w, h) = (sizes[p][0], sizes[p][1]);
        let mut rw = 1;
        while rw <= 129 {
            let mut rh = 1;
            while rh <= 129 {
                for corner in 0..4 {
                    let cw = if rw < w { rw } else { w };
                    let ch = if rh < h { rh } else { h };
                    let r = Rect { x: if corner & 1 != 0 { w - cw } else { 0 }, y: if corner & 2 != 0 { h - ch } else { 0 }, w: cw, h: ch };
                    let q = fbpack_pen_region(r, w, h);
                    check!(rect_contains_rect(Rect { x: 0, y: 0, w, h }, q));
                    check!(rect_contains_rect(q, r));
                    check!(q.w as i64 * q.h as i64 >= if (w as i64) * (h as i64) < 6400 { w as i64 * h as i64 } else { 6400 });
                    if r.w * r.h >= 6400 {
                        check!(q == r);
                    }
                }
                rh += 8;
            }
            rw += 8;
        }
    }
    check!(rect_is_empty(fbpack_pen_region(RECT_EMPTY, 1860, 2480)));
    check!(rect_is_empty(fbpack_pen_region(Rect { x: 0, y: 0, w: 1, h: 1 }, 0, 0)));
}

#[test]
fn test_fbpack() {
    run!(gray_paper_survives_live_ink_and_overlap);
    run!(live_ink_moves_each_pixel_once_toward_its_value);
    run!(black_and_white_pixels_keep_the_ordered_dither);
    run!(no_cache_falls_back_and_invalidating_repaints);
    run!(pen_refreshes_keep_coordinate_processing_at_every_edge);
    done();
}
