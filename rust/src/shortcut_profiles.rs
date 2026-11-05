//! Per-device shortcuts (2.0.0-beta.2, Rust only; not in the C tree). The owner's ruling of 2026-11-04:
//! "I would like to see shortcuts as its own menu in the settings with per device profiles that are
//! saved." A keyboard or a controller (USB or Bluetooth) can have shortcuts of its own, in
//! root/shortcuts/<slug>.ini, used whenever it is connected; a device with none uses the default,
//! shortcuts.ini and controller.ini, as every device did before (docs/SHORTCUTS.md, Device profiles).
//!
//! A device is its name as Grimoire shows it (hidkeys_name) and EVIOCGID's vendor and product numbers.
//! Its Bluetooth address (EVIOCGUNIQ) is left out: nothing has shown the Kindle's helper reporting one,
//! let alone the same one at every connection, and a key that changed would lose a profile. So two
//! controllers of the same model and name share one profile.
//!
//! A profile is three files, side by side, named by the device's slug (shortcut_profile_slug):
//!   <slug>.ini             its shortcuts, in shortcuts.ini's grammar (key. and pad. lines), under a
//!                          header naming the device ("# Device:", "# Id:"); Settings > Shortcuts' Own
//!                          shortcuts for this device writes it once, a copy of the default, and never
//!                          again: the owner edits it as shortcuts.ini.
//!   <slug>.controller.ini  the buttons Settings > Shortcuts > Buttons learned for it, with
//!                          controller.ini's rules: while it is there, the profile's pad. lines are
//!                          set aside (shortcuts_load_all).
//!   <slug>.off             there while its own shortcuts are turned off: the device uses the default
//!                          again, and the profile is kept, to be used again when they are turned on.
//! Every write is atomic. Nothing here writes shortcuts.ini or controller.ini; they are only read, for
//! the copy a new profile starts from.

use crate::prelude::*;

/// At most this many profiles are read (the folder's first, by name): each holds a map in memory.
pub const PROFILES_MAX: usize = 32;
/// A slug's name part, at most (hidkeys_name keeps 40 bytes; the numbers come after it).
const SLUG_NAME_MAX: usize = 40;

#[derive(Clone, Debug)]
pub struct ShortcutProfile {
    /// The file's name without ".ini": shortcut_profile_slug of the device.
    pub slug: String,
    /// The device as the file's header names it ("# Device:"), else the slug.
    pub name: String,
    /// From the header's "# Id:" line, else the slug's end (0 where neither says).
    pub vendor: u16,
    pub product: u16,
    /// No <slug>.off beside it: the device uses these shortcuts.
    pub on: bool,
    /// Its shortcuts while on (shortcuts_load_all of <slug>.ini and <slug>.controller.ini, the default's
    /// rules: refused lines counted, the D-pad's starter lines for a file that binds none); empty while off.
    pub map: Shortcuts,
}

/// The folder of the profiles: root/shortcuts.
pub fn shortcut_profiles_dir() -> String {
    cformat!("%s/shortcuts", &store_root())
}

/// A device's file name without ".ini": its name in lower case, every run of other characters one '-'
/// (none at either end; "device" for a name with no letter or digit), then its vendor and product numbers
/// in hex: "8BitDo Pro 2" with 2dc8:6006 is "8bitdo-pro-2-2dc8-6006".
pub fn shortcut_profile_slug(name: &str, vendor: u16, product: u16) -> String {
    let mut out = String::new();
    for word in name.split(|c: char| !c.is_ascii_alphanumeric()).filter(|w| !w.is_empty()) {
        if !out.is_empty() {
            out.push('-');
        }
        out.push_str(&word.to_ascii_lowercase());
    }
    out.truncate(SLUG_NAME_MAX); // (ASCII: any byte is a boundary)
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        out.push_str("device");
    }
    out.push_str(&format!("-{vendor:04x}-{product:04x}"));
    out
}

/// root/shortcuts/<slug>.ini: the profile's shortcuts.
pub fn shortcut_profile_ini(dir: &str, slug: &str) -> String {
    format!("{dir}/{slug}.ini")
}
/// root/shortcuts/<slug>.controller.ini: the buttons learned for it (controller.ini's rules).
pub fn shortcut_profile_pads(dir: &str, slug: &str) -> String {
    format!("{dir}/{slug}.controller.ini")
}
/// root/shortcuts/<slug>.off: there while its own shortcuts are off.
fn profile_off(dir: &str, slug: &str) -> String {
    format!("{dir}/{slug}.off")
}

