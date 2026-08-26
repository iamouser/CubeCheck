//! Tiny std-only launcher for the CubeCheck universal bundle.
//! Picks OS/arch, then execs `payload/<id>/cubecheck[.exe]`.
//! Designed to run on Windows 7+ (no extra crates / Win8+ APIs).

#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{self, Command};

fn main() {
    if let Err(e) = run() {
        die(&e);
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    let self_exe = env::current_exe().map_err(|e| format!("Не удалось определить путь лаунчера: {e}"))?;
    let root = self_exe
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| "Не удалось определить папку лаунчера".to_string())?;

    let candidates = payload_candidates();
    let (kind, payload_dir, exe) = find_payload(&root, &candidates)?;

    let mut cmd = Command::new(&exe);
    cmd.current_dir(&payload_dir);
    cmd.args(&args);
    cmd.env("CUBECHECK_PORTABLE", "1");
    cmd.env("CUBECHECK_LAUNCHER_OS", &kind);
    if offline_marker(&root) || offline_marker(&payload_dir) {
        cmd.env("CUBECHECK_OFFLINE", "1");
    }

    let status = cmd
        .status()
        .map_err(|e| format!("Не удалось запустить {}: {e}", exe.display()))?;
    if let Some(code) = status.code() {
        process::exit(code);
    }
    if !status.success() {
        return Err(format!("{} завершился с ошибкой", exe.display()));
    }
    Ok(())
}

fn offline_marker(dir: &Path) -> bool {
    dir.join(".offline").is_file() || dir.join("assets").join(".offline").is_file()
}

fn payload_candidates() -> Vec<(&'static str, &'static str)> {
    let mut out = Vec::new();
    let os = env::consts::OS;
    match os {
        "windows" => {
            if windows_is_64bit() {
                out.push(("windows-x64", "windows-x64"));
                out.push(("windows-x86", "windows-x86"));
            } else {
                out.push(("windows-x86", "windows-x86"));
                out.push(("windows-x64", "windows-x64"));
            }
        }
        "linux" => {
            if unix_is_64bit() {
                out.push(("linux-x64", "linux-x64"));
                out.push(("linux-x86", "linux-x86"));
            } else {
                out.push(("linux-x86", "linux-x86"));
                out.push(("linux-x64", "linux-x64"));
            }
        }
        "macos" => {
            out.push(("macos-universal", "macos-universal"));
            if cfg!(target_arch = "aarch64") {
                out.push(("macos-arm64", "macos-arm64"));
                out.push(("macos-x64", "macos-x64"));
            } else {
                out.push(("macos-x64", "macos-x64"));
                out.push(("macos-arm64", "macos-arm64"));
            }
        }
        other => {
            let _ = other;
        }
    }
    out
}

fn windows_is_64bit() -> bool {
    let arch = env::var("PROCESSOR_ARCHITECTURE").unwrap_or_default();
    let wow = env::var("PROCESSOR_ARCHITEW6432").unwrap_or_default();
    let upper = |s: &str| s.eq_ignore_ascii_case("AMD64") || s.eq_ignore_ascii_case("ARM64");
    upper(&arch) || upper(&wow) || cfg!(target_pointer_width = "64")
}

fn unix_is_64bit() -> bool {
    match env::consts::ARCH {
        "x86_64" | "aarch64" | "powerpc64" | "riscv64" => true,
        _ => cfg!(target_pointer_width = "64"),
    }
}

fn find_payload(
    root: &Path,
    candidates: &[(&str, &str)],
) -> Result<(String, PathBuf, PathBuf), String> {
    let payload_root = root.join("payload");
    let mut tried = Vec::new();
    for (kind, folder) in candidates {
        let dir = payload_root.join(folder);
        if let Some(exe) = payload_exe(&dir) {
            return Ok((kind.to_string(), dir, exe));
        }
        tried.push(format!("{} ({})", kind, dir.display()));
    }

    let os = env::consts::OS;
    let arch = env::consts::ARCH;
    let listed = list_payloads(&payload_root);
    Err(format!(
        "Нет сборки CubeCheck для {os}/{arch}.\n\
         Искали:\n  {}\n\
         В payload/: {}\n\
         Соберите нужный артефакт (build.bat / build.sh) или запустите с ОС, для которой есть payload.",
        if tried.is_empty() {
            "(нет кандидатов для этой ОС)".into()
        } else {
            tried.join("\n  ")
        },
        if listed.is_empty() {
            "(пусто)".into()
        } else {
            listed.join(", ")
        }
    ))
}

fn payload_exe(dir: &Path) -> Option<PathBuf> {
    let names = if cfg!(windows) {
        ["cubecheck.exe", "cubecheck"].as_slice()
    } else {
        ["cubecheck", "cubecheck.exe"].as_slice()
    };
    for name in names {
        let path = dir.join(name);
        if path.is_file() {
            return Some(path);
        }
    }
    None
}

fn list_payloads(payload_root: &Path) -> Vec<String> {
    let Ok(entries) = fs::read_dir(payload_root) else {
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .flatten()
        .filter(|e| e.path().is_dir())
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();
    names.sort();
    names
}

fn die(msg: &str) -> ! {
    eprintln!("CubeCheck: {msg}");
    if let Ok(exe) = env::current_exe() {
        if let Some(dir) = exe.parent() {
            let log = dir.join("CubeCheck-error.txt");
            let body = format!("CubeCheck\r\n\r\n{msg}\r\n");
            let _ = fs::write(&log, body);
            #[cfg(windows)]
            {
                let _ = Command::new("cmd")
                    .arg("/C")
                    .arg("start")
                    .arg("")
                    .arg(&log)
                    .spawn();
                std::thread::sleep(std::time::Duration::from_millis(400));
            }
        }
    }
    process::exit(1);
}
