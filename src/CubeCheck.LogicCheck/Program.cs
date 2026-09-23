using CubeCheck;

static void Expect(bool cond, string msg)
{
    if (!cond) throw new InvalidOperationException(msg);
}

Expect(!AppVersionNumber.IsNewer("1.1.1", "1.1.1"), "same version");
Expect(AppVersionNumber.IsNewer("1.1.2", "1.1.1"), "patch newer");
Expect(!AppVersionNumber.IsNewer("1.1.0", "1.1.1"), "patch older");
Expect(!AppVersionNumber.IsNewer("1.1.0-beta", "1.1.1"), "beta is not newer than 1.1.1");
Expect(AppVersionNumber.IsNewer("1.1.1", "1.1.0-beta"), "1.1.1 is newer than beta");
Expect(AppVersionNumber.IsNewer("1.2.0", "1.1.1"), "minor");
Expect(!AppVersionNumber.IsNewer("not-a-version", "1.1.1"), "garbage");
Expect(Content.AppVersion == "1.1.1", "app version");

Expect(HexColor.TryParse("#D4AF37", out var gold) && gold[0] == 212 && gold[1] == 175 && gold[2] == 55, "hex");
Expect(HexColor.TryParse("#fff", out var white) && white[0] == 255 && white[1] == 255 && white[2] == 255, "short");
Expect(HexColor.TryParse("#80D4AF37", out var alpha) && alpha[0] == 0xD4 && alpha[1] == 0xAF && alpha[2] == 0x37, "alpha");
Expect(!HexColor.TryParse("zz", out _), "bad hex");
Expect(!HexColor.TryParse("", out _), "empty hex");
Expect(!HexColor.TryParse("#12", out _), "short junk");
Expect(HexColor.Format([212, 175, 55]) == "#D4AF37", "format");

var mixed = AppConfig.FromJson("""{"glow":{"color":[1,2,3],"color2":"#010203"},"check_updates":false}""");
Expect(mixed.Glow.Color[0] == 1 && mixed.Glow.Color[1] == 2 && mixed.Glow.Color[2] == 3, "legacy rgb");
Expect(mixed.Glow.Color2[0] == 1 && mixed.Glow.Color2[1] == 2 && mixed.Glow.Color2[2] == 3, "hex color2");
Expect(!mixed.CheckUpdates, "updates off");
var json = mixed.ToJson();
Expect(json.Contains("#010203", StringComparison.OrdinalIgnoreCase), "stores hex");
Expect(!json.Contains("[1, 2, 3]") && !json.Contains("[1,2,3]"), "does not store rgb array");
var again = AppConfig.FromJson(json);
Expect(again.Glow.Color[0] == 1 && !again.CheckUpdates, "roundtrip");

var bad = AppConfig.FromJson("""{"glow":{"color":"#zzzzzz"}}""");
Expect(bad.Glow.Color[0] == 212 && bad.CheckUpdates, "bad hex falls back, updates default on");

Expect(AppPaths.ShouldPreserveOnUpgrade("settings.json"), "root settings");
Expect(AppPaths.ShouldPreserveOnUpgrade("payload\\settings.json"), "nested settings");
Expect(AppPaths.ShouldPreserveOnUpgrade("payload/settings.json"), "posix nested");
Expect(!AppPaths.ShouldPreserveOnUpgrade("assets/settings.default.json"), "template");
Expect(!AppPaths.ShouldPreserveOnUpgrade("cubecheck.exe"), "exe");
Expect(!AppPaths.ShouldPreserveOnUpgrade(""), "empty");

var offer = AppUpdate.ParseOffer("""{"version":"9.9.9","installer":"https://example.com/CubeCheck-Setup.exe"}""");
Expect(offer != null && offer.InstallerUrl.EndsWith(".exe", StringComparison.Ordinal), "newer offer");
Expect(AppUpdate.ParseOffer("""{"version":"1.1.1","installer":"https://example.com/a.exe"}""") == null, "same remote");
Expect(AppUpdate.ParseOffer("""{"version":"1.1.0-beta","installer":"https://example.com/a.exe"}""") == null, "older remote");
Expect(AppUpdate.ParseOffer("""{"version":"9.9.9"}""") == null, "no installer");
Expect(AppUpdate.ParseOffer("""{"version":"9.9.9","installer":"http://example.com/a.exe"}""") == null, "http rejected");

Console.WriteLine("logic checks ok");
