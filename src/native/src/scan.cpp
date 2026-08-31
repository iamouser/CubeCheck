#define NOMINMAX
#define WIN32_LEAN_AND_MEAN
#define UNICODE
#define _UNICODE
#ifndef CUBCHECK_NATIVE_EXPORTS
#define CUBCHECK_NATIVE_EXPORTS
#endif

#include "../include/cubecheck_native.h"

#include <windows.h>
#include <tlhelp32.h>
#include <shlobj.h>
#include <knownfolders.h>

#include <algorithm>
#include <cctype>
#include <cstring>
#include <fstream>
#include <functional>
#include <string>
#include <vector>

#pragma comment(lib, "advapi32.lib")
#pragma comment(lib, "shell32.lib")
#pragma comment(lib, "ole32.lib")

namespace {

const wchar_t* kCheatNames[] = {
    L"impact", L"wurst", L"bleachhack", L"aristois", L"huzuni", L"skillclient", L"inertia", L"ares", L"sigma",
    L"meteor", L"liquidbounce", L"nurik", L"nursultan", L"celestial", L"calestial", L"celka", L"expensive",
    L"neverhook", L"excellent", L"wexside", L"wildclient", L"minced", L"deadcode", L"akrien", L"jigsaw",
    L"jessica", L"dreampool", L"norules", L"konas", L"richclient", L"rusherhack", L"thunderhack",
    L"moonhack", L"doomsday", L"nightware", L"ricardo", L"extazyy", L"troxill", L"antileak", L"arbuz", L".akr",
    L".wex", L"dauntiblyat", L"rename_me_please", L"editme", L"takker", L"fuzeclient", L"wisefolder", L"flauncher",
    L"vec.dll", L"USBOblivion.exe", L"Feather", L"venus", L"baritone", L"spambot", L"CleanCut",
    L"spam_bot", L"inventory_walk", L"player_highlighter", L"aimbot", L"freecam", L"bedrock_breaker_mode",
    L"viaversion", L"double_hotbar", L"elytra_swap", L"armor_hotswap", L"smart_moving", L"savesearcher",
    L"topkautobuy", L"topkaautobuy", L"tweakeroo", L"mob_hitbox", L"librarian_trade_finder", L"sacurachorusfind",
    L"autoattack", L"entity_outliner", L"invmove", L"viabackwards", L"viarewind", L"viafabric", L"viaforge",
    L"viaproxy", L"vialoader", L"viamcp", L"hitbox", L"elytrahack", L"DiamondSim", L"ForgeHax", L"clientcommands",
    L"Control-Tweaks", L"SwingThroughGrass", L"CutThrough", L"Haruka", L"NewLauncher", L"Blade", L"Hachclient",
    L"Fluger", L"Exloader", L"CatLean", L"cproject", L"eternity", L"melonity", L"relake", L"rockstar", L"verist",
    L"zamorozka", L"phobos", L"pyro", L"novoline", L"vape", L"astolfo", L"koid", L"nix", L"spirt", L"salhack",
    L"gamesense"};

const wchar_t* kIgnored[] = {
    L"svchost.exe", L"winlogon.exe", L"csrss.exe", L"services.exe", L"lsass.exe", L"wininit.exe", L"spoolsv.exe",
    L"taskhostw.exe", L"dwm.exe", L"ctfmon.exe", L"smss.exe", L"system", L"system idle process", L"chrome.exe",
    L"firefox.exe", L"msedge.exe", L"opera.exe", L"discord.exe", L"telegram.exe", L"skype.exe", L"whatsapp.exe",
    L"steam.exe", L"epicgameslauncher.exe", L"origin.exe", L"battle.net.exe", L"winword.exe", L"excel.exe",
    L"powerpnt.exe", L"outlook.exe", L"explorer.exe", L"notepad.exe", L"calc.exe", L"cmd.exe", L"powershell.exe",
    L"conhost.exe", L"taskmgr.exe", L"regedit.exe", L"msinfo32.exe", L"cubecheck.exe"};

const char* kLogSuspicious[] = {
    "inject", "injection", "hook", "hooked", "bypass", "cheat", "hack", "loader", "fatal", "hitbox",
    "expand", "reach", "aimbot", "killaura", "autoclick", "fly", "speed"};

const char* kInject[] = {
    "inject", "injection", "hooked", "hook", "dll", "loader", "native", "jni", "agent", "attached",
    "transform", "classloader", "modify", "bytecode", "asm"};

const wchar_t* kHitbox[] = {
    L"hitbox", L"expand", L"reach", L"aimbot", L"killaura", L"autoclick", L"fly", L"speed"};

const wchar_t* kKnownDll[] = {L"lwjgl", L"jinput", L"openal", L"javaw", L"msvcp", L"vcruntime"};

std::wstring lower(std::wstring s) {
    for (auto& ch : s) {
        ch = static_cast<wchar_t>(towlower(ch));
    }
    return s;
}

bool contains_cheat(const std::wstring& text) {
    const std::wstring hay = lower(text);
    for (const wchar_t* pat : kCheatNames) {
        if (hay.find(lower(pat)) != std::wstring::npos) {
            return true;
        }
    }
    return false;
}

bool is_ignored_process(const std::wstring& name) {
    const std::wstring n = lower(name);
    for (const wchar_t* p : kIgnored) {
        if (n == lower(p)) {
            return true;
        }
    }
    return false;
}

const wchar_t* first_match_w(const std::wstring& text, const wchar_t* const* pats, size_t count) {
    const std::wstring hay = lower(text);
    for (size_t i = 0; i < count; ++i) {
        if (hay.find(lower(pats[i])) != std::wstring::npos) {
            return pats[i];
        }
    }
    return nullptr;
}

const char* first_match_a(const std::string& line, const char* const* pats, size_t count) {
    std::string hay = line;
    std::transform(hay.begin(), hay.end(), hay.begin(), [](unsigned char c) { return static_cast<char>(tolower(c)); });
    for (size_t i = 0; i < count; ++i) {
        std::string pat = pats[i];
        std::transform(pat.begin(), pat.end(), pat.begin(), [](unsigned char c) { return static_cast<char>(tolower(c)); });
        if (hay.find(pat) != std::string::npos) {
            return pats[i];
        }
    }
    return nullptr;
}

std::wstring env_path(const wchar_t* name, const wchar_t* fallback) {
    wchar_t buf[MAX_PATH]{};
    DWORD n = GetEnvironmentVariableW(name, buf, MAX_PATH);
    if (n == 0 || n >= MAX_PATH) {
        return fallback;
    }
    return buf;
}

std::wstring minecraft_dir() {
    return env_path(L"APPDATA", L"") + L"\\.minecraft";
}

std::wstring known_or_user(REFKNOWNFOLDERID id, const wchar_t* relative) {
    PWSTR path = nullptr;
    if (SUCCEEDED(SHGetKnownFolderPath(id, KF_FLAG_DEFAULT, nullptr, &path)) && path) {
        std::wstring out = path;
        CoTaskMemFree(path);
        return out;
    }
    return env_path(L"USERPROFILE", L".") + L"\\" + relative;
}

std::wstring desktop_dir() {
    return known_or_user(FOLDERID_Desktop, L"Desktop");
}

std::wstring downloads_dir() {
    return known_or_user(FOLDERID_Downloads, L"Downloads");
}

std::wstring temp_dir() {
    wchar_t buf[MAX_PATH]{};
    GetTempPathW(MAX_PATH, buf);
    return buf;
}

void emit(cc_line_cb on_line, void* user, const std::wstring& line) {
    if (on_line) {
        on_line(line.c_str(), user);
    }
}

void walk_limited(const std::wstring& dir, int depth, int maxDepth, const std::function<void(const std::wstring&)>& fn) {
    if (depth > maxDepth) {
        return;
    }
    std::wstring pattern = dir;
    if (!pattern.empty() && pattern.back() != L'\\' && pattern.back() != L'/') {
        pattern += L'\\';
    }
    pattern += L"*";

    WIN32_FIND_DATAW fd{};
    HANDLE h = FindFirstFileW(pattern.c_str(), &fd);
    if (h == INVALID_HANDLE_VALUE) {
        return;
    }
    do {
        if (wcscmp(fd.cFileName, L".") == 0 || wcscmp(fd.cFileName, L"..") == 0) {
            continue;
        }
        std::wstring path = dir;
        if (!path.empty() && path.back() != L'\\' && path.back() != L'/') {
            path += L'\\';
        }
        path += fd.cFileName;
        if (fd.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY) {
            walk_limited(path, depth + 1, maxDepth, fn);
        } else {
            fn(path);
        }
    } while (FindNextFileW(h, &fd));
    FindClose(h);
}

void scan_processes(cc_line_cb on_line, void* user) {
    HANDLE snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
    if (snap == INVALID_HANDLE_VALUE) {
        return;
    }
    PROCESSENTRY32W pe{};
    pe.dwSize = sizeof(pe);
    if (!Process32FirstW(snap, &pe)) {
        CloseHandle(snap);
        return;
    }
    do {
        std::wstring name = pe.szExeFile;
        if (is_ignored_process(name)) {
            continue;
        }
        std::wstring exe;
        HANDLE proc = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, FALSE, pe.th32ProcessID);
        if (proc) {
            wchar_t path[MAX_PATH]{};
            DWORD size = MAX_PATH;
            if (QueryFullProcessImageNameW(proc, 0, path, &size)) {
                exe = path;
            }
            CloseHandle(proc);
        }
        std::wstring hay = name + L" " + exe;
        if (contains_cheat(hay)) {
            emit(on_line, user, L"Процесс: " + name + L" (путь: " + exe + L")");
        }
    } while (Process32NextW(snap, &pe));
    CloseHandle(snap);
}

