use eframe::egui;

use crate::config::{
    AutosaveMode, GlowArea, GLOW_INTENSITY_MAX, GLOW_INTENSITY_MIN, GLOW_RADIUS_MAX,
    GLOW_RADIUS_MIN, GLOW_SPEED_MAX, GLOW_SPEED_MIN,
};
use crate::content::{ABOUT_TEXT, APP_VERSION, AUTHORS};
use crate::scan::{cheat_list_text, ScanState};
use crate::system::{computer_name, os_info_label, recycle_bin_last_change, user_name, windows_install_date};
use crate::theme::ThemeId;
use crate::tools::catalog::{autocheck_search_status_line, util_index};
#[cfg(not(windows))]
use crate::tools::catalog::load_inspect;
use crate::tools::paths::tool_installed;
use crate::tools::{
    open_recycle_bin, open_telegram, run_system_info, run_util, UTILS,
};
#[cfg(not(windows))]
use crate::tools::open_path;

use super::app::{ComponentStatus, CubeCheckApp, View};
use super::color_picker;
use super::glow;
use super::layout;
use super::widgets::{
    settings_checkbox_row, settings_choice_row, settings_row, settings_section_title,
    settings_slider_row, settings_wide_button, SETTINGS_BODY_SIZE, SETTINGS_ROW_HEIGHT,
    SETTINGS_SCROLL_GUTTER, SETTINGS_SECTION_GAP,
};

pub(super) fn draw_utils_panel(app: &mut CubeCheckApp, ui: &mut egui::Ui) {
    #[cfg(not(windows))]
    {
        if let View::Util(i) = app.view {
            if let Some(util) = UTILS.get(i).copied() {
                if util.is_in_app() {
                    draw_inspect_panel(app, ui, util);
                    return;
                }
            }
        }
    }

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
                let installed = tool_installed(util.id);
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
                                        if util.is_in_app() {
                                            app.view = View::Util(i);
                                        } else if let Err(e) = run_util(&key) {
                                            app.set_status(e, true);
                                        }
                                    }
                                } else if app.accent_button(ui, if cfg!(windows) { "Скачать" } else { "Как поставить" }).clicked() {
                                    if crate::download::downloads_enabled() {
                                        app.view = View::Components;
                                        app.start_downloads(vec![key.clone()], false);
                                    } else if crate::tools::paths::is_offline() && cfg!(windows) {
                                        app.set_status(
                                            format!(
                                                "{name} нет в assets/. Офлайн-сборка не качает файлы из сети."
                                            ),
                                            true,
                                        );
                                    } else {
                                        app.view = View::Components;
                                        #[cfg(not(windows))]
                                        {
                                            ui.output_mut(|o| {
                                                o.copied_text = crate::tools::posix::install_hint(&key).to_string();
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

            draw_recycle_card(app, ui);
        });
}

fn draw_recycle_card(app: &mut CubeCheckApp, ui: &mut egui::Ui) {
    let is_selected = matches!(app.view, View::Recycle);
    let clicked = layout::layout_move(ui, "utils.card.recycle", |ui| {
        let card = app.util_card_frame(is_selected).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.set_max_width((ui.available_width() - 180.0).max(120.0));
                    ui.label(
                        egui::RichText::new("Корзина")
                            .size(16.0)
                            .strong()
                            .color(app.colors.fg),
                    );
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new(if cfg!(windows) {
                            "Открыть корзину Windows."
                        } else if cfg!(target_os = "macos") {
                            "Открыть ~/.Trash."
                        } else {
                            "Открыть корзину XDG (trash:///)."
                        })
                            .size(12.0)
                            .color(app.colors.text_dim),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if app
                        .accent_button(ui, "Открыть корзину")
                        .on_hover_text("Открыть корзину")
                        .clicked()
                    {
                        if let Err(e) = open_recycle_bin() {
                            app.set_status(e, true);
                        }
                    }
                });
            });
        });
        card.response.clicked()
    });

    if clicked {
        app.view = View::Recycle;
    }
}

