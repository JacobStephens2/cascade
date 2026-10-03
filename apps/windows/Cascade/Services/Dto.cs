using System.Collections.Generic;
using System.Text.Json;
using System.Text.Json.Serialization;

namespace Cascade.Services;

/// <summary>
/// Mirrors the serde-tagged enums in `cascade-core`. The Rust crate is the
/// source of truth — these classes describe the JSON wire shape so we can
/// talk to the bridge in a typed way.
/// </summary>
public static class CascadeJson
{
    public static readonly JsonSerializerOptions Options = new()
    {
        PropertyNamingPolicy = JsonNamingPolicy.CamelCase,
        WriteIndented = false,
    };
}

// ---------- Commands ----------

[JsonPolymorphic(TypeDiscriminatorPropertyName = "type")]
[JsonDerivedType(typeof(PlayCommand), "play")]
[JsonDerivedType(typeof(PauseCommand), "pause")]
[JsonDerivedType(typeof(TogglePlaybackCommand), "togglePlayback")]
[JsonDerivedType(typeof(SetVolumeCommand), "setVolume")]
[JsonDerivedType(typeof(ToggleMuteCommand), "toggleMute")]
[JsonDerivedType(typeof(StartSleepTimerCommand), "startSleepTimer")]
[JsonDerivedType(typeof(StartPomodoroCommand), "startPomodoro")]
[JsonDerivedType(typeof(StartStopwatchCommand), "startStopwatch")]
[JsonDerivedType(typeof(CancelTimerCommand), "cancelTimer")]
[JsonDerivedType(typeof(TickCommand), "tick")]
[JsonDerivedType(typeof(PlatformPlaybackStartedCommand), "platformPlaybackStarted")]
[JsonDerivedType(typeof(PlatformPlaybackPausedCommand), "platformPlaybackPaused")]
[JsonDerivedType(typeof(PlatformPlaybackErrorCommand), "platformPlaybackError")]
[JsonDerivedType(typeof(SetListeningTrackingCommand), "setListeningTracking")]
[JsonDerivedType(typeof(RestoreCommand), "restore")]
[JsonDerivedType(typeof(BeginListeningSyncCommand), "beginListeningSync")]
[JsonDerivedType(typeof(ListeningSyncSucceededCommand), "listeningSyncSucceeded")]
[JsonDerivedType(typeof(ListeningSyncFailedCommand), "listeningSyncFailed")]
[JsonDerivedType(typeof(ResetListeningDataCommand), "resetListeningData")]
public abstract record CascadeCommand;

public sealed record PlayCommand : CascadeCommand;
public sealed record PauseCommand : CascadeCommand;
public sealed record TogglePlaybackCommand : CascadeCommand;
public sealed record SetVolumeCommand(int Percent) : CascadeCommand;
public sealed record ToggleMuteCommand : CascadeCommand;
public sealed record StartSleepTimerCommand(int Minutes) : CascadeCommand;
public sealed record StartPomodoroCommand(int Minutes) : CascadeCommand;
public sealed record StartStopwatchCommand : CascadeCommand;
public sealed record CancelTimerCommand : CascadeCommand;
public sealed record TickCommand(ulong ElapsedMs) : CascadeCommand;
public sealed record PlatformPlaybackStartedCommand : CascadeCommand;
public sealed record PlatformPlaybackPausedCommand : CascadeCommand;
public sealed record PlatformPlaybackErrorCommand(string Message) : CascadeCommand;
public sealed record SetListeningTrackingCommand(bool Enabled) : CascadeCommand;
public sealed record RestoreCommand(string SettingsJson, string ListeningJson, string FallbackDeviceId) : CascadeCommand;
public sealed record BeginListeningSyncCommand(SyncReason Reason) : CascadeCommand;
public sealed record ListeningSyncSucceededCommand(ulong ServerTotalMs) : CascadeCommand;
public sealed record ListeningSyncFailedCommand(bool Unauthorized) : CascadeCommand;
public sealed record ResetListeningDataCommand(string NewDeviceId) : CascadeCommand;

/// <summary>
/// Why the shell is asking to sync. The shell decides when it can talk; the
/// reason lets the core decide whether there is anything to say.
/// </summary>
[JsonConverter(typeof(SyncReasonConverter))]
public enum SyncReason { Threshold, Flush, Refresh }

