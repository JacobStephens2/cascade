# Cascade — Windows

Native WinUI 3 / C# / .NET shell over `cascade-core` via a hand-rolled C ABI.

End users: install from
[cascade.stephens.page/apps](https://cascade.stephens.page/apps)
(unsigned zip). This README is for building from source.

## What's here

```
apps/windows/
├── Cascade.sln
├── scripts/
│   ├── build.ps1                 # one-shot: assets + Rust + DLL copy
│   ├── build-asset.ps1           # OGG → MP3 via ffmpeg
│   └── build-rust.ps1            # cargo cross-compile + copy DLLs
└── Cascade/
    ├── Cascade.csproj            # WinUI 3, .NET 8 (net8.0-windows10.0.19041)
    ├── App.xaml / App.xaml.cs
    ├── MainWindow.xaml / .xaml.cs
    ├── Converters.cs             # x:Bind static helpers
    ├── Package.appxmanifest      # bundle id, capabilities, StartupTask
    ├── app.manifest              # DPI / UTF-8 codepage
    ├── ViewModels/
    │   └── AppViewModel.cs       # MVVM root: snapshot + RelayCommands
    ├── Services/
    │   ├── CoreBridge.cs         # [DllImport] over cascade_uniffi.dll
    │   ├── Dto.cs                # System.Text.Json tagged-enum DTOs
    │   ├── AudioEngine.cs        # MediaPlayer + MediaPlaybackList
    │   ├── SmtcController.cs     # System Media Transport Controls
    │   ├── TickScheduler.cs      # DispatcherQueueTimer 250ms cadence
    │   ├── SettingsStore.cs      # %LOCALAPPDATA%\Cascade\settings.json
    │   └── PowerController.cs    # SetThreadExecutionState
    ├── Native/
    │   ├── x64/cascade_uniffi.dll    # built, gitignored
    │   └── arm64/cascade_uniffi.dll  # built, gitignored
    └── Assets/                   # icons + waterfall.mp3 (gitignored)
```

## Build prerequisites (Windows machine)

```powershell
# Rust toolchain + Windows targets
winget install Rustlang.Rustup
rustup target add x86_64-pc-windows-msvc aarch64-pc-windows-msvc

# Tooling
winget install Microsoft.VisualStudio.2022.Community `
    --override "--add Microsoft.VisualStudio.Workload.NativeDesktop `
                --add Microsoft.VisualStudio.Workload.ManagedDesktop `
                --add Microsoft.VisualStudio.Workload.Universal `
                --add Microsoft.VisualStudio.Component.Windows11SDK.22621"
winget install Gyan.FFmpeg
winget install Microsoft.WindowsSDK.10.0.22621
```

## First build

```powershell
git pull
cd apps/windows
pwsh ./scripts/build.ps1
start Cascade.sln                  # opens in Visual Studio
```

In Visual Studio:
1. Set the active configuration to **Debug | x64** (or arm64 if running on
   ARM64 Windows).
2. F5 to build + launch.

Or from the CLI:

```powershell
dotnet build Cascade/Cascade.csproj -c Debug /p:Platform=x64
# The .exe lands in Cascade/bin/x64/Debug/net8.0-windows10.0.19041.0/win-x64/
```

## Architectural choices

| Concern | Owner |
|---|---|
| State machine, timer math, settings schema, snapshots | `cascade-core` (Rust, shared) |
| Rust ↔ C# bridge | hand-rolled C ABI in `cascade-uniffi` (see `crates/cascade-uniffi/src/lib.rs` `c_abi` module). No third-party `uniffi-bindgen-cs` dependency. |
| Audio playback | `AudioEngine.cs` — `MediaPlayer` + `MediaPlaybackList { AutoRepeatEnabled = true }`. Gapless via the playlist loop, not the raw player. |
| Lock-screen / media keys | `SmtcController.cs` — `SystemMediaTransportControls` rides the same `MediaPlayer` instance. |
| Settings | `%LOCALAPPDATA%\Cascade\settings.json` — same JSON blob the web / Android / macOS clients round-trip. |
| Power | `PowerController.cs` — `SetThreadExecutionState(CONTINUOUS|SYSTEM_REQUIRED)` during active sessions. |
| Tick loop | `DispatcherQueueTimer` at 250 ms; only runs while a timer is active. |

The Rust core declares **intent** (`Effect.startPlayback { volumePercent }`);
the Windows shell decides **how** (MediaPlayer, with a square-law volume
curve matching the web / macOS shells).

## Day-to-day iteration

| You changed... | Run |
|---|---|
| A C# / XAML file | F5 in Visual Studio |
| A Rust file in `cascade-core` or `cascade-uniffi` | `pwsh ./scripts/build-rust.ps1` then F5 |
| `waterfall.ogg` | `pwsh ./scripts/build-asset.ps1` then F5 |

## Known follow-ups

- No tray icon yet — H.NotifyIcon.WinUI is the planned add. Lives behind a
  `TrayController` service so it can plug in without touching the
  ViewModel.
- No single-instance enforcement — relies on the user not launching twice.
  Adding via `Microsoft.Windows.AppLifecycle.AppInstance.FindOrRegisterForKey`
  needs a hand-rolled `Main` (Program.cs), which collides with WinUI's
  auto-generated entry point. Deferred.
- The `StartupTask` extension in the manifest only takes effect under an
  MSIX install. The Settings page wiring (`StartupTask.GetAsync(...)`) is
  the planned follow-up.
- Unpackaged build outputs the .exe + DLLs side by side. For real
  distribution, switch the csproj to `<WindowsPackageType>MSIX</WindowsPackageType>`
  and add a Package Project, then sign with a self-signed cert.
  Hosting an `.appinstaller` at `cascade.stephens.page/windows/` is
  **not built**. Until then, the unsigned zip is on
  [cascade.stephens.page/apps](https://cascade.stephens.page/apps).
- App icons are placeholders generated from the existing web icon at
  scaffold time; replace with proper Windows asset sizes before
  publishing.
