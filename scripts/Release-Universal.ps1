# Dot-sourced from build.ps1. Uses $root $dist $buildOut $Version and helper fns.

function Test-MachO([string]$Path) {
    if (-not (Test-Path -LiteralPath $Path)) { return $false }
    $fs = [System.IO.File]::OpenRead($Path)
    try {
        $b = New-Object byte[] 4
        if ($fs.Read($b, 0, 4) -lt 4) { return $false }
        $be = ($b[0] -eq 0xFE -and $b[1] -eq 0xED -and $b[2] -eq 0xFA -and ($b[3] -eq 0xCE -or $b[3] -eq 0xCF))
        $le = ($b[0] -eq 0xCF -or $b[0] -eq 0xCE) -and $b[1] -eq 0xFA -and $b[2] -eq 0xED -and $b[3] -eq 0xFE
        $fat = $b[0] -eq 0xCA -and $b[1] -eq 0xFE -and $b[2] -eq 0xBA -and $b[3] -eq 0xBE
        $fatLe = $b[0] -eq 0xBE -and $b[1] -eq 0xBA -and $b[2] -eq 0xFE -and $b[3] -eq 0xCA
        return $be -or $le -or $fat -or $fatLe
    } finally { $fs.Close() }
}

function Test-MacPayload([string]$Src) {
    $bin = Join-Path $Src "cubecheck"
    if (-not (Test-MachO $bin)) { return $false }
    if ((Get-DllFiles $Src).Count -gt 0) { return $false }
    return $true
}

function Fetch-Url([string]$Url, [string]$Dest) {
    [Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
    $dir = Split-Path -Parent $Dest
    New-Item -ItemType Directory -Force -Path $dir | Out-Null
    if ((Test-Path -LiteralPath $Dest) -and (Get-Item -LiteralPath $Dest).Length -gt 2048) {
        return $true
    }
    $tmp = "$Dest.part"
    if (Test-Path -LiteralPath $tmp) { Remove-Item -LiteralPath $tmp -Force }
    Info "download $Url"
    $ua = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) CubeCheck/1.1-beta"
    & curl.exe -L --fail --retry 3 --retry-delay 2 -A $ua -o $tmp $Url
    if ($LASTEXITCODE -ne 0 -or -not (Test-Path -LiteralPath $tmp) -or (Get-Item -LiteralPath $tmp).Length -lt 1024) {
        Warn "не скачалось: $Url"
        if (Test-Path -LiteralPath $tmp) { Remove-Item -LiteralPath $tmp -Force -ErrorAction SilentlyContinue }
        return $false
    }
    Move-Item -LiteralPath $tmp -Destination $Dest -Force
    return $true
}

