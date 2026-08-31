using System.IO.Compression;
using System.Net.Http;
using System.Security.Cryptography;
using System.Text.Json;
using System.Text.Json.Serialization;

namespace CubeCheck;

public enum ToolProgressKind
{
    Connecting,
    Receiving,
    Verifying,
    Extracting
}

public readonly record struct ToolProgress(ToolProgressKind Kind, long Received, long? Total);

public sealed class ExtractRule
{
    [JsonPropertyName("from")] public string From { get; set; } = "";
    [JsonPropertyName("to")] public string To { get; set; } = "";
}

public sealed class ToolSpec
{
    [JsonPropertyName("id")] public string Id { get; set; } = "";
    [JsonPropertyName("name")] public string Name { get; set; } = "";
    [JsonPropertyName("url")] public string Url { get; set; } = "";
    [JsonPropertyName("mirrors")] public List<string> Mirrors { get; set; } = [];
    [JsonPropertyName("sha256")] public string? Sha256 { get; set; }
    [JsonPropertyName("publisher")] public string Publisher { get; set; } = "";
    [JsonPropertyName("kind")] public string Kind { get; set; } = "";
    [JsonPropertyName("extract")] public List<ExtractRule> Extract { get; set; } = [];
    [JsonPropertyName("verify")] public List<string> Verify { get; set; } = [];
}

public sealed class ToolsManifest
{
    [JsonPropertyName("tools")] public List<ToolSpec> Tools { get; set; } = [];

    public ToolSpec? Get(string id) => Tools.Find(t => t.Id == id);
}

