$ErrorActionPreference = 'Stop'
$upload = Join-Path $PSScriptRoot '..\build\github-upload' | Resolve-Path
$lines = New-Object System.Collections.Generic.List[string]
Get-ChildItem -LiteralPath $upload -Recurse -File | Where-Object { $_.Name -ne 'SHA256SUMS' } | ForEach-Object {
    $rel = $_.FullName.Substring($upload.Path.Length).TrimStart('\', '/').Replace('\', '/')
    $hash = (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
    [void]$lines.Add("$hash  $rel")
}
$utf8 = New-Object System.Text.UTF8Encoding $false
$sums = Join-Path $upload 'SHA256SUMS'
[System.IO.File]::WriteAllLines($sums, $lines, $utf8)
$zip = Join-Path (Join-Path $PSScriptRoot '..\build' | Resolve-Path) 'CubeCheck-1.1.0-beta-github-payload.zip'
if (Test-Path -LiteralPath $zip) { Remove-Item -LiteralPath $zip -Force }
Compress-Archive -Path (Join-Path $upload '*') -DestinationPath $zip -CompressionLevel Optimal
Get-Item -LiteralPath $zip | Format-List FullName, Length, LastWriteTime
