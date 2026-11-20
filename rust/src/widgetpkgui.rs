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
const H_UPDATE_ALL: i32 = 111;
const H_LICENSE_VIEW: i32 = 112;
const H_QR: i32 = 113;
const H_LICENSE_PREV: i32 = 114;
const H_LICENSE_NEXT: i32 = 115;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PkgView {
    #[default]
    List,
    Detail,
    Install,
    Busy,
    Trust,
    License,
    Qr,
    Remove,
    Error,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FetchStage {
    Branch,
    Manifest,
    /// a license file named by the manifest or found beside it (one the manifest named must read,
    /// one only discovered may be absent)
    LicenseProbe,
    Artifact,
}

struct InstallJob {
    stage: FetchStage,
    fetch: WidgetFetch,
    url: String,
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
    /// the control under the contact and the pressed look it got, as every Settings row's
    pressed: Option<Rect>,
    pressed_hit: Option<Hit>,
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
    /// the selected package's own manifest, re-read when its detail page opens (its author and
    /// license are not all in the index)
    detail_manifest: Option<WidgetPackage>,
    /// the license files to try, how many were tried, and whether they were only a search (a
    /// LICENSE file beside the manifest) rather than a license_url the manifest promised
    license_candidates: Vec<String>,
    license_try: usize,
    license_required: bool,
    /// the License page was opened from the detail page's View license, not the install flow
    license_viewer: bool,
    /// the package whose license text is loaded, so a second View license does not fetch again
    license_for: String,
    /// the license text's page (0-based), for a license longer than one page
    license_page: i32,
    /// the GitHub repository page the QR view shows
    qr_url: String,
    job: Option<InstallJob>,
    /// Update all: the packages left to check, how many were updated and failed, and the last
    /// failure's text (the summary names it if nothing updated).
    update_all: bool,
    update_queue: Vec<WidgetInstall>,
    update_pos: usize,
    update_ok: i32,
    update_failed: i32,
    update_note: String,
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
    ui.detail_manifest = None;
    ui.license_candidates.clear();
    ui.license_try = 0;
    ui.license_required = false;
    ui.license_viewer = false;
    ui.license_for.clear();
    ui.license_page = 0;
    ui.qr_url.clear();
    ui.manifest = None;
    ui.manifest_yaml.clear();
    ui.input.clear();
    ui.updating = false;
    ui.job = None;
    ui.pressed = None;
    ui.pressed_hit = None;
    ui.update_all = false;
    ui.update_queue.clear();
    ui.update_pos = 0;
    ui.update_ok = 0;
    ui.update_failed = 0;
    ui.update_note.clear();
    ui.refresh();
    crate::library::packages_redraw(a);
}

fn clear_update_all(ui: &mut WidgetPkgUi) {
    ui.update_all = false;
    ui.update_queue.clear();
    ui.update_pos = 0;
    ui.update_ok = 0;
    ui.update_failed = 0;
    ui.update_note.clear();
}

fn close(a: &mut App) {
    let ui = &mut a.pkg;
    ui.open = false;
    ui.job = None;
    clear_update_all(ui);
    keyboard_close(&mut ui.kb);
    crate::library::packages_closed(a);
}

/// Back from the first page: Settings itself, as a Settings submenu's Back does.
fn back_to_settings(a: &mut App) {
    let ui = &mut a.pkg;
    ui.open = false;
    ui.job = None;
    clear_update_all(ui);
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

/// A package's own icon: the manifest's name, else its widget definition's (a calculator package
/// installed before icons were written into the index still shows the calculator), else the grid.
fn install_icon(it: &WidgetInstall) -> IconKind {
    if !it.icon.trim().is_empty() {
        return widget_icon(&it.icon);
    }
    widget_def(&it.id).map_or(IconKind::Grid, |d| d.icon)
}

fn stage_status(stage: FetchStage, of: Option<(usize, usize)>) -> String {
    let what = match stage {
        FetchStage::Branch => tr("Looking up the repository..."),
        FetchStage::Manifest => tr("Reading package.yaml..."),
        FetchStage::LicenseProbe => tr("Reading the license..."),
        FetchStage::Artifact => tr("Downloading the widget..."),
    };
    match of {
        Some((n, all)) => format!("{what} ({n}/{all})"),
        None => what.to_string(),
    }
}

/// Starts a fetch and makes the page Busy. Err leaves the page as it was: a caller that has more to
/// do (Update all) counts it, and one that does not shows it (start_job).
fn new_job(ui: &mut WidgetPkgUi, stage: FetchStage, url: &str, input: &str, manifest_url: &str, of: Option<(usize, usize)>) -> Result<(), String> {
    let cap = if stage == FetchStage::Artifact { WIDGET_PKG_ARTIFACT_MAX as usize } else { 512 * 1024 };
    let fetch = WidgetFetch::start(url, cap)?;
    ui.status = stage_status(stage, of);
    ui.view = PkgView::Busy;
    ui.job = Some(InstallJob {
        stage,
        fetch,
        url: url.to_string(),
        input: input.to_string(),
        manifest_url: manifest_url.to_string(),
    });
    Ok(())
}

fn start_job(ui: &mut WidgetPkgUi, stage: FetchStage, url: String, input: &str, manifest_url: &str) {
    if let Err(e) = new_job(ui, stage, &url, input, manifest_url, None) {
        ui.error = e;
        ui.view = PkgView::Error;
        ui.job = None;
    }
}

/// The license files a repository may keep beside its manifest, in the order tried.
const LICENSE_FILES: [&str; 5] = ["LICENSE", "LICENSE.md", "LICENSE.txt", "COPYING", "COPYING.md"];

fn license_urls(base: &str) -> Vec<String> {
    LICENSE_FILES.iter().map(|name| format!("{base}/{name}")).collect()
}

/// The license files to try for a manifest that is being installed. A `license_url` is the only
/// answer; a manifest that names a license (`license: MIT`) but no URL gets its repository's
/// LICENSE file, so the agreement shows without the author having to write the URL.
fn manifest_license_candidates(manifest_url: &str, p: &WidgetPackage) -> Vec<String> {
    if !p.license_url.is_empty() {
        return vec![p.license_url.clone()];
    }
    if p.license.is_empty() {
        return Vec::new();
    }
    let base = manifest_url.rsplit_once('/').map_or("", |(b, _)| b);
    if base.is_empty() {
        return Vec::new();
    }
    license_urls(base)
}

/// The same for a package that is already installed: its `license_url`, else the LICENSE file of
/// the repository it came from (a direct `.yaml` address or a GitHub repository).
fn installed_license_candidates(source: &str, p: &WidgetPackage) -> Vec<String> {
    if !p.license_url.is_empty() {
        return vec![p.license_url.clone()];
    }
    if p.license.is_empty() {
        return Vec::new();
    }
    let s = source.trim();
    let mut out = Vec::new();
    if s.starts_with("https://") || s.starts_with("http://") {
        let path = s.split(['?', '#']).next().unwrap_or(s);
        if path.to_ascii_lowercase().ends_with(".yaml") {
            if let Some((base, _)) = path.rsplit_once('/') {
                out.extend(license_urls(base));
            }
            return out;
        }
    }
    if let Some(repo) = widget_github_repo(s) {
        // (the branch the install used is not kept, so try GitHub's two usual defaults)
        for branch in ["main", "master"] {
            out.extend(license_urls(&format!("https://raw.githubusercontent.com/{repo}/{branch}")));
        }
    }
    out
}

/// Starts the next license fetch. False when no candidate is left (or none starts at all).
fn start_next_license(ui: &mut WidgetPkgUi) -> bool {
    while ui.license_try < ui.license_candidates.len() {
        let url = ui.license_candidates[ui.license_try].clone();
        ui.license_try += 1;
        let (input, manifest_url) = (ui.input.clone(), ui.manifest_url.clone());
        if new_job(ui, FetchStage::LicenseProbe, &url, &input, &manifest_url, None).is_ok() {
            return true;
        }
    }
    false
}

/// No license file answered: the viewer says so, while an install goes on (the manifest only
/// named a license, not a file that must be read).
fn license_exhausted(ui: &mut WidgetPkgUi) {
    ui.job = None;
    if ui.license_viewer {
        ui.license_text.clear();
        ui.view = PkgView::License;
    } else {
        start_artifact(ui);
    }
}

fn start_artifact(ui: &mut WidgetPkgUi) {
    let url = ui.artifact_url.clone();
    let input = ui.input.clone();
    let manifest_url = ui.manifest_url.clone();
    start_job(ui, FetchStage::Artifact, url, &input, &manifest_url);
}

fn begin_install(ui: &mut WidgetPkgUi, input: &str) {
    let input = input.trim().to_string();
    ui.input = input.clone();
    ui.updating = false;
    ui.update_all = false;
    ui.license_viewer = false;
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

/// The repository address to check an installed package from. A package installed before the
/// index kept repositories may say `https://github.com/user/repo`: its manifest is the raw YAML
/// there, and the update asks the repository for its default branch first.
fn update_repo_input(source: &str) -> String {
    let s = source.trim();
    let rest = s
        .strip_prefix("https://github.com/")
        .or_else(|| s.strip_prefix("http://github.com/"));
    if let Some(rest) = rest {
        let rest = rest.trim_end_matches('/');
        let rest = rest.strip_suffix(".git").unwrap_or(rest);
        let parts: Vec<&str> = rest.split('/').filter(|p| !p.is_empty()).collect();
        if parts.len() == 2 {
            return format!("{}/{}", parts[0], parts[1]);
        }
    }
    s.to_string()
}

/// Starts an update check from an installed package's repository (or its raw .yaml), resolving
/// GitHub's default branch when the source is a repository rather than a direct manifest.
fn start_update_fetch(ui: &mut WidgetPkgUi, input: &str, of: Option<(usize, usize)>) -> Result<(), String> {
    ui.input = input.to_string();
    if let Some(api) = widget_github_api(input) {
        return new_job(ui, FetchStage::Branch, &api, input, "", of);
    }
    let (url, _) = widget_manifest_url(input)?;
    new_job(ui, FetchStage::Manifest, &url, input, &url, of)
}

fn begin_update(ui: &mut WidgetPkgUi, it: &WidgetInstall) {
    ui.updating = true;
    ui.update_all = false;
    ui.license_viewer = false;
    if it.source_url.is_empty() {
        ui.error = "This package was installed by hand; there is no repository to check.".into();
        ui.view = PkgView::Error;
        return;
    }
    let input = update_repo_input(&it.source_url);
    if let Err(e) = start_update_fetch(ui, &input, None) {
        ui.error = e;
        ui.view = PkgView::Error;
        ui.job = None;
    }
}

/// Update all: checks every installed package that came from a repository, in list order, and
/// installs the newer ones as their manifests and artifacts arrive. The packages were trusted when
/// they were installed, so this does not ask again for each one.
fn begin_update_all(ui: &mut WidgetPkgUi) {
    ui.updating = false;
    ui.update_all = false;
    ui.license_viewer = false;
    ui.refresh();
    ui.update_queue = ui.items.iter().filter(|it| !it.source_url.is_empty()).cloned().collect();
    if ui.update_queue.is_empty() {
        ui.status = "No widget packages have a repository to update from.".into();
        ui.view = PkgView::List;
        return;
    }
    ui.update_all = true;
    ui.update_pos = 0;
    ui.update_ok = 0;
    ui.update_failed = 0;
    ui.update_note.clear();
    ui.selected = None;
    ui.view = PkgView::Busy;
    update_all_next(ui);
}

fn update_all_next(ui: &mut WidgetPkgUi) {
    while ui.update_pos < ui.update_queue.len() {
        let it = ui.update_queue[ui.update_pos].clone();
        let n = ui.update_pos + 1;
        let all = ui.update_queue.len();
        ui.update_pos += 1;
        ui.status = format!("Checking {}... ({n}/{all})", it.name);
        let input = update_repo_input(&it.source_url);
        if let Err(e) = start_update_fetch(ui, &input, Some((n, all))) {
            ui.update_failed += 1;
            ui.update_note = e;
            continue;
        }
        ui.status = format!("Checking {}... ({n}/{all})", it.name);
        return;
    }
    update_all_done(ui);
}

fn update_all_done(ui: &mut WidgetPkgUi) {
    let (ok, failed) = (ui.update_ok, ui.update_failed);
    let note = std::mem::take(&mut ui.update_note);
    ui.update_all = false;
    ui.update_queue.clear();
    ui.update_pos = 0;
    ui.update_ok = 0;
    ui.update_failed = 0;
    ui.job = None;
    ui.view = PkgView::List;
    ui.selected = None;
    ui.refresh();
    ui.status = if ok == 0 && failed == 0 {
        "All widget packages are up to date.".into()
    } else if failed == 0 {
        format!("Updated {ok} widget package{}.", if ok == 1 { "" } else { "s" })
    } else if ok == 0 && !note.is_empty() {
        format!("Could not update: {note}")
    } else {
        format!("Updated {ok} widget package{}; {failed} failed.", if ok == 1 { "" } else { "s" })
    };
}

fn after_manifest(ui: &mut WidgetPkgUi, data: &[u8], a: &App, manifest_url: &str) {
    let text = String::from_utf8_lossy(data);
    let p = match widget_package_parse(&text).and_then(|p| {
        widget_package_validate(&p, a.w, a.h - status_reserved(a)).map(|_| p)
    }) {
        Ok(p) => p,
        Err(e) => {
            if ui.update_all {
                ui.update_failed += 1;
                ui.update_note = e;
                update_all_next(ui);
            } else {
                ui.error = e;
                ui.view = PkgView::Error;
            }
            return;
        }
    };
    let artifact = if let Some(u) = widget_package_artifact_url(manifest_url, &p) {
        u
    } else {
        if ui.update_all {
            ui.update_failed += 1;
            ui.update_note = "The manifest names no artifact that can be downloaded.".into();
            update_all_next(ui);
        } else {
            ui.error = "The manifest names no artifact that can be downloaded.".into();
            ui.view = PkgView::Error;
        }
        return;
    };
    if ui.update_all {
        let Some(old) = ui.items.iter().find(|x| x.id == p.id).cloned() else {
            // A repository may not turn an update into a new, untrusted package.
            ui.update_failed += 1;
            ui.update_note = "The repository no longer names the installed package.".into();
            update_all_next(ui);
            return;
        };
        if update_version_cmp(&p.version, &old.version) <= 0 {
            ui.status = format!("{} is up to date.", old.name);
            update_all_next(ui);
            return;
        }
        ui.license_url = p.license_url.clone();
        ui.artifact_url = artifact.clone();
        ui.manifest_url = manifest_url.to_string();
        ui.manifest = Some(p);
        ui.status.clear();
        let of = Some((ui.update_pos, ui.update_queue.len()));
        let mu = manifest_url.to_string();
        if let Err(e) = new_job(ui, FetchStage::Artifact, &artifact, &mu, &mu, of) {
            ui.update_failed += 1;
            ui.update_note = e;
            update_all_next(ui);
        }
        return;
    }
    if ui.updating {
        if let Some(old) = widget_package_list().into_iter().find(|x| x.id == p.id) {
            if update_version_cmp(&p.version, &old.version) <= 0 {
                ui.status = format!("{} is up to date ({}).", old.name, old.version);
                ui.view = PkgView::Detail;
                return;
            }
        }
    }
    ui.license_candidates = manifest_license_candidates(manifest_url, &p);
    ui.license_try = 0;
    ui.license_required = !p.license_url.is_empty();
    ui.license_viewer = false;
    ui.license_for.clear();
    ui.license_text.clear();
    ui.license_page = 0;
    ui.license_url = p.license_url.clone();
    ui.artifact_url = artifact;
    ui.manifest_url = manifest_url.to_string();
    ui.manifest = Some(p);
    ui.status.clear();
    ui.view = PkgView::Trust;
}

/// An open frame of a package that just changed takes the new definition now, not at its next open.
fn apply_new_def(a: &mut App, it: &WidgetInstall) {
    if let Some(ed) = a.ed.as_deref_mut() {
        if let Some(i) = ed.widgets.index_of(&it.id) {
            if let Some(def) = widget_def(&it.id) {
                ed.widgets.frames[i].def = def;
            }
        }
    }
}

fn after_artifact(ui: &mut WidgetPkgUi, data: &[u8], a: &mut App) {
    if ui.manifest.is_none() || ui.manifest_yaml.is_empty() {
        ui.error = "The package's manifest was lost.".into();
        ui.view = PkgView::Error;
        return;
    }
    let manifest = ui.manifest_yaml.clone();
    let source = if ui.input.is_empty() { ui.manifest_url.as_str() } else { ui.input.as_str() };
    match widget_package_install(&manifest, data, source, a.w, a.h - status_reserved(a)) {
        Ok(it) => {
            apply_new_def(a, &it);
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

fn after_artifact_update_all(ui: &mut WidgetPkgUi, data: &[u8], a: &mut App) {
    if ui.manifest.is_none() || ui.manifest_yaml.is_empty() {
        ui.update_failed += 1;
        ui.update_note = "The package's manifest was lost.".into();
        update_all_next(ui);
        return;
    }
    let manifest = ui.manifest_yaml.clone();
    let source = if ui.input.is_empty() { ui.manifest_url.as_str() } else { ui.input.as_str() };
    match widget_package_install(&manifest, data, source, a.w, a.h - status_reserved(a)) {
        Ok(it) => {
            apply_new_def(a, &it);
            ui.update_ok += 1;
            update_all_next(ui);
        }
        Err(e) => {
            ui.update_failed += 1;
            ui.update_note = e;
            update_all_next(ui);
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
                        if ui.update_all {
                            ui.update_failed += 1;
                            ui.update_note = e;
                            update_all_next(&mut ui);
                        } else if job.stage == FetchStage::Branch {
                            // The GitHub API is an optimization: a failed lookup falls back to main.
                            let url = widget_manifest_url(&job.input).map(|(u, _)| u).unwrap_or_default();
                            if !url.is_empty() {
                                start_job(&mut ui, FetchStage::Manifest, url.clone(), &job.input, &url);
                            } else {
                                ui.error = e;
                                ui.view = PkgView::Error;
                            }
                        } else if job.stage == FetchStage::LicenseProbe {
                            if ui.license_required {
                                ui.error = e;
                                ui.view = PkgView::Error;
                            } else if !start_next_license(&mut ui) {
                                license_exhausted(&mut ui);
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
                        FetchStage::LicenseProbe => {
                            ui.license_text = String::from_utf8_lossy(&data).into_owned();
                            ui.license_url = job.url.clone();
                            ui.license_page = 0;
                            ui.view = PkgView::License;
                        }
                        FetchStage::Artifact => {
                            if ui.update_all {
                                after_artifact_update_all(&mut ui, &data, a);
                            } else {
                                after_artifact(&mut ui, &data, a);
                            }
                        }
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
        PkgView::Qr => {
            ui.view = PkgView::Detail;
        }
        PkgView::License if ui.license_viewer => {
            ui.view = PkgView::Detail;
            ui.license_viewer = false;
            ui.job = None;
        }
        PkgView::Trust | PkgView::License | PkgView::Busy => {
            ui.view = PkgView::List;
            ui.job = None;
            clear_update_all(&mut ui);
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
                ui.selected = Some(it.id.clone());
                ui.detail_manifest = widget_package_installed_manifest(&it.id);
                ui.manifest = None;
                ui.license_text.clear();
                ui.license_for.clear();
                ui.qr_url.clear();
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
            if !start_next_license(&mut ui) {
                if ui.license_required {
                    ui.error = "The manifest's license address could not be fetched.".into();
                    ui.view = PkgView::Error;
                } else {
                    start_artifact(&mut ui);
                }
            }
        }
        H_LICENSE_YES => start_artifact(&mut ui),
        H_LICENSE_VIEW => {
            if let Some(it) = ui.items.iter().find(|x| Some(&x.id) == ui.selected.as_ref()).cloned() {
                if ui.license_for == it.id && !ui.license_text.is_empty() {
                    ui.license_page = 0;
                    ui.view = PkgView::License;
                    ui.status.clear();
                } else {
                    ui.license_viewer = true;
                    ui.license_text.clear();
                    ui.license_page = 0;
                    ui.license_for = it.id.clone();
                    ui.status.clear();
                    ui.error.clear();
                    ui.license_candidates = widget_package_installed_manifest(&it.id)
                        .map(|m| installed_license_candidates(&it.source_url, &m))
                        .unwrap_or_default();
                    ui.license_try = 0;
                    ui.license_required = false;
                    if !start_next_license(&mut ui) {
                        ui.view = PkgView::License;
                    }
                }
            }
        }
        H_LICENSE_PREV => ui.license_page = (ui.license_page - 1).max(0),
        H_LICENSE_NEXT => ui.license_page += 1,
        H_QR => {
            if let Some(it) = ui.items.iter().find(|x| Some(&x.id) == ui.selected.as_ref()).cloned() {
                let repo = ui
                    .detail_manifest
                    .as_ref()
                    .and_then(|m| widget_github_repo_url(&m.repository))
                    .or_else(|| widget_github_repo_url(&it.source_url));
                if let Some(url) = repo {
                    ui.qr_url = url;
                    ui.view = PkgView::Qr;
                }
            }
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
        H_UPDATE_ALL => begin_update_all(&mut ui),
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
        a.pkg.pressed = None;
        a.pkg.pressed_hit = None;
        // Exactly a Settings row's pressed look: the control under the contact, in the panel now.
        if let Some(h) = hits_find(&a.pkg.hits, ev.x, ev.y).copied() {
            app_press_feedback(a, h.r);
            a.pkg.pressed = Some(h.r);
            a.pkg.pressed_hit = Some(h);
        }
        return;
    }
    press_move(&mut a.pkg.press, ev.x, ev.y);
    if ph != 2 {
        return;
    }
    // The lift takes the pressed look off and refreshes only that row; a contact that moved or
    // touched nothing does no more than that (the page is not drawn again).
    let mut pressed = a.pkg.pressed.take();
    let back = app_press_release(a, &mut pressed);
    let cancelled = touch_cancelled(ev);
    let hit = a.pkg.pressed_hit.take().filter(|h| !cancelled && press_is_tap(&a.pkg.press, h.r));
    if let Some(h) = hit {
        do_tap(a, h);
        return;
    }
    if !rect_is_empty(back) {
        app_invalidate(a, back, RefreshMode::Ui);
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

/// One action row, disabled (and without a hit) when `enabled` is false, as a pager's end arrow.
fn action_row(c: &mut Canvas, ui: &mut WidgetPkgUi, r: Rect, label: &str, icon: IconKind, id: i32, enabled: bool) {
    let mut row = Row::new(icon, label);
    if !enabled {
        row.style = ButtonStyle::Disabled;
    }
    ui_row(c, r, &row);
    if enabled {
        hits_add(&mut ui.hits, r, id, 0);
    }
}

/// A license's text as lines that fit `width` at `px`, at its own line breaks (a license therefore
/// keeps its paragraphs instead of shrinking to one size-fit block). A word longer than the page is
/// broken between its characters.
fn license_lines(text: &str, px: f32, width: i32) -> Vec<String> {
    let f = FontFace::Regular;
    let space = font_measure(f, px, " ");
    let mut out: Vec<String> = Vec::new();
    for raw in text.lines() {
        let raw = raw.trim_end_matches('\r');
        if raw.trim().is_empty() {
            out.push(String::new());
            continue;
        }
        let mut line = String::new();
        let mut w = 0i32;
        for word in raw.split_whitespace() {
            let ww = font_measure(f, px, word);
            if !line.is_empty() && w + space + ww > width {
                out.push(std::mem::take(&mut line));
                w = 0;
            }
            if !line.is_empty() {
                line.push(' ');
                w += space;
            }
            if ww <= width {
                line.push_str(word);
                w += ww;
                continue;
            }
            let mut rest = word;
            while font_measure(f, px, rest) > width {
                let mut cut = 0usize;
                for (i, _) in rest.char_indices().skip(1) {
                    if font_measure(f, px, &rest[..i]) > width {
                        break;
                    }
                    cut = i;
                }
                if cut == 0 {
                    break;
                }
                out.push(rest[..cut].to_string());
                rest = &rest[cut..];
            }
            line.push_str(rest);
            w = font_measure(f, px, rest);
        }
        out.push(line);
    }
    if out.is_empty() {
        out.push(String::new());
    }
    out
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
        PkgView::Busy if a.pkg.license_viewer => tr("License"),
        PkgView::Busy => tr("Installing"),
        PkgView::Trust => tr("Install this widget?"),
        PkgView::License if a.pkg.license_viewer => tr("License"),
        PkgView::License => tr("License agreement"),
        PkgView::Qr => tr("Repository"),
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
            let actions = 2 + n;
            let rows_area = Rect { x: content.x, y: content.y, w: content.w, h: bottom - content.y };
            let list = ui_row_page(rows_area, actions, ui.page, ui_px(118.0), ui_px(74.0));
            ui.page = list.page;
            for (i, rr) in list.rows.iter().enumerate() {
                let k = list.first + i;
                if k == 0 {
                    // The one package-list-wide action, above the packages it updates.
                    let row = Row::new(IconKind::Update, tr("Update all"));
                    ui_row(c, *rr, &row);
                    hits_add(&mut ui.hits, *rr, H_UPDATE_ALL, 0);
                } else if k == 1 {
                    let row = Row::new(IconKind::Download, tr("Install from repository..."));
                    ui_row(c, *rr, &row);
                    hits_add(&mut ui.hits, *rr, H_INSTALL, 0);
                } else if let Some(it) = ui.items.get(k - 2) {
                    let label = labeled(&it.name, &it.version);
                    let row = Row::new(install_icon(it), &label);
                    ui_row(c, *rr, &row);
                    hits_add(&mut ui.hits, *rr, H_ROW, (k - 2) as i32);
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
            let icon = Rect { x: content.x, y: content.y, w: ui_px(58.0), h: ui_px(58.0) };
            ui_icon(c, install_icon(&it), icon, 0);
            let tx = icon.x + icon.w + ui_px(18.0);
            ui_text(c, FontFace::Bold, ui_text_size(), ui_small_size(), Rect { x: tx, y: content.y, w: content.w - (tx - content.x), h: ui_px(78.0) }, &title, 1, false, 0);
            let m = ui.detail_manifest.as_ref();
            let author = m.map(|m| m.author.as_str()).filter(|s| !s.is_empty()).unwrap_or(&it.author);
            let license = m.map(|m| m.license.as_str()).unwrap_or("");
            let source = m.map(|m| m.repository.as_str()).filter(|s| !s.is_empty()).unwrap_or(&it.source_url);
            let repo = widget_github_repo_url(source).unwrap_or_else(|| source.to_string());
            let github = m.and_then(|m| widget_github_repo_url(&m.repository)).or_else(|| widget_github_repo_url(&it.source_url));
            let mut line = String::new();
            let mut y = content.y + ui_px(82.0);
            if !author.is_empty() {
                csnprintf!(&mut line, 512, "%s %s", tr("Author:"), author);
                body_text(c, Rect { x: content.x, y, w: content.w, h: ui_px(46.0) }, &line);
                y += ui_px(48.0);
            }
            if !it.description.is_empty() {
                body_text(c, Rect { x: content.x, y, w: content.w, h: ui_px(100.0) }, &it.description);
                y += ui_px(104.0);
            }
            if !license.is_empty() {
                csnprintf!(&mut line, 512, "%s %s", tr("License:"), license);
                body_text(c, Rect { x: content.x, y, w: content.w, h: ui_px(46.0) }, &line);
                y += ui_px(48.0);
            }
            if !repo.is_empty() {
                csnprintf!(&mut line, 1100, "%s %s", tr("Repository:"), &repo);
                body_text(c, Rect { x: content.x, y, w: content.w, h: ui_px(46.0) }, &line);
                y += ui_px(48.0);
            }
            if !ui.status.is_empty() {
                // Its own row above the actions: the buttons used to start inside this text.
                body_text(c, Rect { x: content.x, y, w: content.w, h: ui_px(70.0) }, &ui.status);
                y += ui_px(70.0);
            }
            let mut actions: Vec<(&str, IconKind, i32)> = vec![(tr("Check for updates"), IconKind::Update, H_UPDATE)];
            if !license.is_empty() || m.is_some_and(|m| !m.license_url.is_empty()) {
                actions.push((tr("View license"), IconKind::Paper, H_LICENSE_VIEW));
            }
            if github.is_some() {
                actions.push((tr("View QR code"), IconKind::Link, H_QR));
            }
            actions.push((tr("Remove..."), IconKind::Close, H_REMOVE));
            draw_actions(c, ui, Rect { x: content.x, y: content.y, w: content.w, h: bottom - content.y }, y + ui_px(20.0), &actions);
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
            let name = ui
                .manifest
                .as_ref()
                .map(|m| m.license.as_str())
                .or_else(|| ui.detail_manifest.as_ref().map(|m| m.license.as_str()))
                .unwrap_or("");
            let head = if name.is_empty() { tr("License").to_string() } else { name.to_string() };
            ui_text(c, FontFace::Bold, ui_small_size(), ui_small_size(), Rect { x: content.x, y: content.y, w: content.w, h: ui_px(80.0) }, &head, 2, false, 0);
            let text = if ui.license_text.is_empty() {
                tr("No license text was found in the repository.")
            } else {
                ui.license_text.as_str()
            };
            let px = ui_text_size();
            let line_h = (font_ascent(FontFace::Regular, px) + font_descent(FontFace::Regular, px)).max(1);
            let bh = ui_px(112.0);
            let gap = ui_px(18.0);
            // The pager (two rows) and an agreement's button keep their room, so the text never
            // runs under them.
            let reserve_rows = if ui.license_viewer { 2 } else { 3 };
            let reserve = reserve_rows * bh + (reserve_rows - 1) * gap + ui_px(16.0);
            let body = Rect {
                x: content.x,
                y: content.y + ui_px(80.0),
                w: content.w,
                h: (bottom - content.y - ui_px(80.0) - reserve).max(line_h),
            };
            let lines = license_lines(text, px, content.w);
            let per = (body.h / line_h).max(1);
            let pages = (((lines.len() as i32) + per - 1) / per).max(1);
            let page = ui.license_page.clamp(0, pages - 1);
            ui.license_page = page;
            let from = page as usize * per as usize;
            let to = (from + per as usize).min(lines.len());
            for (i, line) in lines[from..to].iter().enumerate() {
                font_draw_left(c, FontFace::Regular, px, Rect { x: body.x, y: body.y + i as i32 * line_h, w: body.w, h: line_h }, line, 0);
            }
            let rows = (if pages > 1 { 2 } else { 0 }) + i32::from(!ui.license_viewer);
            if rows > 0 {
                let mut y = bottom - (rows * bh + (rows - 1) * gap);
                if pages > 1 {
                    action_row(c, ui, Rect { x: content.x, y, w: content.w, h: bh }, tr("Previous page"), IconKind::Prev, H_LICENSE_PREV, page > 0);
                    y += bh + gap;
                    action_row(c, ui, Rect { x: content.x, y, w: content.w, h: bh }, tr("Next page"), IconKind::Next, H_LICENSE_NEXT, page + 1 < pages);
                    y += bh + gap;
                }
                if !ui.license_viewer {
                    action_row(c, ui, Rect { x: content.x, y, w: content.w, h: bh }, tr("I have read and agree"), IconKind::Download, H_LICENSE_YES, true);
                }
            }
        }
        PkgView::Qr => {
            font_draw_left(c, FontFace::Regular, ui_text_size(), Rect { x: content.x, y: content.y, w: content.w, h: ui_px(70.0) }, tr("Scan with a phone's camera"), 0);
            let top = content.y + ui_px(110.0);
            let side = (bottom - top - ui_px(170.0)).max(0).min(content.w);
            let sym = crate::qr::ui_qr(c, Rect { x: content.x + (content.w - side) / 2, y: top, w: side, h: side }, &ui.qr_url, false);
            let ty = if rect_is_empty(sym) { top } else { sym.y + sym.h + ui_px(30.0) };
            ui_text(c, FontFace::Regular, ui_text_size(), ui_small_size(), Rect { x: content.x, y: ty, w: content.w, h: ui_px(90.0) }, &ui.qr_url, 2, true, 0);
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
        "update-all" => H_UPDATE_ALL,
        "trust" => H_TRUST,
        "license-yes" => H_LICENSE_YES,
        "license-view" => H_LICENSE_VIEW,
        "license-prev" => H_LICENSE_PREV,
        "license-next" => H_LICENSE_NEXT,
        "qr" => H_QR,
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
