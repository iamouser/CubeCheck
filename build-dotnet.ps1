# Compatibility wrapper — canonical build is scripts\build.ps1
param(
    [Parameter(Position = 0)]
    [string]$Target = "release"
)
& "$PSScriptRoot\scripts\build.ps1" $Target
exit $LASTEXITCODE
