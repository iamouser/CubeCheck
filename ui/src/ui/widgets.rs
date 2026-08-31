use std::ops::RangeInclusive;

use eframe::egui;

use crate::config::{GlowArea, GlowConfig};
use crate::theme::ThemeColors;

use super::app::CubeCheckApp;
use super::glow;

pub(super) const SETTINGS_FORM_WIDTH: f32 = 548.0;
pub(super) const SETTINGS_LABEL_WIDTH: f32 = 152.0;
pub(super) const SETTINGS_ROW_HEIGHT: f32 = 30.0;
pub(super) const SETTINGS_VALUE_WIDTH: f32 = 58.0;
pub(super) const SETTINGS_COL_GAP: f32 = 12.0;
pub(super) const SETTINGS_ROW_GAP: f32 = 6.0;
pub(super) const SETTINGS_SECTION_GAP: f32 = 22.0;
pub(super) const SETTINGS_BODY_SIZE: f32 = 13.5;
pub(super) const SETTINGS_TITLE_SIZE: f32 = 16.0;

pub(super) fn settings_section_title(ui: &mut egui::Ui, colors: &ThemeColors, title: &str) {
    ui.label(
        egui::RichText::new(title)
            .size(SETTINGS_TITLE_SIZE)
            .strong()
            .color(colors.fg),
    );
    ui.add_space(10.0);
}

pub(super) fn settings_row(
    ui: &mut egui::Ui,
    colors: &ThemeColors,
    label: &str,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    let width = ui.available_width();
    let control_w = (width - SETTINGS_LABEL_WIDTH - SETTINGS_COL_GAP).max(48.0);
    ui.allocate_ui_with_layout(
        egui::vec2(width, SETTINGS_ROW_HEIGHT),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.spacing_mut().item_spacing.x = SETTINGS_COL_GAP;
            ui.allocate_ui_with_layout(
                egui::vec2(SETTINGS_LABEL_WIDTH, SETTINGS_ROW_HEIGHT),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(label)
                                .size(SETTINGS_BODY_SIZE)
                                .color(colors.fg),
                        )
                        .extend(),
                    );
                },
            );
            ui.allocate_ui_with_layout(
                egui::vec2(control_w, SETTINGS_ROW_HEIGHT),
                egui::Layout::left_to_right(egui::Align::Center),
                add_contents,
            );
        },
    );
    ui.add_space(SETTINGS_ROW_GAP);
}

pub(super) fn settings_checkbox_row(
    ui: &mut egui::Ui,
    colors: &ThemeColors,
    label: &str,
    checked: &mut bool,
) -> bool {
    let mut changed = false;
    settings_row(ui, colors, label, |ui| {
        changed = ui.add(egui::Checkbox::without_text(checked)).changed();
    });
    changed
}

pub(super) fn settings_choice_row(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui)) {
    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), SETTINGS_ROW_HEIGHT),
        egui::Layout::left_to_right(egui::Align::Center),
        add_contents,
    );
    ui.add_space(SETTINGS_ROW_GAP);
}

pub(super) fn settings_slider_row(
    ui: &mut egui::Ui,
    colors: ThemeColors,
    label: &str,
    value: &mut f32,
    range: RangeInclusive<f32>,
    decimals: usize,
) -> bool {
    let mut changed = false;
    settings_row(ui, &colors, label, |ui| {
        let gap = 10.0;
        let slider_w = (ui.available_width() - SETTINGS_VALUE_WIDTH - gap).max(80.0);
        ui.spacing_mut().item_spacing.x = gap;
        colors.with_slider_visuals(ui, |ui| {
            ui.spacing_mut().slider_width = slider_w;
            changed |= ui
                .add(
                    egui::Slider::new(value, range.clone())
                        .show_value(false)
                        .min_decimals(decimals)
                        .max_decimals(decimals),
                )
                .changed();
        });
        let speed = if decimals == 0 { 1.0 } else { 0.01 };
        changed |= ui
            .add_sized(
                [SETTINGS_VALUE_WIDTH, 22.0],
                egui::DragValue::new(value)
                    .range(range)
                    .speed(speed)
                    .min_decimals(decimals)
                    .max_decimals(decimals),
            )
            .changed();
    });
    changed
}

pub(super) fn settings_wide_button(
    ui: &mut egui::Ui,
    colors: &ThemeColors,
    label: &str,
) -> egui::Response {
    ui.add_sized(
        [ui.available_width(), 32.0],
        egui::Button::new(
            egui::RichText::new(label)
                .size(14.0)
                .strong()
                .color(colors.fg),
        )
        .fill(colors.button_bg)
        .stroke(colors.button_stroke())
        .rounding(egui::Rounding::same(6.0)),
    )
}

fn sidebar_selected_fill(colors: ThemeColors) -> egui::Color32 {
    let accent = colors.accent;
    let select = colors.select;
    egui::Color32::from_rgb(
        ((u16::from(select.r()) * 2 + u16::from(accent.r())) / 3) as u8,
        ((u16::from(select.g()) * 2 + u16::from(accent.g())) / 3) as u8,
        ((u16::from(select.b()) * 2 + u16::from(accent.b())) / 3) as u8,
    )
}

impl CubeCheckApp {
    pub(super) const ACTION_BUTTON_SIZE: egui::Vec2 = egui::vec2(110.0, 32.0);

    pub(super) fn sidebar_frame(&self) -> egui::Frame {
        egui::Frame::none()
            .fill(self.colors.bg)
            .inner_margin(egui::Margin {
                left: 12.0,
                right: 16.0,
                top: 10.0,
                bottom: 10.0,
            })
    }