#[cfg(not(windows))]
fn draw_inspect_panel(app: &mut CubeCheckApp, ui: &mut egui::Ui, util: crate::tools::Util) {
    if app.inspect_id != util.id {
        app.inspect_id = util.id.to_string();
        app.inspect_rows = load_inspect(util.id);
    }

    ui.horizontal(|ui| {
        if layout::layout_move(ui, "inspect.back", |ui| {
            app.accent_button(ui, "Назад").clicked()
        }) {
            app.inspect_id.clear();
            app.view = View::Util(0);
        }
        if layout::layout_move(ui, "inspect.refresh", |ui| {
            app.accent_button(ui, "Обновить").clicked()
        }) {
            app.inspect_rows = load_inspect(util.id);
        }
        if util.id == "autoruns" {
            if layout::layout_move(ui, "inspect.open_ext", |ui| {
                app.accent_button(ui, "Системные настройки").clicked()
            }) {
                if let Err(e) = run_util("autoruns") {
                    app.set_status(e, true);
                }
            }
        }
    });
    ui.add_space(8.0);
    layout::layout_move(ui, "inspect.title", |ui| {
        ui.label(
            egui::RichText::new(util.name)
                .size(22.0)
                .strong()
                .color(app.colors.fg),
        );
    });
    ui.add_space(4.0);
    layout::layout_move(ui, "inspect.hint", |ui| {
        ui.label(
            egui::RichText::new(util.desc)
                .size(13.0)
                .color(app.colors.text_dim),
        );
    });
    ui.add_space(10.0);

    if app.inspect_rows.is_empty() {
        layout::layout_move(ui, "inspect.empty", |ui| {
            ui.label(
                egui::RichText::new("Список пуст.")
                    .size(13.0)
                    .color(app.colors.text_dim),
            );
        });
        return;
    }

    let rows = app.inspect_rows.clone();
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for (i, row) in rows.iter().enumerate() {
                let clicked = layout::layout_move(ui, &format!("inspect.row.{i}"), |ui| {
                    let card = app.util_card_frame(false).show(ui, |ui| {
                        ui.label(
                            egui::RichText::new(&row.title)
                                .size(14.0)
                                .strong()
                                .color(app.colors.fg),
                        );
                        if !row.detail.is_empty() {
                            ui.label(
                                egui::RichText::new(&row.detail)
                                    .size(12.0)
                                    .color(app.colors.text_dim),
                            );
                        }
                    });
                    card.response.clicked()
                });
                if clicked {
                    if let Some(path) = row.path.as_ref() {
                        if let Err(e) = open_path(path) {
                            app.set_status(e, true);
                        }
                    }
                }
                ui.add_space(6.0);
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
        let hint = if crate::tools::paths::is_offline() && cfg!(windows) {
            "Офлайн-сборка: файлы берутся из папки assets, без загрузки из сети."
        } else if !cfg!(windows) {
            "На Linux/macOS CubeCheck не скачивает сторонние .exe. Нужны системные программы (пакетный менеджер) или встроенные панели."
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

    if cfg!(windows) {
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
    }

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for util in UTILS {
                let status = snapshot
                    .get(util.id)
                    .cloned()
                    .unwrap_or(ComponentStatus::Missing);
                draw_install_card(app, ui, util.id, util.name, &status);
                ui.add_space(8.0);
            }
        });
}

