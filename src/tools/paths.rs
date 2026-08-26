use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

#[cfg(windows)]
use std::os::windows::process::CommandExt;
#[cfg(windows)]
use std::process::Command;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub const INSTALL_DIR: &str = r"C:\Program Files\CubeCheck";

static DATA_DIR: OnceLock<PathBuf> = OnceLock::new();

pub fn exe_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn env_flag(name: &str) -> bool {
    match std::env::var(name) {
        Ok(v) => {
            let v = v.trim();
            v == "1" || v.eq_ignore_ascii_case("true") || v.eq_ignore_ascii_case("yes")
        }
        Err(_) => false,
    }
}

/// Offline SKU: never download third-party tools.
/// Compile-time `offline` feature, env `CUBECHECK_OFFLINE`, or a `.offline` marker next to the exe.
pub fn is_offline() -> bool {
    if cfg!(feature = "offline") {
        return true;
    }
    env_flag("CUBECHECK_OFFLINE")
        || exe_dir().join(".offline").is_file()
        || exe_dir().join("assets").join(".offline").is_file()
}

/// Portable layout: settings/assets/reports next to the exe (universal bundle, offline pack, Linux/macOS).
pub fn is_portable() -> bool {
    if is_offline() || env_flag("CUBECHECK_PORTABLE") {
        return true;
    }
    if cfg!(not(windows)) {
        return true;
    }
    exe_dir().join(".portable").is_file() || exe_dir().join("portable.txt").is_file()
}

pub fn forensic_tools_supported() -> bool {
    cfg!(windows)
}

pub fn data_dir() -> PathBuf {
    if is_portable() {
        return exe_dir();
    }
    DATA_DIR.get().cloned().unwrap_or_else(|| {
        let install = PathBuf::from(INSTALL_DIR);
        if install.is_dir() {
            install
        } else {
            exe_dir()
        }
    })
}

pub fn ensure_install_dir() -> Result<(), String> {
    if is_portable() {
        return ensure_portable_layout();
    }

    #[cfg(not(windows))]
    {
        return ensure_portable_layout();
    }

    #[cfg(windows)]
    {
        let install = PathBuf::from(INSTALL_DIR);
        if try_create_install(&install).is_ok() {
            let _ = DATA_DIR.set(install);
            return Ok(());
        }
        if !crate::win::is_elevated() {
            crate::win::relaunch_as_admin()
                .map_err(|_| "Нужны права администратора, чтобы создать папку установки".to_string())?;
            std::process::exit(0);
        }
        try_create_install(&install)?;
        let _ = DATA_DIR.set(install);
        Ok(())
    }
}

fn ensure_portable_layout() -> Result<(), String> {
    let install = exe_dir();
    fs::create_dir_all(install.join("assets"))
        .map_err(|e| format!("Не удалось создать папку assets: {e}"))?;
    fs::create_dir_all(install.join("reports"))
        .map_err(|e| format!("Не удалось создать папку отчётов: {e}"))?;
    let settings = install.join("settings.json");
    if !settings.exists() {
        let _ = fs::write(
            &settings,
            include_str!("../../assets/settings.default.json"),
        );
    }
    let src_assets = install.join("assets");
    let bundled = exe_dir().join("assets");
    for name in ["tools.json", "cubecheck.ico"] {
        let dest = src_assets.join(name);
        if !dest.exists() {
            let from = bundled.join(name);
            if from.exists() {
                let _ = fs::copy(from, dest);
            }
        }
    }
    let _ = DATA_DIR.set(install);
    Ok(())
}

fn try_create_install(install: &Path) -> Result<(), String> {
    let assets = install.join("assets");
    let reports = install.join("reports");
    fs::create_dir_all(&assets).map_err(|e| format!("Не удалось создать папку установки: {e}"))?;
    fs::create_dir_all(&reports).map_err(|e| format!("Не удалось создать папку отчётов: {e}"))?;

    let settings = install.join("settings.json");
    migrate_legacy_settings();
    if !settings.exists() && !legacy_settings_exists() {
        let _ = fs::write(
            &settings,
            include_str!("../../assets/settings.default.json"),
        );
    }

    if let Ok(exe) = std::env::current_exe() {
        let dest_exe = install.join("cubecheck.exe");
        if installed_exe_needs_refresh(&exe, &dest_exe) {
            let _ = fs::copy(&exe, dest_exe);
        }
    }
    let src_assets = exe_dir().join("assets");
    for name in ["tools.json", "cubecheck.ico"] {
        let dest = assets.join(name);
        if !dest.exists() {
            let from = src_assets.join(name);
            if from.exists() {
                let _ = fs::copy(from, dest);
            }
        }
    }

    let _ = grant_users_modify(install, true);
    let _ = grant_users_modify(&assets, true);
    let _ = grant_users_modify(&reports, true);
    let _ = grant_users_modify(&settings, false);
    Ok(())
}

fn same_exe(a: &Path, b: &Path) -> bool {
    if a == b {
        return true;
    }
    match (fs::canonicalize(a), fs::canonicalize(b)) {
        (Ok(ca), Ok(cb)) => ca == cb,
        _ => false,
    }
}

