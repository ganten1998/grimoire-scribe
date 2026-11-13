//! Nonblocking access to the launcher's Kindle Wi-Fi helper (kindle/app/wifi.sh). Only explicit UI
//! actions change the radio or the network; queries run only while Share's Wi-Fi card or Settings >
//! Wi-Fi is visible. Of the saved networks only their names come here, in hex, from the helper, which
//! keeps their passwords out of its output; no name is logged (grimoire.log goes to public bug reports).
use crate::qlog;
use std::cell::RefCell;
use std::io::Read;
use std::os::fd::AsRawFd;
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WifiStatus {
    pub wireless: bool,
    pub enabled: bool,
    pub connected: Option<bool>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WifiAction {
    Status,
    On,
    Off,
    /// the status, the network connected to and the saved networks (Settings > Wi-Fi)
    Networks,
    /// the status and the network connected to (watching a switch)
    Current,
    /// connect to the saved network wifi_join names
    Join,
}
impl WifiAction {
    /// a change of the radio or of the network, not a read
    fn changes(self) -> bool {
        matches!(self, WifiAction::On | WifiAction::Off | WifiAction::Join)
    }
}
/// A switch to a saved network (wifi_join): asked for, connected to it, or not (the helper refused it,
/// or no connection to it came in JOIN_WAIT).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WifiJoin {
    Pending,
    Connected,
    Failed,
}
/// How long a switch to a saved network may take before it counts as failed: the Share card's watch.
pub const JOIN_WAIT: Duration = Duration::from_secs(45);
/// The most saved networks a report may list (the helper prints no more).
const MAX_SAVED: usize = 64;
/// The most a report may hold: 64 names of 32 bytes in hex and the status.
const MAX_OUTPUT: usize = 8192;
struct Job {
    child: Child,
    start: Instant,
    output: Vec<u8>,
    failed: bool,
    action: WifiAction,
}
#[derive(Default)]
struct State {
    script: String,
    /// the helper was there at init (it does not come or go while Grimoire runs: checked once, not at
    /// every Share tick and draw)
    available: bool,
    job: Option<Job>,
    status: Option<WifiStatus>,
    failed: bool,
    queued: Option<WifiAction>,
    /// the network connected to, as the last read that says so had it (None: none, or not known)
    current: Option<Vec<u8>>,
    /// the saved networks, as the last Networks read had them (None: not read, or could not be)
    saved: Option<Vec<Vec<u8>>>,
    join: Option<Joining>,
}
struct Joining {
    essid: Vec<u8>,
    start: Instant,
    phase: WifiJoin,
}
thread_local! {static STATE:RefCell<State>=RefCell::new(State::default());}

