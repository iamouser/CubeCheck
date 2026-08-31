# CubeCheck multi-platform build
# Default (all|release|universal): real installers in build\ (no zip named setup):
#   CubeCheck-<ver>-universal-windows-setup.exe            thin WPF, HTTPS GitHub payload
#   CubeCheck-<ver>-universal-windows-offline-setup.exe    same wizard, embedded zip, no HTTP
#   CubeCheck-<ver>-universal-linux-setup.run              install.sh + ELF, may download
#   CubeCheck-<ver>-universal-linux-offline-setup.run      ELF + assets/bin, no download
#   CubeCheck-<ver>-universal-macos-setup.run              install script; README if no Mach-O
#   CubeCheck-<ver>-universal-macos-offline-setup.run      Darwin portables, no download
#   CubeCheck-<ver>-universal-macos-README.txt
#   CubeCheck-<ver>-github-payload.zip
# Other targets: github | installer | wizard | windows-x64 | windows-x86 | linux-x64 | ...
# Linux/macOS UI = Rust egui ELF/Mach-O. Avalonia publish is not a release product.
# CubeCheck.Installer = thin WPF FDD wizard (online downloads GitHub payload; offline embeds zip).
param(
    [Parameter(Position = 0)]
    [string]$Target = "all"
)

$ErrorActionPreference = "Stop"
$root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
$dotnet = Join-Path $root "src"
$native = Join-Path $dotnet "native"
$outNative = Join-Path $native "bin"
$dist = Join-Path $root "dist"
$buildOut = Join-Path $root "build"
$Version = "1.1.0-beta"
$cfg = "Release"

New-Item -ItemType Directory -Force -Path $outNative, $dist, $buildOut | Out-Null

function Info($m) { Write-Host "==> $m" }
function Warn($m) { Write-Host "WARN: $m" }

function Zip-Dir($src, $zip) {
    if (Test-Path $zip) { Remove-Item $zip -Force }
    Compress-Archive -Path (Join-Path $src "*") -DestinationPath $zip -Force
}

function Copy-Tree($src, $dst) {
    if (Test-Path $dst) { Remove-Item $dst -Recurse -Force }
    New-Item -ItemType Directory -Force -Path $dst | Out-Null
    Copy-Item -Path (Join-Path $src "*") -Destination $dst -Recurse -Force
}

function Use-MsvcEnv([string]$Arch) {
    $msvcRoot = "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC"
    if (-not (Test-Path $msvcRoot)) {
        $msvcRoot = "C:\Program Files\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC"
    }
    $msvcDirs = @(Get-ChildItem $msvcRoot -Directory -ErrorAction SilentlyContinue |
        Where-Object {
            (Test-Path (Join-Path $_.FullName "include\excpt.h")) -and
            (Test-Path (Join-Path $_.FullName "bin\Hostx64\x64\cl.exe"))
        })
    # NativeAOT / ILC on this machine is wired to MSVC 14.39; prefer it over newer toolsets.
    $msvc = $msvcDirs | Where-Object { $_.Name -like "14.39*" } | Sort-Object Name -Descending | Select-Object -First 1
    if (-not $msvc) {
        $msvc = $msvcDirs | Sort-Object Name -Descending | Select-Object -First 1
    }
    if (-not $msvc) { throw "MSVC не найден (нужен Visual Studio Build Tools с C++: cl.exe + excpt.h)" }
    $msvc = $msvc.FullName
    $sdkRoot = "C:\Program Files (x86)\Windows Kits\10"
    $sdkVer = Get-ChildItem (Join-Path $sdkRoot "Include") -Directory |
        Where-Object { Test-Path (Join-Path $_.FullName "um\windows.h") } |
        Sort-Object Name -Descending |
        Select-Object -First 1
    if (-not $sdkVer) { throw "Windows SDK не найден" }
    $ver = $sdkVer.Name
    $hostCl = if ($Arch -eq "x86") { "x86" } else { "x64" }
    $cl = Join-Path $msvc "bin\Hostx64\$hostCl\cl.exe"
    if (-not (Test-Path $cl)) { $cl = Join-Path $msvc "bin\Hostx64\x64\cl.exe" }

    $env:INCLUDE = "$msvc\include;$sdkRoot\Include\$ver\ucrt;$sdkRoot\Include\$ver\um;$sdkRoot\Include\$ver\shared"
    $env:LIB = "$msvc\lib\$Arch;$sdkRoot\Lib\$ver\ucrt\$Arch;$sdkRoot\Lib\$ver\um\$Arch"
    $env:PATH = "$(Join-Path $msvc "bin\Hostx64\$hostCl");$(Join-Path $msvc "bin\Hostx64\x64");" + $env:PATH
    return $cl
}

function Compile-Native([string]$Arch) {
    $cl = Use-MsvcEnv $Arch

    $outDir = Join-Path $outNative $Arch
    New-Item -ItemType Directory -Force -Path $outDir | Out-Null
    $machine = if ($Arch -eq "x86") { "/MACHINE:X86" } else { "/MACHINE:X64" }

    Info "C++ cubecheck_native.dll ($Arch, Win7+)"
    Push-Location $outDir
    try {
        & $cl /nologo /LD /O2 /EHsc /std:c++17 /utf-8 /MT `
            /DWINVER=0x0601 /D_WIN32_WINNT=0x0601 /DCUBCHECK_NATIVE_EXPORTS `
            /I (Join-Path $native "include") `
            (Join-Path $native "src\dllmain.cpp") `
            (Join-Path $native "src\win32.cpp") `
            (Join-Path $native "src\scan.cpp") `
            /Fe:cubecheck_native.dll `
            /link /DLL $machine /SUBSYSTEM:WINDOWS,6.01 `
            wintrust.lib crypt32.lib ole32.lib shell32.lib advapi32.lib user32.lib kernel32.lib
        if ($LASTEXITCODE -ne 0) { throw "Сборка native DLL $Arch не удалась" }

        if ($Arch -eq "x64") {
            Info "C++ cubecheck-launcher.exe (Win7+)"
            & $cl /nologo /O2 /EHsc /std:c++17 /utf-8 /MT `
                /DWINVER=0x0601 /D_WIN32_WINNT=0x0601 `
                (Join-Path $native "src\launcher.cpp") `
                /Fe:cubecheck-launcher.exe `
                /link /SUBSYSTEM:WINDOWS,6.01 user32.lib shell32.lib
            if ($LASTEXITCODE -ne 0) { throw "Сборка launcher не удалась" }
            Copy-Item "cubecheck-launcher.exe" (Join-Path $outNative "cubecheck-launcher.exe") -Force
            Copy-Item "cubecheck_native.dll" (Join-Path $outNative "cubecheck_native.dll") -Force
        }
    }
    finally {
        Pop-Location
    }
}

