//! nav.h: a notebook's section names (nav.ini) and links (links.ini), kept apart from page ink. Page
//! ids, not page positions, keep them attached when pages move or notes are added.

use crate::prelude::*;

pub const NAV_NAME_LEN: usize = 96;
pub const NAV_FILE_MAX: usize = 256 * 1024;
pub const NAV_UNKNOWN_MAX: usize = 16 * 1024;
/// in a notebook
pub const NAV_LINKS_MAX: i32 = 4000;
pub const NAV_LINKS_PER_PAGE: i32 = 64;
pub const NAV_TARGET_LEN: usize = 16;
/// nav_link_page: the link goes to the Index
pub const NAV_TO_INDEX: i32 = -2;
/// a link's box is at least this a side, to tap or click
pub const NAV_LINK_MIN_MM: f32 = 7.0;

#[derive(Clone, Copy, Debug, Default)]
pub struct NavName {
    pub pid: u32,
    pub text: FixedStr<NAV_NAME_LEN>,
}
/// A link (link.<pid>=<seed>,<target> in links.ini): the ink strokes on page pid with this seed are
/// its box, drawn as ordinary ink, so erasing or undoing them takes the link away, and older versions
/// show the box. Targets: "index" (the Index), "p<page id>" (a page); any other (a newer version's)
/// is carried and does nothing. A link whose box is gone stays, doing nothing, until its page goes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NavLink {
    pub pid: u32,
    pub seed: u32,
    pub target: FixedStr<NAV_TARGET_LEN>,
}
#[derive(Clone, Debug, Default)]
pub struct Nav {
    /// sorted by page id (the C's names, count, capacity)
    pub names: Vec<NavName>,
    /// nav.ini's lines this version does not read, as read (bytes: carried, never interpreted)
    pub unknown: Vec<u8>,
    /// false after an unreadable or oversized nav.ini: never overwrite it
    pub writable: bool,
    /// sorted by page id, then seed (the C's links, nlinks, links_cap)
    pub links: Vec<NavLink>,
    /// links.ini's lines this version does not read, carried as nav.ini's are
    pub links_unknown: Vec<u8>,
    /// the same for links.ini
    pub links_writable: bool,
    /// links.ini is there (else it is written only once there is a link)
    pub links_file: bool,
}

pub fn nav_init(nav: &mut Nav) {
    todo!()
}
pub fn nav_free(nav: &mut Nav) {
    todo!()
}
/// nav must be initialized. Missing is an empty writable index; other read failures hide the
/// names and refuse edits. links.ini the same, on its own: either can fail with the other read.
/// Unknown lines of both are kept. -1 when either failed.
pub fn nav_load(id: &str, nav: &mut Nav) -> i32 {
    todo!()
}
/// empty when unnamed
pub fn nav_name(nav: &Nav, pid: u32) -> &str {
    todo!()
}
/// A single line, trimmed, at most 95 bytes, cut between UTF-8 characters. Empty removes a name.
pub fn nav_set_name(nav: &mut Nav, pid: u32, name: &str) -> i32 {
    todo!()
}
/// Atomic. Names of deleted pages are pruned only after a successful save; unknown lines stay.
pub fn nav_save(id: &str, nav: &mut Nav, nb: &NotebookInfo) -> i32 {
    todo!()
}
/// Named pages in notebook order; returns the full count, writes at most cap page indices.
pub fn nav_index(nav: &Nav, nb: &NotebookInfo, indices: &mut [i32]) -> i32 {
    todo!()
}
/// page index, or -1
pub fn nav_section_of(nav: &Nav, nb: &NotebookInfo, index: i32) -> i32 {
    todo!()
}

// ---- links ----

/// Adds the link of the strokes with `seed` on page pid, or changes its target. target: "index" or
/// "p<page id>". -1 when it cannot (not writable, a bad target, the limits above).
pub fn nav_link_set(nav: &mut Nav, pid: u32, seed: u32, target: &str) -> i32 {
    todo!()
}
pub fn nav_link_remove(nav: &mut Nav, pid: u32, seed: u32) {
    todo!()
}
/// Page pid's links: how many, and the first in nav->links (*first).
pub fn nav_links_of(nav: &Nav, pid: u32, first: &mut i32) -> i32 {
    todo!()
}
/// Where a link goes in nb: a page index, NAV_TO_INDEX, or -1 (a page gone, a target this version
/// does not know).
pub fn nav_link_page(l: &NavLink, nb: &NotebookInfo) -> i32 {
    todo!()
}
/// The boxes of page p's links (p is page pid): for each link whose strokes are there, the box of
/// their points (the ink strokes only), made at least min_px a side around its centre. Writes at most
/// cap links and their boxes; returns how many it wrote.
pub fn nav_link_areas(nav: &Nav, p: &Page, pid: u32, min_px: f32, links: &mut [NavLink], areas: &mut [Rect]) -> i32 {
    todo!()
}
/// The link whose box holds (x, y) on page p, the one drawn last when boxes overlap. False for none.
pub fn nav_link_at(nav: &Nav, p: &Page, pid: u32, x: f32, y: f32, min_px: f32, out: &mut NavLink) -> bool {
    todo!()
}
/// The seed for a new ink stroke on page p (page pid): want, or the next after it that no ink stroke on
/// p and no link of pid has, so a link's box never takes in a stroke drawn after it.
pub fn nav_new_seed(nav: &Nav, p: &Page, pid: u32, want: u32) -> u32 {
    todo!()
}
/// Writes links.ini, leaving out the links of pages no longer in nb and those going to such pages.
/// -1 when not links_writable or the write failed (then nothing is left out).
pub fn nav_save_links(id: &str, nav: &mut Nav, nb: &NotebookInfo) -> i32 {
    todo!()
}
