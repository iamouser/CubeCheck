using System.Diagnostics;
using CubeCheck;

namespace CubeCheck.Setup;

static class Program
{
    const string InstallDir = AppPaths.InstallDir;

    [STAThread]
    static void Main()
    {
        try
        {
            Run();
        }
        catch (Exception ex)
        {
            try { Native.MessageBox("Ошибка установки", ex.Message); }
            catch { MessageBox.Show(ex.Message, "Ошибка установки"); }
            Environment.Exit(1);
        }
    }

    static void Run()
    {
        EnsureAdmin();
        var dest = InstallDir;
        var cubecheck = Path.Combine(dest, "cubecheck.exe");
        var assets = Path.Combine(dest, "assets");
        Directory.CreateDirectory(assets);
        StopRunningCubecheck();

        var payload = FindPayload();
        CopyTree(payload, dest);

        var reports = Path.Combine(dest, "reports");
        Directory.CreateDirectory(reports);
        var settings = Path.Combine(dest, "settings.json");
        if (!File.Exists(settings))
        {
            var def = Path.Combine(assets, "settings.default.json");
            if (File.Exists(def)) File.Copy(def, settings);
            else File.WriteAllText(settings, AppPaths.DefaultSettingsJson);
        }

        GrantUsersModify(dest, true);
        GrantUsersModify(settings, false);
        GrantUsersModify(reports, true);
        GrantUsersModify(assets, true);

        var icon = Path.Combine(assets, "cubecheck.ico");
        try
        {
            Native.InstallShortcuts(cubecheck, dest, File.Exists(icon) ? icon : null);
        }
        catch
        {
            // shortcuts must not block launch
        }

        Launch(cubecheck);
    }

    static void EnsureAdmin()
    {
        if (CanWriteInstallDir() && !CubecheckRunning()) return;
        if (Native.IsElevated) return;
        Native.RelaunchAsAdmin();
        Environment.Exit(0);
    }

    static bool CanWriteInstallDir()
    {
        try
        {
            Directory.CreateDirectory(InstallDir);
            var test = Path.Combine(InstallDir, ".cubecheck_install_test");
            File.WriteAllText(test, "ok");
            File.Delete(test);
            return true;
        }
        catch
        {
            return false;
        }
    }

    static bool CubecheckRunning()
    {
        return Process.GetProcessesByName("cubecheck").Length > 0;
    }

    static void StopRunningCubecheck()
    {
        foreach (var p in Process.GetProcessesByName("cubecheck"))
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
        Thread.Sleep(400);
    }

    static string FindPayload()
    {
        var me = Path.GetDirectoryName(Environment.ProcessPath) ?? AppContext.BaseDirectory;
        var candidates = new[]
        {
            Path.Combine(me, "payload"),
            Path.Combine(me, "CubeCheck"),
            me
        };
        foreach (var dir in candidates)
        {
            var exe = Path.Combine(dir, "cubecheck.exe");
            var native = Path.Combine(dir, "cubecheck_native.dll");
            if (File.Exists(exe) && File.Exists(native) &&
                !string.Equals(Path.GetFullPath(exe), Path.GetFullPath(Environment.ProcessPath ?? ""), StringComparison.OrdinalIgnoreCase))
            {
                return dir;
            }
        }
        throw new InvalidOperationException(
            "Рядом с установщиком нет cubecheck.exe и cubecheck_native.dll.\nСоберите через build-dotnet.ps1.");
    }

    static void CopyTree(string src, string dest)
    {
        foreach (var file in Directory.GetFiles(src, "*", SearchOption.AllDirectories))
        {
            var name = Path.GetFileName(file);
            if (name.Equals("CubeCheck-Setup.exe", StringComparison.OrdinalIgnoreCase)) continue;
            var rel = Path.GetRelativePath(src, file);
            var target = Path.Combine(dest, rel);
            Directory.CreateDirectory(Path.GetDirectoryName(target)!);
            CopyRetry(file, target);
        }
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

    static void Launch(string exe)
    {
        if (!File.Exists(exe))
        {
            throw new InvalidOperationException($"Установка не записала программу: {exe}");
        }
        var dir = Path.GetDirectoryName(exe)!;
        Process.Start(new ProcessStartInfo
        {
            FileName = exe,
            WorkingDirectory = dir,
            UseShellExecute = true
        });
    }
}
