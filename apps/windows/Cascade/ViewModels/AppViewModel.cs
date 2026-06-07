using System;
using System.Text.Json;
using System.Threading.Tasks;
using Cascade.Services;
using CommunityToolkit.Mvvm.ComponentModel;
using CommunityToolkit.Mvvm.Input;
using Microsoft.UI.Dispatching;
using Microsoft.UI.Xaml;

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

    // ---- account / sync ----
    private readonly AccountStore _accountStore = new();
    private readonly SyncApi _syncApi = new();
    private bool _syncing;
    private const long SyncThresholdMs = 30_000;

    public bool SyncAvailable => SyncConfig.Available;

    [ObservableProperty]
    private Account? account;

    [ObservableProperty]
    private string? syncStatus;

    [ObservableProperty]
    private string emailInput = "";

    [ObservableProperty]
    private string signInLinkInput = "";

    public string AccountEmail => Account?.Email ?? "";

    // Bound directly (not via an x:Bind function) so the signed-in / signed-out
    // panels flip whenever Account changes at runtime — function bindings on
    // Account proved not to re-evaluate, stranding the view after sign-out / a
    // 401. Explicit notification in OnAccountChanged drives these.
    public Visibility SignedInVisibility =>
        Account is not null ? Visibility.Visible : Visibility.Collapsed;

    public Visibility SignedOutVisibility =>
        Account is not null ? Visibility.Collapsed : Visibility.Visible;

    partial void OnAccountChanged(Account? value)
    {
        OnPropertyChanged(nameof(AccountEmail));
        OnPropertyChanged(nameof(SignedInVisibility));
        OnPropertyChanged(nameof(SignedOutVisibility));
    }

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

        Account = _accountStore.ReadAccount();
        if (Account is not null)
        {
            _ = SyncAsync();
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

        // Sync cadence lives in the shell: push once enough unsynced time has
        // accrued. Started on the UI thread, so the await continuation (and the
        // ApplySyncedTotal dispatch) resume on it too.
        if (Account is not null && update.Snapshot.Listening.UnsyncedMs >= (ulong)SyncThresholdMs)
        {
            _ = SyncAsync();
        }
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

    // ---- account / sync commands ----

    private async Task SyncAsync()
    {
        if (_syncing || Account is null) return;
        _syncing = true;
        try
        {
            var deviceTotal = (long)Snapshot.Listening.DeviceTotalMs;
            var res = await _syncApi.PutListeningAsync(
                Account.SessionToken, _accountStore.DeviceId(), deviceTotal);
            Send(new ApplySyncedTotalCommand((ulong)deviceTotal, (ulong)res.ServerTotalMs));
        }
        catch (SyncHttpException e) when (e.Status == 401)
        {
            // The await above may resume off the UI thread; marshal the account
            // mutation back so the bound visibility actually updates (and we
            // never touch observable state from a background thread).
            _dispatcher.TryEnqueue(() =>
            {
                Account = null;
                _accountStore.ClearAccount();
                SyncStatus = "Signed out — sign in again to sync.";
            });
        }
        catch
        {
            // offline / transient — retry on the next trigger
        }
        finally
        {
            _syncing = false;
        }
    }

    [RelayCommand]
    private async Task RequestLink()
    {
        var email = EmailInput.Trim();
        if (email.Length == 0) return;
        try
        {
            await _syncApi.RequestLinkAsync(email);
            SyncStatus = $"Check {email} for a sign-in link.";
        }
        catch
        {
            SyncStatus = "Couldn't send the sign-in link.";
        }
    }

    /// <summary>
    /// Entry point for a <c>cascade://auth?token=…</c> deep link: reuse the
    /// exact paste-and-sign-in path so the link handoff and manual paste behave
    /// identically (same token extraction, verify, persist, and status text).
    /// </summary>
    public Task SignInWithLinkAsync(string link)
    {
        SignInLinkInput = link;
        return CompleteSignInCommand.ExecuteAsync(null);
    }

    [RelayCommand]
    private async Task CompleteSignIn()
    {
        var token = ExtractToken(SignInLinkInput);
        if (token is null) { SyncStatus = "Paste the full sign-in link."; return; }
        try
        {
            var res = await _syncApi.VerifyAsync(token);
            Account = new Account(res.SessionToken, res.Email);
            _accountStore.WriteAccount(Account);
            SignInLinkInput = "";
            SyncStatus = $"Signed in as {res.Email}.";
            await SyncAsync();
        }
        catch
        {
            SyncStatus = "That sign-in link was invalid or expired.";
        }
    }

    [RelayCommand]
    private async Task SignOut()
    {
        var prev = Account;
        Account = null;
        _accountStore.ClearAccount();
        SyncStatus = null;
        if (prev is not null)
        {
            try { await _syncApi.LogoutAsync(prev.SessionToken); } catch { }
        }
    }

    [RelayCommand]
    private async Task DeleteListeningData()
    {
        if (Account is null) return;
        try
        {
            await _syncApi.DeleteListeningAsync(Account.SessionToken);
            _accountStore.RotateDeviceId();
            Send(new ResetListeningDataCommand());
            SyncStatus = "Listening data deleted.";
        }
        catch { SyncStatus = "Couldn't delete listening data."; }
    }

    [RelayCommand]
    private async Task DeleteAccount()
    {
        if (Account is null) return;
        try
        {
            await _syncApi.DeleteAccountAsync(Account.SessionToken);
            _accountStore.RotateDeviceId();
            Send(new ResetListeningDataCommand());
            Account = null;
            _accountStore.ClearAccount();
            SyncStatus = "Account deleted.";
        }
        catch { SyncStatus = "Couldn't delete the account."; }
    }

    /// Pull the token out of a pasted sign-in URL (…/auth?token=XYZ), or accept
    /// a raw token. Desktop protocol-activation is the on-device follow-up.
    private static string? ExtractToken(string input)
    {
        var s = input.Trim();
        if (s.Length == 0) return null;
        var idx = s.IndexOf("token=", StringComparison.OrdinalIgnoreCase);
        if (idx >= 0)
        {
            var rest = s.Substring(idx + "token=".Length);
            var amp = rest.IndexOf('&');
            return amp >= 0 ? rest.Substring(0, amp) : rest;
        }
        return s;
    }

    public void Dispose()
    {
        _tick.Stop();
        _smtc.Dispose();
        _audio.Dispose();
        _bridge.Dispose();
    }
}
