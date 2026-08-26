use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use aho_corasick::AhoCorasick;
use sysinfo::{ProcessRefreshKind, RefreshKind, System, UpdateKind};

use super::cheat_list::{
    CHEAT_NAMES, HITBOX_FILES, IGNORED_PROCESSES, INJECTION_KEYWORDS, KNOWN_DLLS, LOG_SUSPICIOUS,
};
use crate::tools::paths::minecraft_dir;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ScanPhase {
    Processes,
    Files,
    Registry,
    Logs,
}

impl ScanPhase {
    pub fn label(self) -> &'static str {
        match self {
            Self::Processes => "процессы",
            Self::Files => "файлы",
    Self::Registry => {
                if cfg!(windows) {
                    "реестр"
                } else {
                    "автозагрузка"
                }
            }
            Self::Logs => "логи",
        }
    }
}

pub enum ScanState {
    Idle,
    Running(ScanPhase),
    Done(Vec<String>),
}

fn build_ac(patterns: &[&str]) -> AhoCorasick {
    AhoCorasick::builder()
        .ascii_case_insensitive(true)
        .build(patterns)
        .expect("aho-corasick patterns")
}

fn cheat_matcher() -> &'static AhoCorasick {
    static AC: OnceLock<AhoCorasick> = OnceLock::new();
    AC.get_or_init(|| build_ac(CHEAT_NAMES))
}

fn ignored_matcher() -> &'static AhoCorasick {
    static AC: OnceLock<AhoCorasick> = OnceLock::new();
    AC.get_or_init(|| build_ac(IGNORED_PROCESSES))
}

fn log_matcher() -> &'static AhoCorasick {
    static AC: OnceLock<AhoCorasick> = OnceLock::new();
    AC.get_or_init(|| build_ac(LOG_SUSPICIOUS))
}

fn inject_matcher() -> &'static AhoCorasick {
    static AC: OnceLock<AhoCorasick> = OnceLock::new();
    AC.get_or_init(|| build_ac(INJECTION_KEYWORDS))
}

fn hitbox_matcher() -> &'static AhoCorasick {
    static AC: OnceLock<AhoCorasick> = OnceLock::new();
    AC.get_or_init(|| build_ac(HITBOX_FILES))
}

fn desktop_dir() -> PathBuf {
    if let Ok(profile) = std::env::var("USERPROFILE") {
        if !profile.is_empty() {
            return PathBuf::from(profile).join("Desktop");
        }
    }
    std::env::var("HOME")
        .map(|h| PathBuf::from(h).join("Desktop"))
        .unwrap_or_else(|_| PathBuf::from("Desktop"))
}

fn downloads_dir() -> PathBuf {
    if let Ok(profile) = std::env::var("USERPROFILE") {
        if !profile.is_empty() {
            return PathBuf::from(profile).join("Downloads");
        }
    }
    std::env::var("HOME")
        .map(|h| PathBuf::from(h).join("Downloads"))
        .unwrap_or_else(|_| PathBuf::from("Downloads"))
}

#[cfg(windows)]
fn scan_startup() -> Vec<String> {
    use winreg::enums::*;
    use winreg::RegKey;

    let mut found = Vec::new();
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    for key_path in [
        r"Software\Microsoft\Windows\CurrentVersion\Run",
        r"Software\Microsoft\Windows\CurrentVersion\RunOnce",
    ] {
        if let Ok(key) = hkcu.open_subkey(key_path) {
            for (name, value) in key.enum_values().flatten() {
                let value_str = value.to_string();
                if contains_cheat(&name) || contains_cheat(&value_str) {
                    found.push(format!("Автозагрузка: {name} → {value_str}"));
                }
            }
        }
    }
    found
}

#[cfg(not(windows))]
fn scan_startup() -> Vec<String> {
    let mut found = Vec::new();
    for row in crate::tools::posix::autostart_entries() {
        let hay = format!("{} {}", row.title, row.detail);
        if contains_cheat(&hay) {
            found.push(format!("Автозагрузка: {} → {}", row.title, row.detail));
        }
    }
    for row in crate::tools::posix::recent_entries() {
        let hay = format!("{} {}", row.title, row.detail);
        if contains_cheat(&hay) {
            found.push(format!("Недавний файл: {}", row.detail));
        }
    }
    found
}

fn contains_cheat(text: &str) -> bool {
    cheat_matcher().is_match(text)
}

fn first_match<'a>(ac: &AhoCorasick, text: &'a str) -> Option<String> {
    ac.find(text).map(|m| text[m.start()..m.end()].to_string())
}

fn is_ignored_process(name: &str) -> bool {
    ignored_matcher()
        .find_iter(name)
        .any(|m| m.start() == 0 && m.end() == name.len())
        || IGNORED_PROCESSES
            .iter()
            .any(|p| name.eq_ignore_ascii_case(p))
}