pub fn wifi_init(script: Option<&str>) {
    wifi_free();
    STATE.with(|s| {
        let script = script.unwrap_or("").to_string();
        let available = std::fs::metadata(&script).is_ok_and(|m| m.is_file());
        *s.borrow_mut() = State {
            script,
            available,
            ..State::default()
        }
    });
}
pub fn wifi_available() -> bool {
    STATE.with(|s| s.borrow().available)
}
pub fn wifi_changing() -> bool {
    STATE.with(|s| {
        let s = s.borrow();
        s.queued.is_some() || s.job.as_ref().is_some_and(|j| j.action.changes())
    })
}
pub fn wifi_busy() -> bool {
    STATE.with(|s| s.borrow().job.is_some())
}
pub fn wifi_status() -> Option<WifiStatus> {
    STATE.with(|s| s.borrow().status)
}
pub fn wifi_failed() -> bool {
    STATE.with(|s| s.borrow().failed)
}
/// The network connected to, by name (bytes, as the Kindle has it), when a read said so.
pub fn wifi_current() -> Option<Vec<u8>> {
    STATE.with(|s| s.borrow().current.clone())
}
/// The saved networks, by name, as Settings > Wi-Fi last read them; None when they could not be read.
pub fn wifi_saved() -> Option<Vec<Vec<u8>>> {
    STATE.with(|s| s.borrow().saved.clone())
}
/// The switch asked for last (wifi_join), and how it went.
pub fn wifi_joining() -> Option<(Vec<u8>, WifiJoin)> {
    STATE.with(|s| s.borrow().join.as_ref().map(|j| (j.essid.clone(), j.phase)))
}
/// Forget the last switch's outcome (Settings > Wi-Fi opened again). One still asked for is kept.
pub fn wifi_join_clear() {
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        if s.join.as_ref().is_some_and(|j| j.phase != WifiJoin::Pending) {
            s.join = None;
        }
    })
}
/// Ask the helper to connect to one of the saved networks, by its name. False when it cannot be asked now
/// (no helper, or another change under way). Whether it connects shows in wifi_joining, as reads of the
/// network connected to (WifiAction::Current) come in; after JOIN_WAIT without it, it failed.
pub fn wifi_join(essid: &[u8]) -> bool {
    if essid.is_empty() || essid.len() > 32 {
        return false;
    }
    let was = STATE.with(|s| {
        s.borrow_mut().join.replace(Joining { essid: essid.to_vec(), start: Instant::now(), phase: WifiJoin::Pending })
    });
    if wifi_request(WifiAction::Join) {
        return true;
    }
    STATE.with(|s| s.borrow_mut().join = was);
    false
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
pub fn wifi_request(action: WifiAction) -> bool {
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        if s.script.is_empty() {
            return false;
        }
        if action == WifiAction::Join && s.join.as_ref().is_none_or(|j| j.phase != WifiJoin::Pending) {
            return false;
        }
        if let Some(job) = &s.job {
            if !job.action.changes() && action.changes() && s.queued.is_none() {
                s.queued = Some(action);
                if matches!(action, WifiAction::On | WifiAction::Off) {
                    s.join = None;
                }
                return true;
            }
            return false;
        }
        let verb = match action {
            WifiAction::Status => "status",
            WifiAction::On => "on",
            WifiAction::Off => "off",
            WifiAction::Networks => "networks",
            WifiAction::Current => "current",
            WifiAction::Join => "join",
        };
        let mut command = Command::new("/bin/sh");
        command.arg(&s.script).arg(verb);
        if action == WifiAction::Join {
            // (the name in hex: no byte of it is ever taken by the shell for anything but data)
            command.arg(hex(&s.join.as_ref().unwrap().essid));
        }
        command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .process_group(0);
        // SAFETY: after fork, only async-signal-safe fcntl calls touch this child's descriptor
        // table. Keep the Rust spawn error pipe alive until exec, but inherit no app sockets,
        // framebuffer or input handles into the shell/helper. This matches the other script jobs.
        unsafe {
            command.pre_exec(|| {
                for fd in 3..1024 {
                    let flags = libc::fcntl(fd, libc::F_GETFD);
                    if flags >= 0 {
                        libc::fcntl(fd, libc::F_SETFD, flags | libc::FD_CLOEXEC);
                    }
                }
                Ok(())
            });
        }
        let spawned = command.spawn();
        let Ok(mut child) = spawned else {
            s.failed = true;
            return false;
        };
        let fd = child.stdout.as_ref().unwrap().as_raw_fd();
        // SAFETY: fcntl modifies the pipe descriptor owned by this Child.
        let nonblocking = unsafe {
            let flags = libc::fcntl(fd, libc::F_GETFL);
            flags >= 0 && libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) >= 0
        };
        if !nonblocking {
            // SAFETY: terminate only the process group created for this helper, including
            // a lipc command it may already have forked before pipe setup failed.
            unsafe {
                libc::kill(-(child.id() as i32), libc::SIGKILL);
            }
            let _ = child.wait();
            s.failed = true;
            return false;
        }
        s.failed = false;
        if matches!(action, WifiAction::On | WifiAction::Off) {
            s.join = None;
        }
        s.job = Some(Job {
            child,
            start: Instant::now(),
            output: Vec::new(),
            failed: false,
            action,
        });
        true
    })
}
/// What the helper reported: the status, and the lines a Networks or Current read adds: the network
/// connected to (None: the line was not there; Some(None): no network) and the saved networks (None: no
/// list; Some(None): list=0, they could not be read).
#[derive(Debug, Default, PartialEq)]
struct Report {
    status: WifiStatus,
    current: Option<Option<Vec<u8>>>,
    saved: Option<Option<Vec<Vec<u8>>>>,
}
/// A network's name as the helper writes it: lowercase hex of 1 to 32 bytes, none a control character.
fn unhex(h: &str) -> Option<Vec<u8>> {
    let b = h.as_bytes();
    if b.is_empty() || b.len() > 64 || b.len() % 2 != 0 || !b.iter().all(|c| matches!(c, b'0'..=b'9' | b'a'..=b'f')) {
        return None;
    }
    let name: Vec<u8> = b.chunks(2).map(|p| u8::from_str_radix(std::str::from_utf8(p).unwrap(), 16).unwrap()).collect();
    (!name.iter().any(|&c| c < 0x20 || c == 0x7f)).then_some(name)
}
fn parse(bytes: &[u8]) -> Option<Report> {
    let text = std::str::from_utf8(bytes).ok()?;
    let (mut w, mut e, mut c) = (None, None, None);
    let mut current: Option<Option<Vec<u8>>> = None;
    let mut list: Option<bool> = None;
    let mut saved: Vec<Vec<u8>> = Vec::new();
    for line in text.lines() {
        match line {
            "wireless=0" if w.is_none() => w = Some(false),
            "wireless=1" if w.is_none() => w = Some(true),
            "enabled=0" if e.is_none() => e = Some(false),
            "enabled=1" if e.is_none() => e = Some(true),
            "connected=0" if c.is_none() => c = Some(Some(false)),
            "connected=1" if c.is_none() => c = Some(Some(true)),
            "connected=unknown" if c.is_none() => c = Some(None),
            "list=0" | "list=1" if list.is_none() => list = Some(line == "list=1"),
            _ => {
                if let Some(h) = line.strip_prefix("current=") {
                    if current.is_some() {
                        return None;
                    }
                    current = Some(if h.is_empty() { None } else { Some(unhex(h)?) });
                } else if let Some(h) = line.strip_prefix("saved=") {
                    // (the names come before list=1, which says they are all there)
                    if list.is_some() || saved.len() >= MAX_SAVED {
                        return None;
                    }
                    let name = unhex(h)?;
                    if !saved.contains(&name) {
                        saved.push(name);
                    }
                } else {
                    return None;
                }
            }
        }
    }
    let (wireless, enabled, connected) = (w?, e?, c?);
    if connected == Some(true) && !(wireless && enabled) {
        return None;
    }
    // A name connected to with no connection, or names with no list=1, is no report this helper makes.
    if current.as_ref().is_some_and(|n| n.is_some()) && connected != Some(true) {
        return None;
    }
    if !saved.is_empty() && list != Some(true) {
        return None;
    }
    Some(Report {
        status: WifiStatus {
            wireless,
            enabled,
            connected,
        },
        current,
        saved: list.map(|l| l.then_some(saved)),
    })
}
fn drain(job: &mut Job) {
    let mut buf = [0; 256];
    loop {
        match job.child.stdout.as_mut().unwrap().read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                if job.output.len() + n > MAX_OUTPUT {
                    job.failed = true;
                    break;
                }
                job.output.extend_from_slice(&buf[..n]);
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => {
                job.failed = true;
                break;
            }
        }
    }
}
/// Reap a bounded helper. A stalled helper and its descendants are killed together after 12 s. A switch
/// to a saved network with no connection to it after JOIN_WAIT has failed.
pub fn wifi_tick() -> bool {
    let late = STATE.with(|s| {
        let mut s = s.borrow_mut();
        match s.join.as_mut() {
            Some(j) if j.phase == WifiJoin::Pending && j.start.elapsed() > JOIN_WAIT => {
                j.phase = WifiJoin::Failed;
                qlog!("wifi: the switch to a saved network did not connect in %d s", JOIN_WAIT.as_secs() as i32);
                true
            }
            _ => false,
        }
    });
    let changed = STATE.with(|s| {
        let mut s = s.borrow_mut();
        let Some(job) = s.job.as_mut() else {
            return false;
        };
        drain(job);
        if job.failed || job.start.elapsed() > Duration::from_secs(12) {
            job.failed = true;
            // SAFETY: this child was spawned in a new process group whose ID is its PID.
            unsafe {
                libc::kill(-(job.child.id() as i32), libc::SIGKILL);
            }
        }
        let ended = match job.child.try_wait() {
            Ok(Some(exit)) => Some(exit.success()),
            Ok(None) => None,
            Err(_) => Some(false),
        };
        let Some(success) = ended else {
            return false;
        };
        drain(job);
        let job = s.job.take().unwrap();
        let report = if success && !job.failed { parse(&job.output) } else { None };
        s.status = report.as_ref().map(|r| r.status);
        s.failed = s.status.is_none();
        match &report {
            Some(r) => {
                if let Some(c) = &r.current {
                    s.current = c.clone();
                } else if r.status.connected != Some(true) {
                    s.current = None;
                }
                if let Some(l) = &r.saved {
                    s.saved = l.clone();
                }
            }
            None => {
                s.current = None;
                if job.action == WifiAction::Networks {
                    s.saved = None;
                }
            }
        }
        // A switch is done once a read has the Kindle connected to that network; a refused one failed.
        let connected_to = report.as_ref().is_some_and(|r| r.status.connected == Some(true));
        let current = s.current.clone();
        if let Some(j) = s.join.as_mut().filter(|j| j.phase == WifiJoin::Pending) {
            if job.action == WifiAction::Join && report.is_none() {
                j.phase = WifiJoin::Failed;
                qlog!("wifi: the switch to a saved network was refused or failed");
            } else if connected_to && current.as_deref() == Some(&j.essid[..]) {
                j.phase = WifiJoin::Connected;
                qlog!("wifi: switched to a saved network");
            }
        }
        // A Turn on/off tapped during the first status check still runs when that check failed (the
        // helper checks the radio itself before it changes anything); only a failed change drops it.
        if s.failed && job.action.changes() {
            s.queued = None;
        }
        true
    });
    let next = STATE.with(|s| {
        let mut s = s.borrow_mut();
        if s.job.is_none() {
            s.queued.take()
        } else {
            None
        }
    });
    if let Some(action) = next {
        wifi_request(action);
    }
    late || changed || next.is_some()
}
/// At exit: a read is stopped; a radio change or a switch in flight is let finish on its own (its two
/// lipc steps cut between would leave the radio half on for KOReader). It is in its own process group,
/// about a second from done, and init reaps it once Grimoire is gone.
pub fn wifi_free() {
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        s.queued = None;
        if let Some(mut job) = s.job.take() {
            if job.action.changes() {
                // (dropping the Child neither kills nor waits; its report to the closed pipe comes last)
                return;
            }
            // SAFETY: the group belongs only to this helper, not to Grimoire or KOReader.
            unsafe {
                libc::kill(-(job.child.id() as i32), libc::SIGKILL);
            }
            let _ = job.child.wait();
        }
    });
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn status_is_strict_and_does_not_claim_a_connection_on_a_disabled_radio() {
        assert_eq!(
            parse(b"wireless=1\nenabled=1\nconnected=1\n")
                .unwrap()
                .status
                .connected,
            Some(true)
        );
        assert_eq!(
            parse(b"wireless=1\nenabled=1\nconnected=unknown\n")
                .unwrap()
                .status
                .connected,
            None
        );
        for bad in [
            "wireless=0\nenabled=1\nconnected=1\n",
            "wireless=1\nenabled=1\nconnected=0\nwireless=0\n",
            "wireless=1\nenabled=1\n",
            "nonsense",
        ] {
            assert!(parse(bad.as_bytes()).is_none());
        }
    }
    #[test]
    fn commands_are_nonblocking_and_fail_closed_on_timeout_or_bad_output() {
        use std::fs;
        let root = std::env::temp_dir().join(format!("wifi-job-{}", std::process::id()));
        fs::create_dir_all(&root).unwrap();
        let script = root.join("helper.sh");
        let marker = fs::File::create(root.join("private-handle")).unwrap();
        // SAFETY: duplicate an owned descriptor for the inheritance regression test.
        let private_fd = unsafe { libc::fcntl(marker.as_raw_fd(), libc::F_DUPFD, 200) };
        assert!(private_fd >= 200);
        fs::write(&script,format!("test ! -e /dev/fd/{private_fd} || exit 1\nprintf 'wireless=1\\nenabled=1\\nconnected=1\\n'\n")).unwrap();
        wifi_init(script.to_str());
        assert!(wifi_request(WifiAction::Status));
        while wifi_busy() {
            wifi_tick();
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(!wifi_failed());
        unsafe {
            libc::close(private_fd);
        }
        fs::write(
            &script,
            "sleep 0.15\nprintf 'wireless=1\\nenabled=1\\nconnected=1\\n'\n",
        )
        .unwrap();
        wifi_init(script.to_str());
        assert!(wifi_request(WifiAction::On));
        assert!(!wifi_request(WifiAction::Off));
        let began = Instant::now();
        assert!(!wifi_tick());
        assert!(began.elapsed() < Duration::from_millis(100));
        while wifi_busy() {
            wifi_tick();
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(wifi_status().unwrap().connected, Some(true));
        assert!(!wifi_failed());
        assert!(wifi_request(WifiAction::Status));
        assert!(wifi_request(WifiAction::On));
        assert!(wifi_changing());
        assert!(!wifi_request(WifiAction::Off));
        while wifi_busy() {
            wifi_tick();
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(wifi_status().is_some());
        fs::write(&script,"while :; do printf 'xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx'; done\n").unwrap();
        assert!(wifi_request(WifiAction::Status));
        while wifi_busy() {
            wifi_tick();
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(wifi_failed());
        assert!(wifi_status().is_none());
        fs::write(&script, "sleep 60 &\nwait\n").unwrap();
        assert!(wifi_request(WifiAction::Status));
        STATE.with(|s| {
            s.borrow_mut().job.as_mut().unwrap().start = Instant::now() - Duration::from_secs(13)
        });
        for _ in 0..100 {
            wifi_tick();
            if !wifi_busy() {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(!wifi_busy());
        assert!(wifi_failed());
        wifi_free();
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn a_radio_change_finishes_after_exit_and_a_tap_survives_a_failed_status() {
        use std::fs;
        let root = std::env::temp_dir().join(format!("wifi-exit-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let script = root.join("helper.sh");
        let done = root.join("changed");
        // status fails (no report); on/off changes the radio after a moment, then reports
        fs::write(&script, format!("case $1 in status) sleep 0.1; exit 1;; esac\nsleep 0.2\ntouch '{}'\nprintf 'wireless=1\\nenabled=1\\nconnected=0\\n'\n", done.display())).unwrap();
        wifi_init(script.to_str());
        assert!(wifi_available());
        // A Turn on tapped while the first status check runs: the check fails, the tap still runs.
        assert!(wifi_request(WifiAction::Status));
        assert!(wifi_request(WifiAction::On));
        let began = Instant::now();
        while (wifi_busy() || wifi_changing()) && began.elapsed() < Duration::from_secs(5) {
            wifi_tick();
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(done.exists(), "the queued Turn on was dropped");
        assert!(!wifi_failed());
        fs::remove_file(&done).unwrap();
        // Exit during a radio change: the helper is not killed, and finishes.
        assert!(wifi_request(WifiAction::Off));
        wifi_free();
        assert!(!wifi_busy());
        let began = Instant::now();
        while !done.exists() && began.elapsed() < Duration::from_secs(5) {
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(done.exists(), "the radio change was killed at exit");
        // The helper is checked once: availability does not touch storage again.
        fs::remove_file(&script).unwrap();
        assert!(wifi_available());
        wifi_init(None);
        assert!(!wifi_available());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn networks_come_in_hex_and_anything_else_fails_closed() {
        let r = parse(b"wireless=1\nenabled=1\nconnected=1\ncurrent=486f6d65\nsaved=486f6d65\nsaved=e78cabe381aee5aeb6\nsaved=486f6d65\nlist=1\n").unwrap();
        assert_eq!(r.current, Some(Some(b"Home".to_vec())));
        // (a name twice is listed once)
        assert_eq!(r.saved, Some(Some(vec![b"Home".to_vec(), "\u{732b}\u{306e}\u{5bb6}".as_bytes().to_vec()])));
        let r = parse(b"wireless=0\nenabled=0\nconnected=0\ncurrent=\nlist=0\n").unwrap();
        assert_eq!((r.current, r.saved), (Some(None), Some(None)));
        let r = parse(b"wireless=1\nenabled=1\nconnected=1\ncurrent=\nlist=1\n").unwrap();
        assert_eq!((r.current, r.saved), (Some(None), Some(Some(Vec::new()))));
        // the status alone (the Share card's) says nothing of either
        let r = parse(b"wireless=1\nenabled=1\nconnected=0\n").unwrap();
        assert_eq!((r.current, r.saved), (None, None));
        let head = "wireless=1\nenabled=1\nconnected=1\n";
        let many: String = (0..65).map(|i| format!("saved={:02x}41\n", i + 0x30)).collect();
        for bad in [
            "wireless=1\nenabled=1\nconnected=0\ncurrent=486f6d65\n".to_string(),
            "wireless=1\nenabled=1\nconnected=unknown\ncurrent=486f6d65\n".to_string(),
            format!("{head}list=1\nsaved=486f6d65\n"),
            format!("{head}saved=486f6d65\n"),
            format!("{head}saved=486f6d65\nlist=0\n"),
            format!("{head}current=\ncurrent=\n"),
            format!("{head}list=1\nlist=1\n"),
            format!("{head}saved=486F6D65\nlist=1\n"),
            format!("{head}saved=486f6d6\nlist=1\n"),
            format!("{head}saved=\nlist=1\n"),
            format!("{head}saved={}\nlist=1\n", "41".repeat(33)),
            format!("{head}saved=486f0a65\nlist=1\n"),
            format!("{head}saved=486f7f65\nlist=1\n"),
            format!("{head}saved=48 6f\nlist=1\n"),
            format!("{head}psk=secret\nlist=1\n"),
            format!("{head}{many}list=1\n"),
        ] {
            assert!(parse(bad.as_bytes()).is_none(), "{bad}");
        }
        let fine: String = (0..64).map(|i| format!("saved={:02x}41\n", i + 0x30)).collect();
        assert_eq!(parse(format!("{head}{fine}list=1\n").as_bytes()).unwrap().saved.unwrap().unwrap().len(), 64);
    }
    #[test]
    fn a_switch_names_its_network_in_hex_and_ends_connected_refused_or_late() {
        use std::fs;
        let root = std::env::temp_dir().join(format!("wifi-join-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let (script, log, cur, done) = (root.join("helper.sh"), root.join("args"), root.join("current"), root.join("done"));
        // A name a shell would run, were it ever taken for code (the helper runs in root: its files would land
        // there).
        let evil = "a'\"; touch pwned; $(touch p2)`x`".to_string();
        assert_eq!(evil.len(), 32);
        let far = hex(b"Far away");
        let slow = hex(b"Slow");
        fs::write(&script, format!(r#"echo "$*" >> '{log}'
cd '{root}'
case $1 in
networks) printf 'wireless=1\nenabled=1\nconnected=1\ncurrent=486f6d65\nsaved=486f6d65\nsaved={evil}\nsaved={far}\nsaved={slow}\nlist=1\n'; sleep 0.1;;
join) [ "$2" = {far} ] && exit 3; [ "$2" = {slow} ] || echo "$2" > '{cur}'; sleep 0.2; touch '{done}'; printf 'wireless=1\nenabled=1\nconnected=1\n';;
current) printf 'wireless=1\nenabled=1\nconnected=1\ncurrent=%s\n' "$(cat '{cur}' 2>/dev/null || echo 486f6d65)";;
*) printf 'wireless=1\nenabled=1\nconnected=1\n';;
esac
"#, log = log.display(), root = root.display(), cur = cur.display(), done = done.display(), evil = hex(evil.as_bytes()))).unwrap();
        let settle = || {
            let began = Instant::now();
            while (wifi_busy() || wifi_changing()) && began.elapsed() < Duration::from_secs(5) {
                wifi_tick();
                std::thread::sleep(Duration::from_millis(5));
            }
        };
        wifi_init(script.to_str());
        assert!(!wifi_join(b""));
        assert!(!wifi_join(&[b'x'; 33]));
        // A switch asked for while the list is read waits for it, then runs.
        assert!(wifi_request(WifiAction::Networks));
        assert!(wifi_join(evil.as_bytes()));
        assert!(wifi_changing());
        settle();
        let saved = wifi_saved().unwrap();
        assert_eq!(saved.len(), 4);
        assert_eq!(saved[1], evil.as_bytes());
        // The helper's own report says connected (to Home still): no success until a read names the network.
        assert_eq!(wifi_joining(), Some((evil.as_bytes().to_vec(), WifiJoin::Pending)));
        assert_eq!(fs::read_to_string(&log).unwrap().lines().collect::<Vec<_>>(), ["networks", &format!("join {}", hex(evil.as_bytes()))]);
        assert!(!root.join("pwned").exists() && !root.join("p2").exists() && !root.join("x").exists());
        assert!(wifi_request(WifiAction::Current));
        settle();
        assert_eq!(wifi_joining().unwrap().1, WifiJoin::Connected);
        assert_eq!(wifi_current().unwrap(), evil.as_bytes());
        // One switch at a time: another while it runs is refused, and the first stays the one watched.
        assert!(wifi_join(b"Home"));
        assert!(!wifi_join(b"Slow"));
        assert_eq!(wifi_joining().unwrap().0, b"Home");
        settle();
        // Refused by the helper (no such saved network, or wifid said no): failed at once.
        assert!(wifi_join(b"Far away"));
        settle();
        assert_eq!(wifi_joining(), Some((b"Far away".to_vec(), WifiJoin::Failed)));
        wifi_join_clear();
        assert_eq!(wifi_joining(), None);
        // Asked for, but no read names it: failed after JOIN_WAIT, and the tick says something changed.
        assert!(wifi_join(b"Slow"));
        settle();
        assert!(wifi_request(WifiAction::Current));
        settle();
        assert_eq!(wifi_joining().unwrap().1, WifiJoin::Pending);
        wifi_join_clear();
        assert_eq!(wifi_joining().unwrap().1, WifiJoin::Pending, "a switch under way is kept");
        STATE.with(|s| s.borrow_mut().join.as_mut().unwrap().start = Instant::now() - JOIN_WAIT - Duration::from_secs(1));
        assert!(wifi_tick());
        assert_eq!(wifi_joining().unwrap().1, WifiJoin::Failed);
        // Turning the radio on or off is a new choice: the switch is forgotten.
        assert!(wifi_join(b"Slow"));
        settle();
        assert!(wifi_request(WifiAction::Off));
        assert_eq!(wifi_joining(), None);
        settle();
        // Exit during a switch: the helper is not killed, and finishes.
        let _ = fs::remove_file(&done);
        assert!(wifi_join(b"Home"));
        wifi_free();
        let began = Instant::now();
        while !done.exists() && began.elapsed() < Duration::from_secs(5) {
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(done.exists(), "the switch was killed at exit");
        fs::remove_dir_all(root).unwrap();
    }
}
