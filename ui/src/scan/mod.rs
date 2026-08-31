use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ScanPhase {
    Processes,
    Files,
    Registry,
    Logs,
}

impl ScanPhase {
    pub fn from_i32(value: i32) -> Self {
        match value {
            1 => Self::Files,
            2 => Self::Registry,
            3 => Self::Logs,
            _ => Self::Processes,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Processes => "процессы",
            Self::Files => "файлы",
            Self::Registry => {
                if cfg!(windows) {
                    "реестр"
                } else {
                    "автозагрузка"
                }
            }
            Self::Logs => "логи",
        }
    }
}

pub enum ScanState {
    Idle,
    Running(ScanPhase),
    Done(Vec<String>),
}

pub fn perform_scan(state: Arc<Mutex<ScanState>>) -> Vec<String> {
    match crate::backend::perform_scan(|phase| {
        if let Ok(mut guard) = state.lock() {
            *guard = ScanState::Running(phase);
        }
    }) {
        Ok(lines) => lines,
        Err(e) => vec![e],
    }
}

pub fn cheat_list_text() -> String {
    crate::backend::cheat_list_text()
}
