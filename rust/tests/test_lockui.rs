// test_lockui.c: exercise the real native UI handover with command stand-ins: order, failure and
// timeout. (light_kindle is compiled in every build, as the C test includes light_kindle.c.)
mod testlib;
use grimoire::light_kindle::sys_kindle_ui;
use grimoire::prelude::*;
use std::os::unix::fs::PermissionsExt;
use testlib::*;

fn native_ui_boundary() {
    let dir = format!("{}/lock-ui-{}", tl_tmp(), std::process::id());
    check!(mkdir_p(&dir) == 0);
    let log = format!("{}/calls", dir);
    std::env::set_var("LOCK_TEST_LOG", &log);
    let before = std::env::var("PATH").ok();
    let saved = before.unwrap_or_else(|| "/usr/bin:/bin".to_string());
    let search = format!("{}:{}", dir, saved);
    std::env::set_var("PATH", &search);
    let mut path = format!("{}/killall", dir);
    let killshim = "#!/bin/sh\nprintf 'killall %s\\n' \"$*\" >>\"$LOCK_TEST_LOG\"\n";
    check!(write_file_atomic(&path, killshim.as_bytes()) == 0 && chmod(&path, 0o700));
    path = format!("{}/lipc-set-prop", dir);
    let lipcshim = "#!/bin/sh\nprintf 'lipc %s\\n' \"$*\" >>\"$LOCK_TEST_LOG\"\n\
                    [ \"${LOCK_TEST_FAIL:-}\" = yes ] && exit 1\nexit 0\n";
    check!(write_file_atomic(&path, lipcshim.as_bytes()) == 0 && chmod(&path, 0o700));
    std::env::set_var("GRIMOIRE_AWESOME_STOPPED", "yes");
    std::env::set_var("GRIMOIRE_PILLOW_DISABLED", "yes");
    check!(sys_kindle_ui(true));
    check!(sys_kindle_ui(false));
    let data = read_file(&log);
    let want = "killall -CONT awesome\nlipc com.lab126.pillow disableEnablePillow enable\n\
                lipc com.lab126.pillow disableEnablePillow disable\nkillall -STOP awesome\n";
    check!(data.as_deref() == Some(want.as_bytes()));
    std::env::set_var("LOCK_TEST_FAIL", "yes");
    check!(!sys_kindle_ui(false));
    std::env::remove_var("LOCK_TEST_FAIL");
    let slow = "#!/bin/sh\nsleep 6\n";
    check!(write_file_atomic(&path, slow.as_bytes()) == 0 && chmod(&path, 0o700));
    let started = now_ms();
    check!(!sys_kindle_ui(false));
    check!(now_ms().wrapping_sub(started) < 5000); // a stuck lipc must not trap the app indefinitely
    std::env::remove_var("GRIMOIRE_AWESOME_STOPPED");
    std::env::remove_var("GRIMOIRE_PILLOW_DISABLED");
    std::env::set_var("PATH", &saved);
}

fn chmod(path: &str, mode: u32) -> bool {
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).is_ok()
}

#[test]
fn test_lockui() {
    run!(native_ui_boundary);
    done();
}
