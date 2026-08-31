use std::path::{Path, PathBuf};
use std::process::Command;

use eframe::egui;

use crate::theme::{ThemeColors, ThemeId};

const GOLD: egui::Color32 = egui::Color32::from_rgb(0xD4, 0xAF, 0x37);
const WINDOW_W: f32 = 480.0;
const WINDOW_H: f32 = 300.0;
const OVERLAY: egui::Color32 = egui::Color32::from_black_alpha(180);

pub fn run(go: bool) -> eframe::Result<()> {
    if go {
        match perform_uninstall() {
            Ok(()) => std::process::exit(0),
            Err(e) => {
                crate::win::message_box("CubeCheck", &e);
                std::process::exit(1);
            }
        }
    }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([WINDOW_W, WINDOW_H])
            .with_min_inner_size([WINDOW_W, WINDOW_H])
            .with_max_inner_size([WINDOW_W, WINDOW_H])
            .with_resizable(false)
            .with_title("CubeCheck — удаление")
            .with_icon(crate::app_icon()),
        centered: true,
        vsync: true,
        ..Default::default()
    };

    eframe::run_native(
        "CubeCheck Uninstall",
        options,
        Box::new(|cc| {
            crate::fonts::setup_fonts(&cc.egui_ctx);
            let colors = ThemeColors::for_theme(ThemeId::Black);
            colors.apply(&cc.egui_ctx);
            Ok(Box::new(UninstallApp {
                colors,
                show_confirm: false,
                error: None,
            }))
        }),
    )
}

struct UninstallApp {
    colors: ThemeColors,
    show_confirm: bool,
    error: Option<String>,
}

impl eframe::App for UninstallApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let colors = self.colors;
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(colors.bg))
            .show(ctx, |ui| {
                draw_header(ui, colors);
                ui.add_space(16.0);
                ui.add_space(4.0);
                egui::Frame::none()
                    .fill(colors.card)
                    .stroke(egui::Stroke::new(1.0, colors.border))
                    .rounding(egui::Rounding::same(8.0))
                    .inner_margin(egui::Margin::same(18.0))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.label(
                            egui::RichText::new("Удалить CubeCheck с этого компьютера?")
                                .size(16.0)
                                .strong()
                                .color(colors.fg),
                        );
                        ui.add_space(8.0);
                        ui.label(
                            egui::RichText::new(
                                "Будут удалены файлы программы, ярлыки, UnInstall.url и папка установки.",
                            )
                            .size(13.0)
                            .color(colors.text_dim),
                        );
                        if let Some(err) = &self.error {
                            ui.add_space(10.0);
                            ui.label(
                                egui::RichText::new(err)
                                    .size(12.0)
                                    .color(egui::Color32::from_rgb(0xE0, 0x80, 0x80)),
                            );
                        }
                        ui.add_space(18.0);
                        ui.horizontal(|ui| {
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.spacing_mut().item_spacing.x = 8.0;
                                    if themed_button(ui, colors, "Удалить", 120.0, true).clicked() {
                                        self.show_confirm = true;
                                    }
                                    if themed_button(ui, colors, "Отмена удаления", 168.0, false)
                                        .clicked()
                                    {
                                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                                    }
                                },
                            );
                        });
                    });
            });

        if self.show_confirm {
            draw_confirm(self, ctx);
        }
    }
}

fn draw_header(ui: &mut egui::Ui, colors: ThemeColors) {
    egui::Frame::none()
        .fill(egui::Color32::BLACK)
        .inner_margin(egui::Margin::symmetric(18.0, 14.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new("CubeCheck")
                            .size(22.0)
                            .strong()
                            .color(colors.fg),
                    );
                    ui.label(
                        egui::RichText::new("Удаление")
                            .size(13.0)
                            .color(colors.text_dim),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                    ui.vertical(|ui| {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                            ui.label(
                                egui::RichText::new(crate::content::APP_VERSION)
                                    .strong()
                                    .color(GOLD),
                            );
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                            ui.label(
                                egui::RichText::new(crate::content::AUTHORS)
                                    .size(12.0)
                                    .color(GOLD),
                            );
                        });
                    });
                });
            });
        });
}

