using System.Text.Json.Serialization;

namespace CubeCheck;

[JsonSourceGenerationOptions(
    PropertyNamingPolicy = JsonKnownNamingPolicy.SnakeCaseLower,
    PropertyNameCaseInsensitive = true,
    WriteIndented = true,
    UseStringEnumConverter = false,
    ReadCommentHandling = System.Text.Json.JsonCommentHandling.Skip,
    AllowTrailingCommas = true)]
[JsonSerializable(typeof(AutosaveMode))]
[JsonSerializable(typeof(AppConfig))]
[JsonSerializable(typeof(GlowConfig))]
[JsonSerializable(typeof(GlowAreas))]
[JsonSerializable(typeof(ToolsManifest))]
[JsonSerializable(typeof(ToolSpec))]
[JsonSerializable(typeof(ExtractRule))]
[JsonSerializable(typeof(byte[]))]
public partial class CubeCheckJsonContext : JsonSerializerContext
{
}
