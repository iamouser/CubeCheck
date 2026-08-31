using System.IO;
using System.Text;

namespace CubeCheck.Installer;

static class UninstallFiles
{
    public static void Write(string dest)
    {
        Directory.CreateDirectory(dest);
        var assets = Path.Combine(dest, "assets");
        Directory.CreateDirectory(assets);

        var icoAssets = Path.Combine(assets, "UnInstall.ico");
        var icoRoot = Path.Combine(dest, "UnInstall.ico");
        if (!File.Exists(icoAssets) && File.Exists(icoRoot))
        {
            try { File.Copy(icoRoot, icoAssets, overwrite: true); }
            catch { /* ignore */ }
        }
        TryDelete(icoRoot);

        var cmdAssets = Path.Combine(assets, "UnInstall.cmd");
        var cmdRoot = Path.Combine(dest, "UnInstall.cmd");
        if (!File.Exists(cmdAssets) && File.Exists(cmdRoot))
        {
            try { File.Copy(cmdRoot, cmdAssets, overwrite: true); }
            catch { /* ignore */ }
        }
        WriteCmd(cmdAssets);
        RemoveLegacyRootFiles(dest);

        var cmdFull = Path.GetFullPath(cmdAssets);
        var icoFull = Path.GetFullPath(icoAssets);
        var uri = new Uri(cmdFull).AbsoluteUri;

        var url = new StringBuilder();
        url.Append("[InternetShortcut]\r\n");
        url.Append("URL=").Append(uri).Append("\r\n");
        if (File.Exists(icoFull))
        {
            url.Append("IconFile=").Append(icoFull).Append("\r\n");
            url.Append("IconIndex=0\r\n");
        }
        File.WriteAllText(
            Path.Combine(dest, "UnInstall.url"),
            url.ToString(),
            new UTF8Encoding(encoderShouldEmitUTF8Identifier: false));
    }

    static void WriteCmd(string dest)
    {
        File.WriteAllText(
            dest,
            "@echo off\r\ncd /d \"%~dp0..\"\r\nstart \"\" \"%~dp0..\\cubecheck.exe\" -uninstall\r\n",
            new UTF8Encoding(encoderShouldEmitUTF8Identifier: false));
    }

    public static void RemoveLegacyRootFiles(string dest)
    {
        foreach (var name in new[]
        {
            "cubecheck_api.dll",
            "cubecheck_native.dll",
            "UnInstall.ico",
            "UnInstall.cmd"
        })
        {
            TryDelete(Path.Combine(dest, name));
        }
    }

    static void TryDelete(string path)
    {
        try
        {
            if (File.Exists(path)) File.Delete(path);
        }
        catch
        {
            // leftover root helper from an older payload
        }
    }
}