fn draw_confirm(app: &mut UninstallApp, ctx: &egui::Context) {
    let mut confirmed = false;
    let mut cancelled = false;
    let modal_id = egui::Id::new("uninstall_confirm");
    let modal = egui::Modal::new(modal_id)
        .area(egui::Modal::default_area(modal_id).movable(false))
        .backdrop_color(OVERLAY)
        .frame(
            egui::Frame::none()
                .fill(app.colors.card)
                .stroke(egui::Stroke::new(1.0, app.colors.border))
                .rounding(egui::Rounding::same(8.0))
                .inner_margin(egui::Margin::symmetric(22.0, 20.0)),
        )
        .show(ctx, |ui| {
            ui.set_min_width(340.0);
            ui.set_max_width(340.0);
            ui.label(
                egui::RichText::new("Удалить CubeCheck с компьютера?")
                    .size(16.0)
                    .strong()
                    .color(app.colors.fg),
            );
            ui.add_space(8.0);
            ui.label(
                egui::RichText::new("Это действие нельзя отменить.")
                    .size(13.0)
                    .color(app.colors.text_dim),
            );
            ui.add_space(16.0);
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    if themed_button(ui, app.colors, "Да", 100.0, true).clicked() {
                        confirmed = true;
                    }
                    if themed_button(ui, app.colors, "Нет", 100.0, false).clicked() {
                        cancelled = true;
                    }
                });
            });
        });

    if cancelled || modal.should_close() {
        app.show_confirm = false;
    }
    if confirmed {
        app.show_confirm = false;
        match begin_uninstall() {
            Ok(()) => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                std::process::exit(0);
            }
            Err(e) => app.error = Some(e),
        }
    }
}

fn themed_button(
    ui: &mut egui::Ui,
    colors: ThemeColors,
    label: &str,
    width: f32,
    accent: bool,
) -> egui::Response {
    let fill = if accent { colors.accent } else { colors.button_bg };
    let stroke = if accent { colors.accent } else { colors.widget_outline };
    ui.add_sized(
        [width, 34.0],
        egui::Button::new(
            egui::RichText::new(label)
                .size(13.0)
                .strong()
                .color(colors.fg),
        )
        .fill(fill)
        .stroke(egui::Stroke::new(1.0, stroke))
        .rounding(egui::Rounding::same(6.0)),
    )
}

fn begin_uninstall() -> Result<(), String> {
    let dir = install_dir()?;
    if needs_elevation(&dir) {
        crate::win::relaunch_as_admin_args(&["-uninstall", "--go"])?;
        return Ok(());
    }
    perform_uninstall()
}

fn perform_uninstall() -> Result<(), String> {
    let dir = install_dir()?;
    if needs_elevation(&dir) {
        crate::win::relaunch_as_admin_args(&["-uninstall", "--go"])?;
        return Ok(());
    }

    let exe = std::env::current_exe().map_err(|e| format!("Нет пути к cubecheck.exe: {e}"))?;
    stop_other_instances();
    crate::win::remove_app_shortcuts();
    delete_dir_contents(&dir, &exe)?;
    schedule_remove_dir(&dir, std::process::id())?;
    Ok(())
}

fn install_dir() -> Result<PathBuf, String> {
    let exe = std::env::current_exe().map_err(|e| format!("Нет пути к cubecheck.exe: {e}"))?;
    let dir = exe
        .parent()
        .ok_or_else(|| "Нет папки установки.".to_string())?;
    validate_install_dir(dir)
}

fn validate_install_dir(dir: &Path) -> Result<PathBuf, String> {
    let dir_abs = std::fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf());
    if !dir_abs.join("cubecheck.exe").is_file() && !dir.join("cubecheck.exe").is_file() {
        return Err("В этой папке нет cubecheck.exe.".into());
    }
    if is_protected_root(&dir_abs) {
        return Err("Отказ: нельзя удалить системную папку.".into());
    }
    let name = dir_abs
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    let looks = name.eq_ignore_ascii_case("CubeCheck")
        || dir_abs.join("cubecheck_api.dll").is_file()
        || dir_abs.join("assets").join("cubecheck_api.dll").is_file()
        || dir_abs.join("UnInstall.url").is_file()
        || dir_abs.join("UnInstall.cmd").is_file()
        || dir_abs.join("assets").join("UnInstall.cmd").is_file();
    if !looks {
        return Err("Папка не похожа на установку CubeCheck.".into());
    }
    Ok(dir_abs)
}

