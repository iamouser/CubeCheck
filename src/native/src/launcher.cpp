#define NOMINMAX
#define WIN32_LEAN_AND_MEAN
#define UNICODE
#define _UNICODE

#include <windows.h>
#include <shellapi.h>

#include <string>
#include <vector>

#pragma comment(lib, "user32.lib")
#pragma comment(lib, "shell32.lib")

namespace {

bool file_exists(const std::wstring& path) {
    DWORD attr = GetFileAttributesW(path.c_str());
    return attr != INVALID_FILE_ATTRIBUTES && !(attr & FILE_ATTRIBUTE_DIRECTORY);
}

bool dir_exists(const std::wstring& path) {
    DWORD attr = GetFileAttributesW(path.c_str());
    return attr != INVALID_FILE_ATTRIBUTES && (attr & FILE_ATTRIBUTE_DIRECTORY);
}

std::wstring exe_dir() {
    wchar_t buf[MAX_PATH]{};
    GetModuleFileNameW(nullptr, buf, MAX_PATH);
    std::wstring path = buf;
    size_t slash = path.find_last_of(L"\\/");
    return slash == std::wstring::npos ? L"." : path.substr(0, slash);
}

bool offline_marker(const std::wstring& dir) {
    return file_exists(dir + L"\\.offline") || file_exists(dir + L"\\assets\\.offline");
}

bool windows_is_64bit() {
    wchar_t arch[64]{};
    wchar_t wow[64]{};
    GetEnvironmentVariableW(L"PROCESSOR_ARCHITECTURE", arch, 64);
    GetEnvironmentVariableW(L"PROCESSOR_ARCHITEW6432", wow, 64);
    auto upper = [](const wchar_t* s) {
        return _wcsicmp(s, L"AMD64") == 0 || _wcsicmp(s, L"ARM64") == 0;
    };
    return upper(arch) || upper(wow);
}

std::wstring payload_exe(const std::wstring& dir) {
    const wchar_t* names[] = {L"cubecheck.exe", L"cubecheck"};
    for (const wchar_t* name : names) {
        std::wstring path = dir + L"\\" + name;
        if (file_exists(path)) {
            return path;
        }
    }
    return {};
}

void die(const std::wstring& msg) {
    std::wstring root = exe_dir();
    std::wstring log = root + L"\\CubeCheck-error.txt";
    std::wstring body = L"CubeCheck\r\n\r\n" + msg + L"\r\n";
    HANDLE file = CreateFileW(log.c_str(), GENERIC_WRITE, FILE_SHARE_READ, nullptr, CREATE_ALWAYS, FILE_ATTRIBUTE_NORMAL, nullptr);
    if (file != INVALID_HANDLE_VALUE) {
        int bytes = WideCharToMultiByte(CP_UTF8, 0, body.c_str(), -1, nullptr, 0, nullptr, nullptr);
        if (bytes > 1) {
            std::string utf8(static_cast<size_t>(bytes - 1), '\0');
            WideCharToMultiByte(CP_UTF8, 0, body.c_str(), -1, utf8.data(), bytes, nullptr, nullptr);
            DWORD written = 0;
            WriteFile(file, utf8.data(), static_cast<DWORD>(utf8.size()), &written, nullptr);
        }
        CloseHandle(file);
        ShellExecuteW(nullptr, L"open", log.c_str(), nullptr, nullptr, SW_SHOWNORMAL);
    } else {
        MessageBoxW(nullptr, msg.c_str(), L"CubeCheck", MB_OK | MB_ICONERROR);
    }
}

}  // namespace

int WINAPI wWinMain(HINSTANCE, HINSTANCE, PWSTR cmdLine, int) {
    (void)cmdLine;
    const std::wstring root = exe_dir();
    std::vector<std::pair<std::wstring, std::wstring>> candidates;
    if (windows_is_64bit()) {
        candidates.emplace_back(L"windows-x64", L"windows-x64");
        candidates.emplace_back(L"windows-x86", L"windows-x86");
    } else {
        candidates.emplace_back(L"windows-x86", L"windows-x86");
        candidates.emplace_back(L"windows-x64", L"windows-x64");
    }

    std::wstring kind;
    std::wstring payloadDir;
    std::wstring exe;
    std::wstring tried;
    for (const auto& [k, folder] : candidates) {
        std::wstring dir = root + L"\\payload\\" + folder;
        std::wstring found = payload_exe(dir);
        if (!found.empty()) {
            kind = k;
            payloadDir = dir;
            exe = found;
            break;
        }
        tried += L"  " + k + L" (" + dir + L")\r\n";
    }

    if (exe.empty()) {
        die(L"Нет сборки CubeCheck для этой ОС.\r\nИскали:\r\n" + tried);
        return 1;
    }

    SetEnvironmentVariableW(L"CUBECHECK_PORTABLE", L"1");
    SetEnvironmentVariableW(L"CUBECHECK_LAUNCHER_OS", kind.c_str());
    if (offline_marker(root) || offline_marker(payloadDir)) {
        SetEnvironmentVariableW(L"CUBECHECK_OFFLINE", L"1");
    }

    STARTUPINFOW si{};
    si.cb = sizeof(si);
    PROCESS_INFORMATION pi{};
    std::wstring cmd = L"\"" + exe + L"\"";
    if (!CreateProcessW(exe.c_str(), cmd.data(), nullptr, nullptr, FALSE, 0, nullptr, payloadDir.c_str(), &si, &pi)) {
        die(L"Не удалось запустить " + exe +
            L"\r\n\r\nНа Windows 7/8 нужна .NET Framework 4.8:\r\nhttps://dotnet.microsoft.com/download/dotnet-framework/net48");
        return 1;
    }
    WaitForSingleObject(pi.hProcess, INFINITE);
    DWORD code = 0;
    GetExitCodeProcess(pi.hProcess, &code);
    CloseHandle(pi.hThread);
    CloseHandle(pi.hProcess);
    return static_cast<int>(code);
}
