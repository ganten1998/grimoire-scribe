//! The safe loader (2.1, DESIGN.md 3.7, 4.2 and 4.3): one worker for every untrusted parser (PDFs, pictures
//! and fonts), a fresh exec of this very binary that inherits nothing and reaches nothing.
//!
//! ```text
//! UI process (root)     never parses untrusted bytes; holds the import lock; queues jobs
//!   | fork (bgjob)
//! writer: bgjob child   opens the source through fsview, owns the work folder, validates every record
//!   | fork, arrange fds 0 to 5, execve("/proc/self/exe", "grimoire --work <kind> ...", {})
//! worker                fds: 0 1 2 on /dev/null, 3 the PDF (read only), 4 the pipe to the writer, 5 the
//!                       pipe from the writer; the steps below, in order, then it parses on one thread
//! ```
//!
//! The steps (worker side, each checked; on the Kindle a failed one sends `E sandbox <step>` and exits before
//! a byte of the source is read): oom_score_adj 1000; unshare(CLONE_NEWNET | CLONE_NEWIPC); chroot to the
//! writer's empty directory on /tmp; setgroups(0), setgid and setuid to a uid of its own; no_new_privs and not
//! dumpable; the parent-death signal (set after the uid change, then getppid checked); rlimits (NOFILE 0,
//! NPROC 0, FSIZE 0, CORE 0, AS, CPU); a seccomp allow-list whose violation traps into a reporter.
//!
//! Off the Kindle (the simulator, CI, the Mac, not root) the steps that need root or Linux are skipped with
//! one log line (plain process isolation); the Linux CI job runs the real ones as root.

use crate::prelude::*;
use std::ffi::CString;

/// The numbers the device spike (S1, DESIGN.md section 9) sets. ONE table: nothing else in the tree holds a copy
/// of any of them. Measured on the owner's Scribe on 2026-11-09 (Linux 4.9, 2 cores, 433 MB available with
/// KOReader open): every sandbox step works there, a private read-only mapping of a file on /mnt/us works (so
/// PDFs are mapped, `PDF_MAX_MAPPED`), and hayro draws a planner page in 90 ms (median; 146 ms at most) and a
/// photo page in 230 ms (275 ms at most), using 27 to 38 MB whole and 13 to 25 MB in bands. The values marked
/// "set by S1" were the plan's placeholders (4.3, 7) and are kept: each has 30 to 60 times the room those
/// pages need (the page watchdog against the slowest page, the job's CPU budget against a 380-page planner's
/// 35 s, a slice's CPU cap against its pages). A page that is one huge scan was not measured.
pub mod s1 {
    /// A slice of a PDF import is at most this many pages ... (set by S1)
    pub const SLICE_PAGES: usize = 25;
    /// ... or this long in wall time, checked between pages. (set by S1)
    pub const SLICE_WALL_MS: u32 = 20_000;
    /// RLIMIT_CPU of one worker, a slice's: seconds of CPU. (set by S1)
    pub const SLICE_CPU_SECS: u64 = 60;
    /// A job's CPU in all, counted across slices: this plus ... (set by S1)
    pub const JOB_CPU_BASE_SECS: u64 = 300;
    /// ... this many seconds a page. (set by S1)
    pub const JOB_CPU_PER_PAGE_SECS: u64 = 3;
    /// A page that sends no record for this long is killed (CLOCK_MONOTONIC, which stops in sleep). (set by S1)
    pub const PAGE_WATCHDOG_MS: u32 = 30_000;
    /// Below this MemAvailable, pages render in bands (4 bands: four times the interpretation, a quarter of
    /// the memory). (set by S1)
    pub const BAND_MODE_BELOW_AVAILABLE: u64 = 200 << 20;
    /// How many bands band mode makes of a page. (set by S1)
    pub const BANDS: u32 = 4;
    /// The biggest PDF mapped private and read only. (set by S1)
    pub const PDF_MAX_MAPPED: u64 = 512 << 20;
    /// The biggest PDF when the device refuses the private mapping (the worker reads it whole). (set by S1)
    pub const PDF_MAX_READ_WHOLE: u64 = 64 << 20;
    /// Pixels come down the pipe (false) or through a memfd buffer (true), whichever the spike found cheaper.
    /// (set by S1; only the pipe is built)
    pub const PIXELS_VIA_MEMFD: bool = false;
    /// The worker's image, in address space (bytes): the binary, its stack and the allocator's first arena.
    /// (set by S1)
    pub const WORKER_IMAGE: u64 = 11 << 20;
    /// Address space beyond the image and the file: whole-page mode ... (set by S1)
    pub const AS_MARGIN_WHOLE: u64 = 160 << 20;
    /// ... and band mode. (set by S1)
    pub const AS_MARGIN_BAND: u64 = 96 << 20;
    /// Pictures (lane L7): a worker serves at most this many pictures, then the next one gets a fresh exec (a
    /// 2000-picture folder costs a few dozen execs). (set by S1)
    pub const PICTURE_SLICE: usize = 32;
    /// RLIMIT_CPU of a picture worker, a slice's: seconds of CPU. (set by S1)
    pub const PICTURE_CPU_SECS: u64 = 300;
    /// A picture that sends no record for this long is killed (CLOCK_MONOTONIC, which stops in sleep). (set
    /// by S1)
    pub const PICTURE_WATCHDOG_MS: u32 = 60_000;
    /// A picture worker's address space beyond its image: the decode budget (256 MiB, the file's own bytes
    /// among them), the copy of the decoded picture that is kept (up to 120 MiB), the fitted page and
    /// slack. Address space, not memory. (set by S1)
    pub const PICTURE_AS_MARGIN: u64 = (256 + 128 + 32) << 20;
    /// Fonts (lane L10): a font worker checks glyphs for this long (its own CPU time), then the next slice gets a
    /// fresh exec, resumed by glyph index. (set by S1)
    pub const FONT_SLICE_CPU_SECS: u64 = 20;
    /// A font's check in all, counted across slices: past it the font is "Took too long to read.". (set by S1)
    pub const FONT_JOB_CPU_SECS: u64 = 600;
    /// RLIMIT_CPU of one font worker: a slice, and room for a glyph that never ends to be caught. (set by S1)
    pub const FONT_CPU_SECS: u64 = 60;
    /// A font worker that sends no record for this long is killed (CLOCK_MONOTONIC, which stops in sleep). (set
    /// by S1)
    pub const FONT_WATCHDOG_MS: u32 = 30_000;
    /// A font worker's address space beyond its image and the file's bytes: skrifa's draw memory, the glyph
    /// bitmaps at the largest size, stb's edges. (set by S1)
    pub const FONT_AS_MARGIN: u64 = 64 << 20;
    /// D12's trigger: if the median planner page takes longer than this whole on the Scribe, a PDFium helper is
    /// weighed before this goes on. Nothing in the code reads it. (S1 measured 90 ms: not triggered.)
    pub const D12_TRIGGER_PAGE_MS: u32 = 2_000;
    /// The seccomp allow-list, as the spike's self-test found the calls musl really makes (the time64 calls
    /// musl tries first answer ENOSYS instead: ENOSYS_CALLS). (set by S1)
    pub fn allow() -> Vec<u32> {
        super::allow_list()
    }
}

/// The uid and gid of the worker: a fixed id that no account uses (the writer checks once that /etc/passwd and
/// /etc/group do not list it), not 65534, which other daemons may share and against which kill() and
/// RLIMIT_NPROC would count.
pub const WORKER_ID: u32 = 62001;

/// The record protocol's number: the worker and the writer must agree (the handshake's build line).
pub const WORK_PROTOCOL: u32 = 1;

