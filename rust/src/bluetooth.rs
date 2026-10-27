//! bluetooth.h: the Bluetooth helper (kindle-hid-passthrough; docs/SHORTCUTS.md, Bluetooth) as Grimoire
//! runs it. With Settings > Controller's Bluetooth on, Grimoire starts the helper after its first
//! frame (kindle/app/bluetooth.sh start: only one of its own, never someone else's) and stops it when
//! that is turned off; the launcher's restore stops it as Grimoire leaves, crash or not, and gives
//! the Kindle its own Bluetooth jobs back. Pairing goes through the helper's local API
//! (127.0.0.1:8321). Every script run and request is a forked child: bt_tick reaps them, so the UI
//! never waits. Names and addresses from the helper are made safe first (btapi.h).

use crate::prelude::*;
use std::cell::RefCell;
use std::ffi::CString;
use std::os::unix::fs::PermissionsExt;

const REPLY_MAX: usize = 64 * 1024;

#[repr(i32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BtState {
    /// not started by Grimoire (or stopped)
    #[default]
    Stopped = 0,  // BT_STOPPED
    Starting = 1,  // BT_STARTING
    /// started by Grimoire, answering
    Owned = 2,  // BT_OWNED
    /// another one runs (KOReader's, say): Grimoire uses it, never stops it
    External = 3,  // BT_EXTERNAL
    /// it would not start (bluetooth.sh's log says why)
    Failed = 4,  // BT_FAILED
    Stopping = 5,  // BT_STOPPING
}

#[repr(i32)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BtEvent {
    #[default]
    None = 0,  // BT_EV_NONE
    State = 1,  // BT_EV_STATE
    Reply = 2,  // BT_EV_REPLY
    Failed = 3,  // BT_EV_FAILED
}

// The script run and the request, each a child writing to a pipe.
#[derive(Default)]
struct Job {
    pid: libc::pid_t,
    fd: i32,
    buf: Vec<u8>, // REPLY_MAX + 1 bytes once a child is spawned (the answer, then a NUL)
    len: usize,
}

const JOB_SCRIPT: usize = 0;
const JOB_REQUEST: usize = 1;

struct State {
    script: FixedStr<512>,
    root: FixedStr<512>,
    host: FixedStr<64>,
    port: i32,
    /// there is an API to ask (the launcher's script, or a test's stand-in)
    api: bool,
    /// the API is a stand-in that runs already (tests): nothing to start or stop
    standin: bool,
    state: BtState,
    job: [Job; 2],
}

impl Default for State {
    fn default() -> State {
        State {
            script: FixedStr::default(),
            root: FixedStr::default(),
            host: FixedStr::from_str("127.0.0.1"),
            port: 8321,
            api: false,
            standin: false,
            state: BtState::Stopped,
            job: [Job::default(), Job::default()],
        }
    }
}

thread_local! {
    static S: RefCell<State> = RefCell::new(State::default());
}

fn with<R>(f: impl FnOnce(&mut State) -> R) -> R {
    S.with(|s| f(&mut s.borrow_mut()))
}

/// atoi(3): leading white space, an optional sign, then digits up to the first other character.
fn atoi(s: &str) -> i32 {
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() && (b[i] == b' ' || (9..=13).contains(&b[i])) {
        i += 1;
    }
    let mut neg = false;
    if i < b.len() && (b[i] == b'-' || b[i] == b'+') {
        neg = b[i] == b'-';
        i += 1;
    }
    let mut v: i32 = 0;
    while i < b.len() && b[i].is_ascii_digit() {
        v = v.wrapping_mul(10).wrapping_add((b[i] - b'0') as i32);
        i += 1;
    }
    if neg {
        v.wrapping_neg()
    } else {
        v
    }
}

/// root: Grimoire's data folder. script: kindle/app/bluetooth.sh (the launcher says where), or NULL.
/// standin: "host:port" of a stand-in helper that runs already (tests/fake_bthelper.py), or NULL.
pub fn bt_init(root: &str, script: Option<&str>, standin: Option<&str>) {
    with(|s| {
        s.root.set(root);
        s.script.set(script.unwrap_or(""));
        s.host.set("127.0.0.1");
        s.port = 8321;
        s.api = !s.script.is_empty();
        s.standin = false;
        s.state = BtState::Stopped;
        let colon = standin.and_then(|x| x.rfind(':').map(|c| (x, c)));
        if let Some((standin, colon)) = colon {
            if colon > 0 && colon < 64 {
                s.host.set(&standin[..colon]);
                s.port = atoi(&standin[colon + 1..]);
                s.api = true;
                s.standin = true;
                s.script.clear();
            }
        }
        for j in s.job.iter_mut() {
            if j.pid <= 0 {
                j.fd = -1;
            }
        }
    })
}

