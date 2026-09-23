use serde::de::Deserializer;
use serde::{Deserialize, Serialize};

use crate::theme::ThemeId;

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
    #[serde(
        default = "default_color",
        deserialize_with = "de_color",
        serialize_with = "ser_color"
    )]
    pub color: [u8; 3],
    #[serde(
        default = "default_color2",
        deserialize_with = "de_color2",
        serialize_with = "ser_color"
    )]
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
    #[serde(default = "default_check_updates")]
    pub check_updates: bool,
}

fn default_theme() -> String {
    "black".into()
}

fn default_zoom() -> f32 {
    1.0
}

fn default_check_updates() -> bool {
    true
}

fn default_color() -> [u8; 3] {
    [212, 175, 55]
}

fn default_color2() -> [u8; 3] {
    [255, 214, 90]
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
            check_updates: true,
        }
    }
}

impl AppConfig {
    pub fn load() -> Self {
        match crate::backend::load_settings() {
            Ok(text) => Self::from_json(&text),
            Err(_) => Self::default(),
        }
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
        let mut cfg = self.clone();
        cfg.sanitize();
        let text = serde_json::to_string_pretty(&cfg)
            .map_err(|e| format!("Не удалось сохранить настройки: {e}"))?;
        crate::backend::save_settings(&text)
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

fn ser_color<S>(rgb: &[u8; 3], serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&format_hex(*rgb))
}

fn de_color<'de, D>(deserializer: D) -> Result<[u8; 3], D::Error>
where
    D: Deserializer<'de>,
{
    parse_color_value(deserializer, default_color())
}

fn de_color2<'de, D>(deserializer: D) -> Result<[u8; 3], D::Error>
where
    D: Deserializer<'de>,
{
    parse_color_value(deserializer, default_color2())
}

fn parse_color_value<'de, D>(deserializer: D, fallback: [u8; 3]) -> Result<[u8; 3], D::Error>
where
    D: Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    Ok(parse_rgb_value(&value).unwrap_or(fallback))
}

fn parse_rgb_value(value: &serde_json::Value) -> Option<[u8; 3]> {
    match value {
        serde_json::Value::String(text) => parse_hex(text),
        serde_json::Value::Array(items) if items.len() >= 3 => {
            let r = items[0].as_u64()?;
            let g = items[1].as_u64()?;
            let b = items[2].as_u64()?;
            if r > 255 || g > 255 || b > 255 {
                return None;
            }
            Some([r as u8, g as u8, b as u8])
        }
        _ => None,
    }
}

pub(crate) fn format_hex(rgb: [u8; 3]) -> String {
    format!("#{:02X}{:02X}{:02X}", rgb[0], rgb[1], rgb[2])
}

pub(crate) fn parse_hex(text: &str) -> Option<[u8; 3]> {
    let s = text.trim();
    let s = s.strip_prefix('#').unwrap_or(s);
    let s = if s.len() == 3 || s.len() == 4 {
        let mut expanded = String::new();
        for c in s.chars() {
            expanded.push(c);
            expanded.push(c);
        }
        expanded
    } else {
        s.to_string()
    };
    let s = if s.len() == 8 { s[2..].to_string() } else { s };
    if s.len() != 6 || !s.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let r = u8::from_str_radix(&s[0..2], 16).ok()?;
    let g = u8::from_str_radix(&s[2..4], 16).ok()?;
    let b = u8::from_str_radix(&s[4..6], 16).ok()?;
    Some([r, g, b])
}