/// The build the handshake names: the version and the protocol. (A worker is always a fresh exec of the
/// writer's own binary, /proc/self/exe; a different build can only turn up where that path is not used.)
pub fn build_id() -> String {
    format!("{} {}", GRIMOIRE_VERSION, WORK_PROTOCOL)
}

/// What a worker parses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkKind {
    /// a PDF: probe, then pages to gray or RGB pixels
    Pdf,
    /// pictures: templates, photos and picture-folder imports (picwork.rs, lane L7): the bytes come down fd 5
    /// one picture after another, validated pixels go back on fd 4
    Picture,
    /// a font of grimoire/fonts (fontcheck.rs, lane L10): its bytes come down fd 5, a verdict's records and a
    /// preview go back on fd 4
    Font,
}

impl WorkKind {
    pub fn name(self) -> &'static str {
        match self {
            WorkKind::Pdf => "pdf",
            WorkKind::Picture => "pic",
            WorkKind::Font => "font",
        }
    }
    pub fn parse(s: &str) -> Option<WorkKind> {
        match s {
            "pdf" => Some(WorkKind::Pdf),
            "pic" => Some(WorkKind::Picture),
            "font" => Some(WorkKind::Font),
            _ => None,
        }
    }
}

/// The worker's fixed descriptors.
pub const FD_SOURCE: i32 = 3;
pub const FD_OUT: i32 = 4;
pub const FD_IN: i32 = 5;

// ---- the seccomp allow-list (classic BPF over the calls' numbers) ----

pub const SECCOMP_RET_TRAP: u32 = 0x0003_0000;
pub const SECCOMP_RET_ERRNO: u32 = 0x0005_0000;
pub const SECCOMP_RET_ALLOW: u32 = 0x7fff_0000;

/// The system call numbers of one architecture that the filter names (written out, so the filter of every
/// architecture is built and tested on any host; a test on Linux checks the host's table against libc's).
pub struct SysNums {
    pub audit_arch: u32,
    pub read: u32,
    pub write: u32,
    /// mmap2 on the 32-bit ARM, mmap elsewhere
    pub mmap: u32,
    pub munmap: u32,
    pub mremap: u32,
    pub madvise: u32,
    pub mprotect: u32,
    pub brk: u32,
    pub futex: u32,
    pub clock_gettime: u32,
    pub rt_sigreturn: u32,
    pub exit: u32,
    pub exit_group: u32,
    /// calls that answer ENOSYS instead of trapping: the time64 calls musl tries first on the 32-bit ARM, and
    /// gettid, which only a panic's message asks for (S1 saw it trap there), so a worker that panics still
    /// reports "E panic" and ends by its own `_exit`
    pub enosys: &'static [u32],
    /// calls that reach out, which no worker may make (the tests check they trap): socket, execve, kill,
    /// ptrace, mount, chroot
    pub reach_out: [u32; 6],
}

/// ARM EABI (kernel 4.9, the Scribe).
pub const ARM: SysNums = SysNums {
    audit_arch: 0x4000_0028,
    read: 3,
    write: 4,
    mmap: 192,
    munmap: 91,
    mremap: 163,
    madvise: 220,
    mprotect: 125,
    brk: 45,
    futex: 240,
    clock_gettime: 263,
    rt_sigreturn: 173,
    exit: 1,
    exit_group: 248,
    // clock_gettime64, clock_getres_time64, clock_nanosleep_time64, ppoll_time64, futex_time64; gettid
    enosys: &[403, 406, 407, 414, 422, 224],
    reach_out: [281, 11, 37, 26, 21, 61],
};

/// x86-64 (the Linux CI job).
pub const X86_64: SysNums = SysNums {
    audit_arch: 0xc000_003e,
    read: 0,
    write: 1,
    mmap: 9,
    munmap: 11,
    mremap: 25,
    madvise: 28,
    mprotect: 10,
    brk: 12,
    futex: 202,
    clock_gettime: 228,
    rt_sigreturn: 15,
    exit: 60,
    exit_group: 231,
    enosys: &[186], // gettid
    reach_out: [41, 59, 62, 101, 165, 161],
};

/// AArch64.
pub const AARCH64: SysNums = SysNums {
    audit_arch: 0xc000_00b7,
    read: 63,
    write: 64,
    mmap: 222,
    munmap: 215,
    mremap: 216,
    madvise: 233,
    mprotect: 226,
    brk: 214,
    futex: 98,
    clock_gettime: 113,
    rt_sigreturn: 139,
    exit: 93,
    exit_group: 94,
    enosys: &[178], // gettid
    reach_out: [198, 221, 129, 117, 40, 51],
};

/// This build's architecture.
pub fn this_arch() -> &'static SysNums {
    #[cfg(target_arch = "arm")]
    {
        &ARM
    }
    #[cfg(target_arch = "x86_64")]
    {
        &X86_64
    }
    #[cfg(not(any(target_arch = "arm", target_arch = "x86_64")))]
    {
        &AARCH64
    }
}

/// The design's allow-list (3.7 step 9): read, write, the mmap family (musl's allocator also needs mprotect and
/// madvise), brk, futex, clock_gettime, rt_sigreturn and exit. exit_group is on it too: musl's `_exit` ends the
/// process through it, and it only ever ends the worker itself.
pub fn allow_list_for(n: &SysNums) -> Vec<u32> {
    vec![n.read, n.write, n.mmap, n.munmap, n.mremap, n.madvise, n.mprotect, n.brk, n.futex, n.clock_gettime, n.rt_sigreturn, n.exit, n.exit_group]
}
pub fn allow_list() -> Vec<u32> {
    allow_list_for(this_arch())
}

/// "The time64 calls musl tries first answer ENOSYS" (the 32-bit ARM only).
pub fn enosys_calls() -> Vec<u32> {
    this_arch().enosys.to_vec()
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BpfIns {
    pub code: u16,
    pub jt: u8,
    pub jf: u8,
    pub k: u32,
}

const BPF_LD_W_ABS: u16 = 0x20;
const BPF_JEQ_K: u16 = 0x15;
const BPF_RET_K: u16 = 0x06;

/// An arch check (another arch: trap), then one jump per allowed number to ALLOW, one per ENOSYS number to
/// ERRNO(ENOSYS), and everything else TRAPs.
pub fn build_filter(arch: u32, allow: &[u32], enosys: &[u32]) -> Vec<BpfIns> {
    let n = allow.len() + enosys.len();
    assert!(n < 240, "a jump is one byte");
    let mut v = vec![
        BpfIns { code: BPF_LD_W_ABS, jt: 0, jf: 0, k: 4 }, // seccomp_data.arch
        BpfIns { code: BPF_JEQ_K, jt: 1, jf: 0, k: arch },
        BpfIns { code: BPF_RET_K, jt: 0, jf: 0, k: SECCOMP_RET_TRAP },
        BpfIns { code: BPF_LD_W_ABS, jt: 0, jf: 0, k: 0 }, // seccomp_data.nr
    ];
    let first = v.len();
    let trap_at = first + n;
    let allow_at = trap_at + 1;
    let errno_at = trap_at + 2;
    for (i, &s) in allow.iter().enumerate() {
        let here = first + i;
        v.push(BpfIns { code: BPF_JEQ_K, jt: (allow_at - (here + 1)) as u8, jf: 0, k: s });
    }
    for (i, &s) in enosys.iter().enumerate() {
        let here = first + allow.len() + i;
        v.push(BpfIns { code: BPF_JEQ_K, jt: (errno_at - (here + 1)) as u8, jf: 0, k: s });
    }
    v.push(BpfIns { code: BPF_RET_K, jt: 0, jf: 0, k: SECCOMP_RET_TRAP });
    v.push(BpfIns { code: BPF_RET_K, jt: 0, jf: 0, k: SECCOMP_RET_ALLOW });
    v.push(BpfIns { code: BPF_RET_K, jt: 0, jf: 0, k: SECCOMP_RET_ERRNO | 38 }); // ENOSYS
    v
}

/// A tiny classic-BPF interpreter for the three instruction kinds above, so the filter's verdicts are tested
/// on any host.
pub fn eval_filter(f: &[BpfIns], arch: u32, nr: u32) -> u32 {
    let (mut pc, mut a) = (0usize, 0u32);
    loop {
        let i = f[pc];
        pc += 1;
        match i.code {
            BPF_LD_W_ABS => a = if i.k == 4 { arch } else { nr },
            BPF_JEQ_K => pc += if a == i.k { i.jt as usize } else { i.jf as usize },
            BPF_RET_K => return i.k,
            c => panic!("bad BPF op {c}"),
        }
    }
}

// ---- the steps, and the fail-closed rule ----

/// One sandbox step: its name (as `E sandbox <name>` and the log say it) and what it does.
pub struct Step<'a> {
    pub name: &'static str,
    pub run: Box<dyn FnMut() -> Result<String, i32> + 'a>,
}

