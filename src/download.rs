use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
#[cfg(windows)]
use std::process::Command;
use std::time::Duration;
#[cfg(windows)]
use std::time::Instant;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

use serde::Deserialize;
use sha2::{Digest, Sha256};
use zip::ZipArchive;

use crate::tools::paths::{assets_dir, forensic_tools_supported, is_offline, resource_path};
use crate::win;

const EMBEDDED_MANIFEST: &str = include_str!("../assets/tools.json");
const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) CubeCheck/1.0-beta";

fn http_agent() -> Result<ureq::Agent, String> {
    Ok(ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(25))
        .timeout_read(Duration::from_secs(180))
        .timeout_write(Duration::from_secs(30))
        .user_agent(USER_AGENT)
        .build())
}

pub fn downloads_enabled() -> bool {
    forensic_tools_supported() && !is_offline()
}

pub fn offline_missing_message(name: &str, path: &Path) -> String {
    format!(
        "{name} не найден в офлайн-сборке.\nОжидался файл: {}\nЗагрузка из сети отключена.",
        path.display()
    )
}

#[derive(Debug, Clone, Deserialize)]
pub struct ToolsManifest {
    pub tools: Vec<ToolSpec>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ToolSpec {
    pub id: String,
    pub name: String,
    pub url: String,
    #[serde(default)]
    pub mirrors: Vec<String>,
    #[serde(default)]
    pub sha256: Option<String>,
    pub publisher: String,
    pub kind: String,
    #[serde(default)]
    pub extract: Vec<ExtractRule>,
    #[serde(default)]
    pub verify: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ExtractRule {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone)]
pub enum ToolProgress {
    Connecting,
    Receiving { received: u64, total: Option<u64> },
    Verifying,
    Extracting,
}

impl ToolsManifest {
    pub fn get(&self, id: &str) -> Option<&ToolSpec> {
        self.tools.iter().find(|t| t.id == id)
    }
}

pub fn load_manifest() -> Result<ToolsManifest, String> {
    let path = resource_path("tools.json");
    let text = if path.exists() {
        fs::read_to_string(&path).map_err(|e| format!("Не удалось прочитать список загрузок: {e}"))?
    } else {
        EMBEDDED_MANIFEST.to_string()
    };
    serde_json::from_str(&text).map_err(|e| format!("Повреждён список загрузок: {e}"))
}

pub fn tool_installed(spec: &ToolSpec) -> bool {
    crate::tools::paths::tool_installed(&spec.id)
}

pub fn missing_tools(manifest: &ToolsManifest) -> Vec<ToolSpec> {
    manifest
        .tools
        .iter()
        .filter(|t| !tool_installed(t))
        .cloned()
        .collect()
}

/// Download a tool if missing. Offline / non-Windows builds never hit the network.
pub fn download_tool(
    spec: &ToolSpec,
    force: bool,
    mut on_progress: impl FnMut(ToolProgress),
) -> Result<(), String> {
    if !force && tool_installed(spec) {
        return Ok(());
    }

    if !forensic_tools_supported() {
        return Err(format!(
            "{} работает только в Windows. На этой ОС файл не загружается.",
            spec.name
        ));
    }

    if is_offline() {
        let path = crate::tools::paths::tool_path(&spec.id);
        return Err(offline_missing_message(&spec.name, &path));
    }

    let urls = candidate_urls(spec)?;

    on_progress(ToolProgress::Connecting);
    let dest_dir = assets_dir();
    fs::create_dir_all(&dest_dir).map_err(|e| format!("Не удалось создать папку загрузок: {e}"))?;

    let tmp_dir = std::env::temp_dir().join("cubecheck-dl");
    fs::create_dir_all(&tmp_dir).map_err(|e| format!("Не удалось создать временную папку: {e}"))?;
    let tmp_file = tmp_dir.join(format!("{}.part", spec.id));

    download_https(&urls, spec.kind != "exe", &tmp_file, |received, total| {
        on_progress(ToolProgress::Receiving { received, total });
    })?;

    if let Some(expected) = spec.sha256.as_deref().filter(|s| !s.is_empty()) {
        on_progress(ToolProgress::Verifying);
        let actual = sha256_file(&tmp_file)?;
        if actual != expected.to_ascii_lowercase() {
            let _ = fs::remove_file(&tmp_file);
            return Err(format!("{}: файл повреждён, скачайте снова", spec.name));
        }
    }

    on_progress(ToolProgress::Extracting);
    if spec.id == "systeminformer" {
        let _ = fs::remove_dir_all(dest_dir.join("SystemInformer"));
    }
    match spec.kind.as_str() {
        "exe" => {
            let to = spec
                .extract
                .first()
                .map(|r| r.to.as_str())
                .or_else(|| spec.verify.first().map(String::as_str))
                .unwrap_or("tool.exe");
            let dest = safe_dest(&dest_dir, to)?;
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            fs::copy(&tmp_file, &dest)
                .map_err(|e| format!("Не удалось сохранить {}: {e}", dest.display()))?;
        }
        _ => extract_selected(&tmp_file, &dest_dir, &spec.extract)?,
    }
    if spec.id == "systeminformer" {
        finalize_systeminformer(&dest_dir)?;
    }
    let _ = fs::remove_file(&tmp_file);

    on_progress(ToolProgress::Verifying);
    let verify_err = spec.verify.iter().find_map(|rel| {
        let path = dest_dir.join(rel);
        win::verify_authenticode_publisher(&path, &spec.publisher).err()
    });
    if let Some(err) = verify_err {
        for rule in &spec.extract {
            let _ = fs::remove_file(dest_dir.join(&rule.to));
        }
        for rel in &spec.verify {
            let _ = fs::remove_file(dest_dir.join(rel));
        }
        return Err(err);
    }
    Ok(())
}

fn candidate_urls(spec: &ToolSpec) -> Result<Vec<String>, String> {
    let mut urls = Vec::with_capacity(1 + spec.mirrors.len());
    urls.push(spec.url.clone());
    urls.extend(spec.mirrors.iter().cloned());
    urls.retain(|u| !u.is_empty());
    if urls.iter().any(|u| !u.starts_with("https://")) {
        return Err(format!("{}: ссылка должна быть https", spec.name));
    }
    if urls.is_empty() {
        return Err(format!("{}: нет URL для загрузки", spec.name));
    }
    Ok(urls)
}

fn download_https(
    urls: &[String],
    expect_zip: bool,
    dest: &Path,
    mut on_chunk: impl FnMut(u64, Option<u64>),
) -> Result<(), String> {
    let mut errors = Vec::new();

    #[cfg(windows)]
    if let Some(curl) = curl_exe() {
        for url in urls {
            match download_via_curl(&curl, url, dest, &mut on_chunk)
                .and_then(|_| accept_payload(dest, expect_zip, url))
            {
                Ok(()) => return Ok(()),
                Err(e) => errors.push(e),
            }
        }
    }

    for url in urls {
        match download_via_ureq(url, dest, &mut on_chunk)
            .and_then(|_| accept_payload(dest, expect_zip, url))
        {
            Ok(()) => return Ok(()),
            Err(e) => errors.push(e),
        }
    }

    Err(match errors.as_slice() {
        [] => "Не удалось скачать файл".into(),
        [single] => single.clone(),
        many => format!("Не удалось скачать:\n{}", many.join("\n")),
    })
}

#[cfg(windows)]
fn curl_exe() -> Option<PathBuf> {
    for candidate in [
        r"C:\Windows\System32\curl.exe",
        r"C:\Windows\Sysnative\curl.exe",
    ] {
        let path = PathBuf::from(candidate);
        if path.exists() {
            return Some(path);
        }
    }
    let mut cmd = Command::new("where");
    cmd.arg("curl.exe");
    cmd.creation_flags(CREATE_NO_WINDOW);
    let Ok(out) = cmd.output() else {
        return None;
    };
    if !out.status.success() {
        return None;
    }
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(PathBuf::from)
        .filter(|path| path.exists())
}

#[cfg(windows)]
fn download_via_curl(
    curl: &Path,
    url: &str,
    dest: &Path,
    on_chunk: &mut impl FnMut(u64, Option<u64>),
) -> Result<(), String> {
    let _ = fs::remove_file(dest);
    let mut cmd = Command::new(curl);
    cmd.args([
        "-fL",
        "--connect-timeout",
        "20",
        "--max-time",
        "180",
        "--retry",
        "2",
        "--retry-delay",
        "1",
        "-A",
        USER_AGENT,
        "-o",
    ]);
    cmd.arg(dest);
    cmd.arg(url);
    cmd.creation_flags(CREATE_NO_WINDOW);

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Не удалось запустить загрузчик: {e}"))?;

    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {
                if let Ok(meta) = fs::metadata(dest) {
                    on_chunk(meta.len(), None);
                }
                if started.elapsed() > Duration::from_secs(200) {
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = fs::remove_file(dest);
                    return Err(format!("Превышено время загрузки {url}"));
                }
                std::thread::sleep(Duration::from_millis(200));
            }
            Err(e) => {
                let _ = child.kill();
                return Err(format!("Сбой загрузки {url}: {e}"));
            }
        }
    };

    if !status.success() {
        let _ = fs::remove_file(dest);
        return Err(format!(
            "Ошибка загрузки {url} (код {})",
            status.code().unwrap_or(-1)
        ));
    }

    let received = fs::metadata(dest).map(|m| m.len()).unwrap_or(0);
    if received < 1024 {
        let _ = fs::remove_file(dest);
        return Err(format!(
            "Загруженный файл слишком маленький ({received} байт) — {url}"
        ));
    }
    on_chunk(received, Some(received));
    Ok(())
}

