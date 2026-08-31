using System.Diagnostics;

namespace CubeCheck;

public static class ToolLauncher
{
    static string Missing(string name) =>
        AppPaths.IsOffline
            ? $"{name} не найден в папке assets. Офлайн-сборка не загружает файлы из сети."
            : Compat.IsWindows
                ? $"{name} не найден. Скачайте его в разделе «Компоненты»."
                : $"{name} не найден. Установите программу из репозитория вашей ОС.";

    static string Quote(string a)
    {
        if (a.IndexOfAny([' ', '"', '\t']) < 0) return a;
        return "\"" + a.Replace("\"", "\\\"") + "\"";
    }

    static void Spawn(string exe, params string[] args)
    {
        var dir = Path.GetDirectoryName(exe) ?? ".";
        var psi = new ProcessStartInfo
        {
            FileName = exe,
            WorkingDirectory = Directory.Exists(dir) ? dir : ".",
            UseShellExecute = false,
            CreateNoWindow = true
        };
#if NETFRAMEWORK
        if (args.Length > 0) psi.Arguments = string.Join(" ", args.Select(Quote));
#else
        foreach (var a in args) psi.ArgumentList.Add(a);
#endif
        Process.Start(psi);
    }

    static void SpawnFirst(string[] names, params string[] args)
    {
        var exe = Which.Find(names) ?? throw new InvalidOperationException(Missing(names[0]));
        Spawn(exe, args);
    }

    static void OpenPath(string path)
    {
        if (Compat.IsWindows)
        {
            Process.Start(new ProcessStartInfo { FileName = path, UseShellExecute = true });
            return;
        }
        if (Compat.IsMac)
        {
            Spawn("open", path);
            return;
        }
        var opener = Which.Find("xdg-open") ?? "xdg-open";
        Spawn(opener, path);
    }

    static void LaunchTool(string path, string? args = null)
    {
        if (!File.Exists(path))
        {
            throw new InvalidOperationException(Missing(Path.GetFileName(path)));
        }
        var dir = Path.GetDirectoryName(path) ?? ".";
        ThreadPool.QueueUserWorkItem(_ =>
        {
            try
            {
                Native.ShellExecute(path, dir, "open", args);
            }
            catch
            {
                try { Native.ShellExecute(path, dir, "runas", args); } catch { /* ignore */ }
            }
        });
    }

    public static void RunUtil(string key)
    {
        if (!Compat.IsWindows)
        {
            RunPosixUtil(key);
            return;
        }
        switch (key)
        {
            case "everything":
            case "search":
                RunEverything();
                break;
            case "shellbag":
                LaunchTool(AppPaths.ToolPath("shellbag"));
                break;
            case "systeminformer":
            case "processes":
                RunSystemInformer();
                break;
            case "procmon":
            case "monitor":
                LaunchTool(AppPaths.ToolPath("procmon"));
                break;
            case "autoruns":
                LaunchTool(AppPaths.ToolPath("autoruns"));
                break;
            case "procexp":
                LaunchTool(AppPaths.ToolPath("procexp"));
                break;
            default:
                throw new InvalidOperationException("Неизвестная программа");
        }
    }

    static void RunPosixUtil(string key)
    {
        switch (key)
        {
            case "search":
            case "everything":
                RunPosixSearch(null);
                break;
            case "files":
            case "shellbag":
                OpenRecentFiles();
                break;
            case "processes":
            case "systeminformer":
            case "procexp":
                OpenProcessUi();
                break;
            case "monitor":
            case "procmon":
                OpenMonitor();
                break;
            case "autoruns":
                OpenAutoruns();
                break;
            default:
                throw new InvalidOperationException("Неизвестная программа");
        }
    }

    public static void RunEverything()
    {
        if (!Compat.IsWindows)
        {
            RunPosixSearch(null);
            return;
        }
        var path = AppPaths.ToolPath("everything");
        if (!File.Exists(path)) throw new InvalidOperationException(Missing("Everything.exe"));
        Spawn(path);
    }

    public static void RunEverythingWithSearch(IEnumerable<string> terms)
    {
        if (!Compat.IsWindows)
        {
            RunPosixSearch(Catalog.EverythingSearchQuery(terms));
            return;
        }
        var path = AppPaths.ToolPath("everything");
        if (!File.Exists(path)) throw new InvalidOperationException(Missing("Everything.exe"));
        Spawn(path, "-search", Catalog.EverythingSearchQuery(terms));
    }

