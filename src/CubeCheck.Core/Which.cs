namespace CubeCheck;

public static class Which
{
    static IEnumerable<string> SearchDirs()
    {
        foreach (var dir in ExtraBinDirs())
        {
            if (!string.IsNullOrWhiteSpace(dir)) yield return dir;
        }
        var path = Environment.GetEnvironmentVariable("PATH") ?? "";
        foreach (var dir in path.Split(Path.PathSeparator))
        {
            if (!string.IsNullOrWhiteSpace(dir)) yield return dir;
        }
    }

    static IEnumerable<string> ExtraBinDirs()
    {
        var exe = AppPaths.ExeDir;
        yield return exe;
        yield return Path.Combine(exe, "assets", "bin");
        yield return Path.Combine(exe, "extras", "bin");
        var parent = Directory.GetParent(exe);
        if (parent != null)
        {
            yield return Path.Combine(parent.FullName, "extras", "bin");
            yield return Path.Combine(parent.FullName, "assets", "bin");
            var root = parent.Parent;
            if (root != null)
            {
                yield return Path.Combine(root.FullName, "extras", "bin");
                yield return Path.Combine(root.FullName, "assets", "bin");
            }
        }
    }

    public static string? Find(params string[] names)
    {
        var exts = Compat.IsWindows ? new[] { "", ".exe", ".cmd", ".bat" } : new[] { "" };
        foreach (var name in names)
        {
            if (string.IsNullOrWhiteSpace(name)) continue;
            if (name.IndexOfAny(['/', '\\']) >= 0)
            {
                if (File.Exists(name)) return Path.GetFullPath(name);
                continue;
            }
            foreach (var dir in SearchDirs())
            {
                foreach (var ext in exts)
                {
                    try
                    {
                        var full = Path.Combine(dir, name + ext);
                        if (File.Exists(full)) return Path.GetFullPath(full);
                    }
                    catch
                    {
                        // skip
                    }
                }
            }
        }
        return null;
    }

    public static bool Has(params string[] names) => Find(names) != null;
}