fn download_via_ureq(
    url: &str,
    dest: &Path,
    on_chunk: &mut impl FnMut(u64, Option<u64>),
) -> Result<(), String> {
    let response = http_agent()?
        .get(url)
        .call()
        .map_err(|e| format!("Ошибка загрузки {url}: {e}"))?;

    if response.status() < 200 || response.status() >= 300 {
        return Err(format!("HTTP {} при загрузке {url}", response.status()));
    }

    let total = response
        .header("Content-Length")
        .and_then(|v| v.parse::<u64>().ok());
    let mut reader = response.into_reader();
    let mut file = File::create(dest).map_err(|e| format!("Не удалось создать временный файл: {e}"))?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    let mut received = 0u64;

    loop {
        let n = reader
            .read(&mut buf)
            .map_err(|e| format!("Обрыв загрузки: {e}"))?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n])
            .map_err(|e| format!("Ошибка записи: {e}"))?;
        hasher.update(&buf[..n]);
        received += n as u64;
        on_chunk(received, total);
    }
    file.flush().map_err(|e| e.to_string())?;

    if received < 1024 {
        let _ = fs::remove_file(dest);
        return Err("Скачался не тот файл. Нажмите «Повтор».".into());
    }
    let _ = hasher;
    Ok(())
}

fn looks_like_zip(path: &Path) -> bool {
    let Ok(mut file) = File::open(path) else {
        return false;
    };
    let mut magic = [0u8; 2];
    file.read_exact(&mut magic).is_ok() && magic == *b"PK"
}

