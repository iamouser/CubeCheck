using System.Windows.Media;
using CubeCheck;

namespace CubeCheck.App;

static class ThemeHelper
{
    public static Color ToMedia(Rgb rgb) => Color.FromRgb(rgb.R, rgb.G, rgb.B);

    public static Color ToMedia(byte[] rgb, Rgb fallback)
    {
        var c = ThemeColors.FromBytes(rgb, fallback);
        return Color.FromRgb(c.R, c.G, c.B);
    }

    public static SolidColorBrush Brush(Rgb rgb)
    {
        var b = new SolidColorBrush(ToMedia(rgb));
        if (b.CanFreeze) b.Freeze();
        return b;
    }

    public static SolidColorBrush Brush(Color c)
    {
        var b = new SolidColorBrush(c);
        if (b.CanFreeze) b.Freeze();
        return b;
    }
}
