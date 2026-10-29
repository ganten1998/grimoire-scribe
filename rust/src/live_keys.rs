//! Live view has its own keyboard map and a bounded main-loop mailbox. No input hardware is opened.
use crate::prelude::*;
use std::collections::VecDeque;

pub const LIVE_KEY_QUEUE: usize = 32;
pub const LIVE_KEY_LEASE_MS: u32 = 1500;
fn ms_since(now: u32, before: u32) -> u32 { now.wrapping_sub(before) }
const MAX_HELD: usize = 16;
const MAX_AGE_MS: u32 = 500;

#[derive(Clone, Debug)]
pub struct LiveKey {
    pub code: i32,
    pub mods: i32,
    pub down: bool,
    pub cookie: String,
    at: u32,
}
struct Held {
    key: LiveKey,
    refreshed: u32,
}
#[derive(Default)]
pub(crate) struct LiveKeys {
    pub map: Shortcuts,
    events: VecDeque<LiveKey>,
    held: Vec<Held>,
    credit: u32,
    clock: u32,
}

/// Built-in keyboard defaults only, independent of the owner's physical keyboard/controller map.
/// Reuse the existing parser and its built-in defaults so their actions cannot drift apart.
pub fn live_keys_load(path: &str) -> Shortcuts {
    if !std::path::Path::new(path).exists() {
        let mut defaults = Shortcuts::default();
        shortcuts_load(&mut defaults, path);
        let mut text = String::from("# Live view keyboard. Reload by stopping and starting Live view.\n");
        for b in defaults.bindings.iter().take(defaults.count as usize).filter(|b| b.src == ActionSrc::Key) {
            text.push_str(&format!("key.{}{}{}{}{}={}\n",
                if b.mods & KEYMOD_CTRL != 0 { "Ctrl+" } else { "" },
                if b.mods & KEYMOD_SHIFT != 0 { "Shift+" } else { "" },
                if b.mods & KEYMOD_ALT != 0 { "Alt+" } else { "" },
                if b.mods & KEYMOD_META != 0 { "Meta+" } else { "" },
                live_key_name(b.code), shortcut_action_name(b.action).unwrap_or("")));
        }
        // create_new preserves an existing file, including an intentionally empty one.
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new().write(true).create_new(true).open(path) {
            if f.write_all(text.as_bytes()).is_err() { qlog!("live: could not write keyboard defaults"); }
        }
    }
    let mut map = Shortcuts::default();
    shortcuts_load(&mut map, path);
    let keys: Vec<_> = map.bindings.iter().take(map.count as usize).filter(|b| b.src == ActionSrc::Key).copied().collect();
    map.bindings.fill(ShortcutBinding::default());
    map.bindings[..keys.len()].copy_from_slice(&keys);
    map.count = keys.len() as i32;
    map
}

pub fn live_key_name(code: i32) -> String {
    for name in ["1", "2", "3", "4", "5", "6", "7", "8", "9", "0", "q", "w", "e", "r", "t", "y", "u", "i", "o", "p",
        "a", "s", "d", "f", "g", "h", "j", "k", "l", "z", "x", "c", "v", "b", "n", "m", "Left", "Right", "Up", "Down",
        "PageUp", "PageDown", "Space", "Enter", "Tab", "Escape", "VolumeUp", "VolumeDown", "PlayPause", "NextSong", "PreviousSong", "Home", "End", "Back",
    ] {
        if shortcut_keycode(name) == code { return name.to_owned(); }
    }
    format!("code{code}")
}

