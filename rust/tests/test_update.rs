// test_update.c: Settings > Updates' pure parts: versions as KPM orders them (as tools/kpm-package.py
// encodes them), the newest Grimoire in `kpm search grimoire`'s output, and the request file.
mod testlib;
use grimoire::prelude::*;
use testlib::*;

fn versions_order_as_kpm_orders_them() {
    let mut k = [0u32; 3];
    check!(update_version_key("2.0.0-beta.5", &mut k) && k[0] == 2 && k[1] == 0 && k[2] == 1005);
    check!(update_version_key("2.0.0-rc.1", &mut k) && k[2] == 5001);
    check!(update_version_key("2.0.0", &mut k) && k[2] == 9999);
    check!(update_version_key("2.0.1", &mut k) && k[2] == 19999);
    check!(update_version_key("1.2.0", &mut k) && k[0] == 1 && k[1] == 2 && k[2] == 9999);
    check!(!update_version_key("2.0", &mut k) && !update_version_key("2.0.0-beta.0", &mut k) && !update_version_key("2.0.0-beta.01", &mut k));
    check!(!update_version_key("2.0.0-alpha.1", &mut k) && !update_version_key("2.0.0-beta.1000", &mut k) && !update_version_key("v2.0.0", &mut k));
    // (The C also asks with NULL: Rust has no NULL string.)
    check!(!update_version_key("2.0.0 ", &mut k) && !update_version_key("", &mut k));
    check!(update_version_cmp("2.0.0-beta.4", "2.0.0-beta.5") < 0);
    check!(update_version_cmp("2.0.0-beta.9", "2.0.0-rc.1") < 0);
    check!(update_version_cmp("2.0.0-rc.3", "2.0.0") < 0);
    check!(update_version_cmp("2.0.0", "2.0.1-beta.1") < 0); // a hotfix's beta is newer than the release
    check!(update_version_cmp("2.0.1", "2.1.0-beta.1") < 0);
    check!(update_version_cmp("1.2.0", "2.0.0-beta.1") < 0);
    check!(update_version_cmp("2.0.0-beta.5", "2.0.0-beta.5") == 0);
    check!(update_version_cmp("junk", "1.0.0") < 0 && update_version_cmp("1.0.0", "junk") > 0);
    check!(update_is_prerelease("2.0.0-beta.5") && update_is_prerelease("2.0.0-rc.2") && !update_is_prerelease("2.0.0"));
}

fn the_newest_grimoire_in_kpms_search() {
    let mut v = String::new();
    // As KPM 0.2 prints it: an INFO line, then a line per package found, colour codes about.
    let out = "\x1b[32mFound 3 package(s) for grimoire:\x1b[0m\n\
               \x20 - grimoire (Grimoire): Handwriting notebooks for the jailbroken Kindle Scribe. 2.0.0-beta.4\r\n\
               \x1b[32m  - grimoire (Grimoire): Handwriting notebooks for the jailbroken Kindle Scribe. 2.0.0-beta.6\x1b[0m\n\
               \x20 - grimoire (Grimoire): Handwriting notebooks for the jailbroken Kindle Scribe. 1.2.0\n\
               \x20 - grimoire-tools (Grimoire tools): Not ours 9.9.9\n";
    check!(update_latest(out, &mut v) && v == "2.0.0-beta.6");
    check!(!update_latest("Could not find any packages for 'grimoire'\n", &mut v) && v.is_empty());
    check!(!update_latest("  - grimoire (Grimoire): a description with no version\n", &mut v));
    // (The C also asks with NULL: Rust has no NULL string.)
    check!(!update_latest("", &mut v));
}

fn the_request_is_written_for_the_launcher() {
    let path = format!("{}/grimoire-update-{}", tl_tmp(), std::process::id());
    let script = format!("{}/no-such-update.sh", tl_tmp());
    update_init(Some(&script), Some(&path));
    check!(!update_available() && !update_check(true)); // no script: no updates here
    check!(update_request(true) == 0);
    let text = read_file(&path);
    check!(text.as_deref() == Some(&b"preview\n"[..]));
    check!(update_request(false) == 0);
    let text = read_file(&path);
    check!(text.as_deref() == Some(&b"stable\n"[..]));
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_update() {
    run!(versions_order_as_kpm_orders_them);
    run!(the_newest_grimoire_in_kpms_search);
    run!(the_request_is_written_for_the_launcher);
    done();
}
