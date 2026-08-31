#define NOMINMAX
#define WIN32_LEAN_AND_MEAN
#define UNICODE
#define _UNICODE
#ifndef CUBCHECK_NATIVE_EXPORTS
#define CUBCHECK_NATIVE_EXPORTS
#endif

#include "../include/cubecheck_native.h"

#include <windows.h>
#include <shellapi.h>
#include <shlobj.h>
#include <shobjidl.h>
#include <knownfolders.h>
#include <wintrust.h>
#include <softpub.h>
#include <wincrypt.h>

#include <string>
#include <vector>
#include <cstring>

#pragma comment(lib, "wintrust.lib")
#pragma comment(lib, "crypt32.lib")
#pragma comment(lib, "ole32.lib")
#pragma comment(lib, "shell32.lib")
#pragma comment(lib, "advapi32.lib")
#pragma comment(lib, "user32.lib")

namespace {

void set_err(wchar_t* err, int err_cch, const wchar_t* msg) {
    if (!err || err_cch <= 0) {
        return;
    }
    wcsncpy_s(err, static_cast<size_t>(err_cch), msg, _TRUNCATE);
}

std::wstring file_name(const wchar_t* path) {
    const wchar_t* slash = wcsrchr(path, L'\\');
    const wchar_t* slash2 = wcsrchr(path, L'/');
    if (slash2 && (!slash || slash2 > slash)) {
        slash = slash2;
    }
    return slash ? slash + 1 : path;
}

std::vector<std::wstring> signature_display_names(const wchar_t* path) {
    std::vector<std::wstring> names;
    DWORD encoding = 0;
    DWORD contentType = 0;
    DWORD formatType = 0;
    HCERTSTORE store = nullptr;
    HCRYPTMSG msg = nullptr;

    if (!CryptQueryObject(
            CERT_QUERY_OBJECT_FILE,
            path,
            CERT_QUERY_CONTENT_FLAG_PKCS7_SIGNED_EMBED,
            CERT_QUERY_FORMAT_FLAG_BINARY,
            0,
            &encoding,
            &contentType,
            &formatType,
            &store,
            &msg,
            nullptr)) {
        return names;
    }

    PCCERT_CONTEXT cert = nullptr;
    while ((cert = CertEnumCertificatesInStore(store, cert)) != nullptr) {
        wchar_t buf[256]{};
        DWORD n = CertGetNameStringW(cert, CERT_NAME_SIMPLE_DISPLAY_TYPE, 0, nullptr, buf, 256);
        if (n > 1) {
            std::wstring name(buf);
            bool dup = false;
            for (const auto& existing : names) {
                if (_wcsicmp(existing.c_str(), name.c_str()) == 0) {
                    dup = true;
                    break;
                }
            }
            if (!dup && !name.empty()) {
                names.push_back(std::move(name));
            }
        }
    }
    if (store) {
        CertCloseStore(store, 0);
    }
    if (msg) {
        CryptMsgClose(msg);
    }
    return names;
}

bool known_folder_path(REFKNOWNFOLDERID id, wchar_t* buf, int cch) {
    PWSTR path = nullptr;
    if (FAILED(SHGetKnownFolderPath(id, KF_FLAG_DEFAULT, nullptr, &path)) || !path) {
        return false;
    }
    wcsncpy_s(buf, static_cast<size_t>(cch), path, _TRUNCATE);
    CoTaskMemFree(path);
    return true;
}

}  // namespace

extern "C" int cc_is_elevated(void) {
    return IsUserAnAdmin() ? 1 : 0;
}

extern "C" int cc_is_pe_amd64(const wchar_t* path) {
    if (!path) {
        return 0;
    }
    HANDLE file = CreateFileW(path, GENERIC_READ, FILE_SHARE_READ, nullptr, OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL, nullptr);
    if (file == INVALID_HANDLE_VALUE) {
        return 0;
    }
    unsigned char header[0x40]{};
    DWORD read = 0;
    BOOL ok = ReadFile(file, header, sizeof(header), &read, nullptr);
    if (!ok || read < 0x40 || header[0] != 'M' || header[1] != 'Z') {
        CloseHandle(file);
        return 0;
    }
    DWORD pe = *reinterpret_cast<DWORD*>(header + 0x3C);
    if (SetFilePointer(file, static_cast<LONG>(pe), nullptr, FILE_BEGIN) == INVALID_SET_FILE_POINTER) {
        CloseHandle(file);
        return 0;
    }
    unsigned char sig[6]{};
    ok = ReadFile(file, sig, sizeof(sig), &read, nullptr);
    CloseHandle(file);
    return ok && read == 6 && sig[0] == 'P' && sig[1] == 'E' && sig[2] == 0 && sig[3] == 0 && sig[4] == 0x64 &&
           sig[5] == 0x86;
}

