use std::time::Instant;

use eframe::egui;

use crate::tools::{clear_minecraft_logs, open_holycheck};

use super::app::{CubeCheckApp, FOOTER_HEIGHT, SIDEBAR_WIDTH};
use super::layout;

const RESET_OVERLAY: egui::Color32 = egui::Color32::from_black_alpha(150);
const DIALOG_WIDTH: f32 = 320.0;
const DIALOG_WIDTH_WIDE: f32 = 460.0;
const DIALOG_ACTION_GAP: f32 = 10.0;
const DIALOG_BODY_GAP: f32 = 14.0;
const UNDO_TOAST_GAP: f32 = 10.0;

pub(super) fn draw_dialogs(app: &mut CubeCheckApp, ctx: &egui::Context) {
    draw_reset_confirm(app, ctx);
    draw_reset_undo(app, ctx);

    if app.show_holy_confirm {
        let mut open = true;
        egui::Window::new("HolyCheck")
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .default_width(DIALOG_WIDTH)
            .frame(app.dialog_frame())
            .show(ctx, |ui| {
                begin_dialog_body(ui, DIALOG_WIDTH);
                dialog_body_label(ui, app, "Открыть сайт HolyWorld?");
                ui.add_space(6.0);
                ui.hyperlink("https://mods.holyworld.me/");
                ui.add_space(DIALOG_BODY_GAP);
                match dialog_action_row(
                    ui,
                    app,
                    DIALOG_WIDTH,
                    &[("dialog.holy.yes", "Да"), ("dialog.holy.cancel", "Отмена")],
                ) {
                    Some(0) => {
                        open_holycheck();
                        app.show_holy_confirm = false;
                    }
                    Some(1) => app.show_holy_confirm = false,
                    _ => {}
                }
            });
        if !open {
            app.show_holy_confirm = false;
        }
    }

    if app.show_clear_confirm {
        let mut open = true;
        egui::Window::new("Логи")
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .default_width(DIALOG_WIDTH)
            .frame(app.dialog_frame())
            .show(ctx, |ui| {
                begin_dialog_body(ui, DIALOG_WIDTH);
                dialog_body_label(ui, app, "Удалить логи Minecraft?");
                ui.add_space(DIALOG_BODY_GAP);
                match dialog_action_row(
                    ui,
                    app,
                    DIALOG_WIDTH,
                    &[
                        ("dialog.clear.confirm", "Очистить"),
                        ("dialog.clear.cancel", "Отмена"),
                    ],
                ) {
                    Some(0) => {
                        match clear_minecraft_logs() {
                            Ok(_) => app.set_status("Логи удалены", false),
                            Err(e) => app.set_status(e, true),
                        }
                        app.show_clear_confirm = false;
                    }
                    Some(1) => app.show_clear_confirm = false,
                    _ => {}
                }
            });
        if !open {
            app.show_clear_confirm = false;
        }
    }
}

fn draw_reset_confirm(app: &mut CubeCheckApp, ctx: &egui::Context) {
    if !app.show_reset_confirm {
        return;
    }

    let mut confirmed = false;
    let mut cancelled = false;
    let modal_id = egui::Id::new("reset_confirm_modal");
    let modal = egui::Modal::new(modal_id)
        .area(egui::Modal::default_area(modal_id).movable(false))
        .backdrop_color(RESET_OVERLAY)
        .frame(app.dialog_frame())
        .show(ctx, |ui| {
            begin_dialog_body(ui, DIALOG_WIDTH);
            ui.label(
                egui::RichText::new("Сброс настроек")
                    .size(16.0)
                    .strong()
                    .color(app.colors.fg),
            );
            ui.add_space(8.0);
            ui.label(
                egui::RichText::new("Вернуть настройки по умолчанию?")
                    .size(13.0)
                    .color(app.colors.text_dim),
            );
            ui.add_space(DIALOG_BODY_GAP);
            match dialog_action_row(
                ui,
                app,
                DIALOG_WIDTH,
                &[
                    ("dialog.reset.confirm", "Сбросить"),
                    ("dialog.reset.cancel", "Отмена"),
                ],
            ) {
                Some(0) => confirmed = true,
                Some(1) => cancelled = true,
                _ => {}
            }
        });

    if cancelled || modal.should_close() {
        app.show_reset_confirm = false;
    }
    if confirmed {
        app.show_reset_confirm = false;
        app.confirm_reset_settings(ctx);
    }
}

fn undo_toast_frame(app: &CubeCheckApp) -> egui::Frame {
    egui::Frame::none()
        .fill(app.colors.button_bg)
        .stroke(app.colors.chip_stroke(false))
        .inner_margin(egui::Margin::symmetric(16.0, 10.0))
        .rounding(egui::Rounding::same(6.0))
}

