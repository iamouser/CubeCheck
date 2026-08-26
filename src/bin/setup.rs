//! Тонкий установщик: только cubecheck.exe + assets. Утилиты качаются при первом запуске.
//!
//! Всегда ставит файлы в `C:\Program Files\CubeCheck` (даже если копия уже есть),
//! закрывает старый процесс, затем запускает именно установленный exe по полному пути.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;
#[cfg(windows)]
const CREATE_BREAKAWAY_FROM_JOB: u32 = 0x0100_0000;

#[path = "../win.rs"]
mod win;

const INSTALL_DIR: &str = r"C:\Program Files\CubeCheck";
const CUBECHECK_EXE: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/cubecheck_payload.exe"));
const TOOLS_JSON: &[u8] = include_bytes!("../../assets/tools.json");
const ICON: &[u8] = include_bytes!("../../assets/cubecheck.ico");

fn main() {
    if let Err(e) = run() {
        win::message_box("Ошибка установки", &e);
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    ensure_admin()?;

    let dest = PathBuf::from(INSTALL_DIR);
    let cubecheck = dest.join("cubecheck.exe");
    let assets = dest.join("assets");
    fs::create_dir_all(&assets).map_err(|e| format!("Не удалось создать папку установки: {e}"))?;

    // Старый cubecheck.exe держит файл — без закрытия копия молча не обновляется.
    stop_running_cubecheck(&cubecheck);

    install_cubecheck_exe(&cubecheck)?;
    write_file_with_retry(&assets.join("tools.json"), TOOLS_JSON)?;
    write_file_with_retry(&assets.join("cubecheck.ico"), ICON)?;

    let reports = dest.join("reports");
    fs::create_dir_all(&reports).map_err(|e| format!("Не удалось создать папку отчётов: {e}"))?;
    let settings = dest.join("settings.json");
    if !settings.exists() {
        if !copy_legacy_settings(&settings) {
            write_file_with_retry(
                &settings,
                include_bytes!("../../assets/settings.default.json"),
            )?;
        }
    }
    let _ = grant_users_modify(&dest, true);
    let _ = grant_users_modify(&settings, false);
    let _ = grant_users_modify(&reports, true);
    let _ = grant_users_modify(&assets, true);

    let icon = assets.join("cubecheck.ico");
    let icon_ref = icon.exists().then_some(icon.as_path());
    // Ярлыки не должны блокировать запуск новой версии.
    let _ = win::install_shortcuts(&cubecheck, &dest, icon_ref);

    launch_cubecheck(&cubecheck)?;
    Ok(())
}

fn ensure_admin() -> Result<(), String> {
    let running = cubecheck_image_running();
    if can_write_to_install_dir() && !running {
        return Ok(());
    }
    if win::is_elevated() {
        return Ok(());
    }
    win::relaunch_as_admin()?;
    std::process::exit(0);
}

fn cubecheck_image_running() -> bool {
    #[cfg(windows)]
    {
        let mut cmd = Command::new("tasklist");
        cmd.args(["/FI", "IMAGENAME eq cubecheck.exe", "/NH"]);
        cmd.creation_flags(CREATE_NO_WINDOW);
        match cmd.output() {
            Ok(out) => String::from_utf8_lossy(&out.stdout)
                .to_ascii_lowercase()
                .contains("cubecheck.exe"),
            Err(_) => true,
        }
    }
    #[cfg(not(windows))]
    {
        false
    }
}

fn can_write_to_install_dir() -> bool {
    let dest = PathBuf::from(INSTALL_DIR);
    if fs::create_dir_all(&dest).is_err() {
        return false;
    }
    let test = dest.join(".cubecheck_install_test");
    match fs::write(&test, b"ok") {
        Ok(()) => {
            let _ = fs::remove_file(&test);
            true
        }
        Err(_) => false,
    }
}

/// Embedded payload is the version this Setup was built with. A sibling
/// cubecheck.exe in the same folder is often an *older* portable copy
/// (Downloads, Program Files, dist/) — using it was the upgrade bug.
fn install_cubecheck_exe(dest: &Path) -> Result<(), String> {
    if !CUBECHECK_EXE.is_empty() {
        write_file_with_retry(dest, CUBECHECK_EXE)?;
        verify_written(dest, CUBECHECK_EXE.len() as u64)?;
        return Ok(());
    }
    if let Some(sibling) = sibling_cubecheck() {
        if paths_equal(&sibling, dest) {
            return Err(
                "В установщике нет встроенной программы, а cubecheck.exe рядом — это уже папка установки.\n\
                 Соберите установщик через build.bat."
                    .into(),
            );
        }
        copy_file_with_retry(&sibling, dest)?;
        return Ok(());
    }
    Err(
        "Рядом с установщиком нет cubecheck.exe. Положите его в ту же папку или соберите через build.bat."
            .into(),
    )
}

fn verify_written(dest: &Path, expected: u64) -> Result<(), String> {
    let meta = fs::metadata(dest).map_err(|e| {
        format!(
            "После установки не найден {}: {e}",
            dest.display()
        )
    })?;
    if meta.len() != expected {
        return Err(format!(
            "Установленный файл повреждён (размер {} вместо {expected}). Повторите установку.",
            meta.len()
        ));
    }
    Ok(())
}

fn copy_legacy_settings(dest: &Path) -> bool {
    let mut candidates = Vec::new();
    if let Ok(appdata) = std::env::var("APPDATA") {
        let dir = PathBuf::from(appdata).join("CubeCheck");
        candidates.push(dir.join("settings.json"));
        candidates.push(dir.join("config.json"));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join("settings.json"));
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join("settings.json"));
    }
    for src in candidates {
        if src == dest || !src.is_file() {
            continue;
        }
        if fs::copy(&src, dest).is_ok() {
            return true;
        }
    }
    false
}

