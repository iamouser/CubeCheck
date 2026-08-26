use eframe::egui;

use crate::config::GlowArea;
use crate::tools::UTILS;

use super::app::{CubeCheckApp, View};
use super::glow;
use super::layout;

pub(super) fn draw_sidebar(app: &mut CubeCheckApp, ui: &mut egui::Ui) {
    glow::request_repaint(ui, ui.max_rect(), app.glow_for(GlowArea::Sidebar));
    layout::layout_move(ui, "sidebar.title", |ui| {
        glow::label(
            ui,
            egui::RichText::new("МЕНЮ")
                .size(18.0)
                .strong()
                .color(app.colors.fg),
            app.glow_for(GlowArea::Sidebar),
        );
    });
    ui.add_space(10.0);

    layout::layout_move(ui, "sidebar.section.programs", |ui| {
        app.section_label(ui, "ПРОГРАММЫ");
    });

    draw_utils_list(app, ui);

    layout::layout_move(ui, "sidebar.separator.check", |ui| {
        app.separator(ui);
    });
    layout::layout_move(ui, "sidebar.section.check", |ui| {
        app.section_label(ui, "ПРОВЕРКА");
    });

    let item_w = app.sidebar_item_width(ui);
    if layout::layout_move(ui, "sidebar.menu.autocheck", |ui| {
        app.sidebar_button(ui, "Автопроверка", matches!(app.view, View::AutoCheck), item_w)
            .clicked()
    }) {
        app.start_auto_check();
    }
    if layout::layout_move(ui, "sidebar.menu.save_report", |ui| {
        app.sidebar_button(ui, "Сохранить отчёт", false, item_w)
            .clicked()
    }) {
        save_report(app);
    }
    if layout::layout_move(ui, "sidebar.menu.clear_logs", |ui| {
        app.sidebar_button(ui, "Очистить логи", false, item_w)
            .clicked()
    }) {
        app.show_clear_confirm = true;
    }

    layout::layout_move(ui, "sidebar.separator.components", |ui| {
        app.separator(ui);
    });
    layout::layout_move(ui, "sidebar.section.components", |ui| {
        app.section_label(ui, "КОМПОНЕНТЫ");
    });
    if layout::layout_move(ui, "sidebar.menu.components", |ui| {
        app.sidebar_button(
            ui,
            "Компоненты",
            matches!(app.view, View::Components),
            item_w,
        )
        .clicked()
    }) {
        app.view = View::Components;
    }

    layout::layout_move(ui, "sidebar.separator.settings", |ui| {
        app.separator(ui);
    });
    layout::layout_move(ui, "sidebar.section.settings", |ui| {
        app.section_label(ui, "НАСТРОЙКИ");
    });
    if layout::layout_move(ui, "sidebar.menu.settings", |ui| {
        app.sidebar_button(ui, "Настройки", matches!(app.view, View::Settings), item_w)
            .clicked()
    }) {
        app.view = View::Settings;
    }

    layout::layout_move(ui, "sidebar.separator.info", |ui| {
        app.separator(ui);
    });
    layout::layout_move(ui, "sidebar.section.info", |ui| {
        app.section_label(ui, "ИНФО");
    });

    if layout::layout_move(ui, "sidebar.menu.about", |ui| {
        app.sidebar_button(ui, "О программе", matches!(app.view, View::About), item_w)
            .clicked()
    }) {
        app.view = View::About;
    }
    if layout::layout_move(ui, "sidebar.menu.system", |ui| {
        app.sidebar_button(ui, "Система", matches!(app.view, View::System), item_w)
            .clicked()
    }) {
        app.view = View::System;
    }
    if layout::layout_move(ui, "sidebar.menu.holycheck", |ui| {
        app.sidebar_button(ui, "HolyCheck", false, item_w).clicked()
    }) {
        app.show_holy_confirm = true;
    }
}

