//! Linux / macOS analogs of the Windows checker tools.
//! Does not launch PE search/process utilities from the Windows catalog.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use super::catalog::InspectRow;
#[cfg(target_os = "macos")]
use crate::scan::{mdfind_or_query, posix_regex_or_query};
#[cfg(all(unix, not(target_os = "macos")))]
use crate::scan::{fsearch_or_query, posix_regex_or_query};

pub fn run_util_id(key: &str) -> Result<(), String> {
    match key {
        "search" => run_search_gui(),
        "recent" => Ok(()),
        "processes" => run_process_viewer(),
        "activity" => run_file_activity(),
        "autoruns" => open_autoruns_settings(),
        "procexp" => run_process_tree(),
        _ => Err("Неизвестная программа".into()),
    }
}

pub fn tool_available(id: &str) -> bool {
    match id {
        "search" => search_backend_available(),
        "recent" | "autoruns" => true,
        "processes" => process_viewer_available(),
        "activity" => activity_tool_available(),
        "procexp" => process_tree_available(),
        _ => false,
    }
}

pub fn install_hint(id: &str) -> &'static str {
    #[cfg(target_os = "macos")]
    {
        let _ = id;
        return "Встроено в macOS.";
    }
    #[cfg(not(target_os = "macos"))]
    {
        match id {
            "search" => {
                "Установите: sudo apt install fsearch catfish plocate\nили: sudo dnf install fsearch catfish plocate\nили: sudo pacman -S fsearch catfish plocate"
            }
            "processes" => {
                "Установите Mission Center (flatpak install flathub io.missioncenter.MissionCenter)\nили: sudo apt install gnome-system-monitor"
            }
            "activity" => "Установите sysdig: sudo apt install sysdig  (запасной вариант: lsof).",
            "procexp" => "Установите: sudo apt install gnome-system-monitor htop",
            "autoruns" | "recent" => "Встроено в CubeCheck.",
            _ => "Утилита недоступна.",
        }
    }
}

pub fn load_inspect(id: &str) -> Vec<InspectRow> {
    match id {
        "recent" => recent_entries(),
        "autoruns" => autostart_entries(),
        _ => Vec::new(),
    }
}

pub fn run_disk_search(terms: &[&str]) -> Result<(), String> {
    if terms.is_empty() {
        return run_search_gui();
    }

    let regex_q = posix_regex_or_query(terms);
    copy_search_query(terms);

    #[cfg(target_os = "macos")]
    {
        let _ = regex_q;
        let _ = run_search_gui();
        return run_mdfind_query(terms);
    }

    #[cfg(not(target_os = "macos"))]
    {
        let gui = spawn_if_exists("fsearch", &[]) || match std::env::var("HOME") {
            Ok(h) => spawn_if_exists("catfish", &[h.as_str()]),
            Err(_) => spawn_if_exists("catfish", &[]),
        };
        match run_locate_query(&regex_q) {
            Ok(()) => Ok(()),
            Err(_) if gui => Ok(()),
            Err(e) => Err(e),
        }
    }
}

fn run_search_gui() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        if Command::new("open").args(["-a", "Finder"]).spawn().is_ok() {
            return Ok(());
        }
        return Err("Не удалось открыть Finder / Spotlight.".into());
    }

    #[cfg(not(target_os = "macos"))]
    {
        if spawn_if_exists("fsearch", &[]) {
            return Ok(());
        }
        if let Ok(home) = std::env::var("HOME") {
            if spawn_if_exists("catfish", &[home.as_str()]) {
                return Ok(());
            }
        } else if spawn_if_exists("catfish", &[]) {
            return Ok(());
        }
        if command_on_path("plocate") || command_on_path("locate") {
            return Err(
                "FSearch/Catfish не установлены. Автопроверка использует plocate; GUI: sudo apt install fsearch"
                    .into(),
            );
        }
        Err(install_hint("search").into())
    }
}

fn run_process_viewer() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        return open_mac_app("Activity Monitor");
    }
    #[cfg(not(target_os = "macos"))]
    {
        if spawn_if_exists("missioncenter", &[]) {
            return Ok(());
        }
        if spawn_flatpak("io.missioncenter.MissionCenter") {
            return Ok(());
        }
        for cmd in [
            "gnome-system-monitor",
            "plasma-systemmonitor",
            "ksysguard",
            "xfce4-taskmanager",
            "mate-system-monitor",
        ] {
            if spawn_if_exists(cmd, &[]) {
                return Ok(());
            }
        }
        if command_on_path("htop") {
            return spawn_in_terminal("htop");
        }
        Err(install_hint("processes").into())
    }
}

