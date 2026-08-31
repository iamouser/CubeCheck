using System.Diagnostics;
using System.IO;
using System.Security.Principal;

namespace CubeCheck.Installer;

static class Elevation
{
    public static bool IsElevated
    {
        get
        {
            if (!OperatingSystem.IsWindows()) return false;
            try
            {
                using var id = WindowsIdentity.GetCurrent();
                var principal = new WindowsPrincipal(id);
                return principal.IsInRole(WindowsBuiltInRole.Administrator);
            }
            catch
            {
                return false;
            }
        }
    }

    public static void RelaunchElevated(string[] args)
    {
        if (!OperatingSystem.IsWindows())
        {
            throw new InvalidOperationException("Для этой папки нужны права администратора (root). Запустите установщик от root.");
        }

        var exe = Environment.ProcessPath ?? throw new InvalidOperationException("Нет пути к установщику");
        var psi = new ProcessStartInfo
        {
            FileName = exe,
            Arguments = string.Join(" ", args.Select(Quote)),
            UseShellExecute = true,
            Verb = "runas",
            WorkingDirectory = Path.GetDirectoryName(exe) ?? InstallerConfig.ExeDir
        };
        Process.Start(psi);
    }

    static string Quote(string value)
    {
        if (string.IsNullOrEmpty(value)) return "\"\"";
        if (value.Contains(' ') || value.Contains('"'))
        {
            return "\"" + value.Replace("\"", "\\\"") + "\"";
        }
        return value;
    }
}