    public static void RunAutocheckSearch() => RunEverythingWithSearch(Catalog.CheatNames);

    static void RunPosixSearch(string? query)
    {
        if (Compat.IsMac)
        {
            var mdfind = Which.Find("mdfind");
            if (mdfind != null && !string.IsNullOrEmpty(query))
            {
                var orQuery = string.Join(" OR ", Catalog.CheatNames.Select(n => n.Contains(' ') ? "\"" + n + "\"" : n));
                var script = "mdfind " + Quote(orQuery);
                Spawn("osascript", "-e", "tell application \"Terminal\" to do script " + Quote(script));
                return;
            }
            Spawn("open", "-a", "Spotlight");
            return;
        }

        if (Which.Find("catfish") is { } catfish)
        {
            if (!string.IsNullOrEmpty(query)) Spawn(catfish, "--start", query);
            else Spawn(catfish);
            return;
        }
        if (Which.Find("fsearch") is { } fsearch)
        {
            Spawn(fsearch);
            return;
        }
        var locate = Which.Find("plocate", "locate");
        if (locate != null && !string.IsNullOrEmpty(query))
        {
            var term = Which.Find("x-terminal-emulator", "gnome-terminal", "konsole", "xfce4-terminal", "xterm")
                       ?? throw new InvalidOperationException(Missing("терминал"));
            Spawn(term, "-e", locate + " " + Catalog.CheatNames[0]);
            return;
        }
        if (Which.Find("fzf") is { } fzf && Which.Find("fd") is { } fdFzf)
        {
            RunInTerminal("/bin/sh", "-c", Quote(fdFzf) + " --type f | " + Quote(fzf));
            return;
        }
        if (Which.Find("fd") is { } fd)
        {
            RunInTerminal(fd, string.IsNullOrEmpty(query) ? "." : query);
            return;
        }
        if (Which.Find("rg") is { } rg)
        {
            RunInTerminal(rg, string.IsNullOrEmpty(query) ? "." : query);
            return;
        }
        throw new InvalidOperationException("В assets/bin нет fd/rg/fzf.");
    }

    static void OpenRecentFiles()
    {
        if (Compat.IsMac)
        {
            var recent = Path.Combine(
                Environment.GetFolderPath(Environment.SpecialFolder.UserProfile),
                "Library", "Application Support", "com.apple.sharedfilelist");
            if (Directory.Exists(recent)) OpenPath(recent);
            else Spawn("open", "-a", "Finder");
            return;
        }
        var home = Environment.GetFolderPath(Environment.SpecialFolder.UserProfile);
        var recentLinux = Path.Combine(home, ".local", "share", "recently-used.xbel");
        var target = File.Exists(recentLinux) ? Path.GetDirectoryName(recentLinux)! : home;
        if (Which.Find("lf") is { } lf)
        {
            RunInTerminal(lf, target);
            return;
        }
        OpenPath(target);
    }

    static void OpenProcessUi()
    {
        if (Compat.IsMac)
        {
            Spawn("open", "-a", "Activity Monitor");
            return;
        }
        SpawnFirst([
            "missioncenter", "gnome-system-monitor", "plasma-systemmonitor",
            "xfce4-taskmanager", "ksysguard", "btop", "btm", "procs", "htop"
        ]);
    }

    static void OpenMonitor()
    {
        if (Compat.IsMac)
        {
            var term = Which.Find("Terminal") != null;
            Spawn("osascript", "-e", "tell application \"Terminal\" to do script \"sudo fs_usage\"");
            return;
        }
        if (Which.Find("lsof") is { } lsof)
        {
            RunInTerminal(lsof, "-nP");
            return;
        }
        if (Which.Find("busybox") is { } busy)
        {
            RunInTerminal(busy, "lsof");
            return;
        }
        if (Which.Find("fatrace") is { } fatrace)
        {
            RunInTerminal("sudo", fatrace);
            return;
        }
        if (Which.Find("sysdig") is { } sysdig)
        {
            RunInTerminal(sysdig);
            return;
        }
        OpenProcessUi();
    }

