use super::launch::run_util_id;

#[derive(Clone, Copy, Debug)]
pub struct Util {
    pub id: &'static str,
    pub name: &'static str,
    pub desc: &'static str,
}

impl Util {
    pub fn is_search(self) -> bool {
        self.id == "everything" || self.id == "search"
    }

    pub fn is_in_app(self) -> bool {
        #[cfg(windows)]
        {
            let _ = self;
            false
        }
        #[cfg(not(windows))]
        {
            matches!(self.id, "autoruns" | "recent")
        }
    }
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct InspectRow {
    pub title: String,
    pub detail: String,
    pub path: Option<std::path::PathBuf>,
}

#[cfg(windows)]
pub const UTILS: &[Util] = &[
    Util {
        id: "everything",
        name: "Everything",
        desc: "Поиск файлов на компьютере.",
    },
    Util {
        id: "shellbag",
        name: "Shellbag Analyzer",
        desc: "Какие папки открывали в проводнике.",
    },
    Util {
        id: "systeminformer",
        name: "System Informer",
        desc: "Список запущенных программ.",
    },
    Util {
        id: "procmon",
        name: "Process Monitor",
        desc: "Что программы делают прямо сейчас.",
    },
    Util {
        id: "autoruns",
        name: "Autoruns",
        desc: "Что запускается вместе с Windows.",
    },
    Util {
        id: "procexp",
        name: "Process Explorer",
        desc: "Подробности о процессах.",
    },
];

#[cfg(target_os = "linux")]
pub const UTILS: &[Util] = &[
    Util {
        id: "search",
        name: "Поиск файлов",
        desc: "FSearch / Catfish / plocate — поиск имён на диске (аналог Everything).",
    },
    Util {
        id: "recent",
        name: "Недавние файлы",
        desc: "Недавно открытые файлы и папки (аналог Shellbag Analyzer).",
    },
    Util {
        id: "processes",
        name: "Процессы",
        desc: "Mission Center / монитор системы (аналог System Informer).",
    },
    Util {
        id: "activity",
        name: "Активность файлов",
        desc: "sysdig — что программы делают с файлами (аналог Process Monitor).",
    },
    Util {
        id: "autoruns",
        name: "Автозагрузка",
        desc: "systemd и ~/.config/autostart (аналог Autoruns).",
    },
    Util {
        id: "procexp",
        name: "Дерево процессов",
        desc: "GNOME System Monitor / htop (аналог Process Explorer).",
    },
];

#[cfg(target_os = "macos")]
pub const UTILS: &[Util] = &[
    Util {
        id: "search",
        name: "Поиск файлов",
        desc: "Spotlight (mdfind) — поиск имён на диске (аналог Everything).",
    },
    Util {
        id: "recent",
        name: "Недавние файлы",
        desc: "Недавно использованные файлы (аналог Shellbag Analyzer).",
    },
    Util {
        id: "processes",
        name: "Процессы",
        desc: "Activity Monitor (аналог System Informer).",
    },
    Util {
        id: "activity",
        name: "Активность файлов",
        desc: "fs_usage в Терминале (аналог Process Monitor).",
    },
    Util {
        id: "autoruns",
        name: "Автозагрузка",
        desc: "Login Items и LaunchAgents (аналог Autoruns).",
    },
    Util {
        id: "procexp",
        name: "Дерево процессов",
        desc: "Activity Monitor — подробности о процессах.",
    },
];

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
pub const UTILS: &[Util] = &[
    Util {
        id: "search",
        name: "Поиск файлов",
        desc: "plocate / find — поиск имён на диске.",
    },
    Util {
        id: "recent",
        name: "Недавние файлы",
        desc: "Недавно открытые файлы.",
    },
    Util {
        id: "processes",
        name: "Процессы",
        desc: "Монитор системы / htop.",
    },
    Util {
        id: "activity",
        name: "Активность файлов",
        desc: "lsof — открытые файлы.",
    },
    Util {
        id: "autoruns",
        name: "Автозагрузка",
        desc: "Точки автозапуска сессии.",
    },
    Util {
        id: "procexp",
        name: "Дерево процессов",
        desc: "htop — подробности о процессах.",
    },
];

pub fn run_util(key: &str) -> Result<(), String> {
    run_util_id(key)
}

pub fn util_index(id: &str) -> Option<usize> {
    UTILS.iter().position(|u| u.id == id)
}

#[cfg_attr(windows, allow(dead_code))]
pub fn load_inspect(id: &str) -> Vec<InspectRow> {
    #[cfg(not(windows))]
    {
        super::posix::load_inspect(id)
    }
    #[cfg(windows)]
    {
        let _ = id;
        Vec::new()
    }
}

pub fn autocheck_search_status_line() -> &'static str {
    if cfg!(windows) {
        "Everything открыт с поиском по читам."
    } else if cfg!(target_os = "macos") {
        "Spotlight (mdfind) запущен с поиском по именам читов."
    } else {
        "Поиск по именам читов запущен (FSearch / plocate)."
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_has_six_roles() {
        assert_eq!(UTILS.len(), 6);
        assert!(UTILS.iter().any(|u| u.is_search()));
        assert!(UTILS.iter().any(|u| u.id == "autoruns"));
        assert!(UTILS.iter().any(|u| u.id == "procexp"));
    }

    #[cfg(windows)]
    #[test]
    fn windows_ids_unchanged() {
        let ids: Vec<_> = UTILS.iter().map(|u| u.id).collect();
        assert_eq!(
            ids,
            [
                "everything",
                "shellbag",
                "systeminformer",
                "procmon",
                "autoruns",
                "procexp"
            ]
        );
        assert!(!UTILS.iter().any(|u| u.is_in_app()));
    }

    #[cfg(not(windows))]
    #[test]
    fn posix_catalog_has_search_not_everything() {
        assert!(UTILS.iter().any(|u| u.id == "search"));
        assert!(!UTILS.iter().any(|u| u.id == "everything"));
        assert!(UTILS.iter().any(|u| u.id == "recent" && u.is_in_app()));
        assert!(UTILS.iter().any(|u| u.id == "autoruns" && u.is_in_app()));
    }
}
