//! Nonblocking access to the launcher's Kindle Wi-Fi helper (kindle/app/wifi.sh). Only explicit UI
//! actions change the radio or the network; queries run only while Share's Wi-Fi card or Settings >
//! Wi-Fi is visible. Of the saved networks only their names come here, in hex, from the helper, which
//! keeps their passwords out of its output; no name is logged (grimoire.log goes to public bug reports).
//! A WPA/WPA2 add derives its PSK here (wpa.rs): the helper gets those 64 hex digits, never the passphrase.
//! A WPA2-Enterprise add sends its username and password to the helper only on stdin; the helper feeds
//! them to wpa_cli's own stdin, so neither is in an argument, and the profile lives in the supplicant.
use crate::qlog;
use std::cell::RefCell;
use std::io::{Read, Write};
use std::os::fd::AsRawFd;
use std::os::unix::ffi::OsStrExt;
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
    /// drop the network connected to but leave the radio on
    Disconnect,
    /// the status, the network connected to and the saved networks (Settings > Wi-Fi)
    Networks,
    /// the status and the network connected to (watching a switch)
    Current,
    /// connect to the saved network wifi_join names
    Join,
    /// ask wifid for a scan, then read scanList
    Scan,
    /// create a profile from the waiting password, then read the networks
    Add,
    /// delete one saved profile, then read the networks
    Delete,
}
impl WifiAction {
    /// a change of the radio or of a profile, not a read
    fn changes(self) -> bool {
        matches!(
            self,
            WifiAction::On
                | WifiAction::Off
                | WifiAction::Disconnect
                | WifiAction::Join
                | WifiAction::Add
                | WifiAction::Delete
        )
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
/// A nearby network's security, as the helper classified it from wifid's scanList. Unknown is kept so the
/// sheet can show that the network is there without offering to add it (nothing is ever guessed into Open).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WifiSecurity {
    Open,
    Wep,
    Wpa,
    Wpa2,
    Wpa3,
    Owe,
    Enterprise,
    /// WPA3-Enterprise 192-bit (WPA-EAP-SUITE-B-192): EAP-TLS and GCMP-256, which this helper does
    /// not create; kept apart so the sheet says so instead of trying PEAP on it
    Enterprise192,
    #[default]
    Unknown,
}
impl WifiSecurity {
    fn parse(s: &str) -> Self {
        match s {
            "open" => Self::Open,
            "wep" => Self::Wep,
            "wpa" => Self::Wpa,
            "wpa2" => Self::Wpa2,
            "wpa3" => Self::Wpa3,
            "owe" => Self::Owe,
            "enterprise" => Self::Enterprise,
            "enterprise192" => Self::Enterprise192,
            _ => Self::Unknown,
        }
    }
}
/// One network a scan found. `known` is wifid's word for one the Kindle has a profile for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WifiNetwork {
    pub essid: Vec<u8>,
    pub security: WifiSecurity,
    pub signal: i32,
    pub signal_max: i32,
    pub known: bool,
}
/// How long a switch to a saved network may take before it counts as failed: the Share card's watch.
pub const JOIN_WAIT: Duration = Duration::from_secs(45);
/// The most saved networks a report may list (the helper prints no more).
const MAX_SAVED: usize = 64;
/// The most nearby networks a scan may list.
const MAX_NEARBY: usize = 64;
/// The most a report may hold: 64 names of 32 bytes in hex and the status, or a scan's networks.
const MAX_OUTPUT: usize = 65536;
struct Job {
    child: Child,
    start: Instant,
    output: Vec<u8>,
    failed: bool,
    action: WifiAction,
    /// WifiAction::Add: the network the profile was created for, so the sheet can watch its
    /// connection like a switch (None: any other action)
    added: Option<Vec<u8>>,
    /// WifiAction::Add with EAP or WPA3-Personal: the profile is in the supplicant, not in wifid's
    /// saved list
    supplicant: bool,
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
    /// the nearby networks, as the last Scan read had them (None: not scanned yet, or scanning failed)
    nearby: Option<Vec<WifiNetwork>>,
    scan_failed: bool,
    /// the scan under way is a quiet refresh: its list is merged into the one already shown, in its order
    quiet_scan: bool,
    /// the last Add failed, so Settings > Wi-Fi can name that rather than the generic read error
    add_failed: bool,
    /// the last Delete failed, so Settings > Wi-Fi can name that rather than the generic read error
    delete_failed: bool,
    /// the last Disconnect failed, with the connection left as it was
    disconnect_failed: bool,
    join: Option<Joining>,
    /// a profile to create, with the PSK derived from its passphrase (never in an argument or a log),
    /// and one to delete
    add: Option<Adding>,
    delete: Option<Vec<u8>>,
}
struct Adding {
    essid: Vec<u8>,
    /// the 64 lower-case hex digits of the WPA/WPA2 PSK, the WPA3-Personal passphrase (SAE needs it
    /// itself), or empty for Open; wiped after it reaches the helper
    key: Vec<u8>,
    /// the EAP identity and password (empty otherwise), wiped after they reach the helper
    user: Vec<u8>,
    pass: Vec<u8>,
    security: WifiSecurity,
}
impl Adding {
    fn wipe(&mut self) {
        self.key.fill(0);
        self.user.fill(0);
        self.pass.fill(0);
    }
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
/// The last profile Add failed (the helper refused it, or its report could not be read).
pub fn wifi_add_failed() -> bool {
    STATE.with(|s| s.borrow().add_failed)
}
/// The last Forget failed (the helper refused it, or its report could not be read).
pub fn wifi_delete_failed() -> bool {
    STATE.with(|s| s.borrow().delete_failed)
}
/// The last Disconnect failed; the connection is still the one the sheet showed.
pub fn wifi_disconnect_failed() -> bool {
    STATE.with(|s| s.borrow().disconnect_failed)
}
/// The network connected to, by name (bytes, as the Kindle has it), when a read said so.
pub fn wifi_current() -> Option<Vec<u8>> {
    STATE.with(|s| s.borrow().current.clone())
}
/// The saved networks, by name, as Settings > Wi-Fi last read them; None when they could not be read.
pub fn wifi_saved() -> Option<Vec<Vec<u8>>> {
    STATE.with(|s| s.borrow().saved.clone())
}
/// The nearby networks, as the last scan read them; None when there is no scan or it failed.
pub fn wifi_nearby() -> Option<Vec<WifiNetwork>> {
    STATE.with(|s| s.borrow().nearby.clone())
}
pub fn wifi_scan_failed() -> bool {
    STATE.with(|s| s.borrow().scan_failed)
}
/// Create a profile, then read the networks back. For WPA/WPA2, `pass` is the passphrase: its PSK is
/// derived here and only those 64 hex digits go to the helper, on stdin. For WPA3-Personal the
/// passphrase itself goes to the helper on stdin (SAE derives its password element from it, not from
/// the PSK). None is taken for Open. False when the security mode is not one this helper creates, or
/// the values are not ones the helper takes.
pub fn wifi_add(essid: &[u8], pass: &[u8], security: WifiSecurity) -> bool {
    if essid.is_empty()
        || essid.len() > 32
        || essid.iter().any(|&b| b < 0x20 || b == 0x7f)
        || !matches!(
            security,
            WifiSecurity::Open | WifiSecurity::Wpa | WifiSecurity::Wpa2 | WifiSecurity::Wpa3
        )
    {
        return false;
    }
    if security != WifiSecurity::Open
        && (pass.len() < 8 || pass.len() > 63 || pass.iter().any(|&b| b < 0x20 || b > 0x7e))
    {
        return false;
    }
    let key = match security {
        WifiSecurity::Open => Vec::new(),
        // SAE takes the passphrase itself; a PSK would make the password element disagree with the AP's.
        WifiSecurity::Wpa3 => pass.to_vec(),
        _ => crate::wpa::psk_hex(pass, essid).to_vec(),
    };
    STATE.with(|s| s.borrow_mut().add_failed = false);
    let was = STATE.with(|s| {
        s.borrow_mut().add.replace(Adding {
            essid: essid.to_vec(),
            key,
            user: Vec::new(),
            pass: Vec::new(),
            security,
        })
    });
    if wifi_request(WifiAction::Add) {
        return true;
    }
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        if let Some(mut old) = was {
            old.wipe();
        }
        if let Some(mut a) = s.add.take() {
            a.wipe();
        }
    });
    false
}