/// Whether a failed step refuses the work: always on the Kindle. Elsewhere (the simulator, CI, the Mac) only
/// when GRIMOIRE_SANDBOX=require asks for it (the Linux CI job as root does), and never otherwise: those
/// builds run with plain process isolation.
pub fn sandbox_required() -> bool {
    #[cfg(feature = "kindle")]
    let ask: Option<String> = None; // (no environment variable can loosen the Kindle build)
    #[cfg(not(feature = "kindle"))]
    let ask = std::env::var("GRIMOIRE_SANDBOX").ok();
    required_for(cfg!(feature = "kindle"), ask.as_deref())
}

/// The rule itself: the Kindle always, any other build only when asked.
pub fn required_for(kindle: bool, ask: Option<&str>) -> bool {
    kindle || ask == Some("require")
}

/// Runs the steps in order. A step that fails stops the run with its name when `required` (the worker then
/// parses nothing: no byte of the source has been read yet); when not required it is noted and the next one
/// still runs. `note(name, ok, detail)` hears every step.
pub fn apply_steps(steps: &mut [Step], required: bool, mut note: impl FnMut(&str, bool, &str)) -> Result<(), &'static str> {
    for s in steps.iter_mut() {
        match (s.run)() {
            Ok(d) => note(s.name, true, &d),
            Err(e) => {
                note(s.name, false, &format!("errno {e}"));
                if required {
                    return Err(s.name);
                }
            }
        }
    }
    Ok(())
}

/// The limits one worker takes (the writer decides them, 4.3).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Limits {
    pub as_bytes: u64,
    pub cpu_secs: u64,
    /// the PDF is mapped (private, read only); false: read whole through fd 3
    pub mapped: bool,
    /// the PDF's length (the worker can no longer fstat once the sandbox is on)
    pub file_len: u64,
}

/// The worker's address space for a file of `file_len` bytes (4.3): its image, the file, and the margin.
pub fn worker_as_bytes(file_len: u64, band_mode: bool) -> u64 {
    s1::WORKER_IMAGE + file_len + if band_mode { s1::AS_MARGIN_BAND } else { s1::AS_MARGIN_WHOLE }
}

/// The worker's own command line, after "grimoire --work": `<kind> <writer pid> <as bytes> <cpu secs> <map|read>
/// <file length> <chroot dir or ->`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkArgs {
    pub kind: WorkKind,
    pub writer_pid: i32,
    pub limits: Limits,
    pub jail: Option<String>,
    /// a step to fail on purpose (headless and test builds only)
    pub fail_step: Option<String>,
}

pub fn parse_work_args(args: &[String]) -> Option<WorkArgs> {
    // args: grimoire --work kind pid as cpu mode file_len jail
    let i = args.iter().position(|a| a == "--work")?;
    let r = &args[i + 1..];
    // (an eighth word, fail=<step>, is the headless and test builds' injected failure: GRIMOIRE_WORK_FAIL_STEP)
    if r.len() != 7 && !(r.len() == 8 && r[7].starts_with("fail=")) {
        return None;
    }
    Some(WorkArgs {
        kind: WorkKind::parse(&r[0])?,
        writer_pid: r[1].parse().ok()?,
        limits: Limits {
            as_bytes: r[2].parse().ok()?,
            cpu_secs: r[3].parse().ok()?,
            mapped: match r[4].as_str() {
                "map" => true,
                "read" => false,
                _ => return None,
            },
            file_len: r[5].parse().ok()?,
        },
        jail: (r[6] != "-").then(|| r[6].clone()),
        fail_step: r.get(7).and_then(|w| w.strip_prefix("fail=")).map(str::to_string),
    })
}

// ---- the steps on Linux ----

#[cfg(target_os = "linux")]
mod linux {
    use super::*;

    pub fn errno() -> i32 {
        std::io::Error::last_os_error().raw_os_error().unwrap_or(0)
    }

    pub fn oom_score_adj(val: i32) -> Result<String, i32> {
        std::fs::write("/proc/self/oom_score_adj", val.to_string()).map_err(|e| e.raw_os_error().unwrap_or(libc::EIO))?;
        Ok(format!("oom_score_adj {val}"))
    }

    pub fn unshare_net_ipc() -> Result<String, i32> {
        // SAFETY: unshare takes flags only.
        if unsafe { libc::unshare(libc::CLONE_NEWNET | libc::CLONE_NEWIPC) } != 0 {
            return Err(errno());
        }
        Ok("no network, no System V IPC".to_string())
    }

    pub fn chroot_to(dir: &str) -> Result<String, i32> {
        let c = CString::new(dir).map_err(|_| libc::EINVAL)?;
        // SAFETY: chroot and chdir read NUL-terminated paths we own.
        unsafe {
            if libc::chroot(c.as_ptr()) != 0 {
                return Err(errno());
            }
            if libc::chdir(c"/".as_ptr()) != 0 {
                return Err(errno());
            }
        }
        // inside: nothing of the old tree may be visible
        if std::fs::metadata("/etc/passwd").is_ok() || std::fs::metadata("/proc/self").is_ok() {
            return Err(libc::EACCES);
        }
        Ok("chroot to an empty directory".to_string())
    }

    pub fn drop_ids(id: u32) -> Result<String, i32> {
        // SAFETY: plain credential calls; setuid(0) must then fail.
        unsafe {
            if libc::setgroups(0, std::ptr::null()) != 0 || libc::setgid(id) != 0 || libc::setuid(id) != 0 {
                return Err(errno());
            }
            if libc::setuid(0) == 0 {
                return Err(libc::EPERM);
            }
        }
        Ok(format!("uid and gid {id}"))
    }

