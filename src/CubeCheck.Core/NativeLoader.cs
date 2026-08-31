#if !NETFRAMEWORK
using System.Reflection;
#endif
using System.Runtime.InteropServices;

namespace CubeCheck;

/// <summary>
/// Resolves cubecheck_native from assets/ (install layout) or the exe directory (old installs).
/// </summary>
public static class NativeLoader
{
    static int _ready;

    public static void Ensure()
    {
        if (Interlocked.Exchange(ref _ready, 1) != 0) return;

#if NETFRAMEWORK
        TrySetDllDirectory(FirstSearchDir());
#else
        NativeLibrary.SetDllImportResolver(typeof(Native).Assembly, Resolve);
#endif
    }

#if !NETFRAMEWORK
    [System.Runtime.CompilerServices.ModuleInitializer]
    internal static void Auto() => Ensure();

    static IntPtr Resolve(string libraryName, Assembly _, DllImportSearchPath? __)
    {
        if (libraryName.IndexOf("cubecheck_native", StringComparison.OrdinalIgnoreCase) < 0)
        {
            return IntPtr.Zero;
        }

        foreach (var path in CandidateFiles())
        {
            if (!File.Exists(path)) continue;
            if (NativeLibrary.TryLoad(path, out var handle)) return handle;
        }

        return IntPtr.Zero;
    }
#endif

    static IEnumerable<string> CandidateFiles()
    {
        var names = new[]
        {
            "cubecheck_native.dll",
            "libcubecheck_native.so",
            "cubecheck_native.so",
            "libcubecheck_native.dylib",
            "cubecheck_native.dylib"
        };

        var env = Environment.GetEnvironmentVariable("CUBECHECK_NATIVE_DLL");
        if (!string.IsNullOrWhiteSpace(env)) yield return env.Trim();

        foreach (var dir in SearchDirs())
        {
            foreach (var name in names)
            {
                yield return Path.Combine(dir, name);
            }
        }
    }

    static IEnumerable<string> SearchDirs()
    {
        var seen = new HashSet<string>(StringComparer.OrdinalIgnoreCase);

        IEnumerable<string> Once(string? dir)
        {
            if (string.IsNullOrWhiteSpace(dir)) yield break;
            string full;
            try { full = Path.GetFullPath(dir); }
            catch { yield break; }
            if (!seen.Add(full)) yield break;
            yield return full;
        }

        foreach (var dir in Once(Path.Combine(AppContext.BaseDirectory, "assets"))) yield return dir;
        foreach (var dir in Once(AppContext.BaseDirectory)) yield return dir;

        var exeDir = Compat.BaseDir;
        foreach (var dir in Once(Path.Combine(exeDir, "assets"))) yield return dir;
        foreach (var dir in Once(exeDir)) yield return dir;

        foreach (var dir in Once(Path.Combine(AppPaths.InstallDir, "assets"))) yield return dir;
        foreach (var dir in Once(AppPaths.InstallDir)) yield return dir;

        var cwd = Directory.GetCurrentDirectory();
        foreach (var dir in Once(Path.Combine(cwd, "assets"))) yield return dir;
        foreach (var dir in Once(cwd)) yield return dir;
    }

    static string? FirstSearchDir()
    {
        foreach (var dir in SearchDirs())
        {
            if (Directory.Exists(dir)) return dir;
        }
        return null;
    }

#if NETFRAMEWORK
    [DllImport("kernel32", CharSet = CharSet.Unicode, SetLastError = true)]
    static extern bool SetDllDirectory(string lpPathName);

    static void TrySetDllDirectory(string? dir)
    {
        if (string.IsNullOrEmpty(dir)) return;
        try { SetDllDirectory(dir); }
        catch { /* leftover net48 host only */ }
    }
#endif
}
