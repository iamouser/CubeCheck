use std::path::{Path, PathBuf};

#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;

#[cfg(windows)]
fn wide(s: impl AsRef<std::ffi::OsStr>) -> Vec<u16> {
    s.as_ref().encode_wide().chain(Some(0)).collect()
}

pub fn shell_execute(path: &Path, dir: &Path, verb: &str) -> Result<(), String> {
    shell_execute_ex(path, dir, verb, None)
}

pub fn shell_execute_ex(
    path: &Path,
    dir: &Path,
    verb: &str,
    params: Option<&str>,
) -> Result<(), String> {
    #[cfg(windows)]
    {
        use windows::core::PCWSTR;
        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::Shell::ShellExecuteW;
        use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

        let op = wide(verb);
        let file = wide(path.as_os_str());
        let directory = wide(dir.as_os_str());
        let params_w = params.map(wide);
        let code = unsafe {
            ShellExecuteW(
                HWND::default(),
                PCWSTR(op.as_ptr()),
                PCWSTR(file.as_ptr()),
                params_w
                    .as_ref()
                    .map(|p| PCWSTR(p.as_ptr()))
                    .unwrap_or_else(PCWSTR::null),
                PCWSTR(directory.as_ptr()),
                SW_SHOWNORMAL,
            )
        };
        if (code.0 as isize) <= 32 {
            return Err(format!(
                "Не удалось запустить {} (код: {})",
                path.display(),
                code.0 as isize
            ));
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = (path, dir, verb, params);
        Err("Запуск поддерживается только в Windows".into())
    }
}

pub fn message_box(title: &str, text: &str) {
    #[cfg(windows)]
    {
        use windows::core::PCWSTR;
        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_OK};

        let t = wide(title);
        let m = wide(text);
        unsafe {
            let _ = MessageBoxW(HWND::default(), PCWSTR(m.as_ptr()), PCWSTR(t.as_ptr()), MB_OK);
        }
    }
    #[cfg(not(windows))]
    {
        println!("{title}\n{text}");
    }
}

pub fn is_elevated() -> bool {
    #[cfg(windows)]
    {
        use windows::Win32::UI::Shell::IsUserAnAdmin;
        unsafe { IsUserAnAdmin().as_bool() }
    }
    #[cfg(not(windows))]
    {
        false
    }
}

pub fn relaunch_as_admin() -> Result<(), String> {
    relaunch_as_admin_args(&[])
}

pub fn relaunch_as_admin_args(args: &[&str]) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let dir = exe.parent().unwrap_or(Path::new("."));
    let params = join_args(args);
    let params_opt = if params.is_empty() {
        None
    } else {
        Some(params.as_str())
    };
    shell_execute_ex(&exe, dir, "runas", params_opt)
}

fn join_args(args: &[&str]) -> String {
    args.iter()
        .map(|a| {
            if a.is_empty() || a.contains(' ') || a.contains('"') {
                format!("\"{}\"", a.replace('"', "\\\""))
            } else {
                (*a).to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn remove_app_shortcuts() {
    let mut links = shortcut_link_paths();
    #[cfg(windows)]
    {
        if let Some(dir) = known_folder(&windows::Win32::UI::Shell::FOLDERID_PublicDesktop) {
            links.push(dir.join("CubeCheck.lnk"));
        }
    }
    for link in links {
        let _ = std::fs::remove_file(link);
    }
}

pub fn install_shortcuts(target: &Path, work_dir: &Path, icon: Option<&Path>) -> Result<(), String> {
    let mut ok = 0usize;
    let mut last_err = None;
    for link in shortcut_link_paths() {
        match create_shortcut(&link, target, work_dir, icon) {
            Ok(()) => ok += 1,
            Err(e) => last_err = Some(format!("{}: {e}", link.display())),
        }
    }
    if ok == 0 {
        Err(last_err.unwrap_or_else(|| "Не удалось создать ярлык на рабочем столе".into()))
    } else {
        Ok(())
    }
}

fn shortcut_link_paths() -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut push_dir = |dir: PathBuf| {
        if dir.is_dir() {
            let link = dir.join("CubeCheck.lnk");
            if !out.contains(&link) {
                out.push(link);
            }
        }
    };

    #[cfg(windows)]
    {
        if let Some(dir) = known_folder(&windows::Win32::UI::Shell::FOLDERID_Desktop) {
            push_dir(dir);
        } else if let Ok(home) = std::env::var("USERPROFILE") {
            push_dir(PathBuf::from(home).join("Desktop"));
        }
        if let Some(dir) = known_folder(&windows::Win32::UI::Shell::FOLDERID_Programs) {
            push_dir(dir);
        } else if let Ok(appdata) = std::env::var("APPDATA") {
            push_dir(PathBuf::from(appdata).join(r"Microsoft\Windows\Start Menu\Programs"));
        }
    }

    #[cfg(not(windows))]
    {
        if let Ok(home) = std::env::var("USERPROFILE") {
            push_dir(PathBuf::from(home).join("Desktop"));
        }
    }

    out
}

#[cfg(windows)]
fn known_folder(id: &windows::core::GUID) -> Option<PathBuf> {
    use windows::Win32::System::Com::CoTaskMemFree;
    use windows::Win32::UI::Shell::{KF_FLAG_DEFAULT, SHGetKnownFolderPath};

    unsafe {
        let pw = SHGetKnownFolderPath(id, KF_FLAG_DEFAULT, None).ok()?;
        let path = pw.to_string().ok().map(PathBuf::from);
        CoTaskMemFree(Some(pw.as_ptr() as *const _));
        path
    }
}

pub fn create_shortcut(
    link_path: &Path,
    target: &Path,
    work_dir: &Path,
    icon: Option<&Path>,
) -> Result<(), String> {
    #[cfg(windows)]
    {
        use windows::core::{Interface, PCWSTR};
        use windows::Win32::Foundation::TRUE;
        use windows::Win32::System::Com::{
            CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED,
            IPersistFile,
        };
        use windows::Win32::UI::Shell::{IShellLinkW, ShellLink};

        if let Some(parent) = link_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Не удалось создать папку ярлыка: {e}"))?;
        }

        let target_w = wide(target.as_os_str());
        let dir_w = wide(work_dir.as_os_str());
        let desc_w = wide("CubeCheck");
        let link_w = wide(link_path.as_os_str());
        let icon_w = icon.map(|p| wide(p.as_os_str()));

        unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            let link: IShellLinkW = CoCreateInstance(&ShellLink, None, CLSCTX_INPROC_SERVER)
                .map_err(|e| format!("IShellLink: {e}"))?;
            link.SetPath(PCWSTR(target_w.as_ptr()))
                .map_err(|e| format!("SetPath: {e}"))?;
            link.SetWorkingDirectory(PCWSTR(dir_w.as_ptr()))
                .map_err(|e| format!("SetWorkingDirectory: {e}"))?;
            if let Some(icon_w) = icon_w.as_ref() {
                link.SetIconLocation(PCWSTR(icon_w.as_ptr()), 0)
                    .map_err(|e| format!("SetIconLocation: {e}"))?;
            }
            link.SetDescription(PCWSTR(desc_w.as_ptr()))
                .map_err(|e| format!("SetDescription: {e}"))?;
            let persist: IPersistFile = link.cast().map_err(|e| format!("IPersistFile: {e}"))?;
            persist
                .Save(PCWSTR(link_w.as_ptr()), TRUE)
                .map_err(|e| format!("Save shortcut: {e}"))?;
        }
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = (link_path, target, work_dir, icon);
        Err("Ярлыки поддерживаются только в Windows".into())
    }
}
