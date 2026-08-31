using System.IO;
using System.Reflection;
using System.Text.Json;
using System.Text.Json.Serialization;

namespace CubeCheck.Installer;

public sealed class SetupFile
{
    [JsonPropertyName("url")] public string Url { get; set; } = "";
    [JsonPropertyName("version")] public string Version { get; set; } = "1.1.0-beta";
    [JsonPropertyName("product")] public string Product { get; set; } = "CubeCheck";
    [JsonPropertyName("authors")] public string Authors { get; set; } = "AuraStudio, AnProject";
}

public sealed class InstallOptions
{
    public string Destination { get; set; } = "";
    public bool DesktopShortcut { get; set; } = true;
    public bool MenuShortcut { get; set; } = true;
    public bool LaunchAfter { get; set; } = true;
    public bool LicenseAccepted { get; set; }
}

public static class InstallerConfig
{
    public const string Product = "CubeCheck";
    public const string VersionLabel = "1.1 beta";
    public const string Authors = "AuraStudio, AnProject";
    public const string DefaultPayloadUrl =
        "https://github.com/jumpworlds/CubeCheck-payload/archive/refs/heads/main.zip";

    public const string EmbeddedPayloadName = "CubeCheck.Installer.payload.zip";

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

    static readonly JsonSerializerOptions JsonOpts = new()
    {
        PropertyNameCaseInsensitive = true,
        ReadCommentHandling = JsonCommentHandling.Skip,
        AllowTrailingCommas = true
    };

    public static string ExeDir
    {
        get
        {
            var path = Environment.ProcessPath;
            if (!string.IsNullOrEmpty(path))
            {
                var dir = Path.GetDirectoryName(path);
                if (!string.IsNullOrEmpty(dir)) return dir;
            }
            return AppContext.BaseDirectory;
        }
    }

    public static SetupFile LoadSetupFile()
    {
        foreach (var path in EnumerateSetupJsonPaths())
        {
            try
            {
                if (!File.Exists(path)) continue;
                var parsed = JsonSerializer.Deserialize<SetupFile>(File.ReadAllText(path), JsonOpts);
                if (parsed != null) return parsed;
            }
            catch
            {
                // next candidate
            }
        }
        return new SetupFile { Url = DefaultPayloadUrl };
    }

    public static string PayloadUrl()
    {
        if (IsOfflineSetup)
        {
            throw new InvalidOperationException(
                "Офлайн-установщик не загружает файлы из сети.");
        }

        var env = Environment.GetEnvironmentVariable("CUBECHECK_PAYLOAD_URL");
        var raw = !string.IsNullOrWhiteSpace(env) ? env.Trim() : LoadSetupFile().Url?.Trim();
        if (string.IsNullOrWhiteSpace(raw)) raw = DefaultPayloadUrl;
        return RequireHttps(raw);
    }

    public static bool IsOfflineSetup
    {
        get
        {
#if CUBECHECK_OFFLINE_SETUP
            return true;
#else
            if (EnvFlag("CUBECHECK_OFFLINE")) return true;
            var exeName = Path.GetFileName(Environment.ProcessPath ?? "");
            if (exeName.Contains("offline", StringComparison.OrdinalIgnoreCase)) return true;
            if (File.Exists(Path.Combine(ExeDir, ".offline"))) return true;
            if (HasEmbeddedPayload()) return true;
            return false;
#endif
        }
    }

    public static string WizardSubtitle =>
        IsOfflineSetup ? "Мастер установки (офлайн)" : "Мастер установки";

    public static bool HasEmbeddedPayload()
    {
        try
        {
            var asm = Assembly.GetExecutingAssembly();
            return asm.GetManifestResourceNames().Any(IsEmbeddedPayloadName);
        }
        catch
        {
            return false;
        }
    }

    public static bool IsEmbeddedPayloadName(string name) =>
        name.Equals(EmbeddedPayloadName, StringComparison.OrdinalIgnoreCase) ||
        name.EndsWith(".payload.zip", StringComparison.OrdinalIgnoreCase);

