namespace CubeCheck;

public static class SystemInfo
{
    public static string UserName =>
        Environment.GetEnvironmentVariable("USERNAME") ?? Environment.UserName;

    public static string ComputerName =>
        Environment.GetEnvironmentVariable("COMPUTERNAME") ?? Environment.MachineName;

    public static string OsInfoLabel =>
        Compat.IsWindows ? "Дата установки Windows" : Compat.IsMac ? "macOS" : "Система";

    public static string WindowsInstallDate => Native.InstallDate();

    public static string RecycleBinLastChange => Native.RecycleMtime();
}
