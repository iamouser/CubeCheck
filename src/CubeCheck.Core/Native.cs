using System.Runtime.InteropServices;
using System.Text;

[assembly: DefaultDllImportSearchPaths(DllImportSearchPath.AssemblyDirectory | DllImportSearchPath.SafeDirectories)]

namespace CubeCheck;

public static class Native
{
    const string Dll = "cubecheck_native";

    static Native() => NativeLoader.Ensure();

    [UnmanagedFunctionPointer(CallingConvention.Cdecl, CharSet = CharSet.Unicode)]
    delegate void PhaseCb(int phase, IntPtr user);

    [UnmanagedFunctionPointer(CallingConvention.Cdecl, CharSet = CharSet.Unicode)]
    delegate void LineCb(string line, IntPtr user);

    [DllImport(Dll, CallingConvention = CallingConvention.Cdecl)]
    static extern int cc_is_elevated();

    [DllImport(Dll, CallingConvention = CallingConvention.Cdecl, CharSet = CharSet.Unicode)]
    static extern int cc_is_pe_amd64(string path);

    [DllImport(Dll, CallingConvention = CallingConvention.Cdecl, CharSet = CharSet.Unicode)]
    static extern int cc_verify_publisher(string path, string expected, StringBuilder err, int errCch);

    [DllImport(Dll, CallingConvention = CallingConvention.Cdecl, CharSet = CharSet.Unicode)]
    static extern int cc_shell_execute(string path, string? dir, string? verb, string? paramsArgs);

    [DllImport(Dll, CallingConvention = CallingConvention.Cdecl, CharSet = CharSet.Unicode)]
    static extern int cc_create_shortcut(string link, string target, string? workdir, string? icon);

    [DllImport(Dll, CallingConvention = CallingConvention.Cdecl, CharSet = CharSet.Unicode)]
    static extern int cc_install_shortcuts(string target, string workdir, string? icon);

    [DllImport(Dll, CallingConvention = CallingConvention.Cdecl)]
    static extern int cc_relaunch_as_admin();

    [DllImport(Dll, CallingConvention = CallingConvention.Cdecl, CharSet = CharSet.Unicode)]
    static extern void cc_message_box(string title, string text);

    [DllImport(Dll, CallingConvention = CallingConvention.Cdecl)]
    static extern int cc_perform_scan(PhaseCb? onPhase, LineCb? onLine, IntPtr user);

    [DllImport(Dll, CallingConvention = CallingConvention.Cdecl, CharSet = CharSet.Unicode)]
    static extern int cc_get_install_date(StringBuilder buf, int cch);

    [DllImport(Dll, CallingConvention = CallingConvention.Cdecl, CharSet = CharSet.Unicode)]
    static extern int cc_get_recycle_mtime(StringBuilder buf, int cch);

    [DllImport(Dll, CallingConvention = CallingConvention.Cdecl, CharSet = CharSet.Unicode)]
    static extern int cc_known_folder(int which, StringBuilder buf, int cch);

    public static bool IsElevated => cc_is_elevated() != 0;

    public static bool IsPeAmd64(string path) => cc_is_pe_amd64(path) != 0;

    public static void VerifyPublisher(string path, string expected)
    {
        var err = new StringBuilder(512);
        if (cc_verify_publisher(path, expected, err, err.Capacity) != 0)
        {
            throw new InvalidOperationException(err.ToString());
        }
    }

    public static void ShellExecute(string path, string? dir, string verb = "open", string? args = null)
    {
        if (cc_shell_execute(path, dir, verb, args ?? "") != 0)
        {
            throw new InvalidOperationException($"Не удалось запустить {path}");
        }
    }

    public static void InstallShortcuts(string target, string workdir, string? icon)
    {
        if (cc_install_shortcuts(target, workdir, icon) != 0)
        {
            throw new InvalidOperationException("Не удалось создать ярлык на рабочем столе");
        }
    }

    public static void CreateShortcut(string link, string target, string? workdir, string? icon)
    {
        if (cc_create_shortcut(link, target, workdir, icon) != 0)
        {
            throw new InvalidOperationException($"Не удалось создать ярлык {link}");
        }
    }

    public static void RelaunchAsAdmin()
    {
        if (cc_relaunch_as_admin() != 0)
        {
            throw new InvalidOperationException("Нужны права администратора, чтобы создать папку установки");
        }
    }

    public static void MessageBox(string title, string text) => cc_message_box(title, text);

    public static List<string> PerformScan(Action<int>? onPhase)
    {
        if (!Compat.IsWindows)
        {
            return PosixScan.Run(onPhase);
        }

        var lines = new List<string>();
        PhaseCb phase = (p, _) => onPhase?.Invoke(p);
        LineCb line = (s, _) =>
        {
            if (!string.IsNullOrEmpty(s)) lines.Add(s);
        };
        GC.KeepAlive(phase);
        GC.KeepAlive(line);
        cc_perform_scan(phase, line, IntPtr.Zero);
        return lines;
    }

    public static string InstallDate()
    {
        if (!Compat.IsWindows) return PosixScan.OsDescription();
        var buf = new StringBuilder(64);
        cc_get_install_date(buf, buf.Capacity);
        return buf.ToString();
    }

    public static string RecycleMtime()
    {
        if (!Compat.IsWindows) return PosixScan.RecycleMtime();
        var buf = new StringBuilder(64);
        cc_get_recycle_mtime(buf, buf.Capacity);
        return buf.ToString();
    }

    public static string? KnownFolder(int which)
    {
        var buf = new StringBuilder(260);
        return cc_known_folder(which, buf, buf.Capacity) == 0 ? buf.ToString() : null;
    }
}
