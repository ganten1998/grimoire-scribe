// test_pump.c: the work pump (common.h). Long work on the main thread calls work_pump() as it goes,
// so the Kindle reads its pens meanwhile: the raw digitizer's kernel buffer holds about 100 ms of
// hover, and on 2026-10-27 a 1.2 s page render, a 1.3 s Paper sheet and the Display sheet's slider
// steps of 90-100 ms lost its pressure and tilt 26 times. These check that the long loops call it,
// that what they draw does not change for it, and that only the process that set the hook runs it.
mod testlib;
use grimoire::prelude::*;
use std::sync::atomic::{AtomicI32, Ordering};
use testlib::*;

const DPI: f32 = 300.0;
const W: i32 = 1860;
const H: i32 = 2480;

static G_CALLS: AtomicI32 = AtomicI32::new(0);
fn count() {
    G_CALLS.fetch_add(1, Ordering::Relaxed);
}
fn calls() -> i32 {
    G_CALLS.load(Ordering::Relaxed)
}

// A dense page: 56 strokes of 600 points each, the owner's p28 shape made heavier.
fn dense_page() -> Box<Page> {
    let mut pg = page_new(W, H, PAPER_MEDIUM);
    let mut tp = ToolParams::default();
    tool_params_default(TOOL_BALLPOINT, &mut tp);
    for i in 0..56 {
        let mut s = stroke_new(TOOL_BALLPOINT, &tp, 0x1234u32 + i as u32);
        for k in 0..600 {
            let f = k as f32 / 599.0;
            let q = PenSample {
                x: fmla(1500.0, f, 150.0),
                y: fmla(15.0, (f * 40.0).sin(), fmla(40.0, i as f32, 120.0)),
                pressure: fmla(0.5, f, 0.3),
                tilt_x: 10.0,
                tilt_y: -5.0,
                t_ms: k as u32 * 3,
            };
            stroke_append(&mut s, &q, DPI);
        }
        page_add(&mut pg, s);
    }
    pg
}

fn hash(c: &Canvas) -> u32 {
    let mut h: u32 = 2166136261;
    for y in 0..c.h {
        for x in 0..c.w {
            h = (h ^ c.px[(y * c.stride + x) as usize] as u32).wrapping_mul(16777619);
        }
    }
    h
}

// Each stroke pumps after it and every 128 points inside it, and the paper after it is drawn: on the
// Kindle every 128 points is 3-15 ms, depending on the tool. The pixels are the same with no hook.
fn a_page_render_pumps_and_draws_the_same() {
    let pg = dense_page();
    let mut a = Canvas::default();
    let mut b = Canvas::default();
    check!(canvas_init(&mut a, W, H) == 0 && canvas_init(&mut b, W, H) == 0);
    work_pump_set(None);
    page_render(&mut a, &pg, Rect { x: 0, y: 0, w: W, h: H }, DPI);
    work_pump_set(Some(count));
    G_CALLS.store(0, Ordering::Relaxed);
    page_render(&mut b, &pg, Rect { x: 0, y: 0, w: W, h: H }, DPI);
    check!(calls() >= 56 * (1 + 600 / 128) + 1);
    check!(hash(&a) == hash(&b));
    check!(a.px[..(a.stride * H) as usize] == b.px[..(a.stride * H) as usize]);
    // A part of the page (a dirty rect): only the strokes in it are drawn, and they still pump.
    G_CALLS.store(0, Ordering::Relaxed);
    page_render(&mut b, &pg, Rect { x: 0, y: 0, w: W, h: 300 }, DPI);
    check!(calls() >= 4 * (1 + 600 / 128));
    check!(a.px[..(a.stride * H) as usize] == b.px[..(a.stride * H) as usize]);
    // The thumbnail of a page: its render, then the downsample, every 64 rows of the page.
    let mut thumb = Canvas::default();
    check!(canvas_init(&mut thumb, W / 6, H / 6) == 0);
    G_CALLS.store(0, Ordering::Relaxed);
    page_render_thumb(&mut thumb, &pg, DPI);
    check!(calls() >= 56 * (1 + 600 / 128) + H / 64);
    work_pump_set(None);
    canvas_free(&mut thumb);
    canvas_free(&mut a);
    canvas_free(&mut b);
    page_free(pg);
}

