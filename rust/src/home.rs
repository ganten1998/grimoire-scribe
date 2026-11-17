//! The home folder (2.2, issue #160): the place the main menu opens at when Grimoire starts, and where the `home`
//! action goes. settings.ini keeps it as one line, `home=`:
//!
//! - absent, empty or `notebooks`: the Library's top level, as before there was a home;
//! - `notebooks/<folder>`: a Library folder by its path (`notebooks/School`, `notebooks/School/2026`);
//! - `files/<path>`: a folder of the Kindle's storage as Files shows it, by its path from the storage root
//!   (`files/documents/Planners`).
//!
//! The line is anyone's to write (the user, another program, another version; Grimoire runs as root), so it is read
//! strictly and never trusted. `Home::parse` takes those shapes alone, under a length, in UTF-8, with no control
//! character, no absolute path and no empty, `.` or `..` part: any other value names no home. A value that parses
//! still names a folder that may be gone, or one Grimoire never enters. A Library folder is a name looked up in the
//! Library's own list, never a path on the storage. A folder inside Files is reached only through fsview's walk
//! (`FsStorage::folder`: each part opened from the one before, never through a link, on the storage's own device,
//! in no refused place, and not Grimoire's trash). Nothing joins the value to a path and opens that, and nothing is
//! listed, read or made at a home that fails: the screens (library.rs) then show the Library's top, say "Home folder
//! not found." once, and put the setting back to Notebooks (or keep it, when the walk failed on a read error and
//! not because the folder is gone or refused).

use crate::fsview::{FsErr, FsStorage, StoragePath, FS_PATH_MAX};
use crate::prelude::*;

/// A settings.ini `home=` value is shorter than this: `files/` and a storage path (shorter than FS_PATH_MAX).
pub const HOME_MAX: usize = FS_PATH_MAX + 8;

/// Where home is.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Home {
    /// the Library's top level (no line in settings.ini)
    #[default]
    Notebooks,
    /// a Library folder, by its path there ("School", "School/2026")
    Library(String),
    /// a folder of the Kindle's storage, as Files shows it (never the storage root itself)
    Files(StoragePath),
}

impl Home {
    /// What a settings.ini `home=` value names, None for a value that names no home (see the module's words). No
    /// file or folder is looked at.
    pub fn parse(v: &[u8]) -> Option<Home> {
        if v.len() >= HOME_MAX {
            return None;
        }
        let s = std::str::from_utf8(v).ok()?;
        if s.chars().any(|c| c.is_control()) {
            return None;
        }
        if s.is_empty() || s == "notebooks" {
            return Some(Home::Notebooks);
        }
        if let Some(folder) = s.strip_prefix("notebooks/") {
            return store_folder_valid(folder).then(|| Home::Library(folder.to_string()));
        }
        let path = s.strip_prefix("files/")?;
        StoragePath::new(path.as_bytes()).filter(|p| !p.is_root()).map(Home::Files)
    }

    /// The value settings.ini keeps for it ("" for Notebooks: the line is left out).
    pub fn value(&self) -> String {
        match self {
            Home::Notebooks => String::new(),
            Home::Library(f) => format!("notebooks/{f}"),
            Home::Files(p) => format!("files/{}", p.to_string_lossy()),
        }
    }
}

/// Whether the folder `p` of the storage `st` can be home now: a path settings.ini can keep as it is (one that
/// `Home::parse` gives back), which Files' own walk reaches as a folder (`FsStorage::folder`).
pub fn home_folder_ok(st: &FsStorage, p: &StoragePath) -> bool {
    home_folder_check(st, p).is_ok()
}

/// The same with why not: `FsErr::Refused` for a path settings.ini cannot keep, else what the walk says. Every error
/// but `FsErr::Io` says the folder is gone or refused; `Io` is a read that failed for another reason.
pub fn home_folder_check(st: &FsStorage, p: &StoragePath) -> Result<(), FsErr> {
    if Home::parse(Home::Files(p.clone()).value().as_bytes()) != Some(Home::Files(p.clone())) {
        return Err(FsErr::Refused);
    }
    st.folder(p).map(|_| ())
}

