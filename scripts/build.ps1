[CmdletBinding()]
param(
    [Parameter(Position = 0)]
    [string]$Target = "windows-x64"
)

$ErrorActionPreference = "Stop"
$Root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
Set-Location $Root

$script:Results = @()
$HostTriple = "x86_64-pc-windows-msvc"
try {
    $v = & rustc -vV 2>$null
    $line = $v | Where-Object { $_ -like "host:*" } | Select-Object -First 1
    if ($line) { $HostTriple = ($line -split ":", 2)[1].Trim() }
} catch {}

function Get-AppVersion {
    $toml = Get-Content (Join-Path $Root "Cargo.toml") -Raw
    if ($toml -match '(?m)^version\s*=\s*"([^"]+)"') { return $Matches[1] }
    return "1.0.0-beta"
}

$Version = Get-AppVersion
$Dist = Join-Path $Root "dist"
$BuildOut = Join-Path $Root "build"
$VendorList = Join-Path $Root "scripts\vendor-files.txt"

function Write-Info($msg) { Write-Host $msg }
function Write-Warn($msg) { Write-Host "WARNING: $msg" -ForegroundColor Yellow }
function Write-Err($msg) { Write-Host "ERROR: $msg" -ForegroundColor Red }

function Record-Result($name, $status, $detail) {
    $script:Results += [pscustomobject]@{ Name = $name; Status = $status; Detail = $detail }
}

function Ensure-Cargo {
    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        throw "Rust/cargo not found. Install from https://rustup.rs"
    }
}

function Ensure-RustupTarget($triple) {
    $installed = & rustup target list --installed 2>$null
    if ($installed -notcontains $triple) {
        Write-Info "rustup target add $triple"
        & rustup target add $triple
        if ($LASTEXITCODE -ne 0) {
            throw "Failed to add Rust target $triple"
        }
    }
}

function Get-CargoBin($triple, $binName, $targetDir = "target") {
    $exe = if ($binName -notmatch '\.(exe|dll)$' -and $triple -like "*windows*") { "$binName.exe" } else { $binName }
    if ($triple -and $triple -ne $HostTriple) {
        return Join-Path $Root "$targetDir\$triple\release\$exe"
    }
    return Join-Path $Root "$targetDir\release\$exe"
}

function Invoke-CargoBuild {
    param(
        [string[]]$CargoArgs,
        [string]$FailMessage
    )
    Write-Info ("cargo " + ($CargoArgs -join " "))
    & cargo @CargoArgs
    if ($LASTEXITCODE -ne 0) {
        throw $FailMessage
    }
}

function Copy-FileForce($src, $dst) {
    $dir = Split-Path -Parent $dst
    if (-not (Test-Path $dir)) { New-Item -ItemType Directory -Path $dir | Out-Null }
    Copy-Item -LiteralPath $src -Destination $dst -Force
}

function Reset-NativeExit {
    $global:LASTEXITCODE = 0
}

function Copy-Tree($src, $dst) {
    if (Test-Path -LiteralPath $dst) {
        Remove-Item -LiteralPath $dst -Recurse -Force
    }
    New-Item -ItemType Directory -Path $dst -Force | Out-Null
    if (Test-Path -LiteralPath $src) {
        & robocopy $src $dst /E /NFL /NDL /NJH /NJS /nc /ns /np | Out-Null
        if ($LASTEXITCODE -ge 8) {
            throw "robocopy failed ($LASTEXITCODE) $src -> $dst"
        }
        Reset-NativeExit
    }
}

function Write-SkipReadme($dir, $kind, $detail) {
    if (-not (Test-Path -LiteralPath $dir)) {
        New-Item -ItemType Directory -Path $dir | Out-Null
    }
    $txt = @"
CubeCheck '$kind' was not compiled on this machine.

$detail

This folder is a pack layout / recipe, not a fake native binary.
Rebuild on the matching OS, or install cargo-zigbuild + zig (Linux) / an Apple SDK (macOS):

  build.bat $kind
  build.sh $kind
"@
    Set-Content -LiteralPath (Join-Path $dir "README.txt") -Value $txt -Encoding utf8
}

function Get-ReleaseFileName([string]$suffix) {
    return "CubeCheck-$Version-$suffix"
}

function Test-FileMagic([string]$path, [byte[]]$magic) {
    if ([string]::IsNullOrWhiteSpace($path)) { return $false }
    if (-not (Test-Path -LiteralPath $path)) { return $false }
    $item = Get-Item -LiteralPath $path
    if ($item.PSIsContainer -or $item.Length -lt $magic.Length) { return $false }
    $fs = [IO.File]::OpenRead($path)
    try {
        $buf = New-Object byte[] $magic.Length
        $n = $fs.Read($buf, 0, $magic.Length)
        if ($n -lt $magic.Length) { return $false }
        for ($i = 0; $i -lt $magic.Length; $i++) {
            if ($buf[$i] -ne $magic[$i]) { return $false }
        }
        return $true
    } finally { $fs.Dispose() }
}

function Test-RealDeb([string]$path) {
    Test-FileMagic $path ([byte[]](0x21, 0x3C, 0x61, 0x72, 0x63, 0x68, 0x3E))
}

function Test-Elf([string]$path) {
    Test-FileMagic $path ([byte[]](0x7F, 0x45, 0x4C, 0x46))
}

function Test-MachO([string]$path) {
    if ([string]::IsNullOrWhiteSpace($path)) { return $false }
    if (-not (Test-Path -LiteralPath $path)) { return $false }
    $item = Get-Item -LiteralPath $path
    if ($item.PSIsContainer -or $item.Length -lt 4) { return $false }
    $fs = [IO.File]::OpenRead($path)
    try {
        $buf = New-Object byte[] 4
        [void]$fs.Read($buf, 0, 4)
        $be = ([uint32]$buf[0] -shl 24) -bor ([uint32]$buf[1] -shl 16) -bor ([uint32]$buf[2] -shl 8) -bor [uint32]$buf[3]
        return @(
            [uint32]0xFEEDFACE, [uint32]0xCEFAEDFE, [uint32]0xFEEDFACF,
            [uint32]0xCFFAEDFE, [uint32]0xCAFEBABE, [uint32]0xBEBAFECA
        ) -contains $be
    } finally { $fs.Dispose() }
}

function Test-LinuxShInstaller([string]$path) {
    if (-not (Test-Path -LiteralPath $path)) { return $false }
    $item = Get-Item -LiteralPath $path
    if ($item.PSIsContainer -or $item.Length -lt 50000) { return $false }
    $fs = [IO.File]::OpenRead($path)
    try {
        $head = New-Object byte[] 2
        if ($fs.Read($head, 0, 2) -lt 2) { return $false }
        if ($head[0] -ne 0x23 -or $head[1] -ne 0x21) { return $false } # #!
        $fs.Position = 0
        $chunk = New-Object byte[] 65536
        $carry = New-Object byte[] 0
        $foundGz = $false
        while (($n = $fs.Read($chunk, 0, $chunk.Length)) -gt 0) {
            $data = New-Object byte[] ($carry.Length + $n)
            [Array]::Copy($carry, 0, $data, 0, $carry.Length)
            [Array]::Copy($chunk, 0, $data, $carry.Length, $n)
            for ($i = 0; $i -le $data.Length - 2; $i++) {
                if ($data[$i] -eq 0x1F -and $data[$i + 1] -eq 0x8B) { $foundGz = $true; break }
            }
            if ($foundGz) { break }
            $keep = [Math]::Min(1, $data.Length)
            $carry = New-Object byte[] $keep
            [Array]::Copy($data, $data.Length - $keep, $carry, 0, $keep)
        }
        return $foundGz
    } finally { $fs.Dispose() }
}