function Publish-WinNet8([string]$Rid, [string]$OutDir) {
    Info ".NET 8 WPF $Rid (Windows 10/11)"
    New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
    dotnet publish (Join-Path $dotnet "CubeCheck.App\CubeCheck.App.csproj") `
        -c $cfg -f net8.0-windows -r $Rid --self-contained true -o $OutDir
    if ($LASTEXITCODE -ne 0) { throw "publish $Rid не удался" }
    $arch = if ($Rid -eq "win-x86") { "x86" } else { "x64" }
    Copy-Item (Join-Path $outNative "$arch\cubecheck_native.dll") (Join-Path $OutDir "cubecheck_native.dll") -Force
    Set-Content -Path (Join-Path $OutDir ".portable") -Value ""
}

function Publish-WinNet48([string]$Rid, [string]$OutDir) {
    Info ".NET Framework 4.8 WPF $Rid (Windows 7/8/10/11)"
    New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
    $pt = if ($Rid -eq "win-x86") { "x86" } else { "x64" }
    dotnet publish (Join-Path $dotnet "CubeCheck.App\CubeCheck.App.csproj") `
        -c $cfg -f net48 -r $Rid -o $OutDir /p:PlatformTarget=$pt
    if ($LASTEXITCODE -ne 0) { throw "publish net48 $Rid не удался" }
    Copy-Item (Join-Path $outNative "$pt\cubecheck_native.dll") (Join-Path $OutDir "cubecheck_native.dll") -Force
    Set-Content -Path (Join-Path $OutDir ".portable") -Value ""
}

function Publish-Desktop([string]$Rid, [string]$OutDir) {
    throw "Publish-Desktop ($Rid) отключён: Avalonia не является Linux/macOS продуктом. UI = Rust egui."
}

function Publish-Setup([string]$OutExe) {
    Info "Rust cubecheck-setup (встраивает cubecheck.exe)"
    Push-Location $root
    try {
        cargo build -p cubecheck --release --bin cubecheck-setup
        if ($LASTEXITCODE -ne 0) { throw "Сборка cubecheck-setup не удалась" }
    } finally {
        Pop-Location
    }
    Copy-Item (Join-Path $root "target\release\cubecheck-setup.exe") $OutExe -Force
}

function Remove-LegacyInstallRoot([string]$Dir) {
    if (-not (Test-Path -LiteralPath $Dir)) { return }
    foreach ($name in @("cubecheck_api.dll", "cubecheck_native.dll", "UnInstall.ico", "UnInstall.cmd")) {
        $legacy = Join-Path $Dir $name
        if (Test-Path -LiteralPath $legacy) {
            try {
                Remove-Item -LiteralPath $legacy -Force
            } catch {
                Warn "не удалось убрать $name из корня ${Dir}: $($_.Exception.Message)"
            }
        }
    }
}

function Sanitize-WindowsPayload([string]$Dir) {
    $keepRoot = @("cubecheck.exe", ".portable", "UnInstall.url")
    $keepAssets = @(
        "tools.json", "cubecheck.ico", "settings.default.json", "Everything.ini",
        "UnInstall.ico", "UnInstall.cmd", "cubecheck_api.dll", "cubecheck_native.dll"
    )
    if (-not (Test-Path -LiteralPath $Dir)) { return }

    Get-ChildItem -LiteralPath $Dir -Force | ForEach-Object {
        if ($_.PSIsContainer) {
            if ($_.Name -ne "assets") {
                Remove-Item -LiteralPath $_.FullName -Recurse -Force
            }
        } elseif ($keepRoot -notcontains $_.Name) {
            Remove-Item -LiteralPath $_.FullName -Force
        }
    }

    $assets = Join-Path $Dir "assets"
    if (Test-Path -LiteralPath $assets) {
        Get-ChildItem -LiteralPath $assets -Force | ForEach-Object {
            if ($_.PSIsContainer -or ($keepAssets -notcontains $_.Name)) {
                Remove-Item -LiteralPath $_.FullName -Recurse -Force
            }
        }
    }

    Get-ChildItem -LiteralPath $Dir -Recurse -Force -File -ErrorAction SilentlyContinue |
        Where-Object { $_.Extension -match '\.(pdb|xml|exp|lib|obj|ilk)$' } |
        ForEach-Object { Remove-Item -LiteralPath $_.FullName -Force }
}

function Write-UninstallHelpers([string]$Dir) {
    $exe = Join-Path $Dir "cubecheck.exe"
    if (-not (Test-Path -LiteralPath $exe)) { return }
    $assets = Join-Path $Dir "assets"
    New-Item -ItemType Directory -Force -Path $assets | Out-Null
    $srcIco = Join-Path $root "assets\UnInstall.ico"
    $ico = Join-Path $assets "UnInstall.ico"
    if (Test-Path -LiteralPath $srcIco) {
        try {
            Copy-Item -LiteralPath $srcIco -Destination $ico -Force
        } catch {
            Warn "не удалось записать UnInstall.ico: $($_.Exception.Message)"
        }
    }
    $srcCmd = Join-Path $root "assets\UnInstall.cmd"
    $cmd = Join-Path $assets "UnInstall.cmd"
    $cmdText = "@echo off`r`ncd /d `"%~dp0..`"`r`nstart `"`" `"%~dp0..\cubecheck.exe`" -uninstall`r`n"
    if (Test-Path -LiteralPath $srcCmd) {
        $cmdText = [System.IO.File]::ReadAllText($srcCmd)
        if ($cmdText -notmatch '(?i)cubecheck\.exe') {
            $cmdText = "@echo off`r`ncd /d `"%~dp0..`"`r`nstart `"`" `"%~dp0..\cubecheck.exe`" -uninstall`r`n"
        }
    }
    [System.IO.File]::WriteAllText($cmd, $cmdText, (New-Object System.Text.UTF8Encoding $false))
    $uri = ([Uri](Get-Item -LiteralPath $cmd).FullName).AbsoluteUri
    $urlBody = "[InternetShortcut]`r`nURL=$uri`r`n"
    if (Test-Path -LiteralPath $ico) {
        $urlBody += "IconFile=$ico`r`nIconIndex=0`r`n"
    }
    [System.IO.File]::WriteAllText((Join-Path $Dir "UnInstall.url"), $urlBody, (New-Object System.Text.UTF8Encoding $false))
    Remove-LegacyInstallRoot $Dir
}