fn run_process_tree() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        return open_mac_app("Activity Monitor");
    }
    #[cfg(not(target_os = "macos"))]
    {
        for cmd in [
            "gnome-system-monitor",
            "plasma-systemmonitor",
            "ksysguard",
            "xfce4-taskmanager",
            "mate-system-monitor",
            "missioncenter",
        ] {
            if spawn_if_exists(cmd, &[]) {
                return Ok(());
            }
        }
        if spawn_flatpak("io.missioncenter.MissionCenter") {
            return Ok(());
        }
        if command_on_path("htop") {
            return spawn_in_terminal("htop");
        }
        Err(install_hint("procexp").into())
    }
}

fn run_file_activity() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        if spawn_in_terminal("sudo fs_usage -w -f filesys").is_ok() {
            return Ok(());
        }
        if Command::new("open").args(["-a", "Console"]).spawn().is_ok() {
            return Ok(());
        }
        return Err("Не удалось запустить fs_usage или Console.app.".into());
    }
    #[cfg(not(target_os = "macos"))]
    {
        if command_on_path("csysdig") {
            if spawn_if_exists("csysdig", &[]) {
                return Ok(());
            }
            if spawn_in_terminal("csysdig").is_ok() {
                return Ok(());
            }
        }
        if command_on_path("sysdig") {
            return spawn_in_terminal("sudo sysdig");
        }
        if command_on_path("lsof") {
            return spawn_in_terminal("lsof -r 2");
        }
        Err(install_hint("activity").into())
    }
}

fn open_autoruns_settings() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let url = "x-apple.systempreferences:com.apple.LoginItems-Settings.extension";
        if Command::new("open").arg(url).spawn().is_ok() {
            return Ok(());
        }
        if Command::new("open")
            .args(["-b", "com.apple.systempreferences"])
            .spawn()
            .is_ok()
        {
            return Ok(());
        }
        return Ok(());
    }
    #[cfg(not(target_os = "macos"))]
    {
        if spawn_if_exists("gnome-tweaks", &[]) {
            return Ok(());
        }
        if spawn_if_exists("gnome-session-properties", &[]) {
            return Ok(());
        }
        if let Ok(home) = std::env::var("HOME") {
            let dir = PathBuf::from(home).join(".config/autostart");
            let _ = fs::create_dir_all(&dir);
            if spawn_if_exists("xdg-open", &[dir.to_str().unwrap_or(".")]) {
                return Ok(());
            }
        }
        Ok(())
    }
}

pub fn autostart_entries() -> Vec<InspectRow> {
    let mut rows = Vec::new();

    #[cfg(target_os = "macos")]
    {
        rows.extend(macos_login_items());
        rows.extend(macos_launch_items());
    }

    #[cfg(not(target_os = "macos"))]
    {
        if let Ok(home) = std::env::var("HOME") {
            rows.extend(desktop_autostart(
                Path::new(&home).join(".config/autostart"),
                "пользователь",
            ));
        }
        rows.extend(desktop_autostart(
            PathBuf::from("/etc/xdg/autostart"),
            "система",
        ));
        rows.extend(systemd_enabled("--user", "systemd --user"));
        rows.extend(systemd_enabled("", "systemd"));
    }

    rows
}

pub fn recent_entries() -> Vec<InspectRow> {
    #[cfg(target_os = "macos")]
    {
        macos_recent_files()
    }
    #[cfg(not(target_os = "macos"))]
    {
        linux_recent_files()
    }
}

fn search_backend_available() -> bool {
    #[cfg(target_os = "macos")]
    {
        command_on_path("mdfind")
    }
    #[cfg(not(target_os = "macos"))]
    {
        ["fsearch", "catfish"]
            .iter()
            .any(|c| command_on_path(c))
    }
}

fn process_viewer_available() -> bool {
    #[cfg(target_os = "macos")]
    {
        true
    }
    #[cfg(not(target_os = "macos"))]
    {
        ["missioncenter", "gnome-system-monitor", "plasma-systemmonitor", "ksysguard", "htop"]
            .iter()
            .any(|c| command_on_path(c))
            || flatpak_app_exists("io.missioncenter.MissionCenter")
    }
}

