use std::path::{Path, PathBuf};
use std::process::Command;

use super::catalog::InspectRow;

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

fn extra_bin_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(exe) = exe_dir() {
        dirs.push(exe.clone());
        dirs.push(exe.join("assets").join("bin"));
        dirs.push(exe.join("extras").join("bin"));
        if let Some(parent) = exe.parent() {
            dirs.push(parent.join("extras").join("bin"));
            dirs.push(parent.join("assets").join("bin"));
            if let Some(root) = parent.parent() {
                dirs.push(root.join("extras").join("bin"));
                dirs.push(root.join("assets").join("bin"));
            }
        }
    }
    if let Ok(path) = std::env::var("PATH") {
        for dir in path.split(if cfg!(windows) { ';' } else { ':' }) {
            if !dir.is_empty() {
                dirs.push(PathBuf::from(dir));
            }
        }
    }
    dirs
}

pub fn find(names: &[&str]) -> Option<PathBuf> {
    for name in names {
        if name.is_empty() {
            continue;
        }
        if name.contains('/') || name.contains('\\') {
            let p = PathBuf::from(name);
            if p.is_file() {
                return Some(p);
            }
            continue;
        }
        for dir in extra_bin_dirs() {
            let full = dir.join(name);
            if full.is_file() {
                return Some(full);
            }
        }
    }
    None
}

pub fn tool_available_local(id: &str) -> bool {
    match id {
        "search" | "everything" => {
            find(&[
                "fsearch", "catfish", "plocate", "locate", "mdfind", "fd", "rg", "fzf",
            ])
            .is_some()
                || cfg!(target_os = "macos")
        }
        "files" | "shellbag" => {
            cfg!(target_os = "macos")
                || find(&["xdg-open", "nautilus", "dolphin", "thunar", "lf"]).is_some()
        }
        "processes" | "systeminformer" | "procexp" => {
            cfg!(target_os = "macos")
                || find(&[
                    "missioncenter",
                    "gnome-system-monitor",
                    "plasma-systemmonitor",
                    "xfce4-taskmanager",
                    "ksysguard",
                    "btop",
                    "btm",
                    "procs",
                    "htop",
                ])
                .is_some()
        }
        "monitor" | "procmon" => {
            cfg!(target_os = "macos")
                || find(&[
                    "sysdig",
                    "fatrace",
                    "lsof",
                    "busybox",
                    "btop",
                    "btm",
                    "gnome-system-monitor",
                    "journalctl",
                ])
                .is_some()
        }
        "autoruns" => true,
        _ => find(&[id]).is_some(),
    }
}

fn spawn(exe: &Path, args: &[&str]) -> Result<(), String> {
    let dir = exe.parent().unwrap_or_else(|| Path::new("."));
    let mut cmd = Command::new(exe);
    cmd.args(args).current_dir(dir);
    cmd.env("APPIMAGE_EXTRACT_AND_RUN", "1");
    cmd.spawn()
        .map(|_| ())
        .map_err(|e| format!("{}: {e}", exe.display()))
}

fn spawn_named(name: &str, args: &[&str]) -> Result<(), String> {
    if let Some(exe) = find(&[name]) {
        return spawn(&exe, args);
    }
    Command::new(name)
        .args(args)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("{name}: {e}"))
}

fn run_in_terminal(exe: &Path, args: &[&str]) -> Result<(), String> {
    let exe_s = exe.to_string_lossy();
    let line = if args.is_empty() {
        format!("\"{exe_s}\"")
    } else {
        let rest = args
            .iter()
            .map(|a| {
                if a.contains(' ') {
                    format!("\"{a}\"")
                } else {
                    (*a).to_string()
                }
            })
            .collect::<Vec<_>>()
            .join(" ");
        format!("\"{exe_s}\" {rest}")
    };
    if cfg!(target_os = "macos") {
        return spawn_named(
            "osascript",
            &["-e", &format!("tell application \"Terminal\" to do script {line:?}")],
        );
    }
    if let Some(term) = find(&[
        "x-terminal-emulator",
        "gnome-terminal",
        "konsole",
        "xfce4-terminal",
        "xterm",
    ]) {
        return spawn(&term, &["-e", &line]);
    }
    spawn(exe, args)
}

fn open_path(path: &Path) -> Result<(), String> {
    if cfg!(target_os = "macos") {
        return spawn_named("open", &[path.to_str().unwrap_or(".")]);
    }
    if let Some(opener) = find(&["xdg-open"]) {
        return spawn(&opener, &[path.to_str().unwrap_or(".")]);
    }
    if let Some(lf) = find(&["lf"]) {
        return run_in_terminal(&lf, &[path.to_str().unwrap_or(".")]);
    }
    spawn_named("xdg-open", &[path.to_str().unwrap_or(".")])
}

fn home_dir() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}