function Find-ZipEntryBest($Entries, [string]$Wanted) {
    $wanted = $Wanted.Replace('\', '/')
    $wantedL = $wanted.ToLowerInvariant()
    $wantedName = [IO.Path]::GetFileName($wanted)
    $best = $null
    $bestScore = [int]::MinValue
    foreach ($entry in $Entries) {
        $name = $entry.FullName.Replace('\', '/')
        $lower = $name.ToLowerInvariant()
        if ($lower -match '/(plugins|peview|resources)/') { continue }
        if ($wantedL -notmatch 'x86' -and $lower -match '/(x86|win32|i386|ia32|wow64)(/|$)') { continue }
        if ([IO.Path]::GetFileName($name) -ne $wantedName) { continue }
        $score = 0
        if ($lower -match '/(amd64|x64|win64)(/|$)') { $score += 100 }
        if ($lower -eq $wantedL -or $lower.EndsWith("/$wantedL")) { $score += 50 }
        if ($score -gt $bestScore) {
            $bestScore = $score
            $best = $entry
        }
    }
    if ($best) { return $best }
    foreach ($entry in $Entries) {
        if ([IO.Path]::GetFileName($entry.FullName) -eq $wantedName) { return $entry }
    }
    return $null
}

function Expand-ZipRules([string]$ZipPath, [string]$DestDir, $Rules) {
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $archive = [System.IO.Compression.ZipFile]::OpenRead($ZipPath)
    try {
        foreach ($rule in $Rules) {
            $from = [string]$rule.from
            $to = [string]$rule.to
            if (-not $from -or $from -eq ".") { continue }
            $entry = Find-ZipEntryBest $archive.Entries $from
            if (-not $entry) { throw "в архиве нет $from" }
            $dest = Join-Path $DestDir ($to.Replace('/', '\'))
            New-Item -ItemType Directory -Force -Path (Split-Path -Parent $dest) | Out-Null
            if (Test-Path -LiteralPath $dest) { Remove-Item -LiteralPath $dest -Force }
            [System.IO.Compression.ZipFileExtensions]::ExtractToFile($entry, $dest, $true)
        }
    } finally {
        $archive.Dispose()
    }
}

function Fetch-WindowsVendor([string]$DestAssets, [string]$Arch) {
    $manifestPath = Join-Path $root "assets\tools.json"
    $manifest = Get-Content -LiteralPath $manifestPath -Raw -Encoding UTF8 | ConvertFrom-Json
    $cache = Join-Path $dist "vendor-cache"
    New-Item -ItemType Directory -Force -Path $DestAssets, $cache | Out-Null
    $ok = $true
    foreach ($tool in $manifest.tools) {
        $urls = @($tool.url)
        if ($tool.mirrors) { $urls += @($tool.mirrors) }
        if ($Arch -eq "x86" -and $tool.id -eq "everything") {
            $urls = @("https://www.voidtools.com/Everything-1.4.1.1032.x86.zip")
        }
        $part = Join-Path $cache "$($tool.id)-$Arch.download"
        $got = $false
        foreach ($url in $urls) {
            if (Fetch-Url $url $part) { $got = $true; break }
        }
        if (-not $got) {
            Warn "vendor $($tool.id) ($Arch) не скачан"
            $ok = $false
            continue
        }
        try {
            if ($tool.kind -eq "exe") {
                $to = $tool.extract[0].to
                $dest = Join-Path $DestAssets ($to.Replace('/', '\'))
                New-Item -ItemType Directory -Force -Path (Split-Path -Parent $dest) | Out-Null
                Copy-Item -LiteralPath $part -Destination $dest -Force
            } else {
                $rules = @($tool.extract)
                if ($Arch -eq "x86") {
                    $map = @{
                        "Procmon64.exe" = "Procmon.exe"
                        "Autoruns64.exe" = "Autoruns.exe"
                        "procexp64.exe" = "procexp.exe"
                    }
                    $newRules = @()
                    foreach ($r in $rules) {
                        $from = [string]$r.from
                        $base = [IO.Path]::GetFileName($from)
                        if ($map.ContainsKey($base)) {
                            $newRules += [pscustomobject]@{ from = $map[$base]; to = $r.to }
                        } elseif ($tool.id -eq "systeminformer") {
                            $newRules += [pscustomobject]@{ from = ($from -replace 'amd64/', 'x86/'); to = $r.to }
                        } else {
                            $newRules += $r
                        }
                    }
                    $rules = $newRules
                }
                Expand-ZipRules $part $DestAssets $rules
            }
            foreach ($rel in @($tool.verify)) {
                $check = Join-Path $DestAssets ($rel.Replace('/', '\'))
                if (-not (Test-Path -LiteralPath $check)) {
                    throw "нет $rel после распаковки"
                }
            }
            if ($tool.id -eq "systeminformer") {
                $si = Join-Path $DestAssets "SystemInformer"
                foreach ($extra in @("plugins", "peview", "Resources", "x86")) {
                    $p = Join-Path $si $extra
                    if (Test-Path -LiteralPath $p) { Remove-Item -LiteralPath $p -Recurse -Force }
                }
            }
            Info "vendor $($tool.name) ($Arch) готов"
        } catch {
            Warn "vendor $($tool.id): $($_.Exception.Message)"
            $ok = $false
        }
    }
    $ini = Join-Path $root "assets\Everything.ini"
    if (Test-Path -LiteralPath $ini) {
        Copy-Item -LiteralPath $ini -Destination (Join-Path $DestAssets "Everything.ini") -Force
    }
    return $ok
}

function Find-ExtractedBinary([string]$Dir, [string]$Name) {
    Get-ChildItem -LiteralPath $Dir -Recurse -File -ErrorAction SilentlyContinue |
        Where-Object { $_.Name -eq $Name -or $_.Name -eq "$Name.exe" } |
        Select-Object -First 1
}

function Fetch-PosixArch([string]$ArchKey, [string]$DestBin) {
    $manifestPath = Join-Path $root "scripts\posix-tools.json"
    $manifest = Get-Content -LiteralPath $manifestPath -Raw -Encoding UTF8 | ConvertFrom-Json
    $cache = Join-Path $dist "posix-cache"
    $unpack = Join-Path $cache "unpack-$ArchKey"
    New-Item -ItemType Directory -Force -Path $DestBin, $cache | Out-Null
    $have = 0
    foreach ($tool in $manifest.tools) {
        $archProp = $tool.archives.PSObject.Properties[$ArchKey]
        if (-not $archProp) { continue }
        $arch = $archProp.Value
        $leaf = Split-Path -Leaf $arch.url
        $part = Join-Path $cache "$ArchKey-$leaf"
        if (-not (Fetch-Url $arch.url $part)) { continue }
        $dest = Join-Path $DestBin $tool.binary
        try {
            if ($arch.kind -eq "bin" -or $arch.kind -eq "appimage") {
                if ((Get-Item -LiteralPath $part).Length -lt 1024) { throw "слишком маленький файл" }
                Copy-Item -LiteralPath $part -Destination $dest -Force
            } else {
                $here = Join-Path $unpack $tool.id
                if (Test-Path -LiteralPath $here) { Remove-Item -LiteralPath $here -Recurse -Force }
                New-Item -ItemType Directory -Force -Path $here | Out-Null
                if ($arch.kind -eq "zip") {
                    Expand-Archive -LiteralPath $part -DestinationPath $here -Force
                } else {
                    tar -xf $part -C $here
                    if ($LASTEXITCODE -ne 0) { throw "tar $($tool.id) не удался" }
                }
                $bin = Find-ExtractedBinary $here $tool.binary
                if (-not $bin) { throw "в архиве нет $($tool.binary)" }
                Copy-Item -LiteralPath $bin.FullName -Destination $dest -Force
            }
            $have++
            Info "posix $($tool.name) ($ArchKey) готов ($((Get-Item -LiteralPath $dest).Length) байт)"
        } catch {
            Warn "posix $($tool.id) ($ArchKey): $($_.Exception.Message)"
        }
    }
    return $have
}

function Write-HelperIfAbsent([string]$Path, [string]$Text) {
    if ((Test-Path -LiteralPath $Path) -and (Get-Item -LiteralPath $Path).Length -gt 8192) {
        return
    }
    Write-UnixText $Path $Text
}

function Write-PosixWrappers([string]$BinDir) {
    New-Item -ItemType Directory -Force -Path $BinDir | Out-Null
    Write-HelperIfAbsent (Join-Path $BinDir "fsearch") @'
#!/bin/sh
set -eu
HERE=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
run() {
  if command -v x-terminal-emulator >/dev/null 2>&1; then
    exec x-terminal-emulator -e "$@"
  fi
  if command -v xterm >/dev/null 2>&1; then
    exec xterm -e "$@"
  fi
  exec "$@"
}
if [ -x "$HERE/fzf" ] && [ -x "$HERE/fd" ]; then
  run /bin/sh -c "\"$HERE/fd\" --type f | \"$HERE/fzf\""
fi
if [ -x "$HERE/fd" ]; then
  run "$HERE/fd" "$@"
fi
if [ -x "$HERE/rg" ]; then
  run "$HERE/rg" "$@"
fi
echo "CubeCheck: в assets/bin нет fd/rg/fzf" >&2
exit 1
'@
    Write-HelperIfAbsent (Join-Path $BinDir "missioncenter") @'
#!/bin/sh
set -eu
HERE=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
export APPIMAGE_EXTRACT_AND_RUN=1
if [ -x "$HERE/MissionCenter.AppImage" ]; then
  exec "$HERE/MissionCenter.AppImage" "$@"
fi
if [ -x "$HERE/btop" ]; then
  if command -v x-terminal-emulator >/dev/null 2>&1; then
    exec x-terminal-emulator -e "$HERE/btop" "$@"
  fi
  exec "$HERE/btop" "$@"
fi
if [ -x "$HERE/btm" ]; then
  exec "$HERE/btm" "$@"
fi
if [ -x "$HERE/procs" ]; then
  exec "$HERE/procs" "$@"
fi
echo "CubeCheck: в assets/bin нет btop/btm/procs" >&2
exit 1
'@
    Write-HelperIfAbsent (Join-Path $BinDir "sysdig") @'
#!/bin/sh
set -eu
HERE=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
run() {
  if command -v x-terminal-emulator >/dev/null 2>&1; then
    exec x-terminal-emulator -e "$@"
  fi
  exec "$@"
}
if [ -x "$HERE/lsof" ] && [ "$HERE/lsof" != "$0" ]; then
  run "$HERE/lsof" -nP
fi
if [ -x "$HERE/busybox" ]; then
  run "$HERE/busybox" lsof
fi
if command -v lsof >/dev/null 2>&1 && [ "$(command -v lsof)" != "$0" ]; then
  run lsof -nP
fi
if [ -x "$HERE/btop" ]; then
  run "$HERE/btop"
fi
echo "CubeCheck: в assets/bin нет lsof/busybox/btop" >&2
exit 1
'@
    Write-HelperIfAbsent (Join-Path $BinDir "lsof") @'
#!/bin/sh
set -eu
HERE=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
if [ -x "$HERE/busybox" ]; then
  if "$HERE/busybox" --list 2>/dev/null | grep -qx lsof; then
    exec "$HERE/busybox" lsof "$@"
  fi
fi
printf '%s\t%s\t%s\n' "PID" "CMD" "FILE"
for d in /proc/[0-9]*; do
  [ -d "$d/fd" ] || continue
  pid=${d#/proc/}
  comm=$(cat "$d/comm" 2>/dev/null || echo "?")
  ls -l "$d/fd" 2>/dev/null | awk -v pid="$pid" -v comm="$comm" 'NF>=11 { print pid "\t" comm "\t" $11 }'
done
'@
}

function Add-LinuxExtras([string]$BundleRoot, [string]$PayloadKey) {
    $extras = Join-Path $BundleRoot "extras"
    $archBin = if ($PayloadKey -eq "linux-x64") { Join-Path $extras "bin" } else { Join-Path $extras $PayloadKey }
    New-Item -ItemType Directory -Force -Path $extras | Out-Null
    Write-UnixText (Join-Path $extras "install-linux-tools.sh") (Get-Content -LiteralPath (Join-Path $root "scripts\extras-linux\install-linux-tools.sh") -Raw -Encoding UTF8)
    Write-UnixText (Join-Path $extras "README.txt") (Get-Content -LiteralPath (Join-Path $root "scripts\extras-linux\README.txt") -Raw -Encoding UTF8)
    if (Test-Path -LiteralPath $archBin) { Remove-Item -LiteralPath $archBin -Recurse -Force }
    New-Item -ItemType Directory -Force -Path $archBin | Out-Null
    $n = Fetch-PosixArch $PayloadKey $archBin
    Write-PosixWrappers $archBin
    $payloadBin = Join-Path $BundleRoot "payload\$PayloadKey\assets\bin"
    if (Test-Path -LiteralPath $payloadBin) { Remove-Item -LiteralPath $payloadBin -Recurse -Force }
    New-Item -ItemType Directory -Force -Path $payloadBin | Out-Null
    Copy-Item -Path (Join-Path $archBin "*") -Destination $payloadBin -Force -ErrorAction SilentlyContinue
    Info "linux extras $PayloadKey : $n portable binaries"
    return $n
}

function Add-MacExtras([string]$BundleRoot, [string[]]$ArchKeys) {
    $extras = Join-Path $BundleRoot "extras"
    $bin = Join-Path $extras "bin"
    New-Item -ItemType Directory -Force -Path $bin | Out-Null
    Write-UnixText (Join-Path $extras "install-macos-tools.sh") (Get-Content -LiteralPath (Join-Path $root "scripts\extras-macos\install-macos-tools.sh") -Raw -Encoding UTF8)
    Write-UnixText (Join-Path $extras "README.txt") (Get-Content -LiteralPath (Join-Path $root "scripts\extras-macos\README.txt") -Raw -Encoding UTF8)
    $n = 0
    foreach ($key in $ArchKeys) {
        $archBin = Join-Path $extras $key
        New-Item -ItemType Directory -Force -Path $archBin | Out-Null
        $got = Fetch-PosixArch $key $archBin
        $n += $got
        $payloadBin = Join-Path $BundleRoot "payload\$key\assets\bin"
        New-Item -ItemType Directory -Force -Path $payloadBin | Out-Null
        if ($got -gt 0) {
            Copy-Item -Path (Join-Path $archBin "*") -Destination $payloadBin -Force -ErrorAction SilentlyContinue
            Copy-Item -Path (Join-Path $archBin "*") -Destination $bin -Force -ErrorAction SilentlyContinue
        }
    }
    Info "macos extras : $n portable binaries"
    return $n
}

function Mark-Offline([string]$Dir) {
    Set-Content -Path (Join-Path $Dir ".offline") -Value ""
    $assets = Join-Path $Dir "assets"
    if (Test-Path -LiteralPath $assets) {
        Set-Content -Path (Join-Path $assets ".offline") -Value ""
    }
    Get-ChildItem -LiteralPath (Join-Path $Dir "payload") -Directory -ErrorAction SilentlyContinue | ForEach-Object {
        Set-Content -Path (Join-Path $_.FullName ".offline") -Value ""
        $pa = Join-Path $_.FullName "assets"
        if (Test-Path -LiteralPath $pa) {
            Set-Content -Path (Join-Path $pa ".offline") -Value ""
        }
    }
}

function Set-ZigCrossEnv([string]$RustTriple, [string]$ZigTarget) {
    $zig = Resolve-ZigAscii
    $filter = Ensure-ZigFilter
    $wrapDir = Join-Path $root ".zig-wrappers"
    New-Item -ItemType Directory -Force -Path $wrapDir | Out-Null
    $safe = $RustTriple.Replace("-", "_")
    $cc = Join-Path $wrapDir "zig-cc-$RustTriple.cmd"
    $ar = Join-Path $wrapDir "zig-ar-$RustTriple.cmd"
    Set-Content -LiteralPath $cc -Value "@echo off`r`n`"$filter`" `"$zig`" cc $ZigTarget %*" -Encoding ascii
    Set-Content -LiteralPath $ar -Value "@echo off`r`n`"$filter`" `"$zig`" ar %*" -Encoding ascii
    $envNameCc = "CC_$safe"
    $envNameCx = "CXX_$safe"
    $envNameAr = "AR_$safe"
    $linkerKey = ("CARGO_TARGET_{0}_LINKER" -f $RustTriple.Replace("-", "_").ToUpperInvariant())
    $arKey = ("CARGO_TARGET_{0}_AR" -f $RustTriple.Replace("-", "_").ToUpperInvariant())
    Set-Item -Path "env:$envNameCc" -Value $cc
    Set-Item -Path "env:$envNameCx" -Value $cc
    Set-Item -Path "env:$envNameAr" -Value $ar
    Set-Item -Path "env:$linkerKey" -Value $cc
    Set-Item -Path "env:$arKey" -Value $ar
    $env:PKG_CONFIG_ALLOW_CROSS = "1"
    $pkg = Join-Path $wrapDir "empty-pkgconfig"
    New-Item -ItemType Directory -Force -Path $pkg | Out-Null
    $env:PKG_CONFIG_LIBDIR = $pkg
}

function Build-RustWindowsX86([string]$OutDir) {
    Info "C++ + C# API + Rust UI (windows-x86)"
    Compile-Native "x86"
    $apiOut = Join-Path $dist "api-win-x86"
    Publish-Api "win-x86" $apiOut
    $apiDll = Join-Path $apiOut "cubecheck_api.dll"
    $nativeDll = Join-Path $outNative "x86\cubecheck_native.dll"
    if (-not (Test-Path $apiDll)) { throw "нет cubecheck_api.dll (win-x86)" }
    if (-not (Test-Path $nativeDll)) { throw "нет cubecheck_native.dll (x86)" }

    Ensure-RustupTarget "i686-pc-windows-msvc"
    Use-MsvcEnv "x86" | Out-Null
    $triple = "i686-pc-windows-msvc"
    $rel = Join-Path $root "target\$triple\release"
    $relAssets = Join-Path $rel "assets"
    New-Item -ItemType Directory -Force -Path $relAssets | Out-Null
    Copy-Item $apiDll (Join-Path $relAssets "cubecheck_api.dll") -Force
    Copy-Item $nativeDll (Join-Path $relAssets "cubecheck_native.dll") -Force
    $env:CUBECHECK_API_DLL = $apiDll
    $env:CUBECHECK_NATIVE_DLL = $nativeDll

    Push-Location $root
    try {
        cargo build -p cubecheck --release --bin cubecheck --features gui --target $triple
        if ($LASTEXITCODE -ne 0) { throw "Сборка cubecheck i686 не удалась" }
    } finally {
        Pop-Location
    }

    $exe = Join-Path $rel "cubecheck.exe"
    if (-not (Test-Path $exe)) { throw "нет $exe" }
    if (Test-Path $OutDir) { Remove-Item $OutDir -Recurse -Force }
    $outAssets = Join-Path $OutDir "assets"
    New-Item -ItemType Directory -Force -Path $outAssets | Out-Null
    Copy-Item $exe (Join-Path $OutDir "cubecheck.exe") -Force
    Copy-Item $apiDll (Join-Path $outAssets "cubecheck_api.dll") -Force
    Copy-Item $nativeDll (Join-Path $outAssets "cubecheck_native.dll") -Force
    foreach ($name in @("tools.json", "cubecheck.ico", "settings.default.json", "Everything.ini", "UnInstall.ico", "UnInstall.cmd")) {
        $from = Join-Path $root "assets\$name"
        if (Test-Path $from) { Copy-Item $from (Join-Path $outAssets $name) -Force }
    }
    Set-Content -Path (Join-Path $OutDir ".portable") -Value ""
    Sanitize-WindowsPayload $OutDir
    Write-UninstallHelpers $OutDir
}

function Build-RustLinuxX86([string]$OutDir) {
    Info "Rust egui cubecheck (i686-unknown-linux-gnu)"
    Ensure-RustupTarget "i686-unknown-linux-gnu"
    Use-AsciiTemp
    $triple = "i686-unknown-linux-gnu"
    $elf = Join-Path $root "target\$triple\release\cubecheck"
    Set-ZigCrossEnv $triple "x86-linux-gnu.2.17"
    Push-Location $root
    try {
        cargo build -p cubecheck --release --bin cubecheck --features gui --target $triple
        if ($LASTEXITCODE -ne 0) { throw "кросс i686 Linux ELF не удался" }
    } finally {
        Pop-Location
    }
    if (-not (Test-Elf $elf)) { throw "нет ELF i686: $elf" }
    Stage-LinuxX64 $elf $OutDir
}

function Try-BuildMacOs([string]$Triple, [string]$ZigTarget, [string]$OutDir) {
    Info "Пробую Mach-O $Triple"
    Ensure-RustupTarget $Triple
    Use-AsciiTemp
    Set-ZigCrossEnv $Triple $ZigTarget
    $bin = Join-Path $root "target\$Triple\release\cubecheck"
    Push-Location $root
    try {
        cargo build -p cubecheck --release --bin cubecheck --features gui --target $Triple
        if ($LASTEXITCODE -ne 0) { throw "cargo $Triple exit=$LASTEXITCODE" }
    } finally {
        Pop-Location
    }
    if (-not (Test-MachO $bin)) { throw "нет Mach-O: $bin" }
    if (Test-Path $OutDir) { Remove-Item $OutDir -Recurse -Force }
    New-Item -ItemType Directory -Force -Path (Join-Path $OutDir "assets") | Out-Null
    Copy-Item $bin (Join-Path $OutDir "cubecheck") -Force
    foreach ($name in @("tools.json", "cubecheck.ico", "settings.default.json")) {
        $from = Join-Path $root "assets\$name"
        if (Test-Path $from) { Copy-Item $from (Join-Path $OutDir "assets\$name") -Force }
    }
    Set-Content -Path (Join-Path $OutDir ".portable") -Value ""
    if ((Get-DllFiles $OutDir).Count -gt 0) { throw "macOS payload не должен содержать .dll" }
}

function New-UnixUniversalTree([string]$OutDir, [hashtable]$Payloads) {
    if (Test-Path $OutDir) { Remove-Item $OutDir -Recurse -Force }
    New-Item -ItemType Directory -Force -Path (Join-Path $OutDir "payload") | Out-Null
    $sh = Get-Content -LiteralPath (Join-Path $root "scripts\cubecheck.sh") -Raw -Encoding UTF8
    Write-UnixText (Join-Path $OutDir "cubecheck.sh") $sh
    $have = 0
    foreach ($key in $Payloads.Keys) {
        $src = $Payloads[$key]
        if (-not $src -or -not (Test-Path $src)) { continue }
        $bin = Join-Path $src "cubecheck"
        $ok = $false
        if ($key -like "linux-*") { $ok = Test-UnixRustPayload $src }
        elseif ($key -like "osx-*") { $ok = Test-MacPayload $src }
        if (-not $ok) {
            if (Test-Path $bin) { Warn "${key}: пропускаю (не тот формат бинарника)" }
            continue
        }
        Copy-Tree $src (Join-Path $OutDir "payload\$key")
        $have++
    }
    return $have
}

function Write-MacReadme([string]$Path, [bool]$HaveBinary) {
    if ($HaveBinary) {
        Write-UnixText $Path @"
CubeCheck 1.1 beta — macOS
Авторы: AuraStudio, AnProject

Запуск установщика: chmod +x install-macos.sh && ./install-macos.sh
(или самораспаковывающийся .run)

Лаунчер выбирает payload/osx-arm64 или payload/osx-x64.
Windows PE / Avalonia DLL в этом пакете нет.
"@
    } else {
        Write-UnixText $Path @"
CubeCheck 1.1 beta — macOS
Авторы: AuraStudio, AnProject

CubeCheck.app и Mach-O cubecheck в этом пакете НЕТ.
Сборка сделана на Windows: нет Apple SDK / osxcross. Windows .exe сюда не клали.

На Mac соберите GUI:
  rustup target add aarch64-apple-darwin x86_64-apple-darwin
  cargo build -p cubecheck --release --bin cubecheck --features gui --target aarch64-apple-darwin
  cargo build -p cubecheck --release --bin cubecheck --features gui --target x86_64-apple-darwin
  скопируйте бинарник в payload/osx-arm64/cubecheck и/или payload/osx-x64/cubecheck
  chmod +x install-macos.sh payload/osx-*/cubecheck
  ./install-macos.sh

Встроенные утилиты macOS (Spotlight, Activity Monitor, fs_usage, Login Items)
не требуют загрузки. Portable fd/rg/fzf/lf/procs/btm — в assets/bin (офлайн-пакет).
"@
    }
}

function Copy-UnixScript([string]$From, [string]$To) {
    Write-UnixText $To (Get-Content -LiteralPath $From -Raw -Encoding UTF8)
}

function Add-UnixInstallerFiles([string]$TreeDir, [string]$Kind) {
    Copy-Item -LiteralPath (Join-Path $root "LICENSE.md") -Destination (Join-Path $TreeDir "LICENSE.md") -Force
    if ($Kind -eq "linux") {
        Copy-UnixScript (Join-Path $root "scripts\install-linux.sh") (Join-Path $TreeDir "install-linux.sh")
    } else {
        Copy-UnixScript (Join-Path $root "scripts\install-macos.sh") (Join-Path $TreeDir "install-macos.sh")
    }
}

function New-UnixSetupRun {
    param(
        [Parameter(Mandatory = $true)][string]$TreeDir,
        [Parameter(Mandatory = $true)][string]$OutFile,
        [Parameter(Mandatory = $true)][string]$Kind,
        [bool]$Offline = $false
    )
    if (-not (Test-Path -LiteralPath $TreeDir)) { throw "нет дерева для .run: $TreeDir" }
    Add-UnixInstallerFiles $TreeDir $Kind
    if ($Kind -eq "linux") {
        $elfDir = Join-Path $TreeDir "payload\linux-x64"
        if (Test-Path -LiteralPath $elfDir) { Assert-LinuxPayload $elfDir }
        $dlls = Get-DllFiles $TreeDir
        if ($dlls.Count -gt 0) {
            throw "linux .run не должен содержать .dll: $(($dlls | Select-Object -First 8).Name -join ', ')"
        }
    }

    $tar = Join-Path $dist ("run-payload-{0}-{1}.tar.gz" -f $Kind, $(if ($Offline) { "offline" } else { "online" }))
    if (Test-Path -LiteralPath $tar) { Remove-Item -LiteralPath $tar -Force }
    tar -czf $tar -C $TreeDir .
    if ($LASTEXITCODE -ne 0 -or -not (Test-Path -LiteralPath $tar)) { throw "tar для $OutFile не удался" }
    if ((Get-Item -LiteralPath $tar).Length -lt 256) { throw "tar слишком маленький: $tar" }

    $offlineFlag = if ($Offline) { "1" } else { "0" }
    $scriptBody = @'
#!/bin/sh
set -eu
# CubeCheck 1.1 beta setup.run — chmod +x this-file && ./this-file
ARCHIVE_OFFSET={OFFSET}
OFFLINE_FLAG={OFFLINE}
PRODUCT=CubeCheck

THIS=$0
case "$THIS" in
  /*) ;;
  *) THIS=$(pwd)/$THIS ;;
esac

die() { echo "CubeCheck: $*" >&2; exit 1; }

EXTRACT=$(mktemp -d "${TMPDIR:-/tmp}/cubecheck-setup.XXXXXX")
cleanup() { rm -rf "$EXTRACT" 2>/dev/null || true; }
trap cleanup EXIT INT TERM

echo "CubeCheck: распаковка установщика…"
tail -c +$((ARCHIVE_OFFSET + 1)) "$THIS" | tar -xzf - -C "$EXTRACT" || die "не удалось распаковать архив установщика"

export CUBECHECK_SETUP_RUN="$THIS"
if [ "$OFFLINE_FLAG" = "1" ]; then
  export CUBECHECK_OFFLINE=1
  : > "$EXTRACT/.offline"
  mkdir -p "$EXTRACT/assets"
  : > "$EXTRACT/assets/.offline"
fi

INSTALLER=""
os=$(uname -s 2>/dev/null || echo unknown)
if [ "$os" = "Darwin" ]; then
  if [ -f "$EXTRACT/install-macos.sh" ]; then INSTALLER="$EXTRACT/install-macos.sh"; fi
elif [ "$os" = "Linux" ]; then
  if [ -f "$EXTRACT/install-linux.sh" ]; then INSTALLER="$EXTRACT/install-linux.sh"; fi
fi
[ -n "$INSTALLER" ] || die "В пакете нет install-linux.sh / install-macos.sh для $os"
chmod +x "$INSTALLER" 2>/dev/null || true
chmod +x "$EXTRACT/payload/"*/cubecheck "$EXTRACT/cubecheck.sh" "$EXTRACT/payload/"*/assets/bin/* "$EXTRACT/extras/bin/"* 2>/dev/null || true

cd "$EXTRACT"
status=0
/bin/sh "$INSTALLER" || status=$?
exit $status
'@
    $scriptBody = $scriptBody.Replace("{OFFLINE}", $offlineFlag)
    $offset = 2048
    $headerBytes = $null
    for ($i = 0; $i -lt 8; $i++) {
        $text = $scriptBody.Replace("{OFFSET}", "$offset")
        $unix = $text -replace "`r`n", "`n" -replace "`r", "`n"
        if (-not $unix.EndsWith("`n")) { $unix += "`n" }
        $headerBytes = [System.Text.Encoding]::UTF8.GetBytes($unix)
        if ($headerBytes.Length -eq $offset) { break }
        $offset = $headerBytes.Length
    }
    if ($null -eq $headerBytes -or $headerBytes.Length -ne $offset) {
        throw "не удалось зафиксировать ARCHIVE_OFFSET для $OutFile"
    }

    $outDir = Split-Path -Parent $OutFile
    New-Item -ItemType Directory -Force -Path $outDir | Out-Null
    if (Test-Path -LiteralPath $OutFile) { Remove-Item -LiteralPath $OutFile -Force }
    $out = [System.IO.File]::Open($OutFile, [System.IO.FileMode]::Create, [System.IO.FileAccess]::Write)
    try {
        $out.Write($headerBytes, 0, $headerBytes.Length)
        $tarStream = [System.IO.File]::OpenRead($tar)
        try { $tarStream.CopyTo($out) } finally { $tarStream.Dispose() }
    } finally { $out.Dispose() }

    $magic = [System.IO.File]::ReadAllBytes($OutFile)[0..1]
    if ($magic[0] -ne 0x23 -or $magic[1] -ne 0x21) { throw ".run не начинается с #!: $OutFile" }
    Info ".run: $OutFile ($((Get-Item -LiteralPath $OutFile).Length) байт, offline=$Offline)"
}

function New-WindowsSetupZip([string]$TreeDir, [string]$ZipPath) {
    if (Test-Path $ZipPath) { Remove-Item $ZipPath -Force }
    Compress-Archive -Path (Join-Path $TreeDir "*") -DestinationPath $ZipPath -Force
    if ((Get-Item $ZipPath).Length -lt 1000000) {
        throw "payload zip слишком маленький: $ZipPath"
    }
}

function Build-FatWindowsSetup([string]$PayloadZip, [string]$OutExe) {
    Info "Rust cubecheck-setup (fat, встраивает universal zip $((Get-Item $PayloadZip).Length) байт)"
    $env:CUBECHECK_SETUP_ZIP = $PayloadZip
    Push-Location $root
    try {
        cargo build -p cubecheck --release --bin cubecheck-setup --no-default-features --features setup-embed
        if ($LASTEXITCODE -ne 0) { throw "Сборка cubecheck-setup не удалась" }
    } finally {
        Pop-Location
    }
    $built = Join-Path $root "target\release\cubecheck-setup.exe"
    Copy-Item $built $OutExe -Force
    Copy-Item $built (Join-Path $dist "CubeCheck-Setup.exe") -Force
    Copy-Item $built (Join-Path $root "CubeCheck-Setup.exe") -Force
    $len = (Get-Item $OutExe).Length
    if ($len -lt 2000000) {
        throw "universal-windows-setup.exe слишком маленький ($len байт) — это stub, не fat installer"
    }
    Info "setup.exe $len байт"
}

function Clear-LegacyReleaseNames {
    Get-ChildItem -LiteralPath $buildOut -File -ErrorAction SilentlyContinue | Where-Object {
        $_.Name -like "CubeCheck-$Version-windows-x64*" -or
        $_.Name -like "CubeCheck-$Version-linux-x64*" -or
        $_.Name -like "CubeCheck-$Version-windows-x86*" -or
        $_.Name -eq "CubeCheck-$Version-universal.zip" -or
        $_.Name -eq "CubeCheck-$Version-universal-win.zip" -or
        $_.Name -eq "CubeCheck-$Version-universal-local.zip" -or
        $_.Name -eq "CubeCheck-$Version-linux-universal.zip" -or
        $_.Name -like "*setup.zip" -or
        $_.Name -like "*setup.tar.gz" -or
        $_.Name -eq "CubeCheck-$Version-setup.exe" -or
        $_.Name -eq "payload.zip" -or
        $_.Name -eq "setup.json"
    } | ForEach-Object {
        Info "удаляю leftover $($_.Name)"
        Remove-Item -LiteralPath $_.FullName -Force
    }
    Clear-BuildAvaloniaJunk
}

function Publish-UniversalReleaseSet {
    Info "очищаю build/"
    if (Test-Path -LiteralPath $buildOut) {
        Remove-Item -LiteralPath $buildOut -Recurse -Force
    }
    New-Item -ItemType Directory -Force -Path $buildOut | Out-Null

    $win64 = Join-Path $dist "windows-x64"
    $win86 = Join-Path $dist "windows-x86"
    $linux64 = Join-Path $dist "linux-x64"
    $linux86 = Join-Path $dist "linux-x86"
    $osx64 = Join-Path $dist "osx-x64"
    $osxArm = Join-Path $dist "osx-arm64"

    $script:WinX86Ok = $false
    $script:LinuxX86Ok = $false
    $script:OsxX64Ok = $false
    $script:OsxArmOk = $false
    $script:MacBinaryOk = $false

    if (-not (Test-Path (Join-Path $win64 "cubecheck.exe"))) {
        Build-RustWindows $win64 -SkipLegacyRelease
    } else {
        # API/UI may be stale after Core changes — always rebuild Windows x64 for this set
        Build-RustWindows $win64 -SkipLegacyRelease
    }

    try {
        Build-RustWindowsX86 $win86
        $script:WinX86Ok = Test-Path (Join-Path $win86 "cubecheck.exe")
    } catch {
        Warn "windows-x86 не собран: $($_.Exception.Message)"
        $script:WinX86Ok = $false
    }

    $linuxOk = $false
    try {
        Build-RustLinux $linux64
        $linuxOk = Test-UnixRustPayload $linux64
    } catch {
        Warn $_.Exception.Message
        if (Test-Path $linux64) { Remove-DllDump $linux64 "linux-x64" }
    }
    if (-not $linuxOk) { throw "linux-x64 ELF обязателен для universal-linux-setup" }

    try {
        Build-RustLinuxX86 $linux86
        $script:LinuxX86Ok = Test-UnixRustPayload $linux86
    } catch {
        Warn "linux-x86 не собран: $($_.Exception.Message)"
        $script:LinuxX86Ok = $false
    }

    try {
        Try-BuildMacOs "x86_64-apple-darwin" "x86_64-macos" $osx64
        $script:OsxX64Ok = Test-MacPayload $osx64
    } catch {
        Warn "osx-x64 Mach-O: $($_.Exception.Message)"
        if (Test-Path $osx64) { Remove-DllDump $osx64 "osx-x64" }
    }
    try {
        Try-BuildMacOs "aarch64-apple-darwin" "aarch64-macos" $osxArm
        $script:OsxArmOk = Test-MacPayload $osxArm
    } catch {
        Warn "osx-arm64 Mach-O: $($_.Exception.Message)"
        if (Test-Path $osxArm) { Remove-DllDump $osxArm "osx-arm64" }
    }
    $script:MacBinaryOk = $script:OsxX64Ok -or $script:OsxArmOk

    # --- Linux trees ---
    $linuxPayloads = @{ "linux-x64" = $linux64 }
    if ($script:LinuxX86Ok) { $linuxPayloads["linux-x86"] = $linux86 }

    $linuxOnline = Join-Path $dist "universal-linux"
    [void](New-UnixUniversalTree $linuxOnline $linuxPayloads)
    if (-not (Test-UnixRustPayload (Join-Path $linuxOnline "payload\linux-x64"))) {
        throw "universal-linux: нет payload/linux-x64 ELF"
    }
    Assert-LinuxPayload (Join-Path $linuxOnline "payload\linux-x64")

    $linuxOffline = Join-Path $dist "universal-linux-offline"
    Copy-Tree $linuxOnline $linuxOffline
    Mark-Offline $linuxOffline
    $extraCount = Add-LinuxExtras $linuxOffline "linux-x64"
    if ($script:LinuxX86Ok) { [void](Add-LinuxExtras $linuxOffline "linux-x86") }
    $portable = @(Get-ChildItem -LiteralPath (Join-Path $linuxOffline "extras\bin") -File -ErrorAction SilentlyContinue |
        Where-Object { $_.Name -in @("fd", "rg", "fzf", "btop", "btm", "procs", "lf", "busybox", "missioncenter") -and $_.Length -gt 8192 })
    if ($portable.Count -lt 1) {
        throw "linux offline: не скачался ни один portable binary (fd/rg/btop/…). Сеть или URL."
    }
    $needSearch = $portable | Where-Object { $_.Name -in @("fd", "rg", "fzf") }
    $needProc = $portable | Where-Object { $_.Name -in @("btop", "btm", "procs", "missioncenter") }
    if (-not $needSearch -or -not $needProc) {
        throw "linux offline: нужны search (fd/rg/fzf) и processes (btop/btm/procs/missioncenter) в extras/bin"
    }
    Info "linux offline portable: $($portable.Name -join ', ') (fetch=$extraCount)"

    $linuxRun = Join-Path $buildOut "CubeCheck-$Version-universal-linux-setup.run"
    $linuxOffRun = Join-Path $buildOut "CubeCheck-$Version-universal-linux-offline-setup.run"
    New-UnixSetupRun -TreeDir $linuxOnline -OutFile $linuxRun -Kind "linux" -Offline:$false
    New-UnixSetupRun -TreeDir $linuxOffline -OutFile $linuxOffRun -Kind "linux" -Offline:$true
    $dllInRunTar = @(tar -tzf (Join-Path $dist "run-payload-linux-online.tar.gz") | Where-Object { $_ -match '\.dll$' })
    if ($dllInRunTar.Count -gt 0) { throw "linux online tar содержит .dll: $($dllInRunTar -join ', ')" }

    # --- Windows trees ---
    $winOnline = Join-Path $dist "universal-windows"
    $x86Src = if ($script:WinX86Ok) { $win86 } else { Join-Path $dist "windows-x86-missing" }
    New-WinUniversal $winOnline $win64 $x86Src
    $icon = Join-Path $win64 "assets\cubecheck.ico"
    if (Test-Path $icon) {
        New-Item -ItemType Directory -Force -Path (Join-Path $winOnline "assets") | Out-Null
        Copy-Item $icon (Join-Path $winOnline "assets\cubecheck.ico") -Force
        Copy-Item (Join-Path $root "assets\tools.json") (Join-Path $winOnline "assets\tools.json") -Force
    }

    $winOffline = Join-Path $dist "universal-windows-offline"
    Copy-Tree $winOnline $winOffline
    Mark-Offline $winOffline
    $v64 = Join-Path $winOffline "payload\windows-x64\assets"
    if (-not (Fetch-WindowsVendor $v64 "x64")) {
        Warn "не все Windows vendor-файлы скачались — проверяю список"
    }
    foreach ($need in @("Everything.exe", "Shellbag.exe", "Procmon64.exe", "Autoruns64.exe", "procexp64.exe", "SystemInformer\SystemInformer.exe")) {
        if (-not (Test-Path (Join-Path $v64 $need))) {
            throw "offline Windows: нет $need — загрузка с официальных URL не удалась"
        }
    }
    if ($script:WinX86Ok) {
        $v86 = Join-Path $winOffline "payload\windows-x86\assets"
        [void](Fetch-WindowsVendor $v86 "x86")
    }

    $setupZip = Join-Path $dist "universal-windows-payload.zip"
    New-WindowsSetupZip $winOnline $setupZip
    $offlineZip = Join-Path $dist "universal-windows-offline-payload.zip"
    New-WindowsSetupZip $winOffline $offlineZip
    $setupExe = Join-Path $buildOut "CubeCheck-$Version-universal-windows-setup.exe"
    $setupOffExe = Join-Path $buildOut "CubeCheck-$Version-universal-windows-offline-setup.exe"
    Publish-WizardInstaller -OutExe $setupExe
    Publish-WizardInstaller -OutExe $setupOffExe -OfflinePayloadZip $offlineZip

    # --- macOS trees ---
    $macPayloads = @{}
    if ($script:OsxX64Ok) { $macPayloads["osx-x64"] = $osx64 }
    if ($script:OsxArmOk) { $macPayloads["osx-arm64"] = $osxArm }

    $macOnline = Join-Path $dist "universal-macos"
    $macHave = New-UnixUniversalTree $macOnline $macPayloads
    foreach ($key in @("osx-x64", "osx-arm64")) {
        $p = Join-Path $macOnline "payload\$key\assets"
        New-Item -ItemType Directory -Force -Path $p | Out-Null
        foreach ($name in @("tools.json", "cubecheck.ico", "settings.default.json")) {
            $from = Join-Path $root "assets\$name"
            if (Test-Path $from) { Copy-Item $from (Join-Path $p $name) -Force }
        }
    }
    Write-MacReadme (Join-Path $macOnline "README-macos.txt") ($macHave -gt 0)

    $macOffline = Join-Path $dist "universal-macos-offline"
    Copy-Tree $macOnline $macOffline
    Mark-Offline $macOffline
    [void](Add-MacExtras $macOffline @("osx-x64", "osx-arm64"))
    Write-MacReadme (Join-Path $macOffline "README-macos.txt") ($macHave -gt 0)

    $macRun = Join-Path $buildOut "CubeCheck-$Version-universal-macos-setup.run"
    $macOffRun = Join-Path $buildOut "CubeCheck-$Version-universal-macos-offline-setup.run"
    New-UnixSetupRun -TreeDir $macOnline -OutFile $macRun -Kind "macos" -Offline:$false
    New-UnixSetupRun -TreeDir $macOffline -OutFile $macOffRun -Kind "macos" -Offline:$true
    $macReadme = Join-Path $buildOut "CubeCheck-$Version-universal-macos-README.txt"
    Copy-Item -LiteralPath (Join-Path $macOnline "README-macos.txt") -Destination $macReadme -Force

    try { [void](Stage-GithubPayload) } catch { Warn "github-upload: $($_.Exception.Message)" }

    Copy-IfProgramFiles
    foreach ($stale in @(
        (Join-Path $root "CubeCheck-Setup.exe"),
        (Join-Path $dist "CubeCheck-Setup.exe")
    )) {
        if (Test-Path -LiteralPath $stale) {
            Info "удаляю leftover $stale"
            Remove-Item -LiteralPath $stale -Force
        }
    }

    Clear-LegacyReleaseNames

    $keep = @(
        "CubeCheck-$Version-universal-windows-setup.exe",
        "CubeCheck-$Version-universal-windows-offline-setup.exe",
        "CubeCheck-$Version-universal-linux-setup.run",
        "CubeCheck-$Version-universal-linux-offline-setup.run",
        "CubeCheck-$Version-universal-macos-setup.run",
        "CubeCheck-$Version-universal-macos-offline-setup.run",
        "CubeCheck-$Version-universal-macos-README.txt",
        "CubeCheck-$Version-github-payload.zip"
    )
    Get-ChildItem -LiteralPath $buildOut -File -ErrorAction SilentlyContinue | Where-Object {
        $keep -notcontains $_.Name
    } | ForEach-Object {
        Info "удаляю лишний $($_.Name)"
        Remove-Item -LiteralPath $_.FullName -Force
    }

    Write-Host ""
    Write-Host "Universal release set ($Version):"
    foreach ($name in $keep) {
        $p = Join-Path $buildOut $name
        if (Test-Path $p) {
            $i = Get-Item $p
            Write-Host ("  {0,-62} {1,12:N0}  {2}" -f $i.Name, $i.Length, $i.LastWriteTime)
        } else {
            Write-Host "  MISSING $name"
        }
    }
    $badSetups = @(Get-ChildItem -LiteralPath $buildOut -File -ErrorAction SilentlyContinue |
        Where-Object { $_.Name -match '(?i)setup\.(zip|tar\.gz)$' })
    if ($badSetups.Count -gt 0) {
        throw "в build/ нельзя оставлять zip/tar с именем setup: $($badSetups.Name -join ', ')"
    }
    Write-Host ""
    Write-Host "Arches:"
    Write-Host "  windows-x64: yes"
    Write-Host "  windows-x86: $(if ($script:WinX86Ok) { 'yes' } else { 'NO' })"
    Write-Host "  linux-x64: yes"
    Write-Host "  linux-x86: $(if ($script:LinuxX86Ok) { 'yes' } else { 'NO' })"
    Write-Host "  osx-x64 Mach-O: $(if ($script:OsxX64Ok) { 'yes' } else { 'NO (README stub)' })"
    Write-Host "  osx-arm64 Mach-O: $(if ($script:OsxArmOk) { 'yes' } else { 'NO (README stub)' })"
}
