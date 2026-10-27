//! tools/mkicon.c: draws the Library cover (assets/icon.png) with the app's own gothic widgets and the
//! logo's pixel bat. `make icon` rebuilds it; the PNG is committed, so packaging never needs it.
use grimoire::prelude::*;
use std::ffi::CString;

/// stbi_write_png(filename, w, h, comp, data, stride): nonzero on success.
fn stbi_write_png(filename: &str, w: i32, h: i32, comp: i32, data: &[u8], stride: i32) -> i32 {
    let Ok(path) = CString::new(filename) else { return 0 };
    assert!(data.len() >= (h.max(0) as usize) * (stride.max(0) as usize), "stbi_write_png: short image");
    // SAFETY: path is NUL-terminated; data holds h rows of stride bytes (checked above), of which
    // stb reads w * comp per row and writes nothing.
    unsafe { grimoire::ffi_stb::stbi_write_png(path.as_ptr(), w, h, comp, data.as_ptr() as *const libc::c_void, stride) }
}

fn main() {
    let args = std::env::args_os().map(|a| a.to_string_lossy().into_owned()).collect();
    std::process::exit(run(args));
}

fn run(args: Vec<String>) -> i32 {
    let out = if args.len() > 1 { args[1].as_str() } else { "assets/icon.png" };
    const W: i32 = 300;
    const H: i32 = 400;
    let mut c = Canvas::default();
    if canvas_init(&mut c, W, H) != 0 || font_init() != 0 {
        return 1;
    }
    ui_init(100.0); // a third of the screen's scale: ornaments sized for a 300 px cover
    ui_set_theme(UiTheme::Gothic);
    let card = Rect { x: 6, y: 6, w: W - 12, h: H - 12 };
    canvas_chamfer_rect(&mut c, card, 26, 0, false, 8);
    canvas_chamfer_rect(&mut c, rect_inflate(card, -14), 18, 0, false, 2);
    ui_frieze(&mut c, 22, W - 22, 22, 18, 0);
    ui_emblem(&mut c, W as f32 / 2.0, 160.0, 66.0, 0, 255);
    // The bat in the window instead of its lozenge, on the title's grid (86 px Jacquard: 2 px blocks).
    // (66.f * 0.32f + 3.f is folded by the compiler, unfused: its operands are constants)
    canvas_lozenge(&mut c, W as f32 / 2.0, 160.0, 66.0f32 * 0.32f32 + 3.0f32, 255);
    draw_bat(&mut c, W / 2 - BAT_W, 160 - BAT_H, 2);
    font_draw_centered(&mut c, FontFace::Title, 86.0, Rect { x: 14, y: 262, w: W - 28, h: 80 }, "Grimoire", 0);
    canvas_lozenge(&mut c, W as f32 / 2.0, 352.0, 6.0, 0);
    canvas_hline(&mut c, 60, W / 2 - 14, 351, 2, 0);
    canvas_hline(&mut c, W / 2 + 14, W - 60, 351, 2, 0);
    if stbi_write_png(out, W, H, 1, &c.px, c.stride) == 0 {
        return 1;
    }
    print!("{}", cformat!("wrote %s\n", out));
    0
}