function Write-UnixText([string]$path, [string]$text) {
    $n = ($text -replace "`r`n", "`n") -replace "`r", "`n"
    if (-not $n.EndsWith("`n")) { $n += "`n" }
    [IO.File]::WriteAllBytes($path, [Text.Encoding]::UTF8.GetBytes($n))
}

function Get-LinuxShHeader([int]$skip, [string]$kind) {
    $tpl = Get-Content -LiteralPath (Join-Path $Root "scripts\linux-sh-header.sh") -Raw
    return (($tpl -replace '@VERSION@', $Version) -replace '@KIND@', $kind) -replace '@SKIP@', "$skip"
}

function Pack-LinuxShFromStage([string]$stage, [string]$outSh, [string]$kind) {
    $have = $false
    if (Test-Elf (Join-Path $stage "cubecheck")) { $have = $true }
    foreach ($id in @("linux-x64", "linux-x86")) {
        if (Test-Elf (Join-Path $stage "payload\$id\cubecheck")) { $have = $true }
    }
    if (-not $have) {
        Write-Warn "pack .sh skipped: no ELF in $stage"
        return $null
    }

    $tmp = Join-Path ([IO.Path]::GetTempPath()) ("cubecheck-sh-" + [guid]::NewGuid().ToString("n"))
    New-Item -ItemType Directory -Path $tmp | Out-Null
    try {
        $tgz = Join-Path $tmp "payload.tar.gz"
        Push-Location $stage
        try {
            Reset-NativeExit
            & tar -czf $tgz *
            if ($LASTEXITCODE -ne 0) {
                Reset-NativeExit
                & tar -a -cf $tgz *
            }
            if ($LASTEXITCODE -ne 0) { throw "tar.gz for .sh failed" }
        } finally { Pop-Location }

        $headerPath = Join-Path $tmp "header.sh"
        $skip = 0
        for ($pass = 0; $pass -lt 5; $pass++) {
            $hdr = Get-LinuxShHeader $skip $kind
            $hdr = ($hdr -replace "`r`n", "`n") -replace "`r", "`n"
            if (-not $hdr.EndsWith("`n")) { $hdr += "`n" }
            $lineCount = ([regex]::Matches($hdr, "`n")).Count
            $newSkip = $lineCount + 1
            Write-UnixText $headerPath $hdr
            if ($newSkip -eq $skip) { break }
            $skip = $newSkip
        }

        $outDir = Split-Path -Parent $outSh
        if (-not (Test-Path $outDir)) { New-Item -ItemType Directory -Path $outDir | Out-Null }
        $out = [IO.File]::Create($outSh)
        try {
            $h = [IO.File]::OpenRead($headerPath)
            try { $h.CopyTo($out) } finally { $h.Dispose() }
            $t = [IO.File]::OpenRead($tgz)
            try { $t.CopyTo($out) } finally { $t.Dispose() }
        } finally { $out.Dispose() }

        if (-not (Test-LinuxShInstaller $outSh)) {
            Remove-Item -LiteralPath $outSh -Force -ErrorAction SilentlyContinue
            throw "refusing dummy .sh (missing shebang/gzip payload): $outSh"
        }
        Write-Info "packed $outSh"
        return $outSh
    } finally {
        Remove-Item -LiteralPath $tmp -Recurse -Force -ErrorAction SilentlyContinue
    }
}

function Pack-LinuxShSingle([string]$elf, [string]$outSh, [string]$kind) {
    if (-not (Test-Elf $elf)) { return $null }
    $stage = Join-Path $Dist ("sh-stage-" + $kind)
    if (Test-Path -LiteralPath $stage) { Remove-Item -LiteralPath $stage -Recurse -Force }
    New-Item -ItemType Directory -Path $stage | Out-Null
    Copy-FileForce $elf (Join-Path $stage "cubecheck")
    Copy-CoreAssets (Join-Path $stage "assets")
    $lic = Join-Path $Root "LICENSE.md"
    if (Test-Path -LiteralPath $lic) { Copy-FileForce $lic (Join-Path $stage "LICENSE.md") }
    return Pack-LinuxShFromStage $stage $outSh $kind
}

