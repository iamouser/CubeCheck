using System.Diagnostics;
using System.IO;
using System.IO.Compression;
using System.Net.Http;
using System.Reflection;
using System.Runtime.InteropServices;

namespace CubeCheck.Installer;

public readonly record struct InstallProgress(double Percent, string Status, string CurrentFile);

static class InstallerEngine
{
    const string UserAgent = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) CubeCheck-Setup/1.1-beta";

    static readonly HashSet<string> SkipNamesAlways = new(StringComparer.OrdinalIgnoreCase)
    {
        "CubeCheck-Setup.exe",
        "CubeCheck-1.1.0-beta-setup.exe",
        "cubecheck-setup.exe",
        "cubecheck-launcher.exe",
        "setup.json"
    };

    static readonly HashSet<string> SkipVendorFiles = new(StringComparer.OrdinalIgnoreCase)
    {
        "Everything.exe",
        "Shellbag.exe",
        "Procmon64.exe",
        "Procmon.exe",
        "Autoruns64.exe",
        "Autoruns.exe",
        "procexp64.exe",
        "procexp.exe"
    };

    static readonly HashSet<string> SkipDirsAlways = new(StringComparer.OrdinalIgnoreCase)
    {
        "__MACOSX",
        ".git"
    };

    static readonly HashSet<string> SkipDirsOnline = new(StringComparer.OrdinalIgnoreCase)
    {
        "SystemInformer",
        "extras"
    };

    /// <summary>
    /// Live only under assets/. A leaking zip or an older install must not leave them at dest root.
    /// </summary>
    static readonly HashSet<string> AssetsOnlyNames = new(StringComparer.OrdinalIgnoreCase)
    {
        "cubecheck_api.dll",
        "cubecheck_native.dll",
        "UnInstall.ico",
        "UnInstall.cmd"
    };

    public static async Task InstallAsync(
        InstallOptions options,
        IProgress<InstallProgress> progress,
        CancellationToken ct)
    {
        ArgumentException.ThrowIfNullOrWhiteSpace(options.Destination);
        var dest = Path.GetFullPath(options.Destination);
        Directory.CreateDirectory(dest);
        StopRunningCubecheck();

        var offline = InstallerConfig.IsOfflineSetup;
        var temp = CreateTempDir();
        try
        {
            string zip;
            if (offline)
            {
                progress.Report(new InstallProgress(0, "Локальный пакет", "без загрузки из сети"));
                zip = await MaterializeLocalPayloadAsync(temp, progress, ct).ConfigureAwait(false);
            }
            else
            {
                var url = InstallerConfig.PayloadUrl();
                progress.Report(new InstallProgress(0, "Загрузка", url));
                zip = Path.Combine(temp, "download.zip");
                await DownloadAsync(url, zip, progress, ct).ConfigureAwait(false);
            }
            var extracted = Path.Combine(temp, "extracted");
            ExtractZip(zip, extracted, progress, ct);
            var payloadRoot = FindOsPayload(UnwrapGithubPrefix(extracted));
            CopyTree(payloadRoot, dest, progress, ct);
            FinishLayout(dest, offline);
            progress.Report(new InstallProgress(96, "Ярлыки", ""));
            try
            {
                Shortcuts.Create(dest, options.DesktopShortcut, options.MenuShortcut);
            }
            catch
            {
                // shortcuts must not fail the install
            }

            try
            {
                UninstallFiles.Write(dest);
            }
            catch
            {
                // uninstall helper must not fail the install
            }

            if (OperatingSystem.IsWindows())
            {
                GrantUsersModify(dest, true);
                GrantUsersModify(Path.Combine(dest, "settings.json"), false);
                GrantUsersModify(Path.Combine(dest, "reports"), true);
                GrantUsersModify(Path.Combine(dest, "assets"), true);
            }

            progress.Report(new InstallProgress(100, "Готово", ""));
        }
        finally
        {
            try { Directory.Delete(temp, true); } catch { /* ignore */ }
        }
    }

    public static void Launch(string dest)
    {
        var exe = Shortcuts.ResolveAppBinary(dest);
        if (string.IsNullOrEmpty(exe) || !File.Exists(exe))
        {
            throw new InvalidOperationException($"Установка не записала программу: {dest}");
        }

        Process.Start(new ProcessStartInfo
        {
            FileName = exe,
            WorkingDirectory = dest,
            UseShellExecute = true
        });
    }

    /// <summary>
    /// GitHub source zip is CubeCheck-payload-main/... — drop that single top folder.
    /// </summary>
    public static string UnwrapGithubPrefix(string extracted)
    {
        if (!Directory.Exists(extracted)) return extracted;
        var dirs = Directory.GetDirectories(extracted);
        var files = Directory.GetFiles(extracted);
        if (dirs.Length != 1 || files.Length != 0) return extracted;

        var name = Path.GetFileName(dirs[0]);
        if (name.Equals("CubeCheck-payload-main", StringComparison.OrdinalIgnoreCase) ||
            name.StartsWith("CubeCheck-payload-", StringComparison.OrdinalIgnoreCase) ||
            name.EndsWith("-main", StringComparison.OrdinalIgnoreCase) ||
            name.EndsWith("-master", StringComparison.OrdinalIgnoreCase))
        {
            return dirs[0];
        }

        if (LooksLikePayloadTree(dirs[0]) || LooksLikeOsPayload(dirs[0]))
        {
            return dirs[0];
        }

        return extracted;
    }

    public static string FindOsPayload(string root)
    {
        foreach (var key in OsPayloadKeys())
        {
            var direct = Path.Combine(root, key);
            if (LooksLikeOsPayload(direct)) return direct;
            var nested = Path.Combine(root, "payload", key);
            if (LooksLikeOsPayload(nested)) return nested;
            foreach (var sub in SafeDirs(root))
            {
                var inner = Path.Combine(sub, key);
                if (LooksLikeOsPayload(inner)) return inner;
                var innerPayload = Path.Combine(sub, "payload", key);
                if (LooksLikeOsPayload(innerPayload)) return innerPayload;
            }
        }

        if (LooksLikeOsPayload(root)) return root;
        foreach (var sub in SafeDirs(root))
        {
            if (LooksLikeOsPayload(sub)) return sub;
        }

        throw new InvalidOperationException(
            $"В архиве нет payload для этой ОС ({RuntimeInformation.OSDescription}). Нужны папки windows-x64 / linux-x64 / osx-*.");
    }

    static bool LooksLikePayloadTree(string dir)
    {
        if (!Directory.Exists(dir)) return false;
        return Directory.Exists(Path.Combine(dir, "windows-x64")) ||
               Directory.Exists(Path.Combine(dir, "linux-x64")) ||
               Directory.Exists(Path.Combine(dir, "payload", "windows-x64")) ||
               Directory.Exists(Path.Combine(dir, "payload", "linux-x64"));
    }

    static bool LooksLikeOsPayload(string dir)
    {
        if (!Directory.Exists(dir)) return false;
        if (File.Exists(Path.Combine(dir, "cubecheck.exe")))
        {
            return !IsSelf(Path.Combine(dir, "cubecheck.exe"));
        }
        return File.Exists(Path.Combine(dir, "cubecheck"));
    }

    static bool IsSelf(string exe)
    {
        var me = Environment.ProcessPath;
        if (string.IsNullOrEmpty(me)) return false;
        return string.Equals(Path.GetFullPath(exe), Path.GetFullPath(me), StringComparison.OrdinalIgnoreCase);
    }

    static IEnumerable<string> OsPayloadKeys()
    {
        if (OperatingSystem.IsWindows())
        {
            yield return RuntimeInformation.OSArchitecture == Architecture.X86 ? "windows-x86" : "windows-x64";
            yield return "windows-x64";
            yield break;
        }

        if (OperatingSystem.IsMacOS())
        {
            yield return RuntimeInformation.OSArchitecture == Architecture.Arm64 ? "osx-arm64" : "osx-x64";
            yield return "osx-arm64";
            yield return "osx-x64";
            yield break;
        }

        yield return RuntimeInformation.OSArchitecture == Architecture.X86 ? "linux-x86" : "linux-x64";
        yield return "linux-x64";
        yield return "linux-x86";
    }

    static IEnumerable<string> SafeDirs(string root)
    {
        if (!Directory.Exists(root)) yield break;
        foreach (var dir in Directory.GetDirectories(root))
        {
            yield return dir;
        }
    }

    static async Task DownloadAsync(string url, string dest, IProgress<InstallProgress> progress, CancellationToken ct)
    {
        using var handler = new SocketsHttpHandler
        {
            AllowAutoRedirect = false,
            AutomaticDecompression = System.Net.DecompressionMethods.None
        };
        using var http = new HttpClient(handler) { Timeout = TimeSpan.FromMinutes(30) };
        http.DefaultRequestHeaders.UserAgent.ParseAdd(UserAgent);
        http.DefaultRequestHeaders.Accept.ParseAdd("*/*");

        var current = InstallerConfig.RequireHttps(url);
        HttpResponseMessage resp;
        for (var hop = 0; hop < 8; hop++)
        {
            resp = await http.GetAsync(current, HttpCompletionOption.ResponseHeadersRead, ct).ConfigureAwait(false);
            if ((int)resp.StatusCode is >= 300 and < 400)
            {
                var loc = resp.Headers.Location?.ToString();
                resp.Dispose();
                if (string.IsNullOrWhiteSpace(loc))
                {
                    throw new InvalidOperationException("Сервер вернул редирект без адреса.");
                }
                var next = loc.StartsWith('/') && Uri.TryCreate(current, UriKind.Absolute, out var baseUri)
                    ? new Uri(baseUri, loc).AbsoluteUri
                    : loc;
                current = InstallerConfig.RequireHttps(next);
                continue;
            }

            resp.EnsureSuccessStatusCode();
            if (resp.RequestMessage?.RequestUri is { } final)
            {
                InstallerConfig.RequireHttps(final.AbsoluteUri);
            }

            var total = resp.Content.Headers.ContentLength;
            await using var input = await resp.Content.ReadAsStreamAsync(ct).ConfigureAwait(false);
            await using var output = File.Create(dest);
            var buf = new byte[128 * 1024];
            long read = 0;
            int n;
            while ((n = await input.ReadAsync(buf, ct).ConfigureAwait(false)) > 0)
            {
                await output.WriteAsync(buf.AsMemory(0, n), ct).ConfigureAwait(false);
                read += n;
                var pct = total is > 0 ? 5 + 35.0 * read / total.Value : 20;
                progress.Report(new InstallProgress(pct, "Загрузка", $"{read / 1024.0 / 1024.0:0.0} МБ"));
            }
            return;
        }

        throw new InvalidOperationException("Слишком много редиректов при загрузке.");
    }

    static void ExtractZip(string zip, string dest, IProgress<InstallProgress> progress, CancellationToken ct)
    {
        Directory.CreateDirectory(dest);
        var destFull = Path.GetFullPath(dest) + Path.DirectorySeparatorChar;
        using var archive = ZipFile.OpenRead(zip);
        var entries = archive.Entries.Where(e => !string.IsNullOrEmpty(e.Name)).ToList();
        var i = 0;
        foreach (var entry in entries)
        {
            ct.ThrowIfCancellationRequested();
            i++;
            var name = entry.FullName.Replace('/', Path.DirectorySeparatorChar);
            if (ShouldSkipRelative(name)) continue;
            var target = Path.GetFullPath(Path.Combine(dest, name));
            if (!target.StartsWith(destFull, StringComparison.OrdinalIgnoreCase)) continue;
            Directory.CreateDirectory(Path.GetDirectoryName(target)!);
            entry.ExtractToFile(target, overwrite: true);
            var pct = 40 + 25.0 * i / Math.Max(1, entries.Count);
            progress.Report(new InstallProgress(pct, "Распаковка", entry.FullName));
        }
    }

    static void CopyTree(string src, string dest, IProgress<InstallProgress> progress, CancellationToken ct)
    {
        var files = Directory.GetFiles(src, "*", SearchOption.AllDirectories)
            .Where(f => !ShouldSkipRelative(Path.GetRelativePath(src, f)))
            .ToList();
        var i = 0;
        foreach (var file in files)
        {
            ct.ThrowIfCancellationRequested();
            i++;
            var rel = Path.GetRelativePath(src, file);
            var target = Path.Combine(dest, rel);
            Directory.CreateDirectory(Path.GetDirectoryName(target)!);
            CopyRetry(file, target);
            var pct = 65 + 30.0 * i / Math.Max(1, files.Count);
            progress.Report(new InstallProgress(pct, "Копирование", rel.Replace('\\', '/')));
        }
    }

    static async Task<string> MaterializeLocalPayloadAsync(
        string temp,
        IProgress<InstallProgress> progress,
        CancellationToken ct)
    {
        var destZip = Path.Combine(temp, "payload.zip");
        var asm = Assembly.GetExecutingAssembly();
        var resource = asm.GetManifestResourceNames().FirstOrDefault(InstallerConfig.IsEmbeddedPayloadName);
        if (resource != null)
        {
            await using var input = asm.GetManifestResourceStream(resource)
                ?? throw new InvalidOperationException("Не удалось прочитать встроенный пакет.");
            await using var output = File.Create(destZip);
            var buf = new byte[128 * 1024];
            long read = 0;
            int n;
            var total = input.CanSeek ? input.Length : 0L;
            while ((n = await input.ReadAsync(buf, ct).ConfigureAwait(false)) > 0)
            {
                await output.WriteAsync(buf.AsMemory(0, n), ct).ConfigureAwait(false);
                read += n;
                var pct = total > 0 ? 5 + 35.0 * read / total : 20;
                progress.Report(new InstallProgress(pct, "Локальный пакет", $"{read / 1024.0 / 1024.0:0.0} МБ"));
            }
            if (read < 100_000)
            {
                throw new InvalidOperationException("Встроенный пакет слишком маленький.");
            }
            return destZip;
        }

        foreach (var sidecar in InstallerConfig.EnumerateSidecarZips().Distinct(StringComparer.OrdinalIgnoreCase))
        {
            ct.ThrowIfCancellationRequested();
            if (!File.Exists(sidecar)) continue;
            var len = new FileInfo(sidecar).Length;
            if (len < 100_000) continue;
            progress.Report(new InstallProgress(20, "Локальный пакет", Path.GetFileName(sidecar)));
            return sidecar;
        }

        throw new InvalidOperationException(
            "Офлайн-установщик: нет локального пакета (внутри exe или рядом). Загрузка из сети отключена.");
    }

    static bool ShouldSkipRelative(string rel)
    {
        var parts = rel.Split(['/', '\\'], StringSplitOptions.RemoveEmptyEntries);
        var offline = InstallerConfig.IsOfflineSetup;
        if (parts.Any(p => SkipDirsAlways.Contains(p))) return true;
        if (!offline && parts.Any(p => SkipDirsOnline.Contains(p))) return true;
        var name = Path.GetFileName(rel);
        if (SkipNamesAlways.Contains(name)) return true;
        if (!offline && SkipVendorFiles.Contains(name)) return true;
        if (AssetsOnlyNames.Contains(name) &&
            !parts.Any(p => p.Equals("assets", StringComparison.OrdinalIgnoreCase)))
        {
            return true;
        }
        return name.EndsWith(".pdb", StringComparison.OrdinalIgnoreCase) ||
               name.EndsWith(".exp", StringComparison.OrdinalIgnoreCase) ||
               name.EndsWith(".lib", StringComparison.OrdinalIgnoreCase) ||
               name.EndsWith(".ilk", StringComparison.OrdinalIgnoreCase);
    }

    static void CopyRetry(string src, string dest)
    {
        for (var attempt = 0; attempt < 8; attempt++)
        {
            try
            {
                if (File.Exists(dest))
                {
                    try { File.Delete(dest); }
                    catch
                    {
                        var bak = dest + ".old";
                        try { File.Delete(bak); } catch { /* ignore */ }
                        try { File.Move(dest, bak); } catch { /* ignore */ }
                    }
                }
                File.Copy(src, dest, overwrite: true);
                return;
            }
            catch when (attempt < 7)
            {
                Thread.Sleep(400);
            }
        }
        throw new InvalidOperationException($"Не удалось скопировать {Path.GetFileName(dest)}. Закройте CubeCheck и повторите установку.");
    }

    static void FinishLayout(string dest, bool offline)
    {
        var assets = Path.Combine(dest, "assets");
        Directory.CreateDirectory(assets);
        Directory.CreateDirectory(Path.Combine(dest, "reports"));
        var settings = Path.Combine(dest, "settings.json");
        if (!File.Exists(settings))
        {
            var def = Path.Combine(assets, "settings.default.json");
            if (File.Exists(def)) File.Copy(def, settings);
            else File.WriteAllText(settings, InstallerConfig.DefaultSettingsJson);
        }

        if (offline)
        {
            File.WriteAllText(Path.Combine(dest, ".offline"), "");
            File.WriteAllText(Path.Combine(assets, ".offline"), "");
        }

        UninstallFiles.RemoveLegacyRootFiles(dest);

        if (!OperatingSystem.IsWindows())
        {
            var bin = Path.Combine(dest, "cubecheck");
            var sh = Path.Combine(dest, "cubecheck.sh");
            if (File.Exists(bin)) TryChmod(bin);
            if (File.Exists(sh)) TryChmod(sh);
        }
    }

    static void TryChmod(string path)
    {
        try
        {
            using var p = Process.Start(new ProcessStartInfo
            {
                FileName = "chmod",
                Arguments = $"+x \"{path}\"",
                CreateNoWindow = true,
                UseShellExecute = false
            });
            p?.WaitForExit(3000);
        }
        catch
        {
            // ignore
        }
    }

    static void StopRunningCubecheck()
    {
        foreach (var name in new[] { "cubecheck", "cubecheck-launcher" })
        {
            foreach (var p in Process.GetProcessesByName(name))
            {
                try
                {
                    p.CloseMainWindow();
                    if (!p.WaitForExit(1200)) p.Kill();
                }
                catch
                {
                    try { p.Kill(); } catch { /* ignore */ }
                }
            }
        }
        Thread.Sleep(300);
    }

    static void GrantUsersModify(string path, bool inherit)
    {
        try
        {
            if (!File.Exists(path) && !Directory.Exists(path)) return;
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

    static string CreateTempDir()
    {
        var dir = Path.Combine(Path.GetTempPath(), "CubeCheck-Setup-" + Guid.NewGuid().ToString("N")[..8]);
        Directory.CreateDirectory(dir);
        return dir;
    }
}