fn run_search(query: Option<&str>) -> Result<(), String> {
    if cfg!(target_os = "macos") {
        if query.is_some() {
            if find(&["mdfind"]).is_some() {
                return spawn_named("open", &["-a", "Spotlight"]);
            }
        }
        return spawn_named("open", &["-a", "Spotlight"]);
    }
    if let Some(catfish) = find(&["catfish"]) {
        return match query {
            Some(q) => spawn(&catfish, &["--start", q]),
            None => spawn(&catfish, &[]),
        };
    }
    if let Some(fsearch) = find(&["fsearch"]) {
        let meta = std::fs::metadata(&fsearch).ok();
        if meta.is_some_and(|m| m.len() > 8192) {
            return spawn(&fsearch, &[]);
        }
    }
    if let (Some(fzf), Some(fd)) = (find(&["fzf"]), find(&["fd"])) {
        let cmd = format!("\"{}\" --type f | \"{}\"", fd.display(), fzf.display());
        return run_in_terminal(Path::new("/bin/sh"), &["-c", &cmd]);
    }
    if let Some(fd) = find(&["fd"]) {
        return run_in_terminal(&fd, &[query.unwrap_or(".")]);
    }
    if let Some(rg) = find(&["rg"]) {
        return run_in_terminal(&rg, &[query.unwrap_or(".")]);
    }
    Err("В assets/bin нет fd/rg/fzf.".into())
}

fn run_recent_files() -> Result<(), String> {
    if cfg!(target_os = "macos") {
        let recent = home_dir()
            .join("Library")
            .join("Application Support")
            .join("com.apple.sharedfilelist");
        if recent.is_dir() {
            return open_path(&recent);
        }
        return spawn_named("open", &["-a", "Finder"]);
    }
    let recent = home_dir()
        .join(".local")
        .join("share")
        .join("recently-used.xbel");
    let target = if recent.is_file() {
        recent.parent().unwrap_or(recent.as_path()).to_path_buf()
    } else {
        home_dir()
    };
    if let Some(lf) = find(&["lf"]) {
        return run_in_terminal(&lf, &[target.to_str().unwrap_or(".")]);
    }
    open_path(&target)
}

fn run_processes() -> Result<(), String> {
    if cfg!(target_os = "macos") {
        return spawn_named("open", &["-a", "Activity Monitor"]);
    }
    for name in [
        "missioncenter",
        "gnome-system-monitor",
        "plasma-systemmonitor",
        "xfce4-taskmanager",
        "ksysguard",
        "btop",
        "btm",
        "procs",
        "htop",
    ] {
        if let Some(exe) = find(&[name]) {
            if matches!(name, "btop" | "btm" | "procs" | "htop") {
                return run_in_terminal(&exe, &[]);
            }
            return spawn(&exe, &[]);
        }
    }
    Err("В assets/bin нет btop/btm/procs.".into())
}

fn run_monitor() -> Result<(), String> {
    if cfg!(target_os = "macos") {
        return spawn_named(
            "osascript",
            &["-e", "tell application \"Terminal\" to do script \"sudo fs_usage\""],
        );
    }
    if let Some(lsof) = find(&["lsof"]) {
        return run_in_terminal(&lsof, &["-nP"]);
    }
    if let Some(busy) = find(&["busybox"]) {
        return run_in_terminal(&busy, &["lsof"]);
    }
    if let Some(sysdig) = find(&["sysdig"]) {
        return run_in_terminal(&sysdig, &[]);
    }
    run_processes()
}

fn run_autoruns() -> Result<(), String> {
    if cfg!(target_os = "macos") {
        let _ = spawn_named(
            "open",
            &["x-apple.systempreferences:com.apple.LoginItems-Settings.extension"],
        );
        let agents = home_dir().join("Library").join("LaunchAgents");
        if agents.is_dir() {
            return open_path(&agents);
        }
        return Ok(());
    }
    let auto = home_dir().join(".config").join("autostart");
    let _ = std::fs::create_dir_all(&auto);
    if let Some(lf) = find(&["lf"]) {
        return run_in_terminal(&lf, &[auto.to_str().unwrap_or(".")]);
    }
    open_path(&auto)
}

pub fn run_bundled_util(key: &str) -> Result<(), String> {
    match key {
        "search" | "everything" => run_search(None),
        "files" | "shellbag" => run_recent_files(),
        "processes" | "systeminformer" | "procexp" => run_processes(),
        "monitor" | "procmon" => run_monitor(),
        "autoruns" => run_autoruns(),
        _ => Err("Неизвестная программа".into()),
    }
}

pub fn install_hint(id: &str) -> &'static str {
    if env_flag("CUBECHECK_OFFLINE") {
        return "Офлайн-сборка: программы уже в assets/bin.";
    }
    #[cfg(target_os = "macos")]
    {
        let _ = id;
        return "Встроено в macOS.";
    }
    #[cfg(not(target_os = "macos"))]
    {
        match id {
            "search" => "В assets/bin: fd, rg, fzf.",
            "processes" | "procexp" => "В assets/bin: btop, btm, procs, Mission Center.",
            "monitor" | "activity" => "В assets/bin: lsof / busybox.",
            "autoruns" | "files" | "recent" => "Встроено в CubeCheck (папка + lf).",
            _ => "Утилита недоступна.",
        }
    }
}

pub fn run_util_id(key: &str) -> Result<(), String> {
    crate::backend::run_util(key).or_else(|_| run_bundled_util(key))
}

pub fn tool_available(id: &str) -> bool {
    crate::backend::tool_installed(id) || tool_available_local(id)
}

pub fn load_inspect(_id: &str) -> Vec<InspectRow> {
    Vec::new()
}

