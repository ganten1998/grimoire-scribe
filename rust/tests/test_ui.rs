// test_ui.c: widget logic + render sheets for visual review.
mod testlib;

use grimoire::ffi_stb;
use grimoire::prelude::*;
use testlib::*;

fn out_png(name: &str, c: &Canvas) {
    let dir = cformat!("%s/../out", &tl_tmp());
    mkdir_p(&dir);
    let path = cformat!("%s/%s", &dir, name);
    let Ok(path) = std::ffi::CString::new(path) else { return };
    // SAFETY: stbi_write_png reads the NUL-terminated path and c.h rows of c.w bytes, stride bytes
    // apart, from c.px (a canvas holds h * stride bytes).
    unsafe {
        ffi_stb::stbi_write_png(path.as_ptr(), c.w, c.h, 1, c.px.as_ptr() as *const std::ffi::c_void, c.stride);
    }
}

// The C's (IconKind)k for k = 1 .. ICON_COUNT - 1.
const ICONS: [IconKind; IconKind::COUNT - 1] = [
    IconKind::Pencil, IconKind::MechPencil, IconKind::Ballpoint, IconKind::Eraser, IconKind::Undo,
    IconKind::Redo, IconKind::Prev, IconKind::Next, IconKind::Plus, IconKind::Minus,
    IconKind::Close, IconKind::Paper, IconKind::Gear, IconKind::Library, IconKind::Collapse,
    IconKind::Expand, IconKind::Grip, IconKind::Folder, IconKind::Notebook, IconKind::More,
    IconKind::Share, IconKind::Trash, IconKind::Check, IconKind::Back, IconKind::Target,
    IconKind::Exit, IconKind::Tune, IconKind::Rename, IconKind::FolderMove, IconKind::FolderPlus,
    IconKind::Download, IconKind::Highlighter, IconKind::Broom, IconKind::Brightness,
    IconKind::Invert, IconKind::Fountain, IconKind::Fineliner, IconKind::Marker, IconKind::Up,
    IconKind::Down, IconKind::Music, IconKind::Castle, IconKind::Charcoal, IconKind::Calligraphy,
    IconKind::Language, IconKind::Pin, IconKind::Grid, IconKind::Copy, IconKind::Link,
    IconKind::Live, IconKind::Gamepad, IconKind::Update,
];

fn slider_snap() {
    let mut s = Slider { min: 0.0, max: 1.0, step: 0.05, value: 0.0, ..Default::default() };
    ui_slider_layout(&mut s, Rect { x: 0, y: 0, w: 1000, h: 100 });
    let x = fmla(s.track.w as f32, 0.512, s.track.x as f32);
    check!(ui_slider_set_from_x(&mut s, x));
    check_near!(s.value, 0.5f32, 1e-5);
    let x = fmla(s.track.w as f32, 0.51, s.track.x as f32);
    check!(!ui_slider_set_from_x(&mut s, x));
    ui_slider_set_from_x(&mut s, -100.0);
    check!(s.value == 0.0);
    ui_slider_set_from_x(&mut s, 5000.0);
    check!(s.value == 1.0);
}

fn press_logic() {
    let mut p = Press::default();
    press_begin(&mut p, 100.0, 100.0, 0);
    press_move(&mut p, 105.0, 103.0);
    check!(press_is_tap(&p, Rect { x: 50, y: 50, w: 100, h: 100 }));
    press_move(&mut p, 300.0, 100.0);
    check!(!press_is_tap(&p, Rect { x: 50, y: 50, w: 400, h: 100 }));
}