fn sibling_cubecheck() -> Option<PathBuf> {
    let me = std::env::current_exe().ok()?;
    let dir = me.parent()?;
    let sibling = dir.join("cubecheck.exe");
    sibling.is_file().then_some(sibling)
}

fn paths_equal(a: &Path, b: &Path) -> bool {
    if let (Ok(ca), Ok(cb)) = (fs::canonicalize(a), fs::canonicalize(b)) {
        return ca == cb;
    }
    a.to_string_lossy()
        .eq_ignore_ascii_case(&b.to_string_lossy())
}

fn is_cubecheck_exe(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.eq_ignore_ascii_case("cubecheck.exe"))
}

fn is_lock_error(e: &std::io::Error) -> bool {
    matches!(e.raw_os_error(), Some(5) | Some(32) | Some(33))
}

fn backup_path(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_os_string();
    name.push(".old");
    PathBuf::from(name)
}

/// Delete dest, or rename it aside if the running image is still locked.
fn unlock_dest(dest: &Path) {
    if !dest.exists() {
        return;
    }
    if fs::remove_file(dest).is_ok() {
        return;
    }
    let bak = backup_path(dest);
    let _ = fs::remove_file(&bak);
    let _ = fs::rename(dest, &bak);
}

fn cleanup_backup(dest: &Path) {
    let _ = fs::remove_file(backup_path(dest));
}

fn restore_backup(dest: &Path) {
    let bak = backup_path(dest);
    if bak.exists() && !dest.exists() {
        let _ = fs::rename(&bak, dest);
    }
}

fn copy_file_with_retry(src: &Path, dest: &Path) -> Result<(), String> {
    let name = dest.file_name().and_then(|n| n.to_str()).unwrap_or("файл");
    for attempt in 0..8 {
        if (attempt == 3 || attempt == 6) && is_cubecheck_exe(dest) {
            stop_running_cubecheck(dest);
        }
        unlock_dest(dest);
        match fs::copy(src, dest) {
            Ok(_) => {
                cleanup_backup(dest);
                return Ok(());
            }
            Err(e) if is_lock_error(&e) && attempt < 7 => {
                std::thread::sleep(Duration::from_millis(400));
            }
            Err(e) => {
                restore_backup(dest);
                return Err(format!("Не удалось скопировать {name}: {e}"));
            }
        }
    }
    restore_backup(dest);
    Err(format!(
        "Не удалось скопировать {name}: файл занят.\nЗакройте CubeCheck и повторите установку."
    ))
}