internal sealed class SyncReasonConverter : System.Text.Json.Serialization.JsonConverter<SyncReason>
{
    public override SyncReason Read(ref System.Text.Json.Utf8JsonReader reader,
        System.Type typeToConvert, System.Text.Json.JsonSerializerOptions options) =>
        reader.GetString() switch
        {
            "threshold" => SyncReason.Threshold,
            "flush" => SyncReason.Flush,
            "refresh" => SyncReason.Refresh,
            var other => throw new System.Text.Json.JsonException($"unknown SyncReason '{other}'"),
        };

    public override void Write(System.Text.Json.Utf8JsonWriter writer, SyncReason value,
        System.Text.Json.JsonSerializerOptions options) =>
        writer.WriteStringValue(value switch
        {
            SyncReason.Threshold => "threshold",
            SyncReason.Flush => "flush",
            SyncReason.Refresh => "refresh",
            _ => "threshold",
        });
}

// ---------- Effects ----------

[JsonPolymorphic(TypeDiscriminatorPropertyName = "type")]
[JsonDerivedType(typeof(StartPlaybackEffect), "startPlayback")]
[JsonDerivedType(typeof(PausePlaybackEffect), "pausePlayback")]
[JsonDerivedType(typeof(SetPlatformVolumeEffect), "setPlatformVolume")]
[JsonDerivedType(typeof(PersistSettingsEffect), "persistSettings")]
[JsonDerivedType(typeof(PersistListeningEffect), "persistListening")]
[JsonDerivedType(typeof(PushListeningEffect), "pushListening")]
[JsonDerivedType(typeof(ClearSessionEffect), "clearSession")]
public abstract record CascadeEffect;

// Gain is the final output level, 0–1, mute and curve already applied.
public sealed record StartPlaybackEffect(double Gain) : CascadeEffect;
public sealed record PausePlaybackEffect : CascadeEffect;
public sealed record SetPlatformVolumeEffect(double Gain) : CascadeEffect;
public sealed record PersistSettingsEffect(string Json) : CascadeEffect;
public sealed record PersistListeningEffect(string Json) : CascadeEffect;
public sealed record PushListeningEffect(string DeviceId, ulong DeviceTotalMs) : CascadeEffect;
public sealed record ClearSessionEffect : CascadeEffect;

// ---------- Snapshot ----------

[JsonConverter(typeof(TimerKindConverter))]
public enum TimerKind { Off, Sleep, Pomodoro, Stopwatch, JustCompleted }

internal sealed class TimerKindConverter : System.Text.Json.Serialization.JsonConverter<TimerKind>
{
    public override TimerKind Read(ref System.Text.Json.Utf8JsonReader reader,
        System.Type typeToConvert, System.Text.Json.JsonSerializerOptions options) =>
        reader.GetString() switch
        {
            "off" => TimerKind.Off,
            "sleep" => TimerKind.Sleep,
            "pomodoro" => TimerKind.Pomodoro,
            "stopwatch" => TimerKind.Stopwatch,
            "justCompleted" => TimerKind.JustCompleted,
            var other => throw new System.Text.Json.JsonException($"unknown TimerKind '{other}'"),
        };

    public override void Write(System.Text.Json.Utf8JsonWriter writer, TimerKind value,
        System.Text.Json.JsonSerializerOptions options) =>
        writer.WriteStringValue(value switch
        {
            TimerKind.Off => "off",
            TimerKind.Sleep => "sleep",
            TimerKind.Pomodoro => "pomodoro",
            TimerKind.Stopwatch => "stopwatch",
            TimerKind.JustCompleted => "justCompleted",
            _ => "off",
        });
}

public sealed record TimerSnapshot(
    TimerKind Kind,
    string RemainingLabel,
    ulong RemainingMs,
    ulong TotalMs,
    float Progress
);

public sealed record ListeningSnapshot(
    bool TrackingEnabled,
    ulong DeviceTotalMs,
    ulong DisplayedTotalMs,
    ulong UnsyncedMs,
    string TotalLabel
);

public sealed record CascadeSnapshot(
    string Title,
    string Subtitle,
    bool IsPlaying,
    int VolumePercent,
    bool IsMuted,
    // Gain to output right now, 0–1: 0 while muted, else the curve applied.
    double OutputGain,
    string PrimaryButtonLabel,
    TimerSnapshot Timer,
    string? ErrorMessage,
    ListeningSnapshot Listening,
    // How often to send a TickCommand, in ms; 0 means stop. The core owns the cadence.
    int TickIntervalMs
);

public sealed record CascadeUpdate(
    CascadeSnapshot Snapshot,
    List<CascadeEffect> Effects
);
