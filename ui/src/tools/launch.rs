pub fn run_autocheck_search() -> Result<(), String> {
    crate::backend::run_autocheck_search()
}

pub fn run_system_info() -> Result<(), String> {
    crate::backend::run_system_info()
}

pub fn open_recycle_bin() -> Result<(), String> {
    crate::backend::open_recycle_bin()
}

pub fn open_holycheck() {
    if let Err(e) = crate::backend::open_holycheck() {
        eprintln!("{e}");
    }
}

pub fn open_telegram() {
    if let Err(e) = crate::backend::open_telegram() {
        eprintln!("{e}");
    }
}

pub fn clear_minecraft_logs() -> Result<(), String> {
    crate::backend::clear_minecraft_logs()
}

pub fn run_util_id(key: &str) -> Result<(), String> {
    match crate::backend::run_util(key) {
        Ok(()) => Ok(()),
        Err(api_err) => {
            #[cfg(not(windows))]
            {
                match super::posix::run_bundled_util(key) {
                    Ok(()) => Ok(()),
                    Err(e) => Err(if crate::backend::api_loaded() {
                        api_err
                    } else {
                        e
                    }),
                }
            }
            #[cfg(windows)]
            {
                Err(api_err)
            }
        }
    }
}
