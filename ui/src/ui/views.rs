use eframe::egui;

use crate::config::{
    AutosaveMode, GlowArea, GLOW_INTENSITY_MAX, GLOW_INTENSITY_MIN, GLOW_RADIUS_MAX,
    GLOW_RADIUS_MIN, GLOW_SPEED_MAX, GLOW_SPEED_MIN,
};
use crate::content::{ABOUT_TEXT, APP_VERSION, AUTHORS};
use crate::scan::{cheat_list_text, ScanState};
use crate::system::{computer_name, os_info_label, user_name, windows_install_date};
use crate::theme::ThemeId;
use crate::tools::catalog::autocheck_search_status_line;
use crate::tools::paths::tool_installed;
use crate::tools::{open_recycle_bin, open_telegram, run_system_info, run_util, UTILS};

use super::app::{ComponentStatus, CubeCheckApp, View};
use super::color_picker;
use super::glow;
use super::layout;
use super::widgets::{
    settings_checkbox_row, settings_choice_row, settings_row, settings_section_title,
    settings_slider_row, settings_wide_button, SETTINGS_BODY_SIZE, SETTINGS_FORM_WIDTH,
    SETTINGS_ROW_GAP, SETTINGS_ROW_HEIGHT, SETTINGS_SECTION_GAP,
};

pub(super) fn draw_utils_panel(app: &mut CubeCheckApp, ui: &mut egui::Ui) {
    layout::layout_move(ui, "utils.title", |ui| {
        ui.label(
            egui::RichText::new("УТИЛИТЫ")
                .size(22.0)
                .strong()
                .color(app.colors.fg),
        );
    });
    ui.add_space(4.0);
    layout::layout_move(ui, "utils.hint", |ui| {
        ui.label(
            egui::RichText::new("Нажмите «Открыть».")
                .size(13.0)
                .color(app.colors.text_dim),
        );
    });
    ui.add_space(12.0);

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for (i, util) in UTILS.iter().enumerate() {
                let is_selected = matches!(app.view, View::Util(idx) if idx == i);
                let key = util.id.to_string();
                let name = util.name;
                let desc = util.desc;
                let installed = tool_installed(&key);
                let status = if installed {
                    "установлен"
                } else {
                    "не установлен"
                };

                let clicked = layout::layout_move(ui, &format!("utils.card.{key}"), |ui| {
                    let card = app.util_card_frame(is_selected).show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                let reserve = if util.is_search() { 300.0 } else { 160.0 };
                                ui.set_max_width((ui.available_width() - reserve).max(120.0));
                                ui.horizontal(|ui| {
                                    ui.label(
                                        egui::RichText::new(name)
                                            .size(16.0)
                                            .strong()
                                            .color(app.colors.fg),
                                    );
                                    ui.label(
                                        egui::RichText::new(status)
                                            .size(12.0)
                                            .color(if installed {
                                                app.colors.accent
                                            } else {
                                                app.colors.text_dim
                                            }),
                                    );
                                });
                                ui.add_space(4.0);
                                ui.label(
                                    egui::RichText::new(desc)
                                        .size(12.0)
                                        .color(app.colors.text_dim),
                                );
                            });
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if installed {
                                    if app.accent_button(ui, "ОТКРЫТЬ").clicked() {
                                        if let Err(e) = run_util(&key) {
                                            app.set_status(e, true);
                                        }
                                        if util.is_in_app() {
                                            app.view = View::Util(i);
                                        }
                                    }
                                } else if app
                                    .accent_button(
                                        ui,
                                        if cfg!(windows) {
                                            "Скачать"
                                        } else {
                                            "Как поставить"
                                        },
                                    )
                                    .clicked()
                                {
                                    if crate::download::downloads_enabled() {
                                        app.view = View::Components;
                                        app.start_downloads(vec![key.clone()], false);
                                    } else if crate::tools::paths::is_offline() {
                                        app.set_status(
                                            format!(
                                                "{name} нет в assets/bin. Офлайн-сборка не качает файлы из сети."
                                            ),
                                            true,
                                        );
                                    } else {
                                        app.view = View::Components;
                                        #[cfg(not(windows))]
                                        {
                                            ui.output_mut(|o| {
                                                o.copied_text =
                                                    crate::tools::posix::install_hint(&key)
                                                        .to_string();
                                            });
                                            app.set_status("Команда установки скопирована", false);
                                        }
                                        #[cfg(windows)]
                                        {
                                            app.set_status(
                                                "Эта утилита работает только в Windows.".to_string(),
                                                true,
                                            );
                                        }
                                    }
                                }
                                if util.is_search()
                                    && app
                                        .accent_button(ui, "Копировать список")
                                        .on_hover_text("Скопировать названия читов")
                                        .clicked()
                                {
                                    ui.output_mut(|o| o.copied_text = cheat_list_text());
                                    app.set_status("Список скопирован", false);
                                }
                            });
                        });
                    });
                    card.response.clicked()
                });

                if clicked {
                    app.view = View::Util(i);
                }

                ui.add_space(8.0);
            }

            let recycle_selected = matches!(app.view, View::Recycle);
            let recycle_clicked = layout::layout_move(ui, "utils.card.recycle", |ui| {
                let card = app.util_card_frame(recycle_selected).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.set_max_width((ui.available_width() - 160.0).max(120.0));
                            ui.label(
                                egui::RichText::new("Корзина")
                                    .size(16.0)
                                    .strong()
                                    .color(app.colors.fg),
                            );
                            ui.add_space(4.0);
                            ui.label(
                                egui::RichText::new("Открыть корзину Windows.")
                                    .size(12.0)
                                    .color(app.colors.text_dim),
                            );
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if app.accent_button(ui, "ОТКРЫТЬ").clicked() {
                                if let Err(e) = open_recycle_bin() {
                                    app.set_status(e, true);
                                }
                            }
                        });
                    });
                });
                card.response.clicked()
            });
            if recycle_clicked {
                app.view = View::Recycle;
            }
        });
}

