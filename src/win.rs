use std::path::{Path, PathBuf};

#[cfg(windows)]
use std::os::windows::ffi::OsStrExt;

#[cfg(windows)]
fn wide(s: impl AsRef<std::ffi::OsStr>) -> Vec<u16> {
    s.as_ref().encode_wide().chain(Some(0)).collect()
}

#[cfg(all(windows, feature = "gui"))]
fn invalid_hwnd() -> windows::Win32::Foundation::HWND {
    windows::Win32::Foundation::HWND(-1isize as *mut core::ffi::c_void)
}

pub fn shell_execute(path: &Path, dir: &Path, verb: &str) -> Result<(), String> {
    shell_execute_params(path, dir, verb, "")
}

pub fn shell_execute_params(path: &Path, dir: &Path, verb: &str, params: &str) -> Result<(), String> {
    #[cfg(windows)]
    {
        use windows::core::PCWSTR;
        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::Shell::ShellExecuteW;
        use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

        let op = wide(verb);
        let file = wide(path.as_os_str());
        let directory = wide(dir.as_os_str());
        let args = wide(params);

        let code = unsafe {
            ShellExecuteW(
                HWND::default(),
                PCWSTR(op.as_ptr()),
                PCWSTR(file.as_ptr()),
                if params.is_empty() {
                    PCWSTR::null()
                } else {
                    PCWSTR(args.as_ptr())
                },
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

#[allow(dead_code)]
pub fn message_box(title: &str, text: &str) {
    #[cfg(windows)]
    {
        use windows::core::PCWSTR;
        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_OK};

        let t = wide(title);
        let m = wide(text);
        unsafe {
            let _ = MessageBoxW(
                HWND::default(),
                PCWSTR(m.as_ptr()),
                PCWSTR(t.as_ptr()),
                MB_OK,
            );
        }
    }

    #[cfg(not(windows))]
    {
        println!("{title}\n{text}");
    }
}

#[allow(dead_code)] // used by the GUI; setup shares this module
pub fn is_pe_amd64(path: &Path) -> bool {
    let Ok(data) = std::fs::read(path) else {
        return false;
    };
    if data.len() < 0x40 || data[0] != b'M' || data[1] != b'Z' {
        return false;
    }
    let pe = u32::from_le_bytes(data[0x3C..0x40].try_into().unwrap_or_default()) as usize;
    data.get(pe..pe + 6).is_some_and(|s| {
        s[0] == b'P' && s[1] == b'E' && s[2] == 0 && s[3] == 0 && s[4] == 0x64 && s[5] == 0x86
    })
}

#[allow(dead_code)]
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

#[allow(dead_code)]
pub fn relaunch_as_admin() -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let dir = exe.parent().unwrap_or(Path::new("."));
    shell_execute(&exe, dir, "runas")
}

#[allow(dead_code)]
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
            push_dir(
                PathBuf::from(appdata).join(r"Microsoft\Windows\Start Menu\Programs"),
            );
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
    use windows::Win32::UI::Shell::{SHGetKnownFolderPath, KF_FLAG_DEFAULT};

    unsafe {
        let pw = SHGetKnownFolderPath(id, KF_FLAG_DEFAULT, None).ok()?;
        let path = pw.to_string().ok().map(PathBuf::from);
        CoTaskMemFree(Some(pw.as_ptr() as *const _));
        path
    }
}

#[allow(dead_code)] // used by the thin installer
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

#[cfg(feature = "gui")]
pub fn verify_authenticode_publisher(path: &Path, expected: &str) -> Result<(), String> {
    #[cfg(windows)]
    {
        use windows::Win32::Security::WinTrust::{
            WinVerifyTrust, WINTRUST_ACTION_GENERIC_VERIFY_V2, WINTRUST_DATA, WINTRUST_FILE_INFO,
            WTD_CHOICE_FILE, WTD_REVOKE_NONE, WTD_STATEACTION_CLOSE, WTD_STATEACTION_VERIFY,
            WTD_UI_NONE,
        };

        if !path.exists() {
            return Err(format!("Файл не найден: {}", path.display()));
        }

        let path_w = wide(path.as_os_str());
        let mut file_info = WINTRUST_FILE_INFO {
            cbStruct: std::mem::size_of::<WINTRUST_FILE_INFO>() as u32,
            pcwszFilePath: windows::core::PCWSTR(path_w.as_ptr()),
            hFile: windows::Win32::Foundation::HANDLE::default(),
            pgKnownSubject: std::ptr::null_mut(),
        };

        let mut data = WINTRUST_DATA {
            cbStruct: std::mem::size_of::<WINTRUST_DATA>() as u32,
            dwUIChoice: WTD_UI_NONE,
            fdwRevocationChecks: WTD_REVOKE_NONE,
            dwUnionChoice: WTD_CHOICE_FILE,
            Anonymous: windows::Win32::Security::WinTrust::WINTRUST_DATA_0 {
                pFile: &mut file_info,
            },
            dwStateAction: WTD_STATEACTION_VERIFY,
            ..Default::default()
        };

        let mut policy = WINTRUST_ACTION_GENERIC_VERIFY_V2;
        let status = unsafe {
            WinVerifyTrust(invalid_hwnd(), &mut policy, &mut data as *mut _ as *mut _)
        };

        data.dwStateAction = WTD_STATEACTION_CLOSE;
        unsafe {
            let _ = WinVerifyTrust(invalid_hwnd(), &mut policy, &mut data as *mut _ as *mut _);
        }

        if status != 0 {
            return Err(format!(
                "Файл не прошёл проверку: {}",
                path.file_name().and_then(|n| n.to_str()).unwrap_or("файл")
            ));
        }

        let names = signature_display_names(path);
        if names.is_empty() {
            return Err("Не удалось проверить подпись файла".into());
        }

        let expected_l = expected.to_ascii_lowercase();
        if names
            .iter()
            .any(|name| name.to_ascii_lowercase().contains(&expected_l))
        {
            return Ok(());
        }
        Err(format!(
            "Подпись не совпала (нужен «{expected}»)"
        ))
    }

    #[cfg(not(windows))]
    {
        let _ = (path, expected);
        Err("Проверка подписи доступна только в Windows".into())
    }
}

#[cfg(all(windows, feature = "gui"))]
fn signature_display_names(path: &Path) -> Vec<String> {
    use std::ffi::c_void;
    use windows::Win32::Security::Cryptography::{
        CertCloseStore, CertEnumCertificatesInStore, CertGetNameStringW,
        CryptMsgClose, CryptQueryObject, CERT_NAME_SIMPLE_DISPLAY_TYPE,
        CERT_QUERY_CONTENT_FLAG_PKCS7_SIGNED_EMBED, CERT_QUERY_CONTENT_TYPE,
        CERT_QUERY_ENCODING_TYPE, CERT_QUERY_FORMAT_FLAG_BINARY, CERT_QUERY_FORMAT_TYPE,
        CERT_QUERY_OBJECT_FILE, HCERTSTORE,
    };

    let path_w = wide(path.as_os_str());
    let mut encoding = CERT_QUERY_ENCODING_TYPE(0);
    let mut content_type = CERT_QUERY_CONTENT_TYPE(0);
    let mut format_type = CERT_QUERY_FORMAT_TYPE(0);
    let mut store = HCERTSTORE::default();
    let mut msg: *mut c_void = std::ptr::null_mut();

    let ok = unsafe {
        CryptQueryObject(
            CERT_QUERY_OBJECT_FILE,
            path_w.as_ptr() as *const c_void,
            CERT_QUERY_CONTENT_FLAG_PKCS7_SIGNED_EMBED,
            CERT_QUERY_FORMAT_FLAG_BINARY,
            0,
            Some(&mut encoding as *mut _),
            Some(&mut content_type as *mut _),
            Some(&mut format_type as *mut _),
            Some(&mut store as *mut _),
            Some(&mut msg as *mut _),
            None,
        )
    };
    if ok.is_err() {
        return Vec::new();
    }

    unsafe {
        let mut names = Vec::new();
        let mut cert = CertEnumCertificatesInStore(store, None);
        while !cert.is_null() {
            let mut buf = [0u16; 256];
            let n = CertGetNameStringW(
                cert,
                CERT_NAME_SIMPLE_DISPLAY_TYPE,
                0,
                None,
                Some(&mut buf),
            );
            if n > 1 {
                let name = String::from_utf16_lossy(&buf[..n as usize - 1]);
                if !name.is_empty() && !names.iter().any(|n: &String| n.eq_ignore_ascii_case(&name))
                {
                    names.push(name);
                }
            }
            cert = CertEnumCertificatesInStore(store, Some(cert));
        }
        let _ = CertCloseStore(store, 0);
        if !msg.is_null() {
            let _ = CryptMsgClose(Some(msg));
        }
        names
    }
}
