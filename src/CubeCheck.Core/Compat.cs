using System.Reflection;
using System.Runtime.InteropServices;

namespace CubeCheck;

public static class Compat
{
    public static string ProcessPath
    {
        get
        {
#if NETFRAMEWORK
            var entry = Assembly.GetEntryAssembly()?.Location;
            if (!string.IsNullOrEmpty(entry)) return entry;
            return Path.Combine(AppDomain.CurrentDomain.BaseDirectory, "cubecheck.exe");
#else
            return Environment.ProcessPath ?? Path.Combine(AppContext.BaseDirectory, "cubecheck");
#endif
        }
    }

    public static string BaseDir =>
        Path.GetDirectoryName(ProcessPath) ??
#if NETFRAMEWORK
            AppDomain.CurrentDomain.BaseDirectory;
#else
            AppContext.BaseDirectory;
#endif

    public static bool IsWindows =>
#if NETFRAMEWORK
        Environment.OSVersion.Platform == PlatformID.Win32NT;
#else
        OperatingSystem.IsWindows();
#endif

    public static bool IsLinux =>
#if NETFRAMEWORK
        false;
#else
        OperatingSystem.IsLinux();
#endif

    public static bool IsMac =>
#if NETFRAMEWORK
        false;
#else
        OperatingSystem.IsMacOS();
#endif

    public static bool IsUnix => IsLinux || IsMac;

    public static bool IsOsPlatform(OSPlatform platform) =>
        RuntimeInformation.IsOSPlatform(platform);
}
