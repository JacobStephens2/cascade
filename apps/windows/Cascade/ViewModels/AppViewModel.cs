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
    private readonly SyncServer _syncServer = new();

    public bool SyncAvailable => SyncConfig.Available;

    [ObservableProperty]
    private string emailInput = "";

    [ObservableProperty]
    private string signInLinkInput = "";

    // The core holds the account; these only read the snapshot's account
    // section. Bound directly (not via an x:Bind function) so the signed-in /
    // signed-out panels flip whenever the account changes at runtime —
    // function bindings proved not to re-evaluate, stranding the view after
    // sign-out / a 401. Explicit notification in OnSnapshotChanged drives these.
    private bool SignedIn => Snapshot.Account.Email is not null;

    public Visibility SignedInVisibility =>
        SignedIn ? Visibility.Visible : Visibility.Collapsed;

    public Visibility SignedOutVisibility =>
        SignedIn ? Visibility.Collapsed : Visibility.Visible;

    // Every account control but sign-out waits while a request is out.
    private bool AccountIdle => !Snapshot.Account.Busy;

    partial void OnSnapshotChanged(CascadeSnapshot? oldValue, CascadeSnapshot newValue)
    {
        var wasSignedIn = oldValue?.Account.Email is not null;
        if (SignedIn != wasSignedIn)
        {
            // The pasted link is spent once it signs in.
            if (SignedIn) SignInLinkInput = "";
            OnPropertyChanged(nameof(SignedInVisibility));
            OnPropertyChanged(nameof(SignedOutVisibility));
        }
        if (oldValue?.Account.Busy != newValue.Account.Busy)
        {
            RequestSignInLinkCommand.NotifyCanExecuteChanged();
            SubmitSignInLinkCommand.NotifyCanExecuteChanged();
            DeleteListeningDataCommand.NotifyCanExecuteChanged();
            DeleteAccountCommand.NotifyCanExecuteChanged();
        }
    }

    public AppViewModel(DispatcherQueue dispatcher)
    {
        _dispatcher = dispatcher;
        _settings = new SettingsStore();
        _bridge = new CoreBridge();
        _audio = new AudioEngine();
        _smtc = new SmtcController(_audio.Player);
        _power = new PowerController();
        _tick = new TickScheduler(dispatcher);

        snapshot = JsonSerializer.Deserialize<CascadeSnapshot>(
            _bridge.Snapshot(), CascadeJson.Options)!;

        _smtc.BindDispatch(Send);
        _smtc.Update(snapshot);

        // Boot is one step: a fresh core, then one restore carrying both
        // persisted blobs (empty = none) and a fallback device id. The core
        // owns the device id but has no randomness: it adopts the fallback id
        // only if the listening blob carries none. The id older builds stored
        // themselves goes first, so an existing server slot carries over. The
        // core ignores a missing/incompatible blob and never lets a restore
        // lower the counter. The account blob rides along; the core holds the
        // account from here on.
        var fallbackDeviceId = _accountStore.ReadLegacyDeviceId() ?? NewDeviceId();
        Send(new RestoreCommand(
            _settings.ReadSafely() ?? "",
            _settings.ReadListeningSafely() ?? "",
            fallbackDeviceId,
            _accountStore.ReadAccountJson()));

        // Fetch the cross-device total straight away; the core sends nothing
        // while signed out.
        BeginSync(SyncReason.Refresh);
    }

    /// <summary>
    /// Dispatch one command and apply its update, carrying any request it
    /// asks for. Every command comes through here — UI, media keys, the tick
    /// loop, sync begins and settles; the core decides when a routine sync is
    /// due and answers the tick that crosses the threshold with the push.
    /// </summary>
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
                    _audio.Start(start.Gain);
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
                    _audio.SetVolume(setVol.Gain);
                    break;
                case PersistSettingsEffect persist:
                    _settings.WriteSafely(persist.Json);
                    break;
                case PersistListeningEffect persistListening:
                    _settings.WriteListeningSafely(persistListening.Json);
                    break;
                case PersistAccountEffect persistAccount:
                    _accountStore.WriteAccountJson(persistAccount.Json);
                    break;
                case ServerRequestEffect request:
                    _ = CarryAsync(request);
                    break;
            }
        }

        // Drive the tick loop at the cadence the core asks for (0 = stop).
        // Restart only when the cadence actually changes.
        var desiredInterval = update.Snapshot.TickIntervalMs;
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

    /// Start a timer preset or a user-entered duration. `sleep` picks the
    /// timer flavor: sleep timer (play, then stop) vs focus session. The core
    /// clamps `minutes` into its limits.
    public void StartTimer(int minutes, bool sleep) =>
        Send(sleep ? new StartSleepTimerCommand(minutes) : new StartPomodoroCommand(minutes));

    // ---- account / sync commands ----

    // The "windows" platform makes the emailed link carry &app=windows, so the
    // web /auth page hands the token to this app via cascade:// rather than
    // consuming it in the browser.
    [RelayCommand(CanExecute = nameof(AccountIdle))]
    private void RequestSignInLink() => Send(new RequestSignInLinkCommand(EmailInput, "windows"));

    [RelayCommand(CanExecute = nameof(AccountIdle))]
    private void SubmitSignInLink() => Send(new SubmitSignInLinkCommand(SignInLinkInput));

    /// <summary>
    /// Entry point for a <c>cascade://auth?token=…</c> deep link: the core
    /// reads the link exactly as it reads a pasted one.
    /// </summary>
    public void SignInWithLink(string link) => Send(new SubmitSignInLinkCommand(link));

    [RelayCommand]
    private void SignOut() => Send(new SignOutCommand());

    [RelayCommand(CanExecute = nameof(AccountIdle))]
    private void DeleteListeningData() => Send(new DeleteListeningDataCommand(NewDeviceId()));

    [RelayCommand(CanExecute = nameof(AccountIdle))]
    private void DeleteAccount() => Send(new DeleteAccountCommand(NewDeviceId()));

    /// <summary>
    /// The shell decides when it can talk; the core decides whether there is
    /// anything to say, and what. The push in the answer is carried by
    /// <see cref="CarryAsync"/>.
    /// </summary>
    private void BeginSync(SyncReason reason)
    {
        if (SyncConfig.Available) Send(new BeginListeningSyncCommand(reason));
    }

    /// <summary>
    /// Carry one request to the sync server and settle it with the core. Every
    /// request is settled, failures and the sign-out revoke included (status
    /// 0 when nothing came back), or the core never starts another.
    /// </summary>
    private async Task CarryAsync(ServerRequestEffect request)
    {
        var response = await _syncServer.CarryAsync(request);
        // Settle on the UI thread, which owns the dispatch and the bound
        // snapshot, and never inside the dispatch that asked for the request:
        // with no sync server the answer is already here and Apply is still
        // running.
        _dispatcher.TryEnqueue(() => Send(response));
    }

    /// A fresh random device id. The core has no randomness, so the shell
    /// supplies one for the fallback id and each delete's slot rotation.
    private static string NewDeviceId() => Guid.NewGuid().ToString();

    public void Dispose()
    {
        _tick.Stop();
        _smtc.Dispose();
        _audio.Dispose();
        _bridge.Dispose();
    }
}
