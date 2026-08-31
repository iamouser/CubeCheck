#pragma once

#ifdef CUBCHECK_NATIVE_EXPORTS
#define CCAPI __declspec(dllexport)
#else
#define CCAPI __declspec(dllimport)
#endif

#ifdef __cplusplus
extern "C" {
#endif

enum CcScanPhase {
    CC_PHASE_PROCESSES = 0,
    CC_PHASE_FILES = 1,
    CC_PHASE_REGISTRY = 2,
    CC_PHASE_LOGS = 3
};

typedef void (*cc_phase_cb)(int phase, void* user);
typedef void (*cc_line_cb)(const wchar_t* line, void* user);

CCAPI int cc_is_elevated(void);
CCAPI int cc_is_pe_amd64(const wchar_t* path);
CCAPI int cc_verify_publisher(const wchar_t* path, const wchar_t* expected, wchar_t* err, int err_cch);
CCAPI int cc_shell_execute(const wchar_t* path, const wchar_t* dir, const wchar_t* verb, const wchar_t* params);
CCAPI int cc_create_shortcut(const wchar_t* link, const wchar_t* target, const wchar_t* workdir, const wchar_t* icon);
CCAPI int cc_install_shortcuts(const wchar_t* target, const wchar_t* workdir, const wchar_t* icon);
CCAPI int cc_relaunch_as_admin(void);
CCAPI void cc_message_box(const wchar_t* title, const wchar_t* text);
CCAPI int cc_perform_scan(cc_phase_cb on_phase, cc_line_cb on_line, void* user);
CCAPI int cc_get_install_date(wchar_t* buf, int cch);
CCAPI int cc_get_recycle_mtime(wchar_t* buf, int cch);
CCAPI int cc_known_folder(int which, wchar_t* buf, int cch);

#ifdef __cplusplus
}
#endif