    pub(super) fn content_frame(&self) -> egui::Frame {
        egui::Frame::none()
            .fill(self.colors.card)
            .inner_margin(egui::Margin::same(20.0))
            .rounding(egui::Rounding::ZERO)
            .stroke(self.colors.frame_stroke())
    }

    pub(super) fn footer_frame(&self) -> egui::Frame {
        egui::Frame::none()
            .fill(self.colors.bg)
            .inner_margin(egui::Margin::symmetric(12.0, 6.0))
    }

    pub(super) fn dialog_frame(&self) -> egui::Frame {
        egui::Frame::none()
            .fill(self.colors.card)
            .stroke(self.colors.frame_stroke())
            .inner_margin(egui::Margin::symmetric(18.0, 16.0))
            .rounding(egui::Rounding::same(8.0))
    }

    pub(super) fn util_card_frame(&self, selected: bool) -> egui::Frame {
        let stroke = if selected {
            egui::Stroke::new(2.0, self.colors.accent)
        } else {
            self.colors.frame_stroke()
        };
        egui::Frame::none()
            .fill(self.colors.button_bg)
            .stroke(stroke)
            .inner_margin(egui::Margin::same(14.0))
            .rounding(egui::Rounding::same(6.0))
    }

    pub(super) fn sidebar_button(
        &self,
        ui: &mut egui::Ui,
        label: &str,
        selected: bool,
        width: f32,
    ) -> egui::Response {
        let text_color = if selected {
            self.colors.fg
        } else {
            self.colors.text_dim
        };

        let (rect, response) = ui.allocate_exact_size(egui::vec2(width, 28.0), egui::Sense::click());
        let fill = if selected {
            sidebar_selected_fill(self.colors)
        } else if response.hovered() {
            self.colors.hover
        } else {
            egui::Color32::TRANSPARENT
        };
        ui.painter()
            .rect_filled(rect, egui::Rounding::same(6.0), fill);
        let font_id = egui::FontId::proportional(13.0);
        let galley = ui.fonts(|f| f.layout_no_wrap(label.to_owned(), font_id, text_color));
        let galley_pos = egui::pos2(rect.left() + 8.0, rect.center().y - galley.size().y * 0.5);
        let glow = self.glow_for(GlowArea::Sidebar);
        glow::paint_at(ui, galley_pos, galley.as_ref(), text_color, glow);
        glow::paint_frame_glow(ui, rect, 6.0, glow);
        response.on_hover_cursor(egui::CursorIcon::PointingHand)
    }

    pub(super) fn progress_track(&self, ui: &mut egui::Ui, frac: f32, text: &str) {
        let width = ui.available_width().max(1.0);
        let height = 18.0;
        let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
        ui.painter()
            .rect_filled(rect, egui::Rounding::same(4.0), self.colors.bg);
        ui.painter().rect_stroke(
            rect,
            egui::Rounding::same(4.0),
            self.colors.frame_stroke(),
        );
        let fill_w = (rect.width() * frac.clamp(0.0, 1.0)).max(if frac > 0.0 { 6.0 } else { 0.0 });
        if fill_w > 0.0 {
            let mut fill = rect.shrink(1.0);
            fill.max.x = (fill.min.x + fill_w - 2.0).min(rect.right() - 1.0);
            if fill.width() > 0.0 {
                ui.painter()
                    .rect_filled(fill, egui::Rounding::same(3.0), self.colors.accent);
            }
        }
        if !text.is_empty() {
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                text,
                egui::FontId::proportional(11.0),
                self.colors.fg,
            );
        }
    }

    pub(super) fn accent_button(&self, ui: &mut egui::Ui, label: &str) -> egui::Response {
        ui.add(
            egui::Button::new(
                egui::RichText::new(label)
                    .size(13.0)
                    .strong()
                    .color(self.colors.fg),
            )
            .fill(self.colors.button_bg)
            .stroke(egui::Stroke::new(1.0_f32, self.colors.border))
            .rounding(egui::Rounding::same(6.0))
            .min_size(egui::vec2(88.0, 28.0)),
        )
    }

    pub(super) fn accent_button_glow(
        &self,
        ui: &mut egui::Ui,
        label: &str,
        _glow: Option<&GlowConfig>,
    ) -> egui::Response {
        self.accent_button(ui, label)
    }

    pub(super) fn reserve_action_button(&self, ui: &mut egui::Ui) {
        ui.allocate_exact_size(Self::ACTION_BUTTON_SIZE, egui::Sense::hover());
    }

    pub(super) fn section_label(&self, ui: &mut egui::Ui, text: &str) {
        ui.add_space(4.0);
        glow::label(
            ui,
            egui::RichText::new(text)
                .size(11.0)
                .strong()
                .color(self.colors.section),
            self.glow_for(GlowArea::Sidebar),
        );
        ui.add_space(2.0);
    }

    pub(super) fn separator(&self, ui: &mut egui::Ui) {
        ui.add_space(6.0);
        let width = ui.available_width();
        let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 1.0), egui::Sense::hover());
        ui.painter().hline(
            rect.left()..=rect.right(),
            rect.center().y,
            self.colors.frame_stroke(),
        );
        ui.add_space(6.0);
    }

    pub(super) fn sidebar_item_width(&self, ui: &egui::Ui) -> f32 {
        ui.available_width().max(0.0)
    }
}
