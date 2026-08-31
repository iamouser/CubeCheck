#pragma once

/* C ABI for CubeCheck.Core + cubecheck_native.dll. UTF-8. cdecl. */

#ifdef __cplusplus
extern "C" {
#endif

enum CcHostProgress {
    CC_HOST_CONNECTING = 0,
    CC_HOST_RECEIVING = 1,
    CC_HOST_VERIFYING = 2,
    CC_HOST_EXTRACTING = 3,
    CC_HOST_READY = 4,
    CC_HOST_FAILED = 5
};

typedef void (*cc_host_phase_cb)(int phase, void* user);
typedef void (*cc_host_progress_cb)(
    const char* id,
    int stage,
    long long received,
    long long total,
    const char* err,
    void* user);

void cc_host_free(char* p);
char* cc_host_last_error(void);

int cc_host_init(void);
int cc_host_load_settings(char** json_out);
int cc_host_save_settings(const char* json);

int cc_host_is_offline(void);
int cc_host_downloads_enabled(void);
int cc_host_any_tool_missing(void);
int cc_host_tool_installed(const char* id);
int cc_host_missing_tool_ids(char** ids_out);
int cc_host_cheat_list_text(char** text_out);

int cc_host_run_util(const char* id);
int cc_host_run_autocheck_search(void);
int cc_host_open_recycle(void);
int cc_host_open_telegram(void);
int cc_host_open_holycheck(void);
int cc_host_run_system_info(void);
int cc_host_clear_logs(void);

int cc_host_perform_scan(cc_host_phase_cb on_phase, void* user, char** lines_out);
int cc_host_download_tools(
    const char* ids_csv,
    int force,
    cc_host_progress_cb cb,
    void* user);
int cc_host_save_report(const char* findings, char** path_out);

int cc_host_user_name(char** out);
int cc_host_computer_name(char** out);
int cc_host_install_date(char** out);
int cc_host_recycle_mtime(char** out);
int cc_host_os_info_label(char** out);

#ifdef __cplusplus
}
#endif
