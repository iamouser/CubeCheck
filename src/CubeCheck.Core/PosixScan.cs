using System.Diagnostics;
using System.Runtime.InteropServices;

namespace CubeCheck;

public static class PosixScan
{
    static readonly string[] LogSuspicious =
    [
        "inject", "injection", "hook", "hooked", "bypass", "cheat", "hack", "loader", "fatal", "hitbox",
        "expand", "reach", "aimbot", "killaura", "autoclick", "fly", "speed"
    ];

    static readonly string[] Inject =
    [
        "inject", "injection", "hooked", "hook", "dll", "loader", "native", "jni", "agent", "attached",
        "transform", "classloader", "modify", "bytecode", "asm"
    ];

    static readonly string[] Hitbox =
    [
        "hitbox", "expand", "reach", "aimbot", "killaura", "autoclick", "fly", "speed"
    ];

    static readonly string[] KnownDll =
    [
        "lwjgl", "jinput", "openal", "javaw", "msvcp", "vcruntime"
    ];

    static readonly string[] Ignored =
    [
        "systemd", "init", "kthreadd", "chrome", "firefox", "discord", "telegram", "steam",
        "Finder", "WindowServer", "kernel_task", "cubecheck"
    ];

    public static List<string> Run(Action<int>? onPhase)
    {
        var lines = new List<string>();
        onPhase?.Invoke(0);
        ScanProcesses(lines);
        onPhase?.Invoke(1);
        ScanFiles(lines);
        onPhase?.Invoke(2);
        ScanStartup(lines);
        onPhase?.Invoke(3);
        ScanLogs(lines);
        ScanHitbox(lines);
        ScanUnknownDlls(lines);
        lines.Add("Корзина: " + RecycleMtime());
        return lines;
    }

    public static string OsDescription()
    {
        if (Compat.IsMac)
        {
            var ver = ReadFirstLine("/System/Library/CoreServices/SystemVersion.plist") ?? "";
            var product = RunOut("sw_vers", "-productVersion") ?? RuntimeInformation.OSDescription;
            return string.IsNullOrWhiteSpace(product) ? "macOS" : "macOS " + product.Trim();
        }

        foreach (var path in new[] { "/etc/os-release", "/usr/lib/os-release" })
        {
            if (!File.Exists(path)) continue;
            foreach (var line in File.ReadLines(path))
            {
                if (line.StartsWith("PRETTY_NAME=", StringComparison.Ordinal))
                {
                    return line.Substring("PRETTY_NAME=".Length).Trim().Trim('"');
                }
            }
        }
        return RuntimeInformation.OSDescription;
    }

    public static string RecycleMtime()
    {
        var dirs = RecycleDirs();
        DateTime? best = null;
        foreach (var dir in dirs)
        {
            try
            {
                if (!Directory.Exists(dir)) continue;
                var t = Directory.GetLastWriteTime(dir);
                if (best == null || t > best) best = t;
            }
            catch
            {
                // skip
            }
        }
        return best == null ? "нет данных" : best.Value.ToString("dd.MM.yyyy HH:mm");
    }

    public static IEnumerable<string> RecycleDirs()
    {
        var home = Environment.GetFolderPath(Environment.SpecialFolder.UserProfile);
        if (Compat.IsMac)
        {
            yield return Path.Combine(home, ".Trash");
            yield break;
        }
        yield return Path.Combine(home, ".local", "share", "Trash");
        yield return Path.Combine(home, ".local", "share", "Trash", "files");
    }

    static void ScanProcesses(List<string> lines)
    {
        foreach (var p in Process.GetProcesses())
        {
            try
            {
                var name = p.ProcessName;
                if (IsIgnored(name)) continue;
                var exe = "";
                try { exe = p.MainModule?.FileName ?? ""; } catch { /* denied */ }
                var hay = name + " " + exe;
                if (ContainsCheat(hay))
                {
                    lines.Add($"Процесс: {name} (путь: {exe})");
                }
            }
            catch
            {
                // skip
            }
            finally
            {
                p.Dispose();
            }
        }
    }

    static void ScanFiles(List<string> lines)
    {
        foreach (var folder in FileRoots())
        {
            Walk(folder, 0, 2, path =>
            {
                if (ContainsCheat(Path.GetFileName(path)))
                {
                    lines.Add("Файл: " + path);
                }
            });
        }
    }

    static void ScanStartup(List<string> lines)
    {
        foreach (var file in StartupFiles())
        {
            try
            {
                var name = Path.GetFileName(file);
                var text = File.Exists(file) ? File.ReadAllText(file) : "";
                if (ContainsCheat(name) || ContainsCheat(text))
                {
                    lines.Add("Автозагрузка: " + file);
                }
            }
            catch
            {
                // skip
            }
        }
    }

    static void ScanLogs(List<string> lines)
    {
        var path = Path.Combine(AppPaths.MinecraftDir, "logs", "latest.log");
        if (!File.Exists(path)) return;
        string[] all;
        try { all = File.ReadAllLines(path); }
        catch { return; }
        var start = all.Length > 300 ? all.Length - 300 : 0;
        var found = new List<string>();
        for (var i = start; i < all.Length; i++)
        {
            if (FirstMatch(all[i], Inject) is { } inj)
            {
                found.Add("В логах: " + inj);
                continue;
            }
            if (FirstMatch(all[i], LogSuspicious) is { } sus)
            {
                found.Add("В логах: " + sus);
            }
        }
        if (found.Count == 0) return;
        lines.Add("Логи Minecraft:");
        foreach (var item in found.Take(5)) lines.Add("   " + item);
    }