fn draw_install_card(
    app: &mut CubeCheckApp,
    ui: &mut egui::Ui,
    id: &str,
    name: &str,
    status: &ComponentStatus,
) {
    let progress = match status {
        ComponentStatus::Downloading { received, total } => {
            let frac = match total {
                Some(t) if *t > 0 => *received as f32 / *t as f32,
                _ => 0.0,
            };
            let text = if *received > 0 {
                if let Some(t) = total {
                    format!(
                        "{:.1} / {:.1} МБ",
                        *received as f32 / 1_048_576.0,
                        *t as f32 / 1_048_576.0
                    )
                } else {
                    format!("{:.1} МБ", *received as f32 / 1_048_576.0)
                }
            } else {
                String::new()
            };
            Some((frac, text))
        }
        ComponentStatus::Verifying | ComponentStatus::Extracting => Some((1.0, String::new())),
        _ => None,
    };

    layout::layout_move(ui, &format!("components.card.{id}"), |ui| {
    app.util_card_frame(false).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;

            ui.label(
                egui::RichText::new(name)
                    .size(16.0)
                    .strong()
                    .color(app.colors.fg),
            );
            ui.add_space(14.0);
            let status_resp = ui.label(
                egui::RichText::new(status_text(status))
                    .size(12.0)
                    .color(status_color(app, status)),
            );
            if let ComponentStatus::Failed(err) = status {
                status_resp.on_hover_text(err);
            }

            if let Some((frac, text)) = progress {
                ui.add_space(12.0);
                let bar_w =
                    (ui.available_width() - CubeCheckApp::ACTION_BUTTON_SIZE.x - 12.0).max(40.0);
                ui.allocate_ui_with_layout(
                    egui::vec2(bar_w, CubeCheckApp::ACTION_BUTTON_SIZE.y),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        app.progress_track(ui, frac, &text);
                    },
                );
                ui.add_space(12.0);
            } else {
                ui.add_space((ui.available_width() - CubeCheckApp::ACTION_BUTTON_SIZE.x).max(0.0));
            }

            match status {
                ComponentStatus::Ready => {
                    if app.accent_button(ui, "ОТКРЫТЬ").clicked() {
                        if let Some(i) = util_index(id) {
                            if UTILS[i].is_in_app() {
                                app.view = View::Util(i);
                            } else if let Err(e) = run_util(id) {
                                app.set_status(e, true);
                            }
                        } else if let Err(e) = run_util(id) {
                            app.set_status(e, true);
                        }
                    }
                }
                ComponentStatus::Failed(_) => {
                    if app.accent_button(ui, "Повтор").clicked() {
                        app.start_downloads(vec![id.to_string()], true);
                    }
                }
                ComponentStatus::Missing => {
                    #[cfg(windows)]
                    {
                        if app.accent_button(ui, "Скачать").clicked() {
                            app.start_downloads(vec![id.to_string()], false);
                        }
                    }
                    #[cfg(not(windows))]
                    {
                        let hint = crate::tools::posix::install_hint(id);
                        if app
                            .accent_button(ui, "Команда")
                            .on_hover_text(hint)
                            .clicked()
                        {
                            ui.output_mut(|o| o.copied_text = hint.to_string());
                            app.set_status("Команда установки скопирована", false);
                        }
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
    layout::layout_move(ui, "about.block", |ui| {
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
    });
}

pub(super) fn draw_system(app: &mut CubeCheckApp, ui: &mut egui::Ui) {
    let glow = app.glow_for(GlowArea::System);
    glow::request_repaint(ui, ui.max_rect(), glow);
    layout::layout_move(ui, "system.title", |ui| {
        glow::label(
            ui,
            egui::RichText::new("СИСТЕМА")
                .size(22.0)
                .strong()
                .color(app.colors.fg),
            glow,
        );
    });
    ui.add_space(12.0);

    info_row(app, ui, "system.row.user", "Пользователь", &user_name());
    info_row(app, ui, "system.row.computer", "Имя компьютера", &computer_name());
    info_row(app, ui, "system.row.windows", os_info_label(), &windows_install_date());
    info_row(
        app,
        ui,
        "system.row.recycle",
        "Последнее изменение корзины",
        &recycle_bin_last_change(),
    );

    ui.add_space(16.0);
    let glow = app.glow_for(GlowArea::System);
    if layout::layout_move(ui, "system.button.sysinfo", |ui| {
        app.accent_button_glow(ui, "О системе", glow).clicked()
    }) {
        if let Err(e) = run_system_info() {
            app.set_status(e, true);
        }
    }
}

fn info_row(app: &CubeCheckApp, ui: &mut egui::Ui, id: &str, label: &str, value: &str) {
    let glow = app.glow_for(GlowArea::System);
    layout::layout_move(ui, id, |ui| {
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
    });
    ui.add_space(6.0);
}

pub(super) fn draw_settings(app: &mut CubeCheckApp, ui: &mut egui::Ui) {
    let pane_w = ui.available_width();
    ui.set_min_width(pane_w);
    ui.set_max_width(pane_w);

    layout::layout_move(ui, "settings.title", |ui| {
        ui.label(
            egui::RichText::new("НАСТРОЙКИ")
                .size(22.0)
                .strong()
                .color(app.colors.fg),
        );
    });
    ui.add_space(16.0);

    const RESET_BLOCK: f32 = 48.0;
    let scroll_h = (ui.available_height() - RESET_BLOCK).max(120.0);

    egui::ScrollArea::vertical()
        .id_salt("settings_form")
        .max_height(scroll_h)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let content_w = ui.available_width();
            ui.set_min_width(content_w);
            ui.set_max_width(content_w);
            egui::Frame::none()
                .inner_margin(egui::Margin {
                    left: 0.0,
                    right: SETTINGS_SCROLL_GUTTER,
                    top: 0.0,
                    bottom: 0.0,
                })
                .show(ui, |ui| {
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
        });

    ui.add_space(12.0);
    let colors = app.colors;
    // Sticky footer: never under the settings scrollbar, never offset by layout_move.
    if layout::layout_move(ui, "reset.button", |ui| {
        settings_wide_button(ui, &colors, "Сброс всех настроек").clicked()
    }) {
        app.show_reset_confirm = true;
    }
}

fn draw_theme_section(app: &mut CubeCheckApp, ui: &mut egui::Ui) {
    let colors = app.colors;
    layout::layout_move(ui, "settings.section.theme", |ui| {
        settings_section_title(ui, &colors, "Тема");
    });

    let mut chosen = None;
    let themes = ThemeId::ALL;
    ui.spacing_mut().item_spacing.x = 8.0;
    ui.columns(themes.len(), |cols| {
        for (i, theme) in themes.iter().enumerate() {
            let ui = &mut cols[i];
            let is_selected = app.theme_id == *theme;
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
            let chip_w = ui.available_width();
            let btn = egui::Button::new(
                egui::RichText::new(theme.label())
                    .size(SETTINGS_BODY_SIZE)
                    .color(text_color),
            )
            .fill(fill)
            .stroke(app.colors.chip_stroke(is_selected))
            .rounding(egui::Rounding::same(6.0))
            .min_size(egui::vec2(chip_w, SETTINGS_ROW_HEIGHT));
            if layout::layout_move(ui, &format!("settings.theme.{}", theme.as_key()), |ui| {
                ui.add(btn).clicked()
            }) {
                chosen = Some(*theme);
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
    layout::layout_move(ui, "settings.section.glow", |ui| {
        settings_section_title(ui, &colors, "Подсветка");
    });

    let mut changed = false;
    changed |= layout::layout_move(ui, "settings.glow.enabled", |ui| {
        settings_checkbox_row(ui, &colors, "Включена", &mut app.config.glow.enabled)
    });

    changed |= layout::layout_move(ui, "settings.glow.color", |ui| {
        let mut row_changed = false;
        settings_row(ui, &colors, "Цвет", |ui| {
            ui.spacing_mut().interact_size.y = 22.0;
            row_changed |=
                color_picker::color_edit_button_srgb(ui, "glow_color", &mut app.config.glow.color)
                    .changed();
        });
        row_changed
    });
    if app.config.glow.gradient {
        changed |= layout::layout_move(ui, "settings.glow.color2", |ui| {
            let mut row_changed = false;
            settings_row(ui, &colors, "Цвет 2", |ui| {
                ui.spacing_mut().interact_size.y = 22.0;
                row_changed |= color_picker::color_edit_button_srgb(
                    ui,
                    "glow_color2",
                    &mut app.config.glow.color2,
                )
                .changed();
            });
            row_changed
        });
    }

    changed |= layout::layout_move(ui, "settings.glow.gradient", |ui| {
        settings_checkbox_row(ui, &colors, "Градиент", &mut app.config.glow.gradient)
    });

    if app.config.glow.gradient {
        changed |= layout::layout_move(ui, "settings.slider.speed", |ui| {
            settings_slider_row(
                ui,
                colors,
                "Скорость",
                &mut app.config.glow.gradient_speed,
                GLOW_SPEED_MIN..=GLOW_SPEED_MAX,
                2,
            )
        });
    }
    changed |= layout::layout_move(ui, "settings.slider.radius", |ui| {
        settings_slider_row(
            ui,
            colors,
            "Радиус",
            &mut app.config.glow.radius,
            GLOW_RADIUS_MIN..=GLOW_RADIUS_MAX,
            0,
        )
    });
    changed |= layout::layout_move(ui, "settings.slider.intensity", |ui| {
        settings_slider_row(
            ui,
            colors,
            "Интенсивность",
            &mut app.config.glow.intensity,
            GLOW_INTENSITY_MIN..=GLOW_INTENSITY_MAX,
            2,
        )
    });

    if changed {
        app.config.glow.sanitize();
        app.persist_after_change();
    }
}

fn draw_areas_section(app: &mut CubeCheckApp, ui: &mut egui::Ui) {
    let colors = app.colors;
    layout::layout_move(ui, "settings.section.areas", |ui| {
        settings_section_title(ui, &colors, "Области");
    });

    let mut changed = false;
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(16.0, 8.0);
        let mut chip = |ui: &mut egui::Ui, checked: &mut bool, label: &str, id: &str| {
            layout::layout_move(ui, id, |ui| {
                let label = egui::RichText::new(label)
                    .size(SETTINGS_BODY_SIZE)
                    .color(colors.fg);
                if ui.checkbox(checked, label).changed() {
                    changed = true;
                }
            });
        };
        chip(
            ui,
            &mut app.config.glow.areas.sidebar,
            "Меню",
            "settings.area.sidebar",
        );
        chip(
            ui,
            &mut app.config.glow.areas.about,
            "О программе",
            "settings.area.about",
        );
        chip(
            ui,
            &mut app.config.glow.areas.system,
            "Система",
            "settings.area.system",
        );
        chip(
            ui,
            &mut app.config.glow.areas.footer,
            "Подвал",
            "settings.area.footer",
        );
    });

    if changed {
        app.config.glow.sanitize();
        app.persist_after_change();
    }
}

fn autosave_layout_id(mode: AutosaveMode) -> &'static str {
    match mode {
        AutosaveMode::OnExit => "settings.autosave.on_exit",
        AutosaveMode::OnChange => "settings.autosave.on_change",
        AutosaveMode::Off => "settings.autosave.off",
    }
}

fn draw_autosave_section(app: &mut CubeCheckApp, ui: &mut egui::Ui) {
    let colors = app.colors;
    layout::layout_move(ui, "settings.section.autosave", |ui| {
        settings_section_title(ui, &colors, "Автосохранение");
    });

    let prev = app.config.autosave;
    let mut changed = false;
    for mode in AutosaveMode::ALL {
        layout::layout_move(ui, autosave_layout_id(mode), |ui| {
            settings_choice_row(ui, |ui| {
                let label = egui::RichText::new(mode.label())
                    .size(SETTINGS_BODY_SIZE)
                    .color(colors.fg);
                changed |= ui.radio_value(&mut app.config.autosave, mode, label).changed();
            });
        });
    }
    if changed {
        app.persist_autosave_change(prev);
    }
}

fn footer_piece(
    ui: &mut egui::Ui,
    text: impl Into<String>,
    idle: egui::Color32,
    glow: Option<&crate::config::GlowConfig>,
    clickable: bool,
) -> egui::Response {
    let rich = egui::RichText::new(text.into()).size(12.0).color(idle);
    if clickable {
        glow::clickable(ui, rich, glow)
    } else {
        glow::label(ui, rich, glow)
    }
}

pub(super) fn draw_footer(app: &CubeCheckApp, ui: &mut egui::Ui) {
    let glow = app.glow_for(GlowArea::Footer);
    glow::request_repaint(ui, ui.max_rect(), glow);
    ui.horizontal(|ui| {
        layout::layout_move(ui, "footer.version", |ui| {
            footer_piece(
                ui,
                format!("CubeCheck {APP_VERSION}"),
                app.colors.footer,
                glow,
                false,
            );
        });
        layout::layout_move(ui, "footer.sep1", |ui| {
            glow::label(
                ui,
                egui::RichText::new("  •  ").color(app.colors.text_dim),
                glow,
            );
        });
        if layout::layout_move(ui, "footer.telegram", |ui| {
            footer_piece(ui, "@cubecheck", app.colors.footer, glow, true).clicked()
        }) {
            open_telegram();
        }
        layout::layout_move(ui, "footer.sep2", |ui| {
            glow::label(
                ui,
                egui::RichText::new("  •  ").color(app.colors.text_dim),
                glow,
            );
        });
        layout::layout_move(ui, "footer.authors", |ui| {
            footer_piece(
                ui,
                format!("авторы: {AUTHORS}"),
                app.colors.footer,
                glow,
                false,
            );
        });
    });
}
