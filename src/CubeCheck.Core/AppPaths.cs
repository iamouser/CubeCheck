using System.Diagnostics;

namespace CubeCheck;

public static class AppPaths
{
    public const string InstallDir = @"C:\Program Files\CubeCheck";

    static string? _dataDir;

    public static string ExeDir
    {
        get
        {
            return Compat.BaseDir;
        }
    }

    public static bool EnvFlag(string name)
    {
        var v = Environment.GetEnvironmentVariable(name);
        if (string.IsNullOrWhiteSpace(v)) return false;
        v = v.Trim();
        return v == "1" || v.Equals("true", StringComparison.OrdinalIgnoreCase) ||
               v.Equals("yes", StringComparison.OrdinalIgnoreCase);
    }

    public static bool IsOffline =>
        EnvFlag("CUBECHECK_OFFLINE") ||
        File.Exists(Path.Combine(ExeDir, ".offline")) ||
        File.Exists(Path.Combine(ExeDir, "assets", ".offline"));

    public static bool IsPortable
    {
        get
        {
            if (!Compat.IsWindows || IsOffline || EnvFlag("CUBECHECK_PORTABLE")) return true;
            return File.Exists(Path.Combine(ExeDir, ".portable")) ||
                   File.Exists(Path.Combine(ExeDir, "portable.txt"));
        }
    }

    public static bool ForensicToolsSupported => Compat.IsWindows;

    public static string DataDir
    {
        get
        {
            if (IsPortable) return ExeDir;
            if (_dataDir != null) return _dataDir;
            return Directory.Exists(InstallDir) ? InstallDir : ExeDir;
        }
    }

    public static string SettingsPath => Path.Combine(DataDir, "settings.json");
    public static string ReportsDir => Path.Combine(DataDir, "reports");
    public static string AssetsDir => Path.Combine(DataDir, "assets");