    pub fn no_new_privs() -> Result<String, i32> {
        // SAFETY: prctl with plain integer arguments.
        unsafe {
            if libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) != 0 || libc::prctl(libc::PR_SET_DUMPABLE, 0, 0, 0, 0) != 0 {
                return Err(errno());
            }
        }
        Ok("no new privileges, not dumpable".to_string())
    }

    pub fn pdeathsig(writer: i32) -> Result<String, i32> {
        // SAFETY: prctl with plain integer arguments; getppid has no preconditions.
        unsafe {
            if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGKILL as libc::c_ulong, 0, 0, 0) != 0 {
                return Err(errno());
            }
            if libc::getppid() != writer {
                return Err(libc::ESRCH);
            }
        }
        Ok("dies with the writer".to_string())
    }

    // (a macro: the resource constants' type differs between musl and glibc)
    macro_rules! set_limit {
        ($res:expr, $soft:expr, $hard:expr) => {{
            let l = libc::rlimit { rlim_cur: $soft as libc::rlim_t, rlim_max: $hard as libc::rlim_t };
            // SAFETY: setrlimit reads the struct we own.
            if unsafe { libc::setrlimit($res, &l) } != 0 {
                return Err(errno());
            }
        }};
    }

    /// NOFILE 0 (the descriptors it holds keep working; no new one can be made, whatever it closes), NPROC 0,
    /// FSIZE 0, CORE 0, AS and CPU; then proves NOFILE: no new descriptor of any kind.
    pub fn rlimits(l: &Limits) -> Result<String, i32> {
        // SAFETY: signal() with SIG_IGN, so a write past FSIZE 0 is an error and not a signal.
        unsafe { libc::signal(libc::SIGXFSZ, libc::SIG_IGN) };
        set_limit!(libc::RLIMIT_NOFILE, 0, 0);
        set_limit!(libc::RLIMIT_NPROC, 0, 0);
        set_limit!(libc::RLIMIT_FSIZE, 0, 0);
        set_limit!(libc::RLIMIT_CORE, 0, 0);
        set_limit!(libc::RLIMIT_AS, l.as_bytes, l.as_bytes);
        set_limit!(libc::RLIMIT_CPU, l.cpu_secs, l.cpu_secs + 5);
        // SAFETY: dup, socket and open only take plain arguments; none may succeed.
        let (d, s, o) = unsafe {
            (libc::dup(FD_OUT), libc::socket(libc::AF_UNIX, libc::SOCK_STREAM, 0), libc::open(c"/".as_ptr(), libc::O_RDONLY))
        };
        if d >= 0 || s >= 0 || o >= 0 {
            return Err(libc::EMFILE ^ 0x4000);
        }
        Ok(format!("NOFILE 0, NPROC 0, FSIZE 0, CORE 0, AS {} MiB, CPU {} s", l.as_bytes >> 20, l.cpu_secs))
    }

    // The SIGSYS reporter: a call outside the allow-list ends the worker, telling the writer which call (one
    // line on fd 4: write is allowed), by hand-made formatting (no allocation in a signal handler).
    extern "C" fn on_sigsys(_sig: libc::c_int, info: *mut libc::siginfo_t, _ctx: *mut libc::c_void) {
        // SAFETY: the kernel hands a siginfo whose si_syscall is the word after si_call_addr.
        let n = unsafe { *((info as *const u8).add(if cfg!(target_pointer_width = "64") { 24 } else { 16 }) as *const i32) } as u32;
        let mut b = [0u8; 40];
        let head = b"E sigsys ";
        b[..head.len()].copy_from_slice(head);
        let mut p = head.len();
        let mut t = [0u8; 10];
        let (mut k, mut v) = (0, n);
        loop {
            t[k] = b'0' + (v % 10) as u8;
            k += 1;
            v /= 10;
            if v == 0 {
                break;
            }
        }
        while k > 0 {
            k -= 1;
            b[p] = t[k];
            p += 1;
        }
        b[p] = b'\n';
        p += 1;
        // SAFETY: write and _exit are async-signal-safe and on the allow-list.
        unsafe {
            libc::write(FD_OUT, b.as_ptr() as *const libc::c_void, p);
            libc::_exit(5);
        }
    }

    pub fn seccomp() -> Result<String, i32> {
        // SAFETY: the sigaction struct is plain data we fill in; the handler is async-signal-safe.
        unsafe {
            let mut sa: libc::sigaction = std::mem::zeroed();
            sa.sa_sigaction = on_sigsys as *const () as usize;
            sa.sa_flags = libc::SA_SIGINFO | libc::SA_NODEFER;
            libc::sigemptyset(&mut sa.sa_mask);
            if libc::sigaction(libc::SIGSYS, &sa, std::ptr::null_mut()) != 0 {
                return Err(errno());
            }
            let mut set: libc::sigset_t = std::mem::zeroed();
            libc::sigemptyset(&mut set);
            libc::sigaddset(&mut set, libc::SIGSYS);
            libc::sigprocmask(libc::SIG_UNBLOCK, &set, std::ptr::null_mut());
        }
        let f = build_filter(this_arch().audit_arch, &s1::allow(), &enosys_calls());
        let mut raw: Vec<libc::sock_filter> = f.iter().map(|i| libc::sock_filter { code: i.code, jt: i.jt, jf: i.jf, k: i.k }).collect();
        let prog = libc::sock_fprog { len: raw.len() as u16, filter: raw.as_mut_ptr() };
        // SAFETY: the kernel copies the program at this call; prog and raw live until it returns.
        let r = unsafe { libc::prctl(libc::PR_SET_SECCOMP, libc::SECCOMP_MODE_FILTER as libc::c_ulong, &prog as *const libc::sock_fprog as libc::c_ulong, 0, 0) };
        if r != 0 {
            return Err(errno());
        }
        Ok(format!("seccomp allow-list, {} instructions", raw.len()))
    }

    /// Whether /etc/passwd and /etc/group list no entry with this id (the writer checks it once).
    pub fn id_unused(id: u32) -> bool {
        ["/etc/passwd", "/etc/group"].iter().all(|f| {
            std::fs::read_to_string(f).unwrap_or_default().lines().all(|l| {
                let p: Vec<&str> = l.split(':').collect();
                p.len() < 3 || p[2].parse::<u32>().ok() != Some(id)
            })
        })
    }

    /// The writer's empty, read-only chroot directory, on /tmp (checked to be tmpfs: never storage the user
    /// can reach).
    pub fn make_jail_dir() -> Result<String, String> {
        use std::os::unix::fs::PermissionsExt;
        let p = format!("/tmp/grimoire-work-{}", std::process::id());
        let _ = std::fs::remove_dir(&p);
        std::fs::create_dir(&p).map_err(|e| format!("mkdir: {e}"))?;
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o555)).map_err(|e| format!("chmod: {e}"))?;
        let c = CString::new(p.clone()).map_err(|_| "path".to_string())?;
        // SAFETY: s is written by statfs before it is read; c is NUL-terminated.
        let mut s: libc::statfs = unsafe { std::mem::zeroed() };
        if unsafe { libc::statfs(c.as_ptr(), &mut s) } != 0 || s.f_type as u64 != 0x0102_1994 {
            let _ = std::fs::remove_dir(&p);
            return Err("/tmp is not tmpfs".to_string());
        }
        Ok(p)
    }

    pub fn selinux() -> &'static str {
        match std::fs::read_to_string("/sys/fs/selinux/enforce").ok().as_deref().map(str::trim) {
            Some("1") => "enforcing",
            Some("0") => "permissive",
            _ => "off",
        }
    }
}

