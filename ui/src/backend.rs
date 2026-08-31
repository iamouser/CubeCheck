use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::path::PathBuf;
use std::sync::OnceLock;

use libloading::{Library, Symbol};

use crate::scan::ScanPhase;

type FreeFn = unsafe extern "C" fn(*mut c_char);
type LastErrorFn = unsafe extern "C" fn() -> *mut c_char;
type InitFn = unsafe extern "C" fn() -> c_int;
type LoadSettingsFn = unsafe extern "C" fn(*mut *mut c_char) -> c_int;
type SaveSettingsFn = unsafe extern "C" fn(*const c_char) -> c_int;
type FlagFn = unsafe extern "C" fn() -> c_int;
type ToolInstalledFn = unsafe extern "C" fn(*const c_char) -> c_int;
type StringOutFn = unsafe extern "C" fn(*mut *mut c_char) -> c_int;
type RunUtilFn = unsafe extern "C" fn(*const c_char) -> c_int;
type ActionFn = unsafe extern "C" fn() -> c_int;
type PhaseCb = unsafe extern "C" fn(c_int, *mut c_void);
type ScanFn = unsafe extern "C" fn(PhaseCb, *mut c_void, *mut *mut c_char) -> c_int;
type ProgressCb = unsafe extern "C" fn(
    *const c_char,
    c_int,
    i64,
    i64,
    *const c_char,
    *mut c_void,
);
type DownloadFn = unsafe extern "C" fn(*const c_char, c_int, ProgressCb, *mut c_void) -> c_int;
type SaveReportFn = unsafe extern "C" fn(*const c_char, *mut *mut c_char) -> c_int;

struct Api {
    _lib: Library,
    free: FreeFn,
    last_error: LastErrorFn,
    init: InitFn,
    load_settings: LoadSettingsFn,
    save_settings: SaveSettingsFn,
    is_offline: FlagFn,
    downloads_enabled: FlagFn,
    any_tool_missing: FlagFn,
    tool_installed: ToolInstalledFn,
    missing_tool_ids: StringOutFn,
    cheat_list_text: StringOutFn,
    run_util: RunUtilFn,
    run_autocheck_search: ActionFn,
    open_recycle: ActionFn,
    open_telegram: ActionFn,
    open_holycheck: ActionFn,
    run_system_info: ActionFn,
    clear_logs: ActionFn,
    perform_scan: ScanFn,
    download_tools: DownloadFn,
    save_report: SaveReportFn,
    user_name: StringOutFn,
    computer_name: StringOutFn,
    install_date: StringOutFn,
    recycle_mtime: StringOutFn,
    os_info_label: StringOutFn,
}

static API: OnceLock<Result<Api, String>> = OnceLock::new();

fn library_name() -> &'static str {
    if cfg!(windows) {
        "cubecheck_api.dll"
    } else if cfg!(target_os = "macos") {
        "libcubecheck_api.dylib"
    } else {
        "libcubecheck_api.so"
    }
}

fn push_lib(paths: &mut Vec<PathBuf>, dir: &std::path::Path, name: &str) {
    paths.push(dir.join("assets").join(name));
    paths.push(dir.join(name));
}

fn search_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let name = library_name();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            push_lib(&mut paths, dir, name);
        }
    }
    if let Ok(dir) = std::env::var("CUBECHECK_API_DIR") {
        let dir = PathBuf::from(dir);
        paths.push(dir.join(name));
        push_lib(&mut paths, &dir, name);
    }
    if let Ok(explicit) = std::env::var("CUBECHECK_API_DLL") {
        paths.push(PathBuf::from(explicit));
    }
    if let Ok(cwd) = std::env::current_dir() {
        push_lib(&mut paths, &cwd, name);
        paths.push(
            cwd.join("src/CubeCheck.Api/bin/Release/net8.0/win-x64/publish")
                .join(name),
        );
        paths.push(cwd.join("dist/windows-x64/assets").join(name));
        paths.push(cwd.join("target/release/assets").join(name));
    }
    paths
}

