//! The WebAssembly widget host (2.2): a generic frame body, nothing about any one widget.
//!
//! A package's artifact is a WebAssembly module with a `memory` export and two exports the host
//! calls:
//!
//! ```text
//! widget_draw(width: i32, height: i32, scale_milli: i32)
//! widget_tap(id: i32)
//! ```
//!
//! `widget_draw` draws through the `grimoire` imports below. Coordinates are screen pixels inside
//! the frame's body; `scale_milli` is the UI scale times 1000, so a package can keep 300 dpi sizes.
//! `widget_tap` receives the id of the hit region under the tap, or -1 for a tap elsewhere in the
//! body. A package keeps its state through `state_get` / `state_set`, which the host saves under
//! `packages/widgets/<id>/state/runtime.bin`.
//!
//! Imports (`grimoire`):
//!
//! ```text
//! rect(x, y, w, h, style)
//! text(x, y, w, h, ptr, len, px, center)
//! button(id, x, y, w, h, ptr, len, style)
//! hit(id, x, y, w, h)
//! status(ptr, len)
//! request_redraw(after_ms)
//! setting(key_ptr, key_len, out_ptr, out_cap) -> i32
//! state_get(ptr, cap) -> i32
//! state_set(ptr, len)
//! now() -> i64
//! ```
//!
//! `style` is 0 (normal), 1 (selected) or 2 (disabled). Strings are UTF-8 in the module's memory.
//! `setting` and `state_get` return the number of bytes written, at most `out_cap` / `cap`.

use crate::prelude::*;
use std::time::{Duration, Instant};
use wasmi::{Caller, Config, Engine, Extern, Linker, Module, Store, StoreLimits, StoreLimitsBuilder};

const ABI: &str = "grimoire";
const ARTIFACT_MAX: usize = 32 * 1024 * 1024;
const STATE_MAX: usize = 64 * 1024;
const STATE_FILE: &str = "state/runtime.bin";
/// A widget gets this many instructions per call. A drawing pass that needs more is looping.
const FUEL_PER_CALL: u64 = 20_000_000;
/// A widget's linear memory may not grow past this, so one package cannot exhaust the device.
const MEMORY_MAX: usize = 32 * 1024 * 1024;

/// A drawing command recorded by the module and rendered by the host. Coordinates are body-local.
#[derive(Clone, Debug)]
enum DrawCmd {
    Rect { x: i32, y: i32, w: i32, h: i32, style: i32 },
    Text { x: i32, y: i32, w: i32, h: i32, text: String, px: i32, center: bool },
    Button { x: i32, y: i32, w: i32, h: i32, text: String, style: i32 },
}

struct HostState {
    commands: Vec<DrawCmd>,
    hits: Vec<(i32, Rect)>,
    state: Vec<u8>,
    state_path: String,
    settings: Vec<(String, String)>,
    status: String,
    dirty: bool,
    limits: StoreLimits,
    redraw_at: Option<Instant>,
}

/// One open package's WebAssembly instance, its last draw and its saved state.
pub struct WidgetRuntime {
    _engine: Engine,
    store: Store<HostState>,
    instance: wasmi::Instance,
    hits: Vec<(i32, Rect)>,
    texts: Vec<String>,
    status: String,
    error: Option<String>,
}

impl std::fmt::Debug for WidgetRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WidgetRuntime").field("hits", &self.hits.len()).field("error", &self.error).finish()
    }
}