function Clear-BuildOut {
    if (-not (Test-Path -LiteralPath $BuildOut)) {
        New-Item -ItemType Directory -Path $BuildOut | Out-Null
        return
    }
    $items = @(Get-ChildItem -LiteralPath $BuildOut -Force)
    foreach ($item in ($items | Where-Object { $_.Attributes -band [IO.FileAttributes]::ReparsePoint })) {
        $p = $item.FullName
        if ($item.PSIsContainer) { cmd /c "rmdir `"$p`"" | Out-Null }
        else { Remove-Item -LiteralPath $p -Force -ErrorAction SilentlyContinue }
    }
    foreach ($item in ($items | Where-Object { -not ($_.Attributes -band [IO.FileAttributes]::ReparsePoint) })) {
        Remove-Item -LiteralPath $item.FullName -Recurse -Force -ErrorAction SilentlyContinue
    }
}

function Publish-ReleaseFile([string]$src, [string]$destName) {
    if (-not $src -or -not (Test-Path -LiteralPath $src)) { return $false }
    if ((Get-Item -LiteralPath $src).PSIsContainer) { return $false }
    if (-not (Test-Path -LiteralPath $BuildOut)) {
        New-Item -ItemType Directory -Path $BuildOut | Out-Null
    }
    Copy-FileForce $src (Join-Path $BuildOut $destName)
    Write-Info "release: $destName"
    return $true
}

function Show-ReleaseBuild {
    Write-Host ""
    Write-Host "build/ (GitHub Release assets):"
    $files = @(Get-ChildItem -LiteralPath $BuildOut -Force -ErrorAction SilentlyContinue)
    if ($files.Count -eq 0) {
        Write-Host "  (empty)"
        return
    }
    $bad = @()
    foreach ($f in $files) {
        if ($f.PSIsContainer -or ($f.Attributes -band [IO.FileAttributes]::ReparsePoint)) {
            $bad += $f.Name
            Write-Host ("  {0,-12}  {1}  << not a release file" -f $f.Length, $f.Name) -ForegroundColor Yellow
        } else {
            Write-Host ("  {0,12:N0}  {1}" -f $f.Length, $f.Name)
        }
    }
    if ($bad.Count -gt 0) {
        Write-Warn ("build/ should only contain downloadable files, not: " + ($bad -join ", "))
    }
}

function Pack-WindowsPortable([string]$distName, [string]$exeSrc) {
    $outDir = Join-Path $Dist $distName
    $stageRoot = Join-Path $outDir "portable"
    $bundle = Join-Path $stageRoot "CubeCheck"
    if (Test-Path -LiteralPath $stageRoot) { Remove-Item -LiteralPath $stageRoot -Recurse -Force }
    New-Item -ItemType Directory -Path $bundle | Out-Null
    Copy-FileForce $exeSrc (Join-Path $bundle "cubecheck.exe")
    Copy-CoreAssets (Join-Path $bundle "assets")
    $lic = Join-Path $Root "LICENSE.md"
    if (Test-Path -LiteralPath $lic) { Copy-FileForce $lic (Join-Path $bundle "LICENSE.md") }
    $zip = Join-Path $outDir (Get-ReleaseFileName "$distName.zip")
    Compress-Dir $bundle $zip
    return $zip
}

function Pack-MacosZip {
    $bin = Join-Path $Dist "macos-universal\cubecheck-macos-universal"
    if (-not (Test-MachO $bin)) { return $null }
    $stageRoot = Join-Path $Dist "macos-universal\portable"
    $bundle = Join-Path $stageRoot "CubeCheck"
    if (Test-Path -LiteralPath $stageRoot) { Remove-Item -LiteralPath $stageRoot -Recurse -Force }
    New-Item -ItemType Directory -Path $bundle | Out-Null
    Copy-FileForce $bin (Join-Path $bundle "cubecheck")
    foreach ($slice in @(
        @{ src = "macos-arm64\cubecheck-macos-arm64"; name = "cubecheck-arm64" },
        @{ src = "macos-x64\cubecheck-macos-x64"; name = "cubecheck-x64" }
    )) {
        $p = Join-Path $Dist $slice.src
        if (Test-MachO $p) {
            Copy-FileForce $p (Join-Path $bundle $slice.name)
        }
    }
    Copy-CoreAssets (Join-Path $bundle "assets")
    $lic = Join-Path $Root "LICENSE.md"
    if (Test-Path -LiteralPath $lic) { Copy-FileForce $lic (Join-Path $bundle "LICENSE.md") }
    $zip = Join-Path $Dist "macos-universal\$(Get-ReleaseFileName 'macos-universal.zip')"
    Compress-Dir $bundle $zip
    return $zip
}

function Publish-ReleaseAssets {
    Clear-BuildOut

    foreach ($win in @("windows-x64", "windows-x86")) {
        $dir = Join-Path $Dist $win
        $setup = @(
            (Join-Path $dir "CubeCheck-Setup-$win.exe"),
            (Join-Path $dir "CubeCheck-Setup.exe")
        ) | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1
        if ($setup) {
            [void](Publish-ReleaseFile $setup (Get-ReleaseFileName "$win-setup.exe"))
            if ($win -eq "windows-x64") {
                [void](Publish-ReleaseFile $setup "CubeCheck-Setup.exe")
            }
        }
        $exe = Join-Path $dir "cubecheck-$win.exe"
        if (Test-Path -LiteralPath $exe) {
            $zip = Pack-WindowsPortable $win $exe
            [void](Publish-ReleaseFile $zip (Get-ReleaseFileName "$win.zip"))
        }
    }

    $debPairs = @(
        @{ Name = "linux-deb-x64"; Arch = "amd64" },
        @{ Name = "linux-deb-x86"; Arch = "i386" }
    )
    foreach ($pair in $debPairs) {
        $deb = Join-Path $Dist "$($pair.Name)\cubecheck_${Version}_$($pair.Arch).deb"
        $pkgBin = Join-Path $Dist "$($pair.Name)\pkg\usr\bin\cubecheck"
        if ((Test-RealDeb $deb) -and (Test-Elf $pkgBin)) {
            [void](Publish-ReleaseFile $deb (Get-ReleaseFileName "$($pair.Name).deb"))
        }
    }

    $shPairs = @(
        @{ Dist = "linux-deb-x64"; Kind = "linux-x64"; Elf = (Join-Path $Dist "linux-deb-x64\cubecheck-linux-deb-x64") },
        @{ Dist = "linux-deb-x86"; Kind = "linux-x86"; Elf = (Join-Path $Dist "linux-deb-x86\cubecheck-linux-deb-x86") }
    )
    foreach ($pair in $shPairs) {
        $sh = Join-Path $Dist "$($pair.Dist)\$(Get-ReleaseFileName "$($pair.Kind).sh")"
        if (-not ((Test-Path -LiteralPath $sh) -and (Test-LinuxShInstaller $sh))) {
            if (Test-Elf $pair.Elf) {
                try { [void](Pack-LinuxShSingle $pair.Elf $sh $pair.Kind) } catch { Write-Warn $_.Exception.Message }
            }
        }
        if ((Test-Path -LiteralPath $sh) -and (Test-LinuxShInstaller $sh)) {
            [void](Publish-ReleaseFile $sh (Get-ReleaseFileName "$($pair.Kind).sh"))
        }
    }

    $uniSh = Join-Path $Dist "linux-universal\$(Get-ReleaseFileName 'linux-universal.sh')"
    if ((Test-Path -LiteralPath $uniSh) -and (Test-LinuxShInstaller $uniSh)) {
        [void](Publish-ReleaseFile $uniSh (Get-ReleaseFileName "linux-universal.sh"))
    }

    $macZip = Pack-MacosZip
    if ($macZip) {
        [void](Publish-ReleaseFile $macZip (Get-ReleaseFileName "macos-universal.zip"))
    }

    foreach ($uni in @("universal", "universal-local")) {
        $dir = Join-Path $Dist $uni
        $zipVer = Join-Path $dir (Get-ReleaseFileName "$uni.zip")
        $zip = Join-Path $dir "CubeCheck-$uni.zip"
        $src = if (Test-Path -LiteralPath $zipVer) { $zipVer } elseif (Test-Path -LiteralPath $zip) { $zip } else { $null }
        if ($src) {
            [void](Publish-ReleaseFile $src (Get-ReleaseFileName "$uni.zip"))
        }
    }
}

function Write-Marker($path, $text = "") {
    $dir = Split-Path -Parent $path
    if ($dir -and -not (Test-Path $dir)) { New-Item -ItemType Directory -Path $dir | Out-Null }
    Set-Content -LiteralPath $path -Value $text -Encoding ascii
}

function Copy-CoreAssets($destAssets) {
    if (-not (Test-Path $destAssets)) { New-Item -ItemType Directory -Path $destAssets | Out-Null }
    Copy-FileForce (Join-Path $Root "assets\tools.json") (Join-Path $destAssets "tools.json")
    $ico = Join-Path $Root "assets\cubecheck.ico"
    if (Test-Path $ico) { Copy-FileForce $ico (Join-Path $destAssets "cubecheck.ico") }
}

function Get-RequiredVendorFiles {
    Get-Content -LiteralPath $VendorList | ForEach-Object { $_.Trim() } | Where-Object { $_ -and $_ -notlike "#*" }
}

function Test-VendorFiles {
    $missing = @()
    foreach ($rel in (Get-RequiredVendorFiles)) {
        $p = Join-Path $Root "assets\$rel"
        if (-not (Test-Path -LiteralPath $p)) { $missing += $rel }
    }
    return $missing
}

function Copy-VendorAssets($destAssets) {
    $missing = Test-VendorFiles
    if ($missing.Count -gt 0) {
        throw ("universal-local: missing vendor files in assets/:`n  " + ($missing -join "`n  ") + "`nDownload them once with a normal CubeCheck build (Components), then rebuild. Do not commit these files.")
    }
    foreach ($rel in (Get-RequiredVendorFiles)) {
        $src = Join-Path $Root "assets\$rel"
        $dst = Join-Path $destAssets $rel
        Copy-FileForce $src $dst
    }
    foreach ($opt in @("Everything.ini", "SystemInformer\SystemInformer.exe.settings.xml")) {
        $src = Join-Path $Root "assets\$opt"
        if (Test-Path -LiteralPath $src) {
            Copy-FileForce $src (Join-Path $destAssets $opt)
        }
    }
}

function Compress-Dir($folder, $archive) {
    if (Test-Path $archive) { Remove-Item -LiteralPath $archive -Force }
    $parent = Split-Path -Parent $folder
    $name = Split-Path -Leaf $folder
    Push-Location $parent
    try {
        & tar -a -c -f $archive $name
        if ($LASTEXITCODE -ne 0) { throw "tar failed for $archive" }
    } finally {
        Pop-Location
    }
}

function Write-Placeholder($dir, $kind) {
    if (-not (Test-Path $dir)) { New-Item -ItemType Directory -Path $dir | Out-Null }
    $txt = @"
CubeCheck payload '$kind' was not built on this machine.

On Windows this host can usually produce windows-x64 / windows-x86.
Linux/macOS native binaries need a matching linker/SDK, zig, cross, or a CI job.

  build.bat $kind
  build.sh $kind

The universal launcher will print a clear error if you start it on an OS
whose payload is missing.
"@
    Set-Content -LiteralPath (Join-Path $dir "README.txt") -Value $txt -Encoding utf8
}

function Build-LauncherWindows {
    Invoke-CargoBuild -CargoArgs @("build", "--release", "-p", "cubecheck-launcher") -FailMessage "launcher build failed"
    $src = Join-Path $Root "target\release\cubecheck-launcher.exe"
    if (-not (Test-Path $src)) {
        $src = Get-CargoBin $HostTriple "cubecheck-launcher"
    }
    if (-not (Test-Path $src)) { throw "cubecheck-launcher.exe not found after build" }
    return $src
}

function Stage-WindowsGui {
    param(
        [string]$Triple,
        [string]$DistName,
        [string]$OutExeName,
        [string]$TargetDir = "target",
        [switch]$Offline,
        [switch]$WithSetup
    )
    Ensure-RustupTarget $Triple
    $feat = @()
    if ($Offline) { $feat = @("--features", "offline") }
    $args = @("build", "--release", "-p", "cubecheck", "--bin", "cubecheck") + $feat
    if ($TargetDir -ne "target") { $args += @("--target-dir", $TargetDir) }
    if ($Triple -ne $HostTriple) { $args += @("--target", $Triple) }
    Invoke-CargoBuild -CargoArgs $args -FailMessage "cubecheck ($DistName) build failed"

    $src = Get-CargoBin $Triple "cubecheck" $TargetDir
    if (-not (Test-Path $src)) { throw "missing $src" }

    $outDir = Join-Path $Dist $DistName
    if (-not (Test-Path $outDir)) { New-Item -ItemType Directory -Path $outDir | Out-Null }
    $dest = Join-Path $outDir $OutExeName
    Copy-FileForce $src $dest
    Copy-CoreAssets (Join-Path $outDir "assets")

    if ($WithSetup) {
        $env:CUBECHECK_SETUP_PAYLOAD = (Resolve-Path -LiteralPath $src).Path
        $setupSrc = $null
        try {
            $setupArgs = @("build", "--release", "-p", "cubecheck", "--bin", "cubecheck-setup", "--no-default-features")
            if ($Triple -ne $HostTriple) { $setupArgs += @("--target", $Triple) }
            Invoke-CargoBuild -CargoArgs $setupArgs -FailMessage "cubecheck-setup ($DistName) native build failed"
            $setupSrc = Get-CargoBin $Triple "cubecheck-setup"
            if (-not (Test-Path -LiteralPath $setupSrc)) { throw "missing $setupSrc" }
        } catch {
            if ($Triple -eq $HostTriple) { throw }
            Write-Warn $_.Exception.Message
            Write-Info "Building host cubecheck-setup that embeds $DistName payload"
            $tdir = "target-setup-$DistName"
            $setupArgs = @(
                "build", "--release", "-p", "cubecheck", "--bin", "cubecheck-setup",
                "--no-default-features", "--target-dir", $tdir
            )
            Invoke-CargoBuild -CargoArgs $setupArgs -FailMessage "cubecheck-setup host-embed for $DistName failed"
            $setupSrc = Get-CargoBin $HostTriple "cubecheck-setup" $tdir
            if (-not (Test-Path -LiteralPath $setupSrc)) { throw "missing $setupSrc" }
        } finally {
            Remove-Item Env:\CUBECHECK_SETUP_PAYLOAD -ErrorAction SilentlyContinue
        }
        Copy-FileForce $setupSrc (Join-Path $outDir "CubeCheck-Setup.exe")
        Copy-FileForce $setupSrc (Join-Path $outDir "CubeCheck-Setup-$DistName.exe")
        if ($DistName -eq "windows-x64") {
            Copy-FileForce $setupSrc (Join-Path $Root "CubeCheck-Setup.exe")
            Copy-FileForce $src (Join-Path $Root "cubecheck.exe")
        }
    }

    return $dest
}

function Get-ZigExe {
    $cands = @()
    $cmd = Get-Command zig -ErrorAction SilentlyContinue
    if ($cmd) {
        $cands += $cmd.Source
        try {
            $item = Get-Item -LiteralPath $cmd.Source
            if ($item.LinkType -and $item.Target) { $cands += [string]$item.Target }
        } catch {}
    }
    $cands += @(
        "C:\JumpWorld\tools\zig-copy\zig.exe",
        "C:\JumpWorld\tools\zig\zig.exe"
    )
    $ascii = $cands | Where-Object { $_ -and (Test-Path -LiteralPath $_) -and ($_ -notmatch '[^\x00-\x7F]') } | Select-Object -First 1
    if ($ascii) { return $ascii }
    return ($cands | Where-Object { $_ -and (Test-Path -LiteralPath $_) } | Select-Object -First 1)
}

function Find-LinuxLinker($triple) {
    # cargo-zigbuild Windows .bat wrappers break with non-ASCII user paths and
    # `-fno-sanitize=all`. Prefer a tiny ASCII zig-cc.cmd when zig.exe exists.
    if (Get-ZigExe) { return "zig" }
    if (Get-Command cargo-zigbuild -ErrorAction SilentlyContinue) { return "zigbuild" }
    if (Get-Command cross -ErrorAction SilentlyContinue) { return "cross" }
    return $null
}

function Get-ZigCcTarget($triple) {
    switch ($triple) {
        "x86_64-unknown-linux-gnu" { "x86_64-linux-gnu.2.17" }
        "i686-unknown-linux-gnu" { "x86-linux-gnu.2.17" }
        "x86_64-apple-darwin" { "x86_64-macos" }
        "aarch64-apple-darwin" { "aarch64-macos" }
        default { $null }
    }
}

function Find-AppleSdk {
    $cands = @()
    foreach ($e in @("SDKROOT", "OSX_SDK", "OSXCROSS_SDK")) {
        $v = [Environment]::GetEnvironmentVariable($e)
        if ($v) { $cands += $v }
    }
    $cands += @(
        "C:\osxcross\SDK",
        "C:\JumpWorld\tools\osxcross\SDK",
        "C:\JumpWorld\osxcross\SDK"
    )
    foreach ($root in $cands) {
        if (-not $root -or -not (Test-Path -LiteralPath $root)) { continue }
        $use = $root
        $item = Get-Item -LiteralPath $root
        if ($item.PSIsContainer) {
            $sdk = Get-ChildItem -LiteralPath $root -Directory -Filter "MacOSX*.sdk" -ErrorAction SilentlyContinue | Select-Object -First 1
            if ($sdk) { $use = $sdk.FullName }
        }
        $lib = Join-Path $use "usr\lib"
        if ((Test-Path -LiteralPath (Join-Path $lib "libSystem.tbd")) -or (Test-Path -LiteralPath (Join-Path $lib "libSystem.dylib"))) {
            return $use
        }
    }
    return $null
}

function Ensure-ZigFilter {
    $wrapDir = Join-Path $Root ".zig-wrappers"
    if (-not (Test-Path $wrapDir)) { New-Item -ItemType Directory -Path $wrapDir | Out-Null }
    $src = Join-Path $Root "scripts\zig-cc-filter.rs"
    $exe = Join-Path $wrapDir "zig-cc-filter.exe"
    $need = -not (Test-Path -LiteralPath $exe)
    if (-not $need) {
        $need = (Get-Item -LiteralPath $src).LastWriteTimeUtc -gt (Get-Item -LiteralPath $exe).LastWriteTimeUtc
    }
    if ($need) {
        Write-Info "rustc -O zig-cc-filter.rs"
        & rustc -O -o $exe $src
        if ($LASTEXITCODE -ne 0 -or -not (Test-Path -LiteralPath $exe)) {
            throw "failed to build .zig-wrappers/zig-cc-filter.exe (host rustc)"
        }
    }
    return $exe
}

function New-ZigWrappers([string]$triple) {
    $zig = Get-ZigExe
    $zt = Get-ZigCcTarget $triple
    if (-not $zig -or -not $zt) { return $null }
    $filter = Ensure-ZigFilter
    $wrapDir = Join-Path $Root ".zig-wrappers"
    if (-not (Test-Path $wrapDir)) { New-Item -ItemType Directory -Path $wrapDir | Out-Null }
    $tag = $triple -replace '[^a-z0-9]', '_'
    $sdk = $null
    if ($triple -like "*apple*") {
        $sdk = Find-AppleSdk
    }
    $cc = Join-Path $wrapDir "zig-cc-$tag.cmd"
    $ar = Join-Path $wrapDir "zig-ar-$tag.cmd"
    $ccLine = "@echo off`r`n`"$filter`" `"$zig`" cc $zt %*"
    if ($sdk) {
        $ccLine = "@echo off`r`n`"$filter`" `"$zig`" cc $zt -isysroot `"$sdk`" %*"
    }
    Set-Content -LiteralPath $cc -Value $ccLine -Encoding ascii
    Set-Content -LiteralPath $ar -Value "@echo off`r`n`"$filter`" `"$zig`" ar %*" -Encoding ascii
    return [pscustomobject]@{ Cc = $cc; Ar = $ar; Zig = $zig; ZigTarget = $zt; Sdk = $sdk }
}

function Set-CrossCcEnv([string]$triple, $wraps) {
    $lower = $triple -replace '-', '_'
    $upper = $lower.ToUpperInvariant()
    Set-Item -Path "Env:CC_$lower" -Value $wraps.Cc
    Set-Item -Path "Env:CXX_$lower" -Value $wraps.Cc
    Set-Item -Path "Env:AR_$lower" -Value $wraps.Ar
    Set-Item -Path "Env:CC_$upper" -Value $wraps.Cc
    Set-Item -Path "Env:CXX_$upper" -Value $wraps.Cc
    Set-Item -Path "Env:AR_$upper" -Value $wraps.Ar
    Set-Item -Path "Env:CARGO_TARGET_${upper}_LINKER" -Value $wraps.Cc
}

function Invoke-CargoLogged([string[]]$Cmd, [string]$LogPath) {
    Write-Info ($Cmd -join " ")
    $dir = Split-Path -Parent $LogPath
    if (-not (Test-Path $dir)) { New-Item -ItemType Directory -Path $dir | Out-Null }
    $prevEa = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    $sw = New-Object IO.StreamWriter $LogPath, $false, ([Text.Encoding]::UTF8)
    try {
        & $Cmd[0] @($Cmd | Select-Object -Skip 1) 2>&1 | ForEach-Object {
            $line = if ($_ -is [System.Management.Automation.ErrorRecord]) { $_.ToString() } else { "$_" }
            Write-Host $line
            $sw.WriteLine($line)
        }
        return $LASTEXITCODE
    } finally {
        $sw.Dispose()
        $ErrorActionPreference = $prevEa
    }
}

function Build-UnixGui {
    param(
        [string]$Triple,
        [string]$DistName
    )
    try { Ensure-RustupTarget $Triple } catch {
        throw "Cannot install target $Triple : $($_.Exception.Message)"
    }

    $outDir = Join-Path $Dist $DistName
    if (-not (Test-Path $outDir)) { New-Item -ItemType Directory -Path $outDir | Out-Null }
    $logPath = Join-Path $outDir "compile.log"
    $errors = @()
    $env:PKG_CONFIG_ALLOW_CROSS = "1"

    $cargoArgs = @("build", "--release", "-p", "cubecheck", "--bin", "cubecheck", "--target", $Triple)
    $ok = $false

    $wraps = New-ZigWrappers $Triple
    if ($wraps) {
        Set-CrossCcEnv $Triple $wraps
        Write-Info "zig cc $($wraps.ZigTarget) via $($wraps.Zig)"
        $st = Invoke-CargoLogged -Cmd (@("cargo") + $cargoArgs) -LogPath $logPath
        if ($st -eq 0) { $ok = $true }
        else { $errors += "zig cc ($($wraps.ZigTarget)): cargo exit $st" }
    }

    if (-not $ok -and -not $wraps -and (Get-Command cargo-zigbuild -ErrorAction SilentlyContinue)) {
        $st = Invoke-CargoLogged -Cmd @("cargo", "zigbuild", "--release", "-p", "cubecheck", "--bin", "cubecheck", "--target", $Triple) -LogPath $logPath
        if ($st -eq 0) { $ok = $true }
        else { $errors += "cargo zigbuild: exit $st" }
    }

    if (-not $ok -and (Get-Command cross -ErrorAction SilentlyContinue)) {
        $st = Invoke-CargoLogged -Cmd @("cross", "build", "--release", "-p", "cubecheck", "--bin", "cubecheck", "--target", $Triple) -LogPath $logPath
        if ($st -eq 0) { $ok = $true }
        else { $errors += "cross: exit $st" }
    }

    if (-not $ok -and -not $wraps) {
        $st = Invoke-CargoLogged -Cmd (@("cargo") + $cargoArgs) -LogPath $logPath
        if ($st -eq 0) { $ok = $true }
        else { $errors += "cargo: exit $st" }
    }

    $src = Join-Path $Root "target\$Triple\release\cubecheck"
    $isDarwin = $Triple -like "*apple*"
    $good = $false
    if ($ok -and (Test-Path -LiteralPath $src)) {
        $good = if ($isDarwin) { Test-MachO $src } else { Test-Elf $src }
    }

    if (-not $good) {
        $tail = ""
        if (Test-Path -LiteralPath $logPath) {
            $lines = Get-Content -LiteralPath $logPath -ErrorAction SilentlyContinue
            if ($lines) { $tail = ($lines | Select-Object -Last 40) -join "`n" }
        }
        $sdkHint = ""
        if ($isDarwin) {
            $sdk = Find-AppleSdk
            if ($sdk) {
                $sdkHint = "Apple SDK found at $sdk but the link still failed."
            } else {
                $sdkHint = @"
No Apple SDK/sysroot on this machine (SDKROOT / osxcross not set).
A real Mach-O GUI needs Apple's libSystem + AppKit/CoreFoundation — do not download Xcode SDKs from random mirrors.
On a Mac:
  rustup target add aarch64-apple-darwin x86_64-apple-darwin
  cargo build --release --bin cubecheck --target aarch64-apple-darwin
  cargo build --release --bin cubecheck --target x86_64-apple-darwin
  ./build.sh macos-universal
"@
            }
        } else {
            $sdkHint = @"
Linux ELF was not produced. Need zig cc (zstd-sys / ring) or a GNU toolchain.
TLS is rustls (no OpenSSL). Install zig + rustup target, then: build.bat $DistName
"@
        }
        throw @"
Failed to compile $Triple ($DistName).

$($errors -join "`n")

$sdkHint

Last compiler lines:
$tail
"@
    }

    $destName = "cubecheck-$DistName"
    Copy-FileForce $src (Join-Path $outDir $destName)
    Copy-CoreAssets (Join-Path $outDir "assets")
    return (Join-Path $outDir $destName)
}

function Write-DebTree($archDeb, $outDir, $linuxBin) {
    $pkg = Join-Path $outDir "pkg"
    if (Test-Path $pkg) { Remove-Item -Recurse -Force $pkg }
    New-Item -ItemType Directory -Path (Join-Path $pkg "DEBIAN") | Out-Null
    New-Item -ItemType Directory -Path (Join-Path $pkg "usr\bin") | Out-Null
    New-Item -ItemType Directory -Path (Join-Path $pkg "usr\share\cubecheck\assets") | Out-Null
    New-Item -ItemType Directory -Path (Join-Path $pkg "usr\share\doc\cubecheck") | Out-Null
    New-Item -ItemType Directory -Path (Join-Path $pkg "usr\share\applications") | Out-Null

    $control = Get-Content (Join-Path $Root "scripts\debian\control.in") -Raw
    $control = $control.Replace("@VERSION@", $Version).Replace("@ARCH@", $archDeb)
    Set-Content -LiteralPath (Join-Path $pkg "DEBIAN\control") -Value $control -Encoding ascii
    Copy-FileForce (Join-Path $Root "scripts\debian\copyright") (Join-Path $pkg "usr\share\doc\cubecheck\copyright")
    Copy-FileForce (Join-Path $Root "LICENSE.md") (Join-Path $pkg "usr\share\doc\cubecheck\LICENSE.md")
    Copy-CoreAssets (Join-Path $pkg "usr\share\cubecheck\assets")
    $desktop = @"
[Desktop Entry]
Type=Application
Name=CubeCheck
Comment=Minecraft cheat-name checker
Exec=cubecheck
Terminal=false
Categories=Utility;
"@
    Set-Content -LiteralPath (Join-Path $pkg "usr\share\applications\cubecheck.desktop") -Value $desktop -Encoding ascii

    $readme = @"
Debian package tree for CubeCheck $Version ($archDeb).

On Linux:
  chmod +x scripts/pack-deb.sh
  scripts/pack-deb.sh `"$pkg`" `"$(Join-Path $outDir "cubecheck_${Version}_$archDeb.deb")`"

If usr/bin/cubecheck is missing or not a real ELF, this host could not produce Linux:

  rustup target add x86_64-unknown-linux-gnu i686-unknown-linux-gnu
  build.bat linux-deb-x64
  or: cargo zigbuild --release --bin cubecheck --target x86_64-unknown-linux-gnu

TLS is rustls (no OpenSSL sysroot). GitHub Release Linux asset is the .sh launcher, not a dummy tarball.
"@
    Set-Content -LiteralPath (Join-Path $outDir "README.txt") -Value $readme -Encoding utf8
    Copy-FileForce (Join-Path $Root "scripts\pack-deb.sh") (Join-Path $outDir "pack-deb.sh")

    if ($linuxBin -and (Test-Elf $linuxBin)) {
        Copy-FileForce $linuxBin (Join-Path $pkg "usr\bin\cubecheck")
    }
    return $pkg
}

function Try-PackDeb($pkg, $debPath) {
    $bin = Join-Path $pkg "usr\bin\cubecheck"
    if (-not (Test-Elf $bin)) { return $false }
    $bash = Get-Command bash -ErrorAction SilentlyContinue
    if ($bash) {
        & bash (Join-Path $Root "scripts\pack-deb.sh") $pkg $debPath
        return ($LASTEXITCODE -eq 0 -and (Test-Path $debPath))
    }
    if (Get-Command dpkg-deb -ErrorAction SilentlyContinue) {
        & dpkg-deb --build $pkg $debPath
        return ($LASTEXITCODE -eq 0)
    }
    return $false
}

function Get-BundleReadme($local) {
    $extra = if ($local) {
        @"

universal-local is OFFLINE. The app will not download anything.
Windows payloads include third-party binaries copied from assets/ at pack time.

Those third-party files keep THEIR OWN licenses (voidtools Everything,
Microsoft Sysinternals, System Informer, PrivaZer/Goversoft Shellbag).
CubeCheck MIT does NOT cover them. Redistribute only if you have the right
to do so; you may need to host/build this pack yourself.
"@
    } else {
        @"

This SKU may download Everything / Sysinternals / etc. from official HTTPS
URLs the first time you use Components (Windows only).
"@
    }
    return @"
CubeCheck universal
===================

Run cubecheck.exe (Windows), cubecheck.sh (Linux), or cubecheck.command (macOS).
The launcher detects OS/arch and starts payload/<id>/cubecheck.

Supported when the matching payload exists:
  Windows 7 / 10 / 11   payload/windows-x64 or windows-x86
  Linux                 payload/linux-x64 or linux-x86
  macOS                 payload/macos-universal

If a payload is missing, the launcher exits with an error (not a silent crash).

Portable: settings and reports stay next to the payload binary.
$extra
CubeCheck source: MIT (LICENSE.md).
"@
}

function Stage-PayloadFromDist($bundlePayload, $id, $distName, $binFile) {
    $dest = Join-Path $bundlePayload $id
    $srcBin = Join-Path (Join-Path $Dist $distName) $binFile
    $ok = $false
    if ($id -like "linux-*") { $ok = Test-Elf $srcBin }
    elseif ($id -like "macos-*") { $ok = Test-MachO $srcBin }
    else { $ok = Test-Path $srcBin }
    if ($ok) {
        if (-not (Test-Path $dest)) { New-Item -ItemType Directory -Path $dest | Out-Null }
        $leaf = if ($id -like "windows-*") { "cubecheck.exe" } else { "cubecheck" }
        Copy-FileForce $srcBin (Join-Path $dest $leaf)
        Copy-CoreAssets (Join-Path $dest "assets")
        Write-Marker (Join-Path $dest ".portable") ""
        return $true
    }
    Write-Placeholder $dest $id
    return $false
}

function Pack-Universal($local) {
    $name = if ($local) { "universal-local" } else { "universal" }
    $folderName = if ($local) { "CubeCheck-universal-local" } else { "CubeCheck-universal" }
    $outRoot = Join-Path $Dist $name
    $bundle = Join-Path $outRoot $folderName
    if (Test-Path $bundle) { Remove-Item -Recurse -Force $bundle }
    New-Item -ItemType Directory -Path $bundle | Out-Null
    $payload = Join-Path $bundle "payload"
    New-Item -ItemType Directory -Path $payload | Out-Null

    $launcher = Build-LauncherWindows
    Copy-FileForce $launcher (Join-Path $bundle "cubecheck.exe")
    Copy-FileForce (Join-Path $Root "scripts\posix-launcher.sh") (Join-Path $bundle "cubecheck.sh")
    Copy-FileForce (Join-Path $Root "scripts\cubecheck.command") (Join-Path $bundle "cubecheck.command")
    Copy-FileForce (Join-Path $Root "LICENSE.md") (Join-Path $bundle "LICENSE.md")
    Set-Content -LiteralPath (Join-Path $bundle "README.txt") -Value (Get-BundleReadme $local) -Encoding utf8

    if ($local) {
        Write-Marker (Join-Path $bundle ".offline") "CUBECHECK_OFFLINE=1`n"
    }

    $wx64 = if ($local) {
        Stage-WindowsGui -Triple "x86_64-pc-windows-msvc" -DistName "universal-local-build\windows-x64" -OutExeName "cubecheck-windows-x64.exe" -TargetDir "target-offline" -Offline
        Join-Path $Dist "universal-local-build\windows-x64\cubecheck-windows-x64.exe"
    } else {
        Join-Path $Dist "windows-x64\cubecheck-windows-x64.exe"
    }
    $wx86 = if ($local) {
        try {
            [void](Stage-WindowsGui -Triple "i686-pc-windows-msvc" -DistName "universal-local-build\windows-x86" -OutExeName "cubecheck-windows-x86.exe" -TargetDir "target-offline" -Offline)
            Join-Path $Dist "universal-local-build\windows-x86\cubecheck-windows-x86.exe"
        } catch {
            Write-Warn $_.Exception.Message
            $null
        }
    } else {
        Join-Path $Dist "windows-x86\cubecheck-windows-x86.exe"
    }

    $have = 0
    function Put-Win($id, $src) {
        if ($src -and (Test-Path $src)) {
            $dest = Join-Path $payload $id
            New-Item -ItemType Directory -Path $dest -Force | Out-Null
            Copy-FileForce $src (Join-Path $dest "cubecheck.exe")
            Copy-CoreAssets (Join-Path $dest "assets")
            Write-Marker (Join-Path $dest ".portable") ""
            if ($local) {
                Write-Marker (Join-Path $dest ".offline") ""
                Copy-VendorAssets (Join-Path $dest "assets")
            }
            return 1
        }
        Write-Placeholder (Join-Path $payload $id) $id
        return 0
    }

    $have += Put-Win "windows-x64" $wx64
    $have += Put-Win "windows-x86" $wx86

    foreach ($pair in @(
        @{ id = "linux-x64"; dist = "linux-deb-x64"; bin = "cubecheck-linux-deb-x64" },
        @{ id = "linux-x86"; dist = "linux-deb-x86"; bin = "cubecheck-linux-deb-x86" }
    )) {
        $ok = Stage-PayloadFromDist $payload $pair.id $pair.dist $pair.bin
        if ($ok) { $have += 1 }
        if ($ok -and $local) {
            Write-Marker (Join-Path $payload "$($pair.id)\.offline") ""
        }
    }

    $macBin = Join-Path $Dist "macos-universal\cubecheck-macos-universal"
    $macDir = Join-Path $payload "macos-universal"
    if (Test-MachO $macBin) {
        New-Item -ItemType Directory -Path $macDir -Force | Out-Null
        Copy-FileForce $macBin (Join-Path $macDir "cubecheck")
        Copy-CoreAssets (Join-Path $macDir "assets")
        Write-Marker (Join-Path $macDir ".portable") ""
        if ($local) { Write-Marker (Join-Path $macDir ".offline") "" }
        $have += 1
    } else {
        Write-Placeholder $macDir "macos-universal"
    }

    if ($have -lt 1) { throw "${name}: no payload binaries to pack" }

    $zip = Join-Path $outRoot "$folderName.zip"
    Compress-Dir $bundle $zip
    return $bundle
}