fn load_symbol<T>(lib: &Library, name: &[u8]) -> Result<T, String>
where
    T: Copy,
{
    let symbol: Symbol<T> = unsafe {
        lib.get(name)
            .map_err(|e| format!("Нет функции {}: {e}", String::from_utf8_lossy(name)))?
    };
    Ok(*symbol)
}

fn load_api() -> Result<Api, String> {
    let mut last = String::new();
    for path in search_paths() {
        if !path.is_file() {
            continue;
        }
        match unsafe { Library::new(&path) } {
            Ok(lib) => {
                return Ok(Api {
                    free: load_symbol(&lib, b"cc_host_free\0")?,
                    last_error: load_symbol(&lib, b"cc_host_last_error\0")?,
                    init: load_symbol(&lib, b"cc_host_init\0")?,
                    load_settings: load_symbol(&lib, b"cc_host_load_settings\0")?,
                    save_settings: load_symbol(&lib, b"cc_host_save_settings\0")?,
                    is_offline: load_symbol(&lib, b"cc_host_is_offline\0")?,
                    downloads_enabled: load_symbol(&lib, b"cc_host_downloads_enabled\0")?,
                    any_tool_missing: load_symbol(&lib, b"cc_host_any_tool_missing\0")?,
                    tool_installed: load_symbol(&lib, b"cc_host_tool_installed\0")?,
                    missing_tool_ids: load_symbol(&lib, b"cc_host_missing_tool_ids\0")?,
                    cheat_list_text: load_symbol(&lib, b"cc_host_cheat_list_text\0")?,
                    run_util: load_symbol(&lib, b"cc_host_run_util\0")?,
                    run_autocheck_search: load_symbol(&lib, b"cc_host_run_autocheck_search\0")?,
                    open_recycle: load_symbol(&lib, b"cc_host_open_recycle\0")?,
                    open_telegram: load_symbol(&lib, b"cc_host_open_telegram\0")?,
                    open_holycheck: load_symbol(&lib, b"cc_host_open_holycheck\0")?,
                    run_system_info: load_symbol(&lib, b"cc_host_run_system_info\0")?,
                    clear_logs: load_symbol(&lib, b"cc_host_clear_logs\0")?,
                    perform_scan: load_symbol(&lib, b"cc_host_perform_scan\0")?,
                    download_tools: load_symbol(&lib, b"cc_host_download_tools\0")?,
                    save_report: load_symbol(&lib, b"cc_host_save_report\0")?,
                    user_name: load_symbol(&lib, b"cc_host_user_name\0")?,
                    computer_name: load_symbol(&lib, b"cc_host_computer_name\0")?,
                    install_date: load_symbol(&lib, b"cc_host_install_date\0")?,
                    recycle_mtime: load_symbol(&lib, b"cc_host_recycle_mtime\0")?,
                    os_info_label: load_symbol(&lib, b"cc_host_os_info_label\0")?,
                    _lib: lib,
                });
            }
            Err(e) => last = format!("{}: {e}", path.display()),
        }
    }
    Err(if last.is_empty() {
        format!(
            "В assets/ (и рядом с программой) нет {}. Соберите CubeCheck.Api (Windows: cubecheck_api.dll; Linux: libcubecheck_api.so).",
            library_name()
        )
    } else {
        last
    })
}

fn api() -> Result<&'static Api, String> {
    match API.get_or_init(load_api) {
        Ok(api) => Ok(api),
        Err(e) => Err(e.clone()),
    }
}

fn take_string(api: &Api, ptr: *mut c_char) -> String {
    if ptr.is_null() {
        return String::new();
    }
    let text = unsafe { CStr::from_ptr(ptr) }
        .to_string_lossy()
        .into_owned();
    unsafe { (api.free)(ptr) };
    text
}

