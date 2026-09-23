namespace CubeCheck;

public readonly struct AppVersionNumber : IComparable<AppVersionNumber>
{
    public int Major { get; }
    public int Minor { get; }
    public int Patch { get; }
    public bool Prerelease { get; }

    public AppVersionNumber(int major, int minor, int patch, bool prerelease)
    {
        Major = major;
        Minor = minor;
        Patch = patch;
        Prerelease = prerelease;
    }

    public int CompareTo(AppVersionNumber other)
    {
        var c = Major.CompareTo(other.Major);
        if (c != 0) return c;
        c = Minor.CompareTo(other.Minor);
        if (c != 0) return c;
        c = Patch.CompareTo(other.Patch);
        if (c != 0) return c;
        if (Prerelease == other.Prerelease) return 0;
        return Prerelease ? -1 : 1;
    }

    public static bool TryParse(string? text, out AppVersionNumber version)
    {
        version = default;
        if (text == null || string.IsNullOrWhiteSpace(text)) return false;
        text = text.Trim();
        if (text.Length > 1 && (text[0] == 'v' || text[0] == 'V')) text = text.Substring(1);

        var pre = false;
        var cut = text.IndexOfAny(['-', '+', ' ']);
        if (cut >= 0)
        {
            pre = cut < text.Length - 1;
            text = text.Substring(0, cut);
        }

        var parts = text.Split('.');
        if (parts.Length is < 1 or > 4) return false;
        if (!int.TryParse(parts[0], out var major)) return false;
        var minor = 0;
        var patch = 0;
        if (parts.Length >= 2 && !int.TryParse(parts[1], out minor)) return false;
        if (parts.Length >= 3 && !int.TryParse(parts[2], out patch)) return false;
        version = new AppVersionNumber(major, minor, patch, pre);
        return true;
    }

    public static bool IsNewer(string? remote, string? current)
    {
        if (!TryParse(remote, out var next) || !TryParse(current, out var local)) return false;
        return next.CompareTo(local) > 0;
    }
}