extern "C" int cc_verify_publisher(const wchar_t* path, const wchar_t* expected, wchar_t* err, int err_cch) {
    if (!path || !expected) {
        set_err(err, err_cch, L"Некорректные аргументы проверки подписи");
        return 1;
    }
    if (GetFileAttributesW(path) == INVALID_FILE_ATTRIBUTES) {
        set_err(err, err_cch, (L"Файл не найден: " + std::wstring(path)).c_str());
        return 1;
    }

    WINTRUST_FILE_INFO fileInfo{};
    fileInfo.cbStruct = sizeof(fileInfo);
    fileInfo.pcwszFilePath = path;
    fileInfo.hFile = nullptr;
    fileInfo.pgKnownSubject = nullptr;

    WINTRUST_DATA data{};
    data.cbStruct = sizeof(data);
    data.dwUIChoice = WTD_UI_NONE;
    data.fdwRevocationChecks = WTD_REVOKE_NONE;
    data.dwUnionChoice = WTD_CHOICE_FILE;
    data.pFile = &fileInfo;
    data.dwStateAction = WTD_STATEACTION_VERIFY;

    GUID policy = WINTRUST_ACTION_GENERIC_VERIFY_V2;
    LONG status = WinVerifyTrust(reinterpret_cast<HWND>(static_cast<INT_PTR>(-1)), &policy, &data);

    data.dwStateAction = WTD_STATEACTION_CLOSE;
    WinVerifyTrust(reinterpret_cast<HWND>(static_cast<INT_PTR>(-1)), &policy, &data);

    if (status != 0) {
        std::wstring msg = L"Файл не прошёл проверку: " + file_name(path);
        set_err(err, err_cch, msg.c_str());
        return 1;
    }

    auto names = signature_display_names(path);
    if (names.empty()) {
        set_err(err, err_cch, L"Не удалось проверить подпись файла");
        return 1;
    }

    std::wstring expectedL(expected);
    for (auto& ch : expectedL) {
        ch = static_cast<wchar_t>(towlower(ch));
    }
    for (const auto& name : names) {
        std::wstring lower = name;
        for (auto& ch : lower) {
            ch = static_cast<wchar_t>(towlower(ch));
        }
        if (lower.find(expectedL) != std::wstring::npos) {
            return 0;
        }
    }
    std::wstring msg = L"Подпись не совпала (нужен «";
    msg += expected;
    msg += L"»)";
    set_err(err, err_cch, msg.c_str());
    return 1;
}

extern "C" int cc_shell_execute(const wchar_t* path, const wchar_t* dir, const wchar_t* verb, const wchar_t* params) {
    if (!path) {
        return 1;
    }
    INT_PTR code = reinterpret_cast<INT_PTR>(ShellExecuteW(
        nullptr,
        verb && verb[0] ? verb : L"open",
        path,
        params && params[0] ? params : nullptr,
        dir,
        SW_SHOWNORMAL));
    return code > 32 ? 0 : 1;
}

extern "C" int cc_create_shortcut(const wchar_t* link, const wchar_t* target, const wchar_t* workdir, const wchar_t* icon) {
    if (!link || !target) {
        return 1;
    }
    wchar_t parent[MAX_PATH]{};
    wcsncpy_s(parent, link, _TRUNCATE);
    wchar_t* slash = wcsrchr(parent, L'\\');
    if (slash) {
        *slash = 0;
        SHCreateDirectoryExW(nullptr, parent, nullptr);
    }

    CoInitializeEx(nullptr, COINIT_APARTMENTTHREADED);
    IShellLinkW* shellLink = nullptr;
    HRESULT hr = CoCreateInstance(CLSID_ShellLink, nullptr, CLSCTX_INPROC_SERVER, IID_IShellLinkW, reinterpret_cast<void**>(&shellLink));
    if (FAILED(hr) || !shellLink) {
        return 1;
    }
    shellLink->SetPath(target);
    if (workdir) {
        shellLink->SetWorkingDirectory(workdir);
    }
    if (icon && icon[0]) {
        shellLink->SetIconLocation(icon, 0);
    }
    shellLink->SetDescription(L"CubeCheck");

    IPersistFile* persist = nullptr;
    hr = shellLink->QueryInterface(IID_IPersistFile, reinterpret_cast<void**>(&persist));
    if (SUCCEEDED(hr) && persist) {
        hr = persist->Save(link, TRUE);
        persist->Release();
    }
    shellLink->Release();
    return SUCCEEDED(hr) ? 0 : 1;
}