fn last_error(api: &Api) -> String {
    let ptr = unsafe { (api.last_error)() };
    let text = take_string(api, ptr);
    if text.is_empty() {
        "Ошибка CubeCheck.Core".into()
    } else {
        text
    }
}

fn cstr(text: &str) -> Result<CString, String> {
    CString::new(text).map_err(|_| "Некорректная строка для API".into())
}

fn call_action(action: ActionFn) -> Result<(), String> {
    let api = api()?;
    if unsafe { action() } == 0 {
        Ok(())
    } else {
        Err(last_error(api))
    }
}

fn call_string(func: StringOutFn) -> Result<String, String> {
    let api = api()?;
    let mut ptr: *mut c_char = std::ptr::null_mut();
    if unsafe { func(&mut ptr) } != 0 {
        return Err(last_error(api));
    }
    Ok(take_string(api, ptr))
}

pub fn init() -> Result<(), String> {
    let api = api()?;
    if unsafe { (api.init)() } == 0 {
        Ok(())
    } else {
        Err(last_error(api))
    }
}

pub fn load_settings() -> Result<String, String> {
    call_string(api()?.load_settings)
}

pub fn save_settings(json: &str) -> Result<(), String> {
    let api = api()?;
    let json = cstr(json)?;
    if unsafe { (api.save_settings)(json.as_ptr()) } == 0 {
        Ok(())
    } else {
        Err(last_error(api))
    }
}

#[allow(dead_code)]
pub fn api_loaded() -> bool {
    api().is_ok()
}

pub fn is_offline() -> bool {
    api().ok().is_some_and(|a| unsafe { (a.is_offline)() } != 0)
}

pub fn downloads_enabled() -> bool {
    api().ok().is_some_and(|a| unsafe { (a.downloads_enabled)() } != 0)
}

pub fn any_tool_missing() -> bool {
    api().ok().is_some_and(|a| unsafe { (a.any_tool_missing)() } != 0)
}

pub fn tool_installed(id: &str) -> bool {
    let Ok(api) = api() else { return false };
    let Ok(id) = cstr(id) else { return false };
    unsafe { (api.tool_installed)(id.as_ptr()) != 0 }
}

pub fn missing_tool_ids() -> Result<Vec<String>, String> {
    let text = call_string(api()?.missing_tool_ids)?;
    Ok(text
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect())
}

pub fn cheat_list_text() -> String {
    match api() {
        Ok(host) => call_string(host.cheat_list_text).unwrap_or_default(),
        Err(_) => String::new(),
    }
}

pub fn run_util(id: &str) -> Result<(), String> {
    let api = api()?;
    let id = cstr(id)?;
    if unsafe { (api.run_util)(id.as_ptr()) } == 0 {
        Ok(())
    } else {
        Err(last_error(api))
    }
}

pub fn run_autocheck_search() -> Result<(), String> {
    call_action(api()?.run_autocheck_search)
}

pub fn open_recycle_bin() -> Result<(), String> {
    call_action(api()?.open_recycle)
}

pub fn open_telegram() -> Result<(), String> {
    call_action(api()?.open_telegram)
}

pub fn open_holycheck() -> Result<(), String> {
    call_action(api()?.open_holycheck)
}

pub fn run_system_info() -> Result<(), String> {
    call_action(api()?.run_system_info)
}

pub fn clear_minecraft_logs() -> Result<(), String> {
    call_action(api()?.clear_logs)
}

struct PhaseCtx<'a> {
    cb: &'a mut dyn FnMut(ScanPhase),
}

unsafe extern "C" fn phase_trampoline(phase: c_int, user: *mut c_void) {
    let ctx = &mut *(user as *mut PhaseCtx);
    (ctx.cb)(ScanPhase::from_i32(phase));
}