function Pack-LinuxUniversal {
    $outRoot = Join-Path $Dist "linux-universal"
    $bundle = Join-Path $outRoot "CubeCheck-linux-universal"
    if (Test-Path $bundle) { Remove-Item -Recurse -Force $bundle }
    New-Item -ItemType Directory -Path $bundle | Out-Null
    Copy-FileForce (Join-Path $Root "scripts\posix-launcher.sh") (Join-Path $bundle "cubecheck")
    Copy-FileForce (Join-Path $Root "scripts\posix-launcher.sh") (Join-Path $bundle "cubecheck.sh")
    Copy-FileForce (Join-Path $Root "LICENSE.md") (Join-Path $bundle "LICENSE.md")
    Set-Content -LiteralPath (Join-Path $bundle "README.txt") -Value @"
CubeCheck linux-universal (distro-agnostic).
The GitHub Release asset is CubeCheck-$Version-linux-universal.sh
(chmod +x and run it). This folder is a staging layout.
"@ -Encoding utf8

    $payload = Join-Path $bundle "payload"
    $have = 0
    if (Stage-PayloadFromDist $payload "linux-x64" "linux-deb-x64" "cubecheck-linux-deb-x64") { $have++ }
    if (Stage-PayloadFromDist $payload "linux-x86" "linux-deb-x86" "cubecheck-linux-deb-x86") { $have++ }
    $sh = Join-Path $outRoot (Get-ReleaseFileName "linux-universal.sh")
    if (Test-Path $sh) { Remove-Item $sh -Force }
    if ($have -lt 1) {
        Write-SkipReadme $outRoot "linux-universal" @"
No Linux cubecheck ELF was produced on this host.
Do not ship a dummy .sh. Build linux-deb-x64 / linux-deb-x86 first
(zig cc / cargo-zigbuild / a Linux machine), then rebuild this target.
This layout stays in dist/; it is not copied to build/ (GitHub Releases).
"@
        throw "linux-universal: skipped — no Linux ELF on this host. Layout written to dist/linux-universal."
    }

    $stage = Join-Path $outRoot "sh-stage"
    if (Test-Path $stage) { Remove-Item $stage -Recurse -Force }
    New-Item -ItemType Directory -Path $stage | Out-Null
    Copy-Tree $payload (Join-Path $stage "payload")
    Copy-CoreAssets (Join-Path $stage "assets")
    Copy-FileForce (Join-Path $Root "LICENSE.md") (Join-Path $stage "LICENSE.md")
    [void](Pack-LinuxShFromStage $stage $sh "linux-universal")
    return $bundle
}

