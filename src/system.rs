pub fn computer_name() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "PC".into())
}

pub fn user_name() -> String {
    std::env::var("USERNAME")
        .or_else(|_| std::env::var("USER"))
        .unwrap_or_else(|_| "User".into())
}

pub fn os_info_label() -> &'static str {
    if cfg!(windows) {
        "Дата установки Windows"
    } else {
        "Система"
    }
}

pub fn windows_install_date() -> String {
    #[cfg(windows)]
    {
        use winreg::enums::*;
        use winreg::RegKey;

        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        return match hklm.open_subkey(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion") {
            Ok(key) => match key.get_value::<u32, _>("InstallDate") {
                Ok(ts) => {
                    if let Some(dt) = chrono::DateTime::from_timestamp(ts as i64, 0) {
                        return dt.format("%d.%m.%Y %H:%M:%S").to_string();
                    }
                    "Не удалось определить".into()
                }
                Err(_) => "Не удалось определить".into(),
            },
            Err(_) => "Не удалось определить".into(),
        };
    }
    #[cfg(target_os = "macos")]
    {
        let out = std::process::Command::new("sw_vers")
            .arg("-productVersion")
            .output();
        return match out {
            Ok(o) if o.status.success() => {
                format!("macOS {}", String::from_utf8_lossy(&o.stdout).trim())
            }
            _ => "macOS".into(),
        };
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        if let Ok(text) = std::fs::read_to_string("/etc/os-release") {
            for line in text.lines() {
                if let Some(v) = line.strip_prefix("PRETTY_NAME=") {
                    return v.trim_matches('"').to_string();
                }
            }
        }
        "Linux".into()
    }
}

pub fn recycle_bin_last_change() -> String {
    let path = recycle_path();
    match std::fs::metadata(&path) {
        Ok(meta) => match meta.modified() {
            Ok(t) => {
                let dt: chrono::DateTime<chrono::Local> = t.into();
                dt.format("%d.%m.%Y %H:%M:%S").to_string()
            }
            Err(e) => format!("Не удалось определить: {e}"),
        },
        Err(_) => "Папка корзины не найдена".into(),
    }
}

fn recycle_path() -> String {
    #[cfg(windows)]
    {
        let drive = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".into());
        return format!(r"{drive}\$Recycle.Bin");
    }
    #[cfg(target_os = "macos")]
    {
        let home = std::env::var("HOME").unwrap_or_default();
        return format!("{home}/.Trash");
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        let home = std::env::var("HOME").unwrap_or_default();
        format!("{home}/.local/share/Trash")
    }
}
