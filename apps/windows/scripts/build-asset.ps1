# Convert the canonical waterfall.ogg into waterfall.mp3 for the Windows app.
#
# Windows' MediaFoundation doesn't include an OGG Vorbis decoder by default,
# so MediaPlayer cannot play the .ogg straight. AAC and MP3 are both built
# in; MP3 is universally supported. Encoder padding is harmless here —
# MediaPlaybackList.AutoRepeatEnabled = true handles the loop seam.
#
# Run from `apps/windows/`:
#     pwsh ./scripts/build-asset.ps1
#
# Requires:
#   - ffmpeg on PATH  (winget install Gyan.FFmpeg, or scoop install ffmpeg)

$ErrorActionPreference = 'Stop'
Set-Location $PSScriptRoot/..

$source = Resolve-Path '../web/public/sounds/waterfall.ogg'
$dest = 'Cascade/Assets/waterfall.mp3'

if ((Test-Path $dest) -and ((Get-Item $dest).LastWriteTime -gt (Get-Item $source).LastWriteTime)) {
    Write-Host "$dest is up to date"
    exit 0
}

if (-not (Get-Command ffmpeg -ErrorAction SilentlyContinue)) {
    Write-Error "ffmpeg not found. Install via 'winget install Gyan.FFmpeg' or 'scoop install ffmpeg'."
    exit 1
}

New-Item -ItemType Directory -Force -Path (Split-Path $dest) | Out-Null

Write-Host "-> ffmpeg $source -> $dest"
& ffmpeg -y -i $source -codec:a libmp3lame -b:a 192k $dest
if ($LASTEXITCODE -ne 0) { throw 'ffmpeg failed' }

Write-Host "Wrote $dest ($((Get-Item $dest).Length / 1MB) MB)"