function Invoke-One($name) {
    try {
    switch ($name) {
        "windows-x64" {
            $p = Stage-WindowsGui -Triple "x86_64-pc-windows-msvc" -DistName "windows-x64" -OutExeName "cubecheck-windows-x64.exe" -WithSetup
            Record-Result $name "OK" $p
        }
        "windows-x86" {
            $p = Stage-WindowsGui -Triple "i686-pc-windows-msvc" -DistName "windows-x86" -OutExeName "cubecheck-windows-x86.exe" -WithSetup
            Record-Result $name "OK" $p
        }
        "linux-deb-x64" {
            $bin = $null
            try { $bin = Build-UnixGui -Triple "x86_64-unknown-linux-gnu" -DistName "linux-deb-x64" } catch { Write-Warn $_.Exception.Message }
            $pkg = Write-DebTree "amd64" (Join-Path $Dist "linux-deb-x64") $bin
            $deb = Join-Path (Join-Path $Dist "linux-deb-x64") "cubecheck_${Version}_amd64.deb"
            $packed = Try-PackDeb $pkg $deb
            if (-not $bin -or -not (Test-Elf $bin)) {
                Record-Result $name "FAIL" "Linux ELF not built; Debian tree at $pkg"
                throw "linux-deb-x64: no Linux ELF on this host. Debian layout written to dist/linux-deb-x64/pkg. See compile.log. Pack on Linux with scripts/pack-deb.sh."
            }
            $sh = Pack-LinuxShSingle $bin (Join-Path $Dist "linux-deb-x64\$(Get-ReleaseFileName 'linux-x64.sh')") "linux-x64"
            $detail = $sh
            if ($packed) { $detail = "$sh ; $deb" }
            Record-Result $name "OK" $detail
        }
        "linux-deb-x86" {
            $bin = $null
            try { $bin = Build-UnixGui -Triple "i686-unknown-linux-gnu" -DistName "linux-deb-x86" } catch { Write-Warn $_.Exception.Message }
            $pkg = Write-DebTree "i386" (Join-Path $Dist "linux-deb-x86") $bin
            $deb = Join-Path (Join-Path $Dist "linux-deb-x86") "cubecheck_${Version}_i386.deb"
            $packed = Try-PackDeb $pkg $deb
            if (-not $bin -or -not (Test-Elf $bin)) {
                Record-Result $name "FAIL" "Linux i686 ELF not built; Debian tree at $pkg"
                throw "linux-deb-x86: no Linux ELF on this host. Debian layout written to dist/linux-deb-x86/pkg. See compile.log."
            }
            $sh = Pack-LinuxShSingle $bin (Join-Path $Dist "linux-deb-x86\$(Get-ReleaseFileName 'linux-x86.sh')") "linux-x86"
            $detail = $sh
            if ($packed) { $detail = "$sh ; $deb" }
            Record-Result $name "OK" $detail
        }
        "linux-universal" {
            $p = Pack-LinuxUniversal
            Record-Result $name "OK" $p
        }
        "macos-universal" {
            $out = Join-Path $Dist "macos-universal"
            if (-not (Test-Path $out)) { New-Item -ItemType Directory -Path $out | Out-Null }
            $x64 = $null
            $arm = $null
            try { $x64 = Build-UnixGui -Triple "x86_64-apple-darwin" -DistName "macos-x64" } catch { Write-Warn $_.Exception.Message }
            try { $arm = Build-UnixGui -Triple "aarch64-apple-darwin" -DistName "macos-arm64" } catch { Write-Warn $_.Exception.Message }
            $slices = @()
            if (Test-MachO $x64) { $slices += $x64 }
            if (Test-MachO $arm) { $slices += $arm }
            if ($slices.Count -eq 0) {
                Write-Placeholder (Join-Path $out "payload") "macos-universal"
                $x64Log = Join-Path $Dist "macos-x64\compile.log"
                $armLog = Join-Path $Dist "macos-arm64\compile.log"
                $hint = @"
macOS Mach-O was not produced on this host (linker/Apple SDK).

Do not download an Apple SDK from random mirrors. On a Mac (or with a legal SDK / osxcross):

  rustup target add x86_64-apple-darwin aarch64-apple-darwin
  cargo build --release --bin cubecheck --target x86_64-apple-darwin
  cargo build --release --bin cubecheck --target aarch64-apple-darwin
  lipo -create -output dist/macos-universal/cubecheck-macos-universal \
    target/x86_64-apple-darwin/release/cubecheck \
    target/aarch64-apple-darwin/release/cubecheck
  ./build.sh macos-universal

That writes build/CubeCheck-$Version-macos-universal.zip containing extensionless Mach-O ``cubecheck``.
chmod +x cubecheck before running.

See dist/macos-x64/compile.log and dist/macos-arm64/compile.log for the exact linker error.

Typical Windows-host failure after rustc+ring compile:

  xcrun --sdk macosx --show-sdk-path : program not found
  unable to find dynamic system library 'objc' / 'iconv'
  needs -framework AppKit CoreFoundation IOKit OpenGL Foundation …

No MacOSX.sdk. Do not download Xcode SDKs from random mirrors.

"@
                Set-Content -LiteralPath (Join-Path $out "README.txt") -Value $hint -Encoding utf8
                Record-Result $name "FAIL" "skipped: no Apple SDK/linker on this host"
                throw "macos-universal: cannot produce a real Mach-O on this Windows PC. Recipe written to dist/macos-universal/README.txt"
            }
            $dest = Join-Path $out "cubecheck-macos-universal"
            if ($slices.Count -eq 2 -and (Get-Command lipo -ErrorAction SilentlyContinue)) {
                & lipo -create -output $dest $slices
            } else {
                $prefer = if (Test-MachO $arm) { $arm } else { $slices[0] }
                Copy-FileForce $prefer $dest
                if ($slices.Count -eq 2) {
                    Write-Warn "lipo not found; zip will include cubecheck plus cubecheck-arm64 and cubecheck-x64 slices."
                }
            }
            if (-not (Test-MachO $dest)) {
                throw "macos-universal: output is not Mach-O ($dest)"
            }
            Record-Result $name "OK" $dest
        }
        "universal" {
            if (-not (Test-Path (Join-Path $Dist "windows-x64\cubecheck-windows-x64.exe"))) {
                Invoke-One "windows-x64"
            }
            if (-not (Test-Path (Join-Path $Dist "windows-x86\cubecheck-windows-x86.exe"))) {
                try { Invoke-One "windows-x86" } catch { Write-Warn $_.Exception.Message }
            }
            $p = Pack-Universal $false
            Record-Result $name "OK" $p
        }
        "universal-local" {
            $missing = Test-VendorFiles
            if ($missing.Count -gt 0) {
                Record-Result $name "FAIL" ("missing " + ($missing -join ", "))
                throw ("universal-local: missing vendor files in assets/:`n  " + ($missing -join "`n  "))
            }
            $p = Pack-Universal $true
            Record-Result $name "OK" $p
        }
        default { throw "Unknown target '$name'. Try: build.bat help" }
    }
    } finally {
        try { Publish-ReleaseAssets } catch { Write-Warn "publish: $($_.Exception.Message)" }
    }
}

