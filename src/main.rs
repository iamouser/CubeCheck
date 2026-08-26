#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod content;
mod download;
mod fonts;
mod scan;
mod system;
mod theme;
mod tools;
mod ui;
mod win;

const WINDOW_WIDTH: f32 = 980.0;
const WINDOW_HEIGHT: f32 = 780.0;
const WINDOW_MIN_WIDTH: f32 = 920.0;
const WINDOW_MIN_HEIGHT: f32 = 700.0;

fn app_icon() -> egui::IconData {
    egui::IconData {
        rgba: include_bytes!(concat!(env!("OUT_DIR"), "/icon_rgba.bin")).to_vec(),
        width: env!("ICON_WIDTH").parse().expect("ICON_WIDTH"),
        height: env!("ICON_HEIGHT").parse().expect("ICON_HEIGHT"),
    }
}

fn main() -> eframe::Result<()> {
    if let Err(e) = tools::paths::ensure_install_dir() {
        win::message_box("CubeCheck", &e);
    }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([WINDOW_WIDTH, WINDOW_HEIGHT])
            .with_min_inner_size([WINDOW_MIN_WIDTH, WINDOW_MIN_HEIGHT])
            .with_title(window_title())
            .with_icon(app_icon()),
        centered: true,
        vsync: true,
        ..Default::default()
    };

    eframe::run_native(
        "CubeCheck",
        options,
        Box::new(|cc| {
            let config = config::AppConfig::load();
            apply_saved_zoom(&cc.egui_ctx, config.zoom);
            Ok(Box::new(ui::CubeCheckApp::new(cc)))
        }),
    )
}

fn window_title() -> &'static str {
    "CubeCheck"
}

fn apply_saved_zoom(ctx: &egui::Context, zoom: f32) {
    let zoom = config::clamp_zoom(zoom);
    let native = ctx.native_pixels_per_point().unwrap_or(1.0);
    ctx.set_pixels_per_point(zoom * native);
}