/// Create a WPA2- or WPA3-Enterprise network (PEAP/MSCHAPv2, with PMF optional so both join), then read
/// the networks back. `user` and `pass` go to the helper on stdin, never in argv; the helper feeds them
/// to wpa_cli's stdin. The profile is in the supplicant rather than wifid's saved list, so it is not
/// expected in that list afterwards. WPA3-Enterprise 192-bit needs EAP-TLS, which is not created.
/// False when a value is not one the supplicant takes, or another change is under way.
pub fn wifi_add_eap(essid: &[u8], user: &[u8], pass: &[u8]) -> bool {
    if essid.is_empty()
        || essid.len() > 32
        || essid.iter().any(|&b| b < 0x20 || b == 0x7f)
        || user.is_empty()
        || user.len() > 128
        || user.iter().any(|&b| b < 0x20 || b == 0x7f)
        || pass.is_empty()
        || pass.len() > 128
        || pass.iter().any(|&b| b < 0x20 || b == 0x7f)
    {
        return false;
    }
    STATE.with(|s| s.borrow_mut().add_failed = false);
    let was = STATE.with(|s| {
        s.borrow_mut().add.replace(Adding {
            essid: essid.to_vec(),
            key: Vec::new(),
            user: user.to_vec(),
            pass: pass.to_vec(),
            security: WifiSecurity::Enterprise,
        })
    });
    if wifi_request(WifiAction::Add) {
        return true;
    }
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        if let Some(mut old) = was {
            old.wipe();
        }
        if let Some(mut a) = s.add.take() {
            a.wipe();
        }
    });
    false
}

/// Forget a saved network by its name, then read the networks back.
pub fn wifi_delete(essid: &[u8]) -> bool {
    if essid.is_empty() || essid.len() > 32 {
        return false;
    }
    let was = STATE.with(|s| s.borrow_mut().delete.replace(essid.to_vec()));
    if wifi_request(WifiAction::Delete) {
        return true;
    }
    STATE.with(|s| {
        s.borrow_mut().delete = was;
    });
    false
}

