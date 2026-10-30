//! Nonblocking access to the launcher's Kindle Wi-Fi helper. Only explicit UI actions change
//! the radio; queries run only while Share is visible. No network names or credentials are read.
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
}
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
    job: Option<Job>,
    status: Option<WifiStatus>,
    failed: bool,
    queued: Option<WifiAction>,
}
thread_local! {static STATE:RefCell<State>=RefCell::new(State::default());}

pub fn wifi_init(script: Option<&str>) {
    wifi_free();
    STATE.with(|s| {
        *s.borrow_mut() = State {
            script: script.unwrap_or("").to_string(),
            ..State::default()
        }
    });
}
pub fn wifi_available() -> bool {
    STATE.with(|s| std::fs::metadata(&s.borrow().script).is_ok_and(|m| m.is_file()))
}
pub fn wifi_changing() -> bool {
    STATE.with(|s| {
        let s = s.borrow();
        s.queued.is_some()
            || s.job
                .as_ref()
                .is_some_and(|j| j.action != WifiAction::Status)
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
pub fn wifi_request(action: WifiAction) -> bool {
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        if s.script.is_empty() {
            return false;
        }
        if let Some(job) = &s.job {
            if job.action == WifiAction::Status
                && action != WifiAction::Status
                && s.queued.is_none()
            {
                s.queued = Some(action);
                return true;
            }
            return false;
        }
        let verb = match action {
            WifiAction::Status => "status",
            WifiAction::On => "on",
            WifiAction::Off => "off",
        };
        let mut command = Command::new("/bin/sh");
        command
            .arg(&s.script)
            .arg(verb)
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
            let _ = child.kill();
            let _ = child.wait();
            s.failed = true;
            return false;
        }
        s.failed = false;
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
fn parse(bytes: &[u8]) -> Option<WifiStatus> {
    let text = std::str::from_utf8(bytes).ok()?;
    let (mut w, mut e, mut c) = (None, None, None);
    for line in text.lines() {
        match line {
            "wireless=0" if w.is_none() => w = Some(false),
            "wireless=1" if w.is_none() => w = Some(true),
            "enabled=0" if e.is_none() => e = Some(false),
            "enabled=1" if e.is_none() => e = Some(true),
            "connected=0" if c.is_none() => c = Some(Some(false)),
            "connected=1" if c.is_none() => c = Some(Some(true)),
            "connected=unknown" if c.is_none() => c = Some(None),
            _ => return None,
        }
    }
    let (wireless, enabled, connected) = (w?, e?, c?);
    if connected == Some(true) && !(wireless && enabled) {
        return None;
    }
    Some(WifiStatus {
        wireless,
        enabled,
        connected,
    })
}
fn drain(job: &mut Job) {
    let mut buf = [0; 256];
    loop {
        match job.child.stdout.as_mut().unwrap().read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                if job.output.len() + n > 512 {
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
/// Reap a bounded helper. A stalled helper and its descendants are killed together after 12 s.
pub fn wifi_tick() -> bool {
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
        s.status = if success && !job.failed {
            parse(&job.output)
        } else {
            None
        };
        s.failed = s.status.is_none();
        if s.failed {
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
    changed || next.is_some()
}
pub fn wifi_free() {
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        s.queued = None;
        if let Some(mut job) = s.job.take() {
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
                .connected,
            Some(true)
        );
        assert_eq!(
            parse(b"wireless=1\nenabled=1\nconnected=unknown\n")
                .unwrap()
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
}