function Show-Help {
    @"
CubeCheck build

  build.bat                 windows-x64 (default) + setup
  build.bat all             try every artifact
  build.bat publish         rebuild build/ from dist/ (no compile)
  build.bat windows-x64
  build.bat windows-x86
  build.bat linux-deb-x64
  build.bat linux-deb-x86
  build.bat linux-universal
  build.bat macos-universal
  build.bat universal
  build.bat universal-local

GitHub Release assets go to build/ (flat files only):
  CubeCheck-<ver>-windows-x64-setup.exe
  CubeCheck-<ver>-windows-x64.zip
  CubeCheck-<ver>-windows-x86-setup.exe
  CubeCheck-<ver>-windows-x86.zip
  CubeCheck-<ver>-universal.zip
  CubeCheck-<ver>-universal-local.zip
  CubeCheck-Setup.exe                      (alias of the x64 installer)
  CubeCheck-<ver>-linux-x64.sh             (self-extracting; only if ELF exists)
  CubeCheck-<ver>-linux-x86.sh
  CubeCheck-<ver>-linux-universal.sh
  CubeCheck-<ver>-linux-deb-x64.deb        (only if usr/bin/cubecheck is ELF)
  CubeCheck-<ver>-macos-universal.zip      (inner file: Mach-O cubecheck)

Trees, junctions, README placeholders, and Debian pkg/ stay in dist/.
universal-local never downloads; vendor files must already be in assets/.
"@
}