pub fn wifi_scanning() -> bool {
    STATE.with(|s| {
        let s = s.borrow();
        s.queued == Some(WifiAction::Scan) || s.job.as_ref().is_some_and(|j| j.action == WifiAction::Scan)
    })
}
/// A background rescan is under way: it keeps the list shown and merges its answer into it.
pub fn wifi_scan_quieting() -> bool {
    STATE.with(|s| s.borrow().quiet_scan)
}
/// Ask for a scan. False when it cannot be asked for now (no helper, or a change under way). The old
/// results stay until the new scan starts. A quiet refresh already under way serves the tap too.
pub fn wifi_scan() -> bool {
    if wifi_scan_quieting() && wifi_scanning() {
        return true;
    }
    STATE.with(|s| s.borrow_mut().scan_failed = false);
    if !wifi_request(WifiAction::Scan) {
        return false;
    }
    STATE.with(|s| s.borrow_mut().nearby = None);
    true
}
/// Ask for a background scan: the list shown stays, and the answer is merged into it in its order, so
/// networks that did not change do not move or disappear. False when it cannot be asked for now.
pub fn wifi_scan_quiet() -> bool {
    if !wifi_request(WifiAction::Scan) {
        return false;
    }
    STATE.with(|s| s.borrow_mut().quiet_scan = true);
    true
}
/// Disconnect the network connected to, leaving the radio on. False when it cannot be asked for now.
pub fn wifi_disconnect() -> bool {
    wifi_request(WifiAction::Disconnect)
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
        if action != WifiAction::Add {
            s.add_failed = false;
        }
        s.disconnect_failed = false;
        s.delete_failed = false;
        if action == WifiAction::Join && s.join.as_ref().is_none_or(|j| j.phase != WifiJoin::Pending) {
            return false;
        }
        if let Some(job_action) = s.job.as_ref().map(|j| j.action) {
            if action == WifiAction::Scan {
                if !job_action.changes() && s.queued.is_none() {
                    s.queued = Some(action);
                    return true;
                }
                return false;
            }
            if !job_action.changes() && action.changes() {
                if job_action == WifiAction::Scan {
                    // A scan must never stand between the user and a change. Kill and reap it; its
                    // answer was only for the quiet refresh or the old list, and the new action wins.
                    let mut job = s.job.take().unwrap();
                    // SAFETY: this child was spawned in a process group whose ID is its PID.
                    unsafe {
                        libc::kill(-(job.child.id() as i32), libc::SIGKILL);
                    }
                    let _ = job.child.wait();
                    s.quiet_scan = false;
                    s.queued = None;
                } else if s.queued.is_none() || s.queued == Some(WifiAction::Scan) {
                    // A scan waiting behind a read gives way to a change the user just asked for.
                    if s.queued == Some(WifiAction::Scan) {
                        s.quiet_scan = false;
                    }
                    s.queued = Some(action);
                    if matches!(action, WifiAction::On | WifiAction::Off | WifiAction::Disconnect) {
                        s.join = None;
                    }
                    return true;
                } else {
                    return false;
                }
            } else {
                return false;
            }
        }
        let verb = match action {
            WifiAction::Status => "status",
            WifiAction::On => "on",
            WifiAction::Off => "off",
            WifiAction::Disconnect => "disconnect",
            WifiAction::Networks => "networks",
            WifiAction::Current => "current",
            WifiAction::Join => "join",
            WifiAction::Scan => "scan",
            WifiAction::Add => "add",
            WifiAction::Delete => "delete",
        };
        let mut command = Command::new("/bin/sh");
        command.arg(&s.script).arg(verb);
        // 0 no stdin, 1 the PSK, 2 the EAP username and password
        let mut feed = 0u8;
        let mut supplicant = false;
        let mut added = None;
        match action {
            WifiAction::Join => {
                // (the name in hex: no byte of it is ever taken by the shell for anything but data)
                command.arg(hex(&s.join.as_ref().unwrap().essid));
            }
            WifiAction::Add => {
                let Some(a) = s.add.as_ref() else {
                    return false;
                };
                added = Some(a.essid.clone());
                // (the name is data: an argument is never run; the PSK goes only to stdin)
                command.arg(std::ffi::OsStr::from_bytes(&a.essid));
                command.arg(match a.security {
                    WifiSecurity::Open => "open",
                    WifiSecurity::Wpa => "wpa",
                    WifiSecurity::Wpa2 => "wpa2",
                    WifiSecurity::Wpa3 => "wpa3",
                    WifiSecurity::Enterprise => "eap",
                    _ => return false,
                });
                match a.security {
                    WifiSecurity::Open => {}
                    WifiSecurity::Enterprise => {
                        feed = 2;
                        supplicant = true;
                    }
                    WifiSecurity::Wpa3 => {
                        feed = 1;
                        supplicant = true;
                    }
                    _ => feed = 1,
                }
            }
            WifiAction::Delete => {
                let Some(n) = s.delete.as_ref() else {
                    return false;
                };
                command.arg(std::ffi::OsStr::from_bytes(n));
            }
            _ => {}
        }
        command
            .stdin(if feed != 0 { Stdio::piped() } else { Stdio::null() })
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
            if let Some(mut a) = s.add.take() {
                a.wipe();
            }
            s.delete = None;
            s.failed = true;
            return false;
        };
        if feed != 0 {
            if let Some(mut a) = s.add.take() {
                if let Some(mut stdin) = child.stdin.take() {
                    match feed {
                        2 => {
                            let _ = stdin.write_all(&a.user);
                            let _ = stdin.write_all(b"\n");
                            let _ = stdin.write_all(&a.pass);
                            let _ = stdin.write_all(b"\n");
                        }
                        _ => {
                            let _ = stdin.write_all(&a.key);
                            let _ = stdin.write_all(b"\n");
                        }
                    }
                }
                a.wipe();
            }
        } else {
            s.add = None;
        }
        s.delete = None;
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
        if matches!(action, WifiAction::On | WifiAction::Off | WifiAction::Disconnect) {
            s.join = None;
        }
        s.job = Some(Job {
            child,
            start: Instant::now(),
            output: Vec::new(),
            failed: false,
            action,
            added,
            supplicant,
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
    scan: Option<ScanReport>,
}
/// What a Scan read reported: the radio was off (no list), or the networks, strongest first.
#[derive(Debug, PartialEq)]
enum ScanReport {
    Off,
    List(Vec<WifiNetwork>),
}
/// One near= line: the name in hex, signal and its maximum, security, and wifid's known flag.
fn parse_near(line: &str) -> Option<WifiNetwork> {
    let mut p = line.split(',');
    let essid = unhex(p.next()?)?;
    let signal: i32 = p.next()?.parse().ok()?;
    let signal_max: i32 = p.next()?.parse().ok()?;
    if !(0..=100).contains(&signal) || !(1..=100).contains(&signal_max) {
        return None;
    }
    let security = WifiSecurity::parse(p.next()?);
    let known = match p.next()? {
        "0" => false,
        "1" => true,
        _ => return None,
    };
    if p.next().is_some() {
        return None;
    }
    Some(WifiNetwork { essid, security, signal, signal_max, known })
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
    let mut scan: Option<ScanReport> = None;
    let mut near_end = false;
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
            "scan=off" if scan.is_none() => scan = Some(ScanReport::Off),
            "scan=ok" if scan.is_none() => scan = Some(ScanReport::List(Vec::new())),
            "near_end=1" if scan.is_some() && !near_end => near_end = true,
            _ => {
                if let Some(h) = line.strip_prefix("near=") {
                    let ScanReport::List(networks) = scan.as_mut()? else {
                        return None;
                    };
                    if near_end || networks.len() >= MAX_NEARBY {
                        return None;
                    }
                    networks.push(parse_near(h)?);
                } else if let Some(h) = line.strip_prefix("current=") {
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
    if let Some(ScanReport::List(networks)) = &mut scan {
        if !near_end {
            return None;
        }
        // More than one BSSID per name: the one with the strongest signal stands for it.
        networks.sort_by(|a, b| b.signal.cmp(&a.signal));
        let mut seen: Vec<Vec<u8>> = Vec::new();
        networks.retain(|n| {
            if seen.contains(&n.essid) {
                false
            } else {
                seen.push(n.essid.clone());
                true
            }
        });
    }
    Some(Report {
        status: WifiStatus {
            wireless,
            enabled,
            connected,
        },
        current,
        saved: list.map(|l| l.then_some(saved)),
        scan,
    })
}
/// Merge a quiet scan's answer into the list already shown: every name stays where it was, with its new
/// signal and security, the names gone leave, and only names not there before are added at the end.
fn merge_nearby(old: Vec<WifiNetwork>, fresh: Vec<WifiNetwork>) -> Vec<WifiNetwork> {
    let mut merged = Vec::with_capacity(old.len() + fresh.len());
    for was in old {
        if let Some(now) = fresh.iter().find(|n| n.essid == was.essid) {
            merged.push(now.clone());
        }
    }
    for now in fresh {
        if !merged.iter().any(|n| n.essid == now.essid) {
            merged.push(now);
        }
    }
    merged
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
        let limit = match job.action {
            WifiAction::Scan => Duration::from_secs(25),
            // Every lipc and wpa_cli call in a disconnect is bounded (2 s), and it tries several routes:
            // the helper needs longer than a read before this outer limit may kill it without a report.
            WifiAction::Disconnect => Duration::from_secs(25),
            _ => Duration::from_secs(12),
        };
        if job.failed || job.start.elapsed() > limit {
            job.failed = true;
            // SAFETY: this child was spawned in a new process group whose ID is its PID.
            unsafe {
                libc::kill(-(job.child.id() as i32), libc::SIGKILL);
            }
        }
        let outcome = match job.child.try_wait() {
            Ok(Some(exit)) => Some((exit.success(), exit.code())),
            Ok(None) => None,
            Err(_) => Some((false, None)),
        };
        let Some((success, code)) = outcome else {
            return false;
        };
        drain(job);
        let job = s.job.take().unwrap();
        let report = if success && !job.failed { parse(&job.output) } else { None };
        let quiet_scan = job.action == WifiAction::Scan && s.quiet_scan;
        if job.action == WifiAction::Scan {
            // A failed scan says nothing about the radio: keep the status the sheet already had. A quiet
            // refresh also keeps the list it showed, and says nothing when its answer cannot be read.
            if let Some(r) = &report {
                s.status = Some(r.status);
                s.failed = false;
                s.disconnect_failed = false;
                if let Some(scan) = &r.scan {
                    let fresh = match scan {
                        ScanReport::Off => Vec::new(),
                        ScanReport::List(networks) => networks.clone(),
                    };
                    s.nearby = Some(if quiet_scan {
                        merge_nearby(s.nearby.take().unwrap_or_default(), fresh)
                    } else {
                        fresh
                    });
                    s.scan_failed = false;
                } else if !quiet_scan {
                    s.scan_failed = true;
                }
                if let Some(c) = &r.current {
                    s.current = c.clone();
                } else if r.status.connected != Some(true) {
                    s.current = None;
                }
            } else if !quiet_scan {
                s.scan_failed = true;
            }
            s.quiet_scan = false;
        } else {
            let disconnect_failed = job.action == WifiAction::Disconnect
                && report.as_ref().is_none_or(|r| r.status.connected != Some(false));
            if disconnect_failed {
                // The link is still there: keep the status the sheet already had rather than losing it to a
                // generic read error, and let the sheet name the failed disconnect. The helper's exit says
                // which step failed (its disconnect comment lists them; 2 means the script on the device
                // predates the verb; -1 means it was killed): nothing there names the network.
                qlog!("wifi: disconnect did not take (helper exit %d)", code.unwrap_or(-1));
                s.failed = false;
                s.disconnect_failed = true;
            } else {
                s.status = report.as_ref().map(|r| r.status);
                s.failed = s.status.is_none();
                s.disconnect_failed = false;
                if job.action == WifiAction::Add {
                    s.add_failed = report.is_none();
                }
                if job.action == WifiAction::Delete && report.is_none() {
                    // (the exit says which step failed: 21 the radio, 23 no read back, 24 no netid,
                    // 25 wifid kept the profile after the netid write)
                    qlog!("wifi: forgetting the network failed (helper exit %d)", code.unwrap_or(-1));
                    s.delete_failed = true;
                }
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
            }
        }
        if report.is_some() {
            s.add_failed = false;
            s.delete_failed = false;
        }
        // A profile that took is watched like a switch: the sheet says it is connecting, and names
        // it failed when no connection to it comes (a refusal already failed above). A profile that
        // is not among the saved networks afterwards did not stick: that is a failed add, however
        // happily the helper exited. EAP and WPA3-Personal are the supplicant's profiles, not wifid's,
        // so they are not expected in that list.
        if job.action == WifiAction::Add {
            if let Some(r) = &report {
                if let Some(added) = &job.added {
                    if !job.supplicant && matches!(&r.saved, Some(Some(list)) if !list.iter().any(|x| x == added)) {
                        s.add_failed = true;
                        qlog!("wifi: the added network is not among the saved networks (helper exit %d)", code.unwrap_or(-1));
                    } else if r.status.connected != Some(true)
                        || r.current.as_ref().is_none_or(|c| c.as_deref() != Some(&added[..]))
                    {
                        s.join = Some(Joining { essid: added.clone(), start: Instant::now(), phase: WifiJoin::Pending });
                    }
                }
            } else {
                qlog!("wifi: adding the network failed (helper exit %d)", code.unwrap_or(-1));
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
        s.quiet_scan = false;
        s.add_failed = false;
        s.delete_failed = false;
        if let Some(mut a) = s.add.take() {
            a.wipe();
        }
        s.delete = None;
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
    fn a_scan_lists_networks_strongest_first_and_fails_closed() {
        let home = hex(b"Home");
        let cafe = hex("Caf\u{e9}".as_bytes());
        let r = parse(
            format!(
                "wireless=1\nenabled=1\nconnected=1\ncurrent={home}\nscan=ok\nnear={home},3,5,wpa2,1\nnear={cafe},5,5,wpa3,0\nnear={home},5,5,wpa2,1\nnear_end=1\n"
            )
            .as_bytes(),
        )
        .unwrap();
        let Some(ScanReport::List(n)) = r.scan else { panic!("scan") };
        assert_eq!((n[0].essid.as_slice(), n[0].security, n[0].signal, n[0].known), ("Caf\u{e9}".as_bytes(), WifiSecurity::Wpa3, 5, false));
        assert_eq!((n[1].essid.as_slice(), n[1].security, n[1].signal, n[1].known), (b"Home".as_slice(), WifiSecurity::Wpa2, 5, true));
        assert_eq!(n.len(), 2, "one name, its strongest BSSID");
        assert_eq!(r.current, Some(Some(b"Home".to_vec())));
        let off = parse(b"wireless=0\nenabled=0\nconnected=0\nscan=off\n").unwrap();
        assert_eq!(off.scan, Some(ScanReport::Off));
        let unknown = parse(b"wireless=1\nenabled=1\nconnected=0\nscan=ok\nnear=486f6d65,1,5,whatever,0\nnear_end=1\n").unwrap();
        assert_eq!(
            unknown.scan,
            Some(ScanReport::List(vec![WifiNetwork {
                essid: b"Home".to_vec(),
                security: WifiSecurity::Unknown,
                signal: 1,
                signal_max: 5,
                known: false,
            }]))
        );
        // WPA3-Enterprise 192-bit is named apart so the sheet can keep it gated.
        let ent192 = parse(b"wireless=1\nenabled=1\nconnected=0\nscan=ok\nnear=576f726b,1,5,enterprise192,0\nnear_end=1\n").unwrap();
        let Some(ScanReport::List(n)) = ent192.scan else { panic!("scan") };
        assert_eq!(n[0].security, WifiSecurity::Enterprise192);
        let head = "wireless=1\nenabled=1\nconnected=0\n";
        let many: String = (0..65).map(|i| format!("near={:02x}41,1,5,open,0\n", i + 0x30)).collect();
        for bad in [
            format!("{head}scan=ok\nnear=486f6d65,1,5,open,0\n"),
            format!("{head}scan=ok\nnear=486f6d65,1,5,open,0\nnear_end=1\nnear=486f6d65,1,5,open,0\n"),
            format!("{head}scan=off\nnear=486f6d65,1,5,open,0\n"),
            format!("{head}scan=ok\nscan=ok\nnear_end=1\n"),
            format!("{head}scan=ok\nnear=486f6d65,0,0,open,0\nnear_end=1\n"),
            format!("{head}scan=ok\nnear=486f6d65,1,5,open,2\nnear_end=1\n"),
            format!("{head}scan=ok\nnear=486f6d65,1,5,open\nnear_end=1\n"),
            format!("{head}scan=ok\nnear=486f6d65,1,5,open,0,extra\nnear_end=1\n"),
            format!("{head}scan=ok\n{}", many),
        ] {
            assert!(parse(bad.as_bytes()).is_none(), "{bad}");
        }
    }

    #[test]
    fn a_change_cancels_a_running_scan_and_starts_at_once() {
        use std::fs;
        let root = std::env::temp_dir().join(format!("wifi-cancel-scan-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let script = root.join("helper.sh");
        let started = root.join("scan-started");
        let calls = root.join("calls");
        fs::write(
            &script,
            format!(
                r#"case $1 in
 scan) touch '{}'; sleep 30;;
 join) echo join >> '{}'; printf 'wireless=1\nenabled=1\nconnected=1\ncurrent=486f6d65\n';;
esac
"#,
                started.display(),
                calls.display()
            ),
        )
        .unwrap();
        wifi_init(script.to_str());
        assert!(wifi_request(WifiAction::Scan));
        let began = Instant::now();
        while !started.exists() && began.elapsed() < Duration::from_secs(5) {
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(started.exists(), "the slow scan started");
        assert!(wifi_join(b"Home"), "a change is accepted while a scan runs");
        let began = Instant::now();
        while (wifi_busy() || wifi_changing()) && began.elapsed() < Duration::from_secs(5) {
            wifi_tick();
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(fs::read_to_string(&calls).unwrap_or_default().contains("join"));
        assert_eq!(wifi_joining().map(|j| j.1), Some(WifiJoin::Connected));
        wifi_free();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_quiet_scan_keeps_the_order_updates_strengths_and_adds_only_new_names() {
        let net = |name: &[u8], signal: i32| WifiNetwork {
            essid: name.to_vec(),
            security: WifiSecurity::Wpa2,
            signal,
            signal_max: 5,
            known: true,
        };
        let merged = merge_nearby(
            vec![net(b"Home", 3), net(b"Cafe", 2), net(b"Old", 1)],
            vec![net(b"New", 5), net(b"Cafe", 4), net(b"Home", 2)],
        );
        assert_eq!(
            merged.iter().map(|n| n.essid.as_slice()).collect::<Vec<_>>(),
            [b"Home".as_slice(), b"Cafe".as_slice(), b"New".as_slice()]
        );
        assert_eq!((merged[0].signal, merged[1].signal, merged[2].signal), (2, 4, 5));
    }

    #[test]
    fn a_failed_add_is_named_apart_and_a_later_read_clears_it() {
        use std::fs;
        let root = std::env::temp_dir().join(format!("wifi-add-failed-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let script = root.join("helper.sh");
        fs::write(
            &script,
            "case $1 in add) cat >/dev/null; exit 1;; networks) printf 'wireless=1\\nenabled=1\\nconnected=0\\ncurrent=\\nlist=1\\n';; esac\n",
        )
        .unwrap();
        let settle = || {
            let began = Instant::now();
            while (wifi_busy() || wifi_changing()) && began.elapsed() < Duration::from_secs(5) {
                wifi_tick();
                std::thread::sleep(Duration::from_millis(5));
            }
        };
        wifi_init(script.to_str());
        assert!(wifi_add(b"Hidden", b"", WifiSecurity::Open));
        settle();
        assert!(wifi_failed());
        assert!(wifi_add_failed());
        assert!(wifi_request(WifiAction::Networks));
        settle();
        assert!(!wifi_failed());
        assert!(!wifi_add_failed());
        wifi_free();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_failed_disconnect_keeps_the_connection_and_is_named_apart() {
        use std::fs;
        let root = std::env::temp_dir().join(format!("wifi-disconnect-failed-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let script = root.join("helper.sh");
        fs::write(
            &script,
            "case $1 in\n networks) printf 'wireless=1\\nenabled=1\\nconnected=1\\ncurrent=486f6d65\\nsaved=486f6d65\\nlist=1\\n';;\n disconnect) printf 'wireless=1\\nenabled=1\\nconnected=1\\ncurrent=486f6d65\\n'; exit 12;;\n esac\n",
        )
        .unwrap();
        let settle = || {
            let began = Instant::now();
            while (wifi_busy() || wifi_changing()) && began.elapsed() < Duration::from_secs(5) {
                wifi_tick();
                std::thread::sleep(Duration::from_millis(5));
            }
        };
        wifi_init(script.to_str());
        assert!(wifi_request(WifiAction::Networks));
        settle();
        assert_eq!(wifi_status().unwrap().connected, Some(true));
        assert_eq!(wifi_current().as_deref(), Some(b"Home".as_slice()));
        assert!(wifi_disconnect());
        settle();
        assert!(wifi_disconnect_failed());
        assert!(!wifi_failed());
        assert_eq!(wifi_status().unwrap().connected, Some(true));
        assert_eq!(wifi_current().as_deref(), Some(b"Home".as_slice()));
        assert!(wifi_request(WifiAction::Networks));
        settle();
        assert!(!wifi_disconnect_failed());
        wifi_free();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn adding_sends_only_the_psk_on_stdin_and_deleting_names_the_network() {
        use std::fs;
        let root = std::env::temp_dir().join(format!("wifi-add-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let (script, args, pass) = (root.join("helper.sh"), root.join("args"), root.join("pass"));
        fs::write(
            &script,
            format!(
                r#"echo "$*" >> '{args}'
case $1 in
 add) cat > '{pass}'; printf 'wireless=1\nenabled=1\nconnected=0\ncurrent=\nsaved=486f6d65\nsaved=4e6577206e6574\nlist=1\n';;
 *) printf 'wireless=1\nenabled=1\nconnected=1\ncurrent=486f6d65\nsaved=486f6d65\nlist=1\n';;
esac
"#,
                args = args.display(),
                pass = pass.display()
            ),
        )
        .unwrap();
        let settle = || {
            let began = Instant::now();
            while (wifi_busy() || wifi_changing()) && began.elapsed() < Duration::from_secs(5) {
                wifi_tick();
                std::thread::sleep(Duration::from_millis(5));
            }
        };
        wifi_init(script.to_str());
        let secret = b"not in argv or logs";
        assert!(wifi_add(b"New net", secret, WifiSecurity::Wpa2));
        settle();
        // PBKDF2-HMAC-SHA1("not in argv or logs", "New net", 4096, 32), as wpa_passphrase prints it.
        assert_eq!(
            fs::read(&pass).unwrap(),
            [b"77d17789c632f0e384f7eb9015c14e6e4ae5ecbe917b56610714dfe5f7c4bc2f".as_slice(), b"\n"].concat()
        );
        assert!(!fs::read(&pass).unwrap().windows(secret.len()).any(|w| w == secret));
        assert_eq!(fs::read_to_string(&args).unwrap().trim(), "add New net wpa2");
        assert!(!fs::read_to_string(&args).unwrap().contains("not in argv"));
        assert_eq!(wifi_saved(), Some(vec![b"Home".to_vec(), b"New net".to_vec()]));
        // The profile took but the connection is not there yet: the sheet watches it like a switch.
        assert!(!wifi_add_failed());
        assert_eq!(wifi_joining(), Some((b"New net".to_vec(), WifiJoin::Pending)));
        assert!(wifi_delete(b"Home"));
        settle();
        assert_eq!(fs::read_to_string(&args).unwrap().lines().last(), Some("delete Home"));
        assert!(!wifi_add(b"New", b"short", WifiSecurity::Wpa2));
        assert!(!wifi_add(b"New", b"", WifiSecurity::Wpa3));
        wifi_free();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_wpa3_add_sends_the_passphrase_only_on_stdin_and_does_not_need_a_saved_profile() {
        use std::fs;
        let root = std::env::temp_dir().join(format!("wifi-add-wpa3-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let (script, args, input) = (root.join("helper.sh"), root.join("args"), root.join("input"));
        fs::write(
            &script,
            format!(
                r#"echo "$*" >> '{args}'
case $1 in
 add) cat > '{input}'; printf 'wireless=1\nenabled=1\nconnected=1\ncurrent=486f6d652033\nlist=1\n';;
 *) printf 'wireless=1\nenabled=1\nconnected=0\ncurrent=\nlist=1\n';;
esac
"#,
                args = args.display(),
                input = input.display()
            ),
        )
        .unwrap();
        let settle = || {
            let began = Instant::now();
            while (wifi_busy() || wifi_changing()) && began.elapsed() < Duration::from_secs(5) {
                wifi_tick();
                std::thread::sleep(Duration::from_millis(5));
            }
        };
        wifi_init(script.to_str());
        let secret = b"correct horse wifi";
        assert!(wifi_add(b"Home 3", secret, WifiSecurity::Wpa3));
        settle();
        assert_eq!(fs::read_to_string(&args).unwrap().trim(), "add Home 3 wpa3");
        assert_eq!(fs::read(&input).unwrap(), b"correct horse wifi\n");
        assert!(!fs::read_to_string(&args).unwrap().contains("correct horse"));
        // SAE takes the passphrase itself, not the PBKDF2 PSK the helper's WPA path would use.
        assert_ne!(
            fs::read(&input).unwrap(),
            [crate::wpa::psk_hex(secret, b"Home 3").as_slice(), b"\n"].concat()
        );
        assert!(!wifi_add_failed());
        assert_eq!(wifi_current().as_deref(), Some(b"Home 3".as_slice()));
        // A passphrase SAE cannot take never reaches the helper.
        assert!(!wifi_add(b"Home 3", b"short", WifiSecurity::Wpa3));
        assert!(!wifi_add(b"Home 3", &[b'x'; 64], WifiSecurity::Wpa3));
        assert!(!wifi_add(b"Home 3", b"line\nbreak", WifiSecurity::Wpa3));
        wifi_free();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn an_eap_add_sends_only_credentials_on_stdin_and_does_not_need_a_saved_profile() {
        use std::fs;
        let root = std::env::temp_dir().join(format!("wifi-add-eap-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let (script, args, input) = (root.join("helper.sh"), root.join("args"), root.join("input"));
        fs::write(
            &script,
            format!(
                r#"echo "$*" >> '{args}'
case $1 in
 add) cat > '{input}'; printf 'wireless=1\nenabled=1\nconnected=1\ncurrent=5363686f6f6c\nlist=1\n';;
 *) printf 'wireless=1\nenabled=1\nconnected=0\ncurrent=\nlist=1\n';;
esac
"#,
                args = args.display(),
                input = input.display()
            ),
        )
        .unwrap();
        let settle = || {
            let began = Instant::now();
            while (wifi_busy() || wifi_changing()) && began.elapsed() < Duration::from_secs(5) {
                wifi_tick();
                std::thread::sleep(Duration::from_millis(5));
            }
        };
        wifi_init(script.to_str());
        let user = b"student@example.edu";
        let secret = b"school secret";
        assert!(wifi_add_eap(b"School", user, secret));
        settle();
        assert_eq!(fs::read_to_string(&args).unwrap().trim(), "add School eap");
        assert_eq!(fs::read(&input).unwrap(), b"student@example.edu\nschool secret\n");
        assert!(!fs::read_to_string(&args).unwrap().contains("student"));
        assert!(!wifi_add_failed());
        assert_eq!(wifi_current().as_deref(), Some(b"School".as_slice()));
        assert_eq!(wifi_joining(), None);
        // A value the helper cannot take never reaches it.
        assert!(!wifi_add_eap(b"School", b"", secret));
        assert!(!wifi_add_eap(b"School", user, b""));
        assert!(!wifi_add_eap(b"School", b"user\n", secret));
        assert!(!wifi_add_eap(b"School", user, b"secret\n"));
        wifi_free();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn an_add_whose_profile_does_not_stick_fails_apart() {
        use std::fs;
        let root = std::env::temp_dir().join(format!("wifi-add-missing-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let script = root.join("helper.sh");
        fs::write(
            &script,
            "case $1 in\n add) cat >/dev/null;;\n esac\nprintf 'wireless=1\\nenabled=1\\nconnected=0\\ncurrent=\\nsaved=486f6d65\\nlist=1\\n'\n",
        )
        .unwrap();
        let settle = || {
            let began = Instant::now();
            while (wifi_busy() || wifi_changing()) && began.elapsed() < Duration::from_secs(5) {
                wifi_tick();
                std::thread::sleep(Duration::from_millis(5));
            }
        };
        wifi_init(script.to_str());
        assert!(wifi_add(b"New net", b"correct horse", WifiSecurity::Wpa2));
        settle();
        // The helper exited happily but the profile is not among the saved networks: a failed add,
        // and no connection to watch for.
        assert!(!wifi_failed());
        assert!(wifi_add_failed());
        assert_eq!(wifi_joining(), None);
        wifi_free();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn a_failed_forget_is_named_and_a_later_read_clears_it() {
        use std::fs;
        let root = std::env::temp_dir().join(format!("wifi-delete-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let script = root.join("helper.sh");
        fs::write(
            &script,
            r#"case $1 in
 delete) exit 1;;
esac
printf 'wireless=1\nenabled=1\nconnected=0\ncurrent=\nsaved=486f6d65\nlist=1\n'
"#,
        )
        .unwrap();
        let settle = || {
            let began = Instant::now();
            while (wifi_busy() || wifi_changing()) && began.elapsed() < Duration::from_secs(5) {
                wifi_tick();
                std::thread::sleep(Duration::from_millis(5));
            }
        };
        wifi_init(script.to_str());
        assert!(wifi_delete(b"Home"));
        settle();
        // The helper refused the delete: the sheet names that rather than the generic read error.
        assert!(wifi_delete_failed());
        assert!(wifi_failed());
        // Any read that comes back clears it.
        assert!(wifi_request(WifiAction::Networks));
        settle();
        assert!(!wifi_delete_failed());
        assert_eq!(wifi_saved(), Some(vec![b"Home".to_vec()]));
        wifi_free();
        fs::remove_dir_all(root).unwrap();
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