pub(super) fn draw_components(app: &mut CubeCheckApp, ui: &mut egui::Ui) {
    layout::layout_move(ui, "components.title", |ui| {
        ui.label(
            egui::RichText::new("КОМПОНЕНТЫ")
                .size(22.0)
                .strong()
                .color(app.colors.fg),
        );
    });
    ui.add_space(4.0);
    layout::layout_move(ui, "components.hint", |ui| {
        let hint = if crate::tools::paths::is_offline() {
            "Офлайн-сборка: программы уже в assets/, без загрузки из сети."
        } else {
            "Скачивание программ с официальных сайтов."
        };
        ui.label(
            egui::RichText::new(hint)
                .size(13.0)
                .color(app.colors.text_dim),
        );
    });
    ui.add_space(10.0);

    let snapshot = app.refresh_component_snapshot();
    let missing = CubeCheckApp::pending_download_ids(&snapshot);

    ui.horizontal(|ui| {
        if layout::layout_move(ui, "components.button.download_all", |ui| {
            app.accent_button(ui, "Скачать все").clicked()
        }) {
            app.start_downloads(missing, false);
        }
        if app.is_download_busy() {
            layout::layout_move(ui, "components.status", |ui| {
                ui.label(
                    egui::RichText::new("Скачивается...")
                        .size(13.0)
                        .color(app.colors.accent),
                );
            });
        }
    });
    ui.add_space(10.0);

    egui::ScrollArea::vertical()
        .id_salt("components_cards")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.add_space(2.0);
            for util in UTILS {
                let status = snapshot
                    .get(util.id)
                    .cloned()
                    .unwrap_or(ComponentStatus::Missing);
                draw_install_card(app, ui, util.id, util.name, util.desc, &status);
                ui.add_space(8.0);
            }
        });
}

