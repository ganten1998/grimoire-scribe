// test_gesturemap: the built-in gestures and gestures.ini over them (gesturemap.rs; docs/FORMAT.md):
// what a line overrides or turns off, what is refused and counted, what is carried for a later
// version, and the lookups the editor makes (the mirror rule, the gesture of an action, the names).
mod testlib;
use grimoire::prelude::*;
use testlib::*;

fn touch(fingers: u8, kind: TouchKind) -> GestureKey {
    GestureKey::Touch { fingers, kind }
}
fn pen(button: PenBtn, kind: PenKind) -> GestureKey {
    GestureKey::Pen { button, kind }
}
fn parsed(text: &str) -> (GestureMap, i32) {
    let mut m = GestureMap::default();
    gesturemap_defaults(&mut m);
    let n = gesturemap_parse(&mut m, text.as_bytes());
    (m, n)
}

fn the_built_ins() {
    let mut m = GestureMap::default();
    gesturemap_defaults(&mut m);
    check!(gesturemap_action(&m, touch(2, TouchKind::Down)) == SHORTCUT_PAGES);
    check!(gesturemap_action(&m, touch(2, TouchKind::Up)) == SHORTCUT_LINKS);
    check!(gesturemap_action(&m, touch(2, TouchKind::Left)) == SHORTCUT_NEXT_PAGE);
    check!(gesturemap_action(&m, touch(2, TouchKind::Right)) == SHORTCUT_PREV_PAGE);
    check!(gesturemap_action(&m, touch(2, TouchKind::Tap)) == 0);
    for kind in [TouchKind::Up, TouchKind::Down, TouchKind::Left, TouchKind::Right, TouchKind::Tap] {
        check!(gesturemap_action(&m, touch(3, kind)) == 0 && gesturemap_action(&m, touch(1, kind)) == 0);
        check!(!gesturemap_is_set(&m, touch(2, kind)));
    }
    check!(gesturemap_action(&m, pen(PenBtn::Button, PenKind::Triple)) == SHORTCUT_CLEAR_PAGE);
    check!(gesturemap_action(&m, pen(PenBtn::Button, PenKind::Hold)) == SHORTCUT_CLEAR_PAGE);
    check!(gesturemap_action(&m, pen(PenBtn::Button2, PenKind::Triple)) == 0);
    check!(m.refused == 0 && gesturemap_carried(&m).is_empty());
}

fn a_line_overrides_and_nothing_turns_off() {
    let (m, n) = parsed(
        "# Grimoire: gestures\n; a comment too\n\n  touch.2.tap = undo  \ntouch.2.LEFT=previous-page\nTOUCH.3.Up=redo\n\
         touch.2.up=nothing\ntouch.2.down=index\ntouch.2.down=pages\n",
    );
    check!(n == 0 && m.refused == 0);
    check!(gesturemap_action(&m, touch(2, TouchKind::Tap)) == SHORTCUT_UNDO);
    check!(gesturemap_action(&m, touch(2, TouchKind::Left)) == SHORTCUT_PREV_PAGE);
    check!(gesturemap_action(&m, touch(3, TouchKind::Up)) == SHORTCUT_REDO);
    check!(gesturemap_action(&m, touch(2, TouchKind::Up)) == 0); // nothing: the built-in is off
    check!(gesturemap_action(&m, touch(2, TouchKind::Down)) == SHORTCUT_PAGES); // a later line wins
    check!(gesturemap_action(&m, touch(2, TouchKind::Right)) == 0); // the action moved to the left gesture
    check!(gesturemap_is_set(&m, touch(2, TouchKind::Up)) && gesturemap_is_set(&m, touch(2, TouchKind::Tap)));
    check!(gesturemap_is_set(&m, touch(2, TouchKind::Right))); // moving its action writes an explicit nothing
    check!(gesturemap_key_for(&m, SHORTCUT_LINKS, ActionSrc::Touch).is_none());
}

