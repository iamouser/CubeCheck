#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod backend;
mod config;
mod content;
mod download;
mod fonts;
mod scan;
mod system;
mod theme;
mod tools;
mod ui;
#[cfg(windows)]
mod win;
#[cfg(windows)]
mod uninstall;

const WINDOW_WIDTH: f32 = 980.0;
const WINDOW_HEIGHT: f32 = 780.0;
const WINDOW_MIN_WIDTH: f32 = 920.0;
const WINDOW_MIN_HEIGHT: f32 = 700.0;

pub(crate) fn app_icon() -> egui::IconData {
    egui::IconData {
        rgba: include_bytes!(concat!(env!("OUT_DIR"), "/icon_rgba.bin")).to_vec(),
        width: env!("ICON_WIDTH").parse().expect("ICON_WIDTH"),
        height: env!("ICON_HEIGHT").parse().expect("ICON_HEIGHT"),
    }
}

fn window_title() -> String {
    format!("CubeCheck {}", crate::content::APP_VERSION)
}

enum LaunchMode {
    App,
    Uninstall { go: bool },
}

fn parse_args() -> LaunchMode {
    let mut uninstall = false;
    let mut go = false;
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "-uninstall" | "--uninstall" | "/uninstall" => uninstall = true,
            "--go" | "-go" => go = true,
            _ => {}
        }
    }
    if uninstall {
        LaunchMode::Uninstall { go }
    } else {
        LaunchMode::App
    }
}

fn main() -> eframe::Result<()> {
    #[cfg(windows)]
    if let LaunchMode::Uninstall { go } = parse_args() {
        return uninstall::run(go);
    }
    #[cfg(not(windows))]
    if matches!(parse_args(), LaunchMode::Uninstall { .. }) {
        eprintln!("cubecheck -uninstall поддерживается только в Windows");
        std::process::exit(1);
    }

    if let Err(e) = backend::init() {
        #[cfg(windows)]
        win::message_box("CubeCheck", &e);
        #[cfg(not(windows))]
        eprintln!("CubeCheck: {e}");
        std::process::exit(1);
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

fn apply_saved_zoom(ctx: &egui::Context, zoom: f32) {
    let zoom = config::clamp_zoom(zoom);
    let native = ctx.native_pixels_per_point().unwrap_or(1.0);
    ctx.set_pixels_per_point(zoom * native);
}
