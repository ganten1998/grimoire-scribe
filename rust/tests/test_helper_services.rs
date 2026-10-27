// Not a port of a C test: the forked-child paths of the helper services (bluetooth.c's script run,
// update.c's check, light_kindle.c's lipc calls), run end to end against shell stand-ins. The C
// leaves these to tests/bt_check.sh and the launcher checks, which need the whole app; here a
// module's own contract is checked as soon as it is translated.
mod testlib;
use grimoire::light_kindle as lk;
use grimoire::prelude::*;
use std::os::unix::fs::PermissionsExt;
use testlib::*;

fn scratch(name: &str) -> String {
    let dir = format!("{}/helper-{}-{}", tl_tmp(), name, std::process::id());
    let _ = std::fs::remove_dir_all(&dir);
    check!(mkdir_p(&dir) == 0);
    dir
}

fn script(path: &str, body: &str) {
    check!(write_file_atomic(path, body.as_bytes()) == 0);
    check!(std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).is_ok());
}

fn wait_for(mut what: impl FnMut() -> bool) -> bool {
    let t0 = now_ms();
    while now_ms().wrapping_sub(t0) < 10000 {
        if what() {
            return true;
        }
        sleep_ms(10);
    }
    false
}

fn the_update_check_reports_what_the_script_says() {
    let dir = scratch("update");
    let sh = format!("{}/update.sh", dir);
    let kpm = "\x1b[32mFound 2 package(s) for grimoire:\x1b[0m\n  - grimoire (Grimoire): Notebooks. 2.0.0-beta.6\n";
    script(&sh, &format!("#!/bin/sh\necho \"$1 $2\" >> \"$UPDATE_TEST_LOG\"\nprintf '{}'\necho oops >&2\nexit ${{UPDATE_TEST_CODE:-0}}\n", kpm.replace('\x1b', "\\033").replace('\n', "\\n")));
    let log = format!("{}/calls", dir);
    std::env::set_var("UPDATE_TEST_LOG", &log);
    update_init(Some(&sh), Some(&format!("{}/request", dir)));
    check!(update_request_path() == format!("{}/request", dir));
    check!(update_available());
    let mut latest = String::new();
    let mut why = String::new();
    check!(update_tick(&mut latest, &mut why, 16) == UpdateEvent::None); // nothing under way
    for (code, want) in [("0", ""), ("3", "no-kpm"), ("4", "feed"), ("5", "network"), ("1", "failed")] {
        std::env::set_var("UPDATE_TEST_CODE", code);
        check!(update_check(code == "0"));
        check!(update_checking() && !update_check(false)); // one at a time
        let mut e = UpdateEvent::None;
        check!(wait_for(|| {
            e = update_tick(&mut latest, &mut why, 16);
            e != UpdateEvent::None
        }));
        check!(!update_checking());
        if code == "0" {
            check!(e == UpdateEvent::Done && latest == "2.0.0-beta.6");
        } else {
            check!(e == UpdateEvent::Failed && why == want);
        }
    }
    let calls = read_file(&log).map(|b| String::from_utf8_lossy(&b).into_owned()).unwrap_or_default();
    check!(calls == "check preview\ncheck stable\ncheck stable\ncheck stable\ncheck stable\n");
    update_init(None, None);
    check!(!update_available() && update_request_path() == "/tmp/grimoire-update");
    check!(!update_check(true));
    // A cut reason: snprintf's size 4 keeps three bytes.
    update_init(Some(&sh), Some(&format!("{}/request", dir)));
    std::env::set_var("UPDATE_TEST_CODE", "3");
    check!(update_check(false));
    check!(wait_for(|| update_tick(&mut latest, &mut why, 4) != UpdateEvent::None));
    check!(why == "no-");
    std::env::remove_var("UPDATE_TEST_CODE");
}