extern "C" int cc_install_shortcuts(const wchar_t* target, const wchar_t* workdir, const wchar_t* icon) {
    int ok = 0;
    wchar_t desktop[MAX_PATH]{};
    wchar_t programs[MAX_PATH]{};
    if (known_folder_path(FOLDERID_Desktop, desktop, MAX_PATH)) {
        std::wstring link = std::wstring(desktop) + L"\\CubeCheck.lnk";
        if (cc_create_shortcut(link.c_str(), target, workdir, icon) == 0) {
            ok++;
        }
    }
    if (known_folder_path(FOLDERID_Programs, programs, MAX_PATH)) {
        std::wstring link = std::wstring(programs) + L"\\CubeCheck.lnk";
        if (cc_create_shortcut(link.c_str(), target, workdir, icon) == 0) {
            ok++;
        }
    }
    return ok > 0 ? 0 : 1;
}

extern "C" int cc_relaunch_as_admin(void) {
    wchar_t exe[MAX_PATH]{};
    if (!GetModuleFileNameW(nullptr, exe, MAX_PATH)) {
        return 1;
    }
    wchar_t dir[MAX_PATH]{};
    wcsncpy_s(dir, exe, _TRUNCATE);
    if (wchar_t* slash = wcsrchr(dir, L'\\')) {
        *slash = 0;
    }
    return cc_shell_execute(exe, dir, L"runas", L"");
}

extern "C" void cc_message_box(const wchar_t* title, const wchar_t* text) {
    MessageBoxW(nullptr, text ? text : L"", title ? title : L"CubeCheck", MB_OK);
}

extern "C" int cc_get_install_date(wchar_t* buf, int cch) {
    if (!buf || cch <= 0) {
        return 1;
    }
    HKEY key = nullptr;
    if (RegOpenKeyExW(HKEY_LOCAL_MACHINE, L"SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion", 0, KEY_READ, &key) !=
        ERROR_SUCCESS) {
        wcsncpy_s(buf, static_cast<size_t>(cch), L"Не удалось определить", _TRUNCATE);
        return 1;
    }
    DWORD ts = 0;
    DWORD size = sizeof(ts);
    DWORD type = 0;
    LONG st = RegQueryValueExW(key, L"InstallDate", nullptr, &type, reinterpret_cast<LPBYTE>(&ts), &size);
    RegCloseKey(key);
    if (st != ERROR_SUCCESS) {
        wcsncpy_s(buf, static_cast<size_t>(cch), L"Не удалось определить", _TRUNCATE);
        return 1;
    }
    FILETIME ft{};
    ULARGE_INTEGER ull{};
    ull.QuadPart = (static_cast<ULONGLONG>(ts) + 11644473600ULL) * 10000000ULL;
    ft.dwLowDateTime = ull.LowPart;
    ft.dwHighDateTime = ull.HighPart;
    SYSTEMTIME utc{}, local{};
    FileTimeToSystemTime(&ft, &utc);
    SystemTimeToTzSpecificLocalTime(nullptr, &utc, &local);
    swprintf_s(buf, static_cast<size_t>(cch), L"%02u.%02u.%04u %02u:%02u:%02u", local.wDay, local.wMonth, local.wYear,
               local.wHour, local.wMinute, local.wSecond);
    return 0;
}

extern "C" int cc_get_recycle_mtime(wchar_t* buf, int cch) {
    if (!buf || cch <= 0) {
        return 1;
    }
    wchar_t drive[8]{};
    DWORD n = GetEnvironmentVariableW(L"SystemDrive", drive, 8);
    if (n == 0) {
        wcsncpy_s(drive, L"C:", _TRUNCATE);
    }
    std::wstring path = std::wstring(drive) + L"\\$Recycle.Bin";
    WIN32_FILE_ATTRIBUTE_DATA fad{};
    if (!GetFileAttributesExW(path.c_str(), GetFileExInfoStandard, &fad)) {
        wcsncpy_s(buf, static_cast<size_t>(cch), L"Папка корзины не найдена", _TRUNCATE);
        return 1;
    }
    SYSTEMTIME utc{}, local{};
    FileTimeToSystemTime(&fad.ftLastWriteTime, &utc);
    SystemTimeToTzSpecificLocalTime(nullptr, &utc, &local);
    swprintf_s(buf, static_cast<size_t>(cch), L"%02u.%02u.%04u %02u:%02u:%02u", local.wDay, local.wMonth, local.wYear,
               local.wHour, local.wMinute, local.wSecond);
    return 0;
}

extern "C" int cc_known_folder(int which, wchar_t* buf, int cch) {
    if (!buf || cch <= 0) {
        return 1;
    }
    const KNOWNFOLDERID* id = &FOLDERID_Desktop;
    switch (which) {
        case 1:
            id = &FOLDERID_Downloads;
            break;
        case 2:
            id = &FOLDERID_Programs;
            break;
        default:
            id = &FOLDERID_Desktop;
            break;
    }
    return known_folder_path(*id, buf, cch) ? 0 : 1;
}