$env:Path = [System.Environment]::GetEnvironmentVariable("Path", "Machine") + ";" + [System.Environment]::GetEnvironmentVariable("Path", "User")
$cargoBin = Join-Path $env:USERPROFILE ".cargo\bin"
if (Test-Path $cargoBin) { $env:Path = "$cargoBin;$env:Path" }
$zigExe = Get-ZigExe
if ($zigExe) { $env:Path = "$(Split-Path -Parent $zigExe);$env:Path" }

New-Item -ItemType Directory -Path $Dist -Force | Out-Null
New-Item -ItemType Directory -Path $BuildOut -Force | Out-Null

$t = $Target.Trim().ToLowerInvariant()
if ($t -in @("help", "-h", "--help", "/?")) {
    Show-Help
    exit 0
}

Write-Host "========================================"
Write-Host " CubeCheck  ($Version)"
Write-Host "========================================"
Write-Host ""

if ($t -eq "publish") {
    Write-Host "Publishing dist/ -> build/ (GitHub Release assets) ..."
    Publish-ReleaseAssets
    Show-ReleaseBuild
    Write-Host "Done."
    exit 0
}

Ensure-Cargo

$allTargets = @(
    "windows-x64", "windows-x86",
    "linux-deb-x64", "linux-deb-x86", "linux-universal",
    "macos-universal",
    "universal", "universal-local"
)

