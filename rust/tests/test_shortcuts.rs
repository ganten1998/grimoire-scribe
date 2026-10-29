// test_shortcuts.c: the shortcut maps (shortcuts.ini, controller.ini), their names and limits.
//
// Index loops and range comparisons are written as the C test writes them.
#![allow(clippy::needless_range_loop, clippy::manual_range_contains)]
mod testlib;
use grimoire::prelude::*;
use testlib::*;

fn unlink(path: &str) {
    let _ = std::fs::remove_file(path);
}
fn exists(path: &str) -> bool {
    std::path::Path::new(path).exists()
}
// strstr(text, needle) != NULL, on the file's bytes
fn has(text: &[u8], needle: &str) -> bool {
    let n = needle.as_bytes();
    text.windows(n.len()).any(|w| w == n)
}

fn defaults_and_exact_modifiers() {
    let mut m = Shortcuts::default();
    shortcuts_load(&mut m, "/nonexistent/grimoire-shortcuts.ini");
    check!(m.count == 29 && m.rejected == 0);
    check!(shortcuts_lookup(&m, false, 2, 0) == SHORTCUT_TOOL_BASE + TOOL_PENCIL.0);
    // 1 to 9 in the tool picker's order: 3 the charcoal, 4 the ballpoint, 7 the fountain pen.
    for i in 0..TOOL_PENS {
        check!(
            shortcuts_lookup(&m, false, 2 + i as i32, 0) == SHORTCUT_TOOL_BASE + TOOL_ORDER[i].0
        );
    }
    check!(shortcuts_lookup(&m, false, 4, 0) == SHORTCUT_TOOL_BASE + TOOL_CHARCOAL.0);
    check!(shortcuts_lookup(&m, false, 8, 0) == SHORTCUT_TOOL_BASE + TOOL_FOUNTAIN.0);
    check!(shortcuts_lookup(&m, false, 44, KEYMOD_CTRL) == SHORTCUT_UNDO);
    check!(shortcuts_lookup(&m, false, 44, KEYMOD_CTRL | KEYMOD_SHIFT) == SHORTCUT_NONE);
    check!(shortcuts_lookup(&m, true, 304, 0) == SHORTCUT_UNDO);
    check!(shortcuts_lookup(&m, false, 304, 0) == SHORTCUT_NONE);
    check!(shortcut_keycode("code767") == 767 && shortcut_keycode("code768") == -1);
    check!(shortcut_keycode("KEY_A") == -1);
    check!(shortcuts_lookup(&m, true, 546, 0) == SHORTCUT_PREV_PAGE);
    check!(shortcuts_lookup(&m, true, 545, 0) == SHORTCUT_NEXT_TOOL);
    check!(shortcut_keycode("VolumeDown") == 114);
}
fn custom_mappings_and_limits() {
    let path = format!("{}/shortcuts.ini", tl_tmp());
    let text = "# Custom\n key.Ctrl+SHIFT+z = redo\r\nkey.e=tool:pencil\nkey.e=tool:marker\n\
                pad.305=tool:eraser\npad.2=undo\nkey.Hyper+x=undo\nkey.1=erase-all\ngarbage\n";
    check!(write_file_atomic(&path, text.as_bytes()) == 0);
    let mut m = Shortcuts::default();
    shortcuts_load(&mut m, &path);
    check!(m.count == 3 && m.rejected == 4);
    check!(shortcuts_lookup(&m, false, 44, KEYMOD_CTRL | KEYMOD_SHIFT) == SHORTCUT_REDO);
    check!(shortcuts_lookup(&m, false, 18, 0) == SHORTCUT_TOOL_BASE + TOOL_MARKER.0);
    check!(shortcuts_write_defaults(&path) == 0);
    shortcuts_load(&mut m, &path);
    check!(m.count == 3); // enabling or reloading never overwrites user choices
    check!(write_file_atomic(&path, b"") == 0);
    shortcuts_load(&mut m, &path);
    check!(m.count == 0);
    let f = std::fs::write(&path, vec![b'x'; 65537]);
    check!(f.is_ok());
    shortcuts_load(&mut m, &path);
    check!(m.count == 0 && m.rejected == 1);
    unlink(&path);
    check!(shortcuts_write_defaults(&path) == 0);
    shortcuts_load(&mut m, &path);
    check!(m.count == 29);
    unlink(&path);
}
fn settings_are_opt_in() {
    let path = format!("{}/shortcuts-store", tl_tmp());
    check!(store_init(&path) == 0);
    let mut s = Settings::default();
    settings_default(&mut s);
    check!(!s.shortcuts_enabled);
    s.shortcuts_enabled = true;
    check!(settings_save(&s) == 0);
    let mut loaded = Settings::default();
    check!(settings_load(&mut loaded) == 0 && loaded.shortcuts_enabled);
    loaded.shortcuts_enabled = false;
    check!(settings_save(&loaded) == 0);
    check!(settings_load(&mut s) == 0 && !s.shortcuts_enabled);
}
fn controller_ini_learns_one_button_per_action() {
    let sp = format!("{}/shortcuts-c.ini", tl_tmp());
    let cp = format!("{}/controller.ini", tl_tmp());
    unlink(&sp);
    unlink(&cp);
    let mut m = Shortcuts::default();
    shortcuts_load_all(&mut m, &sp, &cp); // neither file: the defaults
    check!(m.count == 29 && shortcuts_lookup(&m, true, 304, 0) == SHORTCUT_UNDO);
    let mut codes = [0i32; 4];
    check!(shortcuts_pads_for(&m, ActionSrc::Pad, SHORTCUT_PREV_PAGE, &mut codes) == 2); // Select and the D-pad's Left
                                                                                         // Learn: East is Undo now; South loses it (it was Undo's), and the eraser loses East.
    check!(shortcuts_learn(&mut m, &cp, ActionSrc::Pad, 305, SHORTCUT_UNDO) == 0);
    check!(
        shortcuts_lookup(&m, true, 305, 0) == SHORTCUT_UNDO
            && shortcuts_lookup(&m, true, 304, 0) == SHORTCUT_NONE
    );
    check!(
        shortcuts_pads_for(
            &m,
            ActionSrc::Pad,
            SHORTCUT_TOOL_BASE + TOOL_ERASER.0,
            &mut []
        ) == 0
    );
    check!(shortcuts_learn(&mut m, &cp, ActionSrc::Pad, 316, SHORTCUT_PREV_PAGE) == 0); // a button with no name: its number
    check!(
        shortcuts_pads_for(&m, ActionSrc::Pad, SHORTCUT_PREV_PAGE, &mut codes) == 1
            && codes[0] == 316
    );
    check!(shortcuts_lookup(&m, false, 44, KEYMOD_CTRL) == SHORTCUT_UNDO); // keys untouched
    let text = read_file(&cp);
    check!(text.as_ref().is_some_and(|t| has(t, "pad.East=undo\n")
        && has(t, "pad.316=previous-page\n")
        && !has(t, "key.")));
    // Read back: the same gamepad buttons, from controller.ini alone; shortcuts.ini's pad. lines set aside.
    let custom = "key.e=tool:marker\npad.South=redo\n";
    check!(write_file_atomic(&sp, custom.as_bytes()) == 0);
    let mut back = Shortcuts::default();
    shortcuts_load_all(&mut back, &sp, &cp);
    check!(
        shortcuts_lookup(&back, true, 304, 0) == SHORTCUT_NONE
            && shortcuts_lookup(&back, true, 305, 0) == SHORTCUT_UNDO
    );
    check!(shortcuts_lookup(&back, false, 18, 0) == SHORTCUT_TOOL_BASE + TOOL_MARKER.0);
    // A key line in controller.ini is not its to hold.
    let odd = "pad.North=redo\nkey.z=undo\n";
    check!(write_file_atomic(&cp, odd.as_bytes()) == 0);
    shortcuts_load_all(&mut back, &sp, &cp);
    check!(
        back.rejected == 1
            && shortcuts_lookup(&back, false, 44, 0) == SHORTCUT_NONE
            && shortcuts_lookup(&back, true, 307, 0) == SHORTCUT_REDO
    );
    // A file that cannot be written leaves the map as it was.
    let bad = format!("{}/no-such-dir/controller.ini", tl_tmp());
    let before = back.clone();
    check!(
        shortcuts_learn(&mut back, &bad, ActionSrc::Pad, 308, SHORTCUT_UNDO) != 0
            && back.count == before.count
            && shortcuts_lookup(&back, true, 308, 0) == shortcuts_lookup(&before, true, 308, 0)
    );
    let mut name = String::new();
    shortcut_pad_name(544, &mut name, 16);
    check!(name == "Up");
    check!(
        shortcut_action_name(SHORTCUT_TOOL_BASE + TOOL_ERASER.0) == Some("tool:eraser")
            && shortcut_action_name(999).is_none()
    );
    unlink(&sp);
    unlink(&cp);
}
// Every action has a name the files keep, which reads back as it; one no version knows yet is
// refused and counted while the other lines load (as older versions treat the names added later).
fn every_action_by_name() {
    let mut named = 0;
    for a in 1..=SHORTCUT_LAST {
        let Some(name) = shortcut_action_name(a) else {
            continue;
        };
        named += 1;
        check!(shortcut_action(name) == a);
    }
    check!(named == 10 + 8 + 28 + 22); // the tools, the first actions, and 2.0.0-beta.5 (the clock corner two, the Index five, the swipe-adds-a-page toggle; Rust only)
    let path = format!("{}/shortcuts-new.ini", tl_tmp());
    let text = "pad.L2=hold-eraser\npad.R2=eraser-mode\npad.600=clear-page\npad.601=first-page\n\
                pad.602=last-page\npad.603=new-page\npad.604=fit-page\npad.605=pages\n\
                pad.606=tool-picker\npad.607=paper\npad.608=display\npad.609=pen-feel\n\
                pad.610=fold-panel\npad.611=library\npad.612=close-menu\nkey.z=undo\n\
                pad.614=picker-previous\nkey.PageDown=picker-next\n\
                pad.615=add-link\nkey.Ctrl+n=name-page\npad.616=status-toggle\n\
                pad.North=menu:frobnicate\nkey.x=hold-pen\npad.613=tool:crayon\n";
    check!(write_file_atomic(&path, text.as_bytes()) == 0);
    let mut m = Shortcuts::default();
    shortcuts_load(&mut m, &path);
    check!(m.count == 21 && m.rejected == 3);
    check!(
        shortcuts_lookup(&m, true, 312, 0) == SHORTCUT_HOLD_ERASER
            && shortcuts_lookup(&m, true, 612, 0) == SHORTCUT_CLOSE_MENU
    );
    // The tool picker's next and previous, beside next and previous tool (the panel's pens).
    check!(
        shortcuts_lookup(&m, true, 614, 0) == SHORTCUT_PICKER_PREV
            && shortcuts_lookup(&m, false, 109, 0) == SHORTCUT_PICKER_NEXT
    );
    check!(
        shortcut_action_name(SHORTCUT_PICKER_NEXT) == Some("picker-next")
            && shortcut_action_name(SHORTCUT_PICKER_PREV) == Some("picker-previous")
    );
    check!(
        shortcut_action("next-tool") == SHORTCUT_NEXT_TOOL
            && shortcut_action("picker-next") != SHORTCUT_NEXT_TOOL
    );
    // Add a link and the open page's name, from the page.
    check!(
        shortcuts_lookup(&m, true, 615, 0) == SHORTCUT_ADD_LINK
            && shortcuts_lookup(&m, false, 49, KEYMOD_CTRL) == SHORTCUT_NAME_PAGE
            && shortcuts_lookup(&m, false, 49, 0) == SHORTCUT_NONE
    );
    check!(
        shortcut_action_name(SHORTCUT_ADD_LINK) == Some("add-link")
            && shortcut_action_name(SHORTCUT_NAME_PAGE) == Some("name-page")
    );
    // The clock and battery corner, on any screen (Rust only).
    check!(
        shortcuts_lookup(&m, true, 616, 0) == SHORTCUT_STATUS
            && shortcut_action("status-toggle") == SHORTCUT_STATUS
    );
    check!(
        shortcut_action_name(SHORTCUT_STATUS) == Some("status-toggle")
            && SHORTCUT_STATUS + 8 == SHORTCUT_STATUS_MODE && SHORTCUT_STATUS_MODE + 22 == SHORTCUT_LAST
    );
    // The Index's actions: by their names, in the files and the Buttons sheet.
    for (name, id) in [
        ("links", SHORTCUT_LINKS),
        ("go-to-page", SHORTCUT_GO_TO_PAGE),
        ("go-back", SHORTCUT_GO_BACK),
        ("link-areas", SHORTCUT_LINK_AREAS),
        ("landscape-fit", SHORTCUT_LANDSCAPE_FIT),
    ] {
        check!(shortcut_action(name) == id && shortcut_action_name(id) == Some(name));
    }
    check!(
        SHORTCUT_LINKS == 48
            && SHORTCUT_LANDSCAPE_FIT + 1 == SHORTCUT_STATUS_CORNER
            && SHORTCUT_STATUS_CORNER + 1 == SHORTCUT_GESTURE_NEW_PAGE
            && SHORTCUT_GESTURE_NEW_PAGE + 1 == SHORTCUT_STATUS_MODE
            && SHORTCUT_STATUS_MODE + 1 == SHORTCUT_ZOOM_OUT && SHORTCUT_UPDATE_CHANNEL == SHORTCUT_LAST
    );
    check!(
        shortcut_action("status-mode") == SHORTCUT_STATUS_MODE
            && shortcut_action_name(SHORTCUT_STATUS_MODE) == Some("status-mode")
    );
    // The clock corner's place steps with its own action, which acts where status-toggle does.
    check!(
        shortcut_action("gesture-new-page") == SHORTCUT_GESTURE_NEW_PAGE
            && shortcut_action_name(SHORTCUT_GESTURE_NEW_PAGE) == Some("gesture-new-page")
    );
    check!(
        shortcut_action("status-corner") == SHORTCUT_STATUS_CORNER
            && shortcut_action_name(SHORTCUT_STATUS_CORNER) == Some("status-corner")
    );
    check!(
        shortcuts_lookup(&m, false, 44, 0) == SHORTCUT_UNDO
            && shortcuts_lookup(&m, true, 307, 0) == SHORTCUT_NONE
    );
    unlink(&path);
    // Learned into controller.ini by name: every action a button, which read back, with room to spare.
    let sp = format!("{}/shortcuts-all.ini", tl_tmp());
    let cp = format!("{}/controller-all.ini", tl_tmp());
    unlink(&sp);
    unlink(&cp);
    shortcuts_load_all(&mut m, &sp, &cp);
    let mut code = 600;
    for a in 1..=SHORTCUT_LAST {
        if shortcut_action_name(a).is_some() && action_sources(a) & SRC_PAD != 0 {
            check!(shortcuts_learn(&mut m, &cp, ActionSrc::Pad, code, a) == 0);
            code += 1;
        }
    }
    check!(
        shortcuts_learn(&mut m, &cp, ActionSrc::Pad, 312, SHORTCUT_HOLD_ERASER) == 0
            && shortcuts_learn(&mut m, &cp, ActionSrc::Pad, 316, SHORTCUT_CLOSE_MENU) == 0
    );
    let saved = read_file(&cp);
    check!(saved
        .as_ref()
        .is_some_and(|t| has(t, "pad.L2=hold-eraser\n")
            && has(t, "pad.316=close-menu\n")
            && has(t, "=tool:calligraphy\n")));
    let mut back = Shortcuts::default();
    shortcuts_load_all(&mut back, &sp, &cp);
    check!(back.rejected == 0 && back.count == 17 + named - 2); // the default keys, and a button each
    for a in 1..=SHORTCUT_LAST {
        if shortcut_action_name(a).is_some() && action_sources(a) & SRC_PAD != 0 {
            check!(shortcuts_pads_for(&back, ActionSrc::Pad, a, &mut []) == 1);
        }
    }
    check!(shortcuts_lookup(&back, true, 312, 0) == SHORTCUT_HOLD_ERASER);
    unlink(&sp);
    unlink(&cp);
}
// No version before 2.0.0-beta.5 read a D-pad, so a shortcuts.ini it wrote binds none of its
// directions: the D-pad's starter lines come with it, in memory alone. Not for a file that binds one,
// an empty file, or controller.ini (whose buttons were learned: a learn from the map writes them there).
fn dpad_for_files_written_before() {
    let sp = format!("{}/shortcuts-old.ini", tl_tmp());
    let cp = format!("{}/controller-old.ini", tl_tmp());
    unlink(&sp);
    unlink(&cp);
    let old = "key.Left=previous-page\npad.304=undo\npad.314=previous-page\n";
    check!(write_file_atomic(&sp, old.as_bytes()) == 0);
    let mut m = Shortcuts::default();
    shortcuts_load_all(&mut m, &sp, &cp);
    check!(m.count == 3 + 4 && m.rejected == 0);
    check!(
        shortcuts_lookup(&m, true, 544, 0) == SHORTCUT_PREV_TOOL
            && shortcuts_lookup(&m, true, 545, 0) == SHORTCUT_NEXT_TOOL
    );
    check!(
        shortcuts_lookup(&m, true, 546, 0) == SHORTCUT_PREV_PAGE
            && shortcuts_lookup(&m, true, 547, 0) == SHORTCUT_NEXT_PAGE
    );
    let text = read_file(&sp);
    check!(text.as_deref() == Some(old.as_bytes())); // the file as it was
    check!(shortcuts_learn(&mut m, &cp, ActionSrc::Pad, 305, SHORTCUT_REDO) == 0);
    let text = read_file(&cp);
    check!(text
        .as_ref()
        .is_some_and(|t| has(t, "pad.Up=previous-tool\n") && has(t, "pad.Right=next-page\n")));
    let learned = "pad.South=redo\n"; // controller.ini without the D-pad: as it says
    check!(write_file_atomic(&cp, learned.as_bytes()) == 0);
    shortcuts_load_all(&mut m, &sp, &cp);
    check!(
        m.count == 2
            && shortcuts_lookup(&m, true, 544, 0) == SHORTCUT_NONE
            && shortcuts_lookup(&m, true, 304, 0) == SHORTCUT_REDO
    );
    unlink(&cp);
    let one = "pad.Left=undo\n";
    check!(write_file_atomic(&sp, one.as_bytes()) == 0);
    shortcuts_load_all(&mut m, &sp, &cp);
    check!(
        m.count == 1
            && shortcuts_lookup(&m, true, 546, 0) == SHORTCUT_UNDO
            && shortcuts_lookup(&m, true, 544, 0) == SHORTCUT_NONE
    );
    check!(write_file_atomic(&sp, b"") == 0);
    shortcuts_load_all(&mut m, &sp, &cp);
    check!(m.count == 0);
    // Nearly full: what there is room for, none refused.
    let mut many = String::new();
    for i in 1..=SHORTCUT_MAX - 2 {
        many.push_str(&cformat!("key.code%d=undo\n", i));
    }
    check!(write_file_atomic(&sp, many.as_bytes()) == 0);
    shortcuts_load_all(&mut m, &sp, &cp);
    check!(
        m.count == SHORTCUT_MAX as i32
            && m.rejected == 0
            && shortcuts_lookup(&m, true, 545, 0) == SHORTCUT_NEXT_TOOL
            && shortcuts_lookup(&m, true, 546, 0) == SHORTCUT_NONE
    );
    unlink(&sp);
}
// A Switch-mode controller's buttons as printed on it; others, and buttons it does not have, as the
// files name them.
fn switch_labels() {
    let mut name = String::new();
    shortcut_pad_label(304, true, &mut name, 16);
    check!(name == "B");
    shortcut_pad_label(306, true, &mut name, 16);
    check!(name == "Y");
    shortcut_pad_label(310, true, &mut name, 16);
    check!(name == "ZL");
    shortcut_pad_label(312, true, &mut name, 16);
    check!(name == "-");
    shortcut_pad_label(317, true, &mut name, 16);
    check!(name == "Capture");
    shortcut_pad_label(318, true, &mut name, 16);
    check!(name == "318");
    shortcut_pad_label(544, true, &mut name, 16);
    check!(name == "Up");
    shortcut_pad_label(304, false, &mut name, 16);
    check!(name == "South");
    shortcut_pad_label(306, false, &mut name, 16);
    check!(name == "306");
}
// A Buttons sheet row held: its actions' buttons go, one row at a time. controller.ini keeps every
// other gamepad binding (shortcuts.ini's pad. lines and the D-pad's starter lines among them, set
// aside from then on), so the row stays as it is after a reload; keys are untouched.
fn a_row_held_forgets_its_buttons() {
    let sp = format!("{}/shortcuts-f.ini", tl_tmp());
    let cp = format!("{}/controller-f.ini", tl_tmp());
    unlink(&sp);
    unlink(&cp);
    let old =
        "key.Ctrl+z=undo\npad.304=undo\npad.314=previous-page\npad.315=next-page\npad.307=redo\n";
    check!(write_file_atomic(&sp, old.as_bytes()) == 0);
    let mut m = Shortcuts::default();
    shortcuts_load_all(&mut m, &sp, &cp); // with the D-pad's starter lines (Left/Right turn pages too)
    check!(
        shortcuts_pads_for(&m, ActionSrc::Pad, SHORTCUT_PREV_PAGE, &mut []) == 2
            && shortcuts_pads_for(&m, ActionSrc::Pad, SHORTCUT_NEXT_PAGE, &mut []) == 2
    );
    // A row with no button: nothing written, no controller.ini yet.
    check!(
        shortcuts_forget(&mut m, &cp, ActionSrc::Pad, SHORTCUT_CLEAR_PAGE, 0) == 0 && !exists(&cp)
    );
    // Undo's row: South goes; the keyboard's Ctrl+Z stays.
    check!(shortcuts_forget(&mut m, &cp, ActionSrc::Pad, SHORTCUT_UNDO, 0) == 0);
    check!(
        shortcuts_lookup(&m, true, 304, 0) == SHORTCUT_NONE
            && shortcuts_lookup(&m, false, 44, KEYMOD_CTRL) == SHORTCUT_UNDO
    );
    check!(shortcuts_lookup(&m, true, 307, 0) == SHORTCUT_REDO);
    // Previous / next page's row: both actions, Select, Start and the D-pad's Left and Right.
    check!(
        shortcuts_forget(
            &mut m,
            &cp,
            ActionSrc::Pad,
            SHORTCUT_PREV_PAGE,
            SHORTCUT_NEXT_PAGE
        ) == 0
    );
    check!(
        shortcuts_pads_for(&m, ActionSrc::Pad, SHORTCUT_PREV_PAGE, &mut []) == 0
            && shortcuts_pads_for(&m, ActionSrc::Pad, SHORTCUT_NEXT_PAGE, &mut []) == 0
    );
    check!(shortcuts_lookup(&m, true, 544, 0) == SHORTCUT_PREV_TOOL); // other rows keep theirs
    let text = read_file(&cp);
    check!(text.as_ref().is_some_and(|t| !has(t, "=undo")
        && !has(t, "page")
        && has(t, "pad.North=redo\n")
        && has(t, "pad.Up=previous-tool\n")
        && !has(t, "key.")));
    // Read back: as it was left (shortcuts.ini's pad. lines set aside, no D-pad starter lines).
    let mut back = Shortcuts::default();
    shortcuts_load_all(&mut back, &sp, &cp);
    check!(back.rejected == 0 && shortcuts_lookup(&back, true, 304, 0) == SHORTCUT_NONE);
    check!(
        shortcuts_pads_for(&back, ActionSrc::Pad, SHORTCUT_PREV_PAGE, &mut []) == 0
            && shortcuts_lookup(&back, true, 546, 0) == SHORTCUT_NONE
    );
    check!(
        shortcuts_lookup(&back, true, 307, 0) == SHORTCUT_REDO
            && shortcuts_lookup(&back, false, 44, KEYMOD_CTRL) == SHORTCUT_UNDO
    );
    // The last ones: controller.ini with none, which stays (Reset buttons deletes it).
    check!(
        shortcuts_forget(&mut back, &cp, ActionSrc::Pad, SHORTCUT_REDO, 0) == 0
            && shortcuts_forget(
                &mut back,
                &cp,
                ActionSrc::Pad,
                SHORTCUT_PREV_TOOL,
                SHORTCUT_NEXT_TOOL
            ) == 0
    );
    shortcuts_load_all(&mut m, &sp, &cp);
    check!(
        m.rejected == 0
            && m.count == 1
            && shortcuts_lookup(&m, false, 44, KEYMOD_CTRL) == SHORTCUT_UNDO
    );
    // A trigger learned and forgotten by its name.
    check!(
        shortcuts_learn(
            &mut m,
            &cp,
            ActionSrc::Pad,
            HIDKEYS_LT,
            SHORTCUT_HOLD_ERASER
        ) == 0
    );
    let text = read_file(&cp);
    check!(text
        .as_ref()
        .is_some_and(|t| has(t, "pad.LT=hold-eraser\n")));
    shortcuts_load_all(&mut back, &sp, &cp);
    check!(shortcuts_lookup(&back, true, HIDKEYS_LT, 0) == SHORTCUT_HOLD_ERASER);
    // A file that cannot be written leaves the map as it was.
    let bad = format!("{}/no-such-dir/controller.ini", tl_tmp());
    check!(
        shortcuts_forget(&mut back, &bad, ActionSrc::Pad, SHORTCUT_HOLD_ERASER, 0) != 0
            && shortcuts_lookup(&back, true, HIDKEYS_LT, 0) == SHORTCUT_HOLD_ERASER
    );
    unlink(&sp);
    unlink(&cp);
}
#[test]
fn test_shortcuts() {
    run!(defaults_and_exact_modifiers);
    run!(custom_mappings_and_limits);
    run!(settings_are_opt_in);
    run!(controller_ini_learns_one_button_per_action);
    run!(every_action_by_name);
    run!(dpad_for_files_written_before);
    run!(switch_labels);
    run!(a_row_held_forgets_its_buttons);
    done();
}