fn accept_payload(dest: &Path, expect_zip: bool, _url: &str) -> Result<(), String> {
    if expect_zip && !looks_like_zip(dest) {
        let _ = fs::remove_file(dest);
        return Err("Скачался не архив. Нажмите «Повтор».".into());
    }
    Ok(())
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let mut file = File::open(path).map_err(|e| e.to_string())?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn extract_selected(zip_path: &Path, dest_dir: &Path, rules: &[ExtractRule]) -> Result<(), String> {
    let file = File::open(zip_path).map_err(|e| format!("Не удалось открыть архив: {e}"))?;
    let mut archive = ZipArchive::new(file).map_err(|e| format!("Повреждён ZIP: {e}"))?;

    for rule in rules {
        if is_skipped_archive_path(&rule.from) {
            continue;
        }
        let index = find_zip_entry(&mut archive, &rule.from).ok_or_else(|| {
            let listed = zip_entry_names(&mut archive);
            format!(
                "В архиве нет файла {} (есть: {})",
                rule.from,
                listed.join(", ")
            )
        })?;
        let mut entry = archive
            .by_index(index)
            .map_err(|e| format!("Ошибка чтения {}: {e}", rule.from))?;
        if entry.is_dir() {
            continue;
        }
        let dest = safe_dest(dest_dir, &rule.to)?;
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let mut out = File::create(&dest)
            .map_err(|e| format!("Не удалось записать {}: {e}", dest.display()))?;
        std::io::copy(&mut entry, &mut out).map_err(|e| format!("Распаковка {}: {e}", rule.to))?;
    }
    Ok(())
}

fn find_zip_entry<R: Read + std::io::Seek>(archive: &mut ZipArchive<R>, wanted: &str) -> Option<usize> {
    let wanted = wanted.replace('\\', "/");
    let wanted_l = wanted.to_ascii_lowercase();
    let wanted_name = Path::new(&wanted)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(&wanted);

    let mut best: Option<(i32, usize)> = None;
    for i in 0..archive.len() {
        let Ok(entry) = archive.by_index(i) else {
            continue;
        };
        let name = entry.name().replace('\\', "/");
        let lower = name.to_ascii_lowercase();
        if is_skipped_archive_path(&name) || path_is_32bit(&lower) {
            continue;
        }
        let file_name = Path::new(&name)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");
        if !file_name.eq_ignore_ascii_case(wanted_name) {
            continue;
        }
        let mut score = arch_score(&lower);
        if lower == wanted_l || lower.ends_with(&format!("/{wanted_l}")) {
            score += 50;
        }
        if best.map(|(s, _)| score > s).unwrap_or(true) {
            best = Some((score, i));
        }
    }
    best.map(|(_, i)| i)
}

fn path_is_32bit(lower: &str) -> bool {
    lower
        .split('/')
        .any(|part| matches!(part, "x86" | "win32" | "i386" | "ia32" | "wow64"))
}

fn arch_score(lower: &str) -> i32 {
    if lower
        .split('/')
        .any(|part| matches!(part, "amd64" | "x64" | "win64"))
    {
        100
    } else {
        0
    }
}

fn finalize_systeminformer(dest_dir: &Path) -> Result<(), String> {
    let si_dir = dest_dir.join("SystemInformer");
    strip_systeminformer_extras(&si_dir);
    let exe = si_dir.join("SystemInformer.exe");
    if exe.exists() && !win::is_pe_amd64(&exe) {
        let _ = fs::remove_dir_all(&si_dir);
        return Err("Скачалась 32-битная версия. Нажмите «Повтор».".into());
    }
    write_systeminformer_settings(&si_dir)
}

pub(crate) fn strip_systeminformer_extras(si_dir: &Path) {
    for extra in ["plugins", "peview", "Resources", "x86"] {
        let _ = fs::remove_dir_all(si_dir.join(extra));
    }
}

pub(crate) fn write_systeminformer_settings(si_dir: &Path) -> Result<(), String> {
    let settings = concat!(
        "<settings>\n",
        "<setting name=\"EnablePlugins\">0</setting>\n",
        "<setting name=\"EnableDefaultSafePlugins\">0</setting>\n",
        "<setting name=\"DisabledPlugins\">",
        "DotNetTools.dll|ExtendedNotifications.dll|ExtendedServices.dll|",
        "ExtendedTools.dll|HardwareDevices.dll|NetworkTools.dll|",
        "OnlineChecks.dll|ToolStatus.dll|Updater.dll|UserNotes.dll|WindowExplorer.dll",
        "</setting>\n",
        "</settings>\n"
    );
    fs::write(si_dir.join("SystemInformer.exe.settings.xml"), settings)
        .map_err(|e| format!("Не удалось записать настройки System Informer: {e}"))
}

fn zip_entry_names<R: Read + std::io::Seek>(archive: &mut ZipArchive<R>) -> Vec<String> {
    (0..archive.len())
        .filter_map(|i| archive.by_index(i).ok().map(|e| e.name().to_string()))
        .collect()
}

fn is_skipped_archive_path(name: &str) -> bool {
    let lower = name.replace('\\', "/").to_ascii_lowercase();
    lower.contains("/plugins/")
        || lower.contains("/peview")
        || lower.contains("/resources/")
        || lower.ends_with("/plugins")
        || lower.ends_with("/resources")
}

fn safe_dest(base: &Path, rel: &str) -> Result<PathBuf, String> {
    let rel = rel.replace('\\', "/");
    if rel.is_empty() || Path::new(&rel).components().any(|c| matches!(c, Component::ParentDir)) {
        return Err("некорректный путь распаковки".into());
    }
    Ok(base.join(rel))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_manifest_parses() {
        let manifest = load_manifest().expect("tools.json");
        assert_eq!(manifest.tools.len(), 6);
        assert!(manifest.get("everything").is_some());
        assert!(manifest.get("shellbag").unwrap().url.starts_with("https://"));
        assert_eq!(
            manifest.get("everything").unwrap().publisher,
            "voidtools PTY LTD"
        );
        let si = manifest.get("systeminformer").unwrap();
        assert!(si
            .extract
            .iter()
            .all(|r| r.from.to_ascii_lowercase().starts_with("amd64/")));
        let urls = candidate_urls(si).expect("si urls");
        assert!(urls.iter().all(|u| u.starts_with("https://")));
        assert!(urls.len() >= 2);
    }

    #[test]
    fn offline_message_is_explicit() {
        let msg = offline_missing_message("Everything", Path::new("assets/Everything.exe"));
        assert!(msg.contains("Everything"));
        assert!(msg.contains("офлайн"));
        assert!(msg.contains("Everything.exe"));
    }

    #[test]
    fn rejects_32bit_archive_paths() {
        assert!(path_is_32bit("x86/systeminformer.exe"));
        assert!(path_is_32bit("win32/app.exe"));
        assert!(!path_is_32bit("amd64/systeminformer.exe"));
        assert_eq!(arch_score("amd64/systeminformer.exe"), 100);
        assert_eq!(arch_score("systeminformer.exe"), 0);
    }
}