fn the_helper_script_runs_as_a_child() {
    let dir = scratch("bt");
    let sh = format!("{}/bluetooth.sh", dir);
    let seen = format!("{}/seen", dir);
    script(
        &sh,
        &format!(
            "#!/bin/sh\necho \"$1 $GRIMOIRE_ROOT\" >> '{}'\ncase \"$1\" in start) echo owned;; stop-owned) echo stopped;; esac\nexit 0\n",
            seen
        ),
    );
    let mut r = BtReply::default();
    bt_init(&dir, Some(&sh), None);
    check!(bt_state() == BtState::Stopped && !bt_present() && !bt_busy());
    check!(bt_tick(&mut r) == BtEvent::None);
    let khp = format!("{}/bluetooth/kindle-hid-passthrough", dir);
    check!(mkdir_p(&format!("{}/bluetooth", dir)) == 0);
    script(&khp, "#!/bin/sh\n");
    check!(bt_present());
    bt_start();
    check!(bt_state() == BtState::Starting);
    check!(wait_for(|| bt_tick(&mut r) == BtEvent::State));
    check!(bt_state() == BtState::Owned);
    bt_start(); // already owned: nothing runs
    check!(bt_state() == BtState::Owned && bt_tick(&mut r) == BtEvent::None);
    bt_stop();
    check!(bt_state() == BtState::Stopping);
    check!(wait_for(|| bt_tick(&mut r) == BtEvent::State));
    check!(bt_state() == BtState::Stopped);
    bt_stop(); // already stopped: nothing runs
    check!(bt_state() == BtState::Stopped);
    let text = read_file(&seen).map(|b| String::from_utf8_lossy(&b).into_owned()).unwrap_or_default();
    check!(text == format!("start {0}\nstop-owned {0}\n", dir));
    // A script that says nothing it knows: failed, but a stop that gets it reads as stopped.
    script(&sh, "#!/bin/sh\necho confused\n");
    bt_start();
    check!(wait_for(|| bt_tick(&mut r) == BtEvent::State));
    check!(bt_state() == BtState::Failed);
    bt_init(&dir, Some(&sh), None);
    script(&sh, "#!/bin/sh\necho external\n");
    bt_start();
    check!(wait_for(|| bt_tick(&mut r) == BtEvent::State));
    check!(bt_state() == BtState::External);
    bt_stop(); // someone else's: left running, only forgotten
    check!(bt_state() == BtState::Stopped);
    script(&sh, "#!/bin/sh\necho confused\n");
    bt_init(&dir, Some(&sh), None);
    bt_start();
    check!(wait_for(|| bt_tick(&mut r) == BtEvent::State));
    check!(bt_state() == BtState::Failed);
    bt_stop();
    check!(wait_for(|| bt_tick(&mut r) == BtEvent::State));
    check!(bt_state() == BtState::Stopped); // (a failed stop is a stop)
    bt_init(&dir, None, None);
}

fn lipc_is_asked_through_the_shell() {
    let dir = scratch("light");
    let log = format!("{}/calls", dir);
    let saved = std::env::var("PATH").unwrap_or_else(|_| "/usr/bin:/bin".to_string());
    std::env::set_var("PATH", format!("{}:{}", dir, saved));
    std::env::set_var("LIGHT_TEST_LOG", &log);
    script(
        &format!("{}/lipc-get-prop", dir),
        "#!/bin/sh\ncase \"$*\" in\n*flIntensity*) echo 7;;\n*currentAmberLevel*) echo 12;;\n*state*) echo '  screenSaver  ';;\nesac\n",
    );
    script(&format!("{}/lipc-set-prop", dir), "#!/bin/sh\necho \"$*\" >> \"$LIGHT_TEST_LOG\"\n");
    let (mut i, mut w) = (0, 0);
    check!(lk::light_warmth() == -1);
    lk::light_get(&mut i, &mut w); // never read: asks
    check!(i == 7 && w == 12 && lk::light_warmth() == 12);
    lk::light_set_intensity(30); // clamped to LIGHT_MAX
    lk::light_set_warmth(-3);
    lk::light_get(&mut i, &mut w);
    check!(i == LIGHT_MAX && w == 0);
    check!(wait_for(|| read_file(&log).is_some_and(|b| b.iter().filter(|&&c| c == b'\n').count() == 2)));
    let calls = String::from_utf8_lossy(&read_file(&log).unwrap_or_default()).into_owned();
    check!(calls.contains("-i com.lab126.powerd flIntensity 24\n") && calls.contains("-i com.lab126.powerd currentAmberLevel 0\n"));
    let mut state = String::new();
    check!(lk::sys_powerd_state(&mut state, 64) == 0 && state == "screenSaver");
    check!(lk::sys_powerd_state(&mut state, 5) == 0 && state == "scre");
    lk::sys_prevent_sleep(true);
    lk::sys_activity();
    lk::sys_request_sleep();
    check!(wait_for(|| read_file(&log).is_some_and(|b| b.iter().filter(|&&c| c == b'\n').count() == 6)));
    let calls = String::from_utf8_lossy(&read_file(&log).unwrap_or_default()).into_owned();
    check!(calls.contains("preventScreenSaver 1\n") && calls.contains("touchScreenSaverTimeout 1\n"));
    check!(calls.contains("preventScreenSaver 0\n") && calls.contains("powerButton 1\n"));
    // A power service that says nothing: no state.
    script(&format!("{}/lipc-get-prop", dir), "#!/bin/sh\n");
    check!(lk::sys_powerd_state(&mut state, 64) == -1);
    // Without the launcher's marks, the native UI is left alone and the call succeeds at once.
    std::env::remove_var("GRIMOIRE_AWESOME_STOPPED");
    std::env::remove_var("GRIMOIRE_PILLOW_DISABLED");
    check!(lk::sys_kindle_ui(true) && lk::sys_kindle_ui(false));
    std::env::set_var("PATH", saved);
}

#[test]
fn test_helper_services() {
    run!(the_update_check_reports_what_the_script_says);
    run!(the_helper_script_runs_as_a_child);
    run!(lipc_is_asked_through_the_shell);
    done();
}
