use egui::{Color32, Rounding, Stroke, Visuals};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ThemeId {
    Black,
    White,
    Purple,
    Blue,
    Lime,
}

impl ThemeId {
    pub const ALL: [ThemeId; 5] = [
        ThemeId::Black,
        ThemeId::White,
        ThemeId::Purple,
        ThemeId::Blue,
        ThemeId::Lime,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ThemeId::Black => "Чёрная",
            ThemeId::White => "Белая",
            ThemeId::Purple => "Фиолетовая",
            ThemeId::Blue => "Синяя",
            ThemeId::Lime => "Лаймовая",
        }
    }

    pub fn as_key(self) -> &'static str {
        match self {
            ThemeId::Black => "black",
            ThemeId::White => "white",
            ThemeId::Purple => "purple",
            ThemeId::Blue => "blue",
            ThemeId::Lime => "lime",
        }
    }

    pub fn from_key(key: &str) -> Self {
        match key.to_ascii_lowercase().as_str() {
            "white" => ThemeId::White,
            "purple" => ThemeId::Purple,
            "blue" => ThemeId::Blue,
            "lime" => ThemeId::Lime,
            _ => ThemeId::Black,
        }
    }
}

#[derive(Clone, Copy)]
pub struct ThemeColors {
    pub bg: Color32,
    pub fg: Color32,
    pub card: Color32,
    pub hover: Color32,
    pub select: Color32,
    pub accent: Color32,
    pub text_dim: Color32,
    pub border: Color32,
    pub button_bg: Color32,
    pub section: Color32,
    pub studio: Color32,
    pub link: Color32,
    pub footer: Color32,
    pub track: Color32,
    pub input_bg: Color32,
    pub widget_outline: Color32,
    pub handle: Color32,
    pub light: bool,
}

impl ThemeColors {
    pub fn for_theme(id: ThemeId) -> Self {
        match id {
            ThemeId::Black => Self {
                bg: hex("#0a0a0f"),
                fg: hex("#e0e0f0"),
                card: hex("#15151f"),
                hover: hex("#20202e"),
                select: hex("#252540"),
                accent: hex("#8a7ad8"),
                text_dim: hex("#a0aabf"),
                border: hex("#3a3a55"),
                button_bg: hex("#1c1c2e"),
                section: hex("#6a6a8a"),
                studio: hex("#4a4a7a"),
                link: hex("#6a8aaf"),
                footer: hex("#4a4a6a"),
                track: hex("#3a3a50"),
                input_bg: hex("#0c0c14"),
                widget_outline: hex("#6a6a88"),
                handle: hex("#d0d0e8"),
                light: false,
            },
            ThemeId::White => Self {
                bg: hex("#e6e6e6"),
                fg: hex("#1a1a1a"),
                card: hex("#ffffff"),
                hover: hex("#d4d4d4"),
                select: hex("#c5d0e0"),
                accent: hex("#3d5a85"),
                text_dim: hex("#3f3f3f"),
                border: hex("#7a7a7a"),
                button_bg: hex("#d0d0d0"),
                section: hex("#4a4a4a"),
                studio: hex("#4a4a4a"),
                link: hex("#3d5a85"),
                footer: hex("#3f3f3f"),
                track: hex("#6a6a6a"),
                input_bg: hex("#e8e8e8"),
                widget_outline: hex("#4a4a4a"),
                handle: hex("#2a2a2a"),
                light: true,
            },
            ThemeId::Purple => Self {
                bg: hex("#0d0a1a"),
                fg: hex("#d4c8f0"),
                card: hex("#1a122a"),
                hover: hex("#2a1a3a"),
                select: hex("#32204e"),
                accent: hex("#a882d8"),
                text_dim: hex("#b0a0c4"),
                border: hex("#4a3470"),
                button_bg: hex("#251838"),
                section: hex("#8a70a0"),
                studio: hex("#6a5080"),
                link: hex("#a882d8"),
                footer: hex("#8a70a8"),
                track: hex("#3a2858"),
                input_bg: hex("#100a18"),
                widget_outline: hex("#7a58a8"),
                handle: hex("#e0d0f8"),
                light: false,
            },
            ThemeId::Blue => Self {
                bg: hex("#0a0f1a"),
                fg: hex("#c8d8f0"),
                card: hex("#121a2a"),
                hover: hex("#1a2a3a"),
                select: hex("#1e3250"),
                accent: hex("#5880c8"),
                text_dim: hex("#9aa8c0"),
                border: hex("#3a5070"),
                button_bg: hex("#1a2438"),
                section: hex("#607090"),
                studio: hex("#506080"),
                link: hex("#5880c8"),
                footer: hex("#7088a8"),
                track: hex("#2a3c58"),
                input_bg: hex("#0a1018"),
                widget_outline: hex("#6080a8"),
                handle: hex("#d0dcec"),
                light: false,
            },
            ThemeId::Lime => Self {
                bg: hex("#0a0f0a"),
                fg: hex("#d0e8c0"),
                card: hex("#121f12"),
                hover: hex("#1a2f1a"),
                select: hex("#1e3a1e"),
                accent: hex("#80c850"),
                text_dim: hex("#90b090"),
                border: hex("#3a5a3a"),
                button_bg: hex("#1a2c1a"),
                section: hex("#608060"),
                studio: hex("#507050"),
                link: hex("#80c850"),
                footer: hex("#80a070"),
                track: hex("#2a4a2a"),
                input_bg: hex("#081008"),
                widget_outline: hex("#5a8a50"),
                handle: hex("#d8f0c8"),
                light: false,
            },
        }
    }

