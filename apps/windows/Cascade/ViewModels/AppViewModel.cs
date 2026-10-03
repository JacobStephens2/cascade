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
        Dispatch(new RestoreCommand(
            _settings.ReadSafely() ?? "",
            _settings.ReadListeningSafely() ?? "",
            fallbackDeviceId,
            _accountStore.ReadAccountJson()));

        // Fetch the cross-device total straight away; the core sends nothing
        // while signed out.
        BeginSync(SyncReason.Refresh);
    }

    /// <summary>
    /// Dispatch a command from the UI, then offer the core a threshold sync
    /// (see <see cref="Dispatch"/> for the plain form the sync flow uses).
    /// </summary>
    public void Send(CascadeCommand command)
    {
        Dispatch(command);

        // Routine check after every update: the shell only says it *can* talk;
        // the core decides whether it is signed in and enough unsynced time has
        // accrued, and answers with nothing while a sync is already in flight.
        BeginSync(SyncReason.Threshold);
    }

    /// <summary>
    /// Dispatch one command and apply its update, carrying any request it
    /// asks for. Unlike <see cref="Send"/>, never offers a sync itself — a
    /// settle dispatches through here so it can't recurse into another begin.
    /// </summary>
    private void Dispatch(CascadeCommand command)
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
                default:
                    CarryRequest(effect);
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

    [RelayCommand(CanExecute = nameof(AccountIdle))]
    private void RequestSignInLink() => Send(new RequestSignInLinkCommand(EmailInput));

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
    private void DeleteListeningData() => Send(new DeleteListeningDataCommand());

    [RelayCommand(CanExecute = nameof(AccountIdle))]
    private void DeleteAccount() => Send(new DeleteAccountCommand());

    /// <summary>
    /// The shell decides when it can talk; the core decides whether there is
    /// anything to say, and what. A PushListening in the answer is carried by
    /// <see cref="CarryRequest"/>.
    /// </summary>
    private void BeginSync(SyncReason reason)
    {
        if (SyncConfig.Available) Dispatch(new BeginListeningSyncCommand(reason));
    }

    /// <summary>
    /// Carry one request effect over HTTP and settle it with the core. Every
    /// request but RevokeSession must be settled (success or failure), or the
    /// core never starts another. Not a request: nothing to do.
    /// </summary>
    private void CarryRequest(CascadeEffect effect)
    {
        static CascadeCommand AccountFailed(bool unauthorized) =>
            new AccountRequestFailedCommand(unauthorized);

        switch (effect)
        {
            case PushListeningEffect push:
                _ = SettleWithAsync(
                    () => _syncApi.PutListeningAsync(
                        push.SessionToken, push.DeviceId, (long)push.DeviceTotalMs),
                    res => new ListeningSyncSucceededCommand((ulong)Math.Max(0L, res.ServerTotalMs)),
                    unauthorized => new ListeningSyncFailedCommand(unauthorized));
                break;
            case SendSignInLinkEffect send:
                _ = SettleWithAsync(
                    () => _syncApi.RequestLinkAsync(send.Email),
                    () => new SignInLinkSentCommand(),
                    AccountFailed);
                break;
            case VerifySignInTokenEffect verify:
                _ = SettleWithAsync(
                    () => _syncApi.VerifyAsync(verify.Token),
                    res => new SignInVerifiedCommand(res.SessionToken, res.Email),
                    AccountFailed);
                break;
            case RevokeSessionEffect revoke:
                _ = RevokeAsync(revoke.SessionToken);
                break;
            case DeleteServerListeningEffect delete:
                _ = SettleWithAsync(
                    () => _syncApi.DeleteListeningAsync(delete.SessionToken),
                    () => new ListeningDataDeletedCommand(NewDeviceId()),
                    AccountFailed);
                break;
            case DeleteServerAccountEffect delete:
                _ = SettleWithAsync(
                    () => _syncApi.DeleteAccountAsync(delete.SessionToken),
                    () => new AccountDeletedCommand(NewDeviceId()),
                    AccountFailed);
                break;
        }
    }

    /// A fresh random device id. The core has no randomness, so the shell
    /// supplies one for the fallback id and each delete's slot rotation.
    private static string NewDeviceId() => Guid.NewGuid().ToString();

    /// Fire-and-forget: nothing settles a revoke.
    private async Task RevokeAsync(string sessionToken)
    {
        try { await _syncApi.LogoutAsync(sessionToken); }
        catch { /* already gone server-side or offline — local sign-out stands */ }
    }

    /// Run one request and settle it with the command its result maps to.
    /// Offline / transient / rejected all fail; only a 401 is unauthorized,
    /// and what that does is the core's call.
    private async Task SettleWithAsync<T>(
        Func<Task<T>> call,
        Func<T, CascadeCommand> succeeded,
        Func<bool, CascadeCommand> failed)
    {
        CascadeCommand outcome;
        try
        {
            outcome = succeeded(await call());
        }
        catch (Exception e)
        {
            outcome = failed(e is SyncHttpException { Status: 401 });
        }
        OnUiThread(() => Dispatch(outcome));
    }

    private Task SettleWithAsync(
        Func<Task> call,
        Func<CascadeCommand> succeeded,
        Func<bool, CascadeCommand> failed) =>
        SettleWithAsync(
            async () => { await call(); return true; },
            _ => succeeded(),
            failed);

    /// Started on the UI thread, so await continuations normally resume on it;
    /// marshal back defensively so the settle dispatch and the bound snapshot
    /// are never touched from a background thread.
    private void OnUiThread(Action action)
    {
        if (_dispatcher.HasThreadAccess) action();
        else _dispatcher.TryEnqueue(() => action());
    }

    public void Dispose()
    {
        _tick.Stop();
        _smtc.Dispose();
        _audio.Dispose();
        _bridge.Dispose();
    }
}
