using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;
using System.Text;
using CubeCheck;

namespace CubeCheck.Api;

public static unsafe class Host
{
    [ThreadStatic] static string? LastError;

    static void Fail(Exception ex) =>
        LastError = string.IsNullOrWhiteSpace(ex.Message) ? ex.GetType().Name : ex.Message;

    static byte* AllocUtf8(string? value)
    {
        var bytes = Encoding.UTF8.GetBytes(value ?? "");
        var ptr = (byte*)NativeMemory.Alloc((nuint)(bytes.Length + 1));
        if (bytes.Length > 0)
        {
            Marshal.Copy(bytes, 0, (IntPtr)ptr, bytes.Length);
        }
        ptr[bytes.Length] = 0;
        return ptr;
    }

    static string PtrToString(byte* ptr) =>
        ptr == null ? "" : Marshal.PtrToStringUTF8((IntPtr)ptr) ?? "";

    static int OkString(byte** dest, string value)
    {
        *dest = AllocUtf8(value);
        return 0;
    }

    static int FailOut(byte** dest, Exception ex)
    {
        Fail(ex);
        *dest = null;
        return 1;
    }

    static int Run(Action action)
    {
        try
        {
            action();
            return 0;
        }
        catch (Exception ex)
        {
            Fail(ex);
            return 1;
        }
    }

    [UnmanagedCallersOnly(EntryPoint = "cc_host_free", CallConvs = [typeof(CallConvCdecl)])]
    public static void Free(byte* ptr)
    {
        if (ptr != null) NativeMemory.Free(ptr);
    }

    [UnmanagedCallersOnly(EntryPoint = "cc_host_last_error", CallConvs = [typeof(CallConvCdecl)])]
    public static byte* GetLastError() => AllocUtf8(LastError ?? "");

    [UnmanagedCallersOnly(EntryPoint = "cc_host_init", CallConvs = [typeof(CallConvCdecl)])]
    public static int Init()
    {
        NativeLoader.Ensure();
        return Run(AppPaths.EnsureInstallDir);
    }

    [UnmanagedCallersOnly(EntryPoint = "cc_host_load_settings", CallConvs = [typeof(CallConvCdecl)])]
    public static int LoadSettings(byte** jsonOut)
    {
        try
        {
            return OkString(jsonOut, AppConfig.Load().ToJson());
        }
        catch (Exception ex)
        {
            return FailOut(jsonOut, ex);
        }
    }

    [UnmanagedCallersOnly(EntryPoint = "cc_host_save_settings", CallConvs = [typeof(CallConvCdecl)])]
    public static int SaveSettings(byte* json) =>
        Run(() => AppConfig.FromJson(PtrToString(json)).Save());

    [UnmanagedCallersOnly(EntryPoint = "cc_host_is_offline", CallConvs = [typeof(CallConvCdecl)])]
    public static int IsOffline() => AppPaths.IsOffline ? 1 : 0;

    [UnmanagedCallersOnly(EntryPoint = "cc_host_downloads_enabled", CallConvs = [typeof(CallConvCdecl)])]
    public static int DownloadsEnabled() => Downloader.DownloadsEnabled ? 1 : 0;

    [UnmanagedCallersOnly(EntryPoint = "cc_host_any_tool_missing", CallConvs = [typeof(CallConvCdecl)])]
    public static int AnyToolMissing() => AppPaths.AnyToolMissing() ? 1 : 0;

    [UnmanagedCallersOnly(EntryPoint = "cc_host_tool_installed", CallConvs = [typeof(CallConvCdecl)])]
    public static int ToolInstalled(byte* id) => AppPaths.ToolInstalled(PtrToString(id)) ? 1 : 0;

    [UnmanagedCallersOnly(EntryPoint = "cc_host_missing_tool_ids", CallConvs = [typeof(CallConvCdecl)])]
    public static int MissingToolIds(byte** idsOut)
    {
        try
        {
            var ids = Downloader.MissingTools(Downloader.LoadManifest()).Select(t => t.Id);
            return OkString(idsOut, string.Join(",", ids));
        }
        catch (Exception ex)
        {
            return FailOut(idsOut, ex);
        }
    }

    [UnmanagedCallersOnly(EntryPoint = "cc_host_cheat_list_text", CallConvs = [typeof(CallConvCdecl)])]
    public static int CheatListText(byte** textOut)
    {
        try
        {
            return OkString(textOut, Catalog.CheatListText());
        }
        catch (Exception ex)
        {
            return FailOut(textOut, ex);
        }
    }

    [UnmanagedCallersOnly(EntryPoint = "cc_host_run_util", CallConvs = [typeof(CallConvCdecl)])]
    public static int RunUtil(byte* id) => Run(() => ToolLauncher.RunUtil(PtrToString(id)));

    [UnmanagedCallersOnly(EntryPoint = "cc_host_run_autocheck_search", CallConvs = [typeof(CallConvCdecl)])]
    public static int RunAutocheckSearch() => Run(ToolLauncher.RunAutocheckSearch);

    [UnmanagedCallersOnly(EntryPoint = "cc_host_open_recycle", CallConvs = [typeof(CallConvCdecl)])]
    public static int OpenRecycle() => Run(ToolLauncher.OpenRecycleBin);

    [UnmanagedCallersOnly(EntryPoint = "cc_host_open_telegram", CallConvs = [typeof(CallConvCdecl)])]
    public static int OpenTelegram() => Run(ToolLauncher.OpenTelegram);