/// The worker's steps for this build, in the design's order (3.7, 2 to 9). Off Linux there are none (plain
/// process isolation).
#[cfg(target_os = "linux")]
pub fn standard_steps<'a>(a: &'a WorkArgs) -> Vec<Step<'a>> {
    use linux::*;
    let mut v: Vec<Step> = vec![Step { name: "oom", run: Box::new(|| oom_score_adj(1000)) }];
    v.push(Step { name: "namespaces", run: Box::new(unshare_net_ipc) });
    if let Some(j) = a.jail.as_deref() {
        v.push(Step { name: "chroot", run: Box::new(move || chroot_to(j)) });
    }
    v.push(Step { name: "uid", run: Box::new(|| drop_ids(WORKER_ID)) });
    v.push(Step { name: "privileges", run: Box::new(no_new_privs) });
    let writer = a.writer_pid;
    v.push(Step { name: "pdeathsig", run: Box::new(move || pdeathsig(writer)) });
    let l = a.limits.clone();
    v.push(Step { name: "rlimits", run: Box::new(move || rlimits(&l)) });
    v.push(Step { name: "seccomp", run: Box::new(seccomp) });
    v
}
#[cfg(not(target_os = "linux"))]
pub fn standard_steps<'a>(_a: &'a WorkArgs) -> Vec<Step<'a>> {
    Vec::new()
}

/// The CPU and core limits the OS honours off Linux too (macOS ignores RLIMIT_AS).
#[cfg(not(target_os = "linux"))]
fn soft_limits(l: &Limits) {
    let lim = libc::rlimit { rlim_cur: l.cpu_secs as libc::rlim_t, rlim_max: (l.cpu_secs + 5) as libc::rlim_t };
    let core = libc::rlimit { rlim_cur: 0, rlim_max: 0 };
    // SAFETY: setrlimit reads structs we own; a failure is fine off the Kindle.
    unsafe {
        libc::setrlimit(libc::RLIMIT_CPU, &lim);
        libc::setrlimit(libc::RLIMIT_CORE, &core);
    }
}

/// A step that fails on purpose (the headless and test builds only: the writer reads GRIMOIRE_WORK_FAIL_STEP=<name>
/// and tells the worker through its command line, whose environment is empty), so a scenario sees what a refusing
/// sandbox does without a Kindle.
fn injected_failure(a: &WorkArgs) -> Option<String> {
    #[cfg(not(feature = "kindle"))]
    {
        a.fail_step.clone().filter(|s| !s.is_empty())
    }
    #[cfg(feature = "kindle")]
    {
        let _ = a;
        None
    }
}

/// What the writer adds to the worker's command line (never on the Kindle).
fn injected_arg() -> Option<String> {
    #[cfg(not(feature = "kindle"))]
    {
        std::env::var("GRIMOIRE_WORK_FAIL_STEP").ok().filter(|s| !s.is_empty()).map(|s| format!("fail={s}"))
    }
    #[cfg(feature = "kindle")]
    {
        None
    }
}

fn out_line(s: &str) {
    // SAFETY: write to the worker's pipe descriptor from a buffer we own; a failure is ignored (the writer
    // sees the worker end).
    unsafe { libc::write(FD_OUT, s.as_ptr() as *const libc::c_void, s.len()) };
}

/// The `--work` entry, called first thing in main() before any UI start-up (no display, input, settings or log
/// file). Steps 1 to 10 of 3.7. Once the steps have run it ends the process itself with `_exit` and the
/// worker's exit status, never returning: the runtime's own exit (the main thread's signal stack let go with
/// `sigaltstack`, atexit handlers) would make calls the filter does not allow, and a clean end would read as
/// `E sigsys 186` on the Kindle. Before the steps (arguments that do not parse) it returns 2.
pub fn work_main(args: &[String]) -> i32 {
    let Some(a) = parse_work_args(args) else {
        return 2;
    };
    // 1. warm up what would touch the filesystem or entropy later (std's feature detection, one RandomState)
    let _ = std::collections::hash_map::RandomState::new();
    let _ = std::time::Instant::now();
    crate::pdfwork::warm_up();
    if a.kind == WorkKind::Font {
        crate::fontcheck::warm_up();
    }
    out_line(&format!("B {}\n", build_id()));
    let fail = injected_failure(&a);
    let required = sandbox_required() || fail.is_some();
    let mut steps = standard_steps(&a);
    if let Some(name) = fail.as_deref() {
        // an injected failure takes the place of the named step (or comes first in a build with none)
        let known = ["oom", "namespaces", "chroot", "uid", "privileges", "pdeathsig", "rlimits", "seccomp"];
        let label: &'static str = known.iter().copied().find(|k| *k == name).unwrap_or("injected");
        let step = Step { name: label, run: Box::new(|| Err(libc::EPERM)) };
        match steps.iter().position(|s| s.name == label) {
            Some(i) => steps[i] = step,
            None => steps.insert(0, step),
        }
    }
    let r = apply_steps(&mut steps, required, |_name, _ok, _detail| {});
    #[cfg(not(target_os = "linux"))]
    soft_limits(&a.limits);
    if let Err(step) = r {
        out_line(&format!("E sandbox {step}\n"));
        // SAFETY: _exit ends the process at once, with no call the steps might refuse.
        unsafe { libc::_exit(3) };
    }
    let sandboxed = !steps.is_empty();
    out_line(&format!("S {}\n", if sandboxed { 1 } else { 0 }));
    // 10. parses on one thread, writes only records and pixels to fd 4
    let rc = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match a.kind {
        WorkKind::Pdf => crate::pdfwork::worker_run(&a.limits),
        WorkKind::Picture => crate::picwork::worker_run(&a.limits),
        WorkKind::Font => crate::fontcheck::worker_run(&a.limits),
    }))
    .unwrap_or_else(|_| {
        out_line("E panic\n");
        4
    });
    // SAFETY: _exit ends the process at once: records and pixels went out with write(2), nothing is buffered.
    unsafe { libc::_exit(rc) }
}

// ---- the writer's side: spawning a worker ----

/// A worker the writer started.
pub struct Worker {
    pub pid: i32,
    /// the pipe from the worker (its fd 4), and to it (its fd 5)
    rx: i32,
    tx: i32,
    jail: Option<String>,
    reaped: bool,
    /// bytes read from rx ahead of the line read_line asked for (read_exact takes them first)
    rbuf: Vec<u8>,
}