// One-finger taps remain reserved; directions load, and newer shape versions are carried.
fn refused_lines_are_counted() {
    let (m, n) = parsed(
        "touch.1.tap=undo\ntouch.1.left=undo\ntouch.4.up=undo\ntouch.2.sideways=undo\ntouch.2=undo\n\
         touch.2.up.more=undo\nfoo.bar=undo\nno equals sign\ntouch.2.up=frobnicate\ntouch.2.up=\n\
         shape.undo=2,3,ab\nshape.undo.x=1,2\npen.Button.tap=un do\ntouch.2.tap=redo\n",
    );
    check!(n == 11 && m.refused == 11);
    check!(gesturemap_action(&m, touch(2, TouchKind::Tap)) == SHORTCUT_REDO);
    check!(gesturemap_action(&m, touch(2, TouchKind::Up)) == SHORTCUT_LINKS);
    check!(gesturemap_carried(&m) == b"shape.undo=2,3,ab\n");
    check!(gesturemap_action(&m, touch(1, TouchKind::Left)) == SHORTCUT_UNDO);
}

// Erase while held is momentary, and a gesture has no release to end it: a touch line naming it is refused
// and counted, the slot keeps what it had (a built-in, or an earlier line), and other actions load.
fn hold_eraser_is_not_a_touch_action() {
    let (m, n) = parsed("touch.2.tap=hold-eraser\ntouch.3.up=Hold-Eraser\ntouch.2.left=undo\n");
    check!(n == 2 && m.refused == 2);
    check!(gesturemap_action(&m, touch(2, TouchKind::Tap)) == 0);
    check!(gesturemap_action(&m, touch(3, TouchKind::Up)) == 0);
    check!(!gesturemap_is_set(&m, touch(2, TouchKind::Tap)));
    check!(gesturemap_action(&m, touch(2, TouchKind::Left)) == SHORTCUT_UNDO);
    // over a built-in, or an earlier line, it changes nothing
    let (m, n) = parsed("touch.2.up=undo\ntouch.2.up=hold-eraser\ntouch.2.down=hold-eraser\n");
    check!(n == 2);
    check!(gesturemap_action(&m, touch(2, TouchKind::Up)) == SHORTCUT_UNDO);
    check!(gesturemap_action(&m, touch(2, TouchKind::Down)) == SHORTCUT_PAGES);
    // and nothing reaches it by looking for the gesture of the action either
    check!(gesturemap_key_for(&m, SHORTCUT_HOLD_ERASER, ActionSrc::Touch).is_none());
}

// Pen lines are parsed and held (the pen's button keeps its own clicks for now); shape lines and lines
// with an action or pen slot this version does not know are carried as they were, one per key (a
// later line wins), and nothing of them acts on the touch gestures.
fn pen_and_shape_lines_are_carried() {
    let (m, n) = parsed(
        "pen.Button.double=undo\npen.button2.TAP=nothing\npen.Eraser.drag=pan\npen.Button.drag=zoom\n\
         shape.undo=2,2,00ff,ff00\nshape.Undo=2,3,aa,bb,cc\nshape.frobnicate=2,2,00,11\n\
         pen.Button.hold=frobnicate\npen.Button3.tap=undo\n",
    );
    check!(n == 0);
    check!(gesturemap_action(&m, pen(PenBtn::Button, PenKind::Double)) == SHORTCUT_UNDO);
    check!(gesturemap_action(&m, pen(PenBtn::Button2, PenKind::Tap)) == 0);
    check!(gesturemap_action(&m, pen(PenBtn::Eraser, PenKind::Drag)) == GESTURE_PAN);
    check!(gesturemap_action(&m, pen(PenBtn::Button, PenKind::Drag)) == GESTURE_ZOOM);
    check!(gesturemap_action(&m, pen(PenBtn::Button, PenKind::Hold)) == SHORTCUT_CLEAR_PAGE); // its built-in
    check!(
        gesturemap_carried(&m)
            == b"shape.Undo=2,3,aa,bb,cc\nshape.frobnicate=2,2,00,11\npen.Button.hold=frobnicate\npen.Button3.tap=undo\n"
    );
    check!(gesturemap_action(&m, touch(2, TouchKind::Down)) == SHORTCUT_PAGES);
    // pan and zoom are a drag slot's alone
    let (_, n) = parsed("pen.Button.tap=pan\n");
    check!(n == 0); // (carried: a name a later version may give an action)
    let (m, _) = parsed("pen.Button.hold=frobnicate\npen.Button.hold=undo\n");
    check!(gesturemap_action(&m, pen(PenBtn::Button, PenKind::Hold)) == SHORTCUT_UNDO && gesturemap_carried(&m).is_empty());
    // The carry's limits: a line of at most 1 KiB, 16 KiB in all.
    let long = format!("shape.undo=1,2,{}\n", "a".repeat(1100));
    let (m, n) = parsed(&long);
    check!(n == 1 && gesturemap_carried(&m).is_empty());
    let mut text = String::new();
    for i in 0..20 {
        text.push_str(&format!("shape.a{i:02}=2,2,{}\n", "b".repeat(990))); // 1005 bytes a line
    }
    let (m, n) = parsed(&text);
    check!(n == 4 && gesturemap_carried(&m).len() == 16 * 1005);
}

