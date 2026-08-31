namespace CubeCheck;

public static class ReportWriter
{
    public static string Save(IReadOnlyList<string> findings)
    {
        var now = DateTime.Now;
        var filename = $"{now:dd.MM.yy}-time{now:HH.mm.ss}.yml";
        var dir = AppPaths.ReportsDir;
        Directory.CreateDirectory(dir);
        var filepath = Path.Combine(dir, filename);

        var yaml = new System.Text.StringBuilder();
        yaml.AppendLine("name: CubeCheck");
        yaml.AppendLine($"authors: {Content.Authors}");
        yaml.AppendLine($"version: \"{Content.AppVersion}\"");
        yaml.AppendLine($"saved_at: {Y(now.ToString("dd.MM.yyyy HH:mm:ss"))}");
        yaml.AppendLine($"computer: {Y(Environment.MachineName)}");
        yaml.AppendLine($"windows_install: {Y(Native.InstallDate())}");
        yaml.AppendLine($"recycle_bin: {Y(Native.RecycleMtime())}");
        yaml.AppendLine("auto_check:");
        if (findings.Count == 0)
        {
            yaml.AppendLine("  ran: false");
            yaml.AppendLine("  findings: []");
        }
        else
        {
            yaml.AppendLine("  ran: true");
            yaml.AppendLine("  findings:");
            foreach (var line in findings)
            {
                yaml.AppendLine($"    - {Y(line)}");
            }
        }
        yaml.AppendLine("utilities:");
        foreach (var util in Catalog.Utils)
        {
            yaml.AppendLine($"  - id: {Y(util.Id)}");
            yaml.AppendLine($"    name: {Y(util.Name)}");
            yaml.AppendLine($"    description: {Y(util.Desc)}");
        }
        yaml.AppendLine("channel: telegram.me/cubecheck");

        File.WriteAllText(filepath, yaml.ToString());
        return filepath;
    }

    static string Y(string value) =>
        "\"" + value.Replace("\\", "\\\\").Replace("\"", "\\\"").Replace("\n", "\\n").Replace("\r", "") + "\"";
}