fn executable(path: &str) -> bool {
    match std::fs::metadata(path) {
        Ok(m) => m.is_file() && (m.permissions().mode() & 0o111) != 0,
        Err(_) => false,
    }
}

/// A helper Grimoire could start: unpacked in <root>/bluetooth/, or KHP's /mnt/us/kindle_hid_passthrough.
pub fn bt_present() -> bool {
    let (standin, script, root) = with(|s| (s.standin, s.script, s.root));
    if standin {
        return true;
    }
    if script.is_empty() {
        return false;
    }
    let path = cformat!("%s/bluetooth/kindle-hid-passthrough", root.as_str());
    executable(&path) || executable("/mnt/us/kindle_hid_passthrough/kindle-hid-passthrough")
}

pub fn bt_state() -> BtState {
    with(|s| s.state)
}

pub fn bt_busy() -> bool {
    with(|s| s.job[JOB_REQUEST].pid > 0)
}

/// A child for job j: its stdout (and the parent's end) a pipe. Returns the child's pid in the
/// child (0) and the parent, -1 when it could not fork.
fn spawn(j: usize) -> libc::pid_t {
    let mut p = [0i32; 2];
    // SAFETY: pipe writes the two descriptors into our array.
    if unsafe { libc::pipe(p.as_mut_ptr()) } != 0 {
        return -1;
    }
    // SAFETY: fork has no preconditions; the child below touches only descriptors and signal
    // dispositions, then returns to a caller that execs or _exits (run_script, request_child). No
    // borrow of the module's state is held here, so the child's copy of it is consistent.
    let pid = unsafe { libc::fork() };
    if pid < 0 {
        // SAFETY: closing the two descriptors pipe just made.
        unsafe {
            libc::close(p[0]);
            libc::close(p[1]);
        }
        return -1;
    }
    if pid == 0 {
        // SAFETY: in the child: dup2/close on descriptors it owns, and sigaction with a zeroed
        // (default) action for four signals.
        unsafe {
            libc::dup2(p[1], 1);
            for fd in 3..1024 {
                libc::close(fd);
            }
            let mut sa: libc::sigaction = std::mem::zeroed();
            sa.sa_sigaction = libc::SIG_DFL;
            libc::sigaction(libc::SIGTERM, &sa, std::ptr::null_mut());
            libc::sigaction(libc::SIGINT, &sa, std::ptr::null_mut());
            libc::sigaction(libc::SIGHUP, &sa, std::ptr::null_mut());
            libc::sigaction(libc::SIGPIPE, &sa, std::ptr::null_mut());
        }
        return 0;
    }
    // SAFETY: closing the write end the parent does not use; fcntl on the read end we own.
    unsafe {
        libc::close(p[1]);
        libc::fcntl(p[0], libc::F_SETFL, libc::fcntl(p[0], libc::F_GETFL, 0) | libc::O_NONBLOCK);
    }
    with(|s| {
        let job = &mut s.job[j];
        job.pid = pid;
        job.fd = p[0];
        job.len = 0;
        job.buf.clear();
        job.buf.resize(REPLY_MAX + 1, 0);
    });
    pid
}

fn run_script(verb: &str, during: BtState) {
    let (script, root, busy) = with(|s| (s.script, s.root, s.job[JOB_SCRIPT].pid > 0));
    if script.is_empty() || busy {
        return;
    }
    // (All the child needs is made before the fork.)
    let c_script = CString::new(script.as_bytes()).unwrap_or_default();
    let c_verb = CString::new(verb).unwrap_or_default();
    let mut env = b"GRIMOIRE_ROOT=".to_vec();
    env.extend_from_slice(root.as_bytes());
    let c_env = CString::new(env).unwrap_or_default();
    let pid = spawn(JOB_SCRIPT);
    if pid == 0 {
        // SAFETY: in the child: putenv keeps the pointer to c_env, which lives until the exec;
        // execl gets NUL-terminated strings and a NULL-terminated argument list; _exit ends the
        // child without the parent's exit handlers if the exec fails.
        unsafe {
            libc::putenv(c_env.as_ptr() as *mut libc::c_char);
            libc::execl(
                c"/bin/sh".as_ptr(),
                c"sh".as_ptr(),
                c_script.as_ptr(),
                c_verb.as_ptr(),
                std::ptr::null::<libc::c_char>(),
            );
            libc::_exit(127);
        }
    }
    if pid > 0 {
        with(|s| s.state = during);
    } else {
        qlog!("bluetooth: could not run the helper's script (%s)", &strerror(errno()));
    }
}

