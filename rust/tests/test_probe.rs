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
        let (Some(dot), Some(eq)) = (line.iter().position(|&c| c == b'.'), line.iter().position(|&c| c == b'=')) else { continue };
        if dot > eq {
            continue;
        }
        let head = &line[..dot];
        if head == b"pen" || head == b"touch" {
            continue; // the calibrations
        }
        let mut t = 0;
        while t < TOOL_COUNT && app_probe_tool(ToolKind(t as i32)).map(str::as_bytes) != Some(head) {
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

#[test]
fn test_probe() {
    let root = format!("{}/probe-{}", tl_tmp(), std::process::id());
    let _ = std::fs::remove_dir_all(&root);
    check!(store_init(&root) == 0);
    run!(tool_names_are_settings_ini_names);
    let _ = std::fs::remove_dir_all(&root);
    done();
}