function Copy-IfProgramFiles {
    $pf = Join-Path ${env:ProgramFiles} "CubeCheck"
    try {
        New-Item -ItemType Directory -Force -Path $pf | Out-Null
    } catch {
        Warn "нет доступа к Program Files\CubeCheck: $($_.Exception.Message)"
        return
    }
    $src = Join-Path $dist "windows-x64"
    $exeFrom = Join-Path $src "cubecheck.exe"
    if (-not (Test-Path -LiteralPath $exeFrom)) { $exeFrom = Join-Path $root "cubecheck.exe" }
    if (Test-Path -LiteralPath $exeFrom) {
        try {
            Copy-Item -LiteralPath $exeFrom -Destination (Join-Path $pf "cubecheck.exe") -Force
        } catch {
            Warn "не удалось скопировать cubecheck.exe в Program Files\CubeCheck: $($_.Exception.Message)"
        }
    }
    $pfAssets = Join-Path $pf "assets"
    $srcAssets = Join-Path $src "assets"
    if (Test-Path -LiteralPath $srcAssets) {
        try {
            New-Item -ItemType Directory -Force -Path $pfAssets | Out-Null
            Copy-Item -Path (Join-Path $srcAssets "*") -Destination $pfAssets -Force
        } catch {
            Warn "не удалось обновить assets в Program Files\CubeCheck: $($_.Exception.Message)"
        }
    }
    Remove-LegacyInstallRoot $pf
    try {
        Write-UninstallHelpers $pf
    } catch {
        Warn "не удалось записать UnInstall.url в Program Files\CubeCheck: $($_.Exception.Message)"
    }
}

function Test-NativeAotDll([string]$Path) {
    if (-not (Test-Path $Path)) { throw "нет $Path после publish" }
    $len = (Get-Item $Path).Length
    if ($len -lt 400000) {
        throw "cubecheck_api.dll слишком маленький ($len байт) — это не NativeAOT"
    }
    $fs = [System.IO.File]::OpenRead($Path)
    try {
        $mz = New-Object byte[] 2
        [void]$fs.Read($mz, 0, 2)
        if ($mz[0] -ne 0x4D -or $mz[1] -ne 0x5A) { throw "cubecheck_api.dll не PE" }
    } finally { $fs.Close() }
}

function Publish-Api([string]$Rid, [string]$OutDir) {
    Info ".NET NativeAOT cubecheck_api.dll ($Rid)"
    $arch = if ($Rid -eq "win-x86") { "x86" } else { "x64" }
    $cl = Use-MsvcEnv $arch
    $linkDir = Split-Path $cl
    if (-not (Test-Path (Join-Path $linkDir "link.exe"))) {
        throw "link.exe нет рядом с cl.exe: $linkDir"
    }
    $whereLink = Get-Command link.exe -ErrorAction SilentlyContinue
    if (-not $whereLink) { throw "link.exe не в PATH после Use-MsvcEnv" }
    Info "MSVC linker: $($whereLink.Source)"
    New-Item -ItemType Directory -Force -Path $OutDir | Out-Null
    dotnet publish (Join-Path $dotnet "CubeCheck.Api\CubeCheck.Api.csproj") `
        -c $cfg -f net8.0 -r $Rid --self-contained true -o $OutDir `
        /p:IlcUseEnvironmentalTools=true
    if ($LASTEXITCODE -ne 0) { throw "publish CubeCheck.Api $Rid не удался" }
    $apiDllOut = Join-Path $OutDir "cubecheck_api.dll"
    Test-NativeAotDll $apiDllOut
    # NativeAOT publish dumps satellites (.pdb/.xml/.exp/.lib, CubeCheck.Core.dll). Keep only the AOT dll.
    Get-ChildItem -LiteralPath $OutDir -Force | Where-Object { $_.Name -ne "cubecheck_api.dll" } |
        ForEach-Object { Remove-Item -LiteralPath $_.FullName -Recurse -Force }
}

function Build-RustWindows([string]$OutDir, [switch]$SkipLegacyRelease) {
    Info "C++ + C# API + Rust UI (windows-x64)"
    if (-not (Test-Path (Join-Path $outNative "x64\cubecheck_native.dll"))) {
        Compile-Native "x64"
    }
    $apiOut = Join-Path $dist "api-win-x64"
    Publish-Api "win-x64" $apiOut
    $apiDll = Join-Path $apiOut "cubecheck_api.dll"
    $nativeDll = Join-Path $outNative "x64\cubecheck_native.dll"
    if (-not (Test-Path $apiDll)) { throw "нет cubecheck_api.dll после publish" }
    if (-not (Test-Path $nativeDll)) { throw "нет cubecheck_native.dll" }

    $rel = Join-Path $root "target\release"
    $relAssets = Join-Path $rel "assets"
    New-Item -ItemType Directory -Force -Path $relAssets | Out-Null
    Copy-Item $apiDll (Join-Path $relAssets "cubecheck_api.dll") -Force
    Copy-Item $nativeDll (Join-Path $relAssets "cubecheck_native.dll") -Force

    $env:CUBECHECK_API_DLL = $apiDll
    $env:CUBECHECK_NATIVE_DLL = $nativeDll

    Push-Location $root
    try {
        cargo build -p cubecheck --release --bin cubecheck --features gui
        if ($LASTEXITCODE -ne 0) { throw "Сборка cubecheck не удалась" }
        if (-not $SkipLegacyRelease) {
            $env:CUBECHECK_SETUP_PAYLOAD = Join-Path $rel "cubecheck.exe"
            cargo build -p cubecheck --release --bin cubecheck-setup --no-default-features --features setup-embed
            if ($LASTEXITCODE -ne 0) { throw "Сборка cubecheck-setup не удалась" }
        }
    } finally {
        Pop-Location
    }

    if (Test-Path $OutDir) { Remove-Item $OutDir -Recurse -Force }
    $outAssets = Join-Path $OutDir "assets"
    New-Item -ItemType Directory -Force -Path $outAssets | Out-Null
    Copy-Item (Join-Path $rel "cubecheck.exe") (Join-Path $OutDir "cubecheck.exe") -Force
    Copy-Item $apiDll (Join-Path $outAssets "cubecheck_api.dll") -Force
    Copy-Item $nativeDll (Join-Path $outAssets "cubecheck_native.dll") -Force
    foreach ($name in @("tools.json", "cubecheck.ico", "settings.default.json", "Everything.ini", "UnInstall.ico", "UnInstall.cmd")) {
        $from = Join-Path $root "assets\$name"
        if (Test-Path $from) {
            Copy-Item $from (Join-Path $outAssets $name) -Force
        }
    }
    Set-Content -Path (Join-Path $OutDir ".portable") -Value ""
    Sanitize-WindowsPayload $OutDir
    Write-UninstallHelpers $OutDir

    Copy-Item (Join-Path $OutDir "cubecheck.exe") (Join-Path $root "cubecheck.exe") -Force
    $rootAssets = Join-Path $root "assets"
    New-Item -ItemType Directory -Force -Path $rootAssets | Out-Null
    Copy-Item $apiDll (Join-Path $rootAssets "cubecheck_api.dll") -Force
    Copy-Item $nativeDll (Join-Path $rootAssets "cubecheck_native.dll") -Force
    Remove-LegacyInstallRoot $root
    Remove-LegacyInstallRoot $rel
    if (-not $SkipLegacyRelease) {
        $setup = Join-Path $rel "cubecheck-setup.exe"
        if (Test-Path $setup) {
            Copy-Item $setup (Join-Path $dist "CubeCheck-Setup.exe") -Force
            Copy-Item $setup (Join-Path $buildOut "CubeCheck-$Version-windows-x64-setup.exe") -Force
            Copy-Item $setup (Join-Path $root "CubeCheck-Setup.exe") -Force
        }
    }
    Copy-IfProgramFiles
}