pub fn bt_start() {
    let (standin, state) = with(|s| (s.standin, s.state));
    if standin {
        // it runs already: nothing to start
        with(|s| s.state = BtState::External);
        return;
    }
    if state == BtState::Owned || state == BtState::External || state == BtState::Starting {
        return;
    }
    qlog!("bluetooth: starting the helper");
    run_script("start", BtState::Starting);
}

pub fn bt_stop() {
    let (standin, state) = with(|s| (s.standin, s.state));
    if state == BtState::External || standin {
        with(|s| s.state = BtState::Stopped); // (someone else's: left running)
        return;
    }
    if state == BtState::Stopped || state == BtState::Stopping {
        return;
    }
    qlog!("bluetooth: stopping the helper");
    run_script("stop-owned", BtState::Stopping);
}

/// The child of a request: a GET over loopback, its body to stdout. 4 s at most: the alarm ends it
/// (connect has no timeout of its own everywhere: on macOS a port bound but not yet listening leaves
/// it waiting, for as long as the helper takes to listen).
#[allow(deprecated)]
fn request_child(path: &str, host: FixedStr<64>, port: i32) -> ! {
    // SAFETY: this whole function runs in the forked child, on descriptors and buffers it owns:
    // each call gets valid pointers and lengths from locals, and every path ends in _exit.
    unsafe {
        libc::alarm(4);
        let fd = libc::socket(libc::AF_INET, libc::SOCK_STREAM, 0);
        if fd < 0 {
            libc::_exit(1);
        }
        let tv = libc::timeval { tv_sec: 3, tv_usec: 0 };
        let tvp = &tv as *const libc::timeval as *const libc::c_void;
        let tvl = std::mem::size_of::<libc::timeval>() as libc::socklen_t;
        libc::setsockopt(fd, libc::SOL_SOCKET, libc::SO_RCVTIMEO, tvp, tvl);
        libc::setsockopt(fd, libc::SOL_SOCKET, libc::SO_SNDTIMEO, tvp, tvl);
        let mut sa: libc::sockaddr_in = std::mem::zeroed();
        sa.sin_family = libc::AF_INET as libc::sa_family_t;
        sa.sin_port = (port as u16).to_be();
        // (inet_pton(AF_INET) is not in the libc crate: a dotted quad parses the same way.)
        let Ok(ip) = host.as_str().parse::<std::net::Ipv4Addr>() else {
            libc::_exit(1);
        };
        sa.sin_addr.s_addr = u32::from_ne_bytes(ip.octets());
        if libc::connect(
            fd,
            &sa as *const libc::sockaddr_in as *const libc::sockaddr,
            std::mem::size_of::<libc::sockaddr_in>() as libc::socklen_t,
        ) != 0
        {
            libc::_exit(1);
        }
        let req = cformat!("GET %s HTTP/1.0\r\nHost: %s\r\nConnection: close\r\n\r\n", path, host.as_str());
        let n = req.len();
        if n >= 700 || libc::send(fd, req.as_ptr() as *const libc::c_void, n, 0) != n as isize {
            libc::_exit(1);
        }
        let mut buf = vec![0u8; REPLY_MAX + 4096];
        let mut len = 0usize;
        loop {
            let r = libc::recv(fd, buf.as_mut_ptr().add(len) as *mut libc::c_void, buf.len() - len, 0);
            if r < 0 && errno() == libc::EINTR {
                continue;
            }
            if r <= 0 {
                break;
            }
            len += r as usize;
            if len == buf.len() {
                libc::_exit(1); // more than any answer it gives
            }
        }
        // (The C reads the answer as a string for these two tests: it ends at a NUL byte, and
        // the buffer past len is zero.)
        let text = &buf[..len];
        let head = &text[..text.iter().position(|&b| b == 0).unwrap_or(len)];
        let Some(gap) = text.windows(4).position(|w| w == b"\r\n\r\n") else {
            libc::_exit(1);
        };
        if !text.starts_with(b"HTTP/1.") || !head.windows(5).any(|w| w == b" 200 ") {
            libc::_exit(1);
        }
        let body = &text[gap + 4..];
        let blen = body.len();
        if blen > REPLY_MAX || libc::write(1, body.as_ptr() as *const libc::c_void, blen) != blen as isize {
            libc::_exit(1);
        }
        libc::_exit(0);
    }
}

