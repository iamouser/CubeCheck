using System.Diagnostics;
using System.IO;

namespace CubeCheck.Installer;

static class Shortcuts
{
    public static void Create(string dest, bool desktop, bool menu)
    {
        if (!desktop && !menu) return;
        var target = ResolveAppBinary(dest);
        if (string.IsNullOrEmpty(target) || !File.Exists(target))
        {
            throw new InvalidOperationException("Нет исполняемого файла CubeCheck для ярлыка");
        }

        var icon = Path.Combine(dest, "assets", "cubecheck.ico");
        if (!File.Exists(icon)) icon = target;

        if (OperatingSystem.IsWindows())
        {
            if (desktop)
            {
                var link = Path.Combine(
                    Environment.GetFolderPath(Environment.SpecialFolder.DesktopDirectory),
                    "CubeCheck.lnk");
                CreateWindowsLink(link, target, dest, icon);
            }

            if (menu)
            {
                var programs = Environment.GetFolderPath(Environment.SpecialFolder.StartMenu);
                var link = Path.Combine(programs, "Programs", "CubeCheck.lnk");
                CreateWindowsLink(link, target, dest, icon);
            }
            return;
        }

        if (OperatingSystem.IsLinux())
        {
            CreateLinuxDesktop(target, dest, icon, desktop, menu);
            return;
        }

        if (OperatingSystem.IsMacOS())
        {
            CreateMacAliases(target, dest, desktop, menu);
        }
    }

    public static string? ResolveAppBinary(string dest)
    {
        if (OperatingSystem.IsWindows())
        {
            var exe = Path.Combine(dest, "cubecheck.exe");
            return File.Exists(exe) ? exe : null;
        }

        var sh = Path.Combine(dest, "cubecheck.sh");
        var bin = Path.Combine(dest, "cubecheck");
        if (File.Exists(sh)) return sh;
        if (File.Exists(bin)) return bin;
        return null;
    }

    static void CreateWindowsLink(string link, string target, string dest, string? icon)
    {
        Directory.CreateDirectory(Path.GetDirectoryName(link)!);
        var iconArg = string.IsNullOrEmpty(icon) ? "" : $"; $s.IconLocation = '{EscapePs(icon)}'";
        var script =
            $"$s = (New-Object -ComObject WScript.Shell).CreateShortcut('{EscapePs(link)}'); " +
            $"$s.TargetPath = '{EscapePs(target)}'; $s.WorkingDirectory = '{EscapePs(dest)}'{iconArg}; $s.Save()";
        Run("powershell", $"-NoProfile -STA -Command \"{script}\"");
    }

    static void CreateLinuxDesktop(string target, string dest, string icon, bool desktop, bool menu)
    {
        TryChmod(target);
        TryChmod(Path.Combine(dest, "cubecheck"));
        var home = Environment.GetFolderPath(Environment.SpecialFolder.UserProfile);
        var body = string.Join('\n',
            "[Desktop Entry]",
            "Type=Application",
            "Name=CubeCheck",
            "Comment=CubeCheck 1.1 beta",
            $"Exec=\"{target}\"",
            $"Path={dest}",
            File.Exists(icon) ? $"Icon={icon}" : "Icon=application-x-executable",
            "Terminal=false",
            "Categories=Utility;",
            "");

        if (menu)
        {
            var apps = Path.Combine(home, ".local", "share", "applications");
            Directory.CreateDirectory(apps);
            var file = Path.Combine(apps, "cubecheck.desktop");
            File.WriteAllText(file, body);
            TryChmod(file);
        }

        if (desktop)
        {
            var desk = Environment.GetFolderPath(Environment.SpecialFolder.DesktopDirectory);
            if (string.IsNullOrEmpty(desk)) desk = Path.Combine(home, "Desktop");
            Directory.CreateDirectory(desk);
            var file = Path.Combine(desk, "CubeCheck.desktop");
            File.WriteAllText(file, body);
            TryChmod(file);
        }
    }

    static void CreateMacAliases(string target, string dest, bool desktop, bool menu)
    {
        TryChmod(target);
        TryChmod(Path.Combine(dest, "cubecheck"));
        var home = Environment.GetFolderPath(Environment.SpecialFolder.UserProfile);

        if (menu)
        {
            var apps = "/Applications";
            var alias = Path.Combine(apps, "CubeCheck");
            if (!string.Equals(Path.GetFullPath(dest), Path.GetFullPath(alias), StringComparison.OrdinalIgnoreCase))
            {
                TryUnixLink(alias, dest);
            }
        }

        if (desktop)
        {
            var desk = Path.Combine(home, "Desktop", "CubeCheck");
            TryUnixLink(desk, dest);
        }
    }

    static void TryUnixLink(string link, string dest)
    {
        try
        {
            if (Directory.Exists(link) || File.Exists(link))
            {
                if (Directory.Exists(link) && !IsSymlink(link)) return;
                if (File.Exists(link)) File.Delete(link);
            }
            if (OperatingSystem.IsWindows()) return;
            File.CreateSymbolicLink(link, dest);
        }
        catch
        {
            Run("ln", $"-sfn \"{dest}\" \"{link}\"");
        }
    }

    static bool IsSymlink(string path)
    {
        try
        {
            return (File.GetAttributes(path) & FileAttributes.ReparsePoint) != 0;
        }
        catch
        {
            return false;
        }
    }

    static void TryChmod(string path)
    {
        if (!File.Exists(path) || OperatingSystem.IsWindows()) return;
        Run("chmod", $"+x \"{path}\"");
    }

    static void Run(string file, string args)
    {
        try
        {
            using var p = Process.Start(new ProcessStartInfo
            {
                FileName = file,
                Arguments = args,
                CreateNoWindow = true,
                UseShellExecute = false
            });
            p?.WaitForExit(8000);
        }
        catch
        {
            // ignore
        }
    }

    static string EscapePs(string value) => value.Replace("'", "''");
}