fn draw_utils_list(app: &mut CubeCheckApp, ui: &mut egui::Ui) {
    let id = egui::Id::new("sidebar_program_list");
    let mut state = egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, false);
    let item_w = app.sidebar_item_width(ui);
    let glow = app.glow_for(GlowArea::Sidebar);
    // sidebar.list — collapsing «Программы» (id kept for layout JSON)
    let header = layout::layout_move(ui, "sidebar.list", |ui| {
        let (rect, response) = ui.allocate_exact_size(egui::vec2(item_w, 28.0), egui::Sense::click());
        let fill = if response.hovered() {
            app.colors.hover
        } else {
            egui::Color32::TRANSPARENT
        };
        ui.painter()
            .rect_filled(rect, egui::Rounding::same(4.0), fill);
        glow::paint_frame_glow(ui, rect, 4.0, glow);

        let header = response.on_hover_cursor(egui::CursorIcon::PointingHand);
        if header.clicked() {
            state.toggle(ui);
        }

        let openness = state.openness(ui.ctx());
        let icon_rect = egui::Rect::from_center_size(
            egui::pos2(rect.left() + 12.0, rect.center().y),
            egui::vec2(12.0, 12.0),
        );
        egui::collapsing_header::paint_default_icon(
            ui,
            openness,
            &header.clone().with_new_rect(icon_rect),
        );

        let font_id = egui::FontId::proportional(13.0);
        let galley = ui.fonts(|f| f.layout_no_wrap("Программы".to_owned(), font_id, app.colors.fg));
        let galley_pos = egui::pos2(rect.left() + 24.0, rect.center().y - galley.size().y * 0.5);
        glow::paint_at(ui, galley_pos, galley.as_ref(), app.colors.fg, glow);
        header
    });

    ui.visuals_mut().indent_has_left_vline = false;
    state.show_body_indented(&header, ui, |ui| {
        for (i, util) in UTILS.iter().enumerate() {
            let selected = matches!(app.view, View::Util(idx) if idx == i);
            let full_w = app.sidebar_item_width(ui);
            if layout::layout_move(ui, &format!("sidebar.list.{}", util.id), |ui| {
                app.sidebar_button(ui, util.name, selected, full_w).clicked()
            }) {
                app.view = View::Util(i);
            }
        }
        let full_w = app.sidebar_item_width(ui);
        let recycle_selected = matches!(app.view, View::Recycle);
        if layout::layout_move(ui, "sidebar.list.recycle", |ui| {
            app.sidebar_button(ui, "Корзина", recycle_selected, full_w)
                .clicked()
        }) {
            app.view = View::Recycle;
        }
    });
}

fn save_report(app: &mut CubeCheckApp) {
    let now = chrono::Local::now();
    let filename = format!("{}.yml", now.format("%d.%m.%y-time%H.%M.%S"));
    let dir = crate::tools::paths::reports_dir();
    if let Err(e) = std::fs::create_dir_all(&dir) {
        app.set_status(format!("Не удалось создать папку отчётов: {e}"), true);
        return;
    }
    let filepath = dir.join(filename);

    let mut yaml = String::new();
    yaml.push_str("name: CubeCheck\n");
    yaml.push_str(&format!("authors: {}\n", crate::content::AUTHORS));
    yaml.push_str(&format!("version: \"{}\"\n", crate::content::APP_VERSION));
    yaml.push_str(&format!(
        "saved_at: {}\n",
        yaml_string(&now.format("%d.%m.%Y %H:%M:%S").to_string())
    ));
    yaml.push_str(&format!(
        "computer: {}\n",
        yaml_string(&crate::system::computer_name())
    ));
    yaml.push_str(&format!(
        "windows_install: {}\n",
        yaml_string(&crate::system::windows_install_date())
    ));
    yaml.push_str(&format!(
        "recycle_bin: {}\n",
        yaml_string(&crate::system::recycle_bin_last_change())
    ));
    yaml.push_str("auto_check:\n");
    if app.saved_results.is_empty() {
        yaml.push_str("  ran: false\n");
        yaml.push_str("  findings: []\n");
    } else {
        yaml.push_str("  ran: true\n");
        yaml.push_str("  findings:\n");
        for line in &app.saved_results {
            yaml.push_str(&format!("    - {}\n", yaml_string(line)));
        }
    }
    yaml.push_str("utilities:\n");
    for util in UTILS {
        yaml.push_str(&format!("  - id: {}\n", yaml_string(util.id)));
        yaml.push_str(&format!("    name: {}\n", yaml_string(util.name)));
        yaml.push_str(&format!("    description: {}\n", yaml_string(util.desc)));
    }
    yaml.push_str("channel: telegram.me/cubecheck\n");

    match std::fs::write(&filepath, yaml) {
        Ok(()) => app.set_status(format!("Отчёт сохранён: {}", filepath.display()), false),
        Err(e) => app.set_status(format!("Не удалось сохранить отчёт: {e}"), true),
    }
}

fn yaml_string(value: &str) -> String {
    format!(
        "\"{}\"",
        value
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\r', "")
    )
}