fn process_tree_available() -> bool {
    process_viewer_available()
}

fn activity_tool_available() -> bool {
    #[cfg(target_os = "macos")]
    {
        true
    }
    #[cfg(not(target_os = "macos"))]
    {
        ["csysdig", "sysdig", "lsof"].iter().any(|c| command_on_path(c))
    }
}

fn copy_search_query(terms: &[&str]) {
    #[cfg(target_os = "macos")]
    let text = mdfind_or_query(terms);
    #[cfg(not(target_os = "macos"))]
    let text = fsearch_or_query(terms);
    let _ = copy_clipboard(&text);
}

fn copy_clipboard(text: &str) -> bool {
    #[cfg(target_os = "macos")]
    {
        return pipe_to("pbcopy", &[], text);
    }
    #[cfg(not(target_os = "macos"))]
    {
        if command_on_path("wl-copy") && pipe_to("wl-copy", &[], text) {
            return true;
        }
        if command_on_path("xclip") && pipe_to("xclip", &["-selection", "clipboard"], text) {
            return true;
        }
        if command_on_path("xsel") && pipe_to("xsel", &["--clipboard", "--input"], text) {
            return true;
        }
        false
    }
}

fn pipe_to(cmd: &str, args: &[&str], text: &str) -> bool {
    let Ok(mut child) = Command::new(cmd)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    else {
        return false;
    };
    if let Some(stdin) = child.stdin.as_mut() {
        use std::io::Write;
        let _ = stdin.write_all(text.as_bytes());
    }
    child.wait().map(|s| s.success()).unwrap_or(false)
}

#[cfg(not(target_os = "macos"))]
fn run_locate_query(regex_q: &str) -> Result<(), String> {
    let quoted = sh_single_quote(regex_q);
    if command_on_path("plocate") {
        return spawn_in_terminal(&format!(
            "plocate -i -r {quoted} 2>/dev/null | head -n 400; echo; echo '--- конец plocate ---'"
        ));
    }
    if command_on_path("locate") {
        return spawn_in_terminal(&format!(
            "locate -i -r {quoted} 2>/dev/null | head -n 400; echo; echo '--- конец locate ---'"
        ));
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    let home_q = sh_single_quote(&home);
    if command_on_path("find") {
        let pattern = format!(".*({regex_q}).*");
        let pattern_q = sh_single_quote(&pattern);
        return spawn_in_terminal(&format!(
            "find {home_q} /tmp -regextype posix-egrep -iregex {pattern_q} 2>/dev/null | head -n 400; echo; echo '--- конец find ---'"
        ));
    }
    Err(install_hint("search").into())
}

#[cfg(target_os = "macos")]
fn run_mdfind_query(terms: &[&str]) -> Result<(), String> {
    let query = mdfind_or_query(terms);
    let quoted = sh_single_quote(&query);
    spawn_in_terminal(&format!(
        "mdfind {quoted} | head -n 400; echo; echo '--- конец mdfind ---'"
    ))
}

#[cfg(not(target_os = "macos"))]
fn desktop_autostart(dir: PathBuf, scope: &str) -> Vec<InspectRow> {
    let mut rows = Vec::new();
    let Ok(entries) = fs::read_dir(&dir) else {
        return rows;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("desktop") {
            continue;
        }
        if let Some(mut row) = parse_desktop(&path) {
            row.detail = format!("{scope}: {}", row.detail);
            rows.push(row);
        }
    }
    rows
}

#[cfg(not(target_os = "macos"))]
fn parse_desktop(path: &Path) -> Option<InspectRow> {
    let text = fs::read_to_string(path).ok()?;
    let mut name = None;
    let mut exec = None;
    let mut hidden = false;
    for line in text.lines() {
        let line = line.trim();
        if let Some(v) = line.strip_prefix("Name=") {
            if name.is_none() && !v.is_empty() {
                name = Some(v.to_string());
            }
        } else if let Some(v) = line.strip_prefix("Exec=") {
            exec = Some(v.to_string());
        } else if line.eq_ignore_ascii_case("Hidden=true")
            || line.eq_ignore_ascii_case("NoDisplay=true")
        {
            hidden = true;
        }
    }
    if hidden {
        return None;
    }
    Some(InspectRow {
        title: name.unwrap_or_else(|| {
            path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("autostart")
                .to_string()
        }),
        detail: exec.unwrap_or_default(),
        path: Some(path.to_path_buf()),
    })
}

#[cfg(not(target_os = "macos"))]
fn systemd_enabled(user_flag: &str, label: &str) -> Vec<InspectRow> {
    let mut cmd = Command::new("systemctl");
    if !user_flag.is_empty() {
        cmd.arg(user_flag);
    }
    cmd.args([
        "list-unit-files",
        "--state=enabled",
        "--no-pager",
        "--no-legend",
        "--plain",
    ]);
    cmd.stdin(Stdio::null());
    cmd.stderr(Stdio::null());
    let Ok(out) = cmd.output() else {
        return Vec::new();
    };
    if !out.status.success() {
        return Vec::new();
    }
    let mut rows = Vec::new();
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        let name = line.split_whitespace().next().unwrap_or("");
        if name.is_empty() {
            continue;
        }
        if name.ends_with(".mount")
            || name.ends_with(".swap")
            || name.ends_with(".device")
            || name.ends_with(".slice")
            || name.ends_with(".scope")
        {
            continue;
        }
        rows.push(InspectRow {
            title: name.to_string(),
            detail: format!("{label}: enabled"),
            path: None,
        });
        if rows.len() >= 250 {
            break;
        }
    }
    rows
}