public static class Downloader
{
    const string UserAgent = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) CubeCheck/1.1-beta";

    public static bool DownloadsEnabled => AppPaths.ForensicToolsSupported && !AppPaths.IsOffline;

    public static string OfflineMissingMessage(string name, string path) =>
        $"{name} не найден в офлайн-сборке.\nОжидался файл: {path}\nЗагрузка из сети отключена.";

    public static ToolsManifest LoadManifest()
    {
        var path = AppPaths.ResourcePath("tools.json");
        var text = File.Exists(path)
            ? File.ReadAllText(path)
            : throw new InvalidOperationException("Не удалось прочитать список загрузок: нет tools.json");
        return JsonSerializer.Deserialize(text, CubeCheckJsonContext.Default.ToolsManifest)
               ?? throw new InvalidOperationException("Повреждён список загрудок");
    }

    public static List<ToolSpec> MissingTools(ToolsManifest manifest) =>
        manifest.Tools.Where(t => !AppPaths.ToolInstalled(t.Id)).ToList();

    public static void DownloadTool(ToolSpec spec, bool force, Action<ToolProgress> onProgress)
    {
        if (!force && AppPaths.ToolInstalled(spec.Id)) return;

        if (!AppPaths.ForensicToolsSupported)
        {
            throw new InvalidOperationException($"{spec.Name} работает только в Windows. На этой ОС файл не загружается.");
        }

        if (AppPaths.IsOffline)
        {
            throw new InvalidOperationException(OfflineMissingMessage(spec.Name, AppPaths.ToolPath(spec.Id)));
        }

        var urls = CandidateUrls(spec);
        onProgress(new ToolProgress(ToolProgressKind.Connecting, 0, null));

        var destDir = AppPaths.AssetsDir;
        Directory.CreateDirectory(destDir);
        var tmpDir = Path.Combine(Path.GetTempPath(), "cubecheck-dl");
        Directory.CreateDirectory(tmpDir);
        var tmpFile = Path.Combine(tmpDir, spec.Id + ".part");

        DownloadHttps(urls, spec.Kind != "exe", tmpFile, (received, total) =>
            onProgress(new ToolProgress(ToolProgressKind.Receiving, received, total)));

        if (!string.IsNullOrWhiteSpace(spec.Sha256))
        {
            onProgress(new ToolProgress(ToolProgressKind.Verifying, 0, null));
            var actual = Sha256File(tmpFile);
            if (!actual.Equals(spec.Sha256, StringComparison.OrdinalIgnoreCase))
            {
                TryDelete(tmpFile);
                throw new InvalidOperationException($"{spec.Name}: файл повреждён, скачайте снова");
            }
        }

        onProgress(new ToolProgress(ToolProgressKind.Extracting, 0, null));
        if (spec.Id == "systeminformer")
        {
            TryDeleteDir(Path.Combine(destDir, "SystemInformer"));
        }

        if (spec.Kind == "exe")
        {
            var to = spec.Extract.FirstOrDefault()?.To ?? spec.Verify.FirstOrDefault() ?? "tool.exe";
            var dest = SafeDest(destDir, to);
            Directory.CreateDirectory(Path.GetDirectoryName(dest)!);
            File.Copy(tmpFile, dest, overwrite: true);
        }
        else
        {
            ExtractSelected(tmpFile, destDir, spec.Extract);
        }

        if (spec.Id == "systeminformer")
        {
            FinalizeSystemInformer(destDir);
        }

        TryDelete(tmpFile);

        onProgress(new ToolProgress(ToolProgressKind.Verifying, 0, null));
        foreach (var rel in spec.Verify)
        {
            var path = Path.Combine(destDir, rel);
            try
            {
                Native.VerifyPublisher(path, spec.Publisher);
            }
            catch (Exception ex)
            {
                foreach (var rule in spec.Extract) TryDelete(Path.Combine(destDir, rule.To));
                foreach (var v in spec.Verify) TryDelete(Path.Combine(destDir, v));
                throw new InvalidOperationException(ex.Message);
            }
        }
    }

    static List<string> CandidateUrls(ToolSpec spec)
    {
        var urls = new List<string> { spec.Url };
        urls.AddRange(spec.Mirrors);
        urls.RemoveAll(string.IsNullOrWhiteSpace);
        if (urls.Exists(u => !u.StartsWith("https://", StringComparison.OrdinalIgnoreCase)))
        {
            throw new InvalidOperationException($"{spec.Name}: ссылка должна быть https");
        }
        if (urls.Count == 0) throw new InvalidOperationException($"{spec.Name}: нет URL для загрузки");
        return urls;
    }

    static void DownloadHttps(List<string> urls, bool expectZip, string dest, Action<long, long?> onChunk)
    {
        var errors = new List<string>();
        foreach (var url in urls)
        {
            try
            {
                DownloadViaHttp(url, dest, onChunk);
                AcceptPayload(dest, expectZip);
                return;
            }
            catch (Exception ex)
            {
                errors.Add(ex.Message);
                TryDelete(dest);
            }
        }
        throw new InvalidOperationException(errors.Count == 1 ? errors[0] : "Не удалось скачать:\n" + string.Join("\n", errors));
    }

    static void DownloadViaHttp(string url, string dest, Action<long, long?> onChunk)
    {
        using var handler = new HttpClientHandler { AllowAutoRedirect = true };
        using var client = new HttpClient(handler) { Timeout = TimeSpan.FromSeconds(180) };
        client.DefaultRequestHeaders.UserAgent.ParseAdd(UserAgent);
        using var response = client.GetAsync(url, HttpCompletionOption.ResponseHeadersRead).GetAwaiter().GetResult();
        if ((int)response.StatusCode is < 200 or >= 300)
        {
            throw new InvalidOperationException($"HTTP {(int)response.StatusCode} при загрузке {url}");
        }

        var total = response.Content.Headers.ContentLength;
        using var stream = response.Content.ReadAsStreamAsync().GetAwaiter().GetResult();
        using var file = File.Create(dest);
        var buf = new byte[64 * 1024];
        long received = 0;
        int n;
        while ((n = stream.Read(buf, 0, buf.Length)) > 0)
        {
            file.Write(buf, 0, n);
            received += n;
            onChunk(received, total);
        }
        file.Flush();
        if (received < 1024)
        {
            throw new InvalidOperationException("Скачался не тот файл. Нажмите «Повтор».");
        }
    }

    static void AcceptPayload(string dest, bool expectZip)
    {
        if (expectZip && !LooksLikeZip(dest))
        {
            TryDelete(dest);
            throw new InvalidOperationException("Скачался не архив. Нажмите «Повтор».");
        }
    }

    static bool LooksLikeZip(string path)
    {
        using var file = File.OpenRead(path);
        var magic = new byte[2];
        return file.Read(magic, 0, 2) == 2 && magic[0] == (byte)'P' && magic[1] == (byte)'K';
    }

    static string Sha256File(string path)
    {
        using var file = File.OpenRead(path);
        using var sha = SHA256.Create();
        var hash = sha.ComputeHash(file);
        return BitConverter.ToString(hash).Replace("-", "").ToLowerInvariant();
    }

    static void ExtractSelected(string zipPath, string destDir, List<ExtractRule> rules)
    {
        using var archive = ZipFile.OpenRead(zipPath);
        foreach (var rule in rules)
        {
            if (IsSkipped(rule.From)) continue;
            var entry = FindZipEntry(archive, rule.From) ??
                        throw new InvalidOperationException($"В архиве нет файла {rule.From}");
            if (string.IsNullOrEmpty(entry.Name) || entry.FullName.EndsWith("/", StringComparison.Ordinal)) continue;
            var dest = SafeDest(destDir, rule.To);
            Directory.CreateDirectory(Path.GetDirectoryName(dest)!);
            entry.ExtractToFile(dest, overwrite: true);
        }
    }

    static ZipArchiveEntry? FindZipEntry(ZipArchive archive, string wanted)
    {
        wanted = wanted.Replace('\\', '/');
        var wantedL = wanted.ToLowerInvariant();
        var wantedName = Path.GetFileName(wanted);
        ZipArchiveEntry? best = null;
        var bestScore = int.MinValue;
        foreach (var entry in archive.Entries)
        {
            var name = entry.FullName.Replace('\\', '/');
            var lower = name.ToLowerInvariant();
            if (IsSkipped(name) || PathIs32Bit(lower)) continue;
            if (!string.Equals(Path.GetFileName(name), wantedName, StringComparison.OrdinalIgnoreCase)) continue;
            var score = ArchScore(lower);
            if (lower == wantedL || lower.EndsWith("/" + wantedL, StringComparison.Ordinal)) score += 50;
            if (score > bestScore)
            {
                bestScore = score;
                best = entry;
            }
        }
        return best;
    }

    static bool PathIs32Bit(string lower) =>
        lower.Split('/').Any(p => p is "x86" or "win32" or "i386" or "ia32" or "wow64");

    static int ArchScore(string lower) =>
        lower.Split('/').Any(p => p is "amd64" or "x64" or "win64") ? 100 : 0;

    static bool IsSkipped(string name)
    {
        var lower = name.Replace('\\', '/').ToLowerInvariant();
        return lower.Contains("/plugins/") || lower.Contains("/peview") || lower.Contains("/resources/") ||
               lower.EndsWith("/plugins") || lower.EndsWith("/resources");
    }

    public static void StripSystemInformerExtras(string siDir)
    {
        foreach (var extra in new[] { "plugins", "peview", "Resources", "x86" })
        {
            TryDeleteDir(Path.Combine(siDir, extra));
        }
    }

    public static void WriteSystemInformerSettings(string siDir)
    {
        Directory.CreateDirectory(siDir);
        const string settings =
            "<settings>\n" +
            "<setting name=\"EnablePlugins\">0</setting>\n" +
            "<setting name=\"EnableDefaultSafePlugins\">0</setting>\n" +
            "<setting name=\"DisabledPlugins\">" +
            "DotNetTools.dll|ExtendedNotifications.dll|ExtendedServices.dll|" +
            "ExtendedTools.dll|HardwareDevices.dll|NetworkTools.dll|" +
            "OnlineChecks.dll|ToolStatus.dll|Updater.dll|UserNotes.dll|WindowExplorer.dll" +
            "</setting>\n" +
            "</settings>\n";
        File.WriteAllText(Path.Combine(siDir, "SystemInformer.exe.settings.xml"), settings);
    }

    static void FinalizeSystemInformer(string destDir)
    {
        var siDir = Path.Combine(destDir, "SystemInformer");
        StripSystemInformerExtras(siDir);
        var exe = Path.Combine(siDir, "SystemInformer.exe");
        if (File.Exists(exe) && !Native.IsPeAmd64(exe))
        {
            TryDeleteDir(siDir);
            throw new InvalidOperationException("Скачалась 32-битная версия. Нажмите «Повтор».");
        }
        WriteSystemInformerSettings(siDir);
    }

    static string SafeDest(string baseDir, string rel)
    {
        rel = rel.Replace('\\', '/');
        if (string.IsNullOrWhiteSpace(rel) || rel.Split('/').Contains(".."))
        {
            throw new InvalidOperationException("некорректный путь распаковки");
        }
        return Path.Combine(baseDir, rel.Replace('/', Path.DirectorySeparatorChar));
    }

    static void TryDelete(string path)
    {
        try { if (File.Exists(path)) File.Delete(path); } catch { /* ignore */ }
    }

    static void TryDeleteDir(string path)
    {
        try { if (Directory.Exists(path)) Directory.Delete(path, recursive: true); } catch { /* ignore */ }
    }
}
