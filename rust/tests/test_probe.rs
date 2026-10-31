// test_probe.c: the scripted runner's probes (app_probe in app.c) name things as the app's files do.
mod testlib;

use grimoire::prelude::*;
use std::io::BufRead;
use testlib::*;

// editor.tool and settings.TOOL.PARAM name tools with app_probe_tool, a copy of store.c's names:
// settings_save writes "<tool>.<param>=" for every tool and every setting, and the probes must
// name each of those tools (a tool added to one list and not the other fails here).
fn tool_names_are_settings_ini_names() {
    let mut s = Settings::default();
    settings_default(&mut s);
    check!(settings_save(&s) == 0);
    let path = format!("{}/settings.ini", store_root());
    let f = std::fs::File::open(&path);
    check!(f.is_ok());
    let Ok(f) = f else { return };
    let n = tool_param_table().len() as i32;
    let mut seen = [0i32; TOOL_COUNT];
    let mut unnamed = 0;
    for line in std::io::BufReader::new(f).split(b'\n') {
        let Ok(line) = line else { break };
        let (Some(dot), Some(eq)) = (
            line.iter().position(|&c| c == b'.'),
            line.iter().position(|&c| c == b'='),
        ) else {
            continue;
        };
        if dot > eq {
            continue;
        }
        let head = &line[..dot];
        if head == b"pen" || head == b"touch" {
            continue; // the calibrations
        }
        let mut t = 0;
        while t < TOOL_COUNT && app_probe_tool(ToolKind(t as i32)).map(str::as_bytes) != Some(head)
        {
            t += 1;
        }
        if t < TOOL_COUNT {
            seen[t] += 1;
        } else {
            unnamed += 1;
        }
    }
    for t in seen {
        check!(t == n);
    }
    check!(unnamed == 0);
    check!(app_probe_tool(ToolKind(TOOL_COUNT as i32)).is_none());
    check!(app_probe_tool(ToolKind(-1)).is_none());
}

// The gestures' settings keys (Index phase 1) are undotted (a dotted head is a calibration's or a
// tool's), come after the keys the build before them wrote, in the order gestures, landscape_fit,
// hints, link_areas (then the clock corner's place, status_corner), and leave every earlier line as that build wrote it: tests/fixtures/
// settings_default_head.ini is a default Settings saved before they existed, through status_size.
fn gesture_keys_follow_the_older_ones() {
    let mut s = Settings::default();
    settings_default(&mut s);
    check!(settings_save(&s) == 0);
    let got = read_file(&format!("{}/settings.ini", store_root())).unwrap_or_default();
    // Keep the historic ordering fixture frozen; apply only the deliberate later default changes.
    let head = include_str!("fixtures/settings_default_head.ini").replace("status_on=0", "status_on=1")
        .replace("highlighter.size_min_mm=3.0000", "highlighter.size_min_mm=5.0000")
        .replace("highlighter.size_max_mm=3.0000", "highlighter.size_max_mm=5.0000")
        .replace("highlighter.opacity_min=0.5000", "highlighter.opacity_min=0.2500")
        .replace("fountain.size_min_mm=0.2800", "fountain.size_min_mm=0.2500")
        .replace("fountain.size_max_mm=0.8000", "fountain.size_max_mm=1.0000")
        .replace("music_layout=1157628049", "music_layout=1124075809"); // (rc.9: treble, bass and tab)
    let want = head.as_bytes();
    check!(got.len() > want.len() && &got[..want.len()] == want);
    let tail = String::from_utf8_lossy(&got[want.len().min(got.len())..]).to_string();
    check!(tail == "gestures=1\nlandscape_fit=1\nhints=0\nlink_areas=0\nstatus_corner=0\nrotation_lock=0\ngesture_new_page=1\nstatus_mode=1\nshapes=1\nshape_hold_ms=550\nshape_quads=1\nshape_round=1\nshape_links=1\nshape_min_mm=0\nwhatsnew_seen=\nwhatsnew_hide=0\n");
    // (undotted: no key before its "=" has a dot)
    for line in tail.lines() {
        let key = line.split('=').next().unwrap_or("");
        check!(!key.contains('.'));
    }
}

#[test]
fn test_probe() {
    let root = format!("{}/probe-{}", tl_tmp(), std::process::id());
    let _ = std::fs::remove_dir_all(&root);
    check!(store_init(&root) == 0);
    run!(tool_names_are_settings_ini_names);
    run!(gesture_keys_follow_the_older_ones);
    let _ = std::fs::remove_dir_all(&root);
    done();
}
