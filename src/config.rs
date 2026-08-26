use serde::{Deserialize, Serialize};

use crate::theme::ThemeId;
use crate::tools::paths::{legacy_settings_paths, migrate_legacy_settings, settings_path};

pub const ZOOM_MIN: f32 = 0.5;
pub const ZOOM_MAX: f32 = 2.5;
pub const GLOW_RADIUS_MIN: f32 = 8.0;
pub const GLOW_RADIUS_MAX: f32 = 80.0;
pub const GLOW_INTENSITY_MIN: f32 = 0.2;
pub const GLOW_INTENSITY_MAX: f32 = 2.0;
pub const GLOW_SPEED_MIN: f32 = 0.1;
pub const GLOW_SPEED_MAX: f32 = 5.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutosaveMode {
    OnExit,
    OnChange,
    Off,
}

impl Default for AutosaveMode {
    fn default() -> Self {
        Self::OnChange
    }
}

impl AutosaveMode {
    pub const ALL: [AutosaveMode; 3] = [Self::OnExit, Self::OnChange, Self::Off];

    pub fn label(self) -> &'static str {
        match self {
            Self::OnExit => "при выключении программы",
            Self::OnChange => "при изменении настроек",
            Self::Off => "не сохранять",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlowArea {
    Sidebar,
    About,
    System,
    Footer,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct GlowAreas {
    pub sidebar: bool,
    pub about: bool,
    pub system: bool,
    pub footer: bool,
}

impl Default for GlowAreas {
    fn default() -> Self {
        Self {
            sidebar: true,
            about: true,
            system: true,
            footer: true,
        }
    }
}

impl GlowAreas {
    pub fn enabled(&self, area: GlowArea) -> bool {
        match area {
            GlowArea::Sidebar => self.sidebar,
            GlowArea::About => self.about,
            GlowArea::System => self.system,
            GlowArea::Footer => self.footer,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct GlowConfig {
    pub enabled: bool,
    pub color: [u8; 3],
    pub color2: [u8; 3],
    pub gradient: bool,
    pub gradient_speed: f32,
    pub radius: f32,
    pub intensity: f32,
    pub areas: GlowAreas,
}

impl Default for GlowConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            color: [212, 175, 55],
            color2: [255, 214, 90],
            gradient: false,
            gradient_speed: 1.0,
            radius: 34.0,
            intensity: 1.0,
            areas: GlowAreas::default(),
        }
    }
}

impl GlowConfig {
    pub fn sanitize(&mut self) {
        self.radius = clamp_finite(self.radius, 34.0, GLOW_RADIUS_MIN, GLOW_RADIUS_MAX);
        self.intensity = clamp_finite(self.intensity, 1.0, GLOW_INTENSITY_MIN, GLOW_INTENSITY_MAX);
        self.gradient_speed = clamp_finite(self.gradient_speed, 1.0, GLOW_SPEED_MIN, GLOW_SPEED_MAX);
    }

    pub fn active_for(&self, area: GlowArea) -> bool {
        self.enabled && self.areas.enabled(area)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    #[serde(default = "default_theme")]
    pub theme: String,
    /// egui zoom factor (`pixels_per_point = zoom * native_dpi`). Ctrl+/- changes this.
    #[serde(default = "default_zoom", alias = "pixels_per_point")]
    pub zoom: f32,
    #[serde(default)]
    pub glow: GlowConfig,
    #[serde(default)]
    pub autosave: AutosaveMode,
}

fn default_theme() -> String {
    "black".into()
}

fn default_zoom() -> f32 {
    1.0
}

pub fn clamp_zoom(zoom: f32) -> f32 {
    if !zoom.is_finite() {
        return default_zoom();
    }
    let clamped = zoom.clamp(ZOOM_MIN, ZOOM_MAX);
    (clamped * 100.0).round() / 100.0
}

fn clamp_finite(value: f32, fallback: f32, min: f32, max: f32) -> f32 {
    if !value.is_finite() {
        return fallback;
    }
    let clamped = value.clamp(min, max);
    (clamped * 100.0).round() / 100.0
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            theme: default_theme(),
            zoom: default_zoom(),
            glow: GlowConfig::default(),
            autosave: AutosaveMode::default(),
        }
    }
}

impl AppConfig {
    pub fn path() -> std::path::PathBuf {
        settings_path()
    }

    pub fn load() -> Self {
        let path = Self::path();
        if path.is_file() {
            if let Ok(text) = std::fs::read_to_string(&path) {
                return Self::from_json(&text);
            }
        }

        migrate_legacy_settings();
        if path.is_file() {
            if let Ok(text) = std::fs::read_to_string(&path) {
                return Self::from_json(&text);
            }
        }

        if let Some(mut legacy) = load_legacy() {
            legacy.sanitize();
            if legacy.autosave == AutosaveMode::OnChange {
                let _ = legacy.save();
            }
            return legacy;
        }
        Self::default()
    }

    fn from_json(text: &str) -> Self {
        let mut cfg = serde_json::from_str::<Self>(text).unwrap_or_default();
        cfg.sanitize();
        cfg
    }

    pub fn sanitize(&mut self) {
        self.zoom = clamp_zoom(self.zoom);
        self.glow.sanitize();
        if ThemeId::from_key(&self.theme).as_key() != self.theme {
            self.theme = ThemeId::from_key(&self.theme).as_key().to_string();
        }
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Не удалось сохранить настройки: {e}"))?;
        }
        let text = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Не удалось сохранить настройки: {e}"))?;
        std::fs::write(&path, text).map_err(|e| format!("Не удалось сохранить настройки: {e}"))
    }

    pub fn theme_id(&self) -> ThemeId {
        ThemeId::from_key(&self.theme)
    }

    pub fn set_theme(&mut self, id: ThemeId) {
        self.theme = id.as_key().to_string();
        self.sanitize();
    }

    pub fn set_zoom(&mut self, zoom: f32) {
        self.zoom = clamp_zoom(zoom);
    }
}

fn load_legacy() -> Option<AppConfig> {
    for path in legacy_settings_paths() {
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        if let Ok(cfg) = serde_json::from_str::<AppConfig>(&text) {
            return Some(cfg);
        }
    }
    None
}