/// A GET of path (with its query) to the helper's API: false while one is under way, or with no API.
pub fn bt_request(path: &str) -> bool {
    let (api, busy, host, port) = with(|s| (s.api, s.job[JOB_REQUEST].pid > 0, s.host, s.port));
    if !api || busy || !path.starts_with('/') || path.len() > 400 {
        return false;
    }
    for &c in path.as_bytes() {
        if c <= b' ' || c > b'~' {
            return false; // (one line, nothing to escape)
        }
    }
    let pid = spawn(JOB_REQUEST);
    if pid == 0 {
        request_child(path, host, port);
    }
    pid > 0
}

/// Reads what job j has written; true once it has ended (its status in *status).
fn finished(j: usize, status: &mut i32) -> bool {
    with(|s| {
        let job = &mut s.job[j];
        if job.pid <= 0 {
            return false;
        }
        loop {
            let room = REPLY_MAX - job.len;
            if room == 0 {
                break;
            }
            // SAFETY: read into the job's buffer, which has REPLY_MAX + 1 bytes: len + room is
            // REPLY_MAX at most.
            let r = unsafe { libc::read(job.fd, job.buf[job.len..].as_mut_ptr() as *mut libc::c_void, room) };
            if r > 0 {
                job.len += r as usize;
            } else {
                break;
            }
        }
        // SAFETY: waitpid writes the status into the caller's int.
        if unsafe { libc::waitpid(job.pid, status, libc::WNOHANG) } != job.pid {
            return false;
        }
        loop {
            // SAFETY: as above (a read of 0 bytes when the buffer is full ends the loop).
            let r = unsafe {
                libc::read(job.fd, job.buf[job.len..].as_mut_ptr() as *mut libc::c_void, REPLY_MAX - job.len)
            };
            if r > 0 {
                job.len += r as usize;
            } else {
                break;
            }
        }
        job.buf[job.len] = 0;
        // SAFETY: closing the read end this job owns.
        unsafe { libc::close(job.fd) };
        job.fd = -1;
        job.pid = 0;
        true
    })
}

/// A C string's bytes: up to the first NUL.
fn cstr(b: &[u8]) -> &[u8] {
    &b[..b.iter().position(|&c| c == 0).unwrap_or(b.len())]
}

/// strstr(hay, needle) != NULL
fn has(hay: &[u8], needle: &[u8]) -> bool {
    hay.windows(needle.len()).any(|w| w == needle)
}

/// Reaps what finished: BT_EV_STATE (bt_state changed), BT_EV_REPLY (*reply: the request's answer)
/// or BT_EV_FAILED (the request got none: the helper is gone or did not answer in 3 s).
pub fn bt_tick(reply: &mut BtReply) -> BtEvent {
    let mut status = 0;
    if finished(JOB_SCRIPT, &mut status) {
        // The script's last word: owned, external, stopped, missing or failed.
        let now = with(|s| {
            let job = &s.job[JOB_SCRIPT];
            let out = cstr(&job.buf[..job.len]);
            let was = s.state;
            s.state = if has(out, b"owned") {
                BtState::Owned
            } else if has(out, b"external") {
                BtState::External
            } else if has(out, b"stopped") {
                BtState::Stopped
            } else {
                BtState::Failed
            };
            if was == BtState::Stopping && s.state == BtState::Failed {
                s.state = BtState::Stopped;
            }
            s.state
        });
        qlog!(
            "bluetooth: the helper is %s",
            if now == BtState::Owned {
                "running (Grimoire's)"
            } else if now == BtState::External {
                "running (not Grimoire's)"
            } else if now == BtState::Stopped {
                "stopped"
            } else {
                "not running"
            }
        );
        return BtEvent::State;
    }
    if finished(JOB_REQUEST, &mut status) {
        let ok = libc::WIFEXITED(status) && libc::WEXITSTATUS(status) == 0;
        // (btapi_parse is pure: it reaches back into no module's state.)
        if ok && with(|s| btapi_parse(&s.job[JOB_REQUEST].buf[..s.job[JOB_REQUEST].len], reply)) == 0 {
            return BtEvent::Reply;
        }
        return BtEvent::Failed;
    }
    BtEvent::None
}