void scan_files(cc_line_cb on_line, void* user) {
    const std::wstring folders[] = {
        minecraft_dir() + L"\\versions",
        minecraft_dir() + L"\\mods",
        desktop_dir(),
        downloads_dir(),
        temp_dir(),
    };
    for (const auto& folder : folders) {
        walk_limited(folder, 0, 2, [&](const std::wstring& path) {
            size_t slash = path.find_last_of(L"\\/");
            std::wstring name = slash == std::wstring::npos ? path : path.substr(slash + 1);
            if (contains_cheat(name)) {
                emit(on_line, user, L"Файл: " + path);
            }
        });
    }
}

void scan_startup(cc_line_cb on_line, void* user) {
    const wchar_t* keys[] = {
        L"Software\\Microsoft\\Windows\\CurrentVersion\\Run",
        L"Software\\Microsoft\\Windows\\CurrentVersion\\RunOnce",
    };
    for (const wchar_t* sub : keys) {
        HKEY key = nullptr;
        if (RegOpenKeyExW(HKEY_CURRENT_USER, sub, 0, KEY_READ, &key) != ERROR_SUCCESS) {
            continue;
        }
        for (DWORD i = 0;; ++i) {
            wchar_t name[256]{};
            wchar_t value[1024]{};
            DWORD nameLen = 256;
            DWORD valueLen = sizeof(value);
            DWORD type = 0;
            LONG st = RegEnumValueW(key, i, name, &nameLen, nullptr, &type, reinterpret_cast<LPBYTE>(value), &valueLen);
            if (st == ERROR_NO_MORE_ITEMS) {
                break;
            }
            if (st != ERROR_SUCCESS) {
                continue;
            }
            std::wstring n = name;
            std::wstring v = (type == REG_SZ || type == REG_EXPAND_SZ) ? value : L"";
            if (contains_cheat(n) || contains_cheat(v)) {
                emit(on_line, user, L"Автозагрузка: " + n + L" → " + v);
            }
        }
        RegCloseKey(key);
    }
}