    public static string MinecraftDir
    {
        get
        {
            if (Compat.IsMac)
            {
                return Path.Combine(
                    Environment.GetFolderPath(Environment.SpecialFolder.UserProfile),
                    "Library", "Application Support", "minecraft");
            }
            var appdata = Environment.GetEnvironmentVariable("APPDATA");
            if (Compat.IsWindows && !string.IsNullOrEmpty(appdata))
            {
                return Path.Combine(appdata, ".minecraft");
            }
            return Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.UserProfile), ".minecraft");
        }
    }

    public static List<string> ListLegacySettingsPaths()
    {
        var dest = SettingsPath;
        var paths = new List<string>();
        void Push(string path)
        {
            if (!path.Equals(dest, StringComparison.OrdinalIgnoreCase) &&
                !paths.Exists(p => p.Equals(path, StringComparison.OrdinalIgnoreCase)))
            {
                paths.Add(path);
            }
        }
        Push(Path.Combine(ExeDir, "settings.json"));
        Push(Path.Combine(Directory.GetCurrentDirectory(), "settings.json"));
        var appdata = Environment.GetEnvironmentVariable("APPDATA");
        if (!string.IsNullOrEmpty(appdata))
        {
            var dir = Path.Combine(appdata, "CubeCheck");
            Push(Path.Combine(dir, "settings.json"));
            Push(Path.Combine(dir, "config.json"));
        }
        return paths;
    }

    public static void MigrateLegacySettings()
    {
        var dest = SettingsPath;
        if (File.Exists(dest)) return;
        var parent = Path.GetDirectoryName(dest);
        if (!string.IsNullOrEmpty(parent)) Directory.CreateDirectory(parent);
        foreach (var src in ListLegacySettingsPaths())
        {
            if (File.Exists(src))
            {
                try { File.Copy(src, dest, overwrite: false); return; }
                catch { /* next */ }
            }
        }
    }

    public static string ResourcePath(string name)
    {
        foreach (var dir in ResourceLookupDirs())
        {
            var path = Path.Combine(dir, name);
            if (File.Exists(path)) return path;
        }
        return Path.Combine(AssetsDir, name);
    }

    static IEnumerable<string> ResourceLookupDirs()
    {
        yield return AssetsDir;
        yield return Path.Combine(ExeDir, "assets");
        if (!IsPortable) yield return Path.Combine(InstallDir, "assets");
    }

    public static string ToolPath(string id) => id switch
    {
        "everything" => Path.Combine(AssetsDir, "Everything.exe"),
        "shellbag" => Path.Combine(AssetsDir, "Shellbag.exe"),
        "systeminformer" => Path.Combine(AssetsDir, "SystemInformer", "SystemInformer.exe"),
        "procmon" => Path.Combine(AssetsDir, "Procmon64.exe"),
        "autoruns" => Path.Combine(AssetsDir, "Autoruns64.exe"),
        "procexp" => Path.Combine(AssetsDir, "procexp64.exe"),
        _ => Path.Combine(AssetsDir, id)
    };

    public static bool ToolInstalled(string id)
    {
        if (!Compat.IsWindows)
        {
            if (IsOffline) return true;
            return id switch
            {
                "search" => Which.Has("fsearch", "catfish", "plocate", "locate", "mdfind", "fd", "rg", "fzf"),
                "files" => Compat.IsMac || Which.Has("xdg-open", "nautilus", "dolphin", "thunar", "lf"),
                "processes" or "procexp" => Compat.IsMac
                    || Which.Has("missioncenter", "gnome-system-monitor", "xfce4-taskmanager",
                        "plasma-systemmonitor", "ksysguard", "btop", "btm", "procs", "htop"),
                "monitor" => Compat.IsMac || Which.Has("sysdig", "fatrace", "lsof", "busybox",
                    "btop", "btm", "gnome-system-monitor", "journalctl"),
                "autoruns" => true,
                _ => Which.Has(id)
            };
        }
        var path = ToolPath(id);
        if (!File.Exists(path)) return false;
        if (id == "systeminformer") return Native.IsPeAmd64(path);
        return true;
    }

    public static bool AnyToolMissing() => Catalog.Utils.Any(u => !ToolInstalled(u.Id));

    public static void EnsureInstallDir()
    {
        if (!Compat.IsWindows || IsPortable)
        {
            EnsurePortableLayout();
            return;
        }

        var install = InstallDir;
        if (TryCreateInstall(install))
        {
            _dataDir = install;
            return;
        }

        if (!Native.IsElevated)
        {
            Native.RelaunchAsAdmin();
            Environment.Exit(0);
        }

        if (!TryCreateInstall(install))
        {
            throw new InvalidOperationException("Не удалось создать папку установки");
        }
        _dataDir = install;
    }

    static void EnsurePortableLayout()
    {
        var install = ExeDir;
        Directory.CreateDirectory(Path.Combine(install, "assets"));
        Directory.CreateDirectory(Path.Combine(install, "reports"));
        var settings = Path.Combine(install, "settings.json");
        if (!File.Exists(settings))
        {
            var def = ResourcePath("settings.default.json");
            if (File.Exists(def)) File.Copy(def, settings);
            else File.WriteAllText(settings, DefaultSettingsJson);
        }
        CopyBundledAssets(Path.Combine(install, "assets"));
        _dataDir = install;
    }

    static bool TryCreateInstall(string install)
    {
        try
        {
            var assets = Path.Combine(install, "assets");
            var reports = Path.Combine(install, "reports");
            Directory.CreateDirectory(assets);
            Directory.CreateDirectory(reports);
            MigrateLegacySettings();
            var settings = Path.Combine(install, "settings.json");
            if (!File.Exists(settings) && !ListLegacySettingsPaths().Any(File.Exists))
            {
                var def = ResourcePath("settings.default.json");
                if (File.Exists(def)) File.Copy(def, settings);
                else File.WriteAllText(settings, DefaultSettingsJson);
            }

            var exe = Compat.ProcessPath;
            if (!string.IsNullOrEmpty(exe))
            {
                var destExe = Path.Combine(install, "cubecheck.exe");
                if (InstalledExeNeedsRefresh(exe, destExe))
                {
                    try { File.Copy(exe, destExe, overwrite: true); } catch { /* ignore */ }
                }
            }

            CopyBundledAssets(assets);
            GrantUsersModify(install, inherit: true);
            GrantUsersModify(assets, inherit: true);
            GrantUsersModify(reports, inherit: true);
            GrantUsersModify(settings, inherit: false);
            return true;
        }
        catch
        {
            return false;
        }
    }

    static void CopyBundledAssets(string destAssets)
    {
        var bundled = Path.Combine(ExeDir, "assets");
        foreach (var name in new[] { "tools.json", "cubecheck.ico", "settings.default.json" })
        {
            var dest = Path.Combine(destAssets, name);
            if (File.Exists(dest)) continue;
            var from = Path.Combine(bundled, name);
            if (File.Exists(from))
            {
                try { File.Copy(from, dest); } catch { /* ignore */ }
            }
        }
    }

    static bool InstalledExeNeedsRefresh(string src, string dest)
    {
        try
        {
            if (string.Equals(Path.GetFullPath(src), Path.GetFullPath(dest), StringComparison.OrdinalIgnoreCase))
            {
                return false;
            }
        }
        catch
        {
            // compare as-is
        }
        if (!File.Exists(dest)) return true;
        try
        {
            return File.GetLastWriteTimeUtc(src) > File.GetLastWriteTimeUtc(dest);
        }
        catch
        {
            return false;
        }
    }

    static void GrantUsersModify(string path, bool inherit)
    {
        try
        {
            var grant = inherit ? "*S-1-5-32-545:(OI)(CI)M" : "*S-1-5-32-545:M";
            using var p = Process.Start(new ProcessStartInfo
            {
                FileName = "icacls",
                Arguments = $"\"{path}\" /grant {grant} /C",
                CreateNoWindow = true,
                UseShellExecute = false
            });
            p?.WaitForExit(4000);
        }
        catch
        {
            // ignore
        }
    }

    public const string DefaultSettingsJson =
        """
        {
          "theme": "black",
          "zoom": 1.0,
          "glow": {
            "enabled": true,
            "color": [212, 175, 55],
            "color2": [255, 214, 90],
            "gradient": false,
            "gradient_speed": 1.0,
            "radius": 34.0,
            "intensity": 1.0,
            "areas": {
              "sidebar": true,
              "about": true,
              "system": true,
              "footer": true
            }
          },
          "autosave": "on_change"
        }
        """;
}
