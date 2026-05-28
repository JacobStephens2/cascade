# Cross-compile cascade-uniffi for the two Windows architectures and copy
# the resulting DLLs into the C# project's Native/ folder so the .csproj's
# `Content` includes pick them up.
#
# Run from `apps/windows/`:
#     pwsh ./scripts/build-rust.ps1
#
# Requires:
#   - cargo on PATH
#   - rustup target add x86_64-pc-windows-msvc aarch64-pc-windows-msvc
#   - the MSVC build tools (Visual Studio 2022 C++ build tools)

$ErrorActionPreference = 'Stop'
Set-Location $PSScriptRoot/..
$repoRoot = (Resolve-Path ../..).Path

$targets = @(
    @{ Triple = 'x86_64-pc-windows-msvc';  OutDir = 'Cascade/Native/x64' },
    @{ Triple = 'aarch64-pc-windows-msvc'; OutDir = 'Cascade/Native/arm64' }
)

foreach ($t in $targets) {
    Write-Host "-> cargo build --release --target $($t.Triple)"
    Push-Location $repoRoot
    try {
        & cargo build --release --target $t.Triple -p cascade-uniffi
        if ($LASTEXITCODE -ne 0) { throw "cargo build failed for $($t.Triple)" }
    } finally { Pop-Location }

    $dll = Join-Path $repoRoot "target/$($t.Triple)/release/cascade_uniffi.dll"
    if (-not (Test-Path $dll)) { throw "expected DLL not found: $dll" }

    New-Item -ItemType Directory -Force -Path $t.OutDir | Out-Null
    Copy-Item -Force $dll (Join-Path $t.OutDir 'cascade_uniffi.dll')
    Write-Host "   wrote $(Join-Path $t.OutDir 'cascade_uniffi.dll')"
}

Write-Host ""
Write-Host "Done. Open Cascade.sln and build (x64 or arm64)."
