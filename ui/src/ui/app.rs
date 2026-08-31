use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use eframe::egui;

use crate::config::{AppConfig, AutosaveMode, GlowArea, GlowConfig};
use crate::download::{self, ToolProgress};
use crate::scan::{perform_scan, ScanPhase, ScanState};
use crate::theme::{ThemeColors, ThemeId};
use crate::tools::run_autocheck_search;

use super::dialogs;
use super::layout;
use super::sidebar;
use super::views;

pub(super) const SIDEBAR_WIDTH: f32 = 248.0;
pub(super) const FOOTER_HEIGHT: f32 = 36.0;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum View {
    Util(usize),
    Recycle,
    Components,
    AutoCheck,
    About,
    System,
    Settings,
}

#[derive(Clone)]
pub(super) enum ComponentStatus {
    Ready,
    Missing,
    Downloading { received: u64, total: Option<u64> },
    Verifying,
    Extracting,
    Failed(String),
}

pub struct CubeCheckApp {
    pub(super) config: AppConfig,
    pub(super) theme_id: ThemeId,
    pub(super) colors: ThemeColors,
    pub(super) view: View,
    pub(super) scan_state: Arc<Mutex<ScanState>>,
    pub(super) scan_started: bool,
    pub(super) scan_phase: Option<ScanPhase>,
    pub(super) saved_results: Vec<String>,
    pub(super) status_message: Option<(String, bool)>,
    pub(super) show_holy_confirm: bool,
    pub(super) show_clear_confirm: bool,
    pub(super) show_reset_confirm: bool,
    pub(super) reset_undo: Option<ResetUndo>,
    pub(super) reset_save_on_exit: bool,
    pub(super) component_state: Arc<Mutex<HashMap<String, ComponentStatus>>>,
    pub(super) download_busy: Arc<AtomicBool>,
    pub(super) auto_download_started: bool,
    #[allow(dead_code)]
    pub(super) inspect_id: String,
    #[allow(dead_code)]
    pub(super) inspect_rows: Vec<crate::tools::InspectRow>,
    last_zoom: f32,
    last_pixels_per_point: f32,
    zoom_apply_pending: bool,
    zoom_apply_frames: u8,
    exit_saved: bool,
}

pub(super) struct ResetUndo {
    pub snapshot: AppConfig,
    pub expires_at: Instant,
}