fn draw_install_card(
    app: &mut CubeCheckApp,
    ui: &mut egui::Ui,
    id: &str,
    name: &str,
    desc: &str,
    status: &ComponentStatus,
) {
    app.util_card_frame(false).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.set_max_width((ui.available_width() - 160.0).max(120.0));
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(name)
                            .size(16.0)
                            .strong()
                            .color(app.colors.fg),
                    );
                    ui.label(
                        egui::RichText::new(status_text(status))
                            .size(12.0)
                            .color(status_color(app, status)),
                    );
                });
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(desc)
                        .size(12.0)
                        .color(app.colors.text_dim),
                );
                match status {
                    ComponentStatus::Downloading { received, total } => {
                        ui.add_space(4.0);
                        let frac = match total {
                            Some(t) if *t > 0 => *received as f32 / *t as f32,
                            _ => 0.0,
                        };
                        let text = if let Some(t) = total {
                            format!(
                                "{:.1} / {:.1} МБ",
                                *received as f32 / 1_048_576.0,
                                *t as f32 / 1_048_576.0
                            )
                        } else if *received > 0 {
                            format!("{:.1} МБ", *received as f32 / 1_048_576.0)
                        } else {
                            String::new()
                        };
                        app.progress_track(ui, frac, &text);
                    }
                    ComponentStatus::Verifying => {
                        ui.add_space(4.0);
                        app.progress_track(ui, 1.0, "проверка файла");
                    }
                    ComponentStatus::Extracting => {
                        ui.add_space(4.0);
                        app.progress_track(ui, 1.0, "распаковка");
                    }
                    ComponentStatus::Failed(err) => {
                        ui.add_space(4.0);
                        ui.label(
                            egui::RichText::new(err)
                                .size(11.0)
                                .color(egui::Color32::from_rgb(220, 90, 90)),
                        );
                    }
                    _ => {}
                }
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                match status {
                    ComponentStatus::Ready => {
                        if app.accent_button(ui, "ОТКРЫТЬ").clicked() {
                            if let Err(e) = run_util(id) {
                                app.set_status(e, true);
                            }
                        }
                    }
                    ComponentStatus::Failed(_) | ComponentStatus::Missing => {
                        if app.accent_button(ui, "Повтор").clicked() {
                            app.start_downloads(vec![id.to_string()], true);
                        }
                    }
                    _ => {
                        app.reserve_action_button(ui);
                    }
                }
            });
        });
    });
}

fn status_color(app: &CubeCheckApp, status: &ComponentStatus) -> egui::Color32 {
    match status {
        ComponentStatus::Ready => app.colors.accent,
        ComponentStatus::Failed(_) => egui::Color32::from_rgb(220, 90, 90),
        ComponentStatus::Downloading { .. }
        | ComponentStatus::Verifying
        | ComponentStatus::Extracting => app.colors.fg,
        ComponentStatus::Missing => app.colors.text_dim,
    }
}

fn status_text(status: &ComponentStatus) -> String {
    match status {
        ComponentStatus::Ready => "установлен".into(),
        ComponentStatus::Missing => "не установлен".into(),
        ComponentStatus::Downloading { .. } => "загрузка".into(),
        ComponentStatus::Verifying => "проверка".into(),
        ComponentStatus::Extracting => "распаковка".into(),
        ComponentStatus::Failed(_) => "ошибка".into(),
    }
}

pub(super) fn draw_auto_check(app: &mut CubeCheckApp, ui: &mut egui::Ui) {
    layout::layout_move(ui, "autocheck.title", |ui| {
        ui.label(
            egui::RichText::new("АВТОПРОВЕРКА")
                .size(22.0)
                .strong()
                .color(app.colors.fg),
        );
    });
    ui.add_space(12.0);

    let mut text = String::new();
    if let Some(phase) = app.scan_phase {
        text.push_str(&format!("Сканирование: {}...\n", phase.label()));
    } else if !app.scan_started {
        text.push_str("Нажмите «Автопроверка» слева.");
    } else if let Ok(state) = app.scan_state.try_lock() {
        if matches!(*state, ScanState::Running(_)) {
            text.push_str("Сканирование...");
        }
    }

    if app.scan_started && app.scan_phase.is_none() {
        let findings: Vec<&String> = app
            .saved_results
            .iter()
            .filter(|line| !line.starts_with("КОРЗИНА"))
            .collect();
        if findings.is_empty() {
            text.push_str("Ничего подозрительного не найдено\n");
            for line in &app.saved_results {
                text.push_str(&format!("{line}\n"));
            }
        } else {
            text.push_str("Найдено:\n\n");
            for (i, line) in app.saved_results.iter().enumerate() {
                text.push_str(&format!("№{}  {line}\n", i + 1));
            }
        }
        text.push_str(&format!("\n{}\n", autocheck_search_status_line()));
    }

    layout::layout_move(ui, "autocheck.body", |ui| {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(text)
                            .monospace()
                            .size(12.0)
                            .color(app.colors.text_dim),
                    )
                    .wrap(),
                );
            });
    });
}