// A missing file is the built-ins; one that cannot be read is the built-ins too, and -1.
fn loading_the_file() {
    let dir = format!("{}/gesturemap", tl_tmp());
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut m = GestureMap::default();
    check!(gesturemap_load(&mut m, &format!("{dir}/gestures.ini")) == 0);
    check!(gesturemap_action(&m, touch(2, TouchKind::Up)) == SHORTCUT_LINKS);
    check!(gesturemap_load(&mut m, &dir) == -1); // a folder
    check!(gesturemap_action(&m, touch(2, TouchKind::Down)) == SHORTCUT_PAGES && m.refused == 0);
    let big = format!("{dir}/big.ini");
    std::fs::write(&big, vec![b'#'; 65537]).unwrap();
    check!(gesturemap_load(&mut m, &big) == -1);
    let path = format!("{dir}/gestures.ini");
    std::fs::write(&path, b"touch.2.up=nothing\ntouch.1.tap=undo\ntouch.2.tap=undo\0touch.2.left=undo\n").unwrap();
    check!(gesturemap_load(&mut m, &path) == 1); // read up to its NUL, as shortcuts.ini is
    check!(gesturemap_action(&m, touch(2, TouchKind::Up)) == 0 && gesturemap_action(&m, touch(2, TouchKind::Tap)) == SHORTCUT_UNDO);
    check!(gesturemap_action(&m, touch(2, TouchKind::Left)) == SHORTCUT_NEXT_PAGE);
    // A reload starts from the built-ins again.
    std::fs::write(&path, b"").unwrap();
    check!(gesturemap_load(&mut m, &path) == 0 && gesturemap_action(&m, touch(2, TouchKind::Up)) == SHORTCUT_LINKS);
    let _ = std::fs::remove_dir_all(&dir);
}

fn the_mirror() {
    check!(gesturemap_mirror(touch(2, TouchKind::Up)) == touch(2, TouchKind::Down));
    check!(gesturemap_mirror(touch(2, TouchKind::Down)) == touch(2, TouchKind::Up));
    check!(gesturemap_mirror(touch(3, TouchKind::Left)) == touch(3, TouchKind::Right));
    check!(gesturemap_mirror(touch(3, TouchKind::Right)) == touch(3, TouchKind::Left));
    check!(gesturemap_mirror(touch(2, TouchKind::Tap)) == touch(2, TouchKind::Tap));
    check!(gesturemap_mirror(pen(PenBtn::Button, PenKind::Drag)) == pen(PenBtn::Button, PenKind::Drag));
}

// The first gesture of an action, in the order 2.down, 2.up, 2.left, 2.right, 2.tap, then three
// fingers; the pen's for the pen; none for keys and controllers.
fn key_for_order() {
    let (m, _) = parsed("touch.2.tap=undo\ntouch.3.down=undo\ntouch.2.right=undo\n");
    check!(gesturemap_key_for(&m, SHORTCUT_UNDO, ActionSrc::Touch) == Some(touch(2, TouchKind::Right)));
    check!(gesturemap_key_for(&m, SHORTCUT_PAGES, ActionSrc::Touch) == Some(touch(2, TouchKind::Down)));
    check!(gesturemap_key_for(&m, SHORTCUT_REDO, ActionSrc::Touch).is_none());
    check!(gesturemap_key_for(&m, SHORTCUT_NONE, ActionSrc::Touch).is_none());
    check!(gesturemap_key_for(&m, SHORTCUT_PAGES, ActionSrc::Key).is_none() && gesturemap_key_for(&m, SHORTCUT_PAGES, ActionSrc::Pad).is_none());
    check!(gesturemap_key_for(&m, SHORTCUT_CLEAR_PAGE, ActionSrc::Pen) == Some(pen(PenBtn::Button, PenKind::Triple)));
    let (m, _) = parsed("touch.3.left=undo\ntouch.3.up=undo\n");
    check!(gesturemap_key_for(&m, SHORTCUT_UNDO, ActionSrc::Touch) == Some(touch(3, TouchKind::Up)));
}