fn slider_fine_drag() {
    let mut s = Slider { min: 0.0, max: 1.0, step: 0.01, value: 0.28, ..Default::default() };
    ui_slider_layout(&mut s, Rect { x: 0, y: 0, w: 1000, h: 100 });
    let mut d = SliderDrag::default();
    // (a + b * c is one fused multiply-add in the C build: PORTING.md, Floating point)
    let mut x = fmla(s.track.w as f32, 0.28, s.track.x as f32);
    let y = fmla(s.track.h as f32, 0.5, s.track.y as f32);
    ui_slider_drag_begin(&s, &mut d, x, 100);
    check!(!ui_slider_drag_to(&mut s, &mut d, x, y + 160.0, 500));
    check_near!(s.value, 0.28f32, 1e-6); // moving away alone never jumps
    x = fmla(s.track.w as f32 * 0.02, 16.0, x);
    check!(ui_slider_drag_to(&mut s, &mut d, x, y + 160.0, 510));
    check_near!(s.value, 0.30f32, 1e-6);
    for i in 0..10 {
        x = fmla(s.track.w as f32 * 0.001, 16.0, x);
        ui_slider_drag_to(&mut s, &mut d, x, y + 160.0, 520u32 + i);
    }
    check_near!(s.value, 0.31f32, 1e-6); // fractional motion accumulates rather than rounding away
    check!(ui_slider_drag_to(&mut s, &mut d, x, y + 10.0, 600));
    check_near!(s.value, 0.76f32, 1e-6); // entering the knob-height band catches up to the pointer
    x = fmla(s.track.w as f32, 0.4, s.track.x as f32);
    ui_slider_drag_to(&mut s, &mut d, x, y - 10.0, 610);
    check_near!(s.value, 0.4f32, 1e-6); // staying in the band gives direct, large adjustments
    check!(!ui_slider_drag_to(&mut s, &mut d, x, y - 160.0, 620));
    check_near!(s.value, 0.4f32, 1e-6); // leaving again alone does not change the value
    x = fmla(s.track.w as f32 * 0.02, 16.0, x);
    ui_slider_drag_to(&mut s, &mut d, x, y - 160.0, 630);
    check_near!(s.value, 0.42f32, 1e-6); // fine motion resumes above as well as below
    ui_slider_drag_to(&mut s, &mut d, x + 10000.0, y - 160.0, 640);
    check_near!(s.value, 1.0f32, 1e-6);
    let to = fmla(-(s.track.w as f32 * 0.1), 16.0, x + 10000.0);
    ui_slider_drag_to(&mut s, &mut d, to, y - 160.0, 650);
    check_near!(s.value, 0.9f32, 1e-6);
    // A fresh press restores ordinary absolute tracking; holding is specific to each gesture.
    ui_slider_drag_begin(&s, &mut d, x, 1000);
    let to = fmla(s.track.w as f32, 0.2, s.track.x as f32);
    ui_slider_drag_to(&mut s, &mut d, to, y, 1100);
    check_near!(s.value, 0.2f32, 1e-6);
    check!(!d.held);
}

fn keyboard_typing() {
    let mut c = Canvas::default();
    canvas_init(&mut c, 1860, 2480);
    let mut kb = Keyboard::default();
    keyboard_open(&mut kb, "Name", b"Hi");
    keyboard_draw(&mut kb, &mut c, Rect { x: 0, y: 0, w: 1860, h: 2480 });
    // Find the 'q' key by scanning: it is the first key of row 2. Tap the center of each
    // key via pointer events derived from the drawn layout through key_at semantics.
    let mut d = Rect::default();
    // The 'a' row starts below; easier: type using the internal layout by probing points.
    let mut typed = false;
    let mut y = kb.area.y;
    while y < kb.area.y + kb.area.h && !typed {
        let mut x = kb.area.x;
        while x < kb.area.x + kb.area.w && !typed {
            keyboard_pointer(&mut kb, 0, x as f32, y as f32, 0, &mut d);
            if kb.pressed_key == 0 {
                // '1'
                keyboard_pointer(&mut kb, 2, x as f32, y as f32, 10, &mut d);
                typed = true;
            } else {
                keyboard_pointer(&mut kb, 2, -1.0, -1.0, 10, &mut d); // release off-key: no input
            }
            x += 20;
        }
        y += 20;
    }
    check!(typed && kb.text == "Hi1");
    keyboard_draw(&mut kb, &mut c, Rect { x: 0, y: 0, w: 1860, h: 2480 });
    out_png("keyboard.png", &c);
    canvas_free(&mut c);
}

