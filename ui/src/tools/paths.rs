use std::path::PathBuf;

fn env_flag(name: &str) -> bool {
    match std::env::var(name) {
        Ok(v) => matches!(v.trim(), "1" | "true" | "yes" | "TRUE" | "YES"),
        Err(_) => false,
    }
}

fn exe_dir() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
}

fn offline_marker_present() -> bool {
    let mut dirs = Vec::new();
    if let Some(exe) = exe_dir() {
        dirs.push(exe.clone());
        dirs.push(exe.join("assets"));
        if let Some(parent) = exe.parent() {
            dirs.push(parent.to_path_buf());
            dirs.push(parent.join("assets"));
            if let Some(root) = parent.parent() {
                dirs.push(root.to_path_buf());
            }
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        dirs.push(cwd.clone());
        dirs.push(cwd.join("assets"));
    }
    dirs.iter().any(|d| d.join(".offline").is_file())
}

pub fn is_offline() -> bool {
    crate::backend::is_offline() || env_flag("CUBECHECK_OFFLINE") || offline_marker_present()
}

pub fn tool_installed(id: &str) -> bool {
    if crate::backend::tool_installed(id) {
        return true;
    }
    #[cfg(not(windows))]
    {
        if is_offline() {
            return true;
        }
        return crate::tools::posix::tool_available_local(id);
    }
    #[cfg(windows)]
    {
        let _ = id;
        false
    }
}

#[allow(dead_code)]
pub fn any_tool_missing() -> bool {
    #[cfg(not(windows))]
    {
        if is_offline() {
            return false;
        }
        if crate::backend::api_loaded() {
            return crate::backend::any_tool_missing();
        }
        return crate::tools::UTILS
            .iter()
            .any(|u| !tool_installed(u.id));
    }
    #[cfg(windows)]
    {
        crate::backend::any_tool_missing()
    }
}

pub fn ensure_install_dir() -> Result<(), String> {
    crate::backend::init()
}