/// A file name the folder may hold as a profile: lower-case letters, digits and '-', as a slug is.
fn slug_ok(slug: &str) -> bool {
    !slug.is_empty() && slug.len() <= SLUG_NAME_MAX + 10 && slug.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
}

/// The vendor and product at a slug's end ("...-2dc8-6006"), else 0 and 0.
fn slug_ids(slug: &str) -> (u16, u16) {
    let mut parts = slug.rsplitn(3, '-');
    let (Some(p), Some(v), Some(_)) = (parts.next(), parts.next(), parts.next()) else { return (0, 0) };
    let hex = |x: &str| if x.len() == 4 { u16::from_str_radix(x, 16).ok() } else { None };
    match (hex(v), hex(p)) {
        (Some(v), Some(p)) => (v, p),
        _ => (0, 0),
    }
}

/// The header a new profile starts with: what it is and the device it is for.
fn profile_header(name: &str, vendor: u16, product: u16) -> String {
    format!(
        "# Grimoire: the shortcuts of one keyboard or controller, used while it is connected (Settings >\n\
         # Shortcuts, Own shortcuts for this device). The lines are shortcuts.ini's (docs/SHORTCUTS.md).\n\
         # Device: {name}\n\
         # Id: {vendor:04x}:{product:04x}\n"
    )
}

/// The device a profile's text names in its header ("# Device: NAME", "# Id: VVVV:PPPP"), each None
/// where it says nothing.
fn header_device(text: &[u8]) -> (Option<String>, Option<(u16, u16)>) {
    let (mut name, mut ids) = (None, None);
    for line in text.split(|&c| c == b'\n') {
        let line = String::from_utf8_lossy(line);
        let line = line.trim_end_matches('\r');
        if !line.starts_with('#') {
            if !line.trim().is_empty() {
                break; // (the header is the comments before the first line that is not one)
            }
            continue;
        }
        if let Some(n) = line.strip_prefix("# Device: ") {
            let mut n = n.trim().to_string();
            while n.len() > 64 {
                n.pop(); // (a header edited by hand: a name as long as a device's, near enough)
            }
            if !n.is_empty() {
                name.get_or_insert(n);
            }
        } else if let Some(id) = line.strip_prefix("# Id: ") {
            if let Some((v, p)) = id.trim().split_once(':') {
                if let (Ok(v), Ok(p)) = (u16::from_str_radix(v, 16), u16::from_str_radix(p, 16)) {
                    ids.get_or_insert((v, p));
                }
            }
        }
    }
    (name, ids)
}

/// The profiles in dir, by slug (at most PROFILES_MAX), each with its map read while it is on. A missing
/// folder has none.
pub fn shortcut_profiles_load(dir: &str) -> Vec<ShortcutProfile> {
    let Ok(d) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut slugs: Vec<String> = d
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_str()?.to_string();
            let slug = name.strip_suffix(".ini")?;
            if slug.ends_with(".controller") || !slug_ok(slug) {
                return None;
            }
            Some(slug.to_string())
        })
        .collect();
    slugs.sort();
    slugs.truncate(PROFILES_MAX);
    slugs.into_iter().map(|slug| shortcut_profile_read(dir, &slug)).collect()
}

/// One profile as its files are now.
pub fn shortcut_profile_read(dir: &str, slug: &str) -> ShortcutProfile {
    let ini = shortcut_profile_ini(dir, slug);
    let (name, ids) = match crate::shortcuts::ini_read_bounded(&ini, 65536) {
        Ok(Some(text)) => header_device(&text),
        _ => (None, None),
    };
    let (vendor, product) = ids.unwrap_or_else(|| slug_ids(slug));
    let on = std::fs::symlink_metadata(profile_off(dir, slug)).is_err();
    let mut map = Shortcuts::default();
    if on {
        shortcuts_load_all(&mut map, &ini, &shortcut_profile_pads(dir, slug));
    }
    ShortcutProfile { slug: slug.to_string(), name: name.unwrap_or_else(|| slug.to_string()), vendor, product, on, map }
}