void scan_logs(cc_line_cb on_line, void* user) {
    std::wstring path = minecraft_dir() + L"\\logs\\latest.log";
    std::ifstream in(path, std::ios::binary);
    if (!in) {
        return;
    }
    std::string content((std::istreambuf_iterator<char>(in)), std::istreambuf_iterator<char>());
    std::vector<std::string> lines;
    std::string cur;
    for (char c : content) {
        if (c == '\n') {
            if (!cur.empty() && cur.back() == '\r') {
                cur.pop_back();
            }
            lines.push_back(std::move(cur));
            cur.clear();
        } else {
            cur.push_back(c);
        }
    }
    if (!cur.empty()) {
        lines.push_back(std::move(cur));
    }

    std::vector<std::wstring> found;
    size_t start = lines.size() > 300 ? lines.size() - 300 : 0;
    for (size_t i = start; i < lines.size(); ++i) {
        if (const char* pat = first_match_a(lines[i], kInject, std::size(kInject))) {
            std::wstring w(pat, pat + strlen(pat));
            found.push_back(L"В логах: " + w);
            continue;
        }
        if (const char* pat = first_match_a(lines[i], kLogSuspicious, std::size(kLogSuspicious))) {
            std::wstring w(pat, pat + strlen(pat));
            found.push_back(L"В логах: " + w);
        }
    }
    if (!found.empty()) {
        emit(on_line, user, L"Логи Minecraft:");
        for (size_t i = 0; i < found.size() && i < 5; ++i) {
            emit(on_line, user, L"   " + found[i]);
        }
    }
}