    public static IEnumerable<string> EnumerateSidecarZips()
    {
        var dir = ExeDir;
        var exeName = Path.GetFileNameWithoutExtension(Environment.ProcessPath ?? "CubeCheck-Setup");
        yield return Path.Combine(dir, "payload.zip");
        yield return Path.Combine(dir, exeName + "-payload.zip");
        yield return Path.Combine(dir, exeName + ".payload.zip");
        yield return Path.Combine(dir, "CubeCheck-payload.zip");
        foreach (var file in SafeDirFiles(dir))
        {
            var name = Path.GetFileName(file);
            if (name.EndsWith(".zip", StringComparison.OrdinalIgnoreCase) &&
                (name.Contains("payload", StringComparison.OrdinalIgnoreCase) ||
                 name.Contains("offline", StringComparison.OrdinalIgnoreCase)))
            {
                yield return file;
            }
        }
    }

    static IEnumerable<string> SafeDirFiles(string dir)
    {
        if (!Directory.Exists(dir)) yield break;
        foreach (var file in Directory.GetFiles(dir, "*.zip"))
        {
            yield return file;
        }
    }

    static bool EnvFlag(string name)
    {
        var v = Environment.GetEnvironmentVariable(name);
        if (string.IsNullOrWhiteSpace(v)) return false;
        return v != "0" &&
               !v.Equals("false", StringComparison.OrdinalIgnoreCase) &&
               !v.Equals("no", StringComparison.OrdinalIgnoreCase);
    }

    public static string RequireHttps(string url)
    {
        if (!Uri.TryCreate(url, UriKind.Absolute, out var uri) ||
            !string.Equals(uri.Scheme, Uri.UriSchemeHttps, StringComparison.OrdinalIgnoreCase))
        {
            throw new InvalidOperationException("Разрешена только загрузка по HTTPS.");
        }
        return uri.AbsoluteUri;
    }

    public static string DefaultDestination()
    {
        if (OperatingSystem.IsWindows()) return @"C:\Program Files\CubeCheck";
        if (OperatingSystem.IsMacOS()) return Path.Combine("/", "Applications", Product);
        if (CanUseOpt()) return Path.Combine("/", "opt", Product);
        var home = Environment.GetFolderPath(Environment.SpecialFolder.UserProfile);
        return Path.Combine(home, ".local", "share", Product);
    }

    public static string MenuShortcutLabel()
    {
        if (OperatingSystem.IsMacOS()) return "Псевдоним в Applications";
        if (OperatingSystem.IsLinux()) return "Файл .desktop в меню приложений";
        return "Ярлык в меню Пуск";
    }

    public static bool CanWriteDirectory(string dir)
    {
        try
        {
            Directory.CreateDirectory(dir);
            var test = Path.Combine(dir, ".cubecheck_install_test");
            File.WriteAllText(test, "ok");
            File.Delete(test);
            return true;
        }
        catch
        {
            return false;
        }
    }

    public static bool NeedsElevation(string dest)
    {
        if (CanWriteDirectory(dest)) return false;
        if (OperatingSystem.IsWindows()) return true;
        if (OperatingSystem.IsLinux() && dest.StartsWith("/opt", StringComparison.Ordinal)) return true;
        if (OperatingSystem.IsMacOS() && dest.StartsWith("/Applications", StringComparison.Ordinal)) return true;
        return false;
    }

    static bool CanUseOpt()
    {
        if (IsRoot()) return true;
        return CanWriteDirectory("/opt") || CanWriteDirectory("/opt/CubeCheck");
    }

    static bool IsRoot()
    {
        try
        {
            if (OperatingSystem.IsWindows()) return false;
            return Environment.GetEnvironmentVariable("USER") == "root" ||
                   Environment.GetEnvironmentVariable("EUID") == "0" ||
                   Environment.GetEnvironmentVariable("UID") == "0";
        }
        catch
        {
            return false;
        }
    }

    static IEnumerable<string> EnumerateSetupJsonPaths()
    {
        yield return Path.Combine(ExeDir, "setup.json");
        yield return Path.Combine(AppContext.BaseDirectory, "setup.json");
        var parent = Directory.GetParent(ExeDir)?.FullName;
        if (!string.IsNullOrEmpty(parent))
        {
            yield return Path.Combine(parent, "setup.json");
        }
    }
}
