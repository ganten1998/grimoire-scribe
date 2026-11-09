//! The caps every background child carries (DESIGN.md 4 and 7, lane L9): `oom_score_adj` 500, so the
//! kernel takes background work before the UI, and `RLIMIT_AS` at the child's address space as it was
//! forked (the UI's, copy-on-write) plus a margin for its kind, so a child that grows without bound
//! fails by itself instead of taking the Kindle with it. The bgjob children, the detached child and
//! the share server's render children all take them. They are Linux's: elsewhere (the host build)
//! nothing is applied, and the plan says so (`child_caps_plan`'s `as_bytes` is None).

use crate::prelude::*;

/// What every background child's `oom_score_adj` is: above the UI's (0), below the PDF worker's (1000).
pub const CHILD_OOM_ADJ: i32 = 500;

/// How much address space beyond its size at fork a child of this kind may take. The mirror streams
/// (one page's canvas, its picture and its compressed stream): 64 MiB. A picture job has its own tighter
/// cap around the decode (bgwork_template.rs), which this one stays above.
pub fn child_margin(kind: BgKind) -> u64 {
    const MIB: u64 = 1 << 20;
    match kind {
        BgKind::MirrorPdf => 64 * MIB,
        BgKind::GraphLayout | BgKind::GraphLayoutAll | BgKind::PagePreviews => 128 * MIB,
        BgKind::Template | BgKind::ImportFolder => 512 * MIB,
        // MERGE-ME: lane L5's BgKind::PdfImport takes its margin here (its writer holds a 4.6 MB gray
        // page and PNG buffers: the default is more than it needs).
        _ => 256 * MIB,
    }
}

/// The share server's render children: a page's PNG, a raw page in a zip, a picture, the spool's blocks.
pub const SHARE_CHILD_MARGIN: u64 = 128 << 20;

/// The caps a child gets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChildCaps {
    pub oom_adj: i32,
    /// the `RLIMIT_AS` soft limit; None where the address space is not known (not Linux)
    pub as_bytes: Option<u64>,
}

/// The caps for a child whose address space is `vm` bytes now and whose kind's margin is `margin`.
pub fn child_caps_plan(vm: Option<u64>, margin: u64) -> ChildCaps {
    ChildCaps { oom_adj: CHILD_OOM_ADJ, as_bytes: vm.map(|v| v + margin) }
}

/// This process's address space in bytes (the first number of /proc/self/statm), Linux only.
pub fn child_vm_bytes() -> Option<u64> {
    #[cfg(target_os = "linux")]
    {
        let statm = std::fs::read("/proc/self/statm").ok()?;
        let digits: Vec<u8> = statm.iter().skip_while(|c| c.is_ascii_whitespace()).take_while(|c| c.is_ascii_digit()).copied().collect();
        let pages = std::str::from_utf8(&digits).ok()?.parse::<u64>().ok()?;
        // SAFETY: sysconf only reads a configuration value.
        let psz = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
        if psz <= 0 {
            return None;
        }
        Some(pages * psz as u64)
    }
    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}

/// Puts the caps on this process (in a child, right after the fork): best effort, each by itself, and
/// the soft limit only, never above the hard one nor over a lower one already set. Returns what took.
pub fn child_caps_apply(margin: u64) -> ChildCaps {
    let plan = child_caps_plan(child_vm_bytes(), margin);
    #[allow(unused_mut)] // (only Linux takes any)
    let mut took = ChildCaps { oom_adj: 0, as_bytes: None };
    #[cfg(target_os = "linux")]
    {
        if std::fs::write("/proc/self/oom_score_adj", plan.oom_adj.to_string()).is_ok() {
            took.oom_adj = plan.oom_adj;
        }
        if let Some(cap) = plan.as_bytes {
            let mut was = libc::rlimit { rlim_cur: 0, rlim_max: 0 };
            // SAFETY: getrlimit writes one rlimit into was, which we own.
            if unsafe { libc::getrlimit(libc::RLIMIT_AS, &mut was) } == 0 {
                let cap = cap as libc::rlim_t;
                let lower = was.rlim_cur != libc::RLIM_INFINITY && was.rlim_cur <= cap;
                let over = was.rlim_max != libc::RLIM_INFINITY && cap > was.rlim_max;
                if lower {
                    took.as_bytes = Some(was.rlim_cur as u64); // as low already
                } else if !over {
                    let r = libc::rlimit { rlim_cur: cap, rlim_max: was.rlim_max };
                    // SAFETY: setrlimit reads one rlimit from r.
                    if unsafe { libc::setrlimit(libc::RLIMIT_AS, &r) } == 0 {
                        took.as_bytes = Some(cap as u64);
                    }
                }
            }
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = plan;
    }
    #[cfg(feature = "sim")]
    child_caps_probe(&took);
    took
}

/// Tests (the simulator's build): a child says what it took, one line, to the file named by
/// GRIMOIRE_CAPS_PROBE, for the test that waits for it to read.
#[cfg(feature = "sim")]
fn child_caps_probe(took: &ChildCaps) {
    use std::io::Write;
    let Some(path) = std::env::var_os("GRIMOIRE_CAPS_PROBE") else { return };
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(f, "oom={} as={}", took.oom_adj, took.as_bytes.map_or(-1, |v| v as i64));
    }
}