fn the_names() {
    let mut s = String::new();
    gesturemap_key_name(touch(2, TouchKind::Down), &mut s);
    check!(s == "touch.2.down");
    gesturemap_key_name(touch(3, TouchKind::Tap), &mut s);
    check!(s == "touch.3.tap");
    gesturemap_key_name(pen(PenBtn::Button2, PenKind::Triple), &mut s);
    check!(s == "pen.Button2.triple");
    gesturemap_key_name(pen(PenBtn::Eraser, PenKind::Drag), &mut s);
    check!(s == "pen.Eraser.drag");
    // touchgest's ids
    check!(gesturemap_key_of_id(G_2_UP) == Some(touch(2, TouchKind::Up)));
    check!(gesturemap_key_of_id(G_2_DOWN) == Some(touch(2, TouchKind::Down)));
    check!(gesturemap_key_of_id(G_2_LEFT) == Some(touch(2, TouchKind::Left)));
    check!(gesturemap_key_of_id(G_2_RIGHT) == Some(touch(2, TouchKind::Right)));
    check!(gesturemap_key_of_id(G_2_TAP) == Some(touch(2, TouchKind::Tap)));
    check!(gesturemap_key_of_id(G_3_UP) == Some(touch(3, TouchKind::Up)));
    check!(gesturemap_key_of_id(G_3_TAP) == Some(touch(3, TouchKind::Tap)));
    check!(gesturemap_key_of_id(0).is_none() && gesturemap_key_of_id(11).is_none());
    for id in G_2_UP..=G_3_TAP {
        let mut name = String::new();
        gesturemap_key_name(gesturemap_key_of_id(id).unwrap(), &mut name);
        check!(name == format!("touch.{}", touchgest_gesture_name(id)));
    }
}

fn recorded_samples(name: &str) -> [FsSample; 2] {
    let s = grimoire::fingershape_synth::ideal(name, &grimoire::fingershape_synth::Draw::default());
    [s, s]
}

fn writing_patterns_and_shapes() {
    let (mut m, _) = parsed("");
    let mut text = Vec::new();
    check!(gesturemap_format(&m, &mut text) && text.starts_with(b"# Grimoire: gestures (Settings > Gestures)."));
    let z = recorded_samples("Z");
    check!(gesturemap_shape_set(&mut m, SHORTCUT_UNDO, &z) == 0);
    check!(gesturemap_set(&mut m, touch(1, TouchKind::Left), SHORTCUT_UNDO) == 0);
    check!(gesturemap_set(&mut m, touch(2, TouchKind::Tap), SHORTCUT_REDO) == 0);
    check!(gesturemap_set(&mut m, touch(2, TouchKind::Tap), SHORTCUT_UNDO) == SHORTCUT_REDO);
    check!(gesturemap_action(&m, touch(1, TouchKind::Left)) == 0);
    check!(gesturemap_shape_of(&m, SHORTCUT_UNDO).is_some()); // a pattern does not clear its shape
    check!(gesturemap_set(&mut m, touch(2, TouchKind::Down), 0) == SHORTCUT_PAGES);
    check!(gesturemap_format(&m, &mut text));
    let formatted = String::from_utf8(text.clone()).unwrap();
    check!(formatted.contains("touch.2.down=nothing\n") && formatted.contains("touch.2.tap=undo\nshape.undo=1,2,"));
    let (mut back, n) = parsed(&formatted);
    check!(n == 0 && gesturemap_shape_of(&back, SHORTCUT_UNDO).unwrap().samples[..2] == z);
    let mut again = Vec::new();
    check!(gesturemap_format(&back, &mut again) && again == text);
    check!(gesturemap_shape_set(&mut back, SHORTCUT_REDO, &z) == SHORTCUT_UNDO);
    check!(gesturemap_shape_of(&back, SHORTCUT_UNDO).is_none() && gesturemap_shape_of(&back, SHORTCUT_REDO).is_some());
    check!(gesturemap_action(&back, touch(2, TouchKind::Tap)) == SHORTCUT_UNDO);
    gesturemap_clear_action(&mut back, SHORTCUT_REDO);
    check!(gesturemap_shape_of(&back, SHORTCUT_REDO).is_none());
    check!(gesturemap_set(&mut back, touch(1, TouchKind::Tap), SHORTCUT_UNDO) == -1);
    check!(gesturemap_set(&mut back, touch(2, TouchKind::Tap), SHORTCUT_HOLD_ERASER) == -1);
    check!(gesturemap_shape_set(&mut back, SHORTCUT_HOLD_ERASER, &z) == -1);
    let (mut newer, refused) = parsed("shape.undo=9,opaque,newer,data\ntouch.1.up=redo\n");
    check!(refused == 0 && gesturemap_shape_newer(&newer, SHORTCUT_UNDO));
    check!(gesturemap_format(&newer, &mut text) && String::from_utf8_lossy(&text).ends_with("shape.undo=9,opaque,newer,data\n"));
    check!(gesturemap_shape_set(&mut newer, SHORTCUT_UNDO, &z) == 0 && !gesturemap_shape_newer(&newer, SHORTCUT_UNDO));
    let (_, refused) = parsed("shape.undo=1,2,ab,cd\nshape.hold-eraser=9,opaque\ntouch.1.tap=redo\n");
    check!(refused == 3);
    // Valid known shapes have a 32-record cap even when every line uses a different action.
    let mut encoded = [0; FS_VALUE_MAX];
    let value = fs_encode(&z, &mut encoded).unwrap();
    let names: Vec<_> = (1..=SHORTCUT_LAST).filter(|&a| action_sources(a) & SRC_SHAPE != 0).take(FS_SHAPES_MAX + 1).collect();
    check!(names.len() == FS_SHAPES_MAX + 1);
    let mut input = String::new();
    for a in names { input.push_str(&format!("shape.{}={value}\n", shortcut_action_name(a).unwrap())); }
    let (capped, refused) = parsed(&input);
    check!(refused == 1 && gesturemap_shapes(&capped).len() == FS_SHAPES_MAX);
}

