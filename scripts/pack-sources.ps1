# Pack CubeCheck sources into build\sources and build\CubeCheck-<ver>-sources.zip
# (no build artifacts, no vendor tools, no dist/build output).
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$Version = '1.1.1'
$staging = Join-Path $root "build\sources"
$zip = Join-Path $root "build\CubeCheck-$Version-sources.zip"

$excludeDirNames = @('obj', 'target', 'build', 'dist', 'dist-dotnet', '.vs', '.zig-wrappers', 'posix-cache', '.git', '.idea', '.vscode')
$excludeFileNames = @(
    'Everything.exe', 'Everything.db', 'Everything.ini',
    'Shellbag.exe', 'Procmon64.exe', 'Autoruns64.exe', 'procexp64.exe',
    'settings.json', 'CubeCheck-error.txt'
)
$excludeExtensions = @('.exe', '.dll', '.pdb', '.log', '.pem', '.key', '.user', '.suo')

function Should-SkipFile([System.IO.FileInfo]$f) {
    if ($excludeFileNames -contains $f.Name) { return $true }
    if ($f.Extension -in $excludeExtensions) { return $true }
    if ($f.FullName -match '\\SystemInformer\\') { return $true }
    return $false
}

function Copy-SourceTree([string]$srcRel) {
    $src = Join-Path $root $srcRel
    if (-not (Test-Path -LiteralPath $src)) { return }
    $dst = Join-Path $staging $srcRel
    Get-ChildItem -LiteralPath $src -Recurse -Force -ErrorAction SilentlyContinue | ForEach-Object {
        $rel = $_.FullName.Substring($src.Length).TrimStart('\', '/')
        if ($rel -match '(^|\\)(obj|target)(\\|$)') { return }
        if ($rel -match '(^|\\)bin(\\|$)' -and $rel -notmatch '(^|\\)src\\bin(\\|$)') { return }
        if ($rel -match '(^|\\)native\\bin(\\|$)') { return }
        foreach ($skip in $excludeDirNames) {
            if ($rel -match "(^|\\)$([regex]::Escape($skip))(\\|$)") { return }
        }
        $out = Join-Path $dst $rel
        if ($_.PSIsContainer) {
            New-Item -ItemType Directory -Force -Path $out | Out-Null
        } elseif (-not (Should-SkipFile $_)) {
            $parent = Split-Path -Parent $out
            if (-not (Test-Path -LiteralPath $parent)) { New-Item -ItemType Directory -Force -Path $parent | Out-Null }
            Copy-Item -LiteralPath $_.FullName -Destination $out -Force
        }
    }
}

if (Test-Path -LiteralPath $staging) { Remove-Item -LiteralPath $staging -Recurse -Force }
New-Item -ItemType Directory -Force -Path $staging | Out-Null

foreach ($dir in @('src', 'ui', 'crates', 'scripts', 'assets', 'dotnet', '.github')) {
    Copy-SourceTree $dir
}

foreach ($file in @('LICENSE.md', 'Cargo.toml', 'Cargo.lock', 'build.bat', 'build-dotnet.ps1', 'START.bat', '.gitignore')) {
    $from = Join-Path $root $file
    if (Test-Path -LiteralPath $from) {
        Copy-Item -LiteralPath $from -Destination (Join-Path $staging $file) -Force
    }
}

if (Test-Path -LiteralPath $zip) { Remove-Item -LiteralPath $zip -Force }
Compress-Archive -Path (Join-Path $staging '*') -DestinationPath $zip -CompressionLevel Optimal

$fileCount = (Get-ChildItem -LiteralPath $staging -Recurse -File -Force).Count
$dirSize = (Get-ChildItem -LiteralPath $staging -Recurse -File -Force | Measure-Object -Property Length -Sum).Sum
$zipSize = (Get-Item -LiteralPath $zip).Length

Write-Host "sources: $staging"
Write-Host "sources zip: $zip"
Write-Host "files: $fileCount"
Write-Host "tree size: $dirSize bytes"
Write-Host "zip size: $zipSize bytes"