    pub fn frame_stroke(&self) -> Stroke {
        Stroke::new(if self.light { 1.5 } else { 1.0 }, self.border)
    }

    pub fn selected_stroke(&self) -> Stroke {
        Stroke::new(if self.light { 2.0 } else { 1.5 }, self.accent)
    }

    pub fn chip_stroke(&self, selected: bool) -> Stroke {
        if selected {
            self.selected_stroke()
        } else {
            self.frame_stroke()
        }
    }

    pub fn button_stroke(&self) -> Stroke {
        Stroke::new(if self.light { 1.5 } else { 1.0 }, self.widget_outline)
    }

    pub fn apply(&self, ctx: &egui::Context) {
        ctx.set_visuals(self.visuals());
        ctx.style_mut(|style| {
            style.spacing.slider_rail_height = if self.light { 10.0 } else { 8.0 };
            style.visuals = self.visuals();
        });
    }

    pub fn with_slider_visuals(self, ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui)) {
        ui.scope(|ui| {
            let stroke_on_track = contrast_on(self.track);
            let stroke_on_handle = contrast_on(self.handle);
            let v = ui.visuals_mut();
            v.widgets.inactive.bg_fill = self.track;
            v.widgets.hovered.bg_fill = self.handle;
            v.widgets.active.bg_fill = self.handle;
            v.widgets.inactive.fg_stroke = Stroke::new(2.0, stroke_on_track);
            v.widgets.hovered.fg_stroke = Stroke::new(2.0, stroke_on_handle);
            v.widgets.active.fg_stroke = Stroke::new(2.0, self.accent);
            add_contents(ui);
        });
    }

    pub fn visuals(&self) -> Visuals {
        let mut v = if self.light {
            Visuals::light()
        } else {
            Visuals::dark()
        };
        v.dark_mode = !self.light;
        v.override_text_color = Some(self.fg);
        v.window_fill = self.bg;
        v.panel_fill = self.bg;
        v.window_stroke = self.frame_stroke();
        v.extreme_bg_color = self.input_bg;
        v.faint_bg_color = self.hover;
        v.code_bg_color = self.button_bg;
        v.hyperlink_color = self.link;
        v.warn_fg_color = Color32::from_rgb(255, 180, 80);
        v.error_fg_color = Color32::from_rgb(220, 70, 70);
        v.window_rounding = Rounding::same(6.0);
        v.slider_trailing_fill = true;
        v.selection.bg_fill = self.select;
        v.selection.stroke = self.selected_stroke();

        let rounding = Rounding::same(3.0);
        let idle_outline = Stroke::new(1.5, self.widget_outline);
        let hover_outline = Stroke::new(1.5, self.accent);
        let active_outline = Stroke::new(2.0, self.accent);

        v.widgets.noninteractive.bg_fill = self.bg;
        v.widgets.noninteractive.weak_bg_fill = self.bg;
        v.widgets.noninteractive.bg_stroke = self.frame_stroke();
        v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, self.text_dim);
        v.widgets.noninteractive.rounding = rounding;
        v.widgets.noninteractive.expansion = 0.0;

        v.widgets.inactive.bg_fill = self.button_bg;
        v.widgets.inactive.weak_bg_fill = self.button_bg;
        v.widgets.inactive.bg_stroke = idle_outline;
        v.widgets.inactive.fg_stroke = Stroke::new(1.5, self.fg);
        v.widgets.inactive.rounding = rounding;
        v.widgets.inactive.expansion = 0.0;

        v.widgets.hovered.bg_fill = self.hover;
        v.widgets.hovered.weak_bg_fill = self.hover;
        v.widgets.hovered.bg_stroke = hover_outline;
        v.widgets.hovered.fg_stroke = Stroke::new(1.5, self.fg);
        v.widgets.hovered.rounding = rounding;
        v.widgets.hovered.expansion = 0.0;

        v.widgets.active.bg_fill = self.select;
        v.widgets.active.weak_bg_fill = self.select;
        v.widgets.active.bg_stroke = active_outline;
        v.widgets.active.fg_stroke = Stroke::new(2.0, self.accent);
        v.widgets.active.rounding = rounding;
        v.widgets.active.expansion = 0.0;

        v.widgets.open.bg_fill = self.select;
        v.widgets.open.weak_bg_fill = self.select;
        v.widgets.open.bg_stroke = idle_outline;
        v.widgets.open.fg_stroke = Stroke::new(1.5, self.fg);
        v.widgets.open.rounding = rounding;
        v.widgets.open.expansion = 0.0;

        v
    }
}

fn contrast_on(bg: Color32) -> Color32 {
    if luminance(bg) > 140.0 {
        Color32::from_rgb(20, 20, 20)
    } else {
        Color32::from_rgb(245, 245, 245)
    }
}

fn luminance(c: Color32) -> f32 {
    0.2126 * f32::from(c.r()) + 0.7152 * f32::from(c.g()) + 0.0722 * f32::from(c.b())
}

fn hex(s: &str) -> Color32 {
    let s = s.trim_start_matches('#');
    let r = u8::from_str_radix(&s[0..2], 16).unwrap_or(0);
    let g = u8::from_str_radix(&s[2..4], 16).unwrap_or(0);
    let b = u8::from_str_radix(&s[4..6], 16).unwrap_or(0);
    Color32::from_rgb(r, g, b)
}
