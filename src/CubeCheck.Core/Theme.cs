namespace CubeCheck;

public readonly record struct Rgb(byte R, byte G, byte B);

public enum ThemeId
{
    Black,
    White,
    Purple,
    Blue,
    Lime
}

public sealed record ThemeColors(
    Rgb Bg,
    Rgb Fg,
    Rgb Card,
    Rgb Hover,
    Rgb Select,
    Rgb Accent,
    Rgb TextDim,
    Rgb Border,
    Rgb ButtonBg,
    Rgb Section,
    Rgb Footer,
    Rgb Track,
    Rgb InputBg,
    Rgb WidgetOutline,
    Rgb Handle,
    bool Light)
{
    public static ThemeId FromKey(string? key) =>
        (key ?? "").Trim().ToLowerInvariant() switch
        {
            "white" => ThemeId.White,
            "purple" => ThemeId.Purple,
            "blue" => ThemeId.Blue,
            "lime" => ThemeId.Lime,
            _ => ThemeId.Black
        };

    public static string ToKey(ThemeId id) => id switch
    {
        ThemeId.White => "white",
        ThemeId.Purple => "purple",
        ThemeId.Blue => "blue",
        ThemeId.Lime => "lime",
        _ => "black"
    };

    public static string Label(ThemeId id) => id switch
    {
        ThemeId.White => "Белая",
        ThemeId.Purple => "Фиолетовая",
        ThemeId.Blue => "Синяя",
        ThemeId.Lime => "Лаймовая",
        _ => "Чёрная"
    };

    public static ThemeId[] All { get; } =
        [ThemeId.Black, ThemeId.White, ThemeId.Purple, ThemeId.Blue, ThemeId.Lime];

    public static ThemeColors For(ThemeId id) => id switch
    {
        ThemeId.White => new(
            Hex("#e6e6e6"), Hex("#1a1a1a"), Hex("#ffffff"), Hex("#d4d4d4"), Hex("#c5d0e0"),
            Hex("#3d5a85"), Hex("#3f3f3f"), Hex("#7a7a7a"), Hex("#d0d0d0"), Hex("#4a4a4a"),
            Hex("#3f3f3f"), Hex("#6a6a6a"), Hex("#e8e8e8"), Hex("#4a4a4a"), Hex("#2a2a2a"), true),
        ThemeId.Purple => new(
            Hex("#0d0a1a"), Hex("#d4c8f0"), Hex("#1a122a"), Hex("#2a1a3a"), Hex("#32204e"),
            Hex("#a882d8"), Hex("#b0a0c4"), Hex("#4a3470"), Hex("#251838"), Hex("#8a70a0"),
            Hex("#8a70a8"), Hex("#3a2858"), Hex("#100a18"), Hex("#7a58a8"), Hex("#e0d0f8"), false),
        ThemeId.Blue => new(
            Hex("#0a0f1a"), Hex("#c8d8f0"), Hex("#121a2a"), Hex("#1a2a3a"), Hex("#1e3250"),
            Hex("#5880c8"), Hex("#9aa8c0"), Hex("#3a5070"), Hex("#1a2438"), Hex("#607090"),
            Hex("#7088a8"), Hex("#2a3c58"), Hex("#0a1018"), Hex("#6080a8"), Hex("#d0dcec"), false),
        ThemeId.Lime => new(
            Hex("#0a0f0a"), Hex("#d0e8c0"), Hex("#121f12"), Hex("#1a2f1a"), Hex("#1e3a1e"),
            Hex("#80c850"), Hex("#90b090"), Hex("#3a5a3a"), Hex("#1a2c1a"), Hex("#608060"),
            Hex("#80a070"), Hex("#2a4a2a"), Hex("#081008"), Hex("#5a8a50"), Hex("#d8f0c8"), false),
        _ => new(
            Hex("#0a0a0f"), Hex("#e0e0f0"), Hex("#15151f"), Hex("#20202e"), Hex("#252540"),
            Hex("#8a7ad8"), Hex("#a0aabf"), Hex("#3a3a55"), Hex("#1c1c2e"), Hex("#6a6a8a"),
            Hex("#4a4a6a"), Hex("#3a3a50"), Hex("#0c0c14"), Hex("#6a6a88"), Hex("#d0d0e8"), false)
    };

    public static Rgb Hex(string s)
    {
        s = s.TrimStart('#');
        return new Rgb(Convert.ToByte(s.Substring(0, 2), 16), Convert.ToByte(s.Substring(2, 2), 16), Convert.ToByte(s.Substring(4, 2), 16));
    }

    public static Rgb FromBytes(byte[]? rgb, Rgb fallback)
    {
        if (rgb is { Length: >= 3 })
        {
            return new Rgb(rgb[0], rgb[1], rgb[2]);
        }
        return fallback;
    }
}