/// The shortcuts a device uses while its own are on (its profile's), else None: the default. A key no
/// device sent (an empty name) has none.
pub fn shortcut_profile_for<'a>(profiles: &'a [ShortcutProfile], name: &str, vendor: u16, product: u16) -> Option<&'a Shortcuts> {
    if name.is_empty() || profiles.is_empty() {
        return None;
    }
    let slug = shortcut_profile_slug(name, vendor, product);
    profiles.iter().find(|p| p.on && p.slug == slug).map(|p| &p.map)
}

/// The device's own shortcuts turned on, from a copy of the default: shortcuts.ini's text (the starter map
/// where there is none) under the header naming the device, and controller.ini, where there is one, as its
/// <slug>.controller.ini; so it does what the default does until it is edited. An existing profile is never
/// written over: it is only turned on again. The default's files are only read. 0, or -1 when a file could
/// not be read or written (nothing is turned on then).
pub fn shortcut_profile_create(dir: &str, name: &str, vendor: u16, product: u16, shortcuts_path: &str, controller_path: &str) -> i32 {
    let slug = shortcut_profile_slug(name, vendor, product);
    let ini = shortcut_profile_ini(dir, &slug);
    if std::fs::symlink_metadata(&ini).is_ok() {
        return shortcut_profile_set_on(dir, &slug, true);
    }
    let text = match crate::shortcuts::ini_read_bounded(shortcuts_path, 65536) {
        Ok(Some(text)) => text,
        Ok(None) => shortcuts_starter().as_bytes().to_vec(),
        Err(()) => return -1,
    };
    let pads = if std::fs::symlink_metadata(controller_path).is_ok() {
        match crate::shortcuts::ini_read_bounded(controller_path, 65536) {
            Ok(text) => text,
            Err(()) => return -1,
        }
    } else {
        None
    };
    if std::fs::create_dir_all(dir).is_err() {
        return -1;
    }
    let pads_path = shortcut_profile_pads(dir, &slug);
    match pads {
        Some(p) => {
            if write_file_atomic(&pads_path, &p) != 0 {
                return -1;
            }
        }
        // (one left from a profile whose .ini was deleted would set the copy's buttons aside)
        None => {
            if let Err(e) = std::fs::remove_file(&pads_path) {
                if e.kind() != std::io::ErrorKind::NotFound {
                    return -1;
                }
            }
        }
    }
    let mut out = profile_header(name, vendor, product).into_bytes();
    out.extend_from_slice(&text);
    if write_file_atomic(&ini, &out) != 0 {
        return -1;
    }
    shortcut_profile_set_on(dir, &slug, true)
}

