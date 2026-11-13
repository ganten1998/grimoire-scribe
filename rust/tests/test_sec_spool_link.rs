// test_sec_spool_link.rs (security review 2026-11, finding "the share spool is written through a link"): the share
// server's render child makes each download in grimoire/.cache/pdf/share-<pid>.spool (httpd.rs spool_create ->
// export.rs FileSink::create_spool), as root, before it sends it; any phone or computer on the LAN asks for one
// while Share is open. The spool was opened create + truncate, following a link: a link planted at that name (the pid
// is in grimoire.log) made root truncate its target and write the whole PDF or zip into it.
use grimoire::prelude::*;

#[test]
fn a_spool_is_never_written_through_a_link() {
    let base = std::env::var("GRIMOIRE_TEST_TMP").map(std::path::PathBuf::from).unwrap_or_else(|_| std::env::temp_dir());
    let top = base.join(format!("grimoire-sec-spool-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&top);
    std::fs::create_dir_all(top.join("cache")).unwrap();
    let victim = top.join("shadow");
    std::fs::write(&victim, b"root:$6$secret:19000:0:99999:7:::\n").unwrap();
    let spool = top.join("cache/share-4242.spool");
    std::os::unix::fs::symlink(&victim, &spool).unwrap();
    let f = FileSink::create_spool(spool.to_str().unwrap());
    let v = std::fs::read(&victim).unwrap();
    eprintln!("  spool made: {}; the linked file now holds {} bytes", f.is_some(), v.len());
    assert_eq!(v, b"root:$6$secret:19000:0:99999:7:::\n", "the spool truncated the file its name's link points at");
    if f.is_some() {
        assert!(std::fs::symlink_metadata(&spool).unwrap().file_type().is_file(), "the spool is the link");
    }
    let _ = std::fs::remove_dir_all(&top);
}