function New-ShBundle([string]$Name, [hashtable]$Payloads, [string]$OutDir) {
    if (Test-Path $OutDir) { Remove-Item $OutDir -Recurse -Force }
    $payloadRoot = Join-Path $OutDir "payload"
    New-Item -ItemType Directory -Force -Path $payloadRoot | Out-Null
    Copy-Item (Join-Path $root "scripts\cubecheck.sh") (Join-Path $OutDir "cubecheck.sh") -Force
    $have = 0
    foreach ($key in $Payloads.Keys) {
        $src = $Payloads[$key]
        if (-not (Test-UnixRustPayload $src)) {
            Warn "${Name}: пропускаю ${key} (нужен Rust ELF cubecheck без .dll, не Avalonia)"
            continue
        }
        Copy-Tree $src (Join-Path $payloadRoot $key)
        $have++
    }
    if ($have -lt 1) { throw "${Name}: нет payload" }
}

function New-WinUniversal([string]$OutDir, [string]$X64, [string]$X86) {
    if (Test-Path $OutDir) { Remove-Item $OutDir -Recurse -Force }
    New-Item -ItemType Directory -Force -Path (Join-Path $OutDir "payload") | Out-Null
    Copy-Item (Join-Path $outNative "cubecheck-launcher.exe") (Join-Path $OutDir "cubecheck-launcher.exe") -Force
    Copy-Item (Join-Path $OutDir "cubecheck-launcher.exe") (Join-Path $OutDir "cubecheck.exe") -Force
    Copy-Tree $X64 (Join-Path $OutDir "payload\windows-x64")
    if (Test-Path (Join-Path $X86 "cubecheck.exe")) {
        Copy-Tree $X86 (Join-Path $OutDir "payload\windows-x86")
    }
}

function Copy-Vendor([string]$DestAssets) {
    $src = Join-Path $root "assets"
    $list = Join-Path $root "scripts\vendor-files.txt"
    if (-not (Test-Path $list)) { return $false }
    $ok = $true
    New-Item -ItemType Directory -Force -Path $DestAssets | Out-Null
    Get-Content $list | ForEach-Object {
        $rel = $_.Trim()
        if (-not $rel -or $rel.StartsWith("#")) { return }
        $from = Join-Path $src $rel
        $to = Join-Path $DestAssets $rel
        if (Test-Path $from) {
            New-Item -ItemType Directory -Force -Path (Split-Path $to) | Out-Null
            Copy-Item $from $to -Force
        } else {
            Warn "нет vendor $rel"
            $ok = $false
        }
    }
    return $ok
}

function Stage-Release($Name, $PathOrDir) {
    $dest = Join-Path $buildOut $Name
    if (Test-Path $PathOrDir -PathType Container) {
        Zip-Dir $PathOrDir $dest
    } else {
        Copy-Item $PathOrDir $dest -Force
    }
}

function Test-FileMagic([string]$Path, [byte[]]$Magic) {
    if (-not (Test-Path -LiteralPath $Path)) { return $false }
    $fs = [System.IO.File]::OpenRead($Path)
    try {
        $buf = New-Object byte[] $Magic.Length
        $n = $fs.Read($buf, 0, $Magic.Length)
        if ($n -lt $Magic.Length) { return $false }
        for ($i = 0; $i -lt $Magic.Length; $i++) {
            if ($buf[$i] -ne $Magic[$i]) { return $false }
        }
        return $true
    } finally { $fs.Close() }
}

function Test-Elf([string]$Path) {
    Test-FileMagic $Path ([byte[]](0x7F, 0x45, 0x4C, 0x46))
}

function Get-DllFiles([string]$Dir) {
    if (-not (Test-Path -LiteralPath $Dir)) { return @() }
    @(Get-ChildItem -LiteralPath $Dir -Recurse -File -Filter "*.dll" -ErrorAction SilentlyContinue)
}

function Test-UnixRustPayload([string]$Src) {
    $bin = Join-Path $Src "cubecheck"
    if (-not (Test-Elf $bin)) { return $false }
    if ((Get-DllFiles $Src).Count -gt 0) { return $false }
    return $true
}

function Assert-LinuxPayload([string]$Dir) {
    $dlls = Get-DllFiles $Dir
    if ($dlls.Count -gt 0) {
        throw "linux-x64 не должен содержать .dll: $(($dlls | Select-Object -First 8).Name -join ', ')"
    }
    $exes = @(Get-ChildItem -LiteralPath $Dir -Recurse -File -Filter "*.exe" -ErrorAction SilentlyContinue)
    if ($exes.Count -gt 0) {
        throw "linux-x64 не должен содержать Windows .exe: $($exes.Name -join ', ')"
    }
    $bin = Join-Path $Dir "cubecheck"
    if (-not (Test-Elf $bin)) { throw "linux-x64: cubecheck не ELF ($bin)" }
}

function Write-UnixText([string]$Path, [string]$Text) {
    $unix = $Text -replace "`r`n", "`n" -replace "`r", "`n"
    if (-not $unix.EndsWith("`n")) { $unix += "`n" }
    $utf8 = New-Object System.Text.UTF8Encoding $false
    [System.IO.File]::WriteAllText($Path, $unix, $utf8)
}