/// A profile's own shortcuts on (its .off marker removed) or off (the marker written; the profile itself
/// is kept, byte for byte). 0, or -1 when the marker could not be changed.
pub fn shortcut_profile_set_on(dir: &str, slug: &str, on: bool) -> i32 {
    let off = profile_off(dir, slug);
    if on {
        return match std::fs::remove_file(&off) {
            Ok(()) => 0,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => 0,
            Err(_) => -1,
        };
    }
    let text = "# Grimoire: this device's own shortcuts are off (Settings > Shortcuts); its .ini is kept.\n";
    write_file_atomic(&off, text.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> String {
        let dir = std::env::temp_dir().join(format!("grimoire-profiles-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir.to_str().unwrap().to_string()
    }
    fn read(path: &str) -> Vec<u8> {
        std::fs::read(path).unwrap()
    }
    const SOUTH: i32 = 304;
    const EAST: i32 = 305;
    const KEY_Z: i32 = 44;

    // The slug: the name in lower case with one '-' for each run of anything else, then the numbers.
    #[test]
    fn slugs() {
        assert_eq!(shortcut_profile_slug("8BitDo Pro 2", 0x2dc8, 0x6006), "8bitdo-pro-2-2dc8-6006");
        assert_eq!(shortcut_profile_slug("  Xbox  Wireless_Controller! ", 0x045e, 0x0b13), "xbox-wireless-controller-045e-0b13");
        assert_eq!(shortcut_profile_slug("?", 0, 0), "device-0000-0000");
        assert_eq!(shortcut_profile_slug("", 1, 2), "device-0001-0002");
        let long = shortcut_profile_slug(&"a".repeat(80), 0xffff, 0xffff);
        assert_eq!(long, format!("{}-ffff-ffff", "a".repeat(SLUG_NAME_MAX)));
        assert!(slug_ok(&long));
        assert_eq!(slug_ids("8bitdo-pro-2-2dc8-6006"), (0x2dc8, 0x6006));
        assert_eq!(slug_ids("my-keyboard"), (0, 0));
    }

    // A profile made from the default is a copy of it under a header naming the device; read back it has
    // the same shortcuts as the default, and the default's own files are left byte for byte as they were.
    #[test]
    fn a_profile_starts_as_a_copy_of_the_default() {
        let root = tmp("copy");
        let sp = format!("{root}/shortcuts.ini");
        let cp = format!("{root}/controller.ini");
        let dir = format!("{root}/shortcuts");
        let keys = b"# mine\nkey.Ctrl+z=undo\npad.South=redo\n";
        let pads = b"# Grimoire: controller buttons\npad.East=undo\n";
        std::fs::write(&sp, keys).unwrap();
        std::fs::write(&cp, pads).unwrap();
        let (sp_t, cp_t) = (std::fs::metadata(&sp).unwrap().modified().unwrap(), std::fs::metadata(&cp).unwrap().modified().unwrap());
        assert_eq!(shortcut_profile_create(&dir, "8BitDo Pro 2", 0x2dc8, 0x6006, &sp, &cp), 0);
        let slug = "8bitdo-pro-2-2dc8-6006";
        let text = read(&shortcut_profile_ini(&dir, slug));
        let mut want = profile_header("8BitDo Pro 2", 0x2dc8, 0x6006).into_bytes();
        want.extend_from_slice(keys);
        assert_eq!(text, want);
        assert_eq!(read(&shortcut_profile_pads(&dir, slug)), pads);
        // the default's files: not written
        assert_eq!(read(&sp), keys);
        assert_eq!(read(&cp), pads);
        assert_eq!(std::fs::metadata(&sp).unwrap().modified().unwrap(), sp_t);
        assert_eq!(std::fs::metadata(&cp).unwrap().modified().unwrap(), cp_t);
        // Read back: the device and the same map as the default's.
        let profiles = shortcut_profiles_load(&dir);
        assert_eq!(profiles.len(), 1);
        let p = &profiles[0];
        assert_eq!((p.slug.as_str(), p.name.as_str(), p.vendor, p.product, p.on), (slug, "8BitDo Pro 2", 0x2dc8, 0x6006, true));
        let mut default = Shortcuts::default();
        shortcuts_load_all(&mut default, &sp, &cp);
        assert_eq!((p.map.count, p.map.rejected), (default.count, default.rejected));
        assert_eq!(shortcuts_lookup(&p.map, ActionSrc::Pad, EAST, 0), SHORTCUT_UNDO);
        assert_eq!(shortcuts_lookup(&p.map, ActionSrc::Pad, SOUTH, 0), SHORTCUT_NONE); // (set aside, as by controller.ini)
        assert_eq!(shortcuts_lookup(&p.map, ActionSrc::Key, KEY_Z, KEYMOD_CTRL), SHORTCUT_UNDO);
        // Made again: the existing profile is kept as it is.
        std::fs::write(shortcut_profile_ini(&dir, slug), b"key.z=redo\n").unwrap();
        assert_eq!(shortcut_profile_create(&dir, "8BitDo Pro 2", 0x2dc8, 0x6006, &sp, &cp), 0);
        assert_eq!(read(&shortcut_profile_ini(&dir, slug)), b"key.z=redo\n");
        let _ = std::fs::remove_dir_all(&root);
    }

    // With no shortcuts.ini the copy is the starter map (what a missing file reads as), and with no
    // controller.ini it has no buttons file; an unreadable shortcuts.ini makes no profile.
    #[test]
    fn the_default_with_no_files() {
        let root = tmp("none");
        let dir = format!("{root}/shortcuts");
        let (sp, cp) = (format!("{root}/shortcuts.ini"), format!("{root}/controller.ini"));
        assert_eq!(shortcut_profile_create(&dir, "Page Turner", 0x1234, 0xabcd, &sp, &cp), 0);
        let slug = "page-turner-1234-abcd";
        let text = read(&shortcut_profile_ini(&dir, slug));
        assert!(text.ends_with(shortcuts_starter().as_bytes()));
        assert!(std::fs::metadata(shortcut_profile_pads(&dir, slug)).is_err());
        assert!(std::fs::metadata(&sp).is_err() && std::fs::metadata(&cp).is_err(), "nothing written for the default");
        let p = shortcut_profile_read(&dir, slug);
        let mut starter = Shortcuts::default();
        shortcuts_load_all(&mut starter, &sp, &cp);
        assert_eq!((p.map.count, p.map.rejected), (starter.count, 0));
        // A shortcuts.ini that cannot be read (a folder in its place): no profile.
        std::fs::create_dir_all(&sp).unwrap();
        assert_eq!(shortcut_profile_create(&dir, "Other", 1, 1, &sp, &cp), -1);
        assert!(std::fs::metadata(shortcut_profile_ini(&dir, "other-0001-0001")).is_err());
        let _ = std::fs::remove_dir_all(&root);
    }

    // A key or button uses the profile of the device it came from while that is on, else the default
    // (None); off keeps the file, byte for byte, and on again uses it as it was.
    #[test]
    fn a_device_uses_its_own_profile_while_on() {
        let root = tmp("lookup");
        let dir = format!("{root}/shortcuts");
        std::fs::create_dir_all(&dir).unwrap();
        let slug = shortcut_profile_slug("Arcade Stick", 0x2dc8, 0x1003);
        let own = b"# Device: Arcade Stick\n# Id: 2dc8:1003\nkey.z=redo\npad.South=tool:eraser\n";
        std::fs::write(shortcut_profile_ini(&dir, &slug), own).unwrap();
        let profiles = shortcut_profiles_load(&dir);
        let map = shortcut_profile_for(&profiles, "Arcade Stick", 0x2dc8, 0x1003).expect("its profile");
        assert_eq!(shortcuts_lookup(map, ActionSrc::Key, KEY_Z, 0), SHORTCUT_REDO);
        assert_eq!(shortcuts_lookup(map, ActionSrc::Pad, SOUTH, 0), SHORTCUT_TOOL_BASE + TOOL_ERASER.0);
        // another device, the same name with other numbers, and a key no device sent: the default
        assert!(shortcut_profile_for(&profiles, "Pro Controller", 0x057e, 0x2009).is_none());
        assert!(shortcut_profile_for(&profiles, "Arcade Stick", 0x2dc8, 0x1004).is_none());
        assert!(shortcut_profile_for(&profiles, "", 0, 0).is_none());
        // Off: the default again, the file kept.
        assert_eq!(shortcut_profile_set_on(&dir, &slug, false), 0);
        let profiles = shortcut_profiles_load(&dir);
        assert!(!profiles[0].on && profiles[0].map.count == 0);
        assert!(shortcut_profile_for(&profiles, "Arcade Stick", 0x2dc8, 0x1003).is_none());
        assert_eq!(read(&shortcut_profile_ini(&dir, &slug)), own);
        // On again: used as it was (a create over it only turns it on).
        assert_eq!(shortcut_profile_create(&dir, "Arcade Stick", 0x2dc8, 0x1003, &format!("{root}/shortcuts.ini"), &format!("{root}/controller.ini")), 0);
        let profiles = shortcut_profiles_load(&dir);
        assert!(profiles[0].on);
        assert_eq!(shortcuts_lookup(shortcut_profile_for(&profiles, "Arcade Stick", 0x2dc8, 0x1003).unwrap(), ActionSrc::Key, KEY_Z, 0), SHORTCUT_REDO);
        assert_eq!(read(&shortcut_profile_ini(&dir, &slug)), own);
        let _ = std::fs::remove_dir_all(&root);
    }

    // A profile's lines are counted as shortcuts.ini's are: refused lines among them, the D-pad's starter
    // lines for a file that binds no direction, and its buttons file's rules.
    #[test]
    fn a_profile_counts_as_shortcuts_ini_does() {
        let root = tmp("count");
        let dir = format!("{root}/shortcuts");
        std::fs::create_dir_all(&dir).unwrap();
        let slug = "keys-0001-0002";
        let text = b"# Device: Keys\nkey.z=undo\nkey.q=no-such-action\nnonsense\npad.North=redo\n";
        std::fs::write(shortcut_profile_ini(&dir, slug), text).unwrap();
        let sp = format!("{root}/plain.ini");
        std::fs::write(&sp, text).unwrap();
        let mut plain = Shortcuts::default();
        shortcuts_load_all(&mut plain, &sp, &format!("{root}/none.ini"));
        let p = shortcut_profile_read(&dir, slug);
        assert_eq!((p.map.count, p.map.rejected), (plain.count, plain.rejected));
        assert_eq!(p.map.rejected, 2);
        assert_eq!(p.name, "Keys");
        assert_eq!((p.vendor, p.product), (1, 2)); // (no Id line: the slug's)
        // Its buttons file sets its pad lines aside, and a key line there is refused and counted.
        std::fs::write(shortcut_profile_pads(&dir, slug), b"pad.West=undo\nkey.x=redo\n").unwrap();
        let p = shortcut_profile_read(&dir, slug);
        assert_eq!(shortcuts_lookup(&p.map, ActionSrc::Pad, 307, 0), SHORTCUT_NONE);
        assert_eq!(shortcuts_lookup(&p.map, ActionSrc::Pad, 308, 0), SHORTCUT_UNDO);
        assert_eq!(p.map.rejected, 3);
        let _ = std::fs::remove_dir_all(&root);
    }

    // Learning a button into a profile writes its buttons file alone: shortcuts.ini, controller.ini and the
    // profile's own .ini stay byte for byte as they were.
    #[test]
    fn learning_into_a_profile_leaves_the_rest_alone() {
        let root = tmp("learn");
        let dir = format!("{root}/shortcuts");
        let (sp, cp) = (format!("{root}/shortcuts.ini"), format!("{root}/controller.ini"));
        std::fs::write(&sp, shortcuts_starter()).unwrap();
        std::fs::write(&cp, b"pad.South=undo\n").unwrap();
        assert_eq!(shortcut_profile_create(&dir, "Pad", 0x0001, 0x0001, &sp, &cp), 0);
        let slug = "pad-0001-0001";
        let ini_before = read(&shortcut_profile_ini(&dir, slug));
        let mut p = shortcut_profile_read(&dir, slug);
        let pads = shortcut_profile_pads(&dir, slug);
        assert_eq!(shortcuts_learn(&mut p.map, &pads, ActionSrc::Pad, EAST, SHORTCUT_REDO), 0);
        assert!(String::from_utf8(read(&pads)).unwrap().contains("pad.East=redo\n"));
        assert_eq!(read(&sp), shortcuts_starter().as_bytes());
        assert_eq!(read(&cp), b"pad.South=undo\n");
        assert_eq!(read(&shortcut_profile_ini(&dir, slug)), ini_before);
        // and the profile read again has it
        let p = shortcut_profile_read(&dir, slug);
        assert_eq!(shortcuts_lookup(&p.map, ActionSrc::Pad, EAST, 0), SHORTCUT_REDO);
        let _ = std::fs::remove_dir_all(&root);
    }

    // The folder: profiles by slug, buttons files and markers not taken for profiles, names that are no
    // slug left out, at most PROFILES_MAX; no folder, none.
    #[test]
    fn the_folder() {
        let root = tmp("folder");
        let dir = format!("{root}/shortcuts");
        assert!(shortcut_profiles_load(&dir).is_empty());
        std::fs::create_dir_all(&dir).unwrap();
        for f in ["b-0000-0001.ini", "a-0000-0002.ini", "a-0000-0002.controller.ini", "a-0000-0002.off", "My Keys.ini", "notes.txt"] {
            std::fs::write(format!("{dir}/{f}"), b"key.z=undo\n").unwrap();
        }
        let p = shortcut_profiles_load(&dir);
        let names: Vec<(&str, bool)> = p.iter().map(|p| (p.slug.as_str(), p.on)).collect();
        assert_eq!(names, [("a-0000-0002", false), ("b-0000-0001", true)]);
        for i in 0..PROFILES_MAX + 3 {
            std::fs::write(format!("{dir}/c{i:03}-0000-0000.ini"), b"").unwrap();
        }
        assert_eq!(shortcut_profiles_load(&dir).len(), PROFILES_MAX);
        let _ = std::fs::remove_dir_all(&root);
    }
}