Write-Host "Publishing dist/ -> build/ (GitHub Release assets) ..."
Publish-ReleaseAssets
Write-Host ""

try {
    if ($t -eq "all") {
        foreach ($item in $allTargets) {
            Write-Host ""
            Write-Host "---- $item ----" -ForegroundColor Cyan
            try { Invoke-One $item }
            catch {
                Write-Err $_.Exception.Message
                if (-not ($script:Results | Where-Object { $_.Name -eq $item })) {
                    Record-Result $item "FAIL" $_.Exception.Message
                }
            }
        }
    } else {
        Invoke-One $t
    }
} catch {
    Write-Err $_.Exception.Message
    if (-not ($script:Results | Where-Object { $_.Name -eq $t })) {
        Record-Result $t "FAIL" $_.Exception.Message
    }
    Show-ReleaseBuild
    Write-Host ""
    Write-Host "Failed."
    exit 1
}

Write-Host ""
Write-Host "======== SUMMARY ========"
foreach ($r in $script:Results) {
    $color = if ($r.Status -eq "OK") { "Green" } else { "Red" }
    Write-Host ("{0,-20} {1,-6} {2}" -f $r.Name, $r.Status, $r.Detail) -ForegroundColor $color
}

Show-ReleaseBuild

$failed = @($script:Results | Where-Object { $_.Status -ne "OK" })
if ($t -ne "all" -and $failed.Count -gt 0) { exit 1 }
if ($t -eq "all") {
    $win = $script:Results | Where-Object { $_.Name -eq "windows-x64" -and $_.Status -eq "OK" }
    if (-not $win) { exit 1 }
    Write-Host ""
    Write-Host "all: finished. Non-Windows targets may be FAIL/skip on this host -- see above."
}
Write-Host "Done."
exit 0