// The Paper sheet's previews: a whole page of each paper, then shrunk keeping its lines.
fn a_paper_preview_pumps() {
    let mut c = Canvas::default();
    let mut small = Canvas::default();
    check!(canvas_init(&mut c, W, H) == 0 && canvas_init(&mut small, 300, 400) == 0);
    work_pump_set(Some(count));
    G_CALLS.store(0, Ordering::Relaxed);
    paper_render(&mut c, PAPER_GRAND, Rect { x: 0, y: 0, w: W, h: H }, DPI);
    check!(calls() == 1);
    G_CALLS.store(0, Ordering::Relaxed);
    canvas_downsample_keep_lines(&mut small, &c, 0.5);
    check!(calls() >= H / 64 - 1 && calls() <= H / 64 + 1);
    G_CALLS.store(0, Ordering::Relaxed);
    canvas_downsample(&mut small, &c);
    check!(calls() >= H / 64 - 1 && calls() <= H / 64 + 1);
    // Nothing to draw: no pump either.
    G_CALLS.store(0, Ordering::Relaxed);
    paper_render(&mut c, PAPER_GRAND, RECT_EMPTY, DPI);
    check!(calls() == 0);
    work_pump_set(None);
    canvas_free(&mut c);
    canvas_free(&mut small);
}

// A save pumps after its write, its fsync, the rename and the folder's fsync: each wait on the
// storage is then one fsync at most (15-25 ms on the Kindle; a page's whole save took 65-104 ms).
fn a_save_pumps_between_its_waits() {
    let path = cformat!("%s/pump-%d.txt", tl_tmp().as_str(), std::process::id() as i32);
    work_pump_set(Some(count));
    G_CALLS.store(0, Ordering::Relaxed);
    check!(write_file_atomic(&path, b"grimoire") == 0);
    check!(calls() == 4);
    let _ = std::fs::remove_file(&path);
    // One that cannot even start (no such folder) has nothing to pump between.
    G_CALLS.store(0, Ordering::Relaxed);
    check!(write_file_atomic("/nonexistent-grimoire-dir/x.txt", b"x") != 0);
    check!(calls() == 0);
    work_pump_set(None);
    G_CALLS.store(0, Ordering::Relaxed);
    work_pump();
    check!(calls() == 0);
}

// Only the process that set the hook runs it. A forked child (a background export, a share server
// render) inherits the hook with the rest of the memory, and its own renders and saves call
// work_pump; reading Grimoire's input devices from there would take Grimoire's pen events (the
// file description is shared), or read a file of the child's that came to reuse their number.
static G_PIPE_W: AtomicI32 = AtomicI32::new(-1);
fn tell() {
    let c: u8 = 1;
    // SAFETY: write of one byte from a local to the pipe's write end, set by the test below.
    let w = unsafe { libc::write(G_PIPE_W.load(Ordering::Relaxed), &c as *const u8 as *const libc::c_void, 1) };
    if w != 1 { /* the check below sees it */ }
}

fn only_the_process_that_set_it_pumps() {
    let mut fds = [0 as libc::c_int; 2];
    // SAFETY: pipe fills the two descriptors of our array; fcntl sets the read end non-blocking.
    unsafe {
        check!(libc::pipe(fds.as_mut_ptr()) == 0);
        libc::fcntl(fds[0], libc::F_SETFL, libc::O_NONBLOCK);
    }
    G_PIPE_W.store(fds[1], Ordering::Relaxed);
    work_pump_set(Some(tell));
    // SAFETY: fork in the test; the child works on its own copy and ends with _exit (PORTING.md, 6).
    let pid = unsafe { libc::fork() };
    if pid == 0 {
        let path = cformat!("%s/pump-child-%d.txt", tl_tmp().as_str(), std::process::id() as i32);
        work_pump();
        write_file_atomic(&path, b"child");
        let _ = std::fs::remove_file(&path);
        let mut c = Canvas::default();
        let mut small = Canvas::default();
        if canvas_init(&mut c, 640, 640) == 0 && canvas_init(&mut small, 64, 64) == 0 {
            canvas_downsample(&mut small, &c);
        }
        // SAFETY: _exit ends the child without running the parent's exit handlers.
        unsafe { libc::_exit(0) };
    }
    check!(pid > 0);
    let mut status: libc::c_int = -1;
    // SAFETY: waitpid on the child forked above, status is our local.
    unsafe { libc::waitpid(pid, &mut status, 0) };
    check!(libc::WIFEXITED(status) && libc::WEXITSTATUS(status) == 0);
    let mut c: u8 = 0;
    // SAFETY: reads of one byte into a local, from the pipe's read end.
    let rd = |c: &mut u8| unsafe { libc::read(fds[0], c as *mut u8 as *mut libc::c_void, 1) };
    check!(rd(&mut c) < 0 && errno() == libc::EAGAIN); // the child never ran it
    work_pump();
    check!(rd(&mut c) == 1); // this process does
    work_pump_set(None);
    work_pump();
    check!(rd(&mut c) < 0);
    // SAFETY: closing the two descriptors made above.
    unsafe {
        libc::close(fds[0]);
        libc::close(fds[1]);
    }
}

#[test]
fn test_pump() {
    run!(a_page_render_pumps_and_draws_the_same);
    run!(a_paper_preview_pumps);
    run!(a_save_pumps_between_its_waits);
    run!(only_the_process_that_set_it_pumps);
    done();
}