fn draw_reset_undo(app: &mut CubeCheckApp, ctx: &egui::Context) {
    if app.show_reset_confirm {
        return;
    }

    let Some(expires_at) = app.reset_undo.as_ref().map(|undo| undo.expires_at) else {
        return;
    };

    let remaining = expires_at.saturating_duration_since(Instant::now());
    if remaining.is_zero() {
        app.reset_undo = None;
        return;
    }

    let secs = remaining.as_secs_f32().ceil() as u64;
    let mut restore = false;
    let x_shift = SIDEBAR_WIDTH * 0.5;
    let y_shift = -(FOOTER_HEIGHT + UNDO_TOAST_GAP);

    egui::Area::new(egui::Id::new("reset_undo_bar"))
        .order(egui::Order::Foreground)
        .anchor(egui::Align2::CENTER_BOTTOM, [x_shift, y_shift])
        .movable(false)
        .interactable(true)
        .sense(egui::Sense::click())
        .show(ctx, |ui| {
            layout::layout_move(ui, "reset.undo", |ui| {
                undo_toast_frame(app).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 12.0;
                        ui.label(
                            egui::RichText::new("Настройки сброшены")
                                .size(13.0)
                                .color(app.colors.fg),
                        );
                        ui.label(
                            egui::RichText::new(format!("{secs} с"))
                                .size(13.0)
                                .color(app.colors.text_dim),
                        );
                        ui.allocate_ui(CubeCheckApp::ACTION_BUTTON_SIZE, |ui| {
                            if app.accent_button(ui, "Вернуть").clicked() {
                                restore = true;
                            }
                        });
                    });
                });
            });
        });

    if restore {
        app.undo_reset_settings(ctx);
    }
}

pub(super) fn draw_status(app: &mut CubeCheckApp, ctx: &egui::Context) {
    let Some((msg, is_error)) = &app.status_message else {
        return;
    };
    let title = if *is_error { "Ошибка" } else { "Успешно" }.to_string();
    let msg_text = msg.clone();
    let width = dialog_width_for_message(&msg_text);
    let display = soft_wrap_message(&msg_text);
    let mut close_window = false;

    egui::Window::new(title)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .default_width(width)
        .frame(app.dialog_frame())
        .show(ctx, |ui| {
            layout::layout_move(ui, "dialog.status", |ui| {
                begin_dialog_body(ui, width);
                dialog_body_label(ui, app, &display);
                ui.add_space(DIALOG_BODY_GAP);
                if dialog_action_row(ui, app, width, &[("dialog.status.ok", "OK")]) == Some(0) {
                    close_window = true;
                }
            });
        });

    if close_window {
        app.status_message = None;
    }
}

fn begin_dialog_body(ui: &mut egui::Ui, width: f32) {
    ui.set_min_width(width);
    ui.set_max_width(width);
    ui.spacing_mut().item_spacing.y = 0.0;
}

fn dialog_body_label(ui: &mut egui::Ui, app: &CubeCheckApp, text: &str) {
    ui.add(
        egui::Label::new(
            egui::RichText::new(text)
                .size(13.0)
                .color(app.colors.fg),
        )
        .wrap(),
    );
}

/// Centered row of equal-size action buttons. Each slot is a fixed rect so
/// `layout_move` cannot stretch the first button across the dialog.
fn dialog_action_row(
    ui: &mut egui::Ui,
    app: &CubeCheckApp,
    width: f32,
    buttons: &[(&'static str, &str)],
) -> Option<usize> {
    let n = buttons.len() as f32;
    if n <= 0.0 {
        return None;
    }
    let btn = CubeCheckApp::ACTION_BUTTON_SIZE;
    let row_w = btn.x * n + DIALOG_ACTION_GAP * (n - 1.0);
    let mut clicked = None;

    ui.allocate_ui_with_layout(
        egui::vec2(width, btn.y),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.spacing_mut().item_spacing.x = DIALOG_ACTION_GAP;
            ui.add_space(((width - row_w) * 0.5).max(0.0));
            for (i, (id, label)) in buttons.iter().enumerate() {
                let pressed = ui
                    .allocate_ui(btn, |ui| {
                        layout::layout_move(ui, id, |ui| app.accent_button(ui, label).clicked())
                    })
                    .inner;
                if pressed {
                    clicked = Some(i);
                }
            }
        },
    );
    clicked
}

fn dialog_width_for_message(msg: &str) -> f32 {
    if msg.contains('\\') || msg.contains('/') || msg.chars().count() > 42 {
        DIALOG_WIDTH_WIDE
    } else {
        DIALOG_WIDTH
    }
}

/// Wrap long paths at `\` / `/` instead of at spaces inside folder names.
fn soft_wrap_message(msg: &str) -> String {
    let mut out = String::with_capacity(msg.len() + 8);
    let mut pathish = false;
    for ch in msg.chars() {
        match ch {
            '\\' | '/' => {
                pathish = true;
                out.push(ch);
                out.push('\u{200B}');
            }
            ' ' if pathish => out.push('\u{00A0}'),
            c => {
                if c.is_whitespace() {
                    pathish = false;
                }
                out.push(c);
            }
        }
    }
    out
}