    [UnmanagedCallersOnly(EntryPoint = "cc_host_open_holycheck", CallConvs = [typeof(CallConvCdecl)])]
    public static int OpenHolyCheck() => Run(ToolLauncher.OpenHolyCheck);

    [UnmanagedCallersOnly(EntryPoint = "cc_host_run_system_info", CallConvs = [typeof(CallConvCdecl)])]
    public static int RunSystemInfo() => Run(ToolLauncher.RunSystemInfo);

    [UnmanagedCallersOnly(EntryPoint = "cc_host_clear_logs", CallConvs = [typeof(CallConvCdecl)])]
    public static int ClearLogs() => Run(ToolLauncher.ClearMinecraftLogs);

    [UnmanagedCallersOnly(EntryPoint = "cc_host_perform_scan", CallConvs = [typeof(CallConvCdecl)])]
    public static int PerformScan(
        delegate* unmanaged[Cdecl]<int, void*, void> onPhase,
        void* user,
        byte** linesOut)
    {
        try
        {
            var lines = Native.PerformScan(phase =>
            {
                if (onPhase != null) onPhase(phase, user);
            });
            return OkString(linesOut, string.Join("\n", lines));
        }
        catch (Exception ex)
        {
            return FailOut(linesOut, ex);
        }
    }

    [UnmanagedCallersOnly(EntryPoint = "cc_host_download_tools", CallConvs = [typeof(CallConvCdecl)])]
    public static int DownloadTools(
        byte* idsCsv,
        int force,
        delegate* unmanaged[Cdecl]<byte*, int, long, long, byte*, void*, void> cb,
        void* user)
    {
        try
        {
            var manifest = Downloader.LoadManifest();
            var ids = PtrToString(idsCsv)
                .Split(',', StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries);
            foreach (var id in ids)
            {
                var spec = manifest.Get(id);
                if (spec == null)
                {
                    Notify(cb, id, 5, 0, -1, "Нет в списке загрудок", user);
                    continue;
                }

                try
                {
                    Downloader.DownloadTool(spec, force != 0, progress =>
                    {
                        Notify(cb, id, (int)progress.Kind, progress.Received, progress.Total ?? -1, null, user);
                    });
                    Notify(cb, id, 4, 0, -1, null, user);
                }
                catch (Exception ex)
                {
                    Notify(cb, id, 5, 0, -1, ex.Message, user);
                }
            }
            return 0;
        }
        catch (Exception ex)
        {
            Fail(ex);
            return 1;
        }
    }

    static void Notify(
        delegate* unmanaged[Cdecl]<byte*, int, long, long, byte*, void*, void> cb,
        string id,
        int stage,
        long received,
        long total,
        string? err,
        void* user)
    {
        if (cb == null) return;
        var idPtr = AllocUtf8(id);
        var errPtr = err == null ? null : AllocUtf8(err);
        try
        {
            cb(idPtr, stage, received, total, errPtr, user);
        }
        finally
        {
            NativeMemory.Free(idPtr);
            if (errPtr != null) NativeMemory.Free(errPtr);
        }
    }

    [UnmanagedCallersOnly(EntryPoint = "cc_host_save_report", CallConvs = [typeof(CallConvCdecl)])]
    public static int SaveReport(byte* findings, byte** pathOut)
    {
        try
        {
            var text = PtrToString(findings);
            var lines = string.IsNullOrEmpty(text)
                ? []
                : text.Split('\n')
                    .Select(line => line.TrimEnd('\r'))
                    .Where(line => line.Length > 0)
                    .ToList();
            return OkString(pathOut, ReportWriter.Save(lines));
        }
        catch (Exception ex)
        {
            return FailOut(pathOut, ex);
        }
    }

    [UnmanagedCallersOnly(EntryPoint = "cc_host_user_name", CallConvs = [typeof(CallConvCdecl)])]
    public static int UserName(byte** dest)
    {
        try { return OkString(dest, SystemInfo.UserName); }
        catch (Exception ex) { return FailOut(dest, ex); }
    }

    [UnmanagedCallersOnly(EntryPoint = "cc_host_computer_name", CallConvs = [typeof(CallConvCdecl)])]
    public static int ComputerName(byte** dest)
    {
        try { return OkString(dest, SystemInfo.ComputerName); }
        catch (Exception ex) { return FailOut(dest, ex); }
    }

    [UnmanagedCallersOnly(EntryPoint = "cc_host_install_date", CallConvs = [typeof(CallConvCdecl)])]
    public static int InstallDate(byte** dest)
    {
        try { return OkString(dest, SystemInfo.WindowsInstallDate); }
        catch (Exception ex) { return FailOut(dest, ex); }
    }

    [UnmanagedCallersOnly(EntryPoint = "cc_host_recycle_mtime", CallConvs = [typeof(CallConvCdecl)])]
    public static int RecycleMtime(byte** dest)
    {
        try { return OkString(dest, SystemInfo.RecycleBinLastChange); }
        catch (Exception ex) { return FailOut(dest, ex); }
    }

    [UnmanagedCallersOnly(EntryPoint = "cc_host_os_info_label", CallConvs = [typeof(CallConvCdecl)])]
    public static int OsInfoLabel(byte** dest)
    {
        try { return OkString(dest, SystemInfo.OsInfoLabel); }
        catch (Exception ex) { return FailOut(dest, ex); }
    }
}