fn is_protected_root(dir: &Path) -> bool {
    let n = normalize(dir);
    let mut roots: Vec<String> = [
        std::env::var("SystemDrive").ok().map(|d| format!("{d}\\")),
        std::env::var("SystemRoot").ok(),
        std::env::var("windir").ok(),
        std::env::var("ProgramFiles").ok(),
        std::env::var("ProgramFiles(x86)").ok(),
        Some(r"C:\Windows".into()),
        Some(r"C:\Program Files".into()),
        Some(r"C:\Program Files (x86)".into()),
    ]
    .into_iter()
    .flatten()
    .collect();
    if let Some(pf) = std::env::var("ProgramFiles").ok() {
        roots.push(format!("{pf}\\WindowsApps"));
    }
    for r in &roots {
        if n == normalize(Path::new(r)) {
            return true;
        }
        let sys32 = Path::new(r).join("System32");
        if n == normalize(&sys32) {
            return true;
        }
    }
    PathBuf::from(&n).components().count() < 3
}

fn normalize(p: &Path) -> String {
    let s = std::fs::canonicalize(p)
        .map(|x| x.to_string_lossy().into_owned())
        .unwrap_or_else(|_| p.to_string_lossy().into_owned());
    s.trim_start_matches(r"\\?\")
        .trim_end_matches(['\\', '/'])
        .to_ascii_lowercase()
}

fn display_path(p: &Path) -> String {
    let s = p.to_string_lossy();
    s.strip_prefix(r"\\?\").unwrap_or(&s).to_string()
}

fn needs_elevation(dir: &Path) -> bool {
    if crate::win::is_elevated() {
        return false;
    }
    let probe = dir.join(".cubecheck_uninstall_write_test");
    match std::fs::write(&probe, b"ok") {
        Ok(()) => {
            let _ = std::fs::remove_file(&probe);
            false
        }
        Err(_) => true,
    }
}

fn stop_other_instances() {
    let pid = std::process::id();
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let script = format!(
            "Get-Process cubecheck,cubecheck-launcher -ErrorAction SilentlyContinue | Where-Object {{ $_.Id -ne {pid} }} | Stop-Process -Force"
        );
        let _ = Command::new("powershell")
            .args(["-NoProfile", "-Command", &script])
            .creation_flags(0x0800_0000)
            .status();
    }
    #[cfg(not(windows))]
    {
        let _ = pid;
    }
}

fn delete_dir_contents(dir: &Path, keep: &Path) -> Result<(), String> {
    let keep_n = normalize(keep);
    let dir_n = normalize(dir);
    if !keep_n.starts_with(&dir_n) {
        return Err("Исполняемый файл не из этой папки.".into());
    }
    let rd = std::fs::read_dir(dir).map_err(|e| format!("Не удалось прочитать папку: {e}"))?;
    for ent in rd.flatten() {
        let p = ent.path();
        if normalize(&p) == keep_n {
            continue;
        }
        if p.is_dir() {
            let _ = delete_dir_contents(&p, keep);
            let _ = std::fs::remove_dir_all(&p);
        } else {
            let _ = std::fs::remove_file(&p);
        }
    }
    Ok(())
}

fn schedule_remove_dir(dir: &Path, pid: u32) -> Result<(), String> {
    let dir_s = display_path(dir);
    if dir_s.contains('"') || dir_s.contains(['\r', '\n', '&', '|', '>', '<']) {
        return Err("Некорректный путь установки.".into());
    }
    let script = format!(
        "@echo off\r\n\
         :wait\r\n\
         tasklist /FI \"PID eq {pid}\" | findstr /I \"{pid}\" >nul\r\n\
         if not errorlevel 1 (\r\n\
         ping -n 2 127.0.0.1 >nul\r\n\
         goto wait\r\n\
         )\r\n\
         for /L %%i in (1,1,20) do (\r\n\
         rmdir /s /q \"{dir_s}\"\r\n\
         if not exist \"{dir_s}\" goto done\r\n\
         ping -n 2 127.0.0.1 >nul\r\n\
         )\r\n\
         :done\r\n\
         del \"%~f0\"\r\n"
    );
    let tmp = std::env::temp_dir().join(format!("cubecheck-uninst-{pid}.cmd"));
    std::fs::write(&tmp, script)
        .map_err(|e| format!("Не удалось записать скрипт удаления: {e}"))?;

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        Command::new("cmd")
            .arg("/C")
            .arg(&tmp)
            .creation_flags(0x0800_0000)
            .spawn()
            .map_err(|e| format!("Не удалось запустить очистку: {e}"))?;
    }
    #[cfg(not(windows))]
    {
        let _ = tmp;
    }
    Ok(())
}