fn atomic_saves_and_read_protection() {
    let dir = format!("{}/gesturemap-write-{}", tl_tmp(), std::process::id());
    std::fs::create_dir_all(&dir).unwrap();
    let path = format!("{dir}/gestures.ini");
    let mut m = GestureMap::default();
    check!(gesturemap_load(&mut m, &path) == 0);
    check!(gesturemap_save(&m, &path) == 1 && !std::path::Path::new(&path).exists());
    check!(gesturemap_set(&mut m, touch(2, TouchKind::Tap), SHORTCUT_UNDO) == 0);
    check!(gesturemap_save(&m, &path) == 0);
    let saved = std::fs::read(&path).unwrap();
    let original = m.clone();
    check!(gesturemap_set(&mut m, touch(2, TouchKind::Tap), SHORTCUT_REDO) == SHORTCUT_UNDO);
    std::fs::create_dir(format!("{path}.tmp")).unwrap();
    check!(gesturemap_save(&m, &path) == -1 && std::fs::read(&path).unwrap() == saved);
    check!(gesturemap_action(&original, touch(2, TouchKind::Tap)) == SHORTCUT_UNDO);
    std::fs::remove_dir(format!("{path}.tmp")).unwrap();
    std::fs::write(&path, vec![b'#'; GESTURES_CAP as usize + 1]).unwrap();
    check!(gesturemap_load(&mut m, &path) == -1 && !m.writable);
    check!(gesturemap_set(&mut m, touch(2, TouchKind::Tap), SHORTCUT_UNDO) == -1);
    check!(gesturemap_shape_set(&mut m, SHORTCUT_UNDO, &recorded_samples("Z")) == -1);
    check!(gesturemap_save(&m, &path) == -1 && std::fs::metadata(&path).unwrap().len() == GESTURES_CAP + 1);
    check!(gesturemap_reset(&mut m, &path) == 0 && m.writable && !std::path::Path::new(&path).exists());
    check!(gesturemap_action(&m, touch(2, TouchKind::Down)) == SHORTCUT_PAGES);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn test_gesturemap() {
    run!(the_built_ins);
    run!(a_line_overrides_and_nothing_turns_off);
    run!(refused_lines_are_counted);
    run!(hold_eraser_is_not_a_touch_action);
    run!(pen_and_shape_lines_are_carried);
    run!(loading_the_file);
    run!(the_mirror);
    run!(key_for_order);
    run!(the_names);
    run!(writing_patterns_and_shapes);
    run!(atomic_saves_and_read_protection);
    done();
}