pub fn perform_scan(mut on_phase: impl FnMut(ScanPhase)) -> Result<Vec<String>, String> {
    let api = api()?;
    let mut ctx = PhaseCtx {
        cb: &mut on_phase,
    };
    let mut lines: *mut c_char = std::ptr::null_mut();
    let code = unsafe {
        (api.perform_scan)(phase_trampoline, &mut ctx as *mut PhaseCtx as *mut c_void, &mut lines)
    };
    if code != 0 {
        return Err(last_error(api));
    }
    let text = take_string(api, lines);
    Ok(text
        .split('\n')
        .map(str::trim_end)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect())
}

pub enum DownloadEvent {
    Progress(crate::download::ToolProgress),
    Ready,
    Failed(String),
}

struct ProgressCtx<'a> {
    cb: &'a mut dyn FnMut(&str, DownloadEvent),
}

unsafe extern "C" fn progress_trampoline(
    id: *const c_char,
    stage: c_int,
    received: i64,
    total: i64,
    err: *const c_char,
    user: *mut c_void,
) {
    let ctx = &mut *(user as *mut ProgressCtx);
    let id = CStr::from_ptr(id).to_string_lossy();
    let event = match stage {
        0 => DownloadEvent::Progress(crate::download::ToolProgress::Connecting),
        1 => DownloadEvent::Progress(crate::download::ToolProgress::Receiving {
            received: received.max(0) as u64,
            total: if total < 0 { None } else { Some(total as u64) },
        }),
        2 => DownloadEvent::Progress(crate::download::ToolProgress::Verifying),
        3 => DownloadEvent::Progress(crate::download::ToolProgress::Extracting),
        4 => DownloadEvent::Ready,
        _ => DownloadEvent::Failed(if err.is_null() {
            "Ошибка загрузки".into()
        } else {
            CStr::from_ptr(err).to_string_lossy().into_owned()
        }),
    };
    (ctx.cb)(&id, event);
}

pub fn download_tools(
    ids: &[String],
    force: bool,
    mut on_event: impl FnMut(&str, DownloadEvent),
) -> Result<(), String> {
    let api = api()?;
    let ids = cstr(&ids.join(","))?;
    let mut ctx = ProgressCtx {
        cb: &mut on_event,
    };
    let code = unsafe {
        (api.download_tools)(
            ids.as_ptr(),
            i32::from(force),
            progress_trampoline,
            &mut ctx as *mut ProgressCtx as *mut c_void,
        )
    };
    if code == 0 {
        Ok(())
    } else {
        Err(last_error(api))
    }
}

pub fn save_report(findings: &[String]) -> Result<String, String> {
    let api = api()?;
    let text = cstr(&findings.join("\n"))?;
    let mut path: *mut c_char = std::ptr::null_mut();
    if unsafe { (api.save_report)(text.as_ptr(), &mut path) } != 0 {
        return Err(last_error(api));
    }
    Ok(take_string(api, path))
}

pub fn user_name() -> String {
    match api() {
        Ok(host) => call_string(host.user_name).unwrap_or_else(|_| "User".into()),
        Err(_) => "User".into(),
    }
}

pub fn computer_name() -> String {
    match api() {
        Ok(host) => call_string(host.computer_name).unwrap_or_else(|_| "PC".into()),
        Err(_) => "PC".into(),
    }
}

pub fn install_date() -> String {
    match api() {
        Ok(host) => call_string(host.install_date).unwrap_or_else(|_| "Не удалось определить".into()),
        Err(_) => "Не удалось определить".into(),
    }
}

pub fn recycle_mtime() -> String {
    match api() {
        Ok(host) => call_string(host.recycle_mtime).unwrap_or_else(|_| "Не удалось определить".into()),
        Err(_) => "Не удалось определить".into(),
    }
}

pub fn os_info_label() -> String {
    match api() {
        Ok(host) => call_string(host.os_info_label).unwrap_or_else(|_| "Система".into()),
        Err(_) => "Система".into(),
    }
}