pub(super) fn draw_about(app: &CubeCheckApp, ui: &mut egui::Ui) {
    let glow = app.glow_for(GlowArea::About);
    glow::request_repaint(ui, ui.max_rect(), glow);
    egui::ScrollArea::vertical().show(ui, |ui| {
        glow::wrapped(
            ui,
            egui::RichText::new(ABOUT_TEXT)
                .monospace()
                .size(12.0)
                .color(app.colors.text_dim),
            glow,
        );
    });
}

pub(super) fn draw_system(app: &mut CubeCheckApp, ui: &mut egui::Ui) {
    let glow = app.glow_for(GlowArea::System);
    glow::request_repaint(ui, ui.max_rect(), glow);
    glow::label(
        ui,
        egui::RichText::new("СИСТЕМА")
            .size(22.0)
            .strong()
            .color(app.colors.fg),
        glow,
    );
    ui.add_space(12.0);

    info_row(app, ui, "Пользователь", &user_name());
    info_row(app, ui, "Имя компьютера", &computer_name());
    info_row(app, ui, &os_info_label(), &windows_install_date());

    ui.add_space(16.0);
    let glow = app.glow_for(GlowArea::System);
    if app.accent_button_glow(ui, "О системе", glow).clicked() {
        if let Err(e) = run_system_info() {
            app.set_status(e, true);
        }
    }
}

fn info_row(app: &CubeCheckApp, ui: &mut egui::Ui, label: &str, value: &str) {
    let glow = app.glow_for(GlowArea::System);
    ui.horizontal(|ui| {
        glow::label(
            ui,
            egui::RichText::new(format!("{label}:"))
                .size(13.0)
                .color(app.colors.section),
            glow,
        );
        glow::label(
            ui,
            egui::RichText::new(value)
                .size(14.0)
                .strong()
                .color(app.colors.fg),
            glow,
        );
    });
    ui.add_space(6.0);
}

pub(super) fn draw_settings(app: &mut CubeCheckApp, ui: &mut egui::Ui) {
    let form_w = SETTINGS_FORM_WIDTH.min(ui.available_width());
    ui.vertical(|ui| {
        ui.set_width(form_w);
        ui.label(
            egui::RichText::new("НАСТРОЙКИ")
                .size(22.0)
                .strong()
                .color(app.colors.fg),
        );
        ui.add_space(16.0);

        let reset_h = 40.0;
        let scroll_h = (ui.available_height() - reset_h - 8.0).max(80.0);
        egui::ScrollArea::vertical()
            .id_salt("settings_form")
            .auto_shrink([false, false])
            .max_height(scroll_h)
            .show(ui, |ui| {
                ui.set_width(form_w);
                ui.spacing_mut().item_spacing.y = 0.0;
                ui.spacing_mut().interact_size.y = 22.0;

                draw_theme_section(app, ui);
                ui.add_space(SETTINGS_SECTION_GAP);
                draw_glow_section(app, ui);
                ui.add_space(SETTINGS_SECTION_GAP);
                draw_areas_section(app, ui);
                ui.add_space(SETTINGS_SECTION_GAP);
                draw_autosave_section(app, ui);
            });

        ui.add_space(8.0);
        layout::layout_move(ui, "reset.button", |ui| {
            let colors = app.colors;
            if settings_wide_button(ui, &colors, "Сброс всех настроек").clicked() {
                app.show_reset_confirm = true;
            }
        });
    });
}

