using System.Net.Http;
using System.Text.Json;

namespace CubeCheck;

public sealed class UpdateOffer
{
    public string Version { get; set; } = "";
    public string Installer { get; set; } = "";

    public string InstallerUrl => Installer;

    public string ToJson() => JsonSerializer.Serialize(this, CubeCheckJsonContext.Default.UpdateOffer);
}

public static class AppUpdate
{
    public static readonly TimeSpan CheckInterval = TimeSpan.FromMinutes(10);
    public static readonly TimeSpan StartupGrace = TimeSpan.FromSeconds(6);
    public static readonly TimeSpan ManifestTimeout = TimeSpan.FromSeconds(12);
    public static readonly TimeSpan DownloadTimeout = TimeSpan.FromMinutes(3);

    public static UpdateOffer? Check()
    {
        try
        {
            return CheckAsync(CancellationToken.None).GetAwaiter().GetResult();
        }
        catch
        {
            return null;
        }
    }

    public static async Task<UpdateOffer?> CheckAsync(CancellationToken ct)
    {
        if (AppPaths.IsOffline) return null;
        var url = ManifestUrl();
        try
        {
            using var cts = CancellationTokenSource.CreateLinkedTokenSource(ct);
            cts.CancelAfter(ManifestTimeout);
            using var client = CreateClient();
            using var response = await client.GetAsync(url, cts.Token).ConfigureAwait(false);
            if (!response.IsSuccessStatusCode) return null;
            var text = await response.Content.ReadAsStringAsync().ConfigureAwait(false);
            return ParseOffer(text);
        }
        catch
        {
            return null;
        }
    }

    public static string ManifestUrl()
    {
        var env = Environment.GetEnvironmentVariable("CUBECHECK_UPDATE_URL");
        if (!string.IsNullOrWhiteSpace(env)) return env.Trim();
        return Content.UpdateManifestUrl;
    }

    public static UpdateOffer? ParseOffer(string json)
    {
        try
        {
            using var doc = JsonDocument.Parse(json);
            var root = doc.RootElement;
            if (root.ValueKind != JsonValueKind.Object) return null;
            if (!root.TryGetProperty("version", out var versionEl) || versionEl.ValueKind != JsonValueKind.String)
            {
                return null;
            }
            var version = versionEl.GetString();
            if (!AppVersionNumber.IsNewer(version, Content.AppVersion)) return null;
            var installer = ReadInstaller(root);
            if (installer == null || string.IsNullOrWhiteSpace(installer)) return null;
            if (!installer.StartsWith("https://", StringComparison.OrdinalIgnoreCase)) return null;
            return new UpdateOffer { Version = version ?? "", Installer = installer };
        }
        catch
        {
            return null;
        }
    }

    public static string DownloadInstaller(string url) =>
        DownloadInstallerAsync(url, CancellationToken.None).GetAwaiter().GetResult();

    public static async Task<string> DownloadInstallerAsync(string url, CancellationToken ct)
    {
        if (string.IsNullOrWhiteSpace(url) ||
            !url.StartsWith("https://", StringComparison.OrdinalIgnoreCase))
        {
            throw new InvalidOperationException("Ссылка на обновление должна быть HTTPS.");
        }

        var dir = Path.Combine(Path.GetTempPath(), "cubecheck-update");
        Directory.CreateDirectory(dir);
        var name = "CubeCheck-Setup.exe";
        if (Uri.TryCreate(url, UriKind.Absolute, out var uri))
        {
            var leaf = Path.GetFileName(uri.LocalPath);
            if (!string.IsNullOrEmpty(leaf) &&
                leaf.EndsWith(".exe", StringComparison.OrdinalIgnoreCase) &&
                leaf.IndexOfAny(Path.GetInvalidFileNameChars()) < 0)
            {
                name = leaf;
            }
        }
        var dest = Path.Combine(dir, name);
        try { if (File.Exists(dest)) File.Delete(dest); } catch { /* overwrite below */ }

        using var cts = CancellationTokenSource.CreateLinkedTokenSource(ct);
        cts.CancelAfter(DownloadTimeout);
        using var client = CreateClient(new HttpClientHandler { AllowAutoRedirect = true });
        using var response = await client
            .GetAsync(url, HttpCompletionOption.ResponseHeadersRead, cts.Token)
            .ConfigureAwait(false);
        if (!response.IsSuccessStatusCode)
        {
            throw new InvalidOperationException($"HTTP {(int)response.StatusCode}");
        }

        var input = await response.Content.ReadAsStreamAsync().ConfigureAwait(false);
        try
        {
            using var output = File.Create(dest);
            var buf = new byte[64 * 1024];
            long received = 0;
            int n;
            while ((n = await input.ReadAsync(buf, 0, buf.Length, cts.Token).ConfigureAwait(false)) > 0)
            {
                await output.WriteAsync(buf, 0, n, cts.Token).ConfigureAwait(false);
                received += n;
            }
            await output.FlushAsync(cts.Token).ConfigureAwait(false);
            if (received < 20_000)
            {
                throw new InvalidOperationException("Файл обновления слишком маленький.");
            }
        }
        catch
        {
            try { File.Delete(dest); } catch { /* ignore */ }
            throw;
        }
        finally
        {
            input.Dispose();
        }

        return dest;
    }

    public static void StartInstaller(string exePath, string installDir)
    {
        if (string.IsNullOrWhiteSpace(exePath) || !File.Exists(exePath))
        {
            throw new InvalidOperationException("Установщик обновления не найден.");
        }
        var dest = string.IsNullOrWhiteSpace(installDir) ? AppPaths.DataDir : installDir;
        var args = "--install --accepted --launch --dest \"" + dest.Replace("\"", "") + "\"";
        ProcessStart(exePath, args);
    }

    static void ProcessStart(string exePath, string args)
    {
        var started = System.Diagnostics.Process.Start(new System.Diagnostics.ProcessStartInfo
        {
            FileName = exePath,
            Arguments = args,
            UseShellExecute = true
        });
        if (started == null)
        {
            throw new InvalidOperationException("Не удалось запустить установщик.");
        }
    }

    static string? ReadInstaller(JsonElement root)
    {
        foreach (var key in new[] { "installer", "setup" })
        {
            if (root.TryGetProperty(key, out var el) && el.ValueKind == JsonValueKind.String)
            {
                var value = el.GetString();
                if (value != null && !string.IsNullOrWhiteSpace(value)) return value.Trim();
            }
        }
        if (root.TryGetProperty("url", out var urlEl) && urlEl.ValueKind == JsonValueKind.String)
        {
            var value = urlEl.GetString();
            if (value != null &&
                !string.IsNullOrWhiteSpace(value) &&
                value.EndsWith(".exe", StringComparison.OrdinalIgnoreCase))
            {
                return value.Trim();
            }
        }
        return null;
    }

    static HttpClient CreateClient(HttpMessageHandler? handler = null)
    {
        var client = handler == null ? new HttpClient() : new HttpClient(handler);
        client.Timeout = Timeout.InfiniteTimeSpan;
        client.DefaultRequestHeaders.UserAgent.ParseAdd(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) CubeCheck/" + Content.AppVersion);
        return client;
    }
}
