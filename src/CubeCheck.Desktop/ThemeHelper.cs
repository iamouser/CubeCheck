using Avalonia.Media;
using CubeCheck;

namespace CubeCheck.Desktop;

static class ThemeHelper
{
    public static Color ToMedia(Rgb rgb) => Color.FromRgb(rgb.R, rgb.G, rgb.B);

    public static Color ToMedia(byte[] rgb, Rgb fallback)
    {
        var c = ThemeColors.FromBytes(rgb, fallback);
        return Color.FromRgb(c.R, c.G, c.B);
    }

    public static SolidColorBrush Brush(Rgb rgb) => new(ToMedia(rgb));

    public static SolidColorBrush Brush(Color c) => new(c);
}
