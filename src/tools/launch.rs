use std::path::{Path, PathBuf};
use std::process::Command;

use super::paths::minecraft_dir;
#[cfg(windows)]
use super::paths::tool_path;

#[cfg(windows)]
use crate::scan::everything_search_query;
#[cfg(windows)]
use crate::win;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

#[cfg(windows)]
fn missing_resource_message(name: &str) -> String {
    if crate::tools::paths::is_offline() {
        format!("{name} не найден в папке assets. Офлайн-сборка не загружает файлы из сети.")
    } else {
        format!("{name} не найден. Скачайте его в разделе «Компоненты».")
    }
}

#[cfg(windows)]
fn resource_dir(path: &Path) -> PathBuf {
    path.parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("."))
}

#[cfg(windows)]
fn spawn_process(exe: &Path, args: &[&str]) -> Result<(), String> {
    let mut cmd = Command::new(exe);
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd.current_dir(resource_dir(exe));
    cmd.args(args);
    cmd.spawn()
        .map_err(|e| format!("Не удалось запустить {}: {e}", exe.display()))?;
    Ok(())
}

#[cfg(windows)]
fn launch_tool(path: PathBuf) -> Result<(), String> {
    launch_tool_params(path, String::new())
}

#[cfg(windows)]
fn launch_tool_params(path: PathBuf, params: String) -> Result<(), String> {
    if !path.exists() {
        return Err(missing_resource_message(
            path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("файл"),
        ));
    }

    std::thread::spawn(move || {
        let dir = resource_dir(&path);
        if win::shell_execute_params(&path, &dir, "open", &params).is_err() {
            let _ = win::shell_execute_params(&path, &dir, "runas", &params);
        }
    });

    Ok(())
}

#[cfg(windows)]
fn run_as_admin(path: &Path) -> Result<(), String> {
    launch_tool(path.to_path_buf())
}

#[cfg(windows)]
pub fn run_everything() -> Result<(), String> {
    let path = tool_path("everything");
    if !path.exists() {
        return Err(missing_resource_message("Everything.exe"));
    }
    spawn_process(&path, &[])
}

#[cfg(windows)]
pub fn run_everything_with_search(terms: &[&str]) -> Result<(), String> {
    let path = tool_path("everything");
    if !path.exists() {
        return Err(missing_resource_message("Everything.exe"));
    }
    let query = everything_search_query(terms);
    spawn_process(&path, &["-search", &query])
}

pub fn run_autocheck_search(terms: &[&str]) -> Result<(), String> {
    #[cfg(windows)]
    {
        run_everything_with_search(terms)
    }
    #[cfg(not(windows))]
    {
        super::posix::run_disk_search(terms)
    }
}

#[cfg(windows)]
pub fn run_shellbag() -> Result<(), String> {
    run_as_admin(&tool_path("shellbag"))
}

#[cfg(windows)]
pub fn run_systeminformer() -> Result<(), String> {
    let path = tool_path("systeminformer");
    if path.exists() && !win::is_pe_amd64(&path) {
        return Err("Нужна 64-битная версия. В «Компонентах» нажмите «Повтор».".into());
    }
    if let Some(dir) = path.parent() {
        crate::download::strip_systeminformer_extras(dir);
        let _ = crate::download::write_systeminformer_settings(dir);
    }
    launch_tool(path)
}

#[cfg(windows)]
pub fn run_procmon() -> Result<(), String> {
    run_as_admin(&tool_path("procmon"))
}

#[cfg(windows)]
pub fn run_autoruns() -> Result<(), String> {
    run_as_admin(&tool_path("autoruns"))
}

#[cfg(windows)]
pub fn run_procexp() -> Result<(), String> {
    run_as_admin(&tool_path("procexp"))
}

pub fn run_system_info() -> Result<(), String> {
    #[cfg(windows)]
    {
        Command::new("msinfo32")
            .spawn()
            .map_err(|e| format!("Не удалось запустить msinfo32: {e}"))?;
        return Ok(());
    }
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .args(["-a", "System Information"])
            .spawn()
            .map_err(|e| format!("Не удалось открыть сведения о системе: {e}"))?;
        return Ok(());
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        for cmd in ["gnome-system-monitor", "ksysguard", "xfce4-taskmanager"] {
            if Command::new(cmd).spawn().is_ok() {
                return Ok(());
            }
        }
        return Err(
            "Не удалось открыть монитор системы. Установите gnome-system-monitor.".into(),
        );
    }
    #[cfg(not(any(windows, unix)))]
    {
        Err("Сведения о системе недоступны на этой ОС.".into())
    }
}

pub fn open_recycle_bin() -> Result<(), String> {
    #[cfg(windows)]
    {
        Command::new("explorer.exe")
            .arg("shell:RecycleBinFolder")
            .spawn()
            .map_err(|e| format!("Не удалось открыть корзину: {e}"))?;
        return Ok(());
    }
    #[cfg(target_os = "macos")]
    {
        let trash = std::env::var("HOME")
            .map(|h| PathBuf::from(h).join(".Trash"))
            .unwrap_or_else(|_| PathBuf::from(".Trash"));
        Command::new("open")
            .arg(&trash)
            .spawn()
            .map_err(|e| format!("Не удалось открыть корзину: {e}"))?;
        return Ok(());
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        if Command::new("gio").args(["open", "trash:///"]).spawn().is_ok() {
            return Ok(());
        }
        if Command::new("xdg-open").arg("trash:///").spawn().is_ok() {
            return Ok(());
        }
        let home = std::env::var("HOME").unwrap_or_default();
        let trash = PathBuf::from(home).join(".local/share/Trash/files");
        if trash.is_dir() && Command::new("xdg-open").arg(&trash).spawn().is_ok() {
            return Ok(());
        }
        return Err("Не удалось открыть корзину (gio/xdg-open trash:///).".into());
    }
    #[cfg(not(any(windows, unix)))]
    {
        Err("Корзина недоступна на этой ОС.".into())
    }
}

#[cfg_attr(windows, allow(dead_code))]
pub fn open_path(path: &Path) -> Result<(), String> {
    #[cfg(windows)]
    {
        Command::new("explorer.exe")
            .arg(path)
            .spawn()
            .map_err(|e| format!("Не удалось открыть {}: {e}", path.display()))?;
        return Ok(());
    }
    #[cfg(not(windows))]
    {
        super::posix::open_path(path)
    }
}

pub fn open_holycheck() {
    let _ = webbrowser::open("https://mods.holyworld.me/");
}

pub fn open_telegram() {
    let _ = webbrowser::open("https://telegram.me/cubecheck");
}

pub fn clear_minecraft_logs() -> Result<(), String> {
    let logs = minecraft_dir().join("logs");

    if !logs.exists() {
        return Err("Папка логов Minecraft не найдена.".into());
    }

    std::fs::remove_dir_all(&logs).map_err(|e| format!("Не удалось очистить логи: {e}"))
}

pub fn run_util_id(key: &str) -> Result<(), String> {
    #[cfg(windows)]
    {
        match key {
            "everything" => run_everything(),
            "shellbag" => run_shellbag(),
            "systeminformer" => run_systeminformer(),
            "procmon" => run_procmon(),
            "autoruns" => run_autoruns(),
            "procexp" => run_procexp(),
            _ => Err("Неизвестная программа".into()),
        }
    }
    #[cfg(not(windows))]
    {
        super::posix::run_util_id(key)
    }
}