function Write-Sha256Sums([string]$Root) {
    $sums = Join-Path $Root "SHA256SUMS"
    $lines = New-Object System.Collections.Generic.List[string]
    Get-ChildItem -LiteralPath $Root -Recurse -File | Where-Object { $_.Name -ne "SHA256SUMS" } | ForEach-Object {
        $rel = $_.FullName.Substring($Root.Length).TrimStart('\', '/').Replace('\', '/')
        $hash = (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
        $lines.Add("$hash  $rel")
    }
    $utf8 = New-Object System.Text.UTF8Encoding $false
    [System.IO.File]::WriteAllLines($sums, $lines, $utf8)
}

function Copy-OnlineOsTree([string]$Src, [string]$Dst) {
    if (-not (Test-Path -LiteralPath $Src)) { return $false }
    Copy-Tree $Src $Dst
    Get-ChildItem -LiteralPath $Dst -Recurse -Force -File -ErrorAction SilentlyContinue | ForEach-Object {
        $name = $_.Name
        if ($name -match '^(Everything\.exe|Shellbag\.exe|Procmon.*\.exe|Autoruns.*\.exe|procexp.*\.exe)$' -or
            $_.FullName -match '\\SystemInformer\\' -or
            $_.Extension -match '\.(pdb|xml|exp|lib|obj|ilk)$') {
            Remove-Item -LiteralPath $_.FullName -Force
        }
    }
    Get-ChildItem -LiteralPath $Dst -Recurse -Directory -Force -ErrorAction SilentlyContinue |
        Where-Object { $_.Name -eq "SystemInformer" -or $_.Name -eq "extras" } |
        ForEach-Object { Remove-Item -LiteralPath $_.FullName -Recurse -Force }
    return $true
}

function Stage-GithubPayload {
    $upload = Join-Path $dist "github-upload"
    Info "GitHub payload (online, без vendor) -> $upload"
    if (Test-Path -LiteralPath $upload) { Remove-Item -LiteralPath $upload -Recurse -Force }
    New-Item -ItemType Directory -Force -Path $upload | Out-Null

    $lic = Join-Path $root "LICENSE.md"
    if (-not (Test-Path -LiteralPath $lic)) { throw "нет LICENSE.md" }
    Copy-Item -LiteralPath $lic -Destination (Join-Path $upload "LICENSE.md") -Force

    $payloadReadme = Join-Path $PSScriptRoot "payload-README.md"
    if (Test-Path -LiteralPath $payloadReadme) {
        Copy-Item -LiteralPath $payloadReadme -Destination (Join-Path $upload "README.md") -Force
    }

    foreach ($sh in @("install-linux.sh", "install-macos.sh")) {
        $from = Join-Path $root "scripts\$sh"
        if (Test-Path -LiteralPath $from) {
            Copy-Item -LiteralPath $from -Destination (Join-Path $upload $sh) -Force
        }
    }

    $win = Join-Path $dist "windows-x64"
    if (-not (Test-Path -LiteralPath (Join-Path $win "cubecheck.exe"))) {
        $uni = Join-Path $dist "universal-windows\payload\windows-x64"
        if (Test-Path -LiteralPath (Join-Path $uni "cubecheck.exe")) { $win = $uni }
    }
    if (Test-Path -LiteralPath (Join-Path $win "cubecheck.exe")) {
        [void](Copy-OnlineOsTree $win (Join-Path $upload "windows-x64"))
        Sanitize-WindowsPayload (Join-Path $upload "windows-x64")
        Write-UninstallHelpers (Join-Path $upload "windows-x64")
    } else {
        Warn "github-upload: нет windows-x64 cubecheck.exe — соберите windows-x64"
    }

    foreach ($key in @("linux-x64", "linux-x86")) {
        $src = Join-Path $dist $key
        if (Test-UnixRustPayload $src) {
            [void](Copy-OnlineOsTree $src (Join-Path $upload $key))
        }
    }

    foreach ($key in @("osx-x64", "osx-arm64")) {
        $src = Join-Path $dist $key
        if (-not (Test-Path -LiteralPath (Join-Path $src "cubecheck"))) {
            $src = Join-Path $dist "universal-macos\payload\$key"
        }
        if (Get-Command Test-MacPayload -ErrorAction SilentlyContinue) {
            if (Test-MacPayload $src) {
                [void](Copy-OnlineOsTree $src (Join-Path $upload $key))
            }
        } elseif ((Test-Path -LiteralPath (Join-Path $src "cubecheck")) -and ((Get-DllFiles $src).Count -eq 0)) {
            [void](Copy-OnlineOsTree $src (Join-Path $upload $key))
        }
    }

    if (-not (Test-Path -LiteralPath (Join-Path $upload "windows-x64\cubecheck.exe")) -and
        -not (Test-Path -LiteralPath (Join-Path $upload "linux-x64\cubecheck"))) {
        throw "github-upload пуст: нет windows-x64 и linux-x64 payload"
    }

    Write-Sha256Sums $upload
    $zip = Join-Path $buildOut "CubeCheck-$Version-github-payload.zip"
    Zip-Dir $upload $zip
    $buildUpload = Join-Path $buildOut "github-upload"
    Copy-Tree $upload $buildUpload
    Info "payload zip: $zip ($((Get-Item $zip).Length) байт)"
    return $zip
}

function Publish-WizardInstaller {
    param(
        [string]$OutExe = $(Join-Path $buildOut "CubeCheck-$Version-universal-windows-setup.exe"),
        [string]$OfflinePayloadZip = ""
    )
    $offline = -not [string]::IsNullOrWhiteSpace($OfflinePayloadZip)
    if ($offline) {
        Info "WPF мастер CubeCheck.Installer (офлайн, встроенный zip)"
        if (-not (Test-Path -LiteralPath $OfflinePayloadZip)) {
            throw "нет офлайн payload zip: $OfflinePayloadZip"
        }
        $zipLen = (Get-Item -LiteralPath $OfflinePayloadZip).Length
        if ($zipLen -lt 1000000) { throw "офлайн payload zip слишком маленький ($zipLen байт)" }
    } else {
        Info "WPF FDD мастер CubeCheck.Installer (онлайн, без payload внутри exe)"
    }

    $pub = if ($offline) { Join-Path $dist "installer-win-x64-offline" } else { Join-Path $dist "installer-win-x64" }
    if (Test-Path -LiteralPath $pub) { Remove-Item -LiteralPath $pub -Recurse -Force }
    New-Item -ItemType Directory -Force -Path $pub | Out-Null
    $proj = Join-Path $dotnet "CubeCheck.Installer\CubeCheck.Installer.csproj"
    $msbuildArgs = @(
        $proj, "-c", $cfg, "-f", "net8.0-windows", "-r", "win-x64",
        "--self-contained", "false", "-o", $pub,
        "/p:PublishSingleFile=true",
        "/p:IncludeNativeLibrariesForSelfExtract=false",
        "/p:IncludeAllContentForSelfExtract=false"
    )
    if ($offline) {
        $msbuildArgs += "/p:OfflinePayloadZip=$OfflinePayloadZip"
        $msbuildArgs += "/p:DefineConstants=CUBECHECK_OFFLINE_SETUP"
    }
    dotnet publish @msbuildArgs
    if ($LASTEXITCODE -ne 0) { throw "publish CubeCheck.Installer не удался" }

    $built = Join-Path $pub "CubeCheck-Setup.exe"
    if (-not (Test-Path -LiteralPath $built)) { throw "нет CubeCheck-Setup.exe после publish" }
    New-Item -ItemType Directory -Force -Path (Split-Path $OutExe) | Out-Null
    Copy-Item -LiteralPath $built -Destination $OutExe -Force
    Get-ChildItem -LiteralPath $pub -Filter "*.zip" -ErrorAction SilentlyContinue | ForEach-Object {
        throw "publish положил zip в установщик: $($_.FullName)"
    }
    $len = (Get-Item $OutExe).Length
    if ($len -lt 20000) { throw "setup.exe слишком маленький ($len байт)" }
    if (-not $offline -and $len -gt 15MB) {
        throw "онлайн setup.exe слишком большой ($len байт) — payload не должен быть внутри"
    }
    if ($offline -and $len -lt 1MB) {
        throw "офлайн setup.exe слишком маленький ($len байт) — нет встроенного пакета"
    }
    if (-not (Test-FileMagic $OutExe ([byte[]](0x4D, 0x5A)))) {
        throw "setup.exe не PE: $OutExe"
    }
    Info "wizard: $OutExe ($len байт)"
    return $OutExe
}

function Write-LinuxLauncher([string]$Path) {
    Write-UnixText $Path @'
#!/bin/sh
set -eu
ROOT=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
chmod +x "$ROOT/cubecheck" "$ROOT/assets/bin/"* 2>/dev/null || true
export CUBECHECK_PORTABLE=1
export APPIMAGE_EXTRACT_AND_RUN=1
if [ -f "$ROOT/.offline" ] || [ -f "$ROOT/assets/.offline" ]; then
  export CUBECHECK_OFFLINE=1
fi
export PATH="$ROOT/assets/bin:$PATH"
cd "$ROOT"
exec "$ROOT/cubecheck" "$@"
'@
}

function Use-AsciiTemp {
    $ascii = "C:\JumpWorld\tmp-ascii"
    New-Item -ItemType Directory -Force -Path $ascii | Out-Null
    $env:TEMP = $ascii
    $env:TMP = $ascii
    $env:ZIG_GLOBAL_CACHE_DIR = Join-Path $ascii "zig-cache"
    $env:ZIG_LOCAL_CACHE_DIR = Join-Path $ascii "zig-local"
}

function Ensure-RustupTarget([string]$Triple) {
    $installed = @(rustup target list --installed)
    if ($installed -notcontains $Triple) {
        Info "rustup target add $Triple"
        rustup target add $Triple
        if ($LASTEXITCODE -ne 0) { throw "не удалось установить rustup target $Triple" }
    }
}

function Ensure-ZigFilter {
    $wrapDir = Join-Path $root ".zig-wrappers"
    New-Item -ItemType Directory -Force -Path $wrapDir | Out-Null
    $src = Join-Path $root "scripts\zig-cc-filter.rs"
    $exe = Join-Path $wrapDir "zig-cc-filter.exe"
    if (-not (Test-Path -LiteralPath $src)) { throw "нет scripts/zig-cc-filter.rs" }
    $need = $true
    if (Test-Path -LiteralPath $exe) {
        if ((Get-Item $exe).LastWriteTime -ge (Get-Item $src).LastWriteTime) { $need = $false }
    }
    if ($need) {
        Info "rustc zig-cc-filter"
        rustc -O -o $exe $src
        if ($LASTEXITCODE -ne 0) { throw "zig-cc-filter не собрался" }
    }
    return $exe
}

function Resolve-ZigAscii {
    $preferred = @(
        "C:\JumpWorld\tools\zig-copy\zig.exe",
        "C:\JumpWorld\tools\zig\zig.exe"
    )
    foreach ($p in $preferred) {
        if (Test-Path -LiteralPath $p) { return $p }
    }
    $zigCmd = Get-Command zig -ErrorAction SilentlyContinue
    if (-not $zigCmd) { throw "zig не найден (нужен для кросс-сборки Linux ELF с Windows)" }
    $link = Get-Item -LiteralPath $zigCmd.Source
    $real = if ($link.Target) { [string]$link.Target } else { $link.FullName }
    $realDir = Split-Path $real
    $asciiDir = "C:\JumpWorld\tools\zig-copy"
    $asciiExe = Join-Path $asciiDir "zig.exe"
    New-Item -ItemType Directory -Force -Path (Split-Path $asciiDir) | Out-Null
    if (-not (Test-Path -LiteralPath $asciiExe)) {
        Info "копирую zig на ASCII-путь $asciiDir (WinGet лежит в профиле с кириллицей)"
        New-Item -ItemType Directory -Force -Path $asciiDir | Out-Null
        Copy-Item -LiteralPath (Join-Path $realDir "*") -Destination $asciiDir -Recurse -Force
    }
    if (-not (Test-Path -LiteralPath $asciiExe)) { throw "не удалось подготовить zig на ASCII-пути" }
    return $asciiExe
}

function Set-ZigLinuxEnv {
    $zig = Resolve-ZigAscii
    Info "zig: $zig"
    $filter = Ensure-ZigFilter
    $wrapDir = Join-Path $root ".zig-wrappers"
    $cc = Join-Path $wrapDir "zig-cc-x86_64-unknown-linux-gnu.cmd"
    $ar = Join-Path $wrapDir "zig-ar-x86_64-unknown-linux-gnu.cmd"
    Set-Content -LiteralPath $cc -Value "@echo off`r`n`"$filter`" `"$zig`" cc x86_64-linux-gnu.2.17 %*" -Encoding ascii
    Set-Content -LiteralPath $ar -Value "@echo off`r`n`"$filter`" `"$zig`" ar %*" -Encoding ascii
    $env:CC_x86_64_unknown_linux_gnu = $cc
    $env:CXX_x86_64_unknown_linux_gnu = $cc
    $env:AR_x86_64_unknown_linux_gnu = $ar
    $env:CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER = $cc
    $env:CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_AR = $ar
    $env:PKG_CONFIG_ALLOW_CROSS = "1"
    $pkg = Join-Path $wrapDir "empty-pkgconfig"
    New-Item -ItemType Directory -Force -Path $pkg | Out-Null
    $env:PKG_CONFIG_LIBDIR = $pkg
}

function Remove-DllDump([string]$Dir, [string]$Why) {
    if (-not (Test-Path -LiteralPath $Dir)) { return }
    if ((Get-DllFiles $Dir).Count -gt 0) {
        Warn "$Why — удаляю $Dir"
        Remove-Item -LiteralPath $Dir -Recurse -Force
    }
}

function Clear-BuildAvaloniaJunk {
    Get-ChildItem -LiteralPath $buildOut -File -ErrorAction SilentlyContinue | Where-Object {
        $_.Name -like "Avalonia*" -or
        $_.Name -like "System.*" -or
        $_.Name -eq "cubecheck.dll" -or
        $_.Name -eq "CubeCheck.Core.dll" -or
        $_.Name -like "Microsoft.*" -or
        $_.Name -eq "HarfBuzzSharp.dll" -or
        $_.Name -eq "SkiaSharp.dll" -or
        $_.Name -eq "WindowsBase.dll" -or
        $_.Name -eq "netstandard.dll" -or
        $_.Name -eq "mscorlib.dll"
    } | ForEach-Object { Remove-Item -LiteralPath $_.FullName -Force }
}

function Try-PublishLinuxApi {
    Info "Пробую NativeAOT libcubecheck_api.so (linux-x64) с этой Windows-машины"
    $out = Join-Path $dist "api-linux-x64"
    New-Item -ItemType Directory -Force -Path $out | Out-Null
    $ok = $false
    try {
        dotnet publish (Join-Path $dotnet "CubeCheck.Api\CubeCheck.Api.csproj") `
            -c $cfg -f net8.0 -r linux-x64 --self-contained true -o $out
        $so = Join-Path $out "libcubecheck_api.so"
        if ($LASTEXITCODE -eq 0 -and (Test-Elf $so)) {
            Info "libcubecheck_api.so готов"
            $ok = $true
        } else {
            Warn "NativeAOT linux-x64 не дал ELF .so (dotnet exit=$LASTEXITCODE). С этой Windows-машины кросс-publish Linux NativeAOT недоступен (нужен Linux clang/sysroot). Не кладу cubecheck_api.dll в Linux-пакет."
        }
    } catch {
        Warn "NativeAOT linux-x64 не собран: $($_.Exception.Message)"
    }
    if (-not $ok) {
        if (Test-Path -LiteralPath $out) { Remove-Item -LiteralPath $out -Recurse -Force -ErrorAction SilentlyContinue }
    }
    return $ok
}

function Stage-LinuxX64([string]$Elf, [string]$OutDir) {
    if (Test-Path -LiteralPath $OutDir) { Remove-Item -LiteralPath $OutDir -Recurse -Force }
    New-Item -ItemType Directory -Force -Path (Join-Path $OutDir "assets") | Out-Null
    Copy-Item -LiteralPath $Elf -Destination (Join-Path $OutDir "cubecheck") -Force
    Write-LinuxLauncher (Join-Path $OutDir "cubecheck.sh")
    foreach ($name in @("tools.json", "cubecheck.ico", "settings.default.json")) {
        $from = Join-Path $root "assets\$name"
        if (Test-Path -LiteralPath $from) {
            Copy-Item -LiteralPath $from -Destination (Join-Path $OutDir "assets\$name") -Force
        }
    }
    Set-Content -Path (Join-Path $OutDir ".portable") -Value ""
    Get-ChildItem -LiteralPath $OutDir -Force | ForEach-Object {
        $keep = @("cubecheck", "cubecheck.sh", "assets", ".portable", "libcubecheck_api.so")
        if ($keep -notcontains $_.Name) {
            Remove-Item -LiteralPath $_.FullName -Recurse -Force
        }
    }
    $so = Join-Path $dist "api-linux-x64\libcubecheck_api.so"
    if ((Test-Path -LiteralPath $so) -and (Test-Elf $so)) {
        Copy-Item -LiteralPath $so -Destination (Join-Path $OutDir "libcubecheck_api.so") -Force
    } else {
        Warn "В пакете нет libcubecheck_api.so. UI — Rust ELF; C# бэкенд нужно собрать на Linux: dotnet publish src/CubeCheck.Api -r linux-x64 -c Release"
    }
    Assert-LinuxPayload $OutDir
}

function Build-RustLinux([string]$OutDir) {
    Info "Rust egui cubecheck (x86_64-unknown-linux-gnu) — не Avalonia"
    Ensure-RustupTarget "x86_64-unknown-linux-gnu"
    Use-AsciiTemp
    [void](Try-PublishLinuxApi)

    $triple = "x86_64-unknown-linux-gnu"
    $elf = Join-Path $root "target\$triple\release\cubecheck"
    $built = $false

    Push-Location $root
    try {
        $haveZigbuild = $false
        $homeAscii = [regex]::IsMatch([string]$env:USERPROFILE, '^[ -~\\:]+$')
        if ($homeAscii -and (Get-Command cargo-zigbuild -ErrorAction SilentlyContinue)) { $haveZigbuild = $true }
        elseif (-not $homeAscii) { Warn "cargo zigbuild пропущен: профиль не ASCII, zig-cc-filter + ASCII zig" }
        if ($haveZigbuild) {
            Info "cargo zigbuild --target $triple"
            cargo zigbuild -p cubecheck --release --bin cubecheck --features gui --target $triple
            if ($LASTEXITCODE -eq 0 -and (Test-Elf $elf)) {
                $built = $true
            } else {
                Warn "cargo zigbuild не дал ELF (exit=$LASTEXITCODE), пробую zig-cc-filter + cargo"
            }
        }
        if (-not $built) {
            Set-ZigLinuxEnv
            Info "cargo build --target $triple (zig cc)"
            cargo build -p cubecheck --release --bin cubecheck --features gui --target $triple
            if ($LASTEXITCODE -ne 0) {
                throw "Кросс-сборка Rust Linux ELF не удалась (exit=$LASTEXITCODE). linux-x64 НЕ будет упакован из Avalonia DLL."
            }
        }
    } finally {
        Pop-Location
    }

    if (-not (Test-Elf $elf)) {
        throw "Нет Linux ELF после cargo: $elf. linux-x64 не пакуем (Avalonia запрещён)."
    }
    Info "Linux ELF: $elf ($((Get-Item $elf).Length) байт)"
    Stage-LinuxX64 $elf $OutDir
}

# --- compile native first (needed by leftover .NET Windows publishes) ---
$needNative = $Target -match "^(?i)(all|release|windows-x64|windows-x86|universal-win|universal|universal-local|github|installer|wizard)$"
if ($needNative) {
    Compile-Native "x64"
    try { Compile-Native "x86" } catch { Warn $_.Exception.Message }
}

. (Join-Path $PSScriptRoot "Release-Universal.ps1")

if ($Target -match "^(?i)(github|installer|wizard)$") {
    if (-not (Test-Path (Join-Path $dist "windows-x64\cubecheck.exe"))) {
        Warn "нет dist\windows-x64 — собираю windows-x64 для payload"
        Build-RustWindows (Join-Path $dist "windows-x64") -SkipLegacyRelease
    }
    Stage-GithubPayload
    Publish-WizardInstaller
    Write-Host ""
    Write-Host "GitHub payload + wizard:"
    foreach ($name in @("CubeCheck-$Version-github-payload.zip", "CubeCheck-$Version-universal-windows-setup.exe")) {
        $p = Join-Path $buildOut $name
        if (Test-Path $p) {
            $i = Get-Item $p
            Write-Host ("  {0,-62} {1,12:N0}" -f $i.Name, $i.Length)
        } else {
            Write-Host "  MISSING $name"
        }
    }
    return
}

$win64Net8 = Join-Path $dist "windows-x64"
$win64Fx = Join-Path $dist "windows-x64-win7"
$win86 = Join-Path $dist "windows-x86"
$linux64 = Join-Path $dist "linux-x64"
$osx64 = Join-Path $dist "osx-x64"
$osxArm = Join-Path $dist "osx-arm64"

if ($Target -match "^(?i)(all|release|universal|universal-local)$") {
    Publish-UniversalReleaseSet
    Write-Host ""
    Write-Host "Готово. Артефакты:"
    Write-Host "  $dist"
    Write-Host "  $buildOut"
    Get-ChildItem $buildOut -ErrorAction SilentlyContinue | ForEach-Object {
        Write-Host ("  {0,-48} {1,10:N0} байт" -f $_.Name, $_.Length)
    }
    return
}

$do = {
    param($name)
    $Target -match "^(?i)($name)$"
}

if (& $do "windows-x64") {
    Build-RustWindows $win64Net8
    Stage-Release "CubeCheck-$Version-windows-x64.zip" $win64Net8
}

if (& $do "windows-x86") {
    Publish-WinNet48 "win-x86" $win86
    Stage-Release "CubeCheck-$Version-windows-x86.zip" $win86
}

if ((& $do "universal-win") -or (& $do "universal") -or (& $do "universal-local")) {
    Publish-WinNet48 "win-x64" $win64Fx
    if (-not (Test-Path (Join-Path $win86 "cubecheck.exe"))) {
        try { Publish-WinNet48 "win-x86" $win86 } catch { Warn $_.Exception.Message }
    }
}

$linuxOk = $false
if ((& $do "linux-x64") -or (& $do "linux-universal") -or (& $do "universal") -or (& $do "universal-local")) {
    try {
        Build-RustLinux $linux64
        $linuxOk = Test-UnixRustPayload $linux64
    } catch {
        Warn $_.Exception.Message
        Remove-DllDump $linux64 "leftover Avalonia/DLL в dist\linux-x64"
        if (& $do "linux-x64") {
            throw "linux-x64: нет Rust ELF. Avalonia больше не используется как замена."
        }
    }
}

if ((& $do "macos-universal") -or (& $do "universal") -or (& $do "universal-local")) {
    Warn "macOS Mach-O с этой Windows-машины не собирается (нет Apple SDK / osxcross). Avalonia DLL не пакуем."
    Remove-DllDump $osx64 "leftover Avalonia в dist\osx-x64"
    Remove-DllDump $osxArm "leftover Avalonia в dist\osx-arm64"
    if (& $do "macos-universal") {
        Warn "macos-universal: пакет не создан — нет честного Mach-O, Avalonia запрещён."
    }
}

if (& $do "linux-x64") {
    if (-not $linuxOk) {
        throw "linux-x64: нет Rust ELF payload. Архив не создан."
    }
    Assert-LinuxPayload $linux64
    Clear-BuildAvaloniaJunk
    $tar = Join-Path $buildOut "CubeCheck-$Version-linux-x64.tar.gz"
    if (Test-Path -LiteralPath $tar) { Remove-Item -LiteralPath $tar -Force }
    tar -czf $tar -C $linux64 .
    if ($LASTEXITCODE -ne 0) { throw "tar linux-x64 не удался" }
    Stage-Release "CubeCheck-$Version-linux-x64.zip" $linux64
    $dllInTar = @(tar -tzf $tar | Where-Object { $_ -match '\.dll$' })
    if ($dllInTar.Count -gt 0) {
        throw "linux-x64.tar.gz всё ещё содержит .dll: $($dllInTar -join ', ')"
    }
}

if ((& $do "linux-universal") -and $linuxOk) {
    $lu = Join-Path $dist "linux-universal"
    New-ShBundle "linux-universal" @{ "linux-x64" = $linux64 } $lu
    Stage-Release "CubeCheck-$Version-linux-universal.zip" $lu
}

if (& $do "universal-win") {
    $uw = Join-Path $dist "universal-win"
    New-WinUniversal $uw $win64Fx $win86
    Stage-Release "CubeCheck-$Version-universal-win.zip" $uw
}

if ((& $do "universal") -or (& $do "universal-local")) {
    $uni = Join-Path $dist "universal\CubeCheck-universal"
    if (Test-Path $uni) { Remove-Item $uni -Recurse -Force }
    New-Item -ItemType Directory -Force -Path (Join-Path $uni "payload") | Out-Null
    Copy-Item (Join-Path $outNative "cubecheck-launcher.exe") (Join-Path $uni "cubecheck-launcher.exe") -Force -ErrorAction SilentlyContinue
    Copy-Item (Join-Path $root "scripts\cubecheck.sh") (Join-Path $uni "cubecheck.sh") -Force
    if (Test-Path (Join-Path $win64Fx "cubecheck.exe")) { Copy-Tree $win64Fx (Join-Path $uni "payload\windows-x64") }
    elseif (Test-Path (Join-Path $win64Net8 "cubecheck.exe")) { Copy-Tree $win64Net8 (Join-Path $uni "payload\windows-x64") }
    if (Test-Path (Join-Path $win86 "cubecheck.exe")) { Copy-Tree $win86 (Join-Path $uni "payload\windows-x86") }
    if (Test-UnixRustPayload $linux64) { Copy-Tree $linux64 (Join-Path $uni "payload\linux-x64") }
    else { Warn "universal: пропускаю linux-x64 (нет Rust ELF без .dll)" }
    if (Test-UnixRustPayload $osx64) { Copy-Tree $osx64 (Join-Path $uni "payload\osx-x64") }
    if (Test-UnixRustPayload $osxArm) { Copy-Tree $osxArm (Join-Path $uni "payload\osx-arm64") }
    Stage-Release "CubeCheck-$Version-universal.zip" $uni

    if (& $do "universal-local") {
        $ul = Join-Path $dist "universal-local\CubeCheck-universal-local"
        Copy-Tree $uni $ul
        Set-Content -Path (Join-Path $ul ".offline") -Value ""
        $assets = Join-Path $ul "payload\windows-x64\assets"
        if (Test-Path $assets) {
            [void](Copy-Vendor $assets)
            Set-Content -Path (Join-Path $assets ".offline") -Value ""
        }
        Stage-Release "CubeCheck-$Version-universal-local.zip" $ul
    }
}

Write-Host ""
Write-Host "Готово. Артефакты:"
Write-Host "  $dist"
Write-Host "  $buildOut"
Get-ChildItem $buildOut -ErrorAction SilentlyContinue | ForEach-Object {
    Write-Host ("  {0,-48} {1,10:N0} байт" -f $_.Name, $_.Length)
}
