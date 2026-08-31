using System.Text.Json;
using System.Text.Json.Serialization;

namespace CubeCheck;

[JsonConverter(typeof(AutosaveModeConverter))]
public enum AutosaveMode
{
    OnExit,
    OnChange,
    Off
}

public sealed class AutosaveModeConverter : JsonConverter<AutosaveMode>
{
    public override AutosaveMode Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options)
    {
        return reader.GetString() switch
        {
            "on_exit" => AutosaveMode.OnExit,
            "off" => AutosaveMode.Off,
            _ => AutosaveMode.OnChange
        };
    }

    public override void Write(Utf8JsonWriter writer, AutosaveMode value, JsonSerializerOptions options)
    {
        writer.WriteStringValue(value switch
        {
            AutosaveMode.OnExit => "on_exit",
            AutosaveMode.Off => "off",
            _ => "on_change"
        });
    }
}

public enum GlowArea
{
    Sidebar,
    About,
    System,
    Footer
}

public sealed class GlowAreas
{
    public bool Sidebar { get; set; } = true;
    public bool About { get; set; } = true;
    public bool System { get; set; } = true;
    public bool Footer { get; set; } = true;

    public bool Enabled(GlowArea area) => area switch
    {
        GlowArea.Sidebar => Sidebar,
        GlowArea.About => About,
        GlowArea.System => System,
        GlowArea.Footer => Footer,
        _ => false
    };
}

public sealed class RgbArrayConverter : JsonConverter<byte[]>
{
    static readonly byte[] Fallback = [212, 175, 55];

    public override byte[] Read(ref Utf8JsonReader reader, Type typeToConvert, JsonSerializerOptions options)
    {
        if (reader.TokenType == JsonTokenType.StartArray)
        {
            var list = new List<byte>(3);
            while (reader.Read() && reader.TokenType != JsonTokenType.EndArray)
            {
                if (reader.TokenType == JsonTokenType.Number && reader.TryGetByte(out var b))
                {
                    list.Add(b);
                }
                else
                {
                    reader.Skip();
                }
            }
            return list.Count >= 3 ? [list[0], list[1], list[2]] : Fallback;
        }

        if (reader.TokenType == JsonTokenType.String)
        {
            var raw = reader.GetString();
            if (!string.IsNullOrEmpty(raw))
            {
                try
                {
                    var decoded = Convert.FromBase64String(raw);
                    if (decoded.Length >= 3) return decoded;
                }
                catch
                {
                    // Rust and settings.default.json use [r,g,b]
                }
            }
        }

        reader.Skip();
        return Fallback;
    }

    public override void Write(Utf8JsonWriter writer, byte[] value, JsonSerializerOptions options)
    {
        var rgb = value is { Length: >= 3 } ? value : Fallback;
        writer.WriteStartArray();
        writer.WriteNumberValue(rgb[0]);
        writer.WriteNumberValue(rgb[1]);
        writer.WriteNumberValue(rgb[2]);
        writer.WriteEndArray();
    }
}

public sealed class GlowConfig
{
    public bool Enabled { get; set; } = true;
    [JsonConverter(typeof(RgbArrayConverter))]
    public byte[] Color { get; set; } = [212, 175, 55];
    [JsonConverter(typeof(RgbArrayConverter))]
    public byte[] Color2 { get; set; } = [255, 214, 90];
    public bool Gradient { get; set; }
    public float GradientSpeed { get; set; } = 1.0f;
    public float Radius { get; set; } = 34.0f;
    public float Intensity { get; set; } = 1.0f;
    public GlowAreas Areas { get; set; } = new();

    public void Sanitize()
    {
        Radius = Clamp(Radius, 34f, AppConfig.GlowRadiusMin, AppConfig.GlowRadiusMax);
        Intensity = Clamp(Intensity, 1f, AppConfig.GlowIntensityMin, AppConfig.GlowIntensityMax);
        GradientSpeed = Clamp(GradientSpeed, 1f, AppConfig.GlowSpeedMin, AppConfig.GlowSpeedMax);
        if (Color is not { Length: 3 }) Color = [212, 175, 55];
        if (Color2 is not { Length: 3 }) Color2 = [255, 214, 90];
        Areas ??= new GlowAreas();
    }

    public bool ActiveFor(GlowArea area) => Enabled && Areas.Enabled(area);

    static float Clamp(float value, float fallback, float min, float max)
    {
        if (!Polyfill.IsFinite(value)) return fallback;
        return Polyfill.Round2(Polyfill.Clamp(value, min, max));
    }
}

public sealed class AppConfig
{
    public const float ZoomMin = 0.5f;
    public const float ZoomMax = 2.5f;
    public const float GlowRadiusMin = 8f;
    public const float GlowRadiusMax = 80f;
    public const float GlowIntensityMin = 0.2f;
    public const float GlowIntensityMax = 2f;
    public const float GlowSpeedMin = 0.1f;
    public const float GlowSpeedMax = 5f;

    public string Theme { get; set; } = "black";
    public float Zoom { get; set; } = 1.0f;
    public GlowConfig Glow { get; set; } = new();
    public AutosaveMode Autosave { get; set; } = AutosaveMode.OnChange;

    [JsonIgnore]
    public ThemeId ThemeId => ThemeColors.FromKey(Theme);

    public void SetTheme(ThemeId id)
    {
        Theme = ThemeColors.ToKey(id);
        Sanitize();
    }

    public void SetZoom(float zoom) => Zoom = ClampZoom(zoom);

    public void Sanitize()
    {
        Zoom = ClampZoom(Zoom);
        Glow ??= new GlowConfig();
        Glow.Sanitize();
        Theme = ThemeColors.ToKey(ThemeColors.FromKey(Theme));
    }

    public static float ClampZoom(float zoom)
    {
        if (!Polyfill.IsFinite(zoom)) return 1f;
        return Polyfill.Round2(Polyfill.Clamp(zoom, ZoomMin, ZoomMax));
    }

    public static AppConfig Load()
    {
        var path = AppPaths.SettingsPath;
        if (File.Exists(path))
        {
            return FromJson(File.ReadAllText(path));
        }

        AppPaths.MigrateLegacySettings();
        if (File.Exists(path))
        {
            return FromJson(File.ReadAllText(path));
        }

        foreach (var legacy in AppPaths.ListLegacySettingsPaths())
        {
            if (!File.Exists(legacy)) continue;
            try
            {
                var cfg = FromJson(File.ReadAllText(legacy));
                if (cfg.Autosave == AutosaveMode.OnChange)
                {
                    try { cfg.Save(); } catch { /* ignore */ }
                }
                return cfg;
            }
            catch
            {
                // next candidate
            }
        }

        return new AppConfig();
    }

    public static AppConfig FromJson(string text)
    {
        try
        {
            var cfg = JsonSerializer.Deserialize(text, CubeCheckJsonContext.Default.AppConfig)
                      ?? new AppConfig();
            cfg.Sanitize();
            return cfg;
        }
        catch
        {
            return new AppConfig();
        }
    }

    public string ToJson()
    {
        Sanitize();
        return JsonSerializer.Serialize(this, CubeCheckJsonContext.Default.AppConfig);
    }

    public AppConfig Clone() => FromJson(ToJson());

    public void Save()
    {
        var path = AppPaths.SettingsPath;
        var dir = Path.GetDirectoryName(path);
        if (!string.IsNullOrEmpty(dir))
        {
            Directory.CreateDirectory(dir);
        }
        File.WriteAllText(path, ToJson());
    }
}