    static void RunInTerminal(string exe, params string[] args)
    {
        if (Compat.IsMac)
        {
            var cmd = Quote(exe) + (args.Length == 0 ? "" : " " + string.Join(" ", args.Select(Quote)));
            Spawn("osascript", "-e", "tell application \"Terminal\" to do script " + Quote(cmd));
            return;
        }
        var term = Which.Find("x-terminal-emulator", "gnome-terminal", "konsole", "xfce4-terminal", "xterm");
        if (term == null)
        {
            Spawn(exe, args);
            return;
        }
        var line = Quote(exe) + (args.Length == 0 ? "" : " " + string.Join(" ", args.Select(Quote)));
        Spawn(term, "-e", line);
    }

    static void OpenAutoruns()
    {
        if (Compat.IsMac)
        {
            Spawn("open", "x-apple.systempreferences:com.apple.LoginItems-Settings.extension");
            var agents = Path.Combine(
                Environment.GetFolderPath(Environment.SpecialFolder.UserProfile),
                "Library", "LaunchAgents");
            if (Directory.Exists(agents)) OpenPath(agents);
            return;
        }
        var home = Environment.GetFolderPath(Environment.SpecialFolder.UserProfile);
        var auto = Path.Combine(home, ".config", "autostart");
        Directory.CreateDirectory(auto);
        OpenPath(auto);
    }

    static void RunSystemInformer()
    {
        var path = AppPaths.ToolPath("systeminformer");
        if (File.Exists(path) && !Native.IsPeAmd64(path))
        {
            throw new InvalidOperationException("Нужна 64-битная версия. В «Компонентах» нажмите «Повтор».");
        }
        var dir = Path.GetDirectoryName(path);
        if (dir != null)
        {
            Downloader.StripSystemInformerExtras(dir);
            Downloader.WriteSystemInformerSettings(dir);
        }
        LaunchTool(path);
    }

    public static void RunSystemInfo()
    {
        if (Compat.IsWindows)
        {
            Process.Start(new ProcessStartInfo { FileName = "msinfo32", UseShellExecute = true });
            return;
        }
        if (Compat.IsMac)
        {
            Spawn("open", "-a", "System Information");
            return;
        }
        if (Which.Find("hardinfo", "hardinfo2") is { } hi)
        {
            Spawn(hi);
            return;
        }
        var term = Which.Find("x-terminal-emulator", "gnome-terminal", "konsole", "xfce4-terminal", "xterm");
        if (term != null)
        {
            Spawn(term, "-e", "uname -a; echo; cat /etc/os-release");
            return;
        }
        throw new InvalidOperationException("Не удалось открыть сведения о системе.");
    }

    public static void OpenRecycleBin()
    {
        if (Compat.IsWindows)
        {
            Process.Start(new ProcessStartInfo
            {
                FileName = "explorer.exe",
                Arguments = "shell:RecycleBinFolder",
                UseShellExecute = true
            });
            return;
        }
        if (Compat.IsMac)
        {
            OpenPath(Path.Combine(Environment.GetFolderPath(Environment.SpecialFolder.UserProfile), ".Trash"));
            return;
        }
        if (Which.Find("xdg-open") != null)
        {
            Spawn("xdg-open", "trash:///");
            return;
        }
        var trash = Path.Combine(
            Environment.GetFolderPath(Environment.SpecialFolder.UserProfile),
            ".local", "share", "Trash", "files");
        Directory.CreateDirectory(trash);
        OpenPath(trash);
    }

    public static void OpenUrl(string url)
    {
        if (Compat.IsWindows)
        {
            Process.Start(new ProcessStartInfo { FileName = url, UseShellExecute = true });
            return;
        }
        if (Compat.IsMac) Spawn("open", url);
        else Spawn(Which.Find("xdg-open") ?? "xdg-open", url);
    }

    public static void OpenHolyCheck() => OpenUrl(Content.HolyCheckUrl);
    public static void OpenTelegram() => OpenUrl(Content.TelegramUrl);

    public static void ClearMinecraftLogs()
    {
        var logs = Path.Combine(AppPaths.MinecraftDir, "logs");
        if (!Directory.Exists(logs))
        {
            throw new InvalidOperationException("Папка логов Minecraft не найдена.");
        }
        Directory.Delete(logs, recursive: true);
    }
}