impl LiveKeys {
    pub fn reset(&mut self, map: Shortcuts) {
        *self = Self { map, credit: 16000, clock: now_ms(), ..Self::default() };
    }
    pub fn is_held(&self, key: &LiveKey, now: u32) -> bool {
        pair_cookie_scopes(&key.cookie) & PAIR_LIVE != 0 && self.held.iter().any(|h|
            {
                h.key.code == key.code && h.key.cookie == key.cookie && ms_since(now, h.refreshed) < LIVE_KEY_LEASE_MS})
    }
    /// phase 0 release, 1 press, 2 held-eraser lease renewal (never dispatches another action).
    pub fn submit(&mut self, cookie: String, code: i32, mods: i32, phase: i32, now: u32) -> i32 {
        if pair_cookie_scopes(&cookie) & PAIR_LIVE == 0 { return 401; }
        if !(1..=767).contains(&code) || !(0..=15).contains(&mods) || !(0..=2).contains(&phase) { return 400; }
        self.held.retain(|h| ms_since(now, h.refreshed) < LIVE_KEY_LEASE_MS);
        let held = self.held.iter().position(|h| h.key.code == code && h.key.cookie == cookie);
        if phase == 2 {
            if let Some(i) = held {
                if shortcuts_lookup(&self.map, ActionSrc::Key, code, self.held[i].key.mods) != SHORTCUT_HOLD_ERASER { return 400; }
                if ms_since(now, self.held[i].refreshed) < 200 { return 429; }
                self.held[i].refreshed = now;
                return 200;
            }
            return 409;
        }
        if phase == 0 {
            if let Some(i) = held { self.held.remove(i); }
        } else {
            if held.is_some() { return 409; } // repeat, even from a client ignoring KeyboardEvent.repeat
            if shortcuts_lookup(&self.map, ActionSrc::Key, code, mods) == SHORTCUT_NONE { return 400; }
            self.credit = (self.credit + ms_since(now, self.clock).min(1000) * 20).min(16000);
            self.clock = now;
            if self.credit < 1000 || self.held.len() >= MAX_HELD { return 429; }
        }
        self.events.retain(|k| ms_since(now, k.at) <= MAX_AGE_MS);
        if self.events.len() >= LIVE_KEY_QUEUE { return 429; }
        let key = LiveKey { code, mods, down: phase == 1, cookie, at: now ,
        };
        if key.down {
            self.credit -= 1000;
            self.held.push(Held { key: key.clone(), refreshed: now ,
            });
        }
        self.events.push_back(key);
        200
    }
    pub fn pop(&mut self, now: u32) -> Option<LiveKey> {
        while let Some(key) = self.events.pop_front() {
            if ms_since(now, key.at) <= MAX_AGE_MS && pair_cookie_scopes(&key.cookie) & PAIR_LIVE != 0 { return Some(key); }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn session(scope: u32) -> String {
        assert_eq!(pair_begin(scope, 0), PAIR_OK);
        let mut token = String::new();
        assert_eq!(pair_redeem(&pair_code(), 0, &mut token), PAIR_OK);
        format!("grimoire_session={token}")
    }
    fn queue() -> LiveKeys {
        pair_reset();
        let mut map = Shortcuts::default();
        shortcuts_load(&mut map, "/nonexistent/live-key-test.ini");
        let mut keys = LiveKeys::default();
        keys.reset(map);
        keys.clock = 0;
        keys
    }
    #[test]
    fn live_keys_auth_bounds_repeats_and_revocation() {
        let mut q = queue();
        assert_eq!(q.submit(String::new(), 2, 0, 1, 0), 401);
        let send = session(PAIR_SEND);
        assert_eq!(q.submit(send, 2, 0, 1, 0), 401);
        let cookie = session(PAIR_LIVE);
        for (code, mods, phase) in [(0, 0, 1), (768, 0, 1), (2, 16, 1), (2, 0, 3), (2, 1, 1)] {
            assert_eq!(q.submit(cookie.clone(), code, mods, phase, 0), 400);
        }
        assert_eq!(q.submit(cookie.clone(), 2, 0, 1, 0), 200);
        assert_eq!(q.submit(cookie.clone(), 2, 0, 1, 0), 409);
        assert_eq!(q.pop(0).unwrap().code, 2);
        assert_eq!(q.submit(cookie.clone(), 2, 0, 0, 0), 200);
        assert!(!q.pop(0).unwrap().down);
        assert_eq!(q.submit(cookie, 2, 0, 1, 0), 200);
        pair_forget(PAIR_LIVE);
        assert!(q.pop(0).is_none());
    }
    #[test]
    fn live_keys_queue_rate_age_and_reset() {
        let mut q = queue();
        let cookie = session(PAIR_LIVE);
        // Release traffic cannot consume unbounded memory, even though releases bypass the press limiter.
        for _ in 0..LIVE_KEY_QUEUE { assert_eq!(q.submit(cookie.clone(), 2, 0, 0, 0), 200); }
        assert_eq!(q.submit(cookie.clone(), 2, 0, 0, 0), 429);
        assert!(q.pop(501).is_none());
        for _ in 0..16 {
            assert_eq!(q.submit(cookie.clone(), 2, 0, 1, 1000), 200);
            assert_eq!(q.submit(cookie.clone(), 2, 0, 0, 1000), 200);
        }
        while q.pop(1000).is_some() {}
        assert_eq!(q.submit(cookie.clone(), 2, 0, 1, 1000), 429);
        assert_eq!(q.submit(cookie.clone(), 2, 0, 1, 1050), 200);
        q.reset(Shortcuts::default());
        assert!(q.pop(1050).is_none());
        // The age check works through the millisecond clock's wrap.
        q.events.push_back(LiveKey { code: 2, mods: 0, down: true, cookie, at: u32::MAX - 20 ,
        });
        assert!(q.pop(30).is_some());
    }
    #[test]
    fn live_keys_eraser_lease_release_and_lost_release() {
        let mut q = queue();
        q.map.bindings[0].action = SHORTCUT_HOLD_ERASER;
        let cookie = session(PAIR_LIVE);
        assert_eq!(q.submit(cookie.clone(), 2, 0, 1, 0), 200);
        let key = q.pop(0).unwrap();
        assert!(q.is_held(&key, 1000));
        assert_eq!(q.submit(cookie.clone(), 2, 0, 2, 500), 200);
        assert!(q.pop(500).is_none()); // a renewal must never dispatch an action again
        assert_eq!(q.submit(cookie.clone(), 2, 0, 2, 501), 429);
        assert!(q.is_held(&key, 1600));
        assert!(!q.is_held(&key, 2000));
        assert_eq!(q.submit(cookie.clone(), 2, 0, 0, 1000), 200);
        assert!(!q.is_held(&key, 1000));
        assert_eq!(q.submit(cookie.clone(), 2, 0, 1, 1100), 200);
        let key = q.pop(1100).unwrap(); // the release (also tests FIFO)
        assert!(!key.down);
        let key = q.pop(1100).unwrap();
        pair_forget(PAIR_LIVE);
        assert!(!q.is_held(&key, 1100));
    }
    #[test]
    fn live_keys_file_defaults_and_independence() {
        let dir = std::env::temp_dir().join(format!("grimoire-live-keys-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("live-keys.ini");
        let path = path.to_str().unwrap();
        let keys = live_keys_load(path);
        let text = std::fs::read_to_string(path).unwrap();
        assert!(text.contains("key.Ctrl+z=undo\n") && !text.contains("pad."));
        let mut defaults = Shortcuts::default();
        shortcuts_load(&mut defaults, "/nonexistent/defaults-test.ini");
        assert_eq!(keys.bindings[..keys.count as usize], defaults.bindings[..keys.count as usize]);
        std::fs::write(dir.join("shortcuts.ini"), b"key.1=redo\n").unwrap();
        assert_eq!(live_keys_load(path).bindings[0].action, SHORTCUT_TOOL_BASE);
        std::fs::write(path, b"key.1=redo\npad.304=undo\nkey.1=undo\n").unwrap();
        let custom = live_keys_load(path);
        assert_eq!(custom.count, 1);
        assert_eq!(custom.bindings[0].action, SHORTCUT_UNDO);
        std::fs::write(path, b"").unwrap();
        assert_eq!(live_keys_load(path).count, 0);
        assert_eq!(std::fs::read(path).unwrap(), b"");
        std::fs::write(path, vec![b'x'; 65537]).unwrap();
        let oversized = live_keys_load(path);
        assert_eq!((oversized.count, oversized.rejected), (0, 1));
        std::fs::remove_dir_all(dir).unwrap();
    }
}