    static void ScanHitbox(List<string> lines)
    {
        var found = new List<string>();
        foreach (var folder in new[]
                 {
                     Path.Combine(AppPaths.MinecraftDir, "mods"),
                     Path.Combine(AppPaths.MinecraftDir, "versions"),
                     DesktopDir(),
                     DownloadsDir()
                 })
        {
            Walk(folder, 0, 2, path =>
            {
                if (FirstMatch(Path.GetFileName(path), Hitbox) is { } pat)
                {
                    found.Add($"Подозрительный файл ({pat}): {path}");
                }
            });
        }
        if (found.Count == 0) return;
        lines.Add("Подозрительные файлы:");
        foreach (var item in found.Take(5)) lines.Add("   " + item);
    }

    static void ScanUnknownDlls(List<string> lines)
    {
        var found = new List<string>();
        foreach (var folder in new[]
                 {
                     Path.Combine(AppPaths.MinecraftDir, "bin"),
                     Path.Combine(AppPaths.MinecraftDir, "versions")
                 })
        {
            Walk(folder, 0, 2, path =>
            {
                if (!path.EndsWith(".dll", StringComparison.OrdinalIgnoreCase) &&
                    !path.EndsWith(".so", StringComparison.OrdinalIgnoreCase) &&
                    !path.EndsWith(".dylib", StringComparison.OrdinalIgnoreCase))
                {
                    return;
                }
                var name = Path.GetFileName(path).ToLowerInvariant();
                if (KnownDll.Any(k => name.IndexOf(k, StringComparison.Ordinal) >= 0)) return;
                found.Add("Неизвестный .dll: " + path);
            });
        }
        if (found.Count == 0) return;
        lines.Add("DLL:");
        foreach (var item in found.Take(5)) lines.Add("   " + item);
    }

    static IEnumerable<string> FileRoots()
    {
        yield return Path.Combine(AppPaths.MinecraftDir, "versions");
        yield return Path.Combine(AppPaths.MinecraftDir, "mods");
        yield return DesktopDir();
        yield return DownloadsDir();
        yield return Path.GetTempPath();
    }

    static IEnumerable<string> StartupFiles()
    {
        var home = Environment.GetFolderPath(Environment.SpecialFolder.UserProfile);
        if (Compat.IsMac)
        {
            foreach (var dir in new[]
                     {
                         Path.Combine(home, "Library", "LaunchAgents"),
                         "/Library/LaunchAgents",
                         "/Library/LaunchDaemons"
                     })
            {
                if (!Directory.Exists(dir)) continue;
                foreach (var f in Directory.EnumerateFiles(dir)) yield return f;
            }
            yield break;
        }

        var autostart = Path.Combine(home, ".config", "autostart");
        if (Directory.Exists(autostart))
        {
            foreach (var f in Directory.EnumerateFiles(autostart)) yield return f;
        }
        var systemd = Path.Combine(home, ".config", "systemd", "user");
        if (Directory.Exists(systemd))
        {
            foreach (var f in Directory.EnumerateFiles(systemd, "*.service")) yield return f;
        }
    }

    static string DesktopDir()
    {
        var home = Environment.GetFolderPath(Environment.SpecialFolder.UserProfile);
        var xdg = Environment.GetEnvironmentVariable("XDG_DESKTOP_DIR");
        if (!string.IsNullOrWhiteSpace(xdg)) return xdg.Trim();
        return Path.Combine(home, "Desktop");
    }

    static string DownloadsDir()
    {
        var home = Environment.GetFolderPath(Environment.SpecialFolder.UserProfile);
        var xdg = Environment.GetEnvironmentVariable("XDG_DOWNLOAD_DIR");
        if (!string.IsNullOrWhiteSpace(xdg)) return xdg.Trim();
        return Path.Combine(home, "Downloads");
    }

    static void Walk(string dir, int depth, int maxDepth, Action<string> fn)
    {
        if (depth > maxDepth || !Directory.Exists(dir)) return;
        IEnumerable<string> entries;
        try { entries = Directory.EnumerateFileSystemEntries(dir); }
        catch { return; }
        foreach (var entry in entries)
        {
            try
            {
                if (Directory.Exists(entry)) Walk(entry, depth + 1, maxDepth, fn);
                else fn(entry);
            }
            catch
            {
                // skip
            }
        }
    }

    static bool ContainsCheat(string text)
    {
        var hay = text.ToLowerInvariant();
        return Catalog.CheatNames.Any(n => hay.IndexOf(n.ToLowerInvariant(), StringComparison.Ordinal) >= 0);
    }

    static bool IsIgnored(string name)
    {
        var n = name.ToLowerInvariant();
        return Ignored.Any(i => n.Equals(i, StringComparison.OrdinalIgnoreCase) ||
                                n.IndexOf(i, StringComparison.OrdinalIgnoreCase) >= 0);
    }

    static string? FirstMatch(string text, string[] pats)
    {
        var hay = text.ToLowerInvariant();
        return pats.FirstOrDefault(p => hay.IndexOf(p, StringComparison.Ordinal) >= 0);
    }

    static string? ReadFirstLine(string path)
    {
        try { return File.Exists(path) ? File.ReadLines(path).FirstOrDefault() : null; }
        catch { return null; }
    }

    static string? RunOut(string file, string args)
    {
        try
        {
            var p = Process.Start(new ProcessStartInfo
            {
                FileName = file,
                Arguments = args,
                RedirectStandardOutput = true,
                UseShellExecute = false,
                CreateNoWindow = true
            });
            if (p == null) return null;
            var text = p.StandardOutput.ReadToEnd();
            p.WaitForExit(2000);
            return text;
        }
        catch
        {
            return null;
        }
    }
}