fn draw_theme_section(app: &mut CubeCheckApp, ui: &mut egui::Ui) {
    let colors = app.colors;
    settings_section_title(ui, &colors, "Тема");

    let mut chosen = None;
    let themes = ThemeId::ALL;
    let gap = 8.0;
    let n = themes.len() as f32;
    let chip_w = ((ui.available_width() - gap * (n - 1.0)) / n - 1.0).max(96.0);

    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(gap, 8.0);
        for theme in themes {
            let is_selected = app.theme_id == theme;
            let fill = if is_selected {
                app.colors.select
            } else {
                app.colors.button_bg
            };
            let text_color = if is_selected {
                app.colors.fg
            } else {
                app.colors.text_dim
            };
            let btn = egui::Button::new(
                egui::RichText::new(theme.label())
                    .size(SETTINGS_BODY_SIZE)
                    .color(text_color),
            )
            .fill(fill)
            .stroke(app.colors.chip_stroke(is_selected))
            .rounding(egui::Rounding::same(6.0))
            .min_size(egui::vec2(chip_w, SETTINGS_ROW_HEIGHT));
            if ui.add(btn).clicked() {
                chosen = Some(theme);
            }
        }
    });
    if let Some(theme) = chosen {
        if theme != app.theme_id {
            app.apply_theme(ui.ctx(), theme);
        }
    }
}

fn draw_glow_section(app: &mut CubeCheckApp, ui: &mut egui::Ui) {
    let colors = app.colors;
    settings_section_title(ui, &colors, "Подсветка");

    let mut changed = false;
    changed |= settings_checkbox_row(ui, &colors, "Включена", &mut app.config.glow.enabled);

    settings_row(ui, &colors, "Цвет", |ui| {
        ui.spacing_mut().interact_size.y = 22.0;
        changed |= color_picker::color_edit_button_srgb(ui, "glow_color", &mut app.config.glow.color)
            .changed();
    });
    if app.config.glow.gradient {
        settings_row(ui, &colors, "Цвет 2", |ui| {
            ui.spacing_mut().interact_size.y = 22.0;
            changed |= color_picker::color_edit_button_srgb(
                ui,
                "glow_color2",
                &mut app.config.glow.color2,
            )
            .changed();
        });
    }

    changed |= settings_checkbox_row(ui, &colors, "Градиент", &mut app.config.glow.gradient);

    if app.config.glow.gradient {
        changed |= settings_slider_row(
            ui,
            colors,
            "Скорость",
            &mut app.config.glow.gradient_speed,
            GLOW_SPEED_MIN..=GLOW_SPEED_MAX,
            2,
        );
    }
    changed |= settings_slider_row(
        ui,
        colors,
        "Радиус",
        &mut app.config.glow.radius,
        GLOW_RADIUS_MIN..=GLOW_RADIUS_MAX,
        0,
    );
    changed |= settings_slider_row(
        ui,
        colors,
        "Интенсивность",
        &mut app.config.glow.intensity,
        GLOW_INTENSITY_MIN..=GLOW_INTENSITY_MAX,
        2,
    );

    if changed {
        app.config.glow.sanitize();
        app.persist_after_change();
    }
}

fn draw_areas_section(app: &mut CubeCheckApp, ui: &mut egui::Ui) {
    let colors = app.colors;
    settings_section_title(ui, &colors, "Области");

    let gap = 12.0;
    let col_w = ((ui.available_width() - gap) * 0.5).max(120.0);
    let mut changed = false;

    let mut cell = |ui: &mut egui::Ui, checked: &mut bool, label: &str| {
        ui.allocate_ui_with_layout(
            egui::vec2(col_w, SETTINGS_ROW_HEIGHT),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                let label = egui::RichText::new(label)
                    .size(SETTINGS_BODY_SIZE)
                    .color(colors.fg);
                if ui.checkbox(checked, label).changed() {
                    changed = true;
                }
            },
        );
    };

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = gap;
        cell(ui, &mut app.config.glow.areas.sidebar, "Меню");
        cell(ui, &mut app.config.glow.areas.about, "О программе");
    });
    ui.add_space(SETTINGS_ROW_GAP);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = gap;
        cell(ui, &mut app.config.glow.areas.system, "Система");
        cell(ui, &mut app.config.glow.areas.footer, "Подвал");
    });

    if changed {
        app.config.glow.sanitize();
        app.persist_after_change();
    }
}

