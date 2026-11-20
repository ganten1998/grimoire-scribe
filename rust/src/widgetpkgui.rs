//! The Settings → Packages → Widgets page (issue #200): the widget package manager.
//!
//! Installed packages are rows, as every Settings submenu's are. Install from a repository,
//! trust and license are asked before anything is written, and Remove and Check for updates are
//! here too. The page draws with the Settings page's header, Back and X, and pages long lists
//! with `ui_row_page`; it is not a separate editor overlay.
//!
//! The state lives on `App` (`pkg`) because Settings can be opened over the Library or over a
//! notebook. Its fetch runs in a child and is polled from the library tick, so no contact waits
//! on the network.

use crate::prelude::*;

const H_CLOSE: i32 = 1;
const H_ROW: i32 = 2;
const H_INSTALL: i32 = 100;
const H_BACK: i32 = 101;
const H_REMOVE: i32 = 102;
const H_REMOVE_YES: i32 = 103;
const H_TRUST: i32 = 105;
const H_LICENSE_YES: i32 = 106;
const H_UPDATE: i32 = 108;
const H_PREV: i32 = 109;
const H_NEXT: i32 = 110;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PkgView {
    #[default]
    List,
    Detail,
    Install,
    Busy,
    Trust,
    License,
    Remove,
    Error,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FetchStage {
    Branch,
    Manifest,
    License,
    Artifact,
}

struct InstallJob {
    stage: FetchStage,
    fetch: WidgetFetch,
    input: String,
    manifest_url: String,
}

#[derive(Default)]
pub struct WidgetPkgUi {
    pub open: bool,
    pub view: PkgView,
    pub kb: Keyboard,
    pub items: Vec<WidgetInstall>,
    pub selected: Option<String>,
    page: i32,
    pub hits: HitList,
    press: Press,
    pub status: String,
    pub error: String,
    /// the manifest being installed or updated, once its repository answered
    manifest: Option<WidgetPackage>,
    /// the repository address the manifest and artifact came from
    manifest_url: String,
    /// the manifest's bytes as fetched, kept for the install after consent
    manifest_yaml: String,
    /// the repository input the install started with
    input: String,
    /// the manifest is for the selected package's Check for updates, not a new install
    updating: bool,
    artifact_url: String,
    license_url: String,
    license_text: String,
    job: Option<InstallJob>,
    /// the page's rect, for feedback and redraw
    pub rect: Rect,
}

impl WidgetPkgUi {
    fn refresh(&mut self) {
        self.items = widget_package_list();
    }
}

/// Opens Settings → Packages → Widgets. The caller has Settings open (library.rs): closing this
/// page comes back to it, not out of Settings.
pub fn packages_ui_open(a: &mut App) {
    let ui = &mut a.pkg;
    ui.open = true;
    ui.view = PkgView::List;
    ui.status.clear();
    ui.error.clear();
    ui.selected = None;
    ui.page = 0;
    ui.license_text.clear();
    ui.manifest = None;
    ui.manifest_yaml.clear();
    ui.input.clear();
    ui.updating = false;
    ui.job = None;
    ui.refresh();
    crate::library::packages_redraw(a);
}

fn close(a: &mut App) {
    let ui = &mut a.pkg;
    ui.open = false;
    ui.job = None;
    keyboard_close(&mut ui.kb);
    crate::library::packages_closed(a);
}

/// Back from the first page: Settings itself, as a Settings submenu's Back does.
fn back_to_settings(a: &mut App) {
    let ui = &mut a.pkg;
    ui.open = false;
    ui.job = None;
    keyboard_close(&mut ui.kb);
    crate::library::packages_redraw(a);
}

fn redraw(a: &mut App) {
    crate::library::packages_redraw(a);
}

/// A row's label from two parts, with the version in parentheses.
fn labeled(name: &str, version: &str) -> String {
    if version.is_empty() {
        name.to_string()
    } else {
        format!("{name} ({version})")
    }
}

fn start_job(ui: &mut WidgetPkgUi, stage: FetchStage, url: String, input: &str, manifest_url: &str) {
    match WidgetFetch::start(&url, if stage == FetchStage::Artifact { WIDGET_PKG_ARTIFACT_MAX as usize } else { 512 * 1024 }) {
        Ok(fetch) => {
            ui.status = match stage {
                FetchStage::Branch => "Looking up the repository...".into(),
                FetchStage::Manifest => "Reading package.yaml...".into(),
                FetchStage::License => "Reading the license...".into(),
                FetchStage::Artifact => "Downloading the widget...".into(),
            };
            ui.view = PkgView::Busy;
            ui.job = Some(InstallJob {
                stage,
                fetch,
                input: input.to_string(),
                manifest_url: manifest_url.to_string(),
            });
        }
        Err(e) => {
            ui.error = e;
            ui.view = PkgView::Error;
            ui.job = None;
        }
    }
}

fn begin_install(ui: &mut WidgetPkgUi, input: &str) {
    let input = input.trim().to_string();
    ui.input = input.clone();
    ui.updating = false;
    if input.is_empty() {
        ui.error = "Enter a repository or a .yaml address.".into();
        ui.view = PkgView::Error;
        return;
    }
    if let Some(api) = widget_github_api(&input) {
        start_job(ui, FetchStage::Branch, api, &input, "");
        return;
    }
    match widget_manifest_url(&input) {
        Ok((url, _)) => start_job(ui, FetchStage::Manifest, url.clone(), &input, &url),
        Err(e) => {
            ui.error = e;
            ui.view = PkgView::Error;
        }
    }
}

fn begin_update(ui: &mut WidgetPkgUi, it: &WidgetInstall) {
    ui.updating = true;
    if it.source_url.is_empty() {
        ui.error = "This package was installed by hand; there is no repository to check.".into();
        ui.view = PkgView::Error;
        return;
    }
    start_job(ui, FetchStage::Manifest, it.source_url.clone(), &it.source_url, &it.source_url);
}

fn license_needed(p: &WidgetPackage) -> bool {
    !p.license_url.is_empty()
}

fn after_manifest(ui: &mut WidgetPkgUi, data: &[u8], a: &App, manifest_url: &str) {
    let text = String::from_utf8_lossy(data);
    let p = match widget_package_parse(&text).and_then(|p| {
        widget_package_validate(&p, a.w, a.h - status_reserved(a)).map(|_| p)
    }) {
        Ok(p) => p,
        Err(e) => {
            ui.error = e;
            ui.view = PkgView::Error;
            return;
        }
    };
    let artifact = if let Some(u) = widget_package_artifact_url(manifest_url, &p) {
        u
    } else {
        ui.error = "The manifest names no artifact that can be downloaded.".into();
        ui.view = PkgView::Error;
        return;
    };
    if ui.updating {
        if let Some(old) = widget_package_list().into_iter().find(|x| x.id == p.id) {
            if update_version_cmp(&p.version, &old.version) <= 0 {
                ui.status = format!("{} is up to date ({}).", old.name, old.version);
                ui.view = PkgView::Detail;
                return;
            }
        }
    }
    ui.license_url = p.license_url.clone();
    ui.artifact_url = artifact;
    ui.manifest_url = manifest_url.to_string();
    ui.manifest = Some(p);
    ui.status.clear();
    ui.view = PkgView::Trust;
}

fn after_artifact(ui: &mut WidgetPkgUi, data: &[u8], a: &mut App) {
    if ui.manifest.is_none() || ui.manifest_yaml.is_empty() {
        ui.error = "The package's manifest was lost.".into();
        ui.view = PkgView::Error;
        return;
    }
    let manifest = ui.manifest_yaml.clone();
    match widget_package_install(&manifest, data, &ui.manifest_url, a.w, a.h - status_reserved(a)) {
        Ok(it) => {
            // An open frame of this package takes the new definition now, not at its next open.
            if let Some(ed) = a.ed.as_deref_mut() {
                if let Some(i) = ed.widgets.index_of(&it.id) {
                    if let Some(def) = widget_def(&it.id) {
                        ed.widgets.frames[i].def = def;
                    }
                }
            }
            ui.status = format!("Installed {} {}.", it.name, it.version);
            ui.view = PkgView::List;
            ui.selected = None;
            ui.refresh();
        }
        Err(e) => {
            ui.error = e;
            ui.view = PkgView::Error;
        }
    }
}

/// Polls the install's curl and runs the open keyboard's hold and settle timers, as every screen's
/// tick does. Called from the library tick.
pub fn packages_ui_tick(a: &mut App, now: u32) {
    if !a.pkg.open {
        return;
    }
    if a.pkg.kb.open {
        // A key draws and refreshes only the keyboard (app_keyboard_pointer); this is its held
        // delete's repeats and the one grayscale clean-up after typing pauses, as in every other
        // keyboard. The page under it does not move while it is open.
        let mut kb = std::mem::take(&mut a.pkg.kb);
        app_keyboard_tick(a, &mut kb, now);
        a.pkg.kb = kb;
        return;
    }
    let mut ui = std::mem::take(&mut a.pkg);
    let mut redraw_page = false;
    if let Some(mut job) = ui.job.take() {
        match job.fetch.tick() {
            None => {
                ui.job = Some(job);
            }
            Some(res) => {
                redraw_page = true;
                match res {
                    Err(e) => {
                        // The GitHub API is an optimization: a failed lookup falls back to main.
                        if job.stage == FetchStage::Branch {
                            let url = widget_manifest_url(&job.input).map(|(u, _)| u).unwrap_or_default();
                            if !url.is_empty() {
                                start_job(&mut ui, FetchStage::Manifest, url.clone(), &job.input, &url);
                            } else {
                                ui.error = e;
                                ui.view = PkgView::Error;
                            }
                        } else {
                            ui.error = e;
                            ui.view = PkgView::Error;
                        }
                    }
                    Ok(data) => match job.stage {
                        FetchStage::Branch => {
                            let input = job.input.clone();
                            let fallback = widget_manifest_url(&input).map(|(u, _)| u).unwrap_or_default();
                            let url = widget_github_branch(&String::from_utf8_lossy(&data))
                                .and_then(|b| {
                                    let parts: Vec<&str> = input.split('/').collect();
                                    (parts.len() == 2).then(|| format!("https://raw.githubusercontent.com/{}/{}/{b}/package.yaml", parts[0], parts[1]))
                                })
                                .unwrap_or(fallback);
                            if url.is_empty() {
                                ui.error = "That repository has no package.yaml.".into();
                                ui.view = PkgView::Error;
                            } else {
                                start_job(&mut ui, FetchStage::Manifest, url.clone(), &input, &url);
                            }
                        }
                        FetchStage::Manifest => {
                            ui.input = job.input.clone();
                            ui.manifest_yaml = String::from_utf8_lossy(&data).into_owned();
                            let mu = job.manifest_url.clone();
                            after_manifest(&mut ui, &data, a, &mu);
                        }
                        FetchStage::License => {
                            ui.license_text = String::from_utf8_lossy(&data).into_owned();
                            ui.view = PkgView::License;
                        }
                        FetchStage::Artifact => after_artifact(&mut ui, &data, a),
                    },
                }
            }
        }
    }
    a.pkg = ui;
    if redraw_page {
        crate::library::packages_redraw(a);
    }
}

fn close_or_cancel(a: &mut App) {
    let mut ui = std::mem::take(&mut a.pkg);
    match ui.view {
        PkgView::List => {
            a.pkg = ui;
            back_to_settings(a);
            return;
        }
        PkgView::Error => {
            ui.error.clear();
            ui.view = PkgView::List;
        }
        PkgView::Detail | PkgView::Install | PkgView::Remove => {
            ui.view = PkgView::List;
            keyboard_close(&mut ui.kb);
            ui.selected = None;
        }
        PkgView::Trust | PkgView::License | PkgView::Busy => {
            ui.view = PkgView::List;
            ui.job = None;
            ui.status.clear();
            ui.error.clear();
        }
    }
    a.pkg = ui;
    redraw(a);
}

fn do_tap(a: &mut App, h: Hit) {
    let mut ui = std::mem::take(&mut a.pkg);
    match h.id {
        H_CLOSE => {
            a.pkg = ui;
            close(a);
            return;
        }
        H_BACK => {
            a.pkg = ui;
            close_or_cancel(a);
            return;
        }
        H_ROW => {
            if let Some(it) = ui.items.get(h.arg as usize).cloned() {
                ui.selected = Some(it.id);
                ui.status.clear();
                ui.error.clear();
                ui.view = PkgView::Detail;
            }
        }
        H_PREV => ui.page = (ui.page - 1).max(0),
        H_NEXT => ui.page += 1,
        H_INSTALL => {
            ui.view = PkgView::Install;
            ui.error.clear();
            keyboard_open(&mut ui.kb, tr("Widget repository"), b"");
        }
        H_TRUST => {
            if ui.manifest.as_ref().is_some_and(license_needed) {
                let url = ui.license_url.clone();
                let input = ui.input.clone();
                let mu = ui.manifest_url.clone();
                start_job(&mut ui, FetchStage::License, url, &input, &mu);
            } else {
                let url = ui.artifact_url.clone();
                let input = ui.input.clone();
                let mu = ui.manifest_url.clone();
                start_job(&mut ui, FetchStage::Artifact, url, &input, &mu);
            }
        }
        H_LICENSE_YES => {
            let url = ui.artifact_url.clone();
            let input = ui.input.clone();
            let mu = ui.manifest_url.clone();
            start_job(&mut ui, FetchStage::Artifact, url, &input, &mu);
        }
        H_REMOVE => ui.view = PkgView::Remove,
        H_REMOVE_YES => {
            if let Some(id) = ui.selected.clone() {
                match widget_package_remove(&id, true) {
                    Ok(()) => {
                        // An open frame goes with its package.
                        if let Some(ed) = a.ed.as_deref_mut() {
                            while let Some(i) = ed.widgets.index_of(&id) {
                                ed.widgets.close(i);
                            }
                            ed.widgets.save();
                        }
                        ui.status = "Package removed.".into();
                        ui.view = PkgView::List;
                        ui.selected = None;
                        ui.refresh();
                    }
                    Err(e) => {
                        ui.error = e;
                        ui.view = PkgView::Error;
                    }
                }
            }
        }
        H_UPDATE => {
            if let Some(it) = ui.items.iter().find(|x| Some(&x.id) == ui.selected.as_ref()).cloned() {
                begin_update(&mut ui, &it);
            }
        }
        _ => {}
    }
    a.pkg = ui;
    redraw(a);
}

pub fn packages_ui_pointer(a: &mut App, ph: i32, ev: &InEvent) {
    if !a.pkg.open {
        return;
    }
    if a.pkg.kb.open {
        // Exactly what a key costs on the Library's keyboard: the text line and the key, each on its
        // own fast refresh, and nothing at all for a sample that changed nothing. The page under it
        // is drawn again only when the keyboard closes (Done or Cancel).
        let mut ui = std::mem::take(&mut a.pkg);
        let r = app_keyboard_pointer(a, &mut ui.kb, ph, ev);
        let finished = r == KbResult::Done || r == KbResult::Cancel;
        if finished {
            keyboard_close(&mut ui.kb);
            if r == KbResult::Done {
                let text = ui.kb.text.as_str().to_string();
                begin_install(&mut ui, &text);
            } else {
                ui.view = PkgView::List;
            }
        }
        a.pkg = ui;
        if finished {
            redraw(a);
        }
        return;
    }
    if ph == 0 {
        press_begin(&mut a.pkg.press, ev.x, ev.y, ev.t_ms);
        return;
    }
    press_move(&mut a.pkg.press, ev.x, ev.y);
    if ph == 2 {
        let hit = hits_find(&a.pkg.hits, ev.x, ev.y).copied();
        let moved = a.pkg.press.moved;
        if !moved && !touch_cancelled(ev) {
            if let Some(h) = hit {
                do_tap(a, h);
                return;
            }
        }
        redraw(a);
    }
}

fn draw_actions(c: &mut Canvas, ui: &mut WidgetPkgUi, area: Rect, start_y: i32, labels: &[(&str, IconKind, i32)]) {
    let bh = ui_px(112.0);
    let gap = ui_px(18.0);
    let mut y = start_y;
    for (label, icon, id) in labels {
        if y + bh > area.y + area.h {
            break;
        }
        let r = Rect { x: area.x, y, w: area.w, h: bh };
        let row = Row::new(*icon, *label);
        ui_row(c, r, &row);
        hits_add(&mut ui.hits, r, *id, 0);
        y += bh + gap;
    }
}

fn body_text(c: &mut Canvas, r: Rect, text: &str) {
    ui_text(c, FontFace::Regular, ui_text_size(), ui_small_size(), r, text, 12, false, 0);
}

pub fn packages_ui_draw(a: &mut App) {
    if !a.pkg.open {
        return;
    }
    let screen = Rect { x: 0, y: 0, w: a.w, h: a.h };
    let title = match a.pkg.view {
        PkgView::List | PkgView::Error => tr("Widgets"),
        PkgView::Detail => tr("Widget package"),
        PkgView::Install => tr("Install a widget"),
        PkgView::Busy => tr("Installing"),
        PkgView::Trust => tr("Install this widget?"),
        PkgView::License => tr("License agreement"),
        PkgView::Remove => tr("Remove this package?"),
    };
    let App { screen: c, pkg: ui, .. } = a;
    ui.hits.h.clear();
    let page = crate::library::settings_page(c, a.w, a.h, title, true, false);
    hits_add(&mut ui.hits, page.back, H_BACK, 0);
    hits_add(&mut ui.hits, page.close, H_CLOSE, 0);
    ui.rect = Rect { x: 0, y: 0, w: a.w, h: a.h };
    let content = page.content;
    let bottom = if ui.kb.open { ui.kb.area.y - ui_px(12.0) } else { content.y + content.h };
    match ui.view {
        PkgView::List => {
            let n = ui.items.len();
            let actions = 1 + n;
            let rows_area = Rect { x: content.x, y: content.y, w: content.w, h: bottom - content.y };
            let list = ui_row_page(rows_area, actions, ui.page, ui_px(118.0), ui_px(74.0));
            ui.page = list.page;
            for (i, rr) in list.rows.iter().enumerate() {
                let k = list.first + i;
                if k == 0 {
                    let row = Row::new(IconKind::Download, tr("Install from repository..."));
                    ui_row(c, *rr, &row);
                    hits_add(&mut ui.hits, *rr, H_INSTALL, 0);
                } else if let Some(it) = ui.items.get(k - 1) {
                    let label = labeled(&it.name, &it.version);
                    let row = Row::new(IconKind::Grid, &label);
                    ui_row(c, *rr, &row);
                    hits_add(&mut ui.hits, *rr, H_ROW, (k - 1) as i32);
                }
            }
            ui_row_pager(c, &list);
            if list.page > 0 {
                hits_add(&mut ui.hits, list.prev, H_PREV, 0);
            }
            if list.page + 1 < list.pages {
                hits_add(&mut ui.hits, list.next, H_NEXT, 0);
            }
            if ui.items.is_empty() {
                ui_text(c, FontFace::Regular, ui_small_size(), ui_small_size(), Rect { x: content.x, y: content.y + ui_px(160.0), w: content.w, h: ui_px(100.0) }, tr("No widget packages are installed."), 2, false, 120);
            }
        }
        PkgView::Detail => {
            let Some(it) = ui.items.iter().find(|x| Some(&x.id) == ui.selected.as_ref()).cloned() else {
                ui.view = PkgView::List;
                return;
            };
            let title = labeled(&it.name, &it.version);
            ui_text(c, FontFace::Bold, ui_text_size(), ui_small_size(), Rect { x: content.x, y: content.y, w: content.w, h: ui_px(78.0) }, &title, 1, false, 0);
            let mut y = content.y + ui_px(82.0);
            if !it.author.is_empty() {
                body_text(c, Rect { x: content.x, y, w: content.w, h: ui_px(46.0) }, &it.author);
                y += ui_px(48.0);
            }
            if !it.description.is_empty() {
                body_text(c, Rect { x: content.x, y, w: content.w, h: ui_px(100.0) }, &it.description);
                y += ui_px(104.0);
            }
            if !it.source_url.is_empty() {
                body_text(c, Rect { x: content.x, y, w: content.w, h: ui_px(46.0) }, &it.source_url);
                y += ui_px(48.0);
            }
            if !ui.status.is_empty() {
                body_text(c, Rect { x: content.x, y, w: content.w, h: ui_px(90.0) }, &ui.status);
            }
            draw_actions(c, ui, Rect { x: content.x, y: content.y, w: content.w, h: bottom - content.y }, y + ui_px(18.0), &[
                (tr("Check for updates"), IconKind::Update, H_UPDATE),
                (tr("Remove..."), IconKind::Close, H_REMOVE),
            ]);
        }
        PkgView::Install => {
            // The repository is typed on the one global keyboard: its text line above the keys is the
            // field, as on every other keyboard, and its Done starts the install. No second text box.
            body_text(c, Rect { x: content.x, y: content.y, w: content.w, h: ui_px(210.0) }, tr("Enter a GitHub repository as user/repo, or an https address ending in .yaml."));
            body_text(c, Rect { x: content.x, y: content.y + ui_px(220.0), w: content.w, h: ui_px(80.0) }, tr("Tap Done on the keyboard to install."));
        }
        PkgView::Busy => {
            body_text(c, Rect { x: content.x, y: content.y, w: content.w, h: ui_px(220.0) }, &ui.status);
        }
        PkgView::Trust => {
            let m = ui.manifest.clone().unwrap_or_default();
            let mut text = String::new();
            csnprintf!(&mut text, 1000, "%s by %s, version %s.\n\nYou MUST trust the author of this package. Grimoire does not review, vet, or verify widget code and is not responsible for any damage, data loss, or malfunction caused by installing or running it.", &m.name, &m.author, &m.version);
            body_text(c, Rect { x: content.x, y: content.y, w: content.w, h: ui_px(520.0) }, &text);
            draw_actions(c, ui, Rect { x: content.x, y: content.y, w: content.w, h: bottom - content.y }, content.y + ui_px(560.0), &[
                (tr("I trust this author, continue"), IconKind::Download, H_TRUST),
            ]);
        }
        PkgView::License => {
            let head = if ui.license_url.is_empty() { tr("License").to_string() } else { ui.license_url.clone() };
            ui_text(c, FontFace::Bold, ui_small_size(), ui_small_size(), Rect { x: content.x, y: content.y, w: content.w, h: ui_px(80.0) }, &head, 2, false, 0);
            let text = if ui.license_text.is_empty() { tr("No license text is shown.") } else { &ui.license_text };
            body_text(c, Rect { x: content.x, y: content.y + ui_px(90.0), w: content.w, h: (bottom - content.y - ui_px(240.0)).max(0) }, text);
            draw_actions(c, ui, Rect { x: content.x, y: content.y, w: content.w, h: bottom - content.y }, bottom - ui_px(130.0), &[
                (tr("I have read and agree"), IconKind::Download, H_LICENSE_YES),
            ]);
        }
        PkgView::Remove => {
            let name = ui.items.iter().find(|x| Some(&x.id) == ui.selected.as_ref()).map(|x| x.name.clone()).unwrap_or_default();
            let text = format!("Remove {name}? Its widget is closed and its files are deleted. Your layout is kept.");
            body_text(c, Rect { x: content.x, y: content.y, w: content.w, h: ui_px(200.0) }, &text);
            draw_actions(c, ui, Rect { x: content.x, y: content.y, w: content.w, h: bottom - content.y }, content.y + ui_px(230.0), &[
                (tr("Remove"), IconKind::Close, H_REMOVE_YES),
            ]);
        }
        PkgView::Error => {
            body_text(c, Rect { x: content.x, y: content.y, w: content.w, h: ui_px(300.0) }, &ui.error);
        }
    }
    if !ui.status.is_empty() && ui.view == PkgView::List {
        ui_text(c, FontFace::Regular, ui_small_size(), ui_small_size(), Rect { x: content.x, y: bottom - ui_px(70.0), w: content.w, h: ui_px(60.0) }, &ui.status, 1, false, 90);
    }
    if ui.kb.open {
        let kr = keyboard_draw(&mut ui.kb, c, screen);
        ui.rect = rect_union(ui.rect, kr);
    }
}

/// A named control on the Packages page, for the scenario runner (`tap panel packages.install`).
pub fn packages_ui_control_point(ui: &WidgetPkgUi, name: &str) -> Option<(f32, f32)> {
    let id = match name {
        "close" => H_CLOSE,
        "install" => H_INSTALL,
        "trust" => H_TRUST,
        "license-yes" => H_LICENSE_YES,
        "remove" => H_REMOVE,
        "remove-yes" => H_REMOVE_YES,
        "update" => H_UPDATE,
        "back" => H_BACK,
        _ => {
            let id = name.strip_prefix("row.")?;
            let i = ui.items.iter().position(|x| x.id == id)? as i32;
            return ui.hits.h.iter().find(|h| h.id == H_ROW && h.arg == i).map(|h| ((h.r.x + h.r.w / 2) as f32, (h.r.y + h.r.h / 2) as f32));
        }
    };
    ui.hits.h.iter().find(|h| h.id == id).map(|h| ((h.r.x + h.r.w / 2) as f32, (h.r.y + h.r.h / 2) as f32))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_carry_the_version_only_when_there_is_one() {
        assert_eq!(labeled("Timer", "1.0.0"), "Timer (1.0.0)");
        assert_eq!(labeled("Timer", ""), "Timer");
    }
}