impl WidgetRuntime {
    /// Compiles and instantiates the installed package's artifact.
    pub fn new(id: &str) -> Result<Self, String> {
        let Some(m) = crate::widgetpkg::widget_package_installed_manifest(id) else {
            return Err("The widget package is not installed.".into());
        };
        if m.spec.runtime != "wasm" {
            return Err("This package is not a WebAssembly widget.".into());
        }
        let path = format!("{}/{}", crate::widgetpkg::widget_package_dir(id), m.artifact.path);
        let bytes = read_file_max(&path, ARTIFACT_MAX).ok_or("The widget's artifact could not be read.")?;
        let mut config = Config::default();
        config.consume_fuel(true);
        let engine = Engine::new(&config);
        let module = Module::new(&engine, &bytes[..]).map_err(|e| format!("The widget's WebAssembly is invalid: {e}"))?;
        let state_path = format!("{}/{STATE_FILE}", crate::widgetpkg::widget_package_dir(id));
        let state = read_file_max(&state_path, STATE_MAX).unwrap_or_default();
        let settings = crate::widgetpkg::widget_settings_load(id, &m.settings);
        let mut store = Store::new(&engine, HostState {
            commands: Vec::new(),
            hits: Vec::new(),
            state,
            state_path,
            settings,
            status: String::new(),
            dirty: false,
            limits: StoreLimitsBuilder::new().memory_size(MEMORY_MAX).build(),
            redraw_at: None,
        });
        store.limiter(|st| &mut st.limits);
        // The start section runs here, before a frame can draw: give it the same budget as a call.
        let _ = store.set_fuel(FUEL_PER_CALL);
        let mut linker = Linker::new(&engine);
        linker.func_wrap(ABI, "rect", |mut caller: Caller<'_, HostState>, x: i32, y: i32, w: i32, h: i32, style: i32| {
            caller.data_mut().commands.push(DrawCmd::Rect { x, y, w, h, style });
        }).map_err(|e| e.to_string())?;
        linker.func_wrap(ABI, "text", |mut caller: Caller<'_, HostState>, x: i32, y: i32, w: i32, h: i32, ptr: i32, len: i32, px: i32, center: i32| {
            let text = String::from_utf8_lossy(&read_mem(&caller, ptr, len)).into_owned();
            caller.data_mut().commands.push(DrawCmd::Text { x, y, w, h, text, px, center: center != 0 });
        }).map_err(|e| e.to_string())?;
        linker.func_wrap(ABI, "button", |mut caller: Caller<'_, HostState>, id: i32, x: i32, y: i32, w: i32, h: i32, ptr: i32, len: i32, style: i32| {
            let text = String::from_utf8_lossy(&read_mem(&caller, ptr, len)).into_owned();
            let st = caller.data_mut();
            st.commands.push(DrawCmd::Button { x, y, w, h, text, style });
            st.hits.push((id, Rect { x, y, w, h }));
        }).map_err(|e| e.to_string())?;
        linker.func_wrap(ABI, "hit", |mut caller: Caller<'_, HostState>, id: i32, x: i32, y: i32, w: i32, h: i32| {
            caller.data_mut().hits.push((id, Rect { x, y, w, h }));
        }).map_err(|e| e.to_string())?;
        linker.func_wrap(ABI, "status", |mut caller: Caller<'_, HostState>, ptr: i32, len: i32| {
            caller.data_mut().status = String::from_utf8_lossy(&read_mem(&caller, ptr, len)).into_owned();
        }).map_err(|e| e.to_string())?;
        linker.func_wrap(ABI, "request_redraw", |mut caller: Caller<'_, HostState>, after_ms: i32| {
            let after_ms = after_ms.clamp(16, 86_400_000) as u64;
            caller.data_mut().redraw_at = Some(Instant::now() + Duration::from_millis(after_ms));
        }).map_err(|e| e.to_string())?;
        linker.func_wrap(ABI, "setting", |mut caller: Caller<'_, HostState>, key_ptr: i32, key_len: i32, out_ptr: i32, out_cap: i32| -> i32 {
            let key = read_mem(&caller, key_ptr, key_len);
            let value = caller.data().settings.iter().find(|(k, _)| k.as_bytes() == key.as_slice()).map(|(_, v)| v.as_bytes().to_vec()).unwrap_or_default();
            write_mem(&mut caller, out_ptr, out_cap, &value)
        }).map_err(|e| e.to_string())?;
        linker.func_wrap(ABI, "state_get", |mut caller: Caller<'_, HostState>, ptr: i32, cap: i32| -> i32 {
            let state = caller.data().state.clone();
            write_mem(&mut caller, ptr, cap, &state)
        }).map_err(|e| e.to_string())?;
        linker.func_wrap(ABI, "state_set", |mut caller: Caller<'_, HostState>, ptr: i32, len: i32| {
            let state = read_mem(&caller, ptr, len);
            let st = caller.data_mut();
            st.state = state.into_iter().take(STATE_MAX).collect();
            st.dirty = true;
        }).map_err(|e| e.to_string())?;
        linker.func_wrap(ABI, "now", || -> i64 {
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64)
        }).map_err(|e| e.to_string())?;
        let instance = linker.instantiate_and_start(&mut store, &module).map_err(|e| format!("The widget could not start: {e}"))?;
        if instance.get_memory(&store, "memory").is_none() {
            return Err("The widget has no memory export.".into());
        }
        Ok(WidgetRuntime { _engine: engine, store, instance, hits: Vec::new(), texts: Vec::new(), status: String::new(), error: None })
    }

    /// Calls `widget_draw` and renders its commands into `body`.
    pub fn draw(&mut self, c: &mut Canvas, body: Rect) {
        if self.error.is_some() {
            return;
        }
        let draw = match self.instance.get_typed_func::<(i32, i32, i32), ()>(&self.store, "widget_draw") {
            Ok(f) => f,
            Err(e) => { self.error = Some(format!("The widget has no widget_draw export: {e}")); return; }
        };
        self.store.data_mut().status.clear();
        self.store.data_mut().redraw_at = None;
        let _ = self.store.set_fuel(FUEL_PER_CALL);
        let scale = ui_px(1000.0);
        if let Err(e) = draw.call(&mut self.store, (body.w, body.h, scale)) {
            self.error = Some(format!("The widget stopped: {e}"));
            return;
        }
        self.save();
        let commands = std::mem::take(&mut self.store.data_mut().commands);
        self.hits = std::mem::take(&mut self.store.data_mut().hits);
        self.status = std::mem::take(&mut self.store.data_mut().status);
        self.texts.clear();
        canvas_fill(c, body, 255);
        for cmd in commands {
            match cmd {
                DrawCmd::Rect { x, y, w, h, style } => {
                    let r = Rect { x: body.x + x, y: body.y + y, w, h };
                    let fill = match style { 1 => 210, _ => 255 };
                    ui_frame(c, r, ui_px(8.0), fill, true, 0);
                    ui_frame(c, r, ui_px(8.0), 0, false, ui_px(2.0));
                }
                DrawCmd::Text { x, y, w, h, text, px, center } => {
                    let r = Rect { x: body.x + x, y: body.y + y, w, h };
                    let px = px.max(1) as f32;
                    ui_text(c, FontFace::Regular, px, px, r, &text, 1, center, 0);
                    self.texts.push(text);
                }
                DrawCmd::Button { x, y, w, h, text, style } => {
                    let r = Rect { x: body.x + x, y: body.y + y, w, h };
                    let style = match style { 1 => ButtonStyle::Selected, 2 => ButtonStyle::Disabled, _ => ButtonStyle::Plain };
                    ui_button(c, r, IconKind::None, &text, style);
                    self.texts.push(text);
                }
            }
        }
    }

    /// Sends a tap in body-local coordinates. Returns whether the module was called.
    pub fn tap(&mut self, x: i32, y: i32) {
        if self.error.is_some() {
            return;
        }
        let id = self.hits.iter().find(|(_, r)| rect_contains(*r, x, y)).map_or(-1, |(id, _)| *id);
        self.store.data_mut().redraw_at = None;
        let tap = match self.instance.get_typed_func::<(i32,), ()>(&self.store, "widget_tap") {
            Ok(f) => f,
            Err(_) => return,
        };
        let _ = self.store.set_fuel(FUEL_PER_CALL);
        if tap.call(&mut self.store, (id,)).is_err() {
            self.error = Some("The widget stopped while handling a tap.".into());
            return;
        }
        self.save();
    }

    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    /// Reads the package's settings again after the settings page saved them.
    pub fn reload_settings(&mut self, id: &str) {
        let Some(m) = crate::widgetpkg::widget_package_installed_manifest(id) else { return };
        self.store.data_mut().settings = crate::widgetpkg::widget_settings_load(id, &m.settings);
    }

    pub fn status(&self) -> &str {
        &self.status
    }

    /// The text drawn in the last frame, joined by `|`, for generic scenario probes.
    pub fn text(&self) -> String {
        self.texts.join("|")
    }

    /// A drawn hit region, for generic scenario taps.
    pub fn hit(&self, id: i32) -> Option<Rect> {
        self.hits.iter().find(|(hit, _)| *hit == id).map(|(_, r)| *r)
    }

    pub fn hit_count(&self) -> usize {
        self.hits.len()
    }

    /// Whether the package asked for its next draw by now.
    pub fn redraw_due(&self, now: Instant) -> bool {
        self.store.data().redraw_at.is_some_and(|at| at <= now)
    }

    fn save(&mut self) {
        if !self.store.data().dirty {
            return;
        }
        let st = self.store.data_mut();
        st.dirty = false;
        if let Some(dir) = st.state_path.rsplit_once('/').map(|(d, _)| d) {
            let _ = mkdir_p(dir);
        }
        if write_file_atomic(&st.state_path, &st.state) != 0 {
            st.dirty = true;
        }
    }
}

fn read_mem(caller: &Caller<'_, HostState>, ptr: i32, len: i32) -> Vec<u8> {
    if ptr < 0 || len <= 0 {
        return Vec::new();
    }
    let Some(memory) = caller.get_export("memory").and_then(Extern::into_memory) else { return Vec::new() };
    let data = memory.data(caller);
    let start = ptr as usize;
    let end = start.saturating_add(len as usize).min(data.len());
    if start >= end { return Vec::new() }
    data[start..end].to_vec()
}

fn write_mem(caller: &mut Caller<'_, HostState>, ptr: i32, cap: i32, bytes: &[u8]) -> i32 {
    if ptr < 0 || cap <= 0 {
        return 0;
    }
    let Some(memory) = caller.get_export("memory").and_then(Extern::into_memory) else { return 0 };
    let data = memory.data_mut(caller);
    let start = ptr as usize;
    if start >= data.len() {
        return 0;
    }
    let n = bytes.len().min(cap as usize).min(data.len() - start);
    data[start..start + n].copy_from_slice(&bytes[..n]);
    n as i32
}
