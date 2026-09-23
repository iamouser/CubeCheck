using System.Globalization;

namespace CubeCheck;

public static class HexColor
{
    public static string Format(byte[]? rgb)
    {
        if (rgb is not { Length: >= 3 }) return "#D4AF37";
        return string.Format(CultureInfo.InvariantCulture, "#{0:X2}{1:X2}{2:X2}", rgb[0], rgb[1], rgb[2]);
    }

    public static bool TryParse(string? text, out byte[] rgb)
    {
        rgb = [0, 0, 0];
        if (text == null || string.IsNullOrWhiteSpace(text)) return false;
        var s = text.Trim();
        if (s[0] == '#') s = s.Substring(1);
        if (s.Length is 3 or 4)
        {
            var expanded = "";
            foreach (var c in s) expanded += new string(c, 2);
            s = expanded;
        }
        if (s.Length == 8) s = s.Substring(2);
        if (s.Length != 6) return false;
        if (!byte.TryParse(s.Substring(0, 2), NumberStyles.HexNumber, CultureInfo.InvariantCulture, out var r)) return false;
        if (!byte.TryParse(s.Substring(2, 2), NumberStyles.HexNumber, CultureInfo.InvariantCulture, out var g)) return false;
        if (!byte.TryParse(s.Substring(4, 2), NumberStyles.HexNumber, CultureInfo.InvariantCulture, out var b)) return false;
        rgb = [r, g, b];
        return true;
    }
}