/// Why no worker could be started: not the file's fault.
#[derive(Debug, PartialEq, Eq)]
pub enum SpawnErr {
    /// a sandbox precondition failed (the step's name): the unit fails, marks nothing, counts no try
    Sandbox(&'static str),
    /// fork or exec failed
    Start,
}

static BROKEN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// A worker in this session reported a failed sandbox step (`E sandbox`): no more PDF or font work is queued
/// until the next start. Pictures then wait where the sandbox is required (the Kindle), and elsewhere fall
/// back to 2.0's path (picwork.rs).
pub fn session_broken() -> bool {
    BROKEN.load(std::sync::atomic::Ordering::Relaxed)
}
pub fn session_set_broken() {
    BROKEN.store(true, std::sync::atomic::Ordering::Relaxed);
}

/// The binary a worker is: this very one (/proc/self/exe on the Kindle, never a path another file could
/// take over), elsewhere the running executable, or GRIMOIRE_WORK_EXE (the tests', whose own executable is a
/// test harness, not Grimoire).
fn work_exe() -> String {
    #[cfg(feature = "kindle")]
    {
        "/proc/self/exe".to_string()
    }
    #[cfg(not(feature = "kindle"))]
    {
        if let Some(p) = std::env::var_os("GRIMOIRE_WORK_EXE") {
            return p.to_string_lossy().into_owned();
        }
        if std::fs::metadata("/proc/self/exe").is_ok() {
            return "/proc/self/exe".to_string();
        }
        std::env::current_exe().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default()
    }
}

/// In the child between fork and exec: a close-on-exec copy of `fd` at 10 or above, so the numbers 0 to 5 are
/// free to be taken. A copy that cannot be made ends the child (status 127): no worker is exec'd.
///
/// # Safety
/// Only in a forked child before its exec: it ends the process on a failure.
unsafe fn park_or_exit(fd: i32) -> i32 {
    let r = libc::fcntl(fd, libc::F_DUPFD_CLOEXEC, 10);
    if r < 0 {
        libc::_exit(127);
    }
    r
}

/// In the child between fork and exec: `from` on the number `to` (dup2 leaves the copy without the close-on-exec
/// flag). Tried until it returns its target, at most 8 times (an interrupted call; errno is not read here), then
/// the child ends (status 127): no worker is exec'd.
///
/// # Safety
/// Only in a forked child before its exec: it ends the process on a failure.
unsafe fn dup2_or_exit(from: i32, to: i32) {
    for _ in 0..8 {
        if libc::dup2(from, to) == to {
            return;
        }
    }
    libc::_exit(127);
}

impl Worker {
    /// Starts a worker: a fresh exec of this binary with descriptors 0 to 5 only, each put in place by a checked
    /// step whatever descriptors the writer itself holds; when one cannot be, no worker is exec'd (the writer
    /// reads the end of the pipe at the handshake). `source` is the PDF (read only), None for kinds that take
    /// none.
    pub fn spawn(kind: WorkKind, source: Option<i32>, limits: &Limits) -> Result<Worker, SpawnErr> {
        #[cfg(target_os = "linux")]
        let jail = if sandbox_required() {
            if !linux::id_unused(WORKER_ID) {
                qlog!("work: uid %d is listed in /etc/passwd or /etc/group; no worker", WORKER_ID as i32);
                return Err(SpawnErr::Sandbox("uid"));
            }
            match linux::make_jail_dir() {
                Ok(d) => Some(d),
                Err(e) => {
                    qlog!("work: no chroot directory (%s)", &e);
                    return Err(SpawnErr::Sandbox("chroot"));
                }
            }
        } else {
            None
        };
        #[cfg(not(target_os = "linux"))]
        let jail: Option<String> = None;
        let exe = work_exe();
        let mut argv_s = vec![
            "grimoire".to_string(),
            "--work".to_string(),
            kind.name().to_string(),
            // SAFETY: getpid has no preconditions.
            unsafe { libc::getpid() }.to_string(),
            limits.as_bytes.to_string(),
            limits.cpu_secs.to_string(),
            if limits.mapped { "map" } else { "read" }.to_string(),
            limits.file_len.to_string(),
            jail.clone().unwrap_or_else(|| "-".to_string()),
        ];
        argv_s.extend(injected_arg());
        // (the command line first: when it cannot be built nothing is open yet)
        let (Ok(cexe), Ok(cargs)) = (CString::new(exe), argv_s.iter().map(|s| CString::new(s.as_str())).collect::<Result<Vec<_>, _>>()) else {
            return Err(SpawnErr::Start);
        };
        let mut argv: Vec<*const libc::c_char> = cargs.iter().map(|c| c.as_ptr()).collect();
        argv.push(std::ptr::null());
        let envp: [*const libc::c_char; 1] = [std::ptr::null()];
        // /dev/null for the worker's 0 to 2, opened here, before the pipes: the child opens nothing.
        // SAFETY: open reads a NUL-terminated literal.
        let dn = unsafe { libc::open(c"/dev/null".as_ptr(), libc::O_RDWR | libc::O_CLOEXEC) };
        if dn < 0 {
            return Err(SpawnErr::Start);
        }
        let mut p4 = [-1i32; 2];
        let mut p5 = [-1i32; 2];
        // SAFETY: pipe2 fills the two-int arrays we own.
        #[cfg(target_os = "linux")]
        let ok = unsafe { libc::pipe2(p4.as_mut_ptr(), libc::O_CLOEXEC) == 0 && libc::pipe2(p5.as_mut_ptr(), libc::O_CLOEXEC) == 0 };
        #[cfg(not(target_os = "linux"))]
        let ok = unsafe {
            let r = libc::pipe(p4.as_mut_ptr()) == 0 && libc::pipe(p5.as_mut_ptr()) == 0;
            if r {
                for fd in [p4[0], p4[1], p5[0], p5[1]] {
                    libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC);
                }
            }
            r
        };
        // what this function opened, closed again when the start goes no further (an end never made is -1)
        let close_all = |fds: &[i32]| {
            for &fd in fds {
                if fd >= 0 {
                    // SAFETY: closing descriptors this function opened.
                    unsafe { libc::close(fd) };
                }
            }
        };
        if !ok {
            close_all(&[dn, p4[0], p4[1], p5[0], p5[1]]);
            return Err(SpawnErr::Start);
        }
        // SAFETY: fork, as PORTING.md's process model has it. The child only rearranges descriptors and execs
        // (or _exits); every pointer it uses was made before the fork.
        let pid = unsafe { libc::fork() };
        if pid < 0 {
            close_all(&[dn, p4[0], p4[1], p5[0], p5[1]]);
            return Err(SpawnErr::Start);
        }
        if pid == 0 {
            // SAFETY: in the child after fork: only async-signal-safe calls, on descriptors this function opened or
            // was given. Every step is checked: a worker is exec'd with all six descriptors in place or not at all.
            unsafe {
                // park what must survive above the numbers it will take (close-on-exec: no copy crosses the exec)
                let dn = park_or_exit(dn);
                let src = source.map(|s| park_or_exit(s));
                let w4 = park_or_exit(p4[1]);
                let r5 = park_or_exit(p5[0]);
                for fd in 0..3 {
                    dup2_or_exit(dn, fd);
                }
                match src {
                    Some(s) => dup2_or_exit(s, FD_SOURCE),
                    None => {
                        libc::close(FD_SOURCE);
                    }
                }
                dup2_or_exit(w4, FD_OUT);
                dup2_or_exit(r5, FD_IN);
                let mut max = libc::sysconf(libc::_SC_OPEN_MAX);
                if !(0..=4096).contains(&max) {
                    max = 4096;
                }
                for fd in 6..max as i32 {
                    libc::close(fd);
                }
                libc::execve(cexe.as_ptr(), argv.as_ptr(), envp.as_ptr());
                libc::_exit(127);
            }
        }
        // the child's ends, and /dev/null, in the parent
        close_all(&[dn, p4[1], p5[0]]);
        Ok(Worker { pid, rx: p4[0], tx: p5[1], jail, reaped: false, rbuf: Vec::new() })
    }

    /// Sends one command line to the worker.
    pub fn send(&mut self, line: &str) -> bool {
        let b = line.as_bytes();
        let mut off = 0;
        while off < b.len() {
            // SAFETY: write from a slice we own to the pipe's descriptor.
            let n = unsafe { libc::write(self.tx, b[off..].as_ptr() as *const libc::c_void, b.len() - off) };
            if n < 0 && std::io::Error::last_os_error().kind() == std::io::ErrorKind::Interrupted {
                continue;
            }
            if n <= 0 {
                return false;
            }
            off += n as usize;
        }
        true
    }