#[cfg(not(target_os = "macos"))]
fn linux_recent_files() -> Vec<InspectRow> {
    let home = std::env::var("HOME").unwrap_or_default();
    let path = PathBuf::from(&home).join(".local/share/recently-used.xbel");
    let Ok(text) = fs::read_to_string(&path) else {
        return Vec::new();
    };
    let mut rows = parse_xbel(&text);
    rows.reverse();
    rows.truncate(80);
    rows
}

#[cfg(not(target_os = "macos"))]
fn parse_xbel(text: &str) -> Vec<InspectRow> {
    let mut rows = Vec::new();
    let mut rest = text;
    while let Some(idx) = rest.find("href=\"") {
        rest = &rest[idx + 6..];
        let Some(end) = rest.find('"') else {
            break;
        };
        let href = &rest[..end];
        rest = &rest[end + 1..];
        if let Some(path) = file_url_to_path(href) {
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("файл")
                .to_string();
            rows.push(InspectRow {
                title: name,
                detail: path.display().to_string(),
                path: Some(path),
            });
        }
    }
    rows
}

#[cfg(not(target_os = "macos"))]
fn file_url_to_path(href: &str) -> Option<PathBuf> {
    let rest = href.strip_prefix("file://")?;
    let rest = rest.strip_prefix("localhost").unwrap_or(rest);
    Some(PathBuf::from(percent_decode(rest)))
}

#[cfg(not(target_os = "macos"))]
fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(target_os = "macos")]
fn macos_login_items() -> Vec<InspectRow> {
    let out = Command::new("osascript")
        .args([
            "-e",
            "tell application \"System Events\" to get the name of every login item",
        ])
        .output();
    let Ok(out) = out else {
        return Vec::new();
    };
    if !out.status.success() {
        return Vec::new();
    }
    String::from_utf8_lossy(&out.stdout)
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|name| InspectRow {
            title: name.to_string(),
            detail: "Login Item".into(),
            path: None,
        })
        .collect()
}

#[cfg(target_os = "macos")]
fn macos_launch_items() -> Vec<InspectRow> {
    let home = std::env::var("HOME").unwrap_or_default();
    let dirs = [
        PathBuf::from(&home).join("Library/LaunchAgents"),
        PathBuf::from(&home).join("Library/LaunchDaemons"),
        PathBuf::from("/Library/LaunchAgents"),
        PathBuf::from("/Library/LaunchDaemons"),
        PathBuf::from("/System/Library/LaunchAgents"),
        PathBuf::from("/System/Library/LaunchDaemons"),
    ];
    let mut rows = Vec::new();
    for (i, dir) in dirs.iter().enumerate() {
        // Skip huge system Launch* dumps except as last resort; still list user + /Library.
        if i >= 4 {
            continue;
        }
        let Ok(entries) = fs::read_dir(dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("plist") {
                continue;
            }
            let name = path
                .file_stem()
                .and_then(|n| n.to_str())
                .unwrap_or("plist")
                .to_string();
            let kind = if name.starts_with("com.apple.") {
                "Apple"
            } else {
                "сторонний"
            };
            rows.push(InspectRow {
                title: name,
                detail: format!("{kind}: {}", dir.display()),
                path: Some(path),
            });
            if rows.len() >= 250 {
                return rows;
            }
        }
    }
    rows
}

