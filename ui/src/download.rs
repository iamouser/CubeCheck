#[derive(Debug, Clone)]
pub enum ToolProgress {
    Connecting,
    Receiving { received: u64, total: Option<u64> },
    Verifying,
    Extracting,
}

pub fn downloads_enabled() -> bool {
    crate::backend::downloads_enabled()
}

pub fn missing_tool_ids() -> Result<Vec<String>, String> {
    crate::backend::missing_tool_ids()
}