    /// Sends `b` (a picture's bytes, up to 64 MB) before `deadline_ms`. A worker that does not read (hung, or
    /// gone) can never block the writer: each write follows a poll for room and is at most PIPE_BUF, which
    /// then cannot block.
    pub fn send_bytes(&mut self, b: &[u8], deadline_ms: u64) -> bool {
        #[cfg(target_os = "linux")]
        const CHUNK: usize = 4096;
        #[cfg(not(target_os = "linux"))]
        const CHUNK: usize = 512;
        let mut off = 0;
        while off < b.len() {
            let left = deadline_ms.saturating_sub(mono_ms());
            if left == 0 {
                return false;
            }
            let mut pfd = libc::pollfd { fd: self.tx, events: libc::POLLOUT, revents: 0 };
            // SAFETY: poll reads and writes the one pollfd we own.
            let r = unsafe { libc::poll(&mut pfd, 1, left.min(1000) as libc::c_int) };
            if r < 0 {
                if std::io::Error::last_os_error().kind() == std::io::ErrorKind::Interrupted {
                    continue;
                }
                return false;
            }
            if r == 0 {
                continue;
            }
            if pfd.revents & (libc::POLLERR | libc::POLLHUP | libc::POLLNVAL) != 0 {
                return false;
            }
            let n = (b.len() - off).min(CHUNK);
            // SAFETY: write from a slice we own to the pipe's descriptor.
            let w = unsafe { libc::write(self.tx, b[off..].as_ptr() as *const libc::c_void, n) };
            if w < 0 && std::io::Error::last_os_error().kind() == std::io::ErrorKind::Interrupted {
                continue;
            }
            if w <= 0 {
                return false;
            }
            off += w as usize;
        }
        true
    }

    /// Reads exactly `buf.len()` bytes before `deadline_ms` (CLOCK_MONOTONIC milliseconds): Ok, or why not. What
    /// read_line read ahead comes first.
    pub fn read_exact(&mut self, buf: &mut [u8], deadline_ms: u64) -> Result<(), ReadErr> {
        let mut got = self.rbuf.len().min(buf.len());
        buf[..got].copy_from_slice(&self.rbuf[..got]);
        self.rbuf.drain(..got);
        while got < buf.len() {
            let left = deadline_ms.saturating_sub(mono_ms());
            if left == 0 {
                return Err(ReadErr::Timeout);
            }
            let mut pfd = libc::pollfd { fd: self.rx, events: libc::POLLIN, revents: 0 };
            // SAFETY: poll reads and writes the one pollfd we own.
            let r = unsafe { libc::poll(&mut pfd, 1, left.min(1000) as libc::c_int) };
            if r < 0 {
                if std::io::Error::last_os_error().kind() == std::io::ErrorKind::Interrupted {
                    continue;
                }
                return Err(ReadErr::End);
            }
            if r == 0 {
                continue;
            }
            // SAFETY: read into the part of buf not yet filled.
            let n = unsafe { libc::read(self.rx, buf[got..].as_mut_ptr() as *mut libc::c_void, buf.len() - got) };
            if n < 0 && std::io::Error::last_os_error().kind() == std::io::ErrorKind::Interrupted {
                continue;
            }
            if n <= 0 {
                return Err(ReadErr::End);
            }
            got += n as usize;
        }
        Ok(())
    }

    /// One text line (without its newline), at most `max` bytes, before the deadline. The pipe is read in
    /// blocks (a 3 MiB link stream is a few hundred reads, not millions); what comes after the line (the pixels
    /// that follow a band's header) stays in the buffer for read_exact.
    pub fn read_line(&mut self, max: usize, deadline_ms: u64) -> Result<Vec<u8>, ReadErr> {
        let mut scanned = 0;
        loop {
            if let Some(i) = self.rbuf[scanned..].iter().position(|&c| c == b'\n') {
                let at = scanned + i;
                if at > max {
                    return Err(ReadErr::TooLong);
                }
                let line = self.rbuf[..at].to_vec();
                self.rbuf.drain(..=at);
                return Ok(line);
            }
            scanned = self.rbuf.len();
            if scanned > max {
                return Err(ReadErr::TooLong);
            }
            let left = deadline_ms.saturating_sub(mono_ms());
            if left == 0 {
                return Err(ReadErr::Timeout);
            }
            let mut pfd = libc::pollfd { fd: self.rx, events: libc::POLLIN, revents: 0 };
            // SAFETY: poll reads and writes the one pollfd we own.
            let r = unsafe { libc::poll(&mut pfd, 1, left.min(1000) as libc::c_int) };
            if r < 0 {
                if std::io::Error::last_os_error().kind() == std::io::ErrorKind::Interrupted {
                    continue;
                }
                return Err(ReadErr::End);
            }
            if r == 0 {
                continue;
            }
            let mut block = [0u8; 4096];
            // SAFETY: read into a buffer we own, at most its length.
            let n = unsafe { libc::read(self.rx, block.as_mut_ptr() as *mut libc::c_void, block.len()) };
            if n < 0 && std::io::Error::last_os_error().kind() == std::io::ErrorKind::Interrupted {
                continue;
            }
            if n <= 0 {
                return Err(ReadErr::End);
            }
            self.rbuf.extend_from_slice(&block[..n as usize]);
        }
    }

    /// Kills the worker (if it still runs) and reaps it; its wait status, once. The wait is bounded: a worker the
    /// kernel cannot end at once (stuck in a read of the storage) is left to it after 5 s, logged, rather than
    /// holding the writer for good.
    pub fn kill_and_reap(&mut self) -> i32 {
        if self.reaped {
            return 0;
        }
        let mut st = 0;
        // SAFETY: kill on the child this struct forked and has not reaped.
        unsafe { libc::kill(self.pid, libc::SIGKILL) };
        let end = mono_ms() + 5000;
        loop {
            // SAFETY: waitpid on the child this struct forked and has not reaped; WNOHANG.
            let r = unsafe { libc::waitpid(self.pid, &mut st, libc::WNOHANG) };
            if r == self.pid || r < 0 {
                break;
            }
            if mono_ms() >= end {
                qlog!("work: worker %d did not end within 5 s of SIGKILL; left to the kernel", self.pid);
                st = libc::SIGKILL; // (as waitpid would say of a process ended by SIGKILL)
                break;
            }
            // SAFETY: usleep has no preconditions.
            unsafe { libc::usleep(1000) };
        }
        self.reaped = true;
        self.cleanup();
        st
    }

    /// Waits for a worker that was asked to end (`Q`), killing it if it does not within `ms`.
    pub fn finish(&mut self, ms: u64) -> i32 {
        if self.reaped {
            return 0;
        }
        let end = mono_ms() + ms;
        loop {
            let mut st = 0;
            // SAFETY: waitpid on the child this struct forked; WNOHANG.
            let r = unsafe { libc::waitpid(self.pid, &mut st, libc::WNOHANG) };
            if r == self.pid {
                self.reaped = true;
                self.cleanup();
                return st;
            }
            if mono_ms() >= end {
                return self.kill_and_reap();
            }
            // SAFETY: usleep has no preconditions.
            unsafe { libc::usleep(2000) };
        }
    }