#[cfg(target_os = "macos")]
fn macos_recent_files() -> Vec<InspectRow> {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    let out = Command::new("mdfind")
        .args([
            "-onlyin",
            &home,
            "kMDItemLastUsedDate >= $time.today(-30)",
        ])
        .output();
    let Ok(out) = out else {
        return Vec::new();
    };
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .take(80)
        .map(|line| {
            let path = PathBuf::from(line);
            let title = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(line)
                .to_string();
            InspectRow {
                title,
                detail: line.to_string(),
                path: Some(path),
            }
        })
        .collect()
}

#[cfg(target_os = "macos")]
fn open_mac_app(name: &str) -> Result<(), String> {
    Command::new("open")
        .args(["-a", name])
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("Не удалось открыть {name}: {e}"))
}

fn spawn_if_exists(bin: &str, args: &[&str]) -> bool {
    if !command_on_path(bin) && !bin.contains('/') {
        return false;
    }
    Command::new(bin)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .is_ok()
}

#[cfg(not(target_os = "macos"))]
fn spawn_flatpak(app_id: &str) -> bool {
    if !command_on_path("flatpak") {
        return false;
    }
    Command::new("flatpak")
        .args(["run", app_id])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .is_ok()
}

#[cfg(not(target_os = "macos"))]
fn flatpak_app_exists(app_id: &str) -> bool {
    if !command_on_path("flatpak") {
        return false;
    }
    Command::new("flatpak")
        .args(["info", app_id])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub fn open_path(path: &Path) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(path)
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("Не удалось открыть {}: {e}", path.display()))
    }
    #[cfg(not(target_os = "macos"))]
    {
        Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("Не удалось открыть {}: {e}", path.display()))
    }
}

fn spawn_in_terminal(script: &str) -> Result<(), String> {
    let wrapped = format!("{script}; echo; printf 'Нажмите Enter...'; read _");

    #[cfg(target_os = "macos")]
    {
        let escaped = applescript_escape(&wrapped);
        let osa = format!("tell application \"Terminal\" to do script \"{escaped}\"");
        return Command::new("osascript")
            .args(["-e", &osa])
            .spawn()
            .map(|_| ())
            .map_err(|e| format!("Не удалось открыть Терминал: {e}"));
    }

    #[cfg(not(target_os = "macos"))]
    {
        let candidates: &[(&str, &[&str])] = &[
            ("x-terminal-emulator", &["-e", "sh", "-c"]),
            ("gnome-terminal", &["--", "sh", "-c"]),
            ("konsole", &["-e", "sh", "-c"]),
            ("xfce4-terminal", &["-e", "sh", "-c"]),
            ("mate-terminal", &["-e", "sh", "-c"]),
            ("kitty", &["sh", "-c"]),
            ("alacritty", &["-e", "sh", "-c"]),
            ("xterm", &["-e", "sh", "-c"]),
        ];
        for (term, prefix) in candidates {
            if !command_on_path(term) {
                continue;
            }
            let mut cmd = Command::new(term);
            cmd.args(*prefix);
            cmd.arg(&wrapped);
            if cmd.spawn().is_ok() {
                return Ok(());
            }
        }
        Err("Не найден терминал. Установите gnome-terminal, konsole или xterm.".into())
    }
}

fn command_on_path(name: &str) -> bool {
    if name.contains(['/', '\\']) {
        return Path::new(name).is_file();
    }
    let Ok(path) = std::env::var("PATH") else {
        return false;
    };
    std::env::split_paths(&path).any(|dir| dir.join(name).is_file())
}

fn sh_single_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

#[cfg(target_os = "macos")]
fn applescript_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn xbel_href_decodes_file_url() {
        let xml = r#"<bookmark href="file:///home/user/My%20Mod.jar"/>"#;
        let rows = parse_xbel(xml);
        assert_eq!(rows.len(), 1);
        assert!(rows[0].detail.contains("My Mod.jar"));
    }

    #[test]
    fn posix_module_never_mentions_everything_exe() {
        let src = include_str!("posix.rs");
        assert!(!src.to_ascii_lowercase().contains("everything.exe"));
    }
}