impl CubeCheckApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let config = AppConfig::load();
        let theme_id = config.theme_id();
        let colors = ThemeColors::for_theme(theme_id);
        crate::fonts::setup_fonts(&cc.egui_ctx);
        colors.apply(&cc.egui_ctx);
        layout::load();

        let view = View::Util(0);
        let last_zoom = crate::config::clamp_zoom(config.zoom);

        Self {
            config,
            colors,
            theme_id,
            view,
            scan_state: Arc::new(Mutex::new(ScanState::Idle)),
            scan_started: false,
            scan_phase: None,
            saved_results: Vec::new(),
            status_message: None,
            show_holy_confirm: false,
            show_clear_confirm: false,
            show_reset_confirm: false,
            reset_undo: None,
            reset_save_on_exit: false,
            component_state: Arc::new(Mutex::new(default_component_status())),
            download_busy: Arc::new(AtomicBool::new(false)),
            auto_download_started: false,
            inspect_id: String::new(),
            inspect_rows: Vec::new(),
            last_zoom,
            last_pixels_per_point: 0.0,
            zoom_apply_pending: true,
            zoom_apply_frames: 0,
            exit_saved: false,
        }
    }

    pub(super) fn expire_reset_undo(&mut self) {
        if let Some(undo) = &self.reset_undo {
            if Instant::now() >= undo.expires_at {
                self.reset_undo = None;
            }
        }
    }

    pub(super) fn confirm_reset_settings(&mut self, ctx: &egui::Context) {
        let snapshot = self.config.clone();
        self.reset_save_on_exit = snapshot.autosave == AutosaveMode::OnExit;
        self.reset_undo = Some(ResetUndo {
            snapshot,
            expires_at: Instant::now() + Duration::from_secs(10),
        });
        self.reset_all_settings(ctx);
    }

    pub(super) fn undo_reset_settings(&mut self, ctx: &egui::Context) {
        let Some(undo) = self.reset_undo.take() else {
            return;
        };
        self.reset_save_on_exit = false;
        self.config = undo.snapshot;
        self.theme_id = self.config.theme_id();
        self.colors = ThemeColors::for_theme(self.theme_id);
        ctx.set_visuals(self.colors.visuals());
        self.last_zoom = crate::config::clamp_zoom(self.config.zoom);
        apply_saved_zoom(ctx, self.last_zoom);
        self.zoom_apply_pending = true;
        self.zoom_apply_frames = 0;
        if self.config.autosave == AutosaveMode::OnChange {
            self.write_settings();
        }
    }

    pub(super) fn set_status(&mut self, msg: impl Into<String>, error: bool) {
        self.status_message = Some((msg.into(), error));
    }

    pub(super) fn glow_for(&self, area: GlowArea) -> Option<&GlowConfig> {
        self.config.glow.active_for(area).then_some(&self.config.glow)
    }

    pub(super) fn persist_after_change(&mut self) {
        if self.config.autosave == AutosaveMode::OnChange {
            self.write_settings();
        }
    }

    pub(super) fn persist_autosave_change(&mut self, prev: AutosaveMode) {
        if self.config.autosave == AutosaveMode::OnChange || prev == AutosaveMode::OnChange {
            self.write_settings();
        }
    }

    fn write_settings(&mut self) {
        self.config.sanitize();
        if let Err(e) = self.config.save() {
            self.set_status(e, true);
        }
    }

    fn save_on_exit(&mut self) {
        if self.exit_saved {
            return;
        }
        self.exit_saved = true;
        if self.config.autosave == AutosaveMode::OnExit {
            self.config.sanitize();
            let _ = self.config.save();
        }
    }

    pub(super) fn apply_theme(&mut self, ctx: &egui::Context, theme: ThemeId) {
        self.theme_id = theme;
        self.colors = ThemeColors::for_theme(theme);
        ctx.set_visuals(self.colors.visuals());
        self.config.set_theme(theme);
        self.config.zoom = crate::config::clamp_zoom(self.last_zoom);
        self.persist_after_change();
    }

    pub(super) fn reset_all_settings(&mut self, ctx: &egui::Context) {
        let was_off = self.config.autosave == AutosaveMode::Off;
        self.config = AppConfig::default();
        self.theme_id = self.config.theme_id();
        self.colors = ThemeColors::for_theme(self.theme_id);
        ctx.set_visuals(self.colors.visuals());
        self.last_zoom = crate::config::clamp_zoom(self.config.zoom);
        apply_saved_zoom(ctx, self.last_zoom);
        self.zoom_apply_pending = true;
        self.zoom_apply_frames = 0;
        if !was_off {
            self.write_settings();
        }
    }

    fn persist_zoom_if_changed(&mut self, ctx: &egui::Context) {
        if self.zoom_apply_pending {
            apply_saved_zoom(ctx, self.last_zoom);
            self.zoom_apply_frames = self.zoom_apply_frames.saturating_add(1);
            if (ctx.zoom_factor() - self.last_zoom).abs() < 0.001 || self.zoom_apply_frames >= 3 {
                self.last_pixels_per_point = ctx.pixels_per_point();
                self.zoom_apply_pending = false;
            }
            return;
        }

        let pixels_per_point = ctx.pixels_per_point();
        if (pixels_per_point - self.last_pixels_per_point).abs() > 0.001 {
            self.last_pixels_per_point = pixels_per_point;
            let zoom = crate::config::clamp_zoom(ctx.zoom_factor());
            if (zoom - ctx.zoom_factor()).abs() > 0.001 {
                apply_saved_zoom(ctx, zoom);
                self.last_pixels_per_point = ctx.pixels_per_point();
            }
            if (zoom - self.last_zoom).abs() > 0.001 {
                self.last_zoom = zoom;
                self.config.set_zoom(zoom);
                self.persist_after_change();
            }
        }
    }

    pub(super) fn start_auto_check(&mut self) {
        self.view = View::AutoCheck;
        self.scan_started = true;
        self.saved_results.clear();
        self.scan_phase = Some(ScanPhase::Processes);
        {
            let mut state = self.scan_state.lock().unwrap();
            *state = ScanState::Running(ScanPhase::Processes);
        }

        if let Err(e) = run_autocheck_search() {
            self.set_status(e, true);
        }

        let scan_state = Arc::clone(&self.scan_state);
        thread::spawn(move || {
            let results = perform_scan(Arc::clone(&scan_state));
            if let Ok(mut state) = scan_state.lock() {
                *state = ScanState::Done(results);
            }
        });
    }

    pub(super) fn poll_scan(&mut self) {
        let Ok(mut state) = self.scan_state.try_lock() else {
            return;
        };
        match &mut *state {
            ScanState::Running(phase) => {
                self.scan_phase = Some(*phase);
            }
            ScanState::Done(_) => {
                if let ScanState::Done(results) = std::mem::replace(&mut *state, ScanState::Idle) {
                    self.saved_results = results;
                    self.scan_phase = None;
                }
            }
            ScanState::Idle => {}
        }
    }

    pub(super) fn refresh_component_snapshot(&self) -> HashMap<String, ComponentStatus> {
        self.component_state
            .try_lock()
            .map(|map| map.clone())
            .unwrap_or_else(|_| default_component_status())
    }

    pub(super) fn pending_download_ids(
        snapshot: &HashMap<String, ComponentStatus>,
    ) -> Vec<String> {
        crate::tools::UTILS
            .iter()
            .filter_map(|util| {
                if crate::tools::paths::tool_installed(util.id) {
                    return None;
                }
                match snapshot.get(util.id) {
                    Some(ComponentStatus::Ready | ComponentStatus::Downloading { .. }) => None,
                    _ => Some(util.id.to_string()),
                }
            })
            .collect()
    }

    pub(super) fn is_download_busy(&self) -> bool {
        self.download_busy.load(Ordering::SeqCst)
    }

    pub(super) fn start_downloads(&mut self, ids: Vec<String>, force: bool) {
        if !download::downloads_enabled() {
            if crate::tools::paths::is_offline() {
                self.set_status(
                    "Офлайн-сборка: загрузка из сети отключена. Файлы должны лежать в assets/.",
                    true,
                );
            } else {
                self.set_status(
                    "Сторонние .exe скачиваются только в Windows. На Linux/macOS установите системные программы из подсказок.",
                    true,
                );
            }
            return;
        }
        let ids: Vec<String> = ids
            .into_iter()
            .filter(|id| force || !crate::tools::paths::tool_installed(id))
            .collect();
        if ids.is_empty() {
            self.set_status("Всё уже скачано", false);
            return;
        }
        if self
            .download_busy
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            self.set_status("Уже скачивается", false);
            return;
        }

        if let Ok(mut map) = self.component_state.lock() {
            for id in &ids {
                map.insert(
                    id.clone(),
                    ComponentStatus::Downloading {
                        received: 0,
                        total: None,
                    },
                );
            }
        }

        let states = Arc::clone(&self.component_state);
        let busy = Arc::clone(&self.download_busy);
        thread::spawn(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                run_download_batch(&states, &ids, force);
            }));
            if let Err(_) = result {
                for id in &ids {
                    set_status(
                        &states,
                        id,
                        ComponentStatus::Failed("Ошибка загрузки".into()),
                    );
                }
            }
            busy.store(false, Ordering::SeqCst);
        });
    }

    pub(super) fn maybe_autodownload(&mut self) {
        if self.auto_download_started || !matches!(self.view, View::Components) {
            return;
        }
        if !download::downloads_enabled() {
            self.auto_download_started = true;
            return;
        }
        self.auto_download_started = true;
        let missing = download::missing_tool_ids().unwrap_or_default();
        if !missing.is_empty() {
            self.start_downloads(missing, false);
        }
    }
}