/// Copy into Program Files only when the installed exe is missing or older.
fn installed_exe_needs_refresh(src: &Path, dest: &Path) -> bool {
    if same_exe(src, dest) {
        return false;
    }
    let dest_meta = match fs::metadata(dest) {
        Ok(meta) => meta,
        Err(_) => return true,
    };
    let src_meta = match fs::metadata(src) {
        Ok(meta) => meta,
        Err(_) => return false,
    };
    match (src_meta.modified(), dest_meta.modified()) {
        (Ok(src_t), Ok(dest_t)) => src_t > dest_t,
        _ => false,
    }
}

fn grant_users_modify(path: &Path, inherit: bool) -> Result<(), String> {
    #[cfg(not(windows))]
    {
        let _ = (path, inherit);
        return Ok(());
    }

    #[cfg(windows)]
    {
        let mut cmd = Command::new("icacls");
        cmd.arg(path);
        cmd.arg("/grant");
        cmd.arg(if inherit {
            "*S-1-5-32-545:(OI)(CI)M"
        } else {
            "*S-1-5-32-545:M"
        });
        cmd.arg("/C");
        cmd.creation_flags(CREATE_NO_WINDOW);
        let status = cmd
            .status()
            .map_err(|e| format!("Не удалось выдать права: {e}"))?;
        if status.success() {
            Ok(())
        } else {
            Err("Не удалось выдать права на папку".into())
        }
    }
}

pub fn settings_path() -> PathBuf {
    data_dir().join("settings.json")
}

/// Locations used before settings lived only in Program Files.
/// After a successful migrate these paths are never read again.
pub fn legacy_settings_paths() -> Vec<PathBuf> {
    let dest = settings_path();
    let mut paths = Vec::new();
    let mut push = |path: PathBuf| {
        if path != dest && !paths.iter().any(|existing| existing == &path) {
            paths.push(path);
        }
    };
    push(exe_dir().join("settings.json"));
    if let Ok(cwd) = std::env::current_dir() {
        push(cwd.join("settings.json"));
    }
    if let Ok(appdata) = std::env::var("APPDATA") {
        let dir = PathBuf::from(appdata).join("CubeCheck");
        push(dir.join("settings.json"));
        push(dir.join("config.json"));
    }
    paths
}

fn legacy_settings_exists() -> bool {
    legacy_settings_paths().iter().any(|p| p.is_file())
}

/// Copy the first old settings file into Program Files when the new file is missing.
pub fn migrate_legacy_settings() {
    let dest = settings_path();
    if dest.exists() {
        return;
    }
    if let Some(parent) = dest.parent() {
        let _ = fs::create_dir_all(parent);
    }
    for src in legacy_settings_paths() {
        if src.is_file() && fs::copy(&src, &dest).is_ok() {
            return;
        }
    }
}

pub fn reports_dir() -> PathBuf {
    data_dir().join("reports")
}

pub fn assets_dir() -> PathBuf {
    data_dir().join("assets")
}

fn resource_lookup_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    let mut push = |path: PathBuf| {
        if !dirs.iter().any(|existing| existing == &path) {
            dirs.push(path);
        }
    };
    push(assets_dir());
    push(exe_dir().join("assets"));
    if !is_portable() {
        push(PathBuf::from(INSTALL_DIR).join("assets"));
    }
    dirs
}

pub fn resource_path(name: &str) -> PathBuf {
    for dir in resource_lookup_dirs() {
        let path = dir.join(name);
        if path.exists() {
            return path;
        }
    }
    assets_dir().join(name)
}

pub fn tool_path(id: &str) -> PathBuf {
    let dir = assets_dir();
    match id {
        "everything" => dir.join("Everything.exe"),
        "shellbag" => dir.join("Shellbag.exe"),
        "systeminformer" => dir.join("SystemInformer").join("SystemInformer.exe"),
        "procmon" => dir.join("Procmon64.exe"),
        "autoruns" => dir.join("Autoruns64.exe"),
        "procexp" => dir.join("procexp64.exe"),
        other => dir.join(other),
    }
}

pub fn tool_installed(id: &str) -> bool {
    #[cfg(windows)]
    {
        let path = tool_path(id);
        if !path.exists() {
            return false;
        }
        if id == "systeminformer" {
            return crate::win::is_pe_amd64(&path);
        }
        true
    }
    #[cfg(not(windows))]
    {
        crate::tools::posix::tool_available(id)
    }
}

pub fn any_tool_missing() -> bool {
    crate::tools::UTILS.iter().any(|u| !tool_installed(u.id))
}

pub fn minecraft_dir() -> PathBuf {
    if let Ok(appdata) = std::env::var("APPDATA") {
        if !appdata.is_empty() {
            return PathBuf::from(appdata).join(".minecraft");
        }
    }
    let home = std::env::var("HOME").unwrap_or_default();
    if cfg!(target_os = "macos") {
        PathBuf::from(home).join("Library/Application Support/minecraft")
    } else {
        PathBuf::from(home).join(".minecraft")
    }
}
