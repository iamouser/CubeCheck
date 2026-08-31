pub fn computer_name() -> String {
    crate::backend::computer_name()
}

pub fn user_name() -> String {
    crate::backend::user_name()
}

pub fn os_info_label() -> String {
    crate::backend::os_info_label()
}

pub fn windows_install_date() -> String {
    crate::backend::install_date()
}

pub fn recycle_bin_last_change() -> String {
    crate::backend::recycle_mtime()
}
