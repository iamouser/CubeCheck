use std::f32::consts::FRAC_PI_2;

use eframe::egui;
use eframe::egui::epaint::{Mesh, PathShape, Vertex, WHITE_UV};

use crate::config::GlowArea;
use crate::tools::UTILS;

use super::app::{CubeCheckApp, View};
use super::glow;

/// Disclosure triangle drawn as painter geometry only — never a font glyph.
/// Closed (`openness == 0`) points right; open (`1`) points down.
fn paint_collapse_triangle(ui: &egui::Ui, openness: f32, center: egui::Pos2, color: egui::Color32) {
    let openness = openness.clamp(0.0, 1.0);
    if (0.0 < openness) && (openness < 1.0) {
        ui.ctx().request_repaint();
    }
    let size = 4.5;
    let local = [
        egui::vec2(-0.55 * size, -0.85 * size),
        egui::vec2(0.90 * size, 0.0),
        egui::vec2(-0.55 * size, 0.85 * size),
    ];
    let angle = openness * FRAC_PI_2;
    let (sin, cos) = angle.sin_cos();
    let points: Vec<egui::Pos2> = local
        .into_iter()
        .map(|d| center + egui::vec2(d.x * cos - d.y * sin, d.x * sin + d.y * cos))
        .collect();

    ui.painter().add(egui::Shape::Path(PathShape::convex_polygon(
        points.clone(),
        color,
        egui::Stroke::NONE,
    )));

    // Filled mesh fallback so the triangle never depends on font atlases.
    let mut mesh = Mesh {
        texture_id: egui::TextureId::default(),
        ..Default::default()
    };
    for pos in points {
        mesh.vertices.push(Vertex {
            pos,
            uv: WHITE_UV,
            color,
        });
    }
    mesh.indices.extend_from_slice(&[0, 1, 2]);
    ui.painter().add(mesh);
}

pub(super) fn draw_sidebar(app: &mut CubeCheckApp, ui: &mut egui::Ui) {
    glow::request_repaint(ui, ui.max_rect(), app.glow_for(GlowArea::Sidebar));
    glow::label(
        ui,
        egui::RichText::new("МЕНЮ")
            .size(18.0)
            .strong()
            .color(app.colors.fg),
        app.glow_for(GlowArea::Sidebar),
    );
    ui.add_space(10.0);

    app.section_label(ui, "ПРОГРАММЫ");
    draw_utils_list(app, ui);

    app.separator(ui);
    app.section_label(ui, "ПРОВЕРКА");

    let item_w = app.sidebar_item_width(ui);
    if app
        .sidebar_button(ui, "Автопроверка", matches!(app.view, View::AutoCheck), item_w)
        .clicked()
    {
        app.start_auto_check();
    }
    if app
        .sidebar_button(ui, "Сохранить отчёт", false, item_w)
        .clicked()
    {
        save_report(app);
    }
    if app
        .sidebar_button(ui, "Очистить логи", false, item_w)
        .clicked()
    {
        app.show_clear_confirm = true;
    }

    app.separator(ui);
    app.section_label(ui, "КОМПОНЕНТЫ");
    if app
        .sidebar_button(
            ui,
            "Компоненты",
            matches!(app.view, View::Components),
            item_w,
        )
        .clicked()
    {
        app.view = View::Components;
    }

    app.separator(ui);
    app.section_label(ui, "НАСТРОЙКИ");
    if app
        .sidebar_button(ui, "Настройки", matches!(app.view, View::Settings), item_w)
        .clicked()
    {
        app.view = View::Settings;
    }

    app.separator(ui);
    app.section_label(ui, "ИНФО");

    if app
        .sidebar_button(ui, "О программе", matches!(app.view, View::About), item_w)
        .clicked()
    {
        app.view = View::About;
    }
    if app
        .sidebar_button(ui, "Система", matches!(app.view, View::System), item_w)
        .clicked()
    {
        app.view = View::System;
    }
    if app.sidebar_button(ui, "HolyCheck", false, item_w).clicked() {
        app.show_holy_confirm = true;
    }
}

fn draw_utils_list(app: &mut CubeCheckApp, ui: &mut egui::Ui) {
    let id = ui.make_persistent_id("sidebar_programs_header");
    let mut state =
        egui::collapsing_header::CollapsingState::load_with_default_open(ui.ctx(), id, true);
    let item_w = app.sidebar_item_width(ui);
    let glow = app.glow_for(GlowArea::Sidebar);
    let (rect, response) = ui.allocate_exact_size(egui::vec2(item_w, 28.0), egui::Sense::click());
    let fill = if response.hovered() {
        app.colors.hover
    } else {
        egui::Color32::TRANSPARENT
    };
    ui.painter()
        .rect_filled(rect, egui::Rounding::same(6.0), fill);

    let header = response.on_hover_cursor(egui::CursorIcon::PointingHand);
    if header.clicked() {
        state.toggle(ui);
    }
    state.store(ui.ctx());

    let openness = state.openness(ui.ctx());
    let icon_center = egui::pos2(rect.left() + 12.0, rect.center().y);
    paint_collapse_triangle(ui, openness, icon_center, app.colors.fg);

    let font_id = egui::FontId::proportional(13.0);
    let galley = ui.fonts(|f| f.layout_no_wrap("Программы".to_owned(), font_id, app.colors.fg));
    let galley_pos = egui::pos2(rect.left() + 24.0, rect.center().y - galley.size().y * 0.5);
    glow::paint_at(ui, galley_pos, galley.as_ref(), app.colors.fg, glow);
    glow::paint_frame_glow(ui, rect, 6.0, glow);

    state.show_body_indented(&header, ui, |ui| {
        ui.add_space(4.0);
        for (i, util) in UTILS.iter().enumerate() {
            let selected = matches!(app.view, View::Util(idx) if idx == i);
            let full_w = app.sidebar_item_width(ui);
            if app.sidebar_button(ui, util.name, selected, full_w).clicked() {
                app.view = View::Util(i);
            }
        }
        let full_w = app.sidebar_item_width(ui);
        if app
            .sidebar_button(ui, "Корзина", matches!(app.view, View::Recycle), full_w)
            .clicked()
        {
            app.view = View::Recycle;
        }
        ui.add_space(4.0);
    });
}

fn save_report(app: &mut CubeCheckApp) {
    match crate::backend::save_report(&app.saved_results) {
        Ok(path) => app.set_status(format!("Отчёт сохранён: {path}"), false),
        Err(e) => app.set_status(e, true),
    }
}