fn draw_autosave_section(app: &mut CubeCheckApp, ui: &mut egui::Ui) {
    let colors = app.colors;
    settings_section_title(ui, &colors, "Автосохранение");

    let prev = app.config.autosave;
    let mut changed = false;
    for mode in AutosaveMode::ALL {
        settings_choice_row(ui, |ui| {
            let label = egui::RichText::new(mode.label())
                .size(SETTINGS_BODY_SIZE)
                .color(colors.fg);
            changed |= ui
                .radio_value(&mut app.config.autosave, mode, label)
                .changed();
        });
    }
    if changed {
        app.persist_autosave_change(prev);
    }
}

const FOOTER_GLOW_GOLD: egui::Color32 = egui::Color32::from_rgb(212, 175, 55);
const FOOTER_GLOW_RADIUS: f32 = 34.0;

fn apply_footer_cursor_glow(
    galley: &egui::Galley,
    galley_pos: egui::Pos2,
    pointer: egui::Pos2,
) -> Option<std::sync::Arc<egui::Galley>> {
    let mut glow = galley.clone();
    let radius = FOOTER_GLOW_RADIUS;
    let r2 = radius * radius;
    let mut any = false;
    for row in &mut glow.rows {
        for v in &mut row.visuals.mesh.vertices {
            let world = galley_pos + v.pos.to_vec2();
            let d2 = world.distance_sq(pointer);
            if d2 >= r2 {
                v.color = egui::Color32::TRANSPARENT;
                continue;
            }
            let t = (1.0 - d2.sqrt() / radius).clamp(0.0, 1.0);
            let t = t * t * (3.0 - 2.0 * t);
            if t > 0.02 {
                any = true;
            }
            v.color = FOOTER_GLOW_GOLD.gamma_multiply(t);
        }
    }
    any.then(|| std::sync::Arc::new(glow))
}

fn footer_piece(
    ui: &mut egui::Ui,
    text: impl Into<String>,
    idle: egui::Color32,
    pointer: Option<egui::Pos2>,
    clickable: bool,
) -> egui::Response {
    let font_id = egui::FontId::proportional(12.0);
    let galley = ui.fonts(|f| f.layout_no_wrap(text.into(), font_id, idle));
    let sense = if clickable {
        egui::Sense::click()
    } else {
        egui::Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(galley.size(), sense);
    let galley_pos = egui::pos2(rect.left(), rect.center().y - galley.size().y * 0.5);
    ui.painter().galley(galley_pos, galley.clone(), idle);

    if let Some(pointer) = pointer {
        if rect.expand(FOOTER_GLOW_RADIUS).contains(pointer) {
            if let Some(glow) = apply_footer_cursor_glow(&galley, galley_pos, pointer) {
                ui.painter().galley(galley_pos, glow, FOOTER_GLOW_GOLD);
            }
        }
    }

    if clickable {
        response.on_hover_cursor(egui::CursorIcon::PointingHand)
    } else {
        response
    }
}

pub(super) fn draw_footer(app: &CubeCheckApp, ui: &mut egui::Ui) {
    glow::request_repaint(ui, ui.max_rect(), app.glow_for(GlowArea::Footer));
    let pointer = ui.input(|i| i.pointer.hover_pos());
    if pointer.is_some_and(|p| ui.max_rect().expand(FOOTER_GLOW_RADIUS).contains(p)) {
        ui.ctx().request_repaint();
    }
    ui.horizontal(|ui| {
        footer_piece(
            ui,
            format!("CubeCheck {APP_VERSION}"),
            app.colors.footer,
            pointer,
            false,
        );
        ui.label(egui::RichText::new("  •  ").color(app.colors.border));
        if footer_piece(ui, "@cubecheck", app.colors.footer, pointer, true).clicked() {
            open_telegram();
        }
        ui.label(egui::RichText::new("  •  ").color(app.colors.border));
        footer_piece(
            ui,
            format!("авторы: {AUTHORS}"),
            app.colors.footer,
            pointer,
            false,
        );
    });
}