fn sheets() {
    let mut c = Canvas::default();
    canvas_init(&mut c, 1860, 2480);
    let mut x = 40;
    let mut y = 40;
    for (i, &icon) in ICONS.iter().enumerate() {
        let k = i + 1;
        let r = Rect { x, y, w: 160, h: 160 };
        ui_button(&mut c, r, icon, "", if k % 5 == 0 { ButtonStyle::Selected } else { ButtonStyle::Plain });
        x += 180;
        if x > 1860 - 180 {
            x = 40;
            y += 180;
        }
    }
    y += 220;
    let labels = ["Pencil", "Mech", "Ballpoint", "Eraser"];
    let icons = [IconKind::Pencil, IconKind::MechPencil, IconKind::Ballpoint, IconKind::Eraser];
    for i in 0..4 {
        ui_button(&mut c, Rect { x: 40 + i as i32 * 200, y, w: 160, h: 150 }, icons[i], labels[i], if i == 1 { ButtonStyle::Selected } else { ButtonStyle::Plain });
    }
    ui_button(&mut c, Rect { x: 880, y, w: 400, h: 150 }, IconKind::None, "Create", ButtonStyle::Primary);
    ui_button(&mut c, Rect { x: 1300, y, w: 400, h: 150 }, IconKind::None, "Disabled", ButtonStyle::Disabled);
    y += 200;
    let mut s = Slider { min: 0.25, max: 4.0, step: 0.05, value: 1.25, label: "Pressure curve", unit: "", ..Default::default() };
    ui_slider_layout(&mut s, Rect { x: 40, y, w: 1780, h: 110 });
    ui_slider_draw(&mut c, &s, None);
    y += 130;
    s.value = 3.0;
    s.label = "Tilt widens";
    s.unit = "x";
    ui_slider_layout(&mut s, Rect { x: 40, y, w: 1780, h: 110 });
    ui_slider_draw(&mut c, &s, None);
    y += 140;
    ui_checkbox(&mut c, Rect { x: 40, y, w: 900, h: 100 }, "Mirror PDFs to Documents", true);
    ui_checkbox(&mut c, Rect { x: 940, y, w: 900, h: 100 }, "Side button erases", false);
    y += 140;
    let inner = ui_sheet(&mut c, Rect { x: 40, y, w: 1780, h: 500 }, "New notebook");
    font_draw_left(&mut c, FontFace::Regular, ui_text_size(), Rect { x: inner.x, y: inner.y, w: inner.w, h: 80 }, "The quick brown fox jumps over the lazy dog. 0123456789", 0);
    ui_toast(&mut c, Rect { x: 0, y: 0, w: 1860, h: 2480 }, "PDF saved to grimoire/PDFs");
    out_png("widgets.png", &c);
    canvas_free(&mut c);
}

// The pin and grid icons (the tool picker's Pin, the panel's tool button when its pen is pinned)
// are in the embedded font. A hollow icon, the pen in use drawing white ink, is the glyph's
// outline: where the glyph is solid it shows the inside colour, around it a ring of the edge
// colour; on a selected (black) button the same in white.
fn pin_grid_and_hollow_icons() {
    let mut c = Canvas::default();
    canvas_init(&mut c, 600, 300);
    canvas_fill(&mut c, Rect { x: 0, y: 0, w: 600, h: 300 }, 255);
    ui_icon(&mut c, IconKind::Pin, Rect { x: 0, y: 0, w: 100, h: 100 }, 0);
    ui_icon(&mut c, IconKind::Grid, Rect { x: 100, y: 0, w: 100, h: 100 }, 0);
    // (and the Pages view's Copy page)
    ui_icon(&mut c, IconKind::Copy, Rect { x: 200, y: 0, w: 100, h: 100 }, 0);
    let mut dark = [0i32; 3];
    for y in 0..100 {
        for x in 0..300 {
            dark[x / 100] += (c.px[y * c.stride as usize + x] < 128) as i32;
        }
    }
    check!(dark[0] > 500 && dark[1] > 500 && dark[2] > 500);
    // Solid, then hollow, the same icon in the same place: the solid's full pixels are the hollow's
    // inside, and the hollow's ring reaches past the solid's edge.
    let r = Rect { x: 0, y: 100, w: 120, h: 120 };
    canvas_fill(&mut c, Rect { x: 0, y: 100, w: 600, h: 200 }, 255);
    ui_icon(&mut c, IconKind::Pencil, r, 0);
    ui_icon_hollow(&mut c, IconKind::Pencil, Rect { x: r.x + 200, y: r.y, w: r.w, h: r.h }, 0, 255);
    let (mut inside, mut inside_white, mut ring) = (0, 0, 0);
    for y in r.y..r.y + r.h {
        for x in r.x..r.x + r.w {
            let at = (y * c.stride + x) as usize;
            let s = c.px[at];
            let h = c.px[at + 200];
            if s < 8 {
                inside += 1;
                inside_white += (h > 247) as i32; // (a glyph blends to within a level)
            }
            if s == 255 && h < 128 {
                ring += 1;
            }
        }
    }
    check!(inside > 300 && inside_white == inside && ring > inside / 4);
    // On a selected button: the white glyph's solid pixels are black, inside a white ring.
    let b = Rect { x: 330, y: 100, w: 120, h: 120 };
    ui_button(&mut c, b, IconKind::Pencil, "", ButtonStyle::Selected);
    ui_button_hollow(&mut c, Rect { x: b.x + 140, y: b.y, w: b.w, h: b.h }, IconKind::Pencil, ButtonStyle::Selected);
    let (mut core, mut core_black, mut white_ring) = (0, 0, 0);
    let inn = rect_inflate(b, -22); // inside the frame and its carved line
    for y in inn.y..inn.y + inn.h {
        for x in inn.x..inn.x + inn.w {
            let at = (y * c.stride + x) as usize;
            let s = c.px[at];
            let h = c.px[at + 140];
            if s == 255 {
                core += 1;
                core_black += (h < 8) as i32;
            }
            if s < 8 && h > 128 {
                white_ring += 1;
            }
        }
    }
    check!(core > 300 && core_black == core && white_ring > core / 4);
    out_png("hollow.png", &c);
    canvas_free(&mut c);
}