fn write_file_with_retry(out: &Path, data: &[u8]) -> Result<(), String> {
    let name = out.file_name().and_then(|n| n.to_str()).unwrap_or("файл");
    for attempt in 0..8 {
        if (attempt == 3 || attempt == 6) && is_cubecheck_exe(out) {
            stop_running_cubecheck(out);
        }
        unlock_dest(out);
        match fs::write(out, data) {
            Ok(()) => {
                cleanup_backup(out);
                return Ok(());
            }
            Err(e) if is_lock_error(&e) && attempt < 7 => {
                std::thread::sleep(Duration::from_millis(400));
            }
            Err(e) => {
                restore_backup(out);
                return Err(format!("Не удалось записать {name}: {e}"));
            }
        }
    }
    restore_backup(out);
    Err(format!(
        "Не удалось записать {name}: файл занят.\nЗакройте CubeCheck и повторите установку."
    ))
}

/// Close running CubeCheck so the install-dir exe can be replaced.
/// Does not touch CubeCheck-Setup.exe (different image name).
fn stop_running_cubecheck(install_exe: &Path) {
    #[cfg(windows)]
    {
        fn taskkill(force: bool) {
            let mut cmd = Command::new("taskkill");
            cmd.arg("/IM").arg("cubecheck.exe");
            if force {
                cmd.arg("/F");
            }
            cmd.creation_flags(CREATE_NO_WINDOW);
            let _ = cmd.output();
        }
        taskkill(false);
        std::thread::sleep(Duration::from_millis(1200));
        taskkill(true);
        std::thread::sleep(Duration::from_millis(400));
        unlock_dest(install_exe);
    }
    #[cfg(not(windows))]
    {
        let _ = install_exe;
    }
}

fn grant_users_modify(path: &Path, inherit: bool) -> Result<(), String> {
    let mut cmd = Command::new("icacls");
    cmd.arg(path);
    cmd.arg("/grant");
    cmd.arg(if inherit {
        "*S-1-5-32-545:(OI)(CI)M"
    } else {
        "*S-1-5-32-545:M"
    });
    cmd.arg("/C");
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    let status = cmd
        .status()
        .map_err(|e| format!("Не удалось выдать права на {}: {e}", path.display()))?;
    if !status.success() {
        return Err(format!(
            "Не удалось выдать права на запись: {}",
            path.display()
        ));
    }
    Ok(())
}

fn normalize_windows_path(path: PathBuf) -> PathBuf {
    let s = path.to_string_lossy();
    if let Some(rest) = s.strip_prefix(r"\\?\") {
        if !rest.starts_with("UNC\\") && !rest.starts_with("unc\\") {
            return PathBuf::from(rest);
        }
    }
    path
}

fn launch_cubecheck(exe: &Path) -> Result<(), String> {
    if !exe.is_file() {
        return Err(format!(
            "Установка не записала программу: {}",
            exe.display()
        ));
    }
    let abs = normalize_windows_path(fs::canonicalize(exe).unwrap_or_else(|_| exe.to_path_buf()));
    let dir = abs
        .parent()
        .ok_or_else(|| "Не удалось определить папку CubeCheck".to_string())?
        .to_path_buf();

    let mut last_err = String::new();
    for _ in 0..8 {
        match spawn_installed(&abs, &dir) {
            Ok(()) => return Ok(()),
            Err(e) => {
                last_err = e;
                std::thread::sleep(Duration::from_millis(250));
            }
        }
    }
    Err(format!(
        "Установка завершена, но не удалось запустить CubeCheck: {last_err}"
    ))
}

fn spawn_installed(exe: &Path, dir: &Path) -> Result<(), String> {
    let mut cmd = Command::new(exe);
    cmd.current_dir(dir);
    #[cfg(windows)]
    cmd.creation_flags(CREATE_BREAKAWAY_FROM_JOB);
    cmd.spawn()
        .map(|_| ())
        .map_err(|e| format!("{}: {e}", exe.display()))
}