/// The same on the storage Files browses (opened for this one look); `FsErr::Io` when there is none or it does not
/// open.
pub fn home_folder_there(p: &StoragePath) -> Result<(), FsErr> {
    let root = crate::pdfimport::storage_root().ok_or(FsErr::Io(libc::ENODEV))?;
    home_folder_check(&FsStorage::open(&root, &store_root())?, p)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn files(p: &str) -> Option<Home> {
        Some(Home::Files(StoragePath::new(p.as_bytes()).unwrap()))
    }

    // The three shapes, and the value each is kept as.
    #[test]
    fn a_home_is_notebooks_a_library_folder_or_a_folder_in_files() {
        assert_eq!(Home::parse(b""), Some(Home::Notebooks));
        assert_eq!(Home::parse(b"notebooks"), Some(Home::Notebooks));
        assert_eq!(Home::parse(b"notebooks/School"), Some(Home::Library("School".into())));
        assert_eq!(Home::parse(b"notebooks/School/2026"), Some(Home::Library("School/2026".into())));
        assert_eq!(Home::parse("notebooks/Álgebra 2".as_bytes()), Some(Home::Library("Álgebra 2".into())));
        assert_eq!(Home::parse(b"files/documents"), files("documents"));
        assert_eq!(Home::parse(b"files/documents/Planners 2026"), files("documents/Planners 2026"));
        assert_eq!(Home::parse(b"files/grimoire/import/Scans"), files("grimoire/import/Scans"));
        for v in ["", "notebooks/School", "notebooks/School/2026", "files/documents", "files/a/b c/d"] {
            let h = Home::parse(v.as_bytes()).unwrap();
            assert_eq!(h.value(), v, "{h:?}");
            assert_eq!(Home::parse(h.value().as_bytes()), Some(h));
        }
        assert_eq!(Home::Notebooks.value(), "");
        assert_eq!(Home::default(), Home::Notebooks);
    }

    // What a hand, another program or another version may have written: none of it is a home.
    #[test]
    fn a_hostile_value_is_no_home() {
        let long_folder = format!("notebooks/{}", "a".repeat(STORE_NAME_LEN));
        let long_path = format!("files/{}", ["a".repeat(200), "b".repeat(200), "c".repeat(198)].join("/"));
        let too_long = format!("files/{}", "a/".repeat(HOME_MAX));
        let bad: Vec<&[u8]> = vec![
            // an absolute path, with a prefix or without
            b"/",
            b"/etc",
            b"/mnt/us/documents",
            b"files//etc/passwd",
            b"files//mnt/us/documents",
            b"notebooks//School",
            // a way up
            b"..",
            b"files/..",
            b"files/../etc",
            b"files/documents/../../etc",
            b"files/documents/..",
            b"notebooks/..",
            b"notebooks/School/../..",
            // a part that names nothing
            b"files/",
            b"files",
            b"notebooks/",
            b"files/documents/",
            b"files/documents//Planners",
            b"files/./documents",
            b"files/documents/.",
            b"notebooks/School/",
            b"notebooks/./School",
            // control bytes
            b"files/documents\n",
            b"files/doc\numents",
            b"files/documents\r",
            b"files/doc\tuments",
            b"files/doc\x00uments",
            b"files/doc\x1buments",
            b"files/doc\x7fuments",
            b"notebooks/Sch\nool",
            b"notebooks/School\x00",
            "files/doc\u{85}uments".as_bytes(),
            // not UTF-8
            b"files/doc\xffuments",
            b"notebooks/\xc3",
            // another shape: a spelling, a place that is no folder, a value of another version
            b"Notebooks",
            b"NOTEBOOKS/School",
            b"Files/documents",
            b"notebooks/School\\..\\x/..",
            b"documents",
            b"file:documents",
            b"files:documents",
            b"home",
            b"recent",
            b"trash",
            b"~",
            b"files\\documents",
            b" files/documents",
            b"notebooks ",
            // too long
            long_folder.as_bytes(),
            long_path.as_bytes(),
            too_long.as_bytes(),
        ];
        for v in bad {
            assert_eq!(Home::parse(v), None, "{:?}", String::from_utf8_lossy(v));
        }
        // (the longest of each that is one)
        let folder = format!("notebooks/{}", "a".repeat(STORE_NAME_LEN - 1));
        assert!(matches!(Home::parse(folder.as_bytes()), Some(Home::Library(_))));
        let path = format!("files/{}", ["a".repeat(200), "b".repeat(200), "c".repeat(197)].join("/"));
        assert!(path.len() < HOME_MAX && matches!(Home::parse(path.as_bytes()), Some(Home::Files(_))));
    }

    // A folder of the storage is home only as Files' walk reaches it: a folder in each kind of place is, and the
    // storage's root, a folder gone, a file where a folder was, a link anywhere on the way (to a folder of the storage
    // or out of it), the trash and what is in it, Grimoire's notebooks, a refused folder and a hidden one are not.
    // Nothing is made at a path that is not.
    #[test]
    fn a_folder_is_home_only_through_the_walk() {
        let base = std::env::temp_dir().join(format!("grimoire-home-walk-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        for d in [
            "documents/Planners/2026",
            "Notes",
            "grimoire/import/Scans",
            "grimoire/photos/Trip",
            "grimoire/PDFs",
            "grimoire/trash/files-20261101-000000-abcd",
            "grimoire/notebooks/20261101-000000-abcd",
            "system/fonts",
            ".hidden",
        ] {
            std::fs::create_dir_all(base.join(d)).unwrap();
        }
        std::fs::write(base.join("documents/file.pdf"), b"%PDF-1.4\n").unwrap();
        std::os::unix::fs::symlink("Planners", base.join("documents/Linked")).unwrap();
        std::os::unix::fs::symlink("/etc", base.join("documents/etc")).unwrap();
        std::os::unix::fs::symlink("../documents", base.join("Notes/up")).unwrap();
        let st = FsStorage::open(base.to_str().unwrap(), base.join("grimoire").to_str().unwrap()).unwrap();
        let ok = |p: &str| home_folder_ok(&st, &StoragePath::new(p.as_bytes()).unwrap());
        for p in ["documents", "documents/Planners", "documents/Planners/2026", "Notes", "grimoire", "grimoire/import", "grimoire/import/Scans", "grimoire/photos/Trip", "grimoire/PDFs"] {
            assert!(ok(p), "{p}");
        }
        for p in [
            "",
            "documents/Nowhere",
            "documents/Nowhere/Deeper",
            "documents/file.pdf",
            "documents/Linked",
            "documents/Linked/2026",
            "documents/etc",
            "Notes/up",
            "Notes/up/Planners",
            "grimoire/trash",
            "grimoire/trash/files-20261101-000000-abcd",
            "grimoire/notebooks",
            "grimoire/notebooks/20261101-000000-abcd",
            "system",
            "system/fonts",
            ".hidden",
            // (a part ending in a dot or a space, and a refused folder by another spelling the storage takes for its
            // name: fsview refuses both, on the Mac's storage too)
            "documents.",
            "documents/Planners ",
            "\u{17f}ystem/fonts",
        ] {
            assert!(!ok(p), "{p}");
        }
        assert!(!base.join("documents/Nowhere").exists(), "nothing is made at a home that is not there");
        // (a folder whose name holds a control character is there, but settings.ini could not keep it as it is)
        if std::fs::create_dir_all(base.join("documents/Ta\tb")).is_ok() {
            assert!(!ok("documents/Ta\tb"));
        }
        drop(st);
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn no_em_dash() {
        let src = include_str!("home.rs");
        assert!(!src.contains(['\u{2014}', '\u{2013}']));
    }
}