// A name keeps every letter in the files; the screens leave out what the fonts cannot draw (it
// would show as a box), and a name left with no letter reads Untitled, with what is left.
fn name_labels() {
    let mut buf = String::new();
    let plain = "Le Petit Prince";
    let greek = "\u{3b1}\u{3b2} \u{412}\u{43e}\u{439}\u{43d}\u{430}";
    check!(font_drawable_copy(plain, &mut buf, 200) == 0 && buf == plain);
    // (the C compares the pointers: the label is the name itself when all of it is drawn)
    check!(std::ptr::eq(ui_name_label(plain, &mut buf, 200), plain) && std::ptr::eq(ui_name_label(greek, &mut buf, 200), greek));
    check!(ui_name_label("1984", &mut buf, 200).as_bytes()[0] == b'1'); // (all of it drawn: as it is)
    // Japanese, in the middle and alone; Vietnamese marks Terminess lacks (e with circumflex and
    // acute, e with circumflex and dot below).
    check!(font_drawable_copy("Pok\u{e9}mon \u{6f2b}\u{753b} Vol. 3", &mut buf, 200) == 2);
    check!(buf == "Pok\u{e9}mon Vol. 3");
    check!(ui_name_label("Pok\u{e9}mon \u{6f2b}\u{753b} Vol. 3", &mut buf, 200) == "Pok\u{e9}mon Vol. 3");
    let ja = "\u{543e}\u{8f29}\u{306f}\u{732b}\u{3067}\u{3042}\u{308b}";
    check!(font_drawable_copy(ja, &mut buf, 200) == 7 && buf.is_empty());
    check!(ui_name_label(ja, &mut buf, 200) == "Untitled");
    let numbered = cformat!("%s 2", ja);
    check!(ui_name_label(&numbered, &mut buf, 200) == "Untitled 2");
    check!(ui_name_label("Ti\u{1ebf}ng Vi\u{1ec7}t", &mut buf, 200) == "Ting Vit");
    // Cut to the buffer between two characters.
    let mut small = String::new();
    check!(font_drawable_copy("ab\u{20ac}\u{20ac}", &mut small, 6) == 0 && small == "ab\u{20ac}");
}

#[test]
fn test_ui() {
    ui_init(300.0);
    if font_init() != 0 {
        panic!("font_init failed");
    }
    run!(pin_grid_and_hollow_icons);
    run!(slider_snap);
    run!(slider_fine_drag);
    run!(press_logic);
    run!(keyboard_typing);
    run!(sheets);
    run!(name_labels);
    done();
}
