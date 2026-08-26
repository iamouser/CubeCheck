mod cheat_list;
mod engine;

pub use cheat_list::{cheat_list_text, everything_search_query, CHEAT_NAMES};
#[cfg(unix)]
pub use cheat_list::posix_regex_or_query;
#[cfg(all(unix, not(target_os = "macos")))]
pub use cheat_list::fsearch_or_query;
#[cfg(target_os = "macos")]
pub use cheat_list::mdfind_or_query;
pub use engine::{perform_scan, ScanPhase, ScanState};