fn set_phase(state: &Mutex<ScanState>, phase: ScanPhase) {
    if let Ok(mut guard) = state.lock() {
        *guard = ScanState::Running(phase);
    }
}

fn scan_minecraft_log() -> Vec<String> {
    let mut found = Vec::new();
    let log_path = minecraft_dir().join("logs").join("latest.log");
    let Ok(content) = std::fs::read_to_string(&log_path) else {
        return found;
    };

    let lines: Vec<&str> = content.lines().collect();
    let start = lines.len().saturating_sub(300);
    for line in &lines[start..] {
        if let Some(pat) = first_match(inject_matcher(), line) {
            found.push(format!("В логах: {pat}"));
            continue;
        }
        if let Some(pat) = first_match(log_matcher(), line) {
            found.push(format!("В логах: {pat}"));
        }
    }
    found
}

fn scan_hitbox_suspicion() -> Vec<String> {
    let mut found = Vec::new();
    let folders = [
        minecraft_dir().join("mods"),
        minecraft_dir().join("versions"),
        desktop_dir(),
        downloads_dir(),
    ];

    for folder in &folders {
        if !folder.exists() {
            continue;
        }
        walk_limited(folder, 2, &mut |path| {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if let Some(pattern) = first_match(hitbox_matcher(), name) {
                    found.push(format!("Подозрительный файл ({pattern}): {}", path.display()));
                }
            }
        });
    }
    found
}

fn scan_unknown_dlls() -> Vec<String> {
    let mut found = Vec::new();
    let dll_folders = [
        minecraft_dir().join("bin"),
        minecraft_dir().join("versions"),
    ];

    for folder in &dll_folders {
        if !folder.exists() {
            continue;
        }
        walk_limited(folder, 2, &mut |path| {
            if path.extension().and_then(|e| e.to_str()) != Some("dll") {
                return;
            }
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            let known = KNOWN_DLLS.iter().any(|k| name.contains(k));
            if !known {
                found.push(format!("Неизвестный .dll: {}", path.display()));
            }
        });
    }
    found
}

fn walk_limited(dir: &Path, max_depth: usize, f: &mut dyn FnMut(&Path)) {
    fn walk(dir: &Path, depth: usize, max_depth: usize, f: &mut dyn FnMut(&Path)) {
        if depth > max_depth {
            return;
        }
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, depth + 1, max_depth, f);
            } else {
                f(&path);
            }
        }
    }
    walk(dir, 0, max_depth, f);
}

pub fn perform_scan(state: Arc<Mutex<ScanState>>) -> Vec<String> {
    let mut found = Vec::new();

    set_phase(&state, ScanPhase::Processes);
    let mut system = System::new_with_specifics(RefreshKind::nothing().with_processes(
        ProcessRefreshKind::nothing()
            .with_exe(UpdateKind::OnlyIfNotSet)
            .with_cmd(UpdateKind::OnlyIfNotSet),
    ));
    system.refresh_processes(sysinfo::ProcessesToUpdate::All, true);

    for (_, proc) in system.processes() {
        let name = proc.name().to_string_lossy();
        if is_ignored_process(&name) {
            continue;
        }
        let exe = proc
            .exe()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        let cmdline = proc
            .cmd()
            .iter()
            .map(|s| s.to_string_lossy())
            .collect::<Vec<_>>()
            .join(" ");
        let haystack = format!("{name} {exe} {cmdline}");
        if contains_cheat(&haystack) {
            found.push(format!("Процесс: {name} (путь: {exe})"));
        }
    }

    set_phase(&state, ScanPhase::Files);
    let folders = [
        minecraft_dir().join("versions"),
        minecraft_dir().join("mods"),
        desktop_dir(),
        downloads_dir(),
        std::env::temp_dir(),
    ];

    for folder in &folders {
        if !folder.exists() {
            continue;
        }
        walk_limited(folder, 2, &mut |path| {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if contains_cheat(name) {
                    found.push(format!("Файл: {}", path.display()));
                }
            }
        });
    }

    set_phase(&state, ScanPhase::Registry);
    found.extend(scan_startup());

    set_phase(&state, ScanPhase::Logs);
    let log_results = scan_minecraft_log();
    if !log_results.is_empty() {
        found.push("Логи Minecraft:".into());
        for item in log_results.iter().take(5) {
            found.push(format!("   {item}"));
        }
    }

    let hitbox_results = scan_hitbox_suspicion();
    if !hitbox_results.is_empty() {
        found.push("Подозрительные файлы:".into());
        for item in hitbox_results.iter().take(5) {
            found.push(format!("   {item}"));
        }
    }

    let dll_results = scan_unknown_dlls();
    if !dll_results.is_empty() {
        found.push("DLL:".into());
        for item in dll_results.iter().take(5) {
            found.push(format!("   {item}"));
        }
    }

    found.push(format!(
        "Корзина: {}",
        crate::system::recycle_bin_last_change()
    ));

    found
}
