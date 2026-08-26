pub const CHEAT_NAMES: &[&str] = &[
    "impact", "wurst", "bleachhack", "aristois", "huzuni", "skillclient", "inertia", "ares", "sigma",
    "meteor", "liquidbounce", "nurik", "nursultan", "celestial", "calestial", "celka", "expensive",
    "neverhook", "excellent", "wexside", "wildclient", "minced", "deadcode", "akrien", "jigsaw",
    "jessica", "dreampool", "norules", "konas", "richclient", "rusherhack", "thunderhack",
    "moonhack", "doomsday", "nightware", "ricardo", "extazyy", "troxill", "antileak", "arbuz", ".akr",
    ".wex", "dauntiblyat", "rename_me_please", "editme", "takker", "fuzeclient", "wisefolder", "flauncher",
    "vec.dll", "USBOblivion.exe", "Feather", "venus", "baritone", "spambot", "CleanCut",
    "spam_bot", "inventory_walk", "player_highlighter", "aimbot", "freecam", "bedrock_breaker_mode",
    "viaversion", "double_hotbar", "elytra_swap", "armor_hotswap", "smart_moving", "savesearcher",
    "topkautobuy", "topkaautobuy", "tweakeroo", "mob_hitbox", "librarian_trade_finder", "sacurachorusfind",
    "autoattack", "entity_outliner", "invmove", "viabackwards", "viarewind", "viafabric", "viaforge",
    "viaproxy", "vialoader", "viamcp", "hitbox", "elytrahack", "DiamondSim", "ForgeHax", "clientcommands",
    "Control-Tweaks", "SwingThroughGrass", "CutThrough", "Haruka", "NewLauncher", "Blade", "Hachclient",
    "Fluger", "Exloader", "CatLean", "cproject", "eternity", "melonity", "relake", "rockstar", "verist",
    "zamorozka", "phobos", "pyro", "novoline", "vape", "astolfo", "koid", "nix", "spirt", "salhack",
    "gamesense",
];

pub const IGNORED_PROCESSES: &[&str] = &[
    "svchost.exe", "winlogon.exe", "csrss.exe", "services.exe", "lsass.exe", "wininit.exe", "spoolsv.exe",
    "taskhostw.exe", "dwm.exe", "ctfmon.exe", "smss.exe", "system", "system idle process", "chrome.exe",
    "firefox.exe", "msedge.exe", "opera.exe", "discord.exe", "telegram.exe", "skype.exe", "whatsapp.exe",
    "steam.exe", "epicgameslauncher.exe", "origin.exe", "battle.net.exe", "winword.exe", "excel.exe",
    "powerpnt.exe", "outlook.exe", "explorer.exe", "notepad.exe", "calc.exe", "cmd.exe", "powershell.exe",
    "conhost.exe", "taskmgr.exe", "regedit.exe", "msinfo32.exe", "cubecheck.exe",
];

pub const LOG_SUSPICIOUS: &[&str] = &[
    "inject", "injection", "hook", "hooked", "bypass", "cheat", "hack", "loader", "fatal", "hitbox",
    "expand", "reach", "aimbot", "killaura", "autoclick", "fly", "speed",
];

pub const INJECTION_KEYWORDS: &[&str] = &[
    "inject", "injection", "hooked", "hook", "dll", "loader", "native", "jni", "agent", "attached",
    "transform", "classloader", "modify", "bytecode", "asm",
];

pub const HITBOX_FILES: &[&str] = &[
    "hitbox", "expand", "reach", "aimbot", "killaura", "autoclick", "fly", "speed",
];

pub const KNOWN_DLLS: &[&str] = &["lwjgl", "jinput", "openal", "javaw", "msvcp", "vcruntime"];

/// Everything OR-search separator: spaces around `|` so the query is readable.
pub const EVERYTHING_OR_SEP: &str = " | ";

#[cfg_attr(windows, allow(dead_code))]
const REGEX_META: &str = r".+*?()[]{}|^$\";

pub fn cheat_list_text() -> String {
    #[cfg(windows)]
    {
        CHEAT_NAMES.join(EVERYTHING_OR_SEP)
    }
    #[cfg(target_os = "macos")]
    {
        mdfind_or_query(CHEAT_NAMES)
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        fsearch_or_query(CHEAT_NAMES)
    }
}

pub fn everything_search_query(terms: &[&str]) -> String {
    format!("({})", terms.join(EVERYTHING_OR_SEP))
}

/// FSearch default syntax: `OR` / `||` between terms (not Everything's `|`).
#[cfg_attr(windows, allow(dead_code))]
pub fn fsearch_or_query(terms: &[&str]) -> String {
    format!("({})", terms.join(" OR "))
}

/// Spotlight / mdfind filename OR query (case-insensitive).
#[cfg_attr(windows, allow(dead_code))]
pub fn mdfind_or_query(terms: &[&str]) -> String {
    terms
        .iter()
        .map(|t| {
            let safe = t.replace(['*', '"', '\''], "");
            format!("kMDItemDisplayName == '*{safe}*'c")
        })
        .collect::<Vec<_>>()
        .join(" || ")
}

/// POSIX regex OR for plocate/locate/find (`|`, with metacharacters escaped).
#[cfg_attr(windows, allow(dead_code))]
pub fn posix_regex_or_query(terms: &[&str]) -> String {
    terms.iter().map(|t| regex_escape(t)).collect::<Vec<_>>().join("|")
}

#[cfg_attr(windows, allow(dead_code))]
fn regex_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if REGEX_META.contains(c) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn everything_join_has_spaces_around_pipe() {
        assert_eq!(EVERYTHING_OR_SEP, " | ");
        assert_eq!(
            everything_search_query(&["impact", "wurst"]),
            "(impact | wurst)"
        );
    }

    #[test]
    fn fsearch_uses_or_keyword_not_pipe() {
        assert_eq!(fsearch_or_query(&["impact", "wurst"]), "(impact OR wurst)");
        assert!(!fsearch_or_query(&["impact", "wurst"]).contains(" | "));
    }

    #[test]
    fn posix_regex_escapes_dots_and_ors() {
        assert_eq!(posix_regex_or_query(&["vec.dll", "wurst"]), r"vec\.dll|wurst");
    }

    #[test]
    fn mdfind_joins_with_spotlight_or() {
        let q = mdfind_or_query(&["impact", "wurst"]);
        assert!(q.contains("||"));
        assert!(q.contains("kMDItemDisplayName == '*impact*'c"));
        assert!(!q.contains("Everything"));
    }
}
