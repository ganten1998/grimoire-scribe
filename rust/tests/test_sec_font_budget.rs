// test_sec_font_budget.rs (security review 2026-11, finding "the font check's budget is the worker's word"): a font's
// check runs in slices, each a fresh worker, until the CPU time the workers REPORT reaches the job's budget
// (`N <next> <ms>`). A worker the font took over reports 1 ms and moves on by one glyph: the writer then runs up to
// 65,535 slices (each up to RLIMIT_CPU's 60 s, and, with a record every 29 s from a worker asleep in futex, as long
// in wall time as it likes), holding the one background slot that exports, mirrors and imports wait on.
//
// The stand-in worker here does exactly that, for at most 40 slices; with a 1 s slice and a 3 s budget, the check must
// end as "took too long" within the slices that budget holds.
#![cfg(feature = "sim")]
use grimoire::fontcheck::*;

#[test]
fn a_worker_that_underreports_its_time_cannot_run_the_check_for_ever() {
    use std::os::unix::fs::PermissionsExt;
    let base = std::env::var("GRIMOIRE_TEST_TMP").map(std::path::PathBuf::from).unwrap_or_else(|_| std::env::temp_dir());
    let dir = base.join(format!("grimoire-sec-fontbudget-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let count = dir.join("slices");
    let script = dir.join("worker.sh");
    std::fs::write(
        &script,
        format!(
            "#!/bin/sh\nPATH=/usr/bin:/bin\nprintf 'B {}\\nS 1\\n' >&4\necho x >> '{c}'\nn=$(wc -l < '{c}')\n\
             read -r cmd len next px slice scale <&5\n\
             if [ \"$n\" -gt 40 ]; then printf 'E bad 0\\n' >&4; sleep 5; exit 0; fi\n\
             if [ \"$next\" = 0 ]; then printf 'F face 1000 800 200 1000 400 0 100\\n' >&4; fi\n\
             printf 'N %d 1\\n' $((next + 1)) >&4\nsleep 5\n",
            grimoire::sandbox::build_id(),
            c = count.display()
        ),
    )
    .unwrap();
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    std::env::set_var("GRIMOIRE_WORK_EXE", &script);
    std::env::set_var("GRIMOIRE_FONT_SLICE_MS", "1000");
    std::env::set_var("GRIMOIRE_FONT_JOB_MS", "3000");
    let data = vec![0u8; 1000];
    let out = check_font(&data, 44);
    let slices = std::fs::read_to_string(&count).unwrap_or_default().lines().count();
    eprintln!("  the check came to {out:?} after {slices} slice(s) (budget 3 s of 1 s slices)");
    assert!(matches!(out, FontOut::Slow), "{out:?}");
    assert!(slices <= 5, "{slices} slices ran on a 3 s budget of 1 s slices");
    let _ = std::fs::remove_dir_all(&dir);
}