fn run_download_batch(
    states: &Mutex<HashMap<String, ComponentStatus>>,
    ids: &[String],
    force: bool,
) {
    if let Err(e) = crate::backend::download_tools(ids, force, |id, event| {
        let status = match event {
            crate::backend::DownloadEvent::Progress(ToolProgress::Connecting) => {
                ComponentStatus::Downloading {
                    received: 0,
                    total: None,
                }
            }
            crate::backend::DownloadEvent::Progress(ToolProgress::Receiving { received, total }) => {
                ComponentStatus::Downloading { received, total }
            }
            crate::backend::DownloadEvent::Progress(ToolProgress::Verifying) => {
                ComponentStatus::Verifying
            }
            crate::backend::DownloadEvent::Progress(ToolProgress::Extracting) => {
                ComponentStatus::Extracting
            }
            crate::backend::DownloadEvent::Ready => ComponentStatus::Ready,
            crate::backend::DownloadEvent::Failed(err) => ComponentStatus::Failed(err),
        };
        set_status(states, id, status);
    }) {
        for id in ids {
            set_status(states, id, ComponentStatus::Failed(e.clone()));
        }
    }
}

fn set_status(map: &Mutex<HashMap<String, ComponentStatus>>, id: &str, status: ComponentStatus) {
    if let Ok(mut g) = map.lock() {
        g.insert(id.to_string(), status);
    }
}

