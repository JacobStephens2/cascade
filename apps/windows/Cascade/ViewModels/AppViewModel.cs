using System;
using System.Text.Json;
using Cascade.Services;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Microsoft.UI.Dispatching;

namespace Cascade.ViewModels;

/// <summary>
/// MVVM root: owns the Rust bridge, the audio engine, the SMTC publisher,
/// the persisted settings store, and the tick scheduler. Every view binds
/// directly to <see cref="Snapshot"/> for rendering and calls one of the
/// generated relay commands for user input.
///
/// Same shape as the Android <c>CascadeBridgeHolder</c>, the macOS
/// <c>AppStore</c>, and the web <c>useCascade</c> hook — just expressed in
/// CommunityToolkit.Mvvm's source-generator style.
/// </summary>
public sealed partial class AppViewModel : ObservableObject, IDisposable
{
    private readonly CoreBridge _bridge;
    private readonly AudioEngine _audio;
    private readonly SmtcController _smtc;
    private readonly SettingsStore _settings;
    private readonly PowerController _power;
    private readonly TickScheduler _tick;
    private readonly DispatcherQueue _dispatcher;

    [ObservableProperty]
    private CascadeSnapshot snapshot;

    [ObservableProperty]
    private string? errorMessage;

    public AppViewModel(DispatcherQueue dispatcher)
    {
        _dispatcher = dispatcher;
        _settings = new SettingsStore();
        _bridge = new CoreBridge(_settings.ReadSafely());
        _audio = new AudioEngine();
        _smtc = new SmtcController(_audio.Player);
        _power = new PowerController();
        _tick = new TickScheduler(dispatcher);

        snapshot = JsonSerializer.Deserialize<CascadeSnapshot>(
            _bridge.Snapshot(), CascadeJson.Options)!;

        _smtc.BindDispatch(Send);
        _smtc.Update(snapshot);

        // Restore the listening ledger once at startup. The core ignores a
        // missing/incompatible blob and never lets a restore lower the counter.
        var listeningJson = _settings.ReadListeningSafely();
        if (!string.IsNullOrEmpty(listeningJson))
        {
            Send(new RestoreListeningCommand(listeningJson));
        }
    }

    public void Send(CascadeCommand command)
    {
        try
        {
            var commandJson = JsonSerializer.Serialize(command, CascadeJson.Options);
            var updateJson = _bridge.Dispatch(commandJson);
            var update = JsonSerializer.Deserialize<CascadeUpdate>(updateJson, CascadeJson.Options)!;
            Apply(update);
            ErrorMessage = update.Snapshot.ErrorMessage;
        }
        catch (Exception ex)
        {
            ErrorMessage = ex.Message;
        }
    }

    private void Apply(CascadeUpdate update)
    {
        Snapshot = update.Snapshot;

        foreach (var effect in update.Effects)
        {
            switch (effect)
            {
                case StartPlaybackEffect start:
                    _audio.Start(start.VolumePercent);
                    _power.Acquire();
                    // Tell the core the platform actually started; the dispatch is
                    // sync, so post via the dispatcher queue to avoid recursing in
                    // the same call frame.
                    _dispatcher.TryEnqueue(() => Send(new PlatformPlaybackStartedCommand()));
                    break;
                case PausePlaybackEffect:
                    _audio.Pause();
                    _power.Release();
                    break;
                case SetPlatformVolumeEffect setVol:
                    _audio.SetVolume(setVol.VolumePercent);
                    break;
                case PersistSettingsEffect persist:
                    _settings.WriteSafely(persist.Json);
                    break;
                case PersistListeningEffect persistListening:
                    _settings.WriteListeningSafely(persistListening.Json);
                    break;
            }
        }

        // Drive the tick loop while a timer is running (fine cadence) and also
        // while audio is simply playing (coarse cadence, just to accrue
        // listening time). Restart only when the cadence actually changes.
        var nowTimer = update.Snapshot.Timer.Kind;
        var timerActive = nowTimer is TimerKind.Sleep or TimerKind.Pomodoro or TimerKind.Stopwatch;
        var desiredInterval = timerActive ? 250 : (update.Snapshot.IsPlaying ? 1000 : 0);
        if (desiredInterval != _tick.IntervalMs)
        {
            _tick.Stop();
            if (desiredInterval > 0)
                _tick.Start(elapsedMs => Send(new TickCommand(elapsedMs)), desiredInterval);
        }

        _smtc.Update(update.Snapshot);
    }

    // ---- Relay commands wired into XAML ----

    [RelayCommand]
    private void TogglePlayback() => Send(new TogglePlaybackCommand());

    [RelayCommand]
    private void StartThirty() => Send(new StartPomodoroCommand(30));

    [RelayCommand]
    private void StartSixty() => Send(new StartPomodoroCommand(60));

    [RelayCommand]
    private void StartEightHours() => Send(new StartPomodoroCommand(480));

    [RelayCommand]
    private void StartSleepFifteen() => Send(new StartSleepTimerCommand(15));

    [RelayCommand]
    private void StartSleepThirty() => Send(new StartSleepTimerCommand(30));

    [RelayCommand]
    private void StartSleepSixty() => Send(new StartSleepTimerCommand(60));

    [RelayCommand]
    private void StartStopwatch() => Send(new StartStopwatchCommand());

    [RelayCommand]
    private void CancelTimer() => Send(new CancelTimerCommand());

    [RelayCommand]
    private void SetVolume(double percent) => Send(new SetVolumeCommand((int)percent));

    [RelayCommand]
    private void ToggleMute() => Send(new ToggleMuteCommand());

    [RelayCommand]
    private void ToggleListeningTracking() =>
        Send(new SetListeningTrackingCommand(!Snapshot.Listening.TrackingEnabled));

    /// Start a user-entered duration. `sleep` picks the timer flavor:
    /// sleep timer (play, then stop) vs focus session.
    public void StartCustom(int minutes, bool sleep)
    {
        if (minutes < 1 || minutes > 1440) return;
        Send(sleep ? new StartSleepTimerCommand(minutes) : new StartPomodoroCommand(minutes));
    }

    public void Dispose()
    {
        _tick.Stop();
        _smtc.Dispose();
        _audio.Dispose();
        _bridge.Dispose();
    }
}