void scan_hitbox(cc_line_cb on_line, void* user) {
    const std::wstring folders[] = {
        minecraft_dir() + L"\\mods",
        minecraft_dir() + L"\\versions",
        desktop_dir(),
        downloads_dir(),
    };
    std::vector<std::wstring> found;
    for (const auto& folder : folders) {
        walk_limited(folder, 0, 2, [&](const std::wstring& path) {
            size_t slash = path.find_last_of(L"\\/");
            std::wstring name = slash == std::wstring::npos ? path : path.substr(slash + 1);
            if (const wchar_t* pat = first_match_w(name, kHitbox, std::size(kHitbox))) {
                found.push_back(L"Подозрительный файл (" + std::wstring(pat) + L"): " + path);
            }
        });
    }
    if (!found.empty()) {
        emit(on_line, user, L"Подозрительные файлы:");
        for (size_t i = 0; i < found.size() && i < 5; ++i) {
            emit(on_line, user, L"   " + found[i]);
        }
    }
}

void scan_unknown_dlls(cc_line_cb on_line, void* user) {
    const std::wstring folders[] = {
        minecraft_dir() + L"\\bin",
        minecraft_dir() + L"\\versions",
    };
    std::vector<std::wstring> found;
    for (const auto& folder : folders) {
        walk_limited(folder, 0, 2, [&](const std::wstring& path) {
            size_t slash = path.find_last_of(L"\\/");
            std::wstring name = slash == std::wstring::npos ? path : path.substr(slash + 1);
            if (name.size() < 4 || _wcsicmp(name.c_str() + name.size() - 4, L".dll") != 0) {
                return;
            }
            std::wstring low = lower(name);
            bool known = false;
            for (const wchar_t* k : kKnownDll) {
                if (low.find(k) != std::wstring::npos) {
                    known = true;
                    break;
                }
            }
            if (!known) {
                found.push_back(L"Неизвестный .dll: " + path);
            }
        });
    }
    if (!found.empty()) {
        emit(on_line, user, L"DLL:");
        for (size_t i = 0; i < found.size() && i < 5; ++i) {
            emit(on_line, user, L"   " + found[i]);
        }
    }
}

}  // namespace

extern "C" int cc_perform_scan(cc_phase_cb on_phase, cc_line_cb on_line, void* user) {
    auto phase = [&](int p) {
        if (on_phase) {
            on_phase(p, user);
        }
    };
    phase(CC_PHASE_PROCESSES);
    scan_processes(on_line, user);
    phase(CC_PHASE_FILES);
    scan_files(on_line, user);
    phase(CC_PHASE_REGISTRY);
    scan_startup(on_line, user);
    phase(CC_PHASE_LOGS);
    scan_logs(on_line, user);
    scan_hitbox(on_line, user);
    scan_unknown_dlls(on_line, user);

    wchar_t recycle[128]{};
    cc_get_recycle_mtime(recycle, 128);
    emit(on_line, user, std::wstring(L"Корзина: ") + recycle);
    return 0;
}