pub(super) fn default_component_status() -> HashMap<String, ComponentStatus> {
    crate::tools::UTILS
        .iter()
        .map(|util| {
            let status = if crate::tools::paths::tool_installed(util.id) {
                ComponentStatus::Ready
            } else {
                ComponentStatus::Missing
            };
            (util.id.to_string(), status)
        })
        .collect()
}

fn apply_saved_zoom(ctx: &egui::Context, zoom: f32) {
    let zoom = crate::config::clamp_zoom(zoom);
    let native = ctx.native_pixels_per_point().unwrap_or(1.0);
    ctx.set_pixels_per_point(zoom * native);
}

impl eframe::App for CubeCheckApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.persist_zoom_if_changed(ctx);
        self.poll_scan();
        self.maybe_autodownload();
        self.expire_reset_undo();
        if self.reset_undo.is_some() {
            ctx.request_repaint_after(Duration::from_millis(200));
        }
        egui::SidePanel::left("sidebar")
            .resizable(false)
            .exact_width(SIDEBAR_WIDTH)
            .frame(self.sidebar_frame().stroke(egui::Stroke::NONE))
            .show_separator_line(false)
            .show(ctx, |ui| {
                sidebar::draw_sidebar(self, ui);
            });

        egui::TopBottomPanel::bottom("footer")
            .exact_height(FOOTER_HEIGHT)
            .frame(self.footer_frame())
            .show_separator_line(false)
            .show(ctx, |ui| {
                views::draw_footer(self, ui);
            });

        egui::CentralPanel::default()
            .frame(self.content_frame())
            .show(ctx, |ui| match self.view {
                View::Util(_) | View::Recycle => views::draw_utils_panel(self, ui),
                View::Components => views::draw_components(self, ui),
                View::AutoCheck => views::draw_auto_check(self, ui),
                View::About => views::draw_about(self, ui),
                View::System => views::draw_system(self, ui),
                View::Settings => views::draw_settings(self, ui),
            });

        dialogs::draw_status(self, ctx);
        dialogs::draw_dialogs(self, ctx);

        if self.scan_phase.is_some() || self.is_download_busy() {
            ctx.request_repaint_after(Duration::from_millis(150));
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if self.reset_save_on_exit && self.config.autosave != AutosaveMode::Off {
            self.config.sanitize();
            let _ = self.config.save();
            self.exit_saved = true;
            return;
        }
        self.save_on_exit();
    }
}
