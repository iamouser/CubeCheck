pub mod catalog;
pub mod launch;
pub mod paths;

#[cfg(not(windows))]
pub mod posix;

pub use catalog::{run_util, InspectRow, UTILS};
#[cfg(not(windows))]
pub use catalog::Util;
pub use launch::{
    clear_minecraft_logs, open_holycheck, open_recycle_bin, open_telegram, run_autocheck_search,
    run_system_info,
};
#[cfg(not(windows))]
pub use launch::open_path;
