# One-shot: convert the asset, cross-compile the Rust core, copy DLLs into
# the C# project. After this, open Cascade.sln in Visual Studio (or run
# `dotnet build`).
#
# Run from `apps/windows/`:
#     pwsh ./scripts/build.ps1      # PowerShell 7
#     powershell ./scripts/build.ps1  # Windows PowerShell 5.1 also works

$ErrorActionPreference = 'Stop'
Set-Location $PSScriptRoot/..

# Run the sub-scripts in the current PowerShell host rather than shelling out
# to `pwsh`, so the build works on stock Windows (Windows PowerShell 5.1)
# without PowerShell 7 installed.
& "$PSScriptRoot/build-asset.ps1"
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

& "$PSScriptRoot/build-rust.ps1"
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

Write-Host ""
Write-Host "Next:"
Write-Host "  - Open apps/windows/Cascade.sln in Visual Studio 2022, set the"
Write-Host "    active configuration to Debug | x64 (or arm64), and F5."
Write-Host "  - Or from CLI:  dotnet build Cascade/Cascade.csproj -c Debug /p:Platform=x64"