    fn cleanup(&mut self) {
        // SAFETY: closing the two descriptors this struct owns, once.
        unsafe {
            libc::close(self.rx);
            libc::close(self.tx);
        }
        self.rx = -1;
        self.tx = -1;
        if let Some(j) = self.jail.take() {
            let _ = std::fs::remove_dir(j);
        }
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        if !self.reaped {
            self.kill_and_reap();
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ReadErr {
    /// the pipe ended (the worker is gone)
    End,
    Timeout,
    TooLong,
}

/// CLOCK_MONOTONIC in milliseconds: it stops while the Kindle sleeps, so a suspend never kills a healthy
/// worker (3.7, Timers).
pub fn mono_ms() -> u64 {
    // SAFETY: ts is written by clock_gettime before it is read.
    let mut ts: libc::timespec = unsafe { std::mem::zeroed() };
    unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &mut ts) };
    ts.tv_sec as u64 * 1000 + ts.tv_nsec as u64 / 1_000_000
}

/// The line the device log gets at the first worker of a session (3.7, The device log).
pub fn sandboxed_line() -> String {
    #[cfg(target_os = "linux")]
    {
        if sandbox_required() {
            return format!(
                "work: sandboxed (uid {}, chroot, netns, no new privileges, seccomp, selinux {})",
                WORKER_ID,
                linux::selinux()
            );
        }
    }
    "work: not sandboxed (plain process isolation: the steps need the Kindle or GRIMOIRE_SANDBOX=require as root on Linux)".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn filter_verdicts() {
        for n in [&ARM, &X86_64, &AARCH64] {
            let (al, en) = (allow_list_for(n), n.enosys.to_vec());
            let f = build_filter(n.audit_arch, &al, &en);
            for &s in &al {
                assert_eq!(eval_filter(&f, n.audit_arch, s), SECCOMP_RET_ALLOW, "{s}");
            }
            for &s in &en {
                assert_eq!(eval_filter(&f, n.audit_arch, s), SECCOMP_RET_ERRNO | 38, "{s}");
            }
            for s in 0..700u32 {
                if !al.contains(&s) && !en.contains(&s) {
                    assert_eq!(eval_filter(&f, n.audit_arch, s), SECCOMP_RET_TRAP, "{s}");
                }
            }
            // another architecture's call numbers are never taken for ours
            assert_eq!(eval_filter(&f, n.audit_arch ^ 1, al[0]), SECCOMP_RET_TRAP);
        }
    }

    // A panic's message asks for gettid (S1 saw it trap on the Scribe): it answers ENOSYS on every architecture,
    // so a worker that panics reports "E panic" instead of dying of SIGSYS.
    #[test]
    fn gettid_answers_enosys() {
        for (n, gettid) in [(&ARM, 224u32), (&X86_64, 186), (&AARCH64, 178)] {
            assert!(n.enosys.contains(&gettid), "{gettid}");
            let f = build_filter(n.audit_arch, &allow_list_for(n), n.enosys);
            assert_eq!(eval_filter(&f, n.audit_arch, gettid), SECCOMP_RET_ERRNO | 38);
        }
        #[cfg(target_os = "linux")]
        assert!(this_arch().enosys.contains(&(libc::SYS_gettid as u32)));
    }

    // The calls a worker must never make (socket, execve, kill, ptrace, mount, chroot) are not on any
    // architecture's list: each traps.
    #[test]
    fn the_calls_that_reach_out_are_trapped() {
        for n in [&ARM, &X86_64, &AARCH64] {
            let f = build_filter(n.audit_arch, &allow_list_for(n), n.enosys);
            for nr in n.reach_out {
                assert!(!allow_list_for(n).contains(&nr), "{nr}");
                assert_eq!(eval_filter(&f, n.audit_arch, nr), SECCOMP_RET_TRAP, "{nr}");
            }
        }
    }

    // The tables are typed by hand: on Linux they are checked against libc's.
    #[cfg(target_os = "linux")]
    #[test]
    fn the_host_table_matches_libc() {
        let n = this_arch();
        assert_eq!(
            [n.read, n.write, n.munmap, n.mremap, n.madvise, n.mprotect, n.brk, n.futex, n.clock_gettime, n.rt_sigreturn, n.exit, n.exit_group],
            [
                libc::SYS_read as u32,
                libc::SYS_write as u32,
                libc::SYS_munmap as u32,
                libc::SYS_mremap as u32,
                libc::SYS_madvise as u32,
                libc::SYS_mprotect as u32,
                libc::SYS_brk as u32,
                libc::SYS_futex as u32,
                libc::SYS_clock_gettime as u32,
                libc::SYS_rt_sigreturn as u32,
                libc::SYS_exit as u32,
                libc::SYS_exit_group as u32
            ]
        );
        #[cfg(target_arch = "arm")]
        assert_eq!(n.mmap, libc::SYS_mmap2 as u32);
        #[cfg(not(target_arch = "arm"))]
        assert_eq!(n.mmap, libc::SYS_mmap as u32);
        assert_eq!(
            n.reach_out,
            [
                libc::SYS_socket as u32,
                libc::SYS_execve as u32,
                libc::SYS_kill as u32,
                libc::SYS_ptrace as u32,
                libc::SYS_mount as u32,
                libc::SYS_chroot as u32
            ]
        );
    }

    // The fail-closed rule, with no Linux needed: a step that fails stops a required run at that step (nothing
    // after it runs, so no byte of the source is read), and a run that is not required notes it and goes on.
    #[test]
    fn a_failed_step_stops_a_required_run() {
        let later = Cell::new(false);
        let mut steps = vec![
            Step { name: "oom", run: Box::new(|| Ok("ok".into())) },
            Step { name: "chroot", run: Box::new(|| Err(libc::EPERM)) },
            Step {
                name: "seccomp",
                run: Box::new(|| {
                    later.set(true);
                    Ok("ok".into())
                }),
            },
        ];
        let mut heard = Vec::new();
        let r = apply_steps(&mut steps, true, |n, ok, _| heard.push((n.to_string(), ok)));
        assert_eq!(r, Err("chroot"));
        assert!(!later.get(), "the step after the failure ran");
        assert_eq!(heard, vec![("oom".to_string(), true), ("chroot".to_string(), false)]);
    }

    #[test]
    fn a_run_not_required_goes_on() {
        let later = Cell::new(false);
        let mut steps = vec![
            Step { name: "chroot", run: Box::new(|| Err(libc::EPERM)) },
            Step {
                name: "seccomp",
                run: Box::new(|| {
                    later.set(true);
                    Ok("ok".into())
                }),
            },
        ];
        assert_eq!(apply_steps(&mut steps, false, |_, _, _| {}), Ok(()));
        assert!(later.get());
    }

    // The Kindle build always requires the sandbox, whatever its environment says (so a refusing step can
    // never fall through to parsing); any other build only when asked.
    #[test]
    fn the_kindle_build_requires_the_sandbox() {
        assert!(required_for(true, None));
        assert!(required_for(true, Some("off")));
        assert!(!required_for(false, None));
        assert!(!required_for(false, Some("off")));
        assert!(required_for(false, Some("require")));
    }

    #[test]
    fn work_args_round_trip() {
        let v: Vec<String> = ["grimoire", "--work", "pdf", "123", "200000000", "60", "map", "4096", "/tmp/j"].iter().map(|s| s.to_string()).collect();
        let a = parse_work_args(&v).unwrap();
        assert_eq!(a.kind, WorkKind::Pdf);
        assert_eq!(a.writer_pid, 123);
        assert_eq!(a.limits, Limits { as_bytes: 200_000_000, cpu_secs: 60, mapped: true, file_len: 4096 });
        assert_eq!(a.jail.as_deref(), Some("/tmp/j"));
        assert_eq!(a.fail_step, None);
        let mut with_fail = v.clone();
        with_fail.push("fail=chroot".into());
        assert_eq!(parse_work_args(&with_fail).unwrap().fail_step.as_deref(), Some("chroot"));
        assert!(parse_work_args(&v[..8]).is_none());
        let mut bad = v.clone();
        bad[2] = "movie".into();
        assert!(parse_work_args(&bad).is_none());
        let mut font = v.clone();
        font[2] = "font".into();
        assert_eq!(parse_work_args(&font).unwrap().kind, WorkKind::Font);
    }

    #[test]
    fn the_worker_address_space_is_its_image_the_file_and_the_margin() {
        assert_eq!(worker_as_bytes(10 << 20, false), (11 << 20) + (10 << 20) + (160 << 20));
        assert_eq!(worker_as_bytes(10 << 20, true), (11 << 20) + (10 << 20) + (96 << 20));
    }

    #[test]
    fn no_em_dash() {
        assert!(!include_str!("sandbox.rs").contains('\u{2014}'));
    }
}
